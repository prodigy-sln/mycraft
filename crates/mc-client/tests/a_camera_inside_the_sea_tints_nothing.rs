//! What a camera standing inside the sea draws, and what it does not.
//!
//! # The pose is declared, and it had to be
//!
//! The camera is in open air at all three declared capture ticks — the eye's own
//! cell holds nothing drawn at tick 0, 59 or 119, which
//! `replay_oracle.rs::the_camera_of_every_judged_frame_stands_in_open_air` is
//! the standing assertion of. The *player* wades; the *eye* does not go under.
//! So a camera inside a translucent cell is not something the declared walk
//! provides, and this reading declares its own pose over the shipped world —
//! which `terrain_probes.rs` and `support/all_opaque.rs` both already do, and
//! for the same reason: the world, the art, the mesher and the draw path stay
//! the shipped ones and only the pose is the fixture's.
//!
//! # The filter and the ranking, stated apart
//!
//! A ranking can only search inside what the filter admitted, so a constraint
//! the filter never applied is invisible to every ordering of it.
//!
//! **The filter.** A candidate is admitted when the eye's own cell holds a block
//! that passes light; when the eye stands at that cell's centre, so it is
//! strictly inside on all three axes and no rounding puts it on a boundary; when
//! the forward direction is not parallel to the world's up axis, where the
//! marching basis is degenerate; and when the sample grid classifies at least
//! one sample as sky and at least one as a surface reached without crossing the
//! sea, so both halves of what is asserted below have something to be about.
//!
//! **What the filter deliberately does not say, measured rather than assumed.**
//! It does not ask that the eye's six neighbours hold water: the shipped sea is
//! **178 cells, 47 at height 33 and 131 at height 34**, so it is one to two deep
//! and *no* cell of it has water on all six sides — a filter demanding that
//! admits nothing at all. What "inside the volume" needs is only that the eye
//! stands strictly inside a cell the content declares translucent, which is what
//! the run rule turns on. It also does not ask that some ray leave the eye and
//! cross a *further* run of the sea: over all **19 767** admitted candidates not
//! one has a sample that does, because there is only one body of water and the
//! eye is in it. So Decision 3's "a further run along the ray still draws its
//! entry face" has no witness at this pose and nothing here claims it.
//!
//! **The ranking.** Of the admitted candidates, the one whose grid splits most
//! evenly between sky and surfaces, so neither half of the assertion rests on a
//! handful of samples. The chosen pose gives **288 sky and 288 surfaces** of the
//! 576 declared samples, and no candidate does better than an even split.
//!
//! # Both directions, because they fail differently
//!
//! The scenario names a comparison — *the colour it draws from a dry camera at
//! the same pose* — and an absolute claim, that no tint reaches the frame as a
//! whole. Both are made here and neither would be enough alone. Two frames
//! differing only in whether the sea passes light can agree while both are
//! wrong, which is a failure this project has shipped; and an absolute reading
//! against a prediction cannot be satisfied by two wrong things agreeing. So the
//! same pose is drawn twice — once over the shipped root and once over a copy of
//! it whose sea declares that it stops all the light — and the two frames have
//! to be identical **pixel for pixel over the whole frame**, not merely at the
//! declared samples.
//!
//! # Why the two frames can be identical at all
//!
//! The run the eye stands in draws nothing: the eye is past its entry face, and
//! the exit face has its normal along every ray that leaves the eye and is
//! back-facing. Nothing else in the world is water. So the sea contributes no
//! fragment to this frame whatever degree it declares, and *identical* is the
//! right expectation rather than a lucky one. Measured: 0 differing pixels of
//! 921 600, and the worst any declared sample stands from the colour predicted
//! for it is **ΔE 0.00**.

mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_render::camera::camera_view;
use mc_sim::camera::CameraPose;
use mc_testkit::frame::Rgba8Image;

use support::composite::Palette;
use support::content::shipped_with_the_sea_declaring;
use support::frames::{CAPTURE_SIZE, ReplayFrame};
use support::oracle::{self, CrossedSample, Voxels};
use support::probe::{SAME_COLOR, pixel_color};
use support::{PreparedScene, TestResult, prepare_scene, prepare_scene_at};

/// Where the eye stands and what it looks at. See this module's header for the
/// filter that admitted it and the ranking that chose it.
const EYE: [f32; 3] = [60.5, 34.5, 8.5];
const LOOK_AT: [f32; 3] = [28.5, 66.5, 8.5];

/// The degree a sea that stops all the light declares, which is what the second
/// frame is drawn over.
const STOPS_EVERYTHING: f32 = 1.0;

/// How far a pixel may sit from the colour predicted for it, in ΔE.
///
/// [`SAME_COLOR`] — the harness's own per-pixel default, so "this pixel is what
/// it should be" means here what it means in a golden comparison. Derived from
/// both directions: the worst any declared sample of this pose is measured to
/// stand is **ΔE 0.00**, because every surface it sees is near enough to show
/// its own texels; and a tint over the frame as a whole would have to be smaller
/// than one code value to hide under it.
const THE_SAME_COLOUR: f64 = SAME_COLOR;

#[test]
fn a_camera_standing_in_the_sea_draws_every_surface_and_the_sky_exactly_as_a_dry_one_does()
-> TestResult {
    let Some(standing) = what_the_submerged_camera_draws()? else {
        return Ok(());
    };

    assert_eq!(
        standing,
        Submerged {
            the_cell_the_eye_stands_in: PASSES_LIGHT,
            surfaces_examined: SOMETHING,
            sky_examined: SOMETHING,
            drawn_at_something_other_than_their_own_colour: Vec::new(),
            differing_from_the_same_pose_with_a_sea_that_stops_all_the_light: 0,
        },
        "a camera inside a cell that passes light gains nothing from the volume it stands in: it \
         is past that run's entry face, and the exit face has its normal along every ray leaving \
         the eye and is culled. So every surface outside the volume draws its own colour and the \
         sky draws the declared clear colour, and the frame is the frame a camera at the same pose \
         would draw in a world whose sea stopped all the light — pixel for pixel, over all 921 600 \
         of them, which is the only form of \"no tint over the frame as a whole\" that a sample \
         grid cannot miss. The first field is the reading's premise: while the sea stops all the \
         light there is no submerged camera to be about, and a green result here would be one this \
         reading had not earned"
    );
    Ok(())
}

/// What the submerged pose came to.
#[derive(Debug, PartialEq, Eq)]
struct Submerged {
    /// Whether the cell the eye occupies holds a block that passes light.
    the_cell_the_eye_stands_in: &'static str,
    surfaces_examined: &'static str,
    sky_examined: &'static str,
    /// Every declared sample drawn further than the tolerance from the colour
    /// predicted for it, named with what it drew.
    drawn_at_something_other_than_their_own_colour: Vec<String>,
    /// How many pixels of the whole frame differ from the same pose drawn over a
    /// sea that stops all the light.
    differing_from_the_same_pose_with_a_sea_that_stops_all_the_light: usize,
}

/// What the eye's own cell reports when it holds a block that passes light, and
/// when it does not.
const PASSES_LIGHT: &str = "a block that passes light";
const STOPS_ALL_THE_LIGHT: &str = "a block that stops all the light, or nothing drawn at all";

/// What a reading holding one or more of the thing in question reports.
const SOMETHING: &str = "one or more";

/// What a reading holding none reports.
const NOTHING: &str = "none at all";

/// The submerged pose drawn over the shipped world and judged, or `None` when
/// the opt-in permitted the absence of a device.
fn what_the_submerged_camera_draws() -> Result<Option<Submerged>, Box<dyn Error>> {
    let wet = prepare_scene()?;
    let (inside, crossings) = what_the_march_says(&wet)?;
    let surfaces = crossings
        .iter()
        .filter(|(_, crossed)| crossed.beyond.is_some())
        .count();
    let sky = crossings.len() - surfaces;

    let Some(context) = support::frames::device()? else {
        return Ok(None);
    };
    let (drawn_wet, drawn_dry) = the_pose_drawn_twice(&wet, &context)?;

    // The colours are only asked for where the premise holds. A ray leaving an
    // eye inside a block that stops all the light met that block with no face to
    // have entered by, and a prediction for it would be an invented one.
    let palette = Palette::of(&wet.registry, &wet.resolution, &wet.texels);
    let off = if inside {
        off_colour(&drawn_wet, &crossings, &palette)?
    } else {
        Vec::new()
    };

    Ok(Some(Submerged {
        the_cell_the_eye_stands_in: if inside {
            PASSES_LIGHT
        } else {
            STOPS_ALL_THE_LIGHT
        },
        surfaces_examined: if surfaces == 0 { NOTHING } else { SOMETHING },
        sky_examined: if sky == 0 { NOTHING } else { SOMETHING },
        drawn_at_something_other_than_their_own_colour: off,
        differing_from_the_same_pose_with_a_sea_that_stops_all_the_light: differing(
            &drawn_wet, &drawn_dry,
        ),
    }))
}

/// Whether the eye's own cell holds a block that passes light, and everything
/// the ray through each declared sample met.
///
/// **The premise comes back beside the classification**, because a reading that
/// asked for colours without it would be asking a prediction about a ray that
/// left an eye inside an opaque block — met with no face to have entered by, and
/// answerable only by inventing one.
fn what_the_march_says(wet: &PreparedScene) -> Result<(bool, Vec<CrossedSample>), Box<dyn Error>> {
    let voxels = Voxels {
        world: &wet.world,
        registry: wet.registry.as_ref(),
    };
    let inside = voxels
        .drawn_degree(Vec3::from_array(EYE).floor().as_ivec3())?
        .is_some_and(|(_block, degree)| degree.passes_light());
    let pose = CameraPose {
        eye: EYE,
        target: LOOK_AT,
    };
    Ok((
        inside,
        oracle::crossed_samples(&pose, CAPTURE_SIZE, &voxels)?,
    ))
}

/// The declared pose drawn over the shipped world, and drawn again over a copy
/// of it whose sea declares that it stops all the light.
fn the_pose_drawn_twice(
    wet: &PreparedScene,
    context: &mc_testkit::frame::gpu::CaptureContext,
) -> Result<(Rgba8Image, Rgba8Image), Box<dyn Error>> {
    let opaque_sea = shipped_with_the_sea_declaring(STOPS_EVERYTHING)?;
    Ok((
        drawn(wet, context, "submerged-wet")?,
        drawn(
            &prepare_scene_at(opaque_sea.path())?,
            context,
            "submerged-dry",
        )?,
    ))
}

/// Every declared sample `frame` draws further than the tolerance from the
/// colour its own crossing predicts, named with what it drew.
fn off_colour(
    frame: &Rgba8Image,
    crossings: &[CrossedSample],
    palette: &Palette<'_>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut off = Vec::new();
    for (pixel, crossed) in crossings {
        let colour = pixel_color(frame, *pixel)?;
        let stands = palette.stands_from(crossed, colour)?;
        if stands > THE_SAME_COLOUR {
            off.push(format!(
                "{pixel:?} looks at {} and drew {colour:?}, ΔE {stands:.2} away",
                crossed.sighted().described()
            ));
        }
    }
    Ok(off)
}

/// How many pixels of two frames of the declared capture size differ.
fn differing(one: &Rgba8Image, other: &Rgba8Image) -> usize {
    (0..CAPTURE_SIZE.height)
        .flat_map(|down| (0..CAPTURE_SIZE.width).map(move |across| (across, down)))
        .filter(|(across, down)| one.pixel(*across, *down) != other.pixel(*across, *down))
        .count()
}

/// The declared pose drawn over `prepared`, at the declared capture size.
fn drawn(
    prepared: &PreparedScene,
    context: &mc_testkit::frame::gpu::CaptureContext,
    name: &str,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let mut renderer = support::frames::prepared_renderer(context, prepared)?;
    let scene = Arc::new(prepared.scene.clone());
    let snapshot = support::frames::snapshot(0, camera_view(EYE, LOOK_AT), &scene);
    let request = support::frames::request(context, name)?;
    let mut frame = ReplayFrame {
        context,
        renderer: &mut renderer,
        snapshot: &snapshot,
    };
    frame.capture(&request)
}
