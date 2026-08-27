//! Where the two numeric scales a declaration may state actually stop, each
//! asserted as a pair either side of one boundary.
//!
//! # Why a pair, and why it is not the question `1e40` asks
//!
//! `luau_declaration_medium_refusals.rs` states `1e40` and asserts it is refused
//! as not finite. That is true and worth knowing, and it is **a decade past the
//! width the engine keeps** — so it proves a ceiling *exists* rather than saying
//! where it is: a loader that had silently come to refuse everything above `1e20`
//! satisfies it exactly as a correct one does. `1e40` stays, because far past the
//! bound and at the bound are different questions, and swapping one for the other
//! would trade a witness rather than add one.
//!
//! What made the gap worth closing is that `docs/modding/blocks-items.md`
//! promises a mod author a **specific number** — `at most 3.4e38`, in its `Bound`
//! column — and a page naming a figure a reader will rely on needs something
//! behind it. The two values below pin the real boundary to about three percent,
//! which is an instrument. A reading taken once and written into a document is a
//! measurement, and a measurement is only ever true of the tree it was taken on.
//!
//! # Why `swim_ascent` gets its own pair rather than riding on the resistance's
//!
//! **Because the page states the figure twice.** That `Bound` column now carries
//! the same sentence on two rows, and each of them is a separate promise to a
//! mod author who will write the number and expect a block.
//!
//! Today the two fields share one reader — `optional_number_at_least_zero` — so
//! it is fair to ask whether the second pair re-proves the first through the same
//! code path, which `standards/global/testing.md` §1 calls a bogus test. It does
//! not, and the distinction is the one the signed-zero fixture already rests on:
//! the resistance's pair proves that **the shared reader** narrows where the page
//! says, and cannot prove that `declared_ascent` calls it. Three fixtures next
//! door establish that it reaches the floor, the finiteness check and the
//! normalisation — and a reader that asked its finiteness question **before**
//! narrowing to the width the physics keeps would pass all three, refuse
//! `math.huge` correctly, and hand a swimmer an infinite launch speed for
//! `3.5e38`. That is the defect this half catches and nothing else does.
//!
//! The two pairs share one reading rather than one being copied, so a rule
//! stated twice cannot come to disagree with itself.
//!
//! # The arithmetic, measured before the assertion was chosen
//!
//! `standards/global/testing.md` §2 asks for the path to be measured rather than
//! the literals inspected, because this is the shape that produces an over-tight
//! assertion — one that reddens against correct code and whose cheapest green is
//! a clamp nobody specced.
//!
//! **`3.4e38` does not retain as `3.4e38`.** An `f32` carries about seven digits,
//! so it retains as `3.3999999521443642e38`, comfortably under `f32::MAX` of
//! `3.4028234663852886e38`. The assertion below is written `3.4e38` and is exact
//! anyway, because in that position the literal is an `f32` and Rust rounds the
//! decimal straight to that same value. That agreement was **checked rather than
//! assumed**: a declared number reaches the host as an `f64` and is narrowed
//! afterwards, so the retained value could in principle differ from the literal
//! by a double rounding. It does not — verified for `3.4e38`, `3.5e38`, `1e30`
//! and `1e40` by comparing the directly-rounded `f32` against the narrowed one.
//!
//! `3.5e38` exceeds `f32::MAX` and narrows to an infinity, which is the branch
//! `1e40` reaches by a very much wider margin. Re-measured when the second field
//! joined, rather than inherited from the paragraph above: `3.4e38` narrowed from
//! an `f64` and `3.4e38` written as an `f32` literal are the same bits
//! (`7f7fc99e`), `3.5e38` narrows to `7f800000`, and the two values sit 2.94 per
//! cent apart. The narrowing is a property of the number and not of the field, so
//! one measurement covers both pairs — which is exactly why it is stated once
//! here and not restated beside each test.
//!
//! # One total reading, so each half rejects the other's outcome
//!
//! Every test here compares the whole of [`AtTheCeiling`]. A comparison against
//! `Registered` rejects a refusal for free and a comparison against `Refused`
//! rejects a registration, so no half can pass because the loader stopped being
//! able to answer — which is the whole point of asserting two adjacent values
//! rather than one. The reading is told **which field to read back** rather than
//! reaching for one by name, so a pair pointed at the ascent cannot be satisfied
//! by a correct resistance sitting beside it.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{AMBER, AMBER_FILE, QUARTZ, declaration_of, raw_field, text_field};
use mc_core::block::source::DefinitionSourceError;
use mc_core::block::{BlockDefinition, BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::content::LuauFileDefinitionSource;
use tempfile::TempDir;

/// The key a declaration states how much its volume slows movement in.
const MOVE_RESISTANCE_FIELD: &str = "move_resistance";

/// The key a declaration states how fast its volume lifts a swimmer in.
const SWIM_ASCENT_FIELD: &str = "swim_ascent";

/// The largest value `docs/modding/blocks-items.md` promises is accepted.
///
/// One constant for both fields because the page states one sentence on both
/// rows: two constants holding the same figure would let a page that changed one
/// row keep half of this file green.
const THE_LARGEST_VALUE_THE_PAGE_PROMISES: &str = "3.4e38";

/// The first round step past that promise, which exceeds `f32::MAX` and narrows
/// to an infinity.
///
/// About three percent above the value beside it, which is how tightly this pair
/// pins the boundary. Nothing needs it tighter: what a mod author reads is the
/// page's figure, and what this has to catch is a ceiling that has moved by an
/// order of magnitude rather than by an ulp.
const A_STEP_PAST_WHAT_THE_PAGE_PROMISES: &str = "3.5e38";

/// What became of a declaration stating a resistance at the top of the scale.
///
/// One value rather than a question about whether a refusal happened followed by
/// a question about what it said: the two halves of this pair assert opposite
/// outcomes, and each has to be able to fail by observing the other rather than
/// by never reaching its assertion.
#[derive(Debug, PartialEq)]
enum AtTheCeiling {
    /// Accepted, retaining this resistance.
    Registered(f32),
    /// Refused as a malformed declaration, blaming this field, saying this.
    Refused {
        field: Option<String>,
        cause: String,
    },
    /// Refused as something other than a malformed declaration, or accepted
    /// without registering the block at all — neither of which either half of
    /// this pair may be allowed to borrow.
    NeitherOfThose(String),
}

/// A root whose one non-solid declaration states `field` as `stated`.
///
/// Non-solid throughout, per the spec's standing fixture rule for anything
/// stating a medium property: a solid fixture is never overlapped, so a test over
/// one measures collision and reports a clean pass.
fn root_stating(directory: &TempDir, field: &str, stated: &str) -> Result<PathBuf, Box<dyn Error>> {
    let fields = vec![
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", "false"),
        raw_field(field, stated),
    ];
    content_root(directory, &[(AMBER_FILE, declaration_of(&fields))])
}

/// What the content root at `root` made of the number it states, read back
/// through `retained`.
///
/// The reader is a parameter rather than a field name looked up here, so the two
/// pairs below share one statement of the rule. A second copy of it is the thing
/// this file exists to prevent one level up: two instruments agreeing with each
/// other about where a scale stops, while neither agrees with the page.
fn at_the_ceiling(root: &Path, retained: fn(&BlockDefinition) -> f32) -> AtTheCeiling {
    let mut registry = BlockRegistry::new();
    match registry.apply(&LuauFileDefinitionSource::new(root)) {
        Ok(()) => match BlockName::parse(AMBER)
            .ok()
            .and_then(|name| registry.resolve(&name).ok())
        {
            Some(definition) => AtTheCeiling::Registered(retained(definition)),
            None => AtTheCeiling::NeitherOfThose(format!(
                "the root was accepted and registered no block called {AMBER}"
            )),
        },
        Err(RegistryError::Source(DefinitionSourceError::Malformed(fault))) => {
            AtTheCeiling::Refused {
                field: fault.field,
                cause: fault.cause,
            }
        }
        Err(other) => AtTheCeiling::NeitherOfThose(other.to_string()),
    }
}

#[test]
fn the_largest_resistance_the_guide_promises_is_registered_at_the_width_the_engine_keeps()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        MOVE_RESISTANCE_FIELD,
        THE_LARGEST_VALUE_THE_PAGE_PROMISES,
    )?;

    assert_eq!(
        at_the_ceiling(&root, |definition| definition.move_resistance),
        AtTheCeiling::Registered(3.4e38),
        "the modding page names this number in its `Bound` column, so a mod author may write it \
         and expect a block. Until this pair existed the page's figure had nothing behind it: \
         the only witness above the scale was a decade past the width the engine keeps, which \
         says a ceiling is somewhere rather than where. The value is the page's own rather than \
         one read back off a run, and it is exact rather than approximate — an `f32` carries \
         about seven digits, so this retains as `3.3999999521443642e38`, and the literal here \
         rounds to that same value, which was measured rather than assumed"
    );
    Ok(())
}

#[test]
fn a_resistance_a_step_past_that_promise_is_refused_as_not_finite() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        MOVE_RESISTANCE_FIELD,
        A_STEP_PAST_WHAT_THE_PAGE_PROMISES,
    )?;

    assert_eq!(
        at_the_ceiling(&root, |definition| definition.move_resistance),
        AtTheCeiling::Refused {
            field: Some(MOVE_RESISTANCE_FIELD.to_owned()),
            cause: format!("`{MOVE_RESISTANCE_FIELD}` must be a finite number"),
        },
        "the other side of the same boundary, three percent up, and what turns its neighbour \
         from a measurement into an instrument: a ceiling that moved would have to move less \
         than that to leave both halves green. It is refused with the sentence about finiteness \
         rather than one about a maximum, because nothing here is a declared ceiling — the value \
         is simply not a number the engine can keep, which is the same fact `1e40` states from \
         very much further out"
    );
    Ok(())
}

#[test]
fn the_largest_ascent_the_guide_promises_is_registered_at_the_width_the_engine_keeps() -> TestResult
{
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        SWIM_ASCENT_FIELD,
        THE_LARGEST_VALUE_THE_PAGE_PROMISES,
    )?;

    assert_eq!(
        at_the_ceiling(&root, |definition| definition.swim_ascent),
        AtTheCeiling::Registered(3.4e38),
        "the modding page names this number in the `Bound` column of this row too, and a promise \
         made twice is two promises. This half says the scale is unbounded above in practice: a \
         volume this fast is unswimmable and is still a finite number with a well-defined \
         answer, so refusing it would need a ceiling nobody can derive and clamping it would \
         hand a mod author a block other than the one they wrote. The value is the page's own \
         rather than one read back off a run"
    );
    Ok(())
}

#[test]
fn an_ascent_a_step_past_that_promise_is_refused_as_not_finite() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(
        &directory,
        SWIM_ASCENT_FIELD,
        A_STEP_PAST_WHAT_THE_PAGE_PROMISES,
    )?;

    assert_eq!(
        at_the_ceiling(&root, |definition| definition.swim_ascent),
        AtTheCeiling::Refused {
            field: Some(SWIM_ASCENT_FIELD.to_owned()),
            cause: format!("`{SWIM_ASCENT_FIELD}` must be a finite number"),
        },
        "the half that is not a restatement of the resistance's, and the reason this field has a \
         pair at all. `3.5e38` is a perfectly finite number where the declaration wrote it and \
         an infinity at the width a tick launches with, so a reader asking its finiteness \
         question **before** narrowing admits it — and that reader refuses `math.huge`, refuses \
         a negative and normalises a signed zero, so every other fixture this field has stays \
         green while a swimmer is launched at an infinity nobody declared. Refused with the \
         sentence about finiteness rather than one about a maximum, because nothing here is a \
         declared ceiling: the value is simply not a number the engine can keep"
    );
    Ok(())
}
