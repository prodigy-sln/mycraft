//! What the gate says about how much of a suite it ran, and what it says about
//! the mode a reader runs in a tight loop.
//!
//! # Why the invocations are enumerated rather than looked up
//!
//! [`super::reading`] already reads this script, and its `SKIPPING_TEST_COMMAND`
//! is a needle deliberately narrow enough to miss two of the four test
//! invocations. That is the right shape for the question it asks and the wrong
//! shape for this one: a hand-maintained list compared by filtering cannot see
//! an invocation nobody added to the list, and a needle that starts matching
//! twice drops silently to zero hits. So every invocation is read *out of* the
//! script and the whole enumeration is compared in order, which makes a missing
//! one, an added one and a reordering three distinct failures.
//!
//! # An invocation is not a line
//!
//! Two of the four are commands of one `&&` chain, and the instrumented one's
//! flags sit on the four lines below the one naming it, behind backtick
//! continuations. "The line carrying the command also carries the flag" is
//! therefore false for a correct script, and an assertion in that shape would go
//! red against a correct fix while the cheapest way to green it is to let the
//! test dictate the script's formatting. Lines are joined at their continuations
//! first, split at `&&` after, and a flag is read against the whole continued
//! command.
//!
//! # A stage is what a reader sees after `ok:`
//!
//! `Invoke-Stage` reports the name it was given, and a stage opened with
//! `Write-StageHeader` reports whatever `Write-Ok` calls follow it. That is why
//! the coverage stage counts as the **two** stages it prints rather than as the
//! one header above them: a run says `ok: tests` and then `ok: coverage 93.85%`,
//! and a document listing those as one row under-reports the gate.

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use super::reading::{Block, blocks_of, code_only};
use super::{gate_script, repository_root};

/// The runner the uninstrumented stages call.
const NEXTEST: &str = "cargo nextest run";

/// The runner the coverage stage calls.
const INSTRUMENTED_NEXTEST: &str = "cargo llvm-cov nextest";

/// The flag that makes a runner finish the suite it was given.
const RUNS_EVERYTHING: &str = "--no-fail-fast";

/// The flag that finishes the suite and then reports success anyway. Forbidden
/// by name: its summary line is byte-identical to the correct flag's, and the
/// exit code they differ in is the only thing `Invoke-Stage` reads.
const HIDES_THE_FAILURE: &str = "--ignore-run-fail";

/// The statement that chooses between the two coverage paths. Matches the
/// `elseif` the script actually writes.
const COVERAGE_CHOICE: &str = "if ($SkipCoverage)";

/// What `-Quick` runs, in the order it runs it, and the words a description has
/// to carry for each of them to count as named.
///
/// **Enumerated, never paraphrased.** Asking three descriptions merely to *agree*
/// is satisfied by making all three vague — "stages 1-3 only" in every one of
/// them agrees perfectly while telling a reader less than the script does today.
/// The vocabulary is closed, so this cannot see a stage nobody listed here; what
/// it can see is a description that stopped naming one.
const WHAT_QUICK_RUNS: [(&str, &[&str]); 5] = [
    ("format", &["format"]),
    ("lint", &["lint"]),
    ("gpu-free tests", &["gpu-free", "test"]),
    ("docs", &["doc"]),
    ("size", &["size"]),
];

/// How much of a suite one invocation runs, and what it does with the verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunExtent {
    /// Every test runs, and a failure among them still fails the run.
    RunsEveryTestItWasGiven,
    /// The run stops at the first failure, so its count bounds nothing.
    CancelsAtTheFirstFailure,
    /// Every test runs and the run exits 0 regardless — the catastrophic case.
    HidesTheFailureFromTheGate,
}

/// What an invocation was pointed at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    /// The whole workspace.
    Workspace,
    /// One named package.
    Package(String),
    /// Neither — the reading found no selector.
    Unstated,
}

/// One test invocation of the gate script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestInvocation {
    /// The runner it calls.
    pub runner: String,
    /// What it was pointed at.
    pub selects: Selection,
    /// How much of the suite it runs.
    pub extent: RunExtent,
}

/// How the GPU-free checks are reported.
#[derive(Debug, PartialEq, Eq)]
pub enum GpuFreeStaging {
    /// Each crate is its own stage, `mc-testkit` first — so a failure in one
    /// leaves the other still run and still reported.
    EachCrateIsItsOwnStage,
    /// One stage name stands for both crates, so a failure in the first hides
    /// everything the second would have said.
    OneStageNamesBothCrates,
    /// There are GPU-free stages, but not one per crate in order.
    TheStagesDoNotNameTheTwoCratesApart,
    /// No GPU-free stage was found — the reading has lost its subject.
    NoGpuFreeStageWasFound,
}

/// What a text says about the `&&` chain that survives inside each stage.
#[derive(Debug, PartialEq, Eq)]
pub enum ChainResidual {
    /// The chain is described, and so is what it still cancels and still hides.
    ItStatesWhatTheChainStillHides,
    /// The chain is described, and the description stops before its residual.
    TheChainIsDescribedWithoutItsResidual,
    /// Nothing here describes the chain at all.
    NothingDescribesTheChain,
}

/// How completely one description of a mode names what that mode runs.
#[derive(Debug, PartialEq, Eq)]
pub enum ModeDescription {
    /// It names every stage the mode runs, in the order the mode runs them.
    NamesEveryStageInOrder,
    /// It names these and only these, in this order.
    NamesOnly(Vec<String>),
    /// There is no such description to read — the reading has lost its subject.
    NoSuchDescriptionWasFound,
}

/// The gate script, read as lines and as blocks.
#[derive(Debug)]
pub struct GateText {
    raw: String,
    code: Vec<String>,
    blocks: Vec<Block>,
}

impl GateText {
    /// The script this repository ships.
    ///
    /// # Errors
    ///
    /// Returns an error if the script cannot be read or its braces do not
    /// balance — a reading that cannot see the blocks answers every question
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
            raw: text.to_owned(),
            code,
            blocks,
        })
    }

    /// Every test invocation the script carries, in source order.
    #[must_use]
    pub fn test_invocations(&self) -> Vec<TestInvocation> {
        continued_commands(&self.code)
            .iter()
            .flat_map(|command| command.split("&&"))
            .filter_map(invocation_of)
            .collect()
    }

    /// The extent flags the script's invocations carry between them, in the
    /// order first met. Empty when every invocation cancels.
    #[must_use]
    pub fn extent_flags(&self) -> Vec<String> {
        let mut flags: Vec<String> = Vec::new();
        for found in self.test_invocations() {
            note_flags(found.extent, &mut flags);
        }
        flags
    }

    /// How the GPU-free checks are reported.
    #[must_use]
    pub fn gpu_free_staging(&self) -> GpuFreeStaging {
        let named: Vec<String> = self
            .stage_names_it_reports()
            .into_iter()
            .filter(|name| stage_key(name) == "gpu-free")
            .collect();
        match (named.len(), named.first(), named.get(1)) {
            (0, _, _) => GpuFreeStaging::NoGpuFreeStageWasFound,
            (1, Some(only), _) if names_both_crates(only) => {
                GpuFreeStaging::OneStageNamesBothCrates
            }
            (2, Some(first), Some(second))
                if names_only(first, "mc-testkit") && names_only(second, "mc-render") =>
            {
                GpuFreeStaging::EachCrateIsItsOwnStage
            }
            _ => GpuFreeStaging::TheStagesDoNotNameTheTwoCratesApart,
        }
    }

    /// The lines carrying the forbidden flag, numbered from one. Empty is the
    /// only correct answer, and the enumeration is what makes a control able to
    /// show the scan can still see one.
    #[must_use]
    pub fn forbidden_flag_lines(&self) -> Vec<usize> {
        self.raw
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(HIDES_THE_FAILURE))
            .map(|(index, _)| index + 1)
            .collect()
    }

    /// What the script's own comments say about the chain's residual.
    #[must_use]
    pub fn chain_residual(&self) -> ChainResidual {
        chain_residual_of(&self.raw)
    }

    /// The stage names the gate prints, in the order it prints them, for a run
    /// that selects everything.
    #[must_use]
    pub fn stage_names_it_reports(&self) -> Vec<String> {
        let skipping = self.coverage_skipping_branch();
        let names = self.literal_assignments();
        let reached: Vec<&String> = self
            .code
            .iter()
            .enumerate()
            .filter(|(line, text)| {
                !skipping.as_ref().is_some_and(|block| block.holds(*line))
                    && !text.trim_start().starts_with("function")
            })
            .map(|(_, text)| text)
            .collect();
        let mut stages: Vec<Vec<String>> = Vec::new();
        for text in reached {
            note_stage(text, &names, &mut stages);
        }
        stages
            .iter()
            .flat_map(|reported| dedupe_by_key(reported))
            .collect()
    }

    /// The same stages, keyed by the word a reader would index them under.
    #[must_use]
    pub fn stage_keys_it_reports(&self) -> Vec<String> {
        self.stage_names_it_reports()
            .iter()
            .map(|name| stage_key(name))
            .collect()
    }

    /// The banner line the gate prints for `-Quick`.
    #[must_use]
    pub fn mode_line(&self) -> Option<&str> {
        self.code
            .iter()
            .find(|line| line.contains("mode: QUICK"))
            .map(String::as_str)
    }

    /// The body of the `.PARAMETER Quick` docstring.
    #[must_use]
    pub fn quick_parameter(&self) -> Option<String> {
        let mut lines = self
            .raw
            .lines()
            .skip_while(|line| !line.trim_start().starts_with(".PARAMETER Quick"));
        lines.next()?;
        Some(
            lines
                .take_while(|line| !line.trim_start().starts_with(".PARAMETER"))
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    fn coverage_skipping_branch(&self) -> Option<Block> {
        let chooses_at = self
            .code
            .iter()
            .position(|line| line.contains(COVERAGE_CHOICE))?;
        self.blocks
            .iter()
            .filter(|block| block.open == chooses_at)
            .min_by_key(|block| block.depth)
            .cloned()
    }

    fn literal_assignments(&self) -> HashMap<String, String> {
        self.code
            .iter()
            .filter_map(|line| literal_assignment(line))
            .collect()
    }
}

impl RunExtent {
    /// The flags that produce this extent.
    #[must_use]
    pub fn flags(self) -> &'static [&'static str] {
        match self {
            Self::RunsEveryTestItWasGiven => &[RUNS_EVERYTHING],
            Self::CancelsAtTheFirstFailure => &[],
            Self::HidesTheFailureFromTheGate => &[HIDES_THE_FAILURE],
        }
    }
}

/// The three places `-Quick` is described, each graded against what it runs.
///
/// # Errors
///
/// Returns an error if the script or the testing document cannot be read.
pub fn quick_descriptions() -> Result<Vec<(&'static str, ModeDescription)>, Box<dyn Error>> {
    let script = GateText::of_the_repository()?;
    let document = fs::read_to_string(testing_document()?)?;
    Ok(vec![
        (
            "the mode line the gate prints",
            describes_quick(script.mode_line()),
        ),
        (
            "the .PARAMETER Quick docstring",
            describes_quick(script.quick_parameter().as_deref()),
        ),
        (
            "docs/technical/testing.md",
            describes_quick(quick_flag_sentence(&document).as_deref()),
        ),
    ])
}

/// How completely `description` names what `-Quick` runs.
#[must_use]
pub fn describes_quick(description: Option<&str>) -> ModeDescription {
    let Some(description) = description else {
        return ModeDescription::NoSuchDescriptionWasFound;
    };
    let said = description.to_lowercase();
    let mut named: Vec<(usize, &str)> = WHAT_QUICK_RUNS
        .iter()
        .filter(|(_, words)| words.iter().all(|word| said.contains(word)))
        .filter_map(|(stage, words)| Some((said.find(words.first()?)?, *stage)))
        .collect();
    named.sort_unstable();
    let named: Vec<String> = named
        .into_iter()
        .map(|(_, stage)| stage.to_owned())
        .collect();
    if named == WHAT_QUICK_RUNS.map(|(stage, _)| stage.to_owned()) {
        ModeDescription::NamesEveryStageInOrder
    } else {
        ModeDescription::NamesOnly(named)
    }
}

/// The stage keys the canonical table in `docs/technical/testing.md` lists.
///
/// # Errors
///
/// Returns an error if the document cannot be read or carries no such table.
pub fn stage_keys_the_document_lists() -> Result<Vec<String>, Box<dyn Error>> {
    let document = fs::read_to_string(testing_document()?)?;
    let rows: Vec<&str> = document
        .lines()
        .skip_while(|line| !(line.starts_with("| #") && line.contains("Stage")))
        .skip(1)
        .take_while(|line| line.starts_with('|'))
        .filter(|line| !line.contains("---"))
        .collect();
    if rows.is_empty() {
        return Err("docs/technical/testing.md carries no stage table to read".into());
    }
    Ok(rows
        .iter()
        .filter_map(|row| row.split('|').nth(2).map(stage_key))
        .collect())
}

/// What `text` says about the `&&` chain inside the GPU-free stages.
#[must_use]
pub fn chain_residual_of(text: &str) -> ChainResidual {
    let mut described = false;
    for paragraph in paragraphs_of(text) {
        let said = paragraph.to_lowercase();
        if !said.contains("gpu-free") || !(said.contains("&&") || said.contains("chain")) {
            continue;
        }
        described = true;
        if said.contains("cancel") && said.contains("hid") {
            return ChainResidual::ItStatesWhatTheChainStillHides;
        }
    }
    if described {
        ChainResidual::TheChainIsDescribedWithoutItsResidual
    } else {
        ChainResidual::NothingDescribesTheChain
    }
}

/// The living testing document, which is the gate's canonical description.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located or the document is
/// not there — a missing document is a broken reading, never a clean scan.
pub fn testing_document() -> Result<PathBuf, Box<dyn Error>> {
    let document = repository_root()?
        .join("docs")
        .join("technical")
        .join("testing.md");
    if !document.is_file() {
        return Err(format!("there is no testing document at {}", document.display()).into());
    }
    Ok(document)
}

/// The part of the testing document's flag paragraph that describes `-Quick`.
fn quick_flag_sentence(document: &str) -> Option<String> {
    let flags = document
        .lines()
        .skip_while(|line| !line.starts_with("Flags: `-Quick`"))
        .take_while(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let from = flags.find("`-Quick`")?;
    let to = flags.find("`-SkipCoverage`").unwrap_or(flags.len());
    flags.get(from..to).map(str::to_owned)
}

/// The lines of a script joined at their backtick continuations.
fn continued_commands(code: &[String]) -> Vec<String> {
    let mut commands = Vec::new();
    let mut carried = String::new();
    for line in code {
        let line = line.trim_end();
        if let Some(head) = line.strip_suffix('`') {
            carried.push_str(head);
            carried.push(' ');
        } else {
            carried.push_str(line);
            commands.push(std::mem::take(&mut carried));
        }
    }
    if !carried.is_empty() {
        commands.push(carried);
    }
    commands
}

fn invocation_of(segment: &str) -> Option<TestInvocation> {
    let runner = if segment.contains(INSTRUMENTED_NEXTEST) {
        INSTRUMENTED_NEXTEST
    } else if segment.contains(NEXTEST) {
        NEXTEST
    } else {
        return None;
    };
    Some(TestInvocation {
        runner: runner.to_owned(),
        selects: selection_of(segment),
        extent: extent_of(segment),
    })
}

fn selection_of(segment: &str) -> Selection {
    let mut words = segment.split_whitespace();
    while let Some(word) = words.next() {
        if word == "--workspace" {
            return Selection::Workspace;
        }
        if word == "-p" {
            return package_named(words.next());
        }
    }
    Selection::Unstated
}

fn package_named(package: Option<&str>) -> Selection {
    package.map_or(Selection::Unstated, |named| {
        Selection::Package(named.to_owned())
    })
}

/// Adds whatever flags `extent` is produced by to `flags`, keeping the order
/// they were first met in.
fn note_flags(extent: RunExtent, flags: &mut Vec<String>) {
    for flag in extent.flags() {
        if !flags.iter().any(|held| held == flag) {
            flags.push((*flag).to_owned());
        }
    }
}

fn extent_of(segment: &str) -> RunExtent {
    if segment.contains(HIDES_THE_FAILURE) {
        RunExtent::HidesTheFailureFromTheGate
    } else if segment.contains(RUNS_EVERYTHING) {
        RunExtent::RunsEveryTestItWasGiven
    } else {
        RunExtent::CancelsAtTheFirstFailure
    }
}

/// Adds whatever `text` declares or reports to the stages read so far.
fn note_stage(text: &str, names: &HashMap<String, String>, stages: &mut Vec<Vec<String>>) {
    if let Some(name) = argument_after(text, "Invoke-Stage ", names) {
        stages.push(vec![name]);
    } else if argument_after(text, "Write-StageHeader ", names).is_some() {
        stages.push(Vec::new());
    } else if let Some(name) = argument_after(text, "Write-Ok ", names)
        && let Some(open) = stages.last_mut()
    {
        open.push(name);
    }
}

/// The quoted or `$Variable` argument following `keyword`.
///
/// A `$Variable` is resolved against the script's own literal assignments; one
/// that resolves to nothing is not a stage, which is what keeps the helper
/// definitions at the top of the script out of the stage list.
fn argument_after(text: &str, keyword: &str, names: &HashMap<String, String>) -> Option<String> {
    let rest = text.split_once(keyword)?.1.trim_start();
    match rest.chars().next()? {
        '\'' | '"' => quoted(rest),
        '$' => {
            let held = rest
                .get(1..)?
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .next()?;
            names.get(held).cloned()
        }
        _ => None,
    }
}

/// The contents of the quoted string `text` opens with.
fn quoted(text: &str) -> Option<String> {
    let quote = text.chars().next()?;
    text.get(1..)?
        .split(quote)
        .next()
        .map(str::to_owned)
        .filter(|held| !held.is_empty())
}

/// A `$Name = 'literal'` assignment, if this line is one.
fn literal_assignment(line: &str) -> Option<(String, String)> {
    let (name, value) = line.trim_start().strip_prefix('$')?.split_once('=')?;
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let value = value.trim();
    matches!(value.chars().next(), Some('\'' | '"'))
        .then(|| quoted(value))
        .flatten()
        .map(|value| (name.to_owned(), value))
}

/// `names` with consecutive entries sharing a key collapsed to the first.
fn dedupe_by_key(names: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for name in names {
        if kept
            .last()
            .is_none_or(|held| stage_key(held) != stage_key(name))
        {
            kept.push(name.clone());
        }
    }
    kept
}

/// The word a stage is indexed under: whatever stands before its parenthetical.
fn stage_key(name: &str) -> String {
    name.split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .to_lowercase()
}

fn names_both_crates(stage: &str) -> bool {
    stage.contains("mc-testkit") && stage.contains("mc-render")
}

fn names_only(stage: &str, crate_name: &str) -> bool {
    stage.contains(crate_name) && !names_both_crates(stage)
}

/// Runs of consecutive non-blank lines.
fn paragraphs_of(text: &str) -> Vec<String> {
    text.lines()
        .collect::<Vec<_>>()
        .split(|line| line.trim().is_empty())
        .filter(|run| !run.is_empty())
        .map(|run| run.join("\n"))
        .collect()
}
