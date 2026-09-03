//! What a ray does with the cell the eye is already in, which is a different
//! question from what it does with every cell it steps into afterwards.
//!
//! The walk considers the origin cell, and until now it judged that cell by the
//! same rule as every stepped one: a block declaring `targetable` stopped the
//! ray there, at distance 0 and with no entry face. That was harmless while only
//! blocks that stop a player were aimable — an eye is never inside one of those
//! — and stopped being harmless the moment content shipped a block a player can
//! stand *inside* and a ray can stop at.
//!
//! **The rule the fix reads is `occludes`, and not solidity and not
//! targetability.** A block you can see through does not stop your ray at the
//! cell your own eye is in; a block you cannot see through does, because the
//! view from inside it is that block and nothing else. Solidity would be the
//! wrong reading twice over: it is a fact about where a player may walk, and
//! reading it here would put back a game rule content cannot override at the one
//! site every action's target comes from.
//!
//! # Why these two fixture blocks
//!
//! `fixture:aimable` stops nobody, a ray stops at it, and it does not occlude —
//! the shape the shipped water has for all three of these questions, declared by
//! a fixture so that a scenario about the *rule* does not depend on the sea's
//! geometry. `fixture:sight-stopping` is its pair one field along: it stops
//! nobody and a ray stops at it too, and it *does* occlude.
//!
//! **The control uses that pair and not an ordinary solid block, and the
//! difference is the whole of what it measures.** Every block content ships has
//! `occludes` and `is_solid` agreeing, so a rule reading solidity at the origin
//! cell answers identically to the right one at every cell a player's eye can
//! occupy — it would satisfy all five of this defect's scenarios while leaving
//! the wrong question written at the site. Against a block that stops nobody and
//! blocks sight the two readings part: occlusion stops the ray at the eye's own
//! cell and solidity carries it four cells along.
//!
//! # Three things the fixtures hold that no assertion can
//!
//! **Every block along a ray carries a different name from the one in the eye's
//! cell.** An edit one cell short of the target changes the same number of cells
//! as the correct one; only the names say which cell went.
//!
//! **The blocks stand in the eye's own voxel row and in no other.** The eye is
//! the feet plus 1.62 blocks, so with the feet on a floor whose top face is at
//! y = 10 the eye sits at 11.62 and the row it is in is 11. Row 10, the row the
//! feet are in, is empty along every ray below, and the floor is three rows
//! under a ray that never descends.
//!
//! **The eye's x is a cell boundary and the ray is exactly +x.** 8.0 floors to
//! cell 8 and the first crossing is a whole block away, so no scenario here
//! depends on a rounding at the origin. A ray with any other component would
//! have to state its own tolerance.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::action::{ActionIntent, EditReport, Hit, REACH, Refusal, TickIntent, targeted};
use mc_sim::player::{BlockPos, MovementIntent, PlayerState, eye_pose};
use mc_sim::simulation::{Simulation, seat};
use mc_world::world::WorldPos;

use support::chamber::{AIMABLE, BlockChamber, SIGHT_STOPPING, at, differences, fixture_content};
use support::{DIRT, NOTHING, STONE, TestResult};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// Where the feet stand: on the floor's top face, at x = 8.0.
const LINED_UP: Vec3 = Vec3::new(8.0, 10.0, 8.5);

/// The voxel row the eye is in, `floor(10.0 + 1.62)`.
const EYE_ROW: u32 = 11;

/// Yaw facing +x, which is where every ray below goes.
const ALONG_THE_ROW: f32 = 0.0;

/// The cell the eye stands in.
const EYE_CELL: WorldPos = at(8, EYE_ROW, 8);

/// The cell four blocks along the ray from the eye.
const FOUR_ALONG: WorldPos = at(12, EYE_ROW, 8);

#[test]
fn a_swing_from_inside_a_block_you_can_see_through_takes_the_block_four_cells_along() -> TestResult
{
    let chamber = seen_through_at_the_eye_and_one_block_four_along();
    let declared = chamber.build()?;
    let broken = after(&chamber, ActionIntent::Break)?;

    assert_eq!(
        differences(&declared, broken.simulation.world()),
        vec![(FOUR_ALONG, DIRT.to_owned(), NOTHING.to_owned())],
        "the block the eye is inside declares that it can be seen through, so the ray does not \
         stop at the cell it started in and reaches the block four cells along. A walk that \
         judges the origin cell by targetability alone empties the eye's own cell instead, and \
         the two blocks carry different names so the diff says which one went"
    );
    Ok(())
}

#[test]
fn a_block_the_eye_is_inside_that_cannot_be_seen_through_is_the_target_at_no_distance() -> TestResult
{
    let world = occluding_at_the_eye_and_one_block_four_along().build()?;
    let (eye, direction) = aimed(&standing());

    assert_eq!(
        targeted(eye, direction, REACH, &world),
        Some(Hit {
            cell: signed(EYE_CELL),
            face: None,
            distance: 0.0,
        }),
        "a block that cannot be seen through fills the whole view from inside it, so it is what \
         the ray meets — at no distance, and through no face, because the ray never crossed one. \
         An implementation that skips the origin cell whenever it is occupied reports the block \
         four cells along instead, which is this scenario's whole job to refuse"
    );
    Ok(())
}

#[test]
fn a_swing_from_inside_a_block_you_can_see_through_with_nothing_beyond_it_finds_no_target()
-> TestResult {
    let chamber = seen_through_at_the_eye_and_nothing_beyond();
    let declared = chamber.build()?;
    let swung = after(&chamber, ActionIntent::Break)?;

    assert_eq!(
        (
            swung.report,
            differences(&declared, swung.simulation.world())
        ),
        (Some(EditReport::Refused(Refusal::NoTarget)), Vec::new()),
        "nothing a ray may stop at stands along this row within reach, and the block the eye is \
         inside is not one either, so the swing finds nothing at all and no cell moves. A walk \
         that still stops at the origin cell reports it as the target and empties it, which is \
         one cell moved rather than none"
    );
    Ok(())
}

/// A floor, a block that can be seen through in the eye's own cell, and an
/// ordinary block four cells along the ray.
fn seen_through_at_the_eye_and_one_block_four_along() -> BlockChamber {
    floored().cell(EYE_CELL, AIMABLE).cell(FOUR_ALONG, DIRT)
}

/// The same, with the eye's cell holding a block that stops nobody and that
/// cannot be seen through.
fn occluding_at_the_eye_and_one_block_four_along() -> BlockChamber {
    floored()
        .cell(EYE_CELL, SIGHT_STOPPING)
        .cell(FOUR_ALONG, DIRT)
}

/// A floor and a block that can be seen through in the eye's own cell, with the
/// whole row beyond it empty.
fn seen_through_at_the_eye_and_nothing_beyond() -> BlockChamber {
    floored().cell(EYE_CELL, AIMABLE)
}

/// Nothing anywhere, with one layer of floor for the player to stand on.
fn floored() -> BlockChamber {
    BlockChamber::empty(COLUMNS).run(at(0, FLOOR_LAYER, 0), at(16, FLOOR_LAYER + 1, 16), STONE)
}

/// A player standing still on the floor, facing along the row with a level view.
fn standing() -> PlayerState {
    PlayerState {
        position: LINED_UP,
        velocity: Vec3::ZERO,
        yaw: ALONG_THE_ROW,
        pitch: 0.0,
        on_ground: true,
    }
}

/// Where the ray leaves from and where it goes, derived exactly as the server
/// derives it for an action.
fn aimed(player: &PlayerState) -> (Vec3, Vec3) {
    let pose = eye_pose(player);
    let eye = Vec3::from_array(pose.eye);
    (eye, Vec3::from_array(pose.target) - eye)
}

/// One simulation and what its single action answered.
struct Swung {
    simulation: Simulation,
    report: Option<EditReport>,
}

/// One tick over a fresh build of `chamber`, asking for no movement and one
/// action.
fn after(chamber: &BlockChamber, action: ActionIntent) -> Result<Swung, Box<dyn Error>> {
    let mut simulation = seat(standing(), chamber.build()?, fixture_content()?).simulation;
    let report = simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(action),
    });
    Ok(Swung { simulation, report })
}

/// The same cell in the signed spelling a hit carries.
const fn signed(cell: WorldPos) -> BlockPos {
    BlockPos {
        x: cell.x as i32,
        y: cell.y as i32,
        z: cell.z as i32,
    }
}
