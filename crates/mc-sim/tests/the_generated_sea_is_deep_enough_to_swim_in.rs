//! The sea a player actually meets: how deep it is, how long crossing it takes,
//! and that surfacing is surfacing rather than being thrown out of it.
//!
//! **A chamber says nothing about this and this says nothing about a chamber.**
//! `water_carries_a_swimmer_at_stated_rates.rs` measures rates in water declared
//! deep enough for a rate to happen in; the generated sea is two voxels deep,
//! which is less water than one second of rise crosses, so no rate can be read
//! off it. What only this file can say is that the sea the world generates is
//! deep enough, and crossable, at all.
//!
//! **Every bound here is absolute, and every watch is derived.** A watch that
//! scales with the declared resistance is the correct watch — it is what makes a
//! sink that breaches its budget arrive as the assertion that cares about it
//! rather than as a fall that never landed. A *threshold* that scaled with it
//! could never report that it moved, which is exactly what these scenarios exist
//! to report.
//!
//! **The crossing band is two-sided because a one-sided budget cannot see the
//! feature missing.** With no declared lift a swimmer rises at the player's own
//! jump speed and crosses the first voxel in nineteen ticks, comfortably inside
//! any upper-only budget. The band's floor is what reports that.

mod support;

use mc_sim::player::{PlayerState, advance_player};

use support::sea::{
    SEA_TOP_FACE, Sea, TOP_WATER_VOXEL, deepest_sea_column, holding_jump, require_resting_at,
    rested, the_shipped_sea, watch_for,
};
use support::{NOTHING, TestResult, WATER, block_at, content_registry, replay_world};

/// The height the deepest column's lakebed presents its top face at, where a
/// player standing on it rests its feet.
const LAKEBED_TOP_FACE: f32 = 33.0;

/// How many cells over that lakebed are read, so that the water's depth is
/// bounded from above as well as below.
///
/// Three and not two: two would say the sea is *at least* two voxels deep, which
/// a sea of any greater depth also satisfies. The third cell is the control that
/// makes "exactly two" a claim.
const CELLS_READ_OVER_THE_LAKEBED: u32 = 3;

/// The earliest tick a swimmer may clear the first water voxel on.
const EARLIEST_CROSSING: usize = 25;

/// The latest tick it may clear it on.
const LATEST_CROSSING: usize = 45;

/// How long the crossing is watched for, in ticks.
///
/// Three times the latest tick the band admits, so a crossing that happens late
/// arrives as a tick number the assertion can name rather than as "never" —
/// derived from the scenario's own bound and from nothing the content declares.
const CROSSING_WATCH: u32 = 3 * LATEST_CROSSING as u32;

/// How long a player that stops asking to rise has to be back on the lakebed,
/// in ticks.
const SINK_BUDGET: u32 = 150;

/// How long the hold that must not expel a swimmer lasts, in ticks.
const HOLD_TICKS: u32 = 600;

/// The height every tick of that hold has to end below, in blocks.
///
/// A ceiling with margin rather than the apex. A swimmer stops being buoyant
/// once its feet clear the sea's top face and then coasts ballistically: one
/// tick's travel at the rise plus `v²/2g` puts the discrete arc near `35.08`, so
/// this bounds it without being satisfied by any rise that throws a player out
/// of the water. It tightens as the rise falls, which is the point.
const EXPULSION_CEILING: f32 = 35.1;

/// The state at the end of every tick of a hold, in order.
fn held(sea: &Sea, from: PlayerState, ticks: u32) -> Vec<PlayerState> {
    let mut state = from;
    (0..ticks)
        .map(|_| {
            state = advance_player(state, &holding_jump(), &sea.voxels);
            state
        })
        .collect()
}

/// The feet height at the end of every tick of a hold.
fn feet_of(hold: &[PlayerState]) -> Vec<f32> {
    hold.iter().map(|state| state.position.y).collect()
}

/// The greatest of a run of heights.
fn highest(feet: &[f32]) -> f32 {
    feet.iter().copied().fold(f32::NEG_INFINITY, f32::max)
}

#[test]
fn the_deepest_column_of_the_generated_sea_stands_two_water_voxels_over_its_lakebed() -> TestResult
{
    let world = replay_world(&content_registry()?)?;
    let deepest = deepest_sea_column(&world)?;
    let over_the_lakebed: Vec<String> = (1..=CELLS_READ_OVER_THE_LAKEBED)
        .map(|up| block_at(&world, deepest.x, deepest.surface + up, deepest.z))
        .collect::<Result<_, _>>()?;

    assert_eq!(
        (deepest.lakebed().to_bits(), over_the_lakebed),
        (
            LAKEBED_TOP_FACE.to_bits(),
            vec![WATER.to_owned(), WATER.to_owned(), NOTHING.to_owned()]
        ),
        "the deepest column of the generated sea has to put its lakebed's top face at \
         {LAKEBED_TOP_FACE} with exactly two water voxels over it. The cells are read whole and \
         in order from the lakebed up, so a shallower sea, a deeper one and a lakebed at another \
         height are three different failures — and the third cell is what makes it 'exactly \
         two' rather than 'at least two'. Column ({}, {}) was read",
        deepest.x,
        deepest.z
    );
    Ok(())
}

#[test]
fn a_swimmer_holding_jump_off_the_lakebed_clears_the_first_water_voxel_inside_a_stated_band()
-> TestResult {
    let sea = the_shipped_sea()?;
    let feet = feet_of(&held(&sea, sea.settled_on_the_lakebed()?, CROSSING_WATCH));
    let crossed = feet
        .iter()
        .position(|&at| at >= TOP_WATER_VOXEL)
        .map(|at| at + 1);

    assert!(
        matches!(crossed, Some(tick) if (EARLIEST_CROSSING..=LATEST_CROSSING).contains(&tick)),
        "a player holding jump from the lakebed of the sea's deepest column has to raise its \
         feet to {TOP_WATER_VOXEL} on a tick between the {EARLIEST_CROSSING}th and the \
         {LATEST_CROSSING}th, and it crossed on {crossed:?} within a watch of {CROSSING_WATCH} \
         reaching {}. The band is two-sided on purpose: a swimmer lifted at the player's own \
         jump speed crosses in nineteen ticks and satisfies every upper bound there is, so it \
         is the floor that reports the declared lift missing",
        highest(&feet)
    );
    Ok(())
}

#[test]
fn a_swimmer_that_stops_asking_to_rise_is_back_on_the_lakebed_inside_a_stated_budget() -> TestResult
{
    let sea = the_shipped_sea()?;
    let hold = held(&sea, sea.settled_on_the_lakebed()?, CROSSING_WATCH);
    let Some(&floating) = hold.last() else {
        return Err("a hold of no ticks leaves nobody floating".into());
    };
    if floating.position.y < TOP_WATER_VOXEL {
        return Err(format!(
            "the fixture: this is about a player floating at the surface, and after \
             {CROSSING_WATCH} ticks of held jump its feet are at {} rather than at or above \
             {TOP_WATER_VOXEL}. What sinks from there is not a player that was floating",
            floating.position.y
        )
        .into());
    }

    let sank = rested(floating, &sea.voxels, watch_for(sea.resistance))?;
    require_resting_at(sank.state, sea.deepest.lakebed(), "the lakebed it left")?;
    assert!(
        sank.tick <= SINK_BUDGET,
        "a player that stops asking to jump has to be back on the lakebed within \
         {SINK_BUDGET} ticks, and this one took {}. Two blocks of sink from rest is 122 ticks \
         in exact arithmetic and 123 as the tick path accumulates it, so this budget is blunt \
         by design — what says the sink rate is right is the second of chamber water, not this",
        sank.tick
    );
    Ok(())
}

#[test]
fn a_swimmer_holding_jump_for_ten_seconds_surfaces_rather_than_being_expelled() -> TestResult {
    let sea = the_shipped_sea()?;
    let feet = feet_of(&held(&sea, sea.settled_on_the_lakebed()?, HOLD_TICKS));
    let breached: Vec<(usize, f32)> = feet
        .iter()
        .enumerate()
        .filter(|(_, at)| **at >= EXPULSION_CEILING)
        .map(|(tick, at)| (tick + 1, *at))
        .collect();

    assert_eq!(
        (breached.len(), highest(&feet) >= SEA_TOP_FACE),
        (0, true),
        "every tick of a {HOLD_TICKS}-tick hold has to end with the feet below \
         {EXPULSION_CEILING}, and the highest of them has to reach the sea's own top face \
         {SEA_TOP_FACE} — without that control a ceiling is satisfied by a player that never \
         left the lakebed, and the ticks this ceiling is really about are the ones spent clear \
         of the water. The highest tick reached {} and the ticks past the ceiling are {:?}",
        highest(&feet),
        breached.first()
    );
    Ok(())
}
