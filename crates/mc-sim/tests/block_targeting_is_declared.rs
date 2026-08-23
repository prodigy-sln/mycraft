//! What a ray stops at is what a block *declares* about being aimed at, and it
//! follows an edit rather than only a load.
//!
//! `block_targeting.rs` is the other half of this question and everything it
//! says about how these fixtures are read applies here unchanged: every run is
//! driven through `Simulation::advance`, the same call a click reaches, and
//! judged by diffing the world against the fixture *as declared*. No number here
//! was read off a run.
//!
//! # The two fixture blocks these scenarios exist for
//!
//! Every block the base game ships has "stops a player" and "a ray stops here"
//! agreeing, so against shipped content alone a walk that stops at the first
//! solid cell and one that stops at the first targetable cell answer identically
//! at every cell. The fixture registry declares two blocks where they disagree,
//! one in each direction — one that stops nobody and that a ray stops at, and one
//! that stops a player and that a ray goes through — and the two are needed
//! rather than one: a rule reading solidity where it means targetability reports
//! the wrong cell in *opposite* directions for them.
//!
//! # Three things the fixtures hold that no assertion can
//!
//! **Every block standing in line along a ray carries a different name.** An edit
//! that took the cell one step beyond the hit, or one step before it, changes the
//! same number of cells as the correct one; only the names say which cell went.
//! The floor carries a third name for the same reason.
//!
//! **The blocks stand in the eye's own voxel row and in no other.** The eye is
//! the feet plus 1.62 blocks, so with the feet on a floor whose top face is at
//! y = 10 the eye sits at 11.62 and the row it is in is 11. The row the feet are
//! in, y = 10, is empty along every ray below.
//!
//! **The cell the second pair of scenarios aims at holds *nothing* when the
//! world is built.** That is what makes them about an edit rather than a load: a
//! view of what may be aimed at that is resolved once at construction and never
//! written again answers "nothing may be aimed at here" for that cell forever,
//! and satisfies every scenario in this file that is about a *declared* world.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, EditReport, TickIntent};
use mc_sim::player::{BlockPos, MovementIntent, PlayerState};
use mc_sim::simulation::{Simulation, seat};
use mc_world::section::Contents;
use mc_world::world::WorldPos;

use support::chamber::{AIMABLE, BlockChamber, UNAIMABLE, at, differences, fixture_content};
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

/// The nearer of two cells standing in line along the ray, two blocks from the
/// eye, and the farther, four blocks from it.
const NEAR: WorldPos = at(10, EYE_ROW, 8);
const FAR: WorldPos = at(12, EYE_ROW, 8);

/// The cell between them, three blocks from the eye, which every fixture here
/// declares empty.
const BETWEEN: WorldPos = at(11, EYE_ROW, 8);

#[test]
fn a_break_takes_a_block_that_stops_nobody_but_declares_a_ray_may_stop_at_it() -> TestResult {
    let chamber = a_block_that_stops_nobody_in_front_of_one_that_does();
    let declared = chamber.build()?;
    let broken = after(&chamber, standing(LINED_UP), ActionIntent::Break)?;

    assert_eq!(
        differences(&declared, broken.world()),
        vec![(NEAR, AIMABLE.to_owned(), NOTHING.to_owned())],
        "the near block declares that a ray may stop at it and declares that it stops nobody, and \
         those are two claims rather than one. The walk stops at the near cell, so exactly that \
         cell moved and the block four blocks along still holds what it was declared with — a \
         walk still reading solidity reaches the far one instead, and the two carry different \
         names so the diff says which"
    );
    Ok(())
}

#[test]
fn a_break_reaches_past_a_solid_block_that_declares_no_ray_may_stop_at_it() -> TestResult {
    let chamber = a_block_that_may_not_be_aimed_at_in_front_of_one_that_may();
    let declared = chamber.build()?;
    let broken = after(&chamber, standing(LINED_UP), ActionIntent::Break)?;

    assert_eq!(
        differences(&declared, broken.world()),
        vec![(FAR, DIRT.to_owned(), NOTHING.to_owned())],
        "the near block stops a player and declares that no ray may stop at it, so the walk goes \
         straight through it and reports what stands beyond. A walk still reading solidity reports \
         the near block itself and empties that cell instead, and the third name on the floor is \
         what keeps either answer from being confused with the ground"
    );
    Ok(())
}

#[test]
fn a_block_placed_into_an_empty_cell_is_what_the_next_ray_across_that_cell_stops_at() -> TestResult
{
    let chamber = one_block_to_build_against();
    let declared = chamber.build()?;
    let mut simulation = seat(standing(LINED_UP), chamber.build()?, fixture_content()?).simulation;

    asking(&mut simulation, place(AIMABLE)?);
    let second = asking(&mut simulation, place(DIRT)?);

    assert_eq!(
        (second, differences(&declared, simulation.world())),
        (
            Some(EditReport::Changed {
                cell: signed(NEAR),
                from: Contents::Empty,
                to: Contents::Holds(BlockName::parse(DIRT)?),
            }),
            vec![
                (NEAR, NOTHING.to_owned(), DIRT.to_owned()),
                (BETWEEN, NOTHING.to_owned(), AIMABLE.to_owned()),
            ]
        ),
        "the first tick puts a block that stops nobody into a cell that held nothing, and the \
         second tick's ray crosses that cell. What a ray may stop at has to have followed the \
         edit: a view resolved once when the world was built answers that the cell is not \
         aimable, carries the ray on to the block behind it, and refuses the second placement \
         because the cell it would step back into is the one that was just filled"
    );
    Ok(())
}

#[test]
fn breaking_the_block_a_ray_stopped_at_lets_the_next_ray_reach_the_one_behind_it() -> TestResult {
    let chamber = a_block_that_stops_nobody_in_front_of_one_that_does();
    let declared = chamber.build()?;
    let mut simulation = seat(standing(LINED_UP), chamber.build()?, fixture_content()?).simulation;

    asking(&mut simulation, ActionIntent::Break);
    asking(&mut simulation, ActionIntent::Break);

    assert_eq!(
        differences(&declared, simulation.world()),
        vec![
            (NEAR, AIMABLE.to_owned(), NOTHING.to_owned()),
            (FAR, DIRT.to_owned(), NOTHING.to_owned()),
        ],
        "two swings from a player who never moved. The first empties the cell the ray stopped at, \
         which has to clear what may be aimed at there, so the second swing reaches the next \
         block along the same ray. A view that was never written again stops the second ray at \
         the cell it just emptied and refuses it as nothing to break, leaving one cell moved \
         instead of two"
    );
    Ok(())
}

/// A floor, a block that stops nobody and that a ray stops at two blocks along
/// the ray, and one that stops a player at four.
fn a_block_that_stops_nobody_in_front_of_one_that_does() -> BlockChamber {
    floored().cell(NEAR, AIMABLE).cell(FAR, DIRT)
}

/// A floor, a block that stops a player and that no ray stops at two blocks
/// along the ray, and one that a ray does stop at four.
fn a_block_that_may_not_be_aimed_at_in_front_of_one_that_may() -> BlockChamber {
    floored().cell(NEAR, UNAIMABLE).cell(FAR, DIRT)
}

/// A floor, and one ordinary block four blocks along the ray with two empty
/// cells in front of it.
fn one_block_to_build_against() -> BlockChamber {
    floored().cell(FAR, DIRT)
}

/// Nothing anywhere, with one layer of floor for the player to stand on.
fn floored() -> BlockChamber {
    BlockChamber::empty(COLUMNS).run(at(0, FLOOR_LAYER, 0), at(16, FLOOR_LAYER + 1, 16), STONE)
}

/// A player standing still on the floor at `feet`, facing along the row with a
/// level view.
fn standing(feet: Vec3) -> PlayerState {
    PlayerState {
        position: feet,
        velocity: Vec3::ZERO,
        yaw: ALONG_THE_ROW,
        pitch: 0.0,
        on_ground: true,
    }
}

/// One tick over a fresh build of `chamber`, asking for no movement and one
/// action.
fn after(
    chamber: &BlockChamber,
    player: PlayerState,
    action: ActionIntent,
) -> Result<Simulation, Box<dyn Error>> {
    let mut simulation = seat(player, chamber.build()?, fixture_content()?).simulation;
    asking(&mut simulation, action);
    Ok(simulation)
}

/// One more tick of `simulation`, asking for no movement and one action.
fn asking(simulation: &mut Simulation, action: ActionIntent) -> Option<EditReport> {
    simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(action),
    })
}

/// A request to place `block`.
fn place(block: &str) -> Result<ActionIntent, Box<dyn Error>> {
    Ok(ActionIntent::Place {
        block: BlockName::parse(block)?,
    })
}

/// The same cell in the signed spelling an edit report carries.
const fn signed(cell: WorldPos) -> BlockPos {
    BlockPos {
        x: cell.x as i32,
        y: cell.y as i32,
        z: cell.z as i32,
    }
}
