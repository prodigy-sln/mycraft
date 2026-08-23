//! How far a player may reach to aim at water, measured from the eye.
//!
//! **The world here is built over the content root this repository ships**, for
//! the reason `shipped_water_is_not_broken_and_is_built_through.rs` gives: the
//! question is what a running game does with the declaration
//! `content/base/blocks/water.luau` carries, and a registry assembled in Rust
//! would be the engine answering on content's behalf.
//!
//! # The two runs differ in the feet's x and in nothing else
//!
//! The one cell of water stands at (13, 11, 8), so its near face is at x = 13.0.
//! The eye is the feet plus 1.62 blocks, so the feet at x = 7.95 put the eye
//! 5.05 blocks from that face and the feet at x = 8.05 put it 4.95 blocks from
//! it. Reach is 5.0. The fixture is untouched between the two runs and the aim
//! is identical; a simulation that refused everything fails on the second, and
//! one that refused nothing fails on the first.
//!
//! # Two things the fixture holds that no assertion can
//!
//! **The water stands in the eye's own voxel row and in no other.** The floor's
//! top face is at y = 10, so the eye sits at 11.62 and the row it is in is 11.
//! Row 10, the row the feet are in, is empty along this ray. A ray cast from the
//! feet therefore meets *nothing at all*, and a reach measured from the feet
//! against a ray still cast from the eye computes sqrt(4.95² + 1.62²) = 5.21
//! against a limit of 5.0 and refuses. Both readings of "measured from the feet"
//! turn the accepting half red and neither turns the refusing half red — on a
//! horizontal ray a feet-measured distance is *longer*, so a feet-measuring
//! implementation refuses more rather than less. The accept side is the only
//! falsifier there is.
//!
//! **The answer this is judged by is the refusal's own name, not a world that
//! did not move.** Both halves leave every cell where it was: water is declared
//! `breakable = false`, so a swing that *does* reach it is refused too. What
//! separates them is which refusal arrives — nothing was found, or something was
//! found and would not yield — and a world comparison cannot tell those apart.

mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, EditReport, Refusal, TickIntent};
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::simulation::seat;
use mc_sim::world::World;
use mc_world::world::{VoxelWorld, WorldPos};

use support::chamber::{at, differences};
use support::{STONE, TestResult, WATER, content_registry, published_content};

/// Every cell at which a run differs from the world as declared.
type Changes = Vec<(WorldPos, String, String)>;

/// What one action answered, and what it did to the world.
type Acted = (Option<EditReport>, Changes);

/// How many chunk columns the world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// How far the floor runs on each horizontal axis: the whole column.
const FLOOR_SPAN: u32 = 16;

/// Where the feet stand, on the floor's top face.
const FEET_Y: f32 = 10.0;

/// The voxel row the eye is in, `floor(10.0 + 1.62)`.
const EYE_ROW: u32 = 11;

/// The one cell of water, whose near face stands at x = 13.0.
const THE_WATER_CELL: WorldPos = at(13, EYE_ROW, 8);

/// 13.0 − 7.95 = 5.05, and 13.0 − 8.05 = 4.95.
const TOO_FAR: Vec3 = Vec3::new(7.95, FEET_Y, 8.5);
const JUST_INSIDE: Vec3 = Vec3::new(8.05, FEET_Y, 8.5);

/// Yaw facing +x, which is where both rays go.
const TOWARD_THE_WATER: f32 = 0.0;

#[test]
fn a_swing_at_water_first_met_beyond_five_blocks_from_the_eye_finds_no_target_at_all() -> TestResult
{
    let refusing = acting(standing(TOO_FAR))?;
    let accepting = acting(standing(JUST_INSIDE))?;

    assert_eq!(
        (refusing, accepting),
        (
            (Some(EditReport::Refused(Refusal::NoTarget)), nothing()),
            (
                Some(EditReport::Refused(Refusal::Indestructible)),
                nothing()
            )
        ),
        "the water's near face is met at 5.05 blocks from the eye, which is past the reach, so \
         there is no target at all — not water that could not be broken, but nothing found. One \
         tenth of a block nearer it is met at 4.95 and the same request from the same fixture \
         finds the water and is turned away by `breakable = false` instead. The pair is what \
         makes this the *boundary* rather than an implementation that finds nothing anywhere, \
         and the two runs differ in the feet's x and in nothing else"
    );
    Ok(())
}

/// One tick over a fresh world asking for one break, and what that did to the
/// world compared with the same world as declared.
fn acting(player: PlayerState) -> Result<Acted, Box<dyn Error>> {
    let declared = one_cell_of_water_over_a_floor()?;
    let played = one_cell_of_water_over_a_floor()?;
    let content = published_content(played.registry())?;
    let mut simulation = seat(player, played, content).simulation;
    let report = simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(ActionIntent::Break),
    });
    Ok((report, differences(&declared, simulation.world())))
}

/// A stone floor across the whole column, and one cell of water standing in the
/// eye's row with nothing behind it.
fn one_cell_of_water_over_a_floor() -> Result<World, Box<dyn Error>> {
    let registry = Arc::new(content_registry()?);
    let mut blocks = VoxelWorld::empty(COLUMNS);
    let stone = BlockName::parse(STONE)?;
    for z in 0..FLOOR_SPAN {
        for x in 0..FLOOR_SPAN {
            blocks.set_block(
                WorldPos {
                    x,
                    y: FLOOR_LAYER,
                    z,
                },
                &stone,
                &registry,
            )?;
        }
    }
    blocks.set_block(THE_WATER_CELL, &BlockName::parse(WATER)?, &registry)?;
    Ok(World::new(blocks, registry)?)
}

/// A player standing still on the floor at `feet`, facing the water with a level
/// view.
fn standing(feet: Vec3) -> PlayerState {
    PlayerState {
        position: feet,
        velocity: Vec3::ZERO,
        yaw: TOWARD_THE_WATER,
        pitch: 0.0,
        on_ground: true,
    }
}

/// No cell of the fixture moved.
fn nothing() -> Changes {
    Vec::new()
}
