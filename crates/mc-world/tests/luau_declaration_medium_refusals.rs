//! Everything a declaration can say about a medium that the engine will not
//! take, and the sentence each of those refusals owes its author.
//!
//! A refused declaration costs a mod author a line they can read. **A coerced
//! one costs them a block that quietly behaves like air**, with no symptom to
//! notice and nothing to search for — which is why every value below is refused
//! rather than clamped, rounded, parsed or defaulted.
//!
//! # `move_resistance` decides the numeric vocabulary every later number inherits
//!
//! It is the first number a declaration may state, so the four things that can be
//! wrong with one are settled here rather than in the field that happens to come
//! second: a value of the wrong kind, a value below the floor, a value that is
//! not a finite number at all, and a value too large for the width the physics
//! divides by. Each is a separate branch and each has a fixture, because
//! `standards/global/testing.md` §5 puts validation rules at 100% coverage — a
//! branch with no fixture is a branch that can be deleted with the suite green.
//!
//! # The two that are not decorative
//!
//! Luau evaluates `0/0` to a NaN and `1/0` to an infinity, and both arrive as a
//! declared number, so the finiteness check is reachable from a content file
//! rather than being a guard against something nobody can write. A NaN reaching
//! the tick path poisons a position permanently — the same failure the walk a
//! client asks for is already guarded against — and it must be refused with the
//! sentence about finiteness rather than with the one about the floor, because
//! `NaN >= 0.0` is false and a floor test alone would blame the wrong thing.
//!
//! # The kind a refusal names is Luau's own word for it
//!
//! `a boolean`, `a string`, `a number`: what a mod author reads is the vocabulary
//! of the file they are looking at, so the sentence sends them to a line rather
//! than to the host's internal name for the same thing.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, Blamed, QUARTZ, blaming, declaration_of, judged, raw_field, text_field,
};
use tempfile::TempDir;

/// The key a declaration states being something a player can swim in.
const SWIMMABLE_FIELD: &str = "swimmable";

/// The key a declaration states how much its volume slows movement in.
const MOVE_RESISTANCE_FIELD: &str = "move_resistance";

/// A swimmability written as a number, which is the mistake a boolean field
/// invites: the two values it accepts are spelled `true` and `false`, and every
/// other language a mod author has written spells one of them `1`.
const A_SWIMMABILITY_WRITTEN_AS_A_NUMBER: &str = "1";

/// A resistance below the floor, written the shortest way anybody writes one.
const A_RESISTANCE_BELOW_THE_FLOOR: &str = "-1";

/// The expression Luau evaluates to a NaN.
const A_RESISTANCE_THAT_IS_NOT_A_NUMBER: &str = "0/0";

/// The expression Luau evaluates to an infinity.
const A_RESISTANCE_WITHOUT_BOUND: &str = "1/0";

/// A finite Luau number too large for the width the physics divides by.
///
/// It is finite where the script wrote it and is not finite once retained, so it
/// is the one fixture here that separates a check applied to what a declaration
/// *said* from one applied to what the engine *keeps*. The first admits it and
/// hands the tick an infinity nobody declared.
const A_RESISTANCE_PAST_THE_RETAINED_WIDTH: &str = "1e40";

/// A resistance written as the wrong kind of value entirely.
const A_RESISTANCE_WRITTEN_AS_A_BOOLEAN: &str = "true";

/// A resistance written as text that looks like the number it is not.
///
/// The mistake a field whose value is a number invites from anybody used to a
/// format where everything is a string. Parsing it is refused for the reason
/// coercion is refused everywhere else on this declaration: a mod author who
/// wrote a string and got a number learns nothing, and the next string they write
/// is one that does not parse.
const A_RESISTANCE_WRITTEN_AS_A_STRING: &str = "'1.0'";

/// What a refusal about a medium field owes: the file, the block, the field, and
/// the sentence a mod author reads.
///
/// The cause travels whole rather than as a substring check. A refusal naming the
/// field and then saying something else entirely about it — the sentence for a
/// field nobody recognises, say, or the floor's sentence where finiteness was the
/// problem — is exactly what a `contains` on the field name cannot see, and it is
/// the likelier of the two failures while a field is new.
#[derive(Debug, PartialEq, Eq)]
struct Refusal {
    blamed: Blamed,
    cause: String,
}

/// A root whose one declaration states `field` as `stated` and is otherwise
/// well formed.
///
/// The shape every fixture here takes: a declaration that would register, and one
/// line in it that must stop it. A fixture built any other way leaves it open
/// whether the refusal was about that line at all. Non-solid throughout, per the
/// spec's standing fixture rule for anything stating a resistance.
fn root_stating(directory: &TempDir, field: &str, stated: &str) -> Result<PathBuf, Box<dyn Error>> {
    let fields = vec![
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", "false"),
        raw_field(field, stated),
    ];
    content_root(directory, &[(AMBER_FILE, declaration_of(&fields))])
}

/// What the root at `root` refused, and what it said about it.
fn refusal_of(root: &Path) -> Refusal {
    let (blamed, cause) = judged(root, AMBER_FILE);
    Refusal { blamed, cause }
}

/// A refusal blaming [`AMBER`]'s `move_resistance` with `cause`.
fn blaming_the_resistance(cause: &str) -> Refusal {
    Refusal {
        blamed: Blamed::Declaration(blaming(AMBER, MOVE_RESISTANCE_FIELD)),
        cause: cause.to_owned(),
    }
}

#[test]
fn a_swimmability_written_as_a_number_is_refused_naming_the_two_values_it_accepts() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        SWIMMABLE_FIELD,
        A_SWIMMABILITY_WRITTEN_AS_A_NUMBER,
    )?;

    assert_eq!(
        refusal_of(&root),
        Refusal {
            blamed: Blamed::Declaration(blaming(AMBER, SWIMMABLE_FIELD)),
            cause: format!("`{SWIMMABLE_FIELD}` must be true or false, but is a number"),
        },
        "`swimmable = 1` is the mistake a boolean field invites, and falling back to the default \
         for it is the worst available outcome here: the default is `false`, so the block would \
         behave exactly as if the line had never been written and the author would never learn \
         it did nothing — they would go looking at the physics for a pool nobody can swim in. \
         Both accepted values are quoted rather than only the field, because an author who \
         wrote `1` needs to be told what to write instead, and the kind found is quoted because \
         it is what tells them which of their lines is the one"
    );
    Ok(())
}

#[test]
fn a_resistance_below_zero_is_refused_naming_the_field_and_the_floor() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        MOVE_RESISTANCE_FIELD,
        A_RESISTANCE_BELOW_THE_FLOOR,
    )?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_resistance(&format!(
            "`{MOVE_RESISTANCE_FIELD}` may not be less than zero"
        )),
        "the scale is a divisor and `0` is already `unaffected`, so there is nothing below zero \
         for a declaration to mean — a negative would divide a movement by less than one and \
         make the volume a place that speeds a player up, or at `-1` divide by nothing at all. \
         Refusing it rather than clamping to zero is what tells the author their line was \
         wrong: clamped, the block behaves exactly as if they had left the field out"
    );
    Ok(())
}

#[test]
fn a_resistance_that_is_not_a_number_at_all_is_refused_as_not_finite() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        MOVE_RESISTANCE_FIELD,
        A_RESISTANCE_THAT_IS_NOT_A_NUMBER,
    )?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_resistance(&format!(
            "`{MOVE_RESISTANCE_FIELD}` must be a finite number"
        )),
        "`0/0` is a line a mod author can write, so this branch is reachable from content \
         rather than decorative, and a NaN reaching the tick path poisons a player's position \
         permanently — every later arithmetic over it answers NaN and nothing recovers. It is \
         refused **as not finite** and not as below the floor: `NaN >= 0.0` is false, so a \
         loader carrying only the floor test refuses this too, with a sentence about zero that \
         sends its author looking for a minus sign they never wrote"
    );
    Ok(())
}

#[test]
fn a_resistance_without_bound_is_refused_as_not_finite() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        MOVE_RESISTANCE_FIELD,
        A_RESISTANCE_WITHOUT_BOUND,
    )?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_resistance(&format!(
            "`{MOVE_RESISTANCE_FIELD}` must be a finite number"
        )),
        "the other value Luau's own division hands a declaration, and the one a floor test \
         cannot see at all — an infinity is comfortably not less than zero, so a loader checking \
         only the floor admits it and every movement through that volume divides to a zero the \
         author never asked for. Stated separately from the NaN because the two take different \
         routes through any plausible reader: one is caught by the comparison and one is not"
    );
    Ok(())
}

#[test]
fn a_resistance_too_large_for_the_width_the_engine_keeps_is_refused_as_not_finite() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        MOVE_RESISTANCE_FIELD,
        A_RESISTANCE_PAST_THE_RETAINED_WIDTH,
    )?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_resistance(&format!(
            "`{MOVE_RESISTANCE_FIELD}` must be a finite number"
        )),
        "the fixture that separates a check applied to what a declaration *said* from one \
         applied to what the engine *keeps*. This value is a perfectly finite Luau number and \
         is an infinity at the width a tick divides by, so a reader that asks its question \
         before narrowing the value admits it and hands the physics a non-finite resistance no \
         author declared — neither the exact value the scale promises nor the refusal a wrong \
         one promises, which is the silent coercion this field refuses everywhere else"
    );
    Ok(())
}

#[test]
fn a_resistance_written_as_a_boolean_is_refused_naming_the_kind_it_found() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        MOVE_RESISTANCE_FIELD,
        A_RESISTANCE_WRITTEN_AS_A_BOOLEAN,
    )?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_resistance(&format!(
            "`{MOVE_RESISTANCE_FIELD}` must be a number, but is a boolean"
        )),
        "the neighbouring field on the same declaration takes exactly this value, so `true` here \
         is a line slipped one row up rather than a typo — and the refusal has to name both the \
         kind it wanted and the kind it found for the author to see which of their two lines \
         moved. The word is Luau's own, because whoever reads this is looking at a Luau file"
    );
    Ok(())
}

#[test]
fn a_resistance_written_as_a_string_is_refused_naming_the_kind_it_found() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        MOVE_RESISTANCE_FIELD,
        A_RESISTANCE_WRITTEN_AS_A_STRING,
    )?;

    assert_eq!(
        refusal_of(&root),
        blaming_the_resistance(&format!(
            "`{MOVE_RESISTANCE_FIELD}` must be a number, but is a string"
        )),
        "text that reads like the number it is not, which is what anybody arriving from a format \
         where every value is quoted writes first. Parsing it is refused for the reason coercion \
         is refused everywhere on this declaration: an author who wrote a string and got a \
         number learns nothing, and the next string they write is one that does not parse — by \
         which time the habit is theirs and the diagnostic is about a value rather than about a \
         kind"
    );
    Ok(())
}
