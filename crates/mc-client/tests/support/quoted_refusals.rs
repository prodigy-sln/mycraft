//! Recognising a refusal quoted on a modding page, and the field order the pages
//! introduce a declaration's fields in.
//!
//! # Why this is a module of its own
//!
//! Two binaries now hold a page against a real run — `documented_refusals.rs`,
//! which asks whether everything the pages quote is text the client writes, and
//! `documented_property_refusals.rs`, which asks whether the refusals a
//! particular field raises are quoted at all. Those are opposite directions over
//! one recogniser, and a second copy of the recogniser would be the thing both
//! guards exist to prevent, one level up: two instruments agreeing with each
//! other about what a fenced block is, while neither agrees with the pages.
//!
//! # The recogniser is derived from the artefact, not agreed with an author
//!
//! **A quoted refusal is a fenced code block whose first line begins
//! [`REFUSAL_PREFIX`].** That prefix is the first thing the reporting writes, so
//! nothing has to be maintained for a newly pasted refusal to come under a
//! comparison — and a page that *stops* quoting one changes a verdict rather than
//! passing silently on an empty set.
//!
//! # The field order is the page's own, written out
//!
//! [`FIELDS_IN_THE_ORDER_THE_GUIDE_STATES`] is spelled here rather than read from
//! the loader, which is the point: an expectation derived from the value under
//! test agrees with whatever that value becomes. What holds it honest is that a
//! test compares it against the list a real refusal quotes, so the mirror is
//! enforced without being derived.
//!
//! **It was silently blind before this, and the shape of the blindness is worth
//! recording.** [`ranked_field`] returns `Option` and its caller skips what it
//! cannot rank — deliberately, because `slid` and `up` are names the guide never
//! introduces. So a list left short of a field the loader had gained ranked that
//! field nowhere, ordered nothing by it, and compiled: an absent instrument and a
//! clean one looked identical (`standards/global/testing.md` §2).

// Each binary that includes this uses a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::path::PathBuf;

use crate::printed_refusals::normalised;
use crate::support;

/// The first thing the reporting writes, and therefore the whole of the
/// recogniser: a quoted refusal is a fenced block whose first line begins here.
pub const REFUSAL_PREFIX: &str = "mycraft: ";

/// How a fenced block opens and closes.
pub const FENCE: &str = "```";

/// The directory of pages a mod author reads, below the repository root.
pub const PAGES: [&str; 2] = ["docs", "modding"];

/// The page a mod author writes a block declaration against, below [`PAGES`].
pub const BLOCKS_PAGE: &str = "blocks-items.md";

/// The words a refusal introduces the recognised-field list with.
const THE_LIST_IS_INTRODUCED_BY: &str = "a declaration may state ";

/// Every field a declaration may state, in the order the guide introduces them.
///
/// Written out here rather than read from the loader — see the module header for
/// why, and for what that cost before anything compared it against a run.
pub const FIELDS_IN_THE_ORDER_THE_GUIDE_STATES: [&str; 9] = [
    "name",
    "texture",
    "solid",
    "replaceable",
    "breakable",
    "breaks_into",
    "drawn",
    "occludes",
    "targetable",
];

/// Where the pages a mod author reads live.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located above this crate.
pub fn pages() -> Result<PathBuf, Box<dyn Error>> {
    Ok(PAGES
        .iter()
        .fold(support::repository_root()?, |below, part| below.join(part)))
}

/// The refusals one page quotes: the fenced blocks whose first line begins with
/// the prefix the reporting writes, in the order they appear.
///
/// A block's info string is not read — `text`, `console` or nothing at all are
/// the same block — because the recogniser is derived from what the program
/// writes and an info string is something an author chooses.
#[must_use]
pub fn quoted_refusals_in(page: &str) -> Vec<String> {
    let mut quoted = Vec::new();
    let mut lines = page.lines();
    while let Some(line) = lines.next() {
        if !is_fence(line) {
            continue;
        }
        let block = lines
            .by_ref()
            .take_while(|line| !is_fence(line))
            .collect::<Vec<_>>()
            .join("\n");
        if block.starts_with(REFUSAL_PREFIX) {
            quoted.push(normalised(&block));
        }
    }
    quoted
}

/// Whether a line opens or closes a fenced block.
#[must_use]
pub fn is_fence(line: &str) -> bool {
    line.trim_start().starts_with(FENCE)
}

/// Where in the guide's own order the field `refusal` blames sits, and what it is
/// called.
///
/// `None` where the blamed name is not one of the fields the guide introduces:
/// `slid` is the misspelling one refusal is *about*, and a facing word belongs to
/// a texture table rather than to a declaration, so ranking either would put a
/// word the guide never introduces in the order it introduces things in.
#[must_use]
pub fn ranked_field(refusal: &str) -> Option<(usize, String)> {
    FIELDS_IN_THE_ORDER_THE_GUIDE_STATES
        .into_iter()
        .enumerate()
        .find(|(_, field)| refusal.contains(&format!("field `{field}`")))
        .map(|(at, field)| (at, field.to_owned()))
}

/// Every field name `refusal` quotes back as one a declaration may state, in the
/// order it quotes them.
///
/// Read out of the refusal rather than filtered against
/// [`FIELDS_IN_THE_ORDER_THE_GUIDE_STATES`], so an extra name, a missing name and
/// a reordering are three different failures. A filter can only answer "which of
/// my names does this mention", which is exactly the answer a stale mirror gives
/// correctly.
///
/// Empty where the refusal introduces no list, which is honest: a refusal about
/// something else quotes nothing to compare.
#[must_use]
pub fn fields_a_refusal_quotes(refusal: &str) -> Vec<String> {
    let Some(at) = refusal.rfind(THE_LIST_IS_INTRODUCED_BY) else {
        return Vec::new();
    };
    refusal[at + THE_LIST_IS_INTRODUCED_BY.len()..]
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// [`FIELDS_IN_THE_ORDER_THE_GUIDE_STATES`] as a comparison reads them.
#[must_use]
pub fn every_field_the_guide_states() -> Vec<String> {
    FIELDS_IN_THE_ORDER_THE_GUIDE_STATES
        .iter()
        .map(|field| (*field).to_owned())
        .collect()
}
