//! What `docs/technical/rendering.md` owes an engine reader about the medium
//! the eye stands in, and which of those things is missing when one is.
//!
//! # A documentation scenario without a guard is a scenario with no falsifier
//!
//! The requirement fires when the document is *read*, which means prose alone
//! satisfies it — and prose can be deleted tomorrow with nothing going red. So
//! the document is read by a test, and the verdict is a **total enumeration**
//! rather than an absence: a page that could not be opened answers
//! `ThePageWasNotRead` rather than answering that nothing was found, which an
//! emptiness check cannot.
//!
//! # Why this is a second file beside the ordering model's own
//!
//! `tests/the_rendering_document_records_the_ordering_model.rs` asks what the
//! same page owes about *how two translucent surfaces compose*. This asks what
//! it owes about *what the medium the eye is inside does to everything drawn*.
//! Two questions, two files: the first stood at 495 lines and the limit on a
//! test file is 600, so the choice was a split by question or a page's worth of
//! prose trimmed to fit — and trimming the reasoning out of a guard to make room
//! for another guard is how both stop being readable.
//!
//! # The five things, and why each is owed
//!
//! Each is a thing a later reader has to be able to act on without opening the
//! shader, and each is a thing the next change could break without noticing:
//!
//! 1. **the law.** `min(1, d / D)` in linear light is a *published content
//!    surface* — an author declaring "you can see twelve blocks through this"
//!    is making a claim this arithmetic honours — so a later change to it is an
//!    amendment to what content means rather than an implementation choice;
//! 2. **where the distance is measured from.** Radial from the eye, not depth
//!    along the view direction. The two agree at the centre of the frame and
//!    nowhere else, which is exactly the shape of defect that looks right in
//!    every screenshot somebody takes of the middle of the picture;
//! 3. **what a pixel drawing no terrain is given.** The tint, through the
//!    clear — so the sky stops being sky. A reader who does not know this looks
//!    for a sky rule that is not there;
//! 4. **that the HUD is not tinted, and why.** It composites over the terrain
//!    frame in a later pass. Written down because the reason is *ordering*
//!    rather than a rule anybody wrote: whoever moves the tint into a later pass
//!    takes the exclusion away without ever deciding to;
//! 5. **that `SCENE_REVISION` did not move, with its reason and its standing
//!    condition.** A bump renames the committed set by deletion and a fresh
//!    mint, the fresh set here would be byte-identical to the deleted one, and
//!    that would destroy the one reading proving the tint stays out of a dry
//!    frame. The condition attached to it — that the next change to the `Frame`
//!    record answers the comparability question afresh, **by rendering rather
//!    than by classifying** — is the half a reader cannot reconstruct, because
//!    it says the *precedent does not transfer*.
//!
//! # The whole verdict in a fixed order, not a filtered list of misses
//!
//! The reading answers one entry per owed thing, in the order they are owed,
//! saying whether the page states it. A filtered list of what is missing is
//! weaker in a way this project has measured twice: a list compared by filtering
//! cannot see a member that was quietly dropped from the list itself. Here a
//! missing item flips its own entry and a re-ordering of what is owed changes
//! the vector, so the two are distinct failures.
//!
//! **What the shape cannot see, said rather than left out.** The list is closed,
//! so a page stating something *extra* is not an offence — a page saying more is
//! not a page saying less. And an item deleted from [`OWED`] takes its own
//! expectation with it, which no assertion here can catch; what stands against
//! that is that these five are the spec's own five, quoted above, and that the
//! doctoring control below drives every one of them.
//!
//! # Ordering by position in the page was considered and rejected
//!
//! Reading the order the page states them in would be the strongest form of
//! "read the list out of the observed text". It cannot be had honestly on this
//! page: `SCENE_REVISION` is discussed in six sections that predate this spec
//! across 2 241 lines, so the position of its first mention is a fact about the
//! re-shoot log rather than about anything owed here. Ordering on that would be
//! comparing noise, and a guard comparing noise is one somebody deletes.
//!
//! # A phrase match, held as loosely as a technical claim allows
//!
//! **When a guard and a document disagree, the one shaped wrongly is usually the
//! guard, because the document has a reader and the guard does not.** Where the
//! page has structure the sibling file reads the structure; these five claims
//! have none, so what is left is a phrase match, and it is held loose in the two
//! ways it can be: **emphasis and line breaks are normalised out of both sides**,
//! so a bolded phrase and one a paragraph wrapped are both free, and **case is
//! deliberately kept**, because `min(1, d / D)` lower-cased is `min(1, d / d)` —
//! which the *wrong* law, `min(1, D / d)`, also lower-cases to. Losing that
//! distinction would leave the guard green on a page stating the law backwards.

mod support;

use std::error::Error;
use std::fs;

use support::{TestResult, repository_root};

/// What the page owes, and the spellings it has to carry for each — **every one
/// of them**, since each claim is a conjunction rather than a choice of
/// wording.
///
/// The **first** spelling of each is the one distinctive to it, and the one the
/// doctoring control strikes out. The others are there to keep a claim from
/// being satisfied by a word the page already uses for something else.
const OWED: [(&str, &[&str]); 5] = [
    (
        "the law a surface's colour is carried toward the medium by",
        &["min(1, d / D)", "linear light"],
    ),
    (
        "that the distance is radial from the eye and not depth along the view",
        &["radial", "view direction"],
    ),
    (
        "that a pixel drawing no terrain is given the tint through the clear",
        &["through the clear", "no terrain"],
    ),
    (
        "that the HUD is not tinted, and that it composites in a later pass",
        &["HUD is not tinted", "later pass"],
    ),
    (
        "that the scene revision did not move, why, and what the next change owes",
        &[
            "SCENE_REVISION did not move",
            "byte-identical",
            "by rendering rather than by classifying",
        ],
    ),
];

/// What a reading of the page came to.
#[derive(Debug, PartialEq, Eq)]
enum WhatThePageOwes {
    /// One entry per thing owed, in the order they are owed, saying whether the
    /// page states it.
    ItemByItem(Vec<(&'static str, bool)>),
    /// There was no page to read, so nothing above could be said.
    ThePageWasNotRead,
}

#[test]
fn the_rendering_page_states_the_law_the_distance_the_clear_the_hud_and_the_revision() -> TestResult
{
    let reading = what_the_page_owes(&rendering_page()?);

    assert_eq!(
        reading,
        everything_stated(),
        "this is the as-built record, and the next medium is added against it. Every one of these \
         is a thing the code cannot say for itself: the law is a published content surface, the \
         radial measurement is the half a screenshot of the middle of the frame cannot tell apart \
         from the wrong one, the clear is why the sky stops being sky, the HUD's exclusion is a \
         consequence of pass order rather than a rule anybody wrote, and the revision that did not \
         move is the one decision whose *reason does not transfer* to the next change"
    );
    Ok(())
}

/// The control this scenario names, driven over the page itself.
///
/// A reading asserting only that everything was found goes green the day it
/// stops being able to find anything, so the same reading is driven over the
/// real page with each item's own spelling struck out of it, and each has to
/// come back as the one thing missing while the other four still read as
/// stated. That is what distinguishes a page short of one from a reading that
/// answers the whole list whatever is in front of it.
#[test]
fn the_rendering_page_with_one_of_them_struck_out_reports_that_one_and_the_rest_as_stated()
-> TestResult {
    let page = normalised(&rendering_page()?);

    let reported: Vec<WhatThePageOwes> = leading_spellings()
        .map(|spelling| what_the_page_owes(&page.replace(spelling, " ")))
        .collect();

    assert_eq!(
        reported,
        short_of_each_in_turn(),
        "each of the five is struck out of the real page in turn. A reading answering more than \
         the one that was taken away was never finding the rest of them in the page to begin \
         with; a reading answering fewer has an item it cannot see the absence of; and a reading \
         naming the wrong one sends whoever has to repair the page to the wrong paragraph"
    );
    Ok(())
}

/// The same discrimination, over a page written to carry every one of them.
///
/// It stands beside the reading above rather than behind it, because the two
/// fail for different reasons and one of them is red until the page is written.
/// **A test red for a known reason reports nothing about anything else**, so
/// while the page above is short of these, this is the only thing saying the
/// reading can tell one missing item from another at all.
#[test]
fn a_reading_driven_over_a_page_short_of_each_in_turn_reports_each_alone() -> TestResult {
    let whole = a_page_carrying_everything();

    let reported: Vec<WhatThePageOwes> = [what_the_page_owes(&whole)]
        .into_iter()
        .chain(
            leading_spellings().map(|spelling| what_the_page_owes(&whole.replace(spelling, " "))),
        )
        .collect();

    assert_eq!(
        reported,
        [everything_stated()]
            .into_iter()
            .chain(short_of_each_in_turn())
            .collect::<Vec<WhatThePageOwes>>(),
        "a reading that cannot answer `everything is stated` is one no page could ever satisfy, \
         and a reading that answers it whatever is in front of it is one no page could ever fail"
    );
    Ok(())
}

/// The vacuity control, and the reason the verdict is enumerated at all.
///
/// A page that has moved or been renamed states none of them, which is exactly
/// what a page short of all five looks like. The two must never compare equal:
/// one is a document to write and the other is a guard that has stopped being
/// able to look.
#[test]
fn a_reading_with_no_page_to_read_says_so_rather_than_reporting_a_page_short_of_all_five()
-> TestResult {
    let reading = what_the_page_owes("");

    assert_eq!(
        reading,
        WhatThePageOwes::ThePageWasNotRead,
        "an empty answer and an answer nobody could look for are different facts, and a guard that \
         cannot tell them apart reports a document to be rewritten when what has actually gone is \
         the guard's own reach"
    );
    Ok(())
}

/// What `page` states of the five, entry by entry and in the order they are
/// owed.
fn what_the_page_owes(page: &str) -> WhatThePageOwes {
    let flattened = normalised(page);
    if flattened.is_empty() {
        return WhatThePageOwes::ThePageWasNotRead;
    }
    WhatThePageOwes::ItemByItem(
        OWED.into_iter()
            .map(|(what, spellings)| {
                (
                    what,
                    spellings
                        .iter()
                        .all(|spelling| flattened.contains(spelling)),
                )
            })
            .collect(),
    )
}

/// The verdict a page carrying every one of them reads as.
fn everything_stated() -> WhatThePageOwes {
    WhatThePageOwes::ItemByItem(OWED.into_iter().map(|(what, _)| (what, true)).collect())
}

/// The verdict for each item struck out in turn, derived from [`OWED`] so an
/// item added to that list without a page short of it is a case nobody ever
/// drove the reading over.
fn short_of_each_in_turn() -> Vec<WhatThePageOwes> {
    (0..OWED.len())
        .map(|away| {
            WhatThePageOwes::ItemByItem(
                OWED.into_iter()
                    .enumerate()
                    .map(|(at, (what, _))| (what, at != away))
                    .collect(),
            )
        })
        .collect()
}

/// The spelling distinctive to each thing owed, which is the one a doctored
/// page has taken out of it.
fn leading_spellings() -> impl Iterator<Item = &'static str> {
    OWED.into_iter()
        .filter_map(|(_, spellings)| spellings.first().copied())
}

/// A page carrying every spelling of every one of them.
///
/// **Written to satisfy the reading, and that is legitimate here and nowhere
/// else.** A control fixture's whole purpose is to be the thing the reading
/// should answer for; the document itself is judged by the readings above.
fn a_page_carrying_everything() -> String {
    OWED.into_iter()
        .map(|(_, spellings)| spellings.join(", "))
        .collect::<Vec<String>>()
        .join(". ")
}

/// `docs/technical/rendering.md` as it stands, or nothing at all where there is
/// no such page.
///
/// **An unreadable page is the empty string rather than an error**, so a page
/// that has moved reaches the verdict's own arm for it instead of ending the
/// test before its assertion ran.
fn rendering_page() -> Result<String, Box<dyn Error>> {
    let at = repository_root()?
        .join("docs")
        .join("technical")
        .join("rendering.md");
    Ok(fs::read_to_string(at).unwrap_or_default())
}

/// `text` with the emphasis a reader sees and the line breaks a paragraph
/// carries taken out, so a phrase is looked for as a phrase.
///
/// **Case is kept.** `min(1, d / D)` and `min(1, D / d)` are the law and its
/// inverse, and lower-casing makes them the same string.
fn normalised(text: &str) -> String {
    text.replace(['*', '`'], "")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}
