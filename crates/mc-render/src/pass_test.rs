//! The one terrain-pass descriptor, and the single field the two paths may
//! disagree about.
//!
//! A windowed frame and a captured frame are only comparable if they were drawn
//! by the same pass. The failure this guards against is not a wrong picture but
//! a **divergent** one: the goldens keep passing while the window shows
//! something else, and nothing in the suite is looking at the window. So there
//! is one descriptor type, both paths construct one, and the test below is that
//! the two are the same value once the colour format is set aside.
//!
//! **The comparison is on the whole struct, deliberately.** Enumerating today's
//! fields one by one would pass forever the day a seventh setting is added and
//! the two constructors disagree about it — the test would still be checking the
//! six it was written against. Normalising the one field they are allowed to
//! differ in and comparing the rest as a value keeps a new field covered from
//! the moment it exists.
//!
//! The offscreen format is a second fact and gets its own test: `offscreen()`
//! declares it, and the harness that reads the frame back declares it too, so
//! the two are asserted to agree rather than trusted to. That one has to name
//! `wgpu::TextureFormat`, which may appear only under `src/gpu/` and in a test
//! target carrying `required-features = ["gpu"]`, so it lives in
//! `tests/pass_format.rs` rather than here.

use super::{ColorFormat, TerrainPassConfig};

/// The format a windowed surface is configured with, which is not the format a
/// capture uses. Any value that differs from the offscreen one serves; this is
/// the format FR-9.1's selection picks from a typical surface.
const WINDOW_FORMAT: ColorFormat = ColorFormat::Bgra8UnormSrgb;

#[test]
fn the_windowed_terrain_pass_differs_from_the_captured_one_in_its_colour_format_alone() {
    let offscreen = TerrainPassConfig::offscreen();
    let windowed = TerrainPassConfig::windowed(WINDOW_FORMAT);

    // Every setting from the windowed path, with the one difference the spec
    // permits removed. What is left must be the offscreen configuration exactly.
    let windowed_but_for_its_colour_target = TerrainPassConfig {
        color_format: offscreen.color_format,
        ..windowed
    };

    assert_eq!(
        offscreen, windowed_but_for_its_colour_target,
        "the depth format, clear colour, cull mode, front face, depth comparison and vertex \
         stride must be the same value on both paths; a setting the two constructors choose \
         differently is a window that draws something the goldens never see"
    );
}
