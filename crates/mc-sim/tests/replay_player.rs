//! Where the replay puts the player, and what the camera does about it.
//!
//! **Nothing here is a number read off a run.** The spawn's height comes from the
//! world's own heightmap, so the same declaration holds for any seed; the resting
//! height after the fall comes from the heightmap of whichever column the player
//! turned out to be over; the drop between the two is the difference of those two
//! declarations and is exactly 2.0 blocks whatever the terrain does; and the
//! scripted turn is exactly 30 degrees because looking around is never obstructed
//! by anything. A horizontal-displacement figure over the replay would have been
//! none of these — where a walk across generated terrain ends is not
//! hand-derivable — and the specification records why it was rejected.
//!
//! **The two "nothing changes" scenarios are the ones to read carefully.** A
//! simulation that published a fixed camera forever satisfies both of them
//! vacuously, and so does one that never advances anything. What they are for is
//! the opposite defect: the orbit this feature replaces moved the eye 5 blocks
//! every tick and swung its look direction through a full turn, so an eye that
//! still travels while the player is standing still is a camera the player does
//! not own. Their controls are the two scenarios beside them — the drop and the
//! turn, which fail against a camera that does *not* follow the player.
//!
//! Comparisons use the declared 1 × 10⁻⁴ epsilon, in blocks and in radians. In
//! radians it is a hundredth of one scripted turn's 0.0175, and two orders of
//! magnitude above the drift thirty `f32` additions can accumulate at this
//! magnitude.

mod support;

use std::error::Error;
use std::sync::Arc;

use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::replay::{
    ReplayWorld, SCRIPT_TICKS, TickIndex, scripted_intent, simulation_for, spawn,
};
use mc_sim::simulation::{SimSnapshot, Simulation};

use support::{
    TestResult, content_registry, exactly_player, published_content, replay_world, surface_height,
};

/// How far two figures this feature calls equal may differ, in blocks or in
/// radians.
const EPSILON: f32 = 1e-4;

/// The block column the player spawns over.
const SPAWN_COLUMN: (u32, u32) = (32, 32);

/// How far above its column's surface height the feet start, in blocks.
const SPAWN_ABOVE_SURFACE: u32 = 3;

/// Which way the player faces at the spawn, in degrees: toward the landmark
/// pillar.
const SPAWN_YAW_DEGREES: f32 = 225.0;

/// How far the feet fall between the spawn and coming to rest, in blocks.
///
/// The difference of two declarations and not a measurement: the feet start
/// three blocks above the column's surface height and a surface height `h` holds
/// them at `h + 1`, so the fall is two blocks whatever `h` is.
const SPAWN_DROP: f32 = SPAWN_ABOVE_SURFACE as f32 - 1.0;

/// The tick the player has settled by.
///
/// A two-block fall lands during tick 22 and a player that has landed stays
/// landed, so reading at 30 has eight ticks of margin and cannot read a player
/// still in the air.
const SETTLED_TICK: u32 = 30;

/// The two ticks the scripted turn runs between, and how far it turns in total.
///
/// The script turns by one degree on each of ticks 60 through 89, and those
/// thirty deltas are applied by the ticks that produce tick 61 through tick 90.
const TURN_FROM_TICK: u32 = 60;
const TURN_TO_TICK: u32 = 90;
const TURN_DEGREES: f32 = 30.0;

/// The declared world, and a simulation of it with the player at its spawn.
///
/// The registry is shared rather than borrowed, because the simulation's world
/// holds it for the life of the run: every edit resolves the name it writes
/// against the same registry the world's solidity was resolved against, and
/// there is no second registry for the two to disagree about.
fn replay() -> Result<(ReplayWorld, Simulation), Box<dyn Error>> {
    let registry = Arc::new(content_registry()?);
    let world = replay_world(&registry)?;
    let simulation = simulation_for(&world, Arc::clone(&registry), published_content(&registry)?)?;
    Ok((world, simulation))
}

/// The declared script's intents, one per tick, in order.
fn declared_script() -> Result<Vec<MovementIntent>, Box<dyn Error>> {
    Ok((0..SCRIPT_TICKS)
        .map(|tick| TickIndex::new(tick).map(scripted_intent))
        .collect::<Result<Vec<_>, _>>()?)
}

/// As many intents asking for nothing at all as the script has ticks.
fn asking_for_nothing() -> Vec<MovementIntent> {
    vec![MovementIntent::default(); SCRIPT_TICKS as usize]
}

/// Every snapshot the simulation publishes, from the spawn's own through the one
/// the last of `intents` produces.
fn published(simulation: &mut Simulation, intents: &[MovementIntent]) -> Vec<Arc<SimSnapshot>> {
    let mut snapshots = vec![simulation.latest()];
    for intent in intents {
        simulation.advance(*intent);
        snapshots.push(simulation.latest());
    }
    snapshots
}

/// The snapshot published at `tick`.
///
/// # Errors
///
/// Returns an error if the run did not reach that tick, which would otherwise
/// leave a scenario asserting nothing.
fn at_tick(snapshots: &[Arc<SimSnapshot>], tick: u32) -> Result<&SimSnapshot, Box<dyn Error>> {
    snapshots
        .get(tick as usize)
        .map(Arc::as_ref)
        .ok_or_else(|| format!("the run published no snapshot for tick {tick}").into())
}

/// The largest disagreement between two positions on either horizontal axis.
fn horizontally_apart(placed: [f32; 3], against: [f32; 3]) -> f32 {
    let [x, _, z] = placed;
    let [other_x, _, other_z] = against;
    (x - other_x).abs().max((z - other_z).abs())
}

/// Which way a camera looks: its target as an offset from its eye.
fn look_direction(camera: &mc_sim::replay::CameraPose) -> [f32; 3] {
    let [eye_x, eye_y, eye_z] = camera.eye;
    let [at_x, at_y, at_z] = camera.target;
    [at_x - eye_x, at_y - eye_y, at_z - eye_z]
}

/// The largest disagreement between two vectors on any one axis.
fn furthest_axis(placed: [f32; 3], against: [f32; 3]) -> f32 {
    placed
        .iter()
        .zip(against.iter())
        .map(|(placed, against)| (placed - against).abs())
        .fold(0.0, f32::max)
}

/// The declaration's rows for a spawn over a column of surface height `surface`,
/// each as what it is, where it is, and where it is declared to be.
///
/// One table, asserted in one place (`spec.md` §Table-driven scenarios): the
/// horizontal centre of the declared column, the feet three blocks over its own
/// surface, the yaw that faces the landmark, level, and at rest on every axis.
fn declared_spawn(start: &PlayerState, surface: u32) -> [(&'static str, f32, f32); 8] {
    let (column_x, column_z) = SPAWN_COLUMN;
    [
        ("feet x", start.position.x, column_x as f32 + 0.5),
        (
            "feet y",
            start.position.y,
            (surface + SPAWN_ABOVE_SURFACE) as f32,
        ),
        ("feet z", start.position.z, column_z as f32 + 0.5),
        ("yaw", start.yaw, SPAWN_YAW_DEGREES.to_radians()),
        ("pitch", start.pitch, 0.0),
        ("velocity x", start.velocity.x, 0.0),
        ("velocity y", start.velocity.y, 0.0),
        ("velocity z", start.velocity.z, 0.0),
    ]
}

#[test]
fn the_player_spawns_over_the_declared_column_facing_the_landmark() -> TestResult {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    let (column_x, column_z) = SPAWN_COLUMN;
    let surface = surface_height(&world, column_x, column_z)?;

    let start = spawn(&world)?;

    let wrong: Vec<String> = declared_spawn(&start, surface)
        .iter()
        .filter(|(_, placed, declared)| (placed - declared).abs() > EPSILON)
        .map(|(what, placed, declared)| format!("{what} is {placed}, not {declared}"))
        .collect();
    assert!(
        wrong.is_empty(),
        "the spawn is derived from the world and not committed: the horizontal centre of \
         column {SPAWN_COLUMN:?}, {SPAWN_ABOVE_SURFACE} blocks above that column's own \
         surface height of {surface}, facing the landmark at {SPAWN_YAW_DEGREES} degrees, \
         level and at rest. These rows say otherwise: {wrong:?}"
    );
    Ok(())
}

#[test]
fn thirty_ticks_of_the_replay_settle_the_player_onto_the_column_beneath_it() -> TestResult {
    let (world, mut simulation) = replay()?;
    let script = declared_script()?;

    let snapshots = published(&mut simulation, &script);

    let settled = at_tick(&snapshots, SETTLED_TICK)?.player;
    let (column_x, column_z) = (
        settled.position.x.floor() as u32,
        settled.position.z.floor() as u32,
    );
    let resting = (surface_height(&world, column_x, column_z)? + 1) as f32;
    assert!(
        (settled.position.y - resting).abs() <= EPSILON && settled.on_ground,
        "the world is the oracle and the physics is the subject: a surface height of \
         {} at column ({column_x}, {column_z}) holds the feet at {resting} with ground \
         contact by tick {SETTLED_TICK} — not at {} with contact {}",
        resting - 1.0,
        settled.position.y,
        settled.on_ground
    );
    Ok(())
}

#[test]
fn two_runs_of_the_same_intents_leave_the_player_identical_at_every_tick() -> TestResult {
    let (_world, mut once) = replay()?;
    let (_world, mut again) = replay()?;
    let script = declared_script()?;

    let first: Vec<_> = published(&mut once, &script)
        .iter()
        .map(|snapshot| exactly_player(&snapshot.player))
        .collect();
    let second: Vec<_> = published(&mut again, &script)
        .iter()
        .map(|snapshot| exactly_player(&snapshot.player))
        .collect();

    assert!(
        first.len() == SCRIPT_TICKS as usize + 1
            && first.iter().any(|state| Some(state) != first.first()),
        "both runs have to reach all {SCRIPT_TICKS} ticks and the player has to end up \
         somewhere other than where it started, or two runs that did nothing would agree \
         about having done nothing"
    );
    assert_eq!(
        first, second,
        "the same intents from the same spawn produce the same position, orientation, \
         velocity and ground contact at every tick — an accumulated state that drifted \
         between two runs of one machine is a golden frame nothing can be shot through"
    );
    Ok(())
}

#[test]
fn a_replay_that_asks_for_nothing_publishes_an_eye_that_never_moves_horizontally() -> TestResult {
    let (_world, mut simulation) = replay()?;

    let snapshots = published(&mut simulation, &asking_for_nothing());

    let spawned = at_tick(&snapshots, 0)?.camera.eye;
    let travelled: Vec<String> = snapshots
        .iter()
        .enumerate()
        .filter(|(_, snapshot)| horizontally_apart(snapshot.camera.eye, spawned) > EPSILON)
        .map(|(tick, snapshot)| format!("tick {tick} puts the eye at {:?}", snapshot.camera.eye))
        .collect();
    assert!(
        travelled.is_empty(),
        "the camera moves only because the player did, and a player asked for nothing goes \
         nowhere horizontally: every one of {SCRIPT_TICKS} ticks publishes an eye over \
         {spawned:?}. These do not, which is a camera on a path of its own: {travelled:?}"
    );
    Ok(())
}

#[test]
fn a_replay_that_asks_for_nothing_publishes_a_look_direction_that_never_changes() -> TestResult {
    let (_world, mut simulation) = replay()?;

    let snapshots = published(&mut simulation, &asking_for_nothing());

    let spawned = look_direction(&at_tick(&snapshots, 0)?.camera);
    let turned: Vec<String> = snapshots
        .iter()
        .enumerate()
        .filter(|(_, snapshot)| furthest_axis(look_direction(&snapshot.camera), spawned) > EPSILON)
        .map(|(tick, snapshot)| {
            format!(
                "tick {tick} looks along {:?}",
                look_direction(&snapshot.camera)
            )
        })
        .collect();
    assert!(
        turned.is_empty(),
        "a player asked for no look delta faces where it spawned, so every tick publishes a \
         camera looking along {spawned:?}. These do not, which is a camera aimed by \
         something other than the player: {turned:?}"
    );
    Ok(())
}

#[test]
fn the_scripted_replay_drops_the_eye_by_the_height_the_spawn_stands_above_the_ground() -> TestResult
{
    let (_world, mut simulation) = replay()?;
    let script = declared_script()?;

    let snapshots = published(&mut simulation, &script);

    let [_, spawned, _] = at_tick(&snapshots, 0)?.camera.eye;
    let [_, settled, _] = at_tick(&snapshots, SETTLED_TICK)?.camera.eye;
    assert!(
        (spawned - settled - SPAWN_DROP).abs() <= EPSILON,
        "the spawn stands {SPAWN_ABOVE_SURFACE} blocks above a surface that holds the feet \
         one block above it, so the eye that follows the player is exactly {SPAWN_DROP} \
         blocks lower once it has settled, whatever the terrain is: {spawned} to {settled} \
         is a drop of {}",
        spawned - settled
    );
    Ok(())
}

#[test]
fn the_scripted_turn_moves_the_published_yaw_by_exactly_the_angle_it_asks_for() -> TestResult {
    let (_world, mut simulation) = replay()?;
    let script = declared_script()?;

    let snapshots = published(&mut simulation, &script);

    let before = at_tick(&snapshots, TURN_FROM_TICK)?.player.yaw;
    let after = at_tick(&snapshots, TURN_TO_TICK)?.player.yaw;
    assert!(
        (after - before - TURN_DEGREES.to_radians()).abs() <= EPSILON,
        "the script asks for one degree of turn on each of the ticks between \
         {TURN_FROM_TICK} and {TURN_TO_TICK}, and nothing obstructs looking, so the yaw at \
         {TURN_TO_TICK} is exactly {TURN_DEGREES} degrees past the yaw at {TURN_FROM_TICK}: \
         {before} to {after} is {} radians, where {} was asked for",
        after - before,
        TURN_DEGREES.to_radians()
    );
    Ok(())
}
