//! The bound that stops a lost device from hanging a test run.
//!
//! Time is behind a trait here, unlike the environment opt-ins, because it is a
//! *stream of observations* rather than a value read once: the deadline has to
//! elapse **during** the wait, and a wait that was already expired when it
//! started never proves the loop can end. A fake clock that advances on
//! observation reproduces a thirty-second bound in microseconds, which is the
//! only reason a thirty-second scenario is testable at all.
//!
//! [`poll_until_deadline`] never sleeps. Sleeping belongs in the caller's step
//! closure, beside the device poll it is pacing.

use std::time::{Duration, Instant};

use thiserror::Error;

use super::image::FrameSizeError;
use super::layout::CaptureId;
use super::readback::ReadbackError;

/// How long something has been going on.
pub trait Clock {
    fn elapsed(&self) -> Duration;
}

/// A [`Clock`] reading real elapsed time.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock(Instant);

impl SystemClock {
    #[must_use]
    pub fn started_now() -> Self {
        Self(Instant::now())
    }
}

impl Clock for SystemClock {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
}

/// Whether the thing being waited for has arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress<T> {
    Ready(T),
    Pending,
}

/// A value, and how long it took to arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elapsed<T> {
    pub value: T,
    pub elapsed: Duration,
}

/// Why a wait ended without the value it was waiting for.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineExpired {
    #[error("the wait passed its {deadline:?} deadline after {elapsed:?}")]
    Expired {
        deadline: Duration,
        elapsed: Duration,
    },
    #[error("the wait ended early")]
    Step(#[source] ReadbackError),
}

impl DeadlineExpired {
    /// Names the capture that was waiting.
    ///
    /// The wait itself does not know which capture it belongs to — that is what
    /// keeps it a pure loop — so the caller attaches the name here. This is the
    /// seam that lets the timeout be asserted without a device.
    #[must_use]
    pub fn into_capture_error(self, capture: CaptureId) -> CaptureError {
        match self {
            Self::Expired { deadline, .. } => CaptureError::ReadbackTimeout { capture, deadline },
            Self::Step(cause) => CaptureError::Readback(cause),
        }
    }
}

/// A capture that did not produce a frame.
#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("the requested capture size cannot be rendered")]
    Size(#[source] FrameSizeError),
    #[error("the caller's draw work failed")]
    DrawWork(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(
        "the capture `{capture}` did not finish reading back within {deadline:?}",
        capture = capture.as_str()
    )]
    ReadbackTimeout {
        capture: CaptureId,
        deadline: Duration,
    },
    #[error("the frame could not be read back from the device")]
    Readback(#[source] ReadbackError),
}

/// Runs `step` until it reports the value, `deadline` passes, or it fails.
///
/// `step` is polled **before** the first expiry check, so the deadline can only
/// be reached during the wait and never before it began.
///
/// # Errors
///
/// Returns [`DeadlineExpired::Expired`] naming the deadline once it has passed,
/// or [`DeadlineExpired::Step`] carrying the step's own failure.
pub fn poll_until_deadline<T>(
    clock: &dyn Clock,
    deadline: Duration,
    mut step: impl FnMut() -> Result<Progress<T>, ReadbackError>,
) -> Result<Elapsed<T>, DeadlineExpired> {
    loop {
        match step() {
            Ok(Progress::Ready(value)) => {
                return Ok(Elapsed {
                    value,
                    elapsed: clock.elapsed(),
                });
            }
            Ok(Progress::Pending) => {}
            Err(cause) => return Err(DeadlineExpired::Step(cause)),
        }

        let elapsed = clock.elapsed();
        if elapsed >= deadline {
            return Err(DeadlineExpired::Expired { deadline, elapsed });
        }
    }
}
