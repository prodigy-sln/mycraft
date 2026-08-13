//! What the server decides you are looking at, and how far you are allowed to
//! reach.
//!
//! Every scenario here is driven through `Simulation::advance`, the same call a
//! click reaches, and judged by reading the world back out of the simulation and
//! diffing it against the fixture *as declared*. Nothing is judged against the
//! production traversal — the expected cell of every ray below is arithmetic over
//! the declaration, worked out in the comment beside it — and no number in this
//! file was read off a run.
//!
//! # Two things the fixtures hold that no assertion can
//!
//! **The target of the reach scenarios stands in the eye's voxel row and in no
//! other.** The eye is the feet plus 1.62 blocks, so with the feet on a floor
//! whose top face is at y = 10 the eye sits at 11.62 and the row it is in is 11.
//! The block at 4.95 blocks is declared at y = 11 alone; the row the feet are in,
//! y = 10, is empty along that ray. A ray cast from the feet instead of the eye
//! therefore meets *nothing at all*, and a reach measured from the feet against a
//! ray still cast from the eye computes `sqrt(4.95² + 1.62²) = 5.21` against a
//! limit of 5.0 and refuses. Both readings of "measured from the feet" turn
//! `a_break_against_a_block_first_met_just_inside_five_blocks_leaves_what_it_breaks_into`
//! red, and neither turns the refusing side red — on a horizontal ray a
//! feet-measured distance is *longer*, so a feet-measuring implementation refuses
//! more rather than less. The accept side is the only falsifier there is.
//!
//! **The two blocks standing in line carry different names.** An edit that took
//! the cell one step beyond the hit, or one step before it, changes the same
//! number of cells as the correct one; only the names say which cell went.
//!
//! # Every refusal here asserts a difference, not an absence
//!
//! "Nothing changed" is satisfied by an implementation that changes nothing, and
//! by a fixture where nothing was ever targeted. So each refusing run is paired
//! in the same test with the minimally different accepting run — one field of the
//! player's state moved, the fixture untouched — and the assertion carries both
//! answers at once. The two expected values differ, so a simulation that edited
//! nothing fails on the accepting half.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::action::{ActionIntent, TickIntent};
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::simulation::Simulation;
use mc_world::world::WorldPos;

use support::chamber::{BlockChamber, at, differences};
use support::{AIR, DIRT, STONE, TestResult};

/// How many chunk columns the fixture world spans on each axis.
///
/// One: 16 × 16 blocks over the full column height, which is the smallest world
/// that contains every ray below and every cell they meet.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// Where the feet stand: on the floor's top face.
const FEET_Y: f32 = 10.0;

/// The voxel row the eye is in, `floor(10.0 + 1.62)`.
///
/// Derived from [`FEET_Y`] and the declared eye height, and written out because
/// it is the number the reach fixture's whole shape rests on: the target stands
/// in this row and not in row 10, which is the row the feet are in.
const EYE_ROW: u32 = 11;

/// Yaw facing +x, and yaw facing −x.
///
/// The declared basis is yaw 0 towards +x (`crates/mc-sim/src/player/mod.rs`), so
/// half a turn faces the other way and is what "no solid block along the look
/// direction" is stated with — the fixture is untouched between the two runs.
const TOWARD_THE_BLOCKS: f32 = 0.0;
const AWAY_FROM_THE_BLOCKS: f32 = std::f32::consts::PI;

/// The nearer and the farther of two blocks standing in line along the ray.
///
/// The eye is at x = 8.0 for these, so the near block's face at x = 10.0 is met
/// at exactly 2 blocks and the far one's at x = 12.0 at exactly 4.
const NEAR: WorldPos = at(10, EYE_ROW, 8);
const FAR: WorldPos = at(12, EYE_ROW, 8);

/// Where the feet stand for the two-in-line scenarios.
const LINED_UP: Vec3 = Vec3::new(8.0, FEET_Y, 8.5);

/// The one block the reach scenarios aim at.
///
/// Its near face is at x = 13.0. The eye stands over the feet, so the two feet
/// positions below meet it at 5.05 and at 4.95 blocks and differ in nothing else.
const AT_THE_LIMIT: WorldPos = at(13, EYE_ROW, 8);

/// 13.0 − 7.95 = 5.05, and 13.0 − 8.05 = 4.95.
const TOO_FAR: Vec3 = Vec3::new(7.95, FEET_Y, 8.5);
const JUST_INSIDE: Vec3 = Vec3::new(8.05, FEET_Y, 8.5);

#[test]
fn a_break_takes_the_nearer_of_two_blocks_standing_in_line_and_leaves_the_farther_one() -> TestResult
{
    let chamber = two_blocks_in_line();
    let declared = chamber.build()?;
    let broken = after_a_break(&chamber, standing(LINED_UP, TOWARD_THE_BLOCKS))?;

    assert_eq!(
        differences(&declared, broken.world()),
        broke(NEAR, STONE, AIR),
        "the ray meets the block two blocks from the eye before the one at four, so exactly one \
         cell of the fixture is allowed to have moved and it is that one. The far cell has to \
         still hold the block it was declared with, and the two carry different names on \
         purpose: taking the cell one step beyond the hit, or one step before it, changes the \
         same number of cells as taking the right one and only the names say which"
    );
    Ok(())
}

#[test]
fn a_break_with_no_solid_block_along_the_look_direction_leaves_every_declared_cell_alone()
-> TestResult {
    let chamber = two_blocks_in_line();
    let declared = chamber.build()?;
    let refusing = after_a_break(&chamber, standing(LINED_UP, AWAY_FROM_THE_BLOCKS))?;
    let accepting = after_a_break(&chamber, standing(LINED_UP, TOWARD_THE_BLOCKS))?;

    assert_eq!(
        (
            differences(&declared, refusing.world()),
            differences(&declared, accepting.world())
        ),
        (nothing(), broke(NEAR, STONE, AIR)),
        "facing away from the two blocks there is nothing solid anywhere along the ray, so every \
         cell of the fixture has to still hold the block it was declared with. The second half \
         is what stops that being satisfied by a simulation that edits nothing whatever: the \
         same request, from the same place, turned to face the blocks, has to change the cell \
         the ray reaches"
    );
    Ok(())
}

#[test]
fn a_break_against_a_block_first_met_beyond_five_blocks_from_the_eye_changes_nothing() -> TestResult
{
    let chamber = one_block_at_the_limit();
    let declared = chamber.build()?;
    let refusing = after_a_break(&chamber, standing(TOO_FAR, TOWARD_THE_BLOCKS))?;
    let accepting = after_a_break(&chamber, standing(JUST_INSIDE, TOWARD_THE_BLOCKS))?;

    assert_eq!(
        (
            differences(&declared, refusing.world()),
            differences(&declared, accepting.world())
        ),
        (nothing(), broke(AT_THE_LIMIT, STONE, AIR)),
        "the block's near face is met at 5.05 blocks from the eye, which is past the reach, so \
         nothing changes. One tenth of a block nearer it is met at 4.95 and the same request \
         goes through — the pair is what makes this the *boundary* rather than an implementation \
         that refuses everything, and the two runs differ in the feet's x and in nothing else"
    );
    Ok(())
}

#[test]
fn a_break_against_a_block_first_met_just_inside_five_blocks_leaves_what_it_breaks_into()
-> TestResult {
    let chamber = one_block_at_the_limit();
    let declared = chamber.build()?;
    let broken = after_a_break(&chamber, standing(JUST_INSIDE, TOWARD_THE_BLOCKS))?;

    assert_eq!(
        differences(&declared, broken.world()),
        broke(AT_THE_LIMIT, STONE, AIR),
        "4.95 blocks from the eye is inside the reach, so the cell ends holding the block its \
         own definition names it breaks into. This is the only assertion in the suite that a \
         reach measured from the feet can fail: on this horizontal ray a feet-measured distance \
         is sqrt(4.95² + 1.62²) = 5.21, longer than the eye's, so a feet-measuring \
         implementation wrongly refuses here while still refusing at 5.05 — and a ray *cast* \
         from the feet misses this block altogether, because it stands in the eye's voxel row \
         and the row beneath it is empty"
    );
    Ok(())
}

/// A floor, and two solid blocks standing in the eye's own row, two and four
/// blocks along the ray from an eye at x = 8.0.
fn two_blocks_in_line() -> BlockChamber {
    floored().cell(NEAR, STONE).cell(FAR, DIRT)
}

/// A floor, and one solid block whose near face stands 5.05 blocks from one of
/// the two declared eye positions and 4.95 from the other.
fn one_block_at_the_limit() -> BlockChamber {
    floored().cell(AT_THE_LIMIT, STONE)
}

/// Air everywhere, with one layer of floor for the player to stand on.
///
/// The floor is a single layer rather than everything beneath it: the player
/// never leaves its top face, and a ray held level at the eye's height never
/// descends to it.
fn floored() -> BlockChamber {
    BlockChamber::filled_with(COLUMNS, AIR).run(
        at(0, FLOOR_LAYER, 0),
        at(16, FLOOR_LAYER + 1, 16),
        STONE,
    )
}

/// A player standing still on the floor at `feet`, facing along `yaw` with a
/// level view.
fn standing(feet: Vec3, yaw: f32) -> PlayerState {
    PlayerState {
        position: feet,
        velocity: Vec3::ZERO,
        yaw,
        pitch: 0.0,
        on_ground: true,
    }
}

/// One tick over a fresh build of `chamber`, asking for no movement and one
/// break.
///
/// A grounded player asked for no movement ends the tick exactly where it began
/// — the fall of one tick is resolved back onto the floor's own face — so the ray
/// is cast from the declared eye and not from somewhere a tick of gravity left.
fn after_a_break(
    chamber: &BlockChamber,
    player: PlayerState,
) -> Result<Simulation, Box<dyn Error>> {
    let mut simulation = Simulation::new(player, chamber.build()?);
    simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(ActionIntent::Break),
    });
    Ok(simulation)
}

/// The one change a break at `cell` is expected to make.
fn broke(cell: WorldPos, from: &str, into: &str) -> Vec<(WorldPos, String, String)> {
    vec![(cell, from.to_owned(), into.to_owned())]
}

/// No cell of the fixture moved.
fn nothing() -> Vec<(WorldPos, String, String)> {
    Vec::new()
}
