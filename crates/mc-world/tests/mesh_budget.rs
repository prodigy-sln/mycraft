//! What the budget check decides, and what the waiver may and may not excuse.
//!
//! A wall-clock threshold is only checkable in its *failing* branch if the
//! verdict is separated from the act of measuring: you cannot make a correct
//! mesher take 250 µs on demand, and a check whose decision is fused to its own
//! stopwatch can only ever be tested green. So the numbers below are supplied
//! rather than observed, no clock is read anywhere in this file, and the two
//! interesting outcomes — a breach and a clean run — are ordinary comparisons.
//!
//! Two of them are the pair that has to hold together. A check that always fails
//! would pass a suite made only of breach tests, so one test here says a mean
//! inside its budget *succeeds*, and says it as a claim about success rather
//! than as the absence of a complaint.
//!
//! **No environment variable is set here, and none can be.** `std::env::set_var`
//! is `unsafe` in edition 2024 and `unsafe_code` is warned under `-D warnings`,
//! so a test that set one would need exactly the escape hatch the quality gate
//! exists to make visible. The waiver is exercised through an injected lookup
//! instead, the way the frame harness's opt-ins already are — which is also the
//! stronger test, because it names the variable the check actually reads rather
//! than trusting a process the test cannot see inside.
//!
//! The variable's name is written out here as a literal string rather than taken
//! from the constant the check uses. An assertion made against that constant
//! would follow it through a rename and stay green, and a renamed variable is a
//! variable nobody's shell is setting.
//!
//! The two work assertions reach a real mesh, because a quad count is a quantity
//! read off one. The count they are judged against is derived rather than
//! observed: an entirely solid section is worth six quads because a cube has six
//! sides, and the *mismatching* expectation is one more than that — derived from
//! the same look at a cube, never from a number a mesher printed.

mod mesh_common;

#[path = "../benches/support/mod.rs"]
mod support;

use std::error::Error;
use std::ffi::OsString;
use std::time::Duration;

use mc_world::mesh::{Neighbours, SectionMesh, mesh_section};
use mesh_common::TestResult;
use support::budget::{
    Budget, BudgetOptIn, ExpectedWork, FixtureTiming, check, judge_timing, judge_work,
};
use support::fixtures;

/// The fixture the 200 µs budget is written for.
const TERRAIN: &str = "terrain";

/// The fixture whose expected work a reader can check by eye.
const SOLID: &str = "solid";

/// The budget the representative workload is held to, and two means either side
/// of it. All three come from the scenarios themselves; none was measured.
const BUDGET: Duration = Duration::from_micros(200);
const OVER_BUDGET: Duration = Duration::from_micros(250);
const INSIDE_BUDGET: Duration = Duration::from_micros(150);

/// The waiver's own name, as a shell would spell it.
///
/// Deliberately not the constant the check reads. See the module note: an
/// assertion written against that constant renames itself along with it.
const WAIVER: &str = "MYCRAFT_SKIP_PERF_BUDGET";

/// A value that is not a request for anything, set anyway.
///
/// Presence is what the waiver turns on, so the falsest value a reader could
/// think of is the one that has to work.
const FALSE_LOOKING_VALUE: &str = "0";

/// How many sides a cube has, and therefore how many quads an entirely solid
/// section with nothing loaded beside it is worth: every interior face is hidden
/// by the voxel next to it, and each of the six outer planes is one unbroken
/// rectangle.
const SIDES_OF_A_CUBE: usize = 6;

/// A quad count no correct mesh of that section can have — one more than a cube
/// shows.
///
/// Derived from the same inspection the established count is, which is the whole
/// point: a wrong expectation taken from a mesher run would be wrong in whatever
/// direction that run happened to be wrong in, including not at all.
const MORE_THAN_A_CUBE_SHOWS: usize = SIDES_OF_A_CUBE + 1;

/// The budget the `terrain` fixture is held to.
const fn terrain_budget() -> Budget {
    Budget {
        fixture: TERRAIN,
        allowed: BUDGET,
    }
}

/// An environment holding nothing the check asks for.
fn nothing_set() -> BudgetOptIn {
    BudgetOptIn::from_lookup(|_| None)
}

/// An environment holding the waiver, set to `value`, and nothing else.
fn waiver_set_to(value: &'static str) -> BudgetOptIn {
    BudgetOptIn::from_lookup(move |name| (name == WAIVER).then(|| OsString::from(value)))
}

/// The entirely solid fixture, meshed with no neighbour supplied.
///
/// # Errors
///
/// Returns an error if the fixture cannot be built, or cannot be meshed against
/// its own registry.
fn solid_mesh() -> Result<SectionMesh, Box<dyn Error>> {
    let fixture = fixtures::solid()?;
    Ok(mesh_section(
        &fixture.section,
        &Neighbours::none(),
        &fixture.registry,
    )?)
}

#[test]
fn a_mean_over_the_budget_fails_the_check() -> TestResult {
    let outcome = check(&[Ok(())], &[(terrain_budget(), OVER_BUDGET)], nothing_set());

    assert!(
        !outcome.is_success(),
        "the representative workload meshed at {OVER_BUDGET:?} against a budget of {BUDGET:?}, \
         which is the whole thing this check exists to notice. Its verdict is the command's exit \
         code, so a check that came back successful here would leave a missed budget as a line of \
         output nobody is obliged to read"
    );
    Ok(())
}

#[test]
fn a_breach_is_reported_with_both_the_mean_measured_and_the_budget_it_missed() -> TestResult {
    let outcome = check(&[Ok(())], &[(terrain_budget(), OVER_BUDGET)], nothing_set());

    let reported = outcome.to_string();
    assert!(
        reported.contains(&format!("{OVER_BUDGET:?}")) && reported.contains(&format!("{BUDGET:?}")),
        "a breach that says only that something was too slow tells whoever reads it nothing about \
         how much too slow, and therefore nothing about whether a slower machine or a real \
         regression is the explanation. Both numbers have to be in it — the {OVER_BUDGET:?} that \
         was measured and the {BUDGET:?} it was measured against. This report said: {reported}"
    );
    Ok(())
}

#[test]
fn a_mean_inside_the_budget_passes_the_check() -> TestResult {
    let outcome = check(
        &[Ok(())],
        &[(terrain_budget(), INSIDE_BUDGET)],
        nothing_set(),
    );

    assert!(
        outcome.is_success(),
        "{INSIDE_BUDGET:?} is inside a budget of {BUDGET:?}, and the work it was measured over \
         checked out, so there is nothing here to complain about. This is the one assertion in \
         the file that a check refusing everything cannot satisfy, which is why it is written as \
         a claim that the run succeeded rather than as the absence of a failure — an empty list \
         of complaints is also what a check that never looked would produce"
    );
    Ok(())
}

#[test]
fn a_waived_timing_announces_the_variable_that_waived_it_by_name() -> TestResult {
    let verdict = judge_timing(&terrain_budget(), OVER_BUDGET, waiver_set_to("1"));

    let announced = verdict.to_string();
    assert!(
        announced.contains(WAIVER),
        "{OVER_BUDGET:?} against {BUDGET:?} is a breach, so a verdict that does not complain \
         about it has waived the comparison — and a waiver nobody can see is indistinguishable \
         from a budget that was met. It has to name the variable that did it, spelled exactly as \
         a shell would set it, so that a reader can find and unset it. This verdict said: \
         {announced}"
    );
    Ok(())
}

#[test]
fn the_waiver_is_read_as_set_rather_than_as_the_value_it_was_set_to() -> TestResult {
    let verdict = judge_timing(
        &terrain_budget(),
        OVER_BUDGET,
        waiver_set_to(FALSE_LOOKING_VALUE),
    );

    assert!(
        !verdict.is_breach(),
        "a variable someone bothered to set is a request, whatever they set it to. \
         `{WAIVER}={FALSE_LOOKING_VALUE}` reads as a refusal to anyone who expects a boolean, and \
         a check that honoured that reading would enforce the budget on a machine whose owner \
         believed they had waived it — an expensive surprise, and the same presence-not-value \
         rule the harness's other opt-ins already follow"
    );
    Ok(())
}

#[test]
fn a_fixture_that_did_not_do_the_work_established_for_it_is_reported_with_no_timing() -> TestResult
{
    let mesh = solid_mesh()?;

    let assessed = judge_work(SOLID, ExpectedWork::Quads(MORE_THAN_A_CUBE_SHOWS), &mesh);
    let outcome = check(
        &[assessed],
        &[(terrain_budget(), INSIDE_BUDGET)],
        nothing_set(),
    );

    let no_timing: Vec<FixtureTiming> = Vec::new();
    assert_eq!(
        (outcome.is_success(), outcome.timings().to_vec()),
        (false, no_timing),
        "a mesher that emitted nothing would benchmark superbly, so a timing means nothing until \
         the run it was taken over is shown to have done the work established for it. A fixture \
         that did something other than that work fails the check, and its timing is not reported \
         at all — reporting it as passing would put a number people optimise against next to a \
         result that measured the wrong thing"
    );
    Ok(())
}

#[test]
fn the_waiver_never_excuses_a_fixture_that_did_not_do_its_work() -> TestResult {
    let mesh = solid_mesh()?;

    let assessed = judge_work(SOLID, ExpectedWork::Quads(MORE_THAN_A_CUBE_SHOWS), &mesh);
    let outcome = check(
        &[assessed],
        &[(terrain_budget(), INSIDE_BUDGET)],
        waiver_set_to("1"),
    );

    assert!(
        !outcome.is_success(),
        "the waiver exists because a wall-clock number is machine-dependent, and a quad count is \
         not: it is the same on a slow laptop as on the machine the budget was written for. \
         Waiving the timing therefore cannot waive the work, or a waived run would mean nothing \
         was verified at all — which is precisely the state someone reaching for `{WAIVER}` on a \
         red build would end up in without noticing"
    );
    Ok(())
}
