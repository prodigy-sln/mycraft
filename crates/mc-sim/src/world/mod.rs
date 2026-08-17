//! The world a simulation owns: one type, two views of it, and one private
//! write.
//!
//! A block store says what is where and a bitset says what stops the player, and
//! the whole difficulty of an editable world is that those two can fall out of
//! step. Deriving one from the other would remove the problem outright — and was
//! rejected for a reason worth restating here, because the alternative looks
//! strictly simpler: the replay's overlap oracle judges the physics by
//! re-reading the world's blocks and asking the registry about every name it
//! finds, and it is the only assertion in the simulation that covers a whole run
//! rather than a declared fixture. A `Solidity` that read the store through the
//! registry would be *that identical chain*, so the one unscoped invariant the
//! simulation has would be judging itself and would go green forever.
//!
//! So the two views are kept, and kept in step **structurally rather than by a
//! calling convention**. This type owns the store, the collision view and the
//! registry both were resolved against; **nothing outside this module can write
//! any of the three**, and the accessors that read them hand out shared borrows,
//! which cannot.
//!
//! **Two functions write any of the three, and each settles solidity before it
//! writes either view.** `write` is one edit; `adopt` is the whole registry
//! replaced by content read while the game was running. The dirty set is not one
//! of the three and never was, which is why the marking functions beside them —
//! including a `pub` one — take nothing away from this claim.
//! This header used to claim exactly one, and hot reload falsified it. What has
//! not changed is what the claim was ever about: neither writes one view without
//! the other, and neither writes anything it has not already resolved. A caller
//! that swapped the registry and left the bitset to a later refresh would reopen
//! the disagreement, and the overlap oracle could not see it — that oracle
//! re-reads the world through the registry and would be agreeing with itself.
//!
//! **The visibility is load-bearing.** `write` and `adopt` carry no `pub` at
//! all, so they are visible in this module and its descendants and nowhere else
//! — and the action resolution and the reload admission that reach them are
//! *child* modules for exactly that reason. A sibling would have forced either
//! to be `pub(crate)`, which is a much weaker claim.

pub(crate) mod action;
pub(crate) mod clearing;
pub(crate) mod reload;
mod remesh;

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::column::{ChunkColumn, ColumnCoordinate};
use mc_world::section::{Contents, Section};
use mc_world::world::{Extent, VoxelWorld, WorldError, WorldPos};

use crate::player::{BlockPos, Solidity};
use crate::replay::SolidVoxels;
use crate::replay::prepare::{PrepareError, SectionQuads, mesh_world};

use remesh::with_its_neighbours;

pub use clearing::Clearing;
pub use remesh::RemeshWork;

/// Which section of which column an edit landed in.
///
/// Keyed by where a section *is* and not by where it was found, so a batch
/// assembled out of one world can be meshed without that world in hand.
///
/// **Ordered so that a drain is deterministic**, which is what keeps a run
/// reproducible when the same two edits arrive in a different order. The order
/// it happens to impose is `(column.x, column.z, index)` and is deliberately not
/// the assembly order `mesh_all` emits in — nothing downstream reads it, because
/// a re-meshed section is spliced back into the place it already occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SectionKey {
    pub column: ColumnCoordinate,
    pub index: usize,
}

/// The blocks a simulation owns, and what the player collides with.
pub struct World {
    blocks: VoxelWorld,
    solid: SolidVoxels,
    registry: Arc<BlockRegistry>,
    /// Which sections have been written since the last drain.
    ///
    /// A set keyed per *section* rather than a list of edits, so twenty thousand
    /// writes into one section leave one entry — the bound is the footprint's
    /// section count however long nothing drains it.
    dirty: BTreeSet<SectionKey>,
}

impl World {
    /// The world `blocks` describes, with its solidity resolved through
    /// `registry`.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownName`] if the blocks hold a name the
    /// registry does not know.
    pub fn new(blocks: VoxelWorld, registry: Arc<BlockRegistry>) -> Result<Self, RegistryError> {
        let solid = SolidVoxels::resolve(&blocks, &registry)?;
        Ok(Self {
            blocks,
            solid,
            registry,
            dirty: BTreeSet::new(),
        })
    }

    /// Which sections have been written since this was last asked, and clears
    /// them.
    ///
    /// Taking rather than reading is what makes a section re-meshed once per
    /// edit instead of once per drain for the rest of the run.
    pub fn take_dirty(&mut self) -> BTreeSet<SectionKey> {
        std::mem::take(&mut self.dirty)
    }

    /// What the cell at `at` holds, or `None` where the world does not reach.
    ///
    /// **The `Option` says the position is inside the world and nothing else.**
    /// A cell the world reaches and that holds nothing answers
    /// `Some(Contents::Empty)`; a cell past the edge answers `None`. A caller
    /// that folds the two together is asking one question where there are two,
    /// which is how a position outside the world gets read as ordinary empty
    /// space.
    #[must_use]
    pub fn block_at(&self, at: BlockPos) -> Option<Contents<&BlockName>> {
        self.blocks.block_at(inside_the_world(at)?).ok()
    }

    /// The registry this world's blocks are named against.
    #[must_use]
    pub fn registry(&self) -> &BlockRegistry {
        &self.registry
    }

    /// Every distinct block at least one cell still holds, ascending.
    ///
    /// All of them, in a declared order: the reload that reads this refuses a
    /// content set for every block it stopped declaring rather than for
    /// whichever was met first. [`SolidVoxels::resolve`] stops at the first and
    /// is not the instrument.
    #[must_use]
    pub fn names_held(&self) -> BTreeSet<&BlockName> {
        self.blocks
            .columns()
            .flat_map(ChunkColumn::sections)
            .flat_map(Section::names_in_use)
            .collect()
    }

    /// The blocks this world is made of.
    ///
    /// **`pub(crate)` and not `pub`**, so nothing outside this crate can reach
    /// them at all — which strengthens the property this module claims rather
    /// than weakening it. The one caller is the save path, which is in this
    /// crate because a save is server state and because which world a launch
    /// plays is policy.
    ///
    /// A shared borrow, so it cannot be a second way to write either view.
    #[must_use]
    pub(crate) const fn blocks(&self) -> &VoxelWorld {
        &self.blocks
    }

    /// How far the world reaches on each axis, in voxels.
    #[must_use]
    pub fn extent(&self) -> Extent {
        self.blocks.extent()
    }

    /// Every section of this world, meshed, in the declared assembly order.
    ///
    /// **The only way to mesh the world a launch actually plays**, and the
    /// reason the crate-visible `blocks` accessor does not have to become `pub`
    /// for one: what comes back is owned quads, which is a value rather than
    /// the store, so the claim that nothing outside this module can write any
    /// of the three views is untouched.
    ///
    /// Takes no registry, because this world owns the one its blocks were
    /// resolved against and a second opinion about a name is exactly the
    /// disagreement this type exists to make unspellable.
    ///
    /// **It marks nothing dirty.** Reading a world is not editing it, and a
    /// launch that handed over geometry while leaving the sections it had just
    /// meshed outstanding would ship the defect this is part of fixing: the
    /// frame path would re-mesh the whole world a batch at a time, drawing the
    /// wrong one until it had finished.
    ///
    /// # Errors
    ///
    /// Returns [`PrepareError::Mesh`] naming the first section in assembly
    /// order which could not be meshed.
    pub fn mesh(&self) -> Result<Vec<SectionQuads>, PrepareError> {
        mesh_world(&self.blocks, &self.registry)
    }

    /// Leaves `residue` where a broken block was.
    ///
    /// The residue is resolved by the caller — the block the broken one's own
    /// definition names, or nothing where it names none. This end of it is the
    /// write and nothing else.
    ///
    /// # Errors
    ///
    /// Returns whatever [`write`](Self::write) refuses.
    fn break_at(
        &mut self,
        cell: WorldPos,
        residue: Contents<&BlockName>,
    ) -> Result<(), WorldError> {
        self.write(cell, residue)
    }

    /// Puts `block` in a cell a placement was allowed to have.
    ///
    /// Whether it was allowed — the name is registered, the cell is replaceable,
    /// the player is not standing in it — is settled by the caller before this
    /// is reached. This end of it is the write and nothing else, exactly as
    /// [`break_at`](Self::break_at) is, and the two are separate functions
    /// because they are separate operations rather than because they do
    /// different things here.
    ///
    /// # Errors
    ///
    /// Returns whatever [`write`](Self::write) refuses.
    fn place_at(&mut self, cell: WorldPos, block: &BlockName) -> Result<(), WorldError> {
        self.write(cell, Contents::Holds(block))
    }

    /// **The one place either view is written**, and there is no other.
    ///
    /// Solidity is settled *before* either write, so a name the registry does
    /// not know refuses without having changed anything — and the store and the
    /// bitset are then written from that one answer. Deleting either line is the
    /// only way to make the two disagree, which is what makes a test that
    /// notices worth having.
    ///
    /// A cell being emptied settles that answer without a registry at all: there
    /// is nothing there to stand on, and no name to look up to find that out.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError::Registry`] if the registry does not know the block
    /// being written, and [`WorldError::OutsideWorld`] or
    /// [`WorldError::Section`] if the store refuses the position.
    fn write(&mut self, at: WorldPos, contents: Contents<&BlockName>) -> Result<(), WorldError> {
        let solid = match contents {
            Contents::Empty => false,
            Contents::Holds(block) => self.registry.resolve(block)?.is_solid,
        };
        match contents {
            Contents::Empty => self.blocks.empty_at(at)?,
            Contents::Holds(block) => self.blocks.set_block(at, block, &self.registry)?,
        }
        self.solid.set(at, solid);
        self.mark_dirty(at);
        Ok(())
    }

    /// **The other place either view is written**, and there is no third.
    ///
    /// Replaces the registry and the solidity it implies together. Solidity is
    /// settled first, exactly as [`write`](Self::write) settles it, so a
    /// registry that does not know a name some cell holds refuses without having
    /// changed anything. Leaving the bitset to a later refresh would reopen the
    /// disagreement this module's header is about, and no oracle in the tree
    /// could see it.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownName`] if the blocks hold a name
    /// `registry` does not know, with this world untouched.
    fn adopt(&mut self, registry: Arc<BlockRegistry>) -> Result<(), RegistryError> {
        let solid = SolidVoxels::resolve(&self.blocks, &registry)?;
        self.solid = solid;
        self.registry = registry;
        Ok(())
    }

    /// Records every section of this world as needing to be meshed again.
    ///
    /// A reload that changed what is drawn marks all of them: what this adds over
    /// a selective rule is the empty sections, which mesh to no quads.
    fn mark_every_section(&mut self) {
        self.dirty.extend(
            self.blocks
                .columns()
                .flat_map(|column| {
                    let coordinate = column.coordinate();
                    (0..column.sections().len()).map(move |index| SectionKey {
                        column: coordinate,
                        index,
                    })
                })
                .collect::<Vec<_>>(),
        );
    }

    /// Records the section holding `at` and the six around it as needing to be
    /// meshed again.
    ///
    /// **All seven, unconditionally — there is no "only if the voxel is on the
    /// boundary" test.** A block's own faces are decided against its
    /// neighbours', so a voxel on a section's outermost layer uncovers a face
    /// that belongs to the section beside it, and a mark that tested for that
    /// would be the fast thing rather than the correct one. An extra mark costs
    /// one section meshed for nothing; a missing one leaves a face that is not
    /// there drawn, or one that is, absent.
    ///
    /// **A section the footprint does not hold is passed over rather than
    /// reported.** The edge of a loaded world is not an error, and it is also
    /// what keeps the set bounded by the footprint's own section count however
    /// many edits accumulate behind a drain.
    fn mark_dirty(&mut self, at: WorldPos) {
        let Some((column, index)) = self.blocks.section_holding(at) else {
            return;
        };
        let held: Vec<SectionKey> = with_its_neighbours(SectionKey { column, index })
            .filter(|key| self.blocks.section_at(key.column, key.index).is_some())
            .collect();
        self.dirty.extend(held);
    }
}

/// The collision view is the **bitset**, never the registry.
///
/// That is what keeps the physics free of a name to look up, a registry to
/// consult, and so of any failure for the tick path to swallow. It is true today
/// and is worth keeping true.
impl Solidity for World {
    fn is_solid(&self, at: BlockPos) -> bool {
        self.solid.is_solid(at)
    }
}

/// A world's blocks would bury whatever a panic message was about, so what is
/// shown is how far it reaches and how many blocks it is named out of.
impl fmt::Debug for World {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("World")
            .field("extent", &self.blocks.extent())
            .field("registered", &self.registry.registered_count())
            .finish_non_exhaustive()
    }
}

/// A signed position as the unsigned one a world is indexed by, or nothing if
/// any coordinate is negative.
///
/// The player is not confined to the world and asks about voxels outside it, so
/// this refuses rather than converting — saturating would answer for the near
/// edge and wrapping for the far one, both silently and both about a column
/// nobody asked about.
pub(crate) fn inside_the_world(at: BlockPos) -> Option<WorldPos> {
    Some(WorldPos {
        x: at.x.try_into().ok()?,
        y: at.y.try_into().ok()?,
        z: at.z.try_into().ok()?,
    })
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
