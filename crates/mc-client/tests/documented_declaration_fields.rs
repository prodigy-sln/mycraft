//! What the modding pages tell a mod author a declaration may state, read out of
//! the pages themselves and compared whole.
//!
//! Two artefacts on those pages say what the fields are, and they say it in two
//! different registers. One is the **refusal** an unrecognised field raises,
//! pasted out of a real run, which quotes the whole list back; the other is the
//! page's own **table**, which names each field beside the value its absence
//! means. A reader meets whichever is nearer, so both have to be true and neither
//! is covered by the other.
//!
//! # Why this is a third binary beside the two documented-refusal guards
//!
//! `documented_refusals.rs` asks whether everything the pages quote is text the
//! client writes; `documented_property_refusals.rs` asks whether a named pair of
//! refusals is quoted at all. **Neither can see a list that is short.** The first
//! compares a page against a run, so it is silent until the run moves and it
//! reports the whole refusal rather than the list inside it; the second names two
//! refusals and is green over a page that quotes them both and lists nine fields
//! where the loader has eleven. What a field addition actually breaks is the list
//! inside a quotation and the row missing from a table, and that is what this
//! file reads.
//!
//! # Read whole and in order, never tested for membership
//!
//! `standards/global/testing.md` §2 records two mirrors of this same list held at
//! six while it grew to nine, neither reddening, because both compared by
//! *filtering* — one filtered its needles by presence in the observed text, the
//! other looked each observed item up and skipped what it could not rank. Both
//! answer "which of my names are there", which is exactly the answer a stale
//! mirror gives correctly. Every comparison below reads the list **out of the
//! artefact** and compares the whole thing in order, so a missing name, an extra
//! name and a reordering are three different failures.
//!
//! # A verdict, and a control on it
//!
//! A scan that read no page, or whose recogniser stopped matching, reports
//! "everything agrees" as loudly as a clean tree does. So the quotation half
//! answers with one of three arms and each test compares the whole of it, which
//! rejects "there was nothing to check" for free — and a second test drives the
//! reading over a page holding a doctored list, which is the only thing watching
//! the disagreeing arm come out of a real comparison. A directory that holds no
//! page at all fails the scan rather than borrowing an arm.

#[path = "support/built_set_refusals.rs"]
mod built_set_refusals;
#[path = "support/entry.rs"]
mod entry;
#[path = "support/input/mod.rs"]
mod input;
#[path = "support/launch_notices.rs"]
mod launch_notices;
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
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use quoted_refusals::{
    every_field_the_guide_states, fields_a_refusal_quotes, pages, quoted_refusals_in,
};
use support::TestResult;

/// The extension a page is written with.
const PAGE_EXTENSION: &str = "md";

/// The first cell of the header row of the table that names every field.
const FIELD_COLUMN: &str = "Field";

/// The header of the column stating what leaving a field out means.
const ABSENT_MEANS_COLUMN: &str = "Absent means";

/// The header of the column stating what values a field may hold.
const BOUND_COLUMN: &str = "Bound";

/// The field a mod author states to make their block one a player can swim in.
const SWIMMABLE_FIELD: &str = "swimmable";

/// The field a mod author states to make their block's volume slow what moves
/// through it.
const MOVE_RESISTANCE_FIELD: &str = "move_resistance";

/// The field a mod author states to say how fast their block's volume lifts a
/// swimmer who asks to rise.
const SWIM_ASCENT_FIELD: &str = "swim_ascent";

/// What the guide says leaving `swimmable` out means, written in the page's own
/// convention for a constant default.
const A_MISSING_SWIMMABLE_MEANS: &str = "`false`";

/// What the guide says leaving `move_resistance` out means.
///
/// Written with a point, because the field is a number and `0` would read as the
/// integer a declaration may also write — the retained value is a fraction and
/// the page says which.
const A_MISSING_RESISTANCE_MEANS: &str = "`0.0`";

/// What the guide says leaving `swim_ascent` out means.
///
/// **The one row in this table whose default is not the value an absent field
/// resembles.** Every other constant default here is what a declaration saying
/// nothing would have been given anyway — `false`, `0.0`, no residue — so a row
/// stating it is a courtesy. This one is the player's own jump speed, which is
/// neither zero nor derivable from anything else on the page, and an author left
/// to guess it will guess `0.0` and write a still pool where they meant to leave
/// the field out.
const A_MISSING_ASCENT_MEANS: &str = "`9.0`";

/// What the guide states bounds a resistance, whole.
///
/// The ceiling in that sentence is the half nothing could see: a page edited to
/// promise `1e40` would be lying about a loader that is perfectly correct, and
/// every other test in the workspace would stay green. `luau_declaration_medium_ceiling.rs`
/// pins where the loader actually stops; this pins where the page says it does,
/// and neither can stand in for the other.
///
/// The figure is deliberately a little under `f32::MAX`, which is
/// `3.4028234663852886e38`. The page names a round number a reader can type
/// rather than the exact last value that narrows, and under-promising is the
/// safe direction for a bound — so a later edit raising it to the exact figure
/// is a decision somebody should have to make on purpose, which is what this
/// reddening would ask of them.
const A_RESISTANCE_IS_BOUNDED_BY: &str = "not less than zero, and at most `3.4e38`";

/// One page's name beside the recognised list one of its quotations introduces.
type QuotedList = (String, Vec<String>);

/// One row of the guide's field table: the field it names, what it says leaving
/// that field out means, and the bound it states on the value.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TabulatedRow {
    field: String,
    absent_means: String,
    bound: String,
}

/// What the pages quote back as the fields a declaration may state.
///
/// Three arms rather than a list and a count, so "every quotation lists them all"
/// and "no quotation lists anything" can never compare equal, and so a scan whose
/// recogniser stopped matching reports that rather than borrowing the good
/// answer.
#[derive(Debug, PartialEq, Eq)]
enum WhatThePagesQuoteBack {
    /// Every quoted refusal that introduces the list quotes every field a
    /// declaration may state, in the order the guide states them.
    EveryQuotationListsEveryFieldInOrder,
    /// This page quotes this list, and it is not that one.
    ThisPageListsOtherwise { page: String, quoted: Vec<String> },
    /// The pages were read and none of their quotations introduces the list at
    /// all.
    NoQuotationListsTheFields,
}

/// Every field the guide's own table names, in the order it names them, beside
/// what it says leaving either medium field out means.
///
/// One record rather than three readings, so a page that gained both rows in the
/// wrong place is not mistaken for one that gained them correctly, and so a
/// mismatch shows the whole table's field order beside the two cells the medium
/// is documented in. `None` is a row the table does not carry, which is a
/// different fact from a row carrying the wrong value.
#[derive(Debug, PartialEq, Eq)]
struct WhatTheGuideTabulates {
    fields_in_order: Vec<String>,
    what_a_missing_swimmable_means: Option<String>,
    what_a_missing_resistance_means: Option<String>,
    what_a_missing_ascent_means: Option<String>,
}

/// Every page under `directory`, at any depth.
///
/// # Errors
///
/// Returns an error if the directory cannot be walked.
fn pages_under(directory: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(pages_under(&path)?);
        } else if path.extension() == Some(OsStr::new(PAGE_EXTENSION)) {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

/// How a page names itself in a verdict.
///
/// The file name alone, never the path: a path renders with OS-specific
/// separators and an expectation carrying one would be a Windows-only or
/// Unix-only test.
fn named(page: &Path) -> String {
    page.file_name()
        .unwrap_or_else(|| OsStr::new("a page with no name"))
        .to_string_lossy()
        .into_owned()
}

/// Every recognised list a quotation on `text` introduces, attributed to `page`.
///
/// A quotation that introduces none is passed over rather than reported as an
/// empty list: a refusal about something else quotes nothing to compare, and
/// letting it answer would put a verdict about the field list on a refusal that
/// was never about it.
fn lists_quoted_on(text: &str, page: &str) -> Vec<QuotedList> {
    quoted_refusals_in(text)
        .iter()
        .map(|refusal| fields_a_refusal_quotes(refusal))
        .filter(|quoted| !quoted.is_empty())
        .map(|quoted| (page.to_owned(), quoted))
        .collect()
}

/// Every recognised list quoted under `directory`, in page order.
///
/// # Errors
///
/// Returns an error if the directory cannot be walked or holds no page — neither
/// is one of the three arms, because a scan that could not look is not entitled
/// to one.
fn lists_quoted_under(directory: &Path) -> Result<Vec<QuotedList>, Box<dyn Error>> {
    let read = pages_under(directory)?;
    if read.is_empty() {
        return Err(format!(
            "no page was read under {}, so nothing below could be said about what they quote",
            directory.display()
        )
        .into());
    }
    let mut listed = Vec::new();
    for page in &read {
        listed.extend(lists_quoted_on(&fs::read_to_string(page)?, &named(page)));
    }
    Ok(listed)
}

/// What the pages under `directory` quote back as the fields a declaration may
/// state.
///
/// # Errors
///
/// Returns an error for the reason [`lists_quoted_under`] does.
fn what_the_pages_quote_back(directory: &Path) -> Result<WhatThePagesQuoteBack, Box<dyn Error>> {
    let listed = lists_quoted_under(directory)?;
    let expected = every_field_the_guide_states();
    match listed.iter().find(|(_, quoted)| *quoted != expected) {
        Some((page, quoted)) => Ok(WhatThePagesQuoteBack::ThisPageListsOtherwise {
            page: page.clone(),
            quoted: quoted.clone(),
        }),
        None if listed.is_empty() => Ok(WhatThePagesQuoteBack::NoQuotationListsTheFields),
        None => Ok(WhatThePagesQuoteBack::EveryQuotationListsEveryFieldInOrder),
    }
}

/// The cells of one markdown table row, trimmed.
fn cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

/// Whether `line` is the header row of the table that names every field.
fn is_the_field_table_header(line: &str) -> bool {
    let header = cells(line);
    header.first().is_some_and(|first| first == FIELD_COLUMN)
        && header.iter().any(|cell| cell == ABSENT_MEANS_COLUMN)
        && header.iter().any(|cell| cell == BOUND_COLUMN)
}

/// Every field the table starting at `header` names, beside what it says leaving
/// each out means, in the order the table names them.
fn tabulated(header: &str, rows: &mut dyn Iterator<Item = &str>) -> Vec<TabulatedRow> {
    let heading = cells(header);
    let at = |wanted: &str| heading.iter().position(|cell| cell == wanted);
    let (Some(absent_at), Some(bound_at)) = (at(ABSENT_MEANS_COLUMN), at(BOUND_COLUMN)) else {
        return Vec::new();
    };
    rows.take_while(|line| line.trim_start().starts_with('|'))
        .map(cells)
        .filter(|row| row.first().is_some_and(|first| first.starts_with('`')))
        .filter_map(|row| {
            Some(TabulatedRow {
                field: row.first()?.trim_matches('`').to_owned(),
                absent_means: row.get(absent_at)?.clone(),
                bound: row.get(bound_at)?.clone(),
            })
        })
        .collect()
}

/// Every table on `text` that names a declaration's fields beside what leaving
/// each out means.
fn field_tables_in(text: &str) -> Vec<Vec<TabulatedRow>> {
    let mut tables = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if is_the_field_table_header(line) {
            // The alignment row between the header and the first field.
            lines.next();
            tables.push(tabulated(line, &mut lines));
        }
    }
    tables
}

/// Every such table under `directory`.
///
/// # Errors
///
/// Returns an error if the directory cannot be walked or a page cannot be read.
fn field_tables_under(directory: &Path) -> Result<Vec<Vec<TabulatedRow>>, Box<dyn Error>> {
    let mut tables = Vec::new();
    for page in pages_under(directory)? {
        tables.extend(field_tables_in(&fs::read_to_string(&page)?));
    }
    Ok(tables)
}

/// The one table under `directory` that names what a declaration may state.
///
/// # Errors
///
/// Returns an error unless exactly one page carries that table: none means the
/// reading has nothing to say and must not answer as though the table were empty,
/// and a second one means this reading would be silently choosing between two
/// statements of the same contract. A table whose `Bound` column has been renamed
/// is found by nothing and reaches the same error, which is the loud failure
/// rather than a silently empty bound.
fn the_field_table(directory: &Path) -> Result<Vec<TabulatedRow>, Box<dyn Error>> {
    let tables = field_tables_under(directory)?;
    let [table] = tables.as_slice() else {
        return Err(format!(
            "exactly one page under {} must tabulate what a declaration may state, and {} do",
            directory.display(),
            tables.len()
        )
        .into());
    };
    Ok(table.clone())
}

/// The bound the guide states on a resistance, or `None` where its table carries
/// no such row.
///
/// # Errors
///
/// Returns an error for the reason [`the_field_table`] does.
fn what_bounds_a_resistance(directory: &Path) -> Result<Option<String>, Box<dyn Error>> {
    Ok(the_field_table(directory)?
        .into_iter()
        .find(|row| row.field == MOVE_RESISTANCE_FIELD)
        .map(|row| row.bound))
}

/// What the guide's field table says about the fields it names.
///
/// # Errors
///
/// Returns an error for the reason [`the_field_table`] does.
fn what_the_guide_tabulates(directory: &Path) -> Result<WhatTheGuideTabulates, Box<dyn Error>> {
    let table = the_field_table(directory)?;
    let stated = |wanted: &str| {
        table
            .iter()
            .find(|row| row.field == wanted)
            .map(|row| row.absent_means.clone())
    };
    Ok(WhatTheGuideTabulates {
        fields_in_order: table.iter().map(|row| row.field.clone()).collect(),
        what_a_missing_swimmable_means: stated(SWIMMABLE_FIELD),
        what_a_missing_resistance_means: stated(MOVE_RESISTANCE_FIELD),
        what_a_missing_ascent_means: stated(SWIM_ASCENT_FIELD),
    })
}

/// A page under `directory` quoting a refusal whose recognised list is `listed`.
fn a_page_quoting(directory: &Path, listed: &[String]) -> Result<(), Box<dyn Error>> {
    let quoted = listed
        .iter()
        .map(|field| format!("`{field}`"))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        directory.join("a-doctored-page.md"),
        format!(
            "# What you may write\n\n```text\nmycraft: the shipped content could not be read: \
             block `example:amber`, field `slid`: `slid` is not a field a declaration may \
             state; a declaration may state {quoted}\n```\n"
        ),
    )?;
    Ok(())
}

#[test]
fn every_refusal_the_modding_pages_quote_lists_every_field_a_declaration_may_state() -> TestResult {
    let quoted_back = what_the_pages_quote_back(&pages()?)?;

    assert_eq!(
        quoted_back,
        WhatThePagesQuoteBack::EveryQuotationListsEveryFieldInOrder,
        "the list a refusal quotes back is the only place a mod author can read what a \
         declaration may say, and every page that pastes one is repeating that promise. A page \
         left holding the list as it stood before a field was added teaches an author that \
         their new field does not exist — and it does so from a fenced block that looks like it \
         came out of a real run, which is the one thing on the page a reader will not doubt. \
         The list is read out of each quotation and compared whole and in order, so a name that \
         is missing, one that should not be there and two in the wrong order are three \
         different failures rather than one silent pass"
    );
    Ok(())
}

#[test]
fn a_page_quoting_a_list_a_field_short_is_reported_with_the_list_it_quotes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut short = every_field_the_guide_states();
    short.pop();
    a_page_quoting(directory.path(), &short)?;

    let quoted_back = what_the_pages_quote_back(directory.path())?;

    assert_eq!(
        quoted_back,
        WhatThePagesQuoteBack::ThisPageListsOtherwise {
            page: "a-doctored-page.md".to_owned(),
            quoted: short,
        },
        "the control on the guard above, and the only thing that watches its disagreeing arm \
         come out of a real comparison. A reading that had come to answer `everything agrees` \
         — a recogniser that stopped finding the list, a comparison that normalised both sides \
         into agreement — would leave that guard green over any tree at all, and the drift \
         would then arrive as somebody's bug report about a field the documentation never \
         mentioned. The list it found travels in the verdict so that whoever repairs a page can \
         see what it says rather than only that it is wrong"
    );
    Ok(())
}

#[test]
fn the_guide_tabulates_every_field_with_the_value_its_absence_means() -> TestResult {
    let tabulated = what_the_guide_tabulates(&pages()?)?;

    assert_eq!(
        tabulated,
        WhatTheGuideTabulates {
            fields_in_order: every_field_the_guide_states(),
            what_a_missing_swimmable_means: Some(A_MISSING_SWIMMABLE_MEANS.to_owned()),
            what_a_missing_resistance_means: Some(A_MISSING_RESISTANCE_MEANS.to_owned()),
            what_a_missing_ascent_means: Some(A_MISSING_ASCENT_MEANS.to_owned()),
        },
        "a mod author writing their first declaration reads this table rather than a refusal, \
         so a field that exists in the loader and not in the table is a field nobody will use — \
         and the value its absence means is half of what the row is for, because a field whose \
         default a reader has to guess is one they will state redundantly on every block to be \
         safe. All three medium rows state a **constant**: none is `whatever you wrote for \
         `solid``, \
         which is what the three rows above them say and what would make every solid block in \
         the game something a player can swim inside. The order is the guide's own and is \
         compared whole, for the reason the quotation above is. The ascent's row carries the \
         constant that is hardest to guess and worst to guess wrong: it is the player's jump \
         speed rather than the zero every other default on this page resembles, so a row that \
         omits it teaches an author their liquid lifts nobody unless they say otherwise"
    );
    Ok(())
}

#[test]
fn the_guide_states_the_bound_a_resistance_is_kept_within() -> TestResult {
    let bound = what_bounds_a_resistance(&pages()?)?;

    assert_eq!(
        bound,
        Some(A_RESISTANCE_IS_BOUNDED_BY.to_owned()),
        "the page-side twin of the loader pair in `luau_declaration_medium_ceiling.rs`, and the          two guard opposite directions of one promise. That pair asks whether the loader still          stops where the page says; this asks whether the page still says where the loader          stops. Nothing could see the second before: an edit making this cell read `at most          1e40` leaves a correct loader, a lying page and every other test in the workspace          green — which is precisely how a mod author comes to write a number that is refused by          the thing that documented it. The cell is compared whole rather than searched for a          figure, so a floor quietly dropped from the sentence is as visible as a ceiling quietly          raised"
    );
    Ok(())
}
