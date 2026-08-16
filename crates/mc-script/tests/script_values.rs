//! What a number is when it stops being script's and becomes the engine's.
//!
//! # One derivation, many consumers
//!
//! Every value that leaves script goes through one classification, and the
//! engine acts on the answer. A block's fuel value, a recipe's yield, a
//! callback's returned count — each arrives as a whole number or as a
//! fractional one, and nothing downstream re-checks. That makes this a shared
//! derivation with a great many dependents and, until this file, no assertion of
//! its own: it was exercised constantly and checked nowhere, which is the shape
//! where a defect is reported as several unrelated failures at once, or as none.
//!
//! # The defect this exists to catch, stated so it is not optimised away
//!
//! The classification is currently made one level down, by the backend, and
//! passed through. The tempting simplification is for the host to decide it
//! instead — take the number and cast it. **A cast saturates.** `1e30 as i64` is
//! not an error and not a wrap: it is `i64::MAX`, silently, and the engine then
//! believes a mod asked for nine quintillion of something. NaN casts to zero by
//! the same rule. Neither is detectable after the fact, because the value that
//! arrives is a perfectly ordinary whole number.
//!
//! So the cases below are chosen at the edges rather than in the middle: the
//! largest and most negative whole numbers that survive the round trip, the
//! first one that does not, a magnitude far past the range, and the two values
//! that are not numbers in the usual sense at all. A test using only small
//! numbers agrees with a saturating cast on every one of them.
//!
//! # Why the comparison is on rendered text
//!
//! Two reasons, and the second is the load-bearing one. Comparing floats
//! directly is a thing this codebase lints against; and rendering carries the
//! *kind* alongside the value, so a whole number and a fractional one holding
//! the same quantity do not compare equal. A saturating cast of `1e30` produces
//! a whole number, which is exactly the confusion an equality on quantity alone
//! would wave through.
//!
//! Every expected value is computed here from the same constant the script
//! names — never transcribed from a run of the host.

use std::error::Error;

use mc_script::{ScriptFault, ScriptHost, ScriptValue};

type TestResult = Result<(), Box<dyn Error>>;

/// Each case: what it is, and the chunk that produces it.
const NUMBERS_AT_THE_EDGES: [(&str, &str); 8] = [
    ("a small whole number", "return 3"),
    ("a fraction", "return 3.5"),
    ("the largest power of two that fits", "return 2^62"),
    ("one power of two past what fits", "return 2^63"),
    ("the most negative whole number that fits", "return -2^63"),
    ("a magnitude far past the range", "return 1e30"),
    ("not a number", "return 0/0"),
    ("positive infinity", "return 1/0"),
];

fn new_host() -> Result<ScriptHost, Box<dyn Error>> {
    match ScriptHost::new() {
        Ok(host) => Ok(host),
        Err(error) => Err(format!("the host could not be constructed: {error:?}").into()),
    }
}

/// What an evaluation produced, as one comparable line carrying the kind as well
/// as the quantity.
fn outcome(evaluated: Result<ScriptValue, ScriptFault>) -> String {
    match evaluated {
        Ok(ScriptValue::Nil) => "nil".to_owned(),
        Ok(ScriptValue::Boolean(flag)) => format!("boolean {flag}"),
        Ok(ScriptValue::Integer(number)) => whole(number),
        Ok(ScriptValue::Number(number)) => fractional(number),
        Ok(ScriptValue::Text(text)) => format!("text {text}"),
        Ok(ScriptValue::Table(_)) => "table".to_owned(),
        Ok(ScriptValue::Function(_)) => "function".to_owned(),
        Ok(ScriptValue::Opaque) => "opaque".to_owned(),
        Err(fault) => format!("fault: {fault}"),
    }
}

/// How a whole number reads.
fn whole(value: i64) -> String {
    format!("integer {value}")
}

/// How a number that is not whole, or not in range, reads.
fn fractional(value: f64) -> String {
    format!("number {value}")
}

#[test]
fn a_number_leaves_script_as_a_whole_number_only_when_it_is_one_and_fits() -> TestResult {
    let mut host = new_host()?;
    let observed: Vec<(&str, String)> = NUMBERS_AT_THE_EDGES
        .iter()
        .map(|(described, source)| (*described, outcome(host.evaluate("numbers.luau", source))))
        .collect();

    assert_eq!(
        observed,
        vec![
            ("a small whole number", whole(3)),
            ("a fraction", fractional(3.5)),
            ("the largest power of two that fits", whole(1_i64 << 62)),
            (
                "one power of two past what fits",
                fractional(2_f64.powi(63))
            ),
            ("the most negative whole number that fits", whole(i64::MIN)),
            ("a magnitude far past the range", fractional(1e30)),
            ("not a number", fractional(f64::NAN)),
            ("positive infinity", fractional(f64::INFINITY)),
        ],
        "the four cases past the range are the ones that decide this. A host that classifies by \
         casting rather than by asking gets every small number right and turns each of those \
         four into a whole number it invented: a cast saturates, so a magnitude far past the \
         range becomes the largest whole number there is, and a value that is not a number \
         becomes zero. Both arrive looking like an ordinary quantity a mod asked for, and \
         nothing downstream re-checks a number that has already been classified."
    );
    Ok(())
}
