//! Swimming in the sea the shipped world generates, asserted against the world
//! and the registry this repository actually ships.
//!
//! **No threshold here is a number somebody chose.** Water's declared resistance
//! is read back out of the shipped registry and never written down, and every
//! bound a scenario carries is arithmetic over it and over the declared physics
//! constants — so a value derived at implementation is *derived*, and a value
//! copied into a test would have nothing left to check it against. The two
//! constants a scenario states outright are the ones the specification states:
//! the `120` ticks the rise is given and the `600` ticks the hold lasts.
//!
//! **The three physics constants are mirrored here, and the mirror is
//! witnessed.** [`TICK_DURATION`], [`GRAVITY`] and [`JUMP_SPEED`] are private to
//! `crates/mc-sim/src/player/physics.rs`, as they are to the eight other test
//! binaries that mirror them. What makes this copy an instrument rather than a
//! second opinion is the last test in this file: one tick of a jump from dry
//! ground has to leave the player at exactly `JUMP_SPEED − GRAVITY ·
//! TICK_DURATION`, compared bit for bit, so any of the three moving in the engine
//! and not here reddens on the spot. [`WALK_SPEED`]'s witness is the shore
//! control inside the walking scenario.
//!
//! **The ceiling is an oracle and not a snapshot.** It is one line of arithmetic
//! over the declared constants and the declared resistance, sharing no code with
//! the simulation, and it is asserted over *every* tick of the hold — including
//! the ticks the box spends clear of the water, which is where the overshoot
//! happens, since a player leaves the water carrying the velocity the medium gave
//! it and coasts ballistically from there.
//!
//! **Every scenario whose wording an ordinary jump already satisfies carries the
//! reading that separates the two.** A jump off the lakebed reaches the surface
//! too, and comes straight back down; the assertions below therefore say where
//! the feet are for the *rest* of the hold, not only that they got there once.

mod support;

use mc_sim::player::{PlayerState, advance_player};

use support::TestResult;
use support::sea::{
    EPSILON, SEA_TOP_FACE, Sea, TOP_WATER_VOXEL, adrift, holding_jump, require_resting_at, rested,
    sink_budget, the_shipped_sea, walking_forward, watch_for,
};

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// How fast a jump leaves whatever launched it, in blocks per second. Declared.
const JUMP_SPEED: f32 = 9.0;

/// How fast a walk carries the player, in blocks per second. Declared.
const WALK_SPEED: f32 = 4.5;

/// How long FR-6.1-S2 gives the rise from the lakebed to the surface.
const RISE_TICKS: u32 = 120;

/// How long FR-6.1-S3 holds the jump for.
const HOLD_TICKS: u32 = 600;

/// Where FR-6.1-S5's fall into the sea begins.
const FALL_FROM: f32 = 44.0;

/// The height FR-6.1-S3 forbids every tick of the hold from reaching.
///
/// The specification's closed form, written over the declared constants rather
/// than over the sixties they happen to produce, so that it moves on its own the
/// day one of them moves. A player leaves the water at
/// `v = (JUMP_SPEED − GRAVITY · TICK_DURATION) / (1 + resistance)`, rises at most
/// `v · TICK_DURATION` on the tick that carries it out, and then coasts at most
/// `v² / (2 · GRAVITY)` further with nothing left to resist it.
///
/// **Arithmetic over declared constants and the declared resistance, sharing no
/// code with the simulation** — an oracle rather than a figure read off a run,
/// and it binds the resistance to nothing, where every constant ceiling would be
/// a hidden lower bound on the value the implementation is supposed to derive.
fn surfacing_ceiling(resistance: f32) -> f32 {
    let leaving = (JUMP_SPEED - GRAVITY * TICK_DURATION) / (1.0 + resistance);
    SEA_TOP_FACE + leaving * TICK_DURATION + leaving * leaving / (2.0 * GRAVITY)
}

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

/// Whether two figures this feature calls equal are within the declared epsilon.
fn near(measured: f32, declared: f32) -> bool {
    (measured - declared).abs() <= EPSILON
}

/// What one tick of a launch leaves the vertical velocity at: the declared jump
/// speed, less the bite gravity takes before the position moves.
fn after_one_launch() -> f32 {
    JUMP_SPEED - GRAVITY * TICK_DURATION
}

#[test]
fn holding_jump_on_the_deepest_lakebed_lifts_the_feet_to_the_surface_and_keeps_them_there()
-> TestResult {
    let sea = the_shipped_sea()?;
    let feet = feet_of(&held(&sea, sea.settled_on_the_lakebed()?, RISE_TICKS));
    let Some(surfaced) = feet.iter().position(|&at| at >= TOP_WATER_VOXEL) else {
        return Err(format!(
            "holding jump from the lakebed of the sea's deepest column has to raise the feet to \
             {TOP_WATER_VOXEL} within {RISE_TICKS} ticks, and in {RISE_TICKS} they reached only {}",
            highest(&feet)
        )
        .into());
    };

    let lowest_after = feet
        .iter()
        .skip(surfaced)
        .copied()
        .fold(f32::INFINITY, f32::min);
    assert!(
        lowest_after >= TOP_WATER_VOXEL,
        "the feet first reached {TOP_WATER_VOXEL} on tick {}, and the lowest tick of the rest of \
         the hold is {lowest_after}. A jump merely arced off the lakebed reaches the surface too \
         and comes straight back down, so where the feet are for the rest of the hold is what \
         separates a player being carried up from one that was thrown",
        surfaced + 1
    );
    Ok(())
}

#[test]
fn holding_jump_for_ten_seconds_floats_the_player_without_launching_it_clear_of_the_sea()
-> TestResult {
    let sea = the_shipped_sea()?;
    let hold = held(&sea, sea.settled_on_the_lakebed()?, HOLD_TICKS);
    let feet = feet_of(&hold);
    let ceiling = surfacing_ceiling(sea.resistance);

    assert!(
        highest(&feet) >= SEA_TOP_FACE,
        "the control, and without it a ceiling is satisfied by a player that never left the \
         lakebed: holding jump has to carry the feet to the sea's own top face {SEA_TOP_FACE} at \
         some tick of the hold — which is what puts the box clear of the water, where the ticks \
         this ceiling is really about happen — and the highest tick of {HOLD_TICKS} reached {}",
        highest(&feet)
    );
    let breached = feet
        .iter()
        .position(|&at| at >= ceiling)
        .map(|tick| (tick + 1, feet.get(tick).copied()));
    assert!(
        breached.is_none(),
        "every tick of a {HOLD_TICKS}-tick hold has to end with the feet below {ceiling}, which is \
         the sea's top face plus one tick's rise at the velocity the medium leaves a surfacing \
         player with plus the ballistic coast that velocity buys once nothing resists it. The \
         first tick past it, and the height it ended at, are {breached:?}"
    );
    Ok(())
}

#[test]
fn releasing_jump_at_the_surface_sinks_the_player_back_onto_the_lakebed_inside_its_budget()
-> TestResult {
    let sea = the_shipped_sea()?;
    let hold = held(&sea, sea.settled_on_the_lakebed()?, RISE_TICKS);
    let Some(&floating) = hold.last() else {
        return Err("a hold of no ticks leaves nobody floating".into());
    };

    assert!(
        floating.position.y >= TOP_WATER_VOXEL,
        "the fixture: this scenario is about a player *floating at the surface*, and jump is \
         released on tick {RISE_TICKS} because that is the tick by which FR-6.1-S2 requires it to \
         be there. After {RISE_TICKS} ticks of held jump the feet are at {} rather than at or \
         above {TOP_WATER_VOXEL}, so what sinks below is not a player that was floating",
        floating.position.y
    );
    let budget = sink_budget(sea.resistance);
    let sank = rested(floating, &sea.voxels, watch_for(sea.resistance))?;
    require_resting_at(sank.state, sea.deepest.lakebed(), "the lakebed it left")?;
    assert!(
        sank.tick as f32 <= budget,
        "a player that stops asking to jump has to be back on the lakebed within \
         {budget} ticks — one and a half times the `120 × depth × resistance` a sink through two \
         voxels of a medium resisting {} takes — and this one took {}",
        sea.resistance,
        sank.tick
    );
    Ok(())
}

#[test]
fn a_fall_into_the_sea_comes_to_rest_later_than_the_same_fall_through_a_sea_that_resists_nothing()
-> TestResult {
    let sea = the_shipped_sea()?;
    let watch = watch_for(sea.resistance);
    let dropped = adrift(sea.deepest.at(FALL_FROM));
    let through_the_sea = rested(dropped, &sea.voxels, watch)?;
    let through_nothing = rested(dropped, &sea.resisting_nothing()?, watch)?;

    require_resting_at(through_the_sea.state, sea.deepest.lakebed(), "the lakebed")?;
    require_resting_at(through_nothing.state, sea.deepest.lakebed(), "the lakebed")?;
    assert!(
        through_the_sea.tick > through_nothing.tick,
        "a fall from {FALL_FROM} onto the same lakebed of the same world takes {} ticks through \
         the shipped sea and {} through the same declarations with every medium taken out of \
         them. The two views differ in nothing else, so a sea that does not slow a fall is a sea \
         that resists nothing whatever its declaration says",
        through_the_sea.tick,
        through_nothing.tick
    );
    Ok(())
}

#[test]
fn a_full_deflection_walk_while_submerged_carries_less_than_the_same_walk_along_the_shore()
-> TestResult {
    let sea = the_shipped_sea()?;
    let swum = advance_player(
        sea.settled_on_the_lakebed()?,
        &walking_forward(),
        &sea.voxels,
    );
    let walked = advance_player(sea.shore_player()?, &walking_forward(), &sea.voxels);

    assert!(
        near(walked.velocity.x, WALK_SPEED),
        "the control: the shore walk this is measured against is the declared {WALK_SPEED} blocks \
         per second and not a figure read off a run. A reading of {} would satisfy 'less far than \
         the shore' for the one reason the comparison cannot see",
        walked.velocity.x
    );
    assert!(
        swum.velocity.x < walked.velocity.x,
        "a player whose box is inside the sea walks at {} blocks per second where the same \
         full-deflection walk along the dry shore carries {}. The velocity is read rather than \
         the distance, because recovering a displacement as one position minus another loses low \
         bits to the coordinate the subtraction is taken at",
        swum.velocity.x,
        walked.velocity.x
    );
    Ok(())
}

#[test]
fn one_tick_of_a_jump_from_the_dry_shore_is_the_arc_the_mirrored_constants_give() -> TestResult {
    let sea = the_shipped_sea()?;
    let ashore = sea.shore_player()?;
    let launched = advance_player(ashore, &holding_jump(), &sea.voxels);

    assert_eq!(
        launched.velocity.y.to_bits(),
        after_one_launch().to_bits(),
        "this file mirrors three constants private to the physics, and every threshold above is \
         arithmetic over them. This is what makes the copy an instrument: one tick of a jump from \
         dry ground leaves the vertical velocity at exactly the declared jump speed less one \
         tick's worth of gravity, which is {} — a reading of {} means one of the three has moved \
         in the engine and not here, and the ceiling above is then being derived from figures the \
         simulation no longer uses",
        after_one_launch(),
        launched.velocity.y
    );
    assert!(
        ashore.position.y >= SEA_TOP_FACE && ashore.on_ground,
        "the reading is taken from a player standing on dry ground at {}, clear of the sea's top \
         face {SEA_TOP_FACE}, so that nothing the medium does is in it",
        ashore.position.y
    );
    Ok(())
}
