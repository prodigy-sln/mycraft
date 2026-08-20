//! The one place a compass word meets an axis, held to being a bijection.
//!
//! Content writes `up`, `down`, `north`, `south`, `east` and `west`; a mesher
//! writes `NegX` … `PosZ`. `Facing::face` is the single total mapping between
//! them, and it lives in the only crate that can see both types — `mc-core`
//! cannot see `Facing`, and `Facing` cannot move, because it is defined in terms
//! of a section's coordinate system.
//!
//! Two enums for six directions reads as duplication on sight, and this file is
//! the answer to that objection: the drift is closed **mechanically**, over both
//! `ALL` arrays, rather than by anybody remembering to keep two lists in step. A
//! facing that answered another facing's face, or two facings that answered the
//! same one, leaves a face no facing names — and a block declaring a key against
//! that word would draw it nowhere while everything still drew.
//!
//! # What this deliberately cannot see, and where that is seen instead
//!
//! **A swap of two words is still a bijection.** Map `north` to `PosZ` and
//! `south` to `NegZ` and every assertion here stays green: six facings, six
//! faces, nothing repeated. What such a swap does is draw the front of a block on
//! its back, and the only witness for it is a block placed in a world with its
//! faces read back by axis — which is a later phase's, because the packer does
//! not resolve a facing's key yet.
//!
//! So this is a completeness guard and says so, rather than a correctness one
//! wearing a completeness guard's clothes.

use std::collections::BTreeSet;

use mc_core::content::Face;
use mc_world::mesh::Facing;

/// The error type this guard propagates with `?`.
type TestResult = Result<(), Box<dyn std::error::Error>>;

/// What the mapping over every facing came to.
///
/// A record rather than three assertions, so one comparison reports the whole
/// shape at once: a mapping that is total but not injective, and one that is
/// injective but misses a face, are different defects and neither may be read as
/// the other.
#[derive(Debug, PartialEq, Eq)]
struct RoundTrip {
    /// How many distinct faces the six facings between them name.
    distinct_faces_named: usize,
    /// Every face no facing names, in `Face::ALL` order.
    faces_no_facing_names: Vec<&'static str>,
}

/// The round trip every facing makes, judged against every face.
fn round_trip() -> RoundTrip {
    let named: BTreeSet<Face> = Facing::ALL.into_iter().map(Facing::face).collect();
    RoundTrip {
        distinct_faces_named: named.len(),
        faces_no_facing_names: Face::ALL
            .into_iter()
            .filter(|face| !named.contains(face))
            .map(Face::as_str)
            .collect(),
    }
}

#[test]
fn every_facing_maps_to_one_face_and_every_face_back_to_one_facing() -> TestResult {
    let mapping = round_trip();

    assert_eq!(
        mapping,
        RoundTrip {
            distinct_faces_named: Face::ALL.len(),
            faces_no_facing_names: Vec::new(),
        },
        "the six words a declaration writes and the six directions a mesher writes are one set \
         seen twice, and this is what says so without anybody keeping two lists in step by hand. \
         A face no facing names is a word a mod author may write against a key that is then drawn \
         on nothing — with every other face still drawing, which is what makes it hard to see"
    );
    Ok(())
}
