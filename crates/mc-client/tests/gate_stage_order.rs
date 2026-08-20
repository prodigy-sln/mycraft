//! Where the gate builds the art, relative to everything else it does.
//!
//! Two questions are asked of the script's text because no bounded run can ask
//! them: whether the set is built before the tests on **both** coverage paths, and
//! whether the tests are unreachable once the build refused. `gate/reading.rs`
//! records why, and what is left to a human.
//!
//! # The reading is graded here too
//!
//! A scan that stopped finding what it looks for reports a clean script exactly as
//! loudly as a clean script does, so every verdict below is a total enumeration
//! including an arm meaning *the reading lost its subject* — and the two control
//! tests feed that same reading scripts that are wrong in each of the ways it is
//! supposed to tell apart. The controls are green from the day they are written,
//! because what they grade is the instrument rather than the gate; the tests
//! either side of them are what grade the gate.
//!
//! **One placement decides two scenarios.** A build sitting outside the coverage
//! choice is reached whichever way that choice goes; a build inside one branch is
//! reached one way only — and which of the two readings stays green says which
//! branch it landed in.

mod gate;

use std::error::Error;

use gate::reading::{
    CoveragePath, EveryStageRunsClaim, GateScript, QuickExitPlacement, SetBuildPlacement,
    TestStageAfterARefusedBuild,
};

type TestResult = Result<(), Box<dyn Error>>;

/// A script shaped the way this phase asks for: the set is built once, after the
/// early exit and outside the coverage choice, so both paths reach it.
const BUILDING_BEFORE_EITHER_PATH: &str = r#"
if ($Quick) {
    Write-Host "QUICK CHECKS PASSED (not a full gate)"
    exit 0
}
Invoke-Stage 'art (voxforge build)' { cargo run -p voxforge --quiet -- build $Manifest }
if ($SkipCoverage) {
    Invoke-Stage 'tests (nextest)' { cargo nextest run --workspace --no-tests=pass }
}
else {
    cargo llvm-cov nextest --workspace
}
"#;

/// The set built inside the coverage-skipping branch, which is the mistake one
/// placement is meant to make impossible.
const BUILDING_INSIDE_THE_SKIPPING_BRANCH: &str = r#"
if ($Quick) {
    Write-Host "QUICK CHECKS PASSED (not a full gate)"
    exit 0
}
if ($SkipCoverage) {
    Invoke-Stage 'art (voxforge build)' { cargo run -p voxforge --quiet -- build $Manifest }
    Invoke-Stage 'tests (nextest)' { cargo nextest run --workspace --no-tests=pass }
}
else {
    cargo llvm-cov nextest --workspace
}
"#;

/// The set built after the suite has already graded whatever was on disk.
const BUILDING_AFTER_THE_TESTS: &str = r#"
if ($SkipCoverage) {
    Invoke-Stage 'tests (nextest)' { cargo nextest run --workspace --no-tests=pass }
}
else {
    cargo llvm-cov nextest --workspace
}
Invoke-Stage 'art (voxforge build)' { cargo run -p voxforge --quiet -- build $Manifest }
"#;

/// A script that never builds the set at all.
const BUILDING_NOWHERE: &str = r#"
if ($SkipCoverage) {
    Invoke-Stage 'tests (nextest)' { cargo nextest run --workspace --no-tests=pass }
}
else {
    cargo llvm-cov nextest --workspace
}
"#;

/// A script whose coverage-skipping path runs no suite, so the reading has lost
/// the subject it is comparing against.
const RUNNING_NO_TESTS_ON_ONE_PATH: &str = r#"
Invoke-Stage 'art (voxforge build)' { cargo run -p voxforge --quiet -- build $Manifest }
if ($SkipCoverage) {
    Write-Warn 'nothing to run'
}
else {
    cargo llvm-cov nextest --workspace
}
"#;

/// The guard this phase asks for: the skip is recorded in one branch and every
/// suite runs in another, chosen after the build.
const SKIPPING_THE_TESTS_AND_RECORDING_IT: &str = r#"
Invoke-Stage 'art (voxforge build)' { cargo run -p voxforge --quiet -- build $Manifest }
if ($artBuildFailed) {
    $Failures.Add('tests (not run: art build failed)')
}
elseif ($SkipCoverage) {
    Invoke-Stage 'tests (nextest)' { cargo nextest run --workspace --no-tests=pass }
}
else {
    cargo llvm-cov nextest --workspace
}
"#;

/// The skip recorded beside the suite rather than instead of it, which is a
/// summary that names the tests as skipped while they run.
const RECORDING_A_SKIP_THAT_DOES_NOT_HAPPEN: &str = r#"
Invoke-Stage 'art (voxforge build)' { cargo run -p voxforge --quiet -- build $Manifest }
if ($artBuildFailed) { $Failures.Add('tests (not run: art build failed)') }
if ($SkipCoverage) {
    Invoke-Stage 'tests (nextest)' { cargo nextest run --workspace --no-tests=pass }
}
else {
    cargo llvm-cov nextest --workspace
}
"#;

/// The guard settled before the build it is supposed to be about.
const CHOOSING_BEFORE_THE_BUILD: &str = r#"
if ($artBuildFailed) {
    $Failures.Add('tests (not run: art build failed)')
}
elseif ($SkipCoverage) {
    Invoke-Stage 'tests (nextest)' { cargo nextest run --workspace --no-tests=pass }
}
else {
    cargo llvm-cov nextest --workspace
}
Invoke-Stage 'art (voxforge build)' { cargo run -p voxforge --quiet -- build $Manifest }
"#;

/// A script with the record and no suite anywhere.
const RECORDING_A_SKIP_AND_RUNNING_NOTHING: &str = r#"
Invoke-Stage 'art (voxforge build)' { cargo run -p voxforge --quiet -- build $Manifest }
if ($artBuildFailed) {
    $Failures.Add('tests (not run: art build failed)')
}
"#;

/// The set built on the way past the `-Quick` early exit, so an edit loop bakes.
const BUILDING_BEFORE_THE_QUICK_EXIT: &str = r#"
Invoke-Stage 'art (voxforge build)' { cargo run -p voxforge --quiet -- build $Manifest }
if ($Quick) {
    Write-Host "QUICK CHECKS PASSED (not a full gate)"
    exit 0
}
if ($SkipCoverage) {
    Invoke-Stage 'tests (nextest)' { cargo nextest run --workspace --no-tests=pass }
}
else {
    cargo llvm-cov nextest --workspace
}
"#;

/// Both readings of one script, instrumented path first.
fn both_paths(script: &str) -> Result<(SetBuildPlacement, SetBuildPlacement), Box<dyn Error>> {
    let read = GateScript::of(script)?;
    Ok((
        read.placement_on(CoveragePath::Instrumented),
        read.placement_on(CoveragePath::Skipping),
    ))
}

#[test]
fn the_instrumented_path_builds_the_set_before_the_stage_that_runs_the_tests() -> TestResult {
    let read = GateScript::of_the_repository()?;

    assert_eq!(
        read.placement_on(CoveragePath::Instrumented),
        SetBuildPlacement::TheSetIsBuiltBeforeTheTests,
        "the suite run under instrumentation grades whatever set is on disk, so the gate has to \
         build it first"
    );
    Ok(())
}

#[test]
fn the_coverage_skipping_path_builds_the_set_before_the_stage_that_runs_the_tests() -> TestResult {
    let read = GateScript::of_the_repository()?;

    assert_eq!(
        read.placement_on(CoveragePath::Skipping),
        SetBuildPlacement::TheSetIsBuiltBeforeTheTests,
        "`-SkipCoverage` runs the same suite against the same set, so it is owed the same build — \
         and a placement that satisfies only the other path is one branch deep"
    );
    Ok(())
}

/// The control for the two readings above, in every direction at once.
#[test]
fn the_same_reading_tells_a_misplaced_set_build_from_a_well_placed_one() -> TestResult {
    use SetBuildPlacement::{
        NoSetIsBuiltOnThisPath as NoSet, NoTestCommandIsRunOnThisPath as NoTests,
        TheSetIsBuiltAfterTheTests as After, TheSetIsBuiltBeforeTheTests as Before,
    };
    let read = [
        BUILDING_BEFORE_EITHER_PATH,
        BUILDING_INSIDE_THE_SKIPPING_BRANCH,
        BUILDING_AFTER_THE_TESTS,
        BUILDING_NOWHERE,
        RUNNING_NO_TESTS_ON_ONE_PATH,
    ]
    .iter()
    .map(|script| both_paths(script))
    .collect::<Result<Vec<_>, Box<dyn Error>>>()?;

    assert_eq!(
        read,
        vec![
            (Before, Before),
            (NoSet, Before),
            (After, After),
            (NoSet, NoSet),
            (NoTests, NoTests),
        ],
        "a reading that answered `built before the tests` for all five, or that stopped finding \
         the commands and answered nothing for all five, would grade the gate the same way in \
         both cases"
    );
    Ok(())
}

/// The control for the reading `gate_art_stages.rs` grades the skip with.
#[test]
fn the_same_reading_tells_a_guarded_test_stage_from_an_unguarded_one() -> TestResult {
    let verdicts = [
        SKIPPING_THE_TESTS_AND_RECORDING_IT,
        BUILDING_BEFORE_EITHER_PATH,
        RECORDING_A_SKIP_THAT_DOES_NOT_HAPPEN,
        CHOOSING_BEFORE_THE_BUILD,
        RECORDING_A_SKIP_AND_RUNNING_NOTHING,
    ]
    .iter()
    .map(|script| Ok(GateScript::of(script)?.test_stage_after_a_refused_build()))
    .collect::<Result<Vec<_>, Box<dyn Error>>>()?;

    assert_eq!(
        verdicts,
        vec![
            TestStageAfterARefusedBuild::TheTestsRunOnlyBesideTheRecordedSkip,
            TestStageAfterARefusedBuild::NothingRecordsThatTheTestsWereSkipped,
            TestStageAfterARefusedBuild::SomeTestRunsWhateverTheArtBuildDid,
            TestStageAfterARefusedBuild::TheSkipIsNotChosenAfterTheSetIsBuilt,
            TestStageAfterARefusedBuild::NoTestCommandWasFound,
        ],
        "each of the five is a way the guard can be wrong or absent, and a reading that could not \
         tell them apart would certify the one shape it was written to check"
    );
    Ok(())
}

#[test]
fn the_set_is_built_after_the_quick_early_exit() -> TestResult {
    let read = GateScript::of_the_repository()?;

    assert_eq!(
        (
            read.quick_exit_placement(),
            GateScript::of(BUILDING_BEFORE_THE_QUICK_EXIT)?.quick_exit_placement()
        ),
        (
            QuickExitPlacement::TheSetIsBuiltAfterIt,
            QuickExitPlacement::TheSetIsBuiltBeforeIt
        ),
        "`-Quick` is the tight edit loop, and a bake on its way past would be paid for on every \
         iteration. The second reading is the control: without it, a scan that stopped finding \
         the build would report the shipped script as well placed forever"
    );
    Ok(())
}

#[test]
fn the_header_states_an_exception_to_every_stage_running() -> TestResult {
    let read = GateScript::of_the_repository()?;

    assert_eq!(
        read.every_stage_runs_claim(),
        EveryStageRunsClaim::AnExceptionIsStatedBesideIt,
        "the header claims every stage runs even after an earlier one failed, and this phase makes \
         that false in one place. A property stated flatly beside code that contradicts it is \
         worse than one never stated"
    );
    Ok(())
}
