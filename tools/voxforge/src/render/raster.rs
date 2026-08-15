//! The image a view produces: how big it is, and what each pixel samples.
//!
//! The extent is the model's own box, projected onto the view's two image axes
//! and scaled — never a fixed square, so a transposed raster is a differently
//! *shaped* image rather than a plausible one.
//!
//! **Row 0 is the top by construction rather than by a flip.** A pixel at
//! `(column, row)` samples `+(column + 0.5)/ppv` along `right` from the box's
//! projected minimum, and `−(row + 0.5)/ppv` along `up` from its projected
//! maximum. There is nowhere in this file for a row order to be reversed,
//! because nothing here ever reverses one.

use std::num::NonZeroU32;

use glam::DVec3;

use crate::material::{Material, MaterialTable};
use crate::render::camera::{Basis, basis_of};
use crate::render::shade::Face;
use crate::render::{Pixel, Preview, Shading, View, dda, shade};
use crate::volume::Volume;

/// How far back along the view direction a ray starts, beyond the model's box.
///
/// Any distance clear of the volume does; the march finds its own entry point.
const STAND_OFF: f64 = 256.0;

/// How finely a projected span is rounded before its pixel count is taken.
///
/// An axis-aligned span is a whole number of voxels, and a projection landing a
/// hair under one would otherwise buy an extra column of background.
const ROUNDING: f64 = 1e6;

/// The model of `volume` as seen from `view`, shaded.
#[must_use]
pub fn render(
    volume: &Volume,
    materials: &MaterialTable,
    view: View,
    pixels_per_voxel: NonZeroU32,
) -> Preview {
    render_with(
        volume,
        materials,
        Settings {
            view,
            pixels_per_voxel,
            shading: Shading::Shaded,
        },
    )
}

/// What one render is asked for, beyond the model and its materials.
///
/// A struct rather than three more parameters: `render` already sits at the
/// four-argument ceiling, which is the constraint that shaped this whole design
/// — the alternative was changing `render`'s own signature and every Phase 4
/// call site with it, including test files a completed phase owns.
pub(super) struct Settings {
    /// Which direction the model is seen from.
    pub view: View,
    /// How many pixels one voxel spans.
    pub pixels_per_voxel: NonZeroU32,
    /// Whether each face is darkened by its own factor.
    pub shading: Shading,
}

/// The one core both public renders forward onto.
///
/// The camera basis, the ray march and the raster are shared by construction —
/// the colour function is the single axis this varies on, and it is resolved
/// **once per render** rather than per pixel. That is why this is not the
/// duplicated code path D3 argues against: the orientation maths is reached by
/// exactly one route whichever forwarder called it.
#[must_use]
pub(super) fn render_with(
    volume: &Volume,
    materials: &MaterialTable,
    settings: Settings,
) -> Preview {
    let Settings {
        view,
        pixels_per_voxel,
        shading,
    } = settings;
    let basis = basis_of(view);
    let frame = Frame::of(volume, basis, pixels_per_voxel);
    let scene = Scene {
        volume,
        materials,
        colour: match shading {
            Shading::Shaded => shade::shade,
            Shading::Flat => |material, _face| shade::flat(material),
        },
    };
    let mut preview = Preview::blank(frame.width, frame.height);
    for row in 0..frame.height {
        paint_row(&mut preview, &frame, &scene, row);
    }
    preview
}

/// What a ray can hit, what it is made of, and what colour that comes out.
struct Scene<'a> {
    /// The assembled model.
    volume: &'a Volume,
    /// What each of its materials looks like.
    materials: &'a MaterialTable,
    /// How a material and the face a ray arrived through become a pixel.
    colour: fn(&Material, Face) -> Pixel,
}

/// One row of the image.
fn paint_row(preview: &mut Preview, frame: &Frame, scene: &Scene<'_>, row: u32) {
    for column in 0..frame.width {
        if let Some(pixel) = frame.sample(scene, column, row) {
            preview.set(column, row, pixel);
        }
    }
}

/// The image plane one view projects a model onto.
struct Frame {
    /// The view's axes.
    basis: Basis,
    /// Where the box's projected minimum sits along `right`.
    left: f64,
    /// Where its projected maximum sits along `up`.
    top: f64,
    /// How many pixels one voxel spans.
    scale: f64,
    /// How many pixels across the image is.
    width: u32,
    /// How many pixels down it is.
    height: u32,
}

impl Frame {
    /// The frame `volume` projects onto through `basis`.
    fn of(volume: &Volume, basis: Basis, pixels_per_voxel: NonZeroU32) -> Self {
        let extent = volume.extent();
        let scale = f64::from(pixels_per_voxel.get());
        let corners = corners_of(extent.x, extent.y, extent.z);
        let across: Vec<f64> = corners.iter().map(|at| at.dot(basis.right)).collect();
        let down: Vec<f64> = corners.iter().map(|at| at.dot(basis.up)).collect();
        let (left, right) = span(&across);
        let (bottom, top) = span(&down);
        Self {
            basis,
            left,
            top,
            scale,
            width: pixels(right - left, scale),
            height: pixels(top - bottom, scale),
        }
    }

    /// What the pixel at `column` and `row` shows.
    fn sample(&self, scene: &Scene<'_>, column: u32, row: u32) -> Option<Pixel> {
        let across = self.left + (f64::from(column) + 0.5) / self.scale;
        let down = self.top - (f64::from(row) + 0.5) / self.scale;
        let on_plane = self.basis.right * across + self.basis.up * down;
        let origin = on_plane - self.basis.direction * STAND_OFF;
        let hit = dda::first_hit(scene.volume, origin, self.basis.direction)?;
        let material = scene.materials.get(scene.volume.material_at(hit.voxel)?)?;
        Some((scene.colour)(material, hit.face))
    }
}

/// The eight corners of a box reaching `x` by `y` by `z` from the origin.
fn corners_of(x: u32, y: u32, z: u32) -> [DVec3; 8] {
    let (x, y, z) = (f64::from(x), f64::from(y), f64::from(z));
    [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(x, 0.0, 0.0),
        DVec3::new(0.0, y, 0.0),
        DVec3::new(0.0, 0.0, z),
        DVec3::new(x, y, 0.0),
        DVec3::new(x, 0.0, z),
        DVec3::new(0.0, y, z),
        DVec3::new(x, y, z),
    ]
}

/// The lowest and highest of `values`.
fn span(values: &[f64]) -> (f64, f64) {
    values.iter().fold((f64::MAX, f64::MIN), |(low, high), at| {
        (low.min(*at), high.max(*at))
    })
}

/// How many pixels a span of `reach` voxels occupies, rounded up.
fn pixels(reach: f64, scale: f64) -> u32 {
    // Rounded before the ceiling: an axis-aligned span is a whole number of
    // voxels, and a projection that lands a hair under it would otherwise buy an
    // extra column of background.
    let exact = reach * scale;
    let rounded = (exact * ROUNDING).round() / ROUNDING;
    let count = rounded.ceil() as u32;
    count.max(1)
}
