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
//! The refusals are the same shape one level widened. A quad whose *facing* has
//! no honest layer index has none for two distinct reasons — the content states
//! no such block at all, or it states one whose key for that facing occupies no
//! layer — and substituting layer 0 draws stone-coloured grass in both cases,
//! which nothing downstream can tell from a deliberate choice. The whole
//! section's build fails and the error names the block, the facing and, where
//! there is one, the key.
//!
//! The two resolution readings are what say a face draws from its **declaration**
//! and not from its block's name. Every block this repository ships spells the
//! two alike, so the property is unobservable except against a fixture that
//! deliberately does not — which is why the block in those two is named for one
//! mineral and declares another.
//!
//! This is a sibling unit test and therefore sees the module's own fields. It
//! uses that for the index buffer alone, which has no public accessor and is
//! precisely what two of the scenarios here are about.

use std::collections::BTreeSet;
use std::error::Error;

use mc_core::block::Opacity;
use mc_core::content::{Face, FaceTextures};
use mc_core::id::{BlockName, TextureKey};
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};

use crate::texture::{TextureLayers, TextureResolution};

use super::{GeometryError, SectionGeometry, SectionOrigin, build_section_geometry};

type TestResult = Result<(), Box<dyn Error>>;

/// The block every quad below names, except the one that is supposed to fail.
const DRAWN_BLOCK: &str = "base:stone";

/// A block the resolution deliberately states nothing about at all, which is
/// what a section still holding quads for a block a reload dropped would name.
const UNSTATED_BLOCK: &str = "base:dirt";

/// A block whose name is not the key it declares, and the key it declares.
///
/// The whole subject of the resolution readings: a packer that parses a block's
/// name as a texture key resolves this one to nothing, or — worse — to whichever
/// layer happens to sit where it looked.
const RENAMED_BLOCK: &str = "example:amber";
const ITS_DECLARED_KEY: &str = "example:gold";

/// A second block beside it, so that the assignment holds more than one layer
/// and "the layer it drew from" is a choice rather than the only answer there is.
const NEIGHBOURING_BLOCK: &str = "example:jade";

/// The layers those two keys hold, and they are deliberately not their sorted
/// positions: `example:gold` sorts ahead of `example:jade` and holds the higher
/// layer. Neither is layer zero for the renamed block, so an implementation
/// falling back to zero fails rather than drawing something plausible.
const SUBSTITUTED_ASSIGNMENT: [(&str, u16); 2] = [(ITS_DECLARED_KEY, 1), (NEIGHBOURING_BLOCK, 0)];

/// A block declaring a different key on each of its six facings.
/// Three blocks whose sorted order is deliberately not their emission order.
///
/// `example:amber` sorts first and so takes layer 0, `example:glass` layer 1 and
/// `example:zinc` layer 2; the fixture emits zinc before amber, so a packer that
/// sorted by layer would swap them and a packer that preserved the mesher's
/// order would not.
const SEEN_THROUGH: &str = "example:glass";
const EARLY_IN_THE_SORT: &str = "example:amber";
const LATE_IN_THE_SORT: &str = "example:zinc";

const BANDED_BLOCK: &str = "example:banded";

/// The six keys it declares, positionally in the order a declaration writes its
/// facings: up, down, north, south, east and west.
const SIX_DECLARED: [&str; 6] = [
    "example:cap",
    "example:floor",
    "example:unlit",
    "example:lit",
    "example:dawn",
    "example:dusk",
];

/// The one of the six the assignment deliberately does not cover, and the facing
/// it is declared against.
const UNCOVERED_KEY: &str = "example:unlit";
const ITS_FACING: Face = Face::North;
const ITS_QUADS_FACING: Facing = Facing::NegZ;

/// A facing of the same block whose key *is* covered, so that a refusal about
/// one facing is not satisfied by a block that refuses on all of them.
const COVERED_KEY: &str = "example:cap";
const A_COVERED_FACING: Facing = Facing::PosY;

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

/// A resolution stating one block per entry, each declaring the key written
/// beside it on all six of its facings, over layers covering exactly those keys.
///
/// The block and the key are two parameters because they are two things. Every
/// fixture in this file that is not about the difference happens to spell them
/// alike, and the ones that are about it pass two different strings.
fn resolving(blocks: &[(&str, &str)]) -> Result<TextureResolution, Box<dyn Error>> {
    let mut stated = Vec::new();
    let mut keys = BTreeSet::new();
    for (block, key) in blocks {
        let parsed = TextureKey::parse(key)?;
        keys.insert(parsed.clone());
        stated.push((
            BlockName::parse(block)?,
            FaceTextures::uniform(parsed),
            Opacity::OPAQUE,
        ));
    }
    Ok(TextureResolution::stating(
        stated,
        TextureLayers::resolve(&keys),
    ))
}

/// A resolution over one block declaring `key` on all six facings, against the
/// layers `assignment` states rather than the ones a sort would give.
fn stating(
    block: &str,
    key: &str,
    assignment: &[(&str, u16)],
) -> Result<TextureResolution, Box<dyn Error>> {
    let mut layers = Vec::new();
    for (named, layer) in assignment {
        layers.push((TextureKey::parse(named)?, *layer));
    }
    Ok(TextureResolution::stating(
        [(
            BlockName::parse(block)?,
            FaceTextures::uniform(TextureKey::parse(key)?),
            Opacity::OPAQUE,
        )],
        TextureLayers::stated(layers),
    ))
}

/// A resolution over [`BANDED_BLOCK`] declaring [`SIX_DECLARED`], covering every
/// one of those keys except [`UNCOVERED_KEY`].
fn banded_but_for_one_facing() -> Result<TextureResolution, Box<dyn Error>> {
    let mut declared = Vec::with_capacity(SIX_DECLARED.len());
    let mut covered = BTreeSet::new();
    for key in SIX_DECLARED {
        let parsed = TextureKey::parse(key)?;
        if key != UNCOVERED_KEY {
            covered.insert(parsed.clone());
        }
        declared.push(parsed);
    }
    let keys: [TextureKey; 6] = declared
        .try_into()
        .map_err(|_unexpected| "a declaration states exactly six facings")?;
    Ok(TextureResolution::stating(
        [(
            BlockName::parse(BANDED_BLOCK)?,
            FaceTextures::stating(keys),
            Opacity::OPAQUE,
        )],
        TextureLayers::resolve(&covered),
    ))
}

#[test]
fn quads_that_stop_all_the_light_are_emitted_before_those_that_pass_some_of_it() -> TestResult {
    // Emitted interleaved and out of layer order, so that three plausible wrong
    // answers are each a different list: no partition at all, a partition that
    // reordered within a half, and a sort by layer index.
    let resolution = resolving_at(&[
        (SEEN_THROUGH, 0.5),
        (LATE_IN_THE_SORT, 1.0),
        (EARLY_IN_THE_SORT, 1.0),
    ])?;
    let quads = [
        quad_of(Facing::PosY, SEEN_THROUGH)?,
        quad_of(Facing::PosY, LATE_IN_THE_SORT)?,
        quad_of(Facing::PosY, EARLY_IN_THE_SORT)?,
        quad_of(Facing::PosY, SEEN_THROUGH)?,
    ];

    let geometry =
        build_section_geometry(&quads, SectionOrigin::new(UNSHIFTED_SECTION), &resolution)?;

    assert_eq!(
        (
            geometry.opaque_quad_count(),
            geometry.quad_count(),
            corner_layers(&geometry),
        ),
        (2, 4, vec![2, 2, 2, 2, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1]),
        "the two terrain draws read fixed halves of one index buffer, so a section's opaque          quads have to come first and one number has to say where they end. The layer list is          what says which quad went where, and the fixture is arranged so that every wrong answer          is a different list: no partition gives [1, 2, 0, 1], a sort by layer gives          [0, 1, 1, 2], and a partition that reordered inside a half gives [0, 2, 1, 1] — the          mesher's order within each half is preserved, which `sweep.rs` forbids re-sorting"
    );
    Ok(())
}

/// A resolution stating one block per entry at the degree written beside it,
/// each declaring a key equal to its own name.
///
/// The layers come from sorting those keys, so a block's name decides its layer
/// index — which is what lets a fixture put emission order and layer order
/// deliberately at odds.
fn resolving_at(blocks: &[(&str, f32)]) -> Result<TextureResolution, Box<dyn Error>> {
    let mut stated = Vec::new();
    let mut keys = BTreeSet::new();
    for (block, degree) in blocks {
        let parsed = TextureKey::parse(block)?;
        keys.insert(parsed.clone());
        stated.push((
            BlockName::parse(block)?,
            FaceTextures::uniform(parsed),
            Opacity::new(*degree).ok_or("a fixture states a degree the engine can keep")?,
        ));
    }
    Ok(TextureResolution::stating(
        stated,
        TextureLayers::resolve(&keys),
    ))
}

/// The layer every emitted corner was packed with, in emission order.
fn corner_layers(geometry: &SectionGeometry) -> Vec<u16> {
    (0usize..)
        .map_while(|corner| geometry.layer_at(corner))
        .collect()
}

/// What packing `quads` against `resolution` came to.
///
/// A total verdict rather than a `Result` propagated out of a test: a build that
/// accepted what it should refuse fails on its own comparison, naming what it
/// produced, instead of ending the test before its assertion ran.
#[derive(Debug, PartialEq, Eq)]
enum Packed {
    /// The layer every corner was packed with, in emission order.
    Corners(Vec<u16>),
    /// The section was refused, naming this block, this facing and this key.
    Refused {
        block: String,
        face: Face,
        key: Option<String>,
    },
    /// The section was refused for a reason that is not about a texture.
    RefusedOtherwise(String),
}

fn packing(quads: &[Quad], resolution: &TextureResolution) -> Packed {
    match build_section_geometry(quads, SectionOrigin::new(UNSHIFTED_SECTION), resolution) {
        Ok(geometry) => Packed::Corners(corner_layers(&geometry)),
        Err(GeometryError::UnresolvedTexture { block, face, key }) => Packed::Refused {
            block: block.as_str().to_owned(),
            face,
            key: key.map(|key| key.as_str().to_owned()),
        },
        Err(other) => Packed::RefusedOtherwise(other.to_string()),
    }
}

/// One quad of `block`, facing `facing`, covering a single voxel side.
fn quad_of(facing: Facing, block: &str) -> Result<Quad, Box<dyn Error>> {
    Ok(Quad {
        facing,
        plane: 3,
        origin: at(0, 0),
        extent: spanning(1, 1),
        block: BlockName::parse(block)?,
    })
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
    let resolution = resolving(&[(DRAWN_BLOCK, DRAWN_BLOCK)])?;
    let quads = [drawn_quad(Facing::PosY, 3, at(2, 5), spanning(4, 2))?];

    let geometry =
        build_section_geometry(&quads, SectionOrigin::new(SHIFTED_SECTION), &resolution)?;

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
    let resolution = resolving(&[(DRAWN_BLOCK, DRAWN_BLOCK)])?;
    let quads = [drawn_quad(Facing::NegY, 3, at(2, 5), spanning(1, 1))?];

    let geometry =
        build_section_geometry(&quads, SectionOrigin::new(SHIFTED_SECTION), &resolution)?;

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
    let resolution = resolving(&[(DRAWN_BLOCK, DRAWN_BLOCK)])?;
    let quads = [drawn_quad(Facing::PosX, 15, at(0, 0), spanning(1, 1))?];

    let geometry =
        build_section_geometry(&quads, SectionOrigin::new(UNSHIFTED_SECTION), &resolution)?;

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
    let resolution = resolving(&[(DRAWN_BLOCK, DRAWN_BLOCK)])?;
    let mut observed = Vec::new();
    let mut expected = Vec::new();

    for (facing, outward) in OUTWARD_NORMALS {
        let quads = [drawn_quad(facing, 5, at(2, 3), spanning(1, 1))?];
        let geometry =
            build_section_geometry(&quads, SectionOrigin::new(UNSHIFTED_SECTION), &resolution)?;

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
    let resolution = resolving(&[(DRAWN_BLOCK, DRAWN_BLOCK)])?;
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
            build_section_geometry(&quads, SectionOrigin::new(UNSHIFTED_SECTION), &resolution)?;
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
fn a_face_draws_the_key_its_block_declared_when_that_key_is_not_its_name() -> TestResult {
    let resolution = stating(RENAMED_BLOCK, ITS_DECLARED_KEY, &SUBSTITUTED_ASSIGNMENT)?;
    let mut quads = Vec::new();
    for (facing, _) in OUTWARD_NORMALS {
        quads.push(quad_of(facing, RENAMED_BLOCK)?);
    }

    let packed = packing(&quads, &resolution);

    assert_eq!(
        packed,
        Packed::Corners(vec![
            layer_stated(ITS_DECLARED_KEY)?;
            OUTWARD_NORMALS.len() * CORNERS_PER_QUAD
        ]),
        "a face draws the key its block declared, on every one of its six facings, and never the \
         key its block's name spells. This block is called {RENAMED_BLOCK} and declares \
         {ITS_DECLARED_KEY}: a packer parsing the name resolves nothing at all, and one falling \
         back to layer zero draws whichever block owns layer zero — which is a picture that is \
         wrong in an entirely plausible way"
    );
    Ok(())
}

#[test]
fn a_facing_key_outside_the_assignment_refuses_the_section_naming_the_block_and_the_facing()
-> TestResult {
    let resolution = banded_but_for_one_facing()?;

    let uncovered = packing(&[quad_of(ITS_QUADS_FACING, BANDED_BLOCK)?], &resolution);
    let covered = packing(&[quad_of(A_COVERED_FACING, BANDED_BLOCK)?], &resolution);

    assert_eq!(
        (uncovered, covered),
        (
            Packed::Refused {
                block: BANDED_BLOCK.to_owned(),
                face: ITS_FACING,
                key: Some(UNCOVERED_KEY.to_owned()),
            },
            Packed::Corners(vec![layer_resolved(COVERED_KEY)?; CORNERS_PER_QUAD]),
        ),
        "a block with six keys refuses on the facing whose key occupies no layer, and the refusal \
         has to name the facing as well as the block — otherwise a reader holding six keys is \
         told one of them is wrong and left to guess which. Drawing layer zero instead would show \
         a plausible picture and report nothing at all. The second half is what says the refusal \
         is about that facing and not about that block: the same block's covered facing still packs"
    );
    Ok(())
}

#[test]
fn a_quad_naming_a_block_the_content_states_nothing_about_is_refused_with_no_key() -> TestResult {
    let resolution = resolving(&[(DRAWN_BLOCK, DRAWN_BLOCK)])?;

    let packed = packing(&[quad_of(Facing::PosY, UNSTATED_BLOCK)?], &resolution);

    assert_eq!(
        packed,
        Packed::Refused {
            block: UNSTATED_BLOCK.to_owned(),
            face: Face::Up,
            key: None,
        },
        "a section may still hold quads for a block the content no longer states — a reload that \
         dropped it, a mesh that outlived its registry — and there is no key to name for one. The \
         refusal carries `None` rather than inventing a key from the block's name, which is the \
         habit this whole change exists to end"
    );
    Ok(())
}

/// The layer [`SUBSTITUTED_ASSIGNMENT`] states for `key`.
///
/// # Errors
///
/// Returns an error if the table names no layer for it.
fn layer_stated(key: &str) -> Result<u16, Box<dyn Error>> {
    SUBSTITUTED_ASSIGNMENT
        .into_iter()
        .find(|(named, _)| *named == key)
        .map(|(_, layer)| layer)
        .ok_or_else(|| format!("this fixture states no layer for `{key}`").into())
}

/// The layer a lexicographic assignment over the covered five of
/// [`SIX_DECLARED`] gives `key`.
///
/// # Errors
///
/// Returns an error if `key` is not a namespaced id or is the uncovered one.
fn layer_resolved(key: &str) -> Result<u16, Box<dyn Error>> {
    let covered: BTreeSet<&str> = SIX_DECLARED
        .into_iter()
        .filter(|declared| *declared != UNCOVERED_KEY)
        .collect();
    covered
        .iter()
        .position(|declared| *declared == key)
        .and_then(|position| u16::try_from(position).ok())
        .ok_or_else(|| format!("`{key}` is not one of the covered five").into())
}
