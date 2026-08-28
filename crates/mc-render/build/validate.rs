//! The WGSL validation this crate performs when it is built.
//!
//! A shader that does not compile is a black window, and a shader that compiles
//! only on the machine it was written on is a black window somewhere else. Both
//! are found here, at build time, rather than at the first draw — which is the
//! whole reason a build script exists in this crate at all.
//!
//! # One validator, two includers
//!
//! `build.rs` includes this file with `#[path]`, and so does
//! `tests/shader_validation.rs`. There is therefore one validator and not a
//! validator plus a test double that agrees with it: the tests exercise the
//! exact code the build runs. Nothing here may reach into the crate being built,
//! because a build script cannot depend on its own package — which is why every
//! value checked below is a second copy rather than an import, and why the test
//! that includes this file also asserts the two copies are equal.
//!
//! The copies themselves live in `validate_tables`, reached by the same
//! `#[path]` mechanism so that both includers still include exactly one thing.
//! **What that split buys is that adding a value to compare and adding a
//! comparison are two different edits in two different files**, which is the
//! seam this file's own history argues for: five of six faces drew wrong while
//! three hand-written copies of one table agreed with each other exactly.
//!
//! # Five checks nothing else can make
//!
//! Beyond "does it compile", the validator enforces five facts the design rests
//! on and that no unit test on either side of the CPU/GPU line can see:
//!
//! - **The storage-binding budget.** The weakest adapter in the declared
//!   hardware range offers four storage buffers per shader stage. A fifth is
//!   that adapter dropping out of the supported set, so it fails the build.
//! - **The winding literal.** The cull shader carries its own copy of the six
//!   indices the geometry builder emits, because reading the CPU's index buffer
//!   on the GPU would need exactly that fifth binding. A quad wound differently
//!   on the two sides draws a hole, and no test on either side can see it.
//! - **The plane-axis table.** The terrain shader picks the two components a
//!   face's texture runs along, and the geometry builder writes that face's
//!   corners under the same convention. Exchanged on one side only, the texture
//!   runs *across* the face rather than along it — which leaves the face's mean
//!   colour untouched, so no probe over a captured frame reports it and a golden
//!   minted from that renderer records it as ground truth.
//! - **The packed vertex's bit layout.** A field read a bit out of place decodes
//!   every corner in the world to a plausible wrong coordinate, texture, section
//!   or degree — each of which leaves a frame that looks like a frame.
//! - **The section record's field list.** Both shaders declare the struct the
//!   section table is read through and neither can check it for itself. A field
//!   inserted, removed or exchanged with its neighbour compiles perfectly and
//!   reads every later field out of the wrong four bytes.
//!
//! # What these checks cannot see, measured
//!
//! **Every check here compares a shader against `validate_tables`' copy, and
//! nothing here compares either against the type it stands for.** Measured, not
//! cautioned: moving `vertex.rs`'s own `OPACITY_SHIFT` by one — this file and
//! both shaders untouched — leaves the build **green**, because the two copies go
//! on agreeing with each other about a number neither reads.
//!
//! **Seven tests redden, and six of them need a device.** The seventh is
//! `tests/shader_validation.rs`'s
//! `the_validators_vertex_layout_is_checked_against_the_bits_packing_actually_writes`,
//! and it is **the only thing standing here**: the sole device-free witness that
//! the packed layout is the one `Vertex` writes. Before it was written there was
//! none. Delete it and this file goes on passing while the packing walks away.
//!
//! So the division of labour is: **this file catches the two shaders drifting
//! from each other; the agreement tests catch either drifting from the code**, by
//! packing through `PackedVertex` and `SceneGeometry` rather than against a third
//! constant. A check added here without its agreement test is a copy agreeing
//! with itself.
//!
//! Validation runs at `Capabilities::empty()`, the downlevel profile, rather
//! than at naga's defaults: a shader using a capability the declared hardware
//! range does not offer must fail on the developer's machine, not on the weakest
//! supported GPU's first draw.

use std::ffi::OsStr;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

use naga::SourceLocation;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use thiserror::Error;

/// Every value the shaders are checked against, each a second copy of one
/// `mc-render` owns.
///
/// Re-exported rather than reached through the module, so the checks below and
/// the agreement test in `tests/shader_validation.rs` name them by one path.
#[path = "validate_tables.rs"]
mod validate_tables;

pub use validate_tables::{
    IMAGE_SIGNS, IMAGE_SWAPS, PLANE_AXES, QUAD_INDEX_PATTERN, SECTION_RECORD, VERTEX_LAYOUT,
};

/// How many storage buffers one shader stage may bind.
const STORAGE_BUDGET: usize = 4;

/// The extension a shader source carries.
const SHADER_EXTENSION: &str = "wgsl";

/// The one shader whose winding literal is checked.
const CULL_SHADER: &str = "cull.wgsl";

/// The one shader whose plane-axis table is checked.
const TERRAIN_SHADER: &str = "terrain.wgsl";

/// How the winding literal's declaration begins.
///
/// Matched as text rather than evaluated as a constant expression: the value is
/// a literal by construction, and walking naga's constant arena to reach it
/// would be a second, larger thing to get wrong.
const INDEX_PATTERN_DECLARATION: &str = "const QUAD_INDEX_PATTERN";

/// How the plane-axis table's declaration begins.
const PLANE_AXES_DECLARATION: &str = "const PLANE_AXES";

/// How the image-swap table's declaration begins.
const IMAGE_SWAPS_DECLARATION: &str = "const IMAGE_SWAPS";

/// How the image-sign table's declaration begins.
const IMAGE_SIGNS_DECLARATION: &str = "const IMAGE_SIGNS";

/// How the section record's declaration begins, in both shaders.
const SECTION_RECORD_DECLARATION: &str = "struct Section {";

/// Why the shipped shaders are not acceptable.
#[derive(Debug, Error)]
pub enum ShaderError {
    #[error(
        "no `.wgsl` source in {}; a validation that reported success over an empty set \
         would pass whatever the shaders did",
        directory.display()
    )]
    NoSources { directory: PathBuf },
    #[error("{file} could not be read: {message}")]
    Unreadable { file: String, message: String },
    #[error("{file}:{line}:{column}: {message}")]
    Invalid {
        file: String,
        line: u32,
        column: u32,
        message: String,
    },
    #[error(
        "{file}: entry point `{entry_point}` uses {found} storage buffers, over the {capacity} \
         the weakest supported adapter offers per stage"
    )]
    StorageBudget {
        file: String,
        entry_point: String,
        found: usize,
        capacity: usize,
    },
    #[error(
        "{file}: the winding {found:?} disagrees with the geometry builder's {expected:?}; \
         a quad wound differently on the two sides draws a hole"
    )]
    IndexPatternMismatch {
        file: String,
        found: Vec<u32>,
        expected: Vec<u32>,
    },
    #[error(
        "{file}: the plane axes {found:?} disagree with the geometry builder's {expected:?}; \
         a face whose two plane axes are exchanged runs its texture across the face instead \
         of along it, which leaves its mean colour unchanged and no probe able to see it"
    )]
    PlaneAxesMismatch {
        file: String,
        found: Vec<u32>,
        expected: Vec<u32>,
    },
    #[error(
        "{file}: the image basis {found:?} disagrees with the geometry builder's {expected:?}; \
         a face whose image is exchanged or runs the wrong way down an axis draws its texture \
         turned or laterally reversed, which leaves every colour in the face unchanged and no \
         reading over means, histograms or set membership able to see it"
    )]
    ImageBasisMismatch {
        file: String,
        found: Vec<u32>,
        expected: Vec<u32>,
    },
    #[error(
        "{file}: `{field}` is declared as {found:?} where the packed vertex puts it at \
         {expected}; a field read a bit out of place decodes every corner in the world to a \
         plausible wrong coordinate, texture, section or degree, and every one of those leaves \
         a frame that looks like a frame"
    )]
    VertexLayoutMismatch {
        file: String,
        field: String,
        found: Option<u32>,
        expected: u32,
    },
    #[error(
        "{file}: the section record is declared as {found:?} where the scene writes \
         {expected:?}; a record whose fields have slid by one reads a coordinate as a quad \
         count, and one short of a field reads every section's box out of the next section's \
         origin"
    )]
    SectionRecordMismatch {
        file: String,
        found: Vec<String>,
        expected: Vec<String>,
    },
}

/// Validates every `.wgsl` source in `directory`, returning their file names in
/// ascending order.
///
/// # Errors
///
/// Returns [`ShaderError::NoSources`] when the directory holds no shader,
/// [`ShaderError::Unreadable`] when one cannot be read, [`ShaderError::Invalid`]
/// naming the first error's location, [`ShaderError::StorageBudget`] when an
/// entry point uses too many storage buffers,
/// [`ShaderError::IndexPatternMismatch`] when the cull shader's winding literal
/// has drifted from the geometry builder's, and
/// [`ShaderError::PlaneAxesMismatch`] when the terrain shader's plane-axis table
/// has, and [`ShaderError::ImageBasisMismatch`] when either of its image-basis
/// tables has.
pub fn validate_shader_directory(directory: &Path) -> Result<Vec<String>, ShaderError> {
    let sources = read_sources(directory)?;
    if sources.is_empty() {
        return Err(ShaderError::NoSources {
            directory: directory.to_path_buf(),
        });
    }

    let mut validated = Vec::with_capacity(sources.len());
    for (file, source) in &sources {
        validate_source(file, source)?;
        validated.push(file.clone());
    }
    Ok(validated)
}

/// Every shader in `directory`, as `(file name, source)` sorted by name.
fn read_sources(directory: &Path) -> Result<Vec<(String, String)>, ShaderError> {
    let entries = fs::read_dir(directory).map_err(|error| ShaderError::Unreadable {
        file: directory.display().to_string(),
        message: error.to_string(),
    })?;

    let mut sources = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| ShaderError::Unreadable {
                file: directory.display().to_string(),
                message: error.to_string(),
            })?
            .path();
        if path.extension() != Some(OsStr::new(SHADER_EXTENSION)) {
            continue;
        }
        let file = file_name(&path)?;
        let source = fs::read_to_string(&path).map_err(|error| ShaderError::Unreadable {
            file: file.clone(),
            message: error.to_string(),
        })?;
        sources.push((file, source));
    }
    sources.sort();
    Ok(sources)
}

/// `path`'s own name, as text.
fn file_name(path: &Path) -> Result<String, ShaderError> {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(str::to_owned)
        .ok_or_else(|| ShaderError::Unreadable {
            file: path.display().to_string(),
            message: "the file name is not valid text".to_owned(),
        })
}

/// One shader: it parses, it validates at the downlevel profile, it stays inside
/// the storage budget, and — if it is the cull shader — it winds quads the way
/// the geometry builder does.
fn validate_source(file: &str, source: &str) -> Result<(), ShaderError> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|error| invalid(file, error.location(source), &error))?;
    let analysis = Validator::new(ValidationFlags::all(), Capabilities::empty())
        .validate(&module)
        .map_err(|error| invalid(file, error.location(source), &error))?;

    check_storage_budget(file, &module, &analysis)?;
    // The tables a shader *reads* come before the layouts it merely *declares*,
    // so a shader whose winding or whose plane pair has drifted is reported as
    // that rather than as whichever fault is noticed first. The two below are
    // the ones a stage can hold without using — the terrain stage declares the
    // whole section record and reads only the origin out of it — so they are
    // the least specific thing to report and go last.
    if file == CULL_SHADER {
        check_index_pattern(file, source)?;
    }
    if file == TERRAIN_SHADER {
        // The plane pair before the image basis, so a shader whose geometry
        // table has drifted is reported as that rather than as whichever of the
        // three faults is noticed first.
        check_plane_axes(file, source)?;
        check_image_basis(file, source, IMAGE_SWAPS_DECLARATION, IMAGE_SWAPS.to_vec())?;
        let signs: Vec<u32> = IMAGE_SIGNS.into_iter().flatten().collect();
        check_image_basis(file, source, IMAGE_SIGNS_DECLARATION, signs)?;
        check_vertex_layout(file, source)?;
    }
    // Both shaders declare the section record and read the same buffer through
    // it, so both are checked against the one layout the scene writes.
    if file == CULL_SHADER || file == TERRAIN_SHADER {
        check_section_record(file, source)?;
    }
    Ok(())
}

/// A parse or validation failure, located.
fn invalid(file: &str, location: Option<SourceLocation>, error: &dyn Display) -> ShaderError {
    let (line, column) = location.map_or((0, 0), |at| (at.line_number, at.line_position));
    ShaderError::Invalid {
        file: file.to_owned(),
        line,
        column,
        message: error.to_string(),
    }
}

/// How many storage buffers each entry point reaches, against the budget.
///
/// Counted per entry point rather than per module, because the limit the
/// hardware states is per shader stage: two entry points binding four each is
/// within it, and one binding five is not.
fn check_storage_budget(
    file: &str,
    module: &naga::Module,
    analysis: &naga::valid::ModuleInfo,
) -> Result<(), ShaderError> {
    for (index, entry) in module.entry_points.iter().enumerate() {
        let uses = analysis.get_entry_point(index);
        let found = module
            .global_variables
            .iter()
            .filter(|(handle, global)| {
                matches!(global.space, naga::AddressSpace::Storage { .. })
                    && !uses[*handle].is_empty()
            })
            .count();
        if found > STORAGE_BUDGET {
            return Err(ShaderError::StorageBudget {
                file: file.to_owned(),
                entry_point: entry.name.clone(),
                found,
                capacity: STORAGE_BUDGET,
            });
        }
    }
    Ok(())
}

/// The cull shader's winding literal against the geometry builder's.
fn check_index_pattern(file: &str, source: &str) -> Result<(), ShaderError> {
    let found = declared_values(source, INDEX_PATTERN_DECLARATION);
    if found == QUAD_INDEX_PATTERN {
        return Ok(());
    }
    Err(ShaderError::IndexPatternMismatch {
        file: file.to_owned(),
        found,
        expected: QUAD_INDEX_PATTERN.to_vec(),
    })
}

/// The terrain shader's plane-axis table against the geometry builder's.
///
/// The shader's copy is one flat list, because a `vec2` constructor per row
/// would put a bracket inside the literal that the reader below would have to
/// understand. The rows are compared flattened for the same reason: what the
/// build has to answer is whether the twelve numbers agree, and reporting them
/// as the shader wrote them is what lets a developer diff the two by eye.
fn check_plane_axes(file: &str, source: &str) -> Result<(), ShaderError> {
    let found = declared_values(source, PLANE_AXES_DECLARATION);
    let expected: Vec<u32> = PLANE_AXES.into_iter().flatten().collect();
    if found == expected {
        return Ok(());
    }
    Err(ShaderError::PlaneAxesMismatch {
        file: file.to_owned(),
        found,
        expected,
    })
}

/// One of the terrain shader's image-basis tables against the geometry
/// builder's, named by its `declaration`.
///
/// Flat and flattened for the reasons [`check_plane_axes`] gives. **And the
/// reason these checks exist rather than trust is worth a sentence: none of them
/// is evidence that the values are right.** They close a drift between two
/// copies, and this project shipped a table on which all three copies agreed and
/// all three were wrong. What can say the values are right is a reading of a
/// drawn face — FR-8.1-S7 for where its bands sit, FR-8.1-S8 for which way it
/// runs.
fn check_image_basis(
    file: &str,
    source: &str,
    declaration: &str,
    expected: Vec<u32>,
) -> Result<(), ShaderError> {
    let found = declared_values(source, declaration);
    if found == expected {
        return Ok(());
    }
    Err(ShaderError::ImageBasisMismatch {
        file: file.to_owned(),
        found,
        expected,
    })
}

/// Fails unless every field of the packed vertex sits where the geometry builder
/// puts it.
///
/// Each shift and width is a named scalar constant the decode itself reads, so
/// this compares the numbers the shader actually uses rather than a comment
/// beside them — which is what the three-hand-written-copies defect turned on.
fn check_vertex_layout(file: &str, source: &str) -> Result<(), ShaderError> {
    for (declaration, expected) in VERTEX_LAYOUT {
        let found = declared_scalar(source, declaration);
        if found != Some(expected) {
            return Err(ShaderError::VertexLayoutMismatch {
                file: file.to_owned(),
                field: declaration.to_owned(),
                found,
                expected,
            });
        }
    }
    Ok(())
}

/// Fails unless the shader's section record is the one the scene writes, field
/// for field and type for type.
///
/// Names as well as types, because the two failures differ: a field renamed is a
/// shader that no longer compiles, but a field **inserted, removed or exchanged
/// with its neighbour** compiles perfectly and reads every later field out of the
/// wrong four bytes. The order is what carries the offsets, so the comparison is
/// over the whole list in order rather than over its membership.
fn check_section_record(file: &str, source: &str) -> Result<(), ShaderError> {
    let found = declared_fields(source, SECTION_RECORD_DECLARATION);
    let expected: Vec<String> = SECTION_RECORD
        .iter()
        .map(|(name, scalar)| format!("{name}: {scalar}"))
        .collect();
    if found == expected {
        return Ok(());
    }
    Err(ShaderError::SectionRecordMismatch {
        file: file.to_owned(),
        found,
        expected,
    })
}

/// The value of a `const NAME: u32 = <literal>u;` declaration, or `None` where
/// the shader has no such declaration or it has outgrown that shape.
///
/// Blunt in the same way [`declared_values`] is, and for the same reason: a
/// declaration this cannot read reports as absent, which is a refusal rather
/// than a pass.
fn declared_scalar(source: &str, declaration: &str) -> Option<u32> {
    let (_, after_name) = source.split_once(declaration)?;
    let (_, after_equals) = after_name.split_once('=')?;
    let (value, _) = after_equals.split_once(';')?;
    value.trim().trim_end_matches('u').parse().ok()
}

/// The `name: type` of every field of a struct declaration, in order.
///
/// Comment lines inside the struct are skipped, so a field may be explained
/// where it is declared without the explanation reading as a field.
fn declared_fields(source: &str, declaration: &str) -> Vec<String> {
    let Some((_, body)) = source.split_once(declaration) else {
        return Vec::new();
    };
    let Some((body, _)) = body.split_once('}') else {
        return Vec::new();
    };
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .map(|line| {
            line.trim_end_matches(',')
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// The values `declaration` names in the source, or nothing when it names none.
///
/// An absent or unreadable declaration returns an empty list rather than a
/// variant of its own: "the shader does not say how it winds a quad" and "the
/// shader winds it differently" are the same defect from the build's point of
/// view, and both are reported by showing what was found.
///
/// The parse is deliberately blunt — the first `(` after the name to the first
/// `)` after that — which is exactly enough for a constructor call over integer
/// literals and nothing else. A declaration that outgrew that shape would read
/// as empty here, which is a refusal rather than a pass.
fn declared_values(source: &str, declaration: &str) -> Vec<u32> {
    let Some((_, after_name)) = source.split_once(declaration) else {
        return Vec::new();
    };
    let Some((_, after_open)) = after_name.split_once('(') else {
        return Vec::new();
    };
    let Some((values, _)) = after_open.split_once(')') else {
        return Vec::new();
    };
    values
        .split(',')
        .filter_map(|value| value.trim().trim_end_matches('u').parse().ok())
        .collect()
}
