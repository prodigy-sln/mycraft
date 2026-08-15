//! The shipped executable, run as a real subprocess.
//!
//! Every other test in this phase calls `run` directly, which grades what the
//! library decides and nothing about whether the binary asks. That distinction
//! is not academic here: `main.rs` is three lines by design, and a `main` that
//! parsed its own arguments — or that ignored the library and did nothing —
//! would leave all of them green. `testing.md` §2 records the measured version
//! of exactly this shape, a client submitting a default intent every tick with
//! 406 of 406 tests passing.
//!
//! So these two run `CARGO_BIN_EXE_voxforge`, the executable Cargo has just
//! built, and grade it through the process boundary: the real exit status and
//! the real streams. They are deliberately one on each side of the fork —
//! success and refusal — because a `main` hard-wired to either answer passes one
//! of them.
//!
//! What they assert of the library is thin on purpose. The filled-voxel count
//! and the empty stdout are enough to say the library was consulted; what the
//! count *should* be for every model is graded where it belongs.

mod common;

use common::cli::{Exited, Filled, built_binary, document_at, filled_in};
use common::{TestResult, shown};
use tempfile::TempDir;

/// A model with no defect: every palette entry is spelled, so the report's exit
/// is zero and the subprocess's status says so.
///
/// Twenty-two of its twenty-four cells are filled — the middle layer's far row
/// carries a two-voxel gap, which is also what spells the empty marker.
const SOUND_MODEL: &str = r#"schema = 1
name = "base:probe"
scale = 16
size = [4, 3, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
"." = "empty"
"r" = "base:ruby"

[[layers]]
y = 0
grid = """
rrrr
rrrr
"""

[[layers]]
y = 1
grid = """
rrrr
r..r
"""

[[layers]]
y = 2
grid = """
rrrr
rrrr
"""
"#;

/// How many voxels [`SOUND_MODEL`] fills: eight, then six, then eight.
const FILLED_VOXELS: usize = 8 + 6 + 8;

/// A document declaring a schema this tool does not support.
///
/// The refusal is the loader's, which is the point: whatever `main` does, the
/// only thing that knows schema 2 is unsupported is the library.
const NEWER_SCHEMA: &str = r#"schema = 2
name = "base:probe"
scale = 16
size = [1, 1, 1]
origin = [0, 0, 0]
slice = "y"

[palette]
"r" = "base:ruby"

[[layers]]
y = 0
grid = """
r
"""
"#;

#[test]
fn the_built_binary_inspects_a_sound_document_and_prints_its_filled_voxel_count() -> TestResult {
    let temp = TempDir::new()?;
    let document = document_at(&temp, "probe.mcvox", SOUND_MODEL)?;

    let run = built_binary(&["inspect", &shown(&document)])?;

    assert_eq!(
        (run.exit, filled_in(&run.out)),
        (Exited::Zero, Filled::Voxels(FILLED_VOXELS)),
        "the count is a fact only the library can state, so a `main` that answered on its own could not produce it. stdout was:\n{out}\nstderr was:\n{err}",
        out = run.out,
        err = run.err
    );
    Ok(())
}

#[test]
fn the_built_binary_refuses_a_document_of_a_newer_schema_without_writing_to_stdout() -> TestResult {
    let temp = TempDir::new()?;
    let document = document_at(&temp, "future.mcvox", NEWER_SCHEMA)?;

    let run = built_binary(&["inspect", &shown(&document)])?;

    assert_eq!(
        (run.exit, run.out.as_str(), run.err.trim().is_empty()),
        (Exited::NonZero(1), "", false),
        "a silent stdout is also what a binary that does nothing produces; the refusal on stderr is what separates the two. stderr was:\n{}",
        run.err
    );
    Ok(())
}
