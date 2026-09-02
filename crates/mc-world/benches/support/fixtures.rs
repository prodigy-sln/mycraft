//! The three sections the meshing benchmark is measured on, and the registries
//! they are meshed against.
//!
//! All three are committed rather than generated at run time, and none of them
//! goes near worldgen: the numbers a benchmark reports are only comparable across
//! months if the thing being measured has not moved, and the renderer that comes
//! next inherits whatever quad counts these produce. The terrain heights
//! therefore come from a fixed integer hash written out in this file, not from a
//! noise library and not from the world generator.
//!
//! **The terrain surface is spatially coherent, and that is not a stylistic
//! choice.** The specification characterises this fixture as long mergeable flat
//! runs broken by a rough surface, which is what a surface chunk actually looks
//! like. Per-column white noise satisfies every scenario written about this
//! fixture — it was measured during the architecture review at roughly 0.29
//! quads per visible face, comfortably inside the "at most half" ceiling — while
//! being the exact opposite workload. A fixture that is wrong in *shape* is
//! invisible to a count-based oracle, so the budget would end up enforced
//! against work the terrain never does. Coherence is built in here instead: a
//! plateau term constant across four columns carries most of the range, and a
//! roughness term constant across two columns breaks it up.
//!
//! Nothing in this file may carry a quantity read off a mesher run. The expected
//! work for each fixture is derived elsewhere and derived from first principles —
//! six faces on a cube, one per facing; 2048 alternating solid voxels times six;
//! and, for terrain, no committed number at all, only the independent oracle's
//! own count. A number snapshotted from the first green run would commit whatever
//! the mesher did that day, including nothing.

use std::error::Error;

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin, Opacity};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, NamespacedIdError, TextureKey};
use mc_world::section::{Contents, LocalPos, PaletteIndex, SECTION_SIZE, Section, SectionData};

/// How many columns a section's footprint has.
pub const TERRAIN_COLUMNS: usize = (SECTION_SIZE * SECTION_SIZE) as usize;

/// The two blocks every fixture is built from.
///
/// An `example:` namespace, never `base:`. The hardcoded-name scan only walks
/// `crates/*/src`, so a shipped name here would not be caught by it — and it
/// would still be the engine's own benchmark knowing a block the base game ships
/// by name.
const SOLID_BLOCK: &str = "example:mesher_stone";
const NON_SOLID_BLOCK: &str = "example:mesher_void";

/// What the fixtures' definitions are attributed to. Nothing asserts it; a
/// definition has to say where it came from.
const FIXTURE_ORIGIN: &str = "the meshing benchmark's fixtures";

/// How far one axis is shifted in a voxel's linear index, and the mask that
/// reads it back. Shifts and masks throughout, because `integer_division` is a
/// gate error and it applies to bench targets too.
const AXIS_SHIFT: u32 = SECTION_SIZE.trailing_zeros();
const AXIS_MASK: u32 = SECTION_SIZE - 1;

/// The fixed integer hash the terrain heights come from — a plain
/// multiply-xorshift finalizer, written out so that the fixture means the same
/// thing in five years' time.
const X_STRIDE: u32 = 0x27d4_eb2d;
const Z_STRIDE: u32 = 0x1656_67b1;
const FIRST_ROUND: u32 = 0x85eb_ca6b;
const SECOND_ROUND: u32 = 0xc2b2_ae35;

/// The two terms a height is the sum of, each salted differently so the same
/// hash can serve both.
///
/// The plateau term is constant across a four-column square and carries six of
/// the nine heights; the roughness term is constant across a two-column square
/// and carries two. That ordering is what makes the surface read as flat ground
/// with texture on it rather than as noise: the smallest flat run is 2x2, and
/// runs of 4x4 and wider are common where neighbouring squares agree.
const PLATEAU_SALT: u32 = 0x9e37_79b9;
const ROUGHNESS_SALT: u32 = 0x7feb_352d;
const PLATEAU_SHIFT: u32 = 2;
const ROUGHNESS_SHIFT: u32 = 1;

/// The heights the two terms range over, as bit counts rather than as a modulus:
/// `integer_division` bans the remainder's natural sibling, and a population
/// count needs neither a division nor a lookup table an index could fall off.
///
/// Six bits give 0..=6 with a mean of 3, two give 0..=2 with a mean of 1, so a
/// height lies in `4..=12` with a mean of 8 — half of a section's sixteen, which
/// is what "roughly half full" means here.
const LOWEST_HEIGHT: u32 = 4;
const PLATEAU_BITS: u32 = 0x3f;
const ROUGHNESS_BITS: u32 = 0x3;

/// A committed fixture and the registry it is meshed against.
///
/// The registry travels with the section because solidity is a property of a
/// registered definition and of nothing else — a fixture handed to the wrong
/// registry would be a different fixture.
#[derive(Debug)]
pub struct Fixture {
    pub section: Section,
    pub registry: BlockRegistry,
}

/// The height declared for every column of the terrain fixture, ordered x
/// fastest then z.
///
/// This is the declaration the fixture is built *from*, exposed so that what was
/// built can be checked against what was declared. Rebuilding the fixture and
/// comparing the two copies would assert only that the builder is a function.
#[must_use]
pub fn terrain_heights() -> [u32; TERRAIN_COLUMNS] {
    std::array::from_fn(|column| {
        let position = column as u32;
        height_at(position & AXIS_MASK, position >> AXIS_SHIFT)
    })
}

/// Solid below each column's declared height and non-solid above it.
///
/// # Errors
///
/// Returns an error if the fixture's own block names do not parse, or if its
/// registry refuses them.
pub fn terrain() -> Result<Fixture, Box<dyn Error>> {
    let declared = terrain_heights();
    built_from(|voxel| {
        declared
            .get(column_of(voxel))
            .is_some_and(|height| voxel.y < *height)
    })
}

/// Entirely solid.
///
/// # Errors
///
/// Returns an error if the fixture's own block names do not parse, or if its
/// registry refuses them.
pub fn solid() -> Result<Fixture, Box<dyn Error>> {
    let registry = fixture_registry()?;
    let section = Section::filled(&BlockName::parse(SOLID_BLOCK)?, &registry)?;
    Ok(Fixture { section, registry })
}

/// Alternating solid and non-solid on every axis, so no two visible faces of the
/// same facing are ever adjacent and nothing merges.
///
/// # Errors
///
/// Returns an error if the fixture's own block names do not parse, or if its
/// registry refuses them.
pub fn checkerboard() -> Result<Fixture, Box<dyn Error>> {
    built_from(|voxel| (voxel.x + voxel.y + voxel.z) & 1 == 0)
}

/// Which of the 256 columns `voxel` stands in, ordered x fastest then z — the
/// same ordering [`terrain_heights`] is written in.
const fn column_of(voxel: LocalPos) -> usize {
    (voxel.x | (voxel.z << AXIS_SHIFT)) as usize
}

/// The height declared for the column at `x`, `z`.
const fn height_at(x: u32, z: u32) -> u32 {
    let plateau = hashed(x >> PLATEAU_SHIFT, z >> PLATEAU_SHIFT, PLATEAU_SALT) & PLATEAU_BITS;
    let roughness =
        hashed(x >> ROUGHNESS_SHIFT, z >> ROUGHNESS_SHIFT, ROUGHNESS_SALT) & ROUGHNESS_BITS;
    LOWEST_HEIGHT + plateau.count_ones() + roughness.count_ones()
}

/// The fixed hash, mixing two coordinates and a salt into one number.
const fn hashed(x: u32, z: u32, salt: u32) -> u32 {
    let mut mixed = x.wrapping_mul(X_STRIDE) ^ z.wrapping_mul(Z_STRIDE) ^ salt;
    mixed ^= mixed >> 15;
    mixed = mixed.wrapping_mul(FIRST_ROUND);
    mixed ^= mixed >> 13;
    mixed = mixed.wrapping_mul(SECOND_ROUND);
    mixed ^ (mixed >> 16)
}

/// The fixture a per-voxel solidity predicate describes.
///
/// Built through `Section::import` in one go rather than through 4096 writes:
/// each write is a palette scan and a packed write, and these fixtures are built
/// inside property cases and inside a benchmark's setup.
fn built_from(is_solid: impl Fn(LocalPos) -> bool) -> Result<Fixture, Box<dyn Error>> {
    let registry = fixture_registry()?;
    let described = SectionData {
        palette: vec![
            Contents::Holds(BlockName::parse(NON_SOLID_BLOCK)?),
            Contents::Holds(BlockName::parse(SOLID_BLOCK)?),
        ],
        indices: every_position()
            .map(|voxel| PaletteIndex::new(u16::from(is_solid(voxel))))
            .collect(),
    };
    let section = Section::import(&described, &registry)?;
    Ok(Fixture { section, registry })
}

/// A registry holding the fixtures' two blocks, the first non-solid and the
/// second solid.
fn fixture_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let declared = vec![
        Ok(definition(NON_SOLID_BLOCK, false)?),
        Ok(definition(SOLID_BLOCK, true)?),
    ];
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        DefinitionOrigin::new(FIXTURE_ORIGIN),
        declared,
    ))?;
    Ok(registry)
}

/// One block, textured by its own name and carrying the solidity given for it.
///
/// Nothing it could declare about being broken, built over or moved through is
/// stated: the budget these fixtures exist for is the mesher's, and a section is
/// meshed from solidity and texture keys alone. Any of the others would be a
/// field the measured work never reads.
fn definition(name: &str, is_solid: bool) -> Result<BlockDefinition, NamespacedIdError> {
    Ok(BlockDefinition {
        name: BlockName::parse(name)?,
        textures: FaceTextures::uniform(TextureKey::parse(name)?),
        is_solid,
        replaceable: false,
        breakable: true,
        breaks_into: None,
        drawn: is_solid,
        occludes: is_solid,
        targetable: is_solid,
        swimmable: false,
        move_resistance: 0.0,
        swim_ascent: 9.0,
        opacity: Opacity::OPAQUE,
        origin: DefinitionOrigin::new(FIXTURE_ORIGIN),
        tint: None,
    })
}

/// Every position a section has, x fastest, then y, then z — the order a
/// description's voxel indices are read in.
pub fn every_position() -> impl Iterator<Item = LocalPos> {
    (0..SECTION_SIZE).flat_map(|z| {
        (0..SECTION_SIZE).flat_map(move |y| (0..SECTION_SIZE).map(move |x| LocalPos { x, y, z }))
    })
}
