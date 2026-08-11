//! Headless frame capture, perceptual comparison and the golden-frame lifecycle.
//!
//! # The seam
//!
//! This module is split into a **core** and a **GPU layer**, and the split is
//! load-bearing rather than cosmetic:
//!
//! > The GPU layer gathers facts and executes. Every branch that *decides*
//! > something lives in a pure function that never sees a device.
//!
//! Adapter preference, limit checking, size validation, row unpadding, deadline
//! expiry, comparison, diffing, the golden lifecycle, report shape and path
//! construction are all decisions. None of them needs a GPU, and none of them is
//! allowed to have one. What is left for the GPU layer is: create an instance,
//! enumerate, request, allocate, encode, submit, map, copy bytes out.
//!
//! Two consequences bind every file under `frame/`:
//!
//! - **`wgpu::` may appear only under `frame/gpu/`.** Everything else consumes
//!   plain values — an `Rgba8Image` and an adapter description — so its tests
//!   construct their inputs by hand and run in a process that holds no adapter
//!   at all.
//! - **The GPU layer is behind a default-on `gpu` Cargo feature.** In a
//!   `--no-default-features` build wgpu is not in the dependency graph, so a
//!   stray `use wgpu::` in the core is a build error rather than a review
//!   finding. That configuration is run by the quality gate; it is the only
//!   process in which no adapter *can* exist.
//!
//! # Pixel format
//!
//! One format, fixed, so captures are comparable across runs: 8-bit RGBA,
//! sRGB-encoded, straight (non-premultiplied) alpha, row 0 = top. **No stage of
//! this crate flips rows or touches alpha.**

mod color;
mod compare;
mod diff;
mod image;
mod png;
mod readback;

pub use compare::{
    Comparison, FailingMask, MismatchReason, ThresholdError, Thresholds, Verdict, compare,
};
pub use diff::render_diff;
pub use image::{FrameSize, FrameSizeError, ImageShapeError, Rgba8Image, validate_frame_size};
pub use png::{ImageIoError, encode_png, read_png, write_png};
pub use readback::{ReadbackError, unpad_rows};
