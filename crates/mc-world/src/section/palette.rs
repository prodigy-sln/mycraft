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

use super::Contents;

/// One thing a section's voxels hold — a block, or nothing — and how many of
/// them hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PaletteEntry {
    contents: Contents,
    voxels_holding: usize,
}

/// What one section's voxels hold, in the order it was first written into them.
///
/// Emptiness is an entry here and not a state beside the palette: a cell holding
/// nothing occupies a palette position exactly as a cell holding a block does,
/// which is what leaves the packed indices, the index widths, the reference
/// counts and compaction with nothing to know about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Palette {
    entries: Vec<PaletteEntry>,
}

impl Palette {
    /// A palette whose single entry `contents` is held by `voxels` voxels.
    ///
    /// One constructor and not two. A section holding nothing everywhere and a
    /// section holding one block everywhere are the same shape with a different
    /// entry at position 0, and a second constructor would be a second place the
    /// initial reference count could be wrong.
    pub(super) fn filled_with(contents: Contents, voxels: usize) -> Self {
        Self {
            entries: vec![PaletteEntry {
                contents,
                voxels_holding: voxels,
            }],
        }
    }

    /// A palette holding what `counted` names, in the order it names them, each
    /// held by the number of voxels beside it.
    ///
    /// The counts come from the voxels they describe rather than from a previous
    /// palette, because a description arriving from outside is the only thing
    /// that knows them — and an entry no voxel names is still kept, since only
    /// compaction removes an entry.
    pub(super) fn rebuilt(counted: impl Iterator<Item = (Contents, usize)>) -> Self {
        Self {
            entries: counted
                .map(|(contents, voxels_holding)| PaletteEntry {
                    contents,
                    voxels_holding,
                })
                .collect(),
        }
    }

    /// One voxel that held the entry at `vacated` now holds `contents`, and this
    /// is the position `contents` occupies.
    ///
    /// The new reference is taken before the old one is given back, and the
    /// order is the whole of it: a voxel overwritten with the block it already
    /// holds would otherwise leave its entry momentarily at zero references —
    /// indistinguishable from an entry nothing holds, and so reclaimable by
    /// anything that looks in between.
    pub(super) fn replace(&mut self, vacated: usize, contents: Contents<&BlockName>) -> usize {
        let position = self.take_reference(contents);
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

    /// What the entry at `position` holds, or `None` if the palette is shorter
    /// than that.
    ///
    /// **The `Option` says the position exists and nothing else.** Whether the
    /// voxels at that position hold a block is the [`Contents`] inside it. The
    /// two questions are one wrapper apart on purpose: a `None` here is a
    /// section whose own indices no longer name its own palette, which is a
    /// corruption, and an empty cell is an ordinary answer.
    pub(super) fn contents_at(&self, position: usize) -> Option<Contents<&BlockName>> {
        self.entries
            .get(position)
            .map(|entry| entry.contents.as_ref())
    }

    /// How many entries the palette holds, referenced or not.
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Every entry, in insertion order.
    pub(super) fn iter(&self) -> impl ExactSizeIterator<Item = Contents<&BlockName>> {
        self.entries.iter().map(|entry| entry.contents.as_ref())
    }

    /// Records one more voxel holding `contents`, adding an entry if the palette
    /// does not hold it yet, and reports where it sits.
    ///
    /// Finding before appending is what keeps a section a player edits over and
    /// over from growing a palette entry per edit.
    fn take_reference(&mut self, contents: Contents<&BlockName>) -> usize {
        let held = match self.position_of(contents) {
            Some(held) => held,
            None => self.append(contents),
        };
        if let Some(entry) = self.entries.get_mut(held) {
            entry.voxels_holding += 1;
        }
        held
    }

    /// Puts `contents` at the end of the palette, held by nothing yet, and
    /// reports where it landed.
    fn append(&mut self, contents: Contents<&BlockName>) -> usize {
        let position = self.entries.len();
        self.entries.push(PaletteEntry {
            contents: contents.cloned(),
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

    /// Where `contents` sits, if the palette holds it.
    ///
    /// A linear scan. A palette is a handful of entries in every section a real
    /// world produces, and a map keyed by what it holds would cost more than it
    /// saved until that stops being true.
    ///
    /// An entry nothing holds is still found, because it is still an entry: a
    /// block written back into a section it briefly left belongs at the position
    /// it already had, not at a second one. That is true of emptiness as well —
    /// a cell emptied, refilled and emptied again keeps one empty entry.
    fn position_of(&self, contents: Contents<&BlockName>) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.contents.as_ref() == contents)
    }
}

#[cfg(test)]
#[path = "palette_test.rs"]
mod tests;
