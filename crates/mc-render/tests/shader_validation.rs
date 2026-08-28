//! The shader validation the build performs, exercised as the build performs
//! it.
//!
//! This file includes `build/validate.rs` through `#[path]`, which is the same
//! file `build.rs` includes. There is one validator, not a validator and a test
//! double that agrees with it — a second copy would let the build accept what
//! this file rejects, which is the one failure a build-time check exists to
//! prevent.
//!
//! Nothing here creates a device, and that is structural rather than a promise:
//! the validator names no GPU type, this target carries no `required-features`,
//! and the quality gate runs it in the configuration where `wgpu` is not in the
//! dependency graph at all.
//!
//! Three of the checks below are not scenarios. They are the facts this design
//! depends on that nothing else can check mechanically:
//!
//! - **The storage-binding budget.** The weakest adapter in the declared range
//!   offers four storage buffers per shader stage. A fifth is not a refactor,
//!   it is that adapter dropping out of the supported set, so it fails the build
//!   — and the accepting half of that test is what keeps the check from being a
//!   validator that simply refuses everything.
//! - **The winding literal.** The cull shader carries its own copy of the six
//!   indices the geometry builder emits, because reading the CPU's index buffer
//!   on the GPU would need that fifth binding. `build.rs` cannot depend on the
//!   crate it builds, so `build/validate.rs` necessarily holds a *third* copy of
//!   that array — and an agreement check between the shader and a private copy
//!   agrees with itself. The assertion that the validator's copy equals the
//!   renderer's constant is the only thing closing that loop, so it lives in the
//!   same test as the check it makes meaningful.
//! - **The plane-axis table.** The terrain shader picks the two components a
//!   face's texture runs along, and the geometry builder places that face's
//!   corners under the same convention. Nothing tied the two together: a drift
//!   runs the texture *across* a face instead of along it, which leaves the
//!   face's mean colour untouched, so no derived probe sees it — and a golden
//!   minted from the drifted renderer records the drift as ground truth. Same
//!   three-part shape as the winding literal, and for the same reason.
//!
//!   **One row per facing, not one per axis, and that is the whole point.** An
//!   axis-indexed table of three rows would leave the shader deriving the axis
//!   as `facing >> 1u` — a second, unguarded expression of `Facing`'s
//!   declaration order, which is the expression conductor ruling 31 names. A
//!   facing-indexed table of six rows removes that arithmetic entirely, so the
//!   only fact the shader still authors is the literal below. Nothing on the
//!   Rust side ties a facing's discriminant to its axis: `Facing::axis()` is a
//!   match on the variant, so reordering the enum leaves `mc-world`'s suite and
//!   `mc-render`'s suite green while moving four of the six rows the shader
//!   reads.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use mc_core::block::Opacity;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::geometry::vertex::{PackedVertex, Vertex};
use mc_render::geometry::{SectionOrigin, build_section_geometry};
use mc_world::mesh::Facing;
use tempfile::TempDir;

#[path = "../build/validate.rs"]
mod validate;

use validate::{
    PLANE_AXES, QUAD_INDEX_PATTERN, SECTION_RECORD, ShaderError, VERTEX_LAYOUT,
    validate_shader_directory,
};

type TestResult = Result<(), Box<dyn Error>>;

/// A shader whose first defect is on line 5 and whose second is on line 6, so
/// reporting the *first* error is distinguishable from reporting the last.
const INVALID_NAME: &str = "broken.wgsl";
const DEFECT_LINE: u32 = 5;
const INVALID_SOURCE: &str = "\
// A deliberately invalid shader. The expression after each `=` is missing, so
// the parser fails on the first of the two semicolons below.
@compute @workgroup_size(1)
fn broken() {
    let first: f32 = ;
    let second: f32 = ;
}
";

/// How many storage buffers one shader stage may bind on the weakest adapter in
/// the declared hardware range.
const STORAGE_BUDGET: usize = 4;

const OVER_BUDGET_NAME: &str = "overbudget.wgsl";
const OVER_BUDGET_SOURCE: &str = "\
@group(0) @binding(0) var<storage, read_write> sink: array<u32>;
@group(0) @binding(1) var<storage, read> first: array<u32>;
@group(0) @binding(2) var<storage, read> second: array<u32>;
@group(0) @binding(3) var<storage, read> third: array<u32>;
@group(0) @binding(4) var<storage, read> fourth: array<u32>;

@compute @workgroup_size(1)
fn main() {
    sink[0] = first[0] + second[0] + third[0] + fourth[0];
}
";

const WITHIN_BUDGET_NAME: &str = "withinbudget.wgsl";
const WITHIN_BUDGET_SOURCE: &str = "\
@group(0) @binding(0) var<storage, read_write> sink: array<u32>;
@group(0) @binding(1) var<storage, read> first: array<u32>;
@group(0) @binding(2) var<storage, read> second: array<u32>;
@group(0) @binding(3) var<storage, read> third: array<u32>;

@compute @workgroup_size(1)
fn main() {
    sink[0] = first[0] + second[0] + third[0];
}
";

/// The entry point both budget fixtures declare.
const BUDGET_ENTRY_POINT: &str = "main";

/// The file the winding literal is read from.
const CULL_NAME: &str = "cull.wgsl";

/// The last two indices exchanged, which is a quad wound into two triangles
/// that overlap instead of two that tile it — visible as a hole, not as a
/// wrong colour.
const DISAGREEING_PATTERN: [u32; 6] = [0, 1, 2, 0, 3, 2];
const DISAGREEING_CULL_SOURCE: &str = "\
const QUAD_INDEX_PATTERN: array<u32, 6> = array<u32, 6>(0u, 1u, 2u, 0u, 3u, 2u);

@compute @workgroup_size(1)
fn cull() {
    let corner = QUAD_INDEX_PATTERN[0];
}
";

/// The file the plane-axis table is read from.
const TERRAIN_NAME: &str = "terrain.wgsl";

/// The `-X` face's two plane axes exchanged, and nothing else.
///
/// That is the drift this guard exists for: the face still draws, still shows
/// its own texture and still averages to the same colour, so FR-8.2's colour
/// probes cannot see it and neither can a golden shot from the drifted
/// renderer. Only the two sides agreeing in writing catches it.
///
/// The table has one row per **facing**, not one per axis, so exchanging two
/// whole rows is the other drift it refuses — and that one is a reordering of
/// `Facing`'s own declaration, which every Rust answer survives because
/// `Facing::axis()` is a match on the variant rather than on its discriminant.
const DISAGREEING_PLANE_AXES: [u32; 12] = [2, 1, 1, 2, 0, 2, 0, 2, 0, 1, 0, 1];
const DISAGREEING_TERRAIN_SOURCE: &str = "\
const PLANE_AXES: array<u32, 12> =
    array<u32, 12>(2u, 1u, 1u, 2u, 0u, 2u, 0u, 2u, 0u, 1u, 0u, 1u);

@compute @workgroup_size(1)
fn terrain() {
    let component = PLANE_AXES[0];
}
";

/// A plane-axis table as a refusal reports it: each facing's primary component
/// followed by its secondary, in `Facing`'s own declaration order.
fn flattened(table: [[u32; 2]; 6]) -> Vec<u32> {
    table.into_iter().flatten().collect()
}

/// A directory holding exactly `files`.
fn directory_holding(files: &[(&str, &str)]) -> Result<TempDir, Box<dyn Error>> {
    let directory = TempDir::new()?;
    for (name, source) in files {
        fs::write(directory.path().join(name), source)?;
    }
    Ok(directory)
}

/// The shipped shader directory, resolved from this crate's own manifest.
fn shipped_directory() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders")
}

/// Every `.wgsl` file in `directory`, enumerated by this test rather than by the
/// validator, so "all of them" means the directory's contents and not whatever
/// the validator chose to look at.
fn shipped_sources(directory: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension() != Some(OsStr::new("wgsl")) {
            continue;
        }
        let name = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or("a shader file name must be readable text")?;
        names.push(name.to_owned());
    }
    names.sort();
    Ok(names)
}

/// The refusal `directory` produces, or an error saying it produced none.
fn refusal_over(directory: &Path) -> Result<ShaderError, Box<dyn Error>> {
    validate_shader_directory(directory)
        .err()
        .ok_or_else(|| format!("{} was expected to fail validation", directory.display()).into())
}

#[test]
fn every_shipped_shader_source_passes_the_validation_the_build_performs() -> TestResult {
    let directory = shipped_directory();

    let validated = validate_shader_directory(&directory)?;
    let shipped = shipped_sources(&directory)?;

    assert_eq!(
        (validated.is_empty(), validated),
        (false, shipped),
        "the validation must accept every `.wgsl` file the crate ships and must have looked \
         at all of them — a validator that skipped a file, or that found none to look at, \
         reports success over a set that is not the shipped one"
    );
    Ok(())
}

#[test]
fn an_invalid_shader_is_refused_by_file_name_and_the_location_of_its_first_error() -> TestResult {
    let directory = directory_holding(&[(INVALID_NAME, INVALID_SOURCE)])?;

    let refusal = refusal_over(directory.path())?;
    let reported = refusal.to_string();
    match refusal {
        ShaderError::Invalid { file, line, .. } => assert_eq!(
            (file.as_str(), line),
            (INVALID_NAME, DEFECT_LINE),
            "the refusal must name the failing file and the line the *first* defect sits on; \
             this source carries a second defect on the following line"
        ),
        other => return Err(format!("expected an invalid-source refusal, got {other:?}").into()),
    }

    assert!(
        reported.contains(INVALID_NAME) && reported.contains(&DEFECT_LINE.to_string()),
        "a build failure a developer has to act on must carry the file and the location in \
         its message, not only in its fields; this one read `{reported}`"
    );
    Ok(())
}

#[test]
fn a_shader_directory_holding_no_source_fails_and_names_the_directory_it_searched() -> TestResult {
    let directory = directory_holding(&[("README.txt", "this directory holds no shader")])?;

    let refusal = refusal_over(directory.path())?;
    let reported = refusal.to_string();
    match refusal {
        ShaderError::NoSources {
            directory: searched,
        } => assert_eq!(
            searched,
            directory.path(),
            "a validation that reported success over an empty set would pass whatever the \
             shaders did, so the empty set fails and the failure names where it looked"
        ),
        other => return Err(format!("expected an empty-directory refusal, got {other:?}").into()),
    }

    assert!(
        reported.contains(&directory.path().display().to_string()),
        "the message must name the searched directory, since the usual cause is that the \
         build looked in the wrong place; this one read `{reported}`"
    );
    Ok(())
}

#[test]
fn an_entry_point_binding_a_fifth_storage_buffer_is_refused_and_a_fourth_is_not() -> TestResult {
    let over = directory_holding(&[(OVER_BUDGET_NAME, OVER_BUDGET_SOURCE)])?;
    let within = directory_holding(&[(WITHIN_BUDGET_NAME, WITHIN_BUDGET_SOURCE)])?;

    match refusal_over(over.path())? {
        ShaderError::StorageBudget {
            file,
            entry_point,
            found,
            capacity,
        } => assert_eq!(
            (file.as_str(), entry_point.as_str(), found, capacity),
            (
                OVER_BUDGET_NAME,
                BUDGET_ENTRY_POINT,
                STORAGE_BUDGET + 1,
                STORAGE_BUDGET
            ),
            "the fifth storage binding is the weakest declared adapter dropping out of the \
             supported set, so the build fails naming the entry point and both figures"
        ),
        other => return Err(format!("expected a storage-budget refusal, got {other:?}").into()),
    }

    assert_eq!(
        validate_shader_directory(within.path())?,
        vec![WITHIN_BUDGET_NAME.to_owned()],
        "four storage buffers is the budget, not one below it; a validator that refused this \
         one would satisfy the refusal above while forbidding the design's own cull shader"
    );
    Ok(())
}

#[test]
fn the_cull_shaders_winding_literal_is_checked_against_the_pattern_the_geometry_emits() -> TestResult
{
    assert_eq!(
        QUAD_INDEX_PATTERN,
        mc_render::geometry::QUAD_INDEX_PATTERN,
        "the build script cannot depend on the crate it builds, so the validator holds its \
         own copy of the winding pattern — and an agreement check against a private copy \
         agrees with itself unless that copy is tied back to the constant the geometry \
         builder actually emits"
    );

    let directory = directory_holding(&[(CULL_NAME, DISAGREEING_CULL_SOURCE)])?;
    match refusal_over(directory.path())? {
        ShaderError::IndexPatternMismatch {
            file,
            found,
            expected,
        } => assert_eq!(
            (file.as_str(), found, expected),
            (
                CULL_NAME,
                DISAGREEING_PATTERN.to_vec(),
                QUAD_INDEX_PATTERN.to_vec()
            ),
            "a cull shader winding its triangles differently from the geometry builder draws \
             a hole where a quad should be, and no unit test on either side can see it"
        ),
        other => return Err(format!("expected a winding-pattern refusal, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn the_terrain_shaders_plane_axis_table_is_checked_against_the_one_corners_are_placed_by()
-> TestResult {
    assert_eq!(
        PLANE_AXES,
        mc_render::geometry::PLANE_AXES,
        "the build script cannot depend on the crate it builds, so the validator holds its \
         own copy of the plane-axis table — and an agreement check against a private copy \
         agrees with itself unless that copy is tied back to the constant the geometry \
         builder actually places corners by. One row per facing in `Facing`'s declaration \
         order, so the shader indexes it with the discriminant it was handed and derives \
         nothing"
    );

    let directory = directory_holding(&[(TERRAIN_NAME, DISAGREEING_TERRAIN_SOURCE)])?;
    let reported = refusal_over(directory.path())?.to_string();
    let expected = flattened(PLANE_AXES);
    assert!(
        reported.contains(TERRAIN_NAME)
            && reported.contains(&format!("{DISAGREEING_PLANE_AXES:?}"))
            && reported.contains(&format!("{expected:?}")),
        "a terrain shader whose plane axes disagree with the geometry builder's runs a face's \
         texture across it instead of along it — the face's mean colour is unchanged, so no \
         derived probe reports it and a golden shot from that renderer records it as ground \
         truth. The refusal has to name the file and both tables; this one read `{reported}`"
    );
    Ok(())
}

/// The only device-free witness to the packed vertex's opacity field.
///
/// **Measured, and it is why this reading is written against packing's own
/// output rather than against a constant.** Move `vertex.rs`'s `OPACITY_SHIFT`
/// and leave `build/validate.rs` and `terrain.wgsl` alone — the direction real
/// drift travels, because nobody edits the validator's copy by accident — and
/// the build stays **green**: the table and the shader go on agreeing with each
/// other about a number neither of them reads from the type. Seven tests redden.
/// Six are rendered frames that need a device. This is the seventh.
///
/// So with `MYCRAFT_ALLOW_NO_GPU` set, deleting this test does not lose a
/// duplicate reading of the bit layout. It leaves the layout observed by nothing
/// at all, and a vertex decoded a bit out draws the whole world at a plausible
/// wrong texture, degree or section — which no mean colour and no golden
/// reports, because a golden is a photograph of whatever shipped.
/// `build/validate.rs`'s header carries the other half of this note, so neither
/// side can be removed in ignorance of the other.
#[test]
fn the_validators_vertex_layout_is_checked_against_the_bits_packing_actually_writes() -> TestResult
{
    // Against the packer's own output rather than against `vertex.rs`'s private
    // constants, which is the stronger tie: it is the bits a vertex buffer
    // carries that a shader decodes, and a constant renamed or left behind would
    // still agree with a copy of itself.
    let written = [
        ("const LAYER_SHIFT", packed_with(|vertex| vertex.layer = 1)?),
        (
            "const SECTION_SHIFT",
            packed_with(|vertex| vertex.section = 1)?,
        ),
        (
            "const OPACITY_SHIFT",
            packed_with(|vertex| vertex.opacity = one_step_of_a_degree())?,
        ),
    ];
    let declared: Vec<(&str, u64)> = written
        .iter()
        .map(|(name, _)| (*name, 1u64 << shift_declared_for(name)))
        .collect();

    assert_eq!(
        written.to_vec(),
        declared,
        "the build script cannot depend on the crate it builds, so the validator holds its own          copy of the packed vertex's bit layout — and an agreement check against a private copy          agrees with itself unless that copy is tied back to what packing emits. Each field is          set to its own lowest step with every other field zero, so the word that comes out is          one bit and that bit names the shift"
    );
    Ok(())
}

#[test]
fn the_validators_section_record_is_checked_against_the_stride_a_scene_writes() -> TestResult {
    // The list's own length is the record's stride, because every scalar in it
    // is four bytes wide. A field added to the shaders' struct without the Rust
    // side growing allocates a section table short by a record per section, and
    // neither side disagrees with itself.
    let one = SceneGeometry::assemble(vec![one_section(A_FIRST_SECTION)?])?;
    let two = SceneGeometry::assemble(vec![
        one_section(A_FIRST_SECTION)?,
        one_section(A_SECOND_SECTION)?,
    ])?;

    let stride = SECTION_RECORD.len() * BYTES_PER_SCALAR;
    assert_eq!(
        (one.section_bytes().len(), two.section_bytes().len()),
        (stride, stride + stride),
        "the validator's field list is the shaders' `Section` struct written a second time, and          its length times four is the stride the section buffer is allocated at. Compared against          the bytes a scene actually writes rather than against `SECTION_RECORD_BYTES`, because two          constants agreeing with each other is the shape this file's own header warns about. Both          a one-section scene and a two-section one are read, so a table off by a field is a          different failure from a writer that emitted one record and stopped"
    );
    Ok(())
}

/// The smallest degree that sets a bit at all.
///
/// **Not `Opacity::CLEAR`**, which is the smallest *degree* and encodes to the
/// byte zero — a vertex packed at it leaves the field empty and the word comes
/// out as nothing, which is a reading that would pass for a field packed
/// anywhere. It is the smallest byte instead, one step above that.
fn one_step_of_a_degree() -> Opacity {
    Opacity::from_quantised(1)
}

/// How wide every scalar of the section record is.
const BYTES_PER_SCALAR: usize = 4;

/// Two section origins, so the stride is read from the gap between two records
/// rather than from the length of one.
const A_FIRST_SECTION: [i32; 3] = [0, 0, 0];
const A_SECOND_SECTION: [i32; 3] = [16, 0, 0];

/// The word a vertex packs to when `set` has moved one field off zero.
///
/// # Errors
///
/// Returns the packing refusal, which none of these values can provoke.
fn packed_with(set: impl FnOnce(&mut Vertex)) -> Result<u64, Box<dyn Error>> {
    let mut vertex = Vertex {
        local: [0, 0, 0],
        facing: Facing::NegX,
        layer: 0,
        section: 0,
        opacity: Opacity::CLEAR,
    };
    set(&mut vertex);
    Ok(u64::from_le_bytes(
        PackedVertex::pack(&vertex)?.to_le_bytes(),
    ))
}

/// The shift the validator's own table declares for `name`.
fn shift_declared_for(name: &str) -> u32 {
    VERTEX_LAYOUT
        .iter()
        .find(|(declared, _)| *declared == name)
        .map_or(u32::MAX, |(_, shift)| *shift)
}

/// One section at `origin` holding a single upward face.
///
/// # Errors
///
/// Returns the parse or packing refusal, neither of which this fixture can
/// provoke.
fn one_section(origin: [i32; 3]) -> Result<mc_render::geometry::SectionGeometry, Box<dyn Error>> {
    let block = mc_core::id::BlockName::parse(A_BLOCK_THE_STRIDE_DOES_NOT_DEPEND_ON)?;
    let key = mc_core::id::TextureKey::parse(A_BLOCK_THE_STRIDE_DOES_NOT_DEPEND_ON)?;
    let resolution = mc_render::texture::TextureResolution::stating(
        [(
            block.clone(),
            mc_core::content::FaceTextures::uniform(key.clone()),
            Opacity::OPAQUE,
        )],
        mc_render::texture::TextureLayers::resolve(&std::collections::BTreeSet::from([key])),
    );
    let quad = mc_world::mesh::Quad {
        facing: Facing::PosY,
        plane: 0,
        origin: mc_world::mesh::PlanePos {
            primary: 0,
            secondary: 0,
        },
        extent: mc_world::mesh::PlaneExtent {
            primary: 1,
            secondary: 1,
        },
        block,
    };
    Ok(build_section_geometry(
        &[quad],
        SectionOrigin::new(origin),
        &resolution,
    )?)
}

/// A block whose only job is to give each section a quad; the record's stride is
/// a property of the table's shape and not of what a section holds.
const A_BLOCK_THE_STRIDE_DOES_NOT_DEPEND_ON: &str = "example:filler";
