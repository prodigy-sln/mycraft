//! An independent prediction of the rectangles and colours the declarations
//! under a content root put on a frame, and a per-pixel reading of a frame
//! against it.
//!
//! # This shares no code with `mc_render::hud`, and that is the whole point
//!
//! The rectangle rule is written in `architecture.md`'s Decision 8 rather than in
//! either implementation, so neither of the two is the other's source. Everything
//! below is re-derived from that text over the parsed declarations: a prediction
//! that called `compose`, or that reused `PaintedRect` or the arithmetic inside
//! `ring_of`, would be the subject agreeing with itself. The precedent is
//! `Frustum::admits` and the WGSL frustum test — the same maths written twice on
//! purpose, because merging them would delete the oracle.
//!
//! What is shared, deliberately, is the **metric**: a distance is a
//! [`distance`](super::probe::distance) against the harness's own `compare`, and
//! the per-pixel tolerance is read out of [`Thresholds::default`] rather than
//! restated as `2.0`. What FR-4.1 replaces is the harness's **area budget**, not
//! its definition of one wrong pixel, and a second copy of that number is a
//! second thing to keep in step.
//!
//! # Why an opaque colour needs no background to be predicted
//!
//! A composite generally depends on what is behind it, and this prediction does
//! not model the scene. It does not have to: `α = 255` makes the blend a no-op,
//! and the CPU-side sRGB decode and the target's hardware re-encode are inverse
//! operations, so the byte a declaration states is the byte the target shows —
//! measured in phase 4 for `#808080`, whose decode is linear 0.2158605 and whose
//! re-encode is byte 128.000. Every colour the base game's crosshair declares is
//! opaque, so the predicted composite *is* the declared colour.
//!
//! A translucent colour, and a textured swatch, are therefore reported as
//! [`PredictedPaint::Unpredictable`] rather than guessed at — and a reading that
//! meets one says so in its verdict instead of quietly looking at fewer pixels.
//!
//! # Two passes, because one crossing bar would otherwise notch the other
//!
//! Every element's outline ring is composed first, in declaration order, and then
//! every element's fill. The base crosshair is two crossing bars that each declare
//! an outline, so at `(640, 359)` the upright's *fill* has to win over the
//! crossbar's *ring*: a per-element outline-then-fill order paints a black notch
//! there. The prediction models the two passes for that reason — the alternative
//! is a prediction that agrees with a defect.
//!
//! # What the prediction is a prediction of
//!
//! It reads declarations, so it predicts what a declaration draws **when its draw
//! resolves**. A textured swatch resolves only while a session holds a block whose
//! texture occupies a layer, so a frame compared against a prediction that
//! includes one has to be a frame where it does.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use mc_core::hud::{Anchor, Draw, HudElement, HudLayout, Rgba8};
use mc_render::surface::SurfaceSize;
use mc_testkit::frame::{Rgba8Image, Thresholds};
use mc_world::content::TomlFileHudSource;

use super::hud_frames::Rect;
use super::probe::distance;

/// The target height at which one UI unit is one physical pixel.
///
/// Everything scales from the height alone, so a declaration says the same thing
/// on a 16:9 target as on an ultrawide.
const REFERENCE_HEIGHT: f64 = 720.0;

/// How far in from each edge an anchored element is held, as a fraction of that
/// edge's own extent.
const SAFE_AREA_FRACTION: f64 = 0.05;

/// How thick a declared outline is, in UI units, before scaling.
const OUTLINE_UNITS: u32 = 1;

/// What a predicted rectangle puts on the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictedPaint {
    /// An opaque declared colour. The composite over any background is the
    /// colour itself, so these three bytes are what the frame has to show.
    Opaque([u8; 3]),
    /// A translucent colour or a block texture: what lands here depends on what
    /// is behind it or on which layer resolved, and neither is predicted from a
    /// declaration alone.
    Unpredictable,
}

/// Where one declared element lands, and what it puts there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictedElement {
    /// The name the declaration registered under.
    pub name: String,
    /// The declared rectangle, clipped to the target. `None` where none of it
    /// falls on the target at all.
    pub fill: Option<Rect>,
    /// The fill grown by the outline's thickness, clipped — the fill itself
    /// where no outline is declared. Fill and ring together.
    pub footprint: Option<Rect>,
    /// What the fill is painted with.
    pub paint: PredictedPaint,
    /// What the ring between the footprint and the fill is painted with, or
    /// nothing where the declaration states no outline.
    pub outline: Option<PredictedPaint>,
}

/// What the declarations under one content root put on a target of one size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prediction {
    elements: Vec<PredictedElement>,
}

impl Prediction {
    /// What the declarations under `root` predict on a target of `target`.
    ///
    /// Read through the loader the client itself reads a content root with, so
    /// the declarations this predicts from are the declarations the product
    /// registers. The *placement* is re-derived here and nowhere else.
    ///
    /// # Errors
    ///
    /// Returns the refusal when the root's declarations do not load — a
    /// prediction from a root that was refused is a prediction of nothing.
    pub fn of(root: &Path, target: SurfaceSize) -> Result<Self, Box<dyn Error>> {
        let layout = HudLayout::load(&TomlFileHudSource::new(root))?;
        Ok(Self {
            elements: layout
                .elements()
                .iter()
                .map(|element| predict(element, target))
                .collect(),
        })
    }

    /// Every element the root declared, in the order it declared them.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.elements
            .iter()
            .map(|element| element.name.as_str())
            .collect()
    }

    /// The element declared under `name`, or nothing where the root declared no
    /// such element.
    #[must_use]
    pub fn element(&self, name: &str) -> Option<&PredictedElement> {
        self.elements.iter().find(|element| element.name == name)
    }

    /// Whether any element's footprint covers `(x, y)`.
    #[must_use]
    pub fn covers(&self, x: u32, y: u32) -> bool {
        self.elements
            .iter()
            .any(|element| element.footprint.is_some_and(|rect| rect.holds(x, y)))
    }

    /// How many pixels the footprints cover between them, counted as a union so
    /// two overlapping elements are not counted twice.
    #[must_use]
    pub fn covered_pixels(&self) -> u64 {
        self.composited().len() as u64
    }

    /// What the two passes leave at every pixel the elements named by `of`
    /// cover, in raster order.
    ///
    /// The composite is built over **every** element and only then narrowed to
    /// the named ones, so a pixel of one element's ring that another element's
    /// fill lands on carries the fill's colour — which is what the two-pass
    /// order means and what a per-element order gets wrong.
    #[must_use]
    pub fn painted(&self, of: &[&str]) -> Vec<PredictedPixel> {
        let named: Vec<&PredictedElement> = self
            .elements
            .iter()
            .filter(|element| of.contains(&element.name.as_str()))
            .collect();
        self.composited()
            .into_iter()
            .filter(|((y, x), _)| {
                named
                    .iter()
                    .any(|element| element.footprint.is_some_and(|rect| rect.holds(*x, *y)))
            })
            .map(|((y, x), paint)| PredictedPixel { x, y, paint })
            .collect()
    }

    /// What every element puts at every pixel it covers, keyed by `(y, x)` so
    /// the order is the frame's own.
    ///
    /// Every ring first and every fill second — one whole pass and then the
    /// other, never one element at a time.
    fn composited(&self) -> Composite {
        let mut painted = Composite::new();
        for element in &self.elements {
            paint_ring(element, &mut painted);
        }
        for element in &self.elements {
            paint_fill(element, &mut painted);
        }
        painted
    }
}

/// What one composition leaves on a target, by pixel.
type Composite = BTreeMap<(u32, u32), PredictedPaint>;

/// Pass one: the ring `element` declares an outline for, if it declares one.
fn paint_ring(element: &PredictedElement, painted: &mut Composite) {
    let Some(outline) = element.outline else {
        return;
    };
    for (x, y) in element.ring_pixels() {
        painted.insert((y, x), outline);
    }
}

/// Pass two: `element`'s own fill, over whatever a ring left there.
fn paint_fill(element: &PredictedElement, painted: &mut Composite) {
    for (x, y) in element.fill.into_iter().flat_map(pixels_of) {
        painted.insert((y, x), element.paint);
    }
}

impl PredictedElement {
    /// Every pixel between this element's footprint and its fill — the ring the
    /// outline covers.
    ///
    /// Stated as a set difference rather than as four strips: what a reading
    /// needs is the pixels, and one subtraction cannot disagree with itself
    /// about where a corner goes.
    fn ring_pixels(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.footprint
            .into_iter()
            .flat_map(pixels_of)
            .filter(|(x, y)| !self.fill.is_some_and(|fill| fill.holds(*x, *y)))
    }
}

/// One pixel a prediction covers, and what it predicts there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PredictedPixel {
    pub x: u32,
    pub y: u32,
    pub paint: PredictedPaint,
}

/// How a frame stands against a prediction, per pixel and with no area budget.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelReading {
    pub verdict: PixelVerdict,
    /// How many predicted pixels were actually judged.
    pub considered: u64,
    /// How many of those sit further than the per-pixel tolerance from the
    /// colour predicted for them. One is enough to be a mismatch.
    pub strayed: u64,
    /// How many predicted pixels carried no predictable colour.
    pub unpredictable: u64,
    /// How many predicted pixels the frame does not have.
    pub absent: u64,
    /// The predicted pixel that strayed furthest, for a reader of the failure.
    pub worst: Option<StrayedPixel>,
}

/// What a reading concluded.
///
/// Every way of not being able to look has a name of its own, so an assertion
/// against the good verdict rejects all of them rather than reading an empty
/// answer as good news.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelVerdict {
    /// Every pixel the prediction covers shows the colour predicted for it.
    EveryPredictedPixelShowsIt,
    /// At least one does not. There is no area budget: one is a mismatch.
    Strayed,
    /// The prediction covered no pixel at all.
    NothingWasPredicted,
    /// A predicted pixel carried no predictable colour, so it was judged
    /// against nothing.
    APredictedColourWasNotPredictable,
    /// A predicted pixel is outside the frame, so it was never compared.
    APredictedPixelWasNotInTheFrame,
}

/// A predicted pixel the frame disagrees with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrayedPixel {
    pub x: u32,
    pub y: u32,
    pub shown: [u8; 3],
    pub predicted: [u8; 3],
    pub delta_e: f64,
}

/// How `frame` stands at every pixel the elements named by `of` cover.
///
/// **Per pixel, with no area budget**: one pixel further than the harness's
/// per-pixel tolerance from its predicted colour is a mismatch, whatever share of
/// the frame that is. That is the whole difference from a golden comparison,
/// whose 0.01% budget at 1280 × 720 forgives 92 pixels — five times the base
/// crosshair's 17-pixel fill.
///
/// # Errors
///
/// Returns the distance metric's own failure.
pub fn per_pixel_reading(
    frame: &Rgba8Image,
    prediction: &Prediction,
    of: &[&str],
) -> Result<PixelReading, Box<dyn Error>> {
    let tolerance = Thresholds::default().per_pixel_delta_e();
    let mut reading = PixelReading {
        verdict: PixelVerdict::NothingWasPredicted,
        considered: 0,
        strayed: 0,
        unpredictable: 0,
        absent: 0,
        worst: None,
    };
    for predicted in prediction.painted(of) {
        judge(frame, predicted, tolerance, &mut reading)?;
    }
    reading.verdict = verdict_of(&reading);
    Ok(reading)
}

/// Judges one predicted pixel into `reading`.
fn judge(
    frame: &Rgba8Image,
    predicted: PredictedPixel,
    tolerance: f64,
    reading: &mut PixelReading,
) -> Result<(), Box<dyn Error>> {
    let PredictedPaint::Opaque(expected) = predicted.paint else {
        reading.unpredictable += 1;
        return Ok(());
    };
    let Some([red, green, blue, _]) = frame.pixel(predicted.x, predicted.y) else {
        reading.absent += 1;
        return Ok(());
    };
    reading.considered += 1;
    let shown = [red, green, blue];
    let delta_e = distance(shown, expected)?;
    if delta_e > tolerance {
        reading.strayed += 1;
        reading.worst = further_of(
            reading.worst,
            StrayedPixel {
                x: predicted.x,
                y: predicted.y,
                shown,
                predicted: expected,
                delta_e,
            },
        );
    }
    Ok(())
}

/// Whichever of the two strayed furthest, keeping the earlier one on a tie so
/// the report is the same on every run.
fn further_of(held: Option<StrayedPixel>, found: StrayedPixel) -> Option<StrayedPixel> {
    match held {
        Some(held) if held.delta_e >= found.delta_e => Some(held),
        _ => Some(found),
    }
}

/// What `reading` concluded, refusing every state that means "I could not look"
/// before it reports on what strayed.
fn verdict_of(reading: &PixelReading) -> PixelVerdict {
    if reading.considered + reading.unpredictable + reading.absent == 0 {
        PixelVerdict::NothingWasPredicted
    } else if reading.absent > 0 {
        PixelVerdict::APredictedPixelWasNotInTheFrame
    } else if reading.unpredictable > 0 {
        PixelVerdict::APredictedColourWasNotPredictable
    } else if reading.strayed > 0 {
        PixelVerdict::Strayed
    } else {
        PixelVerdict::EveryPredictedPixelShowsIt
    }
}

/// Where `element` lands on `target`, and what it paints there.
fn predict(element: &HudElement, target: SurfaceSize) -> PredictedElement {
    let scale = f64::from(target.height) / REFERENCE_HEIGHT;
    let placed = placed_at(element, target, scale);
    let thickness = element.outline.map_or(0, |_| extent(OUTLINE_UNITS, scale));
    PredictedElement {
        name: element.name.as_str().to_owned(),
        fill: clipped(placed, target),
        footprint: clipped(grown_by(placed, thickness), target),
        paint: painted_with(element),
        outline: element.outline.map(composite_of),
    }
}

/// Where `element` sits on `target` at `scale`: anchored, then displaced by the
/// offset it declares.
///
/// The footprint is this same placement grown by the outline's thickness, rather
/// than a second derivation that could disagree with it about where the element
/// is.
fn placed_at(element: &HudElement, target: SurfaceSize, scale: f64) -> Placed {
    let [declared_width, declared_height] = element.size;
    let [rightward, downward] = element.offset;
    let (across, down) = measured(element.anchor);
    let width = extent(declared_width, scale);
    let height = extent(declared_height, scale);
    Placed {
        left: start(across, target.width, width) + displaced(rightward, scale),
        top: start(down, target.height, height) + displaced(downward, scale),
        width,
        height,
    }
}

/// `placed` grown by `thickness` pixels on every side.
const fn grown_by(placed: Placed, thickness: i64) -> Placed {
    Placed {
        left: placed.left - thickness,
        top: placed.top - thickness,
        width: placed.width + 2 * thickness,
        height: placed.height + 2 * thickness,
    }
}

/// What `element`'s fill puts on the frame.
fn painted_with(element: &HudElement) -> PredictedPaint {
    match element.draw {
        Draw::Fill { color } => composite_of(color),
        Draw::BlockTexture { .. } => PredictedPaint::Unpredictable,
    }
}

/// The composite `color` makes over any background, where that is a question a
/// declaration answers on its own.
///
/// An opaque colour composites to itself: the blend is a no-op at `α = 255` and
/// the decode and re-encode around it are inverse operations. Anything else
/// depends on the scene behind it, which this prediction does not model.
fn composite_of(color: Rgba8) -> PredictedPaint {
    if color.a == u8::MAX {
        PredictedPaint::Opaque([color.r, color.g, color.b])
    } else {
        PredictedPaint::Unpredictable
    }
}

/// How an anchor is measured along one axis.
#[derive(Debug, Clone, Copy)]
enum Measured {
    /// The near edge sits on the safe-area box.
    NearEdgeOnTheBox,
    /// The extent is centred on the **target**, whatever the box is.
    CentredOnTheTarget,
    /// The far edge sits on the safe-area box.
    FarEdgeOnTheBox,
}

/// How `anchor` is measured horizontally and then vertically.
const fn measured(anchor: Anchor) -> (Measured, Measured) {
    use Measured::{CentredOnTheTarget, FarEdgeOnTheBox, NearEdgeOnTheBox};
    match anchor {
        Anchor::TopLeft => (NearEdgeOnTheBox, NearEdgeOnTheBox),
        Anchor::Top => (CentredOnTheTarget, NearEdgeOnTheBox),
        Anchor::TopRight => (FarEdgeOnTheBox, NearEdgeOnTheBox),
        Anchor::Left => (NearEdgeOnTheBox, CentredOnTheTarget),
        Anchor::Center => (CentredOnTheTarget, CentredOnTheTarget),
        Anchor::Right => (FarEdgeOnTheBox, CentredOnTheTarget),
        Anchor::BottomLeft => (NearEdgeOnTheBox, FarEdgeOnTheBox),
        Anchor::Bottom => (CentredOnTheTarget, FarEdgeOnTheBox),
        Anchor::BottomRight => (FarEdgeOnTheBox, FarEdgeOnTheBox),
    }
}

/// Where an extent of `extent` pixels starts on an axis of `span` pixels.
///
/// The inset is 5% of that axis's own extent — 64 across and 36 down at
/// 1280 × 720 — and a centred extent is centred on the span rather than on the
/// box.
fn start(measured: Measured, span: u32, extent: i64) -> i64 {
    let inset = whole(f64::from(span) * SAFE_AREA_FRACTION);
    match measured {
        Measured::NearEdgeOnTheBox => inset,
        Measured::CentredOnTheTarget => whole(f64::from(span) / 2.0 - extent as f64 / 2.0),
        Measured::FarEdgeOnTheBox => i64::from(span) - inset - extent,
    }
}

/// A declared extent in physical pixels, never scaled away to nothing.
fn extent(declared: u32, scale: f64) -> i64 {
    whole(f64::from(declared) * scale).max(1)
}

/// A declared displacement in physical pixels, `+x` right and `+y` down.
fn displaced(declared: i32, scale: f64) -> i64 {
    whole(f64::from(declared) * scale)
}

/// `value` at the nearest whole pixel, with a half **away from zero**.
fn whole(value: f64) -> i64 {
    value.round() as i64
}

/// Where a rectangle sits before anything is cut off it, in whole pixels that may
/// stand outside the target.
#[derive(Debug, Clone, Copy)]
struct Placed {
    left: i64,
    top: i64,
    width: i64,
    height: i64,
}

/// The part of `placed` that falls on `target`, or nothing where none of it does.
/// Intersected and never wrapped.
fn clipped(placed: Placed, target: SurfaceSize) -> Option<Rect> {
    let Placed {
        left,
        top,
        width,
        height,
    } = placed;
    let inside_left = left.max(0);
    let inside_top = top.max(0);
    let inside_right = left.saturating_add(width).min(i64::from(target.width));
    let inside_bottom = top.saturating_add(height).min(i64::from(target.height));
    if inside_right <= inside_left || inside_bottom <= inside_top {
        return None;
    }
    Some(Rect {
        x: u32::try_from(inside_left).ok()?,
        y: u32::try_from(inside_top).ok()?,
        width: u32::try_from(inside_right - inside_left).ok()?,
        height: u32::try_from(inside_bottom - inside_top).ok()?,
    })
}

/// Every pixel coordinate `rect` covers.
fn pixels_of(rect: Rect) -> impl Iterator<Item = (u32, u32)> {
    (rect.y..rect.y + rect.height)
        .flat_map(move |y| (rect.x..rect.x + rect.width).map(move |x| (x, y)))
}
