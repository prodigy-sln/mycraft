//! The declared voxel worlds that have walls, ceilings and corners in them.
//!
//! [`super::solidity::Ground`] answers about *columns*: it says how high the
//! floor is at an `x` and knows nothing else, which is exactly what a scenario
//! about falling and standing needs and exactly what a scenario about walking
//! into a wall cannot use. This module is the other half — a world is declared
//! as the union of a handful of rectangular runs of solid voxels, so a test can
//! say "a floor, and one wall at x = 13" and have every expected position be
//! arithmetic over those two sentences.
//!
//! It is deliberately a **second** type rather than a variant added to `Ground`.
//! Two scenarios already in the suite are stated against `Ground::Void` and
//! `Ground::Flat` and depend on what those shapes do *not* contain — most
//! sharply the displacement bound's, whose 300-blocks-per-second rise is only
//! unresolved because nothing is above it. Widening the existing enum is how
//! that would quietly change from measuring the bound to measuring whichever of
//! the bound and a ceiling came first, with every assertion still green.
//!
//! **A voxel occupies `[v, v + 1)` on each axis**, which is what every position
//! in this feature's collision scenarios is derived from: the wall of voxels at
//! `x = 13` presents its near face at `x = 13.0`, so a box reaching
//! `HALF_WIDTH` either side of the feet stops with its feet at `12.7`. The wall
//! of voxels at `x = 7` presents its *far* face at `x = 8.0`, and a box arriving
//! from the other side stops at `8.3`. Both figures are the half-width
//! subtracted from, and added to, a declared integer — which is what makes the
//! player's 0.6-block width falsifiable here rather than carried by derivation.
//!
//! **What these shapes would fail to catch if they were built differently.**
//! A wall is **one voxel thick**, never a half-space, so a resolver that snapped
//! the box to the wrong face of the blocking voxel puts it *inside* or *beyond*
//! the wall rather than landing on the same answer a half-space would have
//! given. A wall spans the axes it is not about, so a walk parallel to it never
//! runs off an end and no test's answer depends on where a fixture stops.
//! [`Slab::voxel`] declares a **single** voxel, which is the only shape that can
//! discriminate the order two axes are resolved in: anything symmetric about the
//! diagonal gives the same answer whichever axis moved first.

use std::error::Error;
use std::sync::Arc;

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_sim::player::{BlockPos, Solidity};
use mc_sim::simulation::PublishedContent;
use mc_sim::world::World;
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};

use super::described;

/// How far a floor, a wall or a ceiling runs on the axes it is not about.
///
/// Far enough on either side that no walk in this suite reaches an end of one,
/// and finite so that no bound of a declared region is an `i32` extreme.
const SPAN: (i32, i32) = (-64, 192);

/// How high a wall reaches. Any value above the player's box would do; this one
/// is the same order as the world's own height so a wall reads as a wall.
const WALL_TOP: i32 = 192;

/// A rectangular run of solid voxels, half-open on every axis in voxel
/// coordinates: `x` covers `x.0` up to but not including `x.1`.
#[derive(Debug, Clone, Copy)]
pub struct Slab {
    x: (i32, i32),
    y: (i32, i32),
    z: (i32, i32),
}

impl Slab {
    /// Everything from `y = 0` up to and including `surface`, everywhere
    /// horizontally: a floor whose top face is at `surface + 1`.
    #[must_use]
    pub const fn floor(surface: i32) -> Self {
        Self {
            x: SPAN,
            y: (0, surface + 1),
            z: SPAN,
        }
    }

    /// A wall one voxel thick standing in the column `x`, so its near face is at
    /// `x` and its far face at `x + 1`.
    #[must_use]
    pub const fn wall_at_x(x: i32) -> Self {
        Self {
            x: (x, x + 1),
            y: (0, WALL_TOP),
            z: SPAN,
        }
    }

    /// A wall one voxel thick standing in the column `z`.
    #[must_use]
    pub const fn wall_at_z(z: i32) -> Self {
        Self {
            x: SPAN,
            y: (0, WALL_TOP),
            z: (z, z + 1),
        }
    }

    /// A slab one voxel thick lying at height `y`, so its bottom face is at `y`.
    #[must_use]
    pub const fn ceiling_at(y: i32) -> Self {
        Self {
            x: SPAN,
            y: (y, y + 1),
            z: SPAN,
        }
    }

    /// One solid voxel, and nothing around it.
    #[must_use]
    pub const fn voxel(x: i32, y: i32, z: i32) -> Self {
        Self {
            x: (x, x + 1),
            y: (y, y + 1),
            z: (z, z + 1),
        }
    }

    /// Whether this run holds the voxel at `at`.
    const fn holds(self, at: BlockPos) -> bool {
        within(at.x, self.x) && within(at.y, self.y) && within(at.z, self.z)
    }
}

/// Whether `value` lies in the half-open range `range`.
const fn within(value: i32, range: (i32, i32)) -> bool {
    range.0 <= value && value < range.1
}

/// A declared world: the union of the runs it was built from, and nothing else.
///
/// Everything outside every run is not solid, which is what lets a fixture say
/// what it means by listing what is there rather than by describing what is not.
#[derive(Debug, Clone, Default)]
pub struct Chamber(Vec<Slab>);

impl Chamber {
    /// A world holding exactly these runs of solid voxels.
    #[must_use]
    pub fn of(slabs: impl IntoIterator<Item = Slab>) -> Self {
        Self(slabs.into_iter().collect())
    }
}

impl Solidity for Chamber {
    fn is_solid(&self, at: BlockPos) -> bool {
        self.0.iter().any(|slab| slab.holds(at))
    }
}

/// What the fixture registry's own three blocks are called.
///
/// **Why the fixtures register blocks of their own at all.** MVP 1's base game
/// deliberately ships no indestructible block — that is a scope choice recorded
/// in the spec, not a fact about how breakability happens to be encoded — so a
/// scenario about a block that cannot be broken has to bring its own. Reaching
/// for a shipped block instead would mean reaching for one that is not solid and
/// therefore can never be *targeted*, and the scenario would go green because
/// nothing was targeted rather than because breakability was respected.
///
/// [`CRUMBLING`] exists for the opposite reason. Breaking any shipped block
/// empties its cell, so a break that emptied the cell unconditionally would
/// satisfy "the block its own definition names" for all of them; this one names
/// dirt, which nothing else in a fixture would produce.
///
/// [`UNBUILDABLE`] is the block where **`replaceable` and `!is_solid`
/// disagree**, and it is the whole reason placement legality can be measured at
/// all. Every block the base game ships has the two agreeing — air and water are
/// non-solid and replaceable, dirt, grass and stone are solid and not — so
/// against shipped content alone a placement check reading `!is_solid` and one
/// reading `replaceable` answer identically at every cell, and the field ruling
/// 62 added is decoration behind a green suite. This one is **not solid and not
/// replaceable**: a player walks through it and a placement may not overwrite
/// it. It is the cheapest of the two disagreeing shapes, and it is what makes a
/// placement into it refused under the right reading and allowed under the wrong
/// one.
///
/// [`AIMABLE`] and [`UNAIMABLE`] are the pair in which **`targetable` and
/// `is_solid` disagree**, one in each direction, and they exist for the reason
/// [`UNBUILDABLE`] does one field along: every block content ships has the two
/// agreeing except water, so against shipped content alone a walk stopping at
/// the first solid cell and one stopping at the first targetable cell answer
/// identically everywhere. [`AIMABLE`] stops nobody and a ray stops at it;
/// [`UNAIMABLE`] stops a player and a ray goes straight through it. A rule
/// reading solidity where it means targetability reports the wrong cell in both,
/// and reports it in opposite directions, which is why there are two.
///
/// [`BUILDABLE`] is the block that is **solid and replaceable at once**. Nothing
/// content ships is: water is replaceable and stops nobody, and dirt, grass and
/// stone stop a player and may not be built over. It is what lets a placement be
/// aimed at a cell a ray already stops at *today*, so a rule about where a
/// placement lands when the cell it hit is itself replaceable can be measured
/// without waiting for anything to become targetable.
pub const UNBREAKABLE: &str = "fixture:unbreakable";
pub const CRUMBLING: &str = "fixture:crumbling";
pub const UNBUILDABLE: &str = "fixture:unbuildable";
pub const AIMABLE: &str = "fixture:aimable";
pub const UNAIMABLE: &str = "fixture:unaimable";
pub const BUILDABLE: &str = "fixture:buildable";

/// What the fixture registry's definitions are attributed to.
const FIXTURE_ORIGIN: &str = "a break-and-place test's declared registry";

/// One block a fixture registry declares.
///
/// A named struct rather than a tuple because five positional fields, three of
/// them booleans, is a row a reader has to count their way across — and getting
/// `replaceable` and `breakable` the wrong way round is a fixture that still
/// builds and still passes.
struct Declared {
    name: &'static str,
    is_solid: bool,
    /// Whether a ray may stop at this block, which is a separate claim from
    /// whether it stops a player and is stated per block for that reason.
    targetable: bool,
    replaceable: bool,
    breakable: bool,
    breaks_into: Option<&'static str>,
}

/// One ordinary solid block: it stops a player, nothing may be built over it,
/// it can be broken, and breaking it empties the cell.
const fn solid(name: &'static str) -> Declared {
    Declared {
        name,
        is_solid: true,
        targetable: true,
        replaceable: false,
        breakable: true,
        breaks_into: None,
    }
}

/// One block that stops nothing and that anything may be built over.
const fn open(name: &'static str) -> Declared {
    Declared {
        name,
        is_solid: false,
        targetable: false,
        replaceable: true,
        breakable: true,
        breaks_into: None,
    }
}

/// The four names content ships, declared as this fixture needs them.
///
/// **It borrows content's *names*, not its *declarations*, and the difference is
/// load-bearing rather than an oversight.** Content declares `base:water`
/// `breakable = false`, `drawn = true` and `targetable = true`; here it is
/// declared breakable and neither drawn nor targetable. Do not "correct" it:
/// `block_breaking.rs` builds a chamber whose *background* is water, to tell
/// "this cell was emptied" apart from "this cell holds the background", and a
/// targetable background stops every ray in that fixture at the first cell it
/// crosses. The scenarios that are about what content really declares build over
/// `super::content_registry()` and read the shipped root.
///
/// **The order is the file-name order `LuauFileDefinitionSource` reads them in,
/// and that is load-bearing** — base content applies before the `fixture:`
/// overlay, so ids are assigned in this order. The assertion that pins it lives
/// in the test file that depends on it, because this list alone cannot say why
/// the order matters.
///
/// **Every block here stops a player except water.** Content declares no block
/// meaning empty space, so the first name in this list is also the first solid
/// one — which is why a scenario about a rule that reads *solidity* out of a
/// registration order has to declare a registry whose first block is not solid
/// rather than reach for this one.
const BASE_CONTENT: [Declared; 4] = [
    solid("base:dirt"),
    solid("base:grass"),
    solid("base:stone"),
    open("base:water"),
];

/// The blocks the `fixture:` overlay adds over base content.
const OVERLAY: [Declared; 6] = [
    Declared {
        name: UNBREAKABLE,
        is_solid: true,
        targetable: true,
        replaceable: false,
        breakable: false,
        breaks_into: None,
    },
    Declared {
        name: CRUMBLING,
        is_solid: true,
        targetable: true,
        replaceable: false,
        breakable: true,
        breaks_into: Some("base:dirt"),
    },
    Declared {
        name: UNBUILDABLE,
        is_solid: false,
        targetable: false,
        replaceable: false,
        breakable: true,
        breaks_into: None,
    },
    Declared {
        name: AIMABLE,
        is_solid: false,
        targetable: true,
        replaceable: false,
        breakable: true,
        breaks_into: None,
    },
    Declared {
        name: UNAIMABLE,
        is_solid: true,
        targetable: false,
        replaceable: false,
        breakable: true,
        breaks_into: None,
    },
    Declared {
        name: BUILDABLE,
        is_solid: true,
        targetable: true,
        replaceable: true,
        breakable: true,
        breaks_into: None,
    },
];

/// A registry holding what content ships, with a `fixture:` overlay over it.
///
/// **Base content applies first and the overlay second**, through two separate
/// batches, exactly as the running game applies a content pack over the base
/// game — so ids are assigned in that order and a repeated name would be
/// refused, which is why the overlay uses a namespace of its own.
///
/// It never reads `content/base` from disk: a test binary's working directory is
/// not the repository root.
///
/// # Errors
///
/// Returns the refusal if a name is not a namespaced id or the registry refuses
/// a batch.
pub fn fixture_registry() -> Result<Arc<BlockRegistry>, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    for (label, batch) in [
        ("base content", &BASE_CONTENT[..]),
        ("overlay", &OVERLAY[..]),
    ] {
        registry.apply(&InMemoryDefinitionSource::new(
            DefinitionOrigin::new(format!("{FIXTURE_ORIGIN}, {label}")),
            declaring(batch, label)?,
        ))?;
    }
    Ok(Arc::new(registry))
}

/// The content a simulation over [`fixture_registry`] publishes at launch.
///
/// Named beside the registry it resolves rather than derived at each call site,
/// because a simulation built over these blocks and publishing content for
/// somebody else's would be a fixture disagreeing with itself.
///
/// # Errors
///
/// Returns the refusal if the registry does not apply, if a registered id cannot
/// be read back, or if the layers do not fit a session's budget.
pub fn fixture_content() -> Result<PublishedContent, Box<dyn Error>> {
    super::published_content(fixture_registry()?.as_ref())
}

/// One batch of definitions, ready for the source to yield.
fn declaring(
    batch: &[Declared],
    label: &str,
) -> Result<
    Vec<Result<BlockDefinition, mc_core::block::source::DefinitionSourceError>>,
    Box<dyn Error>,
> {
    let origin = DefinitionOrigin::new(format!("{FIXTURE_ORIGIN}, {label}"));
    let mut declared = Vec::with_capacity(batch.len());
    for block in batch {
        declared.push(Ok(BlockDefinition {
            name: BlockName::parse(block.name)?,
            textures: FaceTextures::uniform(TextureKey::parse(block.name)?),
            is_solid: block.is_solid,
            replaceable: block.replaceable,
            breakable: block.breakable,
            breaks_into: block.breaks_into.map(BlockName::parse).transpose()?,
            drawn: block.is_solid,
            occludes: block.is_solid,
            targetable: block.targetable,
            origin: origin.clone(),
        }));
    }
    Ok(declared)
}

/// A world position, spelled short enough to sit in a `const`.
#[must_use]
pub const fn at(x: u32, y: u32, z: u32) -> WorldPos {
    WorldPos { x, y, z }
}

/// A declared world of *named blocks*, in [`Chamber`]'s style: say what is
/// there, never what is not.
///
/// The declaration is kept rather than the world it produces, so a test can
/// build the same fixture twice — once to drive and once to compare against —
/// and know the second is the first as declared and not a copy of a run.
#[derive(Debug, Clone)]
pub struct BlockChamber {
    columns: u32,
    /// What every cell holds before any run is written over it: a named block,
    /// or nothing at all.
    ///
    /// **Both are real declarations and neither is the other's default.** A
    /// chamber whose background is *nothing* is the ordinary case, and one whose
    /// background is a named non-solid block is what a scenario reaches for when
    /// it needs to tell "this cell was emptied" apart from "this cell was filled
    /// with whatever the world calls its background".
    fill: Contents<&'static str>,
    runs: Vec<Run>,
}

/// One declared run: the half-open box it covers, and the block written over it.
type Run = (WorldPos, WorldPos, &'static str);

impl BlockChamber {
    /// A world of `columns` squared chunk columns, every cell of which holds
    /// nothing.
    ///
    /// Takes no registry and cannot fail: nothing is not a block, so there is
    /// nothing here for a registry to know about.
    #[must_use]
    pub fn empty(columns: u32) -> Self {
        Self {
            columns,
            fill: Contents::Empty,
            runs: Vec::new(),
        }
    }

    /// A world of `columns` squared chunk columns, every voxel holding `fill`.
    #[must_use]
    pub fn filled_with(columns: u32, fill: &'static str) -> Self {
        Self {
            columns,
            fill: Contents::Holds(fill),
            runs: Vec::new(),
        }
    }

    /// The same world with `block` written over the half-open box from `low` up
    /// to but not including `high`.
    #[must_use]
    pub fn run(mut self, low: WorldPos, high: WorldPos, block: &'static str) -> Self {
        self.runs.push((low, high, block));
        self
    }

    /// The same world with `block` written at one cell.
    #[must_use]
    pub fn cell(self, at: WorldPos, block: &'static str) -> Self {
        let beyond = WorldPos {
            x: at.x + 1,
            y: at.y + 1,
            z: at.z + 1,
        };
        self.run(at, beyond, block)
    }

    /// The world this declaration describes, over the fixture registry.
    ///
    /// **A chamber declared empty is built empty rather than built out of some
    /// block that stands in for emptiness.** Breaking a block in it leaves the
    /// cell holding nothing, which is what the world itself now says rather than
    /// a name the fixture had to choose; a chamber declared with a named
    /// background still gets that block everywhere, and a break in *it* still
    /// leaves nothing — which is the difference that makes a named background
    /// worth declaring.
    ///
    /// # Errors
    ///
    /// Returns the refusal if the registry does not apply, a declared name is
    /// not registered, or a run reaches outside the world.
    pub fn build(&self) -> Result<World, Box<dyn Error>> {
        let registry = fixture_registry()?;
        let mut blocks = match self.fill {
            Contents::Empty => VoxelWorld::empty(self.columns),
            Contents::Holds(name) => {
                VoxelWorld::filled(self.columns, &BlockName::parse(name)?, &registry)?
            }
        };
        for &run in &self.runs {
            written(&mut blocks, run, &registry)?;
        }
        Ok(World::new(blocks, registry)?)
    }
}

/// Writes `block` over the half-open box from `low` to `high`.
fn written(
    blocks: &mut VoxelWorld,
    run: Run,
    registry: &BlockRegistry,
) -> Result<(), Box<dyn Error>> {
    let (low, high, block) = run;
    let name = BlockName::parse(block)?;
    for at in every_cell(low, high) {
        blocks.set_block(at, &name, registry)?;
    }
    Ok(())
}

/// Every cell of the half-open box from `low` to `high`.
fn every_cell(low: WorldPos, high: WorldPos) -> impl Iterator<Item = WorldPos> {
    (low.y..high.y).flat_map(move |y| {
        (low.z..high.z).flat_map(move |z| (low.x..high.x).map(move |x| WorldPos { x, y, z }))
    })
}

/// Every cell at which two worlds hold different contents: where, what the first
/// holds, and what the second holds instead — a block by name, or
/// [`super::NOTHING`] where the cell holds none.
///
/// Both worlds are walked whole rather than at cells a caller nominates, so a
/// scenario expecting one change fails on a second one it did not ask about —
/// an edit that took the right cell *and* another is not a correct edit.
#[must_use]
pub fn differences(declared: &World, after: &World) -> Vec<(WorldPos, String, String)> {
    declared
        .extent()
        .positions()
        .filter_map(|at| difference_at(declared, after, at))
        .collect()
}

/// What the two worlds disagree about at one cell, if they disagree at all.
fn difference_at(
    declared: &World,
    after: &World,
    at: WorldPos,
) -> Option<(WorldPos, String, String)> {
    let signed = BlockPos {
        x: at.x as i32,
        y: at.y as i32,
        z: at.z as i32,
    };
    let (was, now) = (declared.block_at(signed)?, after.block_at(signed)?);
    (was != now).then(|| (at, described(was), described(now)))
}
