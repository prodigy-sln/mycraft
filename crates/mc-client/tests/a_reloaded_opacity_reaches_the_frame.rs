//! A mod author changes how much light a block stops while the game is running,
//! and the next frame draws the block that way — in both directions.
//!
//! # Every expectation here is absolute, and none is a comparison
//!
//! Each expected colour is composited by hand from the two layers' own colours
//! and the degree the declaration states. **Nothing below compares the frame
//! before the reload against the frame after it**, and that is not a preference:
//! two renderings differing only in what was reloaded can agree while both are
//! wrong, and a reload that served the wrong resolution to both would move both
//! together. What a comparison could see, an absolute triple sees too; what it
//! cannot see is exactly the failure this spec exists to remove.
//!
//! The arithmetic is done in linear light because the colour attachment is
//! `Rgba8UnormSrgb` and the hardware decodes both operands, mixes, and
//! re-encodes. Over this fixture's own palette the same mix computed on the
//! stored bytes lands ΔE 15.60 from the right answer, against a tolerance of 6 —
//! red on a correct frame, with a looser tolerance as its cheapest green. That
//! is why the composition lives in `support::art` and in none of these tests.
//!
//! # Both ways back to a whole degree, and they are different code
//!
//! A reload may raise a degree by **stating** `1.0` or by **removing the field**,
//! and only the first reaches the reader that parses a number: the second is the
//! absent-field default. A loader that lost the default, or one that kept the
//! last degree it saw for a field a declaration no longer states, satisfies one
//! and not the other — so each has its own reading rather than one standing in
//! for both.
//!
//! # What the fixture is and where the reading is taken
//!
//! `support/reload_opacity.rs` holds both, with the geometry and the eye derived
//! in its header. The short of it: one pane lying on one floor, meshed once and
//! packed again against whatever the client is serving, drawn from an eye that
//! sees the pane over the floor in every pixel of the frame.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_opacity.rs"]
mod reload_opacity;
#[path = "support/reload_remesh.rs"]
mod reload_remesh;
#[path = "support/reload_upload.rs"]
mod reload_upload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use mc_testkit::frame::Rgba8Image;

use reload_opacity::{
    HALF, PANE_FILE, PIXELS_IN_THE_FRAME, Playing, WHOLE, a_client_playing,
    a_root_whose_pane_declares, drawn_against, pane_declaring, the_four_colours,
};
use reload_upload::{layers_handed_over, until_taken_up};
use reload_watch::{block_path, restating_raw};
use support::TestResult;
use support::pixel_census::{Presence, census, owed, require_told_apart};
use support::translucency::TELLS_THEM_APART;

/// The two presences these readings name.
const MANY: Presence = Presence::AtLeastMany;
const NONE: Presence = Presence::NotOnce;

#[test]
fn a_pane_reloaded_at_half_a_degree_is_thereafter_drawn_blended_over_the_floor_it_lies_on()
-> TestResult {
    let expected = the_four_colours();
    require_told_apart(&expected, TELLS_THEM_APART)?;

    let Some(frame) = drawn_after_restating(Some(WHOLE), &pane_declaring(Some(HALF)))? else {
        return Ok(());
    };

    let counted = census(&frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (counted.considered, counted.shown.clone(), counted.strayed),
        (
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[MANY, NONE, NONE, NONE]),
            NONE,
        ),
        "the client launched with this block stopping all the light reaching it and the author \
         edited that to a half without stopping the game. Every pixel of the frame drawn after is \
         the even blend of the pane's own colour with the floor's, computed in linear light — the \
         pane's own colour appearing anywhere is a reload that changed nothing the packer read, \
         and the floor's own colour appearing is a pane that stopped being drawn at all. First \
         stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

#[test]
fn a_pane_whose_reload_states_a_whole_degree_again_is_thereafter_drawn_at_its_own_colour()
-> TestResult {
    let expected = the_four_colours();
    require_told_apart(&expected, TELLS_THEM_APART)?;

    let Some(frame) = drawn_after_restating(Some(HALF), &pane_declaring(Some(WHOLE)))? else {
        return Ok(());
    };

    let counted = census(&frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (counted.considered, counted.shown.clone(), counted.strayed),
        (
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[NONE, MANY, NONE, NONE]),
            NONE,
        ),
        "an author who has seen the half and wants the block back the way it was writes the \
         degree that means it stops everything. Every pixel is then the pane's own colour with \
         nothing mixed into it, and the half blend standing anywhere at all is a reload that took \
         the file up and went on drawing the degree it had before. First stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

#[test]
fn a_pane_whose_reload_removes_the_degree_is_thereafter_drawn_at_its_own_colour() -> TestResult {
    let expected = the_four_colours();
    require_told_apart(&expected, TELLS_THEM_APART)?;

    let Some(frame) = drawn_after_restating(Some(HALF), &pane_declaring(None))? else {
        return Ok(());
    };

    let counted = census(&frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (counted.considered, counted.shown.clone(), counted.strayed),
        (
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[NONE, MANY, NONE, NONE]),
            NONE,
        ),
        "the other way back, and the one that reaches no number reader at all: the author deletes \
         the line. An absent field means a whole degree, so the frame is the same one the reading \
         above asks for — and a loader that kept the last degree it was told about for a field \
         this declaration no longer states draws the blend instead, which nothing that parses a \
         number could report. First stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

/// The frame drawn after a client launched with the pane declaring `at_launch`
/// has `edited` written into the root it is playing and takes the reload up.
///
/// `None` where the opt-in permitted the absence of a device.
///
/// # Errors
///
/// Returns the root, world, mesh, packing or capture failure, and the refusal
/// where no candidate was taken up — a reload that was refused is not a reload
/// whose frame a reading may go on to judge.
fn drawn_after_restating(
    at_launch: Option<f32>,
    edited: &str,
) -> Result<Option<Rgba8Image>, Box<dyn std::error::Error>> {
    let root = a_root_whose_pane_declares(at_launch)?;
    let Playing {
        mut client,
        reports,
        meshed,
    } = a_client_playing(&root)?;
    let root = restating_raw(root, PANE_FILE, edited)?;

    reports.changed(&[block_path(&root, PANE_FILE)])?;
    let serving = layers_handed_over(until_taken_up(&mut client))?;

    drawn_against(&meshed, serving.stated())
}
