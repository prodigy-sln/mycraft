//! A red gate reports how much of itself is red, and says what it runs.
//!
//! Two defects with one shape: the gate's output says less than a reader takes
//! it to say. Every test invocation stops at its first failure, so a red run's
//! count bounds nothing; and `-Quick` announces itself as running no tests in
//! three separate places while running two suites and a documentation build.
//!
//! # Which instrument answers which question
//!
//! The extent of a red run is a **behaviour**, so it is measured by running a
//! suite — a five-test fixture with three failures, under the extent flags read
//! out of the shipped script rather than chosen here. What the script carries
//! and what it says are **text**, so they are read, and every reading is a total
//! enumeration with an arm meaning *the reading lost its subject*, paired with a
//! control fixture wrong in each way it is supposed to tell apart. The controls
//! are green from the day they are written; the tests either side of them are
//! what grade the gate.

mod gate;

use std::error::Error;
use std::fs;

use gate::extent::{
    ChainResidual, GateText, GpuFreeStaging, ModeDescription, RunExtent, Selection, TestInvocation,
    chain_residual_of, describes_quick, quick_descriptions, stage_keys_the_document_lists,
    testing_document,
};
use gate::suite::{SuiteExtent, five_tests_three_failing};

type TestResult = Result<(), Box<dyn Error>>;

/// One script carrying all three extents, so a reading that could not tell them
/// apart cannot grade any of them. The last invocation's flag sits on a
/// continuation line, which is where the shipped script's flags live too.
const THREE_EXTENTS: &str = r"
Invoke-Stage 'a' { cargo nextest run -p mc-testkit --no-default-features --no-fail-fast }
Invoke-Stage 'b' { cargo nextest run --workspace --no-tests=pass }
cargo llvm-cov nextest `
    --workspace `
    --ignore-run-fail
";

/// The two GPU-free crates reported apart, which is what stops a failure in the
/// first from hiding the second.
const EACH_CRATE_ITS_OWN_STAGE: &str = r"
Invoke-Stage 'gpu-free (mc-testkit, no default features)' { cargo clippy }
Invoke-Stage 'gpu-free (mc-render, no default features)' { cargo clippy }
";

/// One stage name standing for both crates, which is the shape being fixed.
const ONE_STAGE_FOR_BOTH_CRATES: &str = r"
Invoke-Stage 'gpu-free (mc-testkit + mc-render, no default features)' { cargo clippy }
";

/// A script with no GPU-free stage at all, so the reading has lost its subject.
const NO_GPU_FREE_STAGE: &str = r"
Invoke-Stage 'format (cargo fmt --check)' { cargo fmt }
";

#[test]
fn a_red_run_reports_every_test_it_was_given_and_still_fails() -> TestResult {
    let flags = GateText::of_the_repository()?.extent_flags();

    assert_eq!(
        five_tests_three_failing(&flags)?,
        SuiteExtent::TheWholeSuiteRanAndTheRunFailed {
            ran: 5,
            passed: 2,
            failed: 3,
        },
        "the flags come out of the shipped gate, so this is what a red stage of it reports. The \
         count and the verdict are one observation on purpose: `--ignore-run-fail` produces a \
         byte-identical count and exit 0, and exit 0 is the only thing Invoke-Stage reads"
    );
    Ok(())
}

#[test]
fn every_test_invocation_the_gate_runs_finishes_the_suite_it_was_given() -> TestResult {
    let read = GateText::of_the_repository()?;

    assert_eq!(
        read.test_invocations(),
        vec![
            invocation(
                "cargo nextest run",
                Selection::Package("mc-testkit".to_owned())
            ),
            invocation(
                "cargo nextest run",
                Selection::Package("mc-render".to_owned())
            ),
            invocation("cargo nextest run", Selection::Workspace),
            invocation("cargo llvm-cov nextest", Selection::Workspace),
        ],
        "the whole enumeration is compared in order, so a missing invocation, an added one and a \
         reordering are three distinct failures — a hand-maintained list filtered against the \
         script would see none of the three"
    );
    Ok(())
}

/// The control for the enumeration above, in every direction at once.
#[test]
fn the_same_reading_tells_a_cancelling_invocation_from_a_finishing_one() -> TestResult {
    let read = GateText::of(THREE_EXTENTS)?;

    assert_eq!(
        read.test_invocations()
            .into_iter()
            .map(|found| found.extent)
            .collect::<Vec<_>>(),
        vec![
            RunExtent::RunsEveryTestItWasGiven,
            RunExtent::CancelsAtTheFirstFailure,
            RunExtent::HidesTheFailureFromTheGate,
        ],
        "an invocation without the flag has to be named rather than passed over, and the \
         forbidden flag has to be told from the correct one rather than counted as satisfying it"
    );
    Ok(())
}

#[test]
fn the_gate_carries_the_forbidden_flag_nowhere() -> TestResult {
    assert_eq!(
        (
            GateText::of_the_repository()?.forbidden_flag_lines(),
            GateText::of(THREE_EXTENTS)?.forbidden_flag_lines(),
        ),
        (Vec::new(), vec![6]),
        "`--ignore-run-fail` runs every test and exits 0, which makes the gate pass with red \
         tests. The second reading is the control: without it, a scan that stopped looking would \
         report the shipped script as clean forever"
    );
    Ok(())
}

#[test]
fn the_two_gpu_free_crates_are_reported_as_separate_stages() -> TestResult {
    assert_eq!(
        GateText::of_the_repository()?.gpu_free_staging(),
        GpuFreeStaging::EachCrateIsItsOwnStage,
        "one stage name standing for four chained commands means a failing mc-testkit suite hides \
         everything mc-render would have said"
    );
    Ok(())
}

/// The control for the staging reading.
#[test]
fn the_same_reading_tells_two_staged_crates_from_one() -> TestResult {
    let read = [
        EACH_CRATE_ITS_OWN_STAGE,
        ONE_STAGE_FOR_BOTH_CRATES,
        NO_GPU_FREE_STAGE,
    ]
    .iter()
    .map(|script| Ok(GateText::of(script)?.gpu_free_staging()))
    .collect::<Result<Vec<_>, Box<dyn Error>>>()?;

    assert_eq!(
        read,
        vec![
            GpuFreeStaging::EachCrateIsItsOwnStage,
            GpuFreeStaging::OneStageNamesBothCrates,
            GpuFreeStaging::NoGpuFreeStageWasFound,
        ],
        "a reading that answered `each crate is its own stage` for all three would certify the \
         one shape it was written to check"
    );
    Ok(())
}

#[test]
fn what_the_chain_still_hides_is_stated_where_the_chain_is() -> TestResult {
    let script = GateText::of_the_repository()?;
    let document = fs::read_to_string(testing_document()?)?;

    assert_eq!(
        (
            script.chain_residual(),
            chain_residual_of(&document),
            chain_residual_of("The gpu-free stage chains its commands with &&."),
            chain_residual_of("The gate runs stages in order."),
        ),
        (
            ChainResidual::ItStatesWhatTheChainStillHides,
            ChainResidual::ItStatesWhatTheChainStillHides,
            ChainResidual::TheChainIsDescribedWithoutItsResidual,
            ChainResidual::NothingDescribesTheChain,
        ),
        "the chain survives the split, so a clippy failure still hides its own crate's test run. \
         A justification that stopped being complete the moment the line beside it changed is \
         worse than none, and the reader who needs the warning meets it at that comment"
    );
    Ok(())
}

#[test]
fn every_description_of_quick_names_what_quick_runs() -> TestResult {
    assert_eq!(
        quick_descriptions()?,
        vec![
            (
                "the mode line the gate prints",
                ModeDescription::NamesEveryStageInOrder
            ),
            (
                "the .PARAMETER Quick docstring",
                ModeDescription::NamesEveryStageInOrder
            ),
            (
                "docs/technical/testing.md",
                ModeDescription::NamesEveryStageInOrder
            ),
        ],
        "all three are enumerated rather than merely agreeing: three descriptions can be made \
         consistent by making all three vague, and a reader deciding whether -Quick is safe in a \
         tight loop is entitled to know it runs two suites and builds documentation"
    );
    Ok(())
}

/// The control for the three readings above.
#[test]
fn the_same_reading_tells_a_complete_description_from_a_vague_one() -> TestResult {
    assert_eq!(
        [
            describes_quick(Some("format, lint, gpu-free tests, docs and size")),
            describes_quick(Some("Format, lint and size only.")),
            describes_quick(Some("stages 1-3 only, for tight edit loops")),
            describes_quick(None),
        ],
        [
            ModeDescription::NamesEveryStageInOrder,
            ModeDescription::NamesOnly(vec![
                "format".to_owned(),
                "lint".to_owned(),
                "size".to_owned()
            ]),
            ModeDescription::NamesOnly(Vec::new()),
            ModeDescription::NoSuchDescriptionWasFound,
        ],
        "the third is the one that matters: writing `stages 1-3 only` in all three places makes \
         them agree perfectly while naming nothing, and a reading satisfied by that would grade \
         the degraded label as a fix"
    );
    Ok(())
}

#[test]
fn every_stage_the_gate_reports_has_a_row_in_the_stage_table() -> TestResult {
    assert_eq!(
        stage_keys_the_document_lists()?,
        GateText::of_the_repository()?.stage_keys_it_reports(),
        "the table is the canonical list a reader consults instead of the script. Compared in \
         order, so a missing stage, an extra row and a reordering are three distinct failures — \
         and a row conflating two stages the gate reports separately is a missing stage"
    );
    Ok(())
}

/// One invocation that runs everything it was given.
fn invocation(runner: &str, selects: Selection) -> TestInvocation {
    TestInvocation {
        runner: runner.to_owned(),
        selects,
        extent: RunExtent::RunsEveryTestItWasGiven,
    }
}
