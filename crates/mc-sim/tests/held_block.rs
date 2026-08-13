//! Which block a client holds when nothing has chosen one for it.
//!
//! This grades no acceptance scenario. It exists because the selection is a
//! *policy* and policies live in the simulation, not in the client — so the rule
//! has to be stated somewhere a test can read it, and "the first solid block in
//! registration order" is a rule with two ways to be wrong that a client would
//! never notice: it could take the first block registered, which over any
//! registry built from content is air, or it could take whichever block a hash
//! happened to yield first.
//!
//! The fixture registry applies base content before its own overlay and base
//! content is read in file-name order, so the first block registered is air and
//! the first *solid* one is dirt. Two different answers, one assertion.
//!
//! Choosing and cycling the held block, and any HUD for it, is another spec's
//! and is out of scope here.

mod support;

use mc_core::id::BlockName;
use mc_sim::action::default_held_block;

use support::chamber::fixture_registry;
use support::{DIRT, TestResult};

#[test]
fn the_default_held_block_is_the_first_solid_block_in_registration_order() -> TestResult {
    let registry = fixture_registry()?;

    assert_eq!(
        default_held_block(&registry)
            .as_ref()
            .map(BlockName::as_str),
        Some(DIRT),
        "the rule is the first *solid* block in registration order, and the registry this reads \
         registers a non-solid one before it — air is the first name applied, from the first file \
         of base content. An answer of air would be the rule reading registration order alone, \
         and any other answer would be a rule reading something that is not an order at all"
    );
    Ok(())
}
