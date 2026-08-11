//! The readback deadline: a lost device must not hang a test run.
//!
//! The clock here is fake and advances **on observation**, so the thirty-second
//! bound the harness ships with is crossed in microseconds. That is the whole
//! reason a thirty-second scenario is testable at all — and it is why the
//! deadline elapses *during* the wait rather than before it, which is the only
//! version of this behaviour worth asserting: a wait that was already expired
//! when it started never proves the loop can end.
//!
//! Nothing here sleeps, because `poll_until_deadline` must not. Sleeping
//! belongs in the caller's step closure, where the device poll lives.

mod common;

use std::cell::Cell;
use std::time::Duration;

use common::TestResult;
use mc_testkit::frame::{
    CaptureError, CaptureId, Clock, DeadlineExpired, Elapsed, Progress, poll_until_deadline,
};

const CAPTURE: &str = "slow-readback";
const DEADLINE: Duration = Duration::from_secs(30);
/// Each observation of the clock is ten seconds later than the last. The first
/// reads zero, so no expiry check can conclude the deadline has passed before
/// the step closure has run at least once.
const TICK: Duration = Duration::from_secs(10);

#[derive(Debug, Default)]
struct AdvancingClock {
    observations: Cell<u32>,
}

impl Clock for AdvancingClock {
    fn elapsed(&self) -> Duration {
        let observations = self.observations.get();
        self.observations.set(observations.saturating_add(1));
        TICK.saturating_mul(observations)
    }
}

#[test]
fn a_readback_that_outlives_its_deadline_times_out_naming_the_capture_and_the_bound() -> TestResult
{
    let clock = AdvancingClock::default();

    // A device that never reports the mapping complete: the only way out of
    // this loop is the deadline.
    let outcome: Result<Elapsed<u8>, DeadlineExpired> =
        poll_until_deadline(&clock, DEADLINE, || Ok(Progress::Pending));

    let expired = outcome
        .err()
        .ok_or("a readback that never completes must not yield an image")?;
    let error = expired.into_capture_error(CaptureId::new(CAPTURE)?);

    let CaptureError::ReadbackTimeout { capture, deadline } = error else {
        return Err(format!("expected a readback timeout, got {error:?}").into());
    };
    assert_eq!(
        (capture.as_str(), deadline),
        (CAPTURE, DEADLINE),
        "the timeout names the capture that stalled and the bound it broke, so \
         a hung run points at one frame rather than at the suite"
    );
    Ok(())
}
