//! What names a capture, and where its golden and artifacts live on disk.
//!
//! The layout is the one part of this crate that is expensive to reverse: every
//! golden the project ever commits is found through it. It is therefore a
//! directory per capture rather than a flat file per capture, so that the
//! per-adapter variants this spec defers can arrive as a **new file in an
//! existing directory** instead of a rename of the whole set.
//!
//! Paths are deliberately stable across runs — no process id, no timestamp —
//! because a passing run has to find and remove the stale artifacts a previous
//! mismatch left behind, which a per-run unique directory would make impossible.

use std::path::{Path, PathBuf};

use thiserror::Error;

use super::compare::Thresholds;
use super::optins::OptIns;

/// The one golden filename this spec writes and reads.
///
/// Written as a constant rather than resolved: per-adapter variants are Out of
/// Scope, and only the *shape* of the path leaves room for one.
const GOLDEN_STEM: &str = "default";

/// The name of a capture: one path segment, in three roles — the timeout error,
/// the golden directory and the artifact directory.
///
/// Two of those roles turn it into a path, which is what makes the validation
/// input validation on a path-forming public input rather than decoration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureId(String);

/// A name that cannot stand for a capture.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CaptureIdError {
    #[error("a capture needs a name; an empty one would put its files in the root")]
    Empty,
    #[error(
        "the capture name `{name}` contains `{character}`, which is not one of a-z, 0-9, `-` or `_`"
    )]
    IllegalCharacter { name: String, character: char },
}

impl CaptureId {
    /// Validates a capture name.
    ///
    /// The alphabet is lowercase `a-z`, `0-9`, `-` and `_`. That excludes both
    /// path separators and `.`, so `..` cannot survive into a path the harness
    /// writes to, and two captures cannot collide on a case-insensitive
    /// filesystem.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureIdError::Empty`] for an empty name, or
    /// [`CaptureIdError::IllegalCharacter`] naming the first character outside
    /// the alphabet.
    pub fn new(name: &str) -> Result<Self, CaptureIdError> {
        if name.is_empty() {
            return Err(CaptureIdError::Empty);
        }
        if let Some(character) = name.chars().find(|character| !is_legal(*character)) {
            return Err(CaptureIdError::IllegalCharacter {
                name: name.to_owned(),
                character,
            });
        }
        Ok(Self(name.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether `character` may appear in a capture name.
fn is_legal(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit() || matches!(character, '-' | '_')
}

/// Where a capture's committed golden and its provenance sidecar live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoldenPaths {
    directory: PathBuf,
}

impl GoldenPaths {
    pub(crate) fn new(golden_root: &Path, capture: &CaptureId) -> Self {
        Self {
            directory: golden_root.join(capture.as_str()),
        }
    }

    /// The golden frame itself.
    pub(crate) fn image(&self) -> PathBuf {
        self.directory.join(format!("{GOLDEN_STEM}.png"))
    }

    /// The record of which adapter produced that golden, beside it.
    pub(crate) fn provenance(&self) -> PathBuf {
        self.directory
            .join(format!("{GOLDEN_STEM}.provenance.json"))
    }
}

/// Where a capture's four mismatch artifacts live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactPaths {
    directory: PathBuf,
}

impl ArtifactPaths {
    pub(crate) fn new(artifact_root: &Path, capture: &CaptureId) -> Self {
        Self {
            directory: artifact_root.join(capture.as_str()),
        }
    }

    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    pub(crate) fn expected(&self) -> PathBuf {
        self.directory.join("expected.png")
    }

    pub(crate) fn actual(&self) -> PathBuf {
        self.directory.join("actual.png")
    }

    pub(crate) fn diff(&self) -> PathBuf {
        self.directory.join("diff.png")
    }

    pub(crate) fn report(&self) -> PathBuf {
        self.directory.join("report.json")
    }
}

/// Everything the golden lifecycle needs that is not the frame itself.
///
/// Both roots are caller-supplied and never guessed: `golden_root` is typically
/// `Path::new(env!("CARGO_MANIFEST_DIR")).join("goldens")`, which is a
/// compile-time constant expanded in the *calling* crate, so a golden test
/// resolves to its own crate's directory with no runtime environment access and
/// no assumption about the process working directory.
#[derive(Debug, Clone)]
pub struct GoldenSettings {
    pub golden_root: PathBuf,
    pub artifact_root: PathBuf,
    pub capture: CaptureId,
    pub thresholds: Thresholds,
    pub opt_ins: OptIns,
}

#[cfg(test)]
#[path = "layout_test.rs"]
mod tests;
