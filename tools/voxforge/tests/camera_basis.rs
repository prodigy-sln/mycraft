//! The declared vectors, asserted directly.
//!
//! Additional coverage: no scenario in this spec states the camera basis, and
//! every other test in the phase reads it only through pixels. Ten views come
//! out of **one** formula, so a reversed cross product or a mistyped direction
//! breaks several views at once and arrives as a handful of unrelated-looking
//! pixel failures. Asserting the frame itself says which vector is wrong, in one
//! line, before any of them reaches a raster.
//!
//! The expected values are `architecture.md` D4's and the spec's four
//! under-corner rows, transcribed from the mathematics rather than from a run.
//! D4 hand-verifies `front` and `top`; both were re-derived here before being
//! written down (`front`:
//! `d × w = (0,0,−1) × (0,1,0) = (1,0,0)`, so `right` is `+x` and
//! `up = (1,0,0) × (0,0,−1) = (0,1,0)`; `top`: `right` is `+x` and `up` is `−z`,
//! which is what puts `z = 0` at the top of a plan view), and the other eight
//! were derived the same way.
//!
//! **`up` is up in the *model*.** Row 0 is the top of the image, so a larger
//! component along `up` means a *lower* row index. Nothing here asserts
//! otherwise; the row indices themselves are graded in `preview_orientation.rs`.
//!
//! **Why this is split by view class, and it is not a split for tidiness.** The isometric
//! elevation is a deliberately open question — true isometric against 2:1
//! dimetric — and it lives in the `y` component of the four corner directions.
//! Pinning those vectors exactly would pass today and go **red against a correct
//! camera** the day that question is answered, whose cheapest fix is to change
//! the camera back. So:
//!
//! - The six axis views carry no elevation at all and are pinned exactly.
//! - The eight corner views are pinned by what the elevation cannot move:
//!   `right` is exactly `(±1, 0, ±1)/√2` at every elevation, because
//!   `w = (0,1,0)` and `right = normalize(d × w)` drops the `y` component
//!   entirely. Everything else about a corner is asserted as a sign, a length or
//!   a right angle.
//!
//! **The same property costs something, and it is worth knowing where.** Because
//! `right` cannot see `y`, each corner and its under twin share it *exactly* —
//! and they also share orthonormality, the diagonal check and `up.y > 0`. The
//! sign of `direction.y` is the only declared value that differs between them,
//! so it alone separates `iso-fl` from `iso-fl-under`. See `Corner::looks_y`.
//!
//! A third test asserts the two tables are **complete** against `View::ALL`,
//! because a view with no row here is not a failure of either table — it is an
//! absence, and an absent assertion looks exactly like a passing one.
//!
//! **On the tolerance, which was measured rather than guessed.** A temporary
//! probe reported the largest deviation across the whole table as **exactly
//! zero**: in `f64` this derivation reproduces the literals below bit for bit.
//! That is emphatically not a reason to compare bits — this project once had an
//! exact comparison of two camera vectors fail against a **correct** camera,
//! because a value a hair off π made its sine 8.7e−8 rather than 0. Nothing in
//! D4 promises an association order, so writing `up = right × d` as an
//! equivalent expression, or a `glam` release changing its SIMD path, moves a
//! component by an ulp without moving the mathematics. The bound is derived from
//! both ends: above an ulp at this magnitude (2.2e−16), and below the smallest
//! difference it must still catch (`1/√2 ≈ 0.7071` against `2/√6 ≈ 0.8165`,
//! 0.109 apart). **1e−9** sits seven orders above the first and eight below the
//! second.

mod common;

use std::f64::consts::FRAC_1_SQRT_2;

use common::TestResult;
use voxforge::render::View;
use voxforge::render::camera::{Basis, basis_of};

/// `1/√2`, the horizontal component of every corner view's `right`.
///
/// The standard library's own, because spelling the decimal out is the one
/// thing clippy stops here — and it is the same value either way, being the
/// nearest `f64` to `1/√2`.
const ROOT2_INV: f64 = FRAC_1_SQRT_2;

/// Seven orders above an ulp at this magnitude, eight below the smallest
/// difference the table must distinguish. See the note above.
const TOLERANCE: f64 = 1e-9;

/// How far from zero a component must be before its sign means anything.
///
/// Every component a corner view's frame carries stays above this across any
/// plausible elevation: the horizontal components of `direction` are
/// `1/√(2 + k²)`, which is 0.41 at true isometric and does not fall below 0.3
/// anywhere near it.
const DEFINITE: f64 = 0.05;

/// One row of D4's table for a view whose frame is fully determined.
struct Axial {
    /// The view the row is about.
    view: View,
    /// The direction it looks along.
    direction: [f64; 3],
    /// One column further right in its image.
    right: [f64; 3],
    /// One row further up in the model, which is one row *lower* in index.
    up: [f64; 3],
}

/// The six views that carry no elevation, and so are pinned exactly.
const AXIAL: [Axial; 6] = [
    Axial {
        view: View::Front,
        direction: [0.0, 0.0, -1.0],
        right: [1.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0],
    },
    Axial {
        view: View::Back,
        direction: [0.0, 0.0, 1.0],
        right: [-1.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0],
    },
    Axial {
        view: View::Left,
        direction: [1.0, 0.0, 0.0],
        right: [0.0, 0.0, 1.0],
        up: [0.0, 1.0, 0.0],
    },
    Axial {
        view: View::Right,
        direction: [-1.0, 0.0, 0.0],
        right: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
    },
    Axial {
        view: View::Top,
        direction: [0.0, -1.0, 0.0],
        right: [1.0, 0.0, 0.0],
        up: [0.0, 0.0, -1.0],
    },
    Axial {
        view: View::Bottom,
        direction: [0.0, 1.0, 0.0],
        right: [1.0, 0.0, 0.0],
        up: [0.0, 0.0, 1.0],
    },
];

/// One row of D4's table for a corner view, whose elevation is still open.
struct Corner {
    /// The view the row is about.
    view: View,
    /// Its `right`, which no elevation can move.
    ///
    /// **Shared with its twin.** `right = normalize(d × w)` with `w = (0,1,0)`
    /// drops the `y` component, so `iso-fl` and `iso-fl-under` — the same
    /// horizontal corner from opposite sides — have byte-identical `right`
    /// vectors. This field cannot tell a pair apart, and neither can
    /// orthonormality, the diagonal check or `up.y`.
    right: [f64; 3],
    /// Which way along `x` it looks: `+1` or `−1`.
    looks_x: f64,
    /// Which way along `z` it looks.
    looks_z: f64,
    /// Whether it looks down at the model (`−1`) or up at it (`+1`).
    ///
    /// **This is the only thing separating a corner from its under twin**, and
    /// so the most load-bearing single value in this table. Every other field a
    /// pair shares. Before the under-corners existed it was the constant `−1`
    /// and read as a sanity check that a corner descends; it is now the field
    /// that says *which view this is*, and a sign typo in one of the eight rows
    /// — or in `direction_of` — is caught by exactly one assertion in the whole
    /// suite.
    looks_y: f64,
}

/// The eight corner views, pinned by what the elevation cannot move.
///
/// Four horizontal corners, each seen from above and from below. The pairs
/// differ in exactly one declared value.
const CORNERS: [Corner; 8] = [
    Corner {
        view: View::IsoFl,
        right: [ROOT2_INV, 0.0, ROOT2_INV],
        looks_x: 1.0,
        looks_z: -1.0,
        looks_y: -1.0,
    },
    Corner {
        view: View::IsoFr,
        right: [ROOT2_INV, 0.0, -ROOT2_INV],
        looks_x: -1.0,
        looks_z: -1.0,
        looks_y: -1.0,
    },
    Corner {
        view: View::IsoBl,
        right: [-ROOT2_INV, 0.0, ROOT2_INV],
        looks_x: 1.0,
        looks_z: 1.0,
        looks_y: -1.0,
    },
    Corner {
        view: View::IsoBr,
        right: [-ROOT2_INV, 0.0, -ROOT2_INV],
        looks_x: -1.0,
        looks_z: 1.0,
        looks_y: -1.0,
    },
    Corner {
        view: View::IsoFlUnder,
        right: [ROOT2_INV, 0.0, ROOT2_INV],
        looks_x: 1.0,
        looks_z: -1.0,
        looks_y: 1.0,
    },
    Corner {
        view: View::IsoFrUnder,
        right: [ROOT2_INV, 0.0, -ROOT2_INV],
        looks_x: -1.0,
        looks_z: -1.0,
        looks_y: 1.0,
    },
    Corner {
        view: View::IsoBlUnder,
        right: [-ROOT2_INV, 0.0, ROOT2_INV],
        looks_x: 1.0,
        looks_z: 1.0,
        looks_y: 1.0,
    },
    Corner {
        view: View::IsoBrUnder,
        right: [-ROOT2_INV, 0.0, -ROOT2_INV],
        looks_x: -1.0,
        looks_z: 1.0,
        looks_y: 1.0,
    },
];

/// Whether a view has a row in exactly one of the tables above.
#[derive(Debug, PartialEq, Eq)]
enum Coverage {
    /// Exactly one table declares it.
    Graded,
    /// Neither does, so nothing above asserts anything about it at all.
    Ungraded,
    /// Both do, so which one grades it depends on which test runs.
    GradedTwice,
}

/// How one view's frame compares with what it declares.
#[derive(Debug, PartialEq)]
enum Frame {
    /// Everything the view declares holds.
    AsDeclared,
    /// A vector the view pins exactly is not that vector.
    Off {
        /// Which of the three.
        axis: &'static str,
        /// What D4 declares.
        declared: [f64; 3],
        /// What the derivation produced.
        derived: [f64; 3],
    },
    /// The frame is not three unit vectors at right angles.
    NotOrthonormal {
        /// Which quantity was wrong.
        what: &'static str,
        /// What it came to, against an expected 1 or 0.
        value: f64,
    },
    /// A component's sign is not the one declared, or is too near zero to have
    /// one.
    WrongWay {
        /// Which component.
        what: &'static str,
        /// What it came to.
        value: f64,
    },
    /// A corner does not look equally along `x` and `z`, so it is not a corner.
    NotDiagonal {
        /// Its `x` component.
        x: f64,
        /// Its `z` component.
        z: f64,
    },
}

/// Whether `derived` is `declared` to within [`TOLERANCE`].
fn matches(declared: [f64; 3], derived: [f64; 3]) -> bool {
    declared
        .iter()
        .zip(derived.iter())
        .all(|(declared, derived)| (declared - derived).abs() < TOLERANCE)
}

/// Whether the vector `axis` is the one declared.
fn pinned(axis: &'static str, declared: [f64; 3], derived: [f64; 3]) -> Option<Frame> {
    (!matches(declared, derived)).then_some(Frame::Off {
        axis,
        declared,
        derived,
    })
}

/// Whether the frame is three unit vectors at right angles to one another.
///
/// Asserted for every view rather than only the corners: it is what makes the
/// corner rows a complete description despite pinning only one of their three
/// vectors, since `up` is then forced up to a sign.
fn orthonormal(basis: &Basis) -> Option<Frame> {
    let measured = [
        ("|direction|", basis.direction.length(), 1.0),
        ("|right|", basis.right.length(), 1.0),
        ("|up|", basis.up.length(), 1.0),
        ("direction · right", basis.direction.dot(basis.right), 0.0),
        ("direction · up", basis.direction.dot(basis.up), 0.0),
        ("right · up", basis.right.dot(basis.up), 0.0),
    ];
    measured
        .into_iter()
        .find(|(_, value, expected)| (value - expected).abs() >= TOLERANCE)
        .map(|(what, value, _)| Frame::NotOrthonormal { what, value })
}

/// Whether `value` runs the way `declared` does, and definitely enough to say.
fn runs(what: &'static str, value: f64, declared: f64) -> Option<Frame> {
    (value * declared < DEFINITE).then_some(Frame::WrongWay { what, value })
}

/// How the frame derived for an axis view compares with its row.
fn axial_frame(row: &Axial) -> Frame {
    let basis = basis_of(row.view);
    pinned("direction", row.direction, basis.direction.to_array())
        .or_else(|| pinned("right", row.right, basis.right.to_array()))
        .or_else(|| pinned("up", row.up, basis.up.to_array()))
        .or_else(|| orthonormal(&basis))
        .unwrap_or(Frame::AsDeclared)
}

/// How the frame derived for a corner view compares with its row.
///
/// `right` is the only vector pinned outright. The rest is everything the
/// elevation leaves fixed: the look descends, it runs the declared way along
/// both horizontal axes and equally along the two of them, and `up` points
/// upward — which, with the frame orthonormal and `right` known, leaves `up`
/// no freedom at all.
fn corner_frame(row: &Corner) -> Frame {
    let basis = basis_of(row.view);
    let direction = basis.direction.to_array();
    pinned("right", row.right, basis.right.to_array())
        .or_else(|| orthonormal(&basis))
        .or_else(|| {
            runs(
                "direction y (above the model or below it)",
                basis.direction.y,
                row.looks_y,
            )
        })
        .or_else(|| runs("up y (up is up)", basis.up.y, 1.0))
        .or_else(|| runs("direction x", basis.direction.x, row.looks_x))
        .or_else(|| runs("direction z", basis.direction.z, row.looks_z))
        .or_else(|| diagonal(direction))
        .unwrap_or(Frame::AsDeclared)
}

/// Whether a corner looks equally far along `x` and `z`.
fn diagonal(direction: [f64; 3]) -> Option<Frame> {
    let (x, z) = (direction.first().copied(), direction.get(2).copied());
    match (x, z) {
        (Some(x), Some(z)) if (x.abs() - z.abs()).abs() < TOLERANCE => None,
        (Some(x), Some(z)) => Some(Frame::NotDiagonal { x, z }),
        _ => Some(Frame::NotDiagonal {
            x: f64::NAN,
            z: f64::NAN,
        }),
    }
}

/// Which table, if either, declares `view`.
fn coverage_of(view: View) -> Coverage {
    let declared = AXIAL.iter().filter(|row| row.view == view).count()
        + CORNERS.iter().filter(|row| row.view == view).count();
    match declared {
        1 => Coverage::Graded,
        0 => Coverage::Ungraded,
        _ => Coverage::GradedTwice,
    }
}

/// The positive control the two tables above have never had.
///
/// Both other tests iterate their **own** table, so a view added to `View` and
/// to nothing else is not a failure — it is an absence, and an absent assertion
/// and a passing one look identical from the outside. That is not hypothetical:
/// the four under-corners arrived with deliberately wrong directions and the
/// whole suite stayed green at 97/97, because a view that looks the wrong way
/// still renders, still tiles into a sheet and still round-trips through its own
/// name. Nothing but a table of vectors can see it, and a table with no row for
/// a view sees nothing either.
///
/// So this asserts the tables are *complete*, against the only authority on what
/// views exist. `GradedTwice` is the other half: a view in both tables would be
/// graded by whichever test happened to run, which is a coin toss dressed as
/// coverage.
#[test]
fn every_view_the_tool_offers_has_a_row_in_exactly_one_of_these_tables() -> TestResult {
    let covered: Vec<(&str, Coverage)> = View::ALL
        .iter()
        .map(|view| (view.as_str(), coverage_of(*view)))
        .collect();

    assert_eq!(
        covered,
        View::ALL
            .iter()
            .map(|view| (view.as_str(), Coverage::Graded))
            .collect::<Vec<_>>(),
        "a view nothing here declares is a view nothing here checks, and it goes on looking exactly like a view that passed"
    );
    Ok(())
}

#[test]
fn every_axis_view_derives_the_three_vectors_its_own_row_of_the_table_declares() -> TestResult {
    let derived: Vec<(&str, Frame)> = AXIAL
        .iter()
        .map(|row| (row.view.as_str(), axial_frame(row)))
        .collect();

    assert_eq!(
        derived,
        AXIAL
            .iter()
            .map(|row| (row.view.as_str(), Frame::AsDeclared))
            .collect::<Vec<_>>(),
        "six views are one formula over six declared directions, so a wrong one is a wrong row rather than a wrong renderer"
    );
    Ok(())
}

#[test]
fn every_corner_view_derives_the_frame_its_row_declares_at_any_elevation() -> TestResult {
    let derived: Vec<(&str, Frame)> = CORNERS
        .iter()
        .map(|row| (row.view.as_str(), corner_frame(row)))
        .collect();

    assert_eq!(
        derived,
        CORNERS
            .iter()
            .map(|row| (row.view.as_str(), Frame::AsDeclared))
            .collect::<Vec<_>>(),
        "which corner a view looks from is settled; how steeply it looks down is not, and nothing here may decide that by accident"
    );
    Ok(())
}
