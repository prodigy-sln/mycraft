//! The world a driven tick resolves the player's motion against: a floor, and
//! nothing else.
//!
//! **A floor rather than an empty world.** Gravity acts on every tick, so a world
//! that answered "nothing is solid" would drop the player continuously and every
//! scenario asserting that a run matches a no-input control would be comparing
//! two falls — red against a *correct* client, for a reason nothing in the
//! assertion could show.
//!
//! **A floor rather than the walled one the lens scenarios use.** A wall stops a
//! player after about ten ticks, and the binding table is driven for twenty in
//! four directions; walking into a face already touched moves nothing, so a row
//! stopped by a wall would be indistinguishable from a row that never reached the
//! player at all.
//!
//! **Declared block by block rather than by a predicate, and as small as the
//! walk it has to contain.** The simulation owns an editable world now, so a
//! fixture answering "is this solid" is no longer something it can be built
//! from. The smallest world with a whole chunk column in it is 16 × 256 × 16,
//! and every voxel of it is walked once when its solidity is resolved — so the
//! footprint is one column and the spawn is centred in it. That the walk fits is
//! derived and not hoped for: the longest drive in this crate is twenty ticks in
//! one direction at the declared walk speed over a tick of 1/60 s, which is 1.5
//! blocks against 8 blocks of margin to the nearest edge.
//!
//! **The registry is declared here and never read from `content/base`.** A test
//! binary's working directory is not the repository root, and what this fixture
//! needs is a solid block and a non-solid one — not the blocks the game ships.

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockId, BlockRegistry, DefinitionOrigin};
use mc_core::content::{FaceTextures, LayerAssignment, ResolvedBlock, ResolvedContent};
use mc_core::id::{BlockName, TextureKey};
use mc_sim::action::default_held_block;
use mc_sim::player::PlayerState;
use mc_sim::simulation::{PublishedContent, Simulation, seat};
use mc_sim::world::World;
use mc_world::world::{VoxelWorld, WorldPos};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// How many blocks across one column is.
const ACROSS: u32 = 16;

/// The one solid voxel layer of the floor. The feet come to rest on its top
/// face, one above it.
const FLOOR: u32 = 9;

/// Where the player stands: on the floor, centred in the column, facing along
/// +x, holding still.
///
/// `SPAWN.y == FLOOR + 1` with `on_ground: true` is an implicit coupling the
/// dispatch scenarios rest on — a jump asserted from a standing start has
/// nothing to assert if the player was falling.
const SPAWN: Vec3 = Vec3::new(8.5, (FLOOR + 1) as f32, 8.5);

/// What the fixture's own blocks are called, and what each is for.
///
/// Named for the fixture rather than for anything content ships: nothing here is
/// about the blocks the game has, and a shipped name would suggest the physics
/// recognised one.
const GROUND: &str = "fixture:ground";
const OPEN: &str = "fixture:open";

/// What these definitions are attributed to. Nothing asserts it; a definition
/// has to say where it came from.
const FIXTURE_ORIGIN: &str = "the driven client's declared floor";

/// A simulation of that world, with the player standing on it, and the block a
/// place request over it would name.
///
/// The held block is asked of the simulation's own policy rather than spelled
/// here, so this fixture drives the client through the same decision the
/// composition root makes — over this registry it is the ground block, the only
/// solid one declared.
///
/// # Errors
///
/// Returns the refusal if the declared registry or the declared world does not
/// apply — which is this fixture being wrong about itself, and is reported
/// rather than absorbed. A registry declaring no solid block at all is the same
/// kind of wrongness and is reported the same way.
pub fn ground_plane() -> Result<(Simulation, BlockName), Box<dyn Error>> {
    let registry = Arc::new(declared_registry()?);
    let open = BlockName::parse(OPEN)?;
    let mut blocks = VoxelWorld::filled(COLUMNS, &open, &registry)?;
    let ground = BlockName::parse(GROUND)?;
    for (x, z) in every_position() {
        blocks.set_block(WorldPos { x, y: FLOOR, z }, &ground, &registry)?;
    }
    let holding = default_held_block(&registry)
        .ok_or("the driven client's declared registry holds no solid block to place")?;
    let content = published_content(&registry)?;
    Ok((
        seat(
            PlayerState {
                position: SPAWN,
                velocity: Vec3::ZERO,
                yaw: 0.0,
                pitch: 0.0,
                on_ground: true,
            },
            World::new(blocks, registry)?,
            content,
        )
        .simulation,
        holding,
    ))
}

/// The content a simulation over `registry` publishes at launch.
///
/// **The reader's own share, written out again rather than asked of
/// `mc_sim::content::load`.** That door reads a content root, and this fixture's
/// blocks are declared in memory — so there is no root to read. The layers are
/// the ones a session that has spent nothing hands out, because a launch has
/// spent nothing, and the HUD is a client's own empty one: this fixture declares
/// no element, which is a valid answer rather than a missing one.
///
/// The same few lines appear in `support/persistence.rs` and in
/// `support/mod.rs`, and the duplication is the `#[path]` fixture layout's price:
/// these modules are reached by path so that a binary need not pull in every
/// other fixture, which is exactly what stops them sharing one.
///
/// # Errors
///
/// Returns an error if a registered id cannot be read back, if the layers do not
/// fit a session's budget, or if an empty HUD source is refused.
fn published_content(registry: &BlockRegistry) -> Result<PublishedContent, Box<dyn Error>> {
    let mut blocks = Vec::new();
    for position in 0..registry.registered_count() {
        let declared = registry.definition(BlockId::from_raw(u32::try_from(position)?))?;
        blocks.push(ResolvedBlock {
            name: declared.name.clone(),
            textures: declared.textures.clone(),
            is_solid: declared.is_solid,
        });
    }
    let layers = LayerAssignment::none().appending(&registry.texture_keys())?;
    Ok(PublishedContent::first(
        ResolvedContent::stating(blocks, layers),
        mc_sim::content::hud_before_content_is_read()?,
    ))
}

/// Every horizontal position of the fixture's one column.
fn every_position() -> impl Iterator<Item = (u32, u32)> {
    (0..ACROSS).flat_map(|z| (0..ACROSS).map(move |x| (x, z)))
}

/// A registry holding the fixture's two blocks: one the player stands on and one
/// it stands in.
fn declared_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let origin = DefinitionOrigin::new(FIXTURE_ORIGIN);
    let declared = [(OPEN, false), (GROUND, true)]
        .into_iter()
        .map(|(name, is_solid)| {
            Ok(BlockDefinition {
                name: BlockName::parse(name)?,
                textures: FaceTextures::uniform(TextureKey::parse(name)?),
                is_solid,
                // The open block is what a placement may be built over and what
                // a break empties a cell back to; the ground is neither. That is
                // stated per block rather than derived from solidity, because
                // the two are separate claims and this fixture is one of the
                // places that would not notice if they were collapsed.
                replaceable: !is_solid,
                // Both can be broken, and breaking either empties its cell —
                // what a click becomes is asserted against its own fixtures, and
                // a residue named here would be a claim this file cannot check.
                breakable: true,
                breaks_into: None,
                drawn: is_solid,
                occludes: is_solid,
                targetable: is_solid,
                // Constants, never derived from this fixture's own solidity: nothing
                // has ever answered these two, so a derived medium would make the air
                // swimmable and no assertion in this file could see it.
                swimmable: false,
                move_resistance: 0.0,
                origin: origin.clone(),
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        origin,
        declared.into_iter().map(Ok).collect(),
    ))?;
    Ok(registry)
}
