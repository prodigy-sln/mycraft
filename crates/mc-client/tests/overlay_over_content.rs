//! Content that covers the whole screen does not cover the debug overlay.
//!
//! A bad mod must not be able to switch off the instrument somebody would
//! diagnose that mod with. The reach half of that guarantee is held by there being
//! no field a declaration can name the overlay through; this is the other half,
//! and it is the strong form: an element painting every pixel of the target in a
//! fully opaque colour, the overlay shown, and the overlay still in the picture.
//! True by construction from the pass order — terrain, then the HUD, then the
//! overlay — and asserted anyway, because "true by construction" is a claim about
//! today's construction.
//!
//! # The oracle is two backdrops, and it names no colour of the overlay's
//!
//! The obvious reading — "some pixel is not the colour the element declared" —
//! would work, and it would have to know what the overlay paints to say anything
//! stronger than *something happened*. Reading a colour out of the toolkit's
//! default theme is exactly the over-tight assertion this spec has been bitten by:
//! a theme is a vendor default, a version bump moves it, and the cheapest way to
//! green the resulting red is to hardcode a colour in the adapter.
//!
//! So the same overlay is drawn twice, over two covering elements whose colours
//! differ in **every** channel. A pixel the overlay painted at full coverage is
//! the same colour in both frames, because at full coverage nothing of the
//! destination is left in it; every pixel the overlay did not reach differs,
//! because the two backdrops differ everywhere. **The pixels the two shown frames
//! agree at are therefore exactly the overlay's own, and the reading never has to
//! know what colour they are.** It is independent of the theme, of the font, and
//! of how many pixels a glyph happens to cover — none of which is a fact about
//! this client.
//!
//! Its own control comes with it: the same two elements drawn with the overlay
//! **hidden** have to agree at *no* pixel at all. Two frames that agreed anywhere
//! without an overlay would make an agreement with one prove nothing.
//!
//! # The colours are exact rather than tolerated, and that is derived
//!
//! `#FF00FFFF` and `#000000FF` are built from bytes 0 and 255 only, and both are
//! **fixed points of the sRGB transfer function** — the decode a declared colour
//! goes through and the re-encode the target performs are inverses there, so an
//! opaque fill of either lands on the byte it declared exactly. That is what lets
//! the precondition below demand every one of 921 600 pixels, rather than a
//! tolerance nobody can justify.
//!
//! # This is a difference between frames and never a golden
//!
//! Rasterised text must not reach a committed reference: drivers disagree about
//! glyphs, and a golden holding one makes whatever rasterised it the ground truth
//! every machine then has to reproduce. Nothing here is compared against anything
//! on disk. The scenario also refuses to run unless the readout is still confined
//! to the files that paint one and none of those can commit a frame — see
//! [`require_no_declared_capture_can_carry_a_readout`].

mod support;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use mc_core::hud::source::InMemoryHudSource;
use mc_core::hud::{DeclaredValue, HudLayout, HudOrigin, RawHudElement};
use mc_render::hud::HudFrame;
use mc_render::overlay::OverlayReadout;
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::CaptureContext;

use support::hud_frames::compare_frames;
use support::overlay_frames::OverlayFrames;
use support::{TestResult, frames, repository_root};

/// The two fully opaque colours the covering element is declared in, and the
/// bytes each of them renders as.
///
/// The bytes are written out from the hex by hand rather than parsed with the
/// code under test, and they are exact for the reason this file's header derives.
const FIRST_COVER: &str = "#FF00FFFF";
const FIRST_BYTES: [u8; 3] = [255, 0, 255];
const SECOND_COVER: &str = "#000000FF";
const SECOND_BYTES: [u8; 3] = [0, 0, 0];

/// How many pixels one declared capture holds: `1280 × 720`.
const FRAME_PIXELS: u64 = 921_600;

/// What an overlay publishes over a frame drawn before the world lands: the two
/// frame readings, and neither world reading.
///
/// A fixture rather than an expectation — what the lines say is graded by the
/// overlay's own suite, and what this scenario needs is text on the screen.
const WAITING_READOUT: OverlayReadout = OverlayReadout {
    position: None,
    column: None,
    frame_rate: 60.0,
    frame_time_ms: 16.67,
};

/// The four files that may name a readout, each for a stated reason, and every
/// other file in either tests tree may not.
///
/// - the input harness, which *forwards* the client's own answer and constructs
///   nothing;
/// - the one fixture module that can carry a present one, which shoots no golden;
/// - the two scenario files that paint one.
///
/// Every capture this repository declares is shot from a file the scan below
/// reads, so a readout cannot reach a golden without a name appearing where this
/// can see it.
const PAINTS_AN_OVERLAY: [&str; 4] = [
    "crates/mc-client/tests/support/input/mod.rs",
    "crates/mc-client/tests/support/overlay_frames.rs",
    "crates/mc-client/tests/overlay_rendering.rs",
    THIS_FILE,
];

/// This file, which the golden-lifecycle guard below has to pass over for the
/// oldest reason in this codebase's scan family: a needle list is its own hit.
///
/// Measured rather than reasoned about — the first run of this scenario reported
/// all five of that guard's needles against this file, from the constant that
/// declares them. What is lost by exempting it is bounded and visible: whether
/// *this* file commits a golden is answered by reading it, and there is nothing on
/// disk anything here is compared against.
const THIS_FILE: &str = "crates/mc-client/tests/overlay_over_content.rs";

/// Both crates whose tests can record a frame of this product.
const FRAME_SUITES: &[&str] = &["crates/mc-client/tests", "crates/mc-render/tests"];

/// One text guard: where it reads, what it passes over, and what it refuses to
/// find. Whole paths relative to the repository, never bare file names.
#[derive(Debug)]
struct Guard {
    roots: &'static [&'static str],
    exempt: fn(&str) -> bool,
    needles: &'static [&'static str],
}

/// Nothing that records a frame of this product names a readout, save the four
/// files that exist to paint one.
const READOUT_GUARD: Guard = Guard {
    roots: FRAME_SUITES,
    exempt: |path| PAINTS_AN_OVERLAY.contains(&path),
    needles: &["OverlayReadout"],
};

/// The same guard with nothing exempt, which is how the needle is watched: it has
/// to report all four of them.
const UNEXEMPTED_READOUT_GUARD: Guard = Guard {
    exempt: |_| false,
    ..READOUT_GUARD
};

/// What may hold a readout may not commit a frame.
///
/// Read over the four files above and nothing else. The needles are the
/// identifiers a golden path has to spell — the settings type, the outcome, the
/// verifying call, the shared settings builder and the variable that mints one —
/// rather than the word, so prose about goldens is not a use of one.
const GOLDEN_LIFECYCLE_GUARD: Guard = Guard {
    roots: FRAME_SUITES,
    exempt: |path| !PAINTS_AN_OVERLAY.contains(&path) || path == THIS_FILE,
    needles: &[
        "GoldenSettings",
        "GoldenOutcome",
        "capture_and_verify",
        "settings_for",
        "MYCRAFT_UPDATE_GOLDENS",
    ],
};

/// The same guard over everything *but* those four, which is how *its* needles are
/// watched: the golden lifecycle has to still be named somewhere in this tree.
const GOLDEN_SHOOTERS_GUARD: Guard = Guard {
    exempt: |path| PAINTS_AN_OVERLAY.contains(&path),
    ..GOLDEN_LIFECYCLE_GUARD
};

/// What a scan of one guard's roots found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    hits: Vec<String>,
}

/// The four frames one reading is made of: the covering element in each of two
/// colours, each drawn once with the overlay shown and once with it hidden.
#[derive(Debug)]
struct OverContent {
    first_shown: Rgba8Image,
    first_hidden: Rgba8Image,
    second_shown: Rgba8Image,
    second_hidden: Rgba8Image,
}

/// Whether the overlay survived content painting over every pixel of the target.
///
/// The two refusals are arms rather than assertions elsewhere, so a reading that
/// could not mean anything cannot arrive under the good verdict's name.
#[derive(Debug, PartialEq, Eq)]
enum Legibility {
    /// Pixels the overlay painted are in both frames: content covering the whole
    /// target did not cover the overlay.
    OverlaysOwnPixelsSurvive,
    /// The two frames agree at no pixel, so nothing the overlay painted is left in
    /// either.
    ContentCoveredTheOverlay,
    /// A frame drawn with the overlay hidden is not uniformly the colour its
    /// element declares, so this element does not cover the target and nothing
    /// below would be about content that does.
    ContentDidNotCoverTheTarget { straying_at: (u32, u32) },
    /// The two backdrops agree somewhere with the overlay hidden, so an agreement
    /// with it shown is not evidence of anything painted over both.
    BackdropsAgreeWithTheOverlayHidden { at: u64 },
}

/// Every pixel of a frame.
fn everywhere(_x: u32, _y: u32) -> bool {
    true
}

/// A declared field holding `spelled`.
fn text(spelled: &str) -> DeclaredValue {
    DeclaredValue::Text(spelled.to_owned())
}

/// A frame whose layout holds exactly one element: an opaque fill of `color`
/// covering the whole render target.
///
/// At the declared capture height of 720 the scale is 1, so a UI unit is a
/// physical pixel and a `[1280, 720]` element anchored `center` lands at
/// `round(640 − 640) = 0`, `round(360 − 360) = 0` — the whole target, exactly.
///
/// Declared in memory rather than written to a content root on disk: what this
/// needs is a root declaring *only* this element, so that a frame drawn with the
/// overlay hidden is uniformly one colour, and the shipped root declares three
/// elements of its own. It still goes through `HudLayout::load`, which is the only
/// door into a layout, so nothing here hand-builds an element the model would have
/// refused.
///
/// # Errors
///
/// Returns the refusal when the layout declines the declaration, or when it
/// registered anything other than the one element — a layout holding nothing paints
/// nothing, and the covering this scenario is about would not exist.
fn covering(color: &str) -> Result<HudFrame, Box<dyn Error>> {
    let origin = HudOrigin::new("covering.toml");
    let declared = RawHudElement::new(vec![
        ("name".to_owned(), text("test:covering-the-target")),
        ("anchor".to_owned(), text("center")),
        (
            "size".to_owned(),
            DeclaredValue::List(vec![
                DeclaredValue::Integer(i64::from(frames::CAPTURE_SIZE.width)),
                DeclaredValue::Integer(i64::from(frames::CAPTURE_SIZE.height)),
            ]),
        ),
        ("draw".to_owned(), text("fill")),
        ("color".to_owned(), text(color)),
    ]);
    let layout = HudLayout::load(&InMemoryHudSource::new(
        origin.clone(),
        vec![(origin, declared)],
    ))?;
    if layout.elements().len() != 1 {
        return Err(format!(
            "this fixture has to register the one element it declares, or the frames below are of \
             a target nothing covered. It registered {}",
            layout.elements().len()
        )
        .into());
    }
    Ok(HudFrame {
        layout: Arc::new(layout),
        held: None,
    })
}

/// The four frames, drawn through the client's own frame call over one waiting
/// world.
///
/// # Errors
///
/// Returns the fixture, recording or capture failure.
fn over_content(context: &CaptureContext) -> Result<OverContent, Box<dyn Error>> {
    let mut frames_of = OverlayFrames::waiting(context)?;
    let first = covering(FIRST_COVER)?;
    let second = covering(SECOND_COVER)?;
    let shown = Some(&WAITING_READOUT);
    Ok(OverContent {
        first_shown: frames_of.capture(
            &first,
            shown,
            &frames::request(context, "cover-a-shown")?,
        )?,
        first_hidden: frames_of.capture(
            &first,
            None,
            &frames::request(context, "cover-a-hidden")?,
        )?,
        second_shown: frames_of.capture(
            &second,
            shown,
            &frames::request(context, "cover-b-shown")?,
        )?,
        second_hidden: frames_of.capture(
            &second,
            None,
            &frames::request(context, "cover-b-hidden")?,
        )?,
    })
}

/// Whether `frame` shows `declared` at `(x, y)`, a missing pixel counting as a
/// stray.
fn shows(frame: &Rgba8Image, x: u32, y: u32, declared: [u8; 3]) -> bool {
    matches!(frame.pixel(x, y), Some([r, g, b, _]) if [r, g, b] == declared)
}

/// The first pixel of `frame` that is not `declared`, in reading order.
fn first_stray(frame: &Rgba8Image, declared: [u8; 3]) -> Option<(u32, u32)> {
    let (width, height) = (frame.width(), frame.height());
    (0..height)
        .flat_map(move |y| (0..width).map(move |x| (x, y)))
        .find(|(x, y)| !shows(frame, *x, *y, declared))
}

/// What the four frames say about whether the overlay survived.
fn legibility_of(over: &OverContent) -> Legibility {
    let strayed = first_stray(&over.first_hidden, FIRST_BYTES)
        .or_else(|| first_stray(&over.second_hidden, SECOND_BYTES));
    if let Some(straying_at) = strayed {
        return Legibility::ContentDidNotCoverTheTarget { straying_at };
    }
    let backdrops = compare_frames(&over.first_hidden, &over.second_hidden, everywhere);
    if backdrops.same > 0 {
        return Legibility::BackdropsAgreeWithTheOverlayHidden { at: backdrops.same };
    }
    if compare_frames(&over.first_shown, &over.second_shown, everywhere).same == 0 {
        return Legibility::ContentCoveredTheOverlay;
    }
    Legibility::OverlaysOwnPixelsSurvive
}

/// Refuses to go on unless the readout is still confined to the files that paint
/// one, and none of those can commit a frame.
///
/// The overlay's text is rasterised, and a committed golden holding a glyph makes
/// whatever rasterised it the ground truth every driver then has to agree with.
/// This scenario is the first thing in the repository to paint one, so it is where
/// that stays checked. Four questions, each because the other three do not answer
/// it.
///
/// # Errors
///
/// Returns an error naming what was found and where.
fn require_no_declared_capture_can_carry_a_readout(root: &Path) -> Result<(), Box<dyn Error>> {
    require_the_readout_is_confined(root)?;
    require_every_file_that_may_name_one_is_reported(root)?;
    require_nothing_that_may_name_one_commits_a_frame(root)
}

/// Nothing outside the four names a readout, and something was read.
fn require_the_readout_is_confined(root: &Path) -> Result<(), Box<dyn Error>> {
    let confined = scan(root, &READOUT_GUARD)?;
    if confined.files_read == 0 {
        return Err(
            "the scan read no test source at all, so the confinement below would be \
                    vacuous — the suites have moved, or the exemptions have grown to cover them"
                .into(),
        );
    }
    if !confined.hits.is_empty() {
        return Err(format!(
            "a suite that records a frame of this product has learned what a readout is, and the \
             only thing standing between that and a committed golden full of rasterised text is \
             that it never could: {:?}",
            confined.hits
        )
        .into());
    }
    Ok(())
}

/// The needle matches, and no exemption stands for a file that stopped needing
/// one.
fn require_every_file_that_may_name_one_is_reported(root: &Path) -> Result<(), Box<dyn Error>> {
    let unexempted = scan(root, &UNEXEMPTED_READOUT_GUARD)?;
    let unreported: Vec<&str> = PAINTS_AN_OVERLAY
        .into_iter()
        .filter(|exempt| !unexempted.hits.iter().any(|hit| hit.starts_with(exempt)))
        .collect();
    if !unreported.is_empty() {
        return Err(format!(
            "with nothing exempt this scan has to report every file it exempts, or the needle has \
             stopped matching what it watches and an exemption is standing for a file that no \
             longer needs one. It did not report: {unreported:?}"
        )
        .into());
    }
    Ok(())
}

/// None of the four can commit a frame, and the needles that say so still match a
/// path that does.
fn require_nothing_that_may_name_one_commits_a_frame(root: &Path) -> Result<(), Box<dyn Error>> {
    let allowed = scan(root, &GOLDEN_LIFECYCLE_GUARD)?;
    let readable = PAINTS_AN_OVERLAY.len() - 1;
    if allowed.files_read != readable || !allowed.hits.is_empty() {
        return Err(format!(
            "each of the {readable} files that may name a readout and is not this one has to be \
             readable and to name no part of the golden lifecycle, or the separation this rests on \
             is one call away from being undone. Read {} of them, and found: {:?}",
            allowed.files_read, allowed.hits
        )
        .into());
    }
    let shooters = scan(root, &GOLDEN_SHOOTERS_GUARD)?;
    if shooters.hits.is_empty() {
        return Err(
            "the golden lifecycle has to be named somewhere in these two trees, or the \
                    needles above have stopped matching the thing they watch and would pass over a \
                    file that did commit one"
                .into(),
        );
    }
    Ok(())
}

#[test]
fn an_opaque_element_covering_the_whole_target_leaves_the_shown_overlays_pixels_in_the_frame()
-> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    require_no_declared_capture_can_carry_a_readout(&repository_root()?)?;
    let over = over_content(&context)?;

    assert_eq!(
        legibility_of(&over),
        Legibility::OverlaysOwnPixelsSurvive,
        "the overlay is what somebody diagnoses a misbehaving mod with, so a mod must not be able \
         to paint over it — and an element covering every one of the {FRAME_PIXELS} pixels of the \
         target in a fully opaque colour is the most a declaration can do. The pass order is what \
         makes this true, so the way it breaks is an ordering change: compose the HUD after the \
         overlay, or into the same pass, and content that asked for the whole screen gets the whole \
         screen. The reading is the pixels these two frames agree at, which are the overlay's own \
         whatever colour it chose them to be"
    );
    Ok(())
}

/// Reads every source under `guard`'s roots and reports each place one of its
/// needles is named.
///
/// A root that does not exist contributes no files rather than an error, which is
/// what leaves the `files_read` refusal to report a suite that has moved.
fn scan(root: &Path, guard: &Guard) -> Result<Scan, Box<dyn Error>> {
    let mut scanned = Scan::default();
    for named in guard.roots {
        let directory = root.join(named);
        if directory.is_dir() {
            walk(&directory, root, guard, &mut scanned)?;
        }
    }
    Ok(scanned)
}

fn walk(
    directory: &Path,
    root: &Path,
    guard: &Guard,
    scanned: &mut Scan,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, root, guard, scanned)?;
        } else if is_source(&path) {
            read(&path, root, guard, scanned)?;
        }
    }
    Ok(())
}

/// Reads one file, unless the guard exempts it — an exempt file is not read, so it
/// can neither be reported nor counted.
fn read(path: &Path, root: &Path, guard: &Guard, scanned: &mut Scan) -> Result<(), Box<dyn Error>> {
    let relative = relative_spelling(path, root)?;
    if (guard.exempt)(&relative) {
        return Ok(());
    }
    let text = stated_text(&fs::read_to_string(path)?);
    scanned.files_read += 1;
    for needle in guard.needles {
        if text.contains(needle) {
            scanned.hits.push(format!("{relative} names `{needle}`"));
        }
    }
    Ok(())
}

/// Any `.rs` file, sibling unit-test files included: a unit test of the renderer
/// can record a frame too.
fn is_source(path: &Path) -> bool {
    path.extension() == Some(OsStr::new("rs"))
}

/// A file's text with its doc comments removed, because prose about a readout is
/// not a use of one — and both trees are full of prose about this very rule.
fn stated_text(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("///") && !trimmed.starts_with("//!")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where a file sits relative to `root`, spelled with `/` on every platform so an
/// exemption can be written once and compared whole.
fn relative_spelling(path: &Path, root: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(root)?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/"))
}
