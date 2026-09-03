//! A five-test crate, run the way the gate's own invocations are written to run
//! a suite, and read for both halves of what a red run has to say.
//!
//! # Why the count and the verdict are read together
//!
//! `cargo-llvm-cov` carries a second flag whose help text reads more like this
//! defect's ask than the right one does, and it produces a **byte-identical**
//! summary line: `5 tests run: 2 passed, 3 failed` either way. The two differ
//! only in exit code — 100 against 0 — and the exit code is the one thing
//! `Invoke-Stage` inspects. A reading that took the count and stopped would be
//! green on an implementation that makes the gate pass with red tests, so the
//! count and the verdict are one observation here and the two arms that carry
//! the same count are separate variants of it.
//!
//! # What the fixture supplies that the gate does not, and why
//!
//! The run is given `--test-threads=1`. The gate is not, and the difference is
//! deliberate: at default parallelism every test of a five-test suite has
//! already started before the first failure registers, so a fixture this small
//! cannot exhibit cancellation at all — the flag makes the fixture stand in for
//! a suite large enough to, which is the workspace suite the gate actually runs
//! and where PRO-994 recorded `1294/1591 tests run`. Everything the reading is
//! *about* — the extent flags — comes out of the shipped script rather than
//! from here.
//!
//! The crate is built outside the repository with the enclosing run's coverage
//! instrumentation cleared from its environment, so a nested build neither
//! inherits an instrumented profile nor writes profile data into the outer
//! run's collection.

use std::error::Error;
use std::fs;
use std::process::Command;

use tempfile::TempDir;

/// The five tests, three of which fail, in the order a single-threaded run takes
/// them: a pass, three failures, a pass. The first failure has two tests after
/// it, which is what a cancelled run has to leave unexecuted.
const FIVE_TESTS: &str = r"
#[test]
fn a_first_test_passes() {}
#[test]
fn b_second_test_fails() {
    assert_eq!(1, 2);
}
#[test]
fn c_third_test_fails() {
    assert_eq!(1, 2);
}
#[test]
fn d_fourth_test_fails() {
    assert_eq!(1, 2);
}
#[test]
fn e_fifth_test_passes() {}
";

/// A crate of its own, so nothing above it in the filesystem claims it.
const FIXTURE_MANIFEST: &str = r#"
[package]
name = "gate-extent-fixture"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]
"#;

/// The coverage machinery of an enclosing run, cleared before the child starts.
const INHERITED_INSTRUMENTATION: [&str; 8] = [
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
    "RUSTDOCFLAGS",
    "LLVM_PROFILE_FILE",
    "CARGO_LLVM_COV",
    "CARGO_LLVM_COV_TARGET_DIR",
    "CARGO_TARGET_DIR",
];

/// What one run of a suite with three failures among five reported about itself.
#[derive(Debug, PartialEq, Eq)]
pub enum SuiteExtent {
    /// Every test ran, the count is the complete bare form, and the run failed.
    TheWholeSuiteRanAndTheRunFailed {
        /// How many tests executed.
        ran: usize,
        /// How many of them passed.
        passed: usize,
        /// How many of them failed.
        failed: usize,
    },
    /// The run stopped at the first failure: `ran` of `total` ever executed, and
    /// the count says nothing whatever about the rest.
    TheRunWasCancelledAtTheFirstFailure {
        /// How many tests executed.
        ran: usize,
        /// How many there were.
        total: usize,
    },
    /// Every test ran and the failures were reported — and the run exited 0, so
    /// the only thing the gate reads says the stage passed.
    TheWholeSuiteRanAndTheRunPassedAnyway {
        /// How many tests executed.
        ran: usize,
        /// How many of them passed.
        passed: usize,
        /// How many of them failed.
        failed: usize,
    },
    /// The runner refused the flags it was given and ran nothing.
    TheRunnerRefusedTheFlags,
    /// The runner printed no summary this reading could find.
    NoSummaryWasPrinted,
}

/// Runs five tests, three of them failing, under `flags`.
///
/// # Errors
///
/// Returns an error if the fixture cannot be written or `cargo` cannot be
/// started. A missing runner fails the test rather than skipping it: a reading
/// that could not run and a reading that found nothing must not look alike.
pub fn five_tests_three_failing(flags: &[String]) -> Result<SuiteExtent, Box<dyn Error>> {
    let crate_root = TempDir::new()?;
    fs::write(crate_root.path().join("Cargo.toml"), FIXTURE_MANIFEST)?;
    fs::create_dir(crate_root.path().join("src"))?;
    fs::write(crate_root.path().join("src").join("lib.rs"), FIVE_TESTS)?;

    let mut runner = Command::new("cargo");
    runner
        .args(["nextest", "run", "--test-threads=1"])
        .args(flags)
        .current_dir(crate_root.path());
    for inherited in INHERITED_INSTRUMENTATION {
        runner.env_remove(inherited);
    }
    let finished = runner
        .env("CARGO_TARGET_DIR", crate_root.path().join("target"))
        .output()?;
    let said = String::from_utf8_lossy(&finished.stderr);
    Ok(extent_of(&said, finished.status.success()))
}

/// What a run reported, from its summary line and its verdict.
fn extent_of(said: &str, passed: bool) -> SuiteExtent {
    let Some(summary) = said.lines().find(|line| line.contains(" tests run:")) else {
        return if said.contains("unexpected argument") || said.contains("unrecognized") {
            SuiteExtent::TheRunnerRefusedTheFlags
        } else {
            SuiteExtent::NoSummaryWasPrinted
        };
    };
    let Some(counted) = counts_of(summary) else {
        return SuiteExtent::NoSummaryWasPrinted;
    };
    match (counted, passed) {
        ((ran, Some(total), _, _), _) => {
            SuiteExtent::TheRunWasCancelledAtTheFirstFailure { ran, total }
        }
        ((ran, None, passed_count, failed), false) => {
            SuiteExtent::TheWholeSuiteRanAndTheRunFailed {
                ran,
                passed: passed_count,
                failed,
            }
        }
        ((ran, None, passed_count, failed), true) => {
            SuiteExtent::TheWholeSuiteRanAndTheRunPassedAnyway {
                ran,
                passed: passed_count,
                failed,
            }
        }
    }
}

/// How many tests ran, out of how many when the run was cancelled, and how many
/// of them passed and failed.
fn counts_of(summary: &str) -> Option<(usize, Option<usize>, usize, usize)> {
    let (ran, reported) = summary.split_once(" tests run:")?;
    let ran = ran.split_whitespace().next_back()?;
    let (ran, total) = match ran.split_once('/') {
        Some((ran, total)) => (ran.parse().ok()?, Some(total.parse().ok()?)),
        None => (ran.parse().ok()?, None),
    };
    Some((
        ran,
        total,
        counted(reported, "passed"),
        counted(reported, "failed"),
    ))
}

/// The number standing before `label` in a summary's tail, or zero.
fn counted(reported: &str, label: &str) -> usize {
    let mut words = reported.split_whitespace().peekable();
    while let Some(word) = words.next() {
        if words
            .peek()
            .is_some_and(|next| next.trim_matches(',') == label)
        {
            return word.parse().unwrap_or_default();
        }
    }
    0
}
