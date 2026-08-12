//! The three committed fixtures, meshed, and the quantities each of them is
//! worth.
//!
//! **No number in this file was read off a mesher run.** That is the whole
//! discipline of it. A count committed from the first green run makes every later
//! assertion circular — a mesher that emitted nothing would have `0` committed
//! for it and would pass forever, which is the exact mistake this project has
//! already found once. So each quantity is derived here instead: six sides on a
//! cube by inspection, a checkerboard's quads as its own solid voxels counted
//! through the registry times the six sides each of them has, and the terrain
//! fixture by no committed number at all — its quads must cover exactly the faces
//! an independent per-voxel scan, sharing no code with the mesher, reports as
//! visible.
//!
//! The last two of those belong together and neither is optional. "At most half
//! as many quads as visible faces" is comfortably true of a mesh holding no
//! quads, so the covered-area comparison is what stands between that ceiling and
//! an empty answer; the ceiling is what says the merging did any work.
//!
//! The fixtures, the oracle and this file all reach each other through the
//! benchmark's own support module, which is where the specification puts the
//! fixtures — committed with the benchmark they were written for.

mod mesh_common;

#[path = "../benches/support/mod.rs"]
mod support;

use std::collections::BTreeSet;
use std::error::Error;

use mc_world::mesh::{Neighbours, Quad, SectionMesh, mesh_section};
use mesh_common::{TestResult, every_position, some_quads};
use support::fixtures::{self, Fixture};
use support::oracle::{Neighbourhood, visible_faces};

/// How many sides a voxel has, and therefore how many faces a solid voxel with
/// nothing solid around it shows.
const SIDES_OF_A_VOXEL: usize = 6;

/// The ceiling merging is held to on realistic content, written as a multiplier
/// on the quad count rather than as a division of the face count —
/// `clippy::integer_division` is a gate error, and a halved count would round.
const AT_MOST_HALF: usize = 2;

/// A fixture meshed with no neighbour supplied.
///
/// # Errors
///
/// Returns an error if the fixture cannot be meshed against its own registry.
fn meshed(fixture: &Fixture) -> Result<SectionMesh, Box<dyn Error>> {
    Ok(mesh_section(
        &fixture.section,
        &Neighbours::none(),
        &fixture.registry,
    )?)
}

/// How many of a fixture's voxels its own registry reports as solid.
///
/// Counted through the public per-voxel read, so what is counted is the solidity
/// each block was registered with — a count that recognised a name would be a
/// different quantity wearing this one's clothes.
fn solid_voxel_count(fixture: &Fixture) -> Result<usize, Box<dyn Error>> {
    let mut solid = 0;
    for position in every_position() {
        solid += usize::from(fixture.section.is_solid_at(position, &fixture.registry)?);
    }
    Ok(solid)
}

/// How many faces of a fixture the independent scan finds visible, with no
/// neighbour supplied.
fn visible_face_count(fixture: &Fixture) -> Result<usize, Box<dyn Error>> {
    let found = visible_faces(
        &fixture.section,
        &Neighbourhood::default(),
        &fixture.registry,
    )?;
    Ok(found.len())
}

/// How many voxel sides the quads cover in total, counting each quad's whole
/// rectangle.
fn covered_faces(quads: &[Quad]) -> usize {
    quads
        .iter()
        .map(|quad| (quad.extent.primary * quad.extent.secondary) as usize)
        .sum()
}

#[test]
fn an_entirely_solid_section_meshes_to_one_quad_for_each_of_its_six_sides() -> TestResult {
    let fixture = fixtures::solid()?;

    let mesh = meshed(&fixture)?;

    assert_eq!(
        mesh.quads().len(),
        SIDES_OF_A_VOXEL,
        "a section every voxel of which is solid, with nothing loaded beside it, is a cube: \
         every interior face is hidden by the voxel next to it and each of the six outer \
         planes is one unbroken rectangle. Six is what a cube has, counted by looking at one \
         rather than by running this mesher and writing down what it said"
    );
    Ok(())
}

#[test]
fn a_checkerboard_meshes_to_one_quad_for_every_side_of_every_solid_voxel() -> TestResult {
    let fixture = fixtures::checkerboard()?;
    let solid_voxels = solid_voxel_count(&fixture)?;

    let mesh = meshed(&fixture)?;

    assert_eq!(
        mesh.quads().len(),
        solid_voxels * SIDES_OF_A_VOXEL,
        "in a checkerboard no solid voxel touches another, so every one of them shows all six \
         of its sides and no two of those sides are ever adjacent. The expected number is that \
         product, worked out here from a solid-voxel count taken through the registry — the \
         one form of it that cannot degenerate into whatever the mesher happened to emit the \
         day the number was written down"
    );
    Ok(())
}

#[test]
fn every_quad_of_a_checkerboard_covers_exactly_one_voxel_side() -> TestResult {
    let fixture = fixtures::checkerboard()?;

    let mesh = meshed(&fixture)?;

    let extents: BTreeSet<(u32, u32)> = mesh
        .quads()
        .iter()
        .map(|quad| (quad.extent.primary, quad.extent.secondary))
        .collect();
    assert_eq!(
        extents,
        BTreeSet::from([(1, 1)]),
        "nothing in a checkerboard can merge, because no two visible faces of the same facing \
         are ever neighbours. Comparing the set of extents that occur, rather than counting \
         the ones that are 1x1, is deliberate: a mesh holding no quads at all also holds no \
         extent other than 1x1, and would pass the counted form"
    );
    Ok(())
}

#[test]
fn the_terrain_quads_cover_exactly_the_faces_an_independent_scan_finds_visible() -> TestResult {
    let fixture = fixtures::terrain()?;
    let visible = visible_face_count(&fixture)?;

    let mesh = meshed(&fixture)?;

    assert_eq!(
        covered_faces(mesh.quads()),
        visible,
        "every visible face is covered by exactly one quad, so the areas of the quads sum to \
         the number of visible faces — no more, which would mean a face was covered twice or a \
         quad ran over a hidden face, and no fewer, which would mean a face was dropped. The \
         right-hand side is counted by a scan that shares no code with the mesher and derives \
         its own adjacency, so no number had to be committed for this fixture at all"
    );
    Ok(())
}

#[test]
fn the_terrain_faces_merge_into_at_most_half_as_many_quads_as_there_are_faces() -> TestResult {
    let fixture = fixtures::terrain()?;
    let visible = visible_face_count(&fixture)?;

    let mesh = meshed(&fixture)?;

    let quads = some_quads(&mesh)?;
    assert!(
        quads.len() * AT_MOST_HALF <= visible,
        "terrain is long flat runs broken by a rough surface, so merging must at least halve \
         the {visible} visible faces it contains; this mesh used {} quads. A mesher that \
         emitted one quad per face would be correct and useless, and the renderer this feeds \
         inherits the number",
        quads.len()
    );
    Ok(())
}
