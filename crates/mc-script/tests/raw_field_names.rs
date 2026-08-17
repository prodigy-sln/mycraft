//! Asking a table script handed the host **which keys it holds**, without
//! running any of the mod's code to find out.
//!
//! Reading a named field is already raw (`raw_field_reads.rs`). This is the
//! other half of the same question and it meets a different metamethod: a host
//! that can read `solid` but cannot ask *what fields exist* can never tell a
//! typo from an absence, so a misspelled key becomes a silently lost
//! declaration. What that enumeration is exposed to is `__iter` and `__len`,
//! never `__index` — and both are script the table's author chose.
//!
//! # Four properties, and each one has a test here or a scenario next door
//!
//! **Raw** is held by the loader's own scenarios, where a declaration carrying
//! an `__iter` that hides a field must still be refused for it and one carrying
//! an `__iter` that invents a field must still register. `__len` has no scenario
//! and gets one below, because it is the reachable half: measured on this
//! toolchain, `mlua`'s `Table::pairs` is already raw against `__iter` and
//! `__pairs`, while `Table::len` **honours `__len`** and answered `0` for a
//! table holding three keys. An enumeration that sized itself with that call
//! would hand back nothing and report a declaration as carrying no fields at
//! all.
//!
//! **Bounded, with the bound as a parameter.** The bound has to bind *inside*
//! the enumeration: a `Vec<String>` filled first and measured afterwards has
//! already allocated the hundred-thousand-key table the bound exists to refuse.
//! So a table over the bound is reported without its names being copied out, and
//! the test below pins both sides of the limit at once — a bound stated only
//! from the refusing side leaves `>` and `>=` indistinguishable.
//!
//! **Total over key types.** A table may be keyed by anything, and a key that is
//! not a string is still a field a declaration carries. Skipping it would make
//! "a field the loader does not recognise is refused" a promise that holds only
//! for the keys somebody thought of.
//!
//! **Sorted lexicographically, here rather than at the caller.** The state's own
//! key order carries no information — measured, a five-key declaration comes
//! back `solid, name, slid, replacable, texture`, which is neither the order it
//! was written in nor any order a mod author could predict. A refusal that named
//! whichever unrecognised field came back first would be text nobody can quote
//! in a document, and the documentation-agreement guard compares a quoted
//! refusal against a real run line for line.
//!
//! # Where the expectations come from
//!
//! The rendering of a non-string key is not written down here. The chunk prints
//! the same values it uses as keys, and the expectation is assembled from what
//! the host recorded — so the test states the contract ("the same rendering
//! `print` uses") rather than a transcript of one run of the thing under test.

use std::error::Error;
use std::num::NonZeroUsize;

use mc_script::{FieldNames, ScriptHost, ScriptTable, ScriptValue};

type TestResult = Result<(), Box<dyn Error>>;

/// A declaration whose keys, in the state's own order, are neither the order
/// they were written in nor lexicographic order.
///
/// Measured: the state hands them back `solid, name, slid, replacable,
/// texture`. Both misspellings sit between the correctly spelled fields, and
/// `slid` precedes `replacable` — the reverse of the order this file requires —
/// so an enumeration returning the state's order has nowhere to be right.
const A_DECLARATION_WITH_TWO_MISSPELLINGS: &str = "return {\n\
     \tslid = true,\n\
     \treplacable = true,\n\
     \tname = 'example:amber',\n\
     \ttexture = 'example:quartz',\n\
     \tsolid = true,\n\
     }\n";

/// How many keys [`A_DECLARATION_WITH_TWO_MISSPELLINGS`] holds.
///
/// Counted from the chunk above rather than read back from a run, so a bound
/// asserted against it is asserted against the fixture and not against the
/// implementation.
const KEYS_THAT_DECLARATION_HOLDS: usize = 5;

/// A table keyed by three things that are not strings, beside one that is.
///
/// It prints each non-string key before returning, so the expected rendering is
/// the host's own and is never written down here.
const A_TABLE_KEYED_BY_MORE_THAN_STRINGS: &str = "print(1)\n\
     print(2.5)\n\
     print(true)\n\
     return {\n\
     \t[1] = 'first',\n\
     \t[2.5] = 'halfway',\n\
     \t[true] = 'affirmative',\n\
     \tname = 'example:amber',\n\
     }\n";

/// The one string key [`A_TABLE_KEYED_BY_MORE_THAN_STRINGS`] holds.
const THE_ONE_STRING_KEY: &str = "name";

/// A table of three entries whose `__len` reports that it has none.
///
/// The array part is what makes this reachable: measured, `Table::raw_len`
/// answers 3 here and `Table::len` answers 0, because the second consults the
/// metamethod. A host that sized its enumeration with the second reports a
/// declaration carrying no fields whatever it actually holds.
const A_TABLE_THAT_UNDERSTATES_ITS_OWN_LENGTH: &str = "return setmetatable({ 'first', 'second', 'third' }, {\n\
     \t__len = function() return 0 end,\n\
     })\n";

/// The keys that table holds, as the enumeration must render them.
fn the_three_positions() -> Vec<String> {
    vec!["1".to_owned(), "2".to_owned(), "3".to_owned()]
}

/// A bound of `count`, for a caller that will not pass zero.
fn most(count: usize) -> Result<NonZeroUsize, Box<dyn Error>> {
    NonZeroUsize::new(count).ok_or_else(|| "a field-count bound of zero allows nothing".into())
}

/// A host under the shipped limits, and the table `source` returns.
///
/// Every fixture here is a declaration-shaped chunk, so the host is the shipped
/// one: a test-sized limit tripping first would answer a different question.
fn host_holding(source: &str) -> Result<(ScriptHost, ScriptTable), Box<dyn Error>> {
    let mut host = ScriptHost::new()?;
    match host.evaluate("amber.luau", source) {
        Ok(ScriptValue::Table(table)) => Ok((host, table)),
        Ok(other) => {
            Err(format!("this fixture was written to return a table, not {other:?}").into())
        }
        Err(fault) => Err(format!("the fixture chunk did not evaluate: {fault}").into()),
    }
}

/// The names in `unsorted`, lexicographically ordered.
fn sorted(unsorted: &[String]) -> Vec<String> {
    let mut names = unsorted.to_vec();
    names.sort();
    names
}

#[test]
fn every_key_a_table_holds_comes_back_lexicographically_ordered() -> TestResult {
    let (host, declaration) = host_holding(A_DECLARATION_WITH_TWO_MISSPELLINGS)?;

    assert_eq!(
        host.field_names(&declaration, most(KEYS_THAT_DECLARATION_HOLDS)?),
        FieldNames::Enumerated(vec![
            "name".to_owned(),
            "replacable".to_owned(),
            "slid".to_owned(),
            "solid".to_owned(),
            "texture".to_owned(),
        ]),
        "the state's own key order is not an order at all — measured, this table comes back \
         `solid, name, slid, replacable, texture`, which is neither how it was written nor \
         anything a mod author could predict. A refusal listing unrecognised fields in that \
         order is text that cannot be quoted in a document, and the guard comparing a quoted \
         refusal against a real run does so line for line. The fixture is built so that state \
         order, written order and lexicographic order are three different orders, so passing \
         through whatever the backend hands back fails here rather than intermittently \
         somewhere else"
    );
    Ok(())
}

#[test]
fn a_key_that_is_not_a_string_is_rendered_rather_than_passed_over() -> TestResult {
    let (host, table) = host_holding(A_TABLE_KEYED_BY_MORE_THAN_STRINGS)?;

    let mut expected = host.printed().to_vec();
    expected.push(THE_ONE_STRING_KEY.to_owned());

    assert_eq!(
        host.field_names(&table, most(expected.len())?),
        FieldNames::Enumerated(sorted(&expected)),
        "a table may be keyed by anything, and a key that is not a string is still a field the \
         declaration carries. An enumeration that quietly skipped one would make `a field the \
         loader does not recognise is refused` a promise that holds only for the key types \
         somebody happened to think of — and the field it lost is exactly the one nobody \
         intended to write. The expected rendering is the host's own, taken from what the chunk \
         printed of the same three values, because the contract is that a key renders the way \
         `print` renders it and not that it renders some particular way this file wrote down"
    );
    Ok(())
}

#[test]
fn a_table_holding_more_keys_than_the_caller_allowed_copies_none_of_them_out() -> TestResult {
    let (host, declaration) = host_holding(A_DECLARATION_WITH_TWO_MISSPELLINGS)?;
    let allowed = KEYS_THAT_DECLARATION_HOLDS - 1;

    assert_eq!(
        (
            host.field_names(&declaration, most(KEYS_THAT_DECLARATION_HOLDS)?),
            host.field_names(&declaration, most(allowed)?),
        ),
        (
            FieldNames::Enumerated(vec![
                "name".to_owned(),
                "replacable".to_owned(),
                "slid".to_owned(),
                "solid".to_owned(),
                "texture".to_owned(),
            ]),
            FieldNames::MoreThanAllowed { allowed },
        ),
        "both sides of the limit in one comparison, because a bound stated only from the \
         refusing side leaves `>` and `>=` indistinguishable and a bound stated only from the \
         accepting side is not a bound. The refusing side must report rather than truncate: a \
         list silently cut to the allowance hands the caller a table that looks like it holds \
         five fields when it holds a hundred thousand, and the allocation the bound exists to \
         refuse has already happened by then"
    );
    Ok(())
}

#[test]
fn a_length_metamethod_reporting_nothing_does_not_hide_what_a_table_holds() -> TestResult {
    let (host, table) = host_holding(A_TABLE_THAT_UNDERSTATES_ITS_OWN_LENGTH)?;

    assert_eq!(
        host.field_names(&table, most(the_three_positions().len())?),
        FieldNames::Enumerated(the_three_positions()),
        "`__len` is script the table's author chose, and it is the metamethod an enumeration \
         actually reaches: measured on this toolchain the backend's raw length answers 3 for \
         this table and its ordinary length answers 0, because the second consults the \
         metamethod. A host that sized its enumeration with the ordinary call reports every \
         declaration as carrying no fields at all — which refuses nothing, loses every typo, \
         and looks exactly like a table that really was empty"
    );
    Ok(())
}
