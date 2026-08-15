//! What a model measurably is, what is wrong with it, and what is merely worth
//! knowing.
//!
//! The partition is the whole point of this module. A **defect** sets a
//! non-zero exit; an **observation** never does. They are two types rather than
//! one enum carrying a severity, so that [`Report::exit_code`] is structurally
//! unable to consult an observation — a severity field invites
//! `if finding.severity == …` at the exit-code site, where one miscategorised
//! variant changes silently what an agent branches on.
//!
//! Connectivity and symmetry sit on the observation side deliberately: a voxel
//! clear of every other may be a detached hinge or may be a portal particle,
//! and nothing in the document distinguishes them. The tool reports the fact
//! and leaves the judgement to whoever asked.

mod connectivity;
mod stats;

use std::path::Path;
use std::process;

pub use connectivity::{Component, Floating, SymmetryVerdict};
pub use stats::{Bounds, MaterialCount, Stats};

use crate::fault::Fault;
use crate::format::{Axis, Model};
use crate::volume::{StateSelection, Volume};

/// What the process exits with once a report has been printed.
///
/// VoxForge's own two-valued enum rather than [`std::process::ExitCode`], which
/// is opaque: it implements no equality and exposes no value, so nothing could
/// assert that a defective model exits non-zero. The conversion into the
/// standard type is the last thing that happens, in the binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// The model carries no defect.
    Success,
    /// The model carries at least one defect.
    Defective,
}

impl From<ExitCode> for process::ExitCode {
    fn from(code: ExitCode) -> Self {
        match code {
            ExitCode::Success => Self::SUCCESS,
            ExitCode::Defective => Self::FAILURE,
        }
    }
}

/// Something wrong with the document, which sets a non-zero exit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Defect {
    /// The palette declares this key and no grid of the document spells it.
    UnusedPaletteEntry {
        /// The ASCII byte the unused entry is keyed by.
        key: u8,
    },
}

/// Something true about the model that is not, by itself, wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observation {
    /// How the assembled model holds together.
    ///
    /// Both facts ride in one observation because they are one computation: a
    /// voxel touching no other *is* a component of size 1, so the two cannot
    /// disagree unless something has computed them twice.
    Connectivity {
        /// Every face-connected group, ascending by lowest voxel.
        components: Vec<Component>,
        /// Which voxels share a face with nothing.
        floating: Floating,
    },
    /// Whether the model mirrors about one axis' midplane.
    Symmetry {
        /// The axis mirrored about.
        axis: Axis,
        /// What the mirror found.
        verdict: SymmetryVerdict,
    },
}

/// Everything `inspect` has to say about one model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// What the model measurably is.
    stats: Stats,
    /// What is wrong with it.
    defects: Vec<Defect>,
    /// What is worth knowing about it.
    observations: Vec<Observation>,
}

impl Report {
    /// What the model measurably is.
    #[must_use]
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// What is wrong with the model.
    #[must_use]
    pub fn defects(&self) -> &[Defect] {
        &self.defects
    }

    /// What is worth knowing about the model.
    #[must_use]
    pub fn observations(&self) -> &[Observation] {
        &self.observations
    }

    /// What the process exits with: non-zero exactly when a defect was found.
    ///
    /// Reads `defects` and nothing else. An observation cannot reach this
    /// answer, because an observation is not the same type.
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        if self.defects.is_empty() {
            return ExitCode::Success;
        }
        ExitCode::Defective
    }
}

/// What `volume`, assembled from `model`, reports.
#[must_use]
pub fn inspect(volume: &Volume, model: &Model) -> Report {
    Report {
        stats: stats::of(volume),
        defects: defects_of(model),
        observations: vec![
            Observation::Connectivity {
                components: connectivity::components(volume),
                floating: connectivity::floating(volume),
            },
            Observation::Symmetry {
                axis: Axis::X,
                verdict: connectivity::mirrors_on_x(volume),
            },
        ],
    }
}

/// The report the document at `path` earns, assembled under `states`.
///
/// # Errors
///
/// Returns the [`Fault`] loading or assembling the document earns. A refusal
/// carries no report: a document that could not be read has no measurable facts
/// to state, and reporting zeroes for one would be a statement about a model
/// nobody has.
pub fn inspect_document(path: &Path, states: &StateSelection) -> Result<Report, Fault> {
    let model = crate::format::load_document(path)?;
    let volume = crate::volume::assemble(&model, states)?;
    Ok(inspect(&volume, &model))
}

/// Every defect `model` carries.
///
/// Read from the document's own spelled characters rather than from the
/// assembled volume's materials: an entry mapping to the empty marker is used
/// by every grid that spells it and appears in no volume, so a volume-derived
/// answer would report `.` as unused on a perfectly clean document.
fn defects_of(model: &Model) -> Vec<Defect> {
    model
        .unused_palette_keys()
        .into_iter()
        .map(|key| Defect::UnusedPaletteEntry { key })
        .collect()
}
