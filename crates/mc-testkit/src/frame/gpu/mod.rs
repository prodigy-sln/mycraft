//! The wgpu layer: the only place in this crate where a GPU type may be named.
//!
//! Everything here gathers facts and executes. It creates an instance,
//! enumerates, requests, allocates, encodes, submits, maps and copies bytes out.
//! Every branch that *decides* something — which adapter to prefer, whether a
//! size is renderable, what a failed acquisition means, when a deadline has
//! passed, how two frames compare — lives in a pure function on the other side
//! of the seam and never sees a device.
//!
//! # The caller owns the render pass
//!
//! The harness supplies a canvas and never a scene: it creates the texture,
//! hands out an encoder and a view, and lets the caller begin its own passes,
//! including choosing the load op. That is what keeps this crate ignorant of the
//! renderer it exists to verify — the moment it knew what a chunk or a camera
//! was, it would be verifying something it depends on.
//!
//! # Which way is up
//!
//! Because the caller owns the pass, orientation is public interface rather than
//! an internal detail, and it is binding on every caller:
//!
//! - **Framebuffer row 0 is the top**, and stays the top through readback,
//!   comparison, PNG encode and PNG decode. No stage flips rows.
//! - **Clip-space y is up.** wgpu's framebuffer origin and clip-space y point in
//!   opposite directions, which is where most of the ecosystem's flipped frames
//!   come from, and it is the caller's to get right.
//! - Consequently **a caller filling the top half of the target writes y > 0**.

mod acquire;
mod target;

use std::time::Duration;

pub use acquire::{AcquireOptions, Acquisition, CaptureContext};
pub use target::CAPTURE_FORMAT;

use super::clock::CaptureError;
use super::golden::{GoldenOutcome, verify_against_golden};
use super::image::{FrameSize, Rgba8Image};
use super::layout::{CaptureId, GoldenSettings};

/// How long a readback may take before it is called a lost device.
///
/// A liveness bound, not a performance budget: capture speed is not a
/// requirement of this harness, and nothing here asserts how fast a frame
/// arrives — only that a frame that never arrives cannot hang a test run.
pub const DEFAULT_READBACK_DEADLINE: Duration = Duration::from_secs(30);

/// What a caller's draw work reports back.
///
/// Boxed and type-erased rather than a generic error parameter: a parameter
/// would infect every signature in this module and trip the type-complexity
/// budget, for a value that is only ever propagated.
pub type DrawResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Draw work supplied by the caller, recorded into the harness's target.
pub trait DrawWork {
    /// Records the caller's passes into `encoder`, rendering into `target`.
    ///
    /// # Errors
    ///
    /// Returns the caller's own error. It reaches the caller again inside
    /// [`CaptureError::DrawWork`], preserved in the `source()` chain and still
    /// downcastable to its original type.
    fn record(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) -> DrawResult;
}

/// Turns a closure into [`DrawWork`].
///
/// Passing a bare closure to `capture` makes rustc infer a higher-ranked bound
/// over two independent lifetimes, which it routinely fails at with an opaque
/// error. Fixing the shape at the call site is the whole job of this helper.
pub fn draw_fn<F>(record: F) -> impl DrawWork
where
    F: FnMut(&mut wgpu::CommandEncoder, &wgpu::TextureView) -> DrawResult,
{
    Recorded(record)
}

/// The [`DrawWork`] a closure becomes.
struct Recorded<F>(F);

impl<F> DrawWork for Recorded<F>
where
    F: FnMut(&mut wgpu::CommandEncoder, &wgpu::TextureView) -> DrawResult,
{
    fn record(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) -> DrawResult {
        (self.0)(encoder, target)
    }
}

/// One frame to capture: what to call it, how big it is, and how long its
/// readback may take.
#[derive(Debug, Clone)]
pub struct CaptureRequest {
    pub capture: CaptureId,
    pub size: FrameSize,
    pub deadline: Duration,
}

impl CaptureRequest {
    /// A request carrying the harness's own readback deadline.
    ///
    /// The field stays public, so a caller that has a reason to bound a readback
    /// differently can say so without a second constructor.
    #[must_use]
    pub fn new(capture: CaptureId, size: FrameSize) -> Self {
        Self {
            capture,
            size,
            deadline: DEFAULT_READBACK_DEADLINE,
        }
    }
}

/// A captured frame, and how long it took to read back.
#[derive(Debug, Clone)]
pub struct Capture {
    pub image: Rgba8Image,
    pub readback: Duration,
}

impl CaptureContext {
    /// Renders `draw` into an offscreen target and reads the pixels back.
    ///
    /// The texture and the readback buffer belong to this call and drop with it,
    /// so a capture that failed leaves the context exactly as it found it and
    /// the next capture succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError::DrawWork`] carrying the caller's own failure,
    /// [`CaptureError::ReadbackTimeout`] naming the capture and its deadline, or
    /// [`CaptureError::Readback`] when the device did not hand the frame over.
    pub fn capture(
        &self,
        request: &CaptureRequest,
        draw: &mut dyn DrawWork,
    ) -> Result<Capture, CaptureError> {
        target::capture(self.device(), self.queue(), request, draw)
    }
}

/// The composition root of the harness: capture a frame, then judge it against
/// its golden.
///
/// It lives in the GPU layer rather than the core so that the core never needs
/// to invoke a device — [`verify_against_golden`] takes an image and a
/// provenance as plain values, and this is the one function that has both.
///
/// # Errors
///
/// Returns the capture failure. A frame that was captured but did not match its
/// golden is not an error: it is a [`GoldenOutcome`], because a verdict about
/// ground truth is a result and not a malfunction.
pub fn capture_and_verify(
    context: &CaptureContext,
    request: &CaptureRequest,
    draw: &mut dyn DrawWork,
    settings: &GoldenSettings,
) -> Result<GoldenOutcome, CaptureError> {
    let captured = context.capture(request, draw)?;
    Ok(verify_against_golden(
        &captured.image,
        context.provenance(),
        settings,
    ))
}
