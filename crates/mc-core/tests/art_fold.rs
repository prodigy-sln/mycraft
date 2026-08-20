//! Guard. The fold a texture set's index records, over byte sequences stated
//! here, against values this file derives from the published constants by hand.
//!
//! # Why the sequence is the thing under test and the arithmetic is not
//!
//! [`fnv_1a_64`](mc_core::hash::fnv_1a_64) is already guarded, unrolled, in
//! `hash.rs` beside this file. What that guard cannot say anything about is
//! **which bytes get folded**: two programs that agree on FNV-1a-64 and
//! disagree about where one source ends and the next begins compute two
//! different values from one set of files, and the disagreement is silent.
//!
//! So every expected value below is written as an explicit byte array — the
//! whole concatenation, length prefixes included, spelled out — folded by a
//! loop over the two constants restated here. Nothing on the expected side
//! reaches the crate under test, and nothing was taken from a run.
//!
//! # Why the constants are restated rather than imported
//!
//! They are private to `hash.rs`, and reading a constant out of the module
//! being judged makes a changed constant invisible to the guard that exists to
//! see it. These are the published FNV-1a-64 constants, written out.

use mc_core::art::folded_sources;

/// Where an FNV-1a 64 fold starts, and what it multiplies by.
const STATED_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const STATED_PRIME: u64 = 0x0000_0100_0000_01b3;

/// One source: the path `a`, holding the single byte `0x01`.
const ONE_SOURCE: [(&str, &[u8]); 1] = [("a", &[0x01])];

/// What that folds over: the path's length as a little-endian `u64`, the path,
/// the file's length, the file.
const ONE_SOURCES_BYTES: [u8; 18] = [
    0x01, 0, 0, 0, 0, 0, 0, 0,    // "a" is one byte long
    b'a', //
    0x01, 0, 0, 0, 0, 0, 0, 0,    // its contents are one byte long
    0x01, //
];

/// Two sources, the second of them empty — an empty file is a length of zero
/// and no bytes, not a source the fold passes over.
const TWO_SOURCES: [(&str, &[u8]); 2] = [("ab", &[0x02, 0x03]), ("c", &[])];

/// What those two fold over, in that order.
const TWO_SOURCES_BYTES: [u8; 37] = [
    0x02, 0, 0, 0, 0, 0, 0, 0, // "ab"
    b'a', b'b', //
    0x02, 0, 0, 0, 0, 0, 0, 0, // its two bytes
    0x02, 0x03, //
    0x01, 0, 0, 0, 0, 0, 0, 0,    // "c"
    b'c', //
    0x00, 0, 0, 0, 0, 0, 0, 0, // holding nothing
];

/// The same two sources, the other way round.
///
/// Their stated values below differ, which is what says the fold is a fold over
/// an ordered sequence and not a tally: an implementation that sorted its
/// sources, or combined them commutatively, answers identically for this pair
/// and is caught by it and by nothing else here.
const THE_SAME_TWO_REVERSED: [(&str, &[u8]); 2] = [("c", &[]), ("ab", &[0x02, 0x03])];

/// What those fold over.
const REVERSED_BYTES: [u8; 37] = [
    0x01, 0, 0, 0, 0, 0, 0, 0,    // "c"
    b'c', //
    0x00, 0, 0, 0, 0, 0, 0, 0, // holding nothing
    0x02, 0, 0, 0, 0, 0, 0, 0, // "ab"
    b'a', b'b', //
    0x02, 0, 0, 0, 0, 0, 0, 0, // its two bytes
    0x02, 0x03, //
];

/// One source whose path is `ab` and whose contents are empty.
const PATH_CARRIES_THE_B: [(&str, &[u8]); 1] = [("ab", &[])];

/// One source whose path is `a` and whose single content byte is `b`.
///
/// The pair exists because **without the length prefixes these two are the same
/// bytes**: `ab` either way. That is the boundary a separator cannot defend and
/// a length prefix can, and no scenario in this spec reaches it.
const CONTENTS_CARRY_THE_B: [(&str, &[u8]); 1] = [("a", b"b")];

/// What the first of that pair folds over.
const PATH_CARRIES_THE_B_BYTES: [u8; 18] = [
    0x02, 0, 0, 0, 0, 0, 0, 0, // "ab"
    b'a', b'b', //
    0x00, 0, 0, 0, 0, 0, 0, 0, // holding nothing
];

/// What the second folds over.
const CONTENTS_CARRY_THE_B_BYTES: [u8; 18] = [
    0x01, 0, 0, 0, 0, 0, 0, 0,    // "a"
    b'a', //
    0x01, 0, 0, 0, 0, 0, 0, 0,    // holding one byte
    b'b', //
];

/// `bytes` folded with FNV-1a 64, from the restated constants.
///
/// A loop rather than an unrolled expression, because the sequences here are
/// tens of bytes long — but it shares no code with the subject, and the
/// sequences it folds are written out above rather than assembled by any rule
/// the implementation also knows.
fn folded_by_hand(bytes: &[u8]) -> u64 {
    let mut folded = STATED_OFFSET_BASIS;
    for byte in bytes {
        folded ^= u64::from(*byte);
        folded = folded.wrapping_mul(STATED_PRIME);
    }
    folded
}

#[test]
fn the_recorded_value_is_the_fnv_fold_of_the_stated_byte_sequence() {
    let folded = [
        folded_sources(&ONE_SOURCE),
        folded_sources(&TWO_SOURCES),
        folded_sources(&THE_SAME_TWO_REVERSED),
    ];
    let stated = [
        folded_by_hand(&ONE_SOURCES_BYTES),
        folded_by_hand(&TWO_SOURCES_BYTES),
        folded_by_hand(&REVERSED_BYTES),
    ];

    assert_eq!(
        folded, stated,
        "the value an index records is an FNV-1a-64 fold over a stated sequence: per source, the \
         recorded path preceded by its length as a little-endian u64, then the file's bytes \
         preceded by theirs. Every value on the right is that sequence written out as bytes and \
         folded from the published constants restated in this file — never a number a run \
         produced, which is what the scenario's *rather than a value derived from the standard \
         library's hasher* is asking for"
    );
}

#[test]
fn two_sources_whose_bytes_and_paths_could_be_re_split_fold_to_different_values() {
    let folded = (
        folded_sources(&PATH_CARRIES_THE_B),
        folded_sources(&CONTENTS_CARRY_THE_B),
    );
    let stated = (
        folded_by_hand(&PATH_CARRIES_THE_B_BYTES),
        folded_by_hand(&CONTENTS_CARRY_THE_B_BYTES),
    );

    assert_eq!(
        (folded.0, folded.1, folded.0 == folded.1),
        (stated.0, stated.1, false),
        "a length prefix is what makes `ab` with no contents a different fold from `a` holding \
         `b`. Concatenated without prefixes both are the two bytes `ab`, so a fold that dropped \
         them answers one value twice — which the third member of each tuple \
         is here to catch, since two equal wrong values would otherwise still have to match two \
         stated ones to fail"
    );
}
