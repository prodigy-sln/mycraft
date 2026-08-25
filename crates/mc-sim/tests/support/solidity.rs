//! The declared voxel worlds the physics is asserted against.
//!
//! Every one of them is hand-built and tiny, which is the whole point of the
//! physics reading solidity through an interface: a scenario about where a
//! falling player comes to rest is arithmetic over a surface height this file
//! declares, rather than a 64 × 64 × 256 world's answer that would have to be
//! looked up before the expectation could be written down.
//!
//! **A voxel occupies `[v, v + 1)` on each axis**, so a column whose topmost
//! solid voxel is `h` presents its top face at `y = h + 1` — which is where feet
//! come to rest, and is the specification's "surface height `h`, feet at
//! `h + 1`". Below `y = 0` nothing is solid, so a player that falls out of the
//! world keeps falling.
//!
//! **The shape of a fixture is a constraint no assertion can enforce**, so each
//! one is chosen for what it would fail to catch if it were built differently:
//!
//! - [`Ground::Flat`] is the only shape that cannot discriminate anything about
//!   *which* voxel was consulted. It is used where the scenario is about the
//!   vertical answer alone.
//! - [`Ground::Ledge`] has an edge, so a player walking toward it loses its
//!   support at a position this file's own geometry predicts — and it predicts
//!   it from the box's trailing face, not from the feet centre, because a box
//!   still overhanging solid ground is still standing on it.
//! - [`Ground::Step`] gives *adjacent* columns different heights, which is what
//!   makes a box straddling both able to tell the taller answer from the
//!   shorter. Built with the same height on both sides it would assert nothing
//!   about the straddle at all.
//!
//! Solidity depends on `x` and never on `z`, deliberately: every fixture is a
//! prism along z. That leaves the two horizontal axes distinguishable by the
//! *tests*, which place the player at an x and a z that differ — so a query that
//! read a box's z where it meant its x lands on the wrong column of a ledge or a
//! step and is caught, rather than being hidden by a fixture symmetric in both.

use mc_sim::player::{BlockPos, Medium, Solidity, VoxelMedium};

/// A declared world, solid from `y = 0` up to and including a per-column surface
/// height.
#[derive(Debug, Clone, Copy)]
pub enum Ground {
    /// Nothing is solid anywhere: the fixture a fall is measured in.
    Void,
    /// Every column solid up to and including `surface`.
    Flat { surface: i32 },
    /// Solid up to and including `surface` west of `edge`, and nothing from
    /// `edge` eastward — a floor that runs out.
    Ledge { edge: i32, surface: i32 },
    /// Two heights meeting at `boundary`: `west` for the columns below it,
    /// `east` for `boundary` and everything above it.
    Step { boundary: i32, west: i32, east: i32 },
}

impl Ground {
    /// The topmost solid voxel of the column at `x`, where there is one.
    fn surface(self, x: i32) -> Option<i32> {
        match self {
            Self::Void => None,
            Self::Flat { surface } => Some(surface),
            Self::Ledge { edge, surface } => (x < edge).then_some(surface),
            Self::Step {
                boundary,
                west,
                east,
            } => Some(if x < boundary { west } else { east }),
        }
    }
}

impl Solidity for Ground {
    fn is_solid(&self, at: BlockPos) -> bool {
        at.y >= 0 && self.surface(at.x).is_some_and(|surface| at.y <= surface)
    }
}

/// **[`VoxelMedium::NOTHING`] unconditionally, and never a function of this
/// fixture's own solidity.**
///
/// The temptation is concrete rather than hypothetical: this fixture computes
/// solidity from a geometric rule, and the negation of that rule *is* the air,
/// so "the air is the medium" is a one-line change that reads as insight. It
/// would put a buoyancy under every held-jump assertion in the suite and a
/// resistance under every assertion about where a box comes to rest — and no
/// assertion written against a fixture can see its own fixture lying.
///
/// What catches it is not a scenario but **the collision suite staying green**,
/// which it does only because a resistance of zero divides by one and is the
/// velocity itself in every bit.
///
/// **Both halves, with no exemption for the resistance.** A resistance derived
/// here looks inert, on the reasoning that a box never overlaps a solid cell —
/// but that is a maintained invariant and not a geometric fact, two rules hold
/// it and neither of them binds a fixture, and the half nothing watches is the
/// half that rots.
impl Medium for Ground {
    fn medium_at(&self, _: BlockPos) -> VoxelMedium {
        VoxelMedium::NOTHING
    }
}
