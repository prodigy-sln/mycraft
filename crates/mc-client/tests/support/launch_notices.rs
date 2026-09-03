//! The notices a launch writes rather than refuses over — what entry did about a
//! player it found inside solid rock, which of a save's blocks no longer behave
//! as they did, and which of the texture keys the content declares had no image.
//!
//! # Why these live apart from the refusals
//!
//! `printed_refusals.rs` produces the lines a *content root* is turned away with,
//! and it grows one refusal at a time. These are the other kind: **nothing here is
//! a refusal at all.** A launch that meets one of them proceeds — the player is in
//! their world by the time they read it — so what a page quoting one promises is
//! not "this is what stops you" but "this is what you are told". The recogniser in
//! `documented_refusals.rs` does not know the difference and must not: it matches
//! the prefix the reporting writes, so whatever a page shows under it is held to
//! what a run produces either way.
//!
//! The split is by that responsibility rather than by line count, which is the
//! same reason the per-facing and built-set groups have modules of their own.
//!
//! # Every line here is composed by production over a verdict a real launch reached
//!
//! Nothing is written out. A fixture decides which world is read and which save is
//! loaded, and a premise refuses to hand back a line unless the launch arrived at
//! the answer that line is about — because a sentence spelled out in a fixture is a
//! third copy of somebody's belief about the program, and a page held to it would
//! be agreeing with the fixture rather than with the client.

// Each binary that includes this drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use glam::Vec3;
use mc_client::notice::{changed_blocks, entering, stand_ins};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_sim::world::Clearing;
use mc_world::content::LuauFileDefinitionSource;
use mc_world::persistence::{SavedPlayer, save_world};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use crate::entry;
use crate::persistence::{COLUMNS, GROUND, save_in};
use crate::printed_refusals::normalised;
use crate::support;
use crate::support::content::{BLOCK_DIRECTORY, ContentRoot, shipped_copy};

/// Every line a launch writes rather than refuses over: the two an entry can
/// write about where it put the player, the two naming blocks that no longer
/// behave as the save recorded them, and the one naming a declared texture key
/// nothing baked.
///
/// **Five launches and not one.** One launch answers one thing about one player;
/// the two entry answers differ in the world that was read rather than in
/// anything asked for, the two changed-block answers differ in *when the save was
/// written*, which is what decides how many blocks a line names, and the last
/// reads a root that has no save at all.
///
/// # Errors
///
/// Returns an error if a fixture cannot be built or written, if a launch was
/// turned away, or if a launch arrived at an answer the line it is asked for is
/// not about.
pub fn launch_notices() -> Result<Vec<String>, Box<dyn Error>> {
    Ok(vec![
        an_entry_moving_a_trapped_player()?,
        an_entry_with_nowhere_to_put_them()?,
        a_launch_over_a_save_whose_block_behaves_differently()?,
        a_launch_over_a_save_this_build_wrote_and_one_edited_declaration()?,
        a_launch_over_a_root_declaring_a_key_nothing_baked()?,
    ])
}

/// A block declaring a texture key no manifest bakes, and the file it goes in.
const UNDRAWN_FILE: &str = "undrawn.luau";
const UNDRAWN_DECLARATION: &str = "return {\n\tname = \"example:undrawn\",\n\ttexture = \"example:undrawn\",\n\tsolid = true,\n}\n";
const THE_UNDRAWN_KEY: &str = "example:undrawn";

/// What a launch writes about the texture keys its built set left uncovered.
///
/// **One added block over the shipped root, so the line is the singular one.**
/// That is a mod author's first block, which is the case `voxel-models.md` walks
/// through — and the shipped keys beside it all have art, so a producer that
/// named more than the added key would be reporting a root whose set had gone
/// stale rather than one that gained a block.
///
/// Composed by [`mc_client::notice::stand_ins`] over the two sets a real
/// preparation arrived at, through the same two calls the preparation worker
/// makes. Nothing about the sentence is decided here.
///
/// # Errors
///
/// Returns an error if the fixture cannot be built, if the shipped root carries
/// no built set to copy, if the preparation was turned away, or if the launch
/// named anything other than the added key — a run that named the shipped keys
/// too has none of this line for a page to quote.
fn a_launch_over_a_root_declaring_a_key_nothing_baked() -> Result<String, Box<dyn Error>> {
    let root = crate::support::built_sets::a_root_with_a_built_set()?
        .declaring_block(UNDRAWN_FILE, UNDRAWN_DECLARATION)?;
    let prepared = crate::support::prepare_scene_at(root.path())?;
    let uncovered: Vec<String> = mc_client::launch::declared_keys(&prepared.resolution)
        .difference(&prepared.texels.keys())
        .map(|key| key.as_str().to_owned())
        .collect();
    entry::require(
        uncovered == [THE_UNDRAWN_KEY],
        format!(
            "a page quoting the line for one unbaked key needs a launch that left one uncovered, \
             and this preparation left {uncovered:?}"
        ),
    )?;
    stand_ins(
        &mc_client::launch::declared_keys(&prepared.resolution),
        &prepared.texels.keys(),
    )
    .map(|said| normalised(&said))
    .ok_or_else(|| "this producer's launch covered every key it declared".into())
}

/// How many chunk columns square the world an entry is driven in is.
///
/// Three columns is forty-eight blocks across, which holds the whole seventeen
/// blocks of the search cube around the position below with room to spare — the
/// premise that keeps "nothing within eight blocks is clear" a sentence about
/// solid ground rather than about the edge of the world.
const THREE_COLUMNS: u32 = 3;

/// Where the save records the player an entry has to answer for: inside solid
/// rock, feet a quarter of a block above the row they are in.
const TRAPPED_FEET: Vec3 = Vec3::new(16.5, entry::FEET_ROW as f32 + 0.25, 16.5);

/// The two cells the one way out occupies, four blocks along −x and −z of the
/// trap, and the only position inside the search cube a standing player fits.
///
/// **Where the way out is decides what a page has to quote**, since the sentence
/// carries the destination — so it is a cell picked for the coordinate it centres
/// on rather than the first one that would do.
const THE_WAY_OUT_CELLS: [(u32, u32, u32); 2] =
    [(12, entry::FEET_ROW, 12), (12, entry::FEET_ROW + 1, 12)];

/// What a player entry had to move out of solid rock reads.
///
/// **Composed by [`mc_client::notice::entering`] over a verdict a real launch came
/// to.** Nothing about the sentence is decided here: the fixture decides only which
/// of the two answers the launch arrives at, and the premise below refuses to hand
/// back a line if it arrived at the other one.
///
/// # Errors
///
/// Returns an error if the fixture cannot be built or written, if the launch was
/// turned away, or if entry did anything other than move the player — a run that
/// moved nobody has none of this line for a page to quote.
fn an_entry_moving_a_trapped_player() -> Result<String, Box<dyn Error>> {
    let clearing = what_entry_did(&a_wedge_leaving(&THE_WAY_OUT_CELLS)?)?;
    entry::require(
        matches!(clearing, Clearing::MovedTo(_)),
        format!(
            "a page quoting where entry put a player needs a run that put one somewhere, and the \
             launch over the trap at {TRAPPED_FEET:?} answered {clearing:?}"
        ),
    )?;
    said_at_entry(clearing)
}

/// What a player entry could find nowhere to put reads.
///
/// The same wedge with its one way out filled in, so the pair a page carries differs
/// in the world the launch read and not in what this module asked for.
///
/// # Errors
///
/// Returns an error if the fixture cannot be built or written, if the launch was
/// turned away, or if entry found somewhere to put the player after all.
fn an_entry_with_nowhere_to_put_them() -> Result<String, Box<dyn Error>> {
    let clearing = what_entry_did(&a_wedge_leaving(&[])?)?;
    entry::require(
        matches!(clearing, Clearing::NoClearSpaceWithin { .. }),
        format!(
            "a page quoting the answer for a player nothing could be done for needs a run that \
             found nowhere to put one, and the launch over the wedge at {TRAPPED_FEET:?} answered \
             {clearing:?}"
        ),
    )?;
    said_at_entry(clearing)
}

/// The save written before this repository's blocks were Luau, relative to the
/// repository root.
///
/// **Never regenerated**, which is what makes it an oracle: it predates the
/// declarations it is read against, so a line produced over it is a line a real
/// disagreement produced rather than one a fixture arranged.
const OLDER_SAVE: [&str; 5] = [
    "crates",
    "mc-world",
    "tests",
    "fixtures",
    "world_saved_against_the_toml_declarations.mcw",
];

/// What a launch over a save whose block behaves differently writes.
///
/// **Composed by [`mc_client::notice::changed_blocks`] over a list a real load
/// produced**, on the same terms as the two entry notices above: the fixture
/// decides which save is read and never what the sentence says. The save is the
/// committed pre-Luau one and the content is the shipped root, so the block named
/// is whichever one those two genuinely disagree about.
///
/// # Errors
///
/// Returns an error if the repository or the shipped content cannot be read, if
/// the launch was turned away, or if the load reported no changed block at all —
/// a run that found nothing has no line of this for a page to quote.
fn a_launch_over_a_save_whose_block_behaves_differently() -> Result<String, Box<dyn Error>> {
    let registry = Arc::new(support::content_registry()?);
    let launched = mc_client::launch::simulation_to_play(
        &support::repository_root()?.join(OLDER_SAVE.iter().collect::<PathBuf>()),
        mc_sim::persistence::Launching {
            seed: mc_sim::REPLAY_SEED,
            registry: Arc::clone(&registry),
            content: support::published_content(&registry)?,
            accepting: mc_world::persistence::Acceptance::ChangedBlocksToo,
        },
    );
    let (seated, _) = launched.map_err(|refused| refused.to_string())?;
    changed_blocks(&seated.changed)
        .map(|said| normalised(&said))
        .ok_or_else(|| {
            "this producer needs a save the shipped content disagrees with about a block's              behaviour, and the load reported none. There is no line for a page to quote"
                .into()
        })
}

/// The block a mod author edits in the offline edit-and-relaunch loop, the file
/// that declares it, and the one line the edit replaces.
///
/// The edit is the one the hot-reload page walks a player through: make water
/// solid, and nothing else. Matched with its surrounding newlines and its leading
/// tab so it is the table's own line rather than the sentence in the comment
/// above it that quotes the same three words.
const THE_EDITED_BLOCK: &str = "base:water";
const EDITED_FILE: &str = "water.luau";
const AS_SHIPPED: &str = "\n\tsolid = false,\n";
const AS_EDITED: &str = "\n\tsolid = true,\n";

/// Every block the save this build writes holds, and the row it stands them in.
///
/// All four rather than water alone: the line has to name the block the author
/// edited and no other, and a save holding only that block could not tell a
/// correct answer from one that names whatever it finds.
const BLOCKS_THE_SAVE_HOLDS: [&str; 4] = ["base:dirt", "base:grass", "base:stone", "base:water"];
const THE_ROW_THEY_STAND_IN: u32 = 1;

/// Where the save this build writes records the player.
///
/// Somewhere in open air above the row the blocks stand in, so the launch reads
/// the save rather than answering about a player it had to move.
const QUIT_STANDING_HERE: SavedPlayer = SavedPlayer {
    position: [8.5, 12.25, 8.5],
    yaw: 0.75,
    pitch: -0.25,
};

/// What a launch writes for a player who quit with **this** build and then edited
/// one declaration.
///
/// **This is the other half of the changed-blocks line and it is a different
/// number of blocks.** The producer above reads a save written before the
/// behaviour list grew, so every block in it is reported; this one writes its save
/// with the build under test, so the only thing the save and the content disagree
/// about is the declaration an author edited — one block, and the sentence reads
/// in the singular. Both are lines a real load produced and neither is spelled
/// out here.
///
/// It is the offline loop the hot-reload page walks through, step for step: play,
/// quit normally so the save is written, edit `content/base/blocks/water.luau` to
/// make water solid, and start again.
///
/// # Errors
///
/// Returns an error if the shipped content cannot be read or copied, if the save
/// cannot be written, if the launch was turned away, or if the load reported
/// anything other than the one edited block — a run that disagreed about more
/// than the author edited has none of this line for a page to quote.
fn a_launch_over_a_save_this_build_wrote_and_one_edited_declaration()
-> Result<String, Box<dyn Error>> {
    let saved = a_world_saved_against_the_shipped_content()?;
    let edited = shipped_making_water_solid()?;
    let registry = Arc::new(registry_over(edited.path())?);
    let launched = mc_client::launch::simulation_to_play(
        &save_in(&saved),
        mc_sim::persistence::Launching {
            seed: mc_sim::REPLAY_SEED,
            registry: Arc::clone(&registry),
            content: support::published_content(&registry)?,
            accepting: mc_world::persistence::Acceptance::ChangedBlocksToo,
        },
    );
    let (seated, _) = launched.map_err(|refused| refused.to_string())?;
    require_it_named_only_the_edited_block(&seated.changed)?;
    changed_blocks(&seated.changed)
        .map(|said| normalised(&said))
        .ok_or_else(|| "this producer's load reported no changed block at all".into())
}

/// Refuses unless `changed` names the edited block and nothing else.
///
/// The premise the singular sentence rests on. A load disagreeing about more than
/// the author edited composes the plural line, which is a different page's line —
/// and a producer that handed it over would leave both pages quoting whichever one
/// happened to arrive.
///
/// # Errors
///
/// Returns an error naming what the load actually reported.
fn require_it_named_only_the_edited_block(changed: &[BlockName]) -> Result<(), Box<dyn Error>> {
    let named: Vec<&str> = changed.iter().map(BlockName::as_str).collect();
    entry::require(
        named == [THE_EDITED_BLOCK],
        format!(
            "a page quoting the line for one edited declaration needs a load that disagreed about \
             one block, and this load reported {named:?}"
        ),
    )
}

/// A save of a world holding all four shipped blocks, written by **this** build
/// against the content this repository ships.
///
/// The directory travels back so the save outlives this call; dropped one line
/// early it takes the file the launch is about to read with it.
///
/// # Errors
///
/// Returns an error if the shipped content cannot be read, if a name is not a
/// namespaced id, if a block will not go in the world, or if the save cannot be
/// written.
fn a_world_saved_against_the_shipped_content() -> Result<TempDir, Box<dyn Error>> {
    let registry = support::content_registry()?;
    let mut blocks = VoxelWorld::empty(COLUMNS);
    for (along, name) in BLOCKS_THE_SAVE_HOLDS.iter().enumerate() {
        let at = WorldPos {
            x: u32::try_from(along)? + 1,
            y: THE_ROW_THEY_STAND_IN,
            z: 1,
        };
        blocks.set_block(at, &BlockName::parse(name)?, &registry)?;
    }
    let directory = TempDir::new()?;
    save_world(&save_in(&directory), &blocks, QUIT_STANDING_HERE, &registry)?;
    Ok(directory)
}

/// A copy of the shipped content root with water made solid and nothing else
/// touched.
///
/// The shipped text is **edited** rather than rewritten from a builder, which is
/// what the page tells an author to do and what keeps the other six fields water
/// declares exactly as they ship — a rewritten declaration that dropped one of
/// them would move a second fold and the line would name a block for a reason the
/// page does not describe.
///
/// # Errors
///
/// Returns an error if the root cannot be copied or written, or if the shipped
/// declaration does not hold exactly one line to replace.
fn shipped_making_water_solid() -> Result<ContentRoot, Box<dyn Error>> {
    let root = shipped_copy()?;
    let declared = root.path().join(BLOCK_DIRECTORY).join(EDITED_FILE);
    let shipped = fs::read_to_string(&declared)?;
    entry::require(
        shipped.matches(AS_SHIPPED).count() == 1,
        format!(
            "this fixture makes the one edit the offline loop describes, and it has to be able to \
             find the line to make it on: `{EDITED_FILE}` holds {found} lines matching the \
             shipped one",
            found = shipped.matches(AS_SHIPPED).count()
        ),
    )?;
    fs::write(&declared, shipped.replace(AS_SHIPPED, AS_EDITED))?;
    Ok(root)
}

/// A registry holding what the content root at `root` declares.
///
/// # Errors
///
/// Returns whatever the reader refused the root with.
fn registry_over(root: &Path) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root.to_owned()))?;
    Ok(registry)
}

/// The line the client writes about `clearing` when a player enters.
///
/// # Errors
///
/// Returns an error where entry writes nothing at all, which is what every ordinary
/// launch does and is no page's business.
fn said_at_entry(clearing: Clearing) -> Result<String, Box<dyn Error>> {
    entering(clearing)
        .map(|said| normalised(&said))
        .ok_or_else(|| {
            format!("entry wrote nothing about {clearing:?}, so no line of it can be quoted").into()
        })
}

/// What the launch that read `save` did about where the player stands.
///
/// # Errors
///
/// Returns an error if the launch was turned away, in which case no player entered
/// and there is nothing for a page to quote.
fn what_entry_did(save: &entry::ASave) -> Result<Clearing, Box<dyn Error>> {
    let launched = entry::resumed(save, &entry::NO_ARGUMENT)?;
    let (seated, _) = launched.map_err(|refused| refused.to_string())?;
    Ok(seated.clearing)
}

/// A save recording a player inside solid rock, with every position the search may
/// consider filled in except the cells of `left_clear`.
///
/// **A wedge and never an edge.** The world is wide enough to hold the whole search
/// cube, so what leaves a player nowhere to go is solid ground rather than the world
/// running out — and that premise is checked here rather than assumed, because a
/// fixture that had drifted into an edge would still produce a sentence and a page
/// would still quote it.
///
/// # Errors
///
/// Returns an error if the world cannot be built or written, or if the cube does not
/// lie entirely inside the world.
fn a_wedge_leaving(left_clear: &[(u32, u32, u32)]) -> Result<entry::ASave, Box<dyn Error>> {
    let registry = entry::ground_registry()?;
    let mut blocks = entry::floor_of(&registry, THREE_COLUMNS, GROUND)?;
    let cube = entry::the_cube_around(TRAPPED_FEET);
    let inside = entry::inside_a_world(&cube, THREE_COLUMNS);
    entry::require(
        inside.len() == cube.len(),
        format!(
            "this fixture needs every position the search may look at to be inside the world, or \
             it is about an edge rather than about a wedge — {outside} of the {total} positions \
             around {TRAPPED_FEET:?} lie outside a world {THREE_COLUMNS} columns square",
            outside = cube.len() - inside.len(),
            total = cube.len()
        ),
    )?;
    entry::filling(
        &mut blocks,
        &registry,
        &entry::without(&inside, left_clear),
        GROUND,
    )?;
    entry::written(
        blocks,
        &registry,
        Arc::clone(&registry),
        entry::recorded_at(TRAPPED_FEET, 0.0, 0.0),
    )
}
