//! What the shore draws where the sea stands behind it, at every tick the
//! goldens are shot from.
//!
//! # Which pixels are examined, and the stronger rule that has no witness here
//!
//! A face is examined when its ray meets it without crossing the sea, and it
//! stands nearer the eye than the farthest run of the sea the same frame draws —
//! *an opaque block nearer the camera than a translucent one*, read over the
//! frame, which is how the requirement is worded.
//!
//! **The stronger reading was tried first and is not available on this path, and
//! that is measured rather than assumed.** A blended pass ignoring the depth the
//! opaque pass wrote would show itself where an opaque face and the sea land on
//! *one ray*, the nearer one hiding the further. Over all three declared
//! captures the number of declared samples in that position is **zero**: the
//! shore meets the sea along a flat edge, so a ray that stops at the bank
//! continues into the ground rather than out over the water. There is no
//! geometry on the declared path that overlaps the two in screen space, so this
//! reading cannot witness a depth test that was switched off. What can is
//! FR-2.2-S1's pane fixture, where a blocker is placed in front of a pane on
//! purpose.
//!
//! **What this reading does witness** is a partition or a pass that blended
//! faces nobody declared translucent: 245, 241 and 123 opaque faces at the three
//! ticks, each nearer than water the same frame draws, every one of them
//! required to hold its own colour.
//!
//! # It is absolute and it is judged at three poses on the declared path
//!
//! Each expected colour comes from the art on disk and the degree the content
//! declares, never from another rendering. The three poses are the player's own
//! at the ticks the golden set is declared at, so what this reads is the walk a
//! player takes rather than a pose chosen to make the reading easy.
//!
//! # Where the tolerance comes from, in both directions
//!
//! Measured over these three captures with the sea at half a degree. **The
//! floor**: the worst an examined sample stands from the nearest colour its own
//! face can show at any distance is **ΔE 4.96**. That is not renderer error — it
//! is a grass side, four fifths dirt with a strip of turf across its top, seen
//! at the distance where a reduced texel is neither a texel nor the layer's
//! mean. **The ceiling**: the nearest a face's own colours stand from the same
//! face seen through one run of the sea is **ΔE 24.12**, measured over every
//! examined sample at every tick. Past that a pixel showing the blend would be
//! accepted as showing the face.
//!
//! **9.0 sits in that bracket**, 4.04 above the floor and 15.12 below the
//! ceiling, and the lower half is asserted on every run below rather than left
//! to this paragraph.

mod support;

use std::error::Error;
use std::sync::Arc;

use mc_core::id::BlockName;
use mc_render::camera::camera_view;
use mc_sim::camera::CameraPose;
use mc_testkit::frame::Rgba8Image;
use mc_world::mesh::Facing;

use support::composite::{Palette, nearest_between};
use support::frames::{CAPTURE_SIZE, ReplayFrame};
use support::goldens::DECLARED_TICKS;
use support::oracle::{self, Crossed, CrossedSample, Surface, Voxels};
use support::probe::pixel_color;
use support::{TestResult, prepare_scene};

/// The block the shipped world's only translucent body is made of.
const SEA: &str = "base:water";

/// How far a pixel of a nearer opaque face may sit from the nearest colour that
/// face can show, in ΔE.
///
/// Derived from both directions in this module's header: above the ΔE 4.96 a
/// correct frame is measured to be off by, below the ΔE 24.12 that separates a
/// face's own colours from the same face seen through the sea.
const KEEPS_ITS_OWN_COLOUR: f64 = 9.0;

#[test]
fn every_declared_capture_draws_the_shore_in_front_of_the_sea_with_nothing_of_the_sea_in_it()
-> TestResult {
    let Some(examined) = the_shore_in_each_capture()? else {
        return Ok(());
    };

    assert_eq!(
        examined,
        DECLARED_TICKS
            .iter()
            .map(|tick| Nearer {
                tick: *tick,
                draws_something_that_passes_light: SOMETHING,
                opaque_faces_standing_in_front_of_the_sea: SOMETHING,
                drawn_with_the_sea_mixed_in: Vec::new(),
                too_near_the_blend_they_must_not_be: Vec::new(),
            })
            .collect::<Vec<_>>(),
        "the sea is drawn in a second pass that tests the depth the first one wrote, so an opaque \
         face standing between the camera and the sea keeps its own colour and takes nothing of \
         the water into it. The first field is this reading's premise and not its subject: a frame \
         drawing nothing that passes light has no blended pass to have gone wrong, so a reading \
         over it would report a clean result it had not earned. The second is the same premise one \
         step in — an opaque face with the sea behind it is the only place the defect can show. A \
         pixel listed in the third is the sea mixed into a face that hides it, which is a blended \
         draw ignoring depth. The fourth is the reading's own tolerance: a face whose own colours \
         stand within ΔE {KEEPS_ITS_OWN_COLOUR} of the same face seen through the sea is one this \
         reading could not tell apart either way"
    );
    Ok(())
}

/// What one declared capture came to.
///
/// Both counts are reported as "one or more" rather than as themselves, because
/// how many such faces a frame holds is a property of where the walk happens to
/// take the camera. Committing the number would make a moved spawn a failure of
/// this reading rather than of the thing that moved; what must not vary is that
/// it is not zero.
#[derive(Debug, PartialEq, Eq)]
struct Nearer {
    tick: u16,
    draws_something_that_passes_light: &'static str,
    opaque_faces_standing_in_front_of_the_sea: &'static str,
    /// Every examined pixel drawn further than the tolerance from anything its
    /// own face can show, named with what it drew.
    drawn_with_the_sea_mixed_in: Vec<String>,
    /// Every examined face whose own colours stand within the tolerance of the
    /// same face seen through one run of the sea.
    too_near_the_blend_they_must_not_be: Vec<String>,
}

/// What a capture holding one or more of the thing in question reports.
const SOMETHING: &str = "one or more";

/// What a capture holding none reports.
const NOTHING: &str = "none at all";

/// Each declared capture drawn from the player's own camera, or `None` when the
/// opt-in permitted the absence of a device.
fn the_shore_in_each_capture() -> Result<Option<Vec<Nearer>>, Box<dyn Error>> {
    let prepared = prepare_scene()?;
    let Some(context) = support::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = support::frames::prepared_renderer(&context, &prepared)?;
    let voxels = Voxels {
        world: &prepared.world,
        registry: prepared.registry.as_ref(),
    };
    let palette = Palette::of(&prepared.registry, &prepared.resolution, &prepared.texels);
    let sea = BlockName::parse(SEA)?;
    let scene = Arc::new(prepared.scene.clone());

    let mut examined = Vec::new();
    for tick in DECLARED_TICKS {
        let camera =
            support::frames::player_pose(u32::from(tick), &prepared.world, &prepared.registry)?;
        let crossings = oracle::crossed_samples(&camera, CAPTURE_SIZE, &voxels)?;
        let frame = drawn(&context, &mut renderer, &scene, (tick, camera))?;
        examined.push(judged((tick, &frame), &crossings, &palette, &sea)?);
    }
    Ok(Some(examined))
}

/// What one capture came to, drawn and judged.
fn judged(
    shot: (u16, &Rgba8Image),
    crossings: &[CrossedSample],
    palette: &Palette<'_>,
    sea: &BlockName,
) -> Result<Nearer, Box<dyn Error>> {
    let (tick, frame) = shot;
    let passing_light = crossings
        .iter()
        .any(|(_, crossed)| !crossed.layers.is_empty());
    let in_front = standing_in_front_of_the_sea(crossings, sea);
    Ok(Nearer {
        tick,
        draws_something_that_passes_light: if passing_light { SOMETHING } else { NOTHING },
        opaque_faces_standing_in_front_of_the_sea: if in_front.is_empty() {
            NOTHING
        } else {
            SOMETHING
        },
        drawn_with_the_sea_mixed_in: off_colour(frame, &in_front, palette)?,
        too_near_the_blend_they_must_not_be: indistinguishable(&in_front, palette, sea)?,
    })
}

/// The declared samples whose ray meets an opaque face standing nearer the eye
/// than the farthest run of the sea the same frame draws.
///
/// **The sea itself is not a face standing in front of the sea**, which has to
/// be said because before it declares a degree a ray stops at it like anything
/// else — and a reading that did not say so would be judging water against
/// water and reporting a separation of nothing.
fn standing_in_front_of_the_sea(
    crossings: &[CrossedSample],
    sea: &BlockName,
) -> Vec<CrossedSample> {
    let farthest = crossings
        .iter()
        .filter_map(|(_, crossed)| {
            crossed
                .layers
                .iter()
                .find(|layer| &layer.block == sea)
                .map(|layer| layer.along)
        })
        .fold(0.0f32, f32::max);
    crossings
        .iter()
        .filter(|(_, crossed)| {
            crossed.layers.is_empty()
                && crossed
                    .beyond
                    .as_ref()
                    .is_some_and(|surface| &surface.block != sea && surface.along < farthest)
        })
        .cloned()
        .collect()
}

/// Every examined pixel `frame` draws further than the tolerance from anything
/// its own face can show.
fn off_colour(
    frame: &Rgba8Image,
    examined: &[CrossedSample],
    palette: &Palette<'_>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut off = Vec::new();
    for (pixel, crossed) in examined {
        let colour = pixel_color(frame, *pixel)?;
        let stands = palette.stands_from(crossed, colour)?;
        if stands > KEEPS_ITS_OWN_COLOUR {
            off.push(format!(
                "{pixel:?} looks at {} with the sea behind it and drew {colour:?}, ΔE {stands:.2} \
                 from anything that face can show",
                crossed.sighted().described()
            ));
        }
    }
    Ok(off)
}

/// Every examined face whose own colours stand within the tolerance of the same
/// face seen through one run of the sea.
///
/// **The lower half of the tolerance's bracket, asserted rather than quoted.** A
/// face that looks the same blended and unblended is one this reading cannot
/// report either way, and a palette change that brought the two together would
/// otherwise leave a green test asserting nothing.
fn indistinguishable(
    examined: &[CrossedSample],
    palette: &Palette<'_>,
    sea: &BlockName,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut too_close = Vec::new();
    for (pixel, crossed) in examined {
        let own = palette.predicted(crossed)?;
        let through = palette.predicted(&Crossed {
            layers: vec![Surface {
                block: sea.clone(),
                facing: Some(Facing::PosY),
                along: 0.0,
            }],
            beyond: crossed.beyond.clone(),
        })?;
        let nearest = nearest_between(&own, &through)?;
        if nearest <= KEEPS_ITS_OWN_COLOUR {
            too_close.push(format!(
                "{pixel:?} looks at {} whose own colours stand ΔE {nearest:.2} from the same face \
                 seen through the sea",
                crossed.sighted().described()
            ));
        }
    }
    Ok(too_close)
}

/// The frame `shot`'s camera draws at `shot`'s tick, at the declared capture
/// size.
fn drawn(
    context: &mc_testkit::frame::gpu::CaptureContext,
    renderer: &mut mc_render::gpu::TerrainRenderer,
    scene: &Arc<mc_render::geometry::scene::SceneGeometry>,
    shot: (u16, CameraPose),
) -> Result<Rgba8Image, Box<dyn Error>> {
    let (tick, camera) = shot;
    let view = camera_view(camera.eye, camera.target);
    let snapshot = support::frames::snapshot(u32::from(tick), view, scene);
    let request = support::frames::request(context, &format!("shore-over-sea-t{tick:03}"))?;
    let mut frame = ReplayFrame {
        context,
        renderer,
        snapshot: &snapshot,
    };
    frame.capture(&request)
}
