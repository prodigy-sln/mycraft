//! The batch a re-mesh runs on: every section it needs, copied out of the world.
//!
//! **It hands out no borrow of the world it came from**, which is the whole
//! reason it exists as a type rather than as a pair of references. A re-mesh runs
//! on neither the tick thread nor the frame thread, so what it runs on has to be
//! something a thread can be given — and a batch that borrowed the simulation
//! would pin the tick behind the mesher for as long as it took.
//!
//! A section is at most a few kilobytes and a batch is seven of them per edited
//! section at worst, so the copy is well under 100 KB. Sharing them instead
//! (`Arc<Section>`, copy-on-write) is recorded as deferred until the world
//! outgrows a fixed footprint or a profile shows the copy.

use std::collections::BTreeMap;

use mc_world::mesh::{Facing, beside};
use mc_world::section::Section;

use super::{SectionKey, World};

/// Every section a batch of re-meshing needs, and which of them to mesh.
///
/// The two are not the same set: meshing a section decides its boundary faces
/// against the six sections around it, so a batch carries its keys' neighbours
/// as well — and a neighbour the world does not hold is simply absent, which is
/// what makes a face at the edge of the footprint visible rather than an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemeshWork {
    /// The sections to mesh and everything they are meshed against, each copied
    /// once however many keys need it.
    sections: BTreeMap<SectionKey, Section>,
    /// Which of them to mesh, in the dirty set's own ascending order.
    keys: Vec<SectionKey>,
}

impl RemeshWork {
    /// The sections to mesh.
    pub fn keys(&self) -> impl ExactSizeIterator<Item = SectionKey> + '_ {
        self.keys.iter().copied()
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
    pub fn take_remesh_work(&mut self) -> Option<RemeshWork> {
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
        })
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
