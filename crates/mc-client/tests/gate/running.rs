//! Running the gate's art stages, and reading the verdict it printed.
//!
//! # An enumerated verdict, never an absence
//!
//! `assert!(output.contains(...))` cannot tell a stage that passed from a stage
//! that never ran, and it cannot tell either from a script that died on its
//! second line — which is exactly the confusion the stage under test exists to
//! stop one level up. So a run is read as one of three [`GateReport`]s and each
//! test compares the whole of it, which rejects the other two for free.
//! [`GateReport::NoSummaryWasPrinted`] is the arm that means *the reading could
//! not look*, and it is what an unknown parameter, a syntax error or a missing
//! interpreter produce.
//!
//! # What the reading depends on, stated
//!
//! The script ends in a summary block it already had before this phase: one line
//! carrying the word `PASSED` or the word `FAILED`, and under a failure one line
//! per failed stage. The reading takes the stage names from those lines, so it
//! reads what the script tells a human rather than a second channel written for
//! it.
//!
//! **Two encodings meet in one pipe and the reading survives both.** PowerShell
//! re-encodes its own `Write-Host` output to the console's code page when the
//! stream is redirected — measured on this checkout, the summary's em dash
//! arrives as `-` and its `·` bullet as a single byte `0xFA`, which is not UTF-8
//! at all — while a native command's stderr passes through untouched, so
//! `voxforge`'s refusal arrives as the bytes it wrote. The bullet is therefore
//! never matched on: an entry is whatever is left of a line once the run of
//! characters before its first letter or digit is dropped. Colour is stripped for
//! the same reason.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use super::{gate_script, repository_root};

/// The stage that refuses a built image reaching version control.
pub const COMMITTED_SET_STAGE: &str = "art (generated set not committed)";

/// The stage that builds the set.
pub const ART_BUILD_STAGE: &str = "art (voxforge build)";

/// What the script records in place of the test stage when the set build refused.
pub const SKIPPED_TEST_STAGE: &str = "tests (not run: art build failed)";

/// What one run of the gate reported about itself.
#[derive(Debug, PartialEq, Eq)]
pub enum GateReport {
    /// Every stage the run selected passed.
    EveryStageItRanPassed,
    /// The run failed, and these are the stages it listed, in the order listed.
    StagesFailed(Vec<String>),
    /// The run printed neither verdict: it did not reach its own summary.
    NoSummaryWasPrinted,
}

/// One completed run of the gate script.
#[derive(Debug)]
pub struct GateRun {
    /// The process's exit status.
    pub exit_code: i32,
    /// Everything the run wrote to its output stream.
    pub printed: String,
    /// Everything the run wrote to its error stream.
    pub complained: String,
}

impl GateRun {
    /// Runs the gate's art stages alone, against the content root and the
    /// manifest given.
    ///
    /// `content_root` is stated relative to the repository, because the stage it
    /// feeds inspects the repository with `git`; `manifest` is an absolute path
    /// to a tree of this test's own, because the stage it feeds writes into it.
    ///
    /// # Errors
    ///
    /// Returns an error if PowerShell cannot be started or the script cannot be
    /// located. A missing interpreter fails the test rather than skipping it: a
    /// scan that could not run and a scan that found nothing must not look alike.
    pub fn of_the_art_stages(content_root: &str, manifest: &Path) -> Result<Self, Box<dyn Error>> {
        Self::of(&[
            OsStr::new("-ArtOnly"),
            OsStr::new("-ContentRoot"),
            OsStr::new(content_root),
            OsStr::new("-Manifest"),
            manifest.as_os_str(),
        ])
    }

    fn of(arguments: &[&OsStr]) -> Result<Self, Box<dyn Error>> {
        let finished = Command::new("pwsh")
            .arg("-NoProfile")
            .arg("-File")
            .arg(gate_script()?)
            .args(arguments)
            .current_dir(repository_root()?)
            .output()?;
        Ok(Self {
            exit_code: finished
                .status
                .code()
                .ok_or("the gate script was killed by a signal rather than exiting")?,
            printed: without_colour(&String::from_utf8_lossy(&finished.stdout)),
            complained: without_colour(&String::from_utf8_lossy(&finished.stderr)),
        })
    }

    /// The verdict this run printed for itself.
    #[must_use]
    pub fn report(&self) -> GateReport {
        let lines: Vec<&str> = self.printed.lines().map(str::trim_end).collect();
        let summaries: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| says(line, "PASSED") || says(line, "FAILED"))
            .map(|(index, _)| index)
            .collect();
        let (Some(index), 1) = (summaries.first(), summaries.len()) else {
            return GateReport::NoSummaryWasPrinted;
        };
        match lines.get(*index) {
            Some(line) if says(line, "PASSED") => GateReport::EveryStageItRanPassed,
            _ => GateReport::StagesFailed(listed_after(&lines, *index)),
        }
    }

    /// Whether the run put `text` in front of whoever ran it, on either stream.
    #[must_use]
    pub fn writes_through(&self, text: &str) -> bool {
        self.printed.contains(text) || self.complained.contains(text)
    }
}

/// A copy of one of this crate's committed fixture content roots, in a temporary
/// directory, removed when it is dropped.
///
/// **A fixture root is always copied, never built in place.** `voxforge build`
/// writes its output under the manifest's own directory, so a build aimed at the
/// tracked fixture would drop untracked images into the repository — the very
/// thing the stage beside it refuses — and would overwrite the one image that is
/// tracked deliberately.
///
/// # Errors
///
/// Returns an error if the fixture is not there or cannot be copied.
pub fn a_copy_of(fixture: &str) -> Result<TempDir, Box<dyn Error>> {
    let source = repository_root()?.join(super::FIXTURE_ROOTS).join(fixture);
    if !source.is_dir() {
        return Err(format!("there is no fixture content root at {}", source.display()).into());
    }
    let copy = TempDir::new()?;
    copy_tree(&source, copy.path())?;
    Ok(copy)
}

/// The manifest of a copied fixture root.
#[must_use]
pub fn manifest_of(copy: &TempDir) -> PathBuf {
    copy.path().join("textures.toml")
}

fn copy_tree(from: &Path, into: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(into)?;
    for entry in fs::read_dir(from)? {
        let source = entry?.path();
        let Some(name) = source.file_name() else {
            continue;
        };
        let destination = into.join(name);
        if source.is_dir() {
            copy_tree(&source, &destination)?;
        } else {
            fs::copy(&source, &destination)?;
        }
    }
    Ok(())
}

/// Whether a line carries `word` as a word of its own.
fn says(line: &str, word: &str) -> bool {
    line.split_whitespace().any(|spelled| spelled == word)
}

/// The stage names listed under a failing summary, each stripped of whatever
/// stands before its first letter or digit.
fn listed_after(lines: &[&str], summary: usize) -> Vec<String> {
    lines
        .iter()
        .skip(summary + 1)
        .filter_map(|line| {
            let entry = line.trim_start_matches(|c: char| !c.is_ascii_alphanumeric());
            (!entry.trim().is_empty()).then(|| entry.trim().to_owned())
        })
        .collect()
}

/// The same text with its terminal colour sequences removed.
fn without_colour(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut inside = false;
    for character in text.chars() {
        match (inside, character) {
            (false, '\u{1b}') => inside = true,
            (false, _) => plain.push(character),
            (true, 'm') => inside = false,
            (true, _) => {}
        }
    }
    plain
}
