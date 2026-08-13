//! What a solid voxel does to a move that would enter it: walls on the two
//! horizontal axes, a ceiling on the vertical one, and the half-open rule that
//! decides where the stopping happens.
//!
//! **Every stopping position here is the player's half-width added to, or
//! subtracted from, a declared integer**, and that is the whole reason this file
//! exists. A voxel fills `[v, v + 1)`, so the wall of voxels at `x = 13` shows
//! its near face at `x = 13.0` and a box reaching 0.3 blocks either side of the
//! feet comes to rest with its feet at `12.7`; the wall at `x = 7` shows its far
//! face at `x = 8.0` and the same box, arriving from the other side, rests at
//! `8.3`. Until this file, nothing in the suite could tell a 0.6-block-wide
//! player from a box one column wide — the earlier scenarios all place the feet
//! centre in the column that already gives the right answer, so a solidity query
//! reading only that column satisfied them. These two figures are the first that
//! cannot agree with such an implementation, which is why the half-width below
//! is declared here rather than imported from the subject.
//!
//! **The half-open rule is what removes the skin distance.** A face resolved
//! exactly onto a blocking face is not an overlap, so it is not detected again
//! on the next tick and nothing has to hold the box a hair clear of what stopped
//! it. Two tests point at that from opposite sides: a box left flush against a
//! wall with nothing asked of it must stay bit-for-bit where it is, and the same
//! box asked to walk *away* must cover a whole step.
//!
//! **A ceiling is the same rule upward.** A rise that would put the box's top
//! face inside a voxel is stopped with that face exactly on the voxel's bottom
//! face, and the vertical velocity goes with it — otherwise a jump under a low
//! ceiling would spend the rest of its arc pressed against it, still reporting
//! the speed it no longer has.
//!
//! Nothing below is read off a run. The declared constants are the walk speed,
//! the tick duration, gravity, the jump speed and the player box's own
//! dimensions; every expected figure is arithmetic over them.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState, Solidity, advance_player};

use support::chamber::{Chamber, Slab};

type TestResult = Result<(), Box<dyn Error>>;

/// How far two figures this feature calls equal may differ, in blocks or in
/// blocks per second. The specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast a walk carries the player, in blocks per second. Declared.
const WALK_SPEED: f32 = 4.5;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// How fast a jump leaves the ground, in blocks per second. Declared.
const JUMP_SPEED: f32 = 9.0;

/// How far the player's box reaches from the feet centre on x and z. Declared.
///
/// Declared *here*, and not read from the subject, on purpose: every expected
/// position in this file is derived from it, so a subject whose box is a
/// different width cannot agree with any of them.
const HALF_WIDTH: f32 = 0.3;

/// How tall the player's box is, in blocks. Declared.
const PLAYER_HEIGHT: f32 = 1.8;

/// How far one tick of a walk at full deflection carries the player, in blocks.
const WALK_STEP: f32 = WALK_SPEED * TICK_DURATION;

/// The topmost solid voxel of the flat floor every test below stands on, and
/// where its top face — and so a standing player's feet — therefore is.
const FLOOR_SURFACE: i32 = 63;
const FLOOR_TOP: f32 = (FLOOR_SURFACE + 1) as f32;

/// The wall a walk meets going forward, its near face, and where the feet stop.
const WALL_AHEAD: i32 = 13;
const FACE_AHEAD: f32 = WALL_AHEAD as f32;
const REST_AHEAD: f32 = FACE_AHEAD - HALF_WIDTH;

/// The wall a walk meets going backward, its far face, and where the feet stop.
///
/// The *far* face, because a box arriving from the east meets the side of the
/// voxel at `x + 1`. That is the sign the half-width is added on rather than
/// taken off, and getting it the other way round puts the player inside a wall.
const WALL_BEHIND: i32 = 7;
const FACE_BEHIND: f32 = (WALL_BEHIND + 1) as f32;
const REST_BEHIND: f32 = FACE_BEHIND + HALF_WIDTH;

/// The wall a strafe to the right meets, its near face, and where the feet stop.
const WALL_RIGHT: i32 = 13;
const FACE_RIGHT: f32 = WALL_RIGHT as f32;
const REST_RIGHT: f32 = FACE_RIGHT - HALF_WIDTH;

/// The wall a strafe to the left meets, its far face, and where the feet stop.
///
/// A different coordinate from every other wall in this file so that a resolver
/// reading one axis where it meant the other lands nowhere near an expectation.
const WALL_LEFT: i32 = 20;
const FACE_LEFT: f32 = (WALL_LEFT + 1) as f32;
const REST_LEFT: f32 = FACE_LEFT + HALF_WIDTH;

/// How far clear of the left wall's face the single declared step below starts.
///
/// Less than a whole step, so the one tick it is given asks to cross the face by
/// `WALK_STEP - STEP_CLEARANCE` and has to be stopped; more than nothing, so the
/// box genuinely starts clear of the wall rather than already touching it.
const STEP_CLEARANCE: f32 = 0.04;

/// The height of the voxel whose bottom face a jump below is stopped by, that
/// face, and where the feet are when the box's top is flush against it.
const CEILING_VOXEL: i32 = 66;
const CEILING_BOTTOM: f32 = CEILING_VOXEL as f32;
const HEAD_ROOM_FEET: f32 = CEILING_BOTTOM - PLAYER_HEIGHT;

/// How far the feet can rise before the head is against that ceiling.
///
/// The ceiling's bottom face is exactly two blocks above the floor's top face
/// and the box is 1.8 blocks tall, so a jump has 0.2 blocks to spend and no
/// more, however hard it leaves the ground.
const HEAD_ROOM: f32 = HEAD_ROOM_FEET - FLOOR_TOP;

/// How many ticks after leaving the floor the jump below reaches the ceiling.
///
/// Derived from the declared integrator rather than observed: [`risen_after`]
/// gives 0.141666 blocks after one tick, which is still inside the 0.2 blocks of
/// head room, and 0.275 after two, which is not — so the second tick is the one
/// whose rise has to be cut short.
const CONTACT_TICKS: u32 = 2;

/// How long the jump under the ceiling is watched for.
///
/// Longer than the 35 ticks an unobstructed jump takes to come back down, so the
/// highest the feet ever get is inside the window whether the ceiling stopped
/// the rise or not.
const WATCH_TICKS: u32 = 40;

/// How long every walk toward a wall below is given to arrive and settle.
///
/// The furthest wall is 2.2 blocks away and a tick covers 0.075, so 30 ticks
/// reach it; 60 is margin, and a walk the wall has stopped stays stopped, so a
/// longer watch cannot change the answer.
const APPROACH_TICKS: u32 = 60;

/// How long an intent pushing into a wall it is already touching is held.
///
/// Sixty ticks rather than one, so that a resolver creeping into the wall by
/// less than the comparison epsilon each tick still has to answer for the sum.
const PUSH_TICKS: u32 = 60;

/// Where a walk along x starts, and the z it happens at.
const START_X: f32 = 10.5;
const START_Z: f32 = 3.5;

/// Where a walk along z starts, and the x it happens at.
///
/// The two are exchanged relative to the pair above, so a resolver that read a
/// box's z where it meant its x is asserted against a coordinate that disagrees.
const SIDE_START_X: f32 = 3.5;
const SIDE_START_Z: f32 = 10.5;

/// How far above the floor the feet stand `ticks` ticks after a jump left it,
/// with no ceiling in the way.
///
/// The closed form of the declared integrator's sum rather than a second copy of
/// its loop: the tick that jumps sets the velocity to [`JUMP_SPEED`] and each
/// tick takes `GRAVITY × TICK_DURATION` from it before the position moves, so
/// after `n` ticks the feet have risen `dt × (v₀n − g·dt·n(n+1)/2)`.
const fn risen_after(ticks: f32) -> f32 {
    TICK_DURATION * (JUMP_SPEED * ticks - GRAVITY * TICK_DURATION * ticks * (ticks + 1.0) / 2.0)
}

/// A floor, and one wall standing in the column `x`.
fn floor_and_wall_at_x(x: i32) -> Chamber {
    Chamber::of([Slab::floor(FLOOR_SURFACE), Slab::wall_at_x(x)])
}

/// A floor, and one wall standing in the column `z`.
fn floor_and_wall_at_z(z: i32) -> Chamber {
    Chamber::of([Slab::floor(FLOOR_SURFACE), Slab::wall_at_z(z)])
}

/// A floor with a solid slab two blocks above it.
fn floor_and_ceiling() -> Chamber {
    Chamber::of([Slab::floor(FLOOR_SURFACE), Slab::ceiling_at(CEILING_VOXEL)])
}

/// A player standing still on the floor at `(x, z)`, facing +x.
fn standing_at(x: f32, z: f32) -> PlayerState {
    PlayerState {
        position: Vec3::new(x, FLOOR_TOP, z),
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    }
}

/// An intent that asks only to walk.
fn walking(forward: f32, strafe: f32) -> MovementIntent {
    MovementIntent {
        forward,
        strafe,
        ..MovementIntent::default()
    }
}

/// An intent that asks for a jump and nothing else.
fn jumping() -> MovementIntent {
    MovementIntent {
        jump: true,
        ..MovementIntent::default()
    }
}

/// Where `ticks` submissions of `intent` leave `state`.
fn advance(
    state: PlayerState,
    intent: &MovementIntent,
    world: &dyn Solidity,
    ticks: u32,
) -> PlayerState {
    (0..ticks).fold(state, |state, _| advance_player(state, intent, world))
}

/// Where a player that jumped from the floor at `(x, z)` is `ticks` ticks later,
/// having asked for nothing else since.
fn after_jumping(x: f32, z: f32, world: &dyn Solidity, ticks: u32) -> PlayerState {
    let launched = advance_player(standing_at(x, z), &jumping(), world);
    advance(
        launched,
        &MovementIntent::default(),
        world,
        ticks.saturating_sub(1),
    )
}

#[test]
fn walking_into_a_wall_ahead_stops_the_feet_a_half_width_before_its_near_face() -> TestResult {
    let world = floor_and_wall_at_x(WALL_AHEAD);

    let stopped = advance(
        standing_at(START_X, START_Z),
        &walking(1.0, 0.0),
        &world,
        APPROACH_TICKS,
    );

    assert!(
        (stopped.position.x - REST_AHEAD).abs() <= EPSILON,
        "a voxel fills [v, v + 1), so the wall standing in column {WALL_AHEAD} shows its near \
         face at {FACE_AHEAD}, and the box reaches {HALF_WIDTH} blocks ahead of the feet — so the \
         feet come to rest at {REST_AHEAD}. This walk ended at {}, which is where a box one \
         column wide, or one measured from a face it does not have, would have stopped",
        stopped.position.x
    );
    Ok(())
}

#[test]
fn a_move_that_would_overlap_puts_the_leading_face_exactly_on_the_blocking_face() -> TestResult {
    let world = floor_and_wall_at_z(WALL_LEFT);
    let start = standing_at(SIDE_START_X, REST_LEFT + STEP_CLEARANCE);

    let stopped = advance_player(start, &walking(0.0, -1.0), &world);

    let leading_face = stopped.position.z - HALF_WIDTH;
    assert!(
        (leading_face - FACE_LEFT).abs() <= EPSILON,
        "the step asked to move {WALK_STEP} blocks from {STEP_CLEARANCE} blocks clear of the \
         wall, so it asked to put the box's leading face {} blocks inside the voxel at \
         z = {WALL_LEFT}. A move that would overlap is cut back to the face itself — exactly \
         {FACE_LEFT}, with nothing subtracted for a skin — and this one landed at {leading_face}",
        WALK_STEP - STEP_CLEARANCE
    );
    Ok(())
}

#[test]
fn a_box_resting_flush_against_a_wall_is_never_pushed_off_it() -> TestResult {
    let world = floor_and_wall_at_x(WALL_AHEAD);
    let flush = standing_at(REST_AHEAD, START_Z);
    let mut state = flush;
    let mut disturbed = Vec::new();

    for tick in 1..=PUSH_TICKS {
        state = advance_player(state, &MovementIntent::default(), &world);
        if state.position.x.to_bits() != flush.position.x.to_bits() {
            disturbed.push(format!("tick {tick} left it at {}", state.position.x));
        }
    }

    assert!(
        disturbed.is_empty(),
        "the box's leading face is exactly on the wall's face at {FACE_AHEAD}, and a voxel fills \
         [v, v + 1), so touching is not overlapping: there is nothing there to resolve, this tick \
         or any tick after it, and that is what makes a skin distance unnecessary rather than \
         missing. A test that reported an overlap here would push the player somewhere every tick \
         — {} of {PUSH_TICKS} did, the first {:?}",
        disturbed.len(),
        disturbed.first()
    );
    Ok(())
}

#[test]
fn an_intent_pushing_into_a_wall_the_box_already_touches_moves_it_nowhere() -> TestResult {
    let world = floor_and_wall_at_x(WALL_AHEAD);

    let pushed = advance(
        standing_at(REST_AHEAD, START_Z),
        &walking(1.0, 0.0),
        &world,
        PUSH_TICKS,
    );

    assert!(
        (pushed.position.x - REST_AHEAD).abs() <= EPSILON,
        "the box already touches the wall in the direction it is asked to walk, so the axis the \
         wall blocks does not move — not this tick and not after {PUSH_TICKS} of them, because a \
         resolver that let the box creep in by less than a comparison epsilon each tick would \
         have walked it {} blocks into the wall by now. The feet finished at {}, not {REST_AHEAD}",
        PUSH_TICKS as f32 * WALK_STEP,
        pushed.position.x
    );
    Ok(())
}

#[test]
fn a_walk_away_from_a_wall_it_is_touching_covers_a_full_step() -> TestResult {
    let world = floor_and_wall_at_x(WALL_AHEAD);

    let retreated = advance_player(
        standing_at(REST_AHEAD, START_Z),
        &walking(-1.0, 0.0),
        &world,
    );

    let expected = REST_AHEAD - WALK_STEP;
    assert!(
        (retreated.position.x - expected).abs() <= EPSILON,
        "touching a wall is not being held by it: the box at {REST_AHEAD} overlaps nothing, so a \
         step away from the wall is an ordinary step and covers the whole {WALK_STEP} blocks to \
         {expected}. This one reached {}, which is where an implementation that treated an exact \
         touch as an overlap — and went on resolving the axis every tick — leaves it",
        retreated.position.x
    );
    Ok(())
}

#[test]
fn walking_into_a_wall_behind_stops_the_feet_a_half_width_beyond_its_far_face() -> TestResult {
    let world = floor_and_wall_at_x(WALL_BEHIND);

    let stopped = advance(
        standing_at(START_X, START_Z),
        &walking(-1.0, 0.0),
        &world,
        APPROACH_TICKS,
    );

    assert!(
        (stopped.position.x - REST_BEHIND).abs() <= EPSILON,
        "the wall standing in column {WALL_BEHIND} fills [{WALL_BEHIND}, {FACE_BEHIND}), so a box \
         arriving from the east meets its far face at {FACE_BEHIND} and stops with the feet \
         {HALF_WIDTH} beyond it, at {REST_BEHIND}. This walk ended at {}; the half-width is added \
         on this side and taken off the other, and getting that sign the wrong way round puts the \
         player inside the wall",
        stopped.position.x
    );
    Ok(())
}

#[test]
fn walking_sideways_into_a_wall_stops_the_feet_a_half_width_before_its_near_face() -> TestResult {
    let world = floor_and_wall_at_z(WALL_RIGHT);

    let stopped = advance(
        standing_at(SIDE_START_X, SIDE_START_Z),
        &walking(0.0, 1.0),
        &world,
        APPROACH_TICKS,
    );

    assert!(
        (stopped.position.z - REST_RIGHT).abs() <= EPSILON,
        "the wall standing in column z = {WALL_RIGHT} stops a strafe exactly as the one on x \
         stops a walk: its near face is at {FACE_RIGHT} and the feet come to rest at \
         {REST_RIGHT}, not at {}. The two horizontal axes are resolved by the same rule, and an \
         implementation that resolved only the one it was written for passes every x scenario in \
         this file",
        stopped.position.z
    );
    Ok(())
}

#[test]
fn a_jump_into_a_ceiling_stops_with_the_head_on_it_and_the_rise_spent() -> TestResult {
    let world = floor_and_ceiling();

    let stopped = after_jumping(START_X, START_Z, &world, CONTACT_TICKS);

    assert!(
        (stopped.position.y - HEAD_ROOM_FEET).abs() <= EPSILON
            && stopped.velocity.y.abs() <= EPSILON,
        "the rise asked for {} blocks by tick {CONTACT_TICKS} and the box is {PLAYER_HEIGHT} tall, \
         so its top face would have gone {} blocks inside the voxel at y = {CEILING_VOXEL}. It is \
         put exactly on that voxel's bottom face at {CEILING_BOTTOM} instead — feet at \
         {HEAD_ROOM_FEET} — and the rise that took it there is spent, because a jump that kept \
         its speed under a ceiling would report climbing while pressed against one. This tick \
         left the feet at {} going {}",
        risen_after(CONTACT_TICKS as f32),
        FLOOR_TOP + risen_after(CONTACT_TICKS as f32) + PLAYER_HEIGHT - CEILING_BOTTOM,
        stopped.position.y,
        stopped.velocity.y
    );
    Ok(())
}

#[test]
fn a_jump_under_a_ceiling_two_blocks_up_lifts_the_feet_no_higher_than_the_head_room() -> TestResult
{
    let world = floor_and_ceiling();
    let mut state = advance_player(standing_at(START_X, START_Z), &jumping(), &world);
    let mut highest = state.position.y;

    for _ in 1..WATCH_TICKS {
        state = advance_player(state, &MovementIntent::default(), &world);
        highest = highest.max(state.position.y);
    }

    let risen = highest - FLOOR_TOP;
    assert!(
        risen <= HEAD_ROOM + EPSILON,
        "the ceiling's bottom face is two blocks above the floor's top face and the box is \
         {PLAYER_HEIGHT} tall, so there are {HEAD_ROOM} blocks for the feet to rise into and the \
         jump's own {} blocks of arc are irrelevant — a jump is not given room it does not have. \
         Over {WATCH_TICKS} ticks the feet reached {risen} blocks above the floor",
        risen_after(17.0)
    );
    Ok(())
}

#[test]
fn a_jump_with_the_head_already_on_a_ceiling_leaves_no_rise_behind() -> TestResult {
    let world = floor_and_ceiling();
    let pinned = PlayerState {
        position: Vec3::new(START_X, HEAD_ROOM_FEET, START_Z),
        ..standing_at(START_X, START_Z)
    };

    let jumped = advance_player(pinned, &jumping(), &world);

    assert!(
        jumped.velocity.y.abs() <= EPSILON,
        "the box's top face is already exactly on the ceiling's bottom face at {CEILING_BOTTOM}, \
         so the jump has nowhere at all to go: the tick ends with no vertical speed rather than \
         with the {} a jump and its first bite of gravity would otherwise leave. Ground contact \
         is declared here, because a jump is honoured from the ground and from nowhere else and \
         no floor sits {HEAD_ROOM} blocks under a ceiling. This tick reported {}",
        JUMP_SPEED - GRAVITY * TICK_DURATION,
        jumped.velocity.y
    );
    Ok(())
}
