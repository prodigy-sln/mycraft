//! Which world axis each of the six facing words draws on, read off a block that
//! was actually placed, meshed and packed.
//!
//! # These three readings are the only witnesses in the workspace for the mapping
//!
//! `up`, `down`, `north`, `south`, `east` and `west` are what a declaration
//! writes; an axis and a sign are what a mesher works in. One total function
//! joins them, and until this file existed **nothing anywhere disagreed with any
//! wrong answer it could give**: exchanging `north` and `south` in that function
//! was measured against the whole workspace and reddened nothing at all. The
//! round trip over both `ALL` arrays cannot see two words exchanged — it sees
//! only that the mapping is a bijection — and no other consumer reads it.
//!
//! So the mapping is deliberately **not** called here. A test that asked the
//! function which face `NegZ` is would be two copies of one decision agreeing
//! with each other. What is asked instead is the question the requirement asks:
//! a block declaring six keys is put in the world, the world is meshed, the mesh
//! is packed, and each packed face is identified **by where its corners are** —
//! then the layer that face draws from is read back.
//!
//! # How a face is identified, and why not from the quad
//!
//! Each of the six faces of one voxel is a unit square, so exactly one world
//! axis is degenerate across its four corners. That axis is the face's own, and
//! whether the degenerate coordinate sits on the voxel's plane or one past it is
//! the sign. Nothing here reads a `Facing`: the quad's facing is an *input* the
//! mesher chose, and the corners are the *output* a player would see.
//!
//! # The assignment disagrees with the sorted order, deliberately
//!
//! Layers are staged so that no key holds the layer its lexicographic position
//! would give it. A reading taken against a sorted assignment cannot fail
//! whatever the packer did, because both sides would be the same sort.
//!
//! # The fixture's own names carry no answer
//!
//! The six keys are minerals in alphabetical order and the facings they are
//! declared against are not. A key spelled `example:northward` would let a reader
//! — and a future implementation — recover the mapping from the spelling, which
//! is the one thing this file must not allow.

#[path = "support/staged_layers.rs"]
mod staged_layers;

use std::collections::BTreeMap;
use std::error::Error;

use mc_client::content::ContentView;
use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin, Opacity};
use mc_core::content::{FaceTextures, ResolvedBlock, ResolvedContent};
use mc_core::id::{BlockName, TextureKey};
use mc_render::geometry::{SectionGeometry, SectionOrigin, build_section_geometry};
use mc_world::mesh::{Neighbours, mesh_section};
use mc_world::section::{Contents, LocalPos, PaletteIndex, SECTION_SIZE, Section, SectionData};

use staged_layers::assigned;

type TestResult = Result<(), Box<dyn Error>>;

/// The block whose six facings are the subject.
const BANDED: &str = "example:banded";

/// What the rest of the section holds: a block the registry declares non-solid,
/// so the one solid voxel shows all six of its faces.
const VOID: &str = "example:axis_void";

/// The six keys [`BANDED`] declares, in the order a declaration writes its
/// facings: up, down, north, south, east and west.
///
/// Minerals rather than compass words. A key whose spelling names the facing it
/// is declared against would let the mapping be recovered from the fixture, and
/// the whole subject of this file is that the mapping is stated rather than
/// inferable.
const UP_KEY: &str = "example:amber";
const DOWN_KEY: &str = "example:basalt";
const NORTH_KEY: &str = "example:cobalt";
const SOUTH_KEY: &str = "example:diorite";
const EAST_KEY: &str = "example:emerald";
const WEST_KEY: &str = "example:feldspar";

/// The six, positionally in the order the words above are written.
const SIX_KEYS: [&str; 6] = [UP_KEY, DOWN_KEY, NORTH_KEY, SOUTH_KEY, EAST_KEY, WEST_KEY];

/// Which layer each key holds, and **none of them holds the layer its sorted
/// position would give it**.
///
/// Sorted, these seven keys run amber, axis_void, basalt, cobalt, diorite,
/// emerald, feldspar — so the lexicographic assignment would be 0 through 6 in
/// that order. Every entry below differs from it.
const DISAGREEING: [(&str, u16); 7] = [
    (UP_KEY, 3),
    (DOWN_KEY, 5),
    (NORTH_KEY, 0),
    (SOUTH_KEY, 6),
    (EAST_KEY, 1),
    (WEST_KEY, 2),
    (VOID, 4),
];

/// Where the one solid voxel sits.
///
/// No two coordinates are equal and none is on a section edge, so every face is
/// shown and no two of the six can be confused by their coordinates coinciding.
const VOXEL: LocalPos = LocalPos { x: 2, y: 5, z: 9 };

/// How many corners one quad is packed into.
const CORNERS_PER_QUAD: usize = 4;

/// The section's world origin. Section-local and world coordinates are then the
/// same number, which is what lets a face's sign be read against [`VOXEL`].
const SECTION_AT_THE_ORIGIN: [i32; 3] = [0, 0, 0];

/// Which way a packed face points, derived from where its corners landed.
///
/// A test-local vocabulary on purpose: naming `Facing` or `Face` here would put
/// one of the two enums the mapping joins on both sides of the comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Pointing {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

#[test]
fn the_key_declared_for_up_is_drawn_on_the_positive_y_face_and_on_no_other() -> TestResult {
    let drawn = faces_drawn()?;
    let up = layer_of(UP_KEY)?;

    assert_eq!(
        (drawn.get(&Pointing::PosY).copied(), drawing(&drawn, up)),
        (Some(up), vec![Pointing::PosY]),
        "the face pointing along positive Y is the one a declaration calls `up`, and the key \
         declared there draws on that face and on nothing else. The second half is what makes \
         this more than an existence claim: an implementation that drew `up`'s key everywhere \
         would satisfy the first half and paint the whole block with the top of it"
    );
    Ok(())
}

#[test]
fn north_draws_along_negative_z_and_south_along_positive_z() -> TestResult {
    let drawn = faces_drawn()?;

    assert_eq!(
        (
            drawn.get(&Pointing::NegZ).copied(),
            drawn.get(&Pointing::PosZ).copied()
        ),
        (Some(layer_of(NORTH_KEY)?), Some(layer_of(SOUTH_KEY)?)),
        "`north` faces along negative Z and `south` along positive Z. Exchanging the two is a \
         change nothing else in this workspace can see — the mapping stays a bijection, every \
         count stays the same, and the block still draws six textures — so a grass block would \
         simply have two of its sides swapped and no test would say a word"
    );
    Ok(())
}

#[test]
fn east_draws_along_positive_x_and_west_along_negative_x() -> TestResult {
    let drawn = faces_drawn()?;

    assert_eq!(
        (
            drawn.get(&Pointing::PosX).copied(),
            drawn.get(&Pointing::NegX).copied()
        ),
        (Some(layer_of(EAST_KEY)?), Some(layer_of(WEST_KEY)?)),
        "`east` faces along positive X and `west` along negative X, which is the other half of \
         the compass and is exchangeable in exactly the same silent way as north and south"
    );
    Ok(())
}

/// The layer each of the six world directions is packed with, for a block
/// declaring [`SIX_KEYS`] placed at [`VOXEL`] and meshed where it stands.
///
/// # Errors
///
/// Returns an error if a fixture id does not parse, if the section will not
/// import or mesh, if the packer refuses it, or if the six faces read back are
/// not exactly the six world directions — a mesh showing five faces would leave
/// every lookup below answering `None` and every comparison failing for a reason
/// that is about the fixture rather than about the mapping.
fn faces_drawn() -> Result<BTreeMap<Pointing, u16>, Box<dyn Error>> {
    let registry = registry()?;
    let section = one_banded_voxel(&registry)?;
    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;
    let view = ContentView::of(&resolved()?);

    let geometry = build_section_geometry(
        mesh.quads(),
        SectionOrigin::new(SECTION_AT_THE_ORIGIN),
        view.resolution(),
    )?;

    let drawn = read_back(&geometry)?;
    if drawn.len() != Pointing::ALL.len() {
        return Err(format!(
            "a solid voxel with nothing solid beside it shows all {expected} of its faces, and \
             this fixture read back {found:?}. Every comparison in this file is a lookup by \
             direction, so a mesh short of a face would fail them for a reason that is not the \
             mapping",
            expected = Pointing::ALL.len(),
            found = drawn.keys().collect::<Vec<_>>()
        )
        .into());
    }
    Ok(drawn)
}

impl Pointing {
    /// The six directions a voxel's faces point in.
    const ALL: [Self; 6] = [
        Self::NegX,
        Self::PosX,
        Self::NegY,
        Self::PosY,
        Self::NegZ,
        Self::PosZ,
    ];

    /// The direction whose degenerate axis is `axis` and whose sign is `positive`.
    const fn on(axis: usize, positive: bool) -> Option<Self> {
        match (axis, positive) {
            (0, false) => Some(Self::NegX),
            (0, true) => Some(Self::PosX),
            (1, false) => Some(Self::NegY),
            (1, true) => Some(Self::PosY),
            (2, false) => Some(Self::NegZ),
            (2, true) => Some(Self::PosZ),
            _ => None,
        }
    }
}

/// Which direction each packed quad points, and the layer all four of its
/// corners were packed with.
///
/// # Errors
///
/// Returns an error if a quad's corners do not lie in one plane of one axis, if
/// its four corners disagree about their layer — a packer writing the right
/// layer into one corner and a wrong one into the other three would draw three
/// quarters of every face wrong — or if two quads claim the same direction.
fn read_back(geometry: &SectionGeometry) -> Result<BTreeMap<Pointing, u16>, Box<dyn Error>> {
    let mut drawn = BTreeMap::new();
    for quad in 0..geometry.quad_count() {
        let (pointing, layer) = one_face(geometry, quad)?;
        if drawn.insert(pointing, layer).is_some() {
            return Err("two faces of one voxel cannot point the same way".into());
        }
    }
    Ok(drawn)
}

/// Which way the `quad`th packed face points, and the layer all four of its
/// corners carry.
///
/// # Errors
///
/// Returns an error if a corner was never emitted, if the four disagree about
/// their layer — a packer writing the right layer into one corner and a wrong
/// one into the other three would draw three quarters of every face wrong — or
/// if the four do not lie in one axis-aligned plane.
fn one_face(geometry: &SectionGeometry, quad: usize) -> Result<(Pointing, u16), Box<dyn Error>> {
    let first = quad * CORNERS_PER_QUAD;
    let mut corners = Vec::with_capacity(CORNERS_PER_QUAD);
    let mut layers = Vec::with_capacity(CORNERS_PER_QUAD);
    for corner in first..first + CORNERS_PER_QUAD {
        corners.push(
            geometry
                .world_corner(corner)
                .ok_or_else(|| format!("corner {corner} was never emitted"))?,
        );
        layers.push(
            geometry
                .layer_at(corner)
                .ok_or_else(|| format!("corner {corner} carries no layer"))?,
        );
    }
    let (&layer, rest) = layers
        .split_first()
        .ok_or("a quad is packed into four corners")?;
    if rest.iter().any(|other| *other != layer) {
        return Err(format!(
            "the four corners of one face have to be packed with one layer, and these carry \
             {layers:?}"
        )
        .into());
    }
    Ok((pointing_of(&corners)?, layer))
}

/// Which way the face through `corners` points.
///
/// # Errors
///
/// Returns an error unless exactly one world axis is constant across the four
/// corners, and that constant sits either on [`VOXEL`]'s own plane or one step
/// past it.
fn pointing_of(corners: &[[f32; 3]]) -> Result<Pointing, Box<dyn Error>> {
    let voxel = [i64::from(VOXEL.x), i64::from(VOXEL.y), i64::from(VOXEL.z)];
    let mut found = None;
    for (axis, near) in voxel.into_iter().enumerate() {
        let Some(plane) = constant_on(corners, axis)? else {
            continue;
        };
        if found
            .replace(Pointing::on(axis, side_of(plane, near, axis)?))
            .is_some()
        {
            return Err(format!(
                "a unit face is degenerate on exactly one axis, and the corners {corners:?} are \
                 degenerate on more than one"
            )
            .into());
        }
    }
    found
        .flatten()
        .ok_or_else(|| format!("the corners {corners:?} lie in no axis-aligned plane").into())
}

/// The coordinate every corner shares on `axis`, or `None` where they do not
/// share one.
///
/// # Errors
///
/// Returns an error if a corner holds no such axis.
fn constant_on(corners: &[[f32; 3]], axis: usize) -> Result<Option<i64>, Box<dyn Error>> {
    let mut shared = None;
    for corner in corners {
        let coordinate = corner
            .get(axis)
            .copied()
            .map(whole)
            .ok_or_else(|| format!("a corner has no axis {axis}"))?;
        match shared {
            None => shared = Some(coordinate),
            Some(first) if first == coordinate => {}
            Some(_) => return Ok(None),
        }
    }
    Ok(shared)
}

/// Whether a face whose plane sits at `plane` points away from the voxel side at
/// `near`.
///
/// # Errors
///
/// Returns an error unless the plane is the voxel's own or one step past it,
/// which are the only two places a face of that voxel can sit.
fn side_of(plane: i64, near: i64, axis: usize) -> Result<bool, Box<dyn Error>> {
    match plane - near {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(format!(
            "a face of the voxel this fixture placed sits on its own plane or one step past it, \
             and this one sits at {plane} on axis {axis}, where the voxel sits at {near}"
        )
        .into()),
    }
}

/// A world coordinate as the whole number it is.
///
/// Every corner here is a sum of small integers and exact in `f32`;
/// `clippy::float_cmp` is denied and applies to test code, so the comparison
/// runs on whole numbers instead.
fn whole(coordinate: f32) -> i64 {
    coordinate.round() as i64
}

/// Which directions of `drawn` were packed with `layer`.
fn drawing(drawn: &BTreeMap<Pointing, u16>, layer: u16) -> Vec<Pointing> {
    drawn
        .iter()
        .filter(|(_, packed)| **packed == layer)
        .map(|(pointing, _)| *pointing)
        .collect()
}

/// The layer [`DISAGREEING`] states for `key`.
///
/// # Errors
///
/// Returns an error if the table names no layer for it.
fn layer_of(key: &str) -> Result<u16, Box<dyn Error>> {
    DISAGREEING
        .into_iter()
        .find(|(named, _)| *named == key)
        .map(|(_, layer)| layer)
        .ok_or_else(|| format!("this fixture states no layer for `{key}`").into())
}

/// The content the client is handed: the banded block with its six declared
/// keys, the non-solid filler beside it, and the assignment above.
///
/// # Errors
///
/// Returns an error if a fixture id does not parse or if the assignment cannot
/// be staged.
fn resolved() -> Result<ResolvedContent, Box<dyn Error>> {
    Ok(ResolvedContent::stating(
        vec![
            ResolvedBlock {
                name: BlockName::parse(BANDED)?,
                textures: six_keys()?,
                is_solid: true,
                opacity: Opacity::OPAQUE,
            },
            ResolvedBlock {
                name: BlockName::parse(VOID)?,
                textures: FaceTextures::uniform(TextureKey::parse(VOID)?),
                is_solid: false,
                opacity: Opacity::OPAQUE,
            },
        ],
        assigned(&DISAGREEING)?,
    ))
}

/// [`SIX_KEYS`], positionally, as a declaration states them.
///
/// # Errors
///
/// Returns an error if a key is not a namespaced id, or if the six are not
/// pairwise distinct — six faces sharing a layer would satisfy every lookup
/// below without the mapping being read at all.
fn six_keys() -> Result<FaceTextures, Box<dyn Error>> {
    let mut parsed = Vec::with_capacity(SIX_KEYS.len());
    for key in SIX_KEYS {
        parsed.push(TextureKey::parse(key)?);
    }
    let mut distinct = parsed.clone();
    distinct.sort();
    distinct.dedup();
    if distinct.len() != SIX_KEYS.len() {
        return Err(format!(
            "the six keys this fixture declares have to be pairwise distinct, and {SIX_KEYS:?} \
             are not — two facings sharing a key share a layer, and a reading about which face \
             drew which key could not then fail"
        )
        .into());
    }
    let keys: [TextureKey; 6] = parsed
        .try_into()
        .map_err(|_unexpected| "a declaration states exactly six facings")?;
    Ok(FaceTextures::stating(keys))
}

/// The filler the section is made of: solid to nothing, seen by nothing, and
/// not something a swing can find.
///
/// # Errors
///
/// Returns an error if its id or its texture key is not a namespaced id.
fn the_filler(origin: &DefinitionOrigin) -> Result<BlockDefinition, Box<dyn Error>> {
    Ok(BlockDefinition {
        name: BlockName::parse(VOID)?,
        textures: FaceTextures::uniform(TextureKey::parse(VOID)?),
        is_solid: false,
        replaceable: false,
        breakable: true,
        breaks_into: None,
        drawn: false,
        occludes: false,
        targetable: false,
        swimmable: false,
        move_resistance: 0.0,
        swim_ascent: 9.0,
        opacity: Opacity::OPAQUE,
        origin: origin.clone(),
    })
}

/// The one block this fixture is about: solid, seen, and stating a distinct
/// texture key against each of its six facings.
///
/// # Errors
///
/// Returns an error if its id does not parse, or for the reasons [`six_keys`]
/// gives.
fn the_banded_block(origin: &DefinitionOrigin) -> Result<BlockDefinition, Box<dyn Error>> {
    Ok(BlockDefinition {
        name: BlockName::parse(BANDED)?,
        textures: six_keys()?,
        is_solid: true,
        replaceable: false,
        breakable: true,
        breaks_into: None,
        drawn: true,
        occludes: true,
        targetable: true,
        swimmable: false,
        move_resistance: 0.0,
        swim_ascent: 9.0,
        opacity: Opacity::OPAQUE,
        origin: origin.clone(),
    })
}

/// A registry declaring the banded block solid and the filler non-solid.
///
/// # Errors
///
/// Returns an error if a fixture id does not parse or if the registry refuses
/// the batch.
fn registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let origin = DefinitionOrigin::new("the per-face axis fixture");
    let declared = vec![Ok(the_filler(&origin)?), Ok(the_banded_block(&origin)?)];
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(origin, declared))?;
    Ok(registry)
}

/// A section of the non-solid filler holding one [`BANDED`] voxel at [`VOXEL`].
///
/// # Errors
///
/// Returns an error if a fixture id does not parse or if the registry does not
/// register both blocks.
fn one_banded_voxel(registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    let described = SectionData {
        palette: vec![
            Contents::Holds(BlockName::parse(VOID)?),
            Contents::Holds(BlockName::parse(BANDED)?),
        ],
        indices: every_position()
            .map(|voxel| PaletteIndex::new(u16::from(voxel == VOXEL)))
            .collect(),
    };
    Ok(Section::import(&described, registry)?)
}

/// Every position of a section, x fastest, then y, then z — the order a
/// description's voxel indices are read in.
fn every_position() -> impl Iterator<Item = LocalPos> {
    (0..SECTION_SIZE).flat_map(|z| {
        (0..SECTION_SIZE).flat_map(move |y| (0..SECTION_SIZE).map(move |x| LocalPos { x, y, z }))
    })
}
