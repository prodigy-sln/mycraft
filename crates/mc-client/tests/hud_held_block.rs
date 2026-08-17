//! The held-block indicator: what it shows, what it shows instead when the
//! block changes, and what it leaves alone when there is nothing to show.
//!
//! # The swatch is judged against two colours, and one would be wrong
//!
//! A placeholder layer is a **checkerboard of the declared mean plus and minus
//! one fixed step per channel**, so exactly two colours reach it and each covers
//! half of an even-sided texture. Those two sit about ΔE 3.5 from the mean they
//! straddle and about ΔE 7 from each other, so **no textured swatch can land
//! every pixel within ΔE 2 of a single colour** — the only way to satisfy that
//! form is to draw the swatch flat, which is not drawing the texture. The
//! assertion below is therefore a two-colour multiset: every pixel within ΔE 2
//! of *one* of the two, and *both* of them shown. That form reports a flat
//! swatch, another block's texture, and nothing drawn at all, where the
//! single-colour form catches those only by accident.
//!
//! **Both colours come from `placeholder_texels`, which is a generator, and
//! from the key the held block's own name spells.** Nothing here reads a colour
//! out of a rendered frame to find out what that frame should have held; that is
//! how a broken renderer certifies itself.
//!
//! # Every rectangle is derived from the declaration and the target
//!
//! At the declared capture size of 1280 × 720 the scale is 1, so a UI unit is a
//! physical pixel, `round` is half away from zero, and the safe-area insets are
//! `round(0.05 × 1280) = 64` and `round(0.05 × 720) = 36`. The indicator is
//! declared `[24, 24]` at the `bottom` anchor, whose named edge sits on the
//! safe-area box while its free axis centres on the target:
//!
//! - `left = round(640 − 12) = 628`, `top = 720 − 36 − 24 = 660`
//! - grown by the one unit of outline it declares, its footprint is
//!   `627..653 × 659..685`, which is 26 × 26 = 676 pixels
//!
//! The two crosshair bars are `[9, 1]` and `[1, 9]` centred, so their fills are
//! `(636, 360)` and `(640, 356)` and their footprints are `635..646 × 359..362`
//! and `639..642 × 355..366` — 33 pixels each, overlapping in a 3 × 3 square, so
//! **57** between them.
//!
//! # Why the indicator's footprint and not its fill
//!
//! An outline is composed for an element of any draw kind, so a declaration that
//! resolves to nothing could still leave a black ring around nothing at the
//! anchor the indicator names. A reader of that frame sees a bordered empty
//! square where a swatch belongs, which is an indicator. So every assertion here
//! about the indicator *not* being drawn is stated over the footprint, which the
//! ring falls inside; a claim over the fill rectangle alone could not see it.

#[path = "support/input/mod.rs"]
mod input;
mod support;

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use mc_core::block::BlockRegistry;
use mc_core::id::{BlockName, TextureKey};
use mc_render::hud::{HudFrame, held_swatch};
use mc_render::texture::TextureLayers;
use mc_sim::action::default_held_block;
use mc_sim::replay::simulation_for;
use mc_testkit::frame::gpu::CaptureContext;
use mc_world::content::LuauFileDefinitionSource;

use input::InputHarness;
use support::content;
use support::hud_frames::{
    Comparison, HudCapture, Rect, UnpreparedCapture, compare_frames, hud_of, no_hud,
};
use support::probe::distance;
use support::swatch::{SAME_COLOR, TEXEL_COLORS, require, swatch_reading, texel_colors};
use support::{TestResult, content_root, frames};

/// The tick every prepared frame here is drawn at.
const TICK: u32 = 0;

/// The file the base game declares its indicator in, and the two it declares
/// its crosshair in.
const INDICATOR_DECLARATION: &str = "held-block.toml";
const CROSSHAIR_DECLARATIONS: [&str; 2] = ["crosshair-horizontal.toml", "crosshair-vertical.toml"];

/// The block definition whose file name is moved to change which block a run
/// holds, and where it is moved to.
///
/// Blocks register in file-name sorted order and a client holds the first solid
/// one, so `dirt`, `grass`, `stone`, `water` becomes `grass`, `stone`, `water`,
/// `zz-dirt` and the first solid block is a different one. Nothing else about
/// the root changes: the same four blocks register, the same world generates,
/// and the same texture keys occupy the same layers.
const HELD_FIRST: &str = "dirt.luau";
const HELD_LAST: &str = "zz-dirt.luau";

/// Where the indicator's fill lands, derived in this file's header.
const INDICATOR_FILL: Rect = Rect {
    x: 628,
    y: 660,
    width: 24,
    height: 24,
};

/// That fill grown by the one unit of outline the declaration states — the
/// region a ring around nothing would fall inside.
const INDICATOR_FOOTPRINT: Rect = INDICATOR_FILL.grown_by(1);

/// The two crosshair bars' footprints, derived in this file's header.
const CROSSBAR_FOOTPRINT: Rect = Rect {
    x: 635,
    y: 359,
    width: 11,
    height: 3,
};
const UPRIGHT_FOOTPRINT: Rect = Rect {
    x: 639,
    y: 355,
    width: 3,
    height: 11,
};

/// How many pixels the two crosshair footprints cover between them:
/// `33 + 33 − 9`, the nine being the 3 × 3 square where they cross.
const CROSSHAIR_PIXELS: u64 = 57;

/// How many pixels one declared capture holds: `1280 × 720`.
const FRAME_PIXELS: u64 = 921_600;

#[test]
fn the_indicator_shows_both_texel_colours_of_the_block_the_session_holds() -> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    require_declared(&[INDICATOR_DECLARATION])?;
    let (mut frames_of, layers, placing) = replay_holding(&context)?;
    let client = client_holding(&frames_of, &placing)?;

    let held = client.held_block();
    let showing = held_swatch(held.as_ref(), &layers).texture();
    let request = frames::request(&context, "hud-held-block-swatch")?;
    let frame = frames_of.capture(&holding(&content_root()?, showing)?, &request)?;

    let seen = swatch_reading(&frame, INDICATOR_FILL, &texel_colors(&placing)?)?;
    let read = (
        held.as_ref().map(BlockName::as_str),
        seen.strayed,
        seen.shown,
    );
    assert_eq!(
        (read, seen.considered),
        (
            (Some(placing.as_str()), 0, TEXEL_COLORS),
            INDICATOR_FILL.area()
        ),
        "the indicator draws the texture of the block a placement would use: every pixel of it is \
         one of the two colours that block's placeholder layer is made of, and both of them are \
         there — a flat swatch shows one, another block's texture shows neither, and nothing \
         drawn shows the world behind it"
    );
    Ok(())
}

#[test]
fn two_content_roots_holding_different_blocks_show_different_indicator_pixels() -> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    require_declared(&[INDICATOR_DECLARATION])?;
    let moved = content::shipped_renaming_block(HELD_FIRST, HELD_LAST)?;
    let (one, other) = (held_by(&content_root()?)?, held_by(moved.path())?);
    require_distinguishable(&one, &other)?;

    let mut frames_of = HudCapture::ready(&context, TICK)?;
    let layers = frames_of.content.layers.clone();
    let showing = |block: &BlockName| held_swatch(Some(block), &layers).texture();
    let (first, second) = (showing(&one), showing(&other));
    require_both_resolve(first.as_ref(), second.as_ref())?;

    let request = frames::request(&context, "hud-held-block-one")?;
    let from_one = frames_of.capture(&holding(&content_root()?, first)?, &request)?;
    let request = frames::request(&context, "hud-held-block-other")?;
    let from_other = frames_of.capture(&holding(moved.path(), second)?, &request)?;

    let seen = compare_frames(&from_one, &from_other, |x, y| INDICATOR_FILL.holds(x, y));
    assert_eq!(
        (seen.considered, seen.same),
        (INDICATOR_FILL.area(), 0),
        "which block is held is what the indicator draws: an indicator painting a fixed layer, or \
         painting nothing, reads the same in both frames"
    );
    Ok(())
}

#[test]
fn a_client_whose_world_has_not_landed_draws_no_indicator_and_no_ring_around_one() -> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    require_declared(&[INDICATOR_DECLARATION])?;
    let mut waiting = UnpreparedCapture::waiting(&context)?;

    let request = frames::request(&context, "hud-waiting-declared")?;
    let shipped = waiting.capture(&hud_of(&content_root()?)?, &request)?;
    let request = frames::request(&context, "hud-waiting-nothing-declared")?;
    let bare = waiting.capture(&no_hud()?, &request)?;

    let reached = compare_frames(&shipped, &bare, crosshair_footprint);
    require(
        reached.considered == CROSSHAIR_PIXELS && reached.different > 0,
        format!(
            "what content declares has to reach a waiting frame at all, or every claim below \
             about what is absent from one holds for a HUD that composed nothing: {reached:?}"
        ),
    )?;

    let seen = compare_frames(&shipped, &bare, |x, y| INDICATOR_FOOTPRINT.holds(x, y));
    assert_eq!(
        (seen.considered, seen.different),
        (INDICATOR_FOOTPRINT.area(), 0),
        "a client holding nothing draws no swatch and no outline around where one would go: a \
         ring around nothing is an indicator of nothing, which is what this whole footprint — \
         rather than the fill alone — is compared to see"
    );
    Ok(())
}

#[test]
fn a_client_whose_world_has_not_landed_still_draws_the_crosshair() -> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    require_declared(&CROSSHAIR_DECLARATIONS)?;
    let mut waiting = UnpreparedCapture::waiting(&context)?;

    let request = frames::request(&context, "hud-waiting-crosshair")?;
    let shipped = waiting.capture(&hud_of(&content_root()?)?, &request)?;
    let request = frames::request(&context, "hud-waiting-crosshair-bare")?;
    let bare = waiting.capture(&no_hud()?, &request)?;

    let seen = compare_frames(&shipped, &bare, crosshair_footprint);
    assert_eq!(
        (seen.considered, seen.same),
        (CROSSHAIR_PIXELS, 0),
        "a frame drawn before the world lands is a flat clear, so every pixel of the two bars and \
         their outlines has to move: a HUD that waits for the world to hide an indicator it \
         cannot draw hides the crosshair with it, and the crosshair is what a player aims by \
         while the world is still loading"
    );
    Ok(())
}

#[test]
fn holding_nothing_leaves_every_other_declared_element_where_holding_a_block_put_it() -> TestResult
{
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    require_declared(&[INDICATOR_DECLARATION])?;
    let (mut frames_of, layers, placing) = replay_holding(&context)?;
    let showing = held_swatch(Some(&placing), &layers).texture();
    let root = content_root()?;

    let request = frames::request(&context, "hud-holding-a-block")?;
    let held = frames_of.capture(&holding(&root, showing)?, &request)?;
    let request = frames::request(&context, "hud-holding-nothing")?;
    let empty_handed = frames_of.capture(&holding(&root, None)?, &request)?;

    let indicator = compare_frames(&held, &empty_handed, |x, y| INDICATOR_FOOTPRINT.holds(x, y));
    require_indicator_drew(&indicator)?;

    let seen = compare_frames(&held, &empty_handed, |x, y| {
        !INDICATOR_FOOTPRINT.holds(x, y)
    });
    assert_eq!(
        (seen.considered, seen.different),
        (FRAME_PIXELS - INDICATOR_FOOTPRINT.area(), 0),
        "the indicator's absence moves nothing else: a HUD that flowed its elements rather than \
         anchoring each one would shift or resize the crosshair the moment a swatch stopped being \
         drawn, and a player would see the sight move when they emptied their hand"
    );
    Ok(())
}

/// The replay's frames at [`TICK`], the layers its blocks resolved to, and the
/// block a client reading the shipped root would hold.
///
/// One preparation, so a scenario that needs a picture and the content behind it
/// is asking about the same run rather than about two worlds that happen to be
/// generated from the same seed.
///
/// # Errors
///
/// Returns the preparation, pipeline, upload or spawn failure, or the refusal
/// when the shipped content registers no solid block.
fn replay_holding(
    context: &CaptureContext,
) -> Result<(HudCapture<'_>, TextureLayers, BlockName), Box<dyn Error>> {
    let frames_of = HudCapture::ready(context, TICK)?;
    let layers = frames_of.content.layers.clone();
    let placing = first_solid(&frames_of.content.registry)?;
    Ok((frames_of, layers, placing))
}

/// A client's own core, playing the world `frames_of` was prepared from and
/// holding `placing`.
///
/// It is the client's dispatch rather than a stand-in for it, so what the frame
/// below draws the indicator from is what the running client would answer.
///
/// # Errors
///
/// Returns the spawn failure when the world cannot place a player.
fn client_holding(
    frames_of: &HudCapture<'_>,
    placing: &BlockName,
) -> Result<InputHarness, Box<dyn Error>> {
    let mut client = InputHarness::started();
    let world = simulation_for(
        &frames_of.content.world,
        Arc::clone(&frames_of.content.registry),
        support::published_content(&frames_of.content.registry)?,
    )?;
    client.play(world, placing.clone());
    Ok(client)
}

/// Fails unless both held blocks occupy a layer of the array texture.
///
/// # Errors
///
/// Returns a failure naming what each resolved to when either did not.
fn require_both_resolve(
    first: Option<&TextureKey>,
    second: Option<&TextureKey>,
) -> Result<(), Box<dyn Error>> {
    require(
        first.is_some() && second.is_some(),
        format!(
            "both held blocks have to occupy a layer of the array texture, or the two frames \
             compared differ because neither drew an indicator rather than because they hold \
             different blocks: {first:?} and {second:?}"
        ),
    )
}

/// Fails unless the frame that holds a block drew something inside the
/// indicator's footprint.
///
/// # Errors
///
/// Returns a failure carrying the comparison when it did not.
fn require_indicator_drew(indicator: &Comparison) -> Result<(), Box<dyn Error>> {
    require(
        indicator.considered == INDICATOR_FOOTPRINT.area() && indicator.different > 0,
        format!(
            "holding a block has to draw an indicator, or 'everything else is unchanged' is a \
             claim about two frames neither of which has one: {indicator:?}"
        ),
    )
}

/// Whether `(x, y)` falls on either crosshair bar's footprint.
fn crosshair_footprint(x: u32, y: u32) -> bool {
    CROSSBAR_FOOTPRINT.holds(x, y) || UPRIGHT_FOOTPRINT.holds(x, y)
}

/// The shipped HUD with `showing` as the texture the indicator draws.
///
/// # Errors
///
/// Returns the refusal when the root's declarations do not load.
fn holding(root: &Path, showing: Option<TextureKey>) -> Result<HudFrame, Box<dyn Error>> {
    Ok(HudFrame {
        held: showing,
        ..hud_of(root)?
    })
}

/// The block a client reading `root` would hold: the first solid one in
/// registration order, decided by the simulation's own policy.
///
/// # Errors
///
/// Returns the refusal when the root cannot be read, or when it registers no
/// solid block at all.
fn held_by(root: &Path) -> Result<BlockName, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root.to_owned()))?;
    first_solid(&registry)
}

/// The block `registry` gives a client to place.
///
/// # Errors
///
/// Returns a failure when it registers no solid block, which is a fixture that
/// cannot hold anything rather than a client that holds nothing.
fn first_solid(registry: &BlockRegistry) -> Result<BlockName, Box<dyn Error>> {
    default_held_block(registry).ok_or_else(|| {
        "this content root has to register a solid block for a client to hold".into()
    })
}

/// Fails unless the two blocks' placeholder layers are made of four colours no
/// two of which could be mistaken for each other.
///
/// The assertion this guards says **every** indicator pixel differs between two
/// frames, and that is only derivable when a pixel of one texture can never be
/// a pixel of the other. Measured from the generator rather than hoped for: an
/// over-tight assertion is red against a correct renderer, which is the failure
/// that gets fixed by breaking the renderer.
///
/// # Errors
///
/// Returns a failure when the two blocks are the same, or when any colour of one
/// stands within [`SAME_COLOR`] of a colour of the other.
fn require_distinguishable(one: &BlockName, other: &BlockName) -> Result<(), Box<dyn Error>> {
    require(
        one != other,
        format!(
            "the two content roots have to hold different blocks, or the two frames below were \
             never going to differ: both hold `{name}`",
            name = one.as_str()
        ),
    )?;
    for mine in texel_colors(one)? {
        for theirs in texel_colors(other)? {
            require(
                distance(mine, theirs)? > SAME_COLOR,
                format!(
                    "these two blocks' placeholder texels have to be four colours no two of which \
                     read alike, or a pixel of one texture legitimately equals a pixel of the \
                     other and the assertion below is red against a correct renderer: {mine:?} \
                     against {theirs:?}"
                ),
            )?;
        }
    }
    Ok(())
}

/// Fails unless the shipped content root declares every named HUD element file.
///
/// # Errors
///
/// Returns a failure naming the first one it does not.
fn require_declared(files: &[&str]) -> Result<(), Box<dyn Error>> {
    let declared = content_root()?.join(content::HUD_DIRECTORY);
    for file_name in files {
        require(
            declared.join(file_name).is_file(),
            format!(
                "the base game has to declare `{HUD}/{file_name}` for this scenario to be about \
                 anything: without it the frames below are of a HUD that was never going to draw \
                 what is being asked after",
                HUD = content::HUD_DIRECTORY
            ),
        )?;
    }
    Ok(())
}
