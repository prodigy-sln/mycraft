//! Where a view looks from, and which way round its image runs.
//!
//! **Ten views are one derivation and eleven declared vectors, never ten
//! hand-written bases.** Given a unit view direction `d` and an up hint `w`:
//!
//! ```text
//! right = normalize(d × w)
//! up    = right × d
//! ```
//!
//! `w` is `(0, 1, 0)` everywhere except the two plan views, where `d` is
//! parallel to it and a cross product would be zero.
//!
//! The precedent is the mesher, where every per-facing fact derives from one
//! exhaustive match because "a swapped neighbour slot and a reordered emission
//! are the same mistake, and they fail together rather than independently". The
//! same holds here: a sign flip in this formula moves every view at once, which
//! is a defect a fixture can catch. Ten hand-written bases would let one view be
//! wrong on its own.

use glam::DVec3;

use crate::render::View;

/// The axes one view's image runs along.
#[derive(Debug, Clone, Copy)]
pub struct Basis {
    /// The direction the camera looks along.
    pub direction: DVec3,
    /// The direction one column further right in the image.
    pub right: DVec3,
    /// The direction one row further *up* — which is one row *lower* in index,
    /// since row 0 is the top.
    pub up: DVec3,
}

/// The basis `view` sees the model through.
#[must_use]
pub fn basis_of(view: View) -> Basis {
    let direction = direction_of(view);
    let hint = up_hint(view);
    let right = direction.cross(hint).normalize();
    Basis {
        direction,
        right,
        up: right.cross(direction),
    }
}

/// The unit direction `view` looks along.
///
/// The four corner directions carry `−1` on `y` because every isometric view
/// looks *down* at the model. Whether that descent is true isometric or 2:1
/// dimetric is a deferred question, and deliberately does not live here: it
/// changes only the `y` component's magnitude, and `right` drops that component
/// entirely, so the horizontal axis of every corner view is settled either way.
fn direction_of(view: View) -> DVec3 {
    match view {
        View::Front => DVec3::new(0.0, 0.0, -1.0),
        View::Back => DVec3::new(0.0, 0.0, 1.0),
        View::Left => DVec3::new(1.0, 0.0, 0.0),
        View::Right => DVec3::new(-1.0, 0.0, 0.0),
        View::Top => DVec3::new(0.0, -1.0, 0.0),
        View::Bottom => DVec3::new(0.0, 1.0, 0.0),
        View::IsoFl => DVec3::new(1.0, -1.0, -1.0).normalize(),
        View::IsoFr => DVec3::new(-1.0, -1.0, -1.0).normalize(),
        View::IsoBl => DVec3::new(1.0, -1.0, 1.0).normalize(),
        View::IsoBr => DVec3::new(-1.0, -1.0, 1.0).normalize(),
        // The same four horizontal corners, looking *up*. Only the `y` sign
        // differs from the four above, and that one sign is the whole of the
        // difference: `right` cannot see `y` at all, so each under-corner's
        // `right` is byte-identical to its overhead twin's. A typo here is
        // caught by exactly one assertion in the suite.
        View::IsoFlUnder => DVec3::new(1.0, 1.0, -1.0).normalize(),
        View::IsoFrUnder => DVec3::new(-1.0, 1.0, -1.0).normalize(),
        View::IsoBlUnder => DVec3::new(1.0, 1.0, 1.0).normalize(),
        View::IsoBrUnder => DVec3::new(-1.0, 1.0, 1.0).normalize(),
    }
}

/// Which way is up, as far as `view` is concerned.
///
/// The two plan views look straight along the world's up axis, so they need a
/// hint that is not parallel to it. `top` takes `−z`, which puts `z = 0` at the
/// top of the image and agrees with the `slice = "y"` convention that prints
/// `z = 0` first — an agreement that is a *consequence* of this formula rather
/// than a second rule anybody has to maintain.
fn up_hint(view: View) -> DVec3 {
    match view {
        View::Top => DVec3::new(0.0, 0.0, -1.0),
        View::Bottom => DVec3::new(0.0, 0.0, 1.0),
        _ => DVec3::new(0.0, 1.0, 0.0),
    }
}
