//! What holds for any section at all, judged against an independent scan.
//!
//! Every other meshing test in this crate names a fixture and the answer it
//! expects. These three name neither: a section and its six surroundings are
//! generated, and the answer is whatever a separate per-voxel scan — one that
//! shares no code with the mesher, derives its own adjacency and was written
//! before the mesher existed — says the visible faces are. The three properties
//! together are the whole of "the quads cover exactly the visible faces": none
//! of them covers a face that is not visible, every visible face is covered by
//! one, and none is covered twice. Any two of the three are satisfiable by
//! something wrong.
//!
//! The two representations have to be brought together for that comparison, and
//! the bridge is written here rather than borrowed. A quad is a rectangle; the
//! scan reports one (voxel, side) pair at a time. Expanding a quad into the
//! pairs it covers needs the specification's own convention — the plane is the
//! coordinate of the solid voxel that emitted the face, and the plane's two axes
//! are the two that are not the facing's, in x < y < z order, the lower of them
//! primary. That convention is restated below from the specification, because
//! taking it from the mesher would make a mesher that had it wrong agree with
//! itself.
//!
//! The generated content is deliberately blocky rather than per-voxel noise. A
//! section of noise is almost all 1x1 quads and exercises the merge sweep barely
//! at all, so each generated section drops the low bits of its coordinates before
//! hashing and is built out of cubes of one block — which leaves runs to merge,
//! rows to extend along, and boundaries where a run meets a section edge.
//!
//! Thirty-two cases, not the default two hundred and fifty-six. Each case builds
//! up to seven sections, meshes one and scans one against six, and this runs
//! under coverage instrumentation at every phase boundary from here on, in a
//! suite every later specification pays for.

mod mesh_common;

#[path = "../benches/support/mod.rs"]
mod support;

use std::collections::BTreeSet;
use std::error::Error;

use mc_core::block::BlockRegistry;
use mc_world::mesh::{Facing, Neighbours, Quad, mesh_section};
use mc_world::section::{LocalPos, Section};
use mesh_common::{registry_declaring, section_holding};
use proptest::prelude::*;
use support::oracle::{Neighbourhood, Side, VisibleFace, visible_faces};

/// How many generated cases each property is checked over.
const CASES: u32 = 32;

/// The blocks generated content is drawn from: two the registry declares
/// non-solid and two it declares solid.
///
/// Two of each rather than one, so that "non-solid" is never a single
/// distinguished palette entry and a mesher reading a palette position rather
/// than a registered definition has two chances to be caught.
const POOL: [(&str, bool); 4] = [
    ("example:generated_air", false),
    ("example:generated_mist", false),
    ("example:generated_stone", true),
    ("example:generated_clay", true),
];

/// Where the solid half of [`POOL`] starts.
const FIRST_SOLID_ENTRY: u16 = 2;

/// The scale [`Contents::solid_in_256`] is read against.
const OUT_OF: u32 = 256;

/// What one generated section is made of.
///
/// Three small numbers rather than 4096 generated entries: a section is a
/// function of them, so a shrunk case rebuilds exactly, shrinking has three
/// values to work on instead of thousands, and generating seven sections costs
/// nothing worth measuring.
#[derive(Debug, Clone, Copy)]
struct Contents {
    /// What the fixed hash below is seeded with.
    seed: u32,
    /// How much of the section is solid, out of [`OUT_OF`]. Zero leaves nothing
    /// solid in it at all and the full scale fills it.
    solid_in_256: u32,
    /// How many low bits of each coordinate are dropped before hashing, so the
    /// section is built from cubes of one entry 1, 2 or 4 voxels across.
    coarseness: u32,
}

/// One generated section, and everything the two sides of the comparison say
/// about it.
struct Judged {
    /// Every (voxel, side) pair the emitted quads cover, in the order the quads
    /// name them — a list and not a set, because one of the three properties is
    /// about a pair appearing twice.
    covered: Vec<VisibleFace>,
    /// Every pair the independent scan reports visible.
    visible: BTreeSet<VisibleFace>,
}

/// An arbitrary section's worth of contents.
fn contents() -> impl Strategy<Value = Contents> {
    (any::<u32>(), 0..=OUT_OF, 0..3u32).prop_map(|(seed, solid_in_256, coarseness)| Contents {
        seed,
        solid_in_256,
        coarseness,
    })
}

/// Six surroundings, each independently absent or generated.
///
/// Independently, because absence is per neighbour: a case with the section
/// below loaded and the other five missing is the commonest shape a streaming
/// world produces, and it is a different answer from both "all six loaded" and
/// "none of them".
fn surroundings() -> impl Strategy<Value = [Option<Contents>; 6]> {
    proptest::array::uniform6(proptest::option::of(contents()))
}

/// `voxel` with the low `by` bits of each of its coordinates dropped.
const fn coarsened(voxel: LocalPos, by: u32) -> LocalPos {
    LocalPos {
        x: voxel.x >> by,
        y: voxel.y >> by,
        z: voxel.z >> by,
    }
}

/// One value mixed into another.
const fn mixed(value: u32) -> u32 {
    let folded = value ^ (value >> 15);
    folded.wrapping_mul(0x2545_F491)
}

/// A fixed integer hash of a position under a seed.
///
/// Committed here rather than drawn from a generator, so that a section is a
/// function of its three numbers and a case that failed rebuilds identically.
const fn hashed(seed: u32, voxel: LocalPos) -> u32 {
    let seeded = mixed(seed ^ 0x9E37_79B9);
    let along_x = mixed(seeded ^ voxel.x.wrapping_mul(0x85EB_CA6B));
    let along_y = mixed(along_x ^ voxel.y.wrapping_mul(0xC2B2_AE35));
    mixed(along_y ^ voxel.z.wrapping_mul(0x27D4_EB2F))
}

/// Which palette entry a voxel of a generated section holds.
fn entry_at(contents: Contents, voxel: LocalPos) -> u16 {
    let noise = hashed(contents.seed, coarsened(voxel, contents.coarseness));
    let within = ((noise >> 8) & 1) as u16;
    if (noise & (OUT_OF - 1)) < contents.solid_in_256 {
        return FIRST_SOLID_ENTRY + within;
    }
    within
}

/// A section built from `contents`, imported in one go.
fn built(contents: Contents, registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    let palette = POOL.map(|(name, _)| name);
    section_holding(&palette, |voxel| entry_at(contents, voxel), registry)
}

/// A neighbour built from `contents`, or none where none was generated.
fn built_or_absent(
    contents: Option<Contents>,
    registry: &BlockRegistry,
) -> Result<Option<Section>, Box<dyn Error>> {
    match contents {
        Some(contents) => Ok(Some(built(contents, registry)?)),
        None => Ok(None),
    }
}

/// The six surroundings, built, in `Facing::ALL` order — which is the order both
/// containers below read them back in.
fn built_around(
    around: [Option<Contents>; 6],
    registry: &BlockRegistry,
) -> Result<[Option<Section>; 6], Box<dyn Error>> {
    let [neg_x, pos_x, neg_y, pos_y, neg_z, pos_z] = around;
    Ok([
        built_or_absent(neg_x, registry)?,
        built_or_absent(pos_x, registry)?,
        built_or_absent(neg_y, registry)?,
        built_or_absent(pos_y, registry)?,
        built_or_absent(neg_z, registry)?,
        built_or_absent(pos_z, registry)?,
    ])
}

/// Those sections as the mesher takes them.
fn neighbours_of(around: &[Option<Section>; 6]) -> Neighbours<'_> {
    Facing::ALL
        .into_iter()
        .zip(around)
        .fold(
            Neighbours::none(),
            |so_far, (facing, section)| match section {
                Some(section) => so_far.with(facing, section),
                None => so_far,
            },
        )
}

/// The same sections as the independent scan takes them.
fn neighbourhood_of(around: &[Option<Section>; 6]) -> Neighbourhood<'_> {
    let [neg_x, pos_x, neg_y, pos_y, neg_z, pos_z] = around;
    Neighbourhood {
        neg_x: neg_x.as_ref(),
        pos_x: pos_x.as_ref(),
        neg_y: neg_y.as_ref(),
        pos_y: pos_y.as_ref(),
        neg_z: neg_z.as_ref(),
        pos_z: pos_z.as_ref(),
    }
}

/// Which side of a voxel a facing names.
///
/// The scan's six sides and the mesher's six facings are separate enumerations
/// on purpose — one is the judge and the other is the judged — so the
/// correspondence between them is written down here, where both are in scope.
const fn side_of(facing: Facing) -> Side {
    match facing {
        Facing::NegX => Side::NegX,
        Facing::PosX => Side::PosX,
        Facing::NegY => Side::NegY,
        Facing::PosY => Side::PosY,
        Facing::NegZ => Side::NegZ,
        Facing::PosZ => Side::PosZ,
    }
}

/// The voxel a face sits on, from the plane it sits in and where in that plane
/// it is.
///
/// The plane is the coordinate of the solid voxel along the facing's own axis,
/// and the plane's two axes are the two that are not the facing's, in x < y < z
/// order with the lower of them primary: ±X runs primary y and secondary z, ±Y
/// primary x and secondary z, ±Z primary x and secondary y. Restated from the
/// specification rather than read off the mesher, which is what is being judged.
const fn voxel_at(facing: Facing, plane: u32, primary: u32, secondary: u32) -> LocalPos {
    match facing {
        Facing::NegX | Facing::PosX => LocalPos {
            x: plane,
            y: primary,
            z: secondary,
        },
        Facing::NegY | Facing::PosY => LocalPos {
            x: primary,
            y: plane,
            z: secondary,
        },
        Facing::NegZ | Facing::PosZ => LocalPos {
            x: primary,
            y: secondary,
            z: plane,
        },
    }
}

/// Every (voxel, side) pair one quad covers.
fn covered_by(quad: &Quad) -> Vec<VisibleFace> {
    let (facing, plane, origin) = (quad.facing, quad.plane, quad.origin);
    let (across, along) = (quad.extent.secondary, quad.extent.primary);
    (0..across)
        .flat_map(move |secondary| {
            (0..along).map(move |primary| {
                VisibleFace::at(
                    voxel_at(
                        facing,
                        plane,
                        origin.primary + primary,
                        origin.secondary + secondary,
                    ),
                    side_of(facing),
                )
            })
        })
        .collect()
}

/// A generated section meshed, and the same section scanned.
fn judged(contents: Contents, around: [Option<Contents>; 6]) -> Result<Judged, Box<dyn Error>> {
    let registry = registry_declaring(&POOL)?;
    let section = built(contents, &registry)?;
    let neighbours = built_around(around, &registry)?;
    let mesh = mesh_section(&section, &neighbours_of(&neighbours), &registry)?;
    Ok(Judged {
        covered: mesh.quads().iter().flat_map(covered_by).collect(),
        visible: visible_faces(&section, &neighbourhood_of(&neighbours), &registry)?,
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    #[test]
    fn no_face_a_quad_covers_has_a_solid_voxel_against_it(
        contents in contents(),
        around in surroundings(),
    ) {
        let judged = judged(contents, around)
            .map_err(|failure| TestCaseError::fail(failure.to_string()))?;

        let buried: Vec<VisibleFace> = judged
            .covered
            .iter()
            .filter(|face| !judged.visible.contains(face))
            .copied()
            .collect();
        prop_assert!(
            buried.is_empty(),
            "a quad may only cover faces that are actually visible, so every one of the voxel \
             sides under it has something non-solid against it. {} of these do not, the first \
             being {:?} — a face drawn inside solid ground, which costs the renderer triangles \
             it can never see and, on a boundary, draws a wall between two chunks that meet",
            buried.len(),
            buried.first()
        );
    }

    #[test]
    fn every_face_an_independent_scan_finds_visible_is_covered_by_a_quad(
        contents in contents(),
        around in surroundings(),
    ) {
        let judged = judged(contents, around)
            .map_err(|failure| TestCaseError::fail(failure.to_string()))?;

        let covered: BTreeSet<VisibleFace> = judged.covered.iter().copied().collect();
        let missing: Vec<VisibleFace> = judged.visible.difference(&covered).copied().collect();
        prop_assert!(
            missing.is_empty(),
            "every face the independent scan finds visible is covered by some quad, including \
             the ones on a boundary whose neighbour was not generated — those are decided as \
             though nothing solid were beyond them, so they are visible rather than sealed. {} \
             are not covered, the first being {:?}; a hole in the world is the one defect a \
             player sees immediately and the mesher cannot",
            missing.len(),
            missing.first()
        );
    }

    #[test]
    fn no_two_quads_cover_the_same_face(
        contents in contents(),
        around in surroundings(),
    ) {
        let judged = judged(contents, around)
            .map_err(|failure| TestCaseError::fail(failure.to_string()))?;

        let distinct: BTreeSet<VisibleFace> = judged.covered.iter().copied().collect();
        prop_assert_eq!(
            judged.covered.len(),
            distinct.len(),
            "the quads partition the visible faces: each of them is under exactly one \
             rectangle. Two quads overlapping is invisible in a count of covered area and \
             invisible in a rendered frame — the second face is drawn in the same place as the \
             first — and it means a merge sweep that failed to mark what it consumed, which \
             degrades quietly with content rather than failing outright"
        );
    }
}
