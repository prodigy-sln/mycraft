//! One place composes what entry did to the player, one place says it, and it is
//! said once.
//!
//! # Why a scan, and not a run of the program
//!
//! This is the *policy is not wiring* case. A test that calls `notice::entering`
//! and checks the sentence is agreement between two copies of one decision:
//! `collect_preparation` can stop calling it altogether and every such test stays
//! green, because the pure function it duplicates still answers correctly.
//! `crates/mc-client/src/notice_test.rs` is exactly that test, and it is worth
//! having — but it cannot see the wire.
//!
//! **A subprocess cannot reach it either.** `tests/shipped_binary.rs` runs the
//! real binary and works only because a missing content root refuses *before* a
//! device is opened. A launch that gets as far as seating a player has opened a
//! `wgpu::Surface` against a `winit::Window`, and nothing in this workspace's test
//! suite can construct one. So the instrument is a text scan over this crate's own
//! production sources, in the shape of `tests/reporting_seam.rs` and
//! `crates/mc-sim/tests/one_way_seats_a_player.rs`.
//!
//! Deleting `notice::say_entering(prepared.clearing);` from `collect_preparation`
//! is the mutation this file exists for. Nothing else in the workspace reddens.
//!
//! # An enumerated verdict, never an absence
//!
//! `assert!(sites.is_empty())` cannot tell a well-wired client from a walk that
//! broke, a filter that skipped everything, or a source root that has moved. So
//! the answer is one of four verdicts and each test compares the whole of it,
//! which rejects every other answer — the one meaning "I could not look" included
//! — for free.
//!
//! # The reading order, which is inverted from the sibling scan's and deliberately
//!
//! `one_way_seats_a_player.rs` reports a *second source* before a *missing door*,
//! because a crate in both states is one where somebody moved the seating. Here
//! the order is the other way round: **what is not said yet outranks what is said
//! somewhere it should not be.** On the tree this file was written against,
//! `Clearing` is named in `src/app/reload.rs` — a file `report_clearing` takes
//! with it — so an extras-first reading would open the phase with a diagnosis
//! about a file that is about to be deleted, while the fact a reader needs is that
//! nothing composes the sentence at all yet. It is also what makes the deletion
//! mutation answer `ComposedButNeverSaid` rather than something vaguer.
//!
//! **The masking is real and is recorded rather than papered over:** while
//! anything the rule asks for is unstated, a second source saying it is invisible.
//! The two are never both true in a tree anyone would commit, and the moment the
//! first is fixed the second is reported.
//!
//! # The counts, re-derived over production text — a bare `rg` is the wrong
//! instrument
//!
//! [`production_text`] drops every line whose trimmed start is `///` or `//!`, and
//! [`is_production_source`] skips every sibling `*_test.rs`, so **the shell and
//! this scan disagree by construction**. Every figure in [`NEEDLES`] was
//! re-derived on 2026-08-18 against `9a3f0eb` with
//! `rg -n --no-heading -F "<needle>" crates/mc-client/src`, striking those lines
//! out by hand. Three standing counter-examples in this tree, each one edit away
//! from moving the shell's number without moving this scan's:
//!
//! ```text
//! crates/mc-client/src/session/mod.rs:295:    /// Clearing it here makes "a click…
//! crates/mc-client/src/launch.rs:75:/// …the frame path would have had to pick; a [`Clearing`]…
//! crates/mc-client/src/notice_test.rs:48:use mc_sim::world::Clearing;
//! ```
//!
//! `rg -l "Clearing" crates/mc-client/src` reports **five** files where this scan
//! sees **three**: it drops the `///` prose above and the sibling unit file that
//! tests the composition. Before `notice.rs` the three were `src/app/reload.rs`,
//! `src/launch.rs`, `src/session/reload.rs` (9 lines); after it they are
//! `src/launch.rs`, `src/notice.rs`, `src/session/reload.rs`, because
//! `report_clearing` takes all five of `app/reload.rs`'s mentions and its `use`
//! with it, and `src/app/mod.rs` names only the lower-case `prepared.clearing`.
//! **`tasks.md`'s Correction 2 — 8 raw lines across 3 files, 7 visible across 2 —
//! was true at `f260c64` and is not now**: phase 1 put `pub clearing: Clearing` on
//! `PreparedLaunch` (`:98`) with a `///` mention at `:75`, making it 11 raw across
//! 4 and 9 visible across 3. Its *rule* is unchanged and is what binds.
//!
//! # No needle here is a prefix test, and each was checked against its own
//! extension
//!
//! Phase 1 lost a mutation to exactly that: `) -> Seated` is contained in
//! `) -> SeatedPlayer {`, so renaming the door left the scan green.
//!
//! - `fn say_entering(` — the trailing parenthesis is load-bearing. Without it,
//!   renaming the function to `say_entering_now` leaves the definition needle
//!   matching. **This is a deliberate tightening of the `fn say_entering` spelling
//!   in the D6 table**, which was a prefix test; the count is 0 either way today,
//!   so nothing is expected of the implementation by it.
//! - `notice::say_entering(` — the same, and module-qualified because **that
//!   spelling is binding**: the count is then the number of calls and cannot be
//!   moved by import style.
//! - `entered the world inside solid blocks` and `so you were left inside them` —
//!   fragments of a sentence, so an extension of either is a longer sentence
//!   containing it, which is the offence rather than an escape.
//! - `Clearing` — a file set and not a count, precisely because `notice.rs` and
//!   `launch.rs` will legitimately name the type several times and a count there
//!   would churn.
//!
//! # What the needles rest on, and the holes that stay open
//!
//! **The const keeps each clause *single*, not unwrapped** — measured, and not
//! what this paragraph first claimed. `format_strings` is off with no
//! `rustfmt.toml`, so rustfmt never reflows a literal: inlining both clauses left
//! them whole at 115 and 148 characters, and what reddened was the duplicate,
//! caught by `OnlyIn { times: 1 }`.
//!
//! **The wrap hazard is real, rustfmt cannot reach it, a hand-written `\`
//! continuation can, and against that this scan is blind.** [`production_text`]
//! joins lines with `\n` before any `contains`, so a hand-wrapped clause matches
//! nothing, the count returns to 1, and the suite goes green with the idiom
//! abandoned — measured. Review alone sees it, as it alone sees a site importing
//! these consts to compose a third sentence. A `say_entering` that stopped
//! writing is bounded instead: its body is the `eprintln!` and `notice_test.rs`
//! holds the words. And `-D warnings` backs every needle — a name imported and
//! never called fails the gate, so "named but never called" cannot happen here.

use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The one file that composes what entry did, relative to the crate root.
const THE_COMPOSITION: &str = "src/notice.rs";

/// The one file that says it, relative to the crate root.
const WHERE_THE_LAUNCH_IS_COLLECTED: &str = "src/app/mod.rs";

/// What a rule asks of the tree.
enum Expectation {
    /// Said exactly this often in this file, and said nowhere else at all.
    OnlyIn { home: &'static str, times: usize },
    /// Named in exactly these files, however often each of them names it.
    NamedInExactly(&'static [&'static str]),
}

impl Expectation {
    /// Every file the spelling must appear in, and how often it must appear
    /// there.
    fn required(&self) -> Vec<(&'static str, usize)> {
        match *self {
            Expectation::OnlyIn { home, times } => vec![(home, times)],
            Expectation::NamedInExactly(files) => files.iter().map(|file| (*file, 1)).collect(),
        }
    }

    /// Whether `file` saying the spelling `times` times is something this rule
    /// permits.
    ///
    /// A file-set rule sets no ceiling: `notice.rs` names the type once per match
    /// arm, and a count there would churn on every rewording.
    fn permits(&self, file: &str, times: usize) -> bool {
        match *self {
            Expectation::OnlyIn {
                home,
                times: allowed,
            } => file == home && times <= allowed,
            Expectation::NamedInExactly(files) => files.contains(&file),
        }
    }
}

/// One spelling the rule is stated in, and what is expected of it.
struct Rule {
    /// The spelling looked for in production text.
    names: &'static str,
    /// What the tree must do with it.
    expects: Expectation,
}

/// How composing the entry sentence and saying it are spelled, and where each
/// belongs.
///
/// Every figure re-derived on 2026-08-18 at `9a3f0eb` — see this file's header for
/// the command, for the three lines where the shell and this scan disagree, and
/// for why none of these is a prefix test.
///
/// - `entered the world inside solid blocks` — the clause both entry sentences
///   open with, declared once as a const. 0 in production text today; 1 after,
///   only in `notice.rs`.
/// - `so you were left inside them` — the entry refusal's distinguishing tail,
///   which carries no interpolation and so can be matched whole. The refusal's
///   `8` never appears in the source at all: it is
///   `Clearing::NoClearSpaceWithin { blocks }` interpolated. 0 today; 1 after.
/// - `fn say_entering(` — the definition. 0 today; 1 after, only in `notice.rs`.
/// - `notice::say_entering(` — the call. 0 today; 1 after, only in `app/mod.rs`.
///   Splitting the definition from the call is what makes "composed but never
///   said" and "said twice" different answers.
/// - `Clearing` — a file set rather than a count. Three files today
///   (`app/reload.rs`, `launch.rs`, `session/reload.rs`, 9 lines); three after,
///   with `notice.rs` in place of `app/reload.rs`. **A fourth file is a verdict
///   parked somewhere the frame path can re-read it**, which is the shape
///   FR-2.1-S6 forbids — and it is why this needle is scoped to the whole of
///   `src` rather than to `src/app`: `redraw` takes `&mut Session`, and
///   `src/session/reload.rs:79` already parks a `Clearing` on an accepted reload,
///   so the natural place to park a second one is three files from where a
///   `src/app` scan could see it.
const NEEDLES: &[Rule] = &[
    Rule {
        names: "entered the world inside solid blocks",
        expects: Expectation::OnlyIn {
            home: THE_COMPOSITION,
            times: 1,
        },
    },
    Rule {
        names: "so you were left inside them",
        expects: Expectation::OnlyIn {
            home: THE_COMPOSITION,
            times: 1,
        },
    },
    Rule {
        names: "fn say_entering(",
        expects: Expectation::OnlyIn {
            home: THE_COMPOSITION,
            times: 1,
        },
    },
    Rule {
        names: "notice::say_entering(",
        expects: Expectation::OnlyIn {
            home: WHERE_THE_LAUNCH_IS_COLLECTED,
            times: 1,
        },
    },
    Rule {
        names: "Clearing",
        expects: Expectation::NamedInExactly(&[
            "src/launch.rs",
            THE_COMPOSITION,
            "src/session/reload.rs",
        ]),
    },
];

/// One place a spelling was named, and how often it was named there.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Site {
    /// Where it sits, relative to the crate root, `/`-separated on every platform.
    file: String,
    /// The spelling that gave it away.
    names: String,
    /// How many times that file's production text says it.
    times: usize,
}

/// What a walk of this crate's sources found, before it is read as a verdict.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    sites: Vec<Site>,
}

/// What the scan came to.
///
/// Four answers rather than a list and a count. "One place says it", "somewhere
/// else says it", "nothing says it" and "nothing could be looked at" are four
/// different facts, and none of them may compare equal to another.
#[derive(Debug, PartialEq, Eq)]
enum EntrySentence {
    /// One place composes it, one place says it, and no client state holds an
    /// entry verdict.
    ComposedOnceAndSaidWhereTheLaunchIsCollected,
    /// A source composes or says it somewhere else, or says it more than once.
    AnotherSourceComposesOrSaysIt(Vec<Site>),
    /// Something the rule asks for is not stated anywhere — of which the case it
    /// is named for, and the one the deletion mutation produces, is a composition
    /// nothing calls.
    ComposedButNeverSaid,
    /// No production source was read at all, so nothing above could be said.
    NoSourceWasRead,
}

#[test]
fn the_client_composes_the_entry_sentence_once_and_says_it_where_the_launch_is_collected()
-> TestResult {
    let scanned = scan(&crate_root())?;

    assert_eq!(
        verdict_of(&scanned),
        EntrySentence::ComposedOnceAndSaidWhereTheLaunchIsCollected,
        "what entry did to the player reaches them only if something writes it: the composition \
         is a pure function, and a client that composes the sentence and never says it leaves \
         every test of that function green while the player is moved in silence. Unstated here, \
         each `<spelling> in <file>`: {:?}",
        unsaid(&scanned)
    );
    Ok(())
}

/// The control for the scenario above, in three directions at once.
///
/// A walk that broke, a filter that skipped everything, or a needle that matches
/// nothing even when the offence is committed would each report a well-wired
/// client forever. So the second source names **every** needle the rule carries
/// rather than one of them, and the expectation is derived from [`NEEDLES`] — a
/// needle added without a fixture to catch it fails here rather than standing
/// unwatched.
///
/// The third direction is the `*_test.rs` skip: a sibling unit file saying the
/// same words must be passed over, and comparing the whole verdict is what says
/// so, because a scan that read it would report a site this expectation does not
/// hold.
#[test]
fn a_second_source_that_composes_or_says_the_entry_sentence_is_named_by_the_verdict() -> TestResult
{
    let fixture = tempfile::tempdir()?;
    a_client_that_says_it_in_the_frame_path(fixture.path())?;

    let scanned = scan(fixture.path())?;

    assert_eq!(
        verdict_of(&scanned),
        EntrySentence::AnotherSourceComposesOrSaysIt(every_needle_named_in("src/app/frame.rs")),
        "an entry verdict parked where the frame path can re-read it says the sentence again on \
         every frame drawn, and a second composition says it in words nobody reviewed. Both are \
         invisible to a test of the composing function, and both are what a file naming these \
         spellings outside its one home looks like"
    );
    Ok(())
}

/// The second control, feeding the verdict this phase opens red on.
///
/// **This is FR-2.1-S5's actual failure mode.** Without a control feeding it, a
/// scan that stopped being able to find the call site reads as a well-wired
/// client: the hole *inside* the good verdict, where a rule looking for a spelling
/// nothing spells any more answers "one place says it" about a tree where nothing
/// does.
#[test]
fn a_composition_nothing_asks_for_is_named_by_the_verdict() -> TestResult {
    let fixture = tempfile::tempdir()?;
    a_client_that_composes_it_and_never_says_it(fixture.path())?;

    let scanned = scan(fixture.path())?;

    assert_eq!(
        verdict_of(&scanned),
        EntrySentence::ComposedButNeverSaid,
        "the sentence is composed, correct, and unit-tested, and the client never calls it — the \
         state this whole file exists to make visible, and the one every behavioural test of the \
         composition is blind to"
    );
    Ok(())
}

/// The third control, feeding the verdict that says nothing was read.
///
/// A source root that has moved, a crate layout that changed, or a walk that
/// stopped recursing all land here rather than in a green pass.
#[test]
fn a_tree_with_no_production_source_is_named_by_the_verdict() -> TestResult {
    let nowhere = tempfile::tempdir()?;

    let scanned = scan(nowhere.path())?;

    assert_eq!(
        verdict_of(&scanned),
        EntrySentence::NoSourceWasRead,
        "a scan with nothing to read must not answer the same way as a scan that read the crate \
         and found it well wired; a source root that has moved or gone is how this guard stops \
         being able to look, and it has to say so"
    );
    Ok(())
}

/// The fourth control, and the only one that exercises the good verdict before
/// the implementation exists.
///
/// **A rule set nothing can satisfy is the failure this catches**, and it is a
/// real one here because the rules are heterogeneous: a needle whose home
/// contradicts another's, or a sixth needle added with an expectation no tree can
/// meet, would leave the implementer fighting an instrument rather than writing
/// the feature — with the fight looking exactly like a defect in their code. The
/// fixture is built *from* [`NEEDLES`], so a needle added without a satisfiable
/// expectation reddens here rather than at the end of somebody's afternoon.
#[test]
fn a_tree_that_states_every_rule_is_named_by_the_good_verdict() -> TestResult {
    let fixture = tempfile::tempdir()?;
    write_sources(fixture.path(), &the_rules_as_files())?;

    let scanned = scan(fixture.path())?;

    assert_eq!(
        verdict_of(&scanned),
        EntrySentence::ComposedOnceAndSaidWhereTheLaunchIsCollected,
        "every spelling in its stated home, said as often as its rule asks and no more, is what \
         the good verdict means; a rule set that cannot be satisfied by the tree it describes is \
         an instrument nobody can go green against"
    );
    Ok(())
}

/// One site per needle, all in `file`, said once each, in the order the scan
/// reports them.
///
/// Derived from the rule's own list rather than written out, so the expectation
/// cannot fall behind the needles it is expecting.
fn every_needle_named_in(file: &str) -> Vec<Site> {
    NEEDLES
        .iter()
        .map(|rule| Site {
            file: file.to_owned(),
            names: rule.names.to_owned(),
            times: 1,
        })
        .collect()
}

/// A client wired exactly as the rules ask, whose frame path says it again.
///
/// `src/app/frame.rs` names every needle once, which is the offence;
/// `src/app/frame_test.rs` says the same words and must be passed over, because a
/// sibling unit file is not production text.
fn a_client_that_says_it_in_the_frame_path(root: &Path) -> Result<(), Box<dyn Error>> {
    write_sources(root, &the_rules_as_files())?;
    write_sources(
        root,
        &BTreeMap::from([
            ("src/app/frame.rs".to_owned(), every_needle_once()),
            ("src/app/frame_test.rs".to_owned(), every_needle_once()),
        ]),
    )
}

/// A client wired exactly as the rules ask, with the one call deleted.
///
/// **This is the mutation's own shape, and `tasks.md` prescribes a weaker one** —
/// a fixture holding `src/notice.rs` alone. That tree is missing four spellings
/// at once, so it cannot say whether deleting *the call* is what reaches this
/// verdict, which is the only thing T10's M8 needs to know. Here everything else
/// is stated and `src/app/mod.rs` survives with its call gone, so the verdict is
/// attributable to one line. The weaker tree is not left ungraded: today's real
/// client is a superset of it, and the scenario test above reads it.
fn a_client_that_composes_it_and_never_says_it(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut files = the_rules_as_files();
    files.insert(
        WHERE_THE_LAUNCH_IS_COLLECTED.to_owned(),
        "// the launch is collected here, and nothing is said about the player\n".to_owned(),
    );
    write_sources(root, &files)
}

/// One file per home the rules name, holding each spelling as often as its rule
/// asks for it there.
fn the_rules_as_files() -> BTreeMap<String, String> {
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    for (home, text) in NEEDLES.iter().flat_map(one_rules_files) {
        files.entry(home).or_default().push_str(&text);
    }
    files
}

/// Every (file, text) pair one rule asks for.
fn one_rules_files(rule: &Rule) -> Vec<(String, String)> {
    rule.expects
        .required()
        .into_iter()
        .map(|(home, times)| (home.to_owned(), said(rule.names, times)))
        .collect()
}

/// One spelling, on a line of its own, `times` times.
fn said(needle: &str, times: usize) -> String {
    (0..times)
        .map(|_| format!("{needle} // stated here\n"))
        .collect()
}

/// Every needle the rule carries, said once each.
fn every_needle_once() -> String {
    NEEDLES
        .iter()
        .map(|rule| format!("{} // the offence\n", rule.names))
        .collect()
}

/// Writes each `(path, text)` under `root`, creating the directories it needs.
fn write_sources(root: &Path, files: &BTreeMap<String, String>) -> Result<(), Box<dyn Error>> {
    for (file, text) in files {
        let path = root.join(file);
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory)?;
        }
        fs::write(path, text)?;
    }
    Ok(())
}

/// This crate's own directory, which the source root is relative to.
fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Reads every production source under `crate_root`'s `src` and records what it
/// found.
fn scan(crate_root: &Path) -> Result<Scan, Box<dyn Error>> {
    let mut scanned = Scan::default();
    let directory = crate_root.join("src");
    if directory.is_dir() {
        walk(&directory, crate_root, &mut scanned)?;
    }
    Ok(scanned)
}

/// What a scan says, read in the order this file's header sets out.
fn verdict_of(scanned: &Scan) -> EntrySentence {
    if scanned.files_read == 0 {
        return EntrySentence::NoSourceWasRead;
    }
    if !unsaid(scanned).is_empty() {
        return EntrySentence::ComposedButNeverSaid;
    }
    let elsewhere = said_where_no_rule_allows(&scanned.sites);
    if !elsewhere.is_empty() {
        return EntrySentence::AnotherSourceComposesOrSaysIt(elsewhere);
    }
    EntrySentence::ComposedOnceAndSaidWhereTheLaunchIsCollected
}

/// Every `<spelling> in <file>` a rule asks for and the tree does not state often
/// enough.
fn unsaid(scanned: &Scan) -> Vec<String> {
    NEEDLES
        .iter()
        .flat_map(|rule| {
            one_rules_files(rule)
                .into_iter()
                .map(move |pair| (rule, pair))
        })
        .filter(|(rule, (home, _))| {
            said_in(&scanned.sites, home, rule.names) < required(rule, home)
        })
        .map(|(rule, (home, _))| format!("{} in {home}", rule.names))
        .collect()
}

/// How often `rule` asks for its spelling in `home`.
fn required(rule: &Rule, home: &str) -> usize {
    rule.expects
        .required()
        .into_iter()
        .find(|(file, _)| *file == home)
        .map_or(0, |(_, times)| times)
}

/// How often `file`'s production text says `names`.
fn said_in(sites: &[Site], file: &str, names: &str) -> usize {
    sites
        .iter()
        .find(|site| site.file == file && site.names == names)
        .map_or(0, |site| site.times)
}

/// Every site whose file, or whose count in that file, no rule permits.
fn said_where_no_rule_allows(sites: &[Site]) -> Vec<Site> {
    sites
        .iter()
        .filter(|site| !permitted(site))
        .cloned()
        .collect()
}

/// Whether any rule permits this file to say this spelling this often.
fn permitted(site: &Site) -> bool {
    NEEDLES
        .iter()
        .filter(|rule| rule.names == site.names)
        .any(|rule| rule.expects.permits(&site.file, site.times))
}

fn walk(directory: &Path, crate_root: &Path, scanned: &mut Scan) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(directory)?
        .map(|entry| entry.map(|found| found.path()))
        .collect::<Result<Vec<_>, _>>()?;
    // Read in a settled order, so the sites a failure prints are the same list
    // whatever order the filesystem hands its entries back in.
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, crate_root, scanned)?;
        } else if is_production_source(&path) {
            read(&path, crate_root, scanned)?;
        }
    }
    Ok(())
}

/// Reads one file and records how often it says each needle, ignoring the ones it
/// never says.
fn read(path: &Path, crate_root: &Path, scanned: &mut Scan) -> Result<(), Box<dyn Error>> {
    let relative = relative_spelling(path, crate_root)?;
    let text = production_text(&fs::read_to_string(path)?);
    scanned.files_read += 1;
    for rule in NEEDLES {
        let times = text.matches(rule.names).count();
        if times > 0 {
            scanned.sites.push(Site {
                file: relative.clone(),
                names: rule.names.to_owned(),
                times,
            });
        }
    }
    Ok(())
}

/// A `.rs` file that is not a sibling unit-test file.
///
/// Unit tests live beside the code they test, so skipping them is a file-name
/// filter rather than a parse. `src/notice_test.rs` holds all four sentences and
/// is not a place any of them is said.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

/// A file's text with its doc comments removed.
///
/// A rustdoc example is a doc test, so prose *about* a sentence is not a saying of
/// it. See the header: this is exactly why a bare `rg` counts something else.
fn production_text(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("///") && !trimmed.starts_with("//!")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where a file sits relative to the crate root, spelled with `/` on every
/// platform so a site reads the same wherever the suite runs.
fn relative_spelling(path: &Path, crate_root: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(crate_root)?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/"))
}
