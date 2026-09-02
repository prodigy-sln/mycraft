//! Which sections a reload leaves to be meshed again when a candidate changes
//! what a block **draws**: every one of them, or none.
//!
//! # This file is one half of a pair, and the seam is the question asked
//!
//! Every reading here is about a field the marking rule is keyed on — whether a
//! block is drawn, whether it hides what stands behind it, the keys its six
//! faces draw from — or about the *set* of sections such a change leaves. The
//! other half, `reload_marks_no_section.rs`, is about the fields the rule
//! deliberately excludes, and it carries its own controls. **The pair was one
//! file until it outgrew the size a test file is allowed**, and the split is by
//! that question rather than by outcome: the "same six keys restated" reading
//! below marks nothing and belongs here, because it is the control for the
//! re-pointed facing beside it and its subject is a texture key.
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
//! candidate changing what any block *draws* — whether it is drawn at all,
//! whether it hides what stands behind it, or the key any of its six faces draws
//! from — or adding or removing a block, marks *every* section, and one changing
//! none of those marks none. A count derived from "the sections whose own or
//! whose neighbours' blocks include stone" is a lower bound on that set and would
//! redden against a conforming implementation, whose cheapest repair narrows the
//! rule and silently breaks the exactly-256 bound in the same commit.
//!
//! # Solidity is not in that rule, and two scenarios below still turn on it
//!
//! Declared solidity decides whether a player falls through a cell and nothing
//! else, so a candidate that moves it alone has no picture to correct. What it
//! cannot do is move it *alone*: each of `drawn`, `occludes` and `targetable`
//! defaults to whatever its own declaration says about `solid`, so a fixture
//! stating `solid = false` and saying nothing more states drawnness and occlusion
//! as false with it. The solidity scenarios below mark the world for that reason
//! rather than because solidity is in the key — a distinction that costs nothing
//! to state and would cost a future author a wrong conclusion if it were not
//! stated. A fixture that spelled `drawn` explicitly beside `solid` in one of
//! them would quietly change what the scenario is about.
//!
//! # The player stands in open air, and nothing they do writes to the world
//!
//! The spawn is above the landmark pillar's top, so no tick these scenarios advance
//! can edit a cell and no mark can arrive from anywhere but the reload. Each
//! scenario also drains before it reloads, which is both a guard that a launch
//! leaves nothing outstanding and the reason the reading afterwards is the
//! reload's alone. Both are `support/marks_sections.rs`, shared with the other
//! half so the two suites cannot launch into different worlds.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/marks_sections.rs"]
mod marks_sections;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_remesh.rs"]
mod reload_remesh;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;

use input::InputHarness;
use marks_sections::{IN_OPEN_AIR, a_client_over_the_shipped_world, require_nothing_outstanding};
use reload::{
    DIRT, DIRT_FILE, Declaration, GRASS, GRASS_FILE, STONE, STONE_FILE, WATER, WATER_FILE,
    accepted, adoption, candidate, declaring, restating, shipped, shipped_restating_stone,
    stone_that_is_not_solid,
};
use reload_remesh::{Marking, a_client_over, every_section_once, marked};
use reload_world::{published_tick, shipped_world, standing_at};
use support::TestResult;
use support::content::ContentRoot;

/// How many ticks are advanced with the whole world marked.
///
/// More than one, so a boundary really was crossed while the re-mesh was
/// outstanding; the number itself carries nothing else.
const TICKS_ACROSS_THE_RELOAD: u32 = 3;

/// A block declared per facing, and the file it arrives in.
///
/// **The file sorts after all four the game ships**, so it is registered last and
/// the block a player holds is still `base:dirt` — which is what every other
/// scenario in this file expects, and what keeps this pair comparable with them.
const ZIRCON: &str = "base:zircon";
const ZIRCON_FILE: &str = "zircon.luau";

/// The key [`ZIRCON`]'s `north` draws from while the content is serving, and the
/// key a candidate re-points it to.
///
/// **Neither is the block's own name and both are keys nothing else declares.**
/// The other five facings hold the name, so the block still resolves a layer
/// under the name-parsing the mesher has not stopped doing yet; `north` is the one
/// that moves, and it is the only difference between the two roots.
const NORTHS_OWN_KEY: &str = "base:zircon_north";
const A_DIFFERENT_NORTH: &str = "base:zircon_north_reworked";

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
fn a_candidate_re_pointing_one_facing_of_a_block_leaves_every_section_of_the_world_to_mesh()
-> TestResult {
    let (_serving, mut client) = a_client_over_content_declaring_six_facings()?;
    require_nothing_outstanding(&mut client)?;
    let repointed = shipped_declaring_zircon(A_DIFFERENT_NORTH)?;

    let answered = adoption(client.adopt(candidate(repointed.path())?));
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (answered, left_to_mesh),
        (accepted(DIRT), every_section_once()),
        "a block whose `north` alone was re-pointed is a block that draws differently, and the \
         whole world has to be built again to show it. **This is the only edit that separates a \
         comparison over six keys from one over a single key**: every other fixture in this suite \
         states its texture as one string, so a comparison reading `up` alone agrees with all of \
         them. Reading one key here accepts the edit and marks nothing at all — the reload \
         succeeds, the world is never meshed again, and there is no error anywhere"
    );
    Ok(())
}

#[test]
fn a_candidate_restating_the_same_six_facing_keys_leaves_no_section_to_mesh() -> TestResult {
    let (_serving, mut client) = a_client_over_content_declaring_six_facings()?;
    require_nothing_outstanding(&mut client)?;
    let unchanged = shipped_declaring_zircon(NORTHS_OWN_KEY)?;

    let answered = adoption(client.adopt(candidate(unchanged.path())?));
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (answered, left_to_mesh),
        (accepted(DIRT), Marking::NoSectionAtAll),
        "the control the scenario above cannot supply for itself. A comparison that answered \
         `changed` for any table-formed declaration — one that compared the six keys by identity, \
         or gave up and marked whenever a block states a table — would satisfy the re-pointing \
         reading and be caught only here. The two roots differ in one key and in nothing else"
    );
    Ok(())
}

#[test]
fn a_candidate_that_stops_stone_being_drawn_leaves_every_section_of_the_world_to_mesh() -> TestResult
{
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let invisible = shipped_restating_stone(&Declaration::of(STONE).drawn(false))?;

    let answered = adoption(client.adopt(candidate(invisible.path())?));
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (answered, left_to_mesh),
        (accepted(DIRT), every_section_once()),
        "an author who says a block is no longer drawn is asking for it to stop appearing, and \
         until the world is meshed again it goes on appearing exactly as it did. **This is the \
         wiring rather than the policy**: a mesher that reads `drawn` perfectly leaves every \
         scenario about what it emits green while a reload that never learned the field leaves \
         the picture stale until a relaunch. The candidate keeps stone solid, so nothing the \
         player stands on has moved and the mark cannot be solidity's"
    );
    Ok(())
}

#[test]
fn a_candidate_that_stops_stone_occluding_leaves_every_section_of_the_world_to_mesh() -> TestResult
{
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let see_through = shipped_restating_stone(&Declaration::of(STONE).occludes(false))?;

    let answered = adoption(client.adopt(candidate(see_through.path())?));
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (answered, left_to_mesh),
        (accepted(DIRT), every_section_once()),
        "occlusion decides which of a *neighbour's* faces are drawn, so a block that stops hiding \
         what stands behind it changes a picture it is not itself in — every face that was culled \
         against stone has to be emitted. That is why the whole world is the set rather than the \
         sections holding stone. Stone stays solid and stays drawn here, so neither of the other \
         two answers can be what marked it"
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

/// A client serving content that declares a block per facing, and the root it is
/// serving.
///
/// **The root travels back with the client** because it lives in a temporary
/// directory: dropped one line early it takes the content the client is serving
/// with it, and the failure reads as a missing content root.
///
/// The block is declared in a file sorting after all four the game ships, so
/// registration order is unchanged and the block a player holds is still the one
/// every other scenario in this file expects. It is never placed in the world
/// either, so nothing meshes it — what these two scenarios read happens at
/// admission, before a single section is built.
fn a_client_over_content_declaring_six_facings()
-> Result<(ContentRoot, InputHarness), Box<dyn Error>> {
    let serving = shipped_declaring_zircon(NORTHS_OWN_KEY)?;
    let client = a_client_over(serving.path(), standing_at(IN_OPEN_AIR), shipped_world)?;
    Ok((serving, client))
}

/// A copy of the shipped root that also declares [`ZIRCON`], its six facings
/// holding its own name except `north`, which holds `north`.
fn shipped_declaring_zircon(north: &str) -> Result<ContentRoot, Box<dyn Error>> {
    declaring(
        shipped()?,
        ZIRCON_FILE,
        &Declaration::of(ZIRCON).repointing_north(north),
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
