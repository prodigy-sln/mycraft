//! Whether a section answers "is this solid" and "is this drawn" as one question
//! or as two.
//!
//! It is the same shape as `block_semantics.rs`, one property over, and it is
//! separated from it because the two files fail for different reasons: that one
//! catches an engine deciding solidity from a name or a runtime id, and this one
//! catches an engine that has only one answer to give.
//!
//! **A section that answered both questions from solidity would keep every mesh
//! fixture in this crate green and every oracle agreeing with its subject**, since
//! each of them decides drawnness today by asking whether a cell is solid. So the
//! two answers are read at two cells declared the opposite way round from each
//! other, and one comparison holds all four: a section with one answer to give
//! cannot produce them.
//!
//! It links `mesh_common` rather than `common` for one reason — that module is the
//! only place in this crate that can build a registry in which solidity and
//! drawnness disagree, which is exactly what this scenario needs. Nothing here
//! names the mesher.

mod mesh_common;

use mc_core::block::BlockRegistry;
use mc_world::section::{LocalPos, Section};
use mesh_common::{
    DRAWN_ONLY, GHOST, HAZE, SOLID_AND_OCCLUDING, TestResult, at, registry_of_declarations,
    section_of_nothing_but,
};

use std::error::Error;

/// The cell holding a block declared solid and not drawn.
const SOLID_BUT_UNDRAWN: LocalPos = at(1, 2, 3);

/// The cell holding a block declared drawn and not solid — the same two answers
/// the other way round.
const DRAWN_BUT_UNSOLID: LocalPos = at(4, 5, 6);

/// Two cells whose declarations disagree with each other about both questions,
/// against a registry that numbers the undrawn one second.
///
/// Registered second deliberately: an engine answering drawnness from a runtime
/// id rather than from the declaration would have to give the two cells the same
/// answer, and the comparison below wants them opposite.
fn two_cells_declared_opposite_ways() -> Result<(Section, BlockRegistry), Box<dyn Error>> {
    let registry = registry_of_declarations(&[(HAZE, DRAWN_ONLY), (GHOST, SOLID_AND_OCCLUDING)])?;
    let section = section_of_nothing_but(
        &[(SOLID_BUT_UNDRAWN, GHOST), (DRAWN_BUT_UNSOLID, HAZE)],
        &registry,
    )?;
    Ok((section, registry))
}

#[test]
fn a_cell_holding_a_solid_undrawn_block_is_reported_solid_and_not_drawn() -> TestResult {
    let (section, registry) = two_cells_declared_opposite_ways()?;

    let answers = (
        section.is_solid_at(SOLID_BUT_UNDRAWN, &registry)?,
        section.is_drawn_at(SOLID_BUT_UNDRAWN, &registry)?,
        section.is_solid_at(DRAWN_BUT_UNSOLID, &registry)?,
        section.is_drawn_at(DRAWN_BUT_UNSOLID, &registry)?,
    );

    assert_eq!(
        answers,
        (true, false, false, true),
        "the first cell holds a block that stops a player and shows nothing, and the second one \
         holds a block a player walks through and sees. Those are two questions with two \
         answers, and every wrong shape collapses them into one: a section whose drawnness \
         reads its solidity answers (true, true, false, false), one whose solidity reads its \
         drawnness answers (false, false, true, true), and one with a single answer per cell \
         cannot tell the two cells apart at all"
    );
    Ok(())
}
