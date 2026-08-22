//! The table of names a save carries, and what a registry declared each of them
//! to be.
//!
//! One entry per **distinct name the world actually holds**, addressed by an
//! identifier local to that one file. A world repeats itself — a million cells
//! hold the same block — so the names live once in a table and the sections point
//! into it. That is what makes the missing-block report a list a player can read
//! rather than one failure per section.
//!
//! **The table is written ascending, and one decision satisfies four
//! requirements.** A save's bytes must not depend on the order a registry
//! happened to be built in, on which runtime id a block happened to be given, or
//! on which of two hash maps in one process iterated first. Sorting the names is
//! what makes all three true at once, and a missing-name report is ascending for
//! free because the table already is.
//!
//! **No hash-ordered collection appears in this module or anywhere beside it.**
//! Every hash map in a process is seeded separately, so a single hash-ordered
//! iteration reaching the file would make a save's bytes depend on nothing a
//! player did.

use std::collections::BTreeMap;

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;

use super::error::{LoadError, SaveError};
use super::format::{NameEntry, SaveNameId, TableRecord, appearance_of, behaviour_of};
use super::read::reader::{RequiredBlock, SaveRequirements};
use crate::section::{Contents, SectionData};

/// Whether the caller has asked for a save whose blocks have changed since it was
/// written to be refused rather than loaded.
///
/// **Not a `bool` and with no `Default`**, and that is the whole of it: a caller
/// cannot pass `true` by accident, cannot get either answer by forgetting the
/// argument, and cannot express the decision without having read what it is
/// about. What the *client* does when its player types nothing is the client's
/// policy and is stated there; nothing here has an opinion about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acceptance {
    /// Refuse a save whose blocks' declared behaviour has changed. Asked for
    /// explicitly, because opening such a save and playing on rewrites its
    /// recorded hashes and there is then nothing left to notice.
    OnlyUnchangedBlocks,
    /// Load it and report which blocks moved.
    ChangedBlocksToo,
}

/// What a registry makes of what a save needs.
///
/// Every list is ascending lexicographic, whatever order the save's own table
/// held its names in: the report is the one thing a player acts on, and they act
/// on it by reading it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RegistryVerdict {
    /// Names the registry does not hold. Never loadable, with or without
    /// acceptance — nothing can go in the cell, and that is not a judgement a
    /// player is in a position to make.
    pub missing: Vec<BlockName>,
    /// Names the registry holds whose declared behaviour has changed. Loadable
    /// only with the player's acceptance: the data is loadable and whether it
    /// *should* be is a judgement about their world.
    pub changed: Vec<BlockName>,
    /// Names the registry holds whose declared appearance alone has changed.
    ///
    /// **Nothing reports these, and the promise that something would is
    /// withdrawn.** This field said it was "reported so that a caller can say
    /// so"; no caller ever did, and none should — a line after every art edit is
    /// noise the line about a rebalance would hide in, which is the same
    /// reasoning that keeps a retexture out of the refusal.
    ///
    /// What it is for is being the third arm of a **total** classification. Every
    /// name a save carries lands on exactly one of `missing`, `changed` and this,
    /// which is what lets a test compare a whole verdict instead of asserting an
    /// absence. Drop it and a block whose appearance alone moved would fall into
    /// no list at all, so [`resolve`] could no longer be graded as a whole. It is
    /// load-bearing for the evidence rather than for a report.
    pub retextured: Vec<BlockName>,
}

impl RegistryVerdict {
    /// Nothing, or the refusal a load carrying this verdict has to answer with.
    ///
    /// **No acceptance covers a missing name**, and that asymmetry is the whole of
    /// this function. A missing block means nothing can go in the cell, so there
    /// is no answer a caller could give that would make the save loadable; a
    /// changed block means the data is loadable, and it is loaded unless the
    /// caller asked otherwise.
    ///
    /// The refusal carries **both** lists whichever one caused it, so that a
    /// caller who is refused while asking for nothing stricter than the default
    /// can see it was the missing half that turned them away. It is produced only
    /// when at least one of the two is non-empty.
    #[must_use]
    pub fn refusal(&self, accepting: Acceptance) -> Option<LoadError> {
        if !self.refuses(accepting) {
            return None;
        }
        Some(LoadError::Unresolvable {
            missing: self.missing.clone(),
            changed: self.changed.clone(),
        })
    }

    /// Whether a load carrying this verdict is refused, given the caller's
    /// decision.
    ///
    /// A retextured block is never a refusal: the blocks are the same blocks and
    /// only look different, and stopping a player over every texture edit teaches
    /// them that being stopped means nothing.
    fn refuses(&self, accepting: Acceptance) -> bool {
        !self.missing.is_empty()
            || (accepting == Acceptance::OnlyUnchangedBlocks && !self.changed.is_empty())
    }
}

/// What `registry` makes of `required`.
///
/// **Pure — no path, no file, no I/O.** The report a user interface will one day
/// render is reachable from a save's requirements and a registry and nothing
/// else, which is what makes it testable without a world and answerable without
/// reading a chunk.
///
/// A name resolving proves *a* block exists under it, not that it is the block
/// the world was built from — a mod updated, a mod forked, or a different mod
/// claiming the same name all pass a name-only check. So what the save recorded
/// each block to be is compared against what this registry declares it to be,
/// and the two halves of that record are compared separately.
#[must_use]
pub fn resolve(required: &SaveRequirements, registry: &BlockRegistry) -> RegistryVerdict {
    let mut verdict = RegistryVerdict::default();
    for block in required.blocks() {
        judge(&mut verdict, block, registry);
    }
    // Sorted where it is reported and not where it is read, because the table it
    // came out of may hold its names in any order at all.
    verdict.missing.sort();
    verdict.changed.sort();
    verdict.retextured.sort();
    verdict
}

/// Records which of `verdict`'s three lists `block` belongs on, if any.
///
/// Behaviour is asked first and answers alone: a block whose behaviour changed
/// is a block the player has to decide about whatever its texture did, and
/// reporting it twice would put one name in two lists of one report.
fn judge(verdict: &mut RegistryVerdict, block: &RequiredBlock, registry: &BlockRegistry) {
    let Ok(definition) = registry.resolve(&block.name) else {
        verdict.missing.push(block.name.clone());
        return;
    };
    if behaviour_of(definition) != block.behaviour {
        verdict.changed.push(block.name.clone());
    } else if appearance_of(definition) != block.appearance {
        verdict.retextured.push(block.name.clone());
    }
}

/// How many distinct names one save's table can address.
///
/// The identifier is 32 bits wide, and deliberately not the sixteen a section's
/// palette position is: a palette is bounded by a compacted section's 4096
/// entries, and a save-wide table is bounded by the distinct names across the
/// whole save.
const NAMES_A_TABLE_CAN_ADDRESS: usize = u32::MAX as usize;

/// Every distinct name a save needs, ascending, and what each was declared to
/// be.
///
/// Built once for the whole save and consulted per palette entry, which is what
/// lets a load report the complete set of what is missing before it touches a
/// chunk.
#[derive(Debug)]
pub(crate) struct NameTable {
    /// Ascending by name, which is both the written order and the reported one.
    entries: BTreeMap<BlockName, Declared>,
}

/// What a registry declared one block to be, and where it sits in the table.
#[derive(Debug, Clone, Copy)]
struct Declared {
    id: SaveNameId,
    behaviour: u64,
    appearance: u64,
}

impl NameTable {
    /// The table `described` needs, read against `registry`.
    ///
    /// The descriptions are the **compacted** ones, so an entry no voxel refers
    /// to any more never reaches the table. That is not a saving: a vacated entry
    /// naming a block the player has since uninstalled would make the world
    /// refuse to load over a block that is not in it.
    ///
    /// # Errors
    ///
    /// Returns [`SaveError::UnknownBlock`] if a description names a block
    /// `registry` does not declare. A world can legitimately hold one — it was
    /// built against a different registry — and there is nothing honest to record
    /// for it. Inventing a declaration would write a save that reports itself
    /// unchanged the next time it is opened against the registry that has the
    /// block.
    pub(crate) fn of<'a>(
        described: impl IntoIterator<Item = &'a SectionData>,
        registry: &BlockRegistry,
    ) -> Result<Self, SaveError> {
        let mut entries = BTreeMap::new();
        for name in names_among(described) {
            Self::declared_once(&mut entries, name, registry)?;
        }
        Self::numbered(entries)
    }

    /// Records what `registry` declares `name` to be, unless `entries` already
    /// holds it.
    ///
    /// A world repeats itself — a million cells hold the same block, and a
    /// section's palette names it once per section — so the second and every
    /// later sighting of a name costs a lookup and nothing more.
    fn declared_once(
        entries: &mut BTreeMap<BlockName, Declared>,
        name: &BlockName,
        registry: &BlockRegistry,
    ) -> Result<(), SaveError> {
        if entries.contains_key(name) {
            return Ok(());
        }
        entries.insert(name.clone(), Self::declared(name, registry)?);
        Ok(())
    }

    /// The save-table identifier `name` was given, or nothing where the table
    /// does not hold it.
    pub(crate) fn id_of(&self, name: &BlockName) -> Option<SaveNameId> {
        self.entries.get(name).map(|declared| declared.id)
    }

    /// The table as it is written into a save: ascending, each name beside what
    /// it was declared to be.
    pub(crate) fn record(&self) -> TableRecord {
        TableRecord {
            names: self
                .entries
                .iter()
                .map(|(name, declared)| NameEntry {
                    name: name.as_str().to_owned(),
                    behaviour: declared.behaviour,
                    appearance: declared.appearance,
                })
                .collect(),
        }
    }

    /// What `registry` declares `name` to be, as the two hashes a save records.
    fn declared(name: &BlockName, registry: &BlockRegistry) -> Result<Declared, SaveError> {
        let definition = registry
            .resolve(name)
            .ok()
            .ok_or_else(|| SaveError::UnknownBlock { name: name.clone() })?;
        Ok(Declared {
            // The identifier is assigned once the whole set is known, because it
            // is a position in the ascending order and nothing is in ascending
            // order until the last name has arrived.
            id: SaveNameId::new(0),
            behaviour: behaviour_of(definition).get(),
            appearance: appearance_of(definition).get(),
        })
    }

    /// The same entries with each one's identifier set to its position in the
    /// ascending order.
    ///
    /// Numbered here and not as each name arrives, because an identifier is a
    /// position in the ascending order and nothing is in ascending order until
    /// the last name has been seen.
    fn numbered(entries: BTreeMap<BlockName, Declared>) -> Result<Self, SaveError> {
        let found = entries.len();
        let mut numbered = BTreeMap::new();
        for (position, (name, declared)) in entries.into_iter().enumerate() {
            let position = u32::try_from(position)
                .ok()
                .ok_or(SaveError::TooManyNames {
                    found,
                    supported: NAMES_A_TABLE_CAN_ADDRESS,
                })?;
            numbered.insert(
                name,
                Declared {
                    id: SaveNameId::new(position),
                    ..declared
                },
            );
        }
        Ok(Self { entries: numbered })
    }
}

/// Every name the palettes of `described` hold, in the order they hold them.
///
/// Emptiness is skipped and it is the only thing skipped: there is no name in it
/// for a table to carry, and reserving one would give nothing a name at the one
/// place a stored format makes it permanent.
fn names_among<'a>(
    described: impl IntoIterator<Item = &'a SectionData>,
) -> impl Iterator<Item = &'a BlockName> {
    described
        .into_iter()
        .flat_map(|section| section.palette.iter())
        .filter_map(|entry| match entry {
            Contents::Empty => None,
            Contents::Holds(name) => Some(name),
        })
}
