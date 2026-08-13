//! What happens when the registry cannot say whether a block is solid.
//!
//! There is no honest mesh for a voxel whose block nothing resolves. Reading it
//! as non-solid punches a hole through the world; reading it as solid seals a
//! cavity. Both are silent and both are indistinguishable from a correct mesh at
//! the call site, so the whole mesh is refused instead, and the refusal has to
//! carry enough to act on: the block, and where it is.
//!
//! Which voxel is named is the part worth being careful about. The obvious
//! implementation resolves the palette and reports the first entry that fails,
//! and that is a different voxel from the lowest one holding an unresolvable
//! block whenever the palette's order and the section's own linear order
//! disagree. The second scenario below is built so that they disagree.
//!
//! The opposite mistake is refusing too much. A palette entry no voxel holds any
//! more is not part of the section's contents; failing on it would make a mesh
//! depend on the section's write history, so the same contents would mesh
//! differently before and after compaction. A neighbour is the same argument
//! carried one section outwards: only the 256 voxels of the face it shares are
//! ever read, so a block it holds anywhere else is never resolved and never
//! refuses anything. Those two scenarios are a pair — without the narrowing they
//! contradict each other.
//!
//! And a refusal changes nothing. Whatever the mesher had already read by the
//! time it gave up — the section itself, five whole shared faces and part of a
//! sixth — is handed back exactly as it arrived, because the caller on the other
//! side of a failed mesh still owns that content and is about to try again.

mod mesh_common;

use std::error::Error;

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::mesh::{Facing, MeshError, Neighbours, SectionMesh, mesh_section};
use mc_world::section::{Contents, LocalPos, Section};
use mesh_common::{
    SOLID, TestResult, VOID, all_around, at, named, registry_declaring, section_holding,
    sections_around, solid_section, some_quads,
};

/// A block held by two voxels of the meshed section and registered by only one
/// of the two registries below.
const ORPHAN: &str = "example:orphan";

/// Two blocks the registry the mesh is taken against does not register, written
/// in the order that puts the *earlier palette entry* at the *higher* voxel.
const WRITTEN_FIRST: &str = "example:written_first";
const WRITTEN_SECOND: &str = "example:written_second";

/// A block written into one voxel and then written over, so that its palette
/// entry survives with nothing holding it.
const NO_LONGER_HELD: &str = "example:no_longer_held";

/// A block held by a supplied neighbour rather than by the section being meshed.
const ORPHAN_NEXT_DOOR: &str = "example:orphan_next_door";

/// Which neighbour holds it.
///
/// Not the first facing in the emission order, so a refusal that named a fixed
/// facing rather than the one it was reading is visible rather than accidentally
/// right.
const NEIGHBOUR_HOLDING_IT: Facing = Facing::NegZ;

/// Where it sits inside that neighbour, on the face the two sections share.
///
/// A section to the −Z side meets the meshed section's z = 0 with its own
/// z = 15, so this is the shared face — and naming it in the neighbour's own
/// frame is the whole point. The meshed section's frame would call the same
/// voxel (2, 6, 0), and all three coordinates differ from each other, so a
/// mirrored coordinate and a swapped pair are both visible too.
const ON_THE_SHARED_FACE: LocalPos = at(2, 6, 15);

/// Where it sits in the neighbour that holds it away from that face: one step
/// further in, and read by nothing.
const AWAY_FROM_THE_SHARED_FACE: LocalPos = at(2, 6, 14);

/// Which of the six neighbours holds the block that refuses the mesh when all
/// six are supplied, and where inside it.
///
/// The last facing in the emission order, so that everything a mesh reads has
/// been read by the time it gives up: the section itself, the five shared faces
/// before this one, and part of this one. A refusal on the meshed section's own
/// contents would leave most of the comparison below asserting about sections
/// nothing had touched.
const LAST_NEIGHBOUR_READ: Facing = Facing::PosZ;
const ON_ITS_SHARED_FACE: LocalPos = at(4, 9, 0);

/// The refusal a mesh produced, or an explanation of why asserting on it would
/// have been vacuous.
fn refusal(outcome: Result<SectionMesh, MeshError>) -> Result<MeshError, Box<dyn Error>> {
    match outcome {
        Ok(mesh) => Err(format!(
            "a section holding a block the registry cannot resolve has no honest mesh, so this \
             call must be refused rather than answered; it produced {} quads",
            mesh.quads().len()
        )
        .into()),
        Err(refused) => Ok(refused),
    }
}

/// The block and the position a refusal about an unresolvable block names.
fn unresolved_block(
    outcome: Result<SectionMesh, MeshError>,
) -> Result<(String, LocalPos), Box<dyn Error>> {
    let refused = refusal(outcome)?;
    let MeshError::UnresolvedBlock { name, position } = refused else {
        return Err(
            format!("expected a refusal naming a block and its voxel, got {refused:?}").into(),
        );
    };
    Ok((name.as_str().to_owned(), position))
}

/// The block, the facing and the position a refusal about a neighbour's block
/// names.
fn unresolved_neighbour_block(
    outcome: Result<SectionMesh, MeshError>,
) -> Result<(String, Facing, LocalPos), Box<dyn Error>> {
    let refused = refusal(outcome)?;
    let MeshError::UnresolvedNeighbourBlock {
        name,
        facing,
        position,
    } = refused
    else {
        return Err(format!(
            "expected a refusal naming a block, the neighbour holding it and the voxel of that \
             neighbour it sits at, got {refused:?}"
        )
        .into());
    };
    Ok((name.as_str().to_owned(), facing, position))
}

/// A section to stand beyond `facing`: solid, unless it is the one holding a
/// block nothing registers on the face it shares.
fn a_neighbour_beyond(facing: Facing, registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    if facing != LAST_NEIGHBOUR_READ {
        return solid_section(registry);
    }
    section_holding(
        &[SOLID, ORPHAN_NEXT_DOOR],
        |voxel| u16::from(voxel == ON_ITS_SHARED_FACE),
        registry,
    )
}

/// Refuses a section whose palette does not hold exactly `expected`, in that
/// order.
///
/// The order is the fixture: without it, naming the first failing palette entry
/// and naming the lowest failing voxel are the same answer, and the assertion
/// cannot tell them apart.
fn require_palette_order(section: &Section, expected: &[&str]) -> Result<(), Box<dyn Error>> {
    let held: Vec<&str> = section.palette().map(named).collect();
    if held == expected {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the block written first to be the earlier palette entry, so that \
         the palette's order and the section's linear order disagree; the palette is {held:?} \
         rather than {expected:?}"
    )
    .into())
}

/// Refuses a section that does not carry `name` as an entry nothing holds, or a
/// registry that does register it after all.
fn require_unheld_and_unregistered(
    section: &Section,
    name: &str,
    registry: &BlockRegistry,
) -> Result<(), Box<dyn Error>> {
    let parsed = BlockName::parse(name)?;
    let mut compacted = section.clone();
    compacted.compact();
    let still_held = compacted
        .palette()
        .any(|kept| kept == Contents::Holds(&parsed));
    let carried = section
        .palette()
        .any(|entry| entry == Contents::Holds(&parsed));
    if carried && !still_held && registry.id_of(&parsed).is_err() {
        return Ok(());
    }
    Err(format!(
        "`{name}` must be a palette entry no voxel holds and a block this registry does not \
         register, or the comparison below is about neither: carried {carried}, still held \
         after compaction {still_held}"
    )
    .into())
}

#[test]
fn a_block_the_registry_cannot_resolve_refuses_the_mesh_at_its_lowest_voxel() -> TestResult {
    let complete = registry_declaring(&[(VOID, false), (ORPHAN, true)])?;
    let missing_it = registry_declaring(&[(VOID, false)])?;
    let section = section_holding(
        &[VOID, ORPHAN],
        |voxel| u16::from(voxel == at(5, 2, 1) || voxel == at(7, 3, 1)),
        &complete,
    )?;

    let refused = unresolved_block(mesh_section(&section, &Neighbours::none(), &missing_it))?;

    assert_eq!(
        refused,
        (ORPHAN.to_owned(), at(5, 2, 1)),
        "the refusal has to carry both the block and where it is, because a caller told only \
         that something did not resolve cannot do anything about it. The position is the \
         lowest voxel holding that block in the section's own order — x fastest, then y, then \
         z — so of the two voxels holding it, (5, 2, 1) is named and (7, 3, 1) is not"
    );
    Ok(())
}

#[test]
fn two_unresolvable_blocks_are_reported_at_the_lower_voxel_not_the_earlier_entry() -> TestResult {
    let complete =
        registry_declaring(&[(VOID, false), (WRITTEN_FIRST, true), (WRITTEN_SECOND, true)])?;
    let missing_both = registry_declaring(&[(VOID, false)])?;
    let mut section = Section::filled(&BlockName::parse(VOID)?, &complete)?;
    section.set_block(at(5, 0, 0), &BlockName::parse(WRITTEN_FIRST)?, &complete)?;
    section.set_block(at(3, 0, 0), &BlockName::parse(WRITTEN_SECOND)?, &complete)?;
    require_palette_order(&section, &[VOID, WRITTEN_FIRST, WRITTEN_SECOND])?;

    let refused = unresolved_block(mesh_section(&section, &Neighbours::none(), &missing_both))?;

    assert_eq!(
        refused,
        (WRITTEN_SECOND.to_owned(), at(3, 0, 0)),
        "both of these blocks are unresolvable, and the palette names them in the opposite \
         order to the one the voxels are reached in: the block at (5, 0, 0) was written first \
         and is therefore the earlier entry. A resolver walking the palette reports that one; \
         a resolver walking the voxels reports the block at (3, 0, 0), which is the voxel a \
         caller looking for the problem would find first"
    );
    Ok(())
}

#[test]
fn a_palette_entry_no_voxel_holds_is_never_resolved_and_never_refuses_the_mesh() -> TestResult {
    let complete = registry_declaring(&[(VOID, false), (NO_LONGER_HELD, true), (SOLID, true)])?;
    let missing_it = registry_declaring(&[(VOID, false), (SOLID, true)])?;
    let mut section = Section::filled(&BlockName::parse(VOID)?, &complete)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(NO_LONGER_HELD)?, &complete)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(SOLID)?, &complete)?;
    require_unheld_and_unregistered(&section, NO_LONGER_HELD, &missing_it)?;

    let against_all = mesh_section(&section, &Neighbours::none(), &complete)?;
    let against_fewer = mesh_section(&section, &Neighbours::none(), &missing_it)?;

    assert_eq!(
        some_quads(&against_all)?,
        against_fewer.quads(),
        "no voxel holds this entry any more, so it is not part of what the section contains \
         and nothing needs to know whether it is solid. Resolving the palette up front rather \
         than the voxels refuses this mesh — and would make the same contents mesh differently \
         before and after compaction, which is a section's write history reaching its \
         appearance"
    );
    Ok(())
}

#[test]
fn a_block_a_neighbour_holds_against_the_shared_face_refuses_the_mesh_by_its_own_position()
-> TestResult {
    let complete = registry_declaring(&[(VOID, false), (SOLID, true), (ORPHAN_NEXT_DOOR, true)])?;
    let missing_it = registry_declaring(&[(VOID, false), (SOLID, true)])?;
    let section = solid_section(&complete)?;
    let neighbour = section_holding(
        &[VOID, ORPHAN_NEXT_DOOR],
        |voxel| u16::from(voxel == ON_THE_SHARED_FACE),
        &complete,
    )?;

    let refused = unresolved_neighbour_block(mesh_section(
        &section,
        &Neighbours::none().with(NEIGHBOUR_HOLDING_IT, &neighbour),
        &missing_it,
    ))?;

    assert_eq!(
        refused,
        (
            ORPHAN_NEXT_DOOR.to_owned(),
            NEIGHBOUR_HOLDING_IT,
            ON_THE_SHARED_FACE
        ),
        "this voxel decides whether a face of the meshed section is drawn, and nothing can say \
         whether it is solid, so there is no more honest mesh here than there is for an \
         unresolvable block in the section itself. What the refusal carries is what somebody \
         would need to go and look: the block, which of the six neighbours holds it, and where \
         it is inside that neighbour — its own frame, which is the one whoever opens that \
         section will be reading, rather than the meshed section's or the mirrored coordinate \
         the lookup went through"
    );
    Ok(())
}

#[test]
fn a_block_a_neighbour_holds_away_from_the_shared_face_never_refuses_the_mesh() -> TestResult {
    let complete = registry_declaring(&[(VOID, false), (SOLID, true), (ORPHAN_NEXT_DOOR, true)])?;
    let missing_it = registry_declaring(&[(VOID, false), (SOLID, true)])?;
    let section = solid_section(&complete)?;
    let neighbour = section_holding(
        &[VOID, ORPHAN_NEXT_DOOR],
        |voxel| u16::from(voxel == AWAY_FROM_THE_SHARED_FACE),
        &complete,
    )?;

    let mesh = mesh_section(
        &section,
        &Neighbours::none().with(NEIGHBOUR_HOLDING_IT, &neighbour),
        &missing_it,
    )?;

    assert!(
        !mesh.quads().is_empty(),
        "only the 256 voxels of the face a neighbour shares are ever read, and this block is \
         one step behind that face. Nothing about the meshed section depends on it, so \
         resolving it at all is work that buys nothing and refusing on it makes a section \
         unmeshable because of content in a chunk beside it that it never looks at. A mesher \
         resolving a neighbour's whole 4096 voxels refuses this, and the scenario above stops \
         it from simply never resolving a neighbour instead"
    );
    Ok(())
}

#[test]
fn a_refused_mesh_leaves_the_section_and_every_neighbour_it_was_given_untouched() -> TestResult {
    let complete = registry_declaring(&[(SOLID, true), (ORPHAN_NEXT_DOOR, true)])?;
    let missing_it = registry_declaring(&[(SOLID, true)])?;
    let section = solid_section(&complete)?;
    let around = sections_around(|facing| a_neighbour_beyond(facing, &complete))?;
    let untouched_section = section.clone();
    let untouched_around = around.clone();

    refusal(mesh_section(&section, &all_around(&around), &missing_it))?;

    assert_eq!(
        (&section, &around),
        (&untouched_section, &untouched_around),
        "a refusal is not permission to have left something behind. Comparing whole sections is \
         what reaches the reference counts, which have no accessor at all, so these two \
         comparisons cover the palette entries of all seven sections, their order, how many \
         voxels hold each of them and the width the indices are packed at. Compacting an input \
         to simplify a sweep is the tempting shortcut and it moves three of those four — and on \
         the failing path the caller still holds every one of these sections and is about to \
         mesh them again"
    );
    Ok(())
}
