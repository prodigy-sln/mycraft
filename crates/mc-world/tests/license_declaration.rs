//! The declaration decides which licence texts must exist.
//!
//! This is the property that keeps `Cargo.toml` and the shipped texts from
//! drifting apart again. Its neighbour `shipped_license_texts.rs` grades
//! whether the two files in the tree *are* the licences; this file grades
//! whether they are the *right two*, which is a question only the declaration
//! can answer.
//!
//! # A hardcoded pair of filenames would satisfy almost everything
//!
//! `["LICENSE-MIT", "LICENSE-APACHE"]` written out by hand is correct today. It
//! satisfies every scenario about the shipped texts, every scenario about the
//! README, and every scenario about line endings — and it is silently wrong the
//! first time the declaration changes, which is the failure this whole spec
//! exists to prevent.
//!
//! [`a_single_licence_expression_requires_that_licence_alone`] is the **only**
//! test in the suite that a hardcoded pair fails. It is deliberately exact for
//! that reason: weaken it to "contains `LICENSE-MIT`" and nothing anywhere
//! forbids the list any more.
//!
//! # An absence is trivially true of nothing at all
//!
//! "Every licence the expression declares has a text" is satisfied by an
//! expression that declares none — the check finds nothing missing because it
//! looked for nothing.
//! [`an_empty_expression_refuses_rather_than_reporting_every_declared_licence_present`]
//! is the zero-set guard against that, and it hands the check a root holding
//! **both** texts so the refusal cannot be confused with a complaint about a
//! bare tree. [`a_declared_licence_with_no_text_is_reported_by_identifier_and_by_the_path_looked_for`]
//! is the positive control on the other side: the same scan has to be shown
//! finding a gap, or its finding none proves nothing.
//!
//! # Refusing is the feature, not a gap
//!
//! Five expressions below are refused rather than read. Each carries an
//! identifier the closed table *does* cover, so a check that reached for the
//! table too early would report success for a declaration it had not
//! understood. `MIT+` is the sharpest: reduced to `MIT` it finds `LICENSE-MIT`,
//! reports success, and ships no text for the "or later" the project actually
//! declared.
//!
//! # The expected filenames are written here, not imported
//!
//! `LICENSE-MIT` and `LICENSE-APACHE` are spelled out as literals below rather
//! than read from the module under test. An expectation that takes the mapping
//! from the thing that produced it agrees with any mapping at all.

mod common;

use std::error::Error;
use std::fs;

use common::TestResult;
use common::license::{
    Construct, Declaration, Declared, MissingFile, Refusal, RequiredFile, Requirement,
    check_declaration, declared_expression, required_files,
};
use serde_json::json;
use tempfile::TempDir;

/// The identifier the MIT licence is declared as.
const MIT: &str = "MIT";

/// The file that identifier requires.
const MIT_FILE: &str = "LICENSE-MIT";

/// The identifier the Apache License 2.0 is declared as.
const APACHE: &str = "Apache-2.0";

/// The file that identifier requires.
const APACHE_FILE: &str = "LICENSE-APACHE";

/// The expression this workspace actually declares.
const DUAL_LICENSE: &str = "MIT OR Apache-2.0";

/// An identifier outside the closed table, and the one every refusal fixture
/// reaches for.
///
/// Chosen because it is a real, well-known SPDX identifier whose filename is
/// guessable — `LICENSE-MPL` is exactly the plausible-looking file a check that
/// guessed would name.
const UNCOVERED: &str = "MPL-2.0";

/// What a hand-built metadata value calls the package it describes.
///
/// A name no crate in this workspace has, so a failure message quoting it
/// cannot be mistaken for something `cargo metadata` resolved — the same
/// reason the fixture registries in `common` namespace their blocks
/// `fixture:`.
const FIXTURE_PACKAGE: &str = "fixture-workspace";

#[test]
fn an_or_joined_expression_requires_the_text_of_every_licence_it_names() -> TestResult {
    assert_eq!(
        required_files(DUAL_LICENSE),
        Requirement::Files(vec![requires(MIT, MIT_FILE), requires(APACHE, APACHE_FILE)]),
        "a dual licence is an offer of two sets of terms, and a reader gets to pick — so both \
         texts have to be in the tree, and which two is a question only the declaration answers"
    );
    Ok(())
}

#[test]
fn a_single_licence_expression_requires_that_licence_alone() -> TestResult {
    assert_eq!(
        required_files(MIT),
        Requirement::Files(vec![requires(MIT, MIT_FILE)]),
        "this is the one scenario in the spec that a hardcoded [\"LICENSE-MIT\", \
         \"LICENSE-APACHE\"] fails — every other one is satisfied by a pair that happens to be \
         right today. The equality is exact on purpose: naming LICENSE-APACHE here, for an \
         expression that does not declare it, is the failure rather than a harmless extra"
    );
    Ok(())
}

#[test]
fn an_and_joined_expression_requires_the_text_of_every_licence_it_names() -> TestResult {
    assert_eq!(
        required_files("MIT AND Apache-2.0"),
        Requirement::Files(vec![requires(MIT, MIT_FILE), requires(APACHE, APACHE_FILE)]),
        "AND and OR require the same two texts, so refusing AND would be a false alarm on a \
         declaration whose required texts had not changed — and a drift check that cries wolf \
         gets suppressed. The expected set is written out rather than compared against the OR \
         form, because two derivations agreeing with each other is not evidence either is right"
    );
    Ok(())
}

#[test]
fn a_root_holding_every_required_text_reports_each_as_found() -> TestResult {
    let root = a_root_holding(&[MIT_FILE, APACHE_FILE])?;

    assert_eq!(
        check_declaration(DUAL_LICENSE, root.path())?,
        Declaration::Found(vec![requires(MIT, MIT_FILE), requires(APACHE, APACHE_FILE)]),
        "the passing verdict names what it found rather than reporting a bare all-clear: a scan \
         that located nothing reports no gaps in exactly the way a scan that located everything \
         does"
    );
    Ok(())
}

#[test]
fn a_declared_licence_with_no_text_is_reported_by_identifier_and_by_the_path_looked_for()
-> TestResult {
    let root = a_root_holding(&[MIT_FILE])?;

    assert_eq!(
        check_declaration(DUAL_LICENSE, root.path())?,
        Declaration::Missing(vec![MissingFile {
            identifier: APACHE.to_owned(),
            path: root.path().join(APACHE_FILE),
        }]),
        "the reading above is an absence of gaps, and an absence proves nothing unless the same \
         scan can be shown finding one. A report naming only the path cannot say which part of \
         the expression is unbacked; one naming only the identifier cannot say where the text \
         belongs. The expected path is built from the same temporary root the check was handed, \
         so no separator or drive letter is written down here — but an implementation that \
         canonicalises the root before joining will differ from it for reasons that have nothing \
         to do with licences"
    );
    Ok(())
}

#[test]
fn an_identifier_outside_the_table_is_refused_by_name() -> TestResult {
    assert_eq!(
        required_files(UNCOVERED),
        Requirement::Refused(Refusal::UncoveredIdentifier(UNCOVERED.to_owned())),
        "guessing LICENSE-MPL from MPL-2.0 would let a licence change through wearing a \
         plausible-looking file; refusing forces a human to decide what text the new licence \
         needs. Not-understanding is itself a failure here, which is what closes the vacuity \
         hole parsing would otherwise open"
    );
    Ok(())
}

#[test]
fn an_expression_carrying_a_with_exception_is_refused_and_names_the_exception() -> TestResult {
    let with_exception = "Apache-2.0 WITH LLVM-exception";

    assert_eq!(
        required_files(with_exception),
        Requirement::Refused(Refusal::UnreadConstruct(Construct::WithException(
            with_exception.to_owned()
        ))),
        "the identifier before the WITH is in the table, so a check that looked the term up \
         before reading it would derive LICENSE-APACHE and report success for a declaration \
         that is not Apache-2.0"
    );
    Ok(())
}

#[test]
fn a_parenthesised_sub_expression_is_refused_and_quoted_back() -> TestResult {
    assert_eq!(
        required_files("(MIT OR Apache-2.0) AND MPL-2.0"),
        Requirement::Refused(Refusal::UnreadConstruct(Construct::Parenthesised(
            "(MIT OR Apache-2.0)".to_owned()
        ))),
        "this expression also mixes operator kinds and also names an identifier outside the \
         table, so the answer is only stable if the parentheses are seen before either of them \
         — and they are, because the sub-expression is what makes the rest unreadable"
    );
    Ok(())
}

#[test]
fn a_trailing_or_later_suffix_is_refused_rather_than_stripped() -> TestResult {
    let or_later = "MIT+";

    assert_eq!(
        required_files(or_later),
        Requirement::Refused(Refusal::UnreadConstruct(Construct::OrLaterSuffix(
            or_later.to_owned()
        ))),
        "MIT+ naively reduced to MIT finds LICENSE-MIT, reports success, and ships no text for \
         the 'or later' the project declared. A derivation that answers LICENSE-MIT here is \
         wrong even though the file it names exists — which is why the suffix is refused rather \
         than stripped"
    );
    Ok(())
}

#[test]
fn an_expression_mixing_operator_kinds_is_refused() -> TestResult {
    assert_eq!(
        required_files("MIT AND Apache-2.0 OR MPL-2.0"),
        Requirement::Refused(Refusal::UnreadConstruct(Construct::MixedOperators)),
        "with no precedence rule read, MIT AND (Apache-2.0 OR MPL-2.0) and (MIT AND Apache-2.0) \
         OR MPL-2.0 require different texts and there is no basis to pick one. It also names an \
         identifier outside the table, so the mixing has to be seen before the table is reached"
    );
    Ok(())
}

#[test]
fn an_empty_expression_refuses_rather_than_reporting_every_declared_licence_present() -> TestResult
{
    let root = a_root_holding(&[MIT_FILE, APACHE_FILE])?;

    assert_eq!(
        check_declaration("", root.path())?,
        Declaration::Refused(Refusal::NothingDeclared),
        "the root holds both texts, so nothing is missing from it and a bare all-clear would be \
         true of it in exactly the way it is true of an empty tree: 'every licence the \
         expression declares has a text' is trivially satisfied when the expression declares \
         none, and that is how a drift check goes green forever"
    );
    Ok(())
}

#[test]
fn resolved_metadata_carrying_no_licence_declaration_refuses_and_names_the_package() -> TestResult {
    let undeclared = [
        json!({ "name": FIXTURE_PACKAGE }),
        json!({ "name": FIXTURE_PACKAGE, "license": null }),
    ];

    assert_eq!(
        undeclared
            .iter()
            .map(declared_expression)
            .collect::<Vec<_>>(),
        vec![
            undeclared_by(FIXTURE_PACKAGE),
            undeclared_by(FIXTURE_PACKAGE)
        ],
        "both spellings are one fact, and cargo emits either: the key is absent where nothing \
         declares a licence, and null where the field resolved to nothing. A reading that only \
         looked for the absent key would take the null for an expression and grade a package \
         declaring nothing as declaring something — an empty required set derived from it would \
         then report that every declared licence has a text"
    );
    Ok(())
}

/// The requirement an identifier places on the tree, as an expectation spells
/// it out.
fn requires(identifier: &str, file: &str) -> RequiredFile {
    RequiredFile {
        identifier: identifier.to_owned(),
        file: file.to_owned(),
    }
}

/// The verdict a package declaring no licence amounts to.
fn undeclared_by(package: &str) -> Declared {
    Declared::Refused(Refusal::NoDeclaration(package.to_owned()))
}

/// A temporary root holding each of `files`.
///
/// The bodies are placeholder prose, and deliberately not empty: whether a
/// required text *is* the licence is graded by `shipped_license_texts.rs`
/// against the shipped file, and a check that answered "found" for a
/// zero-length file would be indistinguishable here from one that answered it
/// for a real text.
///
/// # Errors
///
/// Returns the I/O failure when the temporary root cannot be written.
fn a_root_holding(files: &[&str]) -> Result<TempDir, Box<dyn Error>> {
    let directory = TempDir::new()?;
    for file in files {
        fs::write(directory.path().join(file), "the text of a licence\n")?;
    }
    Ok(directory)
}
