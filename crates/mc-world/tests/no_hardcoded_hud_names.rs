//! No Rust source names a HUD element, or a HUD colour, the base game declares.
//!
//! Invariant 1 in test form, for the HUD. A crosshair hardcoded in Rust is the
//! same violation as a hardcoded block: the moment a name or a colour the base
//! game declares appears in the engine, the base game has a privilege a
//! third-party mod does not.
//!
//! # The watch list is derived, not copied
//!
//! This is the one thing it does differently from its neighbour
//! `no_hardcoded_block_names.rs`, and the difference is the point. That scan
//! carries a hand-written list of shipped names, so deleting a content file
//! silently stops it watching what that file declared — which is exactly why it
//! needed a second, *retired* list to plug the hole. Here the list is read out
//! of `content/base/` at test time, through the same source the client reads a
//! content root with, so a declaration cannot leave the directory and stay
//! unwatched: it stops being watched only by ceasing to exist.
//!
//! A hand-maintained retired list still covers the remaining case, which
//! deriving cannot: an element **renamed**, whose old spelling means nothing to
//! content any more and must mean nothing to the engine either. An entry there
//! never leaves.
//!
//! # What is watched, and what is deliberately not
//!
//! Every element name the directory declares; every colour those declarations
//! state, in the `#RRGGBBAA` spelling they are written in and as a byte
//! quadruple in both spacings a Rust literal takes. Colour spellings are derived
//! by formatting the parsed bytes, so the hex needle is upper case — which is
//! how the shipped declarations are written. A lower-case hex literal in Rust
//! would not be caught, and that is recorded here rather than papered over: the
//! byte quadruples catch the spelling a Rust constant is far likelier to take.
//!
//! **The nine anchor names are deliberately not watched.** They are the engine's
//! own vocabulary and must appear in Rust — the module that defines them would
//! need an exemption, and an exemption is precisely the thing this guard exists
//! to avoid needing.
//!
//! # An absence proves nothing on its own
//!
//! Two ways this check can go quiet, and each is its own scenario rather than an
//! assertion tucked inside the first: a scan that read no source reports no
//! occurrence for reasons that have nothing to do with the engine, and a watch
//! list that derived nothing reports no occurrence because it is looking for
//! nothing. Both are refusals here, not passes. Their positive counterparts are
//! the two controls: a fixture naming a declared element, and one writing a
//! declared colour as its bytes.
//!
//! There is a third way, and it is the one that actually happened. Rust source
//! is not only under `crates/`: `tools/voxforge` made `tools/` a second member
//! root, and for the length of that change this scan was **green because it was
//! not looking there**. [`MEMBER_ROOTS`] is where the roots are stated, and
//! [`Verdict::ReadNothingUnder`] is the refusal for a root that contributed no
//! source — asked per root, because "the scan read more than zero files" is
//! vacuous when `crates/` alone contributes some three hundred of them and keeps
//! the total healthy over a tree nothing read.
//!
//! The scan reads production text — a file minus its doc comments — under each
//! member's `src/`, skipping sibling `*_test.rs` unit files. A rustdoc example is
//! a doc test, so naming a declared element in one is not the engine knowing
//! about it. Files under `tests/` are not read at all, which is what lets this
//! one say the names out loud.

mod common;

use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use common::{TestResult, repository_root};
use mc_core::hud::{Draw, HudLayout, Rgba8};
use mc_world::content::TomlFileHudSource;
use tempfile::TempDir;

/// The directories holding one subdirectory per workspace member.
///
/// `crates/` is the engine; `tools/` is developer tooling. Both hold production
/// Rust, so both are scanned, and each is accounted for on its own — see
/// [`Verdict::ReadNothingUnder`].
const MEMBER_ROOTS: [&str; 2] = ["crates", "tools"];

/// The HUD element names the base game has retired.
///
/// Empty today, because nothing has been renamed yet — and empty is the honest
/// state rather than a placeholder: a name only belongs here once it has meant
/// something to content and stopped. Until one does, this half of the check
/// below asserts nothing, which is stated here so a reader does not take its
/// green for evidence.
const RETIRED_NAMES: [&str; 0] = [];

/// The spellings a production source may not contain.
///
/// Names and colours are kept apart rather than flattened into one list of
/// needles, so that a control about a colour cannot pass on a name matching and
/// so that "the list derived nothing" can be asked of each.
#[derive(Debug, Default)]
struct WatchList {
    /// Every element name the directory declares, in declaration order.
    names: Vec<String>,
    /// Every colour those declarations state, in declaration order and without
    /// repeats — the two crossing bars of a crosshair state the same two.
    colours: Vec<Rgba8>,
}

impl WatchList {
    /// Every spelling the scan looks for.
    fn needles(&self) -> Vec<String> {
        let mut needles = self.names.clone();
        needles.extend(RETIRED_NAMES.iter().map(|name| (*name).to_owned()));
        for colour in &self.colours {
            needles.push(hex_spelling(*colour));
            needles.push(spaced_bytes(*colour));
            needles.push(packed_bytes(*colour));
        }
        needles
    }

    /// Whether the derivation found anything to watch at all.
    fn watches_nothing(&self) -> bool {
        self.names.is_empty() && self.colours.is_empty()
    }
}

/// A colour as a declaration spells it.
fn hex_spelling(colour: Rgba8) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        colour.r, colour.g, colour.b, colour.a
    )
}

/// A colour as a Rust byte literal is usually written.
fn spaced_bytes(colour: Rgba8) -> String {
    format!("[{}, {}, {}, {}]", colour.r, colour.g, colour.b, colour.a)
}

/// The same literal written without its spaces.
fn packed_bytes(colour: Rgba8) -> String {
    format!("[{},{},{},{}]", colour.r, colour.g, colour.b, colour.a)
}

/// What a scan of one directory tree found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    hits: Vec<String>,
}

/// What a scan amounts to, given what it was looking for.
///
/// A verdict rather than a pair of booleans a caller has to remember to check:
/// "found nothing", "read nothing" and "was looking for nothing" are three
/// different facts, and only the first is good news.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Sources were read, a list was watched, and no source named anything on
    /// it.
    NothingNamed,
    /// The derivation found no name and no colour, so the scan could not have
    /// reported one.
    WatchedNothing,
    /// No production source was read at all.
    ReadNothing,
    /// These member roots contributed no production source, so whatever lives
    /// under them went unscanned while the others kept the total healthy.
    ReadNothingUnder(Vec<&'static str>),
    /// Every place a watched spelling appears.
    Named(Vec<String>),
}

/// The verdict `scanned` amounts to for `watched`.
fn verdict(watched: &WatchList, scanned: &Scan) -> Verdict {
    if watched.watches_nothing() {
        return Verdict::WatchedNothing;
    }
    if scanned.files_read == 0 {
        return Verdict::ReadNothing;
    }
    if scanned.hits.is_empty() {
        return Verdict::NothingNamed;
    }
    Verdict::Named(scanned.hits.clone())
}

#[test]
fn no_production_rust_source_names_a_hud_element_or_colour_the_base_game_declares() -> TestResult {
    let watched = watch_list_of(&shipped_content_root()?)?;
    let production = scan_of_production_sources(&watched.needles())?;

    assert_eq!(
        production_verdict(&watched, &production),
        Verdict::NothingNamed,
        "a HUD element's name and its colours belong to content, never to the engine: the base \
         game is a mod, and a crosshair the engine knows by name or by colour is a privilege no \
         third-party mod has. The verdict names any member root it read nothing under, so a tree \
         this scan stopped looking at cannot report the same clean answer an obedient engine does"
    );
    Ok(())
}

#[test]
fn the_same_scan_reports_a_source_that_names_a_declared_hud_element() -> TestResult {
    let watched = watch_list_of(&shipped_content_root()?)?;
    let declared = watched.names.first().ok_or(
        "the base game has to declare a HUD element for this control to mean anything: a scan \
         watching no name reports nothing whatever a source says",
    )?;

    let (_fixture, scanned) = scan_of(
        &watched.needles(),
        "centre.rs",
        &format!("const CENTRE: &str = \"{declared}\";\n"),
    )?;

    assert_eq!(
        (
            scanned.files_read,
            scanned.hits.len(),
            scanned.hits.iter().any(|hit| hit.contains(declared))
        ),
        (1, 1, true),
        "a source that does name a declared element must be reported, and reported by the name \
         it took, or the check above proves nothing: {:?}",
        scanned.hits
    );
    Ok(())
}

#[test]
fn the_same_scan_reports_a_source_that_writes_a_declared_colour_as_its_bytes() -> TestResult {
    let watched = watch_list_of(&shipped_content_root()?)?;
    let declared = *watched.colours.first().ok_or(
        "the base game has to declare a HUD colour for this control to mean anything: a scan \
         watching no colour reports nothing whatever a source says",
    )?;
    let as_bytes = spaced_bytes(declared);

    let (_fixture, scanned) = scan_of(
        &watched.needles(),
        "palette.rs",
        &format!("const CROSSHAIR: [u8; 4] = {as_bytes};\n"),
    )?;

    assert_eq!(
        (
            scanned.files_read,
            scanned.hits.len(),
            scanned.hits.iter().any(|hit| hit.contains(&as_bytes))
        ),
        (1, 1, true),
        "a colour reaches Rust as four bytes far more readily than as the hex a declaration \
         spells it in, so the byte spelling is the one that has to be watched — and watched \
         means reported: {:?}",
        scanned.hits
    );
    Ok(())
}

#[test]
fn a_scan_that_read_no_production_source_refuses_rather_than_reporting_no_occurrences() -> TestResult
{
    let declaring = a_root_declaring_one_element()?;
    let watched = watch_list_of(declaring.path())?;
    let empty = TempDir::new()?;

    let scanned = scan(empty.path(), &watched.needles())?;

    assert_eq!(
        (
            scanned.files_read,
            scanned.hits.len(),
            verdict(&watched, &scanned)
        ),
        (0, 0, Verdict::ReadNothing),
        "a scan that read no source names no element for a reason that has nothing to do with \
         the engine: the crates moved, or the walk broke. Reporting no occurrence there is how \
         an absence check goes green forever"
    );
    Ok(())
}

#[test]
fn a_watch_list_that_derived_nothing_refuses_rather_than_passing() -> TestResult {
    let declaring_nothing = a_root_declaring_no_hud()?;
    let watched = watch_list_of(declaring_nothing.path())?;

    let (_fixture, scanned) = scan_of(
        &watched.needles(),
        "crosshair.rs",
        "const CENTRE: &str = \"base:crosshair-horizontal\";\n",
    )?;

    assert_eq!(
        (scanned.hits.len(), verdict(&watched, &scanned)),
        (0, Verdict::WatchedNothing),
        "the fixture this scanned does name a crosshair, and a watch list derived from a content \
         root that declares none finds it clean — which is the assertion this check can no \
         longer falsify, and the reason an empty list is a refusal rather than a pass"
    );
    Ok(())
}

#[test]
fn a_declaration_added_under_the_content_directory_is_watched_with_no_rust_edit() -> TestResult {
    let shipped = watch_list_of(&shipped_content_root()?)?;
    let fixture = TempDir::new()?;
    let root = shipped_declarations_and_one_more(&fixture, ADDED_FILE, ADDED_NAME)?;

    let derived = watch_list_of(&root)?;

    assert_eq!(
        (
            shipped.names.is_empty(),
            derived.names.iter().any(|name| name == ADDED_NAME),
            shipped
                .names
                .iter()
                .all(|shipped| derived.names.contains(shipped))
        ),
        (false, true, true),
        "a content author who adds a declaration is watched by this scan the moment the file \
         exists, and the ones already there stay watched — no Rust edit, no list to remember. \
         Shipped: {:?}, derived: {:?}",
        shipped.names,
        derived.names
    );
    Ok(())
}

/// The element name the fixture above adds.
///
/// An `example:` namespace: a fixture borrowing the `base:` one would be a test
/// pretending to be the content it is watching.
const ADDED_NAME: &str = "example:extra-readout";

/// The file it is declared in, sorting after every shipped declaration so the
/// fixture cannot pass by being read first.
const ADDED_FILE: &str = "zz-extra-readout.toml";

/// Where the repository's own content lives.
fn shipped_content_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(repository_root()?.join("content").join("base"))
}

/// Every element name and colour the HUD declarations under `root` state.
///
/// Read through the same source the client reads a content root with, so what is
/// watched and what the game loads are one reading rather than two.
///
/// # Errors
///
/// Returns the refusal when the root's declarations do not load: a watch list
/// derived from a root that was refused would silently be empty.
fn watch_list_of(root: &Path) -> Result<WatchList, Box<dyn Error>> {
    let layout = HudLayout::load(&TomlFileHudSource::new(root))?;
    let mut watched = WatchList::default();
    for element in layout.elements() {
        watched.names.push(element.name.as_str().to_owned());
        if let Draw::Fill { color } = element.draw {
            remember(&mut watched.colours, color);
        }
        if let Some(outline) = element.outline {
            remember(&mut watched.colours, outline);
        }
    }
    Ok(watched)
}

/// Adds `colour` unless it is already watched, keeping declaration order.
fn remember(colours: &mut Vec<Rgba8>, colour: Rgba8) {
    if !colours.contains(&colour) {
        colours.push(colour);
    }
}

/// A content root declaring exactly one HUD element, for the scenarios that need
/// a watch list holding something without depending on what the base game ships.
fn a_root_declaring_one_element() -> Result<TempDir, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let declared = directory.path().join("hud");
    fs::create_dir_all(&declared)?;
    fs::write(
        declared.join("readout.toml"),
        declaration_of("example:readout"),
    )?;
    Ok(directory)
}

/// A content root with no `hud/` directory at all, which loads as no elements.
fn a_root_declaring_no_hud() -> Result<TempDir, Box<dyn Error>> {
    Ok(TempDir::new()?)
}

/// Every shipped HUD declaration, copied into `fixture`, with one more beside
/// them.
///
/// A copy rather than the shipped directory itself: what the scenario asks is
/// what a *fourth* file would do, and answering it by writing into
/// `content/base/` would leave the repository holding a fixture.
fn shipped_declarations_and_one_more(
    fixture: &TempDir,
    file_name: &str,
    declares: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let root = fixture.path().to_owned();
    let declared = root.join("hud");
    fs::create_dir_all(&declared)?;
    for shipped in shipped_declaration_files()? {
        let Some(name) = shipped.file_name() else {
            continue;
        };
        fs::copy(&shipped, declared.join(name))?;
    }
    fs::write(declared.join(file_name), declaration_of(declares))?;
    Ok(root)
}

/// Every declaration file the base game ships, or none where it ships no HUD
/// directory at all.
fn shipped_declaration_files() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let shipped = shipped_content_root()?.join("hud");
    if !shipped.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&shipped)? {
        let path = entry?.path();
        if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

/// A well-formed declaration naming `element` — a centred white fill, the
/// smallest the model accepts.
fn declaration_of(element: &str) -> String {
    format!(
        "name = \"{element}\"\nanchor = \"center\"\nsize = [9, 1]\ndraw = \"fill\"\n\
         color = \"#FFFFFFFF\"\n"
    )
}

/// A scan of one source file written into a temporary directory, nested a
/// directory deep so a walk that stopped at the top level is caught here.
fn scan_of(
    needles: &[String],
    file_name: &str,
    source: &str,
) -> Result<(TempDir, Scan), Box<dyn Error>> {
    let directory = TempDir::new()?;
    let nested = directory.path().join("nested");
    fs::create_dir_all(&nested)?;
    fs::write(nested.join(file_name), source)?;
    let scanned = scan(directory.path(), needles)?;
    Ok((directory, scanned))
}

/// A scan of every crate's production sources.
/// A scan of every member root's production sources, and how many production
/// files each root contributed to it.
#[derive(Debug, Default)]
struct ProductionScan {
    scanned: Scan,
    read_per_root: BTreeMap<&'static str, usize>,
}

/// # Errors
///
/// Returns the I/O failure when a root cannot be read — which is what a root
/// named in [`MEMBER_ROOTS`] but absent from the tree produces. `read_dir`
/// failing loudly is what keeps a mistyped root from narrowing this walk in
/// silence, unlike the gate's `-ErrorAction SilentlyContinue` walk of the same
/// two directories.
fn scan_of_production_sources(needles: &[String]) -> Result<ProductionScan, Box<dyn Error>> {
    let repository = repository_root()?;
    let mut production = ProductionScan::default();
    for member_root in MEMBER_ROOTS {
        let read = scan_members_under(&repository.join(member_root), needles, &mut production)?;
        production.read_per_root.insert(member_root, read);
    }
    Ok(production)
}

/// Scans the `src/` of every member directly under `root`, returning how many
/// production files this call added.
fn scan_members_under(
    root: &Path,
    needles: &[String],
    production: &mut ProductionScan,
) -> Result<usize, Box<dyn Error>> {
    let before = production.scanned.files_read;
    for entry in fs::read_dir(root)? {
        let sources = entry?.path().join("src");
        if sources.is_dir() {
            let found = scan(&sources, needles)?;
            production.scanned.files_read += found.files_read;
            production.scanned.hits.extend(found.hits);
        }
    }
    Ok(production.scanned.files_read - before)
}

/// The verdict the real production tree amounts to for `watched`.
///
/// The unread-root check comes first because it explains away any answer that
/// follows it: a hit list is only evidence about the trees that were read.
fn production_verdict(watched: &WatchList, production: &ProductionScan) -> Verdict {
    let unread: Vec<&'static str> = production
        .read_per_root
        .iter()
        .filter(|(_, read)| **read == 0)
        .map(|(root, _)| *root)
        .collect();
    if !unread.is_empty() {
        return Verdict::ReadNothingUnder(unread);
    }
    verdict(watched, &production.scanned)
}

/// Reads every production Rust source under `root` and reports each place a
/// watched spelling appears.
fn scan(root: &Path, needles: &[String]) -> Result<Scan, Box<dyn Error>> {
    let mut scanned = Scan::default();
    walk(root, needles, &mut scanned)?;
    Ok(scanned)
}

fn walk(directory: &Path, needles: &[String], scanned: &mut Scan) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, needles, scanned)?;
        } else if is_production_source(&path) {
            read(&path, needles, scanned)?;
        }
    }
    Ok(())
}

fn read(path: &Path, needles: &[String], scanned: &mut Scan) -> Result<(), Box<dyn Error>> {
    let text = production_text(&fs::read_to_string(path)?);
    scanned.files_read += 1;
    for needle in needles {
        if text.contains(needle.as_str()) {
            scanned
                .hits
                .push(format!("{} names `{needle}`", path.display()));
        }
    }
    Ok(())
}

/// A `.rs` file that is not a sibling unit-test file.
fn is_production_source(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| file_name.ends_with(".rs") && !file_name.ends_with("_test.rs"))
}

/// A file's text with its doc comments removed.
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
