//! The same contents mesh to the same quads, whatever route the contents took to
//! get there.
//!
//! Meshing the same section twice and comparing the answers is deliberately not
//! one of these. No correct single-threaded mesher can fail it, which makes it
//! the shape that goes green while seeing nothing. The falsifiable form is this
//! one: identical contents reached by different write histories, before and after
//! the entries nothing holds are reclaimed, and under two registries that number
//! the same blocks differently. Each of those moves something a mesher might
//! accidentally read — palette order, palette length, index width, refcounts, a
//! runtime id — while leaving the contents alone.
//!
//! Every comparison here is between two whole quad sequences, blocks included,
//! and every one of them is guarded against being a comparison of two empty
//! lists. Two empty sequences are equal, so without the guard a mesher that
//! emitted nothing would be the most deterministic one in the repository.
//!
//! The fixtures are built by writing rather than by importing, because the
//! difference between the histories is the fixture.

mod mesh_common;

use std::error::Error;

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::mesh::{Neighbours, mesh_section};
use mc_world::section::Section;
use mesh_common::{
    ALPHA, BETA, SOLID, TestResult, VOID, at, named, registry_declaring, some_quads,
};

/// Three further blocks, written into one voxel in turn and each displaced by the
/// next, so that the palette of the section that took the long way round is
/// longer, differently ordered and packed at a wider index than the one that took
/// the short way.
const GAMMA: &str = "example:gamma";
const DELTA: &str = "example:delta";
const EPSILON: &str = "example:epsilon";

/// A block written at one voxel and immediately written over, leaving its palette
/// entry behind with nothing holding it.
const REPLACED: &str = "example:replaced";

/// The contents both of the first pair of sections arrive at: one solid block at
/// (0, 0, 0), a different one at (1, 0, 0), and nothing solid anywhere else.
///
/// Reached in two writes, so the palette ends up three entries long, in the order
/// the two blocks were written.
fn written_the_short_way(registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    let mut section = Section::filled(&BlockName::parse(VOID)?, registry)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(ALPHA)?, registry)?;
    section.set_block(at(1, 0, 0), &BlockName::parse(BETA)?, registry)?;
    Ok(section)
}

/// The same contents, reached the other way about: the second block first, and
/// the first block's voxel written three times over before it settles.
///
/// The palette ends up six entries long — three of them held by nothing — in the
/// opposite order, and wide enough for six entries rather than for three.
fn written_the_long_way_round(registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    let mut section = Section::filled(&BlockName::parse(VOID)?, registry)?;
    section.set_block(at(1, 0, 0), &BlockName::parse(BETA)?, registry)?;
    for name in [GAMMA, DELTA, EPSILON, ALPHA] {
        section.set_block(at(0, 0, 0), &BlockName::parse(name)?, registry)?;
    }
    Ok(section)
}

/// A section holding a solid block at two voxels, one of which had something else
/// in it first — so the palette carries an entry nothing holds.
fn written_over_once(registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    let mut section = Section::filled(&BlockName::parse(VOID)?, registry)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(REPLACED)?, registry)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(SOLID)?, registry)?;
    section.set_block(at(1, 0, 0), &BlockName::parse(SOLID)?, registry)?;
    Ok(section)
}

/// Which solid block this section's palette names first.
fn first_solid_entry(section: &Section) -> Option<String> {
    section
        .palette()
        .map(named)
        .find(|entry| *entry != VOID)
        .map(str::to_owned)
}

/// Refuses a pair of sections whose storage a mesher could read without the
/// comparison below noticing.
///
/// All three have to differ, because a mesher reading any one of them would
/// still agree with itself if the other two moved.
fn require_differing_storage(first: &Section, second: &Section) -> Result<(), Box<dyn Error>> {
    let lengths = (first.palette().len(), second.palette().len());
    let widths = (first.index_width_bits(), second.index_width_bits());
    let orders = (first_solid_entry(first), first_solid_entry(second));
    if lengths.0 != lengths.1 && widths.0 != widths.1 && orders.0 != orders.1 {
        return Ok(());
    }
    Err(format!(
        "these two sections must differ in palette length, index width and palette order, or \
         a mesher reading whichever of the three they share would still produce equal answers \
         here: lengths {lengths:?}, widths {widths:?}, first solid entries {orders:?}"
    )
    .into())
}

/// Refuses a section that had nothing to reclaim, since compacting it would then
/// have changed nothing for the comparison to be about.
fn require_an_entry_was_reclaimed(carried: usize, kept: usize) -> Result<(), Box<dyn Error>> {
    if carried > kept {
        return Ok(());
    }
    Err(format!(
        "compaction has to have reclaimed an entry here, or the two meshes below are taken \
         from a section that never changed: {carried} entries before, {kept} after"
    )
    .into())
}

/// Refuses a pair of registries that did not number both blocks differently.
fn require_differing_runtime_ids(
    first: &BlockRegistry,
    second: &BlockRegistry,
    names: &[&str],
) -> Result<(), Box<dyn Error>> {
    let mut shared = Vec::new();
    for name in names {
        let parsed = BlockName::parse(name)?;
        shared.push((
            *name,
            first.id_of(&parsed)?.get(),
            second.id_of(&parsed)?.get(),
        ));
    }
    if shared.iter().all(|(_, here, there)| here != there) {
        return Ok(());
    }
    Err(format!(
        "both blocks must be numbered differently by the two registries, or a mesher carrying \
         a runtime id into its output would still agree with itself: {shared:?}"
    )
    .into())
}

#[test]
fn identical_contents_reached_by_different_write_orders_mesh_identically() -> TestResult {
    let registry = registry_declaring(&[
        (VOID, false),
        (ALPHA, true),
        (BETA, true),
        (GAMMA, true),
        (DELTA, true),
        (EPSILON, true),
    ])?;
    let short_history = written_the_short_way(&registry)?;
    let long_history = written_the_long_way_round(&registry)?;
    require_differing_storage(&short_history, &long_history)?;

    let from_the_short = mesh_section(&short_history, &Neighbours::none(), &registry)?;
    let from_the_long = mesh_section(&long_history, &Neighbours::none(), &registry)?;

    assert_eq!(
        some_quads(&from_the_short)?,
        from_the_long.quads(),
        "these two sections hold the same block at the same voxels and differ only in how they \
         got there. Everything a section remembers about that — where each block sits in its \
         palette, how many entries the palette has, how wide the packed indices are — is \
         behind the read, so none of it may reach the output. A merge predicate comparing \
         palette positions, or an emission order keyed on them, produces two different \
         sequences here"
    );
    Ok(())
}

#[test]
fn a_section_meshes_the_same_before_and_after_its_unheld_entries_are_reclaimed() -> TestResult {
    let registry = registry_declaring(&[(VOID, false), (REPLACED, true), (SOLID, true)])?;
    let mut section = written_over_once(&registry)?;

    let before = mesh_section(&section, &Neighbours::none(), &registry)?;
    let carried = section.palette().len();
    section.compact();
    require_an_entry_was_reclaimed(carried, section.palette().len())?;
    let after = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        some_quads(&before)?,
        after.quads(),
        "compaction drops the entry nothing holds and renumbers the rest, which changes the \
         palette this section carries and can change the width its indices are packed at, \
         without changing a single voxel. The mesh either side of it has to be the same one — \
         and it is what forces an unheld entry never to be resolved at all, since an entry \
         that failed the mesh before compaction would succeed after it"
    );
    Ok(())
}

#[test]
fn a_section_meshes_the_same_under_a_registry_that_numbers_its_blocks_the_other_way() -> TestResult
{
    let one_way = registry_declaring(&[(ALPHA, true), (BETA, true)])?;
    let the_other_way = registry_declaring(&[(BETA, true), (ALPHA, true)])?;
    require_differing_runtime_ids(&one_way, &the_other_way, &[ALPHA, BETA])?;
    let mut section = Section::filled(&BlockName::parse(ALPHA)?, &one_way)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(BETA)?, &one_way)?;

    let numbered_one_way = mesh_section(&section, &Neighbours::none(), &one_way)?;
    let numbered_the_other = mesh_section(&section, &Neighbours::none(), &the_other_way)?;

    assert_eq!(
        some_quads(&numbered_one_way)?,
        numbered_the_other.quads(),
        "a runtime id means something only to the registry that assigned it, so nothing \
         ordered or labelled by one may reach a mesh — a mesh still in flight when the block \
         set is swapped underneath it would otherwise resolve to different blocks. Both blocks \
         are numbered differently by these two registries, so an id reaching the output shows \
         up as a reordered or relabelled sequence"
    );
    Ok(())
}
