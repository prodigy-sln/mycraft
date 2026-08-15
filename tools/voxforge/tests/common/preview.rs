//! Fixtures and verdicts shared by the preview tests.
//!
//! Every verdict here is an **enumerated** answer rather than a boolean or an
//! absence: "every drawn pixel is this paint" and "nothing was drawn at all"
//! must never compare equal, or a renderer that emits an empty image passes
//! every orientation and occlusion scenario in this phase vacuously.
//!
//! The fixture paints are pure single channels on purpose. Shading multiplies a
//! colour in **linear** space by a factor and re-encodes it, and zero is a fixed
//! point of that path while no positive factor takes a non-zero channel to zero.
//! So every shade of `#ff0000` is `(r, 0, 0)` with `r > 0`, and every shade of
//! `#0000ff` is `(0, 0, b)` — which lets these assertions grade orientation and
//! occlusion without pinning the shading factors, which are deliberately still
//! an open question. The one place the factors *are* graded is the shading
//! test, where the spec names the exact bytes.

use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZeroU32;

use voxforge::fault::Origin;
use voxforge::material::{Emissive, Material, MaterialTable, Srgb8};
use voxforge::name::MaterialKey;
use voxforge::render::{Pixel, Preview, View, render, to_png};
use voxforge::volume::Volume;

/// Eight pixels per voxel: the tool's default, and the scale every derived pixel
/// count in these tests is arithmetic over.
///
/// Spelled as a `match` rather than an `unwrap` because `unwrap` is denied
/// workspace-wide, in tests as much as anywhere else.
pub const EIGHT_PER_VOXEL: NonZeroU32 = match NonZeroU32::new(8) {
    Some(scale) => scale,
    None => NonZeroU32::MIN,
};

/// The first eight bytes of every PNG file.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// The name a preview's bytes are attributed to when a refusal has to name one.
pub const OUTPUT_FILE: &str = "preview.png";

/// The one colour channel a fixture material paints with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Paint {
    /// `#ff0000`.
    Red,
    /// `#00ff00`.
    Green,
    /// `#0000ff`.
    Blue,
}

impl Paint {
    /// Every paint a fixture document may spell.
    pub const ALL: [Self; 3] = [Self::Red, Self::Green, Self::Blue];

    /// The character a fixture grid spells this paint with.
    #[must_use]
    pub fn spelling(self) -> char {
        match self {
            Self::Red => 'r',
            Self::Green => 'g',
            Self::Blue => 'b',
        }
    }

    /// The namespaced material this paint stands for.
    #[must_use]
    pub fn material(self) -> &'static str {
        match self {
            Self::Red => "base:ruby",
            Self::Green => "base:jade",
            Self::Blue => "base:lapis",
        }
    }

    /// The colour that material declares.
    #[must_use]
    pub fn colour(self) -> Srgb8 {
        match self {
            Self::Red => rgb(255, 0, 0),
            Self::Green => rgb(0, 255, 0),
            Self::Blue => rgb(0, 0, 255),
        }
    }

    /// Whether `pixel` is a shade of this paint.
    ///
    /// Shade, not colour: the face factors are still an open decision, so what
    /// is asserted is which channel carries the light, never how much of it
    /// survives.
    #[must_use]
    pub fn shows(self, pixel: Pixel) -> bool {
        if pixel.alpha != 255 {
            return false;
        }
        let (lit, dark) = match self {
            Self::Red => (pixel.red, [pixel.green, pixel.blue]),
            Self::Green => (pixel.green, [pixel.red, pixel.blue]),
            Self::Blue => (pixel.blue, [pixel.red, pixel.green]),
        };
        lit > 0 && dark == [0, 0]
    }
}

/// That colour.
#[must_use]
pub fn rgb(red: u8, green: u8, blue: u8) -> Srgb8 {
    Srgb8 { red, green, blue }
}

/// A material table declaring the three fixture paints, none of them emissive.
///
/// # Errors
///
/// Returns an error when a key is not namespaced, which would leave the table
/// short of a material the fixtures spell for a reason nothing to do with the
/// renderer.
pub fn paints() -> Result<MaterialTable, Box<dyn Error>> {
    let declared: Vec<(&str, Srgb8, f32)> = Paint::ALL
        .iter()
        .map(|paint| (paint.material(), paint.colour(), 0.0))
        .collect();
    table_of(&declared)
}

/// A material table declaring `materials` as key, colour and emissive fraction.
///
/// Built in memory rather than from files: what is under test here is the
/// renderer, and reading the same three declarations off disk would only put
/// Phase 2's contract between the fixture and the assertion.
///
/// # Errors
///
/// Returns an error when a key is not namespaced or an emissive fraction is
/// outside `0.0 ..= 1.0`.
pub fn table_of(materials: &[(&str, Srgb8, f32)]) -> Result<MaterialTable, Box<dyn Error>> {
    let mut declared: BTreeMap<MaterialKey, Material> = BTreeMap::new();
    for (key, color, emissive) in materials {
        let emissive = Emissive::new(*emissive)
            .ok_or_else(|| format!("`{emissive}` is not a fraction from 0.0 to 1.0"))?;
        declared.insert(
            MaterialKey::parse(key)?,
            Material {
                color: *color,
                emissive,
            },
        );
    }
    Ok(MaterialTable::new("materials", declared))
}

/// A one-part document of `size`, sliced on `y`, solid in `paint`.
#[must_use]
pub fn solid(size: (u32, u32, u32), paint: Paint) -> String {
    painted(size, &|_, _, _| Some(paint))
}

/// A one-part document of `size`, sliced on `y`, solid, whose lower half on `x`
/// is `low` and whose upper half is `high`.
///
/// Solid because the scenarios it serves are about **occlusion**: a hollow model
/// would let a ray reach the far half and pass a first-hit assertion for the
/// wrong reason. That is a shape constraint no assertion can enforce, so it is
/// held here, in the code that builds the fixture.
#[must_use]
pub fn halved_on_x(size: (u32, u32, u32), low: Paint, high: Paint) -> String {
    let (extent, _, _) = size;
    let middle = extent.div_euclid(2);
    painted(size, &|x, _, _| Some(if x < middle { low } else { high }))
}

/// A one-part document of `size`, sliced on `y`, solid, whose lower half on `y`
/// is `low` and whose upper half is `high`.
#[must_use]
pub fn halved_on_y(size: (u32, u32, u32), low: Paint, high: Paint) -> String {
    let (_, extent, _) = size;
    let middle = extent.div_euclid(2);
    painted(size, &|_, y, _| Some(if y < middle { low } else { high }))
}

/// A one-part document of `size`, sliced on `y`, holding whatever `paint_of`
/// answers for each voxel.
///
/// A `y` slice prints one layer per `y` plane, its rows running `z` ascending
/// and its columns running `x` ascending — the spec's own table, which is
/// Phase 2's graded contract and is relied on here rather than restated.
#[must_use]
pub fn painted(size: (u32, u32, u32), paint_of: &dyn Fn(u32, u32, u32) -> Option<Paint>) -> String {
    let (extent_x, extent_y, extent_z) = size;
    let layers: String = (0..extent_y)
        .map(|y| {
            let art: Vec<String> = (0..extent_z)
                .map(|z| {
                    (0..extent_x)
                        .map(|x| paint_of(x, y, z).map_or('.', Paint::spelling))
                        .collect()
                })
                .collect();
            format!(
                "\n[[layers]]\ny = {y}\ngrid = \"\"\"\n{}\n\"\"\"\n",
                art.join("\n")
            )
        })
        .collect();
    format!(
        r#"schema = 1
name = "base:fixture"
scale = 16
size = [{extent_x}, {extent_y}, {extent_z}]
origin = [0, 0, 0]
slice = "y"

[palette]
"." = "empty"
"r" = "base:ruby"
"g" = "base:jade"
"b" = "base:lapis"
{layers}"#
    )
}

/// What every drawn pixel of a preview is made of.
#[derive(Debug, PartialEq, Eq)]
pub enum MadeOf {
    /// Every drawn pixel is a shade of the paint asked about, and at least one
    /// pixel is drawn.
    OnlyThePaint,
    /// No pixel is drawn at all, so nothing was graded.
    NothingDrawn,
    /// A drawn pixel is a shade of something else.
    Foreign {
        /// Where it sits.
        column: u32,
        /// Which row it sits in.
        row: u32,
        /// What it holds.
        pixel: Pixel,
    },
    /// A pixel is neither drawn nor background. One opaque sample per pixel
    /// leaves no third answer, so this is a contract violation rather than a
    /// near miss.
    PartlyCovered {
        /// Where it sits.
        column: u32,
        /// Which row it sits in.
        row: u32,
        /// How opaque it is.
        alpha: u8,
    },
}

/// Whether every drawn pixel of `preview` is a shade of `paint`.
#[must_use]
pub fn made_of(preview: &Preview, paint: Paint) -> MadeOf {
    let mut drawn = 0_usize;
    for (column, row, pixel) in pixels(preview) {
        if pixel.is_background() {
            continue;
        }
        if pixel.alpha != 255 {
            return MadeOf::PartlyCovered {
                column,
                row,
                alpha: pixel.alpha,
            };
        }
        if !paint.shows(pixel) {
            return MadeOf::Foreign { column, row, pixel };
        }
        drawn += 1;
    }
    if drawn == 0 {
        return MadeOf::NothingDrawn;
    }
    MadeOf::OnlyThePaint
}

/// How much of a preview a voxel landed on.
#[derive(Debug, PartialEq, Eq)]
pub enum Coverage {
    /// This many pixels are fully opaque, and every other pixel of the image is
    /// fully transparent.
    Drawn(usize),
    /// A pixel is neither, which one opaque sample per pixel forbids.
    PartlyCovered {
        /// Where it sits.
        column: u32,
        /// Which row it sits in.
        row: u32,
        /// How opaque it is.
        alpha: u8,
    },
}

/// How many pixels of `preview` a voxel landed on.
#[must_use]
pub fn coverage(preview: &Preview) -> Coverage {
    let mut drawn = 0_usize;
    for (column, row, pixel) in pixels(preview) {
        match pixel.alpha {
            0 => {}
            255 => drawn += 1,
            alpha => return Coverage::PartlyCovered { column, row, alpha },
        }
    }
    Coverage::Drawn(drawn)
}

/// Where two paints sit relative to one another along one axis of the image.
#[derive(Debug, PartialEq, Eq)]
pub enum Placement {
    /// Every pixel of the first paint sits strictly before every pixel of the
    /// second.
    FirstBeforeSecond,
    /// They share a position, or the second comes first. Each span is inclusive.
    NotSeparated {
        /// The first paint's span.
        first: (u32, u32),
        /// The second paint's span.
        second: (u32, u32),
    },
    /// One of the two is not drawn at all, so the question could not be asked.
    Missing(Paint),
}

/// Whether every `first` pixel sits in a lower row than every `second` pixel.
#[must_use]
pub fn row_order(preview: &Preview, first: Paint, second: Paint) -> Placement {
    separation(
        span(preview, first, Axis::Row),
        span(preview, second, Axis::Row),
        first,
        second,
    )
}

/// Whether every `first` pixel sits in a lower column than every `second` pixel.
#[must_use]
pub fn column_order(preview: &Preview, first: Paint, second: Paint) -> Placement {
    separation(
        span(preview, first, Axis::Column),
        span(preview, second, Axis::Column),
        first,
        second,
    )
}

/// Every distinct drawn colour the preview holds, ascending.
///
/// Background pixels are left out: what these tests grade is what a *voxel*
/// produced, and an image's transparent remainder is graded by [`coverage`].
#[must_use]
pub fn drawn_colours(preview: &Preview) -> Vec<Pixel> {
    let mut found: Vec<Pixel> = pixels(preview)
        .filter(|(_, _, pixel)| !pixel.is_background())
        .map(|(_, _, pixel)| pixel)
        .collect();
    found.sort_unstable();
    found.dedup();
    found
}

/// How two encodings compare.
#[derive(Debug, PartialEq, Eq)]
pub enum Encodings {
    /// The same bytes, byte for byte.
    Identical,
    /// The first byte at which they differ.
    DifferAt {
        /// Its offset.
        byte: usize,
        /// What the first encoding holds there.
        left: u8,
        /// What the second holds.
        right: u8,
    },
    /// One is a prefix of the other.
    DifferInLength {
        /// How many bytes the first holds.
        left: usize,
        /// How many the second holds.
        right: usize,
    },
}

/// How `left` and `right` compare.
#[must_use]
pub fn compared(left: &[u8], right: &[u8]) -> Encodings {
    let difference = left
        .iter()
        .zip(right.iter())
        .enumerate()
        .find(|(_, (left, right))| left != right);
    match difference {
        Some((byte, (left, right))) => Encodings::DifferAt {
            byte,
            left: *left,
            right: *right,
        },
        None if left.len() == right.len() => Encodings::Identical,
        None => Encodings::DifferInLength {
            left: left.len(),
            right: right.len(),
        },
    }
}

/// The PNG encoding of `volume` seen from `view`, at eight pixels per voxel.
///
/// # Errors
///
/// Returns an error when the encode fails, and when what comes back is not a
/// PNG at all — without which two encodings that both went wrong the same way
/// would compare equal and a determinism scenario would pass on nothing.
pub fn png_of(
    volume: &Volume,
    materials: &MaterialTable,
    view: View,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let preview = render(volume, materials, view, EIGHT_PER_VOXEL);
    let bytes = to_png(&preview, Origin::new(OUTPUT_FILE))?;
    if !bytes.starts_with(&PNG_SIGNATURE) {
        return Err(format!(
            "the encoding of the {} view is {} byte(s) and does not open with the PNG signature",
            view.as_str(),
            bytes.len()
        )
        .into());
    }
    Ok(bytes)
}

/// Which way along the image a span is measured.
#[derive(Debug, Clone, Copy)]
enum Axis {
    /// Down the image.
    Row,
    /// Across it.
    Column,
}

/// Every pixel of `preview`, with where it sits.
pub fn pixels(preview: &Preview) -> impl Iterator<Item = (u32, u32, Pixel)> + '_ {
    (0..preview.height()).flat_map(move |row| {
        (0..preview.width())
            .filter_map(move |column| preview.pixel(column, row).map(|pixel| (column, row, pixel)))
    })
}

/// The inclusive span `paint` occupies along `axis`, or `None` where it is not
/// drawn at all.
fn span(preview: &Preview, paint: Paint, axis: Axis) -> Option<(u32, u32)> {
    let found = pixels(preview)
        .filter(|(_, _, pixel)| paint.shows(*pixel))
        .map(|(column, row, _)| match axis {
            Axis::Row => row,
            Axis::Column => column,
        });
    found.fold(None, |reach, at| match reach {
        None => Some((at, at)),
        Some((low, high)) => Some((low.min(at), high.max(at))),
    })
}

/// Whether `first`'s span ends before `second`'s begins.
fn separation(
    first_span: Option<(u32, u32)>,
    second_span: Option<(u32, u32)>,
    first: Paint,
    second: Paint,
) -> Placement {
    let (Some(first_span), Some(second_span)) = (first_span, second_span) else {
        return Placement::Missing(if first_span.is_none() { first } else { second });
    };
    if first_span.1 < second_span.0 {
        return Placement::FirstBeforeSecond;
    }
    Placement::NotSeparated {
        first: first_span,
        second: second_span,
    }
}
