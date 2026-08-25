//! Which of a save's two records the two medium properties join, and what makes
//! two declarations the same block to a save.
//!
//! A save records every block twice: what it is to stand on, and what it looks
//! like. `save_folds_the_split_properties.rs` asks this question of `targetable`,
//! `drawn` and `occludes`, and this file is the same question asked of the two
//! properties that make a volume a medium. Whether a player can hold itself up in
//! a block, and how much that block slows what moves through it, decide what
//! happens when you walk into it and change not one pixel — so both belong with
//! solidity and the drop, and neither belongs beside the texture keys.
//!
//! Putting either on the appearance list would report a change to what the world
//! does to a player as an art edit, which is a report nobody is asked to act on;
//! leaving either off both lists would report it as nothing at all.
//!
//! # Both halves of each record, compared as one value
//!
//! A verdict cannot witness the pair. `resolve` asks about behaviour first and
//! answers alone, so a `changed` answer says nothing whatever about appearance —
//! an implementation that moved *both* folds is reported identically to one that
//! moved the right one. So the readings below go to the recorded hashes directly
//! and compare `(behaviour moved, appearance moved)` as a single value, with an
//! arm for a save that does not name the block at all.
//!
//! # Two content roots, read through the loader, and never two definitions built
//! in memory
//!
//! A `BlockDefinition` assembled in memory skips the declaration reader entirely,
//! so an in-memory fixture can state a combination no declaration produces — and
//! for these two fields it would skip the normalisation that makes `-0.0`
//! register as `0.0`, which is a rule about what two declarations *mean the same
//! thing* and so is exactly this file's subject. The scenarios say two content
//! roots and this file takes that literally: two directories on disk, read by the
//! loader a player's content goes through.
//!
//! Every root here states `solid = true` and varies the one field under test
//! against it, for the reason the file this one is modelled on records: `drawn`,
//! `occludes` and `targetable` each default to `solid`, so a root saying
//! `solid = false` states four things rather than one. The two medium fields do
//! **not** default to solidity — they are absent-means-a-constant — but the three
//! that do are on the same declarations, so the constraint stands unchanged.
//!
//! # The sameness reading carries its own control, in the same comparison
//!
//! "Two declarations meaning one thing fold alike" is an equality, and an equality
//! is satisfied forever by a fold that ignores the field. So the reading that
//! makes it is a pair: two roots meaning the same resistance against two roots
//! meaning different ones, compared as one value. The second half is what says the
//! comparison can report a difference at all, and without it the first half is
//! green against a fold that never learned the field.

mod common;
mod luau_common;

use std::error::Error;
use std::path::PathBuf;

use common::TestResult;
use common::persistence::{saved_requirements, world_at, world_holding};
use luau_common::{declaration_of, raw_field, registry_from, text_field};
use mc_core::block::BlockRegistry;
use mc_world::persistence::SaveRequirements;
use mc_world::world::WorldPos;
use tempfile::TempDir;

/// The block every root here declares, and the key it draws from.
///
/// Deliberately not each other: a fold that wrote a block's own name where its
/// texture key belongs has to have somewhere to be wrong.
const PINNED: &str = "fixture:andesite";
const A_KEY: &str = "fixture:quartz";

/// The file the declaration is written to.
const PINNED_FILE: &str = "andesite.luau";

/// The one cell the fixture world holds it at.
const A_CELL: WorldPos = world_at(1, 1, 1);

/// What a root states beyond the three fields a declaration must carry.
///
/// Written as the field and the value a declaration spells rather than as a Rust
/// value, because what a root states and what it leaves to the default are
/// different fixtures and the difference is the subject here.
const SAYS_NOTHING_MORE: &[(&str, &str)] = &[];
const SWIMMABLE: &[(&str, &str)] = &[("swimmable", "true")];
const RESISTS: &[(&str, &str)] = &[("move_resistance", "4")];

/// Three roots stating a resistance: two that mean `4.0` and one that means
/// something else.
///
/// **`4` and `4.0` are the same resistance written as the two things a
/// declaration may write it as** — a Luau integer and a Luau number — so a fold
/// over the number the loader retained treats them as one block, and a fold over
/// the literal an author typed does not. That is the sameness this file is about:
/// two declarations agreeing about every field they declare, not two declarations
/// spelled identically.
const RESISTS_AS_AN_INTEGER: &[(&str, &str)] = &[("move_resistance", "4")];
const RESISTS_AS_A_NUMBER: &[(&str, &str)] = &[("move_resistance", "4.0")];
const RESISTS_MORE: &[(&str, &str)] = &[("move_resistance", "5.0")];

/// How two saves' records of one block stand against each other.
///
/// **A total verdict.** The second arm is what a save that never named the block
/// produces, and it must not compare equal to any answer about two folds — a
/// fixture whose world lost its block would otherwise report "nothing moved" and
/// read as one of the two properties being on the wrong list.
#[derive(Debug, PartialEq, Eq)]
enum Records {
    /// Both saves name the block, and this is how its two folds compare.
    Folds {
        behaviour_moved: bool,
        appearance_moved: bool,
    },
    /// One of the two saves does not name the block at all.
    ASaveDoesNotNameTheBlock,
}

/// A content root declaring [`PINNED`] and nothing else, and the registry it
/// loads to.
///
/// The directory travels with the registry because it is temporary: dropped one
/// line early it takes the declaration with it, and the failure reads as a root
/// that declares no blocks.
struct Declared {
    registry: BlockRegistry,
    _root: TempDir,
}

#[test]
fn a_block_that_became_something_to_swim_in_records_a_different_behaviour_and_the_same_appearance()
-> TestResult {
    let says_nothing_more = declaring(SAYS_NOTHING_MORE)?;
    let swimmable = declaring(SWIMMABLE)?;

    let records = how_the_records_differ(
        &saved_against(&says_nothing_more)?,
        &saved_against(&swimmable)?,
    );

    assert_eq!(
        records,
        Records::Folds {
            behaviour_moved: true,
            appearance_moved: false,
        },
        "a block a player can now hold itself up in is a different block to walk into: the world \
         used to let you fall through it and now it does not, which is exactly the kind of thing a \
         player is asked about before their world opens. What did not move is a single pixel, so \
         the appearance half has to stand still — a record whose two halves both moved reports a \
         rebalance and a retexture for one edit, and a player told their world looks different as \
         well cannot tell which half to act on"
    );
    Ok(())
}

#[test]
fn a_block_that_began_slowing_what_moves_through_it_records_a_different_behaviour_and_the_same_appearance()
-> TestResult {
    let says_nothing_more = declaring(SAYS_NOTHING_MORE)?;
    let resists = declaring(RESISTS)?;

    let records = how_the_records_differ(
        &saved_against(&says_nothing_more)?,
        &saved_against(&resists)?,
    );

    assert_eq!(
        records,
        Records::Folds {
            behaviour_moved: true,
            appearance_moved: false,
        },
        "a volume that has started slowing what moves through it is a different volume to walk \
         into, and it is a change no still frame can show — so the behaviour half has to move and \
         the appearance half has to stand still. This is the half of the pair that a fold deriving \
         one medium field from the other cannot satisfy alongside the buoyancy reading above: the \
         two roots here differ in a number and say nothing whatever about swimming"
    );
    Ok(())
}

#[test]
fn two_declarations_meaning_one_resistance_record_one_behaviour_and_a_third_meaning_more_records_another()
-> TestResult {
    let as_an_integer = declaring(RESISTS_AS_AN_INTEGER)?;
    let as_a_number = declaring(RESISTS_AS_A_NUMBER)?;
    let resisting_more = declaring(RESISTS_MORE)?;

    let saved = saved_against(&as_an_integer)?;
    let agreement = (
        how_the_records_differ(&saved, &saved_against(&as_a_number)?),
        how_the_records_differ(&saved, &saved_against(&resisting_more)?),
    );

    assert_eq!(
        agreement,
        (
            Records::Folds {
                behaviour_moved: false,
                appearance_moved: false,
            },
            Records::Folds {
                behaviour_moved: true,
                appearance_moved: false,
            }
        ),
        "`move_resistance = 4` and `move_resistance = 4.0` are one resistance written the two ways \
         a declaration may write it, so the two roots state the same value for every field they \
         declare and a save has to record them as one block — a fold reaching the literal an \
         author typed rather than the number the loader retained would tell every player their \
         world was rebalanced by a reformat. **The second half is what makes the first mean \
         anything**: an equality between two folds is satisfied forever by a fold that never \
         learned the field, and `5.0` is the same comparison over a value that really did move"
    );
    Ok(())
}

/// A content root whose one declaration states `beyond` on top of the three
/// fields every declaration carries.
///
/// `solid = true` is stated on every root here rather than varied, because
/// `drawn`, `occludes` and `targetable` each default to it: a root saying
/// `solid = false` states four things and would be four edits rather than one.
///
/// # Errors
///
/// Returns an error if the root cannot be written or is refused.
fn declaring(beyond: &[(&str, &str)]) -> Result<Declared, Box<dyn Error>> {
    let root = TempDir::new()?;
    let mut fields = vec![
        text_field("name", PINNED),
        text_field("texture", A_KEY),
        raw_field("solid", "true"),
    ];
    for (field, value) in beyond {
        fields.push(raw_field(field, value));
    }
    let path = written_root(&root, &declaration_of(&fields))?;
    Ok(Declared {
        registry: registry_from(&path)?,
        _root: root,
    })
}

/// The root `directory` becomes once it holds `declaration` as its one block.
///
/// # Errors
///
/// Returns an error if the directory or the file cannot be written.
fn written_root(directory: &TempDir, declaration: &str) -> Result<PathBuf, Box<dyn Error>> {
    common::content_root(directory, &[(PINNED_FILE, declaration.to_owned())])
}

/// What a save of a world holding [`PINNED`], written against `declared`, says it
/// needs.
///
/// The save goes to a directory of its own and is read back before that directory
/// is dropped, so nothing here shares a path with anything else.
///
/// # Errors
///
/// Returns an error if the world cannot be built, or if the save cannot be
/// written or read back.
fn saved_against(declared: &Declared) -> Result<SaveRequirements, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let world = world_holding(&[(A_CELL, PINNED)], &declared.registry)?;
    saved_requirements(&directory, &world, &declared.registry)
}

/// How the two saves' records of [`PINNED`] stand against each other.
fn how_the_records_differ(one: &SaveRequirements, other: &SaveRequirements) -> Records {
    let (Some(one), Some(other)) = (recorded_in(one), recorded_in(other)) else {
        return Records::ASaveDoesNotNameTheBlock;
    };
    Records::Folds {
        behaviour_moved: one.0 != other.0,
        appearance_moved: one.1 != other.1,
    }
}

/// The two folds `requirements` records for [`PINNED`], or nothing where the save
/// does not name it.
fn recorded_in(requirements: &SaveRequirements) -> Option<(u64, u64)> {
    requirements
        .blocks()
        .iter()
        .find(|block| block.name.as_str() == PINNED)
        .map(|block| (block.behaviour.get(), block.appearance.get()))
}
