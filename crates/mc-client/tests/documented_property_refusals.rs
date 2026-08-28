//! The refusals a declaration's three seeing properties raise are refusals the
//! modding guide quotes, and the order that guide introduces the fields in is the
//! loader's own.
//!
//! # Why this is the opposite direction from `documented_refusals.rs`
//!
//! That binary asks: **is everything the pages quote text the client writes?** It
//! is the direction that keeps a page honest, and it is blind to a refusal nobody
//! quotes at all — a client can gain a refusal, the pages can stay silent about
//! it, and the verdict there is unchanged. Measured on the tree this file arrived
//! in: a mod author can trip roughly fourteen block-declaration refusals and the
//! pages quote three, so the gap is real, deliberate and filed there.
//!
//! This binary asks the other question over a **named pair**: are the two refusals
//! a mod author meets by getting `drawn` wrong quoted on the page they are already
//! reading? Named rather than swept, because sweeping would demand a page for
//! every refusal in the workspace — which is the change that introduces defects at
//! the moment there is least appetite to look for them.
//!
//! `the_modding_guide_states_every_per_facing_refusal_in_the_recognised_field_order`
//! is the same shape for the seven a texture table raises, and it reaches none of
//! these: its input is the texture table's own refusals.
//!
//! # Where a quotation may sit on the page, and why it is not free
//!
//! The per-facing guard ranks each quotation by the field it blames and demands
//! the page introduce them in the guide's order. `drawn` is the seventh field and
//! `texture` the second, so a `drawn` refusal quoted **above** the texture-table
//! refusals puts the page out of that order and reddens a guard in the other
//! binary rather than anything here. The order is the page's contract, not this
//! file's, and it is recorded here because whoever pastes the quotation will meet
//! it as a surprise otherwise.
//!
//! # A verdict, not an absence, and a control on the verdict
//!
//! A reading that came to report every refusal as quoted would agree with any page
//! forever. So the answer is one of three arms — and a second test drives the
//! reading over a page quoting exactly one of the pair, which is the only thing
//! that watches the disagreeing arm come out of a real comparison. The sibling
//! binary records the absence of that control as a known hole; this one does not
//! inherit it.

#[path = "support/built_set_refusals.rs"]
mod built_set_refusals;
#[path = "support/entry.rs"]
mod entry;
#[path = "support/input/mod.rs"]
mod input;
#[path = "support/launch_notices.rs"]
mod launch_notices;
#[path = "support/opacity_refusals.rs"]
mod opacity_refusals;
#[path = "support/per_facing_refusals.rs"]
mod per_facing_refusals;
#[path = "support/persistence.rs"]
mod persistence;
#[path = "support/printed_refusals.rs"]
mod printed_refusals;
#[path = "support/quoted_refusals.rs"]
mod quoted_refusals;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::fs;
use std::path::Path;

use printed_refusals::{a_drawnness_stated_as_a_number, a_field_a_letter_past_drawn};
use quoted_refusals::{
    BLOCKS_PAGE, every_field_the_guide_states, fields_a_refusal_quotes, pages, quoted_refusals_in,
};
use support::TestResult;

/// What a page makes of a named set of refusals.
///
/// Three arms rather than a count, so "the page quotes them all" and "the page
/// quotes nothing" can never compare equal, and so a reading that stopped finding
/// quotations reports that rather than borrowing the good answer.
#[derive(Debug, PartialEq, Eq)]
enum GuideCoverage {
    /// Every refusal in the set appears on the page, verbatim.
    BothRefusalsAreOnThePage,
    /// This refusal is raised and the page does not carry it.
    OneIsMissingFromThePage { refusal: String },
    /// The page was read and holds no quoted refusal at all.
    ThePageQuotesNothingAtAll,
}

/// The two refusals a mod author meets by getting `drawn` wrong: stating it as a
/// number, and misspelling its name.
///
/// # Errors
///
/// Returns an error if either fixture root is accepted, or if a refusal does not
/// name the root it was given.
fn the_two_refusals_drawn_raises() -> Result<Vec<String>, Box<dyn Error>> {
    Ok(vec![
        a_drawnness_stated_as_a_number()?,
        a_field_a_letter_past_drawn()?,
    ])
}

/// What `page` makes of `raised`.
///
/// # Errors
///
/// Returns an error if the page cannot be read, or if nothing was raised for it to
/// be compared against — an empty set is not one of the three arms, because a
/// reading with nothing to look for would answer `BothRefusalsAreOnThePage` about a
/// page it never opened.
fn coverage_of(page: &Path, raised: &[String]) -> Result<GuideCoverage, Box<dyn Error>> {
    if raised.is_empty() {
        return Err("no refusal was raised for the guide to be compared against".into());
    }
    let quoted = quoted_refusals_in(&fs::read_to_string(page)?);
    if quoted.is_empty() {
        return Ok(GuideCoverage::ThePageQuotesNothingAtAll);
    }
    match raised.iter().find(|refusal| !quoted.contains(refusal)) {
        Some(refusal) => Ok(GuideCoverage::OneIsMissingFromThePage {
            refusal: refusal.clone(),
        }),
        None => Ok(GuideCoverage::BothRefusalsAreOnThePage),
    }
}

/// A page under `directory` quoting `refusal` and nothing else.
fn a_page_quoting_only(directory: &Path, refusal: &str) -> Result<(), Box<dyn Error>> {
    fs::write(
        directory.join("half-a-page.md"),
        format!("# When you get it wrong\n\nWhat you read:\n\n```text\n{refusal}\n```\n"),
    )?;
    Ok(())
}

#[test]
fn the_blocks_guide_quotes_both_refusals_a_misdeclared_drawnness_raises() -> TestResult {
    let raised = the_two_refusals_drawn_raises()?;

    let coverage = coverage_of(&pages()?.join(BLOCKS_PAGE), &raised)?;

    assert_eq!(
        coverage,
        GuideCoverage::BothRefusalsAreOnThePage,
        "a mod author who writes `drawn = 1` or `drawnn` reads a line on their terminal and \
         then searches the page they were already reading for it, so the page has to carry \
         that line rather than a description of it. Nothing else in the workspace asks this: \
         the guard next door asks whether what a page quotes is real, which is green over a \
         page quoting neither, and the per-facing guard's input is a texture table's own \
         refusals. What the client writes for these two today is:\n\n{}\n",
        raised.join("\n\n")
    );
    Ok(())
}

#[test]
fn a_page_quoting_one_of_them_is_reported_as_missing_the_other() -> TestResult {
    let raised = the_two_refusals_drawn_raises()?;
    let quoted_one = raised
        .first()
        .ok_or("this control needs a refusal for a page to quote")?;
    let missing = raised
        .get(1)
        .ok_or("this control needs a second refusal for a page to omit")?;
    let pages = tempfile::tempdir()?;
    a_page_quoting_only(pages.path(), quoted_one)?;

    let coverage = coverage_of(&pages.path().join("half-a-page.md"), &raised)?;

    assert_eq!(
        coverage,
        GuideCoverage::OneIsMissingFromThePage {
            refusal: missing.clone()
        },
        "the control on the guard above, and the only thing that watches the disagreeing arm \
         come out of a real comparison. A reading that had come to report every refusal as \
         quoted — a recogniser that matched too much, a comparison that normalised both sides \
         into agreement — would leave that guard green over any page at all, and the redness \
         would then arrive as a documentation problem long after the instrument stopped \
         looking. Naming the refusal the page omits is what makes the report actionable \
         rather than a boolean"
    );
    Ok(())
}

#[test]
fn the_guide_introduces_the_declaration_fields_in_the_order_a_refusal_quotes_them() -> TestResult {
    let refusal = a_field_a_letter_past_drawn()?;

    let quoted = fields_a_refusal_quotes(&refusal);

    assert_eq!(
        quoted,
        every_field_the_guide_states(),
        "the guide's field order is written out by hand, which is what stops it agreeing with \
         whatever the loader becomes — and until this compared it against a run, nothing \
         reddened when the two parted company. The reading that ranks a page's quotations \
         skips a field it cannot place, deliberately, so a list left short of a field the \
         loader had gained ranked that field nowhere and compiled: an absent instrument and a \
         clean one looked identical. The whole list is compared in order, so a missing name, \
         an extra name and a reordering are three different failures. The refusal was:\n\n{refusal}\n"
    );
    Ok(())
}
