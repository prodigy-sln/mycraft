//! Where a declared rectangle lands, on targets from 640 × 360 to 5120 × 1440.
//!
//! **Every expected number below is derived by the rule the architecture pins
//! and written out in the test that uses it.** None is read back from a run of
//! [`compose`] and none is snapshotted from a first green: a count taken from
//! the code under test records whatever that code happened to do that day, and a
//! composition that emitted nothing would have `0` recorded as its expectation
//! and pass forever after.
//!
//! Target `W × H`, `scale = H / 720`, `round` half **away from zero**:
//!
//! - `w = max(1, round(size.x × scale))`, `h = max(1, round(size.y × scale))`
//! - `ox = round(offset.x × scale)`, `oy = round(offset.y × scale)`; `+x` right,
//!   `+y` down
//! - `inset_x = round(0.05 × W)`, `inset_y = round(0.05 × H)`, per axis from that
//!   axis's own extent — 64 and 36 at 1280 × 720
//! - `center` is centred on `(W/2, H/2)` and is not inset; every other anchor
//!   puts its named edges on the safe-area box, and a free axis centres on the
//!   **target** rather than on the box
//! - the origin from a centre `c` and an extent `e` is `left = round(c − e/2)`
//! - every rectangle is intersected with `0..W × 0..H` after offsetting
//!
//! The rectangles are integers, so every comparison here is exact. There is no
//! arithmetic path on which a correct composition lands a fraction of a pixel
//! from these numbers, and a tolerance would only hide the one-pixel errors the
//! rounding rule exists to pin.
//!
//! **The fixtures name nothing the base game ships.** `content/base/hud/` is
//! watched by a scan that forbids its element names and colours in Rust, and a
//! suite that borrowed one would be asserting against content it does not own as
//! well as writing a needle into a file under `src/`.
//!
//! **Every fixture is checked before it is composed.** [`frame_of`] refuses a
//! layout that did not register every declaration it was given, and [`only_rect`]
//! refuses a plan that does not hold the single rectangle the assertion beneath
//! it compares against — an empty plan satisfies a claim about what is *not* in
//! it, and it would satisfy a relative claim about a displacement too.

use std::error::Error;
use std::sync::Arc;

use mc_core::hud::source::InMemoryHudSource;
use mc_core::hud::{ANCHOR_NAMES, DeclaredValue, HudLayout, HudOrigin, RawHudElement, Rgba8};

use crate::surface::SurfaceSize;
use crate::texture::TextureLayers;

use super::{HudFrame, Painted, PaintedRect, compose};

type TestResult = Result<(), Box<dyn Error>>;

/// The colour every declaration below states.
const FILL_COLOR: &str = "#3366CCFF";

/// That same colour as the model holds it, written out rather than parsed.
///
/// `Rgba8::parse` is not public, and deriving this from the declaration through
/// the model would make the comparison a statement about the parser instead of
/// about the colour reaching the rectangle.
const FILL: Rgba8 = Rgba8 {
    r: 0x33,
    g: 0x66,
    b: 0xCC,
    a: 0xFF,
};

/// The reference target: at a height of 720 one UI unit is one pixel, which is
/// what makes the declared numbers and the expected pixels comparable by eye.
const REFERENCE: SurfaceSize = SurfaceSize {
    width: 1280,
    height: 720,
};

/// A thin bar at the crosshair's extents, centred and undisplaced.
const BAR: Declared = Declared {
    name: "fixture:bar",
    anchor: "center",
    size: [9, 1],
    offset: None,
};

/// A rectangle whose two extents differ, so a derivation that transposed the
/// axes lands somewhere else rather than on the same answer.
const PANEL: Declared = Declared {
    name: "fixture:panel",
    anchor: "bottom-right",
    size: [12, 8],
    offset: None,
};

/// A square anchored to the bottom edge, where one axis is named and the other
/// is free.
const SWATCH: Declared = Declared {
    name: "fixture:swatch",
    anchor: "bottom",
    size: [16, 16],
    offset: None,
};

/// A square that is moved from anchor to anchor, one composition each.
///
/// Its two extents are equal, so nothing about where it lands depends on which
/// extent reached which axis — what separates the nine placements is only the
/// anchor. Sixteen units keeps it clear of every edge of the reference target,
/// so no placement is cut and every one of the nine is compared whole.
const TILE: Declared = Declared {
    name: "fixture:tile",
    anchor: "center",
    size: [16, 16],
    offset: None,
};

/// A rectangle wide enough that a displacement can carry part of it off the
/// target while the rest stays on.
const WIDE_BAR: Declared = Declared {
    name: "fixture:wide-bar",
    anchor: "center",
    size: [20, 10],
    offset: None,
};

/// One declaration as this suite writes it: a filled rectangle, in the smallest
/// form the model accepts, plus an optional displacement.
#[derive(Debug, Clone, Copy)]
struct Declared {
    name: &'static str,
    anchor: &'static str,
    size: [i64; 2],
    offset: Option<[i64; 2]>,
}

impl Declared {
    /// The same declaration, displaced by `offset` UI units.
    fn displaced_by(self, offset: [i64; 2]) -> Self {
        Self {
            offset: Some(offset),
            ..self
        }
    }

    /// The same declaration, measured from `anchor`.
    fn anchored_at(self, anchor: &'static str) -> Self {
        Self { anchor, ..self }
    }

    /// This declaration in the form a source hands over.
    fn raw(self) -> RawHudElement {
        let mut fields = vec![
            ("name".to_owned(), text(self.name)),
            ("anchor".to_owned(), text(self.anchor)),
            ("size".to_owned(), pair(self.size)),
            ("draw".to_owned(), text("fill")),
            ("color".to_owned(), text(FILL_COLOR)),
        ];
        if let Some(offset) = self.offset {
            fields.push(("offset".to_owned(), pair(offset)));
        }
        RawHudElement::new(fields)
    }
}

fn text(spelled: &str) -> DeclaredValue {
    DeclaredValue::Text(spelled.to_owned())
}

fn pair(stated: [i64; 2]) -> DeclaredValue {
    let [across, down] = stated;
    DeclaredValue::List(vec![
        DeclaredValue::Integer(across),
        DeclaredValue::Integer(down),
    ])
}

/// A frame whose layout holds exactly `declarations`, in the order given.
///
/// # Errors
///
/// Fails if the layout refused a declaration or registered a different number of
/// them: a plan that covers nothing says nothing about a target when the layout
/// it came from was empty to begin with.
fn frame_of(declarations: &[Declared]) -> Result<HudFrame, Box<dyn Error>> {
    let stated = declarations
        .iter()
        .map(|declared| (HudOrigin::new(declared.name), declared.raw()))
        .collect();
    let layout = HudLayout::load(&InMemoryHudSource::new(
        HudOrigin::new("this suite"),
        stated,
    ))?;
    if layout.elements().len() != declarations.len() {
        return Err(format!(
            "this fixture has to register all {} of its declarations, or what is composed from it \
             is not what it states, but it registered {}",
            declarations.len(),
            layout.elements().len()
        )
        .into());
    }
    Ok(HudFrame {
        layout: Arc::new(layout),
        held: None,
    })
}

/// What `frame` covers on `target`.
///
/// No texture layer is resolved: every declaration in this suite is a fill, and
/// a textured swatch is a later phase's.
fn plan(frame: &HudFrame, target: SurfaceSize) -> Vec<PaintedRect> {
    compose(frame, target, &TextureLayers::default())
}

/// The one rectangle a plan holds.
///
/// # Errors
///
/// Fails if the plan holds any other number of rectangles, because a comparison
/// against a rectangle that was never placed asserts nothing.
fn only_rect(composed: &[PaintedRect]) -> Result<PaintedRect, Box<dyn Error>> {
    match composed {
        [rect] => Ok(*rect),
        _ => Err(format!(
            "this composition has to place exactly one rectangle, or the assertion below is \
             vacuous, but it placed {}",
            composed.len()
        )
        .into()),
    }
}

/// A rectangle painted in the colour every declaration below states.
const fn filled(x: i32, y: i32, width: u32, height: u32) -> PaintedRect {
    PaintedRect {
        x,
        y,
        width,
        height,
        paint: Painted::Fill(FILL),
    }
}

/// `scale = 720/720 = 1`, so the extents are the declared 9 × 1. The centre is
/// `(640, 360)`, and `left = round(640 − 4.5) = 636`, `top = round(360 − 0.5) =
/// 360` — a 9-wide span from 636 to 644 whose middle pixel is 640.
#[test]
fn a_centred_bar_covers_its_declared_extents_at_the_reference_height() -> TestResult {
    let frame = frame_of(&[BAR])?;

    let composed = plan(&frame, REFERENCE);

    assert_eq!(
        composed,
        vec![filled(636, 360, 9, 1)],
        "at a target height of 720 one UI unit is one pixel, so a 9 × 1 declaration covers 9 × 1 \
         pixels centred on the target"
    );
    Ok(())
}

/// `scale = 1440/720 = 2`, so `9 × 1` becomes `18 × 2`. The centre is
/// `(2560, 720)`, and `left = round(2560 − 9) = 2551`, `top = round(720 − 1) =
/// 719`.
#[test]
fn a_target_of_twice_the_reference_height_doubles_the_declared_extents() -> TestResult {
    let frame = frame_of(&[BAR])?;

    let composed = plan(
        &frame,
        SurfaceSize {
            width: 5120,
            height: 1440,
        },
    );

    assert_eq!(
        composed,
        vec![filled(2551, 719, 18, 2)],
        "one UI unit is two pixels at twice the reference height, and the width scales with the \
         height rather than with the width — this target is four times as wide as it is at 1280 × \
         720, and the bar is twice as wide"
    );
    Ok(())
}

/// `scale = 1080/720 = 1.5`, so both extents land on a half pixel and both round
/// away from zero: `round(13.5) = 14` and `round(1.5) = 2`, where a truncation
/// gives 13 and 1. The centre is `(960, 540)`, `left = round(960 − 7) = 953`,
/// `top = round(540 − 1) = 539`.
#[test]
fn extents_landing_on_a_half_pixel_round_away_from_zero() -> TestResult {
    let frame = frame_of(&[BAR])?;

    let composed = plan(
        &frame,
        SurfaceSize {
            width: 1920,
            height: 1080,
        },
    );

    assert_eq!(
        composed,
        vec![filled(953, 539, 14, 2)],
        "9 × 1.5 is 13.5 and 1 × 1.5 is 1.5; rounding half away from zero makes those 14 and 2, \
         where truncating makes them 13 and 1"
    );
    Ok(())
}

/// `scale = 360/720 = 0.5`, so the declared height of 1 scales to 0.5 — which
/// `round` carries to 1 and a truncation carries to 0. An element that covers no
/// pixel at all is the failure this pins; a floor of one pixel per axis is what
/// stops it.
#[test]
fn a_bar_thinner_than_a_pixel_still_covers_one_pixel_of_height() -> TestResult {
    let frame = frame_of(&[BAR])?;

    let composed = plan(
        &frame,
        SurfaceSize {
            width: 640,
            height: 360,
        },
    );

    let heights: Vec<u32> = composed.iter().map(|rect| rect.height).collect();
    assert_eq!(
        heights,
        vec![1],
        "half of one UI unit still has to reach a pixel, so this covers one rectangle one pixel \
         high rather than covering nothing"
    );
    Ok(())
}

/// At the reference height one UI unit is one pixel, so `offset = [40, -20]`
/// displaces by exactly 40 and −20 pixels, `+y` being down.
///
/// Asserted against the same element's undisplaced rectangle rather than against
/// a second absolute position: that is what makes this a claim about the
/// displacement instead of a second copy of the centring claim.
#[test]
fn a_displaced_element_lands_that_far_from_where_it_lands_undisplaced() -> TestResult {
    let undisplaced = only_rect(&plan(&frame_of(&[BAR])?, REFERENCE))?;

    let displaced = only_rect(&plan(&frame_of(&[BAR.displaced_by([40, -20])])?, REFERENCE))?;

    assert_eq!(
        displaced,
        PaintedRect {
            x: undisplaced.x + 40,
            y: undisplaced.y - 20,
            ..undisplaced
        },
        "`+x` is right and `+y` is down, so this element sits 40 pixels right of and 20 pixels \
         above the rectangle it covers with no displacement, at the same extents"
    );
    Ok(())
}

/// `inset_x = round(0.05 × 1280) = 64` and `inset_y = round(0.05 × 720) = 36`,
/// per axis from that axis's own extent. The safe-area box's right edge is
/// therefore at `1280 − 64 = 1216` and its bottom edge at `720 − 36 = 684`, so a
/// 12 × 8 rectangle whose right edge is 1216 starts at 1204 and whose bottom
/// edge is 684 starts at 676.
#[test]
fn a_bottom_right_element_sits_on_the_safe_area_inset_of_each_axis() -> TestResult {
    let frame = frame_of(&[PANEL])?;

    let composed = plan(&frame, REFERENCE);

    assert_eq!(
        composed,
        vec![filled(1204, 676, 12, 8)],
        "the inset is 5% of each axis separately, which is 64 horizontally and 36 vertically here \
         — a single inset taken from one extent would put one of the two edges somewhere else"
    );
    Ok(())
}

/// The named edge sits on the safe-area box — `720 − 36 = 684`, so a 16-high
/// rectangle starts at 668. The free axis centres on the **target**:
/// `left = round(640 − 8) = 632`. At 1280 × 720 the inset is symmetric, so
/// centring on the box would land on the same pixel; the choice is pinned here
/// because the frame-level prediction derives it independently and the two have
/// to agree on a target where it is observable.
#[test]
fn a_bottom_anchored_element_centres_on_the_target_and_sits_on_the_bottom_inset() -> TestResult {
    let frame = frame_of(&[SWATCH])?;

    let composed = plan(&frame, REFERENCE);

    assert_eq!(
        composed,
        vec![filled(632, 668, 16, 16)],
        "the bottom edge is 36 pixels from the target's bottom edge and the free horizontal axis \
         is centred on the target, putting the 16-wide span's middle on 640"
    );
    Ok(())
}

/// Where each of the nine anchors puts [`TILE`] on the reference target,
/// **derived from what the anchor names and never read back from a plan**.
///
/// On 1280 × 720 the scale is 1, so a 16 × 16 declaration covers 16 × 16 pixels,
/// and the safe-area inset is `round(0.05 × 1280) = 64` horizontally against
/// `round(0.05 × 720) = 36` vertically — per axis, from that axis's own extent.
/// That leaves three possible starts per axis, and the nine placements below are
/// their nine combinations:
///
/// | measured | horizontally | vertically |
/// |---|---|---|
/// | at the near edge of the box | `64` | `36` |
/// | centred on the target | `round(640 − 8) = 632` | `round(360 − 8) = 352` |
/// | at the far edge of the box | `1280 − 64 − 16 = 1200` | `720 − 36 − 16 = 668` |
///
/// `center` is the one anchor that is not inset at all; it is centred on the
/// target on both axes, which is the same arithmetic as a free axis and so the
/// same 632 and 352.
///
/// The two axes share no value among those six, so an anchor handed the other
/// axis's treatment always lands somewhere else rather than back on the same
/// pixel — which is what makes a square legitimate here despite a square hiding
/// a transposition of its own extents.
///
/// Listed in the order the model itself lists the anchors, so the test below can
/// hold this table to that list.
const ANCHOR_PLACEMENTS: [(&str, PaintedRect); 9] = [
    ("top-left", filled(64, 36, 16, 16)),
    ("top", filled(632, 36, 16, 16)),
    ("top-right", filled(1200, 36, 16, 16)),
    ("left", filled(64, 352, 16, 16)),
    ("center", filled(632, 352, 16, 16)),
    ("right", filled(1200, 352, 16, 16)),
    ("bottom-left", filled(64, 668, 16, 16)),
    ("bottom", filled(632, 668, 16, 16)),
    ("bottom-right", filled(1200, 668, 16, 16)),
];

/// The whole anchor table, in one test, because the failure it pins is the
/// **table** being wrong rather than any one row of it.
///
/// The anchor vocabulary is everything a content author is given to say where an
/// element goes, and the placements above name three of the nine — a centred
/// one, `bottom-right` and `bottom`. Six spellings can therefore be given each
/// other's axis treatment *all at once* and nothing above notices: the loading
/// suite pins that the nine parse and are listed in a refusal, which is a
/// different claim from where any of them puts an element. **A reader who counts
/// nine assertions here should not split this into nine tests**, and should not
/// delete it as covered by the two anchored placements above: one row of a
/// corrupt table can be right while the table is wrong, and a test per row would
/// still not say that the nine are distinct.
#[test]
fn every_anchor_places_a_square_where_that_anchor_names() -> TestResult {
    let listed: Vec<&str> = ANCHOR_PLACEMENTS
        .iter()
        .map(|(spelled, _)| *spelled)
        .collect();
    if listed.as_slice() != ANCHOR_NAMES.as_slice() {
        return Err(format!(
            "this table has to name every anchor a declaration may state, in the order the model \
             lists them, or the ones it leaves out are graded by nothing at all — it names \
             {listed:?} against {ANCHOR_NAMES:?}"
        )
        .into());
    }

    let mut covered = Vec::with_capacity(ANCHOR_NAMES.len());
    for spelled in ANCHOR_NAMES {
        let frame = frame_of(&[TILE.anchored_at(spelled)])?;
        covered.push((spelled, only_rect(&plan(&frame, REFERENCE))?));
    }

    assert_eq!(
        covered,
        ANCHOR_PLACEMENTS.to_vec(),
        "each anchor puts its named edges on the safe-area box, inset 64 horizontally and 36 \
         vertically, and centres its free axis on the target; `center` is centred on both axes and \
         is not inset"
    );
    Ok(())
}

/// `scale = 1`, so the extents are 20 × 10, the centre `(640, 360)` puts the
/// undisplaced origin at `(630, 355)`, and `+632` carries the left edge to 1262.
/// The unclipped right edge is at 1282, two pixels past a target 1280 wide;
/// intersecting with `0..1280` leaves 18 of the 20 columns.
///
/// The whole plan is compared, which is what states both halves at once: a
/// rectangle reaching past 1280, a second rectangle at the opposite edge holding
/// the two lost columns, and a modulo that placed the whole 20 somewhere in the
/// middle all differ from the single rectangle named here.
#[test]
fn an_element_displaced_past_the_target_is_cut_at_the_edge_rather_than_wrapped() -> TestResult {
    let frame = frame_of(&[WIDE_BAR.displaced_by([632, 0])])?;

    let composed = plan(&frame, REFERENCE);

    assert_eq!(
        composed,
        vec![filled(1262, 355, 18, 10)],
        "the two columns that fell off the right edge are dropped: nothing is painted outside the \
         target and nothing reappears at the opposite edge"
    );
    Ok(())
}

/// A target of no height has no scale to derive — `H / 720` is 0 and every
/// extent would collapse — and a minimised window reports exactly that. The
/// answer is an empty plan, and it is not a failure: the return type carries no
/// error, so "reports no error" is structural rather than asserted.
///
/// Two elements are declared, so an empty plan is a fact about the target rather
/// than about the layout.
#[test]
fn a_target_with_no_height_composes_nothing() -> TestResult {
    let frame = frame_of(&[BAR, PANEL])?;

    let composed = plan(
        &frame,
        SurfaceSize {
            width: 1280,
            height: 0,
        },
    );

    assert_eq!(
        composed,
        Vec::new(),
        "a target with no pixels in it composes no rectangle at all, rather than scaling the two \
         declared elements by a height of zero"
    );
    Ok(())
}
