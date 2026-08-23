//! A first launch into a world it generated starts the player at exactly the
//! spawn that generation derives, and moves them not at all.
//!
//! # Why "covers no solid cell" would be a test that cannot fail
//!
//! The derived spawn stands three blocks above its column's own surface, the sea
//! fills only to a fixed height at or below every surface, and no shipped content
//! can put anything in the two cells the box would cover there. So an assertion
//! that the generated door's player is somewhere clear is green before a line is
//! written, and it would stay green under an entry that moved them, cell-centred
//! them or grounded them.
//!
//! **The failure this is really for runs the other way.** An entry check that
//! touches a player who needed nothing changes where every golden frame is shot
//! from — the capture suites build their simulation through this same door — so
//! the whole player state is compared here rather than a position: velocity,
//! facing, and whether the world is claimed to be holding them up, each of which
//! an over-eager clearing would rewrite.
//!
//! # The spawn is derived twice, on purpose
//!
//! The scenario compares against the heightmap's own answer for the spawn column
//! plus the declared height above it, which is how the launch derives it. The test
//! beside it compares that same answer against **the blocks the generator
//! actually placed** — the topmost grass of that column — which shares no code
//! with the heightmap lookup. A defect in the shared derivation would otherwise
//! surface as dozens of golden-image differences at once and be localised by
//! none of them.
//!
//! Every constant below is a declaration of the replay restated here rather than
//! read out of the crate: a fixture reading the constant it asserts against agrees
//! with a spawn that moved as readily as with one that did not.

mod support;

use std::error::Error;

use mc_sim::persistence::{LaunchError, simulation_at_launch};
use mc_sim::simulation::Seated;
use mc_world::persistence::Acceptance;

use support::launch::{a_world_to_launch_into, launching, save_path};
use support::{GRASS, TestResult, described, exactly_player, surface_height};

/// See `launch_world.rs`: nothing here writes a save, so there is no table for an
/// acceptance to decide anything about.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// The block column the player spawns over, how far above that column's own
/// surface height the feet start, and which way they face, in degrees.
const SPAWN_COLUMN: (u32, u32) = (63, 35);
const SPAWN_ABOVE_SURFACE: u32 = 3;
const SPAWN_YAW_DEGREES: f32 = 230.0;

/// How high a column reaches, in blocks — the top of the range the oracle below
/// searches downward from.
const COLUMN_HEIGHT: u32 = 256;

#[test]
fn a_launch_with_no_save_to_resume_starts_the_player_at_the_spawn_the_generation_derives()
-> TestResult {
    let (registry, generated, directory) = a_world_to_launch_into()?;
    let save = save_path(&directory);
    let (column_x, column_z) = SPAWN_COLUMN;
    let surface = surface_height(&generated, column_x, column_z)?;

    let launched = simulation_at_launch(&save, launching(&registry, ACCEPTING)?);

    assert_eq!(
        (seated(&launched), save.exists()),
        (Ok(the_spawn_the_generation_derives(surface)), false),
        "with nothing at {}, the launch generates a world and stands the player at the spawn that \
         generation derives: horizontally centred over column ({column_x}, {column_z}), feet \
         {SPAWN_ABOVE_SURFACE} blocks above that column's own surface height of {surface}, at \
         rest, facing {SPAWN_YAW_DEGREES} degrees, and not yet standing on anything. **Every one \
         of those five is asserted**, because a player who needed no moving must be left alone in \
         all of them — an entry that cell-centred them, zeroed a velocity that was already zero \
         for a different reason, or claimed ground contact nothing checked, moves the camera every \
         committed golden frame is shot through. It answered {launched:?}",
        save.display()
    );
    Ok(())
}

/// A direct assertion on the derivation the scenario above and every golden frame
/// both rest on.
///
/// The heightmap's answer for the spawn column and the blocks the generator laid
/// into that column are two statements of one surface, and only one of them is
/// what the spawn is derived from. Asserting they agree localises a defect that
/// the golden suites would otherwise report as image differences with nothing
/// naming the cause.
#[test]
fn the_derived_spawn_is_three_blocks_above_its_own_columns_surface_height() -> TestResult {
    let (_, generated, _) = a_world_to_launch_into()?;
    let (column_x, column_z) = SPAWN_COLUMN;

    let reported = surface_height(&generated, column_x, column_z)?;
    let laid_down = topmost_grass(&generated, column_x, column_z)?;

    assert_eq!(
        (reported, reported + SPAWN_ABOVE_SURFACE),
        (laid_down, laid_down + SPAWN_ABOVE_SURFACE),
        "the surface height the world reports for column ({column_x}, {column_z}) and the row it \
         actually laid {GRASS} into are two statements of one surface, and the spawn is derived \
         from the first while a player stands on the second. The heightmap says {reported} and the \
         blocks say {laid_down}; a spawn three blocks above the wrong one of those is a player \
         starting inside the ground or a long way over it, and nothing but a golden image would \
         say so"
    );
    Ok(())
}

/// The highest row of the block column at `(x, z)` holding grass.
///
/// **An oracle over the blocks, sharing no code with the heightmap lookup.** The
/// generator lays grass at the surface and exactly there; water fills the rows
/// above it wherever the surface is under the sea, which is why the topmost block
/// of a column is not the answer and the topmost *grass* is.
///
/// # Errors
///
/// Returns an error if the column holds no grass at all — a world that laid none
/// would otherwise satisfy every assertion about where it is by having none.
fn topmost_grass(
    world: &mc_sim::replay::ReplayWorld,
    x: u32,
    z: u32,
) -> Result<u32, Box<dyn Error>> {
    (0..COLUMN_HEIGHT)
        .rev()
        .find(|y| {
            world
                .block_at(x, *y, z)
                .is_some_and(|contents| described(contents) == GRASS)
        })
        .ok_or_else(|| format!("the block column at ({x}, {z}) holds no {GRASS} at all").into())
}

/// Everything the specification asks about a seated player, as the integers its
/// floats are: where they stand, how fast they are going, which way they face,
/// how far up or down they look, and whether the world is holding them up.
type SeatedPlayer = ([u32; 3], [u32; 3], u32, u32, bool);

/// The player a first launch must seat, derived from the declarations rather than
/// written out.
///
/// **All five fields, because all five are what "moving them not at all" means.**
/// An entry that cell-centred the spawn, took away a velocity that is zero for a
/// different reason, or claimed the ground contact `spawn` deliberately does not
/// claim, is caught here and by nothing else short of a golden image.
fn the_spawn_the_generation_derives(surface: u32) -> SeatedPlayer {
    let (column_x, column_z) = SPAWN_COLUMN;
    (
        [
            (column_x as f32 + 0.5).to_bits(),
            ((surface + SPAWN_ABOVE_SURFACE) as f32).to_bits(),
            (column_z as f32 + 0.5).to_bits(),
        ],
        [0.0_f32.to_bits(); 3],
        SPAWN_YAW_DEGREES.to_radians().to_bits(),
        0.0_f32.to_bits(),
        false,
    )
}

/// The whole player a launch seated, as the integers its floats are — or the
/// refusal it gave instead.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
fn seated(launched: &Result<Seated, LaunchError>) -> Result<SeatedPlayer, String> {
    let playing = launched.as_ref().map_err(LaunchError::to_string)?;
    Ok(exactly_player(&playing.simulation.latest().player))
}
