//! The offscreen pass targets the format the capture harness reads back.
//!
//! The offscreen pass renders into the harness's own target, so the format it
//! declares is not a free choice — it is a fact the harness owns. Comparing the
//! two here is what keeps the renderer from re-declaring a literal that could
//! drift from the one the readback actually uses, which would read a texture the
//! pass never wrote.
//!
//! **Why this is a target of its own.** `TerrainPassConfig` is pure, and its
//! scenario test sits beside it in `src/pass_test.rs` with no GPU type in sight.
//! This check is the one assertion about it that has to name
//! `wgpu::TextureFormat`, and `wgpu::` may be named only under `src/gpu/` and in
//! a test target carrying `required-features = ["gpu"]`. A `#[cfg]` inside the
//! lib's own test module would compile clean and still put a GPU type in a file
//! the pure layer builds — the seam the feature exists to keep sharp. The
//! `required-features` form also *reports* itself as skipped when the feature is
//! off, rather than vanishing quietly.
//!
//! Nothing here creates a device: the assertion is between two constants.

use mc_render::pass::{ColorFormat, TerrainPassConfig};
use mc_testkit::frame::gpu::CAPTURE_FORMAT;
use mc_testkit::frame::wgpu::TextureFormat;

#[test]
fn the_captured_pass_targets_the_format_the_capture_harness_reads_back() {
    // The mapping is this test's own, not the renderer's: a harness format this
    // crate cannot express fails the assertion rather than failing to compile.
    let harness_format = match CAPTURE_FORMAT {
        TextureFormat::Rgba8UnormSrgb => Some(ColorFormat::Rgba8UnormSrgb),
        TextureFormat::Bgra8UnormSrgb => Some(ColorFormat::Bgra8UnormSrgb),
        _ => None,
    };

    assert_eq!(
        Some(TerrainPassConfig::offscreen().color_format),
        harness_format,
        "the offscreen pass must target the format the harness allocates and reads back \
         ({CAPTURE_FORMAT:?}); a second literal here is a second place the two can drift"
    );
}
