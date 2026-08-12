//! The WGSL validation this crate performs when it is built.
//!
//! A shader that does not compile is a black window, and a shader that compiles
//! only on the machine it was written on is a black window somewhere else. Both
//! are found here, at build time, rather than at the first draw — which is the
//! whole reason a build script exists in this crate at all.
//!
//! # One file, two includers
//!
//! `build.rs` includes this file with `#[path]`, and so does
//! `tests/shader_validation.rs`. There is therefore one validator and not a
//! validator plus a test double that agrees with it: the tests exercise the
//! exact code the build runs. Nothing here may reach into the crate being built,
//! because a build script cannot depend on its own package — which is why the
//! winding pattern below is a second copy rather than an import, and why the
//! test that includes this file also asserts the two copies are equal.
//!
//! # Three checks nothing else can make
//!
//! Beyond "does it compile", the validator enforces three facts the design rests
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

/// The six indices four corners are drawn as.
///
/// The build script's own copy of `mc_render::geometry::QUAD_INDEX_PATTERN`.
/// `tests/shader_validation.rs` includes this file and asserts the two are
/// equal, which is the only thing making the shader check below mean anything —
/// an agreement test against a private copy agrees with itself.
pub const QUAD_INDEX_PATTERN: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// Which two components of a corner's local position a face's plane coordinates
/// are written into, one row per facing.
///
/// The build script's own copy of `mc_render::geometry::PLANE_AXES`, held for
/// the same reason and closed by the same test. A shader whose copy has drifted
/// runs a texture *across* a face instead of along it: the face's mean colour is
/// unchanged, so no probe over a captured frame can see it, and a golden minted
/// from that renderer records the drift as ground truth.
pub const PLANE_AXES: [[u32; 2]; 6] = [[1, 2], [1, 2], [0, 2], [0, 2], [0, 1], [0, 1]];

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
/// has.
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
    if file == CULL_SHADER {
        check_index_pattern(file, source)?;
    }
    if file == TERRAIN_SHADER {
        check_plane_axes(file, source)?;
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
