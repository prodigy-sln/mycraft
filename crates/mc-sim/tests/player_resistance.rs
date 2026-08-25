//! What a block's declared resistance does to the tick that moves through its
//! volume: the divisor, what the divisor acts on, and which cells decide it.
//!
//! **Every ratio below is asserted on [`PlayerState::velocity`], never on where
//! the player ended minus where it began.** How far one tick carries the player
//! is the velocity times a tick duration common to both readings, so the ratio
//! on the velocity *is* the ratio on the distance — while recovering the
//! distance by subtracting two positions loses low bits to the coordinate the
//! subtraction is taken at. Measured over 64 integer columns, four fractional
//! offsets and two resistances: the ratio on the displacement mismatches 0 of
//! 512 samples and the same ratio recovered as `end − start` mismatches 425 of
//! 512, the first already at a start of `0.3`. **It holds at the origin and
//! almost nowhere else**, so the fixture that would pass is the naive one, and
//! [`FEET`] deliberately stands where the arithmetic is not flattered.
//!
//! **Two comparisons are exact and the rest carry an epsilon, and the split is
//! derived rather than chosen.** A resistance of `0.0` divides by `1.0`, which
//! is `v` itself in IEEE-754 for every finite `v` — so where the claim is
//! "exactly as far", bits are both the exact form of the question and the form
//! `clippy::float_cmp` has no quarrel with. Where the claim is a *ratio*, the
//! declared `1 × 10⁻⁴` epsilon is above the measured error of nought and far
//! below the nearest wrong answer: the divisors a fold could plausibly reach —
//! `1`, `2`, `4`, `5` — put the candidates at least `0.75` blocks per second
//! apart at this walk speed, so the tolerance is bounded from both directions
//! and was never loosened to reach green.
//!
//! **Every ratio is measured against a walk through air whose own value is
//! checked against the declared speed.** A ratio between two zeroes holds, so a
//! reading that came back stationary would satisfy "half as far" for the worst
//! possible reason; the control is what stops that, and it is arithmetic over
//! the declared constants rather than a figure read off a run.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState, advance_player};

use support::TestResult;
use support::medium::{
    AWKWARD, AWKWARD_RESISTANCE, CLEAR, CLINGING_STONE, EXTENT, FEET, FLOOR_TOP, PLAIN_STONE,
    SETTING, SETTING_RESISTANCE, THICK, THICK_RESISTANCE, THICKER, THICKER_RESISTANCE, flooded,
    floored, hollow, resolved, tall,
};
use support::volume::Cells;

/// How far two figures this feature calls equal may differ, in blocks or in
/// blocks per second. The specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast a walk carries the player, in blocks per second. Declared.
const WALK_SPEED: f32 = 4.5;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// How many ticks of unpowered fall the two speeds are compared after.
const FALLING_TICKS: u32 = 10;

/// The volume every unresisted reading is taken in: a floor, and nothing at all
/// over it.
fn plain_floor() -> Result<Cells, Box<dyn Error>> {
    floored(PLAIN_STONE)
}

/// The same floor, with `block` filling every row above it.
fn over_the_floor(block: &str) -> Result<Cells, Box<dyn Error>> {
    flooded(floored(PLAIN_STONE)?, FLOOR_TOP, EXTENT.y, block)
}

/// A volume with nothing solid anywhere, `block` filling every row of it.
fn falling_through(block: &str) -> Result<Cells, Box<dyn Error>> {
    flooded(hollow(), 0, EXTENT.y, block)
}

/// A player standing still on the floor at yaw 0.
fn standing() -> PlayerState {
    PlayerState {
        position: FEET,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    }
}

/// A player at rest with nothing holding it up.
fn airborne() -> PlayerState {
    PlayerState {
        on_ground: false,
        ..standing()
    }
}

/// A player at rest with nothing holding it up, standing at `at`.
fn adrift(at: Vec3) -> PlayerState {
    PlayerState {
        position: at,
        ..airborne()
    }
}

/// An intent asking for a walk at full deflection and nothing else, which at
/// yaw 0 is a walk along +x.
fn walking_forward() -> MovementIntent {
    MovementIntent {
        forward: 1.0,
        ..MovementIntent::default()
    }
}

/// Where `ticks` submissions of `intent` leave `state`, in the world `volume`
/// declares.
fn advance(
    volume: &Cells,
    state: PlayerState,
    intent: &MovementIntent,
    ticks: u32,
) -> Result<PlayerState, Box<dyn Error>> {
    let world = resolved(volume)?;
    Ok((0..ticks).fold(state, |state, _| advance_player(state, intent, &world)))
}

/// Where one tick of a full-deflection walk leaves a player standing in the
/// world `volume` declares.
fn walked(volume: &Cells) -> Result<PlayerState, Box<dyn Error>> {
    advance(volume, standing(), &walking_forward(), 1)
}

/// Whether two figures this feature calls equal are within the declared epsilon.
fn near(measured: f32, declared: f32) -> bool {
    (measured - declared).abs() <= EPSILON
}

/// A velocity as the integers its floats are. "Exactly as far" is a question
/// about bits, not about nearness.
fn exactly(velocity: Vec3) -> [u32; 3] {
    velocity.to_array().map(f32::to_bits)
}

/// The message every ratio's control carries.
fn unresisted(walk: f32) -> String {
    format!(
        "the unresisted walk every ratio here is measured against is the declared \
         {WALK_SPEED} blocks per second and not a figure read off a run — a reading of {walk} \
         would satisfy a ratio for the one reason a ratio cannot see"
    )
}

#[test]
fn a_walk_through_a_volume_resisting_as_much_as_it_carries_covers_half_the_ground() -> TestResult {
    let air = walked(&plain_floor()?)?;
    let thick = walked(&over_the_floor(THICK)?)?;

    assert!(
        near(air.velocity.x, WALK_SPEED),
        "{}",
        unresisted(air.velocity.x)
    );
    assert!(
        near(thick.velocity.x * (1.0 + THICK_RESISTANCE), air.velocity.x),
        "a block's volume divides what moves through it by 1 + its declared resistance, so one \
         tick of a full-deflection walk through a resistance of {THICK_RESISTANCE} carries half \
         of the {} blocks per second the same walk carries through air — {} rather than {}",
        air.velocity.x,
        thick.velocity.x,
        air.velocity.x / (1.0 + THICK_RESISTANCE)
    );
    Ok(())
}

#[test]
fn a_walk_through_blocks_that_state_no_resistance_covers_exactly_what_air_does() -> TestResult {
    let air = walked(&plain_floor()?)?;
    let clear = walked(&over_the_floor(CLEAR)?)?;
    let thick = walked(&over_the_floor(THICK)?)?;

    assert!(
        near(air.velocity.x, WALK_SPEED),
        "{}",
        unresisted(air.velocity.x)
    );
    assert!(
        near(thick.velocity.x * (1.0 + THICK_RESISTANCE), air.velocity.x),
        "the control: a block written over these very cells by this very route does slow this \
         walk, at {} against {} — so 'exactly as far as air' is a claim about a stated zero and \
         not about a fixture nothing could ever have resisted",
        thick.velocity.x,
        air.velocity.x
    );
    assert_eq!(
        exactly(clear.velocity),
        exactly(air.velocity),
        "a stated resistance of zero divides by one, which is the value itself in every bit, so a \
         walk through blocks declaring it is not nearly the walk through air but exactly it. The \
         cells the box covers hold a real block here rather than nothing, so a resistance read \
         off drawnness, off occlusion or off the absence of solidity reports itself: {:?} against \
         air's {:?}",
        clear.velocity,
        air.velocity
    );
    Ok(())
}

#[test]
fn a_fall_through_a_volume_resisting_as_much_as_it_carries_drops_half_as_fast() -> TestResult {
    let still = MovementIntent::default();
    let air = advance(&hollow(), airborne(), &still, 1)?;
    let thick = advance(&falling_through(THICK)?, airborne(), &still, 1)?;

    assert!(
        near(air.velocity.y, -GRAVITY * TICK_DURATION),
        "the unresisted fall this ratio is measured against is one tick of the declared gravity, \
         {} rather than {}",
        -GRAVITY * TICK_DURATION,
        air.velocity.y
    );
    assert!(
        near(thick.velocity.y * (1.0 + THICK_RESISTANCE), air.velocity.y),
        "the divisor acts on every axis alike, so an unpowered fall through a resistance of \
         {THICK_RESISTANCE} carries half as far in a tick as the same fall through air — {} \
         rather than {}",
        thick.velocity.y,
        air.velocity.y / (1.0 + THICK_RESISTANCE)
    );
    Ok(())
}

#[test]
fn ten_ticks_of_a_resisted_fall_are_still_going_slower_than_ten_ticks_through_air() -> TestResult {
    let still = MovementIntent::default();
    let air = advance(&hollow(), airborne(), &still, FALLING_TICKS)?;
    let thick = advance(&falling_through(THICK)?, airborne(), &still, FALLING_TICKS)?;

    assert!(
        near(
            air.velocity.y,
            -GRAVITY * TICK_DURATION * FALLING_TICKS as f32
        ),
        "the unresisted fall this is compared against accumulates one tick of gravity per tick, \
         so after {FALLING_TICKS} it is {} rather than {}",
        -GRAVITY * TICK_DURATION * FALLING_TICKS as f32,
        air.velocity.y
    );
    assert!(
        thick.velocity.y > air.velocity.y,
        "the divided velocity is what the tick carries forward and not merely what it displaces \
         by, so a fall in a resistant volume never builds the speed the same fall builds through \
         air: after {FALLING_TICKS} ticks it reports {} against air's {}. An implementation that \
         divided only the displacement reports the two as one figure",
        thick.velocity.y,
        air.velocity.y
    );
    Ok(())
}

#[test]
fn a_walk_over_a_resistant_floor_covers_exactly_what_a_walk_over_a_plain_one_does() -> TestResult {
    let plain = walked(&plain_floor()?)?;
    let clinging = walked(&floored(CLINGING_STONE)?)?;
    let steeped = walked(&over_the_floor(THICKER)?)?;

    assert!(
        near(
            steeped.velocity.x * (1.0 + THICKER_RESISTANCE),
            plain.velocity.x
        ),
        "the control: a resistance of {THICKER_RESISTANCE} in the cells this very box does cover \
         is visible from this very position, at {} against {}",
        steeped.velocity.x,
        plain.velocity.x
    );
    assert_eq!(
        exactly(clinging.velocity),
        exactly(plain.velocity),
        "a block resists what moves through its volume and nothing else. A player standing on a \
         floor overlaps only the empty cells above it, so the floor's own resistance is not in \
         the fold at all — a fold that lowered the box first, the way ground contact does, would \
         report the control's {} instead of {}",
        steeped.velocity.x,
        plain.velocity.x
    );
    Ok(())
}

#[test]
fn a_walk_through_a_resistance_beyond_any_scale_the_engine_moves_at_still_goes_forward()
-> TestResult {
    let start = standing();
    let air = walked(&plain_floor()?)?;
    let settled = walked(&over_the_floor(SETTING)?)?;

    assert!(
        settled.velocity.x < air.velocity.x,
        "the control: the resistance is in the fold at all, so this walk is slower than air's {} \
         rather than untouched at {}",
        air.velocity.x,
        settled.velocity.x
    );
    assert!(
        settled.position.is_finite() && settled.position.x >= start.position.x,
        "a declared resistance of {SETTING_RESISTANCE} divides rather than annihilates, so the \
         tick ends at a position that is a number and no further back along the walk than it \
         began: {} from {}",
        settled.position.x,
        start.position.x
    );
    assert!(
        settled.velocity.is_finite() && settled.velocity.x > 0.0,
        "1 + r is at least 1 for every resistance content may declare, so the division can \
         neither make a number out of nothing, nor reverse a sign, nor leave a walk running \
         backwards: {:?}",
        settled.velocity
    );
    Ok(())
}

#[test]
fn a_box_across_two_resistances_is_slowed_by_the_greater_of_them() -> TestResult {
    let air = walked(&plain_floor()?)?;
    let straddling = flooded(over_the_floor(THICK)?, FLOOR_TOP + 1, EXTENT.y, THICKER)?;
    let mixed = walked(&straddling)?;

    assert!(
        near(air.velocity.x, WALK_SPEED),
        "{}",
        unresisted(air.velocity.x)
    );
    assert!(
        near(
            mixed.velocity.x * (1.0 + THICKER_RESISTANCE),
            air.velocity.x
        ),
        "the greatest resistance among the cells the box covers decides, so a box across a \
         resistance of {THICK_RESISTANCE} and one of {THICKER_RESISTANCE} is quartered rather \
         than halved, summed to a fifth, or settled by whichever cell the walk reached first — \
         the greater one is the upper row, which the enumeration reaches last. {} rather than {}",
        mixed.velocity.x,
        air.velocity.x / (1.0 + THICKER_RESISTANCE)
    );
    Ok(())
}

#[test]
fn a_box_half_in_a_resistance_and_half_in_nothing_is_slowed_by_the_resistance() -> TestResult {
    let air = walked(&plain_floor()?)?;
    let overhead = flooded(plain_floor()?, FLOOR_TOP + 1, EXTENT.y, THICKER)?;
    let half_in = walked(&overhead)?;

    assert!(
        near(air.velocity.x, WALK_SPEED),
        "{}",
        unresisted(air.velocity.x)
    );
    assert!(
        near(
            half_in.velocity.x * (1.0 + THICKER_RESISTANCE),
            air.velocity.x
        ),
        "a cell holding no block contributes nothing rather than something, so the box's lower \
         row — which holds nothing at all — neither dilutes the resistance of its upper row nor \
         settles the answer by being reached first: {} rather than {}",
        half_in.velocity.x,
        air.velocity.x / (1.0 + THICKER_RESISTANCE)
    );
    Ok(())
}

/// Where the feet stand for the reading taken wholly outside the world: the
/// whole box above the last row of a volume 256 blocks tall, so every cell it
/// covers is a position the walk never produces rather than a cell holding
/// nothing.
const ABOVE_THE_WORLD: Vec3 = Vec3::new(FEET.x, 300.0, FEET.z);

/// Where the feet stand for the reading taken inside the volume in cells holding
/// no block — far above the declared resistance and far below the roof.
const INSIDE_THE_EMPTY: Vec3 = Vec3::new(FEET.x, 200.0, FEET.z);

/// Where the feet stand for the control: inside the declared resistance near the
/// bottom of that same volume, read through that same view.
const INSIDE_THE_RESISTANCE: Vec3 = Vec3::new(FEET.x, 6.0, FEET.z);

#[test]
fn a_box_wholly_above_the_world_walks_exactly_as_far_as_one_in_the_empty_cells_inside_it()
-> TestResult {
    let volume = flooded(tall(), 0, FLOOR_TOP, THICKER)?;
    let forward = walking_forward();
    let outside = advance(&volume, adrift(ABOVE_THE_WORLD), &forward, 1)?;
    let inside = advance(&volume, adrift(INSIDE_THE_EMPTY), &forward, 1)?;
    let steeped = advance(&volume, adrift(INSIDE_THE_RESISTANCE), &forward, 1)?;

    assert!(
        near(
            steeped.velocity.x * (1.0 + THICKER_RESISTANCE),
            inside.velocity.x
        ),
        "the control: this same view does carry a resistance, at {} against the {} it carries \
         where the cells hold nothing — so 'outside answers nothing' is a claim about outside \
         rather than about a view that answers nothing everywhere",
        steeped.velocity.x,
        inside.velocity.x
    );
    assert_eq!(
        exactly(outside.velocity),
        exactly(inside.velocity),
        "everything outside the world's volume answers the same nothing an empty cell does, by \
         the same bounds test the other views use — so a box 44 rows above a world 256 blocks \
         tall walks not nearly but exactly as far as one inside it: {:?} against {:?}",
        outside.velocity,
        inside.velocity
    );
    Ok(())
}

#[test]
fn a_resistance_whose_divisor_is_not_a_power_of_two_divides_the_walk_rather_than_scaling_it()
-> TestResult {
    let awkward = walked(&over_the_floor(AWKWARD)?)?;

    assert_eq!(
        awkward.velocity.x.to_bits(),
        (WALK_SPEED / (1.0 + AWKWARD_RESISTANCE)).to_bits(),
        "the divisor is a division by `1 + resistance` and never a multiplication by its \
         reciprocal. The two agree wherever `1 + resistance` is a power of two, which every \
         other resistance in this file is, so this is the only reading that can tell them \
         apart — measured, they differ here by one unit in the last place, which is three \
         orders below the epsilon every other comparison uses and invisible to all of them. \
         {:?} rather than {:?}",
        awkward.velocity.x,
        WALK_SPEED / (1.0 + AWKWARD_RESISTANCE)
    );
    Ok(())
}
