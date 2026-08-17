//! A content watch a scenario reports changes on, and what each tick boundary
//! made of them.
//!
//! # The double holds no policy, which is what makes an assertion about the
//! domain an assertion
//!
//! It hands over exactly what it was given, one report per ask, and decides
//! nothing about relevance, coalescing or timing. Whether a path is one the
//! loader reads, how many attempts a burst becomes and when an attempt may begin
//! are the simulation's answers — a double that filtered paths or collapsed
//! reports would agree with the client by construction, and every scenario
//! written against it would be green while the shipped client rebuilt on every
//! event under the root.
//!
//! **One report per ask, so a scenario decides which boundary sees which
//! report.** A burst of writes that arrived between two boundaries is therefore
//! spelled as *one* [`Reports::changed`] carrying several paths, which is what the
//! port's own vocabulary says a change report is: the paths that changed since it
//! was last asked.
//!
//! # An attempt is counted by what it reported, never by a flag
//!
//! There is no accessor for "a build is in flight", and asking for one would put
//! the count this suite is about inside the value under test. What a scenario
//! counts is the reports a run of tick boundaries produced — a taking up or a
//! refusal — which is the same thing a person sees. A boundary that reported
//! nothing contributes nothing, so the length of [`ended`] is the number of
//! attempts that finished and never the number of ticks.
//!
//! # How long a run has to be, and why that is three numbers rather than one
//!
//! **The bounds a run needs are not all the same kind of number, and collapsing them
//! into one is what made a test flaky under the gate's load.** A run expecting *no*
//! attempt needs a **minimum** — a window long enough for the presence it denies,
//! because giving up early reports "nothing began" for a reason about the machine and
//! reports it silently. A run expecting *an* attempt needs a **maximum**, where being
//! generous costs nothing and asserts nothing about timing. Each bound in
//! `reload_watch/runs.rs` says which of the two kinds it is and carries the
//! measurement it was derived from, in both directions.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names types the implementation has not written yet, exactly as
//! [`crate::reload`] and [`crate::reload_content`] do. A binary including this
//! must declare `mod support;`, the input harness and [`crate::reload_world`] as
//! well: the client it starts and the worlds it plays are theirs.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

/// How long a run lasts and how its boundaries are crossed.
///
/// A child module because this file crossed its size limit and **its own header
/// already named the seam**: "how long a run has to be" is a different subject from
/// the double, the vocabulary and the fixtures. Re-exported, so every binary that
/// includes this file reaches the same names it always did.
#[path = "reload_watch/runs.rs"]
mod runs;

pub use runs::*;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use mc_client::session::reload::ReloadReport;
use mc_core::block::BlockRegistry;
use mc_core::content::LayerAssignment;
use mc_render::window::rendered;
use mc_sim::player::PlayerState;
use mc_sim::reload::ContentReload;
use mc_world::content::watch::{ContentChanges, ContentWatch};
use mc_world::world::VoxelWorld;

use crate::input::InputHarness;
use crate::reload::{DIRT, GRASS, STONE, WATER};
use crate::reload_world::{floor_of, playing, standing};
use crate::support::content::{BLOCK_DIRECTORY, ContentRoot, HUD_DIRECTORY};

/// The subdirectory of a content root that block textures are described in.
///
/// **The loader reads nothing here** — `tools/voxforge` is its only reader, and no
/// block declaration names a material. It is spelled out rather than reached for
/// through a loader constant for exactly that reason: there is no constant, because
/// nothing on the content path knows this directory exists.
pub const MATERIALS_DIRECTORY: &str = "materials";

/// The material file the negative case in `tests/reload_reads_only_declarations.rs`
/// names, which the shipped root really holds.
pub const A_MATERIAL_FILE: &str = "dirt.toml";

/// What an editor leaves beside the file it is saving.
///
/// The suffix is appended to a declaration's whole file name, which is how a real
/// editor spells it — `stone.luau.swp` and not `stone.swp`. That is the case worth
/// pinning: a rule that looked for its extension anywhere in the name rather than
/// at the end would read this as a declaration.
pub const SCRATCH_SUFFIX: &str = ".swp";

/// A file that is not a chunk at all, so nothing ever returns a table and there is
/// no field to read a name out of: the whole file is what is wrong.
///
/// Broken as *syntax* rather than by returning the wrong thing, so what reaches the
/// author is the compiler's own complaint carried out through the loader.
pub const NOT_A_CHUNK: &str = "this is not a chunk at all\n";

/// A field nobody recognises, spelled close enough to a real one to be the typo a
/// mod author actually makes.
pub const MISSPELLED_SOLID: &str = "slid";

/// Stone's declaration with `solid` misspelled and nothing else touched.
///
/// The misspelling stands *in place of* the field it was meant to be, which is what
/// an author who typed it has: a declaration carrying both would be refused for the
/// stray field while the block still worked.
pub const STONE_MISSPELLING_SOLID: &str =
    "return {\n\tname = \"base:stone\",\n\ttexture = \"base:stone\",\n\tslid = true,\n}\n";

/// `root` with the declaration in `file_name` replaced by text a fixture wrote out
/// whole.
///
/// The declaration builder in [`crate::reload`] takes a well-formed declaration and
/// cannot spell a broken one, and refusal scenarios are about text no builder would
/// produce. Its check is kept: a root that never declared the file is not a root
/// whose declaration an author edited.
///
/// # Errors
///
/// Returns an error if the root does not declare that file, or if the write fails.
pub fn restating_raw(
    root: ContentRoot,
    file_name: &str,
    declaration: &str,
) -> Result<ContentRoot, Box<dyn Error>> {
    let declared = root.path().join(BLOCK_DIRECTORY).join(file_name);
    if !declared.is_file() {
        return Err(format!(
            "this fixture has to restate `{BLOCK_DIRECTORY}/{file_name}` in the root the client is \
             playing, and that root does not declare it. What it would build is a root that gained \
             a declaration rather than one whose declaration an author broke"
        )
        .into());
    }
    fs::write(&declared, declaration)?;
    Ok(root)
}

/// A content watch that reports what a scenario tells it to.
#[derive(Debug)]
pub struct WatchDouble {
    reported: Receiver<ContentChanges>,
}

impl ContentWatch for WatchDouble {
    fn changes(&mut self) -> ContentChanges {
        match self.reported.try_recv() {
            Ok(changes) => changes,
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => ContentChanges::Nothing,
        }
    }
}

/// The handle a scenario reports changes through.
#[derive(Debug)]
pub struct Reports {
    reporting: Sender<ContentChanges>,
}

impl Reports {
    /// Reports that these paths changed, as one report.
    ///
    /// One call is one answer to one ask, so several paths here are a burst that
    /// arrived between two boundaries — and two calls are two reports arriving at
    /// two of them.
    ///
    /// # Errors
    ///
    /// Returns an error when nothing is listening, which is a client whose reload
    /// was dropped rather than a change nobody noticed.
    pub fn changed(&self, paths: &[PathBuf]) -> Result<(), Box<dyn Error>> {
        self.reporting
            .send(ContentChanges::Changed(paths.to_vec()))?;
        Ok(())
    }
}

/// A watch nothing has reported on yet, and the handle to report on it.
#[must_use]
pub fn watch() -> (WatchDouble, Reports) {
    let (reporting, reported) = channel();
    (WatchDouble { reported }, Reports { reporting })
}

/// A client standing on a floor of `block`, playing the content root at `root`
/// and watching that same root through a double.
///
/// **The root the client plays and the root it watches are one directory**, which
/// is the arrangement a mod author is in: the file they edit is the file the run
/// was started from. A fixture watching a second root would be about a reload of
/// content nobody was playing.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if
/// the content declares no solid block at all.
pub fn a_client_on(
    root: &ContentRoot,
    block: &str,
) -> Result<(InputHarness, Reports), Box<dyn Error>> {
    a_client_over(root, standing(), |registry| floor_of(registry, block))
}

/// The same, over a world and a spawn the caller declared.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if
/// the content declares no solid block at all.
pub fn a_client_over(
    root: &ContentRoot,
    spawn: PlayerState,
    blocks_of: impl FnOnce(&BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>>,
) -> Result<(InputHarness, Reports), Box<dyn Error>> {
    let (simulation, holding) = playing(root.path(), spawn, blocks_of)?;
    let (watching, reports) = watch();
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client.attach_reload(ContentReload::watching(
        root.path().to_owned(),
        Box::new(watching),
    ));
    Ok((client, reports))
}

/// A client with no world yet, already watching the root at `root`.
///
/// The state every run is in before its preparation lands, and the one the boundary
/// case about a change reported before any tick has been advanced is about: there is
/// no tick to swap at yet.
#[must_use]
pub fn a_client_with_no_world(root: &ContentRoot) -> (InputHarness, Reports) {
    let (watching, reports) = watch();
    let mut client = InputHarness::started();
    client.attach_reload(ContentReload::watching(
        root.path().to_owned(),
        Box::new(watching),
    ));
    (client, reports)
}

/// What one tick boundary reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attempt {
    /// A candidate was taken up, and what it declared is now the content serving.
    TakenUp,
    /// A candidate was refused, in the words a person reads.
    Refused { said: String },
}

/// What one boundary's report is, as an [`Attempt`].
#[must_use]
pub fn attempt_of(report: Option<ReloadReport>) -> Option<Attempt> {
    match report {
        None => None,
        Some(ReloadReport::Accepted { .. }) => Some(Attempt::TakenUp),
        Some(ReloadReport::Refused(said)) => Some(Attempt::Refused { said }),
    }
}

/// What the one refusal a run reported said, judged against what it had to name.
///
/// **A total verdict, so an assertion against the good arm rejects every other
/// answer** — a run that reported nothing, one that took the candidate up, one
/// that reported a refusal twice, and one whose text never carried the loader's own
/// words at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// One refusal, carrying the loader's own words and naming everything asked.
    NamedEverythingAsked,
    /// One refusal carrying the loader's own words, which named none of these.
    DidNotName(Vec<String>),
    /// One refusal whose text does not end in the loader's own words.
    NotTheLoadersOwnWords { said: String },
    /// The run did not report exactly one refusal, and reported these instead.
    Reported(Vec<Attempt>),
}

/// What `crossed` refused with, against the loader's own `words` and the `needles`
/// the scenario requires.
///
/// **The loader's own words are asked of a second read of the same root**, so no
/// wording is spelled in a scenario and a reworded refusal moves both sides
/// together. What a *dropped* cause moves is only the reported side, which is the
/// asymmetry this comparison has and a snapshotted string does not.
#[must_use]
pub fn refusal(crossed: &[Option<Attempt>], words: &str, needles: &[String]) -> Refusal {
    let attempts = ended(crossed);
    let [Attempt::Refused { said }] = attempts.as_slice() else {
        return Refusal::Reported(attempts);
    };
    if !said.ends_with(words) {
        return Refusal::NotTheLoadersOwnWords { said: said.clone() };
    }
    refusal_naming(crossed, needles)
}

/// The same, for a refusal no second reader can be asked for the words of.
///
/// A root that cannot be watched at all is refused by nothing the loader ever sees,
/// so there is no independent rendering to compare against and the needles are the
/// whole of what a scenario can require.
#[must_use]
pub fn refusal_naming(crossed: &[Option<Attempt>], needles: &[String]) -> Refusal {
    let attempts = ended(crossed);
    let [Attempt::Refused { said }] = attempts.as_slice() else {
        return Refusal::Reported(attempts);
    };
    let missing: Vec<String> = needles
        .iter()
        .filter(|needle| !said.contains(needle.as_str()))
        .cloned()
        .collect();
    if missing.is_empty() {
        Refusal::NamedEverythingAsked
    } else {
        Refusal::DidNotName(missing)
    }
}

/// The words the one refusal a run reported said, or nothing where it did not
/// report exactly one.
///
/// For the scenario that asks about the *order* two names are given in, which a
/// search for each of them cannot see.
#[must_use]
pub fn refusal_said(crossed: &[Option<Attempt>]) -> Option<String> {
    match ended(crossed).as_slice() {
        [Attempt::Refused { said }] => Some(said.clone()),
        _ => None,
    }
}

/// Whether the one refusal a run reported named `first` before `second`.
///
/// `false` where either is missing or where no single refusal was reported, so this
/// is only ever read beside a verdict that says which of those it was.
#[must_use]
pub fn named_in_order(crossed: &[Option<Attempt>], first: &str, second: &str) -> bool {
    refusal_said(crossed).is_some_and(|said| match (said.find(first), said.find(second)) {
        (Some(before), Some(after)) => before < after,
        _ => false,
    })
}

/// The words the loader itself produces for the content root at `root`, read
/// again with no reload anywhere near it.
///
/// # Errors
///
/// Returns an error if the root reads. A root that is accepted is not a root a
/// refusal can be compared against, and every needle would then be missing for a
/// reason that has nothing to do with the reload.
pub fn the_loaders_own_words(root: &Path) -> Result<String, Box<dyn Error>> {
    match mc_sim::content::load(root, &LayerAssignment::none()) {
        Ok(_) => Err(format!(
            "this scenario needs the content root at {root} to be refused, and it read. There is \
             no refusal for the reload's report to be compared against",
            root = root.display()
        )
        .into()),
        Err(refused) => Ok(rendered(&refused)),
    }
}

/// Which blocks a client is serving and which of them it calls solid, in the
/// order they were registered.
///
/// Read through the content the simulation publishes, which is what a reader draws
/// and predicts with — so a scenario asserting a block here is asserting the value
/// a frame is built from rather than a second accessor beside it.
///
/// # Errors
///
/// Returns an error where nothing is being published, which is a client with no
/// world rather than one serving anything a scenario could read.
pub fn serving(client: &InputHarness) -> Result<Vec<(String, bool)>, Box<dyn Error>> {
    let published = client
        .content()
        .ok_or("this fixture's client publishes no content, so there are no blocks to read")?;
    Ok(published
        .resolved
        .blocks()
        .map(|block| (block.name.as_str().to_owned(), block.is_solid))
        .collect())
}

/// Whether the content a client is serving calls `block` solid, or nothing at all
/// where it does not declare it.
///
/// **`None` rather than `false`**, so a candidate that dropped the block and one
/// that took its solidity away are told apart by shape.
///
/// # Errors
///
/// Returns an error where nothing is being published.
pub fn solidity_of(client: &InputHarness, block: &str) -> Result<Option<bool>, Box<dyn Error>> {
    Ok(serving(client)?
        .into_iter()
        .find(|(name, _)| name.as_str() == block)
        .map(|(_, solid)| solid))
}

/// One block a client serves, with the solidity its declaration states.
#[must_use]
pub fn declaring(block: &str, solid: bool) -> (String, bool) {
    (block.to_owned(), solid)
}

/// What the shipped root's four declarations state, in the file-name order they
/// are registered in.
///
/// **Listed rather than read back**, for the reason [`crate::reload`] lists the
/// names: a fixture that discovered them would go on passing over a root that had
/// stopped declaring one, and every expectation below is a statement about which
/// blocks a candidate built from the whole root carries. Water is the one shipped
/// block its own declaration calls not solid.
#[must_use]
pub fn the_four_shipped_blocks() -> Vec<(String, bool)> {
    vec![
        declaring(DIRT, true),
        declaring(GRASS, true),
        declaring(STONE, true),
        declaring(WATER, false),
    ]
}

/// How a block declaration is named inside a refusal — as a path, so the needle is
/// spelled the way this platform spells one.
#[must_use]
pub fn declaration_named(file_name: &str) -> String {
    Path::new(BLOCK_DIRECTORY)
        .join(file_name)
        .display()
        .to_string()
}

/// The needles a scenario asks a refusal to carry.
#[must_use]
pub fn naming(needles: &[&str]) -> Vec<String> {
    needles.iter().map(|needle| (*needle).to_owned()).collect()
}

/// The path of a block declaration under `root`.
#[must_use]
pub fn block_path(root: &ContentRoot, file_name: &str) -> PathBuf {
    root.path().join(BLOCK_DIRECTORY).join(file_name)
}

/// The path of a HUD declaration under `root`.
#[must_use]
pub fn hud_path(root: &ContentRoot, file_name: &str) -> PathBuf {
    root.path().join(HUD_DIRECTORY).join(file_name)
}

/// The path of a material file under `root` — a file the loader does not read.
#[must_use]
pub fn material_path(root: &ContentRoot, file_name: &str) -> PathBuf {
    root.path().join(MATERIALS_DIRECTORY).join(file_name)
}
