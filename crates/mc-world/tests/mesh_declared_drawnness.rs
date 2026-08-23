//! Which voxels show a face, decided by what their block declares about being
//! drawn rather than by what it declares about stopping a player.
//!
//! **Every meshing fixture written before this file declares one boolean and
//! reads it back three times.** A mesher that answered drawnness from solidity
//! passes all of them by construction and no count can see it, so the fixtures
//! here are the ones in which the answers disagree: a block a player walks
//! through and sees, and a block that stops a player and shows nothing.
//!
//! Two ordering facts are load-bearing and neither is incidental.
//!
//! The drawn block is registered **first**, so it holds runtime id 0. A mesher
//! that read the lowest runtime id as empty space — the shape every engine that
//! hardcodes air takes — would emit nothing for it, and would thereby get the
//! undrawn block's answer right by a route that has nothing to do with what was
//! declared. `require_runtime_id` pins both placements before any comparison
//! reads them.
//!
//! And every scenario asserting that a voxel shows *nothing* is written as a
//! complete list of what the mesh does hold, never as a count of zero. A section
//! full of undrawn rock has an honest empty answer and so does a mesher that
//! emits nothing at all, and those two are indistinguishable from a length. So
//! each of these fixtures carries one drawn voxel whose faces the same comparison
//! also has to account for.

mod mesh_common;

use std::error::Error;

use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::mesh::{Facing, Neighbours, mesh_section};
use mc_world::section::{LocalPos, Section};
use mesh_common::{
    DRAWN_ONLY, GHOST, HAZE, MIST, OCCLUDING_ONLY, SHROUD, SOLID_AND_OCCLUDING, SOLID_ONLY,
    TestResult, at, every_side_of, every_side_of_but, faces, faces_towards,
    registry_of_declarations, require_runtime_id, section_holding, section_of_nothing_but,
    single_face,
};

/// The runtime ids the registry below hands the drawn block and the undrawn one.
const DRAWN_RUNTIME_ID: u32 = 0;
const UNDRAWN_RUNTIME_ID: u32 = 1;

/// The voxel holding the drawn block in the fixtures built from a cell or two.
///
/// Its three coordinates are pairwise distinct, so its six sides sit on three
/// different planes. A voxel whose coordinates are equal puts all six on one
/// plane, which no convention could tell apart from another.
const DRAWN_VOXEL: LocalPos = at(1, 2, 3);

/// The voxel one step towards +X of it.
const BESIDE_IT: LocalPos = at(2, 2, 3);

/// The facing that step crosses, which is therefore the one side of
/// [`DRAWN_VOXEL`] with something other than empty space beyond it.
const TOWARDS_IT: Facing = Facing::PosX;

/// Where the undrawn block sits in the fixture that holds both blocks far apart,
/// and where the drawn one does.
const UNDRAWN_ON_ITS_OWN: LocalPos = at(1, 2, 3);
const DRAWN_ELSEWHERE: LocalPos = at(5, 6, 7);

/// Where the one drawn voxel sits in a section otherwise full of undrawn rock.
///
/// On the −X boundary, so exactly one of its six sides looks out of the section
/// and the other five look at rock. The neighbour beyond that boundary is the one
/// left unsupplied, which is what leaves that side showing.
const DRAWN_IN_THE_ROCK: LocalPos = at(0, 8, 8);

/// The one boundary of that fixture no section is supplied beyond.
const LEFT_UNSUPPLIED: Facing = Facing::NegX;

/// The five facings a section *is* supplied beyond, and what each of those
/// sections is filled with.
///
/// Four different declarations and one section holding nothing, so "however its
/// neighbours are declared" is five different answers rather than one repeated —
/// including an occluding neighbour, a non-occluding one, and no content at all.
const SUPPLIED_BEYOND: [Facing; 5] = [
    Facing::PosX,
    Facing::NegY,
    Facing::PosY,
    Facing::NegZ,
    Facing::PosZ,
];
const FILLED_WITH: [&str; 4] = [GHOST, HAZE, SHROUD, MIST];

/// Where the palette of the rock-and-one-drawn-voxel section puts each block.
const HOLDS_ROCK: u16 = 0;
const HOLDS_DRAWN: u16 = 1;

/// A registry in which no two of the three answers can be derived from each
/// other, with both placements a comparison depends on pinned.
///
/// # Errors
///
/// Returns an error if the registry refuses the batch, or if it numbered either
/// block somewhere other than where the scenarios below need it.
fn registry_where_the_answers_disagree() -> Result<BlockRegistry, Box<dyn Error>> {
    let registry = registry_of_declarations(&[
        (HAZE, DRAWN_ONLY),
        (GHOST, SOLID_AND_OCCLUDING),
        (MIST, SOLID_ONLY),
        (SHROUD, OCCLUDING_ONLY),
    ])?;
    require_runtime_id(&registry, HAZE, DRAWN_RUNTIME_ID)?;
    require_runtime_id(&registry, GHOST, UNDRAWN_RUNTIME_ID)?;
    Ok(registry)
}

/// One section per facing in [`SUPPLIED_BEYOND`], each filled with what
/// [`FILLED_WITH`] names for it and the last of them holding nothing at all.
///
/// Returned owned because a `Neighbours` borrows them and has to be built where
/// they live.
fn variously_declared_neighbours(registry: &BlockRegistry) -> Result<Vec<Section>, Box<dyn Error>> {
    let mut around = Vec::with_capacity(SUPPLIED_BEYOND.len());
    for name in FILLED_WITH {
        around.push(Section::filled(&BlockName::parse(name)?, registry)?);
    }
    around.push(Section::empty());
    Ok(around)
}

/// Those sections, each supplied beyond the facing it was built for.
///
/// Pairs by position with [`SUPPLIED_BEYOND`], which does not name
/// [`LEFT_UNSUPPLIED`] — so that boundary stays absent however many sections are
/// handed in.
fn supplied_beyond(around: &[Section]) -> Neighbours<'_> {
    SUPPLIED_BEYOND
        .into_iter()
        .zip(around)
        .fold(Neighbours::none(), |so_far, (facing, section)| {
            so_far.with(facing, section)
        })
}

#[test]
fn a_voxel_declared_drawn_and_not_solid_shows_all_six_of_its_sides() -> TestResult {
    let registry = registry_where_the_answers_disagree()?;
    let section = section_of_nothing_but(&[(DRAWN_VOXEL, HAZE)], &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        every_side_of(DRAWN_VOXEL),
        "this block stops nobody and is drawn, and every cell around it — the one above it \
         included — holds nothing at all. So all six of its sides are visible, and the upward \
         one is there because the declaration says the block is drawn rather than because it \
         says the block is solid. A mesher reading solidity emits nothing here at all"
    );
    Ok(())
}

#[test]
fn a_voxel_declared_solid_and_not_drawn_shows_none_of_its_sides() -> TestResult {
    let registry = registry_where_the_answers_disagree()?;
    let section = section_of_nothing_but(
        &[(UNDRAWN_ON_ITS_OWN, GHOST), (DRAWN_ELSEWHERE, HAZE)],
        &registry,
    )?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        every_side_of(DRAWN_ELSEWHERE),
        "the solid voxel is declared undrawn, so it shows nothing on any of its six sides, and \
         the whole mesh is the other voxel's six. Comparing a length against zero would say the \
         same thing about a mesher that emits nothing whatever it is handed — the drawn voxel is \
         here so that this comparison has to account for six quads before it can be believed \
         about the absent ones"
    );
    Ok(())
}

#[test]
fn solid_undrawn_rock_shows_nothing_however_its_neighbours_are_declared() -> TestResult {
    let registry = registry_where_the_answers_disagree()?;
    let section = section_holding(
        &[GHOST, HAZE],
        |voxel| {
            if voxel == DRAWN_IN_THE_ROCK {
                HOLDS_DRAWN
            } else {
                HOLDS_ROCK
            }
        },
        &registry,
    )?;
    let around = variously_declared_neighbours(&registry)?;

    let mesh = mesh_section(&section, &supplied_beyond(&around), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        vec![single_face(
            LEFT_UNSUPPLIED,
            DRAWN_IN_THE_ROCK.x,
            (DRAWN_IN_THE_ROCK.y, DRAWN_IN_THE_ROCK.z)
        )],
        "4095 voxels of this section hold a block declared solid and occluding and undrawn, and \
         they meet an occluding neighbour, a non-occluding one, a drawn one, a solid one and a \
         section holding nothing — five different declarations beyond five different boundaries. \
         Not one of them produces a face, because none of them changes what the rock itself \
         declares about being drawn. The single drawn voxel on the unsupplied boundary is what \
         makes that emptiness a measurement rather than a mesher that emits nothing"
    );
    Ok(())
}

#[test]
fn a_drawn_voxel_beside_solid_undrawn_rock_shows_every_side_facing_empty_space() -> TestResult {
    let registry = registry_where_the_answers_disagree()?;
    let section = section_of_nothing_but(&[(DRAWN_VOXEL, HAZE), (BESIDE_IT, GHOST)], &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        every_side_of_but(DRAWN_VOXEL, TOWARDS_IT),
        "the two blocks beside each other disagree about all three answers, and each side of the \
         drawn one is decided against what is actually beyond it: five sides look at empty space \
         and are shown, and the sixth looks at something declared occluding and is not. The \
         undrawn block contributes nothing of its own to this list, so every quad in it belongs \
         to the block a player can see"
    );
    Ok(())
}

#[test]
fn removing_the_solid_undrawn_neighbour_is_what_makes_the_face_toward_it_appear() -> TestResult {
    let registry = registry_where_the_answers_disagree()?;
    let beside_it = section_of_nothing_but(&[(DRAWN_VOXEL, HAZE), (BESIDE_IT, GHOST)], &registry)?;
    let on_its_own = section_of_nothing_but(&[(DRAWN_VOXEL, HAZE)], &registry)?;

    let with_it = mesh_section(&beside_it, &Neighbours::none(), &registry)?;
    let without_it = mesh_section(&on_its_own, &Neighbours::none(), &registry)?;

    assert_eq!(
        (
            faces_towards(with_it.quads(), TOWARDS_IT),
            faces_towards(without_it.quads(), TOWARDS_IT)
        ),
        (
            Vec::new(),
            vec![single_face(
                TOWARDS_IT,
                DRAWN_VOXEL.x,
                (DRAWN_VOXEL.y, DRAWN_VOXEL.z)
            )]
        ),
        "the two fixtures differ in one cell, so the neighbour is the only thing that can \
         account for the difference in what the drawn voxel shows towards it. Comparing the \
         first list against emptiness on its own would be satisfied by a mesher that emits \
         nothing, and the second list against one face by a mesher that emits everything — \
         neither of them can produce both halves of this pair"
    );
    Ok(())
}
