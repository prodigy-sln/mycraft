//! The bytes one HUD composition uploads, and the one place a declared colour
//! becomes linear light.
//!
//! Pure, and deliberately **outside** `src/gpu/`. ADR-013 exempts that subtree
//! from the coverage denominator because only a golden frame can see it, and
//! nothing here needs a device: which colour space a declaration is converted
//! into, where a rectangle's four numbers land in the buffer, and how many
//! rectangles a composition can hold are all arithmetic over plain values.
//!
//! # The conversion this project has already got wrong once
//!
//! The colour target is `Rgba8UnormSrgb`. The blend hardware decodes the
//! destination to linear light, blends, and re-encodes on write, so a declared
//! colour handed over **undecoded** is composited in the wrong space — the
//! defect `docs/technical/rendering.md` records as invisible once shipped,
//! plausible-looking, and wrong in the same direction everywhere. It arrived
//! once at the clear colour, where a unit test of the conversion and a test
//! comparing two configurations *to each other* were both green while every
//! frame shipped visibly wrong. So the decode happens here, through the single
//! [`srgb8_to_linear`] every other consumer uses, and never in whatever
//! configures a pass.
//!
//! **Alpha is not a colour.** It is a coverage fraction, the transfer function
//! does not apply to it, and it is scaled and passed through undecoded.

use mc_core::hud::Rgba8;

use crate::color::srgb8_to_linear;
use crate::surface::SurfaceSize;

use super::{Painted, PaintedRect};

/// How many rectangles one composition can paint.
///
/// A uniform buffer holds a fixed-size array, so this is the array's length and
/// the pass's capacity at once. `content/base/` ships three elements and each
/// contributes at most five rectangles — a fill and its four ring strips — so
/// the ceiling is far above anything the base game reaches; it is what a
/// third-party mod would have to exceed to lose a rectangle.
///
/// `shaders/hud.wgsl` declares the same number and the two are not mechanically
/// tied. A shader whose array were **smaller** would leave the rectangles past
/// its end unpainted, which no test in this spec reaches — recorded as a
/// deferred observation rather than papered over with a comment.
pub const MAX_HUD_RECTS: usize = 256;

/// How many bytes one rectangle occupies: its bounds, its colour, then what it
/// samples — each a four-component vector of 32-bit floats.
const RECT_BYTES: usize = 48;

/// What a rectangle's array layer reads as when it samples no texture.
///
/// Negative rather than a fourth vector or a separate flag: the shader compares
/// against zero, and a layer index is never negative, so one component carries
/// both the question and the answer. A `u16` layer widens to `f32` exactly, so
/// no index this can express is lost on the way.
const PAINTED_FLAT: f32 = -1.0;

/// How many bytes precede the array: the target's extents, padded to the
/// 16-byte alignment a uniform's array member starts on.
const HEADER_BYTES: usize = 16;

/// How large the uniform is, in bytes.
///
/// Fixed rather than sized per composition: the buffer is allocated once, and a
/// pass that reallocated per frame would put the HUD on the allocator every
/// frame for a saving of a few kilobytes.
pub const HUD_UNIFORM_BYTES: usize = HEADER_BYTES + MAX_HUD_RECTS * RECT_BYTES;

/// The largest value an 8-bit channel takes.
const CHANNEL_MAX: f32 = 255.0;

/// What one composition uploads, and how many rectangles it draws.
///
/// The two travel together because they have to agree: a count taken from the
/// plan while the bytes were built from a filtered subset of it would draw
/// instances whose data was never written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudUniform {
    pub bytes: Vec<u8>,
    pub rects: u32,
}

/// The uniform painting `planned` onto a target of `target` pixels.
///
/// Rectangles past [`MAX_HUD_RECTS`] are dropped. Nothing else is: a rectangle
/// [`compose`](super::compose) planned is a rectangle that draws, and a filter
/// here would be a second opinion about that — the composition already declines
/// to plan a swatch it cannot resolve.
#[must_use]
pub fn hud_uniform(planned: &[PaintedRect], target: SurfaceSize) -> HudUniform {
    let painted: Vec<&PaintedRect> = planned.iter().take(MAX_HUD_RECTS).collect();

    let mut bytes = Vec::with_capacity(HUD_UNIFORM_BYTES);
    bytes.extend(vector_bytes([
        target.width as f32,
        target.height as f32,
        0.0,
        0.0,
    ]));
    for rect in &painted {
        bytes.extend(rect_bytes(rect));
    }
    // The tail is never read — the draw issues exactly `rects` instances — but a
    // uniform binding is sized by its declared type rather than by what was
    // written into it, so the buffer has to be whole.
    bytes.resize(HUD_UNIFORM_BYTES, 0);

    HudUniform {
        bytes,
        rects: painted.len() as u32,
    }
}

/// One rectangle: its bounds in physical pixels, its colour in linear light,
/// then the array layer it samples.
fn rect_bytes(rect: &PaintedRect) -> impl Iterator<Item = u8> {
    let bounds = vector_bytes([
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
    ]);
    let sampling = vector_bytes([layer_of(rect.paint), 0.0, 0.0, 0.0]);
    bounds
        .chain(vector_bytes(linear_of(rect.paint)))
        .chain(sampling)
}

/// A painted colour as the shader takes it: linear light, straight alpha.
///
/// A textured rectangle's colour is opaque white because the shader reads the
/// sampled texel instead of it. White rather than transparent black so that the
/// day this becomes a tint it is a neutral one — the same value with nothing
/// multiplying it, rather than a value that would annihilate the swatch.
fn linear_of(paint: Painted) -> [f32; 4] {
    match paint {
        Painted::Fill(color) => linear_of_color(color),
        Painted::Texture(_) => [1.0, 1.0, 1.0, 1.0],
    }
}

/// Which array layer a rectangle samples, or [`PAINTED_FLAT`] where it is painted
/// with its colour alone.
fn layer_of(paint: Painted) -> f32 {
    match paint {
        Painted::Fill(_) => PAINTED_FLAT,
        Painted::Texture(layer) => f32::from(layer),
    }
}

/// A declared colour decoded into linear light, with its alpha scaled but not
/// decoded.
fn linear_of_color(color: Rgba8) -> [f32; 4] {
    let [red, green, blue] = srgb8_to_linear([color.r, color.g, color.b]);
    [
        red as f32,
        green as f32,
        blue as f32,
        f32::from(color.a) / CHANNEL_MAX,
    ]
}

/// Four floats, little-endian, as WGSL reads a `vec4<f32>`.
///
/// Explicit `to_le_bytes` rather than a cast over the whole struct: a buffer's
/// byte order is a stated fact rather than whatever the build host happened to
/// be, which is the rule the terrain uniform is built under too.
fn vector_bytes(components: [f32; 4]) -> impl Iterator<Item = u8> {
    components.into_iter().flat_map(f32::to_le_bytes)
}
