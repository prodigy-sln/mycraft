//! A mod author changes the medium a player is standing in while the game runs,
//! and the next frame draws through the new declaration.
//!
//! # These four readings are the only thing in the suite that can see a cache
//!
//! The tint is resolved from the live registry at **every** publish and is never
//! remembered across ticks. Nothing in FR-2 can witness that: each of those
//! readings declares its own pose and renders one frame, so an implementation
//! that resolved the eye's medium once and kept the answer satisfies every one
//! of them. What it cannot satisfy is a *second* publish, after a reload,
//! disagreeing with the first — which is what each reading below is built on.
//! **A failure here is read as a caching defect before it is read as a reload
//! defect.**
//!
//! The value compared is the one the shipped client draws from:
//! `InputHarness::published` hands back the `SimSnapshot` the simulation wrote,
//! and `App::snapshot` copies that snapshot's `tint` into the frame without
//! computing anything of its own.
//!
//! # Both halves, because they fail differently
//!
//! Each verdict names the published tint on both sides of the reload *and* the
//! colours the frame drew. The published field alone would be satisfied by a
//! renderer that never wrote the uniform; the frame alone would be satisfied by
//! a simulation that published a stale tint into a draw path that carried it
//! faithfully. Neither half is redundant.
//!
//! # Every expectation is absolute, and the comparison beside it is a control
//!
//! Each expected colour is composed by hand from the wall's own flat colour, the
//! colour the medium declares and the distance **measured off the published
//! camera**, in linear light through `support::art`'s transfer pair. A
//! comparison of the two frames could never stand in for that: two renderings
//! differing only in what was reloaded can agree while both are wrong.
//!
//! It is carried all the same, as the **control** the reading about a *removed*
//! tint cannot do without. "The wall is thereafter drawn at its own colour" is
//! satisfied by a renderer that never drew anything else, absolute half
//! included, because the colour predicted for an untinted eye *is* the untinted
//! colour. So each verdict also states that the frame **moved** when the
//! published tint did — over one scene packed against one resolution, so the
//! tint is the only thing that differs between the two.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_remesh.rs"]
mod reload_remesh;
#[path = "support/reload_tint.rs"]
mod reload_tint;
#[path = "support/reload_upload.rs"]
mod reload_upload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::sync::Arc;

use mc_core::block::MediumTint;
use mc_sim::simulation::SimSnapshot;
use mc_testkit::frame::Rgba8Image;

use reload_tint::{
    AT_LAUNCH, AT_NO_DISTANCE, EXAMINED, MEDIUM, MEDIUM_FILE, NARROWED_TO, Playing,
    THE_WALL_STANDS_AT, a_client_standing_in_it, a_root_whose_medium_declares, drawn_through,
    medium_declaring, reaching, straying_from_the_wall, wall_stands_from,
};
use reload_upload::{layers_handed_over, until_taken_up};
use reload_watch::{block_path, restating_raw};
use support::TestResult;
use support::medium::{TINT, told_apart};
use support::submerged::differing;

/// What one reload came to.
#[derive(Debug, PartialEq)]
struct Reloaded {
    /// What the simulation published for the player's own eye before the author
    /// touched the file, and after the reload landed.
    the_medium_published_before: Option<MediumTint>,
    the_medium_published_after: Option<MediumTint>,
    /// How far the wall's face stood from the published camera, measured.
    the_wall_stood_at: f32,
    pixels_examined: usize,
    drawn_at_something_other_than_the_predicted_colour: Vec<String>,
    /// Whether the frame drawn from the publish before the reload differs from
    /// the one drawn after it.
    ///
    /// **The control, and without it a reading that a tint stopped reaching the
    /// frame is satisfied by a renderer that never reached it.** Both frames are
    /// drawn over one scene packed against one resolution, so the published tint
    /// is the only thing that differs between them.
    the_frame_moved_when_the_published_tint_did: bool,
}

#[test]
fn a_medium_that_gains_a_tint_thereafter_draws_the_wall_at_the_mix_that_tint_states() -> TestResult
{
    let Some(reloaded) = a_reload_that_lands(None, &medium_declaring(Some((AT_LAUNCH, TINT))))?
    else {
        return Ok(());
    };
    let expected = carried(reaching(AT_LAUNCH));
    told_apart(expected, support::medium::WALL_COLOUR)?;

    assert_eq!(
        reloaded,
        went_from(None, reaching(AT_LAUNCH)),
        "the client launched over a medium declaring no tint at all and the author added the pair \
         without stopping the game. The next publish resolves the player's own cell through the \
         registry that is now serving, so the wall {THE_WALL_STANDS_AT} blocks away is drawn at \
         {expected:?} — the even mix, because the medium reaches its full strength at \
         {AT_LAUNCH}. A tint still reported as absent after the reload is an answer remembered \
         from before it"
    );
    Ok(())
}

#[test]
fn a_medium_whose_reload_removes_both_fields_thereafter_draws_the_wall_untinted() -> TestResult {
    let Some(reloaded) = a_reload_that_lands(Some((AT_LAUNCH, TINT)), &medium_declaring(None))?
    else {
        return Ok(());
    };
    assert_eq!(
        reloaded,
        went_from(reaching(AT_LAUNCH), None),
        "the other direction, and the one an author reaches by deleting two lines. Both fields go \
         together or neither does, so what the reload leaves is a medium that tints nothing — and \
         the wall is thereafter drawn at its own colour with nothing mixed into it. A tint still \
         in force here is a resolver that kept the last answer it was given for a field the \
         declaration no longer states"
    );
    Ok(())
}

#[test]
fn a_medium_whose_reload_narrows_only_its_reach_thereafter_draws_the_wall_wholly_at_its_colour()
-> TestResult {
    let Some(reloaded) = a_reload_that_lands(
        Some((AT_LAUNCH, TINT)),
        &medium_declaring(Some((NARROWED_TO, TINT))),
    )?
    else {
        return Ok(());
    };
    let expected = carried(reaching(NARROWED_TO));
    told_apart(expected, carried(reaching(AT_LAUNCH)))?;

    assert_eq!(
        reloaded,
        went_from(reaching(AT_LAUNCH), reaching(NARROWED_TO)),
        "the colour is untouched and only the reach moves, from {AT_LAUNCH} blocks to \
         {NARROWED_TO}. The wall stands at exactly {NARROWED_TO}, so it is thereafter drawn \
         wholly at the declared colour {expected:?} with none of its own left in it. This is the \
         reading that catches a reload carrying the new colour and keeping the stale distance: \
         the colour never changed, so nothing but the distance can account for the frame moving"
    );
    Ok(())
}

#[test]
fn a_reload_stating_a_reach_of_no_distance_is_refused_and_leaves_the_tint_in_force() -> TestResult {
    let refused = a_reload_that_is_refused(
        Some((AT_LAUNCH, TINT)),
        &medium_declaring(Some((AT_NO_DISTANCE, TINT))),
    )?;
    assert_eq!(
        (
            refused.the_medium_published_before,
            refused.the_medium_published_after,
            words_missing_from(&refused.said),
        ),
        (reaching(AT_LAUNCH), reaching(AT_LAUNCH), Vec::new()),
        "a medium reaching its full strength at no distance at all is a claim the engine cannot \
         keep, so the whole root is refused and the declaration that was already loaded stays in \
         force — the path every other refused reload takes. The refusal has to name the file, the \
         block, the field and the bound it broke, or an author reads that something is wrong \
         without reading where. It said: {}",
        refused.said
    );
    Ok(())
}

/// What a reload that moved the tint from `before` to `after` owes.
fn went_from(before: Option<MediumTint>, after: Option<MediumTint>) -> Reloaded {
    Reloaded {
        the_medium_published_before: before,
        the_medium_published_after: after,
        the_wall_stood_at: THE_WALL_STANDS_AT,
        pixels_examined: EXAMINED,
        drawn_at_something_other_than_the_predicted_colour: Vec::new(),
        the_frame_moved_when_the_published_tint_did: true,
    }
}

/// The wall's colour seen through a medium declaring `tint`, at the distance the
/// fixture places it.
fn carried(tint: Option<MediumTint>) -> [u8; 3] {
    support::composite::carried(support::medium::WALL_COLOUR, tint, THE_WALL_STANDS_AT)
}

/// What a refused reload came to.
struct Refused {
    the_medium_published_before: Option<MediumTint>,
    the_medium_published_after: Option<MediumTint>,
    said: String,
}

/// Every word the refusal owes an author that it does not carry.
fn words_missing_from(said: &str) -> Vec<&'static str> {
    [MEDIUM_FILE, MEDIUM, "tint_distance", "greater than zero"]
        .into_iter()
        .filter(|owed| !said.contains(owed))
        .collect()
}

/// A client launched over a medium declaring `at_launch`, whose declaration is
/// then restated as `edited` and taken up.
///
/// `None` where the opt-in permitted the absence of a device.
///
/// # Errors
///
/// Returns the root, world, mesh, packing or capture failure, and the refusal
/// where no candidate was taken up — a reload that was refused is not a reload
/// whose frame a reading may go on to judge.
fn a_reload_that_lands(
    at_launch: Option<(f32, [u8; 3])>,
    edited: &str,
) -> Result<Option<Reloaded>, Box<dyn Error>> {
    let root = a_root_whose_medium_declares(at_launch)?;
    let Playing {
        mut client,
        reports,
        meshed,
    } = a_client_standing_in_it(&root)?;
    let before = published(&client)?;
    let root = restating_raw(root, MEDIUM_FILE, edited)?;

    reports.changed(&[block_path(&root, MEDIUM_FILE)])?;
    let serving = layers_handed_over(until_taken_up(&mut client))?;
    let after = the_next_publish(&mut client)?;

    let Some(was) = drawn_through(&meshed, serving.stated(), &before)? else {
        return Ok(None);
    };
    let Some(is) = drawn_through(&meshed, serving.stated(), &after)? else {
        return Ok(None);
    };
    Ok(Some(what_it_came_to(&before, &after, (&was, &is))?))
}

/// That same run where the reload is expected to be refused instead.
///
/// # Errors
///
/// Returns the root or world failure, or an error where the candidate was taken
/// up — a reload that landed is not a reload a refusal can be read from.
fn a_reload_that_is_refused(
    at_launch: Option<(f32, [u8; 3])>,
    edited: &str,
) -> Result<Refused, Box<dyn Error>> {
    let root = a_root_whose_medium_declares(at_launch)?;
    let Playing {
        mut client,
        reports,
        ..
    } = a_client_standing_in_it(&root)?;
    let before = published(&client)?;
    let root = restating_raw(root, MEDIUM_FILE, edited)?;

    reports.changed(&[block_path(&root, MEDIUM_FILE)])?;
    let said = match until_taken_up(&mut client) {
        reload_upload::TakenUp::Refused { said } => said,
        other => {
            return Err(format!(
                "this reading needs the reload to be refused so that it can ask what stays in \
                 force afterwards, and the run came to {other:?}. There is no refusal to read and \
                 nothing was kept"
            )
            .into());
        }
    };
    let after = the_next_publish(&mut client)?;
    Ok(Refused {
        the_medium_published_before: before.tint,
        the_medium_published_after: after.tint,
        said,
    })
}

/// The snapshot the simulation has published, refusing a client that has
/// published none.
fn published(client: &input::InputHarness) -> Result<Arc<SimSnapshot>, Box<dyn Error>> {
    client.published().ok_or_else(|| {
        "this reading compares what the simulation published on either side of a reload, and this \
         client has published nothing at all — so there is no tint to have changed and no camera \
         to measure the wall from"
            .into()
    })
}

/// The publish that follows a reload's uptake.
///
/// **A tick past the boundary that reported it, and that distinction is part of
/// the subject.** The boundary that takes a candidate up publishes what it
/// resolved *before* adopting it, so reading the snapshot that boundary left
/// would be reading the old registry's answer and calling it the new one. What
/// each of these readings is about is the **next** publish — the first one
/// resolved against the content now serving, which is exactly the publish a
/// remembered tint would get wrong.
///
/// # Errors
///
/// Returns an error where that tick published nothing at all.
fn the_next_publish(client: &mut input::InputHarness) -> Result<Arc<SimSnapshot>, Box<dyn Error>> {
    client.tick().ok_or_else(|| {
        "the tick after the reload published no snapshot at all, so there is nothing to have been \
         resolved against the content now serving"
            .into()
    })
}

/// What the two publishes and the frame between them came to.
fn what_it_came_to(
    before: &SimSnapshot,
    after: &SimSnapshot,
    frames: (&Rgba8Image, &Rgba8Image),
) -> Result<Reloaded, Box<dyn Error>> {
    let (was, is) = frames;
    Ok(Reloaded {
        the_medium_published_before: before.tint,
        the_medium_published_after: after.tint,
        the_wall_stood_at: wall_stands_from(after),
        pixels_examined: EXAMINED,
        drawn_at_something_other_than_the_predicted_colour: straying_from_the_wall(is, after)?,
        the_frame_moved_when_the_published_tint_did: differing(was, is) > 0,
    })
}
