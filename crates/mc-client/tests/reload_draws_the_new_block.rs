//! The layer a reload appended, on a real device with no window: a placement of
//! the newly declared block, drawn from the texels that layer was filled with.
//!
//! # This is the half the value-at-the-boundary assertion cannot cover, and the
//! reverse is true as well
//!
//! A report can hand over a perfectly correct assignment and a packer can write a
//! perfectly correct layer index into every corner while nothing ever fills that
//! layer of the array texture — and the block then draws from whatever the
//! allocator left there, which is a picture that is wrong in a plausible way. So
//! this scenario uploads the layers the report handed over, uploads the scene those
//! layers packed, draws it, and reads the pixels back.
//!
//! # The control is the same camera over the same world one reload earlier
//!
//! Two frames, the same declared pose, differing only in whether the reload and the
//! placement happened. The rect at the frame's centre has to be made *entirely* of
//! the new block's two texel colours in the second frame and *none* of them in the
//! first — which a frame that drew nothing, a frame that drew the block from another
//! block's layer, and a frame drawn from an unfilled layer all fail.
//!
//! # Where the camera stands, and why not straight down
//!
//! The placed cell is `(9, 10, 8)`, so its upward face is the unit square at
//! `y = 11` centred on `(9.5, 8.5)`. The eye stands three blocks over that centre
//! and `3 · tan 30° = 1.732` blocks along `+x`, so the view is 30° off vertical:
//! straight down, a look-at matrix has no unique answer because the view direction
//! is the world's own up.
//!
//! At that pose the eye is `√(3² + 1.732²) = 3.464` blocks from the face and the
//! lens takes in `2 · 3.464 · tan 30° = 4.0` world units of height over 720 pixels
//! — 180 pixels per block. The face therefore projects to roughly 180 × 156 pixels
//! (the second foreshortened by `cos 30°`), so the 32-pixel square centred on the
//! target sits well inside it, and 32 pixels spans about 2.8 of the sixteen texels
//! across a block face — which is what makes both colours of the checkerboard
//! appear rather than one.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_remesh.rs"]
mod reload_remesh;
#[path = "support/reload_upload.rs"]
mod reload_upload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::sync::Arc;

use mc_client::startup::scene_of;
use mc_core::id::BlockName;
use mc_render::camera::camera_view;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::TerrainRenderer;
use mc_render::pass::TerrainPassConfig;
use mc_render::texture::TextureLayers;
use mc_sim::replay::{remesh, splice};
use mc_sim::world::World;
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::CaptureContext;

use reload::{AMBER, AMBER_FILE, STONE, amber, shipped};
use reload_remesh::{NOTHING_WAS_LEFT_TO_MESH, placing_over_the_near_cell, require};
use reload_upload::{declaring_after_launch, layers_handed_over, until_taken_up};
use reload_watch::a_client_on;
use reload_world::{Edit, NOTHING, OVER_THE_NEAR_CELL, floor_of, registry_of, wrote};
use support::hud_frames::Rect;
use support::swatch::{TEXEL_COLORS, swatch_reading, texel_colors};
use support::{TestResult, frames};

/// The tick every frame here is drawn at. Nothing about the picture depends on it.
const A_TICK: u32 = 0;

/// What the camera looks at, and where it stands — derived in this file's header.
const THE_PLACED_FACE: [f32; 3] = [9.5, 11.0, 8.5];
const OVER_THE_PLACED_FACE: [f32; 3] = [11.232, 14.0, 8.5];

/// The square at the centre of the frame, which the placed block's upward face
/// covers — derived in this file's header.
const THE_MIDDLE_OF_THE_FACE: Rect = Rect {
    x: 624,
    y: 344,
    width: 32,
    height: 32,
};

#[test]
fn a_placement_of_a_newly_declared_block_is_drawn_from_the_texels_its_layer_was_filled_with()
-> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let Drawn {
        built,
        before,
        after,
    } = a_run_that_places_the_new_block(&context)?;

    let colors = texel_colors(&BlockName::parse(AMBER)?)?;
    let showing = swatch_reading(&after, THE_MIDDLE_OF_THE_FACE, &colors)?;
    let was = swatch_reading(&before, THE_MIDDLE_OF_THE_FACE, &colors)?;
    require_both_frames_were_read(showing.considered, was.considered)?;

    assert_eq!(
        (built, showing.strayed, showing.shown, was.strayed),
        (
            wrote(OVER_THE_NEAR_CELL, NOTHING, AMBER),
            0,
            TEXEL_COLORS,
            THE_MIDDLE_OF_THE_FACE.area()
        ),
        "the appended layer has to be filled and the placed block has to sample it: every pixel of \
         the face's middle is one of the two colours that block's placeholder layer is made of, and \
         both of them are there. The last element is the control, taken through the same code over \
         the same world one reload earlier — none of those pixels was any of those colours before \
         the block existed, so a frame that drew nothing, one that drew the block from another \
         block's layer, and one drawn from a layer nothing ever wrote to all fail"
    );
    Ok(())
}

/// What one run produced: the placement it made, and the two frames either side of
/// the reload that declared what it placed.
struct Drawn {
    built: Edit,
    before: Rgba8Image,
    after: Rgba8Image,
}

/// A run that launches over a stone floor, has an author declare a block, places
/// that block, and draws the same pose before and after.
///
/// **The scene the second frame is drawn from is spliced out of the first one's
/// meshed list**, which is what a client holds across an edit — so the two frames
/// differ only by the reload and the placement.
///
/// # Errors
///
/// Returns the read, world, mesh, packing, upload or capture failure, and the
/// refusal where no candidate was taken up.
fn a_run_that_places_the_new_block(context: &CaptureContext) -> Result<Drawn, Box<dyn Error>> {
    let root = shipped()?;
    let at_launch = registry_of(root.path())?;
    let blocks = floor_of(&at_launch, STONE)?;
    let (mut client, reports) = a_client_on(&root, STONE)?;
    let mut meshed = World::new(blocks, at_launch)?.mesh()?;
    let before_resolution = reload_remesh::resolution_serving(&client)?;
    let before_scene = Arc::new(scene_of(&meshed, &before_resolution)?);

    let declared = declaring_after_launch(&root, AMBER_FILE, &amber())?;
    reports.changed(&[declared])?;
    let resolution = layers_handed_over(until_taken_up(&mut client))?;
    let built = placing_over_the_near_cell(&mut client);
    let work = client.take_remesh_work().ok_or(NOTHING_WAS_LEFT_TO_MESH)?;
    splice(&mut meshed, remesh(&work)?)?;
    let after_scene = Arc::new(scene_of(&meshed, resolution.stated())?);

    Ok(Drawn {
        built,
        before: drawn(
            context,
            before_resolution.layers(),
            &before_scene,
            "reload-before-amber",
        )?,
        after: drawn(
            context,
            resolution.stated().layers(),
            &after_scene,
            "reload-after-amber",
        )?,
    })
}

/// One frame of `scene`, drawn against `layers` through the declared pose.
///
/// # Errors
///
/// Returns the pipeline, upload, recording or capture failure.
fn drawn(
    context: &CaptureContext,
    layers: &TextureLayers,
    scene: &Arc<SceneGeometry>,
    name: &str,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let mut renderer = TerrainRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
        &frames::no_supplied_texels(),
    )?;
    renderer.upload_textures(context.queue(), layers)?;
    renderer.upload_scene(context.queue(), scene)?;
    let snapshot = frames::snapshot(
        A_TICK,
        camera_view(OVER_THE_PLACED_FACE, THE_PLACED_FACE),
        scene,
    );
    let mut frame = frames::ReplayFrame {
        context,
        renderer: &mut renderer,
        snapshot: &snapshot,
    };
    frame.capture(&frames::request(context, name)?)
}

/// Refuses unless both readings looked at the whole square they were given.
///
/// A region that accepts nothing makes "nothing strayed" true, so a rect that fell
/// off the frame would satisfy the assertion for a reason that has nothing to do
/// with what was drawn.
fn require_both_frames_were_read(showing: u64, was: u64) -> Result<(), Box<dyn Error>> {
    require(
        showing == THE_MIDDLE_OF_THE_FACE.area() && was == THE_MIDDLE_OF_THE_FACE.area(),
        format!(
            "both readings have to cover the whole {area}-pixel square, and they covered {showing} \
             and {was}",
            area = THE_MIDDLE_OF_THE_FACE.area()
        ),
    )
}
