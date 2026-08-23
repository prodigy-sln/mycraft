//! Content roots built from the shipped one, and what the simulation answers
//! when it is handed one as a candidate.
//!
//! **A root is always copied, never edited in place.** `content/base/` is the
//! product's own content: a fixture that removed a declaration from it would
//! leave the repository in whatever state the run ended in, and a run that failed
//! half way would leave it broken.
//!
//! **Removing a declaration that was never there is a failure, not a no-op**, and
//! adding one the root already declares is the same failure in the other
//! direction. A root that never declared the block is not a root whose
//! declaration was taken away, and a scenario about a candidate that dropped a
//! block would be about a root nobody stripped.
//!
//! **Every candidate is read back through [`mc_sim::content::load`]**, the one
//! door a content root is read through. A fixture that assembled definitions in
//! Rust would be handing the simulation content no author could have written.
//!
//! This module deliberately restates a little of what
//! `crates/mc-client/tests/support/reload.rs` also holds. The two are separate
//! test crates and share no code; what is here is the smaller half — copy,
//! remove, restate, add — because the scenarios on this side ask about the
//! simulation's own answer rather than about what an author's edit becomes.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names types the implementation has not written yet, and a module declared
//! in `support/mod.rs` is compiled into every binary that says `mod support;` —
//! which would leave the whole crate's tests unable to build for the whole of the
//! window before the swap lands. Reached by path, only the suites that are about
//! a reload are in that window. It is self-contained for the same reason: a
//! `super::` of its own would tie it back to the module it is kept out of.

// Each test binary links this whole module and drives a subset of it.
//
// Stated here rather than inherited: being reached by `#[path]` is exactly what
// keeps this module out of `support/mod.rs`, and so out of the reach of that
// module's own allow.
#![allow(dead_code)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use mc_core::content::LayerAssignment;
use mc_sim::content::LoadedContent;
use mc_sim::reload::ReloadRefusal;
use mc_sim::simulation::Accepted;
use tempfile::TempDir;

/// The subdirectory of a content root that block declarations live in.
pub const BLOCK_DIRECTORY: &str = "blocks";

/// The files the shipped root declares its four blocks in.
pub const DIRT_FILE: &str = "dirt.luau";
pub const GRASS_FILE: &str = "grass.luau";
pub const STONE_FILE: &str = "stone.luau";
pub const WATER_FILE: &str = "water.luau";

/// A block declared for the first time, in a file that sorts before every one of
/// those.
pub const AMBER: &str = "base:amber";
pub const AMBER_FILE: &str = "amber.luau";

/// What a candidate declares [`AMBER`] to be.
///
/// Every optional field is stated **against** its default — `replaceable` is
/// absent-means-false and `breakable` absent-means-true — so a registry
/// answering from the defaults rather than from the declaration answers
/// differently on every one of them.
///
/// Its `texture` is equal to its `name`, which is not an oversight: the mesher
/// resolves a quad's layer by parsing a block's name as a texture key, and the
/// pin on that substitution turns red the day that gap is closed. A fixture
/// declaring a texture of its own would need the gap closed to draw.
pub const AMBER_DECLARATION: &str = "return {\n\
     \tname = \"base:amber\",\n\
     \ttexture = \"base:amber\",\n\
     \tsolid = true,\n\
     \treplaceable = true,\n\
     \tbreakable = false,\n\
     \tbreaks_into = \"base:dirt\",\n\
     }\n";

/// What a candidate says about `base:stone` when an author has taken its
/// solidity away and changed nothing else.
///
/// The shipped declaration with one word altered, spelled out rather than
/// rewritten from the file so that a scenario reading this can see exactly what
/// the author's edit was.
pub const STONE_THAT_IS_NOT_SOLID: &str = "return {\n\
     \tname = \"base:stone\",\n\
     \ttexture = \"base:stone\",\n\
     \tsolid = false,\n\
     }\n";

/// What a candidate says about `base:dirt` and about `base:grass` when an author
/// has taken their solidity away and changed nothing else.
///
/// **Restated rather than removed, and that is what makes a scenario over them
/// worth anything.** A candidate that stopped declaring dirt and grass would
/// leave stone the first block registered *as well as* the first solid one, so a
/// rule reading plain registration order would answer stone too. Restating them
/// keeps dirt first in registration order and makes stone first among the blocks
/// that stop a player, and those are two different answers.
pub const DIRT_THAT_IS_NOT_SOLID: &str = "return {\n\
     \tname = \"base:dirt\",\n\
     \ttexture = \"base:dirt\",\n\
     \tsolid = false,\n\
     }\n";

pub const GRASS_THAT_IS_NOT_SOLID: &str = "return {\n\
     \tname = \"base:grass\",\n\
     \ttexture = \"base:grass\",\n\
     \tsolid = false,\n\
     }\n";

/// What a candidate says about `base:stone` when an author has said no ray may
/// stop at it and has left its solidity alone.
///
/// The one declaration in which what stops a player and what a swing can find
/// disagree, and it is stated *against* the default: `targetable` absent means
/// the block's own `solid`, so a reader that answered from the default rather
/// than from the declaration answers `true` here.
pub const STONE_THAT_MAY_NOT_BE_AIMED_AT: &str = "return {\n\
     \tname = \"base:stone\",\n\
     \ttexture = \"base:stone\",\n\
     \tsolid = true,\n\
     \ttargetable = false,\n\
     }\n";

/// A content root written into a temporary directory, removed when this is
/// dropped.
///
/// The directory is held inside rather than handed back beside the path, because
/// a `TempDir` dropped one line early deletes the tree the candidate is still
/// being read from, and the failure then reads as a missing content root.
#[derive(Debug)]
pub struct ARoot {
    directory: TempDir,
}

impl ARoot {
    /// Where this root sits.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// This root with the named block declarations taken out of `blocks/`.
    ///
    /// # Errors
    ///
    /// Returns an error if a named declaration was not there to remove, or if a
    /// removal fails.
    pub fn not_declaring(self, file_names: &[&str]) -> Result<Self, Box<dyn Error>> {
        let blocks = self.path().join(BLOCK_DIRECTORY);
        for file_name in file_names {
            let declared = blocks.join(file_name);
            require_declared(&declared, file_name)?;
            fs::remove_file(&declared)?;
        }
        Ok(self)
    }

    /// This root with one more block declaration written into `blocks/`.
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails, or if the root already declares that
    /// file.
    pub fn declaring(self, file_name: &str, declaration: &str) -> Result<Self, Box<dyn Error>> {
        let declared = self.path().join(BLOCK_DIRECTORY).join(file_name);
        if declared.exists() {
            return Err(format!(
                "this fixture has to add `{BLOCK_DIRECTORY}/{file_name}` to a copy of the shipped \
                 content root, and the shipped root already declares it. What it would build is a \
                 root whose block came from the shipped content rather than from this fixture"
            )
            .into());
        }
        fs::write(&declared, declaration)?;
        Ok(self)
    }

    /// This root with the declaration in `file_name` replaced by `declaration`.
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails, or if the root does not declare that
    /// file — a root that never declared the block is not a root whose
    /// declaration an author edited.
    pub fn restating(self, file_name: &str, declaration: &str) -> Result<Self, Box<dyn Error>> {
        let declared = self.path().join(BLOCK_DIRECTORY).join(file_name);
        if !declared.is_file() {
            return Err(format!(
                "this fixture has to restate `{BLOCK_DIRECTORY}/{file_name}` in a copy of the \
                 shipped content root, and the shipped root does not declare it. What it would \
                 build is a root that gained a declaration rather than one whose declaration an \
                 author edited"
            )
            .into());
        }
        fs::write(&declared, declaration)?;
        Ok(self)
    }

    /// Everything this root declares, read through the one door, against a
    /// session that has spent no layers.
    ///
    /// Which layers a session has already spent is what the scenarios in
    /// `mc-client` about appending are written over; the scenarios this serves
    /// ask about the simulation's own admission answer and read no layer index,
    /// and a session that has spent nothing is the state every one of them is in.
    ///
    /// # Errors
    ///
    /// Returns whichever reader refused the root.
    pub fn candidate(&self) -> Result<LoadedContent, Box<dyn Error>> {
        Ok(mc_sim::content::load(
            self.path(),
            &LayerAssignment::none(),
        )?)
    }
}

/// The shipped content root, copied whole into a temporary directory.
///
/// # Errors
///
/// Returns an error if the repository's content root cannot be located or
/// copied.
pub fn shipped() -> Result<ARoot, Box<dyn Error>> {
    let directory = TempDir::new()?;
    copy_tree(&shipped_content()?, directory.path())?;
    Ok(ARoot { directory })
}

/// The directory the shipped content is read from, located from the repository
/// rather than from the working directory a test binary happens to start in.
///
/// # Errors
///
/// Returns an error if the manifest directory has no repository root above it.
fn shipped_content() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the crate manifest directory has no repository root above it")?
        .join("content")
        .join("base"))
}

/// What the simulation answered when it was handed a candidate.
///
/// **Enumerated, and the two arms that mean "this suite could not tell" are arms
/// of their own.** A scenario comparing the whole of it rejects an unrecognised
/// refusal rather than reading it as the one it expected, which
/// `assert!(answer.is_err())` cannot do.
///
/// The refusal type itself is deliberately not compared for equality: it grows a
/// variant carrying a content error in a later phase, and that error chain has no
/// `PartialEq` to derive from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adoption {
    /// The candidate is now the content serving, and this is the block a player
    /// holds under it.
    Accepted { holding: String },
    /// The world holds these blocks and the candidate declares none of them, in
    /// the order the refusal named them.
    BlocksTheWorldHolds(Vec<String>),
    /// The candidate registers no solid block, said in these words.
    NothingToPlace { said: String },
    /// A refusal this suite does not recognise, rendered.
    Refused(String),
}

/// What one answer is, as an [`Adoption`].
#[must_use]
pub fn adoption(answered: Result<Accepted, ReloadRefusal>) -> Adoption {
    match answered {
        Ok(accepted) => Adoption::Accepted {
            holding: accepted.holding.as_str().to_owned(),
        },
        Err(refused) => refusal(&refused),
    }
}

/// What one refusal says, as an [`Adoption`].
///
/// **Two questions and a fallback rather than a `match`, and that is not a style
/// choice.** The fallback exists so that a refusal a later phase adds is
/// *reported* — naming which one arrived — instead of breaking this file's
/// compilation. But today's two variants are between them exhaustive, so a
/// `match` carrying a catch-all arm makes that arm an unreachable pattern, which
/// `-D warnings` refuses. Asking about each variant in turn makes the fallback
/// reachable by construction, and it goes on being reachable the day `Content`
/// and `BuilderLost` arrive, without this function being edited to let them
/// through.
fn refusal(refused: &ReloadRefusal) -> Adoption {
    if let ReloadRefusal::BlocksTheWorldHolds { blocks } = refused {
        return Adoption::BlocksTheWorldHolds(
            blocks
                .iter()
                .map(|block| block.as_str().to_owned())
                .collect(),
        );
    }
    if matches!(refused, ReloadRefusal::NothingToPlace) {
        return Adoption::NothingToPlace {
            said: refused.to_string(),
        };
    }
    Adoption::Refused(refused.to_string())
}

/// An acceptance whose held block is `holding`, for a scenario to compare
/// against.
#[must_use]
pub fn accepted(holding: &str) -> Adoption {
    Adoption::Accepted {
        holding: holding.to_owned(),
    }
}

/// A refusal naming these blocks, in this order.
#[must_use]
pub fn holding_blocks_it_does_not_declare(blocks: &[&str]) -> Adoption {
    Adoption::BlocksTheWorldHolds(blocks.iter().map(|block| (*block).to_owned()).collect())
}

/// Refuses unless the shipped root declared `file_name` to begin with.
///
/// A root that never declared the block is not a root whose declaration was
/// taken away, and a scenario about a candidate that dropped a block would be
/// about a root nobody stripped.
fn require_declared(declared: &Path, file_name: &str) -> Result<(), Box<dyn Error>> {
    if declared.is_file() {
        return Ok(());
    }
    Err(format!(
        "this fixture has to remove `{BLOCK_DIRECTORY}/{file_name}` from a copy of the shipped \
         content root, and the shipped root does not declare it. What it would build is a root \
         that never declared the block rather than one whose declaration was taken away, and the \
         two are not the same claim"
    )
    .into())
}

/// Copies every file and directory under `from` into `into`.
fn copy_tree(from: &Path, into: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(into)?;
    for entry in fs::read_dir(from)? {
        let source: PathBuf = entry?.path();
        let Some(name) = source.file_name() else {
            continue;
        };
        let destination = into.join(name);
        if source.is_dir() {
            copy_tree(&source, &destination)?;
        } else {
            fs::copy(&source, &destination)?;
        }
    }
    Ok(())
}
