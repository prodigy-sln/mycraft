//! The camera's pose, as a total function of the tick index.
//!
//! A path that accumulates per-tick deltas looks identical from the outside and
//! is not reproducible: the result then depends on float summation order and on
//! having started at tick 0, which is exactly what a committed golden frame
//! cannot survive. So there is no state here for a delta to accumulate in — no
//! `self`, no field, no accumulator anywhere — and "advance the replay" means
//! increment an index and call this function again.
//!
//! The tick is a validated newtype for the same reason. A guard inside [`pose`]
//! would have to decide what to return for a tick past the end; a constructor
//! that refuses one means the question cannot be asked.

use std::f32::consts::TAU;

use thiserror::Error;

use crate::TICK_COUNT;

/// The point the orbit runs around, and the height it runs at.
const ORBIT_CENTRE: [f32; 3] = [32.0, 56.0, 32.0];

/// How far the eye stands from the orbit's centre.
const ORBIT_RADIUS: f32 = 96.0;

/// What the camera looks at, from wherever it is on the orbit.
const LOOK_AT: [f32; 3] = [32.0, 44.0, 32.0];

/// A tick of this replay, and never one past its end.
///
/// [`pose`] takes one of these rather than a number, so a tick the replay does
/// not have cannot be constructed and therefore cannot be passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TickIndex(u32);

impl TickIndex {
    /// The replay's first tick.
    pub const FIRST: Self = Self(0);

    /// The tick numbered `tick`.
    ///
    /// # Errors
    ///
    /// Returns [`TickError::BeyondReplay`] for a tick at or past the replay's
    /// length, naming that length rather than extrapolating the path.
    pub const fn new(tick: u32) -> Result<Self, TickError> {
        if tick >= TICK_COUNT {
            return Err(TickError::BeyondReplay {
                tick,
                tick_count: TICK_COUNT,
            });
        }
        Ok(Self(tick))
    }

    /// Which tick this is.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Why a tick is not a tick of this replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TickError {
    #[error("tick {tick} is past the end of a replay {tick_count} ticks long")]
    BeyondReplay { tick: u32, tick_count: u32 },
}

/// Where the camera stands and what it looks at.
///
/// Its own type rather than the renderer's `CameraView`: this crate never learns
/// what a view matrix is, and the composition root converts. Three duplicated
/// coordinates are the cheap half of that trade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose {
    pub eye: [f32; 3],
    pub target: [f32; 3],
}

/// The camera's pose at `tick`.
///
/// One multiply and one divide from the index, so the pose at tick 60 is the
/// same value whether it is asked for directly or reached by advancing sixty
/// times. The phase is part of the contract: tick 0 puts the eye at
/// (128, 56, 32) and tick 60 at (-64, 56, 32), and every screen-space
/// expectation this feature verifies against is computed from exactly those two
/// positions.
#[must_use]
pub fn pose(tick: TickIndex) -> CameraPose {
    let theta = TAU * tick.get() as f32 / TICK_COUNT as f32;
    let [centre_x, height, centre_z] = ORBIT_CENTRE;
    CameraPose {
        eye: [
            centre_x + ORBIT_RADIUS * theta.cos(),
            height,
            centre_z + ORBIT_RADIUS * theta.sin(),
        ],
        target: LOOK_AT,
    }
}
