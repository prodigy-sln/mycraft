//! A tick that turns and breaks at once breaks where it turned to.
//!
//! The ray is cast from the eye position and the orientation the tick *ends*
//! with, after the look has been accumulated and limited. Both scenarios here
//! exist to make that a decision rather than a consequence of the order two
//! statements happen to be written in: each declares two candidate blocks — or
//! one block and an empty direction — that the before-view and the after-view
//! reach differently, so the assertion says *which* was taken and not merely that
//! something was.
//!
//! # The pitch scenario's two rays, worked out from the declaration
//!
//! The eye is at (8.5, 11.62, 8.5). A view already pitched 80° up plus a delta of
//! +20° is 100°, which the ±89° limit brings back to 89° before anything reads
//! it. The two directions are therefore
//!
//! - 89°: `(cos 89°, sin 89°, 0)` = (0.01745, 0.99985, 0) — it climbs almost
//!   straight up and drifts a hair towards +x, so it never leaves the column it
//!   started in. It enters (8, 15, 8) at 3.38 blocks, inside the reach.
//! - 100°: `(cos 100°, sin 100°, 0)` = (−0.17365, 0.98481, 0) — it climbs and
//!   drifts towards −x, crossing x = 8.0 at 2.88 blocks while still in row 14, so
//!   it is already in the next column when it reaches row 15. It enters
//!   (7, 15, 8) at 3.43 blocks.
//!
//! Neither ray enters the other's cell, and everything they share on the way up
//! is air. That divergence is what makes the scenario a falsifier: a ray cast
//! from the raw deltas takes the block the limited view does not reach, and the
//! assertion names both cells.
//!
//! # The yaw scenario
//!
//! The eye is at (8.5, 11.62, 8.0) facing +x, where the fixture declares nothing
//! solid at all. A quarter turn puts it on +z, where a block stands with its near
//! face at z = 11.0 — three blocks away, inside the reach. A ray cast before the
//! turn meets nothing and the cell keeps what it was declared with.

mod support;

use glam::Vec3;
use mc_sim::action::{ActionIntent, TickIntent};
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::simulation::Simulation;
use mc_world::world::WorldPos;

use support::chamber::{BlockChamber, at, differences, fixture_content};
use support::{DIRT, NOTHING, STONE, TestResult};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// Where the feet stand: on the floor's top face.
const FEET_Y: f32 = 10.0;

/// The voxel row the eye is in, `floor(10.0 + 1.62)`.
const EYE_ROW: u32 = 11;

/// The view before the tick's delta, and the delta itself, in degrees.
///
/// The spec's own numbers: 80 + 20 = 100, limited to 89.
const ALREADY_PITCHED_UP: f32 = 80.0;
const PITCH_DELTA: f32 = 20.0;

/// The cell the limited 89° view reaches, and the one the unlimited 100° view
/// would have reached instead.
const REACHED_BY_THE_LIMITED_VIEW: WorldPos = at(8, 15, 8);
const REACHED_BY_THE_UNLIMITED_VIEW: WorldPos = at(7, 15, 8);

/// Where the feet stand while looking up.
const UNDER_BOTH_CANDIDATES: Vec3 = Vec3::new(8.5, FEET_Y, 8.5);

/// The block a quarter turn brings into view, three blocks along +z.
const AROUND_THE_CORNER: WorldPos = at(8, EYE_ROW, 11);

/// Where the feet stand before the turn, facing +x, where nothing stands.
const FACING_EMPTY_SPACE: Vec3 = Vec3::new(8.5, FEET_Y, 8.0);

/// A quarter turn, which takes the declared basis from +x to +z.
const A_QUARTER_TURN: f32 = std::f32::consts::FRAC_PI_2;

#[test]
fn a_break_in_the_same_tick_as_a_pitch_delta_reads_the_view_the_limit_left() -> TestResult {
    let chamber = two_candidates_overhead();
    let mut simulation = Simulation::new(pitched_up(), chamber.build()?, fixture_content()?);

    simulation.advance(TickIntent {
        movement: MovementIntent {
            pitch_delta: PITCH_DELTA.to_radians(),
            ..MovementIntent::default()
        },
        action: Some(ActionIntent::Break),
    });

    assert_eq!(
        differences(&chamber.build()?, simulation.world()),
        vec![(
            REACHED_BY_THE_LIMITED_VIEW,
            STONE.to_owned(),
            NOTHING.to_owned()
        )],
        "80° plus 20° is 100°, and the view is limited to 89° before the ray reads it. The two \
         views reach different declared blocks — the limited one stays in the column it started \
         in, the unlimited one has left it by the time it climbs this far — so this assertion \
         says which of the two was taken as well as that one of them was. A ray cast from the \
         raw deltas takes the other cell, which carries the other name"
    );
    Ok(())
}

#[test]
fn a_break_in_the_same_tick_as_a_yaw_delta_reaches_what_the_turn_brought_into_view() -> TestResult {
    let chamber = one_block_around_the_corner();
    let mut simulation =
        Simulation::new(facing_empty_space(), chamber.build()?, fixture_content()?);

    simulation.advance(TickIntent {
        movement: MovementIntent {
            yaw_delta: A_QUARTER_TURN,
            ..MovementIntent::default()
        },
        action: Some(ActionIntent::Break),
    });

    assert_eq!(
        differences(&chamber.build()?, simulation.world()),
        vec![(AROUND_THE_CORNER, STONE.to_owned(), NOTHING.to_owned())],
        "the tick turns the view a quarter turn, from a direction the fixture declares nothing \
         along onto a block three blocks away, and breaks in the same tick — so the ray has to \
         be cast from the orientation the tick ends with. Cast from the one it started with it \
         meets nothing at all and this cell still holds what it was declared with"
    );
    Ok(())
}

/// A floor, and the two blocks the limited and the unlimited views reach.
///
/// They carry different names because the whole content of the scenario is which
/// of the two went.
fn two_candidates_overhead() -> BlockChamber {
    floored()
        .cell(REACHED_BY_THE_LIMITED_VIEW, STONE)
        .cell(REACHED_BY_THE_UNLIMITED_VIEW, DIRT)
}

/// A floor, and one block three blocks along +z from the declared eye.
fn one_block_around_the_corner() -> BlockChamber {
    floored().cell(AROUND_THE_CORNER, STONE)
}

/// Nothing anywhere, with one layer of floor for the player to stand on.
fn floored() -> BlockChamber {
    BlockChamber::empty(COLUMNS).run(at(0, FLOOR_LAYER, 0), at(16, FLOOR_LAYER + 1, 16), STONE)
}

/// A player standing on the floor with its view already 80° above level.
fn pitched_up() -> PlayerState {
    PlayerState {
        position: UNDER_BOTH_CANDIDATES,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: ALREADY_PITCHED_UP.to_radians(),
        on_ground: true,
    }
}

/// A player standing on the floor looking level along +x, where nothing stands.
fn facing_empty_space() -> PlayerState {
    PlayerState {
        position: FACING_EMPTY_SPACE,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    }
}
