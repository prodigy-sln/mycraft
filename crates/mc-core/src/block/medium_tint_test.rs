//! What [`MediumTint::new`] will and will not hold, asked of it directly.
//!
//! # Why this is not covered by the loader that calls it
//!
//! Every declared distance reaches this constructor through the loader's own
//! numeric reader, which refuses a value that is not finite and a value outside
//! its bounds **before** the constructor ever sees one. So the loader's scenarios
//! exercise the loader's guard and not this one: a `new` that cheerfully
//! accepted zero would leave every one of them green, because the refusal they
//! observe is raised a layer up and would go on being raised.
//!
//! That is a single-witness hole on the invariant the whole type exists to
//! carry. The draw path takes the reciprocal of this distance with no guard
//! against dividing by zero, and it is allowed to because this constructor
//! promises the distance is positive — so the promise needs a falsifier of its
//! own, reachable without a content root and without a loader.
//!
//! # Both ends of the pair, because the order of the two checks is observable
//!
//! An infinity passes a `> 0.0` comparison and a NaN fails one. A constructor
//! that asked only about the sign would hold `math.huge`; one that asked only
//! about finiteness would hold zero. Neither omission is visible to a fixture
//! testing a single bad value, which is why both groups below are read whole.
//!
//! # Bits, not values
//!
//! The distance is compared by its bit pattern. `-0.0 == 0.0` is true, so a
//! value comparison cannot say which of the two a constructor kept — and a save
//! folds this number by its bits, where the two are different records for the
//! same declaration.

use super::MediumTint;

/// A colour with three unequal channels, none of them `0x00` or `0xFF`, so a
/// constructor that returned a constant, or that shuffled the three, is
/// reported.
const A_COLOUR: [u8; 3] = [0x3A, 0x6E, 0xA5];

/// A distance an author would write.
const TWELVE_BLOCKS: f32 = 12.0;

/// The smallest distance the engine still keeps at full precision.
///
/// The floor is exclusive, so the interesting boundary is not zero but the
/// smallest thing above it: a constructor whose floor had drifted upward to any
/// round number refuses this and is reported, while one that merely refuses zero
/// is not.
const THE_SMALLEST_NORMAL_DISTANCE: f32 = f32::MIN_POSITIVE;

/// The distances that are not greater than zero.
///
/// `-0.0` is here beside `0.0` because they are the same value and different
/// bits: a constructor comparing bits rather than magnitude would hold one and
/// refuse the other, and `-0.0 > 0.0` is false so both must be refused.
const NOT_GREATER_THAN_ZERO: [f32; 3] = [0.0, -0.0, -1.0];

/// The distances that are not finite.
///
/// All three, because they fail a sign test three different ways: a positive
/// infinity passes `> 0.0`, a negative one fails it, and a NaN fails every
/// comparison there is.
const NOT_FINITE: [f32; 3] = [f32::INFINITY, f32::NEG_INFINITY, f32::NAN];

/// What a constructed tint holds, as values a comparison can be exact about.
fn held(distance: f32) -> Option<([u8; 3], u32)> {
    MediumTint::new(A_COLOUR, distance).map(|tint| (tint.color(), tint.distance().to_bits()))
}

/// What the constructor answered for each of `distances`.
fn held_for(distances: [f32; 3]) -> Vec<Option<([u8; 3], u32)>> {
    distances.into_iter().map(held).collect()
}

#[test]
fn a_finite_distance_greater_than_zero_is_held_with_its_colour_unchanged() {
    assert_eq!(
        (held(TWELVE_BLOCKS), held(THE_SMALLEST_NORMAL_DISTANCE)),
        (
            Some((A_COLOUR, TWELVE_BLOCKS.to_bits())),
            Some((A_COLOUR, THE_SMALLEST_NORMAL_DISTANCE.to_bits())),
        ),
        "the accepting half, and it is what stops the two refusals below being satisfied by a \
         constructor that holds nothing at all. The colour travels out exactly as it went in — \
         this crate performs no I/O and knows no transfer function, so a channel reordered or \
         rescaled here would be a mod author's declared colour altered before anything that \
         could report it. The second value is the boundary that matters: the floor is \
         exclusive, so what a drifted floor takes away is not zero but the smallest thing above \
         it, and a fixture at a round number could not see that"
    );
}

#[test]
fn a_distance_that_is_not_greater_than_zero_is_refused_rather_than_held() {
    assert_eq!(
        held_for(NOT_GREATER_THAN_ZERO),
        vec![None, None, None],
        "a medium reaching its full strength at no distance at all hides everything including \
         the inside of the eye, which is not a weaker claim than the ones this type admits but \
         a different one — and the draw path divides by this number without a guard, which it \
         is only entitled to do because this constructor promises the number is positive. The \
         two zeroes are read together because they are one value and two bit patterns: a \
         constructor comparing bits would hold exactly one of them. Nothing else in the \
         workspace can report this: every declared distance is refused by the loader's numeric \
         reader before it reaches here, so a constructor that accepted all three would leave \
         every loader scenario green"
    );
}

#[test]
fn a_distance_that_is_not_a_finite_number_is_refused_rather_than_held() {
    assert_eq!(
        held_for(NOT_FINITE),
        vec![None, None, None],
        "the three fail a sign test three different ways, which is why they are read together: \
         a positive infinity **passes** `> 0.0`, so a constructor asking only about the sign \
         holds it and hands the draw path a reciprocal of zero; a negative infinity fails that \
         test and would be refused for the wrong reason; and a NaN fails every comparison there \
         is. A fixture holding any one of them alone can see only one of those three mistakes"
    );
}
