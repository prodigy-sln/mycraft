//! Whether a texture may be tiled across a block face.
//!
//! The answer is a **total** verdict over three ordered legs — period, then
//! coverage, then edges — never a boolean and never an absence. `is_empty()`
//! cannot tell an answer of "nothing is wrong" from a check that can no longer
//! look, and the whole of this module exists so that one enumerated value
//! rejects every other answer including that one.
//!
//! The edge metric is **integer, self-calibrating and compared per row**: the
//! difference between two adjacent pixels is the greatest absolute per-channel
//! difference between them, and an axis fails exactly where a row's
//! last-to-first difference exceeds that same row's greatest interior
//! adjacent-pair difference. No threshold is declared and none is snapshotted.

use std::fmt;

use crate::format::Axis;
use crate::render::{Pixel, Preview};

/// Where one pixel sits in an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PixelPos {
    /// How far across the image it sits, counted from the left.
    pub column: u32,
    /// How far down the image it sits, counted from the top.
    pub row: u32,
}

/// Which line of an image an edge failure was measured along.
///
/// Named rather than a bare index, because the same number means two different
/// things: the horizontal axis is measured across each row and the vertical one
/// down each column, and for the plan views neither is the `y` axis at all —
/// `top` runs its columns along `x` and its rows along `z`. Nothing about the
/// model axis alone says which kind of line failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Line {
    /// That image row, counted from the top.
    Row(u32),
    /// That image column, counted from the left.
    Column(u32),
}

impl fmt::Display for Line {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Row(at) => write!(formatter, "row {at}"),
            Self::Column(at) => write!(formatter, "column {at}"),
        }
    }
}

/// What the seam question answered about one texture.
///
/// One variant per leg, plus the passing answer. A face carries at least one of
/// these and, on the reported path, one per failing leg in the declared order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeamVerdict {
    /// Fit to be a block texture.
    TilesAcrossEveryEdge,
    /// The model is not exactly `scale` voxels across one of the view's two
    /// in-plane axes, so the texture's period cannot be the block grid's.
    PeriodIsNotOneBlock {
        /// The in-plane model axis at fault.
        axis: Axis,
        /// How many voxels the assembled model spans along it.
        voxels: u32,
        /// How many the document declares to a block.
        scale: u32,
    },
    /// Some pixel no voxel covers would show the void on a solid block face.
    FaceIsNotOpaque {
        /// How many pixels of the image are not fully opaque.
        transparent: u32,
        /// The first of them, scanning row-major from row 0, column 0.
        first: PixelPos,
    },
    /// The wrap introduces a larger step than the texture already contains
    /// within the same row.
    EdgesDisagree {
        /// The in-plane model axis at fault.
        axis: Axis,
        /// The first line of the image along which it fails.
        at: Line,
        /// The step from that line's last pixel to its first.
        across: u8,
        /// The largest step between two adjacent pixels within it.
        largest_within: u8,
    },
}

impl fmt::Display for SeamVerdict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TilesAcrossEveryEdge => write!(formatter, "tiles across every edge"),
            Self::PeriodIsNotOneBlock {
                axis,
                voxels,
                scale,
            } => write!(
                formatter,
                "the {axis} axis is {voxels} voxels across, where the declared scale of {scale} makes the texture's period one block",
                axis = axis.as_str()
            ),
            Self::FaceIsNotOpaque { transparent, first } => write!(
                formatter,
                "{transparent} pixel(s) are transparent, the first at row {row}, column {column}",
                row = first.row,
                column = first.column
            ),
            Self::EdgesDisagree {
                axis,
                at,
                across,
                largest_within,
            } => write!(
                formatter,
                "the edges disagree along the {axis} axis: {at} steps {across} across the wrap, where its largest step within is {largest_within}",
                axis = axis.as_str()
            ),
        }
    }
}

/// Which model axis a face's image runs along, and how far the model reaches
/// along it.
pub(super) struct InPlane {
    /// The model axis.
    pub axis: Axis,
    /// How many voxels the assembled model spans along it.
    pub voxels: u32,
}

/// Every leg's answer for one texture, in the declared order.
///
/// Never empty: a texture fit to be tiled answers with exactly one
/// [`SeamVerdict::TilesAcrossEveryEdge`].
pub(super) fn judge(
    image: &Preview,
    across: &InPlane,
    down: &InPlane,
    scale: u32,
) -> Vec<SeamVerdict> {
    let mut failures: Vec<SeamVerdict> = Vec::new();
    failures.extend(period(across, scale));
    failures.extend(period(down, scale));
    failures.extend(opacity(image));
    failures.extend(edges(image, across.axis, down.axis));
    if failures.is_empty() {
        return vec![SeamVerdict::TilesAcrossEveryEdge];
    }
    failures
}

/// Leg one: the model must be exactly one block across each in-plane axis.
fn period(reach: &InPlane, scale: u32) -> Option<SeamVerdict> {
    if reach.voxels == scale {
        return None;
    }
    Some(SeamVerdict::PeriodIsNotOneBlock {
        axis: reach.axis,
        voxels: reach.voxels,
        scale,
    })
}

/// Leg two: every pixel of a block face must be opaque.
///
/// Scanned row-major from row 0, column 0, because the verdict names a *first*
/// pixel and "first" is otherwise undefined.
fn opacity(image: &Preview) -> Option<SeamVerdict> {
    let holes: Vec<PixelPos> = (0..image.height())
        .flat_map(|row| (0..image.width()).map(move |column| PixelPos { column, row }))
        .filter(|at| !covered(image, *at))
        .collect();
    Some(SeamVerdict::FaceIsNotOpaque {
        transparent: u32::try_from(holes.len()).unwrap_or(u32::MAX),
        first: *holes.first()?,
    })
}

/// Whether a voxel landed on this pixel.
fn covered(image: &Preview, at: PixelPos) -> bool {
    image
        .pixel(at.column, at.row)
        .is_some_and(|pixel| pixel.alpha == 255)
}

/// Leg three: the wrap may introduce no step the line does not already contain.
///
/// Per line rather than per image, and the difference matters: a maximum over
/// the whole image lets an extreme step *anywhere* license a discontinuity
/// *anywhere else*, so a texture with a black-to-white interior edge in its top
/// row and a seam in its bottom row would score 255 against 255 and pass while
/// showing a visible grid.
fn edges(image: &Preview, across: Axis, down: Axis) -> Vec<SeamVerdict> {
    let mut found = Vec::new();
    for row in 0..image.height() {
        let line: Vec<Pixel> = (0..image.width())
            .filter_map(|at| image.pixel(at, row))
            .collect();
        if let Some(fault) = disagreement(&line, across, Line::Row(row)) {
            found.push(fault);
            break;
        }
    }
    for column in 0..image.width() {
        let line: Vec<Pixel> = (0..image.height())
            .filter_map(|at| image.pixel(column, at))
            .collect();
        if let Some(fault) = disagreement(&line, down, Line::Column(column)) {
            found.push(fault);
            break;
        }
    }
    found
}

/// Whether one line's wrap steps further than anything within it.
///
/// A line one pixel long has no interior pair, so `largest_within` is a maximum
/// over an empty set — 0 — and `across` is the step from a pixel to itself, also
/// 0. It tiles, which is correct: a single column repeated is seamless by
/// definition. That case is the shape that panics or returns a sentinel, and it
/// is why it is written to answer rather than to unwrap.
fn disagreement(line: &[Pixel], axis: Axis, at: Line) -> Option<SeamVerdict> {
    let (first, last) = (line.first()?, line.last()?);
    let across = step(*last, *first);
    let largest_within = line
        .windows(2)
        .filter_map(|pair| Some(step(*pair.first()?, *pair.get(1)?)))
        .max()
        .unwrap_or(0);
    if across <= largest_within {
        return None;
    }
    Some(SeamVerdict::EdgesDisagree {
        axis,
        at,
        across,
        largest_within,
    })
}

/// How far apart two pixels are: the greatest absolute per-channel difference.
///
/// Alpha is not a channel here. Leg two has already established that a texture
/// this leg can pass is fully opaque, so on the binding path every alpha is
/// 255 and including it would change nothing; on the reported path, where
/// transparent pixels survive to be measured, counting alpha would let the
/// *coverage* defect leg two already named surface a second time as an edge
/// one. One defect, one verdict.
fn step(left: Pixel, right: Pixel) -> u8 {
    let channels = [
        left.red.abs_diff(right.red),
        left.green.abs_diff(right.green),
        left.blue.abs_diff(right.blue),
    ];
    channels.into_iter().max().unwrap_or(0)
}
