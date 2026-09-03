//! A launch that resumes a save derives no world from the replay seed.
//!
//! # How "no generation" becomes something a test can watch
//!
//! "It did not generate a world" is not a claim any assertion about geometry can
//! make: a launch that generated one, threw it away and then meshed the save
//! hands over exactly the same picture as one that never generated at all. So the
//! two scenarios here do not assert the absence. They take the generator's
//! ability to run away and then ask what the launch does.
//!
//! The content root both scenarios read is the shipped one with a solid block of
//! this suite's own added and **every block the replay generator places removed**.
//! Against such a root `ReplayWorld::generate` cannot succeed — it names
//! `base:grass`, `base:dirt`, `base:stone` and `base:water` and the registry knows
//! none of them — so a launch that hands over a whole world's geometry against
//! that root is a launch that never asked the seed for one. The green *is* the
//! evidence, which is the only shape this claim has.
//!
//! # The root is not "the four declarations deleted", and that distinction is the
//! whole fixture
//!
//! `BlockRegistry::apply` refuses a source that declares nothing at all, so a root
//! with an empty `blocks/` directory fails at **registration** — before anything
//! reaches a generator. A launch against it would refuse for a reason that has
//! nothing to do with generation, and the resuming scenario below would be red or
//! green for a reason nobody chose. The root therefore declares exactly one block:
//! the solid one the save holds, which is neither of the two things the generator
//! needs and is enough for the registry to accept the source.
//!
//! # Why the refusing scenario is not optional
//!
//! Without it, "a launch resumed the save against a root that cannot generate"
//! is satisfied just as well by generation having become optional *and broken*: a
//! launch that quietly skipped a world it genuinely needed would pass the first
//! scenario and ship a client that cannot start a new game. The second scenario
//! hands the same root a path with no save at it and requires the launch to refuse
//! **naming the block the generator could not place** — so the two scenarios
//! disagree about what the same root means, and no implementation satisfies both
//! by ignoring the generator.
//!
//! It is also this suite's fixture control. If the removal ever stopped removing
//! anything, the root would generate, the launch would start, and this scenario
//! would report that it prepared a launch where a refusal was expected. Measured
//! on 2026-08-15 rather than argued: built from a root with the generator's four
//! declarations left in place, the resuming scenario below goes **green** and this
//! one goes red on exactly that sentence.
//!
//! # Where the numbers come from
//!
//! The saved world is emptiness with one solid block standing in it, sixteen
//! blocks above the highest surface the generator produces anywhere and in the
//! middle of its own chunk column, so it is alone in its section with nothing
//! adjacent in any direction and shows all six of its faces, none of them merged
//! with anything. Six quads for that section, and a section record carrying them
//! rather than an absent one.

#[path = "support/handed.rs"]
mod handed;
mod support;

use std::error::Error;
use std::path::Path;

use mc_client::launch::{PreparedLaunch, prepare_launch};
use mc_client::notice::Notices;
use mc_client::startup::PreparationError;
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_render::window::Ending;
use mc_sim::replay::ReplayWorld;
use mc_world::content::LuauFileDefinitionSource;
use mc_world::persistence::{Acceptance, SavedPlayer};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use handed::{TestResult, resumed, where_no_save_is};
use support::content::{ContentRoot, shipped_copy};

/// The save here is written against the registry the root it names produces, so
/// nothing about its blocks can have changed and the acceptance decides nothing.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// Where this save records the player. Nothing here asserts it, and a save
/// records somebody.
const RECORDED_PLAYER: SavedPlayer = SavedPlayer {
    position: [28.5, 67.0, 28.5],
    yaw: 0.0,
    pitch: 0.0,
};

/// The one solid block the fixture root declares, and the file it declares it in.
///
/// Said out loud in a test under `tests/`, which the hardcoded-name scan does not
/// read. It is deliberately not one of the four the generator places: a root
/// declaring one of those would be a root the generator could still partly build
/// a world from, and "the generator cannot run here" would stop being true.
const BEACON: &str = "fixture:beacon";
const BEACON_DECLARATION_FILE: &str = "zz-beacon.luau";
const BEACON_DECLARATION: &str =
    "return {\n\tname = 'fixture:beacon',\n\ttexture = 'fixture:beacon',\n\tsolid = true,\n}\n";

/// The files the shipped root declares the generator's own blocks in.
///
/// All four go, and the count is not a matter of taste: the generator names
/// `base:grass`, `base:dirt`, `base:stone` and `base:water`, and leaving any one
/// of them behind leaves a root that fails somewhere later in generation instead
/// of at the first block it places — which is still a refusal, but one about a
/// different block than the one this fixture means.
const THE_GENERATORS_DECLARATIONS: [&str; 4] =
    ["dirt.luau", "grass.luau", "stone.luau", "water.luau"];

/// Where the beacon stands in the world only the save holds: chunk column (1, 1),
/// sixteen blocks above the highest surface the generator produces anywhere.
const WHERE_THE_BEACON_STANDS: (u32, u32, u32) = (28, 64, 28);

/// Where that section has its near corner, which is how a scene records a
/// section.
const THE_BEACONS_SECTION: [i32; 3] = [16, 64, 16];

/// How many faces a solid block with nothing adjacent in any direction shows: all
/// six, none of them merged with anything.
const ALONE_IN_ITS_SECTION_SHOWS: u32 = 6;

#[test]
fn a_launch_resuming_a_save_draws_it_against_content_no_world_could_be_generated_from() -> TestResult
{
    let root = a_root_the_generator_cannot_build_a_world_from()?;
    let refused_generation = why_this_root_cannot_generate(root.path())?;
    let saved = resumed(root.path(), RECORDED_PLAYER, a_world_standing_the_beacon)?;

    let launched = prepare_launch(
        root.path(),
        &saved.save(),
        ACCEPTING,
        &Notices::discarding(),
    );

    assert_eq!(
        (
            quads_in(&launched, THE_BEACONS_SECTION),
            saved.stored_at(WHERE_THE_BEACON_STANDS)
        ),
        (Ok(Some(ALONE_IN_ITS_SECTION_SHOWS)), Ok(BEACON.to_owned())),
        "the content root this launch reads declares {BEACON} and none of the blocks the replay \
         generator places, so nothing could have been generated from the seed here — asked \
         directly, the generator refuses with `{refused_generation}`. A launch that nonetheless \
         hands over the saved world's geometry is a launch that established which world it needed \
         before building one, which is the whole claim. The second half is the fixture's own \
         integrity: the save on disk really holds the beacon at that cell, read back through the \
         loader, so this cannot be green because the launch generated a world instead"
    );
    Ok(())
}

#[test]
fn a_launch_with_no_save_refuses_naming_the_block_the_generator_could_not_place() -> TestResult {
    let root = a_root_the_generator_cannot_build_a_world_from()?;
    let refused_generation = why_this_root_cannot_generate(root.path())?;
    let nowhere = TempDir::new()?;

    let launched = prepare_launch(
        root.path(),
        &where_no_save_is(&nowhere),
        ACCEPTING,
        &Notices::discarding(),
    );

    let answer = refusal(&launched)?;
    assert_eq!(
        (answer.contains(&refused_generation), launched.is_ok()),
        (true, false),
        "with no save to resume, a launch against this root has to build a world and cannot: the \
         generator refuses with `{refused_generation}`, which names the block it could not place. \
         That refusal is what a player is shown, so it has to survive the whole way out — a \
         message that only says a world could not be generated leaves nobody able to act on it. \
         This is also the control on the scenario above: a root the generator could still build \
         from would start a launch here, and that scenario's green would then say nothing about \
         which world was built. The launch answered: {answer}"
    );
    Ok(())
}

/// What a launch came to: the preparation it produced, or the refusal it gave
/// instead.
type Launched = Result<PreparedLaunch, PreparationError>;

/// The shipped content root with [`BEACON`] added and every block the replay
/// generator places taken out.
///
/// # Errors
///
/// Returns an error if the root cannot be copied, if the shipped root already
/// declares the beacon, or if it does not declare one of the four.
fn a_root_the_generator_cannot_build_a_world_from() -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?
        .declaring_block(BEACON_DECLARATION_FILE, BEACON_DECLARATION)?
        .not_declaring_blocks(&THE_GENERATORS_DECLARATIONS)
}

/// Why the generator cannot build a world out of what the root at `path`
/// declares, in the generator's own words.
///
/// **Asked through the generator itself rather than spelled out here**, on the
/// launch_world precedent: a fixture stating the wording by hand would agree with
/// a launch quoting the wrong refusal as readily as with one quoting the right
/// one, and it would go stale the day the message is reworded.
///
/// It carries both halves of this suite's fixture integrity. The root has to
/// **register** — a root declaring nothing at all is refused by
/// `BlockRegistry::apply` before any generator is reached, and both scenarios
/// here would then be about a registry rather than about a world. And it has to
/// be one the generator refuses, which is the property the whole file rests on.
///
/// # Errors
///
/// Returns an error if the root does not register, or if the generator builds a
/// world out of it after all.
fn why_this_root_cannot_generate(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(path.to_owned()))?;
    match ReplayWorld::generate(mc_sim::REPLAY_SEED, &registry) {
        Ok(_) => Err(
            "this fixture's content root generates a replay world after all, so a \
                      launch reaching the generator would not refuse and neither scenario here \
                      would be about the world a launch decided to build"
                .into(),
        ),
        Err(refused) => Ok(refused.to_string()),
    }
}

/// An otherwise empty world with the beacon standing alone in it.
///
/// Emptiness rather than a change to the generated world, because the root this
/// is built against is one the generator cannot build a world from at all — which
/// is the point of it.
///
/// # Errors
///
/// Returns an error if the beacon is not a name, or if `registry` does not
/// declare it.
fn a_world_standing_the_beacon(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let (x, y, z) = WHERE_THE_BEACON_STANDS;
    let mut blocks = VoxelWorld::empty(mc_sim::replay::world::FOOTPRINT_COLUMNS);
    blocks.set_block(WorldPos { x, y, z }, &BlockName::parse(BEACON)?, registry)?;
    Ok(blocks)
}

/// How many quads the scene a launch handed over holds for the section whose near
/// corner is `origin` — or the refusal the launch gave instead.
///
/// Nothing where the scene carries no record for that section at all, which is a
/// different answer from a record carrying no quads and is kept apart from it
/// deliberately.
fn quads_in(launched: &Launched, origin: [i32; 3]) -> Result<Option<u32>, String> {
    let prepared = launched.as_ref().map_err(PreparationError::to_string)?;
    Ok(prepared
        .scene
        .sections()
        .iter()
        .find(|record| record.origin == origin)
        .map(|record| record.quad_count))
}

/// The refusal a launch gave, as a player reads it — or what it did instead of
/// refusing.
///
/// **Taken through the door the client goes through, not composed here.** The
/// block the generator could not place is named a layer below the sentence
/// saying why a world was being built at all, so a helper reading the outermost
/// sentence alone would be asking whether a name survived a journey it never
/// took. Going through [`Ending::failed`] and the shipped reporting rather than
/// assembling the same pieces is what makes this a reader of that decision
/// instead of a second copy of it.
///
/// # Errors
///
/// Returns an error if the sink refuses the bytes, which a `Vec` does not, or if
/// what was written is not text.
fn refusal(launched: &Launched) -> Result<String, Box<dyn Error>> {
    match launched {
        Ok(_) => Ok("it prepared a launch".to_owned()),
        Err(turned_away) => support::reported(&Ending::failed(turned_away)),
    }
}
