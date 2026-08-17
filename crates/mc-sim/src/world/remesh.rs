//! The batch a re-mesh runs on: every section it needs, copied out of the world.
//!
//! **It hands out no borrow of the world it came from**, which is the whole
//! reason it exists as a type rather than as a pair of references. A re-mesh runs
//! on neither the tick thread nor the frame thread, so what it runs on has to be
//! something a thread can be given — and a batch that borrowed the simulation
//! would pin the tick behind the mesher for as long as it took.
//!
//! **The copy is well under 100 KB even when the batch is the whole world.** A
//! reload marks all 256 sections; measured, that is 44.5 KB of packed indices
//! across the 54 that carry any, under 70 KB with palettes and map nodes. The
//! other 202 carry none — a one-entry palette needs zero bits per voxel, and the
//! shipped world is uniform except where terrain meets air. **The bound therefore
//! rests on index-width tiering, not on the section count**, which is what makes
//! it survive whole-world marking. Sharing instead (`Arc<Section>`,
//! copy-on-write) stays deferred until the world outgrows a fixed footprint or a
//! profile shows the copy.

use std::collections::BTreeMap;
use std::sync::Arc;

use mc_core::block::BlockRegistry;
use mc_core::content::ContentSerial;
use mc_world::mesh::{Facing, beside};
use mc_world::section::Section;

use super::{SectionKey, World};

/// Every section a batch of re-meshing needs, and which of them to mesh.
///
/// The two are not the same set: meshing a section decides its boundary faces
/// against the six sections around it, so a batch carries its keys' neighbours
/// as well — and a neighbour the world does not hold is simply absent, which is
/// what makes a face at the edge of the footprint visible rather than an error.
#[derive(Debug, Clone)]
pub struct RemeshWork {
    /// The sections to mesh and everything they are meshed against, each copied
    /// once however many keys need it.
    sections: BTreeMap<SectionKey, Section>,
    /// Which of them to mesh, in the dirty set's own ascending order.
    keys: Vec<SectionKey>,
    /// The registry the world these sections came out of was resolved against.
    ///
    /// It travels with the batch so that meshing against a second opinion is
    /// unspellable rather than checked: whoever meshes this takes no registry
    /// argument, so a stale one has no way in.
    registry: Arc<BlockRegistry>,
    /// Which accepted content set the world was serving when this was drained.
    serial: ContentSerial,
}

impl RemeshWork {
    /// The sections to mesh.
    pub fn keys(&self) -> impl ExactSizeIterator<Item = SectionKey> + '_ {
        self.keys.iter().copied()
    }

    /// The registry these sections are named against.
    #[must_use]
    pub fn registry(&self) -> &BlockRegistry {
        &self.registry
    }

    /// Which accepted content set this batch was drained under.
    #[must_use]
    pub const fn serial(&self) -> ContentSerial {
        self.serial
    }

    /// The section at `key`, or nothing where the world this came from held
    /// none.
    #[must_use]
    pub fn section(&self, key: SectionKey) -> Option<&Section> {
        self.sections.get(&key)
    }
}

impl World {
    /// What has to be re-meshed for the edits since the last drain to be seen,
    /// or nothing when there have been none.
    ///
    /// The dirty set is taken, so a section is re-meshed once per edit rather
    /// than once per drain for the rest of the run.
    pub fn take_remesh_work(&mut self, serial: ContentSerial) -> Option<RemeshWork> {
        let dirty = self.take_dirty();
        if dirty.is_empty() {
            return None;
        }
        let mut sections = BTreeMap::new();
        for needed in dirty.iter().copied().flat_map(with_its_neighbours) {
            self.copy_section_into(&mut sections, needed);
        }
        Some(RemeshWork {
            sections,
            keys: dirty.into_iter().collect(),
            registry: Arc::clone(&self.registry),
            serial,
        })
    }

    /// Records `keys` as needing to be meshed again.
    ///
    /// The one caller is a batch discarded for having been meshed against content
    /// that stopped serving: its sections go back into the dirty set or they stay
    /// stale for the rest of the run.
    pub fn mark_for_remesh(&mut self, keys: Vec<SectionKey>) {
        self.dirty.extend(keys);
    }

    /// Copies the section at `key` into `sections`, or leaves it absent where
    /// this world holds no such section.
    fn copy_section_into(&self, sections: &mut BTreeMap<SectionKey, Section>, key: SectionKey) {
        if sections.contains_key(&key) {
            return;
        }
        if let Some(section) = self.blocks.section_at(key.column, key.index) {
            sections.insert(key, section.clone());
        }
    }
}

/// A section and the six around it, as keys.
///
/// The neighbour arithmetic is [`beside`]'s and is not restated here: a facing's
/// own axis and sign decide which column is next to this one and which section
/// is over it, and the mesher answers the same question from the same place.
pub(super) fn with_its_neighbours(key: SectionKey) -> impl Iterator<Item = SectionKey> {
    std::iter::once(key).chain(Facing::ALL.into_iter().filter_map(move |facing| {
        beside(key.column, key.index, facing).map(|(column, index)| SectionKey { column, index })
    }))
}
