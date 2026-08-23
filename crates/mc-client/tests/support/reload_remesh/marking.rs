//! Which sections a reload left to mesh again, as a verdict a scenario compares.
//!
//! The reading is taken through the drain [`super`] describes, so it happens once
//! per scenario — asking a second time finds an empty set and reads as "nothing was
//! marked".

use std::collections::BTreeSet;

use mc_sim::replay::world::FOOTPRINT_COLUMNS;
use mc_sim::world::{RemeshWork, SectionKey};
use mc_world::column::SECTIONS_PER_COLUMN;

use crate::input::InputHarness;

use super::{EVERY_SECTION_OF_THE_SHIPPED_WORLD, Section};

/// Which sections a reload left to be meshed again.
///
/// **A total verdict**, so an assertion against the whole-world arm rejects a
/// reload that marked nothing, one that marked a subset, and one that marked a
/// section twice — the last of which is what a dirty set that stopped being a set
/// would produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marking {
    /// Nothing at all was left to mesh again.
    NoSectionAtAll,
    /// Every section of the shipped world, each exactly once.
    EverySectionOfTheShippedWorld { marked: usize },
    /// Some other set, and how it differs from the shipped world's.
    Sections {
        marked: usize,
        distinct: usize,
        missing: Vec<Section>,
        beyond: Vec<Section>,
    },
}

/// Every section of the shipped world, as the set a whole-world mark produces.
///
/// Built from the footprint's own declarations rather than from a read of a world,
/// so it is an independent statement of what "every section" means.
#[must_use]
pub fn every_section_of_the_shipped_world() -> BTreeSet<Section> {
    let across = i32::try_from(FOOTPRINT_COLUMNS).unwrap_or(0);
    let stacked = usize::try_from(SECTIONS_PER_COLUMN).unwrap_or(0);
    (0..across)
        .flat_map(move |x| (0..across).map(move |z| (x, z)))
        .flat_map(move |(x, z)| (0..stacked).map(move |index| (x, z, index)))
        .collect()
}

/// What `client` was left to mesh again, taken once.
pub fn marked(client: &mut InputHarness) -> Marking {
    let Some(work) = client.take_remesh_work() else {
        return Marking::NoSectionAtAll;
    };
    marking_of(&keys_of(&work))
}

/// What a key list amounts to as a [`Marking`].
#[must_use]
pub fn marking_of(keys: &[SectionKey]) -> Marking {
    let named: Vec<Section> = keys.iter().copied().map(section_of).collect();
    let distinct: BTreeSet<Section> = named.iter().copied().collect();
    let whole = every_section_of_the_shipped_world();
    if named.len() == distinct.len() && distinct == whole {
        return Marking::EverySectionOfTheShippedWorld {
            marked: named.len(),
        };
    }
    Marking::Sections {
        marked: named.len(),
        distinct: distinct.len(),
        missing: whole.difference(&distinct).copied().collect(),
        beyond: distinct.difference(&whole).copied().collect(),
    }
}

/// The whole shipped world, marked once each, for a scenario to compare against.
#[must_use]
pub const fn every_section_once() -> Marking {
    Marking::EverySectionOfTheShippedWorld {
        marked: EVERY_SECTION_OF_THE_SHIPPED_WORLD,
    }
}

/// Which sections `work` will mesh, in the order the dirty set holds them.
#[must_use]
pub fn keys_of(work: &RemeshWork) -> Vec<SectionKey> {
    work.keys().collect()
}

/// One re-mesh key as a comparable triple.
#[must_use]
pub const fn section_of(key: SectionKey) -> Section {
    (key.column.x, key.column.z, key.index)
}
