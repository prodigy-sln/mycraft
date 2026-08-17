//! What a change of declared solidity does to the player standing on it, and
//! what a refused one does not.
//!
//! # Solidity is content's word and the physics has no opinion of its own
//!
//! Nothing in the engine treats a name as implicitly solid or implicitly not, so
//! a block that stops a player stops being one the moment the declaration says
//! so, and a block that never did starts. Both directions are here, because an
//! implementation that only ever cleared bits satisfies one of them.
//!
//! # A refused candidate is the sharpest of the three
//!
//! A swap has to settle everything it could refuse over **before** it writes
//! anything, so that a refusal leaves the world untouched by construction rather
//! than by care. The last scenario hands over a candidate that takes stone's
//! solidity away *and* stops declaring a block the world holds: the refusal is
//! about the second, and the first must not have happened. A swap that wrote the
//! registry first and checked afterwards leaves the player falling through a
//! floor over a candidate nobody accepted.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::path::Path;

use mc_core::id::BlockName;
use mc_sim::player::PlayerState;
use mc_sim::simulation::{SimSnapshot, Simulation};

use input::InputHarness;
use reload::{
    Adoption, Declaration, GRASS, GRASS_FILE, STONE, STONE_FILE, WATER, WATER_FILE, adoption,
    candidate, holding_blocks_it_does_not_declare, restating, shipped, stone_that_is_not_solid,
};
use reload_world::{
    FLOOR, at_rest, falling_from, floor_holding, floor_of, moving_at, playing, resting, standing,
};
use support::{TestResult, content_root};

/// Where the player's feet stand while a floor is holding them up.
const ON_THE_FLOOR: f32 = 10.0;

/// Where the falling scenario drops from, and how many ticks it falls before the
/// candidate arrives — few enough that it is still well above the floor.
const DROPPED_FROM: f32 = 12.0;
const FALLING_TICKS: u32 = 6;

/// How many ticks each scenario advances after the candidate, so a player left
/// unsupported is plainly through the floor rather than a fraction below it.
const AFTERWARDS: u32 = 60;

/// A floor cell nowhere near the player, held by a block the last scenario's
/// candidate stops declaring.
const A_PATCH_OF_GRASS: (i32, i32, i32) = (2, FLOOR, 2);

#[test]
fn stone_declared_not_solid_stops_holding_the_player_up() -> TestResult {
    let mut client = a_client_on(&content_root()?, STONE, standing())?;
    let stood = where_it_stands(&client)?;

    let root = restating(shipped()?, STONE_FILE, &stone_that_is_not_solid())?;
    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    client.ticks(AFTERWARDS);

    assert_eq!(
        (stood, through_the_floor(&client)?),
        ((ON_THE_FLOOR.to_bits(), true), true),
        "the player was standing on stone because content said stone was solid, and content has \
         stopped saying it. Falling *through* the layer is the reading and not merely leaving the \
         ground: a swap that cleared the one cell under their feet and left the rest of the floor \
         alone would look identical for a tick and then catch them"
    );
    Ok(())
}

#[test]
fn water_declared_solid_starts_holding_the_player_up() -> TestResult {
    let mut client = a_client_on(&content_root()?, WATER, falling_from(DROPPED_FROM))?;
    client.ticks(FALLING_TICKS);
    require_still_falling(&client)?;

    let root = restating(
        shipped()?,
        WATER_FILE,
        &Declaration::of(WATER).solid(true).replaceable(true),
    )?;
    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    client.ticks(AFTERWARDS);

    assert_eq!(
        where_it_stands(&client)?,
        (ON_THE_FLOOR.to_bits(), true),
        "nothing in the engine decides that a block called water cannot be stood on — content \
         does, and an author who changes their mind is answered. The player was falling through it \
         a moment ago and comes to rest on its top face, which is where a floor of it puts them: an \
         implementation that only ever *cleared* solidity on a reload satisfies the other direction \
         and fails here"
    );
    Ok(())
}

#[test]
fn a_candidate_refused_for_another_reason_leaves_stone_holding_the_player_up() -> TestResult {
    let mut client = a_client_on_a_floor_with_grass(&content_root()?)?;

    let root = restating(shipped()?, STONE_FILE, &stone_that_is_not_solid())?
        .not_declaring_blocks(&[GRASS_FILE])?;
    let answered = adoption(client.adopt(candidate(root.path())?));
    client.ticks(AFTERWARDS);

    assert_eq!(
        (answered, where_it_stands(&client)?),
        (
            holding_blocks_it_does_not_declare(&[GRASS]),
            (ON_THE_FLOOR.to_bits(), true)
        ),
        "this candidate says two things and one of them is refusable, so neither of them happens. \
         A swap that wrote the registry and checked the world afterwards would drop the player \
         through a floor on the strength of an edit nobody accepted — and it would leave the world \
         named against a registry it was never resolved with, which is the disagreement the type \
         exists to make unspellable"
    );
    Ok(())
}

/// A client on a floor made of `block`, at `spawn`, with the root at `root`
/// serving.
fn a_client_on(
    root: &Path,
    block: &'static str,
    spawn: PlayerState,
) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, spawn, |registry| floor_of(registry, block))?;
    Ok(playing_client(simulation, holding))
}

/// A client standing on a floor of stone with one cell of grass in it, so that
/// the world holds a block a candidate can stop declaring.
fn a_client_on_a_floor_with_grass(root: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, standing(), |registry| {
        floor_holding(registry, STONE, &[(A_PATCH_OF_GRASS, GRASS)])
    })?;
    Ok(playing_client(simulation, holding))
}

/// A started client already playing what it was handed.
fn playing_client(simulation: Simulation, holding: BlockName) -> InputHarness {
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client
}

/// Whatever the client has published, by value: a tick, a pose and a player
/// state, all plain values.
///
/// # Errors
///
/// Returns an error where it has published nothing, which is a client with no
/// world rather than one standing anywhere.
fn published(client: &InputHarness) -> Result<SimSnapshot, Box<dyn Error>> {
    client
        .published()
        .map(|published| *published)
        .ok_or_else(|| "this fixture's client has published no tick to read".into())
}

/// Where the client's player stands and whether the world is holding it up.
///
/// # Errors
///
/// Returns an error where the client has published nothing.
fn where_it_stands(client: &InputHarness) -> Result<(u32, bool), Box<dyn Error>> {
    let (height, on_ground) = resting(&published(client)?);
    Ok((height.to_bits(), on_ground))
}

/// Whether the player's feet have gone below the floor layer altogether.
///
/// # Errors
///
/// Returns an error where the client has published nothing.
fn through_the_floor(client: &InputHarness) -> Result<bool, Box<dyn Error>> {
    let (height, _) = resting(&published(client)?);
    Ok(height < FLOOR as f32)
}

/// Refuses unless the player is still in the air and above the floor.
fn require_still_falling(client: &InputHarness) -> Result<(), Box<dyn Error>> {
    let (height, on_ground) = resting(&published(client)?);
    if on_ground || height <= ON_THE_FLOOR {
        return Err(format!(
            "this fixture has to leave the player falling and above the floor when the candidate \
             arrives, and they are at {height} with the ground under them reported as {on_ground}. \
             A player already resting there would satisfy the reading below whatever the candidate \
             said"
        )
        .into());
    }
    if moving_at(&published(client)?) == at_rest() {
        return Err(ALREADY_AT_REST.into());
    }
    Ok(())
}

/// What a fixture whose player was not moving is told.
const ALREADY_AT_REST: &str =
    "this fixture has to leave the player moving when the candidate arrives, and they are at rest";

/// Refuses unless the candidate was admitted at all.
fn require_admitted(answered: &Adoption) -> Result<(), Box<dyn Error>> {
    if matches!(answered, Adoption::Accepted { .. }) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the candidate to be admitted, and the client answered {answered:?}. \
         Nothing about what stops a player would have changed"
    )
    .into())
}
