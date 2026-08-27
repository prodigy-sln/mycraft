//! What a block's declared ascent does to the tick a swimmer asks to rise in,
//! and what it does to every tick that is not that one.
//!
//! **Every figure is the specification's own absolute number, written as the
//! exact rational it rounds.** `−0.3333` is `−1/3` and `5.6667` is `17/3`; the
//! rounded forms are quoted in the failure messages so a reader can find the
//! scenario, and asserted against never, because at the declared epsilon the
//! rounding is a fifth of the tolerance. Nothing here is copied from a run.
//!
//! **The tolerance is bounded from both directions and was measured, not
//! loosened.** `GRAVITY × TICK_DURATION` rounds to exactly `0.5` in `f32`, so
//! the whole launch path — subtract, then divide by `1 + resistance` — carries
//! **no** error at all against these rationals; and the nearest answer a
//! plausibly wrong implementation reaches is at least `0.6` blocks per second
//! away (`2.0` against `0.6667` for the lesser of a pair, against `5.6667` for
//! an ignored ascent, against `5 999.6667` for an unmasked fold). The declared
//! `1e-4` therefore sits eight orders above the measured error and four below
//! the smallest difference that must still redden.
//!
//! **Two rows, and which one a block is in matters.** The box is `1.8` blocks
//! tall with its feet at `y = 8.0`, so it stands in rows `8` and `9` and in no
//! others. That the fold reaches the *upper* row at all is not restated here:
//! `player_resistance.rs`'s box half in a resistance and half in nothing already
//! reports it, over these very fixtures, and a second copy would be agreement
//! between two copies of one reading. What is stated here is that the *ascent*
//! fold reaches both — which is the pair of orderings the greater-of-two
//! scenario asserts, and what the empty-cell and non-lifting-cell scenarios
//! below then rest on.
//!
//! **Every reading is taken off [`PlayerState::velocity`]** — the vertical
//! quantity a caller can read back — except the one scenario that is about a
//! *displacement*, where the whole claim is that the tick's own bound overrode
//! the velocity.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState, advance_player};

use support::TestResult;
use support::medium::{
    ABSURD_ASCENT, EXTENT, FEET, FLOOR_TOP, HOLDING_DEPTH, HOLDS_NOBODY_UP, LESSER_ASCENT, LIFTING,
    LIFTING_ABSURDLY, LIFTING_ASCENT, LIFTING_BY_DEFAULT, LIFTING_LESS, LIFTING_NOT_AT_ALL,
    LIFTING_RESISTANCE, flooded, hollow, resolved,
};
use support::volume::{AN_UNSTATED_ASCENT, Cells};

/// How far two figures this feature calls equal may differ, in blocks or in
/// blocks per second. The specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// How fast the player's own jump leaves whatever launched it, in blocks per
/// second. Declared.
const JUMP_SPEED: f32 = 9.0;

/// How far a tick may displace the player on any one axis, in blocks. Declared.
const DISPLACEMENT_LIMIT: f32 = 1.0;

/// What one tick of gravity takes from a launch before anything reads it, in
/// blocks per second.
const ONE_TICK_OF_GRAVITY: f32 = GRAVITY * TICK_DURATION;

/// The speed a swimmer held up by a block declaring [`LIFTING_ASCENT`] under
/// [`LIFTING_RESISTANCE`] ends the tick at: `(3.5 − 0.5) / 1.5`, which the
/// specification states as `2.0`.
const LIFTED: f32 = 2.0;

/// What the *lesser* of the declared pair would give on its own:
/// `(1.5 − 0.5) / 1.5`. Never an expectation — the distance between it and
/// [`LIFTED`] is what makes the greater-of-two claim falsifiable.
const LIFTED_LESS: f32 = 2.0 / 3.0;

/// The speed a swimmer ends the tick at where the declared ascent is exactly
/// what gravity takes back: `(0.5 − 0.5) / 1.0`.
const HOLDING: f32 = 0.0;

/// The speed a sink is arrested to where the declared ascent is zero:
/// `(0.0 − 0.5) / 1.5`, which the specification states as `−0.3333`.
const ARRESTED: f32 = -1.0 / 3.0;

/// The speed the same tick ends at when it asks for no jump at all and begins
/// [`SINKING_AT`]: `(−1.0 − 0.5) / 1.5`.
const STILL_SINKING: f32 = -1.0;

/// The speed a tick ends at where nothing holds the player up and nothing
/// resists it: `(0.0 − 0.5) / 1.0`.
const UNLIFTED: f32 = -0.5;

/// The speed the player's own jump leaves a tick at under
/// [`LIFTING_RESISTANCE`]: `(9.0 − 0.5) / 1.5`, which the specification states
/// as `5.6667`.
const A_JUMP_OF_ITS_OWN: f32 = 17.0 / 3.0;

/// The downward speed the arrest scenario begins its tick at.
const SINKING_AT: f32 = -1.0;

/// The two voxel rows the player's box stands in at [`FEET`], and no others: the
/// box is `1.8` blocks tall with its feet on the row boundary at `y = 8.0`.
const LOWER_ROW: u32 = FLOOR_TOP;
const UPPER_ROW: u32 = FLOOR_TOP + 1;

/// A player at rest with nothing holding it up, at [`FEET`].
fn adrift() -> PlayerState {
    PlayerState {
        position: FEET,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// The same player already sinking at `speed` blocks per second.
fn sinking(speed: f32) -> PlayerState {
    PlayerState {
        velocity: Vec3::new(0.0, speed, 0.0),
        ..adrift()
    }
}

/// An intent that asks for a jump and nothing else.
fn jumping() -> MovementIntent {
    MovementIntent {
        jump: true,
        ..MovementIntent::default()
    }
}

/// Where one submission of `intent` leaves `state`, in the world `volume`
/// declares.
fn one_tick(
    volume: &Cells,
    state: PlayerState,
    intent: &MovementIntent,
) -> Result<PlayerState, Box<dyn Error>> {
    Ok(advance_player(state, intent, &resolved(volume)?))
}

/// The vertical velocity a jump asked for from `state` leaves the tick at, in a
/// volume with nothing solid anywhere and `block` filling every row of it.
fn rise_through(block: &str, state: PlayerState) -> Result<f32, Box<dyn Error>> {
    Ok(one_tick(&submerged_in(block)?, state, &jumping())?
        .velocity
        .y)
}

/// A volume with nothing solid anywhere, `block` filling every row of it.
fn submerged_in(block: &str) -> Result<Cells, Box<dyn Error>> {
    flooded(hollow(), 0, EXTENT.y, block)
}

/// A volume with nothing solid anywhere, `lower` filling the row the feet stand
/// in and nothing at all in the row above it.
fn only_the_lower_row(lower: &str) -> Result<Cells, Box<dyn Error>> {
    flooded(hollow(), LOWER_ROW, UPPER_ROW, lower)
}

/// A volume with nothing solid anywhere, `lower` filling the row the feet stand
/// in and `upper` the row above it.
fn a_row_each(lower: &str, upper: &str) -> Result<Cells, Box<dyn Error>> {
    flooded(only_the_lower_row(lower)?, UPPER_ROW, UPPER_ROW + 1, upper)
}

/// Whether two figures this feature calls equal are within the declared epsilon.
fn near(measured: f32, declared: f32) -> bool {
    (measured - declared).abs() <= EPSILON
}

#[test]
fn a_jump_inside_a_block_that_declares_a_lift_leaves_at_the_lift_the_block_declares() -> TestResult
{
    let risen = rise_through(LIFTING, adrift())?;

    assert!(
        near(risen, LIFTED),
        "a swimmer's rise is set from the block's declared {LIFTING_ASCENT} and not from the \
         player's own {JUMP_SPEED}, so the tick ends at {LIFTED} — the specification's 2.0 blocks \
         per second — rather than at the {A_JUMP_OF_ITS_OWN} a jump of its own would leave under \
         the same resistance of {LIFTING_RESISTANCE}. Measured {risen}"
    );
    Ok(())
}

#[test]
fn a_declared_lift_of_exactly_one_tick_of_gravity_holds_a_swimmer_at_its_depth() -> TestResult {
    let risen = rise_through(HOLDING_DEPTH, adrift())?;

    assert!(
        near(risen, HOLDING),
        "a declared ascent is a launch speed that gravity bites before anything reads it, so an \
         ascent of exactly the {ONE_TICK_OF_GRAVITY} one tick takes leaves the swimmer neither \
         rising nor sinking: {HOLDING} rather than {risen}"
    );
    Ok(())
}

#[test]
fn a_declared_lift_of_zero_arrests_a_sink_without_reversing_it() -> TestResult {
    let volume = submerged_in(LIFTING_NOT_AT_ALL)?;
    let start = sinking(SINKING_AT);

    let asked = one_tick(&volume, start, &jumping())?.velocity.y;
    let unasked = one_tick(&volume, start, &MovementIntent::default())?
        .velocity
        .y;

    assert!(
        near(unasked, STILL_SINKING),
        "the control: this is a tick that asked for nothing, and it goes on sinking at \
         {STILL_SINKING} — so what follows is a claim about the *request* rather than about a \
         fixture in which nothing was falling to begin with. Measured {unasked}"
    );
    assert!(
        near(asked, ARRESTED),
        "an ascent of zero is a launch speed of zero and not the absence of a launch, so the \
         request replaces the {SINKING_AT} the tick began at and leaves it sinking at {ARRESTED} \
         — the specification's −0.3333 — rather than reversing it or leaving the sink alone at \
         {unasked}. Measured {asked}"
    );
    Ok(())
}

#[test]
fn a_declared_lift_carries_nobody_the_volume_does_not_hold_up() -> TestResult {
    let refused = rise_through(HOLDS_NOBODY_UP, adrift())?;
    let honoured = rise_through(LIFTING_ABSURDLY, adrift())?;

    assert!(
        honoured > refused,
        "the control: the two blocks declare the same {ABSURD_ASCENT} and differ only in whether \
         a player can hold itself up in them, and the buoyant one does lift — {honoured} against \
         {refused} — so what follows is a claim about buoyancy rather than about an ascent no \
         fixture could ever have delivered"
    );
    assert!(
        near(refused, UNLIFTED),
        "a declared ascent lifts nobody the volume does not hold up, however large it is: a jump \
         asked for inside a block declaring {ABSURD_ASCENT} and no buoyancy is refused exactly as \
         it is in open air, leaving the tick at {UNLIFTED} rather than at {refused}"
    );
    Ok(())
}

#[test]
fn a_box_across_two_declared_lifts_rises_at_the_greater_of_them() -> TestResult {
    let greater_below = one_tick(&a_row_each(LIFTING, LIFTING_LESS)?, adrift(), &jumping())?;
    let greater_above = one_tick(&a_row_each(LIFTING_LESS, LIFTING)?, adrift(), &jumping())?;

    assert!(
        near(greater_below.velocity.y, LIFTED) && near(greater_above.velocity.y, LIFTED),
        "the greatest declared ascent among the cells the box covers decides, matching the rule \
         an overlapped pair's resistance already follows — so a box across {LIFTING_ASCENT} and \
         {LESSER_ASCENT} rises at {LIFTED} whichever row the greater one is in, rather than at \
         the {LIFTED_LESS} the lesser gives, their mean, or whichever cell the fold reached \
         first. Measured {} with the greater below and {} with it above",
        greater_below.velocity.y,
        greater_above.velocity.y
    );
    Ok(())
}

#[test]
fn a_sink_is_governed_by_resistance_alone_whatever_lift_the_block_declares() -> TestResult {
    let more = submerged_in(LIFTING)?;
    let less = submerged_in(LIFTING_LESS)?;

    let sinking = (
        one_tick(&more, adrift(), &MovementIntent::default())?
            .velocity
            .y,
        one_tick(&less, adrift(), &MovementIntent::default())?
            .velocity
            .y,
    );
    let rising = (
        one_tick(&more, adrift(), &jumping())?.velocity.y,
        one_tick(&less, adrift(), &jumping())?.velocity.y,
    );

    assert_ne!(
        rising.0.to_bits(),
        rising.1.to_bits(),
        "the control: these two blocks declare {LIFTING_ASCENT} and {LESSER_ASCENT} and agree on \
         everything else, and a tick that asks to rise tells them apart — {rising:?} — so what \
         follows is a claim about the sink rather than about a pair a fixture made \
         indistinguishable"
    );
    assert_eq!(
        sinking.0.to_bits(),
        sinking.1.to_bits(),
        "a declared ascent governs the tick that asks to rise and no other, so two blocks \
         differing only in it sink a player at the same speed in every bit — the divisor is the \
         resistance they share, and it is the same division. {sinking:?}"
    );
    Ok(())
}

#[test]
fn a_lift_past_what_one_tick_may_spend_raises_the_feet_by_the_ticks_own_bound() -> TestResult {
    let start = adrift();

    let risen = one_tick(&submerged_in(LIFTING_ABSURDLY)?, start, &jumping())?;
    let travelled = risen.position.y - start.position.y;

    assert!(
        near(travelled, DISPLACEMENT_LIMIT),
        "a tick displaces the player by at most {DISPLACEMENT_LIMIT} block on an axis, and that \
         bound is the whole of the upward guard — so an ascent of {ABSURD_ASCENT} through a \
         volume with nothing standing above it raises the feet by exactly that block rather than \
         by the {} blocks the declared ascent asks for. Measured {travelled}",
        (ABSURD_ASCENT - ONE_TICK_OF_GRAVITY) * TICK_DURATION
    );
    Ok(())
}

#[test]
fn an_empty_cell_beside_a_lifting_one_contributes_no_lift_of_its_own() -> TestResult {
    let risen = one_tick(&only_the_lower_row(LIFTING)?, adrift(), &jumping())?;

    assert!(
        near(risen.velocity.y, LIFTED),
        "a cell holding no block contributes the inert ascent and never the value an absent \
         declaration field means, so a box half in a block declaring {LIFTING_ASCENT} and half in \
         nothing at all rises at {LIFTED} — neither diluted toward zero, nor lifted to the \
         {A_JUMP_OF_ITS_OWN} an empty cell reading as {AN_UNSTATED_ASCENT} would give. Measured {}",
        risen.velocity.y
    );
    Ok(())
}

#[test]
fn a_cell_nobody_can_be_held_up_in_contributes_no_lift_to_one_that_holds_a_swimmer() -> TestResult {
    let risen = one_tick(&a_row_each(LIFTING, HOLDS_NOBODY_UP)?, adrift(), &jumping())?;

    assert!(
        near(risen.velocity.y, LIFTED),
        "a volume holding nobody up contributes no lift to a volume that does, so water sharing \
         the box with a plant declaring {ABSURD_ASCENT} and no buoyancy rises the swimmer at the \
         water's own {LIFTED} rather than at the {} an unmasked fold reaches. Measured {}",
        (ABSURD_ASCENT - ONE_TICK_OF_GRAVITY) / (1.0 + LIFTING_RESISTANCE),
        risen.velocity.y
    );
    Ok(())
}

#[test]
fn a_block_that_states_no_lift_at_all_carries_a_swimmer_at_the_speed_its_own_jump_would()
-> TestResult {
    let swimming = rise_through(LIFTING_BY_DEFAULT, adrift())?;
    let standing = one_tick(
        &submerged_in(LIFTING_BY_DEFAULT)?,
        PlayerState {
            on_ground: true,
            ..adrift()
        },
        &jumping(),
    )?;

    assert!(
        near(swimming, A_JUMP_OF_ITS_OWN),
        "a declaration silent about its ascent lifts a swimmer by what the player's own jump \
         does, so a swimmer under a resistance of {LIFTING_RESISTANCE} ends the tick at \
         {A_JUMP_OF_ITS_OWN} — the specification's 5.6667 blocks per second. Measured {swimming}"
    );
    assert_eq!(
        swimming.to_bits(),
        standing.velocity.y.to_bits(),
        "and it is that speed in every bit rather than nearly it: the value an absent ascent \
         means and the player's own jump speed are one number, which is what this asserts and \
         what no prose carrying it by hand could. {swimming} against {}",
        standing.velocity.y
    );
    Ok(())
}
