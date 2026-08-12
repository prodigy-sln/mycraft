//! The meshing benchmark, and the budget check whose verdict is its exit code.
//!
//! Run it with:
//!
//! ```text
//! cargo bench -p mc-world --bench meshing
//! ```
//!
//! **Required at two points, and only these**: this spec's own validation, and
//! MVP 1 exit verification. Both are deliberate acts by somebody who can account
//! for the machine they are on, which is exactly why this is not a stage of
//! `scripts/sdd-gate.ps1`. A wall-clock threshold is not deterministic, and a
//! gate that goes red on a slower machine is a gate people learn to waive; a
//! waived gate protects nothing while costing every future run. The command is
//! repeated here rather than left to the documentation because the first of its
//! two run points falls before anything about it lands in `docs/`.
//!
//! It does three things, in this order:
//!
//! 1. **Asserts the work.** Each fixture must be worth what was independently
//!    established for it, or the run exits non-zero *before a single timing is
//!    measured or reported*. A mesher that emitted nothing would benchmark
//!    superbly, so a timing means nothing until the run it was taken over is
//!    known to have done the right work. This step is never waived.
//! 2. **Measures and reports with criterion**, which is where the committed
//!    baseline and the regression history live.
//! 3. **Judges its own mean** against the budget, printing both numbers.
//!
//! **Two numbers therefore exist for the same work, and only one of them gates.**
//! The check measures its own mean rather than reading criterion's because
//! criterion returns no estimate to the caller and documents its
//! `estimates.json` as a private implementation detail whose structure may
//! change — a verdict built on that breaks silently on an upgrade. So: the
//! check's mean is the only number that decides anything. Criterion's estimate
//! gates nothing, no test reads it, and nothing here compares the two. The run
//! says so in its own output as well, because a reader must not have to guess
//! which number to optimise against.
//!
//! `MYCRAFT_SKIP_PERF_BUDGET` waives step 3 and nothing else. Step 1 still runs
//! and can still fail, so a waived run never means that nothing was verified.

use std::error::Error;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use criterion::Criterion;
use mc_world::mesh::{Neighbours, SectionMesh, mesh_section};

mod support;

use support::budget::{
    Budget, BudgetOptIn, BudgetOutcome, ExpectedWork, WorkMismatch, check, judge_work,
};
use support::fixtures::{self, Fixture};
use support::oracle::{Neighbourhood, visible_faces};

/// The fixtures, named once each.
const TERRAIN: &str = "terrain";
const SOLID: &str = "solid";
const CHECKERBOARD: &str = "checkerboard";

/// What the representative workload is held to. The renderer's own budget, and
/// MVP 1's exit criterion.
const TERRAIN_BUDGET: Duration = Duration::from_micros(200);

/// What the merge-defeating worst case is held to — five times the
/// representative budget for roughly eight times the quads, so an algorithmic
/// blow-up fails here while a constant factor does not.
const CHECKERBOARD_BUDGET: Duration = Duration::from_millis(1);

/// How many sides a cube has, and therefore what an entirely solid section with
/// nothing loaded beside it is worth. Established by looking at a cube, never by
/// running this mesher and writing down what it said.
const SIDES_OF_A_CUBE: usize = 6;

/// How many times a fixture is meshed before the stopwatch starts, and how many
/// times it is meshed under it.
const WARMUP_ITERATIONS: u32 = 100;
const TIMED_ITERATIONS: u32 = 500;

/// One fixture, meshed once, with what it is worth and what it is allowed.
///
/// A fixture with no budget is measured and reported like the others and simply
/// gets no verdict: the entirely solid section would pass any budget worth
/// setting, so a budget on it would assert nothing.
#[derive(Debug)]
struct Subject {
    name: &'static str,
    fixture: Fixture,
    mesh: SectionMesh,
    expected: ExpectedWork,
    budget: Option<Duration>,
}

fn main() -> ExitCode {
    match run() {
        Ok(outcome) => reported(&outcome),
        Err(why) => {
            eprintln!("the meshing benchmark could not run: {why}");
            ExitCode::FAILURE
        }
    }
}

/// The verdict, printed, and the exit code it maps to.
fn reported(outcome: &BudgetOutcome) -> ExitCode {
    print!("{outcome}");
    if outcome.is_success() {
        return ExitCode::SUCCESS;
    }
    ExitCode::FAILURE
}

/// Work first, then criterion's report, then this check's own verdict.
///
/// The ordering rule itself lives in `check`, which is asked twice: once with no
/// timings at all, to decide whether measuring is worth doing, and once with
/// them. That keeps "work first, work never waived" in the one place a test can
/// link against instead of restating it here.
fn run() -> Result<BudgetOutcome, Box<dyn Error>> {
    let subjects = subjects()?;
    let opt_in = BudgetOptIn::from_environment();
    let work = work_assertions(&subjects);

    let before_timing = check(&work, &[], opt_in);
    if !before_timing.is_success() {
        return Ok(before_timing);
    }

    measure_with_criterion(&subjects);
    state_which_number_gates();
    let means = own_means(&subjects)?;
    Ok(check(&work, &means, opt_in))
}

/// The three committed fixtures, meshed, each beside what it is worth.
///
/// Not one of these quantities is a number read off a mesher run. The solid
/// section is six quads because a cube has six sides; the checkerboard is one
/// quad per side of each of its solid voxels, counted through the registry; and
/// terrain has no expected number at all — the sides its quads cover must equal
/// what an independent per-voxel scan finds visible.
fn subjects() -> Result<Vec<Subject>, Box<dyn Error>> {
    let terrain = fixtures::terrain()?;
    let solid = fixtures::solid()?;
    let checkerboard = fixtures::checkerboard()?;

    let terrain_work = ExpectedWork::CoveredFaces(visible_face_count(&terrain)?);
    let checkerboard_work =
        ExpectedWork::Quads(solid_voxel_count(&checkerboard)? * SIDES_OF_A_CUBE);
    let solid_work = ExpectedWork::Quads(SIDES_OF_A_CUBE);

    Ok(vec![
        prepare(TERRAIN, terrain, terrain_work, Some(TERRAIN_BUDGET))?,
        prepare(SOLID, solid, solid_work, None)?,
        prepare(
            CHECKERBOARD,
            checkerboard,
            checkerboard_work,
            Some(CHECKERBOARD_BUDGET),
        )?,
    ])
}

/// One fixture, meshed once so its work can be judged before anything is timed.
fn prepare(
    name: &'static str,
    fixture: Fixture,
    expected: ExpectedWork,
    budget: Option<Duration>,
) -> Result<Subject, Box<dyn Error>> {
    let mesh = meshed(&fixture)?;
    Ok(Subject {
        name,
        fixture,
        mesh,
        expected,
        budget,
    })
}

/// Whether each fixture did the work established for it.
fn work_assertions(subjects: &[Subject]) -> Vec<Result<(), WorkMismatch>> {
    subjects
        .iter()
        .map(|subject| judge_work(subject.name, subject.expected, &subject.mesh))
        .collect()
}

/// Criterion's measurement and report, for all three fixtures.
///
/// This is the diagnostic a breach is investigated with and the baseline a
/// regression is caught against. It decides nothing.
fn measure_with_criterion(subjects: &[Subject]) {
    let mut criterion = Criterion::default().configure_from_args();
    for subject in subjects {
        criterion.bench_function(subject.name, |bencher| {
            bencher.iter(|| meshed(&subject.fixture));
        });
    }
    criterion.final_summary();
}

/// Which of the two numbers a reader should act on.
fn state_which_number_gates() {
    println!(
        "\nTwo numbers exist for the same work, and only one of them decides anything.\n\n\
         The verdicts below are this check's own mean over {TIMED_ITERATIONS} timed iterations, \
         taken after {WARMUP_ITERATIONS} warmup ones. That mean is the only number that gates: it \
         is what the exit code is computed from. Criterion's estimate above gates nothing at all \
         — no test reads it, nothing in this repository compares the two, and it is kept for the \
         committed baseline it carries rather than for any pass or fail. Optimise against the \
         mean below.\n"
    );
}

/// This check's own mean for each budgeted fixture.
fn own_means(subjects: &[Subject]) -> Result<Vec<(Budget, Duration)>, Box<dyn Error>> {
    let mut means = Vec::new();
    for subject in subjects {
        let Some(allowed) = subject.budget else {
            continue;
        };
        let budget = Budget {
            fixture: subject.name,
            allowed,
        };
        means.push((budget, mean_of(&subject.fixture)?));
    }
    Ok(means)
}

/// How long one fixture takes to mesh, on average, measured here.
///
/// Warm up, run a fixed number of timed iterations with the result held behind
/// `black_box` so none of it is optimised away, and divide the total. Thirty
/// lines' worth of stopwatch, rather than a verdict built on criterion's
/// `estimates.json` — a file its own documentation calls a private
/// implementation detail whose structure may change.
fn mean_of(fixture: &Fixture) -> Result<Duration, Box<dyn Error>> {
    for _ in 0..WARMUP_ITERATIONS {
        black_box(meshed(fixture)?);
    }
    let started = Instant::now();
    for _ in 0..TIMED_ITERATIONS {
        black_box(meshed(fixture)?);
    }
    Ok(started.elapsed() / TIMED_ITERATIONS)
}

/// A fixture meshed with no neighbour supplied.
fn meshed(fixture: &Fixture) -> Result<SectionMesh, Box<dyn Error>> {
    Ok(mesh_section(
        &fixture.section,
        &Neighbours::none(),
        &fixture.registry,
    )?)
}

/// How many faces the independent scan finds visible, with no neighbour
/// supplied.
fn visible_face_count(fixture: &Fixture) -> Result<usize, Box<dyn Error>> {
    let found = visible_faces(
        &fixture.section,
        &Neighbourhood::default(),
        &fixture.registry,
    )?;
    Ok(found.len())
}

/// How many of a fixture's voxels its own registry reports as solid.
///
/// Counted through the public per-voxel read, so what is counted is the solidity
/// each block was registered with rather than anything recognised by name.
fn solid_voxel_count(fixture: &Fixture) -> Result<usize, Box<dyn Error>> {
    let mut solid = 0;
    for position in fixtures::every_position() {
        solid += usize::from(fixture.section.is_solid_at(position, &fixture.registry)?);
    }
    Ok(solid)
}
