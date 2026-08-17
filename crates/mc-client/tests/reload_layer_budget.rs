//! The end of the session's layer budget: the layer that still fits, the one that
//! does not, and what an author is told when they run out.
//!
//! # Never renumbering means never reclaiming, and 256 is the whole session
//!
//! Eight bits of the packed vertex carry a layer index, so one session may hand
//! out 256 layers and no more. A session that reloads a hundred times, each
//! adding and removing one texture key, has spent a hundred of them — and the
//! only thing that gives them back is a relaunch. Running out has to be a refusal
//! naming the counts and the way out, because the alternative is a wrong picture
//! with no error anywhere.
//!
//! # A session near its budget is built by appending, not by constructing one
//!
//! Reaching 255 assigned layers organically takes two hundred and fifty reloads.
//! `support::reload_content` gets there through `LayerAssignment::appending` — the
//! only door into the type — over keys that all sort after the four the shipped
//! root declares, so the shipped four are still on the layers a launch would have
//! given them and every expectation here stays arithmetic.
//!
//! # Two of these read the build stage and one reads the client, and that split is
//! forced
//!
//! A candidate that will not fit is refused where the content root is read, before
//! any client is offered anything, so there is nothing for a `Session` to answer.
//! In this increment a refusal reaches nobody — the reporting is a later phase's —
//! so the two scenarios about being turned away read the refusal at the door that
//! produces it. The one about the layer that *does* fit drives the client, because
//! what it asks is what the session goes on publishing afterwards.
//!
//! # The refusal's wording is written out here on purpose
//!
//! `docs/modding/hot-reload.md` is going to quote this sentence, so it is spelled
//! out below rather than assembled from anything the loader owns: an expectation
//! built out of the value under test says whatever that value became. The two
//! counts inside it are arithmetic over the fixture, and the budget is the declared
//! 256 a session may assign — see [`A_SESSIONS_BUDGET`] for where that number comes
//! from and why it is written out.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::sync::Arc;

use mc_sim::simulation::PublishedContent;

use input::InputHarness;
use reload::{
    AMBER, AMBER_FILE, Declaration, STONE, accepted, adoption, amber, declaring, shipped,
};
use reload_content::{
    BERYL, BERYL_FILE, Reading, candidate_against, fresh_layers, layers_beside, publishing,
    reading, spent_all, spent_all_but_one,
};
use reload_world::{floor_of, playing_serving, standing};
use support::{TestResult, content_root};

/// How many array-texture layers one session may assign: the declared budget.
///
/// **Written out rather than read from `LAYERS_A_SESSION_MAY_ASSIGN`.** That
/// constant is the declaration under test, and a message assembled from it would
/// read back whatever it became. The value is the packed vertex's eight-bit layer
/// field — two to the eighth.
const A_SESSIONS_BUDGET: usize = 256;

/// How many layers a session that has spent all but one has spent, and how many
/// one that has spent them all has.
const ALL_BUT_ONE_SPENT: u16 = (A_SESSIONS_BUDGET - 1) as u16;
const ALL_SPENT: u16 = A_SESSIONS_BUDGET as u16;

/// The layer a session with one still free hands the next key it meets.
///
/// The count already spent, which is also the last index the budget holds —
/// derived, so this is the 256th layer without anybody writing 255.
const THE_LAST_LAYER_THE_BUDGET_HOLDS: u16 = ALL_BUT_ONE_SPENT;

#[test]
fn a_candidate_needing_a_layer_past_the_budget_is_refused_naming_the_counts_and_the_way_out()
-> TestResult {
    let spent = spent_all()?;
    let root = shipped()?.declaring_block(AMBER_FILE, &amber().text())?;

    let read = reading(&root, &spent)?;

    assert_eq!(
        read,
        Reading::Refused {
            said: over_budget(A_SESSIONS_BUDGET + 1, A_SESSIONS_BUDGET)
        },
        "the session has handed out every layer it has, and the author has just declared a block \
         needing one more. What they are told has to carry all three things they can act on: how \
         many this content needs, how many a session has, and that relaunching gives back every \
         layer retired since the client started — because without that last sentence the only \
         reading left is that their content is too big, which is not what happened"
    );
    Ok(())
}

#[test]
fn a_candidate_needing_exactly_the_last_layer_the_budget_holds_is_taken_up() -> TestResult {
    let mut client = a_client_having_spent_all_but_one()?;
    let root = declaring(shipped()?, AMBER_FILE, &amber())?;
    let candidate = candidate_against(&root, client.content())?;

    let answered = adoption(client.adopt(candidate));
    let published = publishing(client.content())?;

    assert_eq!(
        (answered, published.layers, published.spent),
        (
            accepted(AMBER),
            layers_beside(&[(AMBER, THE_LAST_LAYER_THE_BUDGET_HOLDS)])?,
            ALL_SPENT
        ),
        "one layer of the session's budget is left and the candidate needs exactly it, so it fits \
         and is taken up. The bound is on what a session may *assign*, so the last layer is as \
         assignable as the first — a comparison one out either way refuses the candidate that fits \
         or accepts the one that does not, and the second of those is a layer index the array \
         texture has no room for"
    );
    Ok(())
}

#[test]
fn a_candidate_introducing_two_keys_with_one_layer_left_appends_neither_of_them() -> TestResult {
    let client = a_client_having_spent_all_but_one()?;
    let root = shipped()?
        .declaring_block(AMBER_FILE, &amber().text())?
        .declaring_block(BERYL_FILE, &Declaration::of(BERYL).text())?;

    let read = reading(&root, serving(&client)?.resolved.layers())?;
    let published = publishing(client.content())?;

    assert_eq!(
        (read, published.layers, published.spent),
        (
            Reading::Refused {
                said: over_budget(A_SESSIONS_BUDGET + 1, A_SESSIONS_BUDGET - 1)
            },
            fresh_layers()?,
            ALL_BUT_ONE_SPENT
        ),
        "one of the two keys would fit and the other would not, and appending the one that fits is \
         the outcome this forbids: the author would be told their content was refused while a \
         layer had quietly gone, and the block that got one would draw while the block beside it \
         drew nothing. Neither is appended, so the session still states the four layers a launch \
         gave it and still has one left for whichever candidate lands next"
    );
    Ok(())
}

/// What a refusal over the session's budget says, in the words a page quotes.
///
/// The counts are the caller's arithmetic; the budget is `A_SESSIONS_BUDGET`.
fn over_budget(needed: usize, spent: usize) -> String {
    format!(
        "this content needs {needed} texture layers and a session has {A_SESSIONS_BUDGET}; \
         {spent} are already assigned, and relaunching reclaims every layer retired since the \
         client started"
    )
}

/// A client playing a one-column floor of stone whose session has already spent
/// every layer of its budget but one.
fn a_client_having_spent_all_but_one() -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing_serving(
        &content_root()?,
        standing(),
        |registry| floor_of(registry, STONE),
        &spent_all_but_one()?,
    )?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// The content `client` is publishing.
///
/// # Errors
///
/// Returns an error where the client publishes none, which is a client with no
/// world rather than one serving anything a candidate could be read against.
fn serving(client: &InputHarness) -> Result<Arc<PublishedContent>, Box<dyn Error>> {
    client.content().ok_or_else(|| {
        "this fixture's client publishes no content, so there are no spent layers to read a \
         candidate against"
            .into()
    })
}
