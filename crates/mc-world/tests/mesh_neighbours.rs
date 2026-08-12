//! What a section on a boundary shows, decided against the section beyond that
//! boundary rather than against nothing.
//!
//! A face on the outside of a section is the one face a section cannot answer
//! for on its own. Deciding it against the section beyond is what makes two
//! loaded chunks meet without a wall of hidden geometry between them; deciding
//! it against nothing at all is what makes the world look continuous while it
//! streams. Both answers are needed, and which one applies is per neighbour and
//! never all-or-nothing: a section is routinely meshed with the chunk below it
//! loaded and the other five still on their way.
//!
//! Three mistakes live in this area and every one of them produces a mesh that
//! looks entirely plausible.
//!
//! The first is reading the wrong neighbour — two slots wired to each other, or
//! one facing's section consulted for another's. The last scenario here is the
//! one written for it: all six neighbours are supplied, each solid but for a
//! single hole of its own, and the six holes sit at coordinates that are
//! pairwise distinct and none of which reads the same forwards as backwards. A
//! swapped slot, a swap of the two plane axes and a read from the wrong end of
//! an axis therefore each land somewhere different rather than on another row's
//! answer, so they fail one at a time instead of cancelling.
//!
//! The second is labelling a face by the direction of the empty voxel rather
//! than of the solid one, or by the coordinate of the face rather than of the
//! voxel that emitted it. Every fixture built from a lone solid voxel is
//! symmetric under both, so the fixture here is the opposite one: solid rock
//! with a single voxel missing from the middle of it. Each of the six faces
//! around that hole is emitted by the voxel beside it and therefore sits on
//! plane 9 or on plane 7 — never on the hole's own plane 8.
//!
//! The third is treating absence as a single flag. Three neighbours supplied and
//! three absent has one answer, and "all six absent" and "all six loaded" are
//! both different from it.

mod mesh_common;

use mc_world::mesh::{Facing, Neighbours, SectionMesh, mesh_section};
use mc_world::section::{LocalPos, SECTION_SIZE};
use mesh_common::{
    Face, TestResult, all_around, at, face, faces, faces_towards, plain_registry, scattered_solids,
    sections_around, single_face, solid_but_for, solid_section, some_quads, walled_in_by,
};

/// The last coordinate any axis of a section has, and therefore the plane the
/// outermost voxels of a positive facing sit on.
const LAST: u32 = SECTION_SIZE - 1;

/// A whole side of a section: sixteen voxels along each of its plane's two axes.
const WHOLE_SIDE: (u32, u32) = (SECTION_SIZE, SECTION_SIZE);

/// How many sides of a solid section are still shown when exactly one of its six
/// neighbours is loaded and solid.
const SIDES_STILL_SHOWING: usize = 5;

/// Where the hole in the single supplied neighbour sits, in that neighbour's own
/// frame.
///
/// On x = 0, which is the face a section to the +X side shares with the one
/// being meshed. The other two coordinates differ from each other and from that
/// one, so a mesher that swapped the plane's two axes reads a different voxel.
const HOLE_BEYOND_THE_POSITIVE_X_FACE: LocalPos = at(0, 3, 5);

/// Where the voxel missing from the middle of otherwise solid rock is.
///
/// All three coordinates equal, deliberately: the six faces around it are
/// emitted by six different voxels, so a mesh that labelled a face by the hole
/// would put all six on one plane while the right answer puts none of them
/// there.
const HOLE_IN_THE_ROCK: LocalPos = at(8, 8, 8);

/// How many quads a mesh holds, and whether any of them points `facing`.
///
/// Two scenarios say "this many quads, none of them facing that way" and mean
/// exactly that, so both halves are compared at once rather than one of them
/// being inferred from the other.
fn quads_and_any_towards(mesh: &SectionMesh, facing: Facing) -> (usize, bool) {
    let quads = mesh.quads();
    (quads.len(), quads.iter().any(|quad| quad.facing == facing))
}

/// The six sides of the cavity a single missing voxel leaves in solid rock, in
/// the order they are emitted.
///
/// Each is emitted by the solid voxel beside the hole and carries that voxel's
/// own coordinate, so the face pointing back towards the hole from x = 9 sits on
/// plane 9 and the one from x = 7 sits on plane 7. Nothing sits on plane 8.
fn the_six_sides_around_the_hole() -> Vec<Face> {
    let before = HOLE_IN_THE_ROCK.x - 1;
    let after = HOLE_IN_THE_ROCK.x + 1;
    let origin = (HOLE_IN_THE_ROCK.y, HOLE_IN_THE_ROCK.z);
    vec![
        single_face(Facing::NegX, after, origin),
        single_face(Facing::PosX, before, origin),
        single_face(Facing::NegY, after, origin),
        single_face(Facing::PosY, before, origin),
        single_face(Facing::NegZ, after, origin),
        single_face(Facing::PosZ, before, origin),
    ]
}

/// The one non-solid voxel each of the six neighbours holds, named in that
/// neighbour's own frame.
///
/// Every one of them sits on the face that neighbour shares with the section
/// being meshed — x = 15 for the section to the −X side, x = 0 for the one to
/// +X, and so on down the six. The two remaining coordinates are distinct across
/// all six pairs and no pair reads the same forwards as backwards, so a
/// neighbour consulted at the wrong slot, at the wrong end of its own axis, or
/// with its plane's two axes swapped lands on a voxel no other row expects.
const fn the_hole_beyond(facing: Facing) -> LocalPos {
    match facing {
        Facing::NegX => at(15, 1, 2),
        Facing::PosX => at(0, 3, 4),
        Facing::NegY => at(5, 15, 6),
        Facing::PosY => at(7, 0, 8),
        Facing::NegZ => at(9, 10, 15),
        Facing::PosZ => at(11, 12, 0),
    }
}

/// The one face each of those holes leaves visible on the section being meshed,
/// in the order the six are emitted in.
///
/// Each sits on the plane of the outermost voxel of its own facing — 0 for a
/// negative facing and 15 for a positive one — and starts at the two coordinates
/// its neighbour's hole shares with it, taken in the plane's own order: primary
/// y and secondary z for ±X, primary x and secondary z for ±Y, primary x and
/// secondary y for ±Z.
fn the_face_opposite_each_hole() -> Vec<Face> {
    vec![
        single_face(Facing::NegX, 0, (1, 2)),
        single_face(Facing::PosX, LAST, (3, 4)),
        single_face(Facing::NegY, 0, (5, 6)),
        single_face(Facing::PosY, LAST, (7, 8)),
        single_face(Facing::NegZ, 0, (9, 10)),
        single_face(Facing::PosZ, LAST, (11, 12)),
    ]
}

#[test]
fn solid_rock_on_every_side_of_a_solid_section_leaves_it_showing_nothing() -> TestResult {
    let registry = plain_registry()?;
    let section = solid_section(&registry)?;
    let beyond = solid_section(&registry)?;

    let mesh = mesh_section(&section, &walled_in_by(&beyond), &registry)?;

    assert_eq!(
        mesh.quads().len(),
        0,
        "this section is solid and so is everything around it, so not one of its faces has \
         anything but rock on the far side of it and there is nothing to draw. A mesher that \
         decided its boundary faces against the section alone shows all six outer sides of it \
         — six walls buried inside solid ground, drawn once for every section a world holds"
    );
    Ok(())
}

#[test]
fn a_single_missing_voxel_in_solid_rock_shows_the_six_sides_around_the_hole() -> TestResult {
    let registry = plain_registry()?;
    let section = solid_but_for(HOLE_IN_THE_ROCK, &registry)?;
    let beyond = solid_section(&registry)?;

    let mesh = mesh_section(&section, &walled_in_by(&beyond), &registry)?;

    assert_eq!(
        faces(some_quads(&mesh)?),
        the_six_sides_around_the_hole(),
        "the only faces in this section are the six looking inwards at the hole, and each of \
         them is carried by the solid voxel that emitted it rather than by the empty one it \
         looks at. So the six sit on planes 9 and 7 and none of them on 8 — which is what \
         separates a face labelled by the solid voxel from one labelled by the air, and both \
         of those from a face labelled by its own coordinate rather than by a voxel's"
    );
    Ok(())
}

#[test]
fn a_solid_neighbour_beyond_one_side_hides_that_side_and_no_other() -> TestResult {
    let registry = plain_registry()?;
    let section = solid_section(&registry)?;
    let beyond = solid_section(&registry)?;

    let neighbours = Neighbours::none().with(Facing::PosX, &beyond);
    let mesh = mesh_section(&section, &neighbours, &registry)?;

    assert_eq!(
        quads_and_any_towards(&mesh, Facing::PosX),
        (SIDES_STILL_SHOWING, false),
        "one neighbour is loaded and it is solid, so the face the two sections share is buried \
         and the other five are still at the edge of what has been loaded. A mesher that read \
         'some neighbour is supplied' as 'the section is surrounded' answers with nothing at \
         all here"
    );
    Ok(())
}

#[test]
fn a_solid_neighbour_below_hides_the_underside_and_no_other_side() -> TestResult {
    let registry = plain_registry()?;
    let section = solid_section(&registry)?;
    let beyond = solid_section(&registry)?;

    let neighbours = Neighbours::none().with(Facing::NegY, &beyond);
    let mesh = mesh_section(&section, &neighbours, &registry)?;

    assert_eq!(
        quads_and_any_towards(&mesh, Facing::NegY),
        (SIDES_STILL_SHOWING, false),
        "the same shape as the scenario above and a different slot, because the section below \
         is the one neighbour a streaming world almost always has. A mesher wired to consult \
         one fixed facing whatever it was handed passes that scenario and fails this one"
    );
    Ok(())
}

#[test]
fn a_neighbour_holding_nothing_solid_leaves_the_whole_side_it_meets_visible() -> TestResult {
    let registry = plain_registry()?;
    let section = solid_section(&registry)?;
    let beyond = scattered_solids(|_| false, &registry)?;

    let neighbours = Neighbours::none().with(Facing::PosX, &beyond);
    let mesh = mesh_section(&section, &neighbours, &registry)?;

    assert_eq!(
        faces_towards(some_quads(&mesh)?, Facing::PosX),
        vec![face(Facing::PosX, LAST, (0, 0), WHOLE_SIDE)],
        "a loaded neighbour holding nothing solid hides nothing, so the whole shared face is \
         still shown — and shown as one rectangle, because every voxel of it holds the same \
         block and nothing breaks the run. This is the open-air case: a cliff face with sky \
         beyond it is a loaded neighbour, not an absent one"
    );
    Ok(())
}

#[test]
fn a_single_hole_in_a_solid_neighbour_shows_a_single_face_opposite_it() -> TestResult {
    let registry = plain_registry()?;
    let section = solid_section(&registry)?;
    let beyond = solid_but_for(HOLE_BEYOND_THE_POSITIVE_X_FACE, &registry)?;

    let neighbours = Neighbours::none().with(Facing::PosX, &beyond);
    let mesh = mesh_section(&section, &neighbours, &registry)?;

    assert_eq!(
        faces_towards(some_quads(&mesh)?, Facing::PosX),
        vec![single_face(Facing::PosX, LAST, (3, 5))],
        "the shared face is decided one voxel at a time and not one section at a time: this \
         neighbour is solid everywhere but one place, so exactly one voxel of the meshed \
         section has something other than rock beyond it. The face is at the plane of that \
         voxel and at the two coordinates it shares with the hole — primary y then secondary \
         z. A mesher deciding the whole face against the neighbour as a whole answers with a \
         16x16 quad or with none"
    );
    Ok(())
}

#[test]
fn absence_is_decided_one_neighbour_at_a_time_rather_than_all_at_once() -> TestResult {
    let registry = plain_registry()?;
    let section = solid_section(&registry)?;
    let beyond = solid_section(&registry)?;

    let neighbours = Neighbours::none()
        .with(Facing::NegX, &beyond)
        .with(Facing::NegY, &beyond)
        .with(Facing::NegZ, &beyond);
    let mesh = mesh_section(&section, &neighbours, &registry)?;

    assert_eq!(
        faces(some_quads(&mesh)?),
        vec![
            face(Facing::PosX, LAST, (0, 0), WHOLE_SIDE),
            face(Facing::PosY, LAST, (0, 0), WHOLE_SIDE),
            face(Facing::PosZ, LAST, (0, 0), WHOLE_SIDE),
        ],
        "three of the six are loaded and solid and the other three have not arrived, so three \
         sides are buried and three are at the edge of the world as it stands. Absence has to \
         be six independent answers: read as one flag, this section either shows all six sides \
         or none of them, and every streaming boundary in the world inherits whichever of the \
         two was chosen"
    );
    Ok(())
}

#[test]
fn every_neighbour_is_read_at_the_face_it_shares_and_at_no_other() -> TestResult {
    let registry = plain_registry()?;
    let section = solid_section(&registry)?;
    let around = sections_around(|facing| solid_but_for(the_hole_beyond(facing), &registry))?;

    let mesh = mesh_section(&section, &all_around(&around), &registry)?;

    assert_eq!(
        faces(some_quads(&mesh)?),
        the_face_opposite_each_hole(),
        "all six neighbours are loaded and solid but for one hole each, so the whole answer is \
         six single faces and the six say which neighbour was read where. The six origins are \
         pairwise distinct and none of them reads the same forwards as backwards, so a \
         neighbour taken from the wrong slot, a read at the wrong end of an axis and a swap of \
         a plane's two axes each move exactly one row somewhere no other row expects — instead \
         of two mistakes landing on each other and cancelling"
    );
    Ok(())
}
