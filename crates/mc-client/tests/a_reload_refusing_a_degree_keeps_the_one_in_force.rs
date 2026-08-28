//! What a mod author reads when the degree they just typed is one the engine
//! cannot keep, and what the game goes on drawing while they fix it.
//!
//! # The refusal is the loader's, reached through a reload
//!
//! A degree above the ceiling is refused by `mc_sim::content::load` today,
//! naming the file, the block, the field and the bound, and a reload's build
//! stage is that same call on a worker. So the first reading here is a **control
//! on the path**: it reddens exactly if the reload comes to reach content
//! through something else, and a fault type written to satisfy it is the signal
//! that it has.
//!
//! **No refusal's wording is spelled here.** The expectation is asked of a
//! second read of the same root, which reaches the failure without going near
//! the reload, and the reported text has to *end* in it — so whatever framing a
//! reload puts above a refusal, the sentence naming the file survives to the
//! person. A reworded refusal moves both sides together; a *dropped* cause moves
//! only the reported side, which is the asymmetry a snapshotted string does not
//! have.
//!
//! What the scenario adds on top is the four things it requires by name. Three
//! of them — the file, the block and the field — are spelled by the fixture
//! itself, so the needle is what the author typed rather than what the loader
//! answers. **The fourth has no such independent spelling**: the bound is `1.0`
//! and the loader writes it as a word, so it is asked for as that word. That is
//! a phrase match and it is the only one here, which is why the sentence
//! carrying it is compared whole against the second read rather than left to the
//! needle alone.
//!
//! # A refused reload has to leave the picture alone, and that is absolute
//!
//! The second reading draws the frame after the refusal and judges it against
//! the colour the degree **already in force** composites to — never against the
//! frame drawn before the edit. A refusal that quietly dropped the content, one
//! that half-applied the candidate, and one that served a resolution built from
//! a root it had just refused all draw something else; a comparison of two
//! renderings could not tell the third of those from a correct client, because
//! it would have moved both.

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

use std::error::Error;

use mc_testkit::frame::Rgba8Image;

use reload_opacity::{
    HALF, PANE, PANE_FILE, PAST_THE_CEILING, PIXELS_IN_THE_FRAME, Playing, a_client_playing,
    a_root_whose_pane_declares, drawn_against, pane_declaring, the_four_colours,
};
use reload_remesh::resolution_serving;
use reload_watch::{
    Attempt, Refusal, block_path, declaration_named, naming, refusal, restating_raw,
    the_loaders_own_words, until_settled,
};
use support::TestResult;
use support::pixel_census::{Presence, census, owed, require_told_apart};
use support::translucency::TELLS_THEM_APART;

/// The field the edit below gets wrong, and the bound it goes past as the loader
/// writes one.
const THE_FIELD: &str = "opacity";
const THE_CEILING: &str = "one";

/// The two presences these readings name.
const MANY: Presence = Presence::AtLeastMany;
const NONE: Presence = Presence::NotOnce;

#[test]
fn a_reload_stating_a_degree_past_the_ceiling_is_refused_naming_the_file_block_field_and_bound()
-> TestResult {
    let run = a_run_whose_reload_states_a_degree_past_the_ceiling()?;

    assert_eq!(
        refusal(
            &run.crossed,
            &run.words,
            &naming(&[&declaration_named(PANE_FILE), PANE, THE_FIELD, THE_CEILING])
        ),
        Refusal::NamedEverythingAsked,
        "`{PAST_THE_CEILING}` is a number the engine understands and cannot keep, so it is refused \
         rather than clamped to the bound — a clamp would register a block that stops everything \
         while the author's file says otherwise, and leave them looking for the mistake in a \
         declaration that loaded. All four of the file, the block, the field and the bound have to \
         reach them, because a degree refused without the bound named leaves an author guessing at \
         what scale the number runs on"
    );
    Ok(())
}

#[test]
fn a_refused_reload_leaves_the_pane_drawing_at_the_degree_that_was_already_in_force() -> TestResult
{
    let expected = the_four_colours();
    require_told_apart(&expected, TELLS_THEM_APART)?;
    let run = a_run_whose_reload_states_a_degree_past_the_ceiling()?;
    let Some(frame) = run.frame else {
        return Ok(());
    };

    let counted = census(&frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (
            refusal(&run.crossed, &run.words, &[]),
            counted.considered,
            counted.shown.clone(),
            counted.strayed,
        ),
        (
            Refusal::NamedEverythingAsked,
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[MANY, NONE, NONE, NONE]),
            NONE,
        ),
        "the half is what the client took up at launch and it is what the frame after the refusal \
         has to be made of, pixel for pixel. The first element is the premise stated inside the \
         assertion rather than propagated out of it: exactly one refusal, ending in the words a \
         second read of the same root produces. A client that dropped the content draws the \
         floor's own colour, one that half-applied the candidate draws the pane's, and one that \
         served a resolution built from the root it had just refused draws something no line here \
         names. First stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

/// What one run whose author typed a degree past the ceiling came to.
struct Refused {
    /// Every tick boundary the run crossed, and what each reported.
    crossed: Vec<Option<Attempt>>,
    /// The words a second read of the same root produces, reached without going
    /// near the reload.
    words: String,
    /// The frame drawn afterwards against whatever the client is still serving,
    /// or nothing where the opt-in permitted the absence of a device.
    frame: Option<Rgba8Image>,
}

/// A client launched with the pane at half a degree, whose author then writes a
/// degree the engine cannot keep.
///
/// **The frame is drawn on both readings' behalf rather than on one's**, so the
/// two are the same run seen twice — a refusal that named everything asked and a
/// picture taken after it, rather than two runs that might have differed.
///
/// # Errors
///
/// Returns the root, world, mesh, packing or capture failure, and the fixture's
/// own refusal where the edited root turns out to read — a root that is accepted
/// is not one a refusal can be compared against.
fn a_run_whose_reload_states_a_degree_past_the_ceiling() -> Result<Refused, Box<dyn Error>> {
    let root = a_root_whose_pane_declares(Some(HALF))?;
    let Playing {
        mut client,
        reports,
        meshed,
    } = a_client_playing(&root)?;
    let root = restating_raw(root, PANE_FILE, &pane_declaring(Some(PAST_THE_CEILING)))?;
    let words = the_loaders_own_words(root.path())?;

    reports.changed(&[block_path(&root, PANE_FILE)])?;
    let crossed = until_settled(&mut client);
    let serving = resolution_serving(&client)?;

    Ok(Refused {
        crossed,
        words,
        frame: drawn_against(&meshed, &serving)?,
    })
}
