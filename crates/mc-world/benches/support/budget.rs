//! The budget check's verdict: what a fixture was worth, whether a mean fits
//! inside its budget, and the one rule about the order those two are asked in.
//!
//! **Nothing here reads a clock.** The measured mean arrives as a value, which is
//! the only thing that makes a *failing* budget testable: you cannot make a
//! correct mesher take 250 µs on demand, so a verdict fused to the act of
//! measuring could only ever be exercised on its passing branch. Separating them
//! turns every scenario about a breach into an ordinary comparison over supplied
//! numbers, and leaves the impure half — one stopwatch and one environment read —
//! at the single edge that `meshing.rs` owns.
//!
//! `Duration` throughout and no floating point anywhere, so there is no
//! approximate comparison to get subtly wrong and `clippy::float_cmp` never comes
//! into it.
//!
//! **A timing is never reported for a run whose work did not check out.** A
//! mesher that emitted nothing would benchmark superbly, so a number describing
//! how fast something ran means nothing until that something is shown to have
//! done the work established for it. That rule lives in [`check`] rather than in
//! the benchmark's `main`, because a rule living in a `main` is a rule no test
//! links against.

use std::ffi::OsString;
use std::fmt;
use std::time::Duration;

use mc_world::mesh::{Quad, SectionMesh};

/// Waives the timing comparison, and nothing else.
///
/// Named for the project rather than for the mesher because the later budgets —
/// the tick p99 among them — will read the same variable.
pub const SKIP_PERF_BUDGET: &str = "MYCRAFT_SKIP_PERF_BUDGET";

/// How long one fixture is allowed to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub fixture: &'static str,
    pub allowed: Duration,
}

/// Which of the check's opt-ins the caller has asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BudgetOptIn {
    pub skip_timing: bool,
}

impl BudgetOptIn {
    /// Reads the opt-in through `lookup`.
    ///
    /// **Presence, not value, enables it**: `MYCRAFT_SKIP_PERF_BUDGET=0` still
    /// asks for the skip, exactly as the frame harness's two opt-ins do. A
    /// variable somebody bothered to set is a request, and reading it as a
    /// boolean would enforce the budget on a machine whose owner believed they
    /// had waived it.
    #[must_use]
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<OsString>) -> Self {
        Self {
            skip_timing: lookup(SKIP_PERF_BUDGET).is_some(),
        }
    }

    /// Reads the opt-in from the process environment.
    #[must_use]
    pub fn from_environment() -> Self {
        Self::from_lookup(environment_lookup)
    }
}

/// Reads one variable from the process environment.
///
/// The only function in this module that names `std::env`, which is what keeps
/// the rest of it a pure comparison over values. It exists as a named function
/// rather than as `from_lookup(std::env::var_os)` because `var_os` is generic
/// over its key type and cannot satisfy a higher-ranked bound directly.
fn environment_lookup(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// What a fixture is known to be worth, and how that was established.
///
/// **Never a snapshot of the mesher's own output.** A count committed from the
/// first green run makes the assertion circular — a mesher that emitted nothing
/// gets `0` committed for it and passes forever. Two variants exist for exactly
/// that reason: comparing a bare count for every fixture is what would let the
/// terrain fixture's expected value degenerate into whatever came out of it the
/// day somebody wrote the number down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedWork {
    /// How many quads, where that number can be reached by looking at the
    /// fixture: six for a cube, and one per side of each of a checkerboard's
    /// solid voxels.
    Quads(usize),
    /// How many voxel sides the quads cover in total, which is what an
    /// independent per-voxel scan counts. This is how a fixture is pinned
    /// without a committed number for it existing at all.
    CoveredFaces(usize),
}

impl ExpectedWork {
    /// The quantity this names, read off `mesh`.
    fn read_from(self, mesh: &SectionMesh) -> usize {
        match self {
            Self::Quads(_) => mesh.quads().len(),
            Self::CoveredFaces(_) => covered_faces(mesh.quads()),
        }
    }

    /// How much of that quantity the fixture is established to be worth.
    const fn established(self) -> usize {
        match self {
            Self::Quads(count) | Self::CoveredFaces(count) => count,
        }
    }

    /// What the numbers are counted in, so a mismatch reads as a sentence.
    const fn counted_in(self) -> &'static str {
        match self {
            Self::Quads(_) => "quads",
            Self::CoveredFaces(_) => "covered voxel sides",
        }
    }
}

/// How many voxel sides `quads` cover in total, counting each quad's whole
/// rectangle.
fn covered_faces(quads: &[Quad]) -> usize {
    quads
        .iter()
        .map(|quad| (quad.extent.primary * quad.extent.secondary) as usize)
        .sum()
}

/// A fixture that did not do the work established for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkMismatch {
    pub fixture: &'static str,
    pub expected: usize,
    pub produced: usize,
    counted_in: &'static str,
}

impl fmt::Display for WorkMismatch {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "the `{fixture}` fixture produced {produced} {counted_in} where {expected} are \
             established for it, so whatever it was measured doing is not the work this budget \
             was written about",
            fixture = self.fixture,
            produced = self.produced,
            counted_in = self.counted_in,
            expected = self.expected
        )
    }
}

/// What a measured mean was judged to be.
///
/// Every variant carries **both** numbers, so a report can never say that
/// something was too slow without saying how much too slow and against what.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingVerdict {
    Within {
        measured: Duration,
        allowed: Duration,
    },
    Exceeded {
        measured: Duration,
        allowed: Duration,
    },
    Waived {
        measured: Duration,
        allowed: Duration,
    },
}

impl TimingVerdict {
    /// Whether this verdict fails the check.
    ///
    /// A waived comparison is not a breach — that is the whole of what the
    /// waiver does.
    #[must_use]
    pub const fn is_breach(self) -> bool {
        matches!(self, Self::Exceeded { .. })
    }
}

impl fmt::Display for TimingVerdict {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Within { measured, allowed } => write!(
                out,
                "meshed in {measured:?} on average, inside its {allowed:?} budget"
            ),
            Self::Exceeded { measured, allowed } => write!(
                out,
                "meshed in {measured:?} on average, over its {allowed:?} budget"
            ),
            // The literal the lookup reads, not a second spelling of it: a
            // warning that named some other variable would send whoever read it
            // looking for a switch nothing consults.
            Self::Waived { measured, allowed } => write!(
                out,
                "meshed in {measured:?} on average against a budget of {allowed:?}, not compared \
                 because {SKIP_PERF_BUDGET} is set"
            ),
        }
    }
}

/// One fixture's timing verdict, and the fixture it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixtureTiming {
    pub fixture: &'static str,
    pub verdict: TimingVerdict,
}

impl fmt::Display for FixtureTiming {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "  {}: {}", self.fixture, self.verdict)
    }
}

/// Everything the check decided, in the order it decided it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetOutcome {
    mismatches: Vec<WorkMismatch>,
    timings: Vec<FixtureTiming>,
}

impl BudgetOutcome {
    /// The fixtures that did not do the work established for them.
    #[must_use]
    pub fn mismatches(&self) -> &[WorkMismatch] {
        &self.mismatches
    }

    /// The timing verdicts reached.
    ///
    /// Empty whenever a work assertion failed. A timing taken over the wrong
    /// work is not a slower or faster version of the right answer; it is a
    /// number about something else, and printing it beside a failure would put
    /// it in front of somebody about to optimise against it.
    #[must_use]
    pub fn timings(&self) -> &[FixtureTiming] {
        &self.timings
    }

    /// Whether the check passed, which is what the command's exit code is.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.mismatches.is_empty() && !self.timings.iter().any(|timing| timing.verdict.is_breach())
    }
}

impl fmt::Display for BudgetOutcome {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        for mismatch in &self.mismatches {
            writeln!(out, "{mismatch}")?;
        }
        if !self.mismatches.is_empty() {
            return writeln!(
                out,
                "no timing was measured or reported: a fixture that did not do its work has \
                 nothing worth timing"
            );
        }
        for timing in &self.timings {
            writeln!(out, "{timing}")?;
        }
        Ok(())
    }
}

/// Reads the quantity `expected` names off `mesh` and compares the two.
///
/// # Errors
///
/// Returns a [`WorkMismatch`] if `mesh` is not worth what `expected` says it is.
pub fn judge_work(
    fixture: &'static str,
    expected: ExpectedWork,
    mesh: &SectionMesh,
) -> Result<(), WorkMismatch> {
    let produced = expected.read_from(mesh);
    if produced == expected.established() {
        return Ok(());
    }
    Err(WorkMismatch {
        fixture,
        expected: expected.established(),
        produced,
        counted_in: expected.counted_in(),
    })
}

/// Compares a **supplied** mean against a budget.
///
/// Reads no clock and measures nothing. The waiver skips the comparison and says
/// so; it never turns a breach into a pass silently.
#[must_use]
pub fn judge_timing(budget: &Budget, measured: Duration, opt_in: BudgetOptIn) -> TimingVerdict {
    let allowed = budget.allowed;
    if opt_in.skip_timing {
        return TimingVerdict::Waived { measured, allowed };
    }
    if measured > allowed {
        return TimingVerdict::Exceeded { measured, allowed };
    }
    TimingVerdict::Within { measured, allowed }
}

/// The whole verdict: work first, work never waived, timing only once every work
/// assertion has passed.
///
/// The order is the point of this function existing. The waiver gates the timing
/// comparison and nothing else, so a waived run still runs the work assertions
/// and can still fail — which is what stops a waived run from meaning that
/// nothing was verified.
#[must_use]
pub fn check(
    work: &[Result<(), WorkMismatch>],
    timings: &[(Budget, Duration)],
    opt_in: BudgetOptIn,
) -> BudgetOutcome {
    let mismatches: Vec<WorkMismatch> = work
        .iter()
        .filter_map(|assessed| assessed.as_ref().err().cloned())
        .collect();
    if !mismatches.is_empty() {
        return BudgetOutcome {
            mismatches,
            timings: Vec::new(),
        };
    }
    BudgetOutcome {
        mismatches,
        timings: timings.iter().map(|pair| timed(pair, opt_in)).collect(),
    }
}

/// One budget and the mean measured against it, judged.
fn timed((budget, measured): &(Budget, Duration), opt_in: BudgetOptIn) -> FixtureTiming {
    FixtureTiming {
        fixture: budget.fixture,
        verdict: judge_timing(budget, *measured, opt_in),
    }
}
