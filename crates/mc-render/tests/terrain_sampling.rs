//! What the terrain sampler does to a captured picture, and what the device
//! does with a request it will not honour.
//!
//! # Why these exist beside the two readings of the constant
//!
//! `src/texture/sampler_test.rs` asserts what the sampler *asks for*. A test
//! that reads back the descriptor it caused to be built is agreement between two
//! copies of one decision: both halves move together the day somebody edits the
//! constant, and neither says anything about the picture. These three are the
//! other half — two captured consequences and one real device refusal. All four
//! are owed and none substitutes for another.
//!
//! # The magnified reading, and why "hard edge" is written as "no third colour"
//!
//! A placeholder layer is a checkerboard of one declared mean plus and minus one
//! fixed step, so a scanline across a magnified face passes through exactly two
//! colours. Nearest magnification shows each of them and nothing between them;
//! linear magnification interpolates between texel centres, so almost every
//! pixel of the same scanline becomes a blend that is neither. **The observable
//! is therefore the absence of a third colour, not the presence of an edge** —
//! an edge is one pixel of the scanline and a blend is most of it, so counting
//! what is not one of the two is the reading with the larger signal. The scanline
//! also has to show *both* colours, or a face drawn flat would satisfy it.
//!
//! # The minified reading, and the regime its fixture has to sit in
//!
//! Two captures half a texel apart, judged twice: once through the terrain
//! sampler and once through a request that minifies without filtering. What
//! separates them is that point sampling answers with *one* texel, so a
//! sub-texel camera movement flips whichever pixels crossed a texel boundary by
//! the full contrast between two texels, while linear minification blends four
//! and moves every pixel by a fraction of it.
//!
//! **That separation only exists where the texture still holds contrast at the
//! mip level the LOD selects, and no assertion can enforce that** — it is a
//! property of the fixture's geometry and of its texels together. Two things are
//! done about it rather than one. The texels are a *coarse* checkerboard, eight
//! texels to a square, so that levels 1, 2 and 3 hold the same two colours at
//! full contrast and only the 1 x 1 level averages them away — a placeholder
//! layer would be useless here, because a checkerboard of period two averages to
//! its own mean at every level below the first. And the unfiltered pair is
//! required to differ at all before the comparison is read, so a fixture that
//! drifted out of the regime fails loudly instead of reporting that filtering
//! helped when neither configuration could see anything.

mod support;

use std::error::Error;

use mc_core::content::TEXTURE_EDGE;
use mc_core::id::{BlockName, TextureKey};
use mc_render::camera::{CameraView, camera_view};
use mc_render::geometry::SectionOrigin;
use mc_render::gpu::{RendererError, TerrainTextures, terrain_sampler};
use mc_render::surface::SurfaceSize;
use mc_render::texture::placeholder::placeholder_texels;
use mc_render::texture::sampler::{Filter, SamplerRequest, TERRAIN_SAMPLER};
use mc_render::texture::supplied::SuppliedTexels;
use mc_testkit::frame::{Rgba8Image, Thresholds, compare};
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};

use support::{Fixture, TestResult};

/// The tick every capture here is labelled with. The scenes are static and the
/// poses are declared, so nothing about any picture depends on it.
const TICK: u32 = 0;

/// The block the magnified face is made of, in the fixture namespace a renderer
/// test uses so that nothing here borrows a shipped block's name.
const MAGNIFIED: &str = "example:magnified";

/// The block the minified wall is made of.
const MINIFIED: &str = "example:minified";

/// The frame the magnified face is drawn into. Square, so the face's projection
/// is square and the scanline below runs along a row of equal-width texels.
const MAGNIFIED_FRAME: SurfaceSize = SurfaceSize {
    width: 256,
    height: 256,
};

/// The frame the minified wall is drawn into.
const MINIFIED_FRAME: SurfaceSize = SurfaceSize {
    width: 128,
    height: 72,
};

/// Where the magnified face's own voxel sits inside its section.
const MAGNIFIED_VOXEL: [u32; 3] = [8, 8, 8];

/// How far the eye stands in front of the magnified face, in blocks.
///
/// **Derived from the lens, not chosen.** The projection's vertical focal length
/// is `cot 30° = √3`, so a face one block tall standing `d` blocks away covers
/// `√3 / d` of the frame's half-height. At `d = 1.5` that is 0.577, so the face
/// spans 57.7% of a 256-pixel frame either side of its centre — 147 pixels for
/// 16 texels, or **9.2 pixels to a texel**. One texel covering more than one
/// screen pixel is the condition the scenario names, and nine of them is enough
/// that a blend and a step cannot be confused with a rounding difference.
const MAGNIFIED_DISTANCE: f32 = 1.5;

/// The columns the scanline runs between, and the row it runs along.
///
/// The face's projected edges land at 54.1 and 201.9 on a 256-pixel frame
/// (`(1 ± 0.577) · 128`). The scan is inset 16 pixels from each of them so that
/// no pixel it reads is a pixel where the face meets the sky — a silhouette
/// pixel is neither of the two texel colours whatever the sampler does, and
/// would be counted as a blend.
const SCAN_FROM: u32 = 70;
const SCAN_TO: u32 = 186;
const SCAN_ROW: u32 = 128;

/// How far a pixel may sit from the texel colour it was drawn from, in ΔE.
///
/// **Derived from both directions rather than loosened until green.** The array
/// texture and the colour target are both sRGB, so a nearest-sampled texel is
/// decoded to linear light and encoded back and the round trip is the byte it
/// started as; the tolerance is for that encode and for nothing else. Two texels
/// of opposite parity stand about ΔE 7 apart, so a blend halfway between them
/// sits about 3.5 from each — a tolerance of 2 is comfortably under that and
/// comfortably over a one-unit encode difference.
const SAME_TEXEL: f64 = 2.0;

/// How far apart two captures' pixels have to stand to be counted as differing.
///
/// The same ΔE 10 this project calls two colours told apart everywhere else. It
/// sits above what linear minification moves a pixel by when the camera shifts
/// half a texel — a fraction of the contrast between two texels — and far below
/// the whole of that contrast, which is what point sampling moves a pixel by
/// when its sample crosses a texel boundary.
const MOVED: f64 = 10.0;

/// The two colours the minified wall's texels are made of, and how many texels
/// wide one square of them is.
///
/// **Eight, so the contrast survives minification.** Halving a checkerboard of
/// period two averages the pair away at the very first level; period sixteen —
/// eight texels to a square, two squares to an edge — keeps both colours at full
/// contrast through levels 1, 2 and 3, and only the single-texel level averages
/// them. The two colours are far apart on every channel so that the contrast the
/// reading rests on is not a matter of a few units.
const DARK: [u8; 4] = [24, 24, 24, 255];
const LIGHT: [u8; 4] = [232, 232, 232, 255];
const SQUARE_TEXELS: u32 = 8;

/// The two capture names the minified pair is taken under.
///
/// One pair of names for both sampler configurations: the second run overwrites
/// the first's artifacts, and nothing here reads one back.
const ONE_EYE: &str = "terrain-minified-one";
const THE_OTHER_EYE: &str = "terrain-minified-other";

/// How many blocks the minified wall is on a side.
const WALL_BLOCKS: u32 = 16;

/// How far the eye stands in front of that wall, in blocks.
///
/// **Chosen to land the minified sample between mip levels, and the arithmetic
/// is here so a later reader can move it deliberately.** The wall's texture
/// repeats once per block, so a face `p` pixels wide shows `16 / p` texels to a
/// pixel. A 72-pixel-high frame covers `2 tan 30° · d = 1.1547 d` blocks, so a
/// block is `62.35 / d` pixels and a pixel covers `0.2566 d` texels. At `d = 22`
/// that is 5.65 texels to a pixel, or a level-of-detail of **2.50** — where the
/// levels either side of it still hold both of the wall's colours.
const MINIFIED_DISTANCE: f32 = 22.0;

/// How far the eye moves between the two minified captures, in blocks.
///
/// Half a texel: the texture repeats once per block and holds sixteen texels, so
/// a texel is a sixteenth of a block and half of one is a thirty-second.
const HALF_A_TEXEL: f32 = 1.0 / 32.0;

/// A request that minifies without filtering — today's sampler, kept here as the
/// thing the terrain sampler is measured against.
const UNFILTERED: SamplerRequest = SamplerRequest {
    magnify: Filter::Nearest,
    minify: Filter::Nearest,
    between_levels: Filter::Nearest,
    anisotropy: 1,
};

/// A request the device refuses: anisotropy asked for beside a nearest filter.
///
/// `wgpu-core-30.0.0/src/device/resource.rs:2288-2316` refuses a clamp above one
/// unless all three filters are linear, in three separate arms — so this fails
/// the first of them.
const REFUSED: SamplerRequest = SamplerRequest {
    magnify: Filter::Nearest,
    minify: Filter::Nearest,
    between_levels: Filter::Nearest,
    anisotropy: 16,
};

#[test]
fn a_magnified_face_shows_its_two_texel_colours_and_nothing_between_them() -> TestResult {
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let colors = two_texel_colors(&TextureKey::parse(MAGNIFIED)?)?;

    let fixture = magnified_face()?;
    let frame = captured(
        &context,
        &Shot {
            fixture: &fixture,
            textures: &TerrainTextures {
                supplied: &SuppliedTexels::none(),
                sampler: TERRAIN_SAMPLER,
            },
            camera: magnified_camera(),
            size: MAGNIFIED_FRAME,
            name: "terrain-magnified-nearest",
        },
    )?;
    let scanned = scanline(&frame, &colors)?;

    assert_eq!(
        (scanned.between, scanned.shown, scanned.read),
        (0, colors.len(), (SCAN_TO - SCAN_FROM) as u64),
        "one texel covers 9.2 screen pixels here, so a scanline across the face passes through \
         about thirteen of them. Nearest magnification draws each texel as itself and steps \
         between them; linear magnification interpolates between texel centres and turns almost \
         the whole scanline into blends that are neither colour. Both colours have to appear as \
         well, or a face drawn flat would satisfy the first half. The scanline read {scanned:?} \
         against the declared texel colours {colors:?}"
    );
    Ok(())
}

#[test]
fn moving_the_eye_half_a_texel_moves_fewer_distant_pixels_through_the_terrain_sampler() -> TestResult
{
    let Some(context) = support::device()? else {
        return Ok(());
    };
    let fixture = minified_wall()?;
    let texels = coarse_checkerboard(&TextureKey::parse(MINIFIED)?);

    let filtered = moved_pixels(&context, &fixture, &texels, TERRAIN_SAMPLER)?;
    let unfiltered = moved_pixels(&context, &fixture, &texels, UNFILTERED)?;

    require(
        unfiltered > 0,
        format!(
            "this comparison is only about filtering where the unfiltered pair can see the \
             camera move at all. It moved {unfiltered} pixels, so the wall is either off the \
             frame or minified past the level where its two colours still differ, and a filtered \
             count of {filtered} would be reporting the fixture rather than the sampler"
        ),
    )?;
    assert!(
        filtered < unfiltered,
        "point sampling answers with one texel, so a camera moving half a texel flips every \
         pixel whose sample crossed a texel boundary by the whole contrast between two of them. \
         Linear minification blends four texels and interpolates between two mip levels, so the \
         same movement moves every pixel by a fraction of that contrast and past ΔE {MOVED} it \
         moves far fewer of them. Through the terrain sampler {filtered} pixels moved; \
         unfiltered, {unfiltered} did"
    );
    Ok(())
}

#[test]
fn a_sampler_the_device_will_not_build_refuses_the_launch_naming_what_was_requested() -> TestResult
{
    let Some(context) = support::device()? else {
        return Ok(());
    };

    let refused = terrain_sampler(context.device(), &REFUSED);

    let Err(RendererError::TerrainSampler { requested }) = refused else {
        return Err(format!(
            "a sampler asking for anisotropy beside a nearest filter is one the device refuses, \
             and the refusal is the device's rather than a rule copied out of it — a pre-check \
             written on this side is a second copy of a vendor constraint that drifts silently \
             the day the vendor changes it. The device answered {refused:?}"
        )
        .into());
    };
    let said = RendererError::TerrainSampler { requested }.to_string();
    assert!(
        said.contains(&REFUSED.to_string()),
        "whoever meets this refusal has one thing to change and it is the combination they asked \
         for, so the message has to carry it: it said `{said}` and the request reads `{REFUSED}`"
    );
    Ok(())
}

/// What a scanline across a magnified face found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Scanline {
    /// How many pixels it read.
    read: u64,
    /// How many of them are neither of the declared texel colours.
    between: u64,
    /// How many of those colours it saw at all.
    shown: usize,
}

/// How the pixels of one row of `frame` stand against `colors`.
fn scanline(frame: &Rgba8Image, colors: &[[u8; 3]]) -> Result<Scanline, Box<dyn Error>> {
    let mut found: Vec<[u8; 3]> = Vec::new();
    let mut seen = Scanline {
        read: 0,
        between: 0,
        shown: 0,
    };
    for x in SCAN_FROM..SCAN_TO {
        let [red, green, blue, _] = frame
            .pixel(x, SCAN_ROW)
            .ok_or_else(|| format!("the captured frame has no pixel at ({x}, {SCAN_ROW})"))?;
        seen.read += 1;
        match nearest_of(colors, [red, green, blue])? {
            Some(color) if !found.contains(&color) => found.push(color),
            Some(_) => {}
            None => seen.between += 1,
        }
    }
    seen.shown = found.len();
    Ok(seen)
}

/// Which of `colors` `shown` stands within [`SAME_TEXEL`] of, or nothing where
/// it stands that close to none of them.
fn nearest_of(colors: &[[u8; 3]], shown: [u8; 3]) -> Result<Option<[u8; 3]>, Box<dyn Error>> {
    for color in colors {
        if support::delta_e(shown, *color)? <= SAME_TEXEL {
            return Ok(Some(*color));
        }
    }
    Ok(None)
}

/// The two colours `key`'s placeholder layer is made of, in a stable order.
///
/// Derived from the generator, which is a pure function of the key, and never
/// from a frame. The count is checked rather than assumed: a layer that turned
/// out to hold some other number of colours would make "no third colour" mean
/// something else entirely.
fn two_texel_colors(key: &TextureKey) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
    let mut distinct: Vec<[u8; 3]> = placeholder_texels(key, TEXTURE_EDGE)
        .into_iter()
        .map(|[red, green, blue, _]| [red, green, blue])
        .collect();
    distinct.sort_unstable();
    distinct.dedup();
    require(
        distinct.len() == 2,
        format!(
            "a placeholder layer is a checkerboard of one mean plus and minus one step, so it \
             holds two colours and the reading below is about a scanline passing through both. \
             `{key}`'s holds {count}: {distinct:?}",
            key = key.as_str(),
            count = distinct.len()
        ),
    )?;
    Ok(distinct)
}

/// A texture of two colours in squares [`SQUARE_TEXELS`] on a side.
///
/// Stated rather than generated from the key: what this fixture needs is
/// contrast that survives three halvings, and a placeholder layer has the
/// opposite property by construction.
fn coarse_checkerboard(key: &TextureKey) -> SuppliedTexels {
    let texels = (0..TEXTURE_EDGE)
        .flat_map(|row| (0..TEXTURE_EDGE).map(move |column| square_of(row, column)))
        .collect();
    SuppliedTexels::stating([(key.clone(), texels)])
}

/// Which of the two colours the texel at `row`, `column` holds.
fn square_of(row: u32, column: u32) -> [u8; 4] {
    let squares = row.div_euclid(SQUARE_TEXELS) + column.div_euclid(SQUARE_TEXELS);
    if squares.is_multiple_of(2) {
        LIGHT
    } else {
        DARK
    }
}

/// One solid block, as the six faces it shows.
fn magnified_face() -> Result<Fixture, Box<dyn Error>> {
    let block = BlockName::parse(MAGNIFIED)?;
    support::assemble(&[(
        SectionOrigin::new([0, 0, 0]),
        support::solid_block(MAGNIFIED_VOXEL, &block),
    )])
}

/// A wall of `WALL_BLOCKS` squared facing the camera, as one merged quad.
///
/// One quad rather than a block each: plane coordinates run in whole blocks and
/// the sampler repeats, so a merged face shows the texture once per block — which
/// is the same texel density a wall of separate blocks would show, at a fraction
/// of the geometry.
fn minified_wall() -> Result<Fixture, Box<dyn Error>> {
    let block = BlockName::parse(MINIFIED)?;
    support::assemble(&[(
        SectionOrigin::new([0, 0, 0]),
        vec![Quad {
            facing: Facing::PosZ,
            plane: 0,
            origin: PlanePos {
                primary: 0,
                secondary: 0,
            },
            extent: PlaneExtent {
                primary: WALL_BLOCKS,
                secondary: WALL_BLOCKS,
            },
            block,
        }],
    )])
}

/// How many pixels move between two captures of `fixture` taken half a texel
/// apart, through `sampler`.
fn moved_pixels(
    context: &mc_testkit::frame::gpu::CaptureContext,
    wall: &Fixture,
    texels: &SuppliedTexels,
    sampler: SamplerRequest,
) -> Result<u64, Box<dyn Error>> {
    let textures = TerrainTextures {
        supplied: texels,
        sampler,
    };
    let shot = |across: f32, name: &'static str| Shot {
        fixture: wall,
        textures: &textures,
        camera: minified_camera(across),
        size: MINIFIED_FRAME,
        name,
    };
    let one = captured(context, &shot(0.0, ONE_EYE))?;
    let other = captured(context, &shot(HALF_A_TEXEL, THE_OTHER_EYE))?;
    let thresholds = Thresholds::new(MOVED, 1.0, f64::MAX)?;
    Ok(compare(&one, &other, &thresholds).failing_pixels)
}

/// One capture of `fixture` at `size`, drawn through `textures` from `camera`.
fn captured(
    context: &mc_testkit::frame::gpu::CaptureContext,
    shot: &Shot<'_>,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let mut renderer = support::prepared_renderer_through(context, shot.fixture, shot.textures)?;
    let request = support::request(context, shot.name, shot.size)?;
    let snapshot = support::snapshot(TICK, shot.camera, shot.fixture);
    Ok(support::render(context, &mut renderer, &snapshot, &request)?.image)
}

/// One capture, as one parameter.
///
/// Borrowed and passed whole so that no function on this path exceeds four
/// arguments, which is the same reason `TerrainTextures` is one value rather
/// than two.
struct Shot<'a> {
    fixture: &'a Fixture,
    textures: &'a TerrainTextures<'a>,
    camera: CameraView,
    size: SurfaceSize,
    name: &'a str,
}

/// The eye standing square in front of the magnified face's own +Z side.
fn magnified_camera() -> CameraView {
    let [x, y, z] = MAGNIFIED_VOXEL.map(|coordinate| coordinate as f32 + 0.5);
    camera_view([x, y, z + 0.5 + MAGNIFIED_DISTANCE], [x, y, z - 0.5])
}

/// The eye standing square in front of the minified wall, `across` blocks to one
/// side of its centre.
fn minified_camera(across: f32) -> CameraView {
    let centre = WALL_BLOCKS as f32 * 0.5;
    camera_view(
        [centre + across, centre, MINIFIED_DISTANCE],
        [centre + across, centre, 0.0],
    )
}

/// Fails with `explanation` unless `holds`.
///
/// A fixture that does not have the property an assertion rests on is a broken
/// fixture rather than a failed behaviour, and it says so before the assertion
/// runs.
fn require(holds: bool, explanation: String) -> Result<(), Box<dyn Error>> {
    if holds {
        Ok(())
    } else {
        Err(explanation.into())
    }
}
