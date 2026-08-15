//! The licence texts this repository ships are the licences it declares.
//!
//! `Cargo.toml` declares `license = "MIT OR Apache-2.0"`, and until this suite
//! existed the tree backed that claim with nothing. What is graded here is
//! structure a paraphrase or a truncation cannot survive: the MIT permission
//! grant and its all-caps warranty disclaimer, the Apache title and version
//! line, all nine numbered section headings, and the appendix marker. A single
//! altered word inside a paragraph no anchor sits in is invisible here, which
//! is why byte-authenticity against the published text stays a manual
//! acceptance check.
//!
//! # The licence text is never the thing adjusted to make a test pass
//!
//! Every anchor below is an exact string, which is the "over-tight assertion
//! invites a real defect" trap in its worst form: an anchor failing against a
//! *genuinely correct* licence has "edit the licence" as its cheapest green,
//! and a licence edited to satisfy a test is not the licence. Any such failure
//! is a defect in the anchor and is fixed here. Two guards against reaching
//! that point: anchors are matched against a whitespace-flattened reading, so a
//! line break landing inside one cannot fail a correct file; and both canonical
//! texts are pure ASCII, so the straight quotes in
//! `THE SOFTWARE IS PROVIDED "AS IS"` are part of the licence and a copy
//! carrying typographic quotes is a defect in the transcription.
//!
//! # Why the placeholder check reads one file and not both
//!
//! The canonical Apache appendix carries the line
//! `Copyright [yyyy] [name of copyright owner]` verbatim — a line with the
//! shape of a copyright notice, carrying a marker a shipped copyright line may
//! not. A placeholder detector run over both shipped texts therefore reddens
//! against a completely correct `LICENSE-APACHE`, and its cheapest green is
//! editing the Apache licence. [`grade_mit`] is the only caller of
//! [`placeholder_in`] and it reads `LICENSE-MIT` alone. That scope is the whole
//! defence, and it is why `LICENSE-APACHE` gets no placeholder check: its
//! appendix boilerplate is published unfilled, and filling it would be the
//! error.
//!
//! # This suite does not know who holds the copyright
//!
//! It grades that *a* copyright line is present, carrying a year field and a
//! holder field, neither still a template marker. It never names the holder,
//! which is written in `LICENSE-MIT` and nowhere else: a test naming it would
//! re-couple the licence text to the suite grading it.
//!
//! # Every fixture is derived from the shipped text
//!
//! A hand-typed second Apache licence is a second licence, free to drift from
//! the first, and it would exceed this file's line budget on its own. Each
//! control reads the shipped text, transforms exactly the element it is a
//! control for, and writes the result into a temporary root the same detector
//! reads. Two scans would leave every control grading a copy of the thing it is
//! meant to control.

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

/// The opening of the MIT permission grant.
const PERMISSION_GRANT: &str = "Permission is hereby granted, free of charge";

/// The opening of the MIT warranty disclaimer, in the all-caps spelling the
/// licence uses.
const WARRANTY_DISCLAIMER: &str = r#"THE SOFTWARE IS PROVIDED "AS IS""#;

/// The title the Apache licence opens with.
const APACHE_TITLE: &str = "Apache License";

/// The version line the Apache License 2.0 states, exactly.
const APACHE_VERSION_LINE: &str = "Version 2.0, January 2004";

/// The appendix marker, without the trailing period the published text puts on
/// it — the anchor is searched for as a substring, so the period is graded by
/// neither presence nor absence.
const APPENDIX_MARKER: &str = "APPENDIX: How to apply the Apache License to your work";

/// The nine numbered section headings of the Apache License 2.0, in order.
const APACHE_SECTIONS: [&str; 9] = [
    "1. Definitions.",
    "2. Grant of Copyright License.",
    "3. Grant of Patent License.",
    "4. Redistribution.",
    "5. Submission of Contributions.",
    "6. Trademarks.",
    "7. Disclaimer of Warranty.",
    "8. Limitation of Liability.",
    "9. Accepting Warranty or Additional Liability.",
];

/// Every template marker a shipped copyright line may not still carry.
///
/// `[name of copyright owner]` is the one to be careful with: the canonical
/// Apache appendix carries it verbatim, so this list is only ever run over the
/// MIT text. See the module header.
const PLACEHOLDERS: [&str; 5] = [
    "<year>",
    "[year]",
    "<holder>",
    "[name of copyright owner]",
    "[fullname]",
];

/// A part of a licence text a detector below grades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Element {
    /// The MIT permission grant.
    PermissionGrant,
    /// The MIT all-caps warranty disclaimer.
    WarrantyDisclaimer,
    /// A line naming a year and a holder.
    CopyrightLine,
    /// The Apache title.
    Title,
    /// The Apache version line.
    VersionLine,
    /// One of the nine numbered Apache section headings, by its number.
    Section(u8),
    /// The Apache appendix marker.
    AppendixMarker,
}

/// What grading a licence text amounts to.
///
/// A verdict rather than a set of booleans a caller has to remember to check:
/// "all there", "no such file", "an element is missing" and "an element states
/// something else" are four facts, and only the first is good news.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// The file was read and carries every element named.
    Present(Vec<Element>),
    /// The root holds no file of that name, so nothing was graded.
    NoSuchFile { file: String },
    /// The file was read and does not carry the elements named.
    Missing {
        file: String,
        elements: Vec<Element>,
    },
    /// The file was read and states something else where the elements named
    /// belong.
    NotMatching {
        file: String,
        elements: Vec<Element>,
    },
    /// The file was read and its copyright line still carries a template
    /// marker.
    Placeholder { file: String, marker: String },
}

#[test]
fn the_shipped_mit_text_carries_its_grant_its_disclaimer_and_a_copyright_line() -> TestResult {
    assert_eq!(
        grade_mit(&repository_root()?)?,
        Verdict::Present(vec![
            Element::PermissionGrant,
            Element::WarrantyDisclaimer,
            Element::CopyrightLine,
        ]),
        "the one claim a third party relies on before using or contributing anything is the \
         licence, and a manifest field is not it: the grant, the disclaimer and a copyright line \
         are what make the shipped file the MIT licence rather than a paraphrase of it"
    );
    Ok(())
}

#[test]
fn an_mit_text_without_its_warranty_disclaimer_is_reported_with_the_disclaimer_named() -> TestResult
{
    let root = a_root_holding(MIT_FILE, |text| {
        without_the_paragraph_carrying(text, WARRANTY_DISCLAIMER)
    })?;

    assert_eq!(
        grade_mit(root.path())?,
        missing_from(MIT_FILE, vec![Element::WarrantyDisclaimer]),
        "the disclaimer is the clause the licence exists to carry, and a text shipped without it \
         grants rights while promising nothing about them — the reading above cannot say so unless \
         this one can"
    );
    Ok(())
}

#[test]
fn an_mit_text_without_its_permission_grant_is_reported_with_the_grant_named() -> TestResult {
    let root = a_root_holding(MIT_FILE, |text| {
        without_the_paragraph_carrying(text, PERMISSION_GRANT)
    })?;

    assert_eq!(
        grade_mit(root.path())?,
        missing_from(MIT_FILE, vec![Element::PermissionGrant]),
        "a licence text that grants nothing is a disclaimer, and it would still satisfy every \
         other detector this file has"
    );
    Ok(())
}

#[test]
fn an_mit_text_carrying_no_copyright_line_is_reported_with_the_line_named() -> TestResult {
    let root = a_root_holding(MIT_FILE, without_the_copyright_line)?;

    assert_eq!(
        grade_mit(root.path())?,
        missing_from(MIT_FILE, vec![Element::CopyrightLine]),
        "the MIT grant is made by somebody, in some year, and a text carrying neither leaves the \
         reader with nobody to have got the rights from"
    );
    Ok(())
}

#[test]
fn a_copyright_line_still_carrying_a_template_marker_is_reported_by_the_marker() -> TestResult {
    let mut reported = Vec::new();
    for marker in PLACEHOLDERS {
        let root = a_root_holding(MIT_FILE, |text| replace_the_holder_with(text, marker))?;
        reported.push(grade_mit(root.path())?);
    }

    assert_eq!(
        reported,
        PLACEHOLDERS
            .into_iter()
            .map(|marker| Verdict::Placeholder {
                file: MIT_FILE.to_owned(),
                marker: marker.to_owned(),
            })
            .collect::<Vec<_>>(),
        "the holder being decided removes the reason to hedge, not the reason to check: the one \
         thing worse than a wrong holder in a licence notice is a template marker shipping in it, \
         and each marker has to come back named rather than merely counted"
    );
    Ok(())
}

#[test]
fn the_shipped_apache_text_carries_its_title_and_its_version_line() -> TestResult {
    assert_eq!(
        grade_apache_header(&repository_root()?)?,
        Verdict::Present(vec![Element::Title, Element::VersionLine]),
        "which Apache licence a file is depends on its version line, and the header block is the \
         only place the text says"
    );
    Ok(())
}

#[test]
fn the_shipped_apache_text_carries_all_nine_numbered_section_headings() -> TestResult {
    assert_eq!(
        grade_apache_sections(&repository_root()?)?,
        Verdict::Present(every_section()),
        "the nine sections are the licence: a copy holding eight of them is a text that reads like \
         the Apache licence and grants something else"
    );
    Ok(())
}

#[test]
fn the_shipped_apache_text_carries_its_appendix_marker() -> TestResult {
    assert_eq!(
        grade_apache_appendix(&repository_root()?)?,
        Verdict::Present(vec![Element::AppendixMarker]),
        "the appendix is where the published text tells a reader how to apply the licence, and it \
         is the part a transcription that stopped at `END OF TERMS AND CONDITIONS` loses"
    );
    Ok(())
}

#[test]
fn an_apache_text_truncated_after_section_eight_is_reported_with_the_ninth_heading_named()
-> TestResult {
    let ninth = last_section_heading()?;
    let root = a_root_holding(APACHE_FILE, |text| truncated_before(text, ninth))?;

    assert_eq!(
        grade_apache_sections(root.path())?,
        missing_from(APACHE_FILE, vec![Element::Section(9)]),
        "a transcription that ran out is the failure mode a section scan exists for, and it has to \
         name which heading went missing rather than report a count"
    );
    Ok(())
}

#[test]
fn an_apache_text_with_every_heading_but_no_appendix_marker_is_reported_by_the_marker() -> TestResult
{
    let root = a_root_holding(APACHE_FILE, |text| {
        without_the_line_carrying(text, APPENDIX_MARKER)
    })?;

    assert_eq!(
        (
            grade_apache_sections(root.path())?,
            grade_apache_appendix(root.path())?
        ),
        (
            Verdict::Present(every_section()),
            missing_from(APACHE_FILE, vec![Element::AppendixMarker])
        ),
        "the fixture keeping all nine headings is the scenario's own antecedent, so it is asserted \
         rather than left to a comment: truncating the text to drop the marker would drop section \
         9 with it, and a fixture missing both elements reports the marker missing whatever the \
         appendix detector does"
    );
    Ok(())
}

#[test]
fn an_apache_text_stating_a_different_version_is_reported_with_the_version_line_named() -> TestResult
{
    let root = a_root_holding(APACHE_FILE, |text| {
        replace_the_version_line_with(text, "Version 1.1, January 2004")
    })?;

    assert_eq!(
        grade_apache_header(root.path())?,
        Verdict::NotMatching {
            file: APACHE_FILE.to_owned(),
            elements: vec![Element::VersionLine],
        },
        "a text stating another version is not a text missing its version line, and reporting the \
         two the same way would let a licence of a different version pass as an incomplete one"
    );
    Ok(())
}

/// The verdict `root`'s MIT text amounts to.
///
/// Structure first, then the placeholder check: a text missing its copyright
/// line is reported as missing it rather than as carrying a marker, which is
/// the order that keeps the two controls apart. A passing verdict therefore
/// also says the copyright line carries no marker, so the placeholder detector
/// has no branch nothing exercises.
///
/// # Errors
///
/// Returns the I/O failure when the file exists and cannot be read. A file that
/// is not there is a verdict, not an error.
fn grade_mit(root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let Some(text) = license_text(root, MIT_FILE)? else {
        return Ok(no_such_file(MIT_FILE));
    };
    let missing = mit_elements_absent_from(&text);
    if !missing.is_empty() {
        return Ok(missing_from(MIT_FILE, missing));
    }
    if let Some(line) = copyright_line(&text)
        && let Some(marker) = placeholder_in(line)
    {
        return Ok(Verdict::Placeholder {
            file: MIT_FILE.to_owned(),
            marker: marker.to_owned(),
        });
    }
    Ok(Verdict::Present(vec![
        Element::PermissionGrant,
        Element::WarrantyDisclaimer,
        Element::CopyrightLine,
    ]))
}

/// The MIT elements `text` does not carry, in the order a verdict names them.
fn mit_elements_absent_from(text: &str) -> Vec<Element> {
    let flat = flattened(text);
    let mut absent = Vec::new();
    if !flat.contains(PERMISSION_GRANT) {
        absent.push(Element::PermissionGrant);
    }
    if !flat.contains(WARRANTY_DISCLAIMER) {
        absent.push(Element::WarrantyDisclaimer);
    }
    if copyright_line(text).is_none() {
        absent.push(Element::CopyrightLine);
    }
    absent
}

/// The verdict `root`'s Apache header block amounts to.
fn grade_apache_header(root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let Some(text) = license_text(root, APACHE_FILE)? else {
        return Ok(no_such_file(APACHE_FILE));
    };
    let stated = version_line(&text);
    let mut missing = Vec::new();
    if !flattened(&text).contains(APACHE_TITLE) {
        missing.push(Element::Title);
    }
    if stated.is_none() {
        missing.push(Element::VersionLine);
    }
    if !missing.is_empty() {
        return Ok(missing_from(APACHE_FILE, missing));
    }
    if stated != Some(APACHE_VERSION_LINE) {
        return Ok(Verdict::NotMatching {
            file: APACHE_FILE.to_owned(),
            elements: vec![Element::VersionLine],
        });
    }
    Ok(Verdict::Present(vec![Element::Title, Element::VersionLine]))
}

/// The verdict `root`'s Apache section headings amount to.
fn grade_apache_sections(root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let Some(text) = license_text(root, APACHE_FILE)? else {
        return Ok(no_such_file(APACHE_FILE));
    };
    let flat = flattened(&text);
    let mut missing = Vec::new();
    for (position, heading) in APACHE_SECTIONS.into_iter().enumerate() {
        if !flat.contains(heading) {
            missing.push(Element::Section(u8::try_from(position + 1)?));
        }
    }
    if missing.is_empty() {
        return Ok(Verdict::Present(every_section()));
    }
    Ok(missing_from(APACHE_FILE, missing))
}

/// The verdict `root`'s Apache appendix marker amounts to.
fn grade_apache_appendix(root: &Path) -> Result<Verdict, Box<dyn Error>> {
    let Some(text) = license_text(root, APACHE_FILE)? else {
        return Ok(no_such_file(APACHE_FILE));
    };
    if flattened(&text).contains(APPENDIX_MARKER) {
        return Ok(Verdict::Present(vec![Element::AppendixMarker]));
    }
    Ok(missing_from(APACHE_FILE, vec![Element::AppendixMarker]))
}

/// The verdict for a root holding no `file` at all.
fn no_such_file(file: &str) -> Verdict {
    Verdict::NoSuchFile {
        file: file.to_owned(),
    }
}

/// The verdict for a `file` that does not carry `elements`.
fn missing_from(file: &str, elements: Vec<Element>) -> Verdict {
    Verdict::Missing {
        file: file.to_owned(),
        elements,
    }
}

/// Every numbered section, as the passing verdict names them.
fn every_section() -> Vec<Element> {
    (1..=9).map(Element::Section).collect()
}

/// The heading of the last numbered section, which is the one a transcription
/// that ran out loses first.
///
/// # Errors
///
/// Returns an error if the list of headings is ever emptied, which would leave
/// the section scan looking for nothing.
fn last_section_heading() -> Result<&'static str, Box<dyn Error>> {
    APACHE_SECTIONS
        .into_iter()
        .next_back()
        .ok_or_else(|| "the Apache licence has to have a last numbered section".into())
}

/// The text of `file` under `root`, or `None` where the root holds no such
/// file.
///
/// `None` rather than an error, because "this root ships no licence text" is an
/// answer the verdicts have a variant for, and a scenario about a missing
/// element wants that reported as a verdict rather than raised as I/O. Every
/// other failure still propagates.
///
/// # Errors
///
/// Returns the I/O failure for anything other than the file not being there.
fn license_text(root: &Path, file: &str) -> Result<Option<String>, Box<dyn Error>> {
    match fs::read_to_string(root.join(file)) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// `text` with every run of whitespace collapsed to one space.
///
/// Anchors are searched for in this reading so that a line break landing inside
/// one cannot fail a correct licence — see the module header.
fn flattened(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The first line of `text` shaped like a copyright notice: the word
/// `Copyright`, then a year field, then a holder field.
///
/// The shape is graded, never what the two fields say — this suite must not
/// know who holds the copyright. [`placeholder_in`] is what turns "a year field
/// and a holder field" into "a real year and a real holder": `<year>` fills the
/// shape and is caught there, by name.
fn copyright_line(text: &str) -> Option<&str> {
    text.lines().find(|line| has_copyright_shape(line))
}

/// Whether `line` has the shape of a copyright notice.
fn has_copyright_shape(line: &str) -> bool {
    let Some(rest) = line.trim().strip_prefix("Copyright") else {
        return false;
    };
    named_fields(rest).count() >= 2
}

/// The fields of a copyright line that say something, which is every field but
/// the copyright sign.
fn named_fields(rest: &str) -> impl Iterator<Item = &str> {
    rest.split_whitespace()
        .filter(|field| !matches!(*field, "(c)" | "(C)" | "©"))
}

/// The template marker `line` still carries, if it carries one.
///
/// Only ever reached for the MIT text. See the module header for what running
/// it over `LICENSE-APACHE` would do.
fn placeholder_in(line: &str) -> Option<&'static str> {
    PLACEHOLDERS
        .into_iter()
        .find(|marker| line.contains(marker))
}

/// The version line `text` states: the first line beginning with `Version`.
fn version_line(text: &str) -> Option<&str> {
    text.lines()
        .map(str::trim)
        .find(|line| line.starts_with("Version"))
}

/// A temporary root holding `file`, whose text is the shipped one put through
/// `change`.
///
/// Deriving from the shipped text rather than pasting a copy is what makes each
/// control a control: the detector reads one text, transformed, rather than a
/// second text that agrees with the first only until somebody edits one of
/// them. It is also why every control below is red while the licence is not in
/// the tree — a transformation of nothing is nothing, and the detector reports
/// a verdict no scenario expects.
///
/// # Errors
///
/// Returns the I/O failure when the temporary root cannot be written.
fn a_root_holding(file: &str, change: impl Fn(&str) -> String) -> Result<TempDir, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let shipped = license_text(&repository_root()?, file)?.unwrap_or_default();
    fs::write(directory.path().join(file), change(&shipped))?;
    Ok(directory)
}

/// `text` without the paragraph carrying `anchor`.
///
/// A paragraph rather than a line: dropping one line of a wrapped sentence
/// leaves the rest of it behind, and a text still carrying half a disclaimer is
/// not the "omits the disclaimer" case a scenario names.
fn without_the_paragraph_carrying(text: &str, anchor: &str) -> String {
    let unwrapped = text.replace("\r\n", "\n");
    unwrapped
        .split("\n\n")
        .filter(|paragraph| !flattened(paragraph).contains(anchor))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// `text` without the line carrying `anchor`.
fn without_the_line_carrying(text: &str, anchor: &str) -> String {
    text.lines()
        .filter(|line| !line.contains(anchor))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `text` without its copyright line.
///
/// Removes exactly what [`copyright_line`] looks for, so the fixture cannot
/// disagree with the detector about which line that is.
fn without_the_copyright_line(text: &str) -> String {
    text.lines()
        .filter(|line| !has_copyright_shape(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `text` with the holder field of its copyright line replaced by `marker`.
///
/// The holder field for every marker, including the two that name a year: the
/// detector reads the whole copyright line, so which field a marker sits in
/// does not change the answer, and rebuilding the line one way keeps the
/// fixture from encoding a second opinion about the licence's shape.
fn replace_the_holder_with(text: &str, marker: &str) -> String {
    text.lines()
        .map(|line| {
            if has_copyright_shape(line) {
                rebuilt_with(line, marker)
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `line` rebuilt with `marker` where its holder stands.
///
/// The year field is carried over from the line being replaced, so the fixture
/// differs from the shipped copyright line in the holder and in nothing else —
/// and this suite still never reads what the holder said.
fn rebuilt_with(line: &str, marker: &str) -> String {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("Copyright").unwrap_or(trimmed);
    let year = named_fields(rest).next().unwrap_or_default();
    format!("Copyright (c) {year} {marker}")
}

/// `text` with its version line replaced by `replacement`.
fn replace_the_version_line_with(text: &str, replacement: &str) -> String {
    let mut replaced = false;
    text.lines()
        .map(|line| {
            if !replaced && line.trim().starts_with("Version") {
                replaced = true;
                return replacement.to_owned();
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `text` cut off immediately before the line carrying `anchor`.
fn truncated_before(text: &str, anchor: &str) -> String {
    text.lines()
        .take_while(|line| !line.contains(anchor))
        .collect::<Vec<_>>()
        .join("\n")
}
