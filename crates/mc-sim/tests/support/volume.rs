//! A declared volume of named voxels, and the registry that says what those
//! names are.
//!
//! The replay's own world can only hold the five blocks the scripted scene
//! places, so a scenario about a block whose *definition* disagrees with what
//! its name suggests cannot be stated against it: there is no way to put a
//! `mod:cloud` into a generated world, and no way to make its `base:stone`
//! anything other than what content declares. This module is the other half —
//! a volume a test declares block by block, resolved through a registry the same
//! test declares definition by definition.
//!
//! **Block names appear here in full, and that is allowed.** Files under
//! `tests/` are not read by `mc-world`'s hardcoded-name scan; the invariant it
//! enforces is that no *engine* code decides what a block is, and a fixture
//! asserting that solidity comes from a definition has to be able to say which
//! name carries which definition — otherwise it is asserting nothing.
//!
//! **What these fixtures would fail to catch if they were built differently.**
//! [`NamedSlab`] is uniform on x and z, so it cannot discriminate a query that
//! read a box's z where it meant its x; that is what the walls of
//! `crates/mc-sim/tests/support/chamber.rs` are for, and it is pinned in phase
//! 4. What it *is* built to catch is the only thing it can: the slab's name and
//! the filler's name are declared with opposite solidity in each of the two
//! scenarios that use it, and the two scenarios choose names that point the
//! wrong way round — a shipped name for a hollow block, an invented one for a
//! solid block — so neither a hardcoded name nor a hardcoded list of known names
//! survives both.

use std::error::Error;

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_sim::replay::{BlockVolume, Extent};
use mc_world::section::Contents;
use mc_world::world::WorldPos;

/// What every registry declared here is attributed to. Nothing asserts it; a
/// definition has to say where it came from.
const FIXTURE_ORIGIN: &str = "a solidity test's declared registry";

/// How fast a volume lifts a swimmer when its declaration says nothing about it:
/// the speed the player's own jump leaves the ground at.
///
/// Written out here rather than read from the loader, which keeps it a fixture's
/// own statement rather than an agreement with whatever the value under test
/// becomes. A fixture that means it says so with this name; a fixture whose
/// scenario is about the ascent states its own number instead.
pub const AN_UNSTATED_ASCENT: f32 = 9.0;

/// A volume filled with one named block up to and including a declared height,
/// and with another above it.
///
/// Two names and nothing else, because two is what the scenarios need: the block
/// being asked about, and something above it for the player to fall through
/// before it gets there.
#[derive(Debug, Clone)]
pub struct NamedSlab {
    extent: Extent,
    top: u32,
    filling: BlockName,
    above: BlockName,
}

impl NamedSlab {
    /// A volume of `extent` holding `filling` from `y = 0` up to and including
    /// `top`, and `above` everywhere over it.
    ///
    /// # Errors
    ///
    /// Returns an error if either name is not a namespaced block name.
    pub fn new(
        extent: Extent,
        top: u32,
        filling: &str,
        above: &str,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            extent,
            top,
            filling: BlockName::parse(filling)?,
            above: BlockName::parse(above)?,
        })
    }
}

impl BlockVolume for NamedSlab {
    fn extent(&self) -> Extent {
        self.extent
    }

    /// **A declared volume says what is there, so its answer comes from the
    /// declaration and from nowhere else.** Every cell inside this volume holds
    /// one of the two blocks it was declared with — that is what makes it a
    /// *slab* — so `Contents::Empty` is not an answer it can give, and the
    /// outer `None` keeps the one meaning it has always had: this position is
    /// outside the volume. Deriving either answer from anything the simulation
    /// resolved would make this fixture agree with whatever it was handed.
    fn block_at(&self, x: u32, y: u32, z: u32) -> Option<Contents<&BlockName>> {
        let inside = x < self.extent.x && y < self.extent.y && z < self.extent.z;
        let held = if y <= self.top {
            &self.filling
        } else {
            &self.above
        };
        inside.then_some(Contents::Holds(held))
    }
}

/// What a block declares about the questions this suite's fixtures ask of one:
/// whether it stops a player, whether it is drawn, whether it hides what is
/// behind it, whether a ray may stop at it, whether a player can hold itself up
/// in its volume, how much that volume slows what moves through it, and how fast
/// it lifts a swimmer who asks to rise.
///
/// **Separate answers, because a fixture that cannot state them separately
/// cannot fail a rule that reads them all off solidity.** Written as a struct
/// rather than as positional fields, so a declaration reads the way its scenario
/// is worded and getting two of them the wrong way round is a fixture that no
/// longer compiles rather than one that still passes.
///
/// It is deliberately not [`Eq`]: a resistance is a number, and a fixture
/// comparing two declarations for exact equality would be asking a question
/// about bits it has no reason to ask.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Declaration {
    pub solid: bool,
    pub drawn: bool,
    pub occludes: bool,
    pub targetable: bool,
    pub swimmable: bool,
    pub move_resistance: f32,
    pub swim_ascent: f32,
}

impl Declaration {
    /// What every fixture written before the four drawing-and-aiming answers
    /// were separable means: all four are the block's solidity.
    ///
    /// **The medium is not among them.** All three medium answers are constants
    /// here and are never derived from `solid`: a declaration whose buoyancy
    /// followed its solidity would make either the air or every wall swimmable,
    /// and no assertion written against a fixture can see its own fixture lying.
    #[must_use]
    pub const fn like_solidity(solid: bool) -> Self {
        Self {
            solid,
            drawn: solid,
            occludes: solid,
            targetable: solid,
            swimmable: false,
            move_resistance: 0.0,
            swim_ascent: AN_UNSTATED_ASCENT,
        }
    }

    /// The same declaration, stating a medium: whether a player can hold itself
    /// up in this block's volume, how much that volume slows what moves through
    /// it, and how fast it lifts a swimmer who asks to rise.
    ///
    /// **All three in one call**, never some of them at a time, and never with a
    /// defaulted argument. They are independent declarations, and a builder that
    /// could state one and leave another standing is how a resistance nobody
    /// wrote arrives under a buoyancy somebody did — which is the one thing the
    /// fixtures for a swimmable block that resists nothing, and for a resistant
    /// block nobody can swim in, exist to tell apart. A third field makes that
    /// hazard wider rather than smaller, so the argument is required and a call
    /// site that meant the loader's default says so in full.
    #[must_use]
    pub const fn stating_a_medium(
        self,
        swimmable: bool,
        move_resistance: f32,
        swim_ascent: f32,
    ) -> Self {
        Self {
            solid: self.solid,
            drawn: self.drawn,
            occludes: self.occludes,
            targetable: self.targetable,
            swimmable,
            move_resistance,
            swim_ascent,
        }
    }
}

/// A block that is seen and may be aimed at, and that stops nobody and hides
/// nothing — the shape the shipped water has, stated without borrowing its name.
///
/// It is the one declaration in which a rule reading *drawnness* or
/// *targetability* where it meant *collision* gives a different answer from the
/// rule as written, which is the whole of what a registry built out of it can
/// prove.
pub const DRAWN_AND_AIMED_AT_ONLY: Declaration = Declaration {
    solid: false,
    drawn: true,
    occludes: false,
    targetable: true,
    swimmable: false,
    move_resistance: 0.0,
    swim_ascent: AN_UNSTATED_ASCENT,
};

/// A registry holding exactly `blocks`, each carrying the solidity declared
/// beside it and textured by its own name.
///
/// Every one of the four questions a definition answers is answered by that
/// solidity, which is what a fixture written before they were separable meant —
/// see [`registry_of_declarations`] for the builder that can state them apart.
///
/// Built through the in-memory definition source because a registry has no other
/// door in — which is the same structural rule content goes through, so a
/// declared definition here is indistinguishable to the engine from a shipped
/// one.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id, or if the registry refuses
/// the batch.
pub fn registry_declaring(blocks: &[(&str, bool)]) -> Result<BlockRegistry, Box<dyn Error>> {
    let declared: Vec<(&str, Declaration)> = blocks
        .iter()
        .map(|&(name, is_solid)| (name, Declaration::like_solidity(is_solid)))
        .collect();
    registry_of_declarations(&declared)
}

/// A registry holding exactly `blocks`, in the order given, each carrying the
/// declaration beside it and textured by its own name.
///
/// The one place these fixtures' definitions are built, so that a fixture stating
/// four answers and a fixture stating one reach the registry by the same route
/// rather than by two that could drift.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id, or if the registry refuses
/// the batch.
pub fn registry_of_declarations(
    blocks: &[(&str, Declaration)],
) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut declared = Vec::with_capacity(blocks.len());
    for &(name, states) in blocks {
        // The scenarios these registries serve resolve what a block is declared
        // to be and never breakability, replaceability or a residue, so each of
        // those is left at what a declaration saying nothing about it means; the
        // fixtures that do break blocks declare their own registry.
        declared.push(Ok(BlockDefinition {
            name: BlockName::parse(name)?,
            textures: FaceTextures::uniform(TextureKey::parse(name)?),
            is_solid: states.solid,
            replaceable: false,
            breakable: true,
            breaks_into: None,
            drawn: states.drawn,
            occludes: states.occludes,
            targetable: states.targetable,
            // Read off the declaration beside the name, never off its solidity:
            // a medium derived here would make the air swimmable and no
            // assertion written against such a fixture could see it.
            swimmable: states.swimmable,
            move_resistance: states.move_resistance,
            swim_ascent: states.swim_ascent,
            origin: DefinitionOrigin::new(FIXTURE_ORIGIN),
        }));
    }
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        DefinitionOrigin::new(FIXTURE_ORIGIN),
        declared,
    ))?;
    Ok(registry)
}

/// A volume of declared cells: every cell holds nothing, except where a run says
/// otherwise.
///
/// [`NamedSlab`] cannot give an empty answer — every cell of a slab holds one of
/// the two blocks it was declared with, which is what makes it a slab — so a
/// scenario about what an *empty* cell contributes has no way to state itself
/// against one. This is that other half, and it is what the medium fixtures are
/// declared against: a resistance is a property of a block's volume, so "a cell
/// holding no block contributes nothing" needs a cell holding no block.
///
/// **Runs are written in the order they are declared and a later run wins**, so
/// a floor laid first and a band written over it read the way they are written
/// rather than in whichever order the walk happens to reach them.
#[derive(Debug, Clone)]
pub struct Cells {
    extent: Extent,
    runs: Vec<(WorldPos, WorldPos, BlockName)>,
}

impl Cells {
    /// A volume of `extent` in which every cell holds nothing.
    #[must_use]
    pub const fn empty(extent: Extent) -> Self {
        Self {
            extent,
            runs: Vec::new(),
        }
    }

    /// The same volume with `block` written over the half-open box from `low` up
    /// to but not including `high`.
    ///
    /// # Errors
    ///
    /// Returns an error if `block` is not a namespaced block name.
    pub fn holding(
        mut self,
        low: WorldPos,
        high: WorldPos,
        block: &str,
    ) -> Result<Self, Box<dyn Error>> {
        self.runs.push((low, high, BlockName::parse(block)?));
        Ok(self)
    }
}

impl BlockVolume for Cells {
    fn extent(&self) -> Extent {
        self.extent
    }

    /// **Three answers and not two.** A cell this volume reaches that no run
    /// covers holds nothing, a cell a run covers holds that run's block, and a
    /// position outside the extent is neither — which is the distinction every
    /// scenario about what lies beyond the world's volume is stated on.
    fn block_at(&self, x: u32, y: u32, z: u32) -> Option<Contents<&BlockName>> {
        let at = WorldPos { x, y, z };
        if !self.extent.contains(at) {
            return None;
        }
        let held = self
            .runs
            .iter()
            .rev()
            .find(|&&(low, high, _)| covers(low, high, at));
        Some(held.map_or(Contents::Empty, |(_, _, name)| Contents::Holds(name)))
    }
}

/// Whether the half-open box from `low` up to but not including `high` covers
/// `at`.
const fn covers(low: WorldPos, high: WorldPos, at: WorldPos) -> bool {
    low.x <= at.x
        && at.x < high.x
        && low.y <= at.y
        && at.y < high.y
        && low.z <= at.z
        && at.z < high.z
}
