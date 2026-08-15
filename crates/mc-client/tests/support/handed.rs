//! What one preparation hands the renderer, where it is told to look for a save,
//! and how two of them compare.
//!
//! # Why the comparison is over bytes and not over per-section quad counts
//!
//! A texture array **layer index** rides inside every packed vertex, and section
//! order is itself part of the contract every committed golden frame was shot
//! under. A comparison of per-section quad counts sees neither: a scene whose
//! layers were resolved against a different key set, or whose sections were
//! assembled in a different order, carries exactly the same counts and draws a
//! different picture. So what is compared is the two byte views the GPU is handed.
//!
//! # Two empty scenes agree about nothing
//!
//! A byte comparison is the easiest assertion in this repository to satisfy for the
//! wrong reason: two preparations that produced no geometry at all are equal. So a
//! comparison first asks whether there is a whole world's worth of geometry on both
//! sides — from the footprint's own declaration, never from a number copied out of
//! a run — and reports the emptiness where there is not.
//!
//! # A difference is reported as one sentence, never as two buffers
//!
//! The replay packs thousands of quads, so an `assert_eq!` over the raw buffers
//! would bury its own message under a hundred kilobytes of hex. What comes back
//! instead is the first way the two disagree: which buffer, at which byte, and what
//! each side holds there.
//!
//! # A save is written here, and read back here
//!
//! A scenario about the world a launch *resumes* needs a save on disk, and it needs
//! that save to be one the launch really read: a save that was never written, or one
//! the reader refused, sends the launch down its generated-world arm — where several
//! of these scenarios would then pass by accident. So [`resumed`] writes one and
//! [`AResumedWorld::stored_at`] reads it back through the loader, so that "the save
//! really holds what the fixture meant" is a value a scenario can assert beside its
//! own claim rather than something the fixture asserts about itself.
//!
//! The world a save holds is spelled by its scenario, because the two spellings
//! answer different questions: a change to the generated world keeps every count
//! derivable from the declared landmark and surface bound, while a world built from
//! nothing is what a scenario about *which blocks a world contains* needs. Both go
//! through the same writer.

// Each binary that includes this module uses a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_render::geometry::scene::SceneGeometry;
use mc_sim::replay::ReplayWorld;
use mc_world::column::SECTIONS_PER_COLUMN;
use mc_world::content::TomlFileDefinitionSource;
use mc_world::persistence::{Acceptance, SavedPlayer, load_world, save_world};
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

/// The error type every scenario using this module propagates with `?`.
pub type TestResult = Result<(), Box<dyn Error>>;

/// How many section records a whole prepared footprint carries.
///
/// Derived from the two declarations that decide it rather than counted from a run:
/// the replay's footprint is square, and every column stacks the same number of
/// sections. A scene carrying fewer was built from less than a world.
pub const SECTIONS_IN_THE_FOOTPRINT: usize = (mc_sim::replay::world::FOOTPRINT_COLUMNS
    * mc_sim::replay::world::FOOTPRINT_COLUMNS
    * SECTIONS_PER_COLUMN) as usize;

/// What a comparison says when there is nothing to report.
pub const NO_DIFFERENCE: &str = "the same geometry, section record and packed vertex alike";

/// What a cell holding nothing is called wherever this module reports contents as
/// text.
///
/// Not a block name and unable to become one: every namespaced name carries a
/// colon, so an expectation of an empty cell and one of a named block can sit side
/// by side without either impersonating the other.
pub const NOTHING: &str = "nothing";

/// The client's own two-component save layout, stated once so that the path a save
/// is written to and the path a launch is told there is no save at cannot drift
/// apart.
const SAVE_LAYOUT: [&str; 2] = ["saves", "world.mcw"];

/// Every save here is written against the registry it is read against, so nothing
/// about its blocks can have changed and the acceptance decides nothing.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// The directory the shipped content is read from, located from the repository.
///
/// Absolute, and that is load-bearing for two of this module's consumers: they move
/// the process's working directory in order to put a save where the client looks
/// for one, and a content root resolved against that directory would then be
/// resolved against the wrong one.
///
/// # Errors
///
/// Returns an error if the manifest directory has no repository root above it.
pub fn shipped_content() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the crate manifest directory has no repository root above it")?
        .join("content")
        .join("base"))
}

/// Where a launch is told to look for a save when the scenario is that there is
/// none.
///
/// The client's own two-component layout, inside a directory nothing has written,
/// so "no save" is read from a path spelled the way a real one is rather than from
/// an obviously impossible one. Neither component is created, because a first
/// launch has no directory waiting for it either.
#[must_use]
pub fn where_no_save_is(directory: &TempDir) -> PathBuf {
    saved_in(directory)
}

/// A save a launch can be told to resume, and the world it was written from.
///
/// The directory is held inside rather than handed back beside the path, because a
/// `TempDir` dropped one line early takes the save with it and the launch then reads
/// the arm this fixture exists to keep it out of.
#[derive(Debug)]
pub struct AResumedWorld {
    directory: TempDir,
    registry: BlockRegistry,
    written: VoxelWorld,
}

impl AResumedWorld {
    /// Where the save sits, in the layout a client looks for one in.
    #[must_use]
    pub fn save(&self) -> PathBuf {
        saved_in(&self.directory)
    }

    /// What the world this fixture wrote holds at `at`, as text.
    #[must_use]
    pub fn written_at(&self, at: (u32, u32, u32)) -> String {
        described(self.written.block_at(where_at(at)))
    }

    /// What the save on disk holds at `at`, read back through the loader — or why it
    /// could not be read at all.
    ///
    /// **This is the half that says the launch had a save to resume.** A scenario
    /// asserting what a resumed world draws is satisfied just as well by a launch
    /// that generated a world because the file was not there, was truncated, or named
    /// blocks the registry had stopped declaring — and every one of those reads as an
    /// ordinary green.
    ///
    /// # Errors
    ///
    /// Returns the refusal, rendered, where the save could not be read.
    pub fn stored_at(&self, at: (u32, u32, u32)) -> Result<String, String> {
        Ok(described(self.stored()?.block_at(where_at(at))))
    }

    /// The whole world the save on disk holds, for a control that has to look at
    /// more than one cell of it.
    ///
    /// # Errors
    ///
    /// Returns the refusal, rendered, where the save could not be read.
    pub fn stored(&self) -> Result<VoxelWorld, String> {
        load_world(&self.save(), &self.registry, ACCEPTING)
            .map(|loaded| loaded.world)
            .map_err(|refusal| refusal.to_string())
    }
}

/// A save at [`AResumedWorld::save`] holding the world `holding` builds out of the
/// registry the content root at `root` declares, with `player` recorded in it.
///
/// The registry is read from the root the launch under test will read, so the
/// definition hashes the save carries are the ones the load compares against and no
/// scenario here is about a save whose blocks have been redeclared.
///
/// # Errors
///
/// Returns an error if the root does not register, if `holding` refuses to build the
/// world, or if the save cannot be written.
pub fn resumed(
    root: &Path,
    player: SavedPlayer,
    holding: impl FnOnce(&BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>>,
) -> Result<AResumedWorld, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&TomlFileDefinitionSource::new(root.to_owned()))?;
    let written = holding(&registry)?;
    let resumed = AResumedWorld {
        directory: TempDir::new()?,
        registry,
        written,
    };
    save_world(&resumed.save(), &resumed.written, player, &resumed.registry)?;
    Ok(resumed)
}

/// The blocks the generated world is made of, for a save spelled as a change to it.
///
/// # Errors
///
/// Returns an error if the world cannot be generated out of what `registry` knows.
pub fn generated_blocks(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    Ok(ReplayWorld::generate(mc_sim::REPLAY_SEED, registry)?
        .blocks()
        .clone())
}

/// What `contents` holds, as text: the block's own name, [`NOTHING`], or why the
/// cell was not one this world reaches.
///
/// A cell outside the world reports the refusal rather than folding into
/// [`NOTHING`], because a world that reached nowhere would otherwise read as a world
/// holding nothing everywhere.
fn described(contents: Result<Contents<&BlockName>, impl fmt::Display>) -> String {
    match contents {
        Err(outside) => outside.to_string(),
        Ok(Contents::Empty) => NOTHING.to_owned(),
        Ok(Contents::Holds(name)) => name.as_str().to_owned(),
    }
}

/// Where a save sits inside `directory`.
///
/// Neither component is created here: `save_world` makes the directories it needs,
/// and a launch told there is no save has none to make.
fn saved_in(directory: &TempDir) -> PathBuf {
    let inside: PathBuf = SAVE_LAYOUT.iter().collect();
    directory.path().join(inside)
}

/// A cell as the world spells a position.
const fn where_at(at: (u32, u32, u32)) -> WorldPos {
    let (x, y, z) = at;
    WorldPos { x, y, z }
}

/// How the geometry in `beside` differs from the geometry in `against`, as one
/// sentence — or [`NO_DIFFERENCE`].
#[must_use]
pub fn how_it_compares(beside: &SceneGeometry, against: &SceneGeometry) -> String {
    nothing_to_compare(beside)
        .or_else(|| nothing_to_compare(against))
        .or_else(|| {
            disagreement(
                "section record",
                &beside.section_bytes(),
                &against.section_bytes(),
            )
        })
        .or_else(|| {
            disagreement(
                "packed vertex",
                &beside.vertex_bytes(),
                &against.vertex_bytes(),
            )
        })
        .unwrap_or_else(|| NO_DIFFERENCE.to_owned())
}

/// Why `scene` is not a whole world's worth of geometry, where it is not.
fn nothing_to_compare(scene: &SceneGeometry) -> Option<String> {
    let records = scene.sections().len();
    if records != SECTIONS_IN_THE_FOOTPRINT {
        return Some(format!(
            "{records} section records were packed where the footprint declares \
             {SECTIONS_IN_THE_FOOTPRINT}"
        ));
    }
    scene
        .vertex_bytes()
        .is_empty()
        .then(|| "no vertices were packed at all".to_owned())
}

/// The first way two byte views of `what` differ, if they differ at all.
fn disagreement(what: &str, beside: &[u8], against: &[u8]) -> Option<String> {
    if beside.len() != against.len() {
        return Some(format!(
            "{} bytes of {what}s were packed where {} were",
            beside.len(),
            against.len()
        ));
    }
    let at = beside
        .iter()
        .zip(against)
        .position(|(here, there)| here != there)?;
    Some(format!(
        "{what} byte {at} is {:?} where {:?} was packed",
        beside.get(at),
        against.get(at)
    ))
}
