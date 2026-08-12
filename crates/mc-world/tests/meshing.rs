//! Which faces a section on its own shows, and in which order they arrive.
//!
//! Nothing here supplies a neighbour, so every face on a section boundary is
//! decided against content that is not loaded — which the specification settles
//! as non-solid, so the boundary is visible rather than sealed.
//!
//! Two things are separated deliberately throughout. Whether a face exists is a
//! question about the *registered definition* of the block holding the voxel and
//! about nothing else: not its name, and not the runtime id a registry happened
//! to hand it. So the registries below are built in the order that puts a block
//! at the runtime id a scenario is about, that placement is checked before the
//! assertion rather than assumed, and one of them registers a block named after
//! air as solid. An engine that recognised either would pass a suite built from
//! ordinary content and fail the day a mod shipped a block of its own.
//!
//! Whether a face is *labelled* correctly is the other question, and the plane is
//! where it shows: a plane is the coordinate of the solid voxel that emitted the
//! face, never of the face itself. A lone voxel at (8, 8, 8) puts all six of its
//! faces on plane 8, which no convention could tell apart from another — so the
//! fixture that separates them is a section with one voxel missing rather than
//! one voxel present, and it lives with the neighbour-aware scenarios.

mod mesh_common;

use std::error::Error;

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::mesh::{Facing, Neighbours, mesh_section};
use mc_world::section::{SECTION_SIZE, Section};
use mesh_common::{
    Face, SOLID, TestResult, VOID, at, face, faces, plain_registry, registry_declaring,
    require_runtime_id, scattered_solids, section_holding, single_face,
};

/// A block registered non-solid whose name is the one the base game ships for
/// the stuff a player swims through. Named out loud because the scenario is that
/// the name buys it nothing at all.
const WATER: &str = "base:water";

/// A block registered *solid* whose name is the one the base game ships for
/// empty space, and which the registry below numbers second.
const AIR: &str = "base:air";

/// The two blocks the deliberately-numbered registries are built from. Which of
/// them is solid changes per scenario; which of them is numbered first does not.
const NUMBERED_FIRST: &str = "example:numbered_first";
const NUMBERED_SECOND: &str = "example:numbered_second";

/// A block written into a section and then written over, so that its palette
/// entry survives with nothing holding it.
const REPLACED: &str = "example:replaced";

/// The runtime ids a registry hands the first and second blocks it registers.
const FIRST_RUNTIME_ID: u32 = 0;
const SECOND_RUNTIME_ID: u32 = 1;

/// The six sides one solid voxel shows when nothing solid touches it, every one
/// of them at `plane` and starting at `origin`.
///
/// Every side is one voxel across and every one carries the coordinate of the
/// voxel that emitted it, so a voxel whose three coordinates are equal puts all
/// six of its sides on the same plane.
fn the_six_sides_of_one_voxel(plane: u32, origin: (u32, u32)) -> Vec<Face> {
    Facing::ALL
        .into_iter()
        .map(|facing| single_face(facing, plane, origin))
        .collect()
}

/// The six sides of a section every one of whose voxels is solid, with nothing
/// loaded beside it.
///
/// Each side is one unbroken rectangle covering a whole face of the section, and
/// each sits on the plane of the voxels that emitted it — the low row of the axis
/// for a negative facing, the high row for a positive one.
fn the_six_sides_of_a_full_section() -> Vec<Face> {
    let whole = (SECTION_SIZE, SECTION_SIZE);
    let far = SECTION_SIZE - 1;
    vec![
        face(Facing::NegX, 0, (0, 0), whole),
        face(Facing::PosX, far, (0, 0), whole),
        face(Facing::NegY, 0, (0, 0), whole),
        face(Facing::PosY, far, (0, 0), whole),
        face(Facing::NegZ, 0, (0, 0), whole),
        face(Facing::PosZ, far, (0, 0), whole),
    ]
}

/// The twelve faces two solid voxels at (2, 0, 0) and (0, 0, 3) show, in the
/// order the total order puts them in: facing, then plane, then secondary, then
/// primary.
///
/// The two voxels differ in plane on the x and z axes and share one on y, so the
/// ±X and ±Z pairs are separated by their planes while the ±Y pair sits on the
/// same plane and is separated by its secondary coordinate. A sequence ordered by
/// anything else — by the order the voxels were reached, by primary before
/// secondary — reorders at least one of those three pairs.
fn the_twelve_faces_of_two_scattered_voxels() -> Vec<Face> {
    vec![
        single_face(Facing::NegX, 0, (0, 3)),
        single_face(Facing::NegX, 2, (0, 0)),
        single_face(Facing::PosX, 0, (0, 3)),
        single_face(Facing::PosX, 2, (0, 0)),
        single_face(Facing::NegY, 0, (2, 0)),
        single_face(Facing::NegY, 0, (0, 3)),
        single_face(Facing::PosY, 0, (2, 0)),
        single_face(Facing::PosY, 0, (0, 3)),
        single_face(Facing::NegZ, 0, (2, 0)),
        single_face(Facing::NegZ, 3, (0, 0)),
        single_face(Facing::PosZ, 0, (2, 0)),
        single_face(Facing::PosZ, 3, (0, 0)),
    ]
}

/// A section holding three palette entries, one of them held by nothing: the
/// fill, a block written at one voxel, and the block that displaced it there.
///
/// Written rather than imported, because an entry nothing holds is the whole
/// point and only a write history produces one.
fn section_carrying_a_vacated_entry(registry: &BlockRegistry) -> Result<Section, Box<dyn Error>> {
    let mut section = Section::filled(&BlockName::parse(VOID)?, registry)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(REPLACED)?, registry)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(SOLID)?, registry)?;
    Ok(section)
}

/// Refuses a section that does not carry three palette entries with one of them
/// held by nothing.
///
/// Compaction is what makes an unheld entry observable at all, so a copy is
/// compacted and the two lengths are compared. Without the unheld entry the
/// comparison below would stay green against a mesher that reclaimed one.
fn require_three_entries_one_of_them_vacated(section: &Section) -> Result<(), Box<dyn Error>> {
    let mut compacted = section.clone();
    compacted.compact();
    let lengths = (section.palette().len(), compacted.palette().len());
    if lengths == (3, 2) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs a section carrying three palette entries one of which nothing \
         holds; this one carries {} and keeps {} of them through compaction, so the \
         comparison below would not notice a mesher that reclaimed an entry",
        lengths.0, lengths.1
    )
    .into())
}

#[test]
fn a_lone_solid_voxel_shows_one_face_on_each_of_its_six_sides() -> TestResult {
    let registry = plain_registry()?;
    let section = scattered_solids(|voxel| voxel == at(8, 8, 8), &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        the_six_sides_of_one_voxel(8, (8, 8)),
        "a solid voxel with nothing solid on any side of it shows all six of its sides, each \
         one voxel across, and each labelled with the plane of the voxel that emitted it \
         rather than of the face — so all six sit on plane 8. A mesher emitting one quad per \
         solid voxel rather than per visible side emits a sixth of this"
    );
    Ok(())
}

#[test]
fn a_section_holding_no_solid_voxel_meshes_to_nothing_without_refusing() -> TestResult {
    let registry = plain_registry()?;
    let section = scattered_solids(|_| false, &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        mesh.quads().len(),
        0,
        "a section of nothing but non-solid blocks has no visible face anywhere in it. It is \
         also the commonest section in a world — everything above the surface is one — so \
         answering it with a refusal rather than with an empty mesh would make the ordinary \
         case an error every caller has to handle"
    );
    Ok(())
}

#[test]
fn a_voxel_whose_block_was_registered_non_solid_shows_no_face() -> TestResult {
    let registry = registry_declaring(&[(VOID, false), (WATER, false)])?;
    let section = section_holding(
        &[VOID, WATER],
        |voxel| u16::from(voxel == at(8, 8, 8)),
        &registry,
    )?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        mesh.quads().len(),
        0,
        "only a solid block emits a face, and solidity is what the definition declared and \
         nothing else. This voxel holds a block declared non-solid, so it shows no side at \
         all — while a mesher that emitted a face wherever a voxel differs from its \
         surroundings would show six"
    );
    Ok(())
}

#[test]
fn a_solid_block_at_the_first_runtime_id_fills_a_section_showing_all_six_sides() -> TestResult {
    let registry = registry_declaring(&[(NUMBERED_FIRST, true), (NUMBERED_SECOND, false)])?;
    require_runtime_id(&registry, NUMBERED_FIRST, FIRST_RUNTIME_ID)?;
    let section = Section::filled(&BlockName::parse(NUMBERED_FIRST)?, &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        the_six_sides_of_a_full_section(),
        "this section is filled with the block the registry numbered first, and that block was \
         declared solid — so the section is a solid cube showing one whole 16x16 side per \
         facing. A mesher treating the first runtime id as empty space, which is the shape \
         every engine that hardcodes air takes, produces nothing at all here"
    );
    Ok(())
}

#[test]
fn a_non_solid_block_at_the_first_runtime_id_fills_a_section_showing_nothing() -> TestResult {
    let registry = registry_declaring(&[(NUMBERED_FIRST, false), (NUMBERED_SECOND, true)])?;
    require_runtime_id(&registry, NUMBERED_FIRST, FIRST_RUNTIME_ID)?;
    let section = Section::filled(&BlockName::parse(NUMBERED_FIRST)?, &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        mesh.quads().len(),
        0,
        "the same runtime id as the scenario above and the opposite answer, because the only \
         thing that changed between them is what the definition declared. A mesher holding any \
         opinion at all about the first runtime id gives both of them the same answer"
    );
    Ok(())
}

#[test]
fn a_solid_block_named_after_empty_space_fills_a_section_showing_all_six_sides() -> TestResult {
    let registry = registry_declaring(&[(NUMBERED_FIRST, false), (AIR, true)])?;
    require_runtime_id(&registry, AIR, SECOND_RUNTIME_ID)?;
    let section = Section::filled(&BlockName::parse(AIR)?, &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        the_six_sides_of_a_full_section(),
        "this block carries the name the base game ships for the emptiest thing there is, and \
         it was registered solid at a runtime id that is not the first. Both halves matter: a \
         mesher recognising the name and a mesher recognising a low id each answer with an \
         empty mesh, and a mod shipping a block of its own would inherit whichever mistake was \
         made"
    );
    Ok(())
}

#[test]
fn quads_arrive_ordered_by_facing_then_plane_then_secondary_then_primary() -> TestResult {
    let registry = plain_registry()?;
    let section = scattered_solids(
        |voxel| voxel == at(2, 0, 0) || voxel == at(0, 0, 3),
        &registry,
    )?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        the_twelve_faces_of_two_scattered_voxels(),
        "the emission order is total, and this fixture is what separates each of its four \
         keys: the facings group first, the ±X and ±Z faces are then split by plane, and the \
         two ±Y faces share a plane and are split by their secondary coordinate rather than by \
         their primary one. Walking voxels and then facings, or ordering primary before \
         secondary, reorders exactly one of those pairs and leaves the rest looking right"
    );
    Ok(())
}

#[test]
fn meshing_leaves_the_palette_entries_their_order_their_counts_and_the_index_width() -> TestResult {
    let registry = registry_declaring(&[(VOID, false), (REPLACED, true), (SOLID, true)])?;
    let section = section_carrying_a_vacated_entry(&registry)?;
    require_three_entries_one_of_them_vacated(&section)?;
    let untouched = section.clone();

    mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        section, untouched,
        "meshing is a pure read. Comparing whole sections is what reaches the reference \
         counts, which have no accessor at all — so this one comparison covers the palette's \
         entries, their order, how many voxels hold each of them, and the width the indices \
         are packed at. Compacting the input to simplify the sweep is the tempting shortcut, \
         and it moves three of those four"
    );
    Ok(())
}
