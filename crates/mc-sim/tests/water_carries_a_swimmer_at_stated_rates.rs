//! What the shipped water does to a player, in absolute blocks and absolute
//! ticks.
//!
//! **Not one threshold here is arithmetic over a declared value, and that
//! reversal is the whole point of this file.** Every other reading of water's
//! declaration in this suite scales with whatever content declares, which is
//! what stops a scenario caging a number it should leave free — and the price is
//! that all of them move with it and none can report that it changed. Play has
//! now judged those numbers, so the displacements below are stated outright.
//! `the_shipped_sea_can_be_swum_in.rs` keeps its derived bounds and keeps its
//! value as a set of sign tests; what it must not be counted as is evidence that
//! these values landed.
//!
//! **The numbers are closed forms, never figures read off a run.** A sink is a
//! geometric sum and not a terminal speed — the shortcut that reads it as a
//! terminal speed gives `1.0` and is wrong by 3.4%, thirty-three times the
//! tolerance below. A rise and a walk are exact in real arithmetic, because the
//! velocity is *set* each tick rather than accumulated, and only the position
//! sums.
//!
//! **Every fixture starts the player moving.** A swimmer at rest cannot tell a
//! launch that replaced its velocity from one that was skipped over a zero, and
//! a walker at rest cannot tell a velocity that was set from one that was kept —
//! `crates/mc-sim/CLAUDE.md` records both, measured. The one exception is the
//! sink, whose closed form is the one for a fall beginning at zero.

mod support;

use mc_sim::player::{MovementIntent, PlayerState, advance_player};

use support::TestResult;
use support::pool::{Pool, a_pool_of_the_shipped_water};

/// How many ticks a second of simulated time is.
const A_SECOND: u32 = 60;

/// How far a swimmer that asks for nothing sinks in a second, in blocks.
///
/// **A sum and not a terminal speed.** Sinking from rest approaches `1.0` blocks
/// per second geometrically at ratio `2/3`, so a second covers
/// `(1/60)·[60 − 2(1 − (2/3)^60)]` = `0.966667`. The `(2/3)^60` term is `2.7e-11`
/// and vanishes; the `−2` does not, and dropping it is what gives the wrong
/// `1.0`.
const SINK_IN_A_SECOND: f32 = 0.9667;

/// How far a held jump raises a swimmer's feet in a second, in blocks.
///
/// The rise is a raw launch re-applied every tick and never accumulates, so it
/// is `(ascent − GRAVITY · TICK_DURATION) / (1 + resistance)` from the first tick
/// onward and a second covers exactly that many blocks.
const RISE_IN_A_SECOND: f32 = 2.0;

/// How far a full-deflection walk carries a submerged player in a second, in
/// blocks.
///
/// Horizontal motion is a velocity *target*: the walk speed divided by
/// `1 + resistance`, set fresh each tick, so a second covers it exactly.
const CARRIED_IN_A_SECOND: f32 = 3.0;

/// How far a measured displacement may sit from the stated one, in blocks.
///
/// **A ceiling derived from both directions rather than a figure loosened until
/// something passed.**
///
/// From below: a position accumulates sixty `f32` additions, and the sum is what
/// drifts. Transcribed independently and compared against the same recurrence in
/// `f64`, the three displacements below are out by `9.0e-6`, `2.7e-5` and
/// `1.1e-5` respectively — bounded above by sixty half-ulps at the magnitudes
/// these fixtures stand at, which is `3.0e-5` near 16 and `6.0e-5` near 32. This
/// sits a factor of thirty-seven above the largest of them.
///
/// From above: the nearest wrong answer any of the three has to reject is the
/// terminal-velocity reading of the sink, `1.0` against `0.9667` — a gap of
/// `0.0333`, thirty-three times this. Every other wrong answer in reach is
/// further off: the rise at the loader's default ascent is `5.667` blocks, the
/// rise at the resistance this feature replaces is `1.154`, and the walk at that
/// resistance is `1.731`.
const TOLERANCE: f32 = 1e-3;

/// An intent that asks to jump and for nothing else.
fn holding_jump() -> MovementIntent {
    MovementIntent {
        jump: true,
        ..MovementIntent::default()
    }
}

/// An intent asking for a walk at full deflection and nothing else, which at yaw
/// 0 is a walk along +x.
fn walking_forward() -> MovementIntent {
    MovementIntent {
        forward: 1.0,
        ..MovementIntent::default()
    }
}

/// Where `ticks` ticks of one intent leave the player.
fn advanced(from: PlayerState, intent: &MovementIntent, pool: &Pool, ticks: u32) -> PlayerState {
    (0..ticks).fold(from, |state, _| advance_player(state, intent, &pool.voxels))
}

#[test]
fn a_swimmer_asking_for_nothing_sinks_a_stated_distance_in_a_second() -> TestResult {
    let pool = a_pool_of_the_shipped_water()?;
    let began = pool.afloat_at_rest()?;
    let ended = advanced(began, &MovementIntent::default(), &pool, A_SECOND);
    pool.require_still_swimming(ended)?;

    let sank = began.position.y - ended.position.y;
    assert!(
        (sank - SINK_IN_A_SECOND).abs() <= TOLERANCE,
        "a player at rest and clear of the floor has to sink {SINK_IN_A_SECOND} blocks in \
         {A_SECOND} ticks of asking for nothing, within {TOLERANCE}, and this one sank {sank}. \
         The figure is a geometric sum and not a terminal speed: a reading of 1.0 means the \
         sum was replaced by the speed it approaches, and a reading near 0.31 means the \
         resistance is still the one this feature replaces"
    );
    Ok(())
}

#[test]
fn a_swimmer_holding_jump_rises_a_stated_distance_in_a_second() -> TestResult {
    let pool = a_pool_of_the_shipped_water()?;
    let began = pool.afloat_and_sinking()?;
    let ended = advanced(began, &holding_jump(), &pool, A_SECOND);
    pool.require_still_swimming(ended)?;

    let rose = ended.position.y - began.position.y;
    assert!(
        (rose - RISE_IN_A_SECOND).abs() <= TOLERANCE,
        "a submerged player clear of the ceiling has to rise {RISE_IN_A_SECOND} blocks in \
         {A_SECOND} ticks of held jump, within {TOLERANCE}, and this one rose {rose}. It began \
         the run already sinking, so a rise that carried that velocity forward instead of \
         replacing it lands below where it started; a reading near 5.67 means the block's \
         declared lift never reached the launch and the loader's default did"
    );
    Ok(())
}

#[test]
fn a_submerged_walk_at_full_deflection_carries_a_stated_distance_in_a_second() -> TestResult {
    let pool = a_pool_of_the_shipped_water()?;
    let began = pool.standing_on_the_floor_drifting_backwards()?;
    let ended = advanced(began, &walking_forward(), &pool, A_SECOND);
    pool.require_still_swimming(ended)?;

    let carried = ended.position.x - began.position.x;
    assert!(
        (carried - CARRIED_IN_A_SECOND).abs() <= TOLERANCE,
        "a submerged player walking at full deflection has to be carried \
         {CARRIED_IN_A_SECOND} blocks in {A_SECOND} ticks, within {TOLERANCE}, and this one \
         was carried {carried}. It began the run drifting the other way, so a walk that kept \
         the velocity it was handed rather than setting it lands three blocks behind; a \
         reading near 1.73 means the resistance is still the one this feature replaces"
    );
    Ok(())
}
