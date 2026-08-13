//! Three outcomes, and the one the player is allowed to overrule.
//!
//! A name resolving proves *a* block exists under it, not that it is the block
//! the world was built from — a mod updated, a mod forked, or a different mod
//! claiming the same name all load silently against a name-only check. So a save
//! records what each block it names was declared to be, and a load compares.
//!
//! What the comparison finds falls into three cases, and only the middle one is
//! a question:
//!
//! - a name the registry does not hold is a **hard refusal**. Nothing can go in
//!   the cell, and that is not a judgement a player is in a position to make;
//! - a name whose declared behaviour has changed is **the player's decision**.
//!   The data is loadable, and whether it *should* be is a judgement about their
//!   own world;
//! - a name present and unchanged loads, silently.
//!
//! **Acceptance never covers a missing name.** That asymmetry is the whole
//! shape, and the middle test here is what makes it consequential rather than
//! merely stated: one of each, loaded with acceptance given, must still be
//! refused — and the report has to let the player see that it was the missing
//! half that refused them, because that is the half acceptance can never answer.
//!
//! This file owns the refusing half of that table only. The three loadable rows
//! need a world to have been produced to compare against, and are asserted where
//! one can be.
//!
//! **Fixture constraint no assertion can enforce.** A block counts as changed
//! when its declared behaviour differs, so the fixtures vary **solidity** and
//! nothing else: the texture stays the block's own name in both registries, and
//! the name is what it always was. A fixture that varied the texture instead
//! would be describing a retexture, which is a different row of the table
//! entirely and is not refused at all.

mod common;

use std::error::Error;

use common::persistence::{saved_requirements, world_at, world_holding};
use common::{TestResult, registry_declaring, registry_of};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{self, Acceptance, LoadError, SaveRequirements};
use mc_world::world::WorldPos;
use tempfile::TempDir;

/// The two blocks whose declared behaviour changes between writing and reading.
const REDECLARED: [&str; 2] = ["fixture:andesite", "fixture:basalt"];

/// The block a save names that the loading registry does not hold, and the one
/// it holds under a changed declaration.
const GONE: &str = "fixture:chert";
const REDECLARED_ALONE: &str = "fixture:andesite";

/// Both of them, in the order a save names them.
const ONE_OF_EACH: [&str; 2] = [REDECLARED_ALONE, GONE];

/// Where the blocks of a fixture world sit.
const CELLS: [WorldPos; 2] = [world_at(1, 1, 1), world_at(2, 3, 4)];

/// `names` as block names, in the order given.
fn block_names(names: &[&str]) -> Result<Vec<BlockName>, Box<dyn Error>> {
    let mut parsed = Vec::with_capacity(names.len());
    for name in names {
        parsed.push(BlockName::parse(name)?);
    }
    Ok(parsed)
}

/// A registry declaring each of `names` with the solidity given.
///
/// Solidity is the behaviour these fixtures vary, because it is behaviour: a
/// block that was solid and is not any more is a world whose ground you fall
/// through, which is exactly the change a player is being asked about.
fn declaring(names: &[&str], solid: bool) -> Result<BlockRegistry, Box<dyn Error>> {
    let declared: Vec<(&str, bool)> = names.iter().map(|name| (*name, solid)).collect();
    registry_declaring(&declared)
}

/// What a save of a world holding `held`, written against `written_against`,
/// says it needs.
fn a_save_naming(
    directory: &TempDir,
    held: &[&str],
    written_against: &BlockRegistry,
) -> Result<SaveRequirements, Box<dyn Error>> {
    let placed: Vec<(WorldPos, &str)> = CELLS.into_iter().zip(held.iter().copied()).collect();
    let world = world_holding(&placed, written_against)?;
    saved_requirements(directory, &world, written_against)
}

#[test]
fn a_save_whose_two_blocks_were_redeclared_is_refused_without_acceptance_naming_both() -> TestResult
{
    let directory = TempDir::new()?;
    let written_against = declaring(&REDECLARED, true)?;
    let required = a_save_naming(&directory, &REDECLARED, &written_against)?;
    let redeclared = declaring(&REDECLARED, false)?;

    let verdict = persistence::resolve(&required, &redeclared);

    assert_eq!(
        verdict.refusal(Acceptance::OnlyUnchangedBlocks),
        Some(LoadError::Unresolvable {
            missing: Vec::new(),
            changed: block_names(&REDECLARED)?
        }),
        "both names resolve, so a load that checked names alone would open this world and say \
         nothing — and the blocks it is made of are no longer the blocks it was built from. Both \
         are named because the player is deciding about a world and not about a block, and a \
         report naming one of two changed blocks is a decision made on half the facts. The \
         missing list is empty, which is the other half of the answer: nothing here is \
         unrecoverable, so this is a question rather than a refusal"
    );
    Ok(())
}

#[test]
fn a_save_naming_one_gone_block_and_one_redeclared_is_refused_even_with_acceptance() -> TestResult {
    let directory = TempDir::new()?;
    let written_against = declaring(&ONE_OF_EACH, true)?;
    let required = a_save_naming(&directory, &ONE_OF_EACH, &written_against)?;
    let now_holding = declaring(&[REDECLARED_ALONE], false)?;

    let verdict = persistence::resolve(&required, &now_holding);

    assert_eq!(
        verdict.refusal(Acceptance::ChangedBlocksToo),
        Some(LoadError::Unresolvable {
            missing: block_names(&[GONE])?,
            changed: block_names(&[REDECLARED_ALONE])?
        }),
        "this is the asymmetry made consequential rather than stated. The player has already said \
         they will take the changed blocks, and it is still refused — because acceptance is a \
         judgement about blocks that exist, and no judgement puts a block back that nobody has. \
         Two lists in one refusal is what lets them see *which* half turned them away: told only \
         `refused`, a player who has already agreed to the risk has no way to tell they were \
         answering the wrong question"
    );
    Ok(())
}

#[test]
fn a_save_naming_a_block_nobody_holds_is_refused_with_acceptance_given() -> TestResult {
    let directory = TempDir::new()?;
    let written_against = registry_of(&[GONE])?;
    let required = a_save_naming(&directory, &[GONE], &written_against)?;

    let verdict = persistence::resolve(&required, &BlockRegistry::new());

    assert_eq!(
        verdict.refusal(Acceptance::ChangedBlocksToo),
        Some(LoadError::Unresolvable {
            missing: block_names(&[GONE])?,
            changed: Vec::new()
        }),
        "acceptance is not a `load anyway`, and this is the plainest statement of it: one missing \
         block, nothing else wrong, the player saying yes, and the answer is still no. A single \
         boolean threaded through the load would be read as permission and this save would open \
         into a world with a hole in it — which is the failure the two-valued decision exists to \
         make unsayable"
    );
    Ok(())
}
