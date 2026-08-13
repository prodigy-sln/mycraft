//! The declared per-tick intent script, and what it does at the end of itself.
//!
//! The script is what replaces the orbit: instead of a camera path the replay
//! reads off a function of the tick, the replay now *asks* for things — nothing
//! for half a second, then a walk, then a turn while walking, then a jump — and
//! whatever the simulation makes of that is what the goldens are shot from. So
//! what is asserted here is the declaration itself, tick by tick, and not what
//! the player does with it.
//!
//! **The table is asserted once, as a table** (`spec.md` §Table-driven
//! scenarios). Its rows are the ticks the declaration names — the first tick,
//! the first of each of the three intervals that follow, and the last — and they
//! are rows of one test rather than five tests, because what is being asserted is
//! one declaration rather than five behaviours.
//!
//! **Tick 91 is asked separately, and it is not a sixth row.** It is the tick
//! after the jump, and the thing it can catch is a jump that leaked past the one
//! tick that asks for it — an interval spelled `90..=91`, or a comparison that
//! should have been an equality. A row inside the same table would assert the
//! declaration; this asserts that the declaration ends where it says it does.
//!
//! **A tick past the script cannot be asked for at all**, because the script
//! takes a validated index rather than a number. That is the refusal being
//! asserted below: not that the script answers something safe for tick 120, but
//! that tick 120 cannot be constructed, so the question cannot reach it and
//! nothing wraps back into the script's beginning.
//!
//! Comparisons use the declared 1 × 10⁻⁴ epsilon.

mod support;

use mc_sim::player::MovementIntent;
use mc_sim::replay::{SCRIPT_TICKS, TickError, TickIndex, scripted_intent};

use support::TestResult;

/// How far two figures this feature calls equal may differ. The specification's
/// declared comparison epsilon, in radians here: a hundredth of the 0.0175
/// radians one scripted turn asks for, so a turn of half the declared size is
/// two orders of magnitude outside it.
const EPSILON: f32 = 1e-4;

/// How far the script turns on each of its turning ticks, in degrees.
const TURN_DEGREES: f32 = 1.0;

/// The tick the jump is asked for, and the one after it.
const JUMP_TICK: u32 = 90;
const AFTER_THE_JUMP: u32 = 91;

/// The last tick the script has an intent for.
const LAST_TICK: u32 = SCRIPT_TICKS - 1;

/// The ticks at or past the script's end that must be refused.
///
/// The first one past it, the second, and the largest tick a caller could name
/// at all — so a bound spelled `>` instead of `>=`, and one that only refused
/// the tick immediately after the end, are both caught.
const PAST_THE_SCRIPT: [u32; 3] = [SCRIPT_TICKS, SCRIPT_TICKS + 1, u32::MAX];

/// An intent that holds forward and asks for nothing else.
fn holding_forward() -> MovementIntent {
    MovementIntent {
        forward: 1.0,
        ..MovementIntent::default()
    }
}

/// The declaration, one row per tick the specification names.
///
/// Ticks 0–29 ask for nothing, 30–59 hold forward, 60–89 hold forward and turn
/// by [`TURN_DEGREES`] each tick, tick 90 holds forward and asks for a jump, and
/// 91–119 hold forward. The rows are the *first* tick of each interval and the
/// last tick of the script, which is where an interval starting or ending one
/// tick out shows up.
fn declared_script() -> [(u32, MovementIntent); 5] {
    [
        (0, MovementIntent::default()),
        (30, holding_forward()),
        (
            60,
            MovementIntent {
                yaw_delta: TURN_DEGREES.to_radians(),
                ..holding_forward()
            },
        ),
        (
            JUMP_TICK,
            MovementIntent {
                jump: true,
                ..holding_forward()
            },
        ),
        (LAST_TICK, holding_forward()),
    ]
}

/// How far apart two intents are, as the largest disagreement on any one field,
/// with a disagreement about the jump counting as a whole unit.
fn furthest_field(asked: &MovementIntent, declared: &MovementIntent) -> f32 {
    let apart = [
        asked.forward - declared.forward,
        asked.strafe - declared.strafe,
        asked.yaw_delta - declared.yaw_delta,
        asked.pitch_delta - declared.pitch_delta,
        f32::from(u8::from(asked.jump != declared.jump)),
    ];
    apart.iter().map(|apart| apart.abs()).fold(0.0, f32::max)
}

/// The intent the script asks for at `tick`.
///
/// # Errors
///
/// Returns the refusal when `tick` is past the script's end.
fn asked_at(tick: u32) -> Result<MovementIntent, TickError> {
    Ok(scripted_intent(TickIndex::new(tick)?))
}

#[test]
fn the_script_asks_at_each_declared_tick_for_what_the_declaration_names() -> TestResult {
    let mut wrong = Vec::new();

    for (tick, declared) in declared_script() {
        let asked = asked_at(tick)?;
        if furthest_field(&asked, &declared) > EPSILON {
            wrong.push(format!("tick {tick} asks {asked:?}, not {declared:?}"));
        }
    }

    assert!(
        wrong.is_empty(),
        "the script is a declaration and these are its rows: nothing until tick 30, forward \
         from 30, forward and {TURN_DEGREES} degree of turn from 60, forward and a jump at \
         {JUMP_TICK}, forward again through {LAST_TICK}. These rows ask for something else: \
         {wrong:?}"
    );
    Ok(())
}

#[test]
fn the_tick_after_the_jump_holds_forward_and_asks_for_no_second_jump() -> TestResult {
    let declared = holding_forward();

    let asked = asked_at(AFTER_THE_JUMP)?;

    assert!(
        furthest_field(&asked, &declared) <= EPSILON,
        "the jump is asked for at tick {JUMP_TICK} and at no other, so tick {AFTER_THE_JUMP} \
         holds forward and nothing else — {asked:?} instead of {declared:?} is an interval \
         that ran a tick long, or a walk that stopped when the jump did"
    );
    Ok(())
}

#[test]
fn a_tick_past_the_end_of_the_script_is_refused_rather_than_wrapping() -> TestResult {
    let last = asked_at(LAST_TICK);

    let accepted: Vec<u32> = PAST_THE_SCRIPT
        .into_iter()
        .filter(|tick| asked_at(*tick).is_ok())
        .collect();

    assert!(
        last.is_ok(),
        "the script's last tick has to be scriptable, or refusing everything past it would \
         be a script that refuses everything"
    );
    assert_eq!(
        (accepted, asked_at(SCRIPT_TICKS).err()),
        (
            Vec::new(),
            Some(TickError::BeyondReplay {
                tick: SCRIPT_TICKS,
                tick_count: SCRIPT_TICKS,
            })
        ),
        "the script is {SCRIPT_TICKS} ticks long and has no intent for a tick past that, so \
         asking for one is refused with the length named — never answered with the intent of \
         a tick inside the script, which is what a wrap or a modulus would hand back"
    );
    Ok(())
}
