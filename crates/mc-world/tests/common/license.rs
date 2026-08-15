//! What the declared licence expression requires of the tree.
//!
//! `Cargo.toml` declares `license = "MIT OR Apache-2.0"`. The declaration is
//! what decides which texts have to ship, so this module derives the required
//! set *from the expression* and never from a list of filenames somebody wrote
//! down — a hardcoded `["LICENSE-MIT", "LICENSE-APACHE"]` is right today and
//! silently wrong the day the declaration changes, which is the whole defect
//! this check exists to prevent.
//!
//! # A shared module rather than one test file's private helpers
//!
//! [`declared_expression`] is read by two test binaries: the declaration suite
//! that grades its refusals against hand-built values, and the consumer suite
//! that feeds it what `cargo metadata` resolved. Two integration-test binaries
//! cannot share a function any other way, and a second copy of the extraction
//! is a second opinion about what "declares nothing" means.
//!
//! # The subject is the expression plus a root, never a workspace directory
//!
//! `cargo metadata --format-version 1 --locked` cannot resolve a fixture
//! workspace in a `TempDir` — there is no `Cargo.lock` — so a check that only
//! accepted a workspace directory would leave every refusal below ungradable
//! through this code path, and would force each control to re-derive the
//! mapping inside its own fixture. Taking the expression as a string is what
//! makes those controls controls. `cargo metadata` is invoked in exactly one
//! place, and it is not here.
//!
//! # How much SPDX is read, and the order the refusals are decided in
//!
//! A flat list of bare identifiers joined by **one** operator kind (` OR ` or
//! ` AND `), mapped through a closed table: `MIT` → `LICENSE-MIT`, `Apache-2.0`
//! → `LICENSE-APACHE`. Both operator kinds are accepted because they yield the
//! same required set, and refusing one would raise a false alarm on a
//! declaration whose required texts had not changed — a drift check that cries
//! wolf gets suppressed.
//!
//! Everything else is refused **by name**, never resolved to a guessed
//! filename. Guessing `LICENSE-MPL` from `MPL-2.0` lets a licence change
//! through wearing a plausible file; a refusal forces a human to decide what
//! text the new licence needs.
//!
//! The refusals are decided in this order, and the order is part of the
//! contract because the expressions that exercise them carry more than one
//! unreadable thing at once:
//!
//! 1. **A parenthesised sub-expression.** `(MIT OR Apache-2.0) AND MPL-2.0`
//!    also mixes operator kinds and also names an identifier outside the table.
//!    The parentheses are what make the rest unreadable, so they are seen
//!    first.
//! 2. **Mixed operator kinds.** `MIT AND Apache-2.0 OR MPL-2.0` also names an
//!    identifier outside the table. With no precedence rule read, the two
//!    groupings require different texts and there is no basis to pick one.
//! 3. **A construct inside one term** — a `WITH` exception, or a trailing `+`.
//!    Both carry an identifier that *is* in the table, so a check that looked
//!    the term up before scanning it would report success for a declaration
//!    that is not the one in the table.
//! 4. **The closed table.** Anything left that is not in it is refused by
//!    name.
//!
//! Steps 3 and 4 are two passes over the whole term list rather than one pass
//! deciding each term completely: `MPL-2.0 OR MIT+` carries an unread construct
//! and an uncovered identifier, and the construct is the more specific fact
//! about what this check failed to read.

use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// A licence file some identifier in the expression requires.
#[derive(Debug, PartialEq, Eq)]
pub struct RequiredFile {
    /// The SPDX identifier that required it, as the expression spelled it.
    pub identifier: String,
    /// The file the closed table maps that identifier to.
    pub file: String,
}

/// A required licence file the root does not hold.
///
/// Names the identifier as well as the path: a report naming only the path
/// cannot say which part of the expression is unbacked, and one naming only the
/// identifier cannot say where the text belongs.
#[derive(Debug, PartialEq, Eq)]
pub struct MissingFile {
    /// The SPDX identifier that required the file.
    pub identifier: String,
    /// The path that was looked at — the root as given, joined with the mapped
    /// filename.
    ///
    /// **Joined, not canonicalised.** A canonicalising implementation returns a
    /// path that differs from the caller's own root on Windows for reasons that
    /// have nothing to do with licences.
    pub path: PathBuf,
}

/// Why a required set could not be derived.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// An identifier the closed table does not cover, named as the expression
    /// spelled it.
    UncoveredIdentifier(String),
    /// A piece of SPDX syntax this check deliberately does not read.
    UnreadConstruct(Construct),
    /// The expression declares no licence at all, so there is nothing to check
    /// and no passing verdict to give.
    NothingDeclared,
    /// The resolved metadata carries no licence declaration for the package
    /// named.
    NoDeclaration(String),
}

/// A piece of SPDX syntax refused rather than read.
///
/// Each variant names what was refused. [`Construct::MixedOperators`] carries
/// nothing because the variant already names both operators, and the fact being
/// reported is their appearing together rather than either one of them.
#[derive(Debug, PartialEq, Eq)]
pub enum Construct {
    /// A `WITH` licence exception, and the term carrying it.
    WithException(String),
    /// A parenthesised sub-expression, quoted as the expression wrote it,
    /// brackets included.
    Parenthesised(String),
    /// A trailing `+` — "or later" — and the term carrying it.
    OrLaterSuffix(String),
    /// Both ` OR ` and ` AND ` in one expression.
    MixedOperators,
}

/// What an expression requires, before anything has been looked for.
#[derive(Debug, PartialEq, Eq)]
pub enum Requirement {
    /// The file each identifier requires, in the order the expression names
    /// them.
    Files(Vec<RequiredFile>),
    /// The expression was not read.
    Refused(Refusal),
}

/// What checking an expression against a root amounts to.
#[derive(Debug, PartialEq, Eq)]
pub enum Declaration {
    /// Every file the expression requires is at the root, named with the
    /// identifier that required it.
    Found(Vec<RequiredFile>),
    /// The required files the root does not hold, in the order the expression
    /// required them. A root missing one of two reports the one.
    Missing(Vec<MissingFile>),
    /// The expression was not read, so no file was looked for.
    Refused(Refusal),
}

/// What a package's resolved metadata declares.
#[derive(Debug, PartialEq, Eq)]
pub enum Declared {
    /// The licence expression the package resolves.
    Expression(String),
    /// The package declares none.
    Refused(Refusal),
}

/// The closed table, identifier to file.
///
/// Closed is the point: an identifier that is not here is refused by name
/// rather than resolved to a guessed filename. It is a table read by the
/// derivation below and never a list of filenames answered directly — the
/// difference is whether `MIT` alone can ever require `LICENSE-APACHE`.
const TABLE: [(&str, &str); 2] = [("MIT", "LICENSE-MIT"), ("Apache-2.0", "LICENSE-APACHE")];

/// The disjunction operator, spaced as SPDX writes it.
const OR: &str = " OR ";

/// The conjunction operator, spaced as SPDX writes it.
const AND: &str = " AND ";

/// The exception operator, which this check refuses rather than reads.
const WITH: &str = " WITH ";

/// The licence files `expression` requires.
///
/// Takes the expression as a string so the shipped declaration and every
/// fixture enter one code path. See the module header for what is read and the
/// order refusals are decided in.
#[must_use]
pub fn required_files(expression: &str) -> Requirement {
    let expression = expression.trim();
    if expression.is_empty() {
        return Requirement::Refused(Refusal::NothingDeclared);
    }
    if let Some(construct) = unread_construct(expression) {
        return Requirement::Refused(Refusal::UnreadConstruct(construct));
    }

    let mut required = Vec::new();
    for term in terms(expression) {
        let Some(file) = mapped_file(term) else {
            return Requirement::Refused(Refusal::UncoveredIdentifier(term.to_owned()));
        };
        required.push(RequiredFile {
            identifier: term.to_owned(),
            file: file.to_owned(),
        });
    }
    Requirement::Files(required)
}

/// The piece of SPDX syntax `expression` carries that this check does not read,
/// decided in the order the module header records.
fn unread_construct(expression: &str) -> Option<Construct> {
    if let Some(quoted) = parenthesised(expression) {
        return Some(Construct::Parenthesised(quoted));
    }
    if expression.contains(OR) && expression.contains(AND) {
        return Some(Construct::MixedOperators);
    }
    terms(expression).find_map(term_construct)
}

/// The construct `term` carries inside itself, if any.
///
/// Both constructs here wrap an identifier the table *does* cover, which is why
/// they are scanned for before the table is reached: `MIT+` looked up as `MIT`
/// finds `LICENSE-MIT` and reports success for a declaration whose "or later"
/// nothing in the tree backs.
fn term_construct(term: &str) -> Option<Construct> {
    if term.contains(WITH) {
        return Some(Construct::WithException(term.to_owned()));
    }
    if term.ends_with('+') {
        return Some(Construct::OrLaterSuffix(term.to_owned()));
    }
    None
}

/// The parenthesised sub-expression `expression` opens, quoted as it wrote it.
///
/// An unclosed bracket quotes back to the end of the expression rather than
/// answering `None`: a refusal that named nothing would be the one outcome
/// worse than quoting too much.
fn parenthesised(expression: &str) -> Option<String> {
    let opened = expression.get(expression.find('(')?..)?;
    // `)` is one byte, so one past it is a boundary.
    let end = opened.find(')').map_or(opened.len(), |closing| closing + 1);
    opened.get(..end).map(str::to_owned)
}

/// The identifiers `expression` names, in the order it names them.
///
/// One operator kind is read, so the separator is whichever of the two the
/// expression uses; an expression using both never reaches here.
fn terms(expression: &str) -> impl Iterator<Item = &str> {
    let separator = if expression.contains(OR) { OR } else { AND };
    expression.split(separator).map(str::trim)
}

/// The file the closed table maps `identifier` to.
fn mapped_file(identifier: &str) -> Option<&'static str> {
    TABLE
        .iter()
        .find(|(spdx, _)| *spdx == identifier)
        .map(|(_, file)| *file)
}

/// Whether `root` holds the licence file every identifier in `expression`
/// requires.
///
/// Takes the root as an argument for the same reason [`required_files`] takes
/// the expression: two scans would leave every control grading a copy of the
/// thing it is meant to control.
///
/// # Errors
///
/// Returns the I/O failure when the root cannot be read. A file that is not
/// there is a verdict, not an error.
pub fn check_declaration(expression: &str, root: &Path) -> Result<Declaration, Box<dyn Error>> {
    let required = match required_files(expression) {
        Requirement::Refused(refusal) => return Ok(Declaration::Refused(refusal)),
        Requirement::Files(files) => files,
    };

    let mut missing = Vec::new();
    for file in &required {
        // Joined, never canonicalised — see `MissingFile::path`.
        let path = root.join(&file.file);
        if !path.try_exists()? {
            missing.push(MissingFile {
                identifier: file.identifier.clone(),
                path,
            });
        }
    }

    if missing.is_empty() {
        Ok(Declaration::Found(required))
    } else {
        Ok(Declaration::Missing(missing))
    }
}

/// The licence expression `package`'s resolved metadata declares.
///
/// `cargo metadata` spells "declares none" two ways — the key absent, and
/// `"license": null` — and both are the same fact. A reading that only looked
/// for the absent key would take the null for an expression.
#[must_use]
pub fn declared_expression(package: &Value) -> Declared {
    match package.get("license").and_then(Value::as_str) {
        Some(expression) => Declared::Expression(expression.to_owned()),
        None => Declared::Refused(Refusal::NoDeclaration(package_name(package))),
    }
}

/// What a metadata value calls the package it describes.
///
/// `cargo metadata` always names a package; the fallback exists so a
/// hand-built value missing the field still refuses under a readable name
/// rather than through a panic in a check whose whole subject is a refusal.
///
/// Public for the same reason [`declared_expression`] is: the consumers name
/// members in their findings, and a second reading of "what is this package
/// called" is a second answer that can disagree with the one a refusal carries.
#[must_use]
pub fn package_name(package: &Value) -> String {
    package
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(UNNAMED_PACKAGE)
        .to_owned()
}

/// What a package whose metadata carries no name is called in a refusal.
const UNNAMED_PACKAGE: &str = "a package whose metadata names it not at all";
