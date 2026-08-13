//! Which block a client holds when nothing has chosen one for it.
//!
//! This grades no acceptance scenario. It exists because the selection is a
//! *policy* and policies live in the simulation, not in the client — so the rule
//! has to be stated somewhere a test can read it, and "the first solid block in
//! registration order" is a rule with two ways to be wrong that a client would
//! never notice: it could take the first block registered whatever that block
//! is, or it could take whichever block a hash happened to yield first.
//!
//! **The registry is declared here rather than borrowed from the chamber
//! fixture, and that is the whole of what makes this falsifiable.** Base content
//! ships no block that stops nobody except water, and water is read last, so
//! over any registry built from base content the first block registered *is* the
//! first solid one and both readings agree. This registry registers a non-solid
//! block first on purpose: the two answers are then different, and one assertion
//! separates them.
//!
//! Choosing and cycling the held block, and any HUD for it, is another spec's
//! and is out of scope here.

mod support;

use std::error::Error;

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_sim::action::default_held_block;

use support::volume::registry_declaring;
use support::{DIRT, STONE, TestResult, WATER};

/// A registry whose first block stops nobody and whose second one does.
///
/// The names are content's own, declared with content's own solidity, so the
/// only thing invented here is the order they are applied in.
fn water_before_the_ground() -> Result<BlockRegistry, Box<dyn Error>> {
    registry_declaring(&[(WATER, false), (DIRT, true), (STONE, true)])
}

#[test]
fn the_default_held_block_is_the_first_solid_block_in_registration_order() -> TestResult {
    let registry = water_before_the_ground()?;

    assert_eq!(
        default_held_block(&registry)
            .as_ref()
            .map(BlockName::as_str),
        Some(DIRT),
        "the rule is the first *solid* block in registration order, and the registry this reads \
         registers a non-solid one before it. An answer of water would be the rule reading \
         registration order alone, an answer of stone would be a rule reading no order at all, \
         and both are answers a registry whose first block already stops a player could never \
         have produced"
    );
    Ok(())
}
