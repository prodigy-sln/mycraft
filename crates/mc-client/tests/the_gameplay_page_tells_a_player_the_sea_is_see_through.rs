//! What `docs/user/gameplay.md` owes a player about the sea, and which of those
//! things is missing when one is.
//!
//! # A documentation scenario without a guard is a scenario with no falsifier
//!
//! The requirement fires when the page is *read*, which means prose alone
//! satisfies it — and prose can be deleted tomorrow with nothing going red. So
//! the page is read by a test, and the verdict is a **total enumeration** rather
//! than an absence: `EverythingAPlayerNeeds` rejects every other answer,
//! including "there was no page to read", which an emptiness check cannot.
//!
//! # Four items, reported apart, because that is what a reader can act on
//!
//! A player is owed four different things and they fail independently. A page
//! saying the water is see-through and naming nothing behind it has told them a
//! property; a page naming the lakebed in a paragraph about swimming has told
//! them a place. Only both together say *what to go and look at*. A boolean
//! verdict would send whoever has to fix it back to the page to work out which
//! half was missing, so the verdict names the half.
//!
//! **The last two are what a submerged eye adds, and they fail apart for the
//! same reason.** A page saying the view turns the sea's own colour once your
//! head is under has told a player what changes; it has not told them that the
//! sea is deep enough for it in one part of itself and nowhere else, so a
//! player who wades in at the beach never sees it and reads the page as wrong.
//! The eye stands 1.62 blocks over the feet and only the two-block-deep columns
//! put it under the surface — 0.38 blocks under — so *where* is as much of the
//! capability as *what*.
//!
//! # The controls are driven over the page itself, not only over a fixture
//!
//! A synthetic page carrying everything proves the reading *can* answer
//! `EverythingAPlayerNeeds`; it proves nothing about whether the reading finds
//! these things in the document a player actually reads. So the same reading is
//! driven over the real page with each item's spellings struck out of it, and
//! each has to be reported alone. A reading that located an item in a fixture
//! and could not locate it in the page would go green on the fixture forever.
//!
//! # This is a phrase match, and that is a patch rather than a fix
//!
//! **When a guard and a document disagree, the one shaped wrongly is usually the
//! guard, because the document has a reader and the guard does not.** Where a
//! page has structure — a table, a coordinate, a figure — the repair is to read
//! the structure and leave wording free. A paragraph of player-facing prose has
//! none, so what is left is a phrase match, and the two things that keep it from
//! editing the document are done here rather than argued for:
//!
//! - **each item is a set of spellings and not one phrase**, so an author may
//!   say see-through, transparent or translucent, and may name a lakebed, a
//!   seabed, a riverbed or a sea floor;
//! - **both sides are normalised first** — emphasis markers out, hyphens made
//!   spaces, line breaks collapsed — so a bolded word, a hyphen the author did
//!   or did not type, and a phrase a paragraph happened to wrap across two lines
//!   are all free.
//!
//! What it still cannot see is a page that states these things *far apart*, and
//! that residual is recorded rather than closed: requiring them in one paragraph
//! would be the guard dictating the shape of the page, and the scenarios ask for
//! namings rather than for one sentence.

mod support;

use std::error::Error;
use std::fs;

use support::{TestResult, repository_root};

/// What the page owes a player, and the spellings each is looked for by.
///
/// **Normalised spellings**, so every needle is written the way [`normalised`]
/// leaves the page: lower case, no emphasis, hyphens as spaces.
///
/// The last two are the ones a medium the eye stands in adds. Neither names a
/// number a player would have to care about beyond the one that decides where
/// to walk: the depth is *why* there is a place to go, and a page stating the
/// change without it has described something most players will never reach.
const OWED: [(&str, &[&str]); 4] = [
    (
        "that the water is see through",
        &["see through", "transparent", "translucent"],
    ),
    (
        "something a player can see through it from the shore",
        &[
            "lakebed",
            "lake bed",
            "seabed",
            "sea bed",
            "seafloor",
            "sea floor",
            "riverbed",
            "river bed",
        ],
    ),
    (
        "that going under the sea changes what a player sees",
        &[
            "head goes under",
            "head is under",
            "once you are under",
            "when you are under",
            "underwater",
            "under the surface",
        ],
    ),
    (
        "where the sea is deep enough for a player to get under it",
        &["two blocks deep", "two voxels deep", "two cells deep"],
    ),
];

/// What a reading of the gameplay page came to.
#[derive(Debug, PartialEq, Eq)]
enum WhatThePageOwes {
    /// Every one of them, so a player is told each property and where to go
    /// and see it.
    EverythingAPlayerNeeds,
    /// The page carries none of these.
    NotStated(Vec<&'static str>),
    /// There was no page to read, so nothing above could be said.
    ThePageWasNotRead,
}

#[test]
fn the_gameplay_page_names_the_water_as_see_through_and_names_something_seen_through_it()
-> TestResult {
    let reading = what_the_page_owes(&gameplay_page()?);

    assert_eq!(
        reading,
        WhatThePageOwes::EverythingAPlayerNeeds,
        "this is the only place the capability is stated in words a player reads, and it is the \
         half of the spec that reaches them at all. A page that says the sea is see-through and \
         names nothing behind it has described a property nobody knows where to go and look at; \
         one that names the lakebed and never says the water passes light has named a place with \
         no reason to walk to it. And a page saying the world turns the sea's own colour once a \
         head goes under, without saying that only the two-deep part of the sea is deep enough \
         for that, has told a player about something they will wade past and never see"
    );
    Ok(())
}

#[test]
fn a_page_short_of_any_of_them_reports_which_ones_it_is_short_of() -> TestResult {
    let reported: Vec<WhatThePageOwes> = a_page_short_of_each()
        .iter()
        .map(|(_, page)| what_the_page_owes(page))
        .collect();

    assert_eq!(
        reported,
        a_page_short_of_each()
            .iter()
            .map(|(short, _)| WhatThePageOwes::NotStated(short.clone()))
            .collect::<Vec<WhatThePageOwes>>(),
        "the same reading is driven over a page carrying all of them and then over that page with \
         each taken away in turn, with the two a submerged eye added taken away together, and \
         with every one of them gone. A reading that answered the whole list whatever was missing \
         would satisfy the last of those and none of the others, which is why they stand here \
         beside it — and a verdict nobody can act on is what sends whoever has to fix the page \
         back to read it themselves"
    );
    Ok(())
}

/// The control driven over the page a player actually reads, rather than over a
/// fixture written to satisfy the reading.
///
/// The fixture control above proves the reading can tell one missing item from
/// another; it cannot prove the reading *finds* these things where they are
/// really written. So the real page is read, each item's spellings are struck
/// out of it in turn, and each has to come back reported alone. A reading that
/// located an item only in prose of its own writing would pass every test above
/// it, and this is what says so.
#[test]
fn the_same_reading_over_the_real_page_with_one_thing_struck_out_reports_that_one() -> TestResult {
    let page = normalised(&gameplay_page()?);

    let reported: Vec<WhatThePageOwes> = OWED
        .into_iter()
        .map(|(_, spellings)| what_the_page_owes(&struck_from(&page, spellings)))
        .collect();

    assert_eq!(
        reported,
        OWED.into_iter()
            .map(|(what, _)| WhatThePageOwes::NotStated(vec![what]))
            .collect::<Vec<WhatThePageOwes>>(),
        "each of these is struck out of the real page in turn, and each has to come back as the \
         one thing reported: a reading answering more than the item that was taken away is one \
         that was never finding the rest of them in the page to begin with"
    );
    Ok(())
}

/// The vacuity control, and the reason the verdict is enumerated at all.
///
/// A page that has moved or been renamed carries none of them, which is exactly
/// what a page short of all of them looks like. The two must never compare
/// equal: one is a document to fix and the other is a guard that has stopped
/// being able to look.
#[test]
fn a_reading_with_no_page_to_read_says_so_rather_than_reporting_a_page_short_of_all() -> TestResult
{
    let reading = what_the_page_owes("");

    assert_eq!(
        reading,
        WhatThePageOwes::ThePageWasNotRead,
        "an empty answer and an answer nobody could look for are different facts, and a guard that \
         cannot tell them apart reports a page to be rewritten when what has actually gone is the \
         guard's own reach"
    );
    Ok(())
}

/// What `page` states of the two things a player is owed.
fn what_the_page_owes(page: &str) -> WhatThePageOwes {
    let flattened = normalised(page);
    if flattened.is_empty() {
        return WhatThePageOwes::ThePageWasNotRead;
    }
    let missing: Vec<&'static str> = OWED
        .into_iter()
        .filter(|(_, spellings)| {
            !spellings
                .iter()
                .any(|spelling| flattened.contains(spelling))
        })
        .map(|(what, _)| what)
        .collect();
    if missing.is_empty() {
        WhatThePageOwes::EverythingAPlayerNeeds
    } else {
        WhatThePageOwes::NotStated(missing)
    }
}

/// A page carrying all of them, with each taken away in turn, then with the two
/// a submerged eye added taken away together, then with every one gone — beside
/// what each is short of.
///
/// **Derived from [`OWED`] rather than written out**, so an item added to that
/// list without a page short of it is a case nobody ever drove the reading over.
fn a_page_short_of_each() -> Vec<(Vec<&'static str>, String)> {
    let each_alone = (0..OWED.len()).map(|at| vec![at]);
    let the_two_a_submerged_eye_added = vec![OWED.len() - 2, OWED.len() - 1];
    let every_one = (0..OWED.len()).collect::<Vec<usize>>();
    each_alone
        .chain([the_two_a_submerged_eye_added, every_one])
        .map(|away| (short_of(&away), a_page_stating(&stating_all_but(&away))))
        .collect()
}

/// What a page missing the items at `away` is short of.
fn short_of(away: &[usize]) -> Vec<&'static str> {
    away.iter()
        .filter_map(|at| OWED.get(*at).map(|(what, _)| *what))
        .collect()
}

/// Which of [`OWED`] a page states, when the items at `away` are the ones taken
/// out of it.
fn stating_all_but(away: &[usize]) -> Vec<bool> {
    (0..OWED.len()).map(|at| !away.contains(&at)).collect()
}

/// `page` with every spelling of one thing taken out of it.
///
/// Applied to a page already normalised, so a spelling is struck in the one form
/// the reading looks for it in.
fn struck_from(page: &str, spellings: &[&str]) -> String {
    spellings.iter().fold(page.to_owned(), |left, spelling| {
        left.replace(spelling, " ")
    })
}

/// A player-facing paragraph stating whichever of the two `stated` says it does.
///
/// **Written to satisfy the reading, and that is legitimate here and nowhere
/// else.** A control fixture's whole purpose is to be the thing the reading
/// should pass over or report; the page itself is judged by the reading above.
fn a_page_stating(stated: &[bool]) -> String {
    let mut page = String::from("## What the world looks like\n\nThe sea is water.\n");
    for ((_, spellings), states) in OWED.into_iter().zip(stated.iter().copied()) {
        let Some(spelling) = spellings.first().filter(|_| states) else {
            continue;
        };
        page.push_str(&format!("Also, the {spelling} of it.\n"));
    }
    page
}

/// `docs/user/gameplay.md` as it stands, or nothing at all where there is no
/// such page.
///
/// **An unreadable page is the empty string rather than an error**, so a page
/// that has moved reaches the verdict's own arm for it instead of ending the
/// test before its assertion ran.
fn gameplay_page() -> Result<String, Box<dyn Error>> {
    let at = repository_root()?
        .join("docs")
        .join("user")
        .join("gameplay.md");
    Ok(fs::read_to_string(at).unwrap_or_default())
}

/// `text` with the emphasis a reader sees, the hyphens an author may or may not
/// type, and the line breaks a paragraph carries taken out, so a phrase is
/// looked for as a phrase.
fn normalised(text: &str) -> String {
    text.replace(['*', '`', '_'], "")
        .replace('-', " ")
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}
