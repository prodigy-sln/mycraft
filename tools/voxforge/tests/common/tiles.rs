//! The model documents the block-texture scenarios are measured against.
//!
//! **Each one is built to fail exactly the leg it is about**, and the numbers
//! are derived here rather than read off a render:
//!
//! - The four-step gradient is `#000000`, `#555555`, `#aaaaaa`, `#ffffff`,
//!   because `255 / 3 = 85` exactly. Its wrap is 255 and its largest interior
//!   step is 85, so `255 > 85` and the axis fails.
//! - The checker is the same four columns alternating black and white: wrap 255
//!   against interior 255, and `255 > 255` is false, so it passes. That is the
//!   whole point of measuring a seam against the content rather than against a
//!   declared constant, and it is the positive control for the edge leg.
//! - The lime gradient is the one fixture here that is not greyscale. Its
//!   interior steps are 64, 64 and `max(128, 127, 128) = 128`, and its wrap is
//!   `(0, 255, 0) → (0, 0, 0)`, which is 255. A metric reading one channel
//!   answers `0 > 128`, one summing channels `255 > 383`, one averaging them
//!   `85 > 127.67` — all three say the texture tiles, and all three are wrong.
//! - The staircase separates *per row* from *per image*. Its top voxel row
//!   steps 255 and its bottom row steps at most 128, while both wrap by 255. Per
//!   row the bottom row fails; per image `largest_within` is the top row's 255,
//!   `255 > 255` is false, and a seam ships.
//!
//! Every notch sits off the model's own symmetry axes and away from the
//! bounding box, so that the assembled extent is unchanged by it — a notch at a
//! corner would shrink the volume and turn a coverage fixture into a period one.

use super::texture::{BLACK, DIM, GRADIENT, GREY, LIME, MARKERS, Tone, WHITE, model};

/// How many voxels the texture fixtures declare to one block edge.
const SCALE: u32 = 4;

/// A solid `[4, 4, 4]` block of one grey, one block across every axis.
#[must_use]
pub fn solid_block() -> String {
    model((4, 4, 4), SCALE, &[GREY], &|_, _, _| Some(GREY))
}

/// A solid `[3, 4, 1]` slab — three voxels on `x` where a block is four.
#[must_use]
pub fn narrow_slab() -> String {
    model((3, 4, 1), SCALE, &[GREY], &|_, _, _| Some(GREY))
}

/// A solid `[4, 8, 1]` slab — eight voxels on `y` where a block is four.
///
/// The other side of the same equality as [`narrow_slab`]: a `voxels <= scale`
/// bound accepts that one and a `voxels >= scale` bound accepts this one, so
/// neither subsumes the other.
#[must_use]
pub fn tall_slab() -> String {
    model((4, 8, 1), SCALE, &[GREY], &|_, _, _| Some(GREY))
}

/// A solid `[4, 4, 7]` block — one block across both of `front`'s in-plane axes
/// and seven voxels deep.
///
/// Every other fixture here is one block deep, so nothing else can tell a
/// two-axis period check from a three-axis one.
#[must_use]
pub fn deep_block() -> String {
    model((4, 4, 7), SCALE, &[GREY], &|_, _, _| Some(GREY))
}

/// A solid `[4, 4, 3]` block — one block across two axes and three voxels on
/// the third.
///
/// Cubic-ness is a precondition of a face-set *request* rather than a seam
/// verdict, so the axis this fails on is deliberately the one `front` does not
/// measure: a set of this model is refused while its `front` texture alone is
/// not.
#[must_use]
pub fn not_a_cube() -> String {
    model((4, 4, 3), SCALE, &[GREY], &|_, _, _| Some(GREY))
}

/// A `[3, 4, 1]` slab missing the cell at `(1, 1, 0)`.
///
/// Fails **two** legs — its period is three voxels and its face shows the void —
/// which is what makes it decidable that period is evaluated first.
#[must_use]
pub fn notched_narrow_slab() -> String {
    model((3, 4, 1), SCALE, &[GREY], &|x, y, _| {
        (x != 1 || y != 1).then_some(GREY)
    })
}

/// A `[4, 4, 1]` slab of one grey missing the cell at `(1, 0, 0)`.
///
/// The cell sits off the diagonal and off both midlines, so a transpose or
/// either mirror moves the transparent pixel this fixture reports.
#[must_use]
pub fn notched_slab() -> String {
    model((4, 4, 1), SCALE, &[GREY], &|x, y, _| {
        (x != 1 || y != 0).then_some(GREY)
    })
}

/// A `[4, 4, 1]` slab whose four voxel columns run the four-step gradient.
#[must_use]
pub fn gradient_columns() -> String {
    model((4, 4, 1), SCALE, &GRADIENT, &|x, _, _| column(x))
}

/// A `[4, 4, 1]` slab whose four voxel *rows* run the four-step gradient.
///
/// A check written for one axis passes [`gradient_columns`] and fails this.
#[must_use]
pub fn gradient_rows() -> String {
    model((4, 4, 1), SCALE, &GRADIENT, &|_, y, _| column(y))
}

/// The four-step gradient with the cell at `(1, 1, 0)` missing.
///
/// Fails coverage *and* edges, which is what makes it decidable both that
/// coverage is evaluated first and that the unflagged path evaluates every leg
/// rather than stopping at the first failure.
#[must_use]
pub fn notched_gradient() -> String {
    model((4, 4, 1), SCALE, &GRADIENT, &|x, y, _| {
        (x != 1 || y != 1).then(|| column(x)).flatten()
    })
}

/// A `[4, 4, 4]` cube whose four voxel *layers on `z`* run the four-step
/// gradient.
///
/// The fixture a face set needs: `front` sees only its `z = 3` layer and `back`
/// only its `z = 0` one, so both are uniform and both tile, while the other four
/// faces run the gradient along one of their in-plane axes and fail. If every
/// face failed, "writes none of the six" would be satisfied by code that never
/// writes on any failure.
#[must_use]
pub fn gradient_depth() -> String {
    model((4, 4, 4), SCALE, &GRADIENT, &|_, _, z| column(z))
}

/// A `[4, 4, 1]` slab whose four voxel columns alternate black and white.
#[must_use]
pub fn checker_columns() -> String {
    model((4, 4, 1), SCALE, &[BLACK, WHITE], &|x, _, _| {
        Some(if x.is_multiple_of(2) { BLACK } else { WHITE })
    })
}

/// A solid `[4, 4, 1]` slab whose four voxel columns run `#000000`, `#404040`,
/// `#808080`, `#00ff00`.
#[must_use]
pub fn lime_columns() -> String {
    model((4, 4, 1), SCALE, &[BLACK, DIM, GREY, LIME], &|x, _, _| {
        [BLACK, DIM, GREY, LIME]
            .get(usize::try_from(x).ok()?)
            .copied()
    })
}

/// A solid `[1, 1, 1]` model declaring `scale = 1`.
///
/// One pixel per voxel makes a one-pixel image, whose axes have no interior
/// adjacent pair at all — a maximum over an empty set, which is the shape that
/// panics or answers with a sentinel.
#[must_use]
pub fn single_voxel() -> String {
    model((1, 1, 1), 1, &[GREY], &|_, _, _| Some(GREY))
}

/// The staircase: a `[4, 4, 1]` slab whose **top** voxel row runs black, white,
/// black, white, whose **bottom** row runs black, grey, grey, white, and whose
/// two middle rows are uniformly grey.
///
/// Image rows 0–7 are the top voxel row and rows 24–31 the bottom one, so the
/// failing row this reports is 24.
#[must_use]
pub fn staircase_rows() -> String {
    model((4, 4, 1), SCALE, &[BLACK, GREY, WHITE], &|x, y, _| {
        Some(match y {
            3 => extremes(x),
            0 => shallow(x),
            _ => GREY,
        })
    })
}

/// The staircase transposed: the same pattern read down columns instead of
/// across rows.
///
/// Image columns 0–7 are the `x = 0` voxel column and columns 24–31 the `x = 3`
/// one, so the failing column this reports is 24.
#[must_use]
pub fn staircase_columns() -> String {
    model((4, 4, 1), SCALE, &[BLACK, GREY, WHITE], &|x, y, _| {
        Some(match x {
            0 => extremes(3 - y.min(3)),
            3 => shallow(3 - y.min(3)),
            _ => GREY,
        })
    })
}

/// The `n`th step of the four-step gradient.
fn column(step: u32) -> Option<Tone> {
    GRADIENT.get(usize::try_from(step).ok()?).copied()
}

/// Black, white, black, white — adjacent steps of 255, and a wrap of 255.
fn extremes(at: u32) -> Tone {
    if at.is_multiple_of(2) { BLACK } else { WHITE }
}

/// Black, grey, grey, white — adjacent steps of at most 128, and a wrap of 255.
fn shallow(at: u32) -> Tone {
    match at {
        0 => BLACK,
        3 => WHITE,
        _ => GREY,
    }
}

/// A `[4, 4, 4]` cube of one grey carrying one marker voxel of a distinct
/// colour per face.
///
/// **A solid uniform cube emits six identical images**, so a fixture without
/// markers is satisfied by an implementation that renders `front` six times.
/// Each marker has exactly one coordinate at an extreme and the other two
/// interior, so no voxel lies on two faces, every boundary row and column stays
/// grey, and the seam legs are unaffected by any of them.
#[must_use]
pub fn marker_cube() -> String {
    let mut palette = vec![GREY];
    palette.extend(MARKERS.iter().map(|(tone, _)| *tone));
    model((4, 4, 4), SCALE, &palette, &|x, y, z| {
        Some(marker(x, y, z).unwrap_or(GREY))
    })
}

/// The marker at that voxel, where one sits there.
fn marker(x: u32, y: u32, z: u32) -> Option<Tone> {
    MARKERS
        .iter()
        .find(|(_, at)| *at == (x, y, z))
        .map(|(tone, _)| *tone)
}
