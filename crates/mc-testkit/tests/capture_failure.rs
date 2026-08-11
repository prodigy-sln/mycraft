//! What happens when the caller's own draw work fails, and what the context is
//! good for afterwards.
//!
//! A test binary captures many frames from one context, so a context that a
//! single failed capture poisons would be a silent cliff: everything after the
//! first failure would fail for a reason that has nothing to do with what it
//! was testing.

mod scene;

use std::error::Error;
use std::fmt;

use mc_testkit::frame::CaptureError;
use mc_testkit::frame::gpu::{DrawResult, DrawWork, draw_fn};
use scene::TestResult;

const EDGE: u32 = 64;
/// A pixel well inside the frame, where a valid capture carries the clear.
const MIDDLE: u32 = 32;

/// The caller's own error, defined here so that finding it again in the harness's
/// error chain means the harness carried *this* value rather than a description
/// of it.
#[derive(Debug)]
struct SceneUnavailable;

impl fmt::Display for SceneUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the caller's scene could not be recorded")
    }
}

impl Error for SceneUnavailable {}

/// Draw work that records nothing and fails.
fn failing_draw() -> impl DrawWork {
    draw_fn(|_encoder, _target| -> DrawResult { Err(Box::new(SceneUnavailable)) })
}

#[test]
fn a_capture_whose_draw_work_fails_returns_that_error_and_no_image() -> TestResult {
    let context = scene::device_context()?;
    let request = scene::request(&context, "failing-draw", EDGE, EDGE)?;
    let mut draw = failing_draw();

    let error = context
        .capture(&request, &mut draw)
        .err()
        .ok_or("draw work that failed must not yield an image")?;

    let CaptureError::DrawWork(_) = &error else {
        return Err(format!("expected the caller's own failure, got {error:?}").into());
    };
    let source = error
        .source()
        .and_then(|cause| cause.downcast_ref::<SceneUnavailable>());
    assert!(
        source.is_some(),
        "the caller's own error must reach the caller intact, got {error:?}"
    );
    Ok(())
}

#[test]
fn a_context_still_captures_after_a_failed_capture() -> TestResult {
    let context = scene::device_context()?;
    let failing_request = scene::request(&context, "poisoning-draw", EDGE, EDGE)?;
    let mut failing = failing_draw();
    let first = context.capture(&failing_request, &mut failing);
    assert!(
        first.is_err(),
        "this test means nothing unless the first capture failed"
    );

    let request = scene::request(&context, "capture-after-failure", EDGE, EDGE)?;
    let mut draw = scene::clear(scene::OPAQUE_RED);
    let image = context.capture(&request, &mut draw)?.image;

    assert_eq!(
        image
            .pixel(MIDDLE, MIDDLE)
            .ok_or("the recovered capture is missing its middle pixel")?,
        scene::OPAQUE_RED_BYTES,
        "the next capture on the same context must produce a valid image"
    );
    Ok(())
}
