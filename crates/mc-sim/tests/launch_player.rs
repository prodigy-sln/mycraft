//! Where a launch stands the player: on the ground the generator made, or
//! exactly where a save left them.
//!
//! **The two placements are each other's control.** A resume reads its player
//! out of the save and derives no height at all; a first launch derives one from
//! the generated world's heightmap and reads nothing. An implementation that
//! derived in both cases passes the second and fails the first, and one that
//! read a stored player in both cases fails the second — so neither can be
//! satisfied by the answer the other wants.
//!
//! **Everything here is read off the simulation a launch handed back**, never
//! off the loader. The loader has two entry points onto one read path and a
//! player it invented is indistinguishable from a player it read, to anything
//! that asks it directly.
//!
//! Nothing below is a number off a run. The recorded player is the fixture's own
//! declaration; the height a first launch stands at is the generated world's
//! heightmap plus the declared height the spawn stands above it; and the speed
//! the player is saved at is the arithmetic of two declared constants.

mod support;

use std::sync::Arc;

use mc_sim::persistence::{self, simulation_at_launch};
use mc_sim::player::MovementIntent;
use mc_sim::replay::simulation_for;
use mc_world::persistence::{Acceptance, save_world};

use support::launch::{a_world_to_launch_into, moving, placed, recorded_player, save_path, stood};
use support::{TestResult, surface_height};

/// See `launch_world.rs`: every save here is written against the registry it is
/// read against, so the acceptance decides nothing.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// The block column the player spawns over, and how far above that column's own
/// surface height the feet start, in blocks.
///
/// Declarations of the replay, restated here rather than read out of the crate:
/// a fixture reading the constant it asserts against would agree with a spawn
/// that moved as readily as with one that did not.
const SPAWN_COLUMN: (u32, u32) = (32, 32);
const SPAWN_ABOVE_SURFACE: u32 = 3;

/// How many ticks the saved player falls for, and how fast that leaves them
/// going, in blocks per second.
///
/// **Arithmetic and not a measurement.** A fall accelerates at 30 blocks per
/// second squared and a tick is a sixtieth of a second, so each tick adds half a
/// block per second and sixteen of them make 8.0 exactly. Those sixteen ticks
/// cover 16 × 17 ÷ 2 ÷ 60 × ½ = 1.13 blocks of the two the spawn stands above
/// the ground, so the player is still in the air — a player who had landed would
/// be at rest before the save was written, and the scenario would be about
/// nothing.
const FALLING_TICKS: u32 = 16;
const FALLING_SPEED: f32 = 8.0;

/// How far two speeds this suite calls equal may differ, in blocks per second.
const EPSILON: f32 = 1e-4;

#[test]
fn a_launch_resuming_from_a_save_stands_the_player_where_the_save_recorded_them() -> TestResult {
    let (registry, generated, directory) = a_world_to_launch_into()?;
    let save = save_path(&directory);
    let recorded = recorded_player();
    save_world(&save, generated.blocks(), recorded, &registry)?;

    let launched =
        simulation_at_launch(&save, mc_sim::REPLAY_SEED, Arc::clone(&registry), ACCEPTING);

    let [_, height, _] = recorded.position;
    assert_eq!(
        (placed(&launched), a_height_a_heightmap_could_report(height)),
        (
            Ok((
                recorded.position.map(f32::to_bits),
                recorded.yaw.to_bits(),
                recorded.pitch.to_bits()
            )),
            false
        ),
        "a resume puts the player where the save recorded them — at {:?}, facing {} and \
         looking {} — and derives no height from anything: {height} is not a height any \
         heightmap could have reported, so a launch that derived one cannot land here by \
         accident. The launch answered {launched:?}",
        recorded.position,
        recorded.yaw,
        recorded.pitch
    );
    Ok(())
}

#[test]
fn a_launch_with_no_save_stands_the_player_at_the_height_the_heightmap_reports() -> TestResult {
    let (registry, generated, directory) = a_world_to_launch_into()?;
    let save = save_path(&directory);
    let (column_x, column_z) = SPAWN_COLUMN;
    let surface = surface_height(&generated, column_x, column_z)?;
    let derived = (surface + SPAWN_ABOVE_SURFACE) as f32;

    let launched =
        simulation_at_launch(&save, mc_sim::REPLAY_SEED, Arc::clone(&registry), ACCEPTING);

    assert_eq!(
        (stood(&launched), save.exists()),
        (Ok(derived.to_bits()), false),
        "with nothing at {}, a launch stands the player on the world it generated: column \
         ({column_x}, {column_z}) reports a surface height of {surface} and the feet start \
         {SPAWN_ABOVE_SURFACE} blocks above it, which is {derived}. The launch answered \
         {launched:?}",
        save.display()
    );
    Ok(())
}

#[test]
fn a_player_saved_while_they_were_moving_resumes_at_rest() -> TestResult {
    let (registry, generated, directory) = a_world_to_launch_into()?;
    let save = save_path(&directory);
    let mut playing = simulation_for(&generated, Arc::clone(&registry))?;
    for _ in 0..FALLING_TICKS {
        playing.advance(MovementIntent::default());
    }
    let speed_when_saved = playing.latest().player.velocity.length();
    persistence::save(&playing, &save)?;

    let launched =
        simulation_at_launch(&save, mc_sim::REPLAY_SEED, Arc::clone(&registry), ACCEPTING);

    assert_eq!(
        (
            moving(&launched),
            (speed_when_saved - FALLING_SPEED).abs() <= EPSILON
        ),
        (Ok([0, 0, 0]), true),
        "a save is written of a player going {FALLING_SPEED} blocks per second, and the \
         launch that resumes it stands them still: no velocity is stored, and a resumed \
         player is at rest before gravity has anything to say. The save was written at \
         {speed_when_saved} blocks per second and the launch answered {launched:?}"
    );
    Ok(())
}

/// Whether `height` is one the generated world's heightmap could have reported.
///
/// A heightmap reports whole numbers of blocks, so a fractional height cannot
/// have been derived from one — which is what makes the recorded height's
/// quarter block the teeth of the resume scenario rather than a decoration.
fn a_height_a_heightmap_could_report(height: f32) -> bool {
    height.floor() >= height
}
