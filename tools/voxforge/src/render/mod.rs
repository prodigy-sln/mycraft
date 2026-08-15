//! Preview rendering: one DDA ray march, one camera-basis formula, ten views.
//!
//! This is what the feature exists for. An agent authors a model, renders it,
//! and corrects itself against the picture — so a mirrored or axis-swapped
//! preview does not merely look wrong, it teaches the agent to "fix" correct
//! geometry to match a broken view. Three properties hold that line:
//!
//! - **One opaque sample per pixel.** A pixel is either a voxel's shaded colour
//!   at alpha 255 or the background at alpha 0. There is no third answer, which
//!   is what makes every per-pixel assertion decidable — anti-aliasing would
//!   make "every non-background pixel derives from A" false against a *correct*
//!   renderer.
//! - **Row 0 is the top**, by construction rather than by a flip: a pixel at
//!   `(column, row)` samples `+(column + 0.5)/ppv` along `right` and
//!   `−(row + 0.5)/ppv` along `up`. See the `raster` module.
//! - **The ten views are one derivation**, `right = normalize(d × w)` and
//!   `up = right × d`, never ten hand-written bases. See [`camera`].

// Public because D4's basis is a **contract** rather than an intermediate: the
// eleven declared vectors are graded directly, so that one wrong cross-product
// order localises here instead of arriving as several unrelated pixel failures.
pub mod camera;

mod dda;
mod raster;
mod shade;
mod sheet;

use std::num::NonZeroU32;

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageError};

use crate::fault::{Fault, Origin};
use crate::material::MaterialTable;
use crate::texture::AxisAlignedView;
use crate::volume::Volume;

/// How many bytes one pixel occupies: red, green, blue, alpha.
const CHANNELS: usize = 4;

/// One of the ten canonical directions a model is previewed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum View {
    /// Along `−z`, from in front of the model.
    Front,
    /// Along `+z`, from behind it.
    Back,
    /// Along `+x`, from its left.
    Left,
    /// Along `−x`, from its right.
    Right,
    /// Along `−y`, from above it.
    Top,
    /// Along `+y`, from below it.
    Bottom,
    /// From the front-left corner, above.
    IsoFl,
    /// From the front-right corner, above.
    IsoFr,
    /// From the back-left corner, above.
    IsoBl,
    /// From the back-right corner, above.
    IsoBr,
    /// From the front-left corner, below.
    IsoFlUnder,
    /// From the front-right corner, below.
    IsoFrUnder,
    /// From the back-left corner, below.
    IsoBlUnder,
    /// From the back-right corner, below.
    IsoBrUnder,
}

impl View {
    /// Every canonical view, in the order a contact sheet tiles them and a
    /// refusal lists them.
    pub const ALL: [Self; 14] = [
        Self::Front,
        Self::Back,
        Self::Left,
        Self::Right,
        Self::Top,
        Self::Bottom,
        Self::IsoFl,
        Self::IsoFr,
        Self::IsoBl,
        Self::IsoBr,
        Self::IsoFlUnder,
        Self::IsoFrUnder,
        Self::IsoBlUnder,
        Self::IsoBrUnder,
    ];

    /// The view as a caller spells it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Back => "back",
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::IsoFl => "iso-fl",
            Self::IsoFr => "iso-fr",
            Self::IsoBl => "iso-bl",
            Self::IsoBr => "iso-br",
            Self::IsoFlUnder => "iso-fl-under",
            Self::IsoFrUnder => "iso-fr-under",
            Self::IsoBlUnder => "iso-bl-under",
            Self::IsoBrUnder => "iso-br-under",
        }
    }
}

/// One pixel of a preview, straight alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pixel {
    /// The red channel.
    pub red: u8,
    /// The green channel.
    pub green: u8,
    /// The blue channel.
    pub blue: u8,
    /// Opacity: 255 where a voxel was hit, 0 where none was.
    pub alpha: u8,
}

impl Pixel {
    /// What a pixel no voxel projects onto holds.
    pub const BACKGROUND: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 0,
    };

    /// The opaque pixel of that colour.
    #[must_use]
    pub fn opaque(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    /// Whether no voxel projected onto this pixel.
    #[must_use]
    pub fn is_background(self) -> bool {
        self.alpha == 0
    }
}

/// One rendered view.
///
/// VoxForge's own image type. It is deliberately not `mc_testkit`'s
/// `Rgba8Image`: the harness is a dev-only comparison tool and must never enter
/// this crate's runtime graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    width: u32,
    height: u32,
    /// `width · height · 4` bytes, row 0 first, each row left to right.
    pixels: Vec<u8>,
}

impl Preview {
    /// How many pixels wide the image is.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// How many pixels tall the image is.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The pixel at `column` and `row`, or `None` where that is outside the
    /// image.
    ///
    /// Row 0 is the top of the image and column 0 its left.
    #[must_use]
    pub fn pixel(&self, column: u32, row: u32) -> Option<Pixel> {
        if column >= self.width || row >= self.height {
            return None;
        }
        let width = usize::try_from(self.width).ok()?;
        let offset = usize::try_from(row)
            .ok()?
            .checked_mul(width)?
            .checked_add(usize::try_from(column).ok()?)?
            .checked_mul(CHANNELS)?;
        let bytes = self.pixels.get(offset..offset.checked_add(CHANNELS)?)?;
        Some(Pixel {
            red: *bytes.first()?,
            green: *bytes.get(1)?,
            blue: *bytes.get(2)?,
            alpha: *bytes.get(3)?,
        })
    }

    /// Paints `pixel` at `column` and `row`, where that is inside the image.
    ///
    /// Out of range is ignored rather than wrapped: a pixel written somewhere
    /// nobody asked for is worse than one not written at all, and the raster
    /// derives its own extent so the case does not arise from a legal render.
    fn set(&mut self, column: u32, row: u32, pixel: Pixel) {
        if column >= self.width || row >= self.height {
            return;
        }
        let offset = usize::try_from(self.width)
            .ok()
            .and_then(|width| usize::try_from(row).ok()?.checked_mul(width))
            .and_then(|start| start.checked_add(usize::try_from(column).ok()?))
            .and_then(|at| at.checked_mul(CHANNELS));
        let Some(bytes) = offset.and_then(|at| self.pixels.get_mut(at..at.checked_add(CHANNELS)?))
        else {
            return;
        };
        for (slot, value) in bytes
            .iter_mut()
            .zip([pixel.red, pixel.green, pixel.blue, pixel.alpha])
        {
            *slot = value;
        }
    }

    /// An image of `width` by `height` on which no voxel has landed yet.
    fn blank(width: u32, height: u32) -> Self {
        let count = usize::try_from(width)
            .ok()
            .and_then(|width| width.checked_mul(usize::try_from(height).ok()?))
            .and_then(|pixels| pixels.checked_mul(CHANNELS))
            .unwrap_or(0);
        Self {
            width,
            height,
            pixels: vec![0; count],
        }
    }
}

/// Where one tile of a contact sheet sits.
///
/// Both positions, because they answer different questions: the grid cell is
/// what the printed mapping names, and the pixel rectangle is what the tile's
/// own render has to match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRect {
    /// The tile grid column this tile occupies, counted from zero.
    pub column: u32,
    /// The tile grid row this tile occupies, counted from zero.
    pub row: u32,
    /// The leftmost pixel column of the tile within the sheet.
    pub left: u32,
    /// The topmost pixel row of the tile within the sheet.
    pub top: u32,
    /// How many pixels wide the tile is.
    pub width: u32,
    /// How many pixels tall the tile is.
    pub height: u32,
}

/// Every canonical view of one model, tiled into a single image.
///
/// The sheet carries no rendered text at all. The tile-to-view mapping is
/// [`ContactSheet::legend`] and reaches the reader on stdout, because a pixel
/// assertion about a label passes on glyphs that are garbage, transposed or all
/// identical, and a printed line does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactSheet {
    image: Preview,
    tiles: Vec<(View, TileRect)>,
}

impl ContactSheet {
    /// The sheet as one image.
    #[must_use]
    pub fn image(&self) -> &Preview {
        &self.image
    }

    /// Every tile, in the sheet's declared order.
    #[must_use]
    pub fn tiles(&self) -> &[(View, TileRect)] {
        &self.tiles
    }

    /// One line per tile naming its grid position and the view it holds, in the
    /// sheet's declared order.
    ///
    /// Each line names the view and spells its position as `column {c}` and
    /// `row {r}`, so that an agent reading stdout can say which tile is which
    /// without any pixel telling it.
    #[must_use]
    pub fn legend(&self) -> Vec<String> {
        self.tiles
            .iter()
            .map(|(view, rect)| {
                format!(
                    "column {column}, row {row}: {view}",
                    column = rect.column,
                    row = rect.row,
                    view = view.as_str()
                )
            })
            .collect()
    }
}

/// The model of `volume` as seen from `view`, at `pixels_per_voxel`.
///
/// A palette key `materials` does not declare renders as empty space; the
/// document loader binds the palette against the table before anything reaches
/// here, so that state does not arise from a legal run.
#[must_use]
pub fn render(
    volume: &Volume,
    materials: &MaterialTable,
    view: View,
    pixels_per_voxel: NonZeroU32,
) -> Preview {
    raster::render(volume, materials, view, pixels_per_voxel)
}

/// Whether a render darkens each face by its own factor, or emits the declared
/// colour whatever face it is looking at.
///
/// An input to the render rather than a property of a material: a material
/// table is shared across a whole art set, so it cannot express the intent of
/// one emission. Flatness is not "emissive 1.0" either — `Flat` emits the
/// declared colour **regardless** of a material's emissive, so that no path
/// exists from a material's self-illumination into a texture the day a shading
/// term appears that is not gated on emissive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shading {
    /// Each face darkened by its own factor.
    Shaded,
    /// The declared colour, on every facing.
    Flat,
}

/// The block texture of `volume`'s `face`, at `pixels_per_voxel`.
///
/// The same camera basis, the same ray march and the same raster as [`render`]
/// — the colour function is the one axis this differs on, and it is resolved
/// once per render rather than per pixel.
#[must_use]
pub fn render_texture(
    volume: &Volume,
    materials: &MaterialTable,
    face: AxisAlignedView,
    pixels_per_voxel: NonZeroU32,
) -> Preview {
    raster::render_with(
        volume,
        materials,
        raster::Settings {
            view: face.view(),
            pixels_per_voxel,
            shading: Shading::Flat,
        },
    )
}

/// Every canonical view of `volume`, tiled into one sheet.
#[must_use]
pub fn contact_sheet(
    volume: &Volume,
    materials: &MaterialTable,
    pixels_per_voxel: NonZeroU32,
) -> ContactSheet {
    sheet::contact_sheet(volume, materials, pixels_per_voxel)
}

/// The PNG encoding of `preview`, destined for `origin`.
///
/// The whole byte vector exists before anyone opens a file, so that a late
/// failure cannot leave a truncated image behind. `origin` is the file those
/// bytes are for, and is carried only so that a failure names it.
///
/// # Errors
///
/// Returns a [`Fault`] carrying the encoder's own message. An in-memory encode
/// of a well-formed RGBA buffer does not fail in practice; it is reported
/// rather than unwrapped because a preview is not worth aborting a process
/// over, and swallowed by nobody because the encoder's words are the only clue
/// there would be.
pub fn to_png(preview: &Preview, origin: Origin) -> Result<Vec<u8>, Fault> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            &preview.pixels,
            preview.width,
            preview.height,
            ExtendedColorType::Rgba8,
        )
        .map_err(|cause: ImageError| {
            Fault::about(
                origin,
                format!("the preview could not be encoded as a PNG: {cause}"),
            )
        })?;
    Ok(bytes)
}

/// The view `text` names, refused against `origin` when it names none.
///
/// # Errors
///
/// Returns a [`Fault`] naming the value given and every view there is, since an
/// agent repairing its own command line has only the message to repair from.
pub fn view_named(text: &str, origin: Origin) -> Result<View, Fault> {
    View::ALL
        .into_iter()
        .find(|view| view.as_str() == text)
        .ok_or_else(|| {
            let names: Vec<&str> = View::ALL.iter().map(|view| view.as_str()).collect();
            Fault::about(
                origin,
                format!(
                    "`{text}` is not a view — the ten canonical views are {offered}",
                    offered = names.join(", ")
                ),
            )
            .in_field("view")
        })
}

/// `requested` pixels per voxel, refused against `origin` when it is zero.
///
/// # Errors
///
/// Returns a [`Fault`] naming the value given and the minimum.
pub fn pixels_per_voxel(requested: u32, origin: Origin) -> Result<NonZeroU32, Fault> {
    NonZeroU32::new(requested).ok_or_else(|| {
        Fault::about(
            origin,
            format!(
                "{requested} pixels per voxel renders an image of no pixels, which is not a smaller preview but no preview — the minimum is 1"
            ),
        )
        .in_field("pixels-per-voxel")
    })
}
