//! What fills a layer after a reload, on one renderer that lived through both
//! uploads.
//!
//! # The decision this is the only guard on
//!
//! The texels a content root's built art offers are handed to the renderer
//! **once, at construction**, and are deliberately *not* carried by `Unuploaded`
//! or by the re-mesh worker's retirement. The built set is a pre-build artefact
//! that does not change while the client runs, so a reload appending a key finds
//! either art that was already read or no art at all — and the second of those is
//! the ordinary fallback reached by a second road.
//!
//! Thread a supply through the reload path instead and there is a value that can
//! arrive **empty**. A reload would then quietly re-fill every layer from the
//! generator, and a world that had been drawing its baked art would go back to
//! drawing hash-derived colours at the moment somebody saved a block file.
//! Nothing else in this phase would report it: every other reading here takes its
//! texels from a launch, and a launch is exactly the path that still works.
//!
//! # One renderer, two uploads, and that is the whole fixture
//!
//! `reload_draws_the_new_block.rs` builds a **fresh** renderer per frame, which
//! is right for what it asks — whether the appended layer was filled at all — and
//! is precisely why it cannot see this: a renderer that never survives an upload
//! cannot lose anything between two of them. So this run constructs one renderer,
//! gives it the supply the content root offers, uploads the pre-reload layers,
//! draws; then uploads the layers a reload handed over **on that same renderer**
//! and draws the same pose again.
//!
//! `upload_textures` re-fills every layer it is given, so the second upload is
//! where a replaced supply would show: stone's layer would be rewritten from the
//! generator.
//!
//! # What it does not cover, said here rather than left to be found
//!
//! It uploads through `TerrainRenderer::upload_textures` rather than through
//! `Unuploaded::uploaded_to`, which takes a `FrameRenderer` the frame path
//! constructs and nothing in this workspace does. `upload.rs`'s own header
//! records that gap and records that the compiler is what covers the omission
//! there — the `#[must_use]` wrapper makes skipping the upload a build failure.
//! What is left uncovered is a `FrameRenderer` that lost its supply while a
//! `TerrainRenderer` kept it, and the two share one field.
//!
//! # Where the camera stands
//!
//! The floor's own upward face, one block below the cell
//! `reload_draws_the_new_block.rs` places into, so nothing is placed here and the
//! square at the centre shows stone in both frames. The eye stands three blocks
//! over the face's centre and `3 · tan 30° = 1.732` along `+x`, so the view is 30°
//! off vertical — straight down, a look-at matrix has no unique answer, because
//! the view direction is the world's own up. That is 3.464 blocks from the face
//! and 180 pixels to a block, so the 32-pixel square at the centre sits well
//! inside a face 180 × 156 pixels across.
//!
//! **The square is not asked to show every colour stone holds.** It spans about
//! 2.8 of the sixteen texels across a block, and stone's two accent shades are
//! 16.8% of its texels each, so a run in which one of them misses the square is a
//! correct run. What is asked is that every pixel *is* one of the supplied
//! colours and *none* is one of the generated ones — which the ΔE 55.56 between
//! the two palettes makes an enormous margin rather than a fine one.

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
use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;
use mc_render::camera::camera_view;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::gpu::{TerrainRenderer, TerrainTextures};
use mc_render::pass::TerrainPassConfig;
use mc_render::texture::placeholder::placeholder_texels;
use mc_render::texture::sampler::TERRAIN_SAMPLER;
use mc_render::texture::supplied::SuppliedTexels;
use mc_sim::world::World;
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::CaptureContext;

use reload::{AMBER_FILE, STONE, amber, shipped};
use reload_upload::{declaring_after_launch, layers_handed_over, until_taken_up};
use reload_watch::a_client_on;
use reload_world::{floor_of, registry_of};
use support::art::{built_texels, drawn_colors};
use support::hud_frames::Rect;
use support::swatch::{require, swatch_reading};
use support::{TestResult, frames};

/// The tick both frames are drawn at. Nothing about either picture depends on it.
const A_TICK: u32 = 0;

/// What the camera looks at, and where it stands — derived in this file's header.
const THE_STONE_FACE: [f32; 3] = [9.5, 10.0, 8.5];
const OVER_THE_STONE_FACE: [f32; 3] = [11.232, 13.0, 8.5];

/// The square at the centre of the frame, which the floor's upward face covers.
const THE_MIDDLE_OF_THE_FACE: Rect = Rect {
    x: 624,
    y: 344,
    width: 32,
    height: 32,
};

#[test]
fn a_reload_that_appends_a_key_finds_the_supply_the_renderer_was_constructed_with() -> TestResult {
    let Some(context) = frames::device()? else {
        return Ok(());
    };
    let stone = TextureKey::parse(STONE)?;

    let Drawn {
        supplied,
        before,
        after,
    } = a_run_that_reloads_between_two_frames(&context, &stone)?;

    let generated = distinct_of(&placeholder_texels(&stone, TEXTURE_EDGE));
    require_the_two_palettes_differ(&supplied, &generated)?;
    let read = the_square_read_three_ways(&before, &after, &supplied, &generated)?;

    assert_eq!(
        read,
        SquareReading {
            strayed_from_the_image: 0,
            strayed_from_the_generator: THE_MIDDLE_OF_THE_FACE.area(),
            strayed_before_the_reload: 0,
            the_two_frames_agree: true,
            considered: THE_MIDDLE_OF_THE_FACE.area(),
        },
        "{THE_SUPPLY_SURVIVES_A_RELOAD}"
    );
    Ok(())
}

/// What the square at the centre of the two frames came to.
///
/// Named rather than a tuple of five, because a failure naming
/// `strayed_from_the_generator` says what went wrong and a failure naming `.1`
/// does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SquareReading {
    /// Pixels of the second frame that are none of the colours the built image
    /// holds.
    strayed_from_the_image: u64,
    /// Pixels of it that are none of the colours the generator makes — the whole
    /// square, where the art is what reached the layer.
    strayed_from_the_generator: u64,
    /// The same reading as the first, taken before the reload.
    strayed_before_the_reload: u64,
    /// Whether the square is pixel-for-pixel the same in both frames.
    the_two_frames_agree: bool,
    /// How many pixels were read, so a square that fell off a frame cannot make
    /// "nothing strayed" true by looking at nothing.
    considered: u64,
}

/// What the reading above is about, kept beside it rather than inside it — for
/// the reason `hud_held_block.rs` gives: a line limit hit while doing something
/// else rejects whatever is cheapest to drop, which is always the explanation.
const THE_SUPPLY_SURVIVES_A_RELOAD: &str = "the supply is given to the renderer once and a reload does not carry one, so the second upload \
     re-fills stone's layer from the same texels the first did. A supply threaded through the \
     reload path is a value that can arrive empty — and then this square goes from the baked greys \
     to the generated teal at the moment somebody saves a block file, which is the second \
     element. The fourth is the same claim from the other side: nothing about this face moved \
     across the reload at all";

/// How far the square strays from the supplied colours after the reload, from
/// the generated ones after it, and from the supplied ones before it; whether the
/// two frames agree over it; and how many pixels were read.
///
/// # Errors
///
/// Returns the distance metric's own failure, or a square that fell off a frame.
fn the_square_read_three_ways(
    before: &Rgba8Image,
    after: &Rgba8Image,
    supplied: &[[u8; 3]],
    generated: &[[u8; 3]],
) -> Result<SquareReading, Box<dyn Error>> {
    let from_the_image = swatch_reading(after, THE_MIDDLE_OF_THE_FACE, supplied)?;
    let from_the_generator = swatch_reading(after, THE_MIDDLE_OF_THE_FACE, generated)?;
    let unmoved = swatch_reading(before, THE_MIDDLE_OF_THE_FACE, supplied)?;
    Ok(SquareReading {
        strayed_from_the_image: from_the_image.strayed,
        strayed_from_the_generator: from_the_generator.strayed,
        strayed_before_the_reload: unmoved.strayed,
        the_two_frames_agree: pixels_of(before, THE_MIDDLE_OF_THE_FACE)?
            == pixels_of(after, THE_MIDDLE_OF_THE_FACE)?,
        considered: from_the_image.considered,
    })
}

/// What one run produced: the colours the content root's art offers for the key
/// under test, and the same pose drawn either side of a reload.
struct Drawn {
    supplied: Vec<[u8; 3]>,
    before: Rgba8Image,
    after: Rgba8Image,
}

/// A run that launches over a stone floor, draws it, has an author declare a
/// block, and draws the same pose again — through **one** renderer.
///
/// Nothing is placed. The reload appends a key and the frame keeps showing the
/// floor, which is what makes the two frames comparable at all: what changed
/// between them is an upload and nothing else.
///
/// # Errors
///
/// Returns the read, world, mesh, packing, pipeline, upload or capture failure,
/// and the refusal where no candidate was taken up.
fn a_run_that_reloads_between_two_frames(
    context: &CaptureContext,
    key: &TextureKey,
) -> Result<Drawn, Box<dyn Error>> {
    let root = shipped()?;
    let texels = built_texels(root.path())?;
    require_the_art_covers_it(key, &texels)?;
    let supplied = drawn_colors(key, &texels);

    let at_launch = registry_of(root.path())?;
    let blocks = floor_of(&at_launch, STONE)?;
    let (mut client, reports) = a_client_on(&root, STONE)?;
    let meshed = World::new(blocks, at_launch)?.mesh()?;
    let before_resolution = reload_remesh::resolution_serving(&client)?;
    let scene = Arc::new(scene_of(&meshed, &before_resolution)?);

    let mut renderer = one_renderer_holding(context, &texels)?;
    renderer.upload_scene(context.queue(), &scene)?;
    renderer.upload_textures(context.queue(), before_resolution.layers())?;
    let before = drawn(context, &mut renderer, &scene, "reload-supply-before")?;

    let declared = declaring_after_launch(&root, AMBER_FILE, &amber())?;
    reports.changed(&[declared])?;
    let appended = layers_handed_over(until_taken_up(&mut client))?;
    renderer.upload_textures(context.queue(), appended.stated().layers())?;
    let after = drawn(context, &mut renderer, &scene, "reload-supply-after")?;

    Ok(Drawn {
        supplied,
        before,
        after,
    })
}

/// Fails unless the content root's built art covers `key`.
///
/// **The premise the whole run rests on.** A key the set covers nothing for is
/// filled from the generator on *both* sides of the reload, so the reading would
/// be comparing the fallback against itself and would go green for a reason that
/// has nothing to do with a supply surviving anything.
///
/// # Errors
///
/// Returns that failure, naming the key.
fn require_the_art_covers_it(
    key: &TextureKey,
    texels: &SuppliedTexels,
) -> Result<(), Box<dyn Error>> {
    require(
        texels.covering(key).is_some(),
        format!(
            "this run is about a key the content root's built art covers, and it covers nothing \
             for `{key}`. Its layer would then be filled from the generator on both sides of the \
             reload, and the reading below would be comparing the fallback against itself",
            key = key.as_str()
        ),
    )
}

/// The one renderer both frames are drawn through, holding `texels` for the
/// whole run.
///
/// **One, and that is the fixture.** A renderer rebuilt between the two uploads
/// could not lose anything between them, which is exactly why the sibling suite
/// that rebuilds one per frame cannot ask this question.
///
/// # Errors
///
/// Returns the pipeline or sampler failure.
fn one_renderer_holding(
    context: &CaptureContext,
    texels: &SuppliedTexels,
) -> Result<TerrainRenderer, Box<dyn Error>> {
    Ok(TerrainRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
        &TerrainTextures {
            supplied: texels,
            sampler: TERRAIN_SAMPLER,
        },
    )?)
}

/// One frame of `scene` through the declared pose, on a renderer already loaded.
///
/// # Errors
///
/// Returns the recording or capture failure.
fn drawn(
    context: &CaptureContext,
    renderer: &mut TerrainRenderer,
    scene: &Arc<SceneGeometry>,
    name: &str,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let snapshot = frames::snapshot(
        A_TICK,
        camera_view(OVER_THE_STONE_FACE, THE_STONE_FACE),
        scene,
    );
    let mut frame = frames::ReplayFrame {
        context,
        renderer,
        snapshot: &snapshot,
    };
    frame.capture(&frames::request(context, name)?)
}

/// The distinct colours of `texels`, ascending.
fn distinct_of(texels: &[[u8; 4]]) -> Vec<[u8; 3]> {
    let mut colors: Vec<[u8; 3]> = texels
        .iter()
        .map(|[red, green, blue, _]| [*red, *green, *blue])
        .collect();
    colors.sort_unstable();
    colors.dedup();
    colors
}

/// Every pixel of `rect`, in reading order.
///
/// # Errors
///
/// Returns a failure naming the first position the frame does not have, which is
/// a rect that fell off the frame rather than a picture that is wrong.
fn pixels_of(frame: &Rgba8Image, rect: Rect) -> Result<Vec<[u8; 4]>, Box<dyn Error>> {
    let mut found = Vec::with_capacity(rect.area() as usize);
    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            found.push(
                frame
                    .pixel(x, y)
                    .ok_or_else(|| format!("the captured frame has no pixel at ({x}, {y})"))?,
            );
        }
    }
    Ok(found)
}

/// Fails unless no colour of the built art reads as a colour of the generated
/// texture for the same key.
///
/// The reading above says the square is made *entirely* of one palette and of
/// *none* of the other, which is only derivable when a pixel cannot belong to
/// both. Measured on the shipped art: the two stand ΔE 55.56 apart at their
/// means, so this is a check that the fixture is still the fixture rather than a
/// margin anybody has to think about.
fn require_the_two_palettes_differ(
    supplied: &[[u8; 3]],
    generated: &[[u8; 3]],
) -> Result<(), Box<dyn Error>> {
    for shown in supplied {
        for other in generated {
            require(
                support::probe::distance(*shown, *other)? > support::swatch::SAME_COLOR,
                format!(
                    "the built art and the generated texture for the same key have to share no \
                     colour, or a pixel of the square could satisfy both halves of the reading \
                     below at once: {shown:?} against {other:?}"
                ),
            )?;
        }
    }
    require(
        supplied.len() > 1,
        format!(
            "the art the set offers has to hold more than one colour for 'every pixel is one of \
             them' to be about a texture rather than about a fill: it holds {}",
            supplied.len()
        ),
    )
}
