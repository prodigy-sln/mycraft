//! What a texture that was **not** declared seamless is told, and emitted
//! anyway.
//!
//! Not every texture has to tile. One destined for a large flat terrain surface
//! must wrap; one for a single decorative block need not, and refusing it would
//! be the tool inventing a rule its author never asked for. So the verdict is
//! **computed on every emission** — the information is free and an author wants
//! it either way — and **binds only when the emission declares the texture
//! seamless**. That is the partition `inspect` already draws: a defect sets a
//! non-zero exit, an observation never does.
//!
//! The ordering differs by policy, deliberately. Under `--seamless` the first
//! failing leg is the answer, so the diagnostic is reproducible. Without it
//! **every** failing leg is reported, because the verdict is advice there and
//! truncating advice costs a round trip against the latency of the authoring
//! loop. It bites hardest on the class that can never pass the coverage leg at
//! all: a glass or leaf texture would report `FaceIsNotOpaque` forever and never
//! learn whether its edges agree.

mod common;

use common::TestResult;
use common::texture::{
    Emission, GRADIENT, GREY, OneFace, STAIRCASE_PALETTE, Tone, emitted, one_face, tiling,
};
use common::tiles::{narrow_slab, notched_gradient, solid_block, staircase_columns};
use voxforge::format::Axis;
use voxforge::render::View;
use voxforge::texture::{Line, PixelPos, SeamVerdict};

/// The palette a one-grey fixture is painted from.
const PLAIN: [Tone; 1] = [GREY];

/// That face, emitted at that size, carrying exactly those verdicts.
fn carrying(width: u32, height: u32, verdicts: Vec<SeamVerdict>) -> OneFace {
    OneFace::Emitted {
        width,
        height,
        verdicts,
    }
}

#[test]
fn a_texture_that_does_not_tile_is_emitted_at_the_size_its_own_voxels_earn() -> TestResult {
    // 3 voxels by 4 at 8 pixels each: 24 by 32, from the assembled extent and
    // not from the declared scale. Every other scenario producing an image has
    // an extent equal to `scale`, so a raster sized from `scale` is invisible to
    // all of them.
    let outcome = emitted(&narrow_slab(), &PLAIN, Emission::reported(View::Front)?)?;

    assert_eq!(
        one_face(&outcome),
        carrying(
            24,
            32,
            vec![SeamVerdict::PeriodIsNotOneBlock {
                axis: Axis::X,
                voxels: 3,
                scale: 4
            }]
        ),
        "an undeclared texture is reported rather than refused, and what it is measured at is what its voxels earn rather than what a block is"
    );
    Ok(())
}

#[test]
fn a_texture_that_does_tile_is_told_so_without_having_been_asked() -> TestResult {
    let outcome = emitted(&solid_block(), &PLAIN, Emission::reported(View::Front)?)?;

    assert_eq!(
        one_face(&outcome),
        tiling(32, 32),
        "the verdict is computed on every emission, so it cannot rot unexercised on the path most emissions take — and printing a constant first variant fails the scenario beside this one"
    );
    Ok(())
}

#[test]
fn an_undeclared_texture_hears_about_every_leg_it_fails_rather_than_the_first() -> TestResult {
    // 64 transparent pixels from the missing voxel at `x = 1, y = 1`, the first
    // at image row 16, column 8; and the gradient's own wrap of 255 against its
    // largest interior step of 85, first failing in image row 0.
    let outcome = emitted(
        &notched_gradient(),
        &GRADIENT,
        Emission::reported(View::Front)?,
    )?;

    assert_eq!(
        one_face(&outcome),
        carrying(
            32,
            32,
            vec![
                SeamVerdict::FaceIsNotOpaque {
                    transparent: 64,
                    first: PixelPos { column: 8, row: 16 }
                },
                SeamVerdict::EdgesDisagree {
                    axis: Axis::X,
                    at: Line::Row(0),
                    across: 255,
                    largest_within: 85
                }
            ]
        ),
        "unflagged, every leg is evaluated rather than stopping at the first failure — a texture that can never pass coverage would otherwise never learn whether its edges agree"
    );
    Ok(())
}

/// Additional coverage: the vertical axis measured per column, on a fixture
/// where the failing column is not the first.
///
/// The scenarios give the vertical axis exactly **one** witness, and its fixture
/// is a gradient whose every column is identical — so the failing column is
/// column 0 and an implementation that always reports the first column, or that
/// measures the vertical axis over the whole image rather than per column,
/// passes it. This is the horizontal per-row fixture transposed: taken over the
/// image the largest step within is the `x = 0` column's 255 and `255 > 255` is
/// false, while the `x = 3` column steps at most 128 and wraps by 255.
///
/// It also pins that a vertical failure names a **column** rather than a row.
/// Nothing else can: the same index means two different things on the two axes,
/// and for `top` and `bottom` neither of them is the `y` axis at all.
#[test]
fn a_seam_in_one_column_is_not_licensed_by_a_larger_step_in_another() -> TestResult {
    let outcome = emitted(
        &staircase_columns(),
        &STAIRCASE_PALETTE,
        Emission::reported(View::Front)?,
    )?;

    assert_eq!(
        one_face(&outcome),
        carrying(
            32,
            32,
            vec![SeamVerdict::EdgesDisagree {
                axis: Axis::Y,
                at: Line::Column(24),
                across: 255,
                largest_within: 128
            }]
        ),
        "per column and not per image, and the column it names is the one that fails rather than the first one it looked at"
    );
    Ok(())
}
