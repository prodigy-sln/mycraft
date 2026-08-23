//! What every meshing test in this crate builds its sections, its registries and
//! its expectations out of.
//!
//! It sits beside `common/` rather than inside it because all of it names
//! `mc_world::mesh`, and every other test binary in this crate links `common`
//! without needing a mesher to exist at all.
//!
//! A quad is compared here as a [`Face`] — facing, plane, origin and extent —
//! rather than as a whole [`Quad`]. The block a quad names is then asserted in
//! exactly one place instead of incidentally everywhere: a mesher that stamped
//! every quad with the first palette entry's name has one test written to catch
//! it, and that test stays the reason it is caught.
//!
//! Sections are built through `Section::import` in one go rather than through
//! 4096 writes, except where a scenario is about the write history itself — a
//! palette that ended up in a particular order, or an entry no voxel holds any
//! more. Those cases say so where they build.

// Each test binary links this whole module and uses a subset of it.
#![allow(dead_code)]

use std::error::Error;

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_world::mesh::{Facing, Neighbours, Quad, SectionMesh};
use mc_world::section::{Contents, LocalPos, PaletteIndex, SECTION_SIZE, Section, SectionData};

/// The error type every meshing test propagates with `?`.
pub type TestResult = Result<(), Box<dyn Error>>;

/// The two blocks most of these sections are built from: one the registry
/// declares non-solid, one it declares solid.
///
/// An `example:` namespace throughout. Files under `tests/` are not scanned for
/// the names the base game ships, but a fixture that borrowed one would still be
/// a test about the engine describing itself in terms of content.
pub const VOID: &str = "example:mesh_void";
pub const SOLID: &str = "example:mesh_stone";

/// Two further solid blocks, for the sections that have to hold more than one.
pub const ALPHA: &str = "example:alpha";
pub const BETA: &str = "example:beta";

/// A block declared drawn and neither solid nor occluding: a player walks
/// through it, sees it, and sees what is behind it.
pub const HAZE: &str = "example:haze";

/// A second block declared exactly as [`HAZE`] is, so that two adjacent cells
/// can hold two *different* drawn, non-occluding blocks. It differs from `HAZE`
/// in its name and in nothing else a declaration carries.
pub const MURK: &str = "example:murk";

/// A block declared solid and occluding and *not* drawn: it stops a player, it
/// hides what is behind it, and it shows nothing of itself.
///
/// The fixture block in which no two of the three answers can be derived from
/// each other — `drawn = is_solid` and `occludes = is_solid` cannot both hold of
/// it — which is why it is what the independence scenarios are built from.
pub const GHOST: &str = "example:ghost";

/// A block declared solid but neither drawn nor occluding: a neighbour that
/// stops a player and hides nothing.
pub const MIST: &str = "example:mist";

/// A block declared occluding but not solid: it hides what is behind it while a
/// player walks through it.
pub const SHROUD: &str = "example:shroud";

/// What the registries built here attribute their definitions to. Nothing
/// asserts it; a definition has to say where it came from.
const FIXTURE_ORIGIN: &str = "a meshing test's registry";

/// What a cell holding nothing is called wherever this suite compares palette
/// entries as text.
///
/// Not a block name and never able to become one: every namespaced name carries
/// a colon.
pub const NOTHING: &str = "nothing";

/// What `contents` holds, as text: the block's own name, or [`NOTHING`].
#[must_use]
pub fn named(contents: Contents<&BlockName>) -> &str {
    match contents {
        Contents::Empty => NOTHING,
        Contents::Holds(name) => name.as_str(),
    }
}

/// What [`some_quads`] says when it is handed a mesh holding nothing.
const EMPTY_MESH: &str = "this section holds solid voxels with nothing solid beside them, so its \
                          mesh must hold quads; every assertion that reads them is vacuous on an \
                          empty mesh";

/// A quad as the scenarios describe one, with the block it names left out.
///
/// Four of the five fields of a quad, deliberately. See the module note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Face {
    pub facing: Facing,
    pub plane: u32,
    pub origin: (u32, u32),
    pub extent: (u32, u32),
}

/// A face spelled out: which way it points, the plane of the solid voxel that
/// emitted it, where it starts and how far it runs.
#[must_use]
pub const fn face(facing: Facing, plane: u32, origin: (u32, u32), extent: (u32, u32)) -> Face {
    Face {
        facing,
        plane,
        origin,
        extent,
    }
}

/// A face covering exactly one voxel side.
#[must_use]
pub const fn single_face(facing: Facing, plane: u32, origin: (u32, u32)) -> Face {
    face(facing, plane, origin, (1, 1))
}

/// The six sides one voxel shows when nothing hides any of them.
///
/// Each side sits on the plane of the voxel that emitted it and starts at the
/// voxel's two remaining coordinates, taken in the plane's own order: primary y
/// and secondary z for ±X, primary x and secondary z for ±Y, primary x and
/// secondary y for ±Z.
///
/// A voxel whose three coordinates are equal collapses all six onto one plane,
/// where no convention could be told apart from another — so a fixture reading
/// this picks a voxel whose coordinates are pairwise distinct, and says so.
#[must_use]
pub fn every_side_of(voxel: LocalPos) -> Vec<Face> {
    vec![
        single_face(Facing::NegX, voxel.x, (voxel.y, voxel.z)),
        single_face(Facing::PosX, voxel.x, (voxel.y, voxel.z)),
        single_face(Facing::NegY, voxel.y, (voxel.x, voxel.z)),
        single_face(Facing::PosY, voxel.y, (voxel.x, voxel.z)),
        single_face(Facing::NegZ, voxel.z, (voxel.x, voxel.y)),
        single_face(Facing::PosZ, voxel.z, (voxel.x, voxel.y)),
    ]
}

/// The same six sides with the one pointing `hidden` left out.
#[must_use]
pub fn every_side_of_but(voxel: LocalPos, hidden: Facing) -> Vec<Face> {
    every_side_of(voxel)
        .into_iter()
        .filter(|side| side.facing != hidden)
        .collect()
}

/// Every quad, as the scenarios describe them.
#[must_use]
pub fn faces(quads: &[Quad]) -> Vec<Face> {
    quads.iter().map(described).collect()
}

/// Every quad pointing `facing`, as the scenarios describe them.
///
/// The scenarios about merging constrain one facing's quads and say nothing
/// about the other five, so the other five are filtered out rather than
/// asserted.
#[must_use]
pub fn faces_towards(quads: &[Quad], facing: Facing) -> Vec<Face> {
    quads
        .iter()
        .filter(|quad| quad.facing == facing)
        .map(described)
        .collect()
}

/// Where each quad pointing `facing` starts, and the block it names there.
#[must_use]
pub fn blocks_towards(quads: &[Quad], facing: Facing) -> Vec<((u32, u32), String)> {
    quads
        .iter()
        .filter(|quad| quad.facing == facing)
        .map(|quad| {
            (
                (quad.origin.primary, quad.origin.secondary),
                quad.block.as_str().to_owned(),
            )
        })
        .collect()
}

/// Every quad as the scenarios describe them, each beside the block it names.
///
/// The module note keeps the block a quad names out of [`faces`] so that it is
/// asserted deliberately rather than incidentally. One scenario is about *which*
/// of two blocks shows a face on the boundary they share, and that is
/// unobservable without the name — so it is asserted deliberately, here.
#[must_use]
pub fn named_faces(quads: &[Quad]) -> Vec<(Face, String)> {
    quads
        .iter()
        .map(|quad| (described(quad), quad.block.as_str().to_owned()))
        .collect()
}

/// One quad, described.
fn described(quad: &Quad) -> Face {
    face(
        quad.facing,
        quad.plane,
        (quad.origin.primary, quad.origin.secondary),
        (quad.extent.primary, quad.extent.secondary),
    )
}

/// The quads a mesh holds, refusing a mesh that holds none.
///
/// Every comparison written against a section that does hold solid voxels is
/// vacuous on an empty mesh — two empty sequences are equal, an empty sequence
/// contains no wrong extent, and no quads at all is comfortably under any
/// ceiling. A mesher that emitted nothing is the one mistake this suite is least
/// able to see, so it is refused here rather than passed on.
///
/// # Errors
///
/// Returns an error if `mesh` holds no quads.
pub fn some_quads(mesh: &SectionMesh) -> Result<&[Quad], Box<dyn Error>> {
    let quads = mesh.quads();
    if quads.is_empty() {
        return Err(EMPTY_MESH.into());
    }
    Ok(quads)
}

/// A local position, spelled out.
#[must_use]
pub const fn at(x: u32, y: u32, z: u32) -> LocalPos {
    LocalPos { x, y, z }
}

/// Every position a section has, x fastest, then y, then z — the order a
/// description's voxel indices are read in, and the order the mesher is expected
/// to resolve its voxels in.
pub fn every_position() -> impl Iterator<Item = LocalPos> {
    (0..SECTION_SIZE).flat_map(|z| {
        (0..SECTION_SIZE).flat_map(move |y| (0..SECTION_SIZE).map(move |x| at(x, y, z)))
    })
}

/// What a block declares about the three questions a mesh asks of it: whether
/// something stands there, whether something is drawn there, and whether what is
/// there hides whatever is behind it.
///
/// The three are separate answers, and a fixture that cannot state them
/// separately cannot fail a mesher that reads all three off solidity. Written as
/// a struct rather than as three positional booleans so a fixture reads the way
/// its scenario is worded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Declaration {
    pub solid: bool,
    pub drawn: bool,
    pub occludes: bool,
}

impl Declaration {
    /// What every fixture written before drawnness and occlusion existed means:
    /// all three answers are the block's solidity.
    #[must_use]
    pub const fn like_solidity(solid: bool) -> Self {
        Self {
            solid,
            drawn: solid,
            occludes: solid,
        }
    }
}

/// What [`HAZE`] and [`MURK`] declare: drawn, and neither solid nor occluding.
pub const DRAWN_ONLY: Declaration = Declaration {
    solid: false,
    drawn: true,
    occludes: false,
};

/// What [`GHOST`] declares: solid and occluding, and never drawn.
pub const SOLID_AND_OCCLUDING: Declaration = Declaration {
    solid: true,
    drawn: false,
    occludes: true,
};

/// What [`MIST`] declares: solid, and neither drawn nor occluding.
pub const SOLID_ONLY: Declaration = Declaration {
    solid: true,
    drawn: false,
    occludes: false,
};

/// What [`SHROUD`] declares: occluding, and neither drawn nor solid.
pub const OCCLUDING_ONLY: Declaration = Declaration {
    solid: false,
    drawn: false,
    occludes: true,
};

/// A registry holding exactly `blocks`, in the order given, each carrying the
/// solidity declared beside it and textured by its own name.
///
/// Every one of the three questions a mesh asks is answered by that solidity,
/// which is what a fixture written before the three were separable meant — see
/// [`registry_of_declarations`] for the builder that can state them apart.
///
/// The order is never incidental: it is what decides the runtime id each block
/// gets, and three scenarios turn on that.
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
/// The one place a meshing fixture's definitions are built, so that a fixture
/// stating three answers and a fixture stating one reach the registry by the same
/// route rather than by two that could drift.
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
        // A mesh is decided by what a block states about being drawn, occluding
        // and solid, and by the texture key — so every block declared for a
        // meshing fixture leaves breakability, replaceability and residue at what
        // a declaration saying nothing about them means. Targetability is read by
        // nothing a meshing fixture drives, and follows solidity for the same
        // reason.
        declared.push(Ok(BlockDefinition {
            name: BlockName::parse(name)?,
            textures: FaceTextures::uniform(TextureKey::parse(name)?),
            is_solid: states.solid,
            replaceable: false,
            breakable: true,
            breaks_into: None,
            drawn: states.drawn,
            occludes: states.occludes,
            targetable: states.solid,
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

/// A registry holding [`VOID`] non-solid and [`SOLID`] solid, in that order.
///
/// # Errors
///
/// Returns an error if the registry refuses either block.
pub fn plain_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    registry_declaring(&[(VOID, false), (SOLID, true)])
}

/// A section whose every voxel holds the palette entry `holder` names for it.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id, or if `registry` does not
/// register one of them.
pub fn section_holding(
    palette: &[&str],
    holder: impl Fn(LocalPos) -> u16,
    registry: &BlockRegistry,
) -> Result<Section, Box<dyn Error>> {
    let mut names = Vec::with_capacity(palette.len());
    for name in palette {
        names.push(Contents::Holds(BlockName::parse(name)?));
    }
    let described = SectionData {
        palette: names,
        indices: every_position()
            .map(|voxel| PaletteIndex::new(holder(voxel)))
            .collect(),
    };
    Ok(Section::import(&described, registry)?)
}

/// A section every cell of which holds nothing, except that the cells `held`
/// names hold the block named beside them.
///
/// **Every other cell is genuinely `Contents::Empty` rather than a block declared
/// to show nothing**, which [`section_holding`] cannot express because it parses
/// every palette entry as a name. A scenario about a voxel with empty space
/// beside it means the absence of a block, and standing a block in for it would
/// leave the same scenario satisfied by a mesher that never looked at emptiness
/// at all.
///
/// Written one cell at a time, which is the only door emptiness has.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id, or if `registry` does not
/// register one of them.
pub fn section_of_nothing_but(
    held: &[(LocalPos, &str)],
    registry: &BlockRegistry,
) -> Result<Section, Box<dyn Error>> {
    let mut section = Section::empty();
    for &(position, name) in held {
        section.set_block(position, &BlockName::parse(name)?, registry)?;
    }
    Ok(section)
}

/// A section of [`VOID`] holding [`SOLID`] at exactly the positions `is_solid`
/// names.
///
/// # Errors
///
/// Returns an error if `registry` does not register both blocks.
pub fn scattered_solids(
    is_solid: impl Fn(LocalPos) -> bool,
    registry: &BlockRegistry,
) -> Result<Section, Box<dyn Error>> {
    section_holding(&[VOID, SOLID], |voxel| u16::from(is_solid(voxel)), registry)
}

/// A section every voxel of which holds [`SOLID`].
///
/// # Errors
///
/// Returns an error if `registry` does not register that block.
pub fn solid_section(registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    Ok(Section::filled(&BlockName::parse(SOLID)?, registry)?)
}

/// A section of [`SOLID`] with one non-solid voxel in it, at `hole`.
///
/// # Errors
///
/// Returns an error if `registry` does not register both blocks.
pub fn solid_but_for(hole: LocalPos, registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    scattered_solids(|voxel| voxel != hole, registry)
}

/// One section supplied beyond all six facings.
///
/// The same section stands beyond each of them, which is what a section buried
/// inside uniform content is surrounded by.
#[must_use]
pub fn walled_in_by(neighbour: &Section) -> Neighbours<'_> {
    Facing::ALL
        .into_iter()
        .fold(Neighbours::none(), |so_far, facing| {
            so_far.with(facing, neighbour)
        })
}

/// One section per facing, built by `build` from the facing it will stand
/// beyond.
///
/// Built in `Facing::ALL` order, which is the order [`all_around`] reads them
/// back in — so the two agree by construction, and the facing a section was
/// built for is the facing it is supplied for. The sections come back owned
/// because a [`Neighbours`] borrows them and has to be built where they live.
///
/// # Errors
///
/// Returns whatever `build` returns for the first facing it refuses.
pub fn sections_around(
    build: impl Fn(Facing) -> Result<Section, Box<dyn Error>>,
) -> Result<Vec<Section>, Box<dyn Error>> {
    let mut around = Vec::with_capacity(Facing::ALL.len());
    for facing in Facing::ALL {
        around.push(build(facing)?);
    }
    Ok(around)
}

/// The sections [`sections_around`] built, each supplied beyond the facing it
/// was built for.
///
/// Pairs by position in `Facing::ALL`, so anything shorter than six leaves the
/// remaining facings absent.
#[must_use]
pub fn all_around(around: &[Section]) -> Neighbours<'_> {
    Facing::ALL
        .into_iter()
        .zip(around)
        .fold(Neighbours::none(), |so_far, (facing, section)| {
            so_far.with(facing, section)
        })
}

/// Refuses a registry that did not number `name` the way the scenario needs.
///
/// Registration order is what decides a runtime id, so a reordered registry
/// would leave the test below asserting something other than what it was written
/// for — and it would still be green.
///
/// # Errors
///
/// Returns an error if `name` is not registered, or is registered under some
/// other runtime id.
pub fn require_runtime_id(
    registry: &BlockRegistry,
    name: &str,
    expected: u32,
) -> Result<(), Box<dyn Error>> {
    let assigned = registry.id_of(&BlockName::parse(name)?)?.get();
    if assigned == expected {
        return Ok(());
    }
    Err(format!(
        "this scenario is about the block a registry numbered {expected}, and `{name}` was \
         numbered {assigned} instead; registration order decides it, so the assertion below \
         would be about a different block than the one it names"
    )
    .into())
}
