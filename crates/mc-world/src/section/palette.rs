//! The distinct blocks a section holds, the position each one is known by
//! inside that section, and how many voxels still hold it.
//!
//! Names, not runtime ids. A palette position means nothing outside its own
//! section and a name means the same thing in every registry, so a section
//! carries no assumption about the registry it was built against — which is what
//! lets a world survive the block set changing underneath it.
//!
//! The counts are maintained as voxels are written, never recounted afterwards.
//! Recounting would make reclamation come out right even when the write path had
//! been keeping the wrong numbers all along, which is the one defect the counts
//! exist to expose rather than to hide.

use mc_core::id::BlockName;

/// One block a section holds, and how many of its voxels hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PaletteEntry {
    name: BlockName,
    voxels_holding: usize,
}

/// The blocks one section holds, in the order they were first written into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Palette {
    entries: Vec<PaletteEntry>,
}

impl Palette {
    /// A palette whose single entry `name` is held by `voxels` voxels.
    pub(super) fn filled_with(name: &BlockName, voxels: usize) -> Self {
        Self {
            entries: vec![PaletteEntry {
                name: name.clone(),
                voxels_holding: voxels,
            }],
        }
    }

    /// A palette holding the blocks `counted` names, in the order it names them,
    /// each held by the number of voxels beside it.
    ///
    /// The counts come from the voxels they describe rather than from a previous
    /// palette, because a description arriving from outside is the only thing
    /// that knows them — and an entry no voxel names is still kept, since only
    /// compaction removes an entry.
    pub(super) fn rebuilt(counted: impl Iterator<Item = (BlockName, usize)>) -> Self {
        Self {
            entries: counted
                .map(|(name, voxels_holding)| PaletteEntry {
                    name,
                    voxels_holding,
                })
                .collect(),
        }
    }

    /// One voxel that held the entry at `vacated` now holds `name`, and this is
    /// the position `name` occupies.
    ///
    /// The new reference is taken before the old one is given back, and the
    /// order is the whole of it: a voxel overwritten with the block it already
    /// holds would otherwise leave its entry momentarily at zero references —
    /// indistinguishable from an entry nothing holds, and so reclaimable by
    /// anything that looks in between.
    pub(super) fn replace(&mut self, vacated: usize, name: &BlockName) -> usize {
        let position = self.take_reference(name);
        self.release(vacated);
        position
    }

    /// How many voxels hold the entry at `position`, or `None` if the palette is
    /// shorter than that.
    ///
    /// Nothing in the engine asks. The counts steer compaction through
    /// [`surviving_entries`](Self::surviving_entries), which is a decision rather
    /// than a reading, so this exists for the guard that watches the counts move
    /// — and it is compiled only for that, rather than becoming a public surface
    /// nobody needed.
    #[cfg(test)]
    pub(super) fn refcount(&self, position: usize) -> Option<usize> {
        self.entries.get(position).map(|entry| entry.voxels_holding)
    }

    /// The positions of the entries at least one voxel still holds, in the order
    /// the palette holds them.
    ///
    /// Read off the counts the write path maintained. Nothing here walks the
    /// voxels, which is what makes a broken count show up as a wrong palette
    /// rather than being quietly corrected by the very operation that is
    /// supposed to expose it.
    pub(super) fn surviving_entries(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.voxels_holding > 0)
            .map(|(position, _)| position)
            .collect()
    }

    /// Keeps exactly the entries `surviving` names, in the order it names them.
    ///
    /// Driven by the same list the voxel indices were rewritten against, so the
    /// palette and the indices cannot disagree about where an entry ended up.
    pub(super) fn narrow_to(&mut self, surviving: &[usize]) {
        self.entries = surviving
            .iter()
            .filter_map(|position| self.entries.get(*position).cloned())
            .collect();
    }

    /// The block at `position`, or `None` if the palette is shorter than that.
    pub(super) fn name_at(&self, position: usize) -> Option<&BlockName> {
        self.entries.get(position).map(|entry| &entry.name)
    }

    /// How many entries the palette holds, referenced or not.
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Every entry, in insertion order.
    pub(super) fn iter(&self) -> impl ExactSizeIterator<Item = &BlockName> {
        self.entries.iter().map(|entry| &entry.name)
    }

    /// Records one more voxel holding `name`, adding it if the palette does not
    /// hold it yet, and reports where it sits.
    ///
    /// Finding before appending is what keeps a section a player edits over and
    /// over from growing a palette entry per edit.
    fn take_reference(&mut self, name: &BlockName) -> usize {
        let held = match self.position_of(name) {
            Some(held) => held,
            None => self.append(name),
        };
        if let Some(entry) = self.entries.get_mut(held) {
            entry.voxels_holding += 1;
        }
        held
    }

    /// Puts `name` at the end of the palette, held by nothing yet, and reports
    /// where it landed.
    fn append(&mut self, name: &BlockName) -> usize {
        let position = self.entries.len();
        self.entries.push(PaletteEntry {
            name: name.clone(),
            voxels_holding: 0,
        });
        position
    }

    /// Records one fewer voxel holding the entry at `position`.
    ///
    /// An entry that reaches zero stays where it is. Dropping it here would
    /// renumber every entry above it in the middle of an edit, which is work the
    /// tick loop should not be doing for a saving only meshing and persistence
    /// ever collect.
    fn release(&mut self, position: usize) {
        if let Some(entry) = self.entries.get_mut(position) {
            entry.voxels_holding = entry.voxels_holding.saturating_sub(1);
        }
    }

    /// Where `name` sits, if the palette holds it.
    ///
    /// A linear scan. A palette is a handful of entries in every section a real
    /// world produces, and a map keyed by name would cost more than it saved
    /// until that stops being true.
    ///
    /// An entry nothing holds is still found, because it is still an entry: a
    /// block written back into a section it briefly left belongs at the position
    /// it already had, not at a second one.
    fn position_of(&self, name: &BlockName) -> Option<usize> {
        self.entries.iter().position(|entry| entry.name == *name)
    }
}

#[cfg(test)]
mod tests {
    //! Guard. What a palette counts, and when.
    //!
    //! Compaction is only allowed to reclaim an entry that nothing refers to, and the
    //! only honest way to know that is a count maintained as voxels are written.
    //! Recounting from the voxel array at compaction time would make compaction come
    //! out right even when the write path had been keeping the wrong numbers all along
    //! — which is the very defect the scenarios about compaction exist to expose. So
    //! the counts are asserted here, directly, because they have no public surface at
    //! all and no behavioural test can see them.
    //!
    //! The third case below is the one worth reading twice. Overwriting a voxel with
    //! the block it already holds must take the new reference *before* giving the old
    //! one back. Done the other way round the entry passes through zero references —
    //! the exact condition that means "nothing holds this any more" — and whatever
    //! reads that condition, now or later, is entitled to act on it.

    use mc_core::id::{BlockName, NamespacedIdError};

    use super::Palette;
    use crate::section::VOXELS_PER_SECTION;

    /// Parsing the fixture names is the only fallible step in any guard here.
    type GuardResult = Result<(), NamespacedIdError>;

    const FILL: &str = "fixture:fill";
    const WRITTEN: &str = "fixture:written";
    const FURTHER: &str = "fixture:further";

    /// A palette in the state a freshly filled section leaves one in: a single entry,
    /// held by every voxel there is.
    fn filled() -> Result<Palette, NamespacedIdError> {
        Ok(Palette::filled_with(
            &BlockName::parse(FILL)?,
            VOXELS_PER_SECTION,
        ))
    }

    #[test]
    fn a_write_takes_a_reference_from_the_block_it_replaced() -> GuardResult {
        let mut palette = filled()?;

        let written = palette.replace(0, &BlockName::parse(WRITTEN)?);

        assert_eq!(
            (written, palette.refcount(0), palette.refcount(written)),
            (1, Some(VOXELS_PER_SECTION - 1), Some(1)),
            "one voxel of a filled section now holds something else, so the fill is held by \
             exactly one fewer voxel and the block that displaced it by exactly one"
        );
        Ok(())
    }

    #[test]
    fn overwriting_the_last_voxel_holding_a_block_leaves_its_entry_unreferenced() -> GuardResult {
        let mut palette = filled()?;
        let written = palette.replace(0, &BlockName::parse(WRITTEN)?);

        let further = palette.replace(written, &BlockName::parse(FURTHER)?);

        assert_eq!(
            (
                palette.refcount(written),
                palette.refcount(further),
                palette.len()
            ),
            (Some(0), Some(1), 3),
            "the entry that overwrite vacated is now held by nothing, which is the one thing \
             compaction may reclaim by — and it stays in the palette until compaction does, \
             because dropping it here would renumber every entry above it in the middle of an \
             edit"
        );
        Ok(())
    }

    #[test]
    fn overwriting_a_voxel_with_the_block_it_already_holds_keeps_its_entry_referenced()
    -> GuardResult {
        let mut palette = filled()?;
        let written = BlockName::parse(WRITTEN)?;
        let entry = palette.replace(0, &written);

        let again = palette.replace(entry, &written);

        assert_eq!(
            (again, palette.refcount(entry), palette.len()),
            (entry, Some(1), 2),
            "the reference has to be taken before the old one is given back: released first, \
             this entry would pass through zero references — indistinguishable from an entry \
             nothing holds — and come back out of the write either duplicated or reclaimable"
        );
        Ok(())
    }
}
