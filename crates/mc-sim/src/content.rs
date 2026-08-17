//! Turning a content root into the content a run is played against.
//!
//! # Why this is the simulation's and not the client's
//!
//! `docs/planning/client-server-split.md` is the binding reasoning and is not
//! re-derived here. The rule it settles: **the client never evaluates anything
//! any other participant, the server included, must agree with.** A content set
//! is the sharpest case of that there is — a layer index rides inside every
//! packed vertex, so one block the server does not have shifts every index after
//! it and the whole world is textured wrong, silently, with no error anywhere.
//! The disagreement is not localised to the block that caused it.
//!
//! So the simulation is what reads a content root, and the client is handed what
//! came back. That is a statement about **who evaluates**, not about which crate
//! a file lives in: the readers themselves stay in `mc-world`, `mc-script` keeps
//! its no-workspace-crate property, and `mc-client` gains no dependency. What
//! moved here is the construction — the sentence that says `new`.
//!
//! # What this deliberately does not do yet
//!
//! It hands back a whole [`BlockRegistry`], carrying the rules by which a world
//! is mutated as well as the fields a client draws with. **That residue is
//! stated rather than hidden**: in this arrangement the client binary *is* the
//! server, and the simulation inside it holds that registry for the whole run.
//! Narrowing what a client receives to what it draws and predicts with is the
//! next increment's, and the registry stops travelling altogether when the
//! composition root moves — which is a later spec, because this spec's exit
//! criterion is that the world renders identically and the golden suites are the
//! instrument that decides it.

use std::path::{Path, PathBuf};

use mc_core::block::{BlockId, BlockRegistry, RegistryError};
use mc_core::content::{ResolvedBlock, ResolvedContent};
use mc_core::hud::source::InMemoryHudSource;
use mc_core::hud::{HudLayout, HudLoadError, HudOrigin};
use mc_world::content::{LuauFileDefinitionSource, TomlFileHudSource};
use thiserror::Error;

/// Where the shipped content is looked for, relative to the directory the
/// process was started in.
const SHIPPED_CONTENT: [&str; 2] = ["content", "base"];

/// What a client that has not read its content yet calls the HUD it draws.
const NOTHING_READ_YET: &str = "a client that has not read its content yet";

/// Everything one content root declares, read and checked.
///
/// The registry and the HUD travel together because they are refused together: a
/// crosshair the content declares is content exactly as a block is, and a root
/// that is good for one and bad for the other is a root that failed.
#[derive(Debug)]
pub struct LoadedContent {
    /// Every block the root declares.
    ///
    /// **The simulation's own copy.** It carries the rules by which the world is
    /// mutated, which the server recomputes and a client may not apply.
    pub registry: BlockRegistry,
    /// The HUD the root declares.
    pub hud: HudLayout,
    /// What a participant that only draws receives.
    ///
    /// Derived here, once, so that the decision about which fields cross the
    /// seam is made in one place and can be read in one place.
    pub resolved: ResolvedContent,
}

/// Why a content root could not be read.
///
/// Two variants and no third: the blocks were refused or the HUD was. Each
/// carries the refusal underneath rather than a sentence of its own, because a
/// message restating what its cause already says is a second place for the two
/// to disagree.
#[derive(Debug, Error)]
pub enum ContentError {
    /// The root's block declarations were refused.
    #[error("the content root's blocks could not be read")]
    Blocks(#[from] RegistryError),
    /// The root's HUD declarations were refused.
    #[error("the content root's HUD declarations could not be read")]
    Hud(#[from] HudLoadError),
}

/// The shipped content directory, checked to exist before anything is started
/// that would fail on it later.
///
/// **Not named for the directory it finds.** `content_root` is one of the four
/// spellings a guard watches for in the client's own sources, and a client that
/// resolved the content directory for itself is a client that can still read it
/// — even if today it does not. The name says whose directory this is instead.
///
/// # Errors
///
/// Returns the path it looked in when there is nothing there, which is the
/// failure somebody running the binary from the wrong directory gets. It names
/// the directory rather than a missing block, because that is the mistake.
pub fn shipped_directory() -> Result<PathBuf, PathBuf> {
    let root = SHIPPED_CONTENT.iter().collect::<PathBuf>();
    if root.is_dir() { Ok(root) } else { Err(root) }
}

/// Everything the content root at `root` declares.
///
/// **This is the only place a block registry is populated and the only place a
/// HUD layout is loaded on the way to a run.** Both readers refuse
/// all-or-nothing, so a root either becomes content or becomes a refusal naming
/// what is wrong with it; there is no partial answer to hand back.
///
/// # Errors
///
/// Returns [`ContentError`] carrying whichever reader refused the root.
pub fn load(root: &Path) -> Result<LoadedContent, ContentError> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root.to_owned()))?;
    let hud = HudLayout::load(&TomlFileHudSource::new(root))?;
    let resolved = resolved_from(&registry);
    Ok(LoadedContent {
        registry,
        hud,
        resolved,
    })
}

/// What a participant that only draws receives, out of what the reader
/// registered.
///
/// **This is the one place the seam is cut**, and the three fields it does not
/// copy are the point: `replaceable`, `breakable` and `breaks_into` are the
/// rules by which a world is mutated, the server recomputes every one of them,
/// and a client holding them would be holding rules it may not apply.
///
/// **The layer assignment is stated here rather than left to whoever draws.** A
/// layer index rides inside every packed vertex, so a client deriving its own
/// from a sort would renumber every index after any block the two disagreed
/// about — silently, and not localised to that block. Assigning it once, where
/// the content was read, is what makes the client's job to *honour* an answer
/// rather than to reproduce a decision.
fn resolved_from(registry: &BlockRegistry) -> ResolvedContent {
    let blocks = (0..registry.registered_count())
        .filter_map(|position| u32::try_from(position).ok())
        .filter_map(|raw| registry.definition(BlockId::from_raw(raw)).ok())
        .map(|definition| ResolvedBlock {
            name: definition.name.clone(),
            texture: definition.texture.clone(),
            is_solid: definition.is_solid,
        })
        .collect::<Vec<_>>();
    // Lexicographic today, and that is an implementation detail rather than a
    // contract: nothing downstream may derive this order for itself, which is
    // exactly what shipping the assignment buys.
    let layers = registry
        .texture_keys()
        .into_iter()
        .zip(0..)
        .collect::<Vec<_>>();
    ResolvedContent::stating(blocks, layers)
}

/// The HUD of a client that has not read its content yet.
///
/// **Built through the loader rather than constructed**, and that is not
/// ceremony: [`HudLayout::load`] is the only door into a layout precisely so
/// that no engine can put an element into one from Rust. A `Default` here would
/// be a second door, and the invariant that the base game holds no privilege a
/// mod lacks would then rest on nobody using it. A source declaring nothing is a
/// valid, empty answer, which is the one place HUD loading diverges from block
/// registration.
///
/// # Errors
///
/// Returns [`HudLoadError`] if a source declaring nothing is ever refused, which
/// is the one thing HUD loading is specified never to do.
pub fn hud_before_content_is_read() -> Result<HudLayout, HudLoadError> {
    HudLayout::load(&InMemoryHudSource::new(
        HudOrigin::new(NOTHING_READ_YET),
        Vec::new(),
    ))
}
