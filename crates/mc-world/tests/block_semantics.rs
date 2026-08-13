//! Where a voxel's solidity comes from, and where it may never come from.
//!
//! Solidity is the first block property this engine reads for itself — the mesher
//! will hide faces with it and physics will stand on it — and it is therefore the
//! first opportunity to write `if name == "base:air"` into the engine. That one
//! line would make the base game privileged: a mod's own transparent block would
//! occlude, and its own walkable block would not hold anyone up. Solidity is a
//! registered property, read back through the registry, and nothing else.
//!
//! Which is why the registries below declare the opposite of what this repository
//! ships. `base:air` is solid here and `base:stone` is not. Every one of these
//! tests is passed by an engine that reads the property and failed by an engine
//! that recognises the name — and the third one is failed by an engine that
//! recognises runtime id 0 instead, which is the same shortcut wearing a number.
//!
//! **`base:air` is now a name neither the engine nor the base game knows at
//! all**, and that strengthens the thesis rather than dating it. A cell holds a
//! block or it holds nothing, nothing is not a block, and no content set here
//! ships one under that name — so the name buys precisely nothing, and a
//! registry is free to hand it whatever definition it likes. This one hands it
//! solidity, which is the least air-like thing a block can be.
//!
//! **A reader meeting this file beside the rule that no production source names
//! a retired block will read the two as opposed. They are not.** That scan reads
//! production sources under `src/` and never `tests/`, so a fixture is free to
//! say a name out loud — which is the only way a fixture about solidity can say
//! which name carries which definition at all.

mod common;

use std::error::Error;

use common::{TestResult, all_positions, at, described, registry_declaring};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::section::Section;

const AIR: &str = "base:air";
const STONE: &str = "base:stone";

/// A section of stone whose lowest corner holds air, against a registry that
/// declares stone first and non-solid, air second and solid.
///
/// Stone is registered first deliberately: it holds runtime id 0 here and it is
/// *not* solid, while air holds id 1 and is. So an engine that answered from a
/// runtime id — the shortcut that reads `id == 0` as air — gets both voxels the
/// wrong way round, and one section is enough to catch it.
fn air_over_stone() -> Result<(Section, BlockRegistry), Box<dyn Error>> {
    let registry = registry_declaring(&[(STONE, false), (AIR, true)])?;
    let mut section = Section::filled(&BlockName::parse(STONE)?, &registry)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(AIR)?, &registry)?;
    Ok((section, registry))
}

#[test]
fn a_voxel_holding_a_block_declared_solid_is_reported_solid() -> TestResult {
    let (section, registry) = air_over_stone()?;

    let solid = section.is_solid_at(at(0, 0, 0), &registry)?;

    assert!(
        solid,
        "this registry declares air solid. It is a strange thing for air to be, and that \
         is exactly the point: the engine has no opinion about what air is, only about \
         what the block registered under that name says it is"
    );
    Ok(())
}

#[test]
fn a_voxel_holding_a_block_declared_non_solid_is_reported_non_solid() -> TestResult {
    let (section, registry) = air_over_stone()?;

    let solid = section.is_solid_at(at(1, 0, 0), &registry)?;

    assert!(
        !solid,
        "this registry declares stone non-solid, and the voxel beside the air one holds \
         stone. An engine that reported solidity from the name, or answered the same for \
         every voxel in the section, cannot have both this and its neighbour right"
    );
    Ok(())
}

#[test]
fn a_section_filled_with_the_first_registered_block_holds_only_it_and_reports_it_solid()
-> TestResult {
    let registry = registry_declaring(&[(STONE, true), (AIR, false)])?;
    let section = Section::filled(&BlockName::parse(STONE)?, &registry)?;

    let mut solid = 0_usize;
    for position in all_positions() {
        if section.is_solid_at(position, &registry)? {
            solid += 1;
        }
    }

    let palette: Vec<String> = section.palette().map(described).collect();
    assert_eq!(
        (palette, solid),
        (vec![STONE.to_owned()], 4096),
        "stone is registered first here, so it holds runtime id 0 — the number an engine \
         that special-cased anything would have given to air. The section holds stone and \
         nothing else, and every one of its 4096 voxels is solid because that is what the \
         block registered under that name declares"
    );
    Ok(())
}
