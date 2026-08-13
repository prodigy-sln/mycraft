//! What a cell holding nothing answers when it is asked whether it is solid, and
//! what asking costs.
//!
//! Solidity is a property a block was registered with, read back through the
//! registry and never recognised from a name — that is asserted next door, in
//! `block_semantics.rs`, and none of it changes here. What changes is that a cell
//! may now hold no block at all, and nothing is not a block: there is no name to
//! resolve, so the answer is `not solid` and the registry is never consulted.
//!
//! **The two unwanted cases pull in opposite directions on purpose, and that is
//! the whole design of this file.** A short-circuit that is too narrow fails the
//! second test, where the registry holds nothing whatever and an empty cell must
//! still answer without a lookup. A short-circuit that is too wide — every cell
//! answered `not solid` without consulting anything — passes both of the first
//! two and fails the third, where a cell holding a block the registry does not
//! know has to be refused by name rather than quietly called empty. Either test
//! alone can be satisfied by the mistake the other is about.

mod common;

use common::{TestResult, at, registry_declaring};
use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::section::{LocalPos, Section, SectionError};

/// A block this file's registries declare solid, so that "not solid" is never
/// the only answer available.
const STONE: &str = "base:stone";

/// A block written into a section against one registry and asked about against
/// another that does not register it.
///
/// An `example:` namespace: the scenario is about a name nothing is registered
/// under, and borrowing a shipped name would suggest the engine had an opinion
/// about which names those are.
const ABSENT: &str = "example:absent";

/// The cell each scenario asks about, and the one beside it that holds a block.
const ASKED_ABOUT: LocalPos = at(0, 0, 0);
const HOLDING_A_BLOCK: LocalPos = at(1, 0, 0);

#[test]
fn a_cell_holding_nothing_is_reported_not_solid() -> TestResult {
    let registry = registry_declaring(&[(STONE, true)])?;
    let mut section = Section::empty();
    section.set_block(HOLDING_A_BLOCK, &BlockName::parse(STONE)?, &registry)?;

    let solid = (
        section.is_solid_at(ASKED_ABOUT, &registry)?,
        section.is_solid_at(HOLDING_A_BLOCK, &registry)?,
    );

    assert_eq!(
        solid,
        (false, true),
        "there is nothing in the first cell to stand on, to hide a face behind or to walk \
         into. The second half is its control and it is in the same section against the same \
         registry: without it, a section answering `not solid` for every cell it has would \
         satisfy the first half exactly as a correct one does"
    );
    Ok(())
}

#[test]
fn a_cell_holding_nothing_is_not_solid_against_a_registry_that_holds_no_block_at_all() -> TestResult
{
    let registry = BlockRegistry::new();
    let section = Section::empty();

    let solid = section.is_solid_at(ASKED_ABOUT, &registry)?;

    assert!(
        !solid,
        "this registry knows no block whatsoever, and the answer still arrives — because \
         nothing is not a block and there was never a name to look up. Resolving an empty cell \
         through the registry anyway refuses here, which would make `is this cell solid` a \
         question a world can fail to answer about the commonest cell it has"
    );
    Ok(())
}

#[test]
fn asking_about_a_cell_holding_a_block_the_registry_does_not_know_is_refused_naming_it()
-> TestResult {
    let complete = registry_declaring(&[(ABSENT, true)])?;
    let mut section = Section::empty();
    section.set_block(ASKED_ABOUT, &BlockName::parse(ABSENT)?, &complete)?;
    let missing_it = registry_declaring(&[(STONE, true)])?;

    let refused = section.is_solid_at(ASKED_ABOUT, &missing_it);

    let described = format!("{refused:?}");
    let Err(SectionError::Registry(RegistryError::UnknownName { name })) = refused else {
        return Err(format!("expected an unknown-name refusal, got {described}").into());
    };
    assert_eq!(
        name.as_str(),
        ABSENT,
        "this cell holds a block, and which block it is has to be read back from a registry \
         that does not have it — so the honest answer is a refusal naming what it could not \
         resolve. Answering `not solid` here is the same shortcut that makes an empty cell \
         cheap, applied one cell too far: it turns a world naming a block nobody registered \
         into a world of holes, silently"
    );
    Ok(())
}
