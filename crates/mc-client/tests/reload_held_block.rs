//! What the player is holding after the content changed under them.
//!
//! # The block in the hand is re-derived, never carried over
//!
//! It is a policy over the registry standing in for an inventory — the first
//! solid block in registration order — and not something the player accumulated.
//! Re-deriving it is the only way a block somebody has just invented becomes
//! something they can go and place before the next launch.
//!
//! # Every scenario here drives the client, and one of them drives it twice
//!
//! The re-derivation can be perfectly correct inside the simulation and never
//! reach the player: the session holds the block a place request names in a field
//! of its own, and nothing else writes it. So the block is read back through the
//! client that would hand it to a placement — and the last scenario goes further
//! and makes the placement, because a session that displays a new block while
//! handing the old one to the world is wrong in exactly the way only a placement
//! can see.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::path::Path;

use mc_client::startup::PreparationError;
use mc_core::id::BlockName;
use winit::event::MouseButton;

use input::InputHarness;
use reload::{
    AMBER, AMBER_FILE, Adoption, DIRT, Declaration, STONE, STONE_FILE, accepted, adoption, amber,
    candidate, declaring, restating, shipped, shipped_declaring_nothing_solid,
};
use reload_world::{
    AIM_AT_THE_FAR_CELL, NOTHING, OVER_THE_FAR_CELL, edit, floor_of, playing, resting, standing,
    wrote,
};
use support::{TestResult, content_root};

/// How many ticks a scenario advances to show that nothing moved after a
/// refusal. Long enough that a player left unsupported would have left the floor.
const A_WHILE: u32 = 30;

/// Where the player's feet stand while the floor is holding them up.
const ON_THE_FLOOR: f32 = 10.0;

#[test]
fn a_reload_that_moves_no_solid_block_to_the_front_leaves_the_same_block_in_the_hand() -> TestResult
{
    let mut client = a_client_playing(&content_root()?)?;
    let held_before = client.held_block();
    let root = restating(
        shipped()?,
        STONE_FILE,
        &Declaration::of(STONE).breakable(false),
    )?;

    let answered = adoption(client.adopt(candidate(root.path())?));

    assert_eq!(
        (answered, named(held_before), named(client.held_block())),
        (accepted(DIRT), Some(DIRT.to_owned()), Some(DIRT.to_owned())),
        "the candidate changes what breaking stone does and nothing about which blocks are solid, \
         so the first solid block in registration order is the block it already was. A \
         re-derivation reaching for anything else — the first block registered, or whatever \
         happened to be last — would put something the player never chose in their hand every time \
         they saved a file"
    );
    Ok(())
}

#[test]
fn a_newly_declared_solid_block_registered_first_arrives_in_the_players_hand() -> TestResult {
    let mut client = a_client_playing(&content_root()?)?;
    let held_before = client.held_block();
    let root = declaring(shipped()?, AMBER_FILE, &amber())?;

    let answered = adoption(client.adopt(candidate(root.path())?));

    assert_eq!(
        (answered, named(held_before), named(client.held_block())),
        (
            accepted(AMBER),
            Some(DIRT.to_owned()),
            Some(AMBER.to_owned())
        ),
        "the author has just invented a block and its file sorts ahead of every other, so it is the \
         first solid block registered and the one a client holds. A session that kept the block it \
         was handed at launch leaves the new block unreachable until the next relaunch — the \
         opposite of what editing a file while the game runs is for"
    );
    Ok(())
}

#[test]
fn a_candidate_registering_no_solid_block_is_refused_saying_there_would_be_nothing_to_place()
-> TestResult {
    let mut client = a_client_playing(&content_root()?)?;
    let root = shipped_declaring_nothing_solid()?;

    let answered = adoption(client.adopt(candidate(root.path())?));
    client.ticks(A_WHILE);

    assert_eq!(
        (
            answered,
            named(client.held_block()),
            where_it_stands(&client)?
        ),
        (
            Adoption::NothingToPlace {
                said: PreparationError::NothingToPlace.to_string()
            },
            Some(DIRT.to_owned()),
            (ON_THE_FLOOR.to_bits(), true)
        ),
        "a content set with nothing solid in it is one a player could place nothing from, and the \
         sentence they are told has to be the sentence a launch would have told them — two \
         wordings for one condition are two places for the answer to drift. The content that was \
         serving goes on serving: the block in the hand is the block it was, and the floor is still \
         a floor"
    );
    Ok(())
}

#[test]
fn a_placement_after_a_reload_writes_the_newly_declared_block_the_client_now_holds() -> TestResult {
    let mut client = a_client_playing(&content_root()?)?;
    let root = declaring(shipped()?, AMBER_FILE, &amber())?;

    let answered = adoption(client.adopt(candidate(root.path())?));
    client.move_pointer(0.0, AIM_AT_THE_FAR_CELL);
    client.click(MouseButton::Right);
    let built = edit(client.edit());

    assert_eq!(
        (answered, built),
        (accepted(AMBER), wrote(OVER_THE_FAR_CELL, NOTHING, AMBER)),
        "what a place request names is read out of the session's own field, and the picture a \
         player is shown is read out of the same one. A session that wrote the re-derived block \
         somewhere a display could see it and handed the world the block it was launched with \
         builds the wrong block into the world with the right one drawn in the corner, and only a \
         placement can tell the two apart"
    );
    Ok(())
}

/// A client playing a floor of stone, with the content root at `root` serving.
fn a_client_playing(root: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, standing(), |registry| floor_of(registry, STONE))?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// Where the client's player stands and whether the world is holding it up.
///
/// # Errors
///
/// Returns an error where the client has published no tick, which is a client
/// with no world rather than one standing anywhere.
fn where_it_stands(client: &InputHarness) -> Result<(u32, bool), Box<dyn Error>> {
    let published = client
        .published()
        .ok_or("this fixture's client has published no tick, so it has no world to stand in")?;
    let (height, on_ground) = resting(&published);
    Ok((height.to_bits(), on_ground))
}

/// One held block as text, so that holding nothing and holding a block are told
/// apart by shape rather than by a sentinel name.
fn named(held: Option<BlockName>) -> Option<String> {
    held.map(|block| block.as_str().to_owned())
}
