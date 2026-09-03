//! Two readings of this repository's quality gate: what a run of it reports, and
//! what its text says about the order its stages run in.
//!
//! **Nothing in this repository has ever tested `scripts/sdd-gate.ps1`**, so both
//! readings are new ground and the split between them is the honest part. A run
//! can answer what a stage *does* — it fails, it names a path, it writes a tool's
//! refusal through — and it can answer nothing about a stage it did not select. A
//! text scan can answer ordering and guarding, and it can answer nothing about
//! behaviour. Each scenario is graded by whichever of the two can see it, and
//! [`reading`]'s header says which questions are left to a human.
//!
//! **The gate script is not this crate's**, and neither is the property under
//! test. The tests live here because this crate is where the repository's other
//! whole-tree scans live — the pages a refusal is quoted on, the blocks the
//! shipped content declares — and because it is excluded from the coverage
//! denominator wholesale, so a subprocess-driving test cannot flatter a number.

#![allow(dead_code)]

use std::error::Error;
use std::path::{Path, PathBuf};

/// Reading how much of a suite the gate's invocations run, and what it says
/// about the mode a reader runs in a tight loop.
pub mod extent;
/// Reading the gate script's text.
pub mod reading;
/// Running the gate script and reading what it reported.
pub mod running;
/// Running a five-test suite the way the gate's invocations are written to.
pub mod suite;

/// The repository's own root, located upwards from the crate this test binary
/// belongs to.
///
/// # Errors
///
/// Returns an error if this crate is not two levels below a repository root.
pub fn repository_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the crate manifest directory has no repository root above it")?
        .to_path_buf())
}

/// The one gate script. There is deliberately no second one.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located, or if there is no
/// script there — which is a broken reading rather than a passing scan.
pub fn gate_script() -> Result<PathBuf, Box<dyn Error>> {
    let script = repository_root()?.join("scripts").join("sdd-gate.ps1");
    if !script.is_file() {
        return Err(format!("there is no gate script at {}", script.display()).into());
    }
    Ok(script)
}

/// Where this crate's committed fixture content roots sit, as the gate has to be
/// given them: relative to the repository root.
///
/// **They are repository paths and not temporary ones, and that is forced.** The
/// stage they are handed to runs `git ls-files` against the real repository, and
/// git refuses a pathspec outside the worktree — `is outside repository at ...`,
/// exit 128 — so a temporary tree makes a clean fixture and a dirty one fail
/// identically, for a reason that has nothing to do with the property. It is the
/// same reason `architecture.md`'s D15 gives for there being no `-RepoRoot`, one
/// level down.
pub const FIXTURE_ROOTS: &str = "crates/mc-client/tests/fixtures/gate";
