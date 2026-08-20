//! The voxel model a texture was baked from, read independently of the baker.
//!
//! # Why a second reader exists
//!
//! `tools/voxforge` already parses `.mcvox` — and nothing under `crates/` may
//! depend on it (`mc-testkit/tests/workspace_layering.rs` asserts that against
//! the resolved dependency graph). That prohibition is convenient here rather
//! than inconvenient: the model is what the *baker* consumes, so a reading that
//! judged the baked image through the baker's own parser and the baker's own
//! face-to-plane mapping would be comparing the tool against itself. This reader
//! is written from `docs/modding/voxel-models.md` and shares no line with it.
//!
//! # The plane a face shows is derived, not tabulated
//!
//! A face texture is a flat axis-aligned view, so each image is one outermost
//! plane of the model. Which voxel lands at which texel follows from one
//! orthonormal triple — `(right, up, normal)`, right-handed — and two documented
//! facts:
//!
//! * `normal` is the face's own outward direction, and `content/base/textures.toml`
//!   records which compass side each face word shows: `front` looks along −z and
//!   so shows the +z side, which is south; `back` shows north; `right` shows east;
//!   `left` shows west.
//! * For the four sides the image runs **up the model**, so `up` is world +y and
//!   `right = up × normal`. For the two plan views there is no `y` in the picture
//!   at all and the documented column axis is `x`, so `right` is +x and
//!   `up = normal × right`.
//!
//! Nothing here is a table of six mappings that could each be wrong on its own:
//! four of the six come out of the same two lines of vector arithmetic, and the
//! two plan views out of the other two. **That matters because a tabulated
//! mapping is exactly the shape of the defect this reading exists to catch** —
//! `mc_render::geometry::PLANE_AXES` is such a table and five of its six rows
//! were wrong.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

/// One `.mcvox` model's filled voxels.
pub struct VoxelModel {
    /// Voxels on each axis. A face set is a cube, so one number covers all three.
    edge: usize,
    /// The material each voxel is of, indexed `x + edge * (y + edge * z)`.
    of: Vec<String>,
}

/// One of the six words a block face is spelled with, and the face it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Face {
    /// The word `textures.toml` spells, so a failure names what the author wrote.
    pub word: &'static str,
    /// The face's outward direction in model coordinates.
    pub normal: [i32; 3],
}

/// Every face word a manifest may name, with the direction each one shows.
///
/// The compass mapping is `content/base/textures.toml`'s own: `front` shows +z
/// (south), `back` −z (north), `right` +x (east), `left` −x (west).
pub const FACES: [Face; 6] = [
    Face {
        word: "front",
        normal: [0, 0, 1],
    },
    Face {
        word: "back",
        normal: [0, 0, -1],
    },
    Face {
        word: "right",
        normal: [1, 0, 0],
    },
    Face {
        word: "left",
        normal: [-1, 0, 0],
    },
    Face {
        word: "top",
        normal: [0, 1, 0],
    },
    Face {
        word: "bottom",
        normal: [0, -1, 0],
    },
];

impl VoxelModel {
    /// The model written at `at`.
    ///
    /// # Errors
    ///
    /// Returns an error for a file that cannot be read, states no `size`, is not
    /// a cube, is sliced on an axis this reader does not know, spells a palette
    /// character no entry declares, or carries a layer of the wrong shape. Each
    /// is a model this reading cannot be taken from rather than a bake that is
    /// wrong, and each says which.
    pub fn read(at: &Path) -> Result<Self, Box<dyn Error>> {
        let written = std::fs::read_to_string(at)
            .map_err(|cause| format!("`{}` could not be read: {cause}", at.display()))?;
        let lines: Vec<&str> = written
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with('#'))
            .collect();
        let edge = cube_edge(&lines, at)?;
        require_sliced_on_y(&lines, at)?;
        let palette = palette_of(&lines, at)?;
        let mut of = vec![String::new(); edge * edge * edge];
        for layer in layers_of(&written, at)? {
            place(&mut of, &layer, (edge, &palette), at)?;
        }
        if let Some(missing) = of.iter().position(String::is_empty) {
            return Err(format!(
                "`{}` leaves voxel {missing} unstated: this reading is about the model's outermost \
                 planes and a hole in one of them has no material to compare a texel against",
                at.display()
            )
            .into());
        }
        Ok(Self { edge, of })
    }

    /// Voxels on each axis.
    #[must_use]
    pub fn edge(&self) -> usize {
        self.edge
    }

    /// The outermost plane `face` shows, row-major with row 0 at the top of the
    /// image and column 0 at the left, as material names.
    ///
    /// See this module's header for where the mapping comes from: it is two lines
    /// of vector arithmetic over a right-handed `(right, up, normal)` triple, not
    /// six tabulated cases.
    #[must_use]
    pub fn plane(&self, face: Face) -> Vec<String> {
        let last = self.edge.saturating_sub(1) as i32;
        let (right, down) = image_basis(face);
        let basis = (face.normal, right, down);
        let side = self.edge as i32;
        (0..side)
            .flat_map(|row| (0..side).map(move |column| (column, row)))
            .map(|walk| {
                let at = [0, 1, 2].map(|axis| coordinate(basis, axis, walk, last));
                self.material_at(at).to_owned()
            })
            .collect()
    }

    /// The material of the voxel at `[x, y, z]`, or the empty string outside the
    /// model.
    fn material_at(&self, [x, y, z]: [i32; 3]) -> &str {
        let (Ok(x), Ok(y), Ok(z)) = (usize::try_from(x), usize::try_from(y), usize::try_from(z))
        else {
            return "";
        };
        self.of
            .get(x + self.edge * (y + self.edge * z))
            .map_or("", String::as_str)
    }
}

/// The image's `right` and `down` directions for `face`, in model coordinates.
///
/// The two vectors [`VoxelModel::plane`] walks, exposed because the *drawn*
/// reading has to walk the same two in world coordinates: "the image's own
/// left-to-right order" means this `right` and nothing else, and the two halves
/// asking that question of the bake and of the frame must not each decide it for
/// themselves. Model and world axes are the same axes here — `content/base/textures.toml`
/// records the compass each face word shows — so one derivation serves both.
#[must_use]
pub fn image_basis(face: Face) -> ([i32; 3], [i32; 3]) {
    let up = up_of(face);
    (cross(up, face.normal), [-up[0], -up[1], -up[2]])
}

/// The face whose outward direction is `normal`, where it is one of the six.
#[must_use]
pub fn face_showing(normal: [i32; 3]) -> Option<Face> {
    FACES.into_iter().find(|face| face.normal == normal)
}

/// The image's up direction for `face`, in model coordinates.
///
/// World up for the four sides, whose images run up the model. For the two plan
/// views world up is not in the picture and the documented column axis is `x`, so
/// up is whatever makes `right` come out as +x — which the right-handed triple
/// decides, not a choice made here.
fn up_of(face: Face) -> [i32; 3] {
    if face.normal[1] == 0 {
        [0, 1, 0]
    } else {
        cross(face.normal, [1, 0, 0])
    }
}

/// The coordinate one axis takes at `walk` on the plane spanned by `basis`.
///
/// Exactly one of the three vectors is non-zero on any given axis, since they are
/// an orthonormal triple of signed axes: the normal pins the plane's own
/// coordinate, and the other two walk it.
fn coordinate(
    basis: ([i32; 3], [i32; 3], [i32; 3]),
    axis: usize,
    walk: (i32, i32),
    last: i32,
) -> i32 {
    let (normal, right, down) = basis;
    let (column, row) = walk;
    let on = |vector: [i32; 3]| vector.get(axis).copied().unwrap_or(0);
    along(on(normal), last)
        .or_else(|| moving(on(right), column, last))
        .or_else(|| moving(on(down), row, last))
        .unwrap_or(0)
}

/// The cross product of two axis-aligned unit vectors.
fn cross(left: [i32; 3], right: [i32; 3]) -> [i32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

/// The fixed coordinate a face's own normal pins on its axis, if it is that axis.
fn along(normal: i32, last: i32) -> Option<i32> {
    match normal {
        1 => Some(last),
        -1 => Some(0),
        _ => None,
    }
}

/// The coordinate a walk of `steps` contributes on its axis, if it is that axis.
fn moving(direction: i32, steps: i32, last: i32) -> Option<i32> {
    match direction {
        1 => Some(steps),
        -1 => Some(last - steps),
        _ => None,
    }
}

/// The one edge length a cube model declares.
fn cube_edge(lines: &[&str], at: &Path) -> Result<usize, Box<dyn Error>> {
    let stated = lines
        .iter()
        .find_map(|line| line.strip_prefix("size"))
        .and_then(|rest| rest.trim_start().strip_prefix('='))
        .ok_or_else(|| format!("`{}` states no `size`", at.display()))?;
    let axes: Vec<usize> = stated
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .filter_map(|number| number.trim().parse().ok())
        .collect();
    match axes.as_slice() {
        [x, y, z] if x == y && y == z && *x > 0 => Ok(*x),
        _ => Err(format!(
            "`{}` states `size` as {axes:?}, and a face set is a block's six faces — so this \
             reading only knows a cube",
            at.display()
        )
        .into()),
    }
}

/// Fails unless the model is sliced on `y`, which is the only slicing this reads.
fn require_sliced_on_y(lines: &[&str], at: &Path) -> Result<(), Box<dyn Error>> {
    let stated = lines
        .iter()
        .find_map(|line| line.strip_prefix("slice"))
        .and_then(|rest| rest.trim_start().strip_prefix('='))
        .map(|rest| rest.trim().trim_matches('"').to_owned())
        .ok_or_else(|| format!("`{}` states no `slice`", at.display()))?;
    if stated == "y" {
        Ok(())
    } else {
        Err(format!(
            "`{}` is sliced on `{stated}`, and this reader only knows `y` — a layer read as a \
             floor plan when it is not one would compare the right image against the wrong plane",
            at.display()
        )
        .into())
    }
}

/// Each palette character and the material it stands for.
fn palette_of(lines: &[&str], at: &Path) -> Result<Palette, Box<dyn Error>> {
    let mut found = BTreeMap::new();
    let mut inside = false;
    for line in lines {
        if line.starts_with('[') {
            inside = *line == "[palette]";
            continue;
        }
        if !inside || line.is_empty() {
            continue;
        }
        let (spelled, material) = line
            .split_once('=')
            .ok_or_else(|| format!("`{}` states `{line}` in its palette", at.display()))?;
        let character = spelled
            .trim()
            .trim_matches('"')
            .chars()
            .next()
            .ok_or_else(|| format!("`{}` names a palette entry with no character", at.display()))?;
        found.insert(character, material.trim().trim_matches('"').to_owned());
    }
    Ok(found)
}

/// One layer of a model: the height it is a floor plan of, and its rows.
type Layer = (usize, Vec<String>);

/// Each palette character and the material it stands for.
type Palette = BTreeMap<char, String>;

/// Every layer `written` states.
///
/// Split on the layer header rather than walked line by line, because the `y` a
/// layer states and the grid it states are two different shapes of thing and a
/// single pass over the lines has to carry both as pending state.
fn layers_of(written: &str, at: &Path) -> Result<Vec<Layer>, Box<dyn Error>> {
    let found: Vec<Layer> = written
        .split("[[layers]]")
        .skip(1)
        .filter_map(layer_in)
        .collect();
    if found.is_empty() {
        return Err(format!("`{}` states no layers", at.display()).into());
    }
    Ok(found)
}

/// The layer one `[[layers]]` block states, where it states a height and a grid.
fn layer_in(block: &str) -> Option<Layer> {
    let stated = |line: &&str| !line.starts_with('#');
    let height = block
        .lines()
        .map(str::trim)
        .filter(stated)
        .find_map(|line| line.strip_prefix('y'))
        .and_then(|rest| rest.trim_start().strip_prefix('='))
        .and_then(|number| number.trim().parse::<usize>().ok())?;
    let rows = block
        .split("\"\"\"")
        .nth(1)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    Some((height, rows))
}

/// Writes one layer's voxels into `of`.
fn place(
    of: &mut [String],
    layer: &Layer,
    model: (usize, &Palette),
    at: &Path,
) -> Result<(), Box<dyn Error>> {
    let (y, grid) = layer;
    let (edge, _) = model;
    if grid.len() != edge {
        return Err(format!(
            "`{}` states {} rows for layer y = {y}, where a cube of edge {edge} wants {edge}",
            at.display(),
            grid.len()
        )
        .into());
    }
    for (z, row) in grid.iter().enumerate() {
        place_row(of, (*y, z, row), model, at)?;
    }
    Ok(())
}

/// Writes one row of one layer into `of`.
fn place_row(
    of: &mut [String],
    cell: (usize, usize, &str),
    model: (usize, &Palette),
    at: &Path,
) -> Result<(), Box<dyn Error>> {
    let (y, z, row) = cell;
    let (edge, palette) = model;
    let spelled: Vec<char> = row.chars().collect();
    if spelled.len() != edge {
        return Err(format!(
            "`{}` states {} characters in row z = {z} of layer y = {y}, where a cube of edge \
             {edge} wants {edge}",
            at.display(),
            spelled.len()
        )
        .into());
    }
    let materials = spelled
        .into_iter()
        .map(|character| material_of(character, palette, at))
        .collect::<Result<Vec<String>, Box<dyn Error>>>()?;
    for (x, material) in materials.into_iter().enumerate() {
        if let Some(cell) = of.get_mut(x + edge * (y + edge * z)) {
            *cell = material;
        }
    }
    Ok(())
}

/// The material `character` stands for.
fn material_of(character: char, palette: &Palette, at: &Path) -> Result<String, Box<dyn Error>> {
    palette.get(&character).cloned().ok_or_else(|| {
        format!(
            "`{}` spells `{character}` in a layer and its palette declares no such entry",
            at.display()
        )
        .into()
    })
}

/// Which face word each texture key is baked from, for the model file `names`.
///
/// Read out of `content/base/textures.toml` as text rather than written down
/// here: a manifest that came to bake a different face under a key would
/// otherwise leave this reading comparing the right image against the wrong
/// plane, and say nothing about it.
///
/// # Errors
///
/// Returns an error when the manifest cannot be read, or when an entry names a
/// face word that is none of the six.
pub fn baked_from(names: &str) -> Result<Vec<(Face, String)>, Box<dyn Error>> {
    let at = super::repository_root()?
        .join("content")
        .join("base")
        .join("textures.toml");
    let written = std::fs::read_to_string(&at)
        .map_err(|cause| format!("`{}` could not be read: {cause}", at.display()))?;
    written
        .split("[[texture]]")
        .skip(1)
        .filter_map(|entry| baking(entry, names))
        .map(|(word, key)| face_named(&word, &key, &at).map(|face| (face, key)))
        .collect()
}

/// The face word and key one manifest entry bakes from a model whose path ends
/// with `names`, where it bakes one at all.
fn baking(entry: &str, names: &str) -> Option<(String, String)> {
    let stated = |wanted: &str| {
        entry
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with('#'))
            .find_map(|line| line.strip_prefix(wanted))
            .and_then(|rest| rest.trim_start().strip_prefix('='))
            .map(|rest| rest.trim().trim_matches('"').to_owned())
    };
    let (key, model, word) = (stated("key")?, stated("model")?, stated("face")?);
    model.ends_with(names).then_some((word, key))
}

/// The face `word` names.
fn face_named(word: &str, key: &str, at: &Path) -> Result<Face, Box<dyn Error>> {
    FACES
        .into_iter()
        .find(|face| face.word == word)
        .ok_or_else(|| {
            format!(
                "`{}` bakes `{key}` from face `{word}`, which is none of the six a block has",
                at.display()
            )
            .into()
        })
}
