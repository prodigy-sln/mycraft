//! The HUD pass in pixels: what a declared element covers, what it composites
//! to, and what it leaves alone.
//!
//! # Every expected byte is derived, and one of them is not the one the hex
//! digits suggest
//!
//! The colour target is `Rgba8UnormSrgb`, so the blend hardware **decodes the
//! destination to linear, blends, and re-encodes on write**. A declared colour
//! is decoded on the CPU and handed to the shader as linear light; alpha is not
//! a colour and passes through undecoded. `#FFFFFF80` over `#000000FF` is
//! therefore `α·1.0 + (1−α)·0.0` at `α = 128/255 = 0.501960…`, which is linear
//! `0.501960…`, which sRGB-encodes as `1.055 · L^(1/2.4) − 0.055 = 0.7366469…`
//! → `187.845` → byte **188**, spelled `#BCBCBC`. Not the 128 the digits read
//! like. The arithmetic is written out beside the constant that carries it.
//!
//! If a blend scenario here goes red where 188 is due, the renderer is wrong,
//! and 128 is what a blend performed in gamma space produces — which on this
//! target means aliasing the view to a non-sRGB format, the change
//! `docs/technical/rendering.md` pins against — invisible once shipped,
//! plausible-looking, and wrong in the same direction everywhere. **No expected
//! value below moves**, and none is read back from a frame or from a green run.
//!
//! **What those two scenarios do *not* grade.** White is a fixed point of the
//! sRGB transfer function: `srgb8_to_linear([255, 255, 255])` is `1.0`, which is
//! what `255 / 255` is too, so a renderer that never decoded a declared colour
//! on the CPU lands on 188 there anyway. That defect is reachable only from a
//! **mid-tone**, which is the one and only job of [`MID_TONE`] below.
//!
//! # Backdrops are chosen so an assertion can fail
//!
//! A black ring on a black backdrop is indistinguishable from no ring, and a red
//! element over red terrain differs from nothing. So the outline scenarios
//! compose over a backdrop that is neither of the colours they assert, the
//! footprint scenarios compose over real terrain, and each test states the
//! property its backdrop has to have before it asserts anything.
//!
//! # Where the fixtures are
//!
//! The declarations, the terrain fixture and the two ways a frame is captured
//! live in `support/hud.rs`; the instruments that read a frame live in
//! `support/frame.rs`. What stays here is the derived geometry each scenario
//! asserts against, beside the scenario that asserts it.
//!
//! # The fixtures name nothing the base game ships
//!
//! Element names use a `fixture:` namespace. The colour literals `#FFFFFFFF`,
//! `#000000FF` and `#808080FF` are the ones these scenarios name and the ones a
//! later phase's scan forbids in production Rust — that scan's roots are
//! `crates/**/src`, and this file is outside them, which is why it is safe here
//! and would not be safe one directory over.
//!
//! # File-name order is asserted by an earlier phase, not by this one
//!
//! `HudLayout::load` preserves the order its source hands over; turning file
//! names into that order belongs to the reader and is graded where the reader
//! is. The two-element fixtures below spell their origins `earlier.toml` and
//! `later.toml` and list them in that order, so what these tests grade is which
//! of two overlapping elements wins — not the sort that decided which is which.

mod support;

use std::error::Error;

use mc_render::color::CLEAR_COLOR_SRGB;
use mc_testkit::frame::Rgba8Image;

use support::TestResult;
use support::frame::{Rect, Strays, compare_frames, require, strays_from};
use support::hud::{Declared, compose_over, hud_frame, render_frame, terrain_alone, wall_scene};

/// How far a channel may sit from its derived value: one unit, for the target's
/// own encode and nothing else.
const ONE_STEP: u8 = 1;

/// Byte equality, for a colour written opaquely with no arithmetic in the way.
const EXACT: u8 = 0;

/// How far a pixel may sit from the declared sky and still be sky, in ΔE.
const SAME_COLOR: f64 = 4.0;

/// How much of the terrain fixture's frame has to be something other than sky.
///
/// A quarter of `1280 × 720 = 921_600`, which is `230_400` — the threshold, and
/// the only number here anybody has to trust. The wall below was measured at
/// `652_320` non-sky pixels, seven tenths of the frame, so the bar reports a
/// fixture that stopped showing terrain rather than pinning a coverage figure
/// read off a run.
const TERRAIN_PIXELS: u64 = 230_400;

/// The two origins a two-element fixture attributes its declarations to.
const EARLIER: &str = "earlier.toml";
const LATER: &str = "later.toml";

/// The backdrop the composition scenarios clear to.
///
/// Neither black nor white nor either of the two fills asserted below, so an
/// element that painted nothing, an outline that bled, and a fill that landed in
/// the wrong pass all show up as this colour where another was due.
const BACKDROP: [u8; 3] = [60, 140, 90];

const BLACK: [u8; 3] = [0, 0, 0];
const RED: [u8; 3] = [255, 0, 0];
const BLUE: [u8; 3] = [0, 0, 255];

/// `#FFFFFF80` over `#000000FF`, derived and not intuited.
///
/// `α = 128/255 = 0.5019607…`; the source `#FFFFFF` decodes to linear 1.0 and
/// the destination `#000000` to linear 0.0; the blend is `α·1.0 + (1−α)·0.0 =
/// 0.5019607…`, which the target re-encodes as `1.055 · 0.5019607^(1/2.4) −
/// 0.055 = 0.7366469…`, `× 255 = 187.845`, which the unorm write rounds to 188.
const HALF_WHITE_ON_BLACK: [u8; 3] = [188, 188, 188];

/// `#808080FF` over black, derived and not intuited — and **not a tautology**.
///
/// It is the one expected value in this file equal to the byte its element
/// declared, and 128 comes back only because two inverse operations both ran. The CPU decodes
/// `128/255 = 0.5019607…` to linear `((0.5019607 + 0.055)/1.055)^2.4 =
/// 0.2158605…`; `α = 1` leaves that untouched, the destination dropping out of
/// `α·src + (1−α)·dst`; the target encodes it back on write, `1.055 ·
/// 0.2158605^(1/2.4) − 0.055 = 0.5019607…`, which is `128.000` of 255. Drop
/// either operation and the identity breaks: skipping the CPU decode hands the
/// shader `0.5019607` itself, encoding to `0.736646…` → `187.84` → byte **188**,
/// sixty away and no tolerance argument. **So reading the assertion below as
/// vacuous and deleting it reopens a hole nothing else here covers** — 0 and 255
/// are fixed points of the transfer function, every other colour this file
/// declares is built from those two, and the decode was measured deletable with
/// all 87 tests of this spec green. Mind the collision: 188 is also
/// [`HALF_WHITE_ON_BLACK`], the *correct* value one test up.
const MID_TONE_ROUND_TRIP: [u8; 3] = [128, 128, 128];

/// A filled rectangle over the crosshair's own extents, outlined — the shape
/// the outline scenarios are stated about.
const BAR: Declared = Declared {
    name: "fixture:bar",
    size: [9, 1],
    color: "#FFFFFFFF",
    outline: Some("#000000FF"),
};

/// `scale = 1`, so the extents are 9 × 1. `left = round(640 − 4.5) = 636` and
/// `top = round(360 − 0.5) = 360`, half away from zero.
const BAR_FILL: Rect = Rect {
    x: 636,
    y: 360,
    width: 9,
    height: 1,
};

/// How many pixels the one-unit ring around [`BAR_FILL`] covers:
/// `11 × 3 − 9 × 1 = 24`.
const BAR_RING_PIXELS: u64 = 24;

/// An opaque element large enough that "every pixel of its footprint" is a
/// claim about hundreds of them.
const MARKER: Declared = Declared {
    name: "fixture:marker",
    size: [24, 16],
    color: "#FF0000FF",
    outline: None,
};

/// The same extents at half alpha, so a composite is measured over a backdrop
/// the test cleared itself.
const VEIL: Declared = Declared {
    name: "fixture:veil",
    size: [24, 16],
    color: "#FFFFFF80",
    outline: None,
};

/// A mid-tone at full alpha, in [`MARKER`]'s and [`VEIL`]'s extents. Opaque so
/// that `α = 1` makes the blend a no-op and the decode is the only thing left
/// under test — any other alpha would grade two things at once. `0x80` because
/// it is the one RGB byte this spec declares that the transfer function moves.
const MID_TONE: Declared = Declared {
    name: "fixture:mid-tone",
    size: [24, 16],
    color: "#808080FF",
    outline: None,
};

/// `left = round(640 − 12) = 628`, `top = round(360 − 8) = 352`.
///
/// [`MARKER`], [`VEIL`] and [`MID_TONE`] all cover it: they differ only in
/// colour, and none declares an outline, so each one's *footprint* — the fill
/// rectangle, expanded by one unit per side only when an outline is declared —
/// is exactly this rectangle.
const WIDE_RECT: Rect = Rect {
    x: 628,
    y: 352,
    width: 24,
    height: 16,
};

/// The opaque black an earlier declaration lays down for a later one to
/// composite against.
const PLATE: Declared = Declared {
    name: "fixture:plate",
    size: [40, 40],
    color: "#000000FF",
    outline: None,
};

/// An opaque blue in the same extents, for the two ordering scenarios.
const PANEL: Declared = Declared {
    name: "fixture:panel",
    size: [40, 40],
    color: "#0000FFFF",
    outline: None,
};

/// `left = round(640 − 20) = 620`, `top = round(360 − 20) = 340`. Covered by
/// both [`PLATE`] and [`PANEL`].
const LARGE_RECT: Rect = Rect {
    x: 620,
    y: 340,
    width: 40,
    height: 40,
};

/// A strip of [`LARGE_RECT`] that no smaller element below reaches, which is
/// where a test asks whether the earlier declaration painted at all.
const LARGE_RECT_EXPOSED: Rect = Rect {
    x: 620,
    y: 340,
    width: 40,
    height: 10,
};

/// Half-alpha white, small enough to sit wholly inside [`LARGE_RECT`].
const SMALL_VEIL: Declared = Declared {
    name: "fixture:small-veil",
    size: [20, 20],
    color: "#FFFFFF80",
    outline: None,
};

/// Opaque red in the same extents.
const BADGE: Declared = Declared {
    name: "fixture:badge",
    size: [20, 20],
    color: "#FF0000FF",
    outline: None,
};

/// The same red, outlined — the element whose *outline* has to land under an
/// earlier element's fill.
const OUTLINED_BADGE: Declared = Declared {
    name: "fixture:outlined-badge",
    size: [20, 20],
    color: "#FF0000FF",
    outline: Some("#000000FF"),
};

/// `left = round(640 − 10) = 630`, `top = round(360 − 10) = 350`. Covered by
/// [`SMALL_VEIL`], [`BADGE`] and [`OUTLINED_BADGE`] alike.
///
/// Grown by one it runs `629..651 × 349..371`, which sits wholly inside
/// [`LARGE_RECT`]'s `620..660 × 340..380` — that containment is what puts an
/// outline pixel inside an earlier element's fill rectangle.
const SMALL_RECT: Rect = Rect {
    x: 630,
    y: 350,
    width: 20,
    height: 20,
};

/// How many pixels the one-unit ring around [`SMALL_RECT`] covers:
/// `22 × 22 − 20 × 20 = 84`.
const SMALL_RING_PIXELS: u64 = 84;

#[test]
fn a_frame_declaring_no_elements_is_the_frame_the_terrain_pass_alone_draws() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = wall_scene()?;
    let bare = hud_frame(&[])?;

    let through_the_frame = render_frame(&context, &fixture, &bare, "hud-zero-element")?;
    let through_terrain_alone = terrain_alone(&context, &fixture, "hud-terrain-alone")?;

    let terrain = support::pixels_away_from(&through_terrain_alone, CLEAR_COLOR_SRGB, SAME_COLOR)?;
    require(
        terrain >= TERRAIN_PIXELS,
        format!(
            "this scene has to put real terrain in view, or two pictures of empty sky agree for a \
             reason that has nothing to do with the HUD: {terrain} of the frame's pixels are \
             something other than the declared sky, against the {TERRAIN_PIXELS} required"
        ),
    )?;

    let seen = compare_frames(&through_the_frame, &through_terrain_alone, |_, _| true);
    assert_eq!(
        (seen.different, seen.first_different),
        (0, None),
        "a HUD holding no elements writes no pixel, so this frame is the one the terrain pass \
         draws with no HUD stage at all — over {} pixels compared",
        seen.considered
    );
    Ok(())
}

#[test]
fn a_declared_element_moves_every_pixel_of_the_footprint_it_covers() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = wall_scene()?;
    let bare = render_frame(&context, &fixture, &hud_frame(&[])?, "hud-footprint-bare")?;
    let declared = hud_frame(&[(EARLIER, MARKER)])?;

    let marked = render_frame(&context, &fixture, &declared, "hud-footprint-marked")?;

    let already = strays_from(&bare, |x, y| WIDE_RECT.holds(x, y), RED, EXACT);
    require(
        already.count == already.considered && already.considered == WIDE_RECT.area(),
        format!(
            "no pixel under the footprint may already be the colour the element declares, or a \
             renderer that drew nothing would satisfy this: {} of {} already carry it",
            already.considered - already.count,
            already.considered
        ),
    )?;

    let seen = compare_frames(&bare, &marked, |x, y| WIDE_RECT.holds(x, y));
    assert_eq!(
        (seen.considered, seen.same),
        (WIDE_RECT.area(), 0),
        "every pixel of the declared element's footprint differs from the frame that declared \
         nothing — a footprint drawn short leaves some of them as they were"
    );
    Ok(())
}

#[test]
fn a_declared_element_leaves_every_pixel_outside_its_footprint_as_it_found_it() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = wall_scene()?;
    let bare = render_frame(&context, &fixture, &hud_frame(&[])?, "hud-outside-bare")?;
    let declared = hud_frame(&[(EARLIER, MARKER)])?;

    let marked = render_frame(&context, &fixture, &declared, "hud-outside-marked")?;

    let inside = compare_frames(&bare, &marked, |x, y| WIDE_RECT.holds(x, y));
    require(
        inside.different > 0,
        format!(
            "the element has to have painted something, or this frame is the zero-element frame \
             under another name and everything outside the footprint matches for free: {} of {} \
             footprint pixels moved",
            inside.different, inside.considered
        ),
    )?;

    let seen = compare_frames(&bare, &marked, |x, y| !WIDE_RECT.holds(x, y));
    assert_eq!(
        (seen.different, seen.first_different),
        (0, None),
        "a pass that repaints beyond what content declared is what this reports — {} pixels \
         outside the footprint were compared",
        seen.considered
    );
    Ok(())
}

#[test]
fn a_half_transparent_white_over_black_shows_the_linear_composite_encoded_back() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let declared = hud_frame(&[(EARLIER, VEIL)])?;

    let composed = compose_over(&context, &declared, BLACK, "hud-alpha-over-black")?;

    let backdrop = strays_from(&composed, |x, y| !WIDE_RECT.holds(x, y), BLACK, ONE_STEP);
    require(
        backdrop.count == 0,
        format!(
            "the pixels this composite is stated over have to be black before it runs, or the \
             derived value below is not what any of them composites to: {} of {} are not",
            backdrop.count, backdrop.considered
        ),
    )?;

    let seen = reads(&composed, WIDE_RECT, HALF_WHITE_ON_BLACK);
    assert_eq!(
        (seen.considered, seen.count, seen.first),
        (WIDE_RECT.area(), 0, None),
        "half-alpha white over black composites in linear light and is encoded back by the \
         target, which is 188 per channel and not the 128 the hex digits read like"
    );
    Ok(())
}

#[test]
fn an_opaque_mid_tone_shows_the_byte_it_declared_after_the_decode_and_re_encode() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let declared = hud_frame(&[(EARLIER, MID_TONE)])?;

    let composed = compose_over(&context, &declared, BLACK, "hud-mid-tone-over-black")?;

    let backdrop = strays_from(&composed, |x, y| !WIDE_RECT.holds(x, y), BLACK, ONE_STEP);
    require(
        backdrop.count == 0,
        format!(
            "the pixels this element is composed over have to be black first, or the rectangle \
             below is not stated over the zero-element value the scenario names: {} of {} are not",
            backdrop.count, backdrop.considered
        ),
    )?;

    let seen = reads(&composed, WIDE_RECT, MID_TONE_ROUND_TRIP);
    assert_eq!(
        (seen.considered, seen.count, seen.first),
        (WIDE_RECT.area(), 0, None),
        "an opaque declaration makes the round trip decode → blend → encode and comes back to the \
         byte it declared — two inverse operations rather than a tautology, and a renderer that \
         dropped the CPU-side decode reads 188 over this whole rectangle"
    );
    Ok(())
}

#[test]
fn a_half_transparent_element_over_an_earlier_opaque_one_shows_their_composite() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let declared = hud_frame(&[(EARLIER, PLATE), (LATER, SMALL_VEIL)])?;
    backdrop_unlike(
        BLACK,
        "the earlier element's own colour, or the composite below holds \
                            whether or not that element painted",
    )?;

    let composed = compose_over(&context, &declared, BACKDROP, "hud-alpha-over-earlier")?;

    let under = reads(&composed, LARGE_RECT_EXPOSED, BLACK);
    require(
        under.count == 0,
        format!(
            "the earlier element has to have painted, or what the later one composites against is \
             the backdrop: {} of {} exposed pixels are not its colour",
            under.count, under.considered
        ),
    )?;

    let seen = reads(&composed, SMALL_RECT, HALF_WHITE_ON_BLACK);
    assert_eq!(
        (seen.considered, seen.count, seen.first),
        (SMALL_RECT.area(), 0, None),
        "the later element composites against what the earlier one left, in linear light, and the \
         target encodes the result back"
    );
    Ok(())
}

#[test]
fn an_outlined_bar_is_ringed_by_the_outline_colour_it_declares() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let declared = hud_frame(&[(EARLIER, BAR)])?;
    backdrop_unlike(
        BLACK,
        "the outline's own colour, or a ring that was parsed and discarded \
                            reads the same as one that was drawn",
    )?;

    let composed = compose_over(&context, &declared, BACKDROP, "hud-outline-ring")?;

    let seen = strays_from(&composed, |x, y| ring_of(BAR_FILL, x, y), BLACK, ONE_STEP);
    assert_eq!(
        (seen.considered, seen.count, seen.first),
        (BAR_RING_PIXELS, 0, None),
        "the outline is the one-pixel border immediately surrounding the 9 × 1 fill — a ring, not \
         a solid rectangle underneath it"
    );
    Ok(())
}

#[test]
fn an_outlined_bar_leaves_every_pixel_beyond_its_ring_as_it_found_it() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let bare = compose_over(&context, &hud_frame(&[])?, BACKDROP, "hud-ring-bare")?;
    let declared = hud_frame(&[(EARLIER, BAR)])?;

    let outlined = compose_over(&context, &declared, BACKDROP, "hud-ring-outlined")?;

    let within = compare_frames(&bare, &outlined, |x, y| BAR_FILL.grown_by(1).holds(x, y));
    require(
        within.different > 0,
        format!(
            "the element has to have painted inside its own ring, or nothing beyond it moved for \
             the trivial reason: {} of {} moved",
            within.different, within.considered
        ),
    )?;

    let seen = compare_frames(&bare, &outlined, |x, y| !BAR_FILL.grown_by(1).holds(x, y));
    assert_eq!(
        (seen.different, seen.first_different),
        (0, None),
        "every pixel further than one from the fill rectangle is the one the zero-element frame \
         shows: a ring that bled is what this reports, over {} pixels",
        seen.considered
    );
    Ok(())
}

#[test]
fn where_two_fills_overlap_the_later_declaration_is_the_one_that_shows() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let declared = hud_frame(&[(EARLIER, PANEL), (LATER, BADGE)])?;

    let composed = compose_over(&context, &declared, BACKDROP, "hud-same-pass-order")?;

    let under = strays_from(
        &composed,
        |x, y| LARGE_RECT.holds(x, y) && !SMALL_RECT.holds(x, y),
        BLUE,
        ONE_STEP,
    );
    require(
        under.count == 0 && under.considered > 0,
        format!(
            "the earlier element has to show outside the overlap, or the overlap is a pixel nobody \
             contested: {} of {} are not its colour",
            under.count, under.considered
        ),
    )?;

    let seen = reads(&composed, SMALL_RECT, RED);
    assert_eq!(
        (seen.considered, seen.count, seen.first),
        (SMALL_RECT.area(), 0, None),
        "two fills reaching the same pixel in one pass resolve in the order their declarations \
         were handed over, so the later one is what is left there"
    );
    Ok(())
}

#[test]
fn a_later_outline_does_not_cut_through_an_earlier_fill() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let declared = hud_frame(&[(EARLIER, PANEL), (LATER, OUTLINED_BADGE)])?;

    let composed = compose_over(&context, &declared, BACKDROP, "hud-outline-under-fill")?;

    let later = reads(&composed, SMALL_RECT, RED);
    require(
        later.count == 0,
        format!(
            "the later element has to have painted its own fill, or its outline pass is not what \
             the ring below is measuring: {} of {} are not its colour",
            later.count, later.considered
        ),
    )?;

    let seen = strays_from(&composed, |x, y| ring_of(SMALL_RECT, x, y), BLUE, ONE_STEP);
    assert_eq!(
        (seen.considered, seen.count, seen.first),
        (SMALL_RING_PIXELS, 0, None),
        "every outline is composed before any fill, so the earlier element's fill lands over the \
         later element's ring — one pass per element would leave a black notch here"
    );
    Ok(())
}

/// Whether `(x, y)` falls on the one-pixel ring immediately surrounding `fill`.
fn ring_of(fill: Rect, x: u32, y: u32) -> bool {
    fill.grown_by(1).holds(x, y) && !fill.holds(x, y)
}

/// How far the pixels of `rect` sit from the colour they are expected to show.
fn reads(frame: &Rgba8Image, rect: Rect, expected: [u8; 3]) -> Strays {
    strays_from(frame, |x, y| rect.holds(x, y), expected, ONE_STEP)
}

/// Fails unless [`BACKDROP`] differs from `colour`.
///
/// A structural check rather than an assertion about a frame: a backdrop that
/// happened to be the colour a test asserts would make that test green whatever
/// the renderer did.
fn backdrop_unlike(colour: [u8; 3], why: &str) -> Result<(), Box<dyn Error>> {
    require(
        BACKDROP != colour,
        format!("the backdrop {BACKDROP:?} may not be {colour:?} — {why}"),
    )
}
