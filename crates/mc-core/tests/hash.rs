//! Guard. The published fold, on byte sequences stated here, against values
//! this file derives from the constants by hand.
//!
//! # Why this lives in `mc-core`'s own tests
//!
//! [`fnv_1a_64`](mc_core::hash::fnv_1a_64) has exactly one caller today — the
//! save format in `mc-world` — and that caller is already covered, thoroughly,
//! through `mc-world`'s own guards. So this file is not here to test the
//! arithmetic a second time. It is here because **the published surface is a
//! different observable from that private use of it.**
//!
//! `voxforge` is the second consumer and it does not exist yet. When it arrives
//! it will fold the same bytes to the same value from the other side of a
//! dependency line that only points one way, and *that* is the contract this
//! crate publishes. Asserting it through `mc-world` asserts that one caller
//! still works; asserting it here asserts that the thing both callers reach is
//! what it says it is.
//!
//! Writing the equivalent inside `mc-world`'s sibling instead would have been
//! the same computation through the same code path, which is a test that
//! re-proves what another test already proves and is worse than no test at all.
//!
//! # The constants are restated, and here that is forced as well as right
//!
//! `FNV_OFFSET_BASIS` and `FNV_PRIME` are private to the module under test, so
//! an integration test could not import them if it wanted to. It would not want
//! to: reading a constant out of the module that is being judged makes a changed
//! constant invisible to the guard that exists to see it. The values below are
//! the published FNV-1a-64 constants, written out.
//!
//! # Every expected value is arithmetic, never a snapshot
//!
//! Each one is a single unrolled expression — exclusive-or a byte, multiply,
//! repeat — evaluated at compile time from the two constants. **Nothing here was
//! taken from a run**, and there is no loop on this side, so the derivation is a
//! statement of what the constants compute rather than a second copy of the
//! implementation that could go wrong the same way.
//!
//! The sequences are chosen to be falsifiable rather than representative, and
//! each says below what it is for.
//!
//! # What this does not assert, and what makes it non-vacuous
//!
//! Not where the fold lives. That was measured: leaving a private copy behind in
//! `mc-world` and calling it reddened nothing at all, so no test in this
//! workspace can see which crate the function is in. The move is compiler-held
//! and reviewer-held, and this file holds the **value** instead.
//!
//! Nor could this guard be red first. `mc_core::hash` had to exist before an
//! integration test could name it, and it landed before this was written — so
//! its falsifiability comes from mutation: changing the offset basis by one, or
//! returning the basis without folding, reddens it. Both outcomes are recorded
//! in the phase's own record.

use mc_core::hash::fnv_1a_64;

/// Where an FNV-1a 64 fold starts, and what it multiplies by.
///
/// The published constants, restated. See the module header: the pair under test
/// is private, and importing it would be the guard agreeing with what it judges.
const STATED_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const STATED_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Nothing at all.
///
/// The boundary, and the only case that separates a fold starting at the
/// published basis from one starting at zero — every other sequence here is
/// answered plausibly by both.
const NOTHING: &[u8] = &[];

/// One byte with no bits set.
///
/// A zero byte leaves the accumulator alone and the multiply still happens, so
/// this is what says a zero is not a byte the fold passes over.
const ONE_EMPTY_BYTE: &[u8] = &[0x00];

/// One byte with every bit set — the other end of a byte's range.
const ONE_FULL_BYTE: &[u8] = &[0xff];

/// Two bytes, and the same two the other way round.
///
/// Their stated values below differ, which is what says the fold is a fold and
/// not a tally: an implementation that added its bytes, or exclusive-ored them
/// together, answers identically for this pair and is caught by it and by
/// nothing else here.
const TWO_BYTES: &[u8] = &[0x01, 0x02];
const THE_SAME_TWO_REVERSED: &[u8] = &[0x02, 0x01];

/// The sequences this file states, in the order their values are stated below.
const STATED_SEQUENCES: [&[u8]; 5] = [
    NOTHING,
    ONE_EMPTY_BYTE,
    ONE_FULL_BYTE,
    TWO_BYTES,
    THE_SAME_TWO_REVERSED,
];

/// What the two constants compute for each of those sequences.
///
/// Unrolled by hand, one expression apiece, in the same order. The second is
/// written without an exclusive-or because a zero byte leaves the accumulator
/// unchanged and spelling `^ 0x00` says the same thing more slowly.
const STATED_VALUES: [u64; 5] = [
    STATED_OFFSET_BASIS,
    STATED_OFFSET_BASIS.wrapping_mul(STATED_PRIME),
    (STATED_OFFSET_BASIS ^ 0xff).wrapping_mul(STATED_PRIME),
    ((STATED_OFFSET_BASIS ^ 0x01).wrapping_mul(STATED_PRIME) ^ 0x02).wrapping_mul(STATED_PRIME),
    ((STATED_OFFSET_BASIS ^ 0x02).wrapping_mul(STATED_PRIME) ^ 0x01).wrapping_mul(STATED_PRIME),
];

#[test]
fn the_fold_of_a_stated_byte_sequence_is_the_value_the_constants_compute() {
    let folded: Vec<(&[u8], u64)> = STATED_SEQUENCES
        .iter()
        .map(|bytes| (*bytes, fnv_1a_64(bytes)))
        .collect();
    let stated: Vec<(&[u8], u64)> = STATED_SEQUENCES
        .iter()
        .zip(STATED_VALUES)
        .map(|(bytes, value)| (*bytes, value))
        .collect();

    assert_eq!(
        folded, stated,
        "the fold this crate publishes is the one its published constants describe: start at the \
         offset basis, exclusive-or each byte in turn, multiply by the prime after each. Every \
         value on the right is that arithmetic written out rather than a number a run produced, \
         and every value on the left came through the surface a second program will reach for — \
         which is the part `mc-world`'s own guards, reaching the same function privately, cannot \
         say anything about"
    );
}
