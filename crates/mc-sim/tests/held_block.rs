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
//! **The split of `solid` into what is drawn, what occludes and what may be
//! aimed at leaves this rule where it was, deliberately.** A held block is one
//! you place to build with and building means an obstacle, so the fourth
//! consumer of the old bit is answered explicitly rather than left to fall out.
//! The registry below is what keeps that honest in one direction — a rule
//! reading plain registration order answers water — and
//! `a_registry_of_blocks_that_stop_nobody_offers_none_though_one_is_drawn`
//! keeps it honest in the other, over a fixture nothing could state before the
//! three properties became separable: a block that is drawn and may be aimed at
//! and still stops nobody.
//!
//! Choosing and cycling the held block, and any HUD for it, is another spec's
//! and is out of scope here.

mod support;

use std::error::Error;

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_sim::action::default_held_block;

use support::volume::{
    DRAWN_AND_AIMED_AT_ONLY, Declaration, registry_declaring, registry_of_declarations,
};
use support::{DIRT, STONE, TestResult, WATER, content_registry};

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

#[test]
fn the_shipped_content_puts_dirt_in_a_new_players_hand() -> TestResult {
    let registry = content_registry()?;

    assert_eq!(
        default_held_block(&registry)
            .as_ref()
            .map(BlockName::as_str),
        Some(DIRT),
        "content is read in file-name order, so the shipped root registers dirt, grass, stone and          water in that order and dirt is both the first block and the first solid one. This          cannot falsify the rule and is not meant to — the registry above is what does that.          What it pins is the shipped answer through a change that gave three of this block's          four old meanings fields of their own, and it is the block the launched client puts in          hand and draws"
    );
    Ok(())
}

#[test]
fn a_registry_of_blocks_that_stop_nobody_offers_none_though_one_is_drawn() -> TestResult {
    let none_of_them_stops_anybody = registry_of_declarations(&[
        (WATER, DRAWN_AND_AIMED_AT_ONLY),
        (STONE, Declaration::like_solidity(false)),
    ])?;
    let one_of_them_stops_a_player = registry_of_declarations(&[
        (WATER, DRAWN_AND_AIMED_AT_ONLY),
        (STONE, Declaration::like_solidity(true)),
    ])?;

    assert_eq!(
        (
            default_held_block(&none_of_them_stops_anybody)
                .as_ref()
                .map(BlockName::as_str),
            default_held_block(&one_of_them_stops_a_player)
                .as_ref()
                .map(BlockName::as_str)
        ),
        (None, Some(STONE)),
        "the first block of both registries is drawn and may be aimed at and stops nobody, which          is a declaration nothing could state before those became separate fields. A rule that          had drifted onto drawnness or onto targetability answers that block in both halves. The          second registry differs from the first in one word of one declaration and has to answer          the block that word made solid, which is what stops `no held block` being satisfied by a          rule that answers nothing for anything"
    );
    Ok(())
}
