//! Whether the debug overlay is in the frame: not when the client starts, and
//! once per press of the key the declared table binds its toggle to.
//!
//! # The observation is pixels, because "renders" is what the two scenarios say
//!
//! Whether the overlay is *visible* is settled elsewhere, by the toggle's own
//! scenarios, against a `bool` no frame is involved in. What is left, and what is
//! here, is whether that bool reaches a drawn frame — so every assertion below is
//! a comparison of two frames recorded through the one call the windowed client
//! makes, with the client's own answer for what its overlay publishes.
//!
//! A test cannot open a window, so "the windowed client" is reached through the
//! object the windowed client owns: a session driven by real key events, and a
//! `FrameRenderer` handed what that session publishes. **What that leaves
//! uncovered is stated rather than glossed:** the client's own two lines — ask the
//! session, hand the answer to the frame — live in `App`, which nothing in this
//! workspace runs, so a client that asked and then discarded the answer would
//! satisfy every scenario here. That gap is the standing one this spec has
//! measured twice already, and it is why this increment also ends with somebody
//! launching the client and pressing the key.
//!
//! # No count of painted pixels appears anywhere here
//!
//! How many pixels a line of text puts down is a fact about a font, a rasteriser
//! and a driver, and a number snapshotted from the first green run would commit
//! whatever that run happened to draw. So the readings below are *differences
//! between frames drawn from the same everything else*: supplying a reading has to
//! move at least one pixel, and taking it away has to put back every pixel it
//! moved. Both are derived from the fixture rather than measured from a run, and
//! neither moves when a font or a theme does.
//!
//! # The frame is the one a client draws before its world lands
//!
//! Which is the frame the first of these two scenarios is literally about — a
//! client that has just started — and it declares no HUD element, exactly as
//! `App` does until its preparation is collected. So any pixel that moves between
//! two of these frames is the overlay's, and nothing else in the picture can
//! account for it.

#[path = "support/input/mod.rs"]
mod input;
mod support;

use std::error::Error;

use mc_render::hud::HudFrame;
use mc_render::overlay::OverlayReadout;
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::{CaptureContext, CaptureRequest};
use winit::keyboard::KeyCode;

use input::InputHarness;
use support::hud_frames::{Comparison, compare_frames, no_hud};
use support::overlay_frames::OverlayFrames;
use support::{TestResult, frames};

/// The key the declared binding table names for the overlay's toggle.
const DECLARED_TOGGLE: KeyCode = KeyCode::F3;

/// How many pixels one declared capture holds: `1280 × 720`.
const FRAME_PIXELS: u64 = 921_600;

/// Two frames of this size agreeing at every pixel.
///
/// `considered` is asserted beside the verdict rather than trusted: a comparison
/// that looked at no pixel reports zero differences too, and would satisfy every
/// "nothing moved" claim below.
const EVERY_PIXEL_UNMOVED: Comparison = Comparison {
    considered: FRAME_PIXELS,
    same: FRAME_PIXELS,
    different: 0,
    first_different: None,
};

/// What an overlay publishes over a frame drawn before the world lands: the two
/// frame readings, and neither world reading.
///
/// **A fixture, not an expectation.** What the lines *say* is graded by the
/// overlay's own suite, which compares them whole with no device anywhere near
/// it; what these two scenarios grade is whether they reach a frame at all. The
/// two numbers are plausible rather than special, so what gets painted is
/// representative text rather than a pair of zeroes — and the absent position is
/// what a client genuinely publishes in the phase these frames are drawn in.
const WAITING_READOUT: OverlayReadout = OverlayReadout {
    position: None,
    column: None,
    frame_rate: 60.0,
    frame_time_ms: 16.67,
};

/// Every pixel of a frame.
fn everywhere(_x: u32, _y: u32) -> bool {
    true
}

/// One frame of `client`'s waiting world, drawn with whatever that client
/// publishes for its overlay.
///
/// The client is asked once per frame, exactly as a frame path asks it, and the
/// answer is carried and not inspected: a fixture that read the visibility and
/// decided for itself what to hand the renderer would be this suite answering its
/// own question.
fn drawn_for(
    frames_of: &mut OverlayFrames<'_>,
    hud: &HudFrame,
    client: &InputHarness,
    request: &CaptureRequest,
) -> Result<Rgba8Image, Box<dyn Error>> {
    frames_of.capture(hud, client.overlay_readout().as_ref(), request)
}

/// One whole keystroke of the toggle: pressed, and let go of.
///
/// A player presses and releases, so that is what is dispatched. Which of the two
/// transitions the visibility turns on is a separate claim with its own scenario,
/// and this deliberately makes no use of the answer.
fn keystroke(client: &mut InputHarness) {
    client.press(DECLARED_TOGGLE);
    client.release(DECLARED_TOGGLE);
}

/// The three frames a client that has just started is read from: its own, one
/// drawn with no reading at all, and one drawn with a reading supplied.
#[derive(Debug)]
struct AtStart {
    published: Rgba8Image,
    withheld: Rgba8Image,
    supplied: Rgba8Image,
}

/// Those three frames, all over one waiting world so nothing but the reading can
/// differ between them.
///
/// # Errors
///
/// Returns the fixture, recording or capture failure.
fn at_start(context: &CaptureContext) -> Result<AtStart, Box<dyn Error>> {
    let mut frames_of = OverlayFrames::waiting(context)?;
    let nothing = no_hud()?;
    let started = InputHarness::started();
    Ok(AtStart {
        published: drawn_for(
            &mut frames_of,
            &nothing,
            &started,
            &frames::request(context, "overlay-at-client-start")?,
        )?,
        withheld: frames_of.capture(
            &nothing,
            None,
            &frames::request(context, "overlay-withheld")?,
        )?,
        supplied: frames_of.capture(
            &nothing,
            Some(&WAITING_READOUT),
            &frames::request(context, "overlay-supplied")?,
        )?,
    })
}

/// The three frames of one run: before either keystroke, after the first, and
/// after the second.
#[derive(Debug)]
struct Toggling {
    before: Rgba8Image,
    after_one: Rgba8Image,
    after_two: Rgba8Image,
}

/// One client, two whole keystrokes, and the frame it drew between each of them.
///
/// # Errors
///
/// Returns the fixture, recording or capture failure.
fn toggling(context: &CaptureContext) -> Result<Toggling, Box<dyn Error>> {
    let mut frames_of = OverlayFrames::waiting(context)?;
    let nothing = no_hud()?;
    let mut client = InputHarness::started();
    let before = drawn_for(
        &mut frames_of,
        &nothing,
        &client,
        &frames::request(context, "overlay-before-any-toggle")?,
    )?;
    keystroke(&mut client);
    let after_one = drawn_for(
        &mut frames_of,
        &nothing,
        &client,
        &frames::request(context, "overlay-after-one-toggle")?,
    )?;
    keystroke(&mut client);
    let after_two = drawn_for(
        &mut frames_of,
        &nothing,
        &client,
        &frames::request(context, "overlay-after-two-toggles")?,
    )?;
    Ok(Toggling {
        before,
        after_one,
        after_two,
    })
}

/// Refuses to go on unless a supplied reading reaches the frame at all.
///
/// The control the scenario below is stated under: "the frame a started client
/// draws is the frame drawn with no reading at all" is satisfied by a client that
/// publishes nothing *and* by a renderer that paints nothing whatever it is
/// handed, and only one of those is the claim. A reading that moves no pixel makes
/// the equality a comparison of two identical frames drawn twice.
///
/// # Errors
///
/// Returns an error reporting the comparison when supplying a reading changed
/// nothing.
fn require_a_supplied_readout_moves_pixels(
    supplied: &Rgba8Image,
    withheld: &Rgba8Image,
) -> Result<(), Box<dyn Error>> {
    let moved = compare_frames(supplied, withheld, everywhere);
    if moved.different > 0 {
        return Ok(());
    }
    Err(format!(
        "a frame handed a readout has to differ from the same frame handed none, or the equality \
         this scenario asserts is about a renderer that paints no overlay however it is asked — \
         which is the one thing it must not be allowed to mean. It reported: {moved:?}"
    )
    .into())
}

/// What two presses of the toggle did to the frame.
#[derive(Debug, PartialEq, Eq)]
enum Toggled {
    /// One press put pixels in the frame and the second press put every one of
    /// them back.
    ShownThenHidden,
    /// The first press changed nothing: nothing in the frame says the overlay was
    /// ever asked for.
    NothingWasShown,
    /// The second press left pixels behind.
    NothingWasHidden { left: u64 },
    /// A comparison looked at no pixel at all, so neither reading above means
    /// anything.
    NoPixelsWereCompared,
}

/// What the three frames of one run say about its two keystrokes.
fn toggled(before: &Rgba8Image, after_one: &Rgba8Image, after_two: &Rgba8Image) -> Toggled {
    let shown = compare_frames(after_one, before, everywhere);
    let hidden = compare_frames(after_two, before, everywhere);
    if shown.considered != FRAME_PIXELS || hidden.considered != FRAME_PIXELS {
        return Toggled::NoPixelsWereCompared;
    }
    if shown.different == 0 {
        return Toggled::NothingWasShown;
    }
    if hidden.different > 0 {
        return Toggled::NothingWasHidden {
            left: hidden.different,
        };
    }
    Toggled::ShownThenHidden
}

/// Why a started client's frame has to be the frame drawn with no reading at all.
const NOTHING_UNTIL_ASKED: &str = "the overlay is engine tooling, hidden until somebody asks for it, so the first frame of every \
     run is the frame of a client that has no overlay in it at all. A client that published a \
     reading before anyone pressed anything would put a panel of text over the game for every \
     player who never wanted one — and the two frames compared here are drawn from the same world, \
     the same layout and the same renderer, so a pixel that moved is the overlay's and nothing \
     else's";

/// Why one press has to show it and the next has to stop showing it.
const ONE_KEY_AND_NO_OTHER_WAY_IN: &str = "one press has to put the instrument in the picture and the next has to take it out, because \
     whoever is diagnosing this engine has one key and no other way in. A toggle that reached the \
     visibility and stopped there is a client whose overlay can be turned on and never seen, which \
     is indistinguishable from not having built one; a toggle that could only show is a key that \
     stops doing anything the moment it has been pressed once, and leaves the panel over the game \
     for the rest of the run";

#[test]
fn a_client_that_has_just_started_draws_a_frame_with_no_overlay_pixels_in_it() -> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let drawn = at_start(&context)?;
    require_a_supplied_readout_moves_pixels(&drawn.supplied, &drawn.withheld)?;

    assert_eq!(
        compare_frames(&drawn.published, &drawn.withheld, everywhere),
        EVERY_PIXEL_UNMOVED,
        "{NOTHING_UNTIL_ASKED}"
    );
    Ok(())
}

#[test]
fn one_press_of_the_toggle_puts_the_overlays_pixels_in_the_frame_and_a_second_takes_them_out()
-> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let run = toggling(&context)?;

    assert_eq!(
        toggled(&run.before, &run.after_one, &run.after_two),
        Toggled::ShownThenHidden,
        "{ONE_KEY_AND_NO_OTHER_WAY_IN}"
    );
    Ok(())
}
