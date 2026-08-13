//! The declared per-tick intent script the replay is driven by, and the tick
//! index it is asked with.
//!
//! The replay's length is a property of *this* script and not of the simulation:
//! a windowed client runs for as long as its window is open, and what runs for
//! exactly 120 ticks is the sequence of intents a golden frame is shot through.
//! So [`SCRIPT_TICKS`] lives here, where it bounds the tick index and therefore
//! bounds the script itself — a tick past the end cannot be constructed, so it
//! cannot be asked for.
//!
//! [`TickIndex`] lives here for the same reason, and it arrived here when the
//! orbit it used to bound was deleted. It is a validated newtype rather than a
//! number because a guard inside [`scripted_intent`] would have to decide what
//! to return for a tick past the end; a constructor that refuses one means the
//! question cannot be asked.
//!
//! The intervals below are what the goldens are shot through, and each one earns
//! its place: half a second of stillness so the spawn's own fall is what the
//! first frame shows, a walk long enough to cross terrain that rises, a turn
//! taken *while* walking so that the camera and the motion are visibly the same
//! player's, and a single jump — asked for on one tick and no other, because a
//! jump held down relaunches on every landing and would make the arc a property
//! of where the ground happened to be.

use thiserror::Error;

use crate::player::MovementIntent;

/// How many ticks the declared intent script runs for.
///
/// It bounds [`TickIndex`], so a tick at or past it is refused rather than
/// wrapping back into the script.
pub const SCRIPT_TICKS: u32 = 120;

/// A tick of this replay, and never one past its end.
///
/// [`scripted_intent`] takes one of these rather than a number, so a tick the
/// script does not have cannot be constructed and therefore cannot be passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TickIndex(u32);

impl TickIndex {
    /// The replay's first tick.
    pub const FIRST: Self = Self(0);

    /// The tick numbered `tick`.
    ///
    /// # Errors
    ///
    /// Returns [`TickError::BeyondReplay`] for a tick at or past the script's
    /// length, naming that length rather than extending the script.
    pub const fn new(tick: u32) -> Result<Self, TickError> {
        if tick >= SCRIPT_TICKS {
            return Err(TickError::BeyondReplay {
                tick,
                tick_count: SCRIPT_TICKS,
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

/// The first tick the script walks from. Everything before it asks for nothing.
const WALK_FROM_TICK: u32 = 30;

/// The ticks the script turns on: from the first, up to but not including the
/// jump.
const TURN_FROM_TICK: u32 = 60;

/// The one tick a jump is asked for.
const JUMP_TICK: u32 = 90;

/// How far the script turns on each of its turning ticks, in degrees.
const TURN_DEGREES: f32 = 1.0;

/// What the declared script asks of `tick`.
///
/// A total function of the index and of nothing else, which is what lets a
/// golden frame be a claim about tick 59 rather than about the fifty-ninth time
/// somebody ran it.
#[must_use]
pub fn scripted_intent(tick: TickIndex) -> MovementIntent {
    let tick = tick.get();
    if tick < WALK_FROM_TICK {
        return MovementIntent::default();
    }
    MovementIntent {
        forward: 1.0,
        yaw_delta: if (TURN_FROM_TICK..JUMP_TICK).contains(&tick) {
            TURN_DEGREES.to_radians()
        } else {
            0.0
        },
        jump: tick == JUMP_TICK,
        ..MovementIntent::default()
    }
}
