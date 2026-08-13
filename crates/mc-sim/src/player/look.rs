//! Where the player is looking, and what one tick's look deltas do to it.
//!
//! Yaw and pitch are treated differently on purpose, and the difference is not
//! symmetry for its own sake: yaw is a direction on a circle, so growing past a
//! full turn is the same direction and wrapping loses nothing, while pitch past
//! the vertical would flip the world's up axis, so it is clamped short of it.

use std::f32::consts::TAU;

use crate::player::{MovementIntent, PlayerState};

/// How far from level the view may tilt, in degrees.
///
/// Short of the vertical rather than at it: on the world's up axis a look
/// direction has no horizontal component and a view matrix has no unique
/// answer, where one degree short of it the horizontal component is
/// `cos 89° = 0.0175` — small, but not a degenerate case.
const PITCH_LIMIT_DEGREES: f32 = 89.0;

/// The two angles a player's view is accumulated into, in radians.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Look {
    /// Wrapped into `[0, 2π)`. 0 faces +x and +π/2 faces +z.
    pub yaw: f32,
    /// Clamped to ±89°. Positive looks up.
    pub pitch: f32,
}

impl Look {
    /// Which way `state` is facing.
    ///
    /// The two angles are carried on the player state rather than as a `Look`
    /// field, because a snapshot's shape is the seam three crates read and a
    /// nested type there would be a conversion at every reader. This is the
    /// other direction of that trade, and it is the only place the pair is
    /// lifted out.
    #[must_use]
    pub const fn of(state: &PlayerState) -> Self {
        Self {
            yaw: state.yaw,
            pitch: state.pitch,
        }
    }

    /// Where one intent's look deltas leave this orientation.
    ///
    /// A delta that is not a finite number on *either* axis leaves *both* angles
    /// untouched: a NaN reaching either accumulator would poison the player's
    /// view for the rest of the run, and there is no half of that worth keeping.
    #[must_use]
    pub fn accumulate(self, intent: &MovementIntent) -> Self {
        if !intent.yaw_delta.is_finite() || !intent.pitch_delta.is_finite() {
            return self;
        }
        let limit = PITCH_LIMIT_DEGREES.to_radians();
        Self {
            yaw: wrapped(self.yaw + intent.yaw_delta),
            pitch: (self.pitch + intent.pitch_delta).clamp(-limit, limit),
        }
    }
}

/// `yaw` brought back inside one turn.
///
/// The guard is not belt-and-braces: `rem_euclid` answers a full turn exactly
/// for a yaw a hair below zero, because adding a value far under one ulp of 2π
/// rounds to 2π — which is outside the half-open range this is the whole
/// definition of.
fn wrapped(yaw: f32) -> f32 {
    let inside = yaw.rem_euclid(TAU);
    if inside < TAU { inside } else { 0.0 }
}
