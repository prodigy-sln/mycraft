//! The declarations a mod author edits, and what the running client answers when
//! it is handed the content those edits produce.
//!
//! # A candidate is a real content root, read through the one door
//!
//! Every candidate below is a copy of the shipped root with files written into
//! it, read back through [`mc_sim::content::load`]. Nothing here constructs a
//! registry, and that is the whole point: a fixture that assembled definitions in
//! Rust would be asserting against content no author could have written, and the
//! refusals an author actually meets — a misspelled field, a chunk that will not
//! compile — would be unreachable from it.
//!
//! **A root is always copied, never edited in place**, which is
//! [`super::content`]'s rule and is inherited unchanged: `content/base/` is the
//! product's own content and a run that failed half way through would leave the
//! repository broken.
//!
//! **Restating a declaration refuses when the file was not there**, for the same
//! reason [`super::content`] refuses removing one that was never declared. A root
//! that never declared the block is not a root whose declaration changed, and a
//! scenario about what the change does would be about neither.
//!
//! # The answer is an enumerated verdict and never a boolean
//!
//! [`Adoption`] names every outcome a phase-1 reload can reach, including the two
//! that mean "there was nothing to look at": a client with no simulation, and a
//! refusal this suite does not recognise. A test comparing the whole of it
//! rejects those for free, which `assert!(result.is_ok())` cannot do — and the
//! unrecognised arm carries the rendered refusal, so a scenario that meets a
//! refusal from a later phase reports which one rather than merely disagreeing.
//!
//! The refusal type itself is deliberately **not** compared for equality: it
//! grows a variant carrying a content error in a later phase, and that error
//! chain has no `PartialEq` to derive from. What is compared is what a person
//! could act on — which refusal, over which blocks, in which order, saying what.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names types the implementation has not written yet, and a module declared
//! in `support/mod.rs` is compiled into every binary that says `mod support;` —
//! which would leave the whole crate's tests unable to build for the whole of the
//! window before the swap lands. Reached by path, only the suites that are about
//! a reload are in that window. A binary including this must declare
//! `mod support;` as well: the content roots it builds on are that module's.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::fs;
use std::path::Path;

use mc_core::content::LayerAssignment;
use mc_sim::content::LoadedContent;
use mc_sim::reload::ReloadRefusal;
use mc_sim::simulation::Accepted;

use crate::support::content::{BLOCK_DIRECTORY, ContentRoot, shipped_copy};

/// The four blocks the shipped root declares, spelled as content spells them.
///
/// Named here rather than derived from a read of the root, because what these
/// scenarios are about is an author editing a declaration they can see: a fixture
/// that discovered the names would go on passing over a root that had stopped
/// declaring any of them.
pub const DIRT: &str = "base:dirt";
pub const GRASS: &str = "base:grass";
pub const STONE: &str = "base:stone";
pub const WATER: &str = "base:water";

/// The texture keys the shipped root declares, ascending — which is **not** the
/// same list as its block names any more.
///
/// The grass block states a key per facing: `base:grass_top` upward,
/// `base:dirt` downward — the same image the dirt block draws — and four side
/// keys of its own. So `base:grass` is a *block* name that no longer spells a
/// texture key at all, and a root that declared four keys now declares eight.
///
/// **Every layer expectation in these suites is a position in this list**, so it
/// is stated once here and derived everywhere else. Dirt and stone still state
/// one key across all six of their own facings.
pub const SHIPPED_TEXTURE_KEYS: [&str; 8] = [
    DIRT,
    GRASS_SIDE_EAST,
    GRASS_SIDE_NORTH,
    GRASS_SIDE_SOUTH,
    GRASS_SIDE_WEST,
    GRASS_TOP,
    STONE,
    WATER,
];

/// The five keys **only** the grass block declares, ascending.
///
/// `base:dirt` is deliberately not among them. The grass block's underside draws
/// it and the dirt block draws it on all six of its own facings, so a root that
/// stops declaring grass retires these five and leaves dirt exactly where it
/// was — which is what makes "one declaration away, several keys retired" a
/// thing a fixture can state.
pub const GRASS_ONLY_KEYS: [&str; 5] = [
    GRASS_SIDE_EAST,
    GRASS_SIDE_NORTH,
    GRASS_SIDE_SOUTH,
    GRASS_SIDE_WEST,
    GRASS_TOP,
];

/// The six keys the grass block declares, one per facing.
pub const GRASS_TOP: &str = "base:grass_top";
pub const GRASS_SIDE_NORTH: &str = "base:grass_side_north";
pub const GRASS_SIDE_SOUTH: &str = "base:grass_side_south";
pub const GRASS_SIDE_EAST: &str = "base:grass_side_east";
pub const GRASS_SIDE_WEST: &str = "base:grass_side_west";

/// A block no declaration declares, named by one that does.
///
/// It exists so that a `breaks_into` can name something the registry will never
/// hold — the late-resolution contract `docs/modding/blocks-items.md` states, and
/// which a reload is required to leave exactly as it is.
pub const MITHRIL: &str = "base:mithril";

/// The block a candidate declares for the first time.
///
/// Its file sorts before `dirt.luau`, which is what puts it first in registration
/// order and so first among the solid blocks.
pub const AMBER: &str = "base:amber";
pub const AMBER_FILE: &str = "amber.luau";

/// The files the shipped root declares its four blocks in.
pub const DIRT_FILE: &str = "dirt.luau";
pub const GRASS_FILE: &str = "grass.luau";
pub const STONE_FILE: &str = "stone.luau";
pub const WATER_FILE: &str = "water.luau";

/// One block declaration, as a mod author writes it.
///
/// Every optional field is an `Option` rather than a value with a default, so a
/// declaration that states nothing about `replaceable` is spelled differently
/// from one that states `false`. The two are the same to the loader today and a
/// fixture that could not tell them apart would be unable to say which it meant.
#[derive(Debug, Clone)]
pub struct Declaration {
    name: String,
    texture: String,
    /// The key `north` draws from, where this declaration states its texture as a
    /// table rather than as one string. See [`Declaration::repointing_north`].
    north: Option<String>,
    solid: bool,
    replaceable: Option<bool>,
    breakable: Option<bool>,
    breaks_into: Option<String>,
}

impl Declaration {
    /// A block of `name`, drawn from a texture key equal to it.
    ///
    /// **The texture is equal to the name, and that is now a convenience rather
    /// than a constraint.** It used to be neither: a quad's layer was resolved by
    /// parsing the block's *name*, so a declaration naming a different texture
    /// loaded and then drew nothing. A face draws the key its block declared now,
    /// on every facing, so a fixture here may state the two apart wherever a
    /// scenario needs it to — and the ones whose subject is the difference do.
    #[must_use]
    pub fn of(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            texture: name.to_owned(),
            north: None,
            solid: true,
            replaceable: None,
            breakable: None,
            breaks_into: None,
        }
    }

    /// The same declaration, stating its texture as a table of six facings, all
    /// holding the block's own name except `north`, which holds `key`.
    ///
    /// **Five of the six keep the block's own name, and that is a constraint on
    /// the fixture rather than a convenience.** It is no longer about what draws:
    /// a face resolves the key its block declared. It is about what the block's
    /// key set *is* — the five that keep the name keep the layer a launch already
    /// gave them, so the one facing that moved is the only thing a reading has to
    /// account for, and the only key a reload has to find room for.
    ///
    /// **One facing and not two**, because a block whose `north` alone moved is
    /// exactly the edit a comparison reading a single key would accept while
    /// marking nothing — and it is the only edit that separates the six-key
    /// comparison from a one-key one.
    #[must_use]
    pub fn repointing_north(mut self, key: &str) -> Self {
        self.north = Some(key.to_owned());
        self
    }

    /// The same declaration, stating `solid` as given.
    #[must_use]
    pub const fn solid(mut self, solid: bool) -> Self {
        self.solid = solid;
        self
    }

    /// The same declaration, stating `replaceable`.
    #[must_use]
    pub const fn replaceable(mut self, replaceable: bool) -> Self {
        self.replaceable = Some(replaceable);
        self
    }

    /// The same declaration, stating `breakable`.
    #[must_use]
    pub const fn breakable(mut self, breakable: bool) -> Self {
        self.breakable = Some(breakable);
        self
    }

    /// The same declaration, stating the block a break leaves behind.
    #[must_use]
    pub fn breaking_into(mut self, residue: &str) -> Self {
        self.breaks_into = Some(residue.to_owned());
        self
    }

    /// The declaration as a Luau chunk returning a table.
    #[must_use]
    pub fn text(&self) -> String {
        let mut chunk = String::from("return {\n");
        chunk.push_str(&format!("\tname = \"{}\",\n", self.name));
        match &self.north {
            None => chunk.push_str(&format!("\ttexture = \"{}\",\n", self.texture)),
            Some(north) => chunk.push_str(&format!(
                "\ttexture = {{\n\
                 \t\tup = \"{own}\",\n\
                 \t\tdown = \"{own}\",\n\
                 \t\tnorth = \"{north}\",\n\
                 \t\tsouth = \"{own}\",\n\
                 \t\teast = \"{own}\",\n\
                 \t\twest = \"{own}\",\n\
                 \t}},\n",
                own = self.texture
            )),
        }
        chunk.push_str(&format!("\tsolid = {},\n", self.solid));
        if let Some(replaceable) = self.replaceable {
            chunk.push_str(&format!("\treplaceable = {replaceable},\n"));
        }
        if let Some(breakable) = self.breakable {
            chunk.push_str(&format!("\tbreakable = {breakable},\n"));
        }
        if let Some(residue) = &self.breaks_into {
            chunk.push_str(&format!("\tbreaks_into = \"{residue}\",\n"));
        }
        chunk.push_str("}\n");
        chunk
    }
}

/// The shipped content root, copied whole.
///
/// # Errors
///
/// Returns an error if the repository's content root cannot be located or
/// copied.
pub fn shipped() -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()
}

/// `root` with the declaration in `file_name` replaced by `declaration`.
///
/// # Errors
///
/// Returns an error if the write fails, or if the root does not declare that
/// file — a root that never declared the block is not a root whose declaration an
/// author changed, and a scenario about what the change does would be about
/// neither.
pub fn restating(
    root: ContentRoot,
    file_name: &str,
    declaration: &Declaration,
) -> Result<ContentRoot, Box<dyn Error>> {
    let declared = root.path().join(BLOCK_DIRECTORY).join(file_name);
    if !declared.is_file() {
        return Err(format!(
            "this fixture has to restate `{BLOCK_DIRECTORY}/{file_name}` in a copy of the shipped \
             content root, and the shipped root does not declare it. What it would build is a root \
             that gained a declaration rather than one whose declaration an author edited, and the \
             two are not the same claim"
        )
        .into());
    }
    fs::write(&declared, declaration.text())?;
    Ok(root)
}

/// `root` with a declaration written into a file it does not yet hold.
///
/// # Errors
///
/// Returns an error if the write fails, or if the root already declares that
/// file.
pub fn declaring(
    root: ContentRoot,
    file_name: &str,
    declaration: &Declaration,
) -> Result<ContentRoot, Box<dyn Error>> {
    root.declaring_block(file_name, &declaration.text())
}

/// A copy of the shipped root whose `stone.luau` says what `stone` says.
///
/// The shorthand every scenario about a changed block uses, so that the one
/// declaration under test is the only thing a test spells.
///
/// # Errors
///
/// Returns an error if the root cannot be copied or the declaration written.
pub fn shipped_restating_stone(stone: &Declaration) -> Result<ContentRoot, Box<dyn Error>> {
    restating(shipped()?, STONE_FILE, stone)
}

/// What a candidate says about `base:stone` when an author has taken its
/// solidity away and changed nothing else.
///
/// The edit every scenario about what a reload leaves untouched is driven with, so
/// that "the world is exactly as it was" is asserted beside something that plainly is
/// not.
#[must_use]
pub fn stone_that_is_not_solid() -> Declaration {
    Declaration::of(STONE).solid(false)
}

/// A copy of the shipped root none of whose four declarations states a solid
/// block.
///
/// Every declaration is restated rather than three of them removed, because a
/// root that stopped declaring blocks the world holds is refused for *that*
/// first — and the scenario would then be about a refusal it was not written for.
///
/// # Errors
///
/// Returns an error if the root cannot be copied or a declaration written.
pub fn shipped_declaring_nothing_solid() -> Result<ContentRoot, Box<dyn Error>> {
    let mut root = shipped()?;
    for (file_name, block) in [
        (DIRT_FILE, DIRT),
        (GRASS_FILE, GRASS),
        (STONE_FILE, STONE),
        (WATER_FILE, WATER),
    ] {
        root = restating(root, file_name, &Declaration::of(block).solid(false))?;
    }
    Ok(root)
}

/// A copy of the shipped root that also declares [`AMBER`], solid.
///
/// # Errors
///
/// Returns an error if the root cannot be copied or the declaration written.
pub fn shipped_declaring_amber() -> Result<ContentRoot, Box<dyn Error>> {
    declaring(shipped()?, AMBER_FILE, &amber())
}

/// What a candidate declares [`AMBER`] to be.
///
/// Every optional field is stated, and each is stated **against** its default —
/// `replaceable` is absent-means-false, `breakable` absent-means-true — so a
/// registry that answered from the defaults rather than from the declaration
/// answers differently on every one of them.
#[must_use]
pub fn amber() -> Declaration {
    Declaration::of(AMBER)
        .solid(true)
        .replaceable(true)
        .breakable(false)
        .breaking_into(DIRT)
}

/// Everything the content root at `root` declares, read through the one door a
/// candidate is built by, against a session that has spent no layers.
///
/// **The layers matter to the scenarios next door and not to these**, and saying
/// so is what keeps the two apart. The scenarios this serves ask what a swap does
/// to a world, a player and a held block; none of them reads a layer index, and
/// a session that has spent nothing is the state every one of them is in. A
/// scenario that *is* about the layer a key holds builds its candidate through
/// `reload_content::candidate_against`, which reads the assignment the client is
/// publishing — which is what a reload's build stage does.
///
/// # Errors
///
/// Returns whichever reader refused the root.
pub fn candidate(root: &Path) -> Result<LoadedContent, Box<dyn Error>> {
    Ok(mc_sim::content::load(root, &LayerAssignment::none())?)
}

/// What a running client answered when it was handed a candidate.
///
/// **Three refusals and two ways of having answered nothing at all**, which is
/// what makes this comparable as a whole: a client with no simulation and a
/// refusal this suite does not know about are separate arms rather than a shrug,
/// so a scenario expecting an acceptance cannot be satisfied by either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adoption {
    /// The candidate is now the content serving, and this is the block the
    /// player holds under it.
    Accepted { holding: String },
    /// The world holds these blocks and the candidate declares none of them, in
    /// the order the refusal named them.
    BlocksTheWorldHolds(Vec<String>),
    /// The candidate registers no solid block, said in these words.
    NothingToPlace { said: String },
    /// A refusal this suite does not recognise, rendered.
    Refused(String),
    /// There was no simulation to hand the candidate to.
    NoSimulation,
}

/// What [`Adoption`] a client's answer is.
///
/// The catch-all arm is deliberate and is not laziness: `ReloadRefusal` gains
/// variants in later phases, and a fixture that matched exhaustively would stop
/// compiling rather than reporting which refusal arrived.
#[must_use]
pub fn adoption(answered: Option<Result<Accepted, ReloadRefusal>>) -> Adoption {
    match answered {
        None => Adoption::NoSimulation,
        Some(Ok(accepted)) => Adoption::Accepted {
            holding: accepted.holding.as_str().to_owned(),
        },
        Some(Err(refused)) => refusal(&refused),
    }
}

/// What one refusal says, as an arm of [`Adoption`].
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
