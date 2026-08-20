//! The terrain pass, drawn offscreen: depth, the compute-decided visible set,
//! the one indirect draw call, and what a frame reports about itself.
//!
//! # What each assertion is judged against
//!
//! Nothing here compares the renderer against itself. The visible set is judged
//! against the *pure* frustum function, the compacted index count against the
//! quad totals of the section table the scene was assembled from, and the drawn
//! colours against `placeholder_mean_color`, which is a declaration derived from
//! a texture key and never a colour read out of a frame.
//!
//! # Two shapes of vacuous green, and the fixtures that close them
//!
//! A renderer that draws **nothing** satisfies "no section visible means an index
//! count of zero" and "no section visible means a frame of clear colour" without
//! ever deciding anything. Those two scenarios are therefore the only ones whose
//! fixture is entirely out of view, and both assert that the scene *held quads*
//! — so "drew nothing" stays distinguishable from "had nothing to draw".
//!
//! A renderer that **ignores culling** satisfies every count comparison whenever
//! the camera admits the whole scene. So every fixture that compares a count
//! against the pure function is half in view and half behind the camera, with
//! per-section quad counts that differ, and each of those tests asserts that
//! shape before it asserts anything about the renderer.
//!
//! # The camera
//!
//! The pose below is written out rather than taken from the simulation, and it
//! must stay that way: the renderer may not resolve the simulation in any
//! dependency kind, and a camera imported from the code under test would not be
//! an independent statement of what this crate draws. It is a declared vantage
//! over the fixture scene and is deliberately not a pose any camera reaches —
//! the orbit it was first derived from no longer exists, and nothing here
//! depended on it having been one.

mod support;

use std::error::Error;
use std::sync::Arc;

use mc_core::id::{BlockName, TextureKey};
use mc_render::camera::{CameraView, camera_view};
use mc_render::color::CLEAR_COLOR_SRGB;
use mc_render::geometry::SectionOrigin;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::{FrameError, RecordTarget, TerrainRenderer};
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_render::texture::placeholder::placeholder_mean_color;
use mc_testkit::frame::gpu::draw_fn;
use mc_testkit::frame::{Rgba8Image, wgpu};

use support::{
    COUNTING_FRAME, DEPTH_FRAME, Fixture, REPLAY_EYE_AT_TICK_60, REPLAY_LOOK_AT, SAME_TEXTURE,
    TestResult,
};

/// How many sections every counting fixture holds.
const SECTIONS: u32 = 64;

/// How many indices one quad is drawn by.
const INDICES_PER_QUAD: u32 = 6;

/// The tick the counting frames are rendered at, which is the tick their camera
/// pose is declared for.
const TICK: u32 = 60;

/// The block the counting fixtures are made of. An `example:` namespace: a
/// fixture borrowing a shipped block name would be the engine describing itself
/// in terms of content.
const FILLER: &str = "example:filler";

/// The two blocks of the depth fixture. Their declared placeholder means stand
/// far apart, and the test asserts that before it asserts anything else — two
/// blocks that looked alike could not tell which one the frame showed.
const NEARER: &str = "example:nearer";
const FARTHER: &str = "example:farther";

/// Where the two stacked sections sit, and where in each of them the one solid
/// block stands.
const NEARER_SECTION: [i32; 3] = [0, 0, 0];
const FARTHER_SECTION: [i32; 3] = [0, 0, -16];
const BLOCK_AT: [u32; 3] = [8, 8, 8];

/// Which of the two stacked sections is handed over first.
#[derive(Debug, Clone, Copy)]
enum Order {
    NearerFirst,
    FartherFirst,
}

#[test]
fn the_nearer_of_two_stacked_blocks_is_what_the_centre_pixel_shows() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let nearer = placeholder_mean_color(&TextureKey::parse(NEARER)?);
    let farther = placeholder_mean_color(&TextureKey::parse(FARTHER)?);
    assert!(
        support::delta_e(nearer, farther)? > SAME_TEXTURE,
        "the two blocks have to be told apart by their declared means before a frame can be \
         asked which one it shows: {nearer:?} against {farther:?}"
    );

    let centre = stacked_centre_pixel(&context, Order::NearerFirst, "terrain-depth-nearer")?;

    let to_nearer = support::delta_e(centre, nearer)?;
    let to_farther = support::delta_e(centre, farther)?;
    assert!(
        to_nearer <= SAME_TEXTURE,
        "the block in front has to be the one the centre pixel shows: {centre:?} sits ΔE \
         {to_nearer:.1} from the nearer block's declared mean and ΔE {to_farther:.1} from the \
         farther one's"
    );
    Ok(())
}

#[test]
fn the_centre_pixel_is_the_same_whichever_order_the_two_sections_arrive_in() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };

    let farther_first =
        stacked_centre_pixel(&context, Order::FartherFirst, "terrain-depth-farther-first")?;
    let nearer_first =
        stacked_centre_pixel(&context, Order::NearerFirst, "terrain-depth-nearer-first")?;

    let to_sky = support::delta_e(farther_first, CLEAR_COLOR_SRGB)?;
    assert!(
        to_sky > SAME_TEXTURE,
        "both frames have to have drawn something, or two pictures of empty sky would agree \
         about the order they were drawn in: the centre pixel {farther_first:?} sits ΔE \
         {to_sky:.1} from the declared clear colour"
    );
    assert_eq!(
        farther_first, nearer_first,
        "which section was handed over first decides nothing about the picture: depth does"
    );
    Ok(())
}

#[test]
fn a_rendered_frame_reports_its_tick_its_sections_and_its_one_draw_call() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = support::grid_scene(&support::origins_half_in_view(), FILLER)?;
    let admitted = support::admitted(&fixture.scene, &replay_camera(), COUNTING_FRAME);
    assert_partly_in_view(&fixture, &admitted);

    let frame = render_counting(&context, &fixture, "terrain-statistics")?;

    assert_eq!(
        (
            frame.stats.tick,
            frame.stats.sections_submitted,
            frame.stats.sections_admitted,
            frame.stats.terrain_draw_calls
        ),
        (TICK, SECTIONS, admitted.len() as u32, 1),
        "a frame reports the tick it was handed, how many sections it was given, how many of \
         them the frustum admits, and the one draw call terrain costs"
    );
    Ok(())
}

#[test]
fn a_terrain_pass_that_cannot_be_recorded_names_the_stage_that_failed() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = support::grid_scene(&support::origins_in_view(), FILLER)?;
    let mut renderer = TerrainRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
        &support::production_textures(),
    )?;
    renderer.upload_textures(context.queue(), fixture.resolution.layers())?;

    let failure = record_without_a_scene(&context, &mut renderer, &fixture)
        .err()
        .ok_or("a renderer that was never handed a scene cannot record a terrain pass")?;

    let message = failure.to_string();
    let FrameError::Recording { stage } = failure else {
        return Err(format!("expected a recording failure, got `{message}`").into());
    };
    assert!(
        !stage.is_empty() && message.contains(stage),
        "the failure reaches the caller naming the stage that failed rather than ending the \
         process: stage `{stage}`, message `{message}`"
    );
    Ok(())
}

#[test]
fn a_scene_of_sixty_four_visible_sections_still_costs_one_terrain_draw_call() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = support::grid_scene(&support::origins_in_view(), FILLER)?;
    let admitted = support::admitted(&fixture.scene, &replay_camera(), COUNTING_FRAME);
    assert_eq!(
        admitted.len() as u32,
        SECTIONS,
        "all {SECTIONS} sections have to be visible, or this is not a scene of {SECTIONS} \
         visible sections"
    );

    let frame = render_counting(&context, &fixture, "terrain-one-draw-call")?;

    assert_eq!(
        frame.stats.terrain_draw_calls, 1,
        "terrain is one indirect draw whatever is in it; a call per section is the regression \
         this counts"
    );
    Ok(())
}

#[test]
fn the_compute_pass_selects_the_sections_the_frustum_function_admits() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = support::grid_scene(&support::origins_half_in_view(), FILLER)?;
    let expected = support::admitted(&fixture.scene, &replay_camera(), COUNTING_FRAME);
    assert_partly_in_view(&fixture, &expected);

    let mut renderer = support::prepared_renderer(&context, &fixture)?;
    let request = support::request(&context, "terrain-visible-set", COUNTING_FRAME)?;
    let snapshot = support::snapshot(TICK, replay_camera(), &fixture);
    support::render(&context, &mut renderer, &snapshot, &request)?;
    let flags = renderer.read_visible_sections(context.device(), context.queue())?;

    assert_eq!(
        selected(&flags),
        expected,
        "the compute pass tests the same six unnormalised planes against the same boxes as the \
         function on this side, so it has to reach the same set — over {} flags read back",
        flags.len()
    );
    Ok(())
}

#[test]
fn a_frame_with_nothing_in_view_still_issues_its_draw_call_with_no_indices() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = support::grid_scene(&support::origins_behind_the_camera(), FILLER)?;
    let admitted = support::admitted(&fixture.scene, &replay_camera(), COUNTING_FRAME);
    assert_nothing_in_view(&fixture, &admitted);

    let mut renderer = support::prepared_renderer(&context, &fixture)?;
    let request = support::request(&context, "terrain-empty-draw", COUNTING_FRAME)?;
    let snapshot = support::snapshot(TICK, replay_camera(), &fixture);
    let frame = support::render(&context, &mut renderer, &snapshot, &request)?;
    let drawn = renderer.read_drawn_index_count(context.device(), context.queue())?;

    assert_eq!(
        (frame.stats.terrain_draw_calls, drawn),
        (1, 0),
        "the draw is still issued when nothing survives culling; it is its index count that \
         goes to zero, not the call"
    );
    Ok(())
}

#[test]
fn a_frame_with_nothing_in_view_comes_back_as_the_declared_clear_colour() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = support::grid_scene(&support::origins_behind_the_camera(), FILLER)?;
    let admitted = support::admitted(&fixture.scene, &replay_camera(), COUNTING_FRAME);
    assert_nothing_in_view(&fixture, &admitted);

    let frame = render_counting(&context, &fixture, "terrain-empty-frame")?;

    let strayed = support::pixels_away_from(&frame.image, CLEAR_COLOR_SRGB, CLEAR_TOLERANCE)?;
    assert_eq!(
        strayed, 0,
        "a frame with nothing in view is the declared clear colour everywhere, and the clear \
         value is specified in linear space so the hardware's sRGB encode lands back on it"
    );
    Ok(())
}

#[test]
fn the_compacted_index_count_covers_exactly_the_quads_of_the_admitted_sections() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = support::grid_scene(&support::origins_half_in_view(), FILLER)?;
    let admitted = support::admitted(&fixture.scene, &replay_camera(), COUNTING_FRAME);
    assert_partly_in_view(&fixture, &admitted);
    let expected = INDICES_PER_QUAD * support::quads_of(&fixture.scene, &admitted);

    let mut renderer = support::prepared_renderer(&context, &fixture)?;
    let request = support::request(&context, "terrain-compaction", COUNTING_FRAME)?;
    let snapshot = support::snapshot(TICK, replay_camera(), &fixture);
    let frame = support::render(&context, &mut renderer, &snapshot, &request)?;
    let drawn = renderer.read_drawn_index_count(context.device(), context.queue())?;

    assert_eq!(
        (frame.stats.sections_submitted, drawn),
        (SECTIONS, expected),
        "every admitted section contributes six indices per quad and no section that was \
         culled contributes any"
    );
    Ok(())
}

#[test]
fn a_frame_drawn_while_the_scene_is_still_being_prepared_is_the_clear_colour_everywhere()
-> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };

    let frame = render_while_preparing(&context, "terrain-preparing")?;

    let strayed = support::pixels_away_from(&frame, CLEAR_COLOR_SRGB, CLEAR_TOLERANCE)?;
    assert_eq!(
        strayed, 0,
        "while the world is still being generated and meshed the frame is written, and what it \
         is written with is the declared clear colour on every pixel. A surface texture that was \
         acquired and left alone shows whatever the driver last had in it, which reads as a \
         crash rather than as waiting"
    );
    Ok(())
}

/// How far a pixel may sit from the declared clear colour and still be it.
const CLEAR_TOLERANCE: f64 = 2.0;

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so nothing was recorded \
     and the pixels below would be the harness's own blank target rather than a frame";

/// One frame of a renderer that has been handed **no scene at all**, recorded
/// in the phase the client is in while the replay world is being generated.
///
/// No scene is the honest fixture: the sibling scenario about a renderer asked
/// for *terrain* before a scene arrived is a refusal, and this is the same
/// renderer in the same state told that the scene is still coming. The pair is
/// what makes the phase — rather than what happens to be uploaded — the thing
/// deciding whether a frame draws.
fn render_while_preparing(
    context: &mc_testkit::frame::gpu::CaptureContext,
    name: &str,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let mut renderer = TerrainRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
        &support::production_textures(),
    )?;
    let request = support::request(context, name, COUNTING_FRAME)?;
    let snapshot = snapshot_of_nothing();
    let mut recorded = false;
    let captured;
    {
        let mut work = draw_fn(|encoder, color| {
            let target = RecordTarget {
                device: context.device(),
                queue: context.queue(),
                encoder,
                color,
                size: COUNTING_FRAME,
            };
            renderer.record_terrain(target, &ScenePhase::Preparing, &snapshot)?;
            recorded = true;
            Ok(())
        });
        captured = context.capture(&request, &mut work)?;
    }
    if !recorded {
        return Err(DRAW_WORK_NEVER_RAN.into());
    }
    Ok(captured.image)
}

/// A snapshot carrying no geometry, which is what the client holds for every
/// frame it draws before preparation finishes.
fn snapshot_of_nothing() -> TerrainSnapshot {
    TerrainSnapshot {
        tick: TICK,
        camera: replay_camera(),
        scene: Arc::new(SceneGeometry::default()),
    }
}

/// The replay's camera at tick 60.
fn replay_camera() -> CameraView {
    camera_view(REPLAY_EYE_AT_TICK_60, REPLAY_LOOK_AT)
}

/// Fails unless `admitted` is a proper, non-empty subset of the fixture whose
/// culled half carries quads of its own.
///
/// Both halves matter. Without the first, a renderer that admitted everything
/// would agree with the pure function; without the second, the quads of the
/// admitted sections would be all the quads there are and a compaction that
/// gathered the whole scene would produce the same total.
fn assert_partly_in_view(fixture: &Fixture, admitted: &[u32]) {
    let seen = support::quads_of(&fixture.scene, admitted);
    let all = support::quads_of_everything(&fixture.scene);
    assert!(
        !admitted.is_empty() && admitted.len() < SECTIONS as usize && seen > 0 && seen < all,
        "the fixture has to be partly in view and partly behind the camera, with quads on both \
         sides: {} of {SECTIONS} sections admitted, {seen} of {all} quads",
        admitted.len()
    );
}

/// Fails unless nothing in the fixture is visible and yet there was something to
/// draw.
fn assert_nothing_in_view(fixture: &Fixture, admitted: &[u32]) {
    let all = support::quads_of_everything(&fixture.scene);
    assert!(
        admitted.is_empty() && all > 0,
        "nothing may be in view while the scene still holds quads, or 'drew nothing' cannot be \
         told from 'had nothing to draw': {} admitted, {all} quads",
        admitted.len()
    );
}

/// The indices whose visibility flag came back set.
fn selected(flags: &[u32]) -> Vec<u32> {
    flags
        .iter()
        .enumerate()
        .filter(|(_, flag)| **flag != 0)
        .map(|(index, _)| index as u32)
        .collect()
}

/// Renders `fixture` from the replay's tick-60 camera into a counting frame.
fn render_counting(
    context: &mc_testkit::frame::gpu::CaptureContext,
    fixture: &Fixture,
    name: &str,
) -> Result<support::Rendered, Box<dyn Error>> {
    let mut renderer = support::prepared_renderer(context, fixture)?;
    let request = support::request(context, name, COUNTING_FRAME)?;
    let snapshot = support::snapshot(TICK, replay_camera(), fixture);
    support::render(context, &mut renderer, &snapshot, &request)
}

/// The centre pixel of the two stacked sections, handed over in `order`.
fn stacked_centre_pixel(
    context: &mc_testkit::frame::gpu::CaptureContext,
    order: Order,
    name: &str,
) -> Result<[u8; 3], Box<dyn Error>> {
    let fixture = stacked_sections(order)?;
    let mut renderer = support::prepared_renderer(context, &fixture)?;
    let request = support::request(context, name, DEPTH_FRAME)?;
    let snapshot = support::snapshot(TICK, stacked_camera(), &fixture);
    let frame = support::render(context, &mut renderer, &snapshot, &request)?;
    support::centre_pixel(&frame.image)
}

/// Two sections along the camera's view axis, each holding one solid block of a
/// different block, handed over in `order`.
fn stacked_sections(order: Order) -> Result<Fixture, Box<dyn Error>> {
    let nearer = (
        SectionOrigin::new(NEARER_SECTION),
        support::solid_block(BLOCK_AT, &BlockName::parse(NEARER)?),
    );
    let farther = (
        SectionOrigin::new(FARTHER_SECTION),
        support::solid_block(BLOCK_AT, &BlockName::parse(FARTHER)?),
    );
    match order {
        Order::NearerFirst => support::assemble(&[nearer, farther]),
        Order::FartherFirst => support::assemble(&[farther, nearer]),
    }
}

/// The camera the two stacked blocks are seen from: straight down `-Z`, with
/// both blocks centred on the view axis so each covers the frame's middle.
fn stacked_camera() -> CameraView {
    camera_view([8.5, 8.5, 16.0], [8.5, 8.5, 0.0])
}

/// Records a terrain pass on a renderer that was never handed a scene.
///
/// The encoder is never submitted: what is under test is the refusal, and a
/// refusal that reached the caller has already happened by the time the encoder
/// would matter.
fn record_without_a_scene(
    context: &mc_testkit::frame::gpu::CaptureContext,
    renderer: &mut TerrainRenderer,
    fixture: &Fixture,
) -> Result<(), FrameError> {
    let texture = support::unread_target(context.device(), COUNTING_FRAME);
    let color = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let snapshot = support::snapshot(TICK, replay_camera(), fixture);
    let phase = ScenePhase::Ready(Arc::clone(&fixture.scene));
    let target = RecordTarget {
        device: context.device(),
        queue: context.queue(),
        encoder: &mut encoder,
        color: &color,
        size: COUNTING_FRAME,
    };
    renderer.record_terrain(target, &phase, &snapshot)?;
    Ok(())
}
