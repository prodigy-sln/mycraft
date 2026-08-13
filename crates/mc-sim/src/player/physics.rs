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
//! Nothing here reads a clock. A tick is a declared quantum of simulated time,
//! which is what lets the same intents replay to the same state on a machine of
//! any speed (`crates/mc-sim/CLAUDE.md`).

use glam::{Vec2, Vec3};

use crate::player::collide;
use crate::player::{Look, MovementIntent, PlayerState, Solidity};

/// How long one tick simulates, in seconds.
///
/// Declared, never measured. The frame loop still drives ticks one to one, so a
/// faster machine runs the world faster — a stated cost rather than a hidden
/// one, and the day a pacing accumulator arrives it feeds this same fixed step.
const TICK_DURATION: f32 = 1.0 / 60.0;

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
#[must_use]
pub fn advance_player(
    state: PlayerState,
    intent: &MovementIntent,
    world: &dyn Solidity,
) -> PlayerState {
    let look = Look::of(&state).accumulate(intent);
    let walk = walk_velocity(intent, look.yaw);
    let velocity = walk.with_y(fallen(launched(&state, intent)));
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
/// Honoured from ground contact and from nowhere else, so asking again in
/// mid-air does nothing to a fall. Contact is whatever the *previous* tick ended
/// on, which is what makes a jump held down through a landing launch again on
/// the very next tick without the request having to be a new one.
///
/// Gravity takes its first bite from this before the position moves, so the
/// declared jump speed is a value no caller ever observes — and that ordering is
/// the whole reason the apex is 1.275 blocks rather than the 1.35 the continuous
/// `v²/2g` gives.
fn launched(state: &PlayerState, intent: &MovementIntent) -> f32 {
    if intent.jump && state.on_ground {
        JUMP_SPEED
    } else {
        state.velocity.y
    }
}

/// What one tick of gravity leaves a vertical velocity at.
///
/// The clamp is on the speed rather than on the displacement, because the
/// terminal speed is a fact about the fall that a scenario reads back off the
/// state.
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
