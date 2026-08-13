//! The declared replay never puts the player inside the world.
//!
//! This is the one assertion in this feature that covers the *whole* run rather
//! than a declared fixture, and it is the only one that could catch a resolver
//! that is wrong somewhere nobody wrote a scenario about. It can do that because
//! it is judged by something that shares no code with the thing it judges: the
//! physics reads a bitset resolved once at construction, and
//! [`support::overlap`] re-reads the world's own block query and asks the
//! registry about every name it finds.
//!
//! **An invariant over a run is only as good as the run.** A replay whose player
//! never moved would satisfy this forever, and a judge that answered "clear" to
//! everything would too. The second is what the control below is for: the same
//! function, pointed at a box placed inside the landmark pillar, has to report
//! the overlap. The first is what the rest of this phase is for — the spawn, the
//! settle, the drop and the turn all say the player went somewhere, and the
//! specification records why a horizontal-displacement floor was rejected as a
//! control here: where a walk across generated terrain ends is not
//! hand-derivable, so a 2-block step early in the walk would stop the player
//! legitimately and fail an assertion that had no business being made.

mod support;

use std::sync::Arc;

use glam::Vec3;
use mc_sim::replay::{SCRIPT_TICKS, TickIndex, scripted_intent, simulation_for};

use support::overlap::overlapping_voxels;
use support::{LANDMARK, LANDMARK_TOP, TestResult, content_registry, replay_world};

/// How many published states the run is judged at: the spawn's own, and one per
/// scripted tick.
const JUDGED_STATES: usize = SCRIPT_TICKS as usize + 1;

/// Where a box placed one block inside the landmark pillar stands.
///
/// The pillar's stone runs up to and including voxel [`LANDMARK_TOP`], so its cap
/// face is one block higher; feet at the top voxel's own height put the box's
/// lowest block squarely inside that voxel, over the centre of the pillar's
/// column.
fn inside_the_pillar() -> Vec3 {
    let (column_x, column_z) = LANDMARK;
    Vec3::new(
        column_x as f32 + 0.5,
        LANDMARK_TOP as f32,
        column_z as f32 + 0.5,
    )
}

#[test]
fn no_tick_of_the_declared_replay_leaves_the_player_inside_a_solid_voxel() -> TestResult {
    // The simulation's world holds the registry for the life of the run, so it
    // is shared rather than borrowed. The oracle below still reads its own copy
    // of the handle and re-resolves every name it finds through it, which is the
    // whole point of this test — one registry, but two lookup chains, so an
    // adapter that resolved a name wrongly cannot make both sides wrong alike.
    let registry = Arc::new(content_registry()?);
    let world = replay_world(&registry)?;
    let mut simulation = simulation_for(&world, Arc::clone(&registry))?;
    let mut standing = vec![simulation.latest().player.position];
    let mut buried = Vec::new();

    for tick in 0..SCRIPT_TICKS {
        simulation.advance(scripted_intent(TickIndex::new(tick)?));
        standing.push(simulation.latest().player.position);
    }

    for (tick, feet) in standing.iter().enumerate() {
        let inside = overlapping_voxels(&world, &registry, *feet)?;
        if !inside.is_empty() {
            buried.push(format!("tick {tick} at {feet:?} is in {inside:?}"));
        }
    }
    assert!(
        standing.len() == JUDGED_STATES,
        "the judge has to have seen all {JUDGED_STATES} published states and it saw {}, so \
         an empty verdict below would be a verdict about nothing",
        standing.len()
    );
    assert!(
        buried.is_empty(),
        "a solid voxel occupies [v, v + 1) and the player's box is never inside one, judged \
         against the world's own blocks rather than against the physics that placed it. \
         These ticks put it inside: {buried:?}"
    );
    Ok(())
}

#[test]
fn the_same_judge_reports_a_box_placed_inside_the_landmark_pillar() -> TestResult {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    let (column_x, column_z) = LANDMARK;
    let cap = (column_x as i32, LANDMARK_TOP as i32, column_z as i32);

    let inside = overlapping_voxels(&world, &registry, inside_the_pillar())?;

    assert!(
        inside.iter().any(|overlap| overlap.voxel == cap),
        "the pillar's stone reaches voxel {cap:?}, and a box standing one block inside it \
         overlaps that voxel — a judge that reported {inside:?} instead would report a clear \
         box for a player buried in the world, and the invariant beside this one would be \
         green whatever the physics did"
    );
    Ok(())
}
