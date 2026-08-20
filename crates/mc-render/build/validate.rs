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
///
/// **This is the geometry's table and not an image basis.** It says where a
/// quad's two extents go, which is a different question from where an image's
/// own left-to-right and top-to-bottom run — and reusing it for the second was
/// the defect that drew five of six faces wrong. That question is
/// [`IMAGE_SWAPS`] and [`IMAGE_SIGNS`].
pub const PLANE_AXES: [[u32; 2]; 6] = [[1, 2], [1, 2], [0, 2], [0, 2], [0, 1], [0, 1]];

/// Whether a face's image runs its horizontal along the **secondary** of
/// [`PLANE_AXES`]' pair rather than the primary, `1` for exchanged.
///
/// The build script cannot depend on the crate it builds, so this is its own
/// answer to `mc_render::geometry::IMAGE_SWAPS`. **Derived rather than
/// tabulated**, for the reason the geometry builder's own comment gives: a
/// six-row table of conventions cannot be checked by reading it, and this
/// project shipped one whose three hand-written copies agreed and were wrong.
pub const IMAGE_SWAPS: [u32; 6] = [
    image_swap([-1, 0, 0], [1, 2]),
    image_swap([1, 0, 0], [1, 2]),
    image_swap([0, -1, 0], [0, 2]),
    image_swap([0, 1, 0], [0, 2]),
    image_swap([0, 0, -1], [0, 1]),
    image_swap([0, 0, 1], [0, 1]),
];

/// Whether each of an image's two coordinates runs against its axis rather than
/// along it, horizontal first, `1` for negated. Same row order.
pub const IMAGE_SIGNS: [[u32; 2]; 6] = [
    image_sign([-1, 0, 0]),
    image_sign([1, 0, 0]),
    image_sign([0, -1, 0]),
    image_sign([0, 1, 0]),
    image_sign([0, 0, -1]),
    image_sign([0, 0, 1]),
];

/// The world directions a face's image runs its right edge and its top edge
/// toward, for a viewer standing outside it.
///
/// A viewer outside a face looks along its inward direction with the world's up
/// as their up, and the image's right edge is then forward crossed with up. The
/// two horizontal faces have no world up in them, so theirs is chosen to match
/// what `voxforge` bakes: the top image's top edge runs toward `-z`, the bottom
/// image's toward `+z`.
///
/// The six outward normals are written at the call sites above and are the only
/// hand-written input here. A normal says which way a face points and nothing
/// about how an image sits on it, so there is no convention in one to get wrong.
const fn image_basis(normal: [i32; 3]) -> ([i32; 3], [i32; 3]) {
    let forward = [-normal[0], -normal[1], -normal[2]];
    let up = if normal[1] == 0 {
        [0, 1, 0]
    } else {
        [0, 0, -normal[1]]
    };
    (cross(forward, up), up)
}

/// Whether the face with this `normal` and this plane `pair` runs its image's
/// horizontal along the pair's secondary.
const fn image_swap(normal: [i32; 3], pair: [u32; 2]) -> u32 {
    let (right, _) = image_basis(normal);
    let (horizontal, _) = axis_of(right);
    let [_, secondary] = pair;
    (horizontal == secondary) as u32
}

/// Whether each of this face's image coordinates is negated.
const fn image_sign(normal: [i32; 3]) -> [u32; 2] {
    let (right, up) = image_basis(normal);
    let (_, horizontal_is_negative) = axis_of(right);
    let (_, up_is_negative) = axis_of(up);
    // An image's rows run downward, so its vertical coordinate always runs
    // against the direction its top edge points.
    [horizontal_is_negative as u32, !up_is_negative as u32]
}

/// The cross product of two unit axis directions.
const fn cross(one: [i32; 3], other: [i32; 3]) -> [i32; 3] {
    [
        one[1] * other[2] - one[2] * other[1],
        one[2] * other[0] - one[0] * other[2],
        one[0] * other[1] - one[1] * other[0],
    ]
}

/// The axis index a unit `direction` lies along, and whether it points the
/// negative way down it.
const fn axis_of(direction: [i32; 3]) -> (u32, bool) {
    if direction[0] != 0 {
        (0, direction[0] < 0)
    } else if direction[1] != 0 {
        (1, direction[1] < 0)
    } else {
        (2, direction[2] < 0)
    }
}

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
