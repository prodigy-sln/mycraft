//! Which sections a reload leaves to be meshed again: every one of them, or none.
//!
//! # One instrument, and it is the set the frame path drains
//!
//! Every reading here is `Session::take_remesh_work`, which is what the client's
//! own frame path asks and what the re-mesh worker is handed. A test that asked
//! the simulation, or that meshed the world and compared pictures, would walk
//! straight past the marking — and the marking is the whole of what this task
//! adds.
//!
//! **The set is taken, so it is read exactly once per reload** and held in a
//! value. A second ask would find it empty, and a scenario reading that would call
//! it "nothing was marked".
//!
//! # 256 is derived and never written
//!
//! The count comes from the footprint's own two declarations — `FOOTPRINT_COLUMNS`
//! squared, times `SECTIONS_PER_COLUMN` — through
//! `reload_remesh::EVERY_SECTION_OF_THE_SHIPPED_WORLD`, and the *set* is derived
//! the same way, so an implementation marking the right number of the wrong
//! sections fails as loudly as one marking too few.
//!
//! **No expectation here is ~82, and that is deliberate rather than an
//! approximation.** The rule the architecture measured into place is binary: a
//! candidate changing some block's declared solidity or declared texture key, or
//! adding or removing a block, marks *every* section, and one changing neither
//! marks none. A count derived from "the sections whose own or whose neighbours'
//! blocks include stone" is a lower bound on that set and would redden against a
//! conforming implementation, whose cheapest repair narrows the rule and silently
//! breaks the exactly-256 bound in the same commit.
//!
//! # The player stands in open air, and nothing they do writes to the world
//!
//! The spawn is above the landmark pillar's top, so no tick these scenarios advance
//! can edit a cell and no mark can arrive from anywhere but the reload. Each
//! scenario also drains before it reloads, which is both a guard that a launch
//! leaves nothing outstanding and the reason the reading afterwards is the
//! reload's alone.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_remesh.rs"]
mod reload_remesh;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;

use glam::Vec3;

use input::InputHarness;
use reload::{
    DIRT, DIRT_FILE, Declaration, GRASS, GRASS_FILE, STONE, STONE_FILE, WATER, WATER_FILE,
    accepted, adoption, candidate, restating, shipped, shipped_restating_stone,
    stone_that_is_not_solid,
};
use reload_remesh::{Marking, a_client_over, every_section_once, marked, require};
use reload_world::{published_tick, shipped_world, standing_at};
use support::content::ContentRoot;
use support::{TestResult, content_root};

/// Where the player stands: over the landmark pillar's top, in open air.
///
/// Nothing a tick does from here writes to a cell, so every mark these scenarios
/// read is the reload's.
const IN_OPEN_AIR: Vec3 = Vec3::new(8.5, 70.0, 8.5);

/// How many ticks are advanced with the whole world marked.
///
/// More than one, so a boundary really was crossed while the re-mesh was
/// outstanding; the number itself carries nothing else.
const TICKS_ACROSS_THE_RELOAD: u32 = 3;

#[test]
fn a_candidate_touching_neither_solidity_nor_a_texture_key_leaves_no_section_to_mesh() -> TestResult
{
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let unbreakable = shipped_restating_stone(&Declaration::of(STONE).breakable(false))?;

    let answered = adoption(client.adopt(candidate(unbreakable.path())?));
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (answered, left_to_mesh),
        (accepted(DIRT), Marking::NoSectionAtAll),
        "`breakable` decides what a click does and nothing about what is drawn, so a reload that \
         changes it alone has no picture to correct. **This is satisfied by an implementation that \
         never meshes anything on any reload**, which is why it is read on the same instrument as \
         the scenario below it: the discrimination is that one of the two marks nothing and the \
         other marks the world"
    );
    Ok(())
}

#[test]
fn a_candidate_taking_stones_solidity_away_leaves_every_section_of_the_world_to_mesh() -> TestResult
{
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let softened = shipped_restating_stone(&stone_that_is_not_solid())?;
    let before = published_at(&client);

    let answered = adoption(client.adopt(candidate(softened.path())?));
    let crossed = client.ticks(TICKS_ACROSS_THE_RELOAD);
    let advanced = ticks_between(before, published_at(&client));
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (answered, advanced, crossed.len(), left_to_mesh),
        (
            accepted(DIRT),
            Some(TICKS_ACROSS_THE_RELOAD),
            TICKS_ACROSS_THE_RELOAD as usize,
            every_section_once()
        ),
        "stone's solidity decides which of its neighbours' faces are drawn, so the picture is wrong \
         everywhere until the world is meshed again. The set has to contain every stone-bearing \
         section and every neighbour of one, and on this instrument that set is the whole world — \
         the sections a selective rule would leave out are the empty ones, which mesh to nothing. \
         The middle of this comparison is the scenario's other half: the ticks went on being \
         advanced while the whole world stood marked"
    );
    Ok(())
}

#[test]
fn a_candidate_changing_all_four_declarations_leaves_each_section_of_the_world_once() -> TestResult
{
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let rewritten = shipped_restating_every_declaration()?;

    let answered = adoption(client.adopt(candidate(rewritten.path())?));
    let left_to_mesh = marked(&mut client);
    let left_over = marked(&mut client);

    assert_eq!(
        (answered, left_to_mesh, left_over),
        (
            accepted(DIRT),
            every_section_once(),
            Marking::NoSectionAtAll
        ),
        "exactly the world's sections, each once, for that one reload: the marked arm requires the \
         set to be the footprint's own and to hold no key twice, and the second reading requires \
         one reload to leave one batch rather than a section re-marked for the rest of the run. A \
         count alone could not tell 256 sections from 128 of them marked twice"
    );
    Ok(())
}

#[test]
fn a_reload_that_changes_no_geometry_and_one_that_does_are_told_apart_on_one_instrument()
-> TestResult {
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let unbreakable = shipped_restating_stone(&Declaration::of(STONE).breakable(false))?;
    let softened = shipped_restating_stone(&stone_that_is_not_solid())?;

    let first = adoption(client.adopt(candidate(unbreakable.path())?));
    let after_the_first = marked(&mut client);
    let second = adoption(client.adopt(candidate(softened.path())?));
    let after_the_second = marked(&mut client);

    assert_eq!(
        (first, after_the_first, second, after_the_second),
        (
            accepted(DIRT),
            Marking::NoSectionAtAll,
            accepted(DIRT),
            every_section_once()
        ),
        "two candidates, one session, one instrument. **Without this pairing, an implementation \
         that meshes nothing on any reload satisfies both the no-section scenario and the \
         exactly-256 one** — the first because it is right and the second because nothing would be \
         comparing them. What is graded here is the discrimination itself: `breakable` moves \
         nothing and declared solidity moves everything"
    );
    Ok(())
}

/// Which tick `client` last published, or nothing where it has published none.
fn published_at(client: &InputHarness) -> Option<u32> {
    client
        .published()
        .map(|published| published_tick(&published))
}

/// How many ticks stand between two readings of what a client published.
fn ticks_between(before: Option<u32>, after: Option<u32>) -> Option<u32> {
    before
        .zip(after)
        .map(|(before, after)| after.saturating_sub(before))
}

/// A client playing the world it launches into, over the shipped content root.
fn a_client_over_the_shipped_world() -> Result<InputHarness, Box<dyn Error>> {
    a_client_over(&content_root()?, standing_at(IN_OPEN_AIR), shipped_world)
}

/// Refuses unless the client has nothing outstanding to mesh.
///
/// **Both a guard and the reason the reading afterwards means anything**: a launch
/// that left sections marked would make every count below the reload's plus
/// something else, and this is also the drain that leaves the set empty for the
/// reload to fill.
fn require_nothing_outstanding(client: &mut InputHarness) -> Result<(), Box<dyn Error>> {
    let outstanding = marked(client);
    require(
        outstanding == Marking::NoSectionAtAll,
        format!(
            "this scenario reads what one reload left to be meshed, so the launch has to have left \
             nothing — and it left {outstanding:?}"
        ),
    )
}

/// A copy of the shipped root in which every one of the four declarations says
/// something the shipped one does not.
///
/// Two of the four changes are geometry — stone's solidity taken away, water's
/// given — and two are not, because the scenario is about all four declarations
/// changing rather than about all four changing the picture. Dirt and grass stay
/// solid, so the candidate still registers a block a player could place.
fn shipped_restating_every_declaration() -> Result<ContentRoot, Box<dyn Error>> {
    let root = restating(
        shipped()?,
        DIRT_FILE,
        &Declaration::of(DIRT).breakable(false),
    )?;
    let root = restating(root, GRASS_FILE, &Declaration::of(GRASS).replaceable(true))?;
    let root = restating(root, STONE_FILE, &Declaration::of(STONE).solid(false))?;
    restating(root, WATER_FILE, &Declaration::of(WATER).solid(true))
}
