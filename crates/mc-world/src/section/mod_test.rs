//! Guard. A position and its linear index name the same voxel, both ways round.
//!
//! Folding three coordinates into one number and reading them back out are two
//! halves of the same layout written twice, and a layout written twice can be
//! half right. A fold that swapped two axes, or an unfold that shifted by the
//! wrong amount, is invisible to anything that only ever goes one way — the
//! section accessors all fold, so every one of them would agree with a wrong
//! fold, and only the mesher, which has to name the voxel it stopped at, ever
//! unfolds.
//!
//! So the two are checked against each other over every position a section has,
//! rather than over a handful. Half of a wrong pair is a mistake at one axis or
//! at one bit, and a spot check lands on it only by luck.

use super::{LocalPos, SECTION_SIZE, Section, SectionError};

/// Every position a section has, x fastest, then y, then z.
fn every_position() -> impl Iterator<Item = LocalPos> {
    (0..SECTION_SIZE).flat_map(|z| {
        (0..SECTION_SIZE).flat_map(move |y| (0..SECTION_SIZE).map(move |x| LocalPos { x, y, z }))
    })
}

#[test]
fn every_position_comes_back_out_of_its_own_linear_index_unchanged() -> Result<(), SectionError> {
    let mut round_tripped = Vec::new();
    for asked in every_position() {
        round_tripped.push((
            asked,
            Section::position_of_voxel(Section::voxel_index(asked)?),
        ));
    }

    let disagreed: Vec<(LocalPos, LocalPos)> = round_tripped
        .into_iter()
        .filter(|(asked, answered)| asked != answered)
        .collect();

    assert!(
        disagreed.is_empty(),
        "a position folded into a linear index and unfolded again is the position that went \
         in. These were not, which means the two directions disagree about which bits of an \
         index belong to which axis — and the fold is what every accessor uses while the \
         unfold is what names the voxel in a refusal, so a caller would be pointed at some \
         other voxel entirely: {disagreed:?}"
    );
    Ok(())
}
