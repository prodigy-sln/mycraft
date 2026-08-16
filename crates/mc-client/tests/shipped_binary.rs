//! The shipped executable, run as a real subprocess.
//!
//! Every other test of the reporting calls the library — it renders a failure, or
//! asks what `report` writes to a sink it was handed. All of that grades what the
//! library decides and nothing about whether the binary asks. **A `main` that
//! dropped its `report` call entirely, or wrote the refusal to standard output,
//! leaves every one of them green**, and so does the scan in
//! `tests/reporting_seam.rs`: nothing there composes a report, because nothing
//! there prints one either. `testing.md` §2 records the measured version of this
//! shape twice over — a client submitting a default intent every tick with 406 of
//! 406 tests passing, and `tools/voxforge/tests/binary.rs`'s own header, where a
//! `main` gutted to ignore its library left 123 of 125 green.
//!
//! So this runs `CARGO_BIN_EXE_mc-client`, the executable Cargo has just built,
//! and grades it through the process boundary: the real streams and the real exit
//! status.
//!
//! **It needs no device and no display server**, which is what makes it cheap
//! enough to have. `run` asks for the content root first and returns on the refusal
//! before it spawns the preparation and before it opens a device, so a binary
//! started somewhere without one refuses without touching the GPU.
//!
//! # What it does not witness, and this must not be over-read
//!
//! It says nothing about the guidance a site supplies. The refusal on this path is
//! a missing content root, whose way out is empty by construction; the one
//! production line that can emit the way-out sentence needs a device and a window,
//! and stays uncovered. A test that closes a real hole is exactly when somebody is
//! most tempted to read it as closing the one beside it.

use std::error::Error;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use mc_client::startup::PreparationError;
use mc_render::window::rendered;

type TestResult = Result<(), Box<dyn Error>>;

/// How a subprocess ended.
///
/// Three-valued rather than a boolean: a process killed by a signal carries no
/// status at all, and that must not read as "it refused".
#[derive(Debug, PartialEq, Eq)]
enum Exited {
    /// Successfully.
    Zero,
    /// With a failing status, whichever one — the mapping from ending to status
    /// is graded where it lives and is not this test's subject.
    NonZero,
    /// Carrying no status at all.
    WithoutACode,
}

#[test]
fn the_shipped_binary_started_away_from_its_content_says_why_on_its_error_stream() -> TestResult {
    let elsewhere = tempfile::tempdir()?;

    let finished = Command::new(env!("CARGO_BIN_EXE_mc-client"))
        .current_dir(elsewhere.path())
        .output()?;
    let said = String::from_utf8(finished.stderr)?;
    let printed = String::from_utf8(finished.stdout)?;

    // Built by refusing the same way the client does and rendering it through the
    // shipped renderer, never by pasting what a run was observed to say. The
    // looked-for directory is assembled from its two components, so it spells
    // itself the way this platform spells a path.
    let refusal = rendered(&PreparationError::NoContentRoot {
        root: ["content", "base"].iter().collect::<PathBuf>(),
    });
    let expected = format!("mycraft: {refusal}\n");

    assert_eq!(
        (
            said.as_str(),
            exit_of(&finished.status),
            printed.contains(&refusal),
            printed.is_empty()
        ),
        (expected.as_str(), Exited::NonZero, false, false),
        "the shipped binary has to reach the reporting, write the whole refusal and nothing else \
         to the stream a mod author reads refusals on, and end with a status a shell can act on. \
         A silent error stream is also what a binary that never reported produces, and a refusal \
         on standard output is one a person piping the client's output past a pager would lose. \
         What it printed was:\n{printed}"
    );
    Ok(())
}

/// How `status` ended, without pinning which failing code it chose.
fn exit_of(status: &ExitStatus) -> Exited {
    match status.code() {
        Some(0) => Exited::Zero,
        Some(_) => Exited::NonZero,
        None => Exited::WithoutACode,
    }
}
