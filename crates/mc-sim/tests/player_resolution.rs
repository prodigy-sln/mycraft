//! The order a tick's displacement is resolved in: x, then z, then y, each axis
//! applied and resolved before the next begins.
//!
//! **Only one fixture in this file can tell that order from any other, and it is
//! the single solid voxel.** Placed diagonally adjacent to the box, it is
//! reached on neither axis alone and on both together — so resolving x first
//! lets the x move through and stops the z one, and resolving z first does the
//! mirror image. The two answers differ, which is the whole of what makes the
//! order verifiable. Everything symmetric about the diagonal gives the same
//! answer whichever axis moved first, so the inside corner below is a
//! **control** and not a discriminator: it says the two walls both stop the
//! player, and it would say exactly that under any order at all. Building the
//! single-voxel fixture symmetrically — two voxels, or one placed square on an
//! axis rather than on the diagonal — turns the order into something nothing in
//! the suite checks, while every test in it stays green. No assertion can catch
//! that; it is held here and by review.
//!
//! **The diagonal walk is one walk, not two.** Full deflection on forward and on
//! strafe together is capped to a magnitude of one before the walk speed scales
//! it, so each axis gets `0.075 / √2` of a block per tick rather than 0.075. The
//! expected travel along the free axis below is that figure times the number of
//! ticks, written as that product.
//!
//! **A wall stops the axis it faces and nothing else.** That is what the walk
//! along a wall asserts, and it is also the half-open rule doing its second job:
//! after x resolves flush against the wall, the box's face lies exactly on the
//! blocking face, so the z move that follows finds no overlap to be stopped by.
//! An implementation whose overlap test included the touching voxel would stop
//! the z move too, and the player would stand still in the corner of a wall it
//! is merely brushing past.
//!
//! Nothing below is read off a run: the walk speed, the tick duration and the
//! box's own half-width are declared, and every expected figure is arithmetic
//! over them.

mod support;

use std::error::Error;
use std::f32::consts::SQRT_2;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState, Traversal, advance_player};

use support::chamber::{Chamber, Slab};

type TestResult = Result<(), Box<dyn Error>>;

/// How far two figures this feature calls equal may differ, in blocks. The
/// specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast a walk carries the player, in blocks per second. Declared.
const WALK_SPEED: f32 = 4.5;

/// How far the player's box reaches from the feet centre on x and z. Declared.
const HALF_WIDTH: f32 = 0.3;

/// How far one tick of a walk at full deflection carries the player, in blocks.
const WALK_STEP: f32 = WALK_SPEED * TICK_DURATION;

/// How far one tick of that same walk carries it on *each* axis when both are
/// asked for at once.
///
/// A request is a direction and a magnitude, and the magnitude is capped at one,
/// so a diagonal is one walk shared between two axes rather than two walks.
const DIAGONAL_STEP: f32 = WALK_STEP / SQRT_2;

/// The topmost solid voxel of the flat floor every walk below happens on, and
/// where its top face — and so a standing player's feet — therefore is.
const FLOOR_SURFACE: i32 = 63;
const FLOOR_TOP: f32 = (FLOOR_SURFACE + 1) as f32;

/// The two perpendicular walls of the inside corner, their near faces, and where
/// the feet come to rest against each.
const CORNER_WALL: i32 = 13;
const CORNER_FACE: f32 = CORNER_WALL as f32;
const REST_IN_CORNER: f32 = CORNER_FACE - HALF_WIDTH;

/// Where every diagonal walk below starts, and the axis-aligned start of the
/// walk that runs along a wall.
const START_X: f32 = 10.5;
const START_Z: f32 = 10.5;
const ALONG_START_Z: f32 = 3.5;

/// How long a diagonal walk is given to reach a wall 2.2 blocks away.
///
/// A diagonal covers 0.053033 blocks per tick per axis, so the corner is reached
/// on tick 42; sixty leaves margin, and a walk a wall has stopped stays stopped.
const APPROACH_TICKS: u32 = 60;

/// The one solid voxel the diagonal step is resolved against, and the face of it
/// the stopped axis comes to rest on.
///
/// It sits at the floor's own top level, so the player's box reaches it without
/// anything having to lift the player, and it is *diagonally* adjacent to the
/// declared start: neither axis alone brings the box into it.
const LONE_VOXEL: (i32, i32, i32) = (10, 64, 10);
const LONE_VOXEL_FACE: f32 = LONE_VOXEL.2 as f32;
const REST_AGAINST_VOXEL: f32 = LONE_VOXEL_FACE - HALF_WIDTH;

/// Where the feet centre stands for that step, on both horizontal axes.
///
/// Declared by the specification. The box reaches to 9.999 on each axis, a
/// thousandth of a block clear of the voxel's near faces, so a single step of
/// either axis alone would still not enter it — which is what makes the step
/// diagonal in substance and not only in name.
const STEP_START: f32 = 9.699;

/// Where that step leaves the axis resolved first, and the axis resolved second.
///
/// The first axis moves before the second has, so the box is still clear of the
/// voxel on the other axis and the move goes through whole. The second axis then
/// moves with the first already committed, finds the voxel, and is cut back to
/// its face. Under the opposite order these two figures are exchanged.
const STEP_FIRST_AXIS: f32 = STEP_START + DIAGONAL_STEP;
const STEP_SECOND_AXIS: f32 = REST_AGAINST_VOXEL;

/// How far the walk along the wall travels on the axis the wall does not block.
const TRAVEL_ALONG_WALL: f32 = APPROACH_TICKS as f32 * DIAGONAL_STEP;

/// A floor, and the two perpendicular walls that meet over it.
fn inside_corner() -> Chamber {
    Chamber::of([
        Slab::floor(FLOOR_SURFACE),
        Slab::wall_at_x(CORNER_WALL),
        Slab::wall_at_z(CORNER_WALL),
    ])
}

/// A floor, and one wall standing in the column x = [`CORNER_WALL`].
fn floor_and_one_wall() -> Chamber {
    Chamber::of([Slab::floor(FLOOR_SURFACE), Slab::wall_at_x(CORNER_WALL)])
}

/// A floor, and one solid voxel resting on it.
fn floor_and_one_voxel() -> Chamber {
    Chamber::of([
        Slab::floor(FLOOR_SURFACE),
        Slab::voxel(LONE_VOXEL.0, LONE_VOXEL.1, LONE_VOXEL.2),
    ])
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

/// An intent that asks to walk forward and to the right at once.
fn walking_diagonally() -> MovementIntent {
    MovementIntent {
        forward: 1.0,
        strafe: 1.0,
        ..MovementIntent::default()
    }
}

/// Where `ticks` submissions of `intent` leave `state`.
fn advance(
    state: PlayerState,
    intent: &MovementIntent,
    world: &dyn Traversal,
    ticks: u32,
) -> PlayerState {
    (0..ticks).fold(state, |state, _| advance_player(state, intent, world))
}

#[test]
fn a_diagonal_walk_into_an_inside_corner_comes_to_rest_touching_both_walls() -> TestResult {
    let world = inside_corner();

    let cornered = advance(
        standing_at(START_X, START_Z),
        &walking_diagonally(),
        &world,
        APPROACH_TICKS,
    );

    assert!(
        (cornered.position.x - REST_IN_CORNER).abs() <= EPSILON
            && (cornered.position.z - REST_IN_CORNER).abs() <= EPSILON,
        "each wall stops the axis that faces it and neither stops the other, so a walk pressed \
         into both ends with the box touching both — feet at ({REST_IN_CORNER}, \
         {REST_IN_CORNER}), not at ({}, {}). An implementation that abandoned the whole \
         displacement as soon as one axis was blocked would leave the player short on the axis \
         that still had room",
        cornered.position.x,
        cornered.position.z
    );
    Ok(())
}

#[test]
fn a_diagonal_step_past_a_single_voxel_keeps_the_axis_it_resolved_first() -> TestResult {
    let world = floor_and_one_voxel();

    let stepped = advance_player(
        standing_at(STEP_START, STEP_START),
        &walking_diagonally(),
        &world,
    );

    assert!(
        (stepped.position.x - STEP_FIRST_AXIS).abs() <= EPSILON
            && (stepped.position.z - STEP_SECOND_AXIS).abs() <= EPSILON,
        "x is applied and resolved before z begins, so from ({STEP_START}, {STEP_START}) the x \
         move happens while the box is still clear of the voxel at {LONE_VOXEL:?} and goes \
         through whole to {STEP_FIRST_AXIS}, and the z move that follows finds the voxel and is \
         cut back to its face at {STEP_SECOND_AXIS}. This step landed at ({}, {}) — the exchange \
         of those two figures is what resolving z first gives, and it is the only thing in this \
         suite that can tell the two apart",
        stepped.position.x,
        stepped.position.z
    );
    Ok(())
}

#[test]
fn a_diagonal_walk_along_a_wall_is_stopped_only_on_the_axis_the_wall_blocks() -> TestResult {
    let world = floor_and_one_wall();
    let start = standing_at(START_X, ALONG_START_Z);

    let walked = advance(start, &walking_diagonally(), &world, APPROACH_TICKS);

    let travelled = walked.position.z - start.position.z;
    assert!(
        (walked.position.x - REST_IN_CORNER).abs() <= EPSILON
            && (travelled - TRAVEL_ALONG_WALL).abs() <= EPSILON,
        "the wall takes the x half of the walk and leaves the z half untouched for all \
         {APPROACH_TICKS} ticks, so the feet hold {REST_IN_CORNER} on x while covering \
         {TRAVEL_ALONG_WALL} blocks on z. This walk finished at x = {} having covered \
         {travelled} — a box held flush against the wall touches it without overlapping it, so \
         there is nothing there for the z move to be stopped by, and an implementation that \
         counted that touch as an overlap would pin the player where it first met the wall",
        walked.position.x
    );
    Ok(())
}
