//! The exit criterion itself: what a player did to the world and to themselves
//! is still true the next time they start the game.
//!
//! Every run below is driven through the client's own dispatch — a real mouse
//! event, a real key transition, real pointer motion — quit through the client's
//! own ending, and started again through the client's own launch, in a process
//! that constructs no event loop, opens no window and acquires no GPU adapter.
//!
//! # The four are two pairs of controls
//!
//! A break leaves a cell holding nothing and a placement leaves one holding a
//! block, so a client that dropped every edit fails both, and one that wrote
//! blocks but never wrote emptiness fails only the break. The player's half is
//! the same pairing one step smaller: a resume that restored where they stood
//! and forgot which way they were looking is caught by the facing alone, which
//! is why the two are separate scenarios and separate assertions rather than one
//! comparison of everything at once.
//!
//! # What was true at the quit is the oracle, and it is read before the quit
//!
//! Where the player walked to is not a number anybody can write down in advance
//! — it is the sum of a declared walk speed over a declared number of ticks —
//! so the expectation is the state the simulation published on the last tick
//! before the quit. Each scenario therefore carries a control saying that state
//! is *not* where the player started, or a resume that placed everybody back at
//! the spawn would satisfy it.
//!
//! # The aim, restated rather than imported
//!
//! `click_dispatch.rs` derives it from the same declared floor: 280 raw counts
//! of downward motion is 35.29° below level, which meets the floor layer at
//! [`LOOKED_AT`] through its upward face, well inside the reach. Restated here
//! because a fixture reading the constant it depends on agrees with an aim that
//! moved.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/persistence.rs"]
mod persistence;

use std::error::Error;
use std::sync::Arc;

use mc_client::startup::simulation_to_play;
use mc_core::block::BlockRegistry;
use mc_render::window::Ending;
use mc_sim::action::EditReport;
use mc_sim::player::{BlockPos, PlayerState};
use mc_sim::replay::ReplayWorld;
use mc_world::persistence::Acceptance;
use tempfile::TempDir;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use input::InputHarness;
use persistence::{
    GROUND, Launched, NOTHING, SPAWN, TestResult, declared, described, facing, generated_world,
    held_at, refusal, registry_of, save_in, standing_on_the_floor, stood_at,
    with_the_replay_blocks,
};

/// Every save here is written against the registry it is read against, so
/// nothing about its blocks can have changed and the acceptance decides nothing.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// How far the pointer is pushed down before a click, in raw device counts.
const AIM_DOWN_COUNTS: f64 = 280.0;

/// The cell that aim first meets: the nearest solid voxel along the ray, in the
/// two spellings the two sides of this file need.
const LOOKED_AT: BlockPos = BlockPos { x: 10, y: 9, z: 8 };
const LOOKED_AT_CELL: (u32, u32, u32) = (10, 9, 8);

/// The cell a placement against that aim lands in — one step back out through
/// the upward face the ray came in by, which is empty before the placement.
const AGAINST_ITS_UPWARD_FACE: BlockPos = BlockPos {
    x: LOOKED_AT.x,
    y: LOOKED_AT.y + 1,
    z: LOOKED_AT.z,
};
const PLACED_CELL: (u32, u32, u32) = (10, 10, 8);

/// A cell of the floor the aim never reaches.
///
/// The break scenario's own control: a resume that came back with an empty world
/// holds nothing at the broken cell too, and this is what tells the two apart.
const UNTOUCHED_CELL: (u32, u32, u32) = (2, 9, 2);

/// How many ticks the player walks forward for before quitting.
///
/// Twenty at the declared walk speed covers about 1.5 blocks, which is well
/// inside the eight blocks of margin between the spawn and the nearest edge of
/// the declared floor — so the walk cannot end by running out of world.
const WALKING_TICKS: u32 = 20;

/// How far the pointer is pushed to turn, in raw device counts: right and down.
///
/// **Both axes, and neither of them zero.** A resume that restored the yaw and
/// forgot the pitch is the defect this scenario is smallest against, and a turn
/// that only ever went sideways could not see it.
const TURN_RIGHT_COUNTS: f64 = 300.0;
const TURN_DOWN_COUNTS: f64 = 200.0;

/// How many ticks it takes to spend a look: one.
const ONE_TICK: u32 = 1;

#[test]
fn a_cell_broken_before_the_quit_holds_nothing_when_the_client_starts_again() -> TestResult {
    let world = a_world_to_play_in()?;
    let save = save_in(&world.directory);
    let mut playing = a_client_playing(&world)?;
    playing.move_pointer(0.0, AIM_DOWN_COUNTS);
    playing.click(MouseButton::Left);
    let broke = playing.edit();
    playing.quit(Ending::Closed, &save);
    drop(playing);

    let started_again = launch(&world, &save);

    assert_eq!(
        (
            held_at(&started_again, LOOKED_AT_CELL),
            held_at(&started_again, UNTOUCHED_CELL),
            change(broke)
        ),
        (
            Ok(NOTHING.to_owned()),
            Ok(GROUND.to_owned()),
            Some((LOOKED_AT, GROUND.to_owned(), NOTHING.to_owned()))
        ),
        "the player dug a hole at {LOOKED_AT:?} and quit, and the hole is still there when they \
         come back — the cell holds nothing, which is what the block that was there breaking into \
         nothing leaves behind. The floor at {UNTOUCHED_CELL:?} is the control: a resume that \
         handed back an empty world would hold nothing at the broken cell too, and it is that \
         second cell which says the world came back rather than went away. It came back as: {}",
        refusal(&started_again)
    );
    Ok(())
}

#[test]
fn a_block_placed_before_the_quit_is_still_there_when_the_client_starts_again() -> TestResult {
    let world = a_world_to_play_in()?;
    let save = save_in(&world.directory);
    let mut playing = a_client_playing(&world)?;
    playing.move_pointer(0.0, AIM_DOWN_COUNTS);
    playing.click(MouseButton::Right);
    let built = playing.edit();
    playing.quit(Ending::Closed, &save);
    drop(playing);

    let started_again = launch(&world, &save);

    assert_eq!(
        (held_at(&started_again, PLACED_CELL), change(built)),
        (
            Ok(GROUND.to_owned()),
            Some((
                AGAINST_ITS_UPWARD_FACE,
                NOTHING.to_owned(),
                GROUND.to_owned()
            ))
        ),
        "the player built a block into an empty cell at {AGAINST_ITS_UPWARD_FACE:?} and quit, and \
         it is still standing there when they come back. This is the other half of the pair the \
         broken cell makes: a client that saved blocks but never saved emptiness passes that one \
         and this one, and a client that saved emptiness but dropped what was placed passes only \
         that one. It came back as: {}",
        refusal(&started_again)
    );
    Ok(())
}

#[test]
fn a_player_who_walked_before_the_quit_stands_there_when_the_client_starts_again() -> TestResult {
    let world = a_world_to_play_in()?;
    let save = save_in(&world.directory);
    let mut playing = a_client_playing(&world)?;
    playing.press(KeyCode::KeyW);
    let walked = last_published(&mut playing, WALKING_TICKS)?;
    playing.quit(Ending::Closed, &save);
    drop(playing);

    let started_again = launch(&world, &save);

    assert_eq!(
        (stood_at(&started_again), at(walked.position) == at(SPAWN)),
        (Ok(at(walked.position)), false),
        "the player walked forward for {WALKING_TICKS} ticks and quit standing at \
         {:?}, and that is where they are standing when they come back — not at a spawn somebody \
         derived for them, and not at the origin a save that recorded nobody would hand back. The \
         second half is the control: they walked far enough that where they ended is not where \
         they started, or a resume that simply re-spawned them would satisfy the first half. It \
         came back as: {}",
        walked.position,
        refusal(&started_again)
    );
    Ok(())
}

#[test]
fn a_player_who_turned_before_the_quit_faces_that_way_when_the_client_starts_again() -> TestResult {
    let world = a_world_to_play_in()?;
    let save = save_in(&world.directory);
    let mut playing = a_client_playing(&world)?;
    playing.move_pointer(TURN_RIGHT_COUNTS, TURN_DOWN_COUNTS);
    let turned = last_published(&mut playing, ONE_TICK)?;
    playing.quit(Ending::Closed, &save);
    drop(playing);

    let started_again = launch(&world, &save);
    let looked = (turned.yaw.to_bits(), turned.pitch.to_bits());

    assert_eq!(
        (
            facing(&started_again),
            looked == (0_f32.to_bits(), 0_f32.to_bits())
        ),
        (Ok(looked), false),
        "the player turned to look right and down and quit facing {} of yaw and {} of pitch, and \
         that is the way they are facing when they come back. **Both angles**, because a resume \
         that restored where somebody stood and forgot which way they were looking is the same \
         defect one step smaller and only this scenario catches it. The second half is the \
         control: the turn moved them off the facing they started at. It came back as: {}",
        turned.yaw,
        turned.pitch,
        refusal(&started_again)
    );
    Ok(())
}

/// The registry a run is played against, the world a launch would generate if
/// there were no save, and the directory the save lives in.
#[derive(Debug)]
struct AWorld {
    registry: Arc<BlockRegistry>,
    generated: ReplayWorld,
    directory: TempDir,
}

/// A registry declaring the fixture floor, the world a launch generates, and
/// somewhere to keep a save.
fn a_world_to_play_in() -> Result<AWorld, Box<dyn Error>> {
    let registry = Arc::new(with_the_replay_blocks(registry_of(vec![declared(
        GROUND, true,
    )?])?)?);
    let generated = generated_world(&registry)?;
    Ok(AWorld {
        registry,
        generated,
        directory: TempDir::new()?,
    })
}

/// A client whose window is open and whose world has landed.
fn a_client_playing(world: &AWorld) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = standing_on_the_floor(Arc::clone(&world.registry))?;
    let mut harness = InputHarness::started();
    harness.play(simulation, holding);
    Ok(harness)
}

/// What the client starts in the next time it is launched.
fn launch(world: &AWorld, save: &std::path::Path) -> Launched {
    simulation_to_play(
        &world.generated,
        Arc::clone(&world.registry),
        save,
        ACCEPTING,
    )
}

/// The player as the last of `ticks` tick steps published them.
///
/// The state the quit that follows will write, read one step before it is
/// written rather than guessed at from the declared walk speed.
fn last_published(playing: &mut InputHarness, ticks: u32) -> Result<PlayerState, Box<dyn Error>> {
    playing
        .ticks(ticks)
        .last()
        .map(|published| published.player)
        .ok_or_else(|| "the driven client published no tick at all".into())
}

/// A position as the integers its floats are.
fn at(position: glam::Vec3) -> [u32; 3] {
    position.to_array().map(f32::to_bits)
}

/// What one tick's report says about the world: the cell it changed, what stood
/// there, and what stands there now.
///
/// `None` is a tick that asked the world for nothing, which is not the same as a
/// refusal and is told apart from one deliberately.
fn change(report: Option<EditReport>) -> Option<(BlockPos, String, String)> {
    match report? {
        EditReport::Changed { cell, from, to } => {
            Some((cell, described(from.as_ref()), described(to.as_ref())))
        }
        EditReport::Refused(_) => None,
    }
}
