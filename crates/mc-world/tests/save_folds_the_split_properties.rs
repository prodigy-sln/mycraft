//! Which of a save's two records each of the three new declaration properties
//! joins — and, for one of them, what a player is told about it.
//!
//! A save records every block twice: what it is to stand on, and what it looks
//! like. The two carry separate revision bytes, and the reason is the whole of
//! this file. `targetable` changes what a swing *does* to a world, so it belongs
//! with solidity and breakability; `drawn` and `occludes` change nothing but the
//! picture, so they belong with the texture keys. Put either of the last two on
//! the behaviour list and every player in existence is told that every block they
//! built with behaves differently, on the strength of a rendering field.
//!
//! # Both halves of each record, compared as one value
//!
//! A verdict cannot witness the pair. `resolve` asks about behaviour first and
//! answers alone, so a `changed` answer says nothing whatever about appearance —
//! an implementation that moved *both* folds is reported identically to one that
//! moved the right one. So the two readings below go to the recorded hashes
//! directly and compare `(behaviour moved, appearance moved)` as a single value,
//! with an arm for a save that does not name the block at all.
//!
//! # Two content roots, read through the loader, and never two definitions built
//! in memory
//!
//! Each of the three properties **defaults to whatever its own declaration says
//! about `solid`**, and that default lives in the Luau reader. A `BlockDefinition`
//! assembled in memory skips it entirely, so an in-memory fixture can state a
//! combination no declaration produces and can miss the very rule that decides
//! whether an edit moved one field or four. The scenarios say *two content roots*
//! and this file takes that literally: two directories on disk, read by the
//! loader a player's content goes through.
//!
//! It is also what makes the fixtures below say what they mean. A root stating
//! `solid = false` and nothing else states all four of solid, drawn, occludes and
//! targetable as false — so every root here keeps `solid = true` on both sides and
//! states the one property under test against it. The one that moves is then the
//! only difference between the two roots, which is a constraint on the fixture
//! that no assertion can enforce.
//!
//! # The occlusion reading is a whole verdict, and the reason is the mirror of
//! the changed-blocks one
//!
//! "No changed block was named" is exactly what an `occludes` folded into
//! **neither** list would produce, so an absence assertion there is green against
//! the defect it exists to catch. The verdict is compared whole — all three
//! lists — and the reading beside it moves `targetable` over the same fixtures so
//! that the empty `changed` list has something saying it can be non-empty at all.

mod common;
mod luau_common;

use std::error::Error;
use std::path::PathBuf;

use common::TestResult;
use common::persistence::{saved_requirements, world_at, world_holding};
use luau_common::{declaration_of, raw_field, registry_from, text_field};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{RegistryVerdict, SaveRequirements, resolve};
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
/// Written as the field and the value a declaration spells rather than as a
/// `bool`, because what a root states and what it leaves to the default are
/// different fixtures and the difference is the subject here.
const SAYS_NOTHING_MORE: &[(&str, &str)] = &[];
const NOT_TARGETABLE: &[(&str, &str)] = &[("targetable", "false")];
const NOT_DRAWN: &[(&str, &str)] = &[("drawn", "false")];
const DOES_NOT_OCCLUDE: &[(&str, &str)] = &[("occludes", "false")];

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
fn a_block_whose_targetability_alone_moved_records_a_different_behaviour_and_the_same_appearance()
-> TestResult {
    let says_nothing_more = declaring(SAYS_NOTHING_MORE)?;
    let not_targetable = declaring(NOT_TARGETABLE)?;

    let records = how_the_records_differ(
        &saved_against(&says_nothing_more)?,
        &saved_against(&not_targetable)?,
    );

    assert_eq!(
        records,
        Records::Folds {
            behaviour_moved: true,
            appearance_moved: false,
        },
        "a block that can no longer be aimed at is a different block to swing at — `breakable` \
         and `breaks_into` only ever mattered because the swing arrived — so what a player has to \
         be asked about really did move. What did not move is a single pixel, so the appearance \
         half has to stand still: a record whose two halves both moved reports a rebalance and a \
         retexture for one edit, and a player told their world looks different as well cannot tell \
         which half to act on"
    );
    Ok(())
}

#[test]
fn a_block_that_stopped_being_drawn_records_the_same_behaviour_and_a_different_appearance()
-> TestResult {
    let says_nothing_more = declaring(SAYS_NOTHING_MORE)?;
    let not_drawn = declaring(NOT_DRAWN)?;

    let records = how_the_records_differ(
        &saved_against(&says_nothing_more)?,
        &saved_against(&not_drawn)?,
    );

    assert_eq!(
        records,
        Records::Folds {
            behaviour_moved: false,
            appearance_moved: true,
        },
        "a block that stopped being drawn is still the same block to stand on, to build through \
         and to break, so nothing a player has to decide about has changed and the behaviour half \
         must stand still. `drawn` on the behaviour list would tell every player in existence that \
         every block they built with behaves differently, over a rendering field — the exact \
         ambiguity the two revision bytes exist to prevent. The appearance half moving is what \
         says the field is recorded at all rather than dropped"
    );
    Ok(())
}

#[test]
fn a_block_that_stopped_occluding_is_reported_as_retextured_and_as_nothing_else() -> TestResult {
    let says_nothing_more = declaring(SAYS_NOTHING_MORE)?;
    let does_not_occlude = declaring(DOES_NOT_OCCLUDE)?;

    let verdict = resolve(
        &saved_against(&says_nothing_more)?,
        &does_not_occlude.registry,
    );

    assert_eq!(
        verdict,
        RegistryVerdict {
            missing: Vec::new(),
            changed: Vec::new(),
            retextured: vec![BlockName::parse(PINNED)?],
        },
        "whether a face behind a block is culled is what the world looks like and nothing else, so \
         this is an art edit and a player is told nothing about it. The whole verdict is compared \
         because the interesting half is an absence: `changed` and `missing` both empty is also \
         exactly what an `occludes` folded into *neither* list would produce, and the `retextured` \
         name is the only part of this that a dropped field cannot satisfy"
    );
    Ok(())
}

/// The control the reading above cannot supply for itself.
///
/// Its `changed` list is empty, and an implementation whose `changed` list is
/// *always* empty satisfies that half forever. This is the same comparison over
/// the same two roots with the behaviour property moved instead, so the two
/// answers differ in which list holds the name rather than in whether one is
/// produced.
#[test]
fn the_same_comparison_reports_a_block_whose_targetability_moved_as_changed_instead() -> TestResult
{
    let says_nothing_more = declaring(SAYS_NOTHING_MORE)?;
    let not_targetable = declaring(NOT_TARGETABLE)?;

    let verdict = resolve(
        &saved_against(&says_nothing_more)?,
        &not_targetable.registry,
    );

    assert_eq!(
        verdict,
        RegistryVerdict {
            missing: Vec::new(),
            changed: vec![BlockName::parse(PINNED)?],
            retextured: Vec::new(),
        },
        "the same two roots, the same save, and one property moved instead of the other — so the \
         name moves from one list to the other and nothing else about the answer changes. Without \
         this reading, `changed: []` above is satisfied by a comparison that can no longer report \
         a changed block at all"
    );
    Ok(())
}

/// A content root whose one declaration states `beyond` on top of the three
/// fields every declaration carries.
///
/// `solid = true` is stated on every root here rather than varied, because each of
/// the three properties defaults to it: a root saying `solid = false` states all
/// four as false and would be four edits rather than one.
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
