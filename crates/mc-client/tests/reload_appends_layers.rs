//! Which array-texture layer a texture key holds once content has been read
//! again under a running client.
//!
//! # Appended, never renumbered, and the falsifier is which way `base:amber`
//! sorts
//!
//! A layer index rides inside every packed vertex. Handed out as a key's position
//! in the sorted key set, one new block renumbers every index after it and the
//! whole world is textured wrong — silently, with no error anywhere, and not
//! localised to the block that caused it. `base:amber` sorts *before*
//! `base:dirt`, so a client that re-derived its assignment would put amber on
//! layer 0 and push the four blocks already on the GPU up by one. Every scenario
//! here reads the whole assignment rather than one entry, so that shift is what a
//! failure reports.
//!
//! # Every scenario drives the client, and the candidate is built against what it
//! is publishing
//!
//! The assignment can be perfectly correct inside the simulation and never reach
//! the reader: it travels in the published content, and the client is what asks
//! for it. So every reading below goes through `Session`, and every candidate is
//! read against the layers the client says it has already spent — which is what a
//! reload's build stage does. A fixture handing over an assignment of its own
//! would be grading itself.
//!
//! # A retired layer is spent and is not live, and that is asserted rather than
//! described
//!
//! The two scenarios about a key going away read the count of layers the session
//! has spent beside the map of live ones. Those two numbers part company the
//! moment a key retires, and an implementation deriving the count from the live
//! entries would answer one short — which is exactly what hands a reintroduced
//! key back the layer it used to hold.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::collections::BTreeMap;
use std::error::Error;

use input::InputHarness;
use reload::{
    AMBER, AMBER_FILE, DIRT, GRASS, GRASS_FILE, STONE, accepted, adoption, amber, declaring,
    holding_blocks_it_does_not_declare, shipped,
};
use reload_content::{
    THE_NEXT_UNUSED_LAYER, candidate_against, layers_beside, layers_without, publishing,
};
use reload_world::{floor_of, playing, standing};
use support::{TestResult, content_root};

#[test]
fn a_texture_key_declared_for_the_first_time_takes_the_first_layer_nothing_holds() -> TestResult {
    let mut client = a_client_over(STONE)?;
    let root = declaring(shipped()?, AMBER_FILE, &amber())?;
    let candidate = candidate_against(&root, client.content())?;

    let answered = adoption(client.adopt(candidate));

    assert_eq!(
        (answered, publishing(client.content())?.layers),
        (
            accepted(AMBER),
            layers_beside(&[(AMBER, THE_NEXT_UNUSED_LAYER)])?
        ),
        "`base:amber` sorts ahead of every key the session already assigned, so a client \
         re-deriving its layers from a sort would give amber layer zero and move all four blocks \
         already drawn up by one — the whole world textured wrong with nothing reporting it. \
         Appending gives amber the first layer nothing holds and leaves the four where they were, \
         which is what keeps every vertex already uploaded valid"
    );
    Ok(())
}

#[test]
fn a_key_no_declaration_names_any_more_leaves_every_remaining_key_on_the_layer_it_held()
-> TestResult {
    let mut client = a_client_over(STONE)?;
    let root = shipped()?.not_declaring_blocks(&[GRASS_FILE])?;
    let candidate = candidate_against(&root, client.content())?;

    let answered = adoption(client.adopt(candidate));
    let published = publishing(client.content())?;

    assert_eq!(
        (answered, published.layers, published.spent),
        (
            accepted(DIRT),
            layers_without(GRASS)?,
            THE_NEXT_UNUSED_LAYER
        ),
        "the author has stopped declaring the block that held the second layer, and no cell holds \
         it, so the candidate is taken up. Stone and water keep 2 and 3 rather than sliding down \
         to 1 and 2 — a slide would retexture every stone and water face on the GPU. The count \
         spent stays where it was while one fewer key is live, because a retired layer is spent \
         and is not live, and a count derived from the live entries is what hands the next key a \
         layer somebody is still sampling"
    );
    Ok(())
}

#[test]
fn a_key_taken_away_and_then_declared_again_gets_a_layer_it_has_never_held() -> TestResult {
    let mut client = a_client_over(STONE)?;
    let without_it = shipped()?.not_declaring_blocks(&[GRASS_FILE])?;
    let dropping = candidate_against(&without_it, client.content())?;
    let retired = adoption(client.adopt(dropping));

    let restored = shipped()?;
    let declaring_it_again = candidate_against(&restored, client.content())?;
    let declared_again = adoption(client.adopt(declaring_it_again));
    let published = publishing(client.content())?;

    assert_eq!(
        (retired, declared_again, published.layers, published.spent),
        (
            accepted(DIRT),
            accepted(DIRT),
            grass_on_a_layer_it_never_held()?,
            THE_NEXT_UNUSED_LAYER + 1
        ),
        "the key is back and the layer it used to hold is not. Vertices uploaded while it was \
         retired still sample layer 1 and still name the block that was drawn there, so handing \
         the key back its old layer draws the reintroduced block over sections nobody re-meshed. \
         It takes the first layer nothing has ever held instead, and the session has now spent one \
         more than a launch does"
    );
    Ok(())
}

#[test]
fn a_refused_candidate_that_would_have_needed_a_layer_leaves_the_next_one_unspent() -> TestResult {
    let mut client = a_client_over(GRASS)?;
    let dropping_a_held_block = shipped()?
        .declaring_block(AMBER_FILE, &amber().text())?
        .not_declaring_blocks(&[GRASS_FILE])?;
    let turned_away = candidate_against(&dropping_a_held_block, client.content())?;
    let refused = adoption(client.adopt(turned_away));

    let corrected = declaring(shipped()?, AMBER_FILE, &amber())?;
    let saved_again = candidate_against(&corrected, client.content())?;
    let took_it_up = adoption(client.adopt(saved_again));

    assert_eq!(
        (refused, took_it_up, publishing(client.content())?.layers),
        (
            holding_blocks_it_does_not_declare(&[GRASS]),
            accepted(AMBER),
            layers_beside(&[(AMBER, THE_NEXT_UNUSED_LAYER)])?
        ),
        "the refused candidate declared a texture key nothing had assigned, and the world holds a \
         block it stopped declaring, so it is turned away. The author corrects the second mistake \
         and saves again: the key they invented takes the layer it would have taken the first \
         time. A refused attempt that spent a layer anyway leaks the session's budget away one \
         typo at a time, with every other scenario green"
    );
    Ok(())
}

/// A client playing a one-column floor of `floor`, with the shipped content root
/// serving and no layer of the session's budget spent but the four a launch
/// assigns.
fn a_client_over(floor: &'static str) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(&content_root()?, standing(), |registry| {
        floor_of(registry, floor)
    })?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// The layers a session states once the second key has been retired and declared
/// again: the launch's four with that key off the layer it held and on the first
/// layer nothing has ever held.
///
/// # Errors
///
/// Returns an error if the shipped key list is not the one this arithmetic rests
/// on.
fn grass_on_a_layer_it_never_held() -> Result<BTreeMap<String, u16>, Box<dyn Error>> {
    let mut layers = layers_without(GRASS)?;
    layers.insert(GRASS.to_owned(), THE_NEXT_UNUSED_LAYER);
    Ok(layers)
}
