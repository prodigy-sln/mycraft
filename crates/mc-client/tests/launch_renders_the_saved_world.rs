//! The geometry a launch hands the renderer when it resumes a save is the saved
//! world's, not the world the seed would have made.
//!
//! # The observation point is the launch's own preparation
//!
//! Every scenario here writes a save, hands its path to the one function the
//! client's startup prepares a launch with, and reads the scene that preparation
//! produced. Meshing the loaded world directly would answer a question nobody asked:
//! what is wrong today is not that a loaded world cannot be meshed, it is that the
//! preparation hands over geometry built from a *different* world than the one it
//! hands over a simulation of. That is only visible where both come out of the same
//! call.
//!
//! No window, no adapter and no working-directory change: the save is an explicit
//! path into a temporary directory, so this binary is free to hold more than one
//! test.
//!
//! # The landmark pillar, and why every count below is derivable
//!
//! Greedy meshing merges adjacent coplanar faces of the same block, so a quad count
//! is only a statement about faces where merging is ruled out. The declared world
//! has one place where it is ruled out completely. The landmark pillar stands in
//! block column (12, 12) — inside chunk column (0, 0), so its sections' near corners
//! are (0, y, 0) — and its stone reaches y = 64, while no surface in the world
//! stands higher than 48.
//!
//! - **The section at (0, 64, 0)** holds exactly one solid voxel, the pillar's
//!   topmost. It shows five of its six faces; the sixth abuts the pillar's next
//!   block down, which lives in the section beneath. Nothing else in the section
//!   emits a face, so none of the five merges with anything. Empty that one cell in
//!   the world a save holds and the section has nothing left to be about: no quads.
//! - **The section at (0, 48, 0)** is filled by the same pillar from y = 48 to
//!   y = 63 and by nothing else, so it shows one merged run per horizontal facing —
//!   four quads — with its top and bottom faces buried. Emptying the cell above
//!   uncovers the upward face at y = 63, the only solid voxel of that plane in the
//!   section, so it merges with nothing: four becomes five.
//! - **A block standing on the pillar's top**, at (12, 65, 12), lands in the section
//!   at (0, 64, 0) again, which spans y = 64 to y = 79. It must differ from the block
//!   it stands on: two runs of the same block merge along the plane they share, and a
//!   merged pair could not tell a placement from nothing at all. The pillar's top then
//!   keeps its four sides and loses its upward face, and the placed block shows five
//!   of its own six. Four and five.
//!
//! The arithmetic is restated here rather than imported from `edit_geometry.rs`,
//! which derives the same numbers for an edit: a fixture reading a constant it
//! depends on agrees with a landmark that moved.
//!
//! # Why "no quads" is not enough on its own, and what makes it speak
//!
//! A preparation that handed over an empty scene, or one that omitted the section
//! record altogether, satisfies "no quads for that section" while showing the player
//! nothing whatsoever. Two things are done about it. The zero is asserted as a
//! **section record carrying zero** and never as a missing record, so an absent
//! section reads as the different failure it is. And the emptied cell's own positive
//! control is a second scenario over the *same* save: one number has to fall to zero
//! and another has to rise to five, and no empty scene does both.
//!
//! # Every scenario also says the save was really there
//!
//! A launch handed a save it cannot read, or a path with nothing at it, generates a
//! world instead — the arm every scenario here exists to keep it out of, and one
//! that fails as an ordinary green rather than as an error. So each assertion carries
//! what the file on disk holds at the cell its scenario is about, read back through
//! the loader.

#[path = "support/handed.rs"]
mod handed;

use std::error::Error;

use mc_client::launch::{PreparedLaunch, prepare_launch};
use mc_client::startup::{PreparationError, scene_of};
use mc_core::block::{BlockId, BlockRegistry};
use mc_core::id::BlockName;
use mc_world::persistence::{Acceptance, SavedPlayer};
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};

use handed::{
    AResumedWorld, NO_DIFFERENCE, NOTHING, TestResult, generated_blocks, how_it_compares, resumed,
    shipped_content,
};

/// Every save here is written against the registry the same content root produces,
/// so nothing about its blocks can have changed and the acceptance decides nothing.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// Where these saves record the player: standing still over the landmark. Nothing
/// here asserts it, and a save records somebody.
const RECORDED_PLAYER: SavedPlayer = SavedPlayer {
    position: [12.5, 67.0, 12.5],
    yaw: 0.0,
    pitch: 0.0,
};

/// The landmark pillar's topmost block, and the cell directly over it.
const THE_LANDMARKS_TOP: (u32, u32, u32) = (12, 64, 12);
const ON_TOP_OF_THE_LANDMARK: (u32, u32, u32) = (12, 65, 12);

/// Where the two sections that block belongs to have their near corners, which is
/// how a scene records a section.
const THE_LANDMARKS_SECTION: [i32; 3] = [0, 64, 0];
const THE_SECTION_BENEATH_IT: [i32; 3] = [0, 48, 0];

/// How many faces the landmark's topmost block shows: five of its six, the downward
/// one buried against the pillar it stands on.
const THE_LANDMARKS_TOP_SHOWS: u32 = 5;

/// How many quads the section beneath shows: one merged run per horizontal facing
/// over the pillar's sixteen blocks, its top and bottom both buried.
const THE_PILLAR_SHOWS_BELOW: u32 = 4;

/// How many faces a block standing on the landmark's top shows: five of six again,
/// the downward one buried against what it stands on.
const A_BLOCK_STANDING_ON_IT_SHOWS: u32 = 5;

/// What a section holding no solid voxel at all carries.
const NO_QUADS: u32 = 0;

#[test]
fn a_launch_resuming_a_save_holds_no_quads_where_the_saved_world_holds_nothing() -> TestResult {
    let content = shipped_content()?;
    let saved = a_save_with_the_landmarks_top_emptied(&content)?;

    let launched = prepare_launch(&content, &saved.save(), ACCEPTING);

    assert_eq!(
        (
            quads_in(&launched, THE_LANDMARKS_SECTION),
            saved.stored_at(THE_LANDMARKS_TOP)
        ),
        (Ok(Some(NO_QUADS)), Ok(NOTHING.to_owned())),
        "the save holds the generated world with {THE_LANDMARKS_TOP:?} emptied, and that cell was \
         the only solid voxel of its section — so the geometry a launch resuming it hands over has \
         nothing to draw there. A launch that meshed the world the seed makes instead hands over \
         {THE_LANDMARKS_TOP_SHOWS} quads for a block the player can walk straight through, which \
         is the defect. The zero is a section record carrying none and not a missing record, so a \
         scene that simply stopped listing the section fails here as something else. The second \
         half is what stops the first from being about a launch that generated: the file really is \
         a readable save and really holds nothing at that cell"
    );
    Ok(())
}

#[test]
fn a_launch_resuming_a_save_shows_the_face_the_emptied_cell_uncovered() -> TestResult {
    let content = shipped_content()?;
    let saved = a_save_with_the_landmarks_top_emptied(&content)?;

    let launched = prepare_launch(&content, &saved.save(), ACCEPTING);

    assert_eq!(
        (
            quads_in(&launched, THE_SECTION_BENEATH_IT),
            saved.stored_at(THE_LANDMARKS_TOP)
        ),
        (Ok(Some(THE_PILLAR_SHOWS_BELOW + 1)), Ok(NOTHING.to_owned())),
        "this is the positive control on the same save the emptied section is asserted over, and \
         neither scenario is worth much without the other: a launch handing over an empty scene, or \
         one that dropped the section record, satisfies \"no quads up there\" while showing the \
         player nothing at all. Here a number has to *rise*. The pillar fills this section from \
         y = 48 to y = 63 and shows {THE_PILLAR_SHOWS_BELOW} merged runs with its top buried, and \
         emptying the cell above uncovers the upward face at y = 63 — the only solid voxel of that \
         plane here, so it merges with nothing. The second half says the save was really read"
    );
    Ok(())
}

#[test]
fn a_launch_resuming_a_save_draws_a_block_the_generated_world_does_not_hold() -> TestResult {
    let content = shipped_content()?;
    let saved = resumed(&content, RECORDED_PLAYER, |registry| {
        let mut blocks = generated_blocks(registry)?;
        let standing = standing_at(&blocks, THE_LANDMARKS_TOP)?;
        let built = a_solid_block_other_than(registry, &standing)?;
        blocks.set_block(cell(ON_TOP_OF_THE_LANDMARK), &built, registry)?;
        Ok(blocks)
    })?;

    let launched = prepare_launch(&content, &saved.save(), ACCEPTING);

    assert_eq!(
        (
            quads_in(&launched, THE_LANDMARKS_SECTION),
            saved.stored_at(ON_TOP_OF_THE_LANDMARK)
        ),
        (
            Ok(Some(
                THE_LANDMARKS_TOP_SHOWS - 1 + A_BLOCK_STANDING_ON_IT_SHOWS
            )),
            Ok(saved.written_at(ON_TOP_OF_THE_LANDMARK))
        ),
        "the save holds a block standing on the landmark at {ON_TOP_OF_THE_LANDMARK:?}, and a \
         launch resuming it has to draw a world with that block in it: the pillar's top keeps its \
         four sides and loses the upward face it is now covered by, and what stands on it shows \
         five of its own six. The block placed differs from the one it stands on, so neither run \
         merges into the other and the count can tell a placement from nothing at all. A launch \
         meshing the generated world hands over {THE_LANDMARKS_TOP_SHOWS} quads here and the \
         player builds into thin air. The second half is the round trip: what the save holds on \
         disk is what this fixture wrote into it"
    );
    Ok(())
}

#[test]
fn a_launch_packs_the_scene_it_hands_over_from_the_sections_it_retains() -> TestResult {
    let content = shipped_content()?;
    let saved = a_save_with_the_landmarks_top_emptied(&content)?;

    let launched = prepare_launch(&content, &saved.save(), ACCEPTING);

    assert_eq!(
        repacked_compared(&launched),
        Ok(NO_DIFFERENCE.to_owned()),
        "the scene the renderer is shown and the sections a later edit splices into are two answers \
         to one question, and the wiring that picks them sits where no assertion can reach it. So \
         what is asserted is that packing the retained sections reproduces the handed scene exactly \
         — which is only true if one world produced both. A launch that meshed the played world for \
         the picture while retaining the generated world's list satisfies every quad count above \
         and leaves the first edit splicing into sections that were never handed over: positional, \
         so it is a wrong picture where the two worlds have the same sections and a refused edit \
         where they do not"
    );
    Ok(())
}

/// What a launch came to: the preparation it produced, or the refusal it gave
/// instead.
type Launched = Result<PreparedLaunch, PreparationError>;

/// A save holding the generated world with the landmark's topmost block emptied.
///
/// Two scenarios read this same world, and they are the pair: one number has to fall
/// to zero and the other has to rise.
fn a_save_with_the_landmarks_top_emptied(
    content: &std::path::Path,
) -> Result<AResumedWorld, Box<dyn Error>> {
    resumed(content, RECORDED_PLAYER, |registry| {
        let mut blocks = generated_blocks(registry)?;
        blocks.empty_at(cell(THE_LANDMARKS_TOP))?;
        Ok(blocks)
    })
}

/// How many quads the scene a launch handed over holds for the section whose near
/// corner is `origin` — or the refusal the launch gave instead.
///
/// Nothing where the scene carries no record for that section at all, which is a
/// different answer from a record carrying no quads and is kept apart from it
/// deliberately: the preparation walks every section of the footprint, so a missing
/// record is a failure of a different kind.
///
/// A refusal comes back as the failed comparison rather than as a propagated error,
/// so that "it refused to prepare anything" and "it prepared the wrong geometry" are
/// one failed assertion instead of two kinds of failure.
fn quads_in(launched: &Launched, origin: [i32; 3]) -> Result<Option<u32>, String> {
    Ok(prepared(launched)?
        .scene
        .sections()
        .iter()
        .find(|record| record.origin == origin)
        .map(|record| record.quad_count))
}

/// How the scene a launch handed over compares with one packed from the sections
/// that same launch retained — or the refusal it gave instead.
fn repacked_compared(launched: &Launched) -> Result<String, String> {
    let prepared = prepared(launched)?;
    let repacked = scene_of(&prepared.meshed, &prepared.resolution)
        .map_err(|refusal: PreparationError| refusal.to_string())?;
    Ok(how_it_compares(&repacked, &prepared.scene))
}

/// What a launch prepared, or the refusal it gave, rendered.
fn prepared(launched: &Launched) -> Result<&PreparedLaunch, String> {
    launched.as_ref().map_err(PreparationError::to_string)
}

/// The block standing at `at` in `blocks`, or why the fixture cannot build the
/// world it says it builds.
///
/// Two of the three answers are the fixture being wrong about itself, and they are
/// wrong in different ways — a single refusal would hide which.
fn standing_at(blocks: &VoxelWorld, at: (u32, u32, u32)) -> Result<BlockName, Box<dyn Error>> {
    match blocks.block_at(cell(at))? {
        Contents::Empty => {
            Err(format!("the generated world holds nothing at {at:?} to stand a block on").into())
        }
        Contents::Holds(name) => Ok(name.clone()),
    }
}

/// The first solid block the registry declares that is not `standing`.
///
/// Derived and never named, because what the scenario needs is *a block that will
/// not merge into the one it stands on* — and a named one would silently stop being
/// that the day the generator's strata change.
fn a_solid_block_other_than(
    registry: &BlockRegistry,
    standing: &BlockName,
) -> Result<BlockName, Box<dyn Error>> {
    (0..u32::try_from(registry.registered_count())?)
        .filter_map(|raw| registry.definition(BlockId::from_raw(raw)).ok())
        .find(|definition| definition.is_solid && &definition.name != standing)
        .map(|definition| definition.name.clone())
        .ok_or_else(|| {
            format!(
                "the content root declares no solid block besides {}, so nothing can be built on \
                 the landmark that its own top would not merge with",
                standing.as_str()
            )
            .into()
        })
}

/// A cell as the world spells a position.
const fn cell(at: (u32, u32, u32)) -> WorldPos {
    let (x, y, z) = at;
    WorldPos { x, y, z }
}
