//! The rectangles a HUD layout covers, derived from what content declared and
//! from the size of the target it is composed onto.
//!
//! One UI unit is one physical pixel at a render-target height of 720 and scales
//! linearly with height, so a declaration is resolution-independent and no
//! element is placed at an absolute pixel. Anchors other than `center` are
//! measured from a 5% safe-area inset on each axis rather than from the raw
//! screen edge.
//!
//! **This module is not the only implementation of that rule, and that is
//! deliberate.** The frame-level prediction that judges a rendered crosshair
//! re-derives it from the declarations alone and shares no code with anything
//! here: two derivations that agree are evidence, one derivation called twice is
//! not. The rule is written down in the architecture so that neither of the two
//! is the other's source — the precedent is `Frustum::admits` and the WGSL
//! frustum test, which are the same maths written twice for the same reason.

use std::sync::Arc;

use mc_core::hud::{Anchor, Draw, HudElement, HudLayout, ReadableValue, Rgba8};
use mc_core::id::TextureKey;

use crate::surface::SurfaceSize;
use crate::texture::TextureLayers;

/// The render-target height at which one UI unit is one physical pixel.
///
/// Everything scales from the height alone, so a declaration says the same
/// thing on a 16:9 target as on the 32:9 one an ultrawide reports — a rule
/// keyed to the width would make a bar twice as thick on the second.
const REFERENCE_HEIGHT: f64 = 720.0;

/// How far in from each edge of the target an anchored element is held, as a
/// fraction of that edge's own extent.
const SAFE_AREA_FRACTION: f64 = 0.05;

/// How thick an outline is, in UI units.
///
/// One unit, scaled by the same `max(1, round(unit × scale))` rule as any
/// declared extent, so an outline is a hairline at every target height rather
/// than a border that vanishes on a small window and thickens on a large one.
const OUTLINE_UNITS: u32 = 1;

/// Everything a composition reads: the elements content declared, and the live
/// state a declaration may bind a draw to.
#[derive(Debug, Clone)]
pub struct HudFrame {
    pub layout: Arc<HudLayout>,
    /// The texture of the block a placement would use, where the session holds
    /// one.
    pub held: Option<TextureKey>,
}

/// What a rectangle is painted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Painted {
    Fill(Rgba8),
    /// The layer of the array texture the rectangle samples.
    Texture(u16),
}

/// One rectangle of the plan, in physical pixels, already clipped to the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintedRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub paint: Painted,
}

/// The rectangles `frame`'s elements cover on a target of size `target`.
///
/// Target `W × H`, `scale = H / 720`, and `round` is half **away from zero**:
///
/// - `w = max(1, round(size.x × scale))`, `h = max(1, round(size.y × scale))`
/// - `ox = round(offset.x × scale)`, `oy = round(offset.y × scale)`; `+x` right,
///   `+y` down
/// - `inset_x = round(0.05 × W)`, `inset_y = round(0.05 × H)` — per axis, from
///   that axis's own extent
/// - `center` is centred on `(W/2, H/2)` and is **not** inset
/// - every other anchor puts its named edges on the safe-area box
///   (`inset_x .. W−inset_x`, `inset_y .. H−inset_y`), and a free axis centres on
///   the **target** rather than on the box
/// - the origin from a centre `c` and an extent `e` is `left = round(c − e/2)`
/// - every rectangle is intersected with `0..W × 0..H` **after** offsetting, so
///   nothing wraps to the opposite edge
/// - a target of no height composes nothing, and that is not an error
///
/// The plan is built in **two passes, each in file-name sorted order**: every
/// element's outline ring first, then every element's fill. Every-outline-then-
/// every-fill rather than per-element, because two outlined bars crossing at the
/// centre would otherwise let the later bar's outline cut a black notch through
/// the earlier bar's fill — which is exactly what the base crosshair's two bars
/// do. That makes it a correctness rule rather than polish, and the falsifier is
/// an outline pixel of a later element landing inside an earlier element's fill:
/// the earlier fill is what has to show there.
///
/// **An element whose draw resolves to nothing contributes nothing at all — no
/// fill and no ring.** A textured swatch resolves to nothing while the session
/// holds no block and while what it holds occupies no layer, and an outline
/// composed on its own would leave a bordered empty square at the anchor: a
/// reader of that frame sees an indicator of nothing and cannot tell it from a
/// broken swatch. The rule is keyed on the paint being unavailable and never on
/// clipping — an element pushed off the target has a fill to draw and rings as
/// far as the target reaches, which is a different question.
pub fn compose(frame: &HudFrame, target: SurfaceSize, layers: &TextureLayers) -> Vec<PaintedRect> {
    // Stated as its own rule rather than left to the clipping below, which
    // would also empty the plan: a target of no height has no scale to derive,
    // and a minimised window reports exactly that every time it is minimised.
    if target.height == 0 {
        return Vec::new();
    }
    let scale = f64::from(target.height) / REFERENCE_HEIGHT;
    // Resolved once, before either pass, because the two have to agree about
    // which elements draw: an outline pass that decided for itself would ring
    // an element the fill pass then declined to paint.
    let drawable: Vec<(&HudElement, Painted)> = frame
        .layout
        .elements()
        .iter()
        .filter_map(|element| Some((element, paint_of(element, frame, layers)?)))
        .collect();
    let outlines = drawable
        .iter()
        .flat_map(|(element, _)| ring_of(element, target, scale));
    let fills = drawable
        .iter()
        .filter_map(|(element, paint)| clipped(placement(element, target, scale), target, *paint));
    outlines.chain(fills).collect()
}

/// What `element`'s fill is painted with, or nothing where what it declares is
/// not there to be drawn.
///
/// A declared colour is always available. A textured swatch is available only
/// while the readable value it names resolves to a layer of the array texture —
/// so the same declaration draws and does not draw over the life of one run,
/// which is what makes an indicator of what a placement would use an indicator
/// at all.
fn paint_of(element: &HudElement, frame: &HudFrame, layers: &TextureLayers) -> Option<Painted> {
    match element.draw {
        Draw::Fill { color } => Some(Painted::Fill(color)),
        Draw::BlockTexture {
            source: ReadableValue::HeldBlock,
        } => frame
            .held
            .as_ref()
            .and_then(|key| layers.layer_of(key))
            .map(Painted::Texture),
    }
}

/// The rectangles `element`'s outline ring covers, or none where it declares no
/// outline.
///
/// A **ring** — the four strips of the expanded rectangle minus the fill — and
/// not a solid rectangle underneath the fill. With a solid one a translucent
/// fill would blend against its own outline colour instead of against the scene,
/// which nothing asks for and which would make an alpha composite depend on
/// whether the element happened to declare an outline.
///
/// Grown from the **same** placement the fill is derived from, which is why
/// placement and clipping are separate: a second derivation here could disagree
/// with the fill's about where the element is, and the ring would sit a pixel
/// off in a way only a rendered frame could show.
///
/// An outline is a property of the element's rectangle rather than of what fills
/// it, so a textured swatch that declares one is ringed by it — but only while it
/// has a texture to draw. [`compose`] calls this for the elements whose draw
/// resolved and for no others, which is where the ring around nothing is refused.
fn ring_of(element: &HudElement, target: SurfaceSize, scale: f64) -> Vec<PaintedRect> {
    let Some(color) = element.outline else {
        return Vec::new();
    };
    let placed = placement(element, target, scale);
    let paint = Painted::Fill(color);
    strips_around(&placed, scaled_extent(OUTLINE_UNITS, scale))
        .into_iter()
        .filter_map(|strip| clipped(strip, target, paint))
        .collect()
}

/// The four strips of a ring `thickness` pixels thick around `placed`.
///
/// The two horizontal strips carry the corners and the two vertical ones run
/// between them, so the four meet edge to edge and no pixel of the ring is
/// covered twice — a rectangle painted twice would composite twice, and a
/// translucent outline would show its own corners darker than its sides.
fn strips_around(placed: &Placed, thickness: i64) -> [Placed; 4] {
    let Placed {
        left,
        top,
        width,
        height,
    } = *placed;
    let outer_left = left - thickness;
    let spanning = width + 2 * thickness;
    [
        (outer_left, top - thickness, spanning, thickness),
        (outer_left, top + height, spanning, thickness),
        (outer_left, top, thickness, height),
        (left + width, top, thickness, height),
    ]
    .map(|(left, top, width, height)| Placed {
        left,
        top,
        width,
        height,
    })
}

/// Where an element sits on the target before anything is cut off it, in
/// physical pixels.
///
/// Kept apart from the clipping so that an outline ring — the same rectangle
/// grown by its thickness, minus the fill — is derived from one placement
/// rather than from a second one that could disagree with it.
struct Placed {
    left: i64,
    top: i64,
    width: i64,
    height: i64,
}

/// Where `element` lands on `target` at `scale`, anchored and then displaced.
fn placement(element: &HudElement, target: SurfaceSize, scale: f64) -> Placed {
    let [declared_width, declared_height] = element.size;
    let width = scaled_extent(declared_width, scale);
    let height = scaled_extent(declared_height, scale);
    let (across, down) = measured_along(element.anchor);
    let [rightward, downward] = element.offset;
    Placed {
        left: start(across, target.width, width) + displacement(rightward, scale),
        top: start(down, target.height, height) + displacement(downward, scale),
        width,
        height,
    }
}

/// How an anchor is measured along one axis.
///
/// `Centred` is centred on the **target** rather than on the safe-area box, and
/// that is the same statement for `center` as for the free axis of `bottom`:
/// the two differ only in whether the other axis is inset, so no anchor needs a
/// case of its own here.
enum Along {
    Near,
    Centred,
    Far,
}

/// How `anchor` is measured horizontally and vertically.
const fn measured_along(anchor: Anchor) -> (Along, Along) {
    match anchor {
        Anchor::TopLeft => (Along::Near, Along::Near),
        Anchor::Top => (Along::Centred, Along::Near),
        Anchor::TopRight => (Along::Far, Along::Near),
        Anchor::Left => (Along::Near, Along::Centred),
        Anchor::Center => (Along::Centred, Along::Centred),
        Anchor::Right => (Along::Far, Along::Centred),
        Anchor::BottomLeft => (Along::Near, Along::Far),
        Anchor::Bottom => (Along::Centred, Along::Far),
        Anchor::BottomRight => (Along::Far, Along::Far),
    }
}

/// Where an extent of `extent` starts on an axis of `span` physical pixels.
///
/// The safe-area inset is 5% of `span` itself, so a target's two axes are inset
/// by different amounts whenever it is not square — 64 and 36 at 1280 × 720.
fn start(along: Along, span: u32, extent: i64) -> i64 {
    let inset = round_half_away(f64::from(span) * SAFE_AREA_FRACTION);
    match along {
        Along::Near => inset,
        Along::Centred => round_half_away(f64::from(span) / 2.0 - extent as f64 / 2.0),
        Along::Far => i64::from(span) - inset - extent,
    }
}

/// A declared extent in physical pixels.
///
/// Floored at one pixel: an element that scaled to nothing would be declared,
/// registered and invisible, which is worse than one pixel too many.
fn scaled_extent(declared: u32, scale: f64) -> i64 {
    round_half_away(f64::from(declared) * scale).max(1)
}

/// A declared displacement in physical pixels, `+x` right and `+y` down.
fn displacement(declared: i32, scale: f64) -> i64 {
    round_half_away(f64::from(declared) * scale)
}

/// `value` at the nearest whole pixel, with a half rounded away from zero.
///
/// Half away from zero rather than to even: the rule has to be stated the same
/// way in the prediction that judges a rendered frame, and "round" written in
/// two places has to mean one thing.
fn round_half_away(value: f64) -> i64 {
    value.round() as i64
}

/// `placed` intersected with the target, or nothing where the two do not meet.
///
/// Intersected rather than wrapped: an element displaced off the right edge
/// loses the columns that fell off it and grows none at the left.
fn clipped(placed: Placed, target: SurfaceSize, paint: Painted) -> Option<PaintedRect> {
    let left = placed.left.max(0);
    let top = placed.top.max(0);
    let right = placed
        .left
        .saturating_add(placed.width)
        .min(i64::from(target.width));
    let bottom = placed
        .top
        .saturating_add(placed.height)
        .min(i64::from(target.height));
    if right <= left || bottom <= top {
        return None;
    }
    Some(PaintedRect {
        x: i32::try_from(left).ok()?,
        y: i32::try_from(top).ok()?,
        width: u32::try_from(right - left).ok()?,
        height: u32::try_from(bottom - top).ok()?,
        paint,
    })
}

#[cfg(test)]
#[path = "plan_test.rs"]
mod tests;
