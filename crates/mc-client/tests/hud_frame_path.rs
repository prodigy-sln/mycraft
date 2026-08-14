//! The one frame call the client makes, and what a crosshair's absence from
//! content does to the frame it draws.
//!
//! # "The windowed client draws one frame" is reached through the object, not
//! through the window
//!
//! A test cannot open a window in CI, and nothing here pretends otherwise. The
//! claim that the client composes the HUD exactly once per frame is held by two
//! joined facts, and **neither is sufficient alone**:
//!
//! 1. The count asserted below — one frame recorded on the renderer the windowed
//!    client owns takes the composition count from none to one.
//! 2. The scan in `hud_entry_point.rs` — the client's production sources name no
//!    other way of drawing a HUD, and no second frame path that draws terrain
//!    with no HUD over it.
//!
//! Fact 1 alone passes for a client that also composed a HUD somewhere else, or
//! that kept a second frame path skipping it; fact 2 alone passes for a client
//! whose one frame call composes nothing. Two halves of one claim, stated here
//! rather than in a commit message, because a reader of either file on its own
//! would otherwise read it as more than it is.
//!
//! # Why the crosshair's absence is worth a scenario
//!
//! The falsifier is a crosshair that survives the deletion of the content that
//! declares it — a HUD drawn from Rust, which is the violation of invariant 1
//! this phase could otherwise introduce, and which no absence-of-a-name scan can
//! see: a hardcoded crosshair drawn from unnamed constants passes every text
//! scan there is.
//!
//! That comparison passes trivially against a renderer that draws no HUD at all,
//! so it is preceded by a guard establishing that the **shipped** root does
//! change the centre. The guard is what makes the scenario evidence rather than
//! an observation about two blank frames.

mod support;

use support::hud_frames::{HudCapture, compare_frames, hud_of, no_hud};
use support::{TestResult, content, content_root, frames};

/// The two files the base game's crosshair is declared in.
///
/// Two, because the engine knows a filled rectangle and not a crosshair: the
/// shape is composed in content from two crossing bars.
const CROSSHAIR_DECLARATIONS: [&str; 2] = ["crosshair-horizontal.toml", "crosshair-vertical.toml"];

/// The tick every frame here is drawn at.
const TICK: u32 = 0;

/// How wide the square this scenario calls "the screen centre" is, in pixels of
/// the declared capture size.
const CENTRE_EXTENT: u32 = 32;

/// Where that square's left edge sits, and where its top edge sits.
///
/// Derived, not eyeballed. At the declared capture size of 1280 × 720 one UI
/// unit is exactly one physical pixel and the target's centre is (640, 360), so
/// a 32-pixel square about it spans x 624..655 and y 344..375. The two base
/// crosshair declarations land inside it with room on every side: the
/// horizontal bar fills `round(640 − 4.5) = 636` to 644 at y 360, and the
/// vertical bar fills x 640 from `round(360 − 4.5) = 356` to 364, each grown by
/// one unit of outline — x 635..645 and y 355..365 between them.
/// `ui-design.md` §5 reserves the centre for the crosshair, so nothing else is
/// declared into this square.
const CENTRE_LEFT: u32 = 624;
const CENTRE_TOP: u32 = 344;

/// How many pixels the centre square holds.
const CENTRE_AREA: u64 = (CENTRE_EXTENT as u64) * (CENTRE_EXTENT as u64);

#[test]
fn a_frame_from_a_root_without_the_crosshair_declarations_leaves_the_centre_as_it_was() -> TestResult
{
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let mut frames_of = HudCapture::ready(&context, TICK)?;

    let request = frames::request(&context, "hud-centre-zero-elements")?;
    let nothing_declared = frames_of.capture(&no_hud()?, &request)?;
    let request = frames::request(&context, "hud-centre-shipped")?;
    let shipped = frames_of.capture(&hud_of(&content_root()?)?, &request)?;
    let declared = compare_frames(&shipped, &nothing_declared, in_the_centre);
    assert!(
        declared.considered == CENTRE_AREA && declared.different > 0,
        "the content the base game ships has to change the screen centre, or the comparison \
         below is between two frames that were never going to differ there and would stay green \
         against a client that drew no HUD at all: {declared:?}"
    );

    let stripped = content::shipped_without(&CROSSHAIR_DECLARATIONS)?;
    let request = frames::request(&context, "hud-centre-without-the-crosshair")?;
    let without_a_crosshair = frames_of.capture(&hud_of(stripped.path())?, &request)?;

    let compared = compare_frames(&without_a_crosshair, &nothing_declared, in_the_centre);
    assert_eq!(
        (compared.considered, compared.different),
        (CENTRE_AREA, 0),
        "delete the declarations and the crosshair goes with them: a pixel still differing at \
         the centre is one the engine drew of its own accord, which is the base game holding a \
         privilege a third-party mod does not have: {compared:?}"
    );
    Ok(())
}

#[test]
fn one_frame_through_the_renderer_the_client_owns_composes_the_hud_exactly_once() -> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let mut frames_of = HudCapture::ready(&context, TICK)?;
    let request = frames::request(&context, "hud-composition-count")?;

    let before = frames_of.renderer.hud_compositions();
    let _frame = frames_of.capture(&hud_of(&content_root()?)?, &request)?;
    let after = frames_of.renderer.hud_compositions();

    assert_eq!(
        (before, after),
        (0, 1),
        "one frame is one composition: a frame path that composed the HUD twice would draw a \
         translucent element over itself, and one that composed it not at all would draw the \
         world with nothing over it while every scenario about the composition itself stayed \
         green"
    );
    Ok(())
}

/// Whether `(x, y)` falls in the square this file calls the screen centre.
fn in_the_centre(x: u32, y: u32) -> bool {
    (CENTRE_LEFT..CENTRE_LEFT + CENTRE_EXTENT).contains(&x)
        && (CENTRE_TOP..CENTRE_TOP + CENTRE_EXTENT).contains(&y)
}
