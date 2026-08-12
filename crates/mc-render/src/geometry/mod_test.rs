//! Turning a merged quad into corners and triangles, in the section's world
//! frame.
//!
//! Four of these five tests are about a single arithmetic seam — where a face
//! sits relative to the voxel that emitted it — and they are written as a set
//! because the mistakes there hide inside each other. A build that adds one to
//! every facing's plane, rather than only to the positive ones, puts the top
//! face exactly where the first test expects it and the bottom face one block
//! too low, which is why there is a second test asserting a `-Y` face against
//! the same section origin and the same plane. A build that clamps a corner to
//! the voxel range passes both and then puts a `+X` face at plane 15 back at
//! x = 15, which is the third.
//!
//! The winding test is the one that cannot be replaced by inspection. Corner
//! order decides which side of a face the rasteriser keeps, the index pattern is
//! deliberately facing-independent, and a quad wound the wrong way round
//! produces a picture that is not subtly wrong but simply absent — so the normal
//! is recomputed here from the emitted corners in the emitted order, for all six
//! facings and both triangles, rather than assumed from the pattern.
//!
//! The fifth is the refusal. A quad naming a block whose texture never resolved
//! has no honest layer index: substituting layer 0 draws stone-coloured grass
//! and nothing downstream can tell that from a deliberate choice, so the whole
//! section's build fails and the error names the block.
//!
//! This is a sibling unit test and therefore sees the module's own fields. It
//! uses that for the index buffer alone, which has no public accessor and is
//! precisely what two of the scenarios here are about.

use std::collections::BTreeSet;
use std::error::Error;

use mc_core::id::{BlockName, TextureKey};
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};

use crate::texture::TextureLayers;

use super::{GeometryError, SectionGeometry, SectionOrigin, build_section_geometry};

type TestResult = Result<(), Box<dyn Error>>;

/// The block every quad below names, except the one that is supposed to fail.
const DRAWN_BLOCK: &str = "base:stone";

/// A block the resolved layers deliberately do not cover.
const UNCOVERED_BLOCK: &str = "base:dirt";

/// The section whose world origin the first two scenarios place their quads in.
/// None of its three coordinates is zero or equal to another, so a build that
/// added the wrong one, or added it to the wrong axis, lands somewhere visible
/// rather than on the same number by coincidence.
const SHIFTED_SECTION: [i32; 3] = [16, 0, 32];

/// A section at the world origin, so the remaining scenarios read section-local
/// and world coordinates as the same number.
const UNSHIFTED_SECTION: [i32; 3] = [0, 0, 0];

/// The six indices a quad becomes: two triangles over four corners, sharing the
/// diagonal. Written out rather than read from the module's own constant, which
/// would compare the implementation against itself.
const TWO_TRIANGLES: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// How many triangles one quad is wound into.
const TRIANGLES_PER_QUAD: usize = 2;

/// How many corners one quad has.
const CORNERS_PER_QUAD: usize = 4;

/// Which way each facing's outward normal points.
const OUTWARD_NORMALS: [(Facing, [f32; 3]); 6] = [
    (Facing::NegX, [-1.0, 0.0, 0.0]),
    (Facing::PosX, [1.0, 0.0, 0.0]),
    (Facing::NegY, [0.0, -1.0, 0.0]),
    (Facing::PosY, [0.0, 1.0, 0.0]),
    (Facing::NegZ, [0.0, 0.0, -1.0]),
    (Facing::PosZ, [0.0, 0.0, 1.0]),
];

/// Corner coordinates are sums of small integers and exact in `f32`, but
/// `clippy::float_cmp` is denied and applies to test code too, so every
/// comparison here runs on a fixed-point form instead. A 1/1024 block is far
/// finer than any error a wrong offset, a swapped axis or an inverted winding
/// could produce, and far coarser than `f32`'s own noise at these magnitudes.
const FIXED_POINT_SCALE: f32 = 1024.0;

fn fixed(value: f32) -> i64 {
    (value * FIXED_POINT_SCALE).round() as i64
}

fn fixed_vector(vector: [f32; 3]) -> [i64; 3] {
    let [x, y, z] = vector;
    [fixed(x), fixed(y), fixed(z)]
}

fn minus(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    let ([lx, ly, lz], [rx, ry, rz]) = (left, right);
    [lx - rx, ly - ry, lz - rz]
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    let ([lx, ly, lz], [rx, ry, rz]) = (left, right);
    [ly * rz - lz * ry, lz * rx - lx * rz, lx * ry - ly * rx]
}

fn unit(vector: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = vector;
    let length = x.hypot(y).hypot(z);
    [x / length, y / length, z / length]
}

/// Where a quad starts inside its plane.
const fn at(primary: u32, secondary: u32) -> PlanePos {
    PlanePos { primary, secondary }
}

/// How far a quad runs inside its plane.
const fn spanning(primary: u32, secondary: u32) -> PlaneExtent {
    PlaneExtent { primary, secondary }
}

/// Layers resolved over exactly `keys`, and nothing else.
fn layers_covering(keys: &[&str]) -> Result<TextureLayers, Box<dyn Error>> {
    let mut resolved = BTreeSet::new();
    for key in keys {
        resolved.insert(TextureKey::parse(key)?);
    }
    Ok(TextureLayers::resolve(&resolved))
}

/// One merged rectangle of the block the layers below do cover.
fn drawn_quad(
    facing: Facing,
    plane: u32,
    origin: PlanePos,
    extent: PlaneExtent,
) -> Result<Quad, Box<dyn Error>> {
    Ok(Quad {
        facing,
        plane,
        origin,
        extent,
        block: BlockName::parse(DRAWN_BLOCK)?,
    })
}

/// Every corner the build emitted, in world space and in emission order.
fn world_corners(geometry: &SectionGeometry) -> Vec<[f32; 3]> {
    (0usize..)
        .map_while(|position| geometry.world_corner(position))
        .collect()
}

/// The in-plane extent the axis scenario uses: 3 blocks along the primary axis
/// and 1 along the secondary.
///
/// Deliberately not square. A quad whose two in-plane extents are equal spans
/// the same distance whichever axis each is read onto, so a build that exchanged
/// the two would emit exactly the same corners and no assertion could see it.
const OBLONG_EXTENT: (u32, u32) = (3, 1);

/// The lowest and highest fixed-point coordinate on each axis across `corners`.
fn corner_bounds(corners: &[[f32; 3]]) -> ([i64; 3], [i64; 3]) {
    let mut lowest = [i64::MAX; 3];
    let mut highest = [i64::MIN; 3];
    for corner in corners {
        let axes = lowest.iter_mut().zip(highest.iter_mut()).zip(corner);
        for ((low, high), coordinate) in axes {
            *low = (*low).min(fixed(*coordinate));
            *high = (*high).max(fixed(*coordinate));
        }
    }
    (lowest, highest)
}

/// How far the emitted corners reach on each world axis.
fn corner_extents(geometry: &SectionGeometry) -> [i64; 3] {
    let (lowest, highest) = corner_bounds(&world_corners(geometry));
    let mut spans = [0i64; 3];
    for ((span, low), high) in spans.iter_mut().zip(lowest).zip(highest) {
        *span = high - low;
    }
    spans
}

/// The unit normal of each emitted triangle, computed from the corners in the
/// order the emitted indices name them.
fn triangle_normals(geometry: &SectionGeometry) -> Result<Vec<[i64; 3]>, Box<dyn Error>> {
    let corners = world_corners(geometry);
    let corner_at = |position: u32| -> Result<[f32; 3], Box<dyn Error>> {
        corners
            .get(position as usize)
            .copied()
            .ok_or_else(|| format!("index {position} names no emitted corner").into())
    };

    let mut normals = Vec::new();
    for triangle in geometry.indices.chunks_exact(3) {
        let &[first, second, third] = triangle else {
            return Err("a triangle is exactly three indices".into());
        };
        let anchor = corner_at(first)?;
        let spans = (
            minus(corner_at(second)?, anchor),
            minus(corner_at(third)?, anchor),
        );
        normals.push(fixed_vector(unit(cross(spans.0, spans.1))));
    }
    Ok(normals)
}

#[test]
fn a_merged_top_face_becomes_four_world_corners_and_two_triangles() -> TestResult {
    let layers = layers_covering(&[DRAWN_BLOCK])?;
    let quads = [drawn_quad(Facing::PosY, 3, at(2, 5), spanning(4, 2))?];

    let geometry = build_section_geometry(&quads, SectionOrigin::new(SHIFTED_SECTION), &layers)?;

    let corners = world_corners(&geometry);
    assert_eq!(
        (corners.len(), corner_bounds(&corners)),
        (
            CORNERS_PER_QUAD,
            (
                [fixed(18.0), fixed(4.0), fixed(37.0)],
                [fixed(22.0), fixed(4.0), fixed(39.0)],
            ),
        ),
        "a 4 x 2 top face on plane 3 of the section at {SHIFTED_SECTION:?} spans x 18 to 22 \
         and z 37 to 39 at y = 4"
    );
    assert_eq!(
        geometry.indices.as_slice(),
        TWO_TRIANGLES.as_slice(),
        "four corners are drawn as two triangles sharing a diagonal"
    );
    Ok(())
}

#[test]
fn a_bottom_face_sits_at_the_plane_of_the_voxel_that_emitted_it() -> TestResult {
    let layers = layers_covering(&[DRAWN_BLOCK])?;
    let quads = [drawn_quad(Facing::NegY, 3, at(2, 5), spanning(1, 1))?];

    let geometry = build_section_geometry(&quads, SectionOrigin::new(SHIFTED_SECTION), &layers)?;

    let ([_, lowest_y, _], [_, highest_y, _]) = corner_bounds(&world_corners(&geometry));
    assert_eq!(
        (lowest_y, highest_y),
        (fixed(3.0), fixed(3.0)),
        "a face pointing down leaves the emitting voxel at its own plane; only a face \
         pointing up is offset by one"
    );
    Ok(())
}

#[test]
fn a_face_on_the_last_plane_reaches_the_far_edge_of_the_section() -> TestResult {
    let layers = layers_covering(&[DRAWN_BLOCK])?;
    let quads = [drawn_quad(Facing::PosX, 15, at(0, 0), spanning(1, 1))?];

    let geometry = build_section_geometry(&quads, SectionOrigin::new(UNSHIFTED_SECTION), &layers)?;

    let ([lowest_x, _, _], [highest_x, _, _]) = corner_bounds(&world_corners(&geometry));
    assert_eq!(
        (lowest_x, highest_x),
        (fixed(16.0), fixed(16.0)),
        "the face a voxel at plane 15 emits along +X sits at x = 16, one past the last \
         voxel coordinate"
    );
    Ok(())
}

#[test]
fn every_facings_triangles_wind_so_that_their_normal_points_outward() -> TestResult {
    let layers = layers_covering(&[DRAWN_BLOCK])?;
    let mut observed = Vec::new();
    let mut expected = Vec::new();

    for (facing, outward) in OUTWARD_NORMALS {
        let quads = [drawn_quad(facing, 5, at(2, 3), spanning(1, 1))?];
        let geometry =
            build_section_geometry(&quads, SectionOrigin::new(UNSHIFTED_SECTION), &layers)?;

        observed.push((facing, triangle_normals(&geometry)?));
        expected.push((facing, vec![fixed_vector(outward); TRIANGLES_PER_QUAD]));
    }

    assert_eq!(
        observed, expected,
        "the normal computed from each triangle's own emitted corner order must point along \
         its facing's outward axis; a facing whose normal is negated is a face the rasteriser \
         will discard"
    );
    Ok(())
}

#[test]
fn a_quads_longer_side_runs_along_its_facings_primary_plane_axis() -> TestResult {
    let layers = layers_covering(&[DRAWN_BLOCK])?;
    let (primary, secondary) = OBLONG_EXTENT;
    let mut observed = Vec::new();

    for facing in [Facing::PosX, Facing::PosZ] {
        let quads = [drawn_quad(
            facing,
            5,
            at(2, 3),
            spanning(primary, secondary),
        )?];
        let geometry =
            build_section_geometry(&quads, SectionOrigin::new(UNSHIFTED_SECTION), &layers)?;
        observed.push((facing, corner_extents(&geometry)));
    }

    assert_eq!(
        observed,
        vec![
            (Facing::PosX, [fixed(0.0), fixed(3.0), fixed(1.0)]),
            (Facing::PosZ, [fixed(3.0), fixed(1.0), fixed(0.0)]),
        ],
        "the fixture is deliberately {primary} by {secondary} rather than square, because a \
         square quad spans the same distance whichever way its two plane axes are read. A \
         +X face's plane axes are y then z, and a +Z face's are x then y, so exchanging the \
         primary and the secondary axis turns a wall three blocks tall into one three blocks \
         wide. Only X and Z facings can show it: the top-face scenario above already pins \
         the mapping for a Y facing."
    );
    Ok(())
}

#[test]
fn a_quad_naming_an_unresolved_texture_fails_the_section_and_names_the_block() -> TestResult {
    let layers = layers_covering(&[DRAWN_BLOCK])?;
    let quads = [Quad {
        facing: Facing::PosY,
        plane: 3,
        origin: at(0, 0),
        extent: spanning(1, 1),
        block: BlockName::parse(UNCOVERED_BLOCK)?,
    }];

    let refusal = build_section_geometry(&quads, SectionOrigin::new(UNSHIFTED_SECTION), &layers)
        .err()
        .ok_or(
            "a quad naming a block with no resolved texture layer must fail the section's \
             build; there is no geometry to hand back and no honest layer to substitute",
        )?;

    match refusal {
        GeometryError::UnresolvedTexture { block } => assert_eq!(
            block.as_str(),
            UNCOVERED_BLOCK,
            "the refusal must name the block whose texture never resolved"
        ),
        other => {
            return Err(format!("expected an unresolved-texture refusal, got {other:?}").into());
        }
    }
    Ok(())
}
