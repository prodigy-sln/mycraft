//! The shipped licence texts are byte-stable across platforms.
//!
//! A redistributor gets the bytes in the working tree, and on Windows a text
//! file is where a checkout silently acquires CRLF. `.gitattributes` carries the
//! `LICENSE-* text eol=lf` rule that pins them; the rule is the mechanism and
//! this file grades the property, because asserting the rule's line exists would
//! only prove somebody wrote a line and whether git honoured it is not
//! observable from a test running on one platform.
//!
//! # Its own file, deliberately
//!
//! Its subject is bytes where its neighbour `shipped_license_texts.rs` grades
//! text, so it shares no fixture with the structural detectors and moves as a
//! unit. Splitting it out keeps that file under stage 3's 600-line cap without
//! reworking anything in it.
//!
//! # A passing verdict names what it graded
//!
//! "No file holds a CR byte" is trivially true of no files at all, so the good
//! verdict carries the set it read rather than a bare all-clear, and a root
//! holding no licence text is a refusal rather than a pass. The set is
//! discovered by reading the root — every file whose name begins with
//! `LICENSE` — rather than looked up in a list, so a text that stopped being
//! shipped changes the verdict instead of quietly leaving the graded set.

mod common;

use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use common::{TestResult, repository_root};
use tempfile::TempDir;

/// The file the MIT licence is shipped as.
const MIT_FILE: &str = "LICENSE-MIT";

/// The file the Apache License 2.0 is shipped as.
const APACHE_FILE: &str = "LICENSE-APACHE";

/// What every licence text is named for.
const LICENSE_PREFIX: &str = "LICENSE";

/// The byte a licence text may not contain.
const CARRIAGE_RETURN: u8 = b'\r';

/// The byte that ends a line in a text stored the way this repository stores
/// one.
const LINE_FEED: u8 = b'\n';

/// What reading the bytes of every licence text under a root amounts to.
///
/// The good verdict names the files it graded, so it cannot be mistaken for a
/// scan that graded none — those are two different facts and the third variant
/// is what keeps them apart.
#[derive(Debug, PartialEq, Eq)]
enum ByteVerdict {
    /// The licence texts read, named, none of which holds a carriage return.
    NoCarriageReturn(Vec<String>),
    /// The licence texts that hold one, named.
    CarriageReturn(Vec<String>),
    /// The root holds no licence text, so there was nothing to grade.
    ReadNothing,
}

#[test]
fn the_two_shipped_licence_texts_are_graded_and_neither_holds_a_carriage_return() -> TestResult {
    assert_eq!(
        grade_bytes(&repository_root()?)?,
        ByteVerdict::NoCarriageReturn(vec![APACHE_FILE.to_owned(), MIT_FILE.to_owned()]),
        "a Windows checkout must not hand a redistributor a differently-encoded licence than a \
         Linux one — and the verdict names both files it read, because an all-clear that graded \
         nothing reads exactly like an all-clear that graded everything"
    );
    Ok(())
}

#[test]
fn a_licence_text_stored_with_crlf_line_endings_is_reported_by_name() -> TestResult {
    let root = a_root_holding(MIT_FILE, &as_crlf)?;

    assert_eq!(
        grade_bytes(root.path())?,
        ByteVerdict::CarriageReturn(vec![MIT_FILE.to_owned()]),
        "the reading above is an absence, and an absence proves nothing unless the same scan can \
         be shown finding the thing: a text that does hold a carriage return has to come back \
         named"
    );
    Ok(())
}

#[test]
fn a_root_holding_no_licence_text_refuses_rather_than_reporting_no_carriage_returns() -> TestResult
{
    let empty = TempDir::new()?;

    assert_eq!(
        grade_bytes(empty.path())?,
        ByteVerdict::ReadNothing,
        "a scan that read no file holds no carriage return for a reason that has nothing to do \
         with line endings — the texts moved, or the walk broke. Reporting a clean verdict there \
         is how this check would go green forever"
    );
    Ok(())
}

/// What the bytes of every licence text under `root` amount to.
///
/// Takes the root as an argument so the shipped tree and every fixture enter one
/// code path; two scans would leave each control grading a copy of the thing it
/// is meant to control.
///
/// # Errors
///
/// Returns the I/O failure when the root or one of its licence texts cannot be
/// read. A root that holds no licence text is a verdict, not an error.
fn grade_bytes(root: &Path) -> Result<ByteVerdict, Box<dyn Error>> {
    let mut graded = Vec::new();
    let mut carrying = Vec::new();
    for file in license_files_under(root)? {
        if fs::read(root.join(&file))?.contains(&CARRIAGE_RETURN) {
            carrying.push(file.clone());
        }
        graded.push(file);
    }
    if graded.is_empty() {
        return Ok(ByteVerdict::ReadNothing);
    }
    if carrying.is_empty() {
        return Ok(ByteVerdict::NoCarriageReturn(graded));
    }
    Ok(ByteVerdict::CarriageReturn(carrying))
}

/// The name of every licence text `root` holds, in order.
///
/// Discovered rather than listed: a hardcoded pair would keep reporting the same
/// two names after one of them stopped being shipped.
///
/// # Errors
///
/// Returns the I/O failure when the root cannot be read.
fn license_files_under(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut named = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_file() && name.starts_with(LICENSE_PREFIX) {
            named.push(name);
        }
    }
    named.sort();
    Ok(named)
}

/// A temporary root holding `file`, whose bytes are the shipped ones put
/// through `change`.
///
/// Derived from the shipped bytes rather than invented, which is what makes the
/// control a control over the same text the reading above grades — and what
/// keeps it red while the licence is not yet in the tree.
///
/// # Errors
///
/// Returns the I/O failure when the temporary root cannot be written.
fn a_root_holding(
    file: &str,
    change: &dyn Fn(&[u8]) -> Vec<u8>,
) -> Result<TempDir, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let shipped = shipped_bytes(file)?;
    fs::write(directory.path().join(file), change(&shipped))?;
    Ok(directory)
}

/// The bytes of the licence text this repository ships as `file`, or none where
/// it ships no such file.
///
/// # Errors
///
/// Returns the I/O failure for anything other than the file not being there.
fn shipped_bytes(file: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    match fs::read(repository_root()?.join(file)) {
        Ok(bytes) => Ok(bytes),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

/// `bytes` with every line feed preceded by a carriage return.
fn as_crlf(bytes: &[u8]) -> Vec<u8> {
    let mut converted = Vec::with_capacity(bytes.len());
    for byte in bytes {
        if *byte == LINE_FEED {
            converted.push(CARRIAGE_RETURN);
        }
        converted.push(*byte);
    }
    converted
}
