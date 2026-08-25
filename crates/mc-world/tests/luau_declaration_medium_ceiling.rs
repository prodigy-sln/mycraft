//! Where the resistance scale actually stops, asserted as a pair either side of
//! one boundary.
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
//! What made the gap worth closing is that `docs/modding/blocks-items.md` now
//! promises a mod author a **specific number** — `at most 3.4e38`, in its `Bound`
//! column — and a page naming a figure a reader will rely on needs something
//! behind it. The two values below pin the real boundary to about three percent,
//! which is an instrument. A reading taken once and written into a document is a
//! measurement, and a measurement is only ever true of the tree it was taken on.
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
//! `1e40` reaches by a very much wider margin.
//!
//! # One total reading, so each half rejects the other's outcome
//!
//! Both tests compare the whole of [`AtTheCeiling`]. A comparison against
//! `Registered` rejects a refusal for free and a comparison against `Refused`
//! rejects a registration, so neither half can pass because the loader stopped
//! being able to answer — which is the whole point of asserting two adjacent
//! values rather than one.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{AMBER, AMBER_FILE, QUARTZ, declaration_of, raw_field, text_field};
use mc_core::block::source::DefinitionSourceError;
use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::content::LuauFileDefinitionSource;
use tempfile::TempDir;

/// The key a declaration states how much its volume slows movement in.
const MOVE_RESISTANCE_FIELD: &str = "move_resistance";

/// The largest resistance `docs/modding/blocks-items.md` promises is accepted.
const THE_LARGEST_RESISTANCE_THE_PAGE_PROMISES: &str = "3.4e38";

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

/// A root whose one non-solid declaration states `move_resistance` as `stated`.
fn root_resisting(directory: &TempDir, stated: &str) -> Result<PathBuf, Box<dyn Error>> {
    let fields = vec![
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", "false"),
        raw_field(MOVE_RESISTANCE_FIELD, stated),
    ];
    content_root(directory, &[(AMBER_FILE, declaration_of(&fields))])
}

/// What the content root at `root` made of the resistance it states.
fn at_the_ceiling(root: &Path) -> AtTheCeiling {
    let mut registry = BlockRegistry::new();
    match registry.apply(&LuauFileDefinitionSource::new(root)) {
        Ok(()) => match BlockName::parse(AMBER)
            .ok()
            .and_then(|name| registry.resolve(&name).ok())
        {
            Some(definition) => AtTheCeiling::Registered(definition.move_resistance),
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
    let root = root_resisting(&directory, THE_LARGEST_RESISTANCE_THE_PAGE_PROMISES)?;

    assert_eq!(
        at_the_ceiling(&root),
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
    let root = root_resisting(&directory, A_STEP_PAST_WHAT_THE_PAGE_PROMISES)?;

    assert_eq!(
        at_the_ceiling(&root),
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
