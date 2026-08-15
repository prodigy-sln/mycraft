//! What a texture **declared seamless** is refused for, and what it is not.
//!
//! Three legs, evaluated in one declared order — period, then coverage, then
//! edges — and under this policy the **first failing leg is the answer**, so the
//! diagnostic is reproducible. Two fixtures here fail two legs each, which is
//! the only way that ordering is decidable at all.
//!
//! **Every scenario asserts through `emit`, never through a pure judgement.**
//! Testing a decision does not test that the emission consults it: a `judge`
//! called only by its own test is agreement between two copies of one decision,
//! and the emitter can stop asking it entirely with everything still green.
//!
//! The positive controls are three, one per failing leg, and they are the point
//! of the file rather than a courtesy. An implementation whose `--seamless`
//! refused **unconditionally** would satisfy every refusal below; the scenarios
//! that must be *emitted* are what stops that shape passing.

mod common;

use common::TestResult;
use common::preview::{Coverage, coverage};
use common::texture::{
    CHECKER_PALETTE, Emission, GRADIENT, GREY, LIME_PALETTE, Leg, Legs, ONE_PER_VOXEL, Refusal,
    STAIRCASE_PALETTE, Tone, emitted, nothing_unnamed, one_face, refusal, tiling,
};
use common::tiles::{
    checker_columns, deep_block, gradient_columns, gradient_rows, lime_columns, narrow_slab,
    notched_gradient, notched_narrow_slab, notched_slab, single_voxel, solid_block, staircase_rows,
    tall_slab,
};
use voxforge::render::View;

/// The palette a one-grey fixture is painted from.
const PLAIN: [Tone; 1] = [GREY];

#[test]
fn a_block_sized_model_declared_seamless_is_emitted_at_the_size_its_voxels_earn() -> TestResult {
    // 4 voxels one block across, 8 pixels each: 32 by 32, by arithmetic.
    let outcome = emitted(&solid_block(), &PLAIN, Emission::seamless(View::Front)?)?;

    assert_eq!(
        one_face(&outcome),
        tiling(32, 32),
        "a texture that tiles is emitted rather than refused — without this, every seamless-declared scenario is a refusal and an implementation whose flag refuses unconditionally passes all of them"
    );
    Ok(())
}

#[test]
fn a_model_narrower_than_one_block_is_refused_naming_its_extent_and_its_scale() -> TestResult {
    let outcome = emitted(&narrow_slab(), &PLAIN, Emission::seamless(View::Front)?)?;

    assert_eq!(
        refusal(&outcome, &["x axis", "3", "4"]),
        Refusal::Named {
            legs: Legs::Only(Leg::Period),
            missing: nothing_unnamed()
        },
        "a period that is not the block grid's is the whole reason the leg exists, and an author repairing it needs the axis, what the model is and what a block is"
    );
    Ok(())
}

#[test]
fn a_model_wider_than_one_block_is_refused_naming_its_extent_and_its_scale() -> TestResult {
    let outcome = emitted(&tall_slab(), &PLAIN, Emission::seamless(View::Front)?)?;

    assert_eq!(
        refusal(&outcome, &["y axis", "8", "4"]),
        Refusal::Named {
            legs: Legs::Only(Leg::Period),
            missing: nothing_unnamed()
        },
        "the two sides of one equality: a `voxels <= scale` bound accepts the narrow slab and a `voxels >= scale` bound accepts this one, so neither subsumes the other"
    );
    Ok(())
}

#[test]
fn a_models_depth_is_not_one_of_the_two_axes_a_face_is_measured_across() -> TestResult {
    let outcome = emitted(&deep_block(), &PLAIN, Emission::seamless(View::Front)?)?;

    assert_eq!(
        one_face(&outcome),
        tiling(32, 32),
        "a face has two in-plane axes and the depth axis is neither — every other fixture is one block deep, so nothing else tells a two-axis check from a three-axis one"
    );
    Ok(())
}

#[test]
fn a_model_failing_both_period_and_coverage_is_refused_for_its_period() -> TestResult {
    let outcome = emitted(
        &notched_narrow_slab(),
        &PLAIN,
        Emission::seamless(View::Front)?,
    )?;

    assert_eq!(
        refusal(&outcome, &["x axis", "3", "4"]),
        Refusal::Named {
            legs: Legs::Only(Leg::Period),
            missing: nothing_unnamed()
        },
        "the legs are ordered and the first failure is the answer, so this model's opacity failure is not what it hears about"
    );
    Ok(())
}

#[test]
fn a_face_showing_the_void_is_refused_naming_how_many_pixels_and_the_first() -> TestResult {
    // The missing voxel is at `x = 1, y = 0`: 8 by 8 pixels, so 64 of them, and
    // the first in a row-major scan is image row 24, column 8.
    let outcome = emitted(&notched_slab(), &PLAIN, Emission::seamless(View::Front)?)?;

    assert_eq!(
        refusal(&outcome, &["64", "row 24", "column 8"]),
        Refusal::Named {
            legs: Legs::Only(Leg::Opacity),
            missing: nothing_unnamed()
        },
        "a block face is opaque, and `first` is only meaningful because the scan order is declared row-major from row 0, column 0"
    );
    Ok(())
}

#[test]
fn a_solid_block_sized_model_covers_every_pixel_of_its_texture() -> TestResult {
    // 32 by 32 pixels is 1024, derived rather than counted off a render.
    let outcome = emitted(&solid_block(), &PLAIN, Emission::seamless(View::Front)?)?;
    let measured = outcome.only().map(|only| coverage(&only.image));

    assert_eq!(
        measured,
        Some(Coverage::Drawn(1024)),
        "one opaque sample per pixel leaves no third answer, so a solid model one block across has no pixel that is neither drawn nor transparent"
    );
    Ok(())
}

#[test]
fn a_face_failing_both_coverage_and_edges_is_refused_for_its_coverage() -> TestResult {
    let outcome = emitted(
        &notched_gradient(),
        &GRADIENT,
        Emission::seamless(View::Front)?,
    )?;

    assert_eq!(
        refusal(&outcome, &["64", "row 16", "column 8"]),
        Refusal::Named {
            legs: Legs::Only(Leg::Opacity),
            missing: nothing_unnamed()
        },
        "coverage is evaluated before edges, so a notched gradient hears about its hole rather than about its wrap"
    );
    Ok(())
}

#[test]
fn a_wrap_stepping_further_than_the_content_does_is_refused_naming_both_steps() -> TestResult {
    // Equal steps of 85, since `255 / 3 = 85`; the wrap is the full 255.
    let outcome = emitted(
        &gradient_columns(),
        &GRADIENT,
        Emission::seamless(View::Front)?,
    )?;

    assert_eq!(
        refusal(&outcome, &["x axis", "255", "85"]),
        Refusal::Named {
            legs: Legs::Only(Leg::Edges),
            missing: nothing_unnamed()
        },
        "the metric is self-calibrating, so the diagnostic has to carry both numbers it compared — no threshold is declared anywhere for a reader to look up"
    );
    Ok(())
}

#[test]
fn a_wrap_stepping_no_further_than_the_content_does_is_emitted() -> TestResult {
    let outcome = emitted(
        &checker_columns(),
        &CHECKER_PALETTE,
        Emission::seamless(View::Front)?,
    )?;

    assert_eq!(
        one_face(&outcome),
        tiling(32, 32),
        "the wrap's 255 is no larger than the 255 the texture already contains, which is the whole point of measuring a seam against the content rather than against a declared constant"
    );
    Ok(())
}

#[test]
fn the_same_gradient_running_down_the_image_is_refused_naming_the_vertical_axis() -> TestResult {
    let outcome = emitted(
        &gradient_rows(),
        &GRADIENT,
        Emission::seamless(View::Front)?,
    )?;

    assert_eq!(
        refusal(&outcome, &["y axis", "255", "85"]),
        Refusal::Named {
            legs: Legs::Only(Leg::Edges),
            missing: nothing_unnamed()
        },
        "a check written for one axis passes the gradient running across and fails this one"
    );
    Ok(())
}

#[test]
fn a_step_is_the_largest_of_the_three_channels_rather_than_their_sum_or_mean() -> TestResult {
    // Interior steps 64, 64 and `max(128, 127, 128) = 128`; the wrap is
    // `(0, 255, 0)` against `(0, 0, 0)`, which is 255. Read one channel and it
    // is `0 > 128`, sum them and it is `255 > 383`, average them and it is
    // `85 > 127.67` — all three say this tiles, and all three are wrong.
    let outcome = emitted(
        &lime_columns(),
        &LIME_PALETTE,
        Emission::seamless(View::Front)?,
    )?;

    assert_eq!(
        refusal(&outcome, &["x axis", "255", "128"]),
        Refusal::Named {
            legs: Legs::Only(Leg::Edges),
            missing: nothing_unnamed()
        },
        "every other fixture in this file is greyscale, so this is the only one that can tell a per-channel maximum from a sum or a mean"
    );
    Ok(())
}

#[test]
fn an_image_one_pixel_across_has_no_interior_pair_and_tiles() -> TestResult {
    let outcome = emitted(
        &single_voxel(),
        &PLAIN,
        Emission::seamless(View::Front)?.at(ONE_PER_VOXEL),
    )?;

    assert_eq!(
        one_face(&outcome),
        tiling(1, 1),
        "the largest step within a one-pixel axis is a maximum over an empty set, and a single column repeated is seamless by definition — the answer is 0 against 0, not a panic and not a sentinel"
    );
    Ok(())
}

#[test]
fn a_seam_in_one_row_is_not_licensed_by_a_larger_step_in_another() -> TestResult {
    // Taken over the whole image, the largest step within is the top row's 255
    // and `255 > 255` is false. Taken per row, the bottom row steps at most 128
    // and wraps by 255, so image row 24 fails.
    let outcome = emitted(
        &staircase_rows(),
        &STAIRCASE_PALETTE,
        Emission::seamless(View::Front)?,
    )?;

    assert_eq!(
        refusal(&outcome, &["row 24", "255", "128"]),
        Refusal::Named {
            legs: Legs::Only(Leg::Edges),
            missing: nothing_unnamed()
        },
        "a maximum taken over the whole image lets an extreme step anywhere license a discontinuity anywhere else, and this fixture shows a seam while scoring 255 against 255"
    );
    Ok(())
}
