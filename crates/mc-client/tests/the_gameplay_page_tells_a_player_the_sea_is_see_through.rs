//! What `docs/user/gameplay.md` owes a player about the sea, and which half of
//! it is missing when one is.
//!
//! # A documentation scenario without a guard is a scenario with no falsifier
//!
//! The requirement fires when the page is *read*, which means prose alone
//! satisfies it — and prose can be deleted tomorrow with nothing going red. So
//! the page is read by a test, and the verdict is a **total enumeration** rather
//! than an absence: `BothThingsAPlayerNeeds` rejects every other answer,
//! including "there was no page to read", which an emptiness check cannot.
//!
//! # Two items, reported apart, because that is what a reader can act on
//!
//! A player is owed two different things and they fail independently. A page
//! saying the water is see-through and naming nothing behind it has told them a
//! property; a page naming the lakebed in a paragraph about swimming has told
//! them a place. Only both together say *what to go and look at*. A boolean
//! verdict would send whoever has to fix it back to the page to work out which
//! half was missing, so the verdict names the half.
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
//! What it still cannot see is a page that states the two things *far apart*,
//! and that residual is recorded rather than closed: requiring them in one
//! paragraph would be the guard dictating the shape of the page, and the
//! scenario asks for two namings rather than for one sentence.

mod support;

use std::error::Error;
use std::fs;

use support::{TestResult, repository_root};

/// What the page owes a player, and the spellings each is looked for by.
///
/// **Normalised spellings**, so every needle is written the way [`normalised`]
/// leaves the page: lower case, no emphasis, hyphens as spaces.
const OWED: [(&str, &[&str]); 2] = [
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
];

/// What a reading of the gameplay page came to.
#[derive(Debug, PartialEq, Eq)]
enum WhatThePageOwes {
    /// Both, so a player is told the property and where to go and see it.
    BothThingsAPlayerNeeds,
    /// The page carries neither of these.
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
        WhatThePageOwes::BothThingsAPlayerNeeds,
        "this is the only place the capability is stated in words a player reads, and it is the \
         half of the spec that reaches them at all. A page that says the sea is see-through and \
         names nothing behind it has described a property nobody knows where to go and look at; \
         one that names the lakebed and never says the water passes light has named a place with \
         no reason to walk to it"
    );
    Ok(())
}

#[test]
fn a_page_naming_neither_reports_which_of_the_two_it_is_short_of() -> TestResult {
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
        "the same reading is driven over a page carrying both and then over that page with each \
         taken away in turn and with both taken away together. A reading that answered the whole \
         list whatever was missing would satisfy the last of the three and none of the first two, \
         which is why they are here beside it — and a verdict nobody can act on is what sends \
         whoever has to fix the page back to read it themselves"
    );
    Ok(())
}

/// The vacuity control, and the reason the verdict is enumerated at all.
///
/// A page that has moved or been renamed carries neither thing, which is exactly
/// what a page short of both looks like. The two must never compare equal: one
/// is a document to fix and the other is a guard that has stopped being able to
/// look.
#[test]
fn a_reading_with_no_page_to_read_says_so_rather_than_reporting_a_page_short_of_both() -> TestResult
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
        WhatThePageOwes::BothThingsAPlayerNeeds
    } else {
        WhatThePageOwes::NotStated(missing)
    }
}

/// A page carrying both, with each taken away in turn and then with both gone,
/// beside what each is short of.
fn a_page_short_of_each() -> Vec<(Vec<&'static str>, String)> {
    vec![
        (vec![OWED[0].0], a_page_stating([false, true])),
        (vec![OWED[1].0], a_page_stating([true, false])),
        (vec![OWED[0].0, OWED[1].0], a_page_stating([false, false])),
    ]
}

/// A player-facing paragraph stating whichever of the two `stated` says it does.
///
/// **Written to satisfy the reading, and that is legitimate here and nowhere
/// else.** A control fixture's whole purpose is to be the thing the reading
/// should pass over or report; the page itself is judged by the reading above.
fn a_page_stating(stated: [bool; 2]) -> String {
    let mut page = String::from("## What the world looks like\n\nThe sea is water.\n");
    for ((_, spellings), states) in OWED.into_iter().zip(stated) {
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
