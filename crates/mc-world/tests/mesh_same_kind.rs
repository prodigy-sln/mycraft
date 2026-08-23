//! Whether a drawn, non-occluding block draws a face against another cell of its
//! own kind.
//!
//! It does not, and that is an engine rule rather than something content states:
//! two cells of one such block show no seam where they meet, and two cells of
//! *different* such blocks each show theirs. Without the rule a body of water is
//! a stack of visible sheets; with it applied to identity rather than to
//! appearance, water meeting glass still shows both faces.
//!
//! **Every scenario here states solidity as well**, and states it `false`. That
//! is what stops the rule being satisfied by accident: a mesher that ignored
//! occlusion altogether and culled on solidity would show these seams, and a
//! mesher that culled on solidity while calling every non-solid cell empty would
//! hide them for a reason that has nothing to do with the two cells being alike.
//!
//! The rule is asked three times because it is answered by different code each
//! time — along a plane's primary axis, along its secondary axis, and across a
//! section boundary, where the cells being compared live in two different
//! sections and the comparison needs an identity that survives the crossing. The
//! third of those is the one a boundary plane carrying a single boolean cannot
//! answer at all, and the shipped sea spans sections, so it is the case a world
//! actually reaches.
//!
//! The fourth scenario is the control, and it is not optional: a rule that culled
//! between *any* two drawn non-occluding cells would satisfy the first three and
//! flatten every boundary between two different transparent blocks in the world.

mod mesh_common;

use std::error::Error;

use mc_core::block::BlockRegistry;
use mc_world::mesh::{Facing, Neighbours, mesh_section};
use mc_world::section::{LocalPos, SECTION_SIZE, Section};
use mesh_common::{
    DRAWN_ONLY, Face, HAZE, MURK, TestResult, at, face, faces, named_faces,
    registry_of_declarations, require_runtime_id, section_of_nothing_but, single_face,
};

/// The runtime ids the two drawn blocks hold. Pinned because the scenarios turn
/// on the two cells holding the *same* block or *different* ones, and a registry
/// that numbered them otherwise would leave the comparisons about other blocks
/// than the ones they name.
const FIRST_DRAWN_RUNTIME_ID: u32 = 0;
const SECOND_DRAWN_RUNTIME_ID: u32 = 1;

/// The two cells side by side along x, in the corner of the section.
///
/// The corner is deliberate: three of each cell's sides look out of the section
/// and are therefore decided against absence, which keeps the expectations below
/// short enough to read while still covering both kinds of decision.
const FIRST_CELL: LocalPos = at(0, 0, 0);
const NEXT_ALONG_X: LocalPos = at(1, 0, 0);

/// The cell directly above the first one, for the same question asked along a
/// plane's secondary axis instead of its primary one.
const NEXT_ALONG_Y: LocalPos = at(0, 1, 0);

/// The cell on the far +X face of the section, and the cell facing it from inside
/// the section beyond that boundary.
///
/// Leaving at one end arrives at the other, so the neighbour's own x is 0.
const ON_THE_SHARED_FACE: LocalPos = at(SECTION_SIZE - 1, 0, 0);
const FACING_IT_NEXT_DOOR: LocalPos = at(0, 0, 0);

/// Which boundary the two sections meet across.
const ACROSS: Facing = Facing::PosX;

/// A registry holding two blocks declared identically — drawn, and neither solid
/// nor occluding — differing in name and in nothing else.
///
/// # Errors
///
/// Returns an error if the registry refuses the batch or numbered either block
/// somewhere other than where these scenarios need it.
fn registry_of_two_alike_drawn_blocks() -> Result<BlockRegistry, Box<dyn Error>> {
    let registry = registry_of_declarations(&[(HAZE, DRAWN_ONLY), (MURK, DRAWN_ONLY)])?;
    require_runtime_id(&registry, HAZE, FIRST_DRAWN_RUNTIME_ID)?;
    require_runtime_id(&registry, MURK, SECOND_DRAWN_RUNTIME_ID)?;
    Ok(registry)
}

/// A section holding one block at each of `cells` and nothing anywhere else.
fn holding_at(
    cells: &[LocalPos],
    name: &str,
    registry: &BlockRegistry,
) -> Result<Section, Box<dyn Error>> {
    let held: Vec<(LocalPos, &str)> = cells.iter().map(|&cell| (cell, name)).collect();
    section_of_nothing_but(&held, registry)
}

/// What two cells side by side along x show, with nothing loaded around them.
///
/// The four sides whose facings lie across the pair merge into one 2x1 rectangle
/// each, because both cells hold the same block and nothing breaks the run. The
/// two sides along x do not: the −X one is emitted only by the cell at x = 0 and
/// the +X one only by the cell at x = 1, and **neither of the two faces on the
/// boundary the cells share is here at all** — which is the scenario.
fn what_a_pair_along_x_shows() -> Vec<Face> {
    let pair = (2, 1);
    vec![
        single_face(Facing::NegX, FIRST_CELL.x, (0, 0)),
        single_face(Facing::PosX, NEXT_ALONG_X.x, (0, 0)),
        face(Facing::NegY, 0, (0, 0), pair),
        face(Facing::PosY, 0, (0, 0), pair),
        face(Facing::NegZ, 0, (0, 0), pair),
        face(Facing::PosZ, 0, (0, 0), pair),
    ]
}

/// What two cells stacked along y show, with nothing loaded around them.
///
/// The transpose of the fixture above and deliberately not its mirror image. On a
/// ±X face y is the *primary* axis, so the pair merges into a 2x1 there; on a ±Z
/// face y is the *secondary* one, so the same pair merges into a 1x2 instead. A
/// mesher that swapped the two roles produces one of those where the other
/// belongs, and this pair of scenarios is what separates them.
fn what_a_pair_along_y_shows() -> Vec<Face> {
    vec![
        face(Facing::NegX, 0, (0, 0), (2, 1)),
        face(Facing::PosX, 0, (0, 0), (2, 1)),
        single_face(Facing::NegY, FIRST_CELL.y, (0, 0)),
        single_face(Facing::PosY, NEXT_ALONG_Y.y, (0, 0)),
        face(Facing::NegZ, 0, (0, 0), (1, 2)),
        face(Facing::PosZ, 0, (0, 0), (1, 2)),
    ]
}

/// What the cell on the shared face shows when the same block faces it from the
/// section beyond.
///
/// Its −X side looks at an empty cell inside its own section and is shown; its
/// −Y and −Z sides look out of the section at nothing loaded and are shown; its
/// +Y and +Z sides look at empty cells and are shown. Its +X side looks across
/// the boundary at its own kind, and is the one side missing.
fn what_a_cell_on_the_shared_face_shows() -> Vec<Face> {
    let along_the_far_face = (ON_THE_SHARED_FACE.x, 0);
    vec![
        single_face(Facing::NegX, ON_THE_SHARED_FACE.x, (0, 0)),
        single_face(Facing::NegY, 0, along_the_far_face),
        single_face(Facing::PosY, 0, along_the_far_face),
        single_face(Facing::NegZ, 0, along_the_far_face),
        single_face(Facing::PosZ, 0, along_the_far_face),
    ]
}

/// What two cells side by side along x show when they hold two *different* drawn
/// blocks, each quad beside the block it names.
///
/// Twelve quads rather than six: nothing merges, because a quad names one block
/// and these cells hold two — and the two faces on the boundary they share are
/// both here, one owned by each of them.
fn what_an_unlike_pair_along_x_shows() -> Vec<(Face, String)> {
    let mut shown = Vec::new();
    for facing in Facing::ALL {
        for (cell, name) in [(FIRST_CELL, HAZE), (NEXT_ALONG_X, MURK)] {
            shown.push((the_side_of(cell, facing), name.to_owned()));
        }
    }
    shown
}

/// The one side of `cell` that points `facing`.
///
/// Derived from the plane's own axis order rather than listed per facing: the
/// plane is the cell's coordinate along the facing's axis, and the origin is its
/// two remaining coordinates, lower-numbered axis first.
fn the_side_of(cell: LocalPos, facing: Facing) -> Face {
    match facing {
        Facing::NegX | Facing::PosX => single_face(facing, cell.x, (cell.y, cell.z)),
        Facing::NegY | Facing::PosY => single_face(facing, cell.y, (cell.x, cell.z)),
        Facing::NegZ | Facing::PosZ => single_face(facing, cell.z, (cell.x, cell.y)),
    }
}

#[test]
fn two_cells_side_by_side_holding_one_drawn_block_show_no_face_between_them() -> TestResult {
    let registry = registry_of_two_alike_drawn_blocks()?;
    let section = holding_at(&[FIRST_CELL, NEXT_ALONG_X], HAZE, &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        what_a_pair_along_x_shows(),
        "both cells hold one drawn block that hides nothing, so the boundary between them \
         carries no face in either direction while every other side of the pair is shown. \
         Neither of the two blocks is solid, so a mesher culling on solidity would show that \
         seam — and the six quads this does expect are what stop a mesher that emits nothing \
         from satisfying the same comparison"
    );
    Ok(())
}

#[test]
fn two_cells_stacked_holding_one_drawn_block_show_no_face_between_them() -> TestResult {
    let registry = registry_of_two_alike_drawn_blocks()?;
    let section = holding_at(&[FIRST_CELL, NEXT_ALONG_Y], HAZE, &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        faces(mesh.quads()),
        what_a_pair_along_y_shows(),
        "the same question asked of a horizontal boundary rather than a vertical one, which is \
         the boundary a body of water actually has most of. The extents are what make this more \
         than a repetition: the pair merges 2x1 across the ±X faces where y is primary and 1x2 \
         across the ±Z faces where y is secondary, so a mesher that swapped the two axis roles \
         fails here while passing the scenario above"
    );
    Ok(())
}

#[test]
fn two_cells_in_neighbouring_sections_holding_one_drawn_block_show_no_face_across_it() -> TestResult
{
    let registry = registry_of_two_alike_drawn_blocks()?;
    let section = holding_at(&[ON_THE_SHARED_FACE], HAZE, &registry)?;
    let beyond = holding_at(&[FACING_IT_NEXT_DOOR], HAZE, &registry)?;

    let mesh = mesh_section(
        &section,
        &Neighbours::none().with(ACROSS, &beyond),
        &registry,
    )?;

    assert_eq!(
        faces(mesh.quads()),
        what_a_cell_on_the_shared_face_shows(),
        "the two cells are alike and they are in different sections, so the rule has to hold \
         across the boundary as well — a sea that stops at every chunk edge and shows a sheet \
         there is what happens when it does not. This is the scenario a boundary carrying one \
         bit per cell cannot answer either way: 'is what is beyond me solid' has no room in it \
         for 'is what is beyond me the same block I am'"
    );
    Ok(())
}

#[test]
fn two_cells_holding_two_different_drawn_blocks_each_show_their_face_between_them() -> TestResult {
    let registry = registry_of_two_alike_drawn_blocks()?;
    let section = section_of_nothing_but(&[(FIRST_CELL, HAZE), (NEXT_ALONG_X, MURK)], &registry)?;

    let mesh = mesh_section(&section, &Neighbours::none(), &registry)?;

    assert_eq!(
        named_faces(mesh.quads()),
        what_an_unlike_pair_along_x_shows(),
        "these two blocks are declared identically and differ only in name, and the rule is \
         about a cell's own kind rather than about how a cell looks — so the boundary between \
         them carries two faces, and the name on each says which of them owns it. A rule culling \
         between any two drawn non-occluding cells satisfies the three scenarios above and loses \
         both of these, and no comparison that threw the names away could tell that apart from \
         losing one of them twice"
    );
    Ok(())
}
