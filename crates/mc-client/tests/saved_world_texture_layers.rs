//! Which array-texture layer a block occupies is decided by what the content
//! declares, and never by which blocks happen to be in the world a launch plays.
//!
//! # Why this is a requirement and not an implementation detail
//!
//! Layers are handed out positionally over a sorted set of texture keys, and the
//! index a key lands on rides inside every packed vertex — so it rides inside every
//! image this repository has committed. Making the geometry a launch hands over
//! depend on the save is the whole of the change these scenarios belong to, and if
//! the key set were the *meshed* world's keys then that change would make every layer
//! index depend on the save too: a save whose world had broken the last stone out of
//! existence would renumber the array texture. That is a worse defect than the one
//! being fixed, because this one is visible on the first frame and that one would be
//! invisible — no golden frame is shot after a resume.
//!
//! # Two scenarios, and they are not the same claim twice
//!
//! The first is a **guard**: a save holding a world with no stone in it anywhere
//! must leave stone exactly where a launch with no save puts it. It is green while
//! the key set comes from the registry and red the moment anything derives it from
//! the world played.
//!
//! The second is a **driver**: a solid block the shipped content does not declare,
//! declared by a content root of this suite's own and standing in a world only the
//! save holds. It has a layer because layers come from what the content declares,
//! and it draws.
//!
//! **Which wrong fix each of the two catches is not the same, and stopped being the
//! same the moment the launch began meshing the saved world.** There are two ways to
//! derive the key set from a world rather than from the registry, and they are no
//! longer one claim:
//!
//! - From the **generated** world's quads — the spelling that shipped before this
//!   change. The beacon stands only in the save, so no generated quad names it, no
//!   layer is resolved for it, and the section it stands in fails to pack. The
//!   driver below catches this, and still does: measured, the launch comes back
//!   `Err("a meshed section could not be packed into vertices")` where six quads
//!   were expected.
//! - From the **played** world's quads — the live wrong fix now that the mesh
//!   follows the save. The beacon *is* in the played world, so it resolves and the
//!   driver below passes. **Only the stone-free guard above catches this one**, by
//!   asking after a block the played world does not hold.
//!
//! So a reader counting two scenarios here as two guards on the key-set definition
//! is counting one. That is why the guard's fixture holds a world with something
//! *missing* from it, and it is the half that carries the weight.
//!
//! # The stone-free world is spelled as emptiness with one block in it
//!
//! "A world holding no `base:stone` anywhere" is what the scenario asks for, and the
//! smallest world satisfying it is emptiness with one solid block that is not stone.
//! It is also the sharpest against the fix this guards against: the played world's
//! quads then name exactly one key, so a key set derived from them would leave stone
//! with no layer at all rather than with a layer that merely moved.
//!
//! # Why the added block declares its texture as its own name
//!
//! It has to, and that is worth writing down because it looks like a missed
//! opportunity. Resolving the key set from a definition's `texture` rather than its
//! `name` is a decision no test in this spec can grade, since all four shipped blocks
//! declare the two identically — and a fixture block declaring them *differently*
//! cannot close that gap either, because the packer still asks for a quad's layer by
//! the block's **name**. Such a block would fail to pack under a correct
//! implementation as readily as under a wrong one. That line stays held by a reader.

#[path = "support/handed.rs"]
mod handed;
mod support;

use std::error::Error;

use mc_client::launch::{PreparedLaunch, prepare_launch};
use mc_client::startup::PreparationError;
use mc_core::block::BlockRegistry;
use mc_core::id::{BlockName, TextureKey};
use mc_world::column::SECTIONS_PER_COLUMN;
use mc_world::persistence::{Acceptance, SavedPlayer};
use mc_world::section::Contents;
use mc_world::section::SECTION_SIZE;
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use handed::{TestResult, generated_blocks, resumed, shipped_content, where_no_save_is};
use support::content::shipped_copy;

/// Every save here is written against the registry the root it names produces, so
/// nothing about its blocks can have changed and the acceptance decides nothing.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// Where these saves record the player. Nothing here asserts it, and a save records
/// somebody.
const RECORDED_PLAYER: SavedPlayer = SavedPlayer {
    position: [12.5, 67.0, 12.5],
    yaw: 0.0,
    pitch: 0.0,
};

/// The blocks these scenarios are about, spelled as content spells them.
///
/// Said out loud in a test under `tests/`, which the hardcoded-name scan does not
/// read: which layer a *named* block occupies is the whole subject, and a fixture
/// deriving the name from the registry could not tell one block's layer from
/// another's.
const STONE: &str = "base:stone";
const NOT_STONE: &str = "base:dirt";

/// The layer stone occupies in every golden frame this repository has committed.
///
/// Written out rather than derived, because that is what "the save cannot move a
/// layer index" means when written down: derived from the code that assigns it, this
/// assertion would agree with whatever that code did today.
const STONES_LAYER: u16 = 2;

/// The one block the stone-free world holds, somewhere the generator draws nothing.
const WHERE_THE_ONE_BLOCK_STANDS: (u32, u32, u32) = (8, 40, 8);

/// A solid block the shipped content does not declare, and the file this suite
/// declares it in.
///
/// The file name sorts after every shipped declaration, so the added block registers
/// last and the block a client holds — the first solid one in registration order —
/// does not move.
const BEACON: &str = "fixture:beacon";
const BEACON_DECLARATION_FILE: &str = "zz-beacon.luau";
const BEACON_DECLARATION: &str =
    "return {\n\tname = 'fixture:beacon',\n\ttexture = 'fixture:beacon',\n\tsolid = true,\n}\n";

/// Where the beacon stands in the world only the save holds: chunk column (1, 1),
/// sixteen blocks above the highest surface the generator produces anywhere, so it is
/// alone in its section with nothing adjacent in any direction.
const WHERE_THE_BEACON_STANDS: (u32, u32, u32) = (28, 64, 28);

/// Where that section has its near corner, which is how a scene records a section.
const THE_BEACONS_SECTION: [i32; 3] = [16, 64, 16];

/// How many faces a solid block with nothing adjacent in any direction shows: all
/// six, none of them merged with anything.
const ALONE_IN_ITS_SECTION_SHOWS: u32 = 6;

/// How many cells a world of the replay's footprint declares.
///
/// Derived from the three declarations that decide it rather than counted from a run,
/// so a scan that visited a smaller world than it claimed to is a failed assertion
/// rather than a silent one.
const EVERY_DECLARED_CELL: usize = (mc_sim::replay::world::FOOTPRINT
    * mc_sim::replay::world::FOOTPRINT
    * SECTIONS_PER_COLUMN
    * SECTION_SIZE) as usize;

#[test]
fn a_save_whose_world_holds_no_stone_leaves_stone_on_the_layer_it_always_had() -> TestResult {
    let content = shipped_content()?;
    let nowhere = TempDir::new()?;
    let saved = resumed(&content, RECORDED_PLAYER, a_world_without_stone)?;

    let resuming = prepare_launch(&content, &saved.save(), ACCEPTING);
    let starting_fresh = prepare_launch(&content, &where_no_save_is(&nowhere), ACCEPTING);

    assert_eq!(
        (
            layer_of(&resuming, STONE),
            layer_of(&starting_fresh, STONE),
            saved.stored().map(|world| where_it_holds(&world, STONE))
        ),
        (
            Ok(Some(STONES_LAYER)),
            Ok(Some(STONES_LAYER)),
            Ok(nowhere_at_all(STONE))
        ),
        "the world this save holds contains no {STONE} at all, and stone still occupies the layer \
         every committed frame was shot with. Both launches are asserted, because the claim is that \
         the two cannot differ — and the layer is named rather than merely compared, since two \
         launches that both left stone with no layer whatsoever would agree just as well. The third \
         half is the fixture's own integrity: the save really came back, and the scan that found no \
         stone in it really looked at a whole world's worth of cells rather than at nothing"
    );
    Ok(())
}

#[test]
fn a_launch_draws_a_saved_block_whose_faces_the_generated_world_draws_nowhere() -> TestResult {
    let root = shipped_copy()?.declaring_block(BEACON_DECLARATION_FILE, BEACON_DECLARATION)?;
    let saved = resumed(root.path(), RECORDED_PLAYER, a_world_standing_the_beacon)?;

    let launched = prepare_launch(root.path(), &saved.save(), ACCEPTING);

    assert_eq!(
        (
            quads_in(&launched, THE_BEACONS_SECTION),
            saved.stored().map(|world| where_it_holds(&world, BEACON))
        ),
        (
            Ok(Some(ALONE_IN_ITS_SECTION_SHOWS)),
            Ok(standing_at(BEACON, WHERE_THE_BEACON_STANDS))
        ),
        "{BEACON} is declared by the content root this launch reads and placed nowhere by the \
         generator, so the only world it appears in is the one the save holds — and a launch \
         resuming that save has to draw all six of its faces. The block has a layer because the \
         layers come from what the content declares; built out of the *generated* world's quads \
         instead, the key set would have no entry for a block no generated quad names, and this \
         section would fail to pack rather than draw. It says nothing about a key set built out of \
         the *played* world's quads, which holds the beacon and resolves it — that one is the \
         stone-free guard's to catch, and only its. The second half says the save really holds the \
         block, exactly there"
    );
    Ok(())
}

/// What a launch came to: the preparation it produced, or the refusal it gave
/// instead.
type Launched = Result<PreparedLaunch, PreparationError>;

/// The generated world with the beacon standing where the generator puts nothing.
///
/// # Errors
///
/// Returns an error if the world cannot be generated, if the beacon is not a name, or
/// if `registry` does not declare it.
fn a_world_standing_the_beacon(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = generated_blocks(registry)?;
    blocks.set_block(
        cell(WHERE_THE_BEACON_STANDS),
        &BlockName::parse(BEACON)?,
        registry,
    )?;
    Ok(blocks)
}

/// An otherwise empty world with one solid block that is not stone standing in it.
///
/// # Errors
///
/// Returns an error if the block it stands is not a name, or if `registry` does not
/// declare it.
fn a_world_without_stone(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = VoxelWorld::empty(mc_sim::replay::world::FOOTPRINT_COLUMNS);
    blocks.set_block(
        cell(WHERE_THE_ONE_BLOCK_STANDS),
        &BlockName::parse(NOT_STONE)?,
        registry,
    )?;
    Ok(blocks)
}

/// Which layer `key` occupies in the layers a launch resolved — or the refusal the
/// launch gave instead.
///
/// A refusal comes back as the failed comparison rather than as a propagated error,
/// so that "it refused to prepare anything" and "it put the block on the wrong layer"
/// are one failed assertion instead of two kinds of failure.
fn layer_of(launched: &Launched, key: &str) -> Result<Option<u16>, String> {
    let prepared = launched.as_ref().map_err(PreparationError::to_string)?;
    let key = TextureKey::parse(key).map_err(|refusal| refusal.to_string())?;
    Ok(prepared.layers.layer_of(&key))
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

/// Where `block` first stands in `blocks`, and how many cells were looked at on the
/// way.
///
/// **The count is fixture integrity and not the scenario's claim.** A scan that
/// visited nothing finds nothing, and "no stone anywhere" read off an empty walk is
/// the easiest false green in this suite.
fn where_it_holds(blocks: &VoxelWorld, block: &str) -> String {
    let mut visited = 0;
    let mut found = None;
    for at in blocks.extent().positions() {
        visited += 1;
        if found.is_none()
            && matches!(blocks.block_at(at), Ok(Contents::Holds(name)) if name.as_str() == block)
        {
            found = Some(at);
        }
    }
    found.map_or_else(
        || format!("no {block} in any of {visited} cells"),
        |at| standing_at(block, (at.x, at.y, at.z)),
    )
}

/// What [`where_it_holds`] says where the block is not in the world at all.
fn nowhere_at_all(block: &str) -> String {
    format!("no {block} in any of {EVERY_DECLARED_CELL} cells")
}

/// What [`where_it_holds`] says where the block stands at `at`.
fn standing_at(block: &str, at: (u32, u32, u32)) -> String {
    let (x, y, z) = at;
    format!("{block} at ({x}, {y}, {z})")
}

/// A cell as the world spells a position.
const fn cell(at: (u32, u32, u32)) -> WorldPos {
    let (x, y, z) = at;
    WorldPos { x, y, z }
}
