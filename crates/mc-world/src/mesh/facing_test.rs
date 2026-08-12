//! Guard. The two authored facts about a facing, and everything derived from
//! them.
//!
//! A facing is written down exactly twice: once as the order the variants are
//! declared in, which is also the order faces are emitted in and the order the
//! neighbour slots are in, and once as the axis-and-sign pair each variant
//! carries. Everything else — which plane a face sits on, which two axes are the
//! primary and the secondary, which end of the neighbour is consulted, where the
//! step from a voxel lands — is computed from that pair.
//!
//! The first test is what keeps the list of all facings from becoming a third
//! authored fact. It is written out once and indexed by the discriminant
//! everywhere else, so if the two ever disagree the neighbour handed to one
//! facing is read for another, and nothing else would say so.
//!
//! The second is the derivation itself, probed for every facing at three places:
//! in the middle of the section, on the boundary plane at the facing's own end,
//! and on the boundary plane at the far end. Those three separate the mistakes
//! that hide in each other. A sign inversion swaps which end goes across. A
//! wrong-side read makes the far boundary cross instead of the near one. An axis
//! confusion moves the answer onto a coordinate that is not the facing's — which
//! is why the two coordinates that are *not* the facing's own differ from each
//! other and from the one that varies, so a mistake lands somewhere visible
//! rather than on the same number by coincidence.

use super::{Adjacent, Facing};
use crate::section::LocalPos;

/// A local position, spelled out.
const fn at(x: u32, y: u32, z: u32) -> LocalPos {
    LocalPos { x, y, z }
}

/// Every facing, probed at three voxels, and where the step off that facing
/// lands from each of them.
///
/// `Across` is the position *within the neighbour section*, which is the
/// mirrored coordinate: a step off the low face at 0 arrives at 15 of the
/// section below, and a step off the high face at 15 arrives at 0 of the one
/// above.
const ADJACENCIES: [(Facing, LocalPos, Adjacent); 18] = [
    (Facing::NegX, at(0, 3, 5), Adjacent::Across(at(15, 3, 5))),
    (Facing::NegX, at(8, 3, 5), Adjacent::Inside(at(7, 3, 5))),
    (Facing::NegX, at(15, 3, 5), Adjacent::Inside(at(14, 3, 5))),
    (Facing::PosX, at(0, 3, 5), Adjacent::Inside(at(1, 3, 5))),
    (Facing::PosX, at(8, 3, 5), Adjacent::Inside(at(9, 3, 5))),
    (Facing::PosX, at(15, 3, 5), Adjacent::Across(at(0, 3, 5))),
    (Facing::NegY, at(3, 0, 5), Adjacent::Across(at(3, 15, 5))),
    (Facing::NegY, at(3, 8, 5), Adjacent::Inside(at(3, 7, 5))),
    (Facing::NegY, at(3, 15, 5), Adjacent::Inside(at(3, 14, 5))),
    (Facing::PosY, at(3, 0, 5), Adjacent::Inside(at(3, 1, 5))),
    (Facing::PosY, at(3, 8, 5), Adjacent::Inside(at(3, 9, 5))),
    (Facing::PosY, at(3, 15, 5), Adjacent::Across(at(3, 0, 5))),
    (Facing::NegZ, at(3, 5, 0), Adjacent::Across(at(3, 5, 15))),
    (Facing::NegZ, at(3, 5, 8), Adjacent::Inside(at(3, 5, 7))),
    (Facing::NegZ, at(3, 5, 15), Adjacent::Inside(at(3, 5, 14))),
    (Facing::PosZ, at(3, 5, 0), Adjacent::Inside(at(3, 5, 1))),
    (Facing::PosZ, at(3, 5, 8), Adjacent::Inside(at(3, 5, 9))),
    (Facing::PosZ, at(3, 5, 15), Adjacent::Across(at(3, 5, 0))),
];

#[test]
fn every_facing_is_listed_at_the_position_its_own_discriminant_names() {
    let listed: Vec<Option<Facing>> = Facing::ALL
        .iter()
        .map(|facing| Facing::ALL.get(*facing as usize).copied())
        .collect();

    assert_eq!(
        listed,
        Facing::ALL.map(Some).to_vec(),
        "the list of every facing and the discriminants are one fact, not two: a neighbour is \
         stored at the slot its facing's discriminant names and read back through this list. \
         A list whose order drifted from the declaration order would hand the section beyond \
         one facing to another, and would reorder the emitted quads by exactly the same amount \
         — so the mesh would still look self-consistent"
    );
}

#[test]
fn each_facing_steps_to_its_own_neighbour_and_mirrors_across_its_own_boundary() {
    let stepped: Vec<Adjacent> = ADJACENCIES
        .iter()
        .map(|(facing, voxel, _)| facing.adjacent(*voxel))
        .collect();
    let landings: Vec<Adjacent> = ADJACENCIES.iter().map(|(_, _, landing)| *landing).collect();

    assert_eq!(
        stepped, landings,
        "each facing steps one voxel along its own axis, in its own direction, and leaves the \
         section only at the end it points at — arriving at the opposite end of the neighbour \
         there. Every row here is one of the six rows of the derived table, and a table with \
         one wrong row breaks the facing nobody happened to write a scenario about"
    );
}
