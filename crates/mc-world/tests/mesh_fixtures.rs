//! What the benchmark's terrain fixture is actually made of.
//!
//! A benchmark fixture is the one piece of test material nothing else checks: it
//! is not asserted by the scenarios that use it, it is the thing they are
//! measured against. A fixture that quietly built less terrain than it declared
//! would make every timing taken on it flattering and every quad count derived
//! from it small, and both would look like good news.
//!
//! So the fixture is asserted against its own declaration rather than against a
//! second copy of itself. The heights are the declaration; the section is what
//! was built from them; and the two are compared through the public per-voxel
//! read a caller outside this crate would use. Building the fixture twice and
//! comparing the results would assert only that the builder is a function.

mod common;

#[path = "../benches/support/mod.rs"]
mod support;

use std::error::Error;

use common::{TestResult, all_positions};
use support::fixtures::{self, Fixture};

/// The lowest and highest column height the terrain fixture may declare.
///
/// Both ends matter and neither is decoration. Below the low end the fixture
/// stops being terrain and starts being mostly air, which benchmarks a workload
/// the client never streams; above the high end a column reaches the top of the
/// section and its top face stops existing, which silently removes visible faces
/// from the very count the budget is asserted against.
const LOWEST_DECLARED_HEIGHT: u32 = 4;
const HIGHEST_DECLARED_HEIGHT: u32 = 12;

/// How many of a fixture's voxels its own registry reports as solid.
///
/// Counted through `is_solid_at`, so what is counted is the solidity the block
/// was registered with and nothing else — a count that recognised a name would
/// be a different assertion wearing this one's clothes.
fn solid_voxel_count(fixture: &Fixture) -> Result<u32, Box<dyn Error>> {
    let mut solid = 0;
    for position in all_positions() {
        if fixture.section.is_solid_at(position, &fixture.registry)? {
            solid += 1;
        }
    }
    Ok(solid)
}

#[test]
fn the_terrain_fixture_is_solid_up_to_each_column_s_declared_height_and_no_further() -> TestResult {
    let declared_heights = fixtures::terrain_heights();
    let fixture = fixtures::terrain()?;

    let heights_outside_the_allowed_range: Vec<u32> = declared_heights
        .into_iter()
        .filter(|height| !(LOWEST_DECLARED_HEIGHT..=HIGHEST_DECLARED_HEIGHT).contains(height))
        .collect();
    let counted_solids = solid_voxel_count(&fixture)?;

    assert_eq!(
        (counted_solids, heights_outside_the_allowed_range),
        (declared_heights.iter().sum::<u32>(), Vec::new()),
        "each of the 256 columns is solid from y = 0 up to the height declared for it and \
         non-solid above, so the solid voxels are the declared heights summed — and every \
         one of those heights lies in 4..=12, which is what keeps the fixture roughly half \
         full and keeps every column's top face inside the section. A fixture that filled \
         one voxel too few per column, filled above the height, or drew a height outside \
         the range would still look like terrain and would still benchmark quickly"
    );
    Ok(())
}
