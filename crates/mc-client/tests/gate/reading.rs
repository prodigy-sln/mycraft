//! Reading the gate script's own text, for the questions a run of it cannot
//! answer.
//!
//! # Why anything here is read rather than run
//!
//! Three of this phase's scenarios are about *order and reachability across the
//! whole gate*: the set is built before the stage that runs the tests, on both
//! coverage paths, and the test stage does not run at all after a refused build.
//! Running the whole gate to observe any of them costs a clippy pass over the
//! workspace, a documentation build, two supply-chain scans and the entire suite,
//! which is minutes of a test's time and would put a second full run of the suite
//! inside the suite. Running the art stages alone answers none of them, because
//! the stages they are about are the ones it does not select.
//!
//! So the ordering is read off the script. That is a weaker instrument than a run
//! and is named as one: it can tell that a command sits before another and inside
//! or outside a branch, and it cannot tell that either command does what its stage
//! claims. `architecture.md`'s D15 ties the two together — the stage selector
//! restates no stage, so there is one implementation of each, and the run grades
//! what a stage does while this grades where it sits.
//!
//! **What is left to a human is one composition and it is named**: that a real
//! gate run over a broken manifest reports the test stage as *not run* rather than
//! as run and passing. `tasks.md`'s T38 asks for that check by hand, once.
//!
//! # How the text is read
//!
//! Comment lines are blanked before anything else, so a banner or a sentence of
//! prose quoting a command is not mistaken for the command. Whole-line comments
//! and the `<# … #>` header block are recognised; a comment trailing a command on
//! the same line is not, which is the one shape this reading would misread.
//!
//! Braces are then counted over what is left, which gives every block its opening
//! line, its closing line and its nesting. Two blocks are branches of one
//! statement when they are consecutive at the same nesting and the text joining
//! them says `else`. That is what lets a verdict distinguish *the tests run
//! instead of the skip being recorded* from *the tests run beside it*.
//!
//! Order is line order, which is execution order here: the gate is a straight run
//! of top-level statements with no loop and no function containing a stage.

use std::error::Error;
use std::fs;

use super::gate_script;

/// The command the set-building stage runs.
const ART_BUILD_COMMAND: &str = "cargo run -p voxforge";

/// The command the instrumented test stage runs.
const INSTRUMENTED_TEST_COMMAND: &str = "cargo llvm-cov nextest";

/// The command the coverage-skipping test stage runs. Narrower than
/// `cargo nextest run`, which the GPU-free stage also runs against two packages.
const SKIPPING_TEST_COMMAND: &str = "cargo nextest run --workspace";

/// The statement that chooses between the two coverage paths. Written to match
/// `elseif` as well, since the guard this phase adds may take the `if`.
const COVERAGE_CHOICE: &str = "if ($SkipCoverage)";

/// What the script records in place of the test stage after a refused build.
const SKIP_RECORD: &str = "tests (not run: art build failed)";

/// The line the `-Quick` early exit prints before leaving.
const QUICK_EXIT: &str = "QUICK CHECKS PASSED";

/// The property the script's header claims for itself.
const EVERY_STAGE_RUNS: &str = "Every stage runs even if an earlier one fails";

/// Which of the two paths through the test stage is being asked about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoveragePath {
    /// The path that runs the suite under coverage instrumentation.
    Instrumented,
    /// The path `-SkipCoverage` takes.
    Skipping,
}

/// Where the set is built relative to the tests, on one path.
#[derive(Debug, PartialEq, Eq)]
pub enum SetBuildPlacement {
    /// The set is built, and it is built before the tests run.
    TheSetIsBuiltBeforeTheTests,
    /// The set is built on this path, but only after the tests have run.
    TheSetIsBuiltAfterTheTests,
    /// Nothing on this path builds the set.
    NoSetIsBuiltOnThisPath,
    /// This path runs no test command — the reading has lost its subject.
    NoTestCommandIsRunOnThisPath,
}

/// What becomes of the test stage when the set build refuses.
#[derive(Debug, PartialEq, Eq)]
pub enum TestStageAfterARefusedBuild {
    /// Every test command runs in a branch beside the one recording the skip,
    /// and the choice between them is made after the set is built.
    TheTestsRunOnlyBesideTheRecordedSkip,
    /// Nothing records that the tests were skipped, so a reader of the summary
    /// cannot tell a stage that did not run from a stage that is not listed.
    NothingRecordsThatTheTestsWereSkipped,
    /// The skip is recorded, but a test command runs whatever the build did.
    SomeTestRunsWhateverTheArtBuildDid,
    /// The tests are guarded, but by something settled before the set is built.
    TheSkipIsNotChosenAfterTheSetIsBuilt,
    /// No test command was found at all — the reading has lost its subject.
    NoTestCommandWasFound,
}

/// Where the art stages sit relative to the `-Quick` early exit.
#[derive(Debug, PartialEq, Eq)]
pub enum QuickExitPlacement {
    /// `-Quick` returns before the set is built, so an edit loop pays nothing.
    TheSetIsBuiltAfterIt,
    /// `-Quick` builds the set on its way past.
    TheSetIsBuiltBeforeIt,
    /// Nothing builds the set.
    NoSetIsBuiltAtAll,
    /// There is no early exit — the reading has lost its subject.
    NoQuickExitWasFound,
}

/// What the header says about every stage running.
#[derive(Debug, PartialEq, Eq)]
pub enum EveryStageRunsClaim {
    /// The property is claimed, and an exception to it is stated beside it.
    AnExceptionIsStatedBesideIt,
    /// The property is claimed flatly, and this phase makes it false.
    TheClaimStandsUnqualified,
    /// The property is not claimed at all.
    NothingClaimsIt,
}

/// One `{ … }` of the script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The line the brace opened on.
    pub open: usize,
    /// The line it closed on.
    pub close: usize,
    /// How many blocks enclose it.
    pub depth: usize,
}

impl Block {
    /// Whether `line` falls inside this block.
    #[must_use]
    pub fn holds(&self, line: usize) -> bool {
        self.open <= line && line <= self.close
    }
}

/// One branch of the coverage choice, and the test command it runs.
#[derive(Debug)]
struct Branch {
    block: Block,
    tests_at: usize,
}

/// The gate script, read as lines and as blocks.
#[derive(Debug)]
pub struct GateScript {
    header: String,
    code: Vec<String>,
    blocks: Vec<Block>,
}

impl GateScript {
    /// The script this repository ships.
    ///
    /// # Errors
    ///
    /// Returns an error if the script cannot be read, or if its braces do not
    /// balance — a reading that cannot see the blocks would answer every question
    /// below with something that looks like a finding.
    pub fn of_the_repository() -> Result<Self, Box<dyn Error>> {
        Self::of(&fs::read_to_string(gate_script()?)?)
    }

    /// A script written out by a test, for grading this reading itself.
    ///
    /// # Errors
    ///
    /// Returns an error if the braces do not balance.
    pub fn of(text: &str) -> Result<Self, Box<dyn Error>> {
        let code = code_only(text);
        let (blocks, unclosed) = blocks_of(&code);
        if unclosed != 0 {
            return Err(format!("the script leaves {unclosed} brace(s) unclosed").into());
        }
        Ok(Self {
            header: header_of(text),
            code,
            blocks,
        })
    }

    /// Where the set is built relative to the tests, on `path`.
    #[must_use]
    pub fn placement_on(&self, path: CoveragePath) -> SetBuildPlacement {
        let (Some(mine), Some(theirs)) = (self.branch_of(path), self.branch_of(path.other()))
        else {
            return SetBuildPlacement::NoTestCommandIsRunOnThisPath;
        };
        let Some(built_at) = self.sole_line_of(ART_BUILD_COMMAND) else {
            return SetBuildPlacement::NoSetIsBuiltOnThisPath;
        };
        if theirs.block.holds(built_at) {
            return SetBuildPlacement::NoSetIsBuiltOnThisPath;
        }
        if built_at < mine.tests_at {
            SetBuildPlacement::TheSetIsBuiltBeforeTheTests
        } else {
            SetBuildPlacement::TheSetIsBuiltAfterTheTests
        }
    }

    /// What becomes of the test stage when the set build refuses.
    #[must_use]
    pub fn test_stage_after_a_refused_build(&self) -> TestStageAfterARefusedBuild {
        let running_at = self.test_command_lines();
        if running_at.is_empty() {
            return TestStageAfterARefusedBuild::NoTestCommandWasFound;
        }
        let Some(recorded_in) = self
            .sole_line_of(SKIP_RECORD)
            .and_then(|line| self.innermost_block_holding(line))
        else {
            return TestStageAfterARefusedBuild::NothingRecordsThatTheTestsWereSkipped;
        };
        let chain = self.chain_of(&recorded_in);
        if !runs_only_beside(&chain, &recorded_in, &running_at) {
            return TestStageAfterARefusedBuild::SomeTestRunsWhateverTheArtBuildDid;
        }
        self.chosen_after_the_build(&chain)
    }

    /// Where the art stages sit relative to the `-Quick` early exit.
    #[must_use]
    pub fn quick_exit_placement(&self) -> QuickExitPlacement {
        let Some(exits_at) = self.sole_line_of(QUICK_EXIT) else {
            return QuickExitPlacement::NoQuickExitWasFound;
        };
        match self.sole_line_of(ART_BUILD_COMMAND) {
            None => QuickExitPlacement::NoSetIsBuiltAtAll,
            Some(built_at) if built_at > exits_at => QuickExitPlacement::TheSetIsBuiltAfterIt,
            Some(_) => QuickExitPlacement::TheSetIsBuiltBeforeIt,
        }
    }

    /// What the header claims about every stage running.
    ///
    /// The exception is recognised by the header saying `except` at all, which
    /// cannot tell a well-stated exception from a badly stated one. It can tell
    /// one that was never written, which is the failure this guards.
    #[must_use]
    pub fn every_stage_runs_claim(&self) -> EveryStageRunsClaim {
        if !self.header.contains(EVERY_STAGE_RUNS) {
            return EveryStageRunsClaim::NothingClaimsIt;
        }
        if self.header.to_lowercase().contains("except") {
            EveryStageRunsClaim::AnExceptionIsStatedBesideIt
        } else {
            EveryStageRunsClaim::TheClaimStandsUnqualified
        }
    }

    fn chosen_after_the_build(&self, chain: &[Block]) -> TestStageAfterARefusedBuild {
        match (self.sole_line_of(ART_BUILD_COMMAND), chain.first()) {
            (Some(built_at), Some(first)) if first.open > built_at => {
                TestStageAfterARefusedBuild::TheTestsRunOnlyBesideTheRecordedSkip
            }
            _ => TestStageAfterARefusedBuild::TheSkipIsNotChosenAfterTheSetIsBuilt,
        }
    }

    fn test_command_lines(&self) -> Vec<usize> {
        [SKIPPING_TEST_COMMAND, INSTRUMENTED_TEST_COMMAND]
            .iter()
            .filter_map(|command| self.sole_line_of(command))
            .collect()
    }

    /// The branch of the coverage choice that runs `path`'s tests.
    fn branch_of(&self, path: CoveragePath) -> Option<Branch> {
        let chooses_at = self.sole_line_of(COVERAGE_CHOICE)?;
        let opening = self
            .blocks
            .iter()
            .filter(|block| block.open == chooses_at)
            .min_by_key(|block| block.depth)?;
        let chain = self.chain_of(opening);
        let tests_at = self.sole_line_of(path.command())?;
        let block = chain.iter().find(|block| block.holds(tests_at))?.clone();
        Some(Branch { block, tests_at })
    }

    /// The one line carrying `needle`, or nothing when no line does or more than
    /// one does. Two lines is as much a broken reading as none.
    fn sole_line_of(&self, needle: &str) -> Option<usize> {
        let mut carrying = self
            .code
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(needle))
            .map(|(index, _)| index);
        let first = carrying.next()?;
        carrying.next().is_none().then_some(first)
    }

    fn innermost_block_holding(&self, line: usize) -> Option<Block> {
        self.blocks
            .iter()
            .filter(|block| block.holds(line))
            .max_by_key(|block| block.depth)
            .cloned()
    }

    /// Every branch of the statement `block` is a branch of, in source order.
    fn chain_of(&self, block: &Block) -> Vec<Block> {
        let peers = self.peers_of(block);
        let Some(position) = peers.iter().position(|peer| peer == block) else {
            return vec![block.clone()];
        };
        let mut first = position;
        while first > 0 && self.are_joined(peers.get(first - 1), peers.get(first)) {
            first -= 1;
        }
        let mut last = position;
        while self.are_joined(peers.get(last), peers.get(last + 1)) {
            last += 1;
        }
        peers.get(first..=last).unwrap_or_default().to_vec()
    }

    fn peers_of(&self, block: &Block) -> Vec<Block> {
        let mut peers: Vec<Block> = self
            .blocks
            .iter()
            .filter(|peer| peer.depth == block.depth)
            .cloned()
            .collect();
        peers.sort_by_key(|peer| (peer.open, peer.close));
        peers
    }

    /// Whether the text closing `earlier` and opening `later` says `else`.
    fn are_joined(&self, earlier: Option<&Block>, later: Option<&Block>) -> bool {
        let (Some(earlier), Some(later)) = (earlier, later) else {
            return false;
        };
        self.code
            .get(earlier.close..=later.open)
            .is_some_and(|joining| joining.join("\n").contains("else"))
    }
}

impl CoveragePath {
    fn other(self) -> Self {
        match self {
            Self::Instrumented => Self::Skipping,
            Self::Skipping => Self::Instrumented,
        }
    }

    fn command(self) -> &'static str {
        match self {
            Self::Instrumented => INSTRUMENTED_TEST_COMMAND,
            Self::Skipping => SKIPPING_TEST_COMMAND,
        }
    }
}

/// Whether every test command runs in a branch of `chain` other than `recorded_in`.
fn runs_only_beside(chain: &[Block], recorded_in: &Block, running_at: &[usize]) -> bool {
    running_at.iter().all(|line| {
        chain
            .iter()
            .any(|branch| branch != recorded_in && branch.holds(*line))
    })
}

/// The script's lines with every whole-line comment blanked, so that prose
/// quoting a command is not read as the command.
#[must_use]
pub fn code_only(text: &str) -> Vec<String> {
    let mut inside_block = false;
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("<#") {
                inside_block = true;
            }
            let blanked = inside_block || trimmed.starts_with('#');
            if trimmed.contains("#>") {
                inside_block = false;
            }
            if blanked {
                String::new()
            } else {
                line.to_owned()
            }
        })
        .collect()
}

/// The script's `<# … #>` header block, which is what a reader of the file meets
/// first and what states its properties.
fn header_of(text: &str) -> String {
    text.lines()
        .skip_while(|line| !line.trim_start().starts_with("<#"))
        .take_while(|line| !line.contains("#>"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every block of the code, and how many braces were left unclosed.
#[must_use]
pub fn blocks_of(code: &[String]) -> (Vec<Block>, usize) {
    let mut open_at: Vec<usize> = Vec::new();
    let mut blocks = Vec::new();
    for (index, line) in code.iter().enumerate() {
        for character in line.chars() {
            note_brace(character, index, &mut open_at, &mut blocks);
        }
    }
    (blocks, open_at.len())
}

fn note_brace(character: char, line: usize, open_at: &mut Vec<usize>, blocks: &mut Vec<Block>) {
    if character == '{' {
        open_at.push(line);
    } else if character == '}'
        && let Some(open) = open_at.pop()
    {
        blocks.push(Block {
            open,
            close: line,
            depth: open_at.len(),
        });
    }
}
