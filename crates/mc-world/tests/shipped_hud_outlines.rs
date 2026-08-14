//! Every HUD element the base game ships states a contrast outline.
//!
//! `ui-design.md` §2 is titled non-negotiable and this is the requirement it
//! states: the world behind the HUD is any colour, any brightness, and it moves,
//! so no element may sit on it untreated. A white crosshair has to survive snow
//! *and* an unlit cave. This is a product rule, not a style preference, which is
//! why it is asserted rather than noted.
//!
//! # Why this is a content assertion and not a frame assertion
//!
//! Measured: removing `outline` from `held-block.toml` leaves **all 651 tests
//! green**. Every footprint assertion the render and client suites make is a
//! frame-to-frame *equality*, and a missing ring satisfies it on both sides — the
//! comparison shifts with the thing it compares. A predicted footprint does not
//! close it either where the prediction is derived from the declaration: drop the
//! outline and the prediction says 24 × 24, the frame draws 24 × 24, and the two
//! agree. A prediction that follows its input is not an oracle.
//!
//! So the only thing that can grade this is the declaration itself.
//!
//! # Why the declarations are parsed and not searched
//!
//! Two of the three shipped files mention `outline` **in a prose comment** as
//! well as in the field, and the mutation this test exists to catch blanks the
//! field and leaves the comment. A scan for the string `outline` stays green
//! under precisely that mutation. What is read here is the parsed element's
//! `outline` field, through the same source the client reads a content root with,
//! because an `Option` is the only thing that can tell a stated field from a
//! sentence about one.
//!
//! # Why the element list is derived
//!
//! Read out of the directory at test time rather than written down here, so a
//! fourth shipped element is covered with no Rust edit. What pins *which* three
//! elements ship is a separate scenario's business; this pins a property every
//! shipped element must have, however many there are.
//!
//! # An absence proves nothing on its own
//!
//! "No element states no outline" is true of a directory holding no elements, so
//! the day `content/base/hud/` is emptied, renamed or moved this check would go
//! green forever and report that every element is treated. It cannot: reading no
//! declaration file, and deriving no element from the ones read, are two distinct
//! refusals of their own rather than assertions tucked inside the first, and
//! neither can arrive under the good verdict's name.
//!
//! # And neither does a verdict of "all treated", on its own
//!
//! Those two refusals answer *the directory vanished*. They say nothing about
//! *the scan stopped being able to detect an untreated element* — and if the
//! search for untreated elements came to report none unconditionally, the shipped
//! reading above would answer "every element is treated" forever while the
//! mutation it exists to catch reddened nothing at all. A verdict of all-treated
//! cannot be told apart from a scan that can no longer say otherwise, however
//! many arms the verdict has: they look mutually covering and are not.
//!
//! So the same scan is run a second time over an element that **must** be
//! reported, and its report is read. That element is a **fixture**, written under
//! a temporary root — a control that reached its verdict by editing
//! `content/base/` would be a test that can only pass while the product is
//! broken.

mod common;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use common::hud::{declared_by, hud_content_root, hud_file};
use common::{TestResult, repository_root};
use mc_core::hud::{HudElement, HudLayout, HudOrigin};
use mc_world::content::TomlFileHudSource;
use tempfile::TempDir;

/// What reading one content root's HUD declarations found.
#[derive(Debug, Default)]
struct Scanned {
    /// How many declaration files the directory holds.
    ///
    /// Counted rather than inferred from the elements below: "the directory is
    /// gone" and "the loader stopped registering what is in it" are different
    /// facts, and a single count cannot say which happened.
    declaration_files: usize,
    /// Every element those files declare, in the order they load.
    elements: Vec<HudElement>,
}

impl Scanned {
    /// Every element that states no outline, named with the file that stated it.
    ///
    /// Named with its origin because a bare "not every element states an
    /// outline" over a derived list of unknown length is a failure nobody can
    /// act on.
    fn untreated(&self) -> Vec<String> {
        self.elements
            .iter()
            .filter(|element| element.outline.is_none())
            .map(|element| {
                format!(
                    "`{}` states no outline, declared by {}",
                    element.name.as_str(),
                    file_named_by(&element.origin)
                )
            })
            .collect()
    }
}

/// The name of the file `origin` points at, or the label as it stands where it
/// names no file.
///
/// The whole path is deliberately not quoted back. A fixture root is a temporary
/// directory whose absolute path differs per run, so a report carrying one could
/// not be asserted on at all — and this suite compares an origin by the name of
/// the file or directory it points at throughout, for the same reason plus the
/// OS-specific separators a path renders with.
fn file_named_by(origin: &HudOrigin) -> String {
    Path::new(origin.as_str())
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_else(|| origin.as_str())
        .to_owned()
}

/// What a reading of a root's declarations amounts to.
///
/// A verdict rather than a bare list of offenders, because "every element is
/// treated", "there was nothing to read" and "nothing was declared by what was
/// read" are three different facts and only the first is good news.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Declarations were read, elements came out of them, and every one states
    /// an outline.
    EveryElementStatesAnOutline,
    /// The declaration directory holds no file to read at all.
    NoDeclarationFileWasRead,
    /// Files were read and declared no element between them.
    NoElementWasDeclared,
    /// Every element sitting on the world untreated.
    Untreated(Vec<String>),
}

/// The verdict `scanned` amounts to.
fn verdict(scanned: &Scanned) -> Verdict {
    if scanned.declaration_files == 0 {
        return Verdict::NoDeclarationFileWasRead;
    }
    if scanned.elements.is_empty() {
        return Verdict::NoElementWasDeclared;
    }
    let untreated = scanned.untreated();
    if untreated.is_empty() {
        return Verdict::EveryElementStatesAnOutline;
    }
    Verdict::Untreated(untreated)
}

#[test]
fn every_hud_element_the_base_game_ships_states_a_contrast_outline() -> TestResult {
    let shipped = scan(&shipped_content_root()?)?;

    assert_eq!(
        verdict(&shipped),
        Verdict::EveryElementStatesAnOutline,
        "the world behind a HUD element is any colour, any brightness, and it moves, so an element \
         shipped without an outline is one that vanishes against some terrain — and no rendered \
         frame can say so, because every footprint assertion compares one frame against another \
         and a missing ring shifts both sides equally"
    );
    Ok(())
}

/// The file stem, and so the element, the control's fixture declares.
///
/// Distinctive on purpose: it is a name no shipped file could carry, so a reader
/// meeting it in a failure message cannot mistake this control's element for one
/// of the base game's.
const UNTREATED_FIXTURE: &str = "no-outline-fixture";

#[test]
fn an_element_stating_no_outline_is_reported_by_name_with_the_file_that_declared_it() -> TestResult
{
    let directory = TempDir::new()?;
    // The fixture is untreated because `hud_file` writes every field the model
    // requires and none it does not, and `outline` is one it does not — a fixture
    // constraint no assertion below can enforce. Were that builder to start
    // stating an outline, this test would redden rather than quietly pass, which
    // is the direction that failure has to fall in.
    let root = hud_content_root(
        &directory,
        &[(
            &format!("{UNTREATED_FIXTURE}.toml"),
            hud_file(&declared_by(UNTREATED_FIXTURE)),
        )],
    )?;

    assert_eq!(
        verdict(&scan(&root)?),
        Verdict::Untreated(vec![
            "`base:no-outline-fixture` states no outline, declared by no-outline-fixture.toml"
                .to_owned()
        ]),
        "an element that states no outline has to come back reported, and the report has to name \
         the element and the file that declared it — because the shipped reading's good verdict is \
         indistinguishable from a scan that has stopped being able to find an untreated element, \
         and this is the only test that can tell those two apart"
    );
    Ok(())
}

/// Where the base game's own content lives.
fn shipped_content_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(repository_root()?.join("content").join("base"))
}

/// The HUD declarations under `root`, and what they declare.
///
/// Loaded through the same source the client reads a content root with, so what
/// is graded here and what the game shows are one reading rather than two. Taking
/// the root as an argument is what lets the shipped content and the control's
/// fixture be read by one scan; two scans would leave the control grading a copy
/// of the thing it is meant to be a control for.
///
/// # Errors
///
/// Returns the refusal when the declarations do not load: a list derived from a
/// root that was refused would silently be empty, and an empty list is what this
/// whole check has to be unable to mistake for good news.
fn scan(root: &Path) -> Result<Scanned, Box<dyn Error>> {
    let layout = HudLayout::load(&TomlFileHudSource::new(root))?;
    Ok(Scanned {
        declaration_files: declaration_files_under(&root.join(HUD_DIRECTORY))?,
        elements: layout.elements().to_vec(),
    })
}

/// The directory of a content root that HUD declarations live in.
const HUD_DIRECTORY: &str = "hud";

/// The extension a declaration file carries.
const DECLARATION_EXTENSION: &str = "toml";

/// How many declaration files `directory` holds, or none where there is no such
/// directory.
///
/// Counts the extension the loader reads rather than every file, so a note left
/// beside the declarations is not mistaken for one.
///
/// # Errors
///
/// Returns an error if a directory that exists cannot be listed.
fn declaration_files_under(directory: &Path) -> Result<usize, Box<dyn Error>> {
    if !directory.is_dir() {
        return Ok(0);
    }
    let mut counted = 0;
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_file() && path.extension().and_then(OsStr::to_str) == Some(DECLARATION_EXTENSION)
        {
            counted += 1;
        }
    }
    Ok(counted)
}
