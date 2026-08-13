//! The declared voxel worlds that have walls, ceilings and corners in them.
//!
//! [`super::solidity::Ground`] answers about *columns*: it says how high the
//! floor is at an `x` and knows nothing else, which is exactly what a scenario
//! about falling and standing needs and exactly what a scenario about walking
//! into a wall cannot use. This module is the other half — a world is declared
//! as the union of a handful of rectangular runs of solid voxels, so a test can
//! say "a floor, and one wall at x = 13" and have every expected position be
//! arithmetic over those two sentences.
//!
//! It is deliberately a **second** type rather than a variant added to `Ground`.
//! Two scenarios already in the suite are stated against `Ground::Void` and
//! `Ground::Flat` and depend on what those shapes do *not* contain — most
//! sharply the displacement bound's, whose 300-blocks-per-second rise is only
//! unresolved because nothing is above it. Widening the existing enum is how
//! that would quietly change from measuring the bound to measuring whichever of
//! the bound and a ceiling came first, with every assertion still green.
//!
//! **A voxel occupies `[v, v + 1)` on each axis**, which is what every position
//! in this feature's collision scenarios is derived from: the wall of voxels at
//! `x = 13` presents its near face at `x = 13.0`, so a box reaching
//! `HALF_WIDTH` either side of the feet stops with its feet at `12.7`. The wall
//! of voxels at `x = 7` presents its *far* face at `x = 8.0`, and a box arriving
//! from the other side stops at `8.3`. Both figures are the half-width
//! subtracted from, and added to, a declared integer — which is what makes the
//! player's 0.6-block width falsifiable here rather than carried by derivation.
//!
//! **What these shapes would fail to catch if they were built differently.**
//! A wall is **one voxel thick**, never a half-space, so a resolver that snapped
//! the box to the wrong face of the blocking voxel puts it *inside* or *beyond*
//! the wall rather than landing on the same answer a half-space would have
//! given. A wall spans the axes it is not about, so a walk parallel to it never
//! runs off an end and no test's answer depends on where a fixture stops.
//! [`Slab::voxel`] declares a **single** voxel, which is the only shape that can
//! discriminate the order two axes are resolved in: anything symmetric about the
//! diagonal gives the same answer whichever axis moved first.

use mc_sim::player::{BlockPos, Solidity};

/// How far a floor, a wall or a ceiling runs on the axes it is not about.
///
/// Far enough on either side that no walk in this suite reaches an end of one,
/// and finite so that no bound of a declared region is an `i32` extreme.
const SPAN: (i32, i32) = (-64, 192);

/// How high a wall reaches. Any value above the player's box would do; this one
/// is the same order as the world's own height so a wall reads as a wall.
const WALL_TOP: i32 = 192;

/// A rectangular run of solid voxels, half-open on every axis in voxel
/// coordinates: `x` covers `x.0` up to but not including `x.1`.
#[derive(Debug, Clone, Copy)]
pub struct Slab {
    x: (i32, i32),
    y: (i32, i32),
    z: (i32, i32),
}

impl Slab {
    /// Everything from `y = 0` up to and including `surface`, everywhere
    /// horizontally: a floor whose top face is at `surface + 1`.
    #[must_use]
    pub const fn floor(surface: i32) -> Self {
        Self {
            x: SPAN,
            y: (0, surface + 1),
            z: SPAN,
        }
    }

    /// A wall one voxel thick standing in the column `x`, so its near face is at
    /// `x` and its far face at `x + 1`.
    #[must_use]
    pub const fn wall_at_x(x: i32) -> Self {
        Self {
            x: (x, x + 1),
            y: (0, WALL_TOP),
            z: SPAN,
        }
    }

    /// A wall one voxel thick standing in the column `z`.
    #[must_use]
    pub const fn wall_at_z(z: i32) -> Self {
        Self {
            x: SPAN,
            y: (0, WALL_TOP),
            z: (z, z + 1),
        }
    }

    /// A slab one voxel thick lying at height `y`, so its bottom face is at `y`.
    #[must_use]
    pub const fn ceiling_at(y: i32) -> Self {
        Self {
            x: SPAN,
            y: (y, y + 1),
            z: SPAN,
        }
    }

    /// One solid voxel, and nothing around it.
    #[must_use]
    pub const fn voxel(x: i32, y: i32, z: i32) -> Self {
        Self {
            x: (x, x + 1),
            y: (y, y + 1),
            z: (z, z + 1),
        }
    }

    /// Whether this run holds the voxel at `at`.
    const fn holds(self, at: BlockPos) -> bool {
        within(at.x, self.x) && within(at.y, self.y) && within(at.z, self.z)
    }
}

/// Whether `value` lies in the half-open range `range`.
const fn within(value: i32, range: (i32, i32)) -> bool {
    range.0 <= value && value < range.1
}

/// A declared world: the union of the runs it was built from, and nothing else.
///
/// Everything outside every run is not solid, which is what lets a fixture say
/// what it means by listing what is there rather than by describing what is not.
#[derive(Debug, Clone, Default)]
pub struct Chamber(Vec<Slab>);

impl Chamber {
    /// A world holding exactly these runs of solid voxels.
    #[must_use]
    pub fn of(slabs: impl IntoIterator<Item = Slab>) -> Self {
        Self(slabs.into_iter().collect())
    }
}

impl Solidity for Chamber {
    fn is_solid(&self, at: BlockPos) -> bool {
        self.0.iter().any(|slab| slab.holds(at))
    }
}
