//! One tick of the player's motion: the velocity an intent asks for, the gravity
//! that acts on it, and where the two leave the player.
//!
//! **Integration is semi-implicit Euler**, and the order is the whole content of
//! the numbers this feature is asserted against: gravity is taken from the
//! velocity, and only then is the velocity applied to the position. A jump of
//! 9.0 blocks per second therefore reaches 1.275 blocks and not the 1.35 the
//! continuous `v²/2g` gives — the discrete sum is what the player actually
//! traces, and the continuous figure is a number no correct implementation of
//! this model can produce.
//!
//! **A jump the medium alone admits leaves at what the medium declares**, not at
//! the player's own jump speed: `launched` reads the volume's `swim_ascent`
//! there, and the ground path is untouched. The same order applies to it — the
//! declared ascent is launched, gravity bites, and only then does the resistance
//! divide — so the rise a swimmer observes is `(ascent − g·dt) / (1 + r)` and
//! never the declared number itself.
//!
//! **Both declared coefficients are therefore bound to this 60 Hz tick.** The
//! gravity bite is one tick's worth whatever a tick is worth, so a declaration
//! that rises at some rate here rises at another rate at another tick rate. The
//! binding is stated rather than removed: taking the bite before the launch
//! would make the rise rate-free, and the scenarios this feature is asserted
//! against are worded on the order above.
//!
//! Nothing here reads a clock. A tick is a declared quantum of simulated time,
//! which is what lets the same intents replay to the same state on a machine of
//! any speed (`crates/mc-sim/CLAUDE.md`).

use std::time::Duration;

use glam::{Vec2, Vec3};

use crate::player::collide;
use crate::player::{Look, MovementIntent, PlayerState, Traversal, VoxelMedium};

/// How long one tick simulates, in seconds.
///
/// Declared, never measured — and what a driver spends *into* it is elapsed time
/// rather than frames. A client's frame path reads its clock once a frame and
/// spends whole quanta of [`TICK_QUANTUM`] out of the interval, so a walk covers
/// the same ground per second whatever rate the frames arrive at.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// The same quantum, in the units a frame path measures elapsed time in.
///
/// **One number, spelled twice, and the sibling test is what keeps them one.**
/// The physics needs seconds as an `f32` to multiply a velocity by; a driver
/// needs a [`Duration`] to subtract from an interval, because a driver
/// accumulates and `f32` seconds accumulated over an hour lose the resolution a
/// tick is measured at. Deriving either from the other at compile time is not
/// available — neither `Duration::from_secs_f32` nor `Duration::as_secs_f32` is
/// `const` — so they are declared side by side and asserted equal to within a
/// nanosecond by `physics_test.rs`.
///
/// A sixtieth of a second is 16 666 666.67 nanoseconds, so this is the nearest
/// nanosecond and sits a third of one above the exact quantum. Over a full hour
/// of play that is 0.07 ms of simulated time — five orders of magnitude below one
/// tick.
pub const TICK_QUANTUM: Duration = Duration::from_nanos(16_666_667);

/// How fast a walk carries the player, in blocks per second.
const WALK_SPEED: f32 = 4.5;

/// How fast a fall accelerates, in blocks per second squared.
const GRAVITY: f32 = 30.0;

/// How fast a jump leaves the ground, in blocks per second.
///
/// The arc it buys clears a one-block rise and never a two-block one, which is
/// what makes stepping up automatically unnecessary rather than missing.
const JUMP_SPEED: f32 = 9.0;

/// The fastest a fall ever goes, in blocks per second.
///
/// Not comfort: 48.0 blocks per second is 0.8 blocks in a tick, and per-axis
/// resolution is only exact while a tick moves the box less than a whole block
/// on each axis, because only then can the box newly overlap the adjacent voxel
/// layer.
const TERMINAL_SPEED: f32 = 48.0;

/// The largest a walk request is ever taken to be.
///
/// A request is a direction *and* a magnitude, so this is a cap rather than a
/// normalisation: what is asked for below full deflection is what is walked.
const FULL_DEFLECTION: f32 = 1.0;

/// How far a tick may displace the player on any one axis, in blocks.
const DISPLACEMENT_LIMIT: f32 = 1.0;

/// Where one tick leaves the player.
///
/// Pure: the same state, intent and world always give the same answer, nothing
/// reads a clock, and the state that comes back is a new value rather than a
/// mutation of the one handed in — which is what keeps the simulation the only
/// owner of the state a snapshot is published from, and what lets a scenario
/// about a velocity no intent can express be stated at all.
///
/// **The look is accumulated before the walk reads it**, so a tick that turns
/// and walks at once walks where it turned to rather than where it came from.
/// Deferring it by a tick would be a camera that lags its own player by one
/// frame — invisible in a still, and exactly the drift a replay accumulates over
/// a scripted turn.
///
/// **The medium is read once, from the box at the *start* of the tick.** The
/// resistance decides the displacement, so a medium read from the resolved end
/// position would be read from a position the resistance itself produced — and
/// "while the player's box overlaps" is a fact about where the tick begins.
///
/// **One object answers both questions, and that is what the composite door is
/// for.** Two parameters would let a caller hand a solidity view of one world
/// beside a medium view of another, which is the disagreement `World::adopt`
/// exists to prevent. `Targetable` is deliberately not among what
/// [`Traversal`] names: a tick of motion has no aiming question to ask.
#[must_use]
pub fn advance_player(
    state: PlayerState,
    intent: &MovementIntent,
    world: &dyn Traversal,
) -> PlayerState {
    let medium = collide::medium_around(state.position, world);
    let look = Look::of(&state).accumulate(intent);
    let walk = walk_velocity(intent, look.yaw);
    let velocity = slowed(
        walk.with_y(fallen(launched(&state, intent, medium))),
        medium.resistance,
    );
    let displacement = bounded(velocity * TICK_DURATION);
    let resolved = collide::resolved_position(state.position, displacement, world);
    let on_ground = collide::on_ground(resolved.feet, world);
    PlayerState {
        position: resolved.feet,
        velocity: settled(velocity, on_ground, resolved.stopped_vertically),
        yaw: look.yaw,
        pitch: look.pitch,
        on_ground,
    }
}

/// The horizontal velocity an intent asks for, in the declared basis.
///
/// *Set* rather than accumulated, which is why a walk covers exactly its speed
/// for as long as it is held: there is no acceleration to build up and no
/// inertia to shed, so a player asked for nothing stops on the tick it is asked.
///
/// The basis is the contract. Yaw 0 faces +x and a quarter turn takes forward to
/// +z, so forward is `(cos yaw, 0, sin yaw)` and the right hand of it is
/// `(−sin yaw, 0, cos yaw)`. Exchanging the sine and the cosine is a quarter turn
/// and negating the right vector is a mirror, and both stay smooth, total and
/// reproducible while being wrong.
fn walk_velocity(intent: &MovementIntent, yaw: f32) -> Vec3 {
    let Vec2 {
        x: forward_request,
        y: strafe_request,
    } = requested_walk(intent);
    let (sin, cos) = yaw.sin_cos();
    let forward = Vec3::new(cos, 0.0, sin);
    let right = Vec3::new(-sin, 0.0, cos);
    (forward * forward_request + right * strafe_request) * WALK_SPEED
}

/// How much of a walk the simulation is willing to take an intent to have asked
/// for, as `(forward, strafe)`.
///
/// The pair is a plane of *requests* and not of world axes — its second component
/// is the strafe, where a [`Vec3`] in this module means the vertical. It is a
/// vector so that "the magnitude of what was asked for" is the one thing said
/// about it, which is exactly what the cap below is about.
///
/// **The clamp is on the receiving side**, which is what makes the authority a
/// structure rather than a promise: a client asking to walk a thousand times as
/// hard as a keyboard can express gets exactly what a well-behaved one gets.
///
/// It caps the magnitude and never normalises it. Both are the same answer at
/// full deflection and for an absurd request, and they differ for everything in
/// between — a normalising walk turns a stick pushed half over into a sprint,
/// while it still looks like it is doing the honest thing.
///
/// A request that is not a finite number is dropped whole, both axes together: a
/// request half of which cannot be read cannot be trusted on the other half
/// either. It is sanitised rather than refused because a malformed intent is a
/// client fact and not a server error — and because letting one through poisons
/// the position permanently, and every tick and every frame after it.
fn requested_walk(intent: &MovementIntent) -> Vec2 {
    let request = Vec2::new(intent.forward, intent.strafe);
    if !request.is_finite() {
        return Vec2::ZERO;
    }
    let magnitude = request.length();
    if magnitude > FULL_DEFLECTION {
        request * (FULL_DEFLECTION / magnitude)
    } else {
        request
    }
}

/// How far a tick is allowed to move the box, given how far it asked to.
///
/// Resolving one axis at a time is exact only while a tick moves the box less
/// than a whole block on each, because only then can the box newly overlap the
/// adjacent voxel layer. Under the declared constants the largest displacement is
/// one tick of the terminal fall, so the property already holds by derivation and
/// this bound never binds; it is what keeps it holding the day a constant
/// changes, rather than letting a tunnelling bug appear silently and only in the
/// places nothing looks at.
///
/// The bound is on the displacement and never on the velocity. The velocity is
/// what a caller reads back off the state, so clamping that would report a fall
/// that is not happening.
fn bounded(displacement: Vec3) -> Vec3 {
    displacement.clamp(
        Vec3::splat(-DISPLACEMENT_LIMIT),
        Vec3::splat(DISPLACEMENT_LIMIT),
    )
}

/// The vertical velocity a jump leaves the tick starting from.
///
/// Honoured from ground contact, from a medium the player can hold itself up in,
/// and from nowhere else — so asking again in mid-air does nothing to a fall
/// through air. Contact is whatever the *previous* tick ended on, which is what
/// makes a jump held down through a landing launch again on the very next tick
/// without the request having to be a new one.
///
/// **One condition widened at the one site that already answers "may this tick
/// launch", rather than a second launch path.** That is what keeps a jump asked
/// for in mid-air outside any such medium reading as exactly the rule it read as
/// before: the medium is not swimmable there, and the expression is the one it
/// was. Buoyancy is the only thing it adds — being swimmable resists nothing by
/// itself, and resisting nothing holds nobody up.
///
/// **Ground beats medium, and what a medium launches at is the medium's own.**
/// From ground contact the speed is [`JUMP_SPEED`] whether or not the box is
/// submerged, because that is the player's own jump and the ground is what it
/// pushes against. From buoyancy alone it is the volume's declared
/// [`swim_ascent`](VoxelMedium::swim_ascent), which is what lets content set how
/// fast water carries a swimmer without any engine constant moving.
///
/// **The whole medium is taken rather than the ascent alone**, and that is not a
/// convenience. A non-swimmable volume's ascent is masked to `0.0` before it
/// reaches here, so `{swimmable, 0.0}` and `{not swimmable, 0.0}` are the only
/// two values this ever sees where the ascent is zero — and they must answer
/// differently, one arresting a sink and the other leaving a mid-air jump
/// changed by nothing. An ascent handed over on its own cannot tell them apart.
///
/// Gravity takes its first bite from whatever this returns before the position
/// moves, so a declared launch speed is a value no caller ever observes — and
/// that ordering is the whole reason a jump's apex is 1.275 blocks rather than
/// the 1.35 the continuous `v²/2g` gives. It is also why a declared ascent is
/// **bound to the 60 Hz tick**: the bite is one tick of gravity whatever the
/// tick is worth, so the same declaration read at another rate rises at another
/// speed.
fn launched(state: &PlayerState, intent: &MovementIntent, medium: VoxelMedium) -> f32 {
    if !(intent.jump && (state.on_ground || medium.swimmable)) {
        return state.velocity.y;
    }
    if state.on_ground {
        JUMP_SPEED
    } else {
        medium.swim_ascent
    }
}

/// What a medium leaves a velocity at, once it has resisted it.
///
/// **A division by `1 + resistance`, never a multiplication by its reciprocal.**
/// The two agree only where `1 + resistance` is a power of two, and content
/// declares whatever it likes. A resistance of zero is therefore bit-exact
/// identity — `v / 1.0` is `v` for every finite `v` in IEEE-754 — which is what
/// lets a scenario say a walk through an unresisting volume covers *exactly*
/// what air does, and be asserted by equality rather than by a tolerance.
///
/// **The whole velocity, on every axis alike, and after gravity.** Applying it
/// to the whole velocity is what makes the medium's own terminal speed the fixed
/// point of `v <- (v - g·dt) / (1 + resistance)`; applying it only to the
/// displacement would leave the velocity a caller reads back accumulating as if
/// the medium were not there.
///
/// The loader admits only a finite resistance not less than zero, so `1 + r` is
/// at least one: the division can neither make a NaN out of a finite velocity,
/// nor amplify one, nor reverse its sign.
fn slowed(velocity: Vec3, resistance: f32) -> Vec3 {
    velocity / (1.0 + resistance)
}

/// What one tick of gravity leaves a vertical velocity at.
///
/// The clamp is on the speed rather than on the displacement, because the
/// terminal speed is a fact about the fall that a scenario reads back off the
/// state.
///
/// It stays *before* a medium's own resistance. The medium's terminal speed
/// `GRAVITY · TICK_DURATION / resistance` lies below [`TERMINAL_SPEED`] for
/// every resistance above `0.5 / 48`, so in any medium heavier than that the two
/// clamps never compete; below it this one binds first, which is the right
/// answer for a medium that is almost not there.
fn fallen(vertical: f32) -> f32 {
    (vertical - GRAVITY * TICK_DURATION).max(-TERMINAL_SPEED)
}

/// The velocity a tick reports, once the world has had its say.
///
/// A fall that ends on the ground has arrived, so it keeps none of the speed it
/// arrived with: carrying it over would start the next tick by driving the
/// player into the floor with everything the fall had built up, and a player
/// standing still would accumulate a downward speed for as long as it stood
/// there.
///
/// A vertical move the world *resolved* is spent in either direction, and that
/// is the wider of the two conditions rather than a second spelling of the same
/// one: a rise stopped by a ceiling ends the tick nowhere near the ground, and
/// keeping its speed would have it climbing on paper for the rest of an arc it
/// is spending pressed against a voxel.
fn settled(velocity: Vec3, on_ground: bool, stopped_vertically: bool) -> Vec3 {
    if stopped_vertically || (on_ground && velocity.y < 0.0) {
        velocity.with_y(0.0)
    } else {
        velocity
    }
}

#[cfg(test)]
#[path = "physics_test.rs"]
mod tests;
