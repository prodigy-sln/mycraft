//! A block nobody had declared before is registered by the candidate that
//! declares it, and the registry answers for what that declaration said.
//!
//! # Every optional field is stated against its default
//!
//! `replaceable` is absent-means-false and `breakable` is absent-means-true, so a
//! declaration stating `replaceable = true` and `breakable = false` disagrees
//! with the defaults on both. A registry that answered from the defaults — or one
//! that registered the name and dropped the table it came with — answers
//! differently on every field this reads, which is what makes "answers for its
//! declared fields" a thing to check rather than a thing to say.
//!
//! `breaks_into` names a block the same content declares, so what is asked here
//! is that the residue was *recorded*. Whether an undeclared one is accepted is a
//! separate contract with a scenario of its own.
//!
//! # Registration and field answers only
//!
//! The texture layer this block will one day be drawn from, and the indicator
//! that shows it in a player's hand, are separate scenarios in a later phase.
//! Nothing here asks for a layer, and nothing here draws.
//!
//! # Why this drives the simulation rather than a client
//!
//! What a registry declares a block to be has no surface a client exposes: the
//! session hands out no borrow of the world and no borrow of the registry, by
//! design. The scenarios asking whether the *client* honours a newly declared
//! block — that it reaches the player's hand, and that a placement writes it —
//! live in `crates/mc-client/tests/`.

#[path = "support/roots.rs"]
mod roots;
mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::block::BlockDefinition;
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_sim::player::PlayerState;
use mc_sim::simulation::{Simulation, seat};
use mc_sim::world::World;
use mc_world::world::{VoxelWorld, WorldPos};

use roots::{AMBER, AMBER_DECLARATION, AMBER_FILE, Adoption, adoption, shipped};
use support::{DIRT, STONE, TestResult, content_registry, published_content};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The one cell this world holds, so that the candidate has a world to be
/// admitted against rather than an empty one.
const A_BLOCK: WorldPos = WorldPos { x: 1, y: 9, z: 1 };

/// Where the player stands. Nothing here is about the player.
const ABOVE_EVERYTHING: Vec3 = Vec3::new(8.5, 40.0, 8.5);

/// Everything a registry answers about one block, as owned values a scenario can
/// compare whole.
type Declared = (String, String, bool, bool, bool, Option<String>);

/// What the candidate declares `base:amber` to be, read off the declaration the
/// fixture writes rather than off the registry that answered.
fn as_declared() -> Declared {
    (
        AMBER.to_owned(),
        AMBER.to_owned(),
        true,
        true,
        false,
        Some(DIRT.to_owned()),
    )
}

#[test]
fn a_block_declared_for_the_first_time_is_registered_and_answers_for_its_declared_fields()
-> TestResult {
    let mut simulation = playing()?;
    let candidate = shipped()?.declaring(AMBER_FILE, AMBER_DECLARATION)?;

    let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(
        &mut simulation,
        candidate.candidate()?,
    ));
    require_admitted(&answered)?;

    assert_eq!(
        declared(&simulation, AMBER),
        Some(as_declared()),
        "a block an author has just invented has to exist in the content now serving, and it has \
         to exist as the thing they wrote — every field of it, including the two whose defaults \
         say the opposite of what this declaration says. A reload that registered the name and \
         answered from the defaults would leave an author's declaration silently half-read, with \
         the block they can see in the world behaving like one they did not write"
    );
    Ok(())
}

/// A simulation holding one block, named against the shipped content.
fn playing() -> Result<Simulation, Box<dyn Error>> {
    let registry = Arc::new(content_registry()?);
    let mut blocks = VoxelWorld::empty(COLUMNS);
    blocks.set_block(A_BLOCK, &BlockName::parse(STONE)?, &registry)?;
    let spawn = PlayerState {
        position: ABOVE_EVERYTHING,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    };
    let content = published_content(&registry)?;
    Ok(seat(spawn, World::new(blocks, registry)?, content).simulation)
}

/// Refuses unless the candidate was admitted at all.
///
/// A guard rather than part of the claim: which block a player ends up holding
/// once a solid one is registered ahead of the rest is a scenario of its own, and
/// folding it in here would put two claims in one assertion.
fn require_admitted(answered: &Adoption) -> Result<(), Box<dyn Error>> {
    if matches!(answered, Adoption::Accepted { .. }) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the candidate declaring {AMBER} to be admitted, and it answered \
         {answered:?}. There is no content now serving for the registration below to be read from"
    )
    .into())
}

/// What the registry the simulation's world is named against declares `block` to
/// be, or nothing where it declares no such block.
fn declared(simulation: &Simulation, block: &str) -> Option<Declared> {
    let name = BlockName::parse(block).ok()?;
    let definition: &BlockDefinition = simulation.world().registry().resolve(&name).ok()?;
    Some((
        definition.name.as_str().to_owned(),
        textured(&definition.textures),
        definition.is_solid,
        definition.replaceable,
        definition.breakable,
        definition
            .breaks_into
            .as_ref()
            .map(|residue| residue.as_str().to_owned()),
    ))
}

/// Every key a block's six facings draw from, joined — one key where all six
/// agree, and a list where they do not.
///
/// **Total over the six rather than a reading of one of them.** Every fixture in
/// this file states its texture as a single string, so the answer is one key; a
/// resolver that lost five facings, or that answered one facing's key for all six
/// while the declaration said otherwise, changes this string rather than hiding
/// behind whichever facing happened to be read.
fn textured(textures: &FaceTextures) -> String {
    textures
        .keys()
        .iter()
        .map(TextureKey::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}
