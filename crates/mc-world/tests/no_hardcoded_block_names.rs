//! No Rust source names a block the base game ships.
//!
//! Invariant 1 in test form. The base game is a mod, so the engine may know the
//! *shape* of a block definition and nothing about any particular block; the
//! moment a name appears in Rust, the base game has a privilege a third-party mod
//! does not.
//!
//! The scan reads every `.rs` file under a member's `src/` except the sibling
//! `*_test.rs` unit files, and looks at its **production text**: the file minus
//! its doc comments. Both halves of that are deliberate. Unit tests live in
//! sibling files (`docs/technical/testing.md`), so skipping test code is a
//! file-name filter rather than a parse; a rustdoc example is a doc test, so it
//! is test code that does live in a production file, and dropping doc comments is
//! what lets `/// BlockName::parse("base:stone")` say the most natural thing.
//! Tests under `tests/` are not scanned at all — which is why this one may say
//! the names out loud.
//!
//! # Every member root, and each one accounted for separately
//!
//! Rust source is not only under `crates/`: `tools/voxforge` made `tools/` a
//! second member root, and for the length of that change this scan was **green
//! because it was not looking there**. [`MEMBER_ROOTS`] is where the roots are
//! stated.
//!
//! Widening the walk is half the fix. The other half is that a root's
//! contribution is counted *per root*, because the obvious guard — "the scan
//! read more than zero files" — is vacuous at the granularity that matters:
//! `crates/` alone contributes some three hundred files, so a root that
//! contributes none leaves the total healthy and the absence check green over a
//! tree nothing read. That is the same defect one level down, and it is the
//! defect this whole file exists to refuse.
//!
//! It scans for two lists. A name the base game *ships* must not be in the
//! engine; a name the base game has *retired* must not be anywhere, and that
//! second list exists because leaving `SHIPPED_NAMES` is otherwise how a name
//! stops being watched.

mod common;

use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use common::{TestResult, repository_root};
use tempfile::TempDir;

/// The directories holding one subdirectory per workspace member.
///
/// `crates/` is the engine; `tools/` is developer tooling. Both hold production
/// Rust, so both are scanned. A new root has to be stated here before anything
/// living in it is watched, which is why the guard below reports per root rather
/// than in total: a root nobody added is indistinguishable from a root that
/// happens to be empty, unless each is counted on its own.
const MEMBER_ROOTS: [&str; 2] = ["crates", "tools"];

/// The blocks this repository ships as content.
const SHIPPED_NAMES: [&str; 4] = ["base:stone", "base:dirt", "base:grass", "base:water"];

/// The block names the base game has retired.
///
/// A name that leaves [`SHIPPED_NAMES`] stops being watched, which is the one
/// way the invariant above quietly loosens: `base:air` was removed from the
/// content set precisely because a cell may now hold nothing, and after that
/// removal nothing mechanical stood between the engine and writing the name
/// again. An entry here never leaves.
const RETIRED_NAMES: [&str; 1] = ["base:air"];

/// What a scan of one directory tree found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    hits: Vec<String>,
    /// Every place a [`RETIRED_NAMES`] entry appears, kept apart from `hits`.
    ///
    /// Two lists rather than one, so that the absence check and its control each
    /// assert on the retired result alone. A single list would let a control
    /// naming a retired block pass on a shipped-name match, which is the reading
    /// the control exists to rule out.
    retired: Vec<String>,
    /// Every production file the exemption filter caused to be skipped.
    ///
    /// Recorded at the filter rather than read off [`EXEMPT_FILES`], and that is
    /// the whole point: a second exemption need not touch that constant at all —
    /// it can arrive as a new constant and a second clause in the filter — so a
    /// pin that compares the constant against itself would stay green through
    /// exactly the change it exists to catch. This measures what the scan
    /// *does*.
    exempted: Vec<PathBuf>,
}

/// The production files this scan does not read.
///
/// **This is not a weakening of invariant 1.** The invariant forbids hardcoded
/// block *definitions*; what these files do is *reference* a block in order to
/// place it, while texture and solidity still come only from
/// `content/base/blocks/*.toml`. Forbidding every mention was a free
/// over-approximation of the invariant right up until something legitimately
/// needed to reference a block, and this list is where that over-approximation
/// is recorded rather than where the rule is relaxed.
///
/// `crates/mc-sim/src/replay/world.rs` builds the scripted demo scene the
/// renderer is verified against: a fixed world of grass over dirt over stone,
/// with water to a declared sea level. It has to say which content-defined
/// blocks it places where, and the signature it is built behind
/// (`ReplayWorld::generate(seed, &BlockRegistry)`) carries no content root it
/// could read that choice out of.
///
/// **Delete an entry the day content can author what it needed.** For the
/// scripted scene the missing hook is content-authored worldgen, which is MVP
/// 2/3 work rather than PRO-852's; closing it deletes both the entry and
/// `the_exemption_skips_exactly_one_file_of_the_production_tree`. Until then
/// each exempt file is held by review instead of by this scan, which is exactly
/// what an entry costs — so that test pins how many files the filter skips, and
/// a second one has to be argued for rather than appear in a diff nobody reads.
///
/// Note that the pin is on the *filter's behaviour*, not on this constant: an
/// exemption need not be spelled as an entry here to take effect.
const EXEMPT_FILES: [&str; 1] = ["crates/mc-sim/src/replay/world.rs"];

/// A `.rs` file that is not a sibling unit-test file.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

/// Whether `path` is one of [`EXEMPT_FILES`].
///
/// Matched on the path's trailing components rather than on a substring, so the
/// answer depends on neither where the repository sits nor which separator the
/// platform writes — and so that an entry cannot accidentally match a directory
/// or a file of the same name somewhere else.
fn is_exempt(path: &Path) -> bool {
    let components: Vec<String> = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    EXEMPT_FILES.iter().any(|exempt| {
        let wanted: Vec<&str> = exempt.split('/').collect();
        components
            .windows(wanted.len())
            .any(|tail| tail.iter().map(String::as_str).eq(wanted.iter().copied()))
    })
}

/// A file's text with its doc comments removed.
///
/// Line doc comments are the only form this repository uses; a `/** */` block
/// would be read whole, which errs toward reporting rather than toward silence.
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

/// Reads every production Rust source under `root` and reports each place a
/// shipped or retired block name appears in one's production text.
///
/// One walk answering both lists, on purpose: a second scan would be a second
/// set of filters to keep in step, and the exemption and the test-file skip must
/// mean the same thing for both lists or the pins on them stop covering either.
fn scan_for_shipped_names(root: &Path) -> Result<Scan, Box<dyn Error>> {
    let mut scan = Scan::default();
    scan_directory(root, &mut scan)?;
    Ok(scan)
}

fn scan_directory(directory: &Path, scan: &mut Scan) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        scan_entry(&entry?.path(), scan)?;
    }
    Ok(())
}

/// One directory entry: recursed into, read, skipped as test code, or skipped as
/// exempt — and the last of those is recorded rather than silent, so that what
/// the exemption actually removes from the scan is observable.
fn scan_entry(path: &Path, scan: &mut Scan) -> Result<(), Box<dyn Error>> {
    if path.is_dir() {
        return scan_directory(path, scan);
    }
    if !is_production_source(path) {
        return Ok(());
    }
    if is_exempt(path) {
        scan.exempted.push(path.to_owned());
        return Ok(());
    }
    scan_file(path, scan)
}

fn scan_file(path: &Path, scan: &mut Scan) -> Result<(), Box<dyn Error>> {
    let text = production_text(&fs::read_to_string(path)?);
    scan.files_read += 1;
    for name in SHIPPED_NAMES {
        if text.contains(name) {
            scan.hits.push(format!("{} names `{name}`", path.display()));
        }
    }
    for name in RETIRED_NAMES {
        if text.contains(name) {
            scan.retired
                .push(format!("{} names `{name}`", path.display()));
        }
    }
    Ok(())
}

/// Every member's production source directory, kept under the root it came
/// from so that each root's contribution can be counted on its own.
///
/// # Errors
///
/// Returns the I/O failure when a root cannot be read — which is what a root
/// named in [`MEMBER_ROOTS`] but absent from the tree produces. `read_dir`
/// failing loudly is what keeps a mistyped root from narrowing this walk in
/// silence, unlike the gate's `-ErrorAction SilentlyContinue` walk of the same
/// two directories.
fn source_directories_by_root() -> Result<BTreeMap<&'static str, Vec<PathBuf>>, Box<dyn Error>> {
    let repository = repository_root()?;
    let mut by_root = BTreeMap::new();
    for member_root in MEMBER_ROOTS {
        by_root.insert(
            member_root,
            source_directories_under(&repository.join(member_root))?,
        );
    }
    Ok(by_root)
}

/// The `src/` directory of every member directly under `root`.
fn source_directories_under(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut directories = Vec::new();
    for entry in fs::read_dir(root)? {
        let sources = entry?.path().join("src");
        if sources.is_dir() {
            directories.push(sources);
        }
    }
    Ok(directories)
}

/// A scan of every member root's production tree, and how many production files
/// each root contributed to it.
#[derive(Debug, Default)]
struct ProductionScan {
    scan: Scan,
    read_per_root: BTreeMap<&'static str, usize>,
}

/// What a scan of the whole production tree amounts to.
///
/// A total verdict rather than a pair of assertions a caller has to remember to
/// write in the right order: "nothing named it" and "a root went unread" are
/// different facts, and only the first is good news. An `is_empty()` on the hit
/// list cannot tell them apart.
#[derive(Debug, PartialEq, Eq)]
enum TreeVerdict {
    /// Every member root contributed production source, and none of it named a
    /// watched block.
    NothingNamed,
    /// These member roots contributed no production source at all, so whatever
    /// lives under them went unscanned.
    ReadNothingUnder(Vec<&'static str>),
    /// Every place a watched block name appears.
    Named(Vec<String>),
}

fn scan_the_production_tree() -> Result<ProductionScan, Box<dyn Error>> {
    let mut production = ProductionScan::default();
    for (root, directories) in source_directories_by_root()? {
        let read = accumulate_scans(&directories, &mut production.scan)?;
        production.read_per_root.insert(root, read);
    }
    Ok(production)
}

/// Scans each of `directories` into `scan`, returning how many production files
/// this call added to it.
fn accumulate_scans(directories: &[PathBuf], scan: &mut Scan) -> Result<usize, Box<dyn Error>> {
    let before = scan.files_read;
    for directory in directories {
        let found = scan_for_shipped_names(directory)?;
        scan.files_read += found.files_read;
        scan.hits.extend(found.hits);
        scan.retired.extend(found.retired);
        scan.exempted.extend(found.exempted);
    }
    Ok(scan.files_read - before)
}

/// The verdict `production` amounts to for one of its hit lists.
///
/// The unread-root check comes first because it explains away any answer that
/// follows it: a hit list is only evidence about the trees that were read.
fn tree_verdict(production: &ProductionScan, hits: &[String]) -> TreeVerdict {
    let unread: Vec<&'static str> = production
        .read_per_root
        .iter()
        .filter(|(_, read)| **read == 0)
        .map(|(root, _)| *root)
        .collect();
    if !unread.is_empty() {
        return TreeVerdict::ReadNothingUnder(unread);
    }
    if hits.is_empty() {
        return TreeVerdict::NothingNamed;
    }
    TreeVerdict::Named(hits.to_vec())
}

/// A scan of the given files, written into a temporary directory.
fn scan_of(files: &[(&str, &str)]) -> Result<(TempDir, Scan), Box<dyn Error>> {
    let directory = TempDir::new()?;
    for (file_name, source) in files {
        fs::write(directory.path().join(file_name), source)?;
    }
    let scanned = scan_for_shipped_names(directory.path())?;
    Ok((directory, scanned))
}

#[test]
fn no_production_rust_source_names_a_block_the_base_game_ships() -> TestResult {
    let production = scan_the_production_tree()?;

    assert_eq!(
        tree_verdict(&production, &production.scan.hits),
        TreeVerdict::NothingNamed,
        "a block's name belongs to content, never to the engine. The verdict names the member \
         roots it read nothing under, so a tree this scan stopped looking at reports as a refusal \
         rather than as the same clean answer an obedient engine gives"
    );
    Ok(())
}

/// A guard rather than a scenario, and the reason the check above cannot go
/// quiet. A scan whose directory walk or whose matcher broke would report nothing
/// forever — including on the day the invariant it guards is actually violated.
/// The fixture is nested one directory deep on purpose: a walk that stopped at
/// the top level would otherwise still look healthy here.
#[test]
fn the_scan_reports_a_source_that_does_name_a_block_the_base_game_ships() -> TestResult {
    let directory = TempDir::new()?;
    let nested = directory.path().join("nested");
    fs::create_dir_all(&nested)?;
    fs::write(
        nested.join("blocks.rs"),
        "const FILL: &str = \"base:grass\";\n",
    )?;

    let scanned = scan_for_shipped_names(directory.path())?;

    assert!(
        !scanned.hits.is_empty(),
        "a source that does name a shipped block must be reported, or this scan proves nothing"
    );
    Ok(())
}

/// The same invariant read backwards, and the reason retiring a name is not the
/// same as keeping watch on it. `base:air` left `SHIPPED_NAMES` the day the base
/// game stopped declaring it, and a name in neither list is a name nothing at all
/// would notice returning to production Rust.
///
/// Three limitations are inherited from the scan rather than lifted here, and
/// each is deliberate: the exemption applies to retired names too, doc comments
/// are stripped before matching, and `tests/` is not read at all — which is what
/// lets `block_semantics.rs` declare a *solid* block named `base:air` on purpose
/// without contradicting this.
///
/// This asserts an absence, so it would go green forever the day the scan stopped
/// working. The test below it is its control.
#[test]
fn no_production_rust_source_names_a_block_the_base_game_has_retired() -> TestResult {
    let production = scan_the_production_tree()?;

    assert_eq!(
        tree_verdict(&production, &production.scan.retired),
        TreeVerdict::NothingNamed,
        "a name the base game has retired means nothing to the engine and nothing to content, so \
         it belongs in no production source — and a member root that went unread is not evidence \
         that no such name lives under it"
    );
    Ok(())
}

/// The control for the check above, and what a scan whose retired-name match
/// never fires looks like: nothing reported, forever, including on the day the
/// retired name comes back.
///
/// The fixture names a retired block and **no shipped one**, which is the whole
/// separation the second hit list buys. A `retired` list quietly fed by the
/// shipped-name matcher finds nothing in this file; a control that also named a
/// shipped block would be green either way and prove nothing about the retired
/// match. That constraint is held by the fixture — no assertion can enforce it —
/// so it is stated here, and the assertion below deliberately says nothing at all
/// about the shipped-name result.
#[test]
fn the_scan_reports_a_source_that_does_name_a_block_the_base_game_has_retired() -> TestResult {
    let (_directory, scanned) = scan_of(&[("sky.rs", "const SKY: &str = \"base:air\";\n")])?;

    assert!(
        scanned
            .retired
            .iter()
            .any(|hit| hit.contains("sky.rs") && hit.contains("base:air")),
        "a source that does name a retired block must be reported, and reported by file and by \
         name, or the check above proves nothing: {:?}",
        scanned.retired
    );
    Ok(())
}

/// The second half of that guard: the file-name filter must skip test code and
/// *nothing else*. A filter that had drifted into skipping too much — matching
/// `test` anywhere in the name, say — would leave the real check above green
/// while scanning almost nothing. So the fixture puts a shipped name in a sibling
/// unit-test file and another in the module it tests.
#[test]
fn a_name_in_a_sibling_unit_test_file_is_skipped_and_one_beside_it_is_still_found() -> TestResult {
    let (_directory, scanned) = scan_of(&[
        (
            "blocks_test.rs",
            "const NAMED_IN_A_TEST: &str = \"base:dirt\";\n",
        ),
        (
            "blocks.rs",
            "const NAMED_IN_PRODUCTION: &str = \"base:stone\";\n",
        ),
    ])?;

    assert!(
        scanned.hits.len() == 1 && scanned.hits.join(" ").contains("base:stone"),
        "the sibling file is test code and must be skipped; the module beside it is production \
         source and must still be found. Exactly one hit, and it is the second: {:?}",
        scanned.hits
    );
    Ok(())
}

/// The exemption is pinned by what it *removes from the real scan*, never by
/// what a constant says.
///
/// An absence assertion with an escape hatch goes green forever the day somebody
/// widens the hatch, so the hatch needs a pin — but the obvious pin is the one
/// that cannot work. Comparing [`EXEMPT_FILES`] against a copy of itself catches
/// only the exemption that arrives as an entry in *that* constant; one that
/// arrives as a second constant and a second clause in the filter leaves it
/// untouched and green. That is this project's recurring defect in miniature:
/// green that could not have been red.
///
/// So this walks the production tree the real check walks and asserts the set of
/// files the filter actually skipped. A second exemption has to skip some real
/// file to have any effect at all, and skipping one turns this red however it was
/// spelled. It is self-guarding in the other direction too: a walk that resolved
/// nothing skips nothing, and an empty set is not the expected one.
#[test]
fn the_exemption_skips_exactly_one_file_of_the_production_tree() -> TestResult {
    let root = repository_root()?;
    let mut skipped: Vec<String> = scan_the_production_tree()?
        .scan
        .exempted
        .iter()
        .map(|path| {
            path.strip_prefix(&root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    skipped.sort();

    assert_eq!(
        skipped,
        ["crates/mc-sim/src/replay/world.rs"],
        "every skipped file is a place invariant 1 is held by review instead of by this \
         scan, and this is the count of them. Adding one is a decision, not an edit: say in \
         the commit which content hook is missing and what deleting it again will depend on"
    );
    Ok(())
}

/// The control for the exemption, and the reason it cannot quietly grow. An
/// exemption matched too loosely — on `mc-sim`, on `replay`, on any file called
/// `world.rs` — would leave the real check above green while no longer reading a
/// tree it is supposed to read. So the fixture puts a shipped name in the exempt
/// file and another in its sibling, at the same depth, and exactly one is
/// reported.
#[test]
fn a_name_in_the_scripted_scene_is_skipped_and_one_in_the_module_beside_it_is_not() -> TestResult {
    let directory = TempDir::new()?;
    let replay = directory.path().join("crates/mc-sim/src/replay");
    fs::create_dir_all(&replay)?;
    fs::write(
        replay.join("world.rs"),
        "const SURFACE: &str = \"base:grass\";\n",
    )?;
    fs::write(
        replay.join("height.rs"),
        "const DEPTHS: &str = \"base:stone\";\n",
    )?;

    let scanned = scan_for_shipped_names(directory.path())?;

    assert!(
        scanned.hits.len() == 1 && scanned.hits.join(" ").contains("base:stone"),
        "the scripted scene is the one exempt file; the module beside it is not, and neither \
         is any other `world.rs`. Exactly one hit, and it is the second: {:?}",
        scanned.hits
    );
    Ok(())
}

/// Why a rustdoc example may say `base:stone` out loud. A doc example is a doc
/// test — test code that happens to live in a production file — and the most
/// natural example for a namespaced-id type is the real namespace. This asserts
/// the scan agrees, so the rule stays a decision rather than a trap someone
/// rediscovers by turning the suite red.
#[test]
fn a_name_in_a_doc_example_is_not_a_hardcoded_block_name() -> TestResult {
    let (_directory, scanned) = scan_of(&[(
        "names.rs",
        concat!(
            "/// ```\n",
            "/// let name = BlockName::parse(\"base:water\")?;\n",
            "/// ```\n",
            "pub fn parse_a_name() {}\n",
        ),
    )])?;

    assert!(
        scanned.hits.is_empty(),
        "a doc example is a doc test, so naming a shipped block in one is not the engine knowing \
         about it: {:?}",
        scanned.hits
    );
    Ok(())
}
