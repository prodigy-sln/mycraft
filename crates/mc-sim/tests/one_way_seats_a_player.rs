//! There is exactly one way to seat a player in a world, and it says what it did.
//!
//! The compiler holds this against every caller **outside** the crate: the
//! constructor is module-private and the admission door is the only public way to
//! make a simulation. It holds nothing at all against `mc-sim`'s own sources — a
//! second seating path added inside this crate reopens the hole with the whole
//! suite green, and a networked join is exactly that shape. This scan is what
//! makes adding one visible.
//!
//! # An enumerated verdict, never an absence
//!
//! `assert!(sites.is_empty())` cannot tell a clean crate from a walk that broke, a
//! filter that skipped everything, or a source root that has moved. So the answer
//! is one of four verdicts and the tests compare the whole of it, which rejects
//! every other answer — the two meaning "I could not look" included — for free.
//!
//! # `times`, and the hole it closes
//!
//! `tests/reporting_seam.rs` pushes one site per (file, needle) pair, so a
//! *second* offence in a file already named is invisible there. A site here
//! carries how many times its file said the spelling, and the rule states how many
//! times the door itself may say it — so a second construction added inside
//! `simulation.rs`, which is the one file every needle is at home in, moves a count
//! rather than adding a file and is caught by that alone.
//!
//! # The counts are derived over production text, and a bare `rg` is the wrong
//! instrument
//!
//! [`production_text`] drops every line whose trimmed start is `///` or `//!`
//! before anything is counted, so **the shell and the scan disagree by
//! construction**. The standing counter-example in this tree is
//! `crates/mc-sim/src/reload/mod.rs:91`, where the English word "published"
//! followed by a colon sits inside a `///` line:
//!
//! ```text
//! $ rg -n --no-heading -F "published:" crates/mc-sim/src
//! crates/mc-sim/src/reload/mod.rs:91:/// Call it after the tick it follows has been published: a tick answers every
//! crates/mc-sim/src/simulation.rs:118:    published: ArcSwap<SimSnapshot>,
//! crates/mc-sim/src/simulation.rs:145:            published: ArcSwap::from_pointee(SimSnapshot {
//! ```
//!
//! `rg` reports **three**; this scan sees **two**, both in `simulation.rs`, which
//! is the number [`NEEDLES`] states. Every count in that table was re-derived from
//! the tree with the command above, with the `///` and `//!` lines struck out by
//! hand — never copied from a green run, and never from the architecture.
//!
//! **The residual hole is one character wide, and it is recorded rather than
//! papered over.** Turning that `///` into `//` — a plausible edit, since the line
//! is prose — makes the count three and reddens this scan with no second seating
//! door anywhere. The needle stays, because a struct-literal second door inside
//! `simulation.rs` is the shape it is paid to catch, and a scan that reddens
//! naming `reload/mod.rs` is a diagnosis a reader can act on in a minute.
//!
//! **No ordinary `//` comment under `crates/mc-sim/src` may name a needle.** Only
//! `///` and `//!` are stripped, so `// moved off Simulation::new` left behind by
//! whoever moves the constructor *is* a site, and this scan will say so.
//!
//! # Shape
//!
//! `crates/mc-client/tests/reporting_seam.rs`'s: production text with its doc
//! comments removed, sibling `*_test.rs` unit files passed over, `/`-separated
//! relative paths, and `tempfile` trees as the positive controls. The root is
//! `crates/mc-sim/src` and there is no exemption list — a guard whose scope is a
//! hand-maintained list of permitted sites goes green with one more entry on the
//! day a new door is opened.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The one file that seats a player, relative to the crate root.
const THE_DOOR: &str = "src/simulation.rs";

/// One spelling the rule is stated in: where it belongs, and how often.
struct Rule {
    /// The spelling looked for in production text.
    names: &'static str,
    /// The one file allowed to say it, or `None` where no file may.
    home: Option<&'static str>,
    /// How many times that file must say it. Zero where there is no home.
    times: usize,
}

/// What seating a player is spelled as, and what the rule says about each
/// spelling.
///
/// Every count was re-derived from the tree on 2026-08-18 with
/// `rg -n --no-heading -F "<needle>" crates/mc-sim/src`, dropping the `///` and
/// `//!` lines by hand so the figure matches what [`production_text`] leaves
/// behind. See this file's header for the one needle where the two disagree.
///
/// - `Simulation::new(` — the construction itself. Two sites today
///   (`src/persistence.rs`, `src/replay/spawn.rs`), both of which move onto the
///   admission door; afterwards the door is its only caller.
/// - `Self::new(` — the same construction under the name it would have inside its
///   own `impl`. Never anywhere: it is the spelling a second door added beside the
///   first would most naturally use.
/// - `published:` — the field the first snapshot is stored in, and so the
///   spelling of a simulation built by struct literal rather than through the
///   constructor. Twice in the door and nowhere else: the declaration and the one
///   initialiser.
/// - `self.player =` — the assignment that puts a player into a simulation after
///   it exists. Once, in the tick.
/// - `) -> Seated {` — the door handing its clearing back. Exactly one function
///   may return a seating, and this is what says the door still does. **The
///   trailing brace is load-bearing, and it was measured into this needle rather
///   than reasoned into it.** Without it the needle is a prefix test:
///   `) -> SeatedPlayer {` contains `) -> Seated`, so renaming the return type to
///   anything beginning with `Seated` left the scan green over a door that no
///   longer hands back the type the rule names. A rename to `Admitted` bit either
///   way — it is only the `Seated`-prefixed rename that escaped, and that is
///   exactly the rename somebody makes. The brace rejects it outright, because a
///   renamed type carries its own characters between the name and the body.
const NEEDLES: &[Rule] = &[
    Rule {
        names: "Simulation::new(",
        home: Some(THE_DOOR),
        times: 1,
    },
    Rule {
        names: "Self::new(",
        home: None,
        times: 0,
    },
    Rule {
        names: "published:",
        home: Some(THE_DOOR),
        times: 2,
    },
    Rule {
        names: "self.player =",
        home: Some(THE_DOOR),
        times: 1,
    },
    Rule {
        names: ") -> Seated {",
        home: Some(THE_DOOR),
        times: 1,
    },
];

/// One place a needle was named, and how often it was named there.
///
/// The count is what distinguishes a second offence inside an already-named file
/// from the first one, which is precisely what a one-site-per-pair scan cannot
/// see.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Site {
    /// Where it sits, relative to the crate root, `/`-separated on every platform.
    file: String,
    /// The spelling that gave it away.
    names: String,
    /// How many times that file's production text says it.
    times: usize,
}

/// What the scan came to.
///
/// Four answers rather than a list and a count. "Nothing was found", "the door is
/// not where the rule says it is" and "nothing could be looked at" are three
/// different facts, and none of them may compare equal to another.
#[derive(Debug, PartialEq, Eq)]
enum Seating {
    /// The one door makes the one simulation, and it hands the clearing back.
    OneWaySeatsAPlayerAndItReportsItsClearing,
    /// These sources put a player into a simulation too, or the door does it twice.
    AnotherSourceSeatsAPlayer(Vec<Site>),
    /// A spelling the door is known by is not where the rule says it is — it
    /// moved, was renamed, or the scan can no longer see it.
    TheDoorNoLongerSeatsAPlayer(Vec<String>),
    /// No production source was read at all, so nothing above could be said.
    NoSourceWasRead,
}

#[test]
fn the_simulation_crates_sources_state_one_way_to_seat_a_player_and_it_reports_its_clearing()
-> TestResult {
    let scanned = verdict_over(&crate_root())?;

    assert_eq!(
        scanned,
        Seating::OneWaySeatsAPlayerAndItReportsItsClearing,
        "every way into a world — a resume, a first launch, a golden capture, every fixture — has \
         to pass through one function that asks whether the player it is about to seat is inside \
         something solid, and hands back what it did. A second source that builds a simulation of \
         its own is a door where that question is not asked, and it is invisible to every \
         behavioural test in this crate because the world it makes is perfectly good; what is \
         missing is the asking"
    );
    Ok(())
}

/// The control for the scenario above, in three directions at once.
///
/// A walk that broke, a filter that skipped everything, or a needle that matches
/// nothing even when the offence is committed would each report a clean crate
/// forever. So the second source names **every** needle the rule carries rather
/// than one of them, and the expectation is derived from [`NEEDLES`] — a needle
/// added without a fixture to catch it fails here rather than standing unwatched.
///
/// The third direction is the `*_test.rs` skip: a sibling unit file saying the
/// same spellings must be passed over, and comparing the whole verdict is what
/// says so, because a scan that read it would report a site this expectation does
/// not hold.
#[test]
fn a_second_source_that_seats_a_player_is_named_by_the_verdict() -> TestResult {
    let fixture = tempfile::tempdir()?;
    a_crate_with_a_second_door(fixture.path())?;

    let scanned = verdict_over(fixture.path())?;

    assert_eq!(
        scanned,
        Seating::AnotherSourceSeatsAPlayer(every_needle_named_once_in("src/join.rs")),
        "the crate's own join puts a player into a world without going through the door, which is \
         the shape a networked join takes and the one the compiler cannot refuse. The scan has to \
         reach into the source directory, read the file that does it, report every spelling it did \
         it by, and pass over the sibling unit file that says the same words"
    );
    Ok(())
}

/// The second control, feeding the verdict that says the scan can no longer see
/// the door.
///
/// Without it a renamed, moved or deleted door reads as a clean crate forever:
/// that is the hole *inside* the good verdict, where a rule looking for a spelling
/// nothing spells any more answers "one way seats a player" about a crate where
/// nothing does.
#[test]
fn a_door_that_no_longer_hands_a_seating_back_is_named_by_the_verdict() -> TestResult {
    let fixture = tempfile::tempdir()?;
    a_crate_whose_door_returns_something_else(fixture.path())?;

    let scanned = verdict_over(fixture.path())?;

    assert_eq!(
        scanned,
        Seating::TheDoorNoLongerSeatsAPlayer(vec![") -> Seated {".to_owned()]),
        "the door still constructs the one simulation and no other source seats anybody, and it \
         has stopped handing back what it did about the player it seated. That is not a clean \
         crate and it must not be reported as one — a rule looking for a spelling nothing spells \
         any more is a rule that has stopped asking anything"
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

    let scanned = verdict_over(nowhere.path())?;

    assert_eq!(
        scanned,
        Seating::NoSourceWasRead,
        "a scan with nothing to read must not answer the same way as a scan that read the crate \
         and found one door; a source root that has moved or gone is how this guard stops being \
         able to look, and it has to say so"
    );
    Ok(())
}

/// One site per needle, all in `file`, said once each, in the order the scan
/// reports them.
///
/// Derived from the rule's own list rather than written out, so the expectation
/// cannot fall behind the needles it is expecting.
fn every_needle_named_once_in(file: &str) -> Vec<Site> {
    NEEDLES
        .iter()
        .map(|rule| Site {
            file: file.to_owned(),
            names: rule.names.to_owned(),
            times: 1,
        })
        .collect()
}

/// A crate whose door is well formed and whose join seats a player anyway.
///
/// Three files. `simulation.rs` satisfies every rule exactly; `join.rs` names
/// every needle once, which is the offence; `join_test.rs` says the same words and
/// must be passed over, because a sibling unit file is not production text.
fn a_crate_with_a_second_door(root: &Path) -> Result<(), Box<dyn Error>> {
    let sources = root.join("src");
    fs::create_dir_all(&sources)?;
    fs::write(
        sources.join("simulation.rs"),
        the_door_as_the_rule_wants_it(),
    )?;
    fs::write(sources.join("join.rs"), every_needle_once())?;
    fs::write(sources.join("join_test.rs"), every_needle_once())?;
    Ok(())
}

/// A crate whose door constructs the one simulation and hands nothing back.
///
/// One file, satisfying every rule but the last: no function returns a seating,
/// which is the door having been renamed or moved out from under the rule.
fn a_crate_whose_door_returns_something_else(root: &Path) -> Result<(), Box<dyn Error>> {
    let sources = root.join("src");
    fs::create_dir_all(&sources)?;
    fs::write(
        sources.join("simulation.rs"),
        the_door_as_the_rule_wants_it().replace(") -> Seated {", ") -> Simulation {"),
    )?;
    Ok(())
}

/// A door saying each needle exactly as often as the rule allows.
fn the_door_as_the_rule_wants_it() -> String {
    let mut door = String::from("pub fn seat(spawn: PlayerState) -> Seated {\n");
    door.push_str("    let simulation = Simulation::new(spawn);\n");
    door.push_str("    Seated { simulation, clearing }\n}\n");
    door.push_str("struct Simulation {\n    published: ArcSwap<SimSnapshot>,\n}\n");
    door.push_str("fn build() -> Self {\n    Self {\n        published: first,\n    }\n}\n");
    door.push_str("fn advance(&mut self) {\n    self.player = walked;\n}\n");
    door
}

/// Every needle the rule carries, said once each.
fn every_needle_once() -> String {
    NEEDLES
        .iter()
        .map(|rule| format!("{} // the offence\n", rule.names))
        .collect()
}

/// This crate's own directory, which the source root is relative to.
fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Reads every production source under `crate_root`'s `src` and says what it
/// found.
///
/// **The order of the answers is the whole of the reading.** A source that seats a
/// player is reported before a door that has gone missing, because a crate in both
/// states is one where somebody moved the seating rather than one where the door
/// was renamed — and that is what a reader has to be told first. Today's tree is
/// exactly that crate.
fn verdict_over(crate_root: &Path) -> Result<Seating, Box<dyn Error>> {
    let mut counted = Vec::new();
    let mut files_read = 0;
    let directory = crate_root.join("src");
    if directory.is_dir() {
        walk(&directory, crate_root, &mut counted, &mut files_read)?;
    }
    if files_read == 0 {
        return Ok(Seating::NoSourceWasRead);
    }
    let elsewhere = said_away_from_home(&counted);
    if !elsewhere.is_empty() {
        return Ok(Seating::AnotherSourceSeatsAPlayer(elsewhere));
    }
    let missing = said_less_often_than_the_rule_asks(&counted);
    if !missing.is_empty() {
        return Ok(Seating::TheDoorNoLongerSeatsAPlayer(missing));
    }
    Ok(Seating::OneWaySeatsAPlayerAndItReportsItsClearing)
}

/// Every site whose file is not the one its rule allows, or whose file is the one
/// its rule allows and says it more often than the rule does.
fn said_away_from_home(counted: &[Site]) -> Vec<Site> {
    counted
        .iter()
        .filter(|site| {
            NEEDLES
                .iter()
                .any(|rule| rule.names == site.names && said_too_much(rule, site))
        })
        .cloned()
        .collect()
}

/// Whether one file's count of one needle is more than that needle's rule allows
/// there.
fn said_too_much(rule: &Rule, site: &Site) -> bool {
    rule.home != Some(site.file.as_str()) || site.times > rule.times
}

/// Every needle the door says fewer times than the rule asks it to.
fn said_less_often_than_the_rule_asks(counted: &[Site]) -> Vec<String> {
    NEEDLES
        .iter()
        .filter(|rule| {
            rule.home
                .is_some_and(|home| at_home(counted, rule, home) < rule.times)
        })
        .map(|rule| rule.names.to_owned())
        .collect()
}

/// How often `rule`'s needle is said in the file it is at home in.
fn at_home(counted: &[Site], rule: &Rule, home: &str) -> usize {
    counted
        .iter()
        .find(|site| site.file == home && site.names == rule.names)
        .map_or(0, |site| site.times)
}

fn walk(
    directory: &Path,
    crate_root: &Path,
    counted: &mut Vec<Site>,
    files_read: &mut usize,
) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(directory)?
        .map(|entry| entry.map(|found| found.path()))
        .collect::<Result<Vec<_>, _>>()?;
    // Read in a settled order, so the sites a failure prints are the same list
    // whatever order the filesystem hands its entries back in.
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, crate_root, counted, files_read)?;
        } else if is_production_source(&path) {
            read(&path, crate_root, counted, files_read)?;
        }
    }
    Ok(())
}

/// Reads one file and records how often it says each needle, ignoring the ones it
/// never says.
fn read(
    path: &Path,
    crate_root: &Path,
    counted: &mut Vec<Site>,
    files_read: &mut usize,
) -> Result<(), Box<dyn Error>> {
    let relative = relative_spelling(path, crate_root)?;
    let text = production_text(&fs::read_to_string(path)?);
    *files_read += 1;
    for rule in NEEDLES {
        let times = text.matches(rule.names).count();
        if times > 0 {
            counted.push(Site {
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
/// filter rather than a parse.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

/// A file's text with its doc comments removed.
///
/// A rustdoc example is a doc test, so prose describing a door is not a door. See
/// the header: this is exactly why a bare `rg` counts something else.
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
