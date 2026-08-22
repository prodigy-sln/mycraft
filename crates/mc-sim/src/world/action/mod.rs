//! What a client may ask of the world, and what the server answers.
//!
//! A request says *what* — break, or place this block — and never *where*.
//! Where is the server's answer, recomputed every tick from the player state the
//! server itself owns, which is invariant 4 written into the shape of a type
//! rather than checked at a boundary.
//!
//! **A refusal is a value and not an error.** "You cannot reach that", "that
//! block cannot be broken" and "there is already something there" are answers to
//! a legitimate question, and the caller that wants to know *which* is a test
//! asserting the refusal it meant to construct — the only way to tell a
//! correctly refused operation from a wrongly shaped fixture.

mod trace;

use glam::Vec3;
use mc_core::block::{BlockId, BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::section::Contents;
use mc_world::world::WorldError;

use crate::player::{BlockPos, MovementIntent, PlayerState, eye_pose, occupies};
use crate::world::{World, inside_the_world};

use trace::stepped;
pub use trace::{Hit, targeted};

/// How far the player can reach, in blocks, measured from the eye to the point
/// where the ray meets the block.
///
/// Declared once, here, and read by nothing else.
pub const REACH: f32 = 5.0;

/// What a client asks the world for.
///
/// An enum and not a struct with an operation field, so "a break request
/// carrying a block name" is unrepresentable rather than merely unused. It
/// carries no position, no coordinate, no cell and no absolute orientation:
/// naming what you wish to place is an intent, and naming where it goes is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionIntent {
    Break,
    Place { block: BlockName },
}

/// Everything a client asks of one tick.
///
/// The movement half is unchanged and stays exactly five fields; the action half
/// is absent on almost every tick, which is what the `Option` says.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TickIntent {
    pub movement: MovementIntent,
    pub action: Option<ActionIntent>,
}

/// A tick that asks for movement and nothing else, which is what every call site
/// predating actions submits.
impl From<MovementIntent> for TickIntent {
    fn from(movement: MovementIntent) -> Self {
        Self {
            movement,
            action: None,
        }
    }
}

/// What one requested action did to the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditReport {
    Changed {
        cell: BlockPos,
        /// What the cell held before, which is nothing where a placement landed
        /// in an empty cell.
        from: Contents,
        /// What the cell holds now, which is nothing where a break declared no
        /// residue.
        to: Contents,
    },
    Refused(Refusal),
}

/// Why an action changed nothing.
///
/// **Only what can be constructed is declared.** A variant nothing builds is
/// dead code that reads as covered, so each arrives with the scenarios that
/// grade it rather than ahead of them.
///
/// **There is deliberately no `OutOfReach`.** The reach is bounded at a single
/// site: the walk stops when the next voxel's entry distance exceeds it, and
/// there is no second comparison. So "nothing is there" and "something is there
/// but it is too far" arrive at the same place, and telling them apart would
/// need a search *past* the reach — the unbounded traversal that does not
/// terminate, because `Solidity` is total. Both are [`Refusal::NoTarget`].
///
/// **There is deliberately no `NotSolid` either.** A place naming a block that
/// is not solid used to be refused outright, which made water unplaceable; the
/// rule it was guarding — a client placing air to delete a block it could not
/// break — is already forbidden by [`Occupied`](Refusal::Occupied), since a
/// block content does not declare replaceable may not be overwritten by
/// *anything*, air included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Nothing solid lies along the look direction inside the reach — including
    /// the case where something solid lies just past it.
    NoTarget,
    /// The ray began inside the block it met, so there is no face to place
    /// against and no cell the placement could be meant for.
    ///
    /// Only a placement can meet this: a break needs no entry face.
    NoFace,
    /// Content declares this block cannot be broken.
    ///
    /// **Unreachable for a block content declares non-solid, and the shipped
    /// content has one.** [`targeted`] returns a hit only where `is_solid`
    /// answers true, so the walk steps through a non-solid cell and `broken` is
    /// never called on it. `base:water` declares
    /// `solid = false` and `breakable = false`, and a swing at a water cell
    /// empties whatever solid block stands behind it — measured in
    /// `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs`,
    /// which reddens the day that stops being true.
    ///
    /// **Splitting `solid` into drawn, occludes and targetable makes this live.**
    /// The moment a non-solid block can be targeted, `broken` is called on one for
    /// the first time and `breakable = false` acquires a player-visible
    /// consequence it does not have today. Whoever makes that split owes the
    /// scenario that cannot be written now: a break swung at water is refused and
    /// leaves the water in the cell. It is recorded here rather than in a spec
    /// folder because this is where somebody changing targetability reads.
    Indestructible,
    /// The cell the placement would land in holds a block content does not
    /// declare replaceable.
    ///
    /// **A question about the block being overwritten, never about the block
    /// being placed**, and read from what content declared rather than derived
    /// from whether the block stops a player. Solidity is a physics fact and
    /// replaceability a placement rule; they coincide across everything MVP 1
    /// ships and that coincidence is an accident, so deriving one from the other
    /// would put a game rule in the engine that content cannot override.
    Occupied,
    /// The cell the placement would land in is one the player's own box
    /// occupies.
    InsidePlayer,
    /// The place request names a block the registry does not know.
    ///
    /// Carried by name rather than collapsed into a storage refusal: the store
    /// would refuse an unknown name at the write in any case, so a caller that
    /// could not tell the two apart could not tell whether the check before the
    /// write was there at all.
    UnknownBlock { name: BlockName },
    /// The cell lies outside the world's storable range, on either side.
    OutsideWorld { at: BlockPos },
    /// Everything the store refuses that is not an out-of-world position.
    Storage(WorldError),
}

/// What `action` does to `world`, decided from the player state the server owns.
///
/// The ray leaves the **eye** — `position + Y * EYE_HEIGHT` — along the
/// orientation the tick has already turned and limited, and reaches [`REACH`]
/// blocks.
pub(crate) fn resolve(
    action: &ActionIntent,
    player: &PlayerState,
    world: &mut World,
) -> EditReport {
    let (eye, direction) = aim(player);
    let Some(hit) = targeted(eye, direction, REACH, world) else {
        return EditReport::Refused(Refusal::NoTarget);
    };
    match action {
        ActionIntent::Break => broken(hit.cell, world),
        ActionIntent::Place { block } => placed(&hit, block, player, world),
    }
}

/// What breaking the block at `cell` does.
///
/// **Breakability and residue are two independent claims, and reading one off
/// the other is the mistake this shape exists to prevent.** Whether a block can
/// be broken is what content declares in `breakable`; what the cell then holds
/// is what it declares in `breaks_into`, and naming none means the cell is left
/// **empty** rather than that the block is indestructible. Nothing is not a
/// block, so there is no name for the engine to pick here and none is picked.
fn broken(cell: BlockPos, world: &mut World) -> EditReport {
    let Some(at) = inside_the_world(cell) else {
        return EditReport::Refused(Refusal::OutsideWorld { at: cell });
    };
    // Three arms and never two. `let Some(Contents::Holds(..)) = .. else` would
    // answer "there is no such cell" and "this cell holds nothing" with one
    // refusal, and both readings reach the same outcome here — which is what
    // would make the collapse invisible in the report.
    let broken = match world.block_at(cell) {
        None => return EditReport::Refused(Refusal::NoTarget),
        // The walk stops only at a solid cell and an empty cell is not solid, so
        // nothing reaches this. It keeps its own arm so that a break that ever
        // did reach an empty cell refuses rather than being read as a cell the
        // world does not have.
        Some(Contents::Empty) => return EditReport::Refused(Refusal::NoTarget),
        Some(Contents::Holds(name)) => name.clone(),
    };
    let declared = match world.registry().resolve(&broken) {
        Ok(definition) => (definition.breakable, definition.breaks_into.clone()),
        Err(refused) => return EditReport::Refused(Refusal::Storage(refused.into())),
    };
    let (breakable, named) = declared;
    if !breakable {
        return EditReport::Refused(Refusal::Indestructible);
    }
    // `breaks_into` absent means the cell is left empty; present names the block
    // left behind. No fallback, because there is no name for the engine to fall
    // back to and picking one would be a game rule content could not override.
    let residue = named.map_or(Contents::Empty, Contents::Holds);
    match world.break_at(at, residue.as_ref()) {
        Ok(()) => EditReport::Changed {
            cell,
            from: Contents::Holds(broken),
            to: residue,
        },
        Err(refused) => EditReport::Refused(Refusal::Storage(refused)),
    }
}

/// What placing `block` against the face `hit` came in through does.
///
/// **The cell is the one the ray came from**, one step back through the face it
/// entered by — so a placement lands on the near side of what you are looking
/// at, never inside it and never behind it.
///
/// The four refusals below are asked in the order they are written, and each
/// answers before anything is written. Their order is what a report says when
/// more than one applies, and the one thing it must not do is let a rule's
/// refusal arrive under another rule's name.
fn placed(hit: &Hit, block: &BlockName, player: &PlayerState, world: &mut World) -> EditReport {
    let Some(face) = hit.face else {
        return EditReport::Refused(Refusal::NoFace);
    };
    let cell = stepped(hit.cell, face);
    if let Err(refused) = world.registry().resolve(block) {
        return EditReport::Refused(unresolved(block, refused));
    }
    if let Err(refused) = overwritable(cell, world) {
        return EditReport::Refused(refused);
    }
    if occupies(player.position, cell) {
        return EditReport::Refused(Refusal::InsidePlayer);
    }
    // Both ways out of the world under one name: a negative cell has no unsigned
    // position to be stored at, and a cell past the far edge is one the world
    // does not reach. That is one thing a caller asked about, so it answers with
    // one name — and a fixture built at either edge measures the same rule.
    let Some(at) = inside_the_world(cell) else {
        return EditReport::Refused(Refusal::OutsideWorld { at: cell });
    };
    // An empty cell keeps its own arm: it is a cell the world reaches that holds
    // nothing, and it is what a placement replaces *nothing* over.
    let replaced = match world.block_at(cell) {
        None => return EditReport::Refused(Refusal::OutsideWorld { at: cell }),
        Some(contents) => contents.cloned(),
    };
    match world.place_at(at, block) {
        Ok(()) => EditReport::Changed {
            cell,
            from: replaced,
            to: Contents::Holds(block.clone()),
        },
        Err(refused) => EditReport::Refused(outside_or_storage(cell, refused)),
    }
}

/// Whether a placement may overwrite what `cell` already holds.
///
/// **Read from what content declared and never derived from whether the block
/// stops a player.** Solidity is a physics fact and replaceability a placement
/// rule; they coincide across everything MVP 1 ships and that coincidence is an
/// accident, so `!is_solid` here would be a game rule in the engine that content
/// cannot override.
///
/// **A cell the world holds no block for is not an occupied cell**, and there
/// are now two of those rather than one. It is either a cell outside the world —
/// whose refusal is the range one two steps below, since calling it occupied
/// would report a cell past the edge under the name of a content rule — or a
/// cell inside the world that holds nothing. Both permit, and they are written
/// as separate arms because they are separate facts: reading one as the other is
/// how a position past the edge becomes ordinary empty space.
///
/// **An empty cell is overwritable because it is empty**, not because content
/// said so. `replaceable` applies to real blocks only and content can no longer
/// declare otherwise, which is correct: nothing is not content.
fn overwritable(cell: BlockPos, world: &World) -> Result<(), Refusal> {
    match world.block_at(cell) {
        None => Ok(()),
        Some(Contents::Empty) => Ok(()),
        Some(Contents::Holds(held)) => match world.registry().resolve(held) {
            Ok(definition) if definition.replaceable => Ok(()),
            Ok(_) => Err(Refusal::Occupied),
            Err(refused) => Err(Refusal::Storage(refused.into())),
        },
    }
}

/// Why the registry had nothing to say about the block a request named.
///
/// A name it does not know is the scenario; anything else it could refuse is a
/// registry that is not in the state the caller believes, and that is a storage
/// refusal rather than a rule about this request.
fn unresolved(block: &BlockName, refused: RegistryError) -> Refusal {
    match refused {
        RegistryError::UnknownName { .. } => Refusal::UnknownBlock {
            name: block.clone(),
        },
        other => Refusal::Storage(other.into()),
    }
}

/// The store's refusal as the action's, collapsing both ways out of the world
/// into one answer.
///
/// A negative cell is refused at the conversion above and a cell past the far
/// edge inside the store, and they are the one thing a caller asked about — so
/// they arrive under one name, carrying the cell that was asked for rather than
/// the unsigned one the store saw.
fn outside_or_storage(cell: BlockPos, refused: WorldError) -> Refusal {
    match refused {
        WorldError::OutsideWorld { .. } => Refusal::OutsideWorld { at: cell },
        other => Refusal::Storage(other),
    }
}

/// The block a client holds when nothing has chosen one for it: the first solid
/// block in registration order.
///
/// **A selection rule and not a legality one** — the server refuses a placement
/// naming a block the registry does not know and asks nothing else about it, so
/// this decides only what an MVP-1 client asks for by default. Choosing and
/// cycling it is another spec's.
///
/// It lives here rather than in the client because it is a policy, and the
/// client carries none. `None` means the registry holds no solid block at all,
/// which is a content pack a client can place nothing in.
#[must_use]
pub fn default_held_block(registry: &BlockRegistry) -> Option<BlockName> {
    (0..registry.registered_count())
        .filter_map(|raw| {
            registry
                .definition(BlockId::from_raw(u32::try_from(raw).ok()?))
                .ok()
        })
        .find(|definition| definition.is_solid)
        .map(|definition| definition.name.clone())
}

/// Where the player is looking: the eye it looks from, and the direction it
/// looks along.
///
/// Separated from the resolution because the ray's *origin* is where measuring
/// reach from the feet instead of the eye would be spelled, and a caller that
/// computed it inline would spell it in as many places as there are callers.
fn aim(player: &PlayerState) -> (Vec3, Vec3) {
    let pose = eye_pose(player);
    let eye = Vec3::from_array(pose.eye);
    (eye, Vec3::from_array(pose.target) - eye)
}
