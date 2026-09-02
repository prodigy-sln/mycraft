//! The shipped world, seen from a declared pose inside its sea and from one
//! just above it.
//!
//! # The pose is declared, and it had to be
//!
//! The camera is in open air at all three declared capture ticks — the eye's own
//! cell holds nothing drawn at tick 0, 59 or 119, which
//! `replay_oracle.rs::the_camera_of_every_judged_frame_stands_in_open_air` is
//! the standing assertion of. The *player* wades; the *eye* does not go under.
//! So a camera inside the sea is not something the declared walk provides, and
//! every reading here declares its own pose over the shipped world — which
//! `terrain_probes.rs` and `support/all_opaque.rs` both already do, and for the
//! same reason: the world, the art, the mesher and the draw path stay the
//! shipped ones and only the pose is the fixture's.
//!
//! # The filter and the ranking, stated apart
//!
//! Inherited from the reading this one supersedes, because a ranking can only
//! search inside what the filter admitted and a constraint the filter never
//! applied is invisible to every ordering of it.
//!
//! **The filter.** A candidate is admitted when the eye's own cell holds a block
//! that passes light; when the eye stands at that cell's centre, so it is
//! strictly inside on all three axes and no rounding puts it on a boundary; when
//! the forward direction is not parallel to the world's up axis, where the
//! marching basis is degenerate; and when the sample grid classifies at least
//! one sample as sky and at least one as a surface reached without crossing the
//! sea, so both halves of what is asserted have something to be about.
//!
//! **What the filter deliberately does not say, measured rather than assumed.**
//! It does not ask that the eye's six neighbours hold water: the shipped sea is
//! **178 cells, 47 at height 33 and 131 at height 34**, so it is one to two deep
//! and no cell of it has water on all six sides. It also does not ask that some
//! ray cross a *further* run of the sea: over all **19 767** admitted candidates
//! not one has a sample that does, because there is one body of water and the
//! eye is in it.
//!
//! **The ranking.** Of the admitted candidates, the one whose grid splits most
//! evenly between sky and surfaces, so neither half of an assertion rests on a
//! handful of samples. The chosen pose gives **288 sky and 288 surfaces** of the
//! 576 declared samples, and no candidate does better than an even split.
//!
//! # Why a frame-to-frame comparison is never made alone here
//!
//! **A pure identity claim is satisfied by a constant wash applied regardless of
//! the eye's medium.** Two frames differing only in whether a tint is declared
//! can agree while both are wrong, which is a failure this project has shipped;
//! so every reading below carries an **absolute per-sample claim in the same
//! assertion** as any comparison it makes, predicted from the world's own voxels
//! and declarations through [`super::oracle`] and [`super::composite`], which
//! share no code with the draw path.
//!
//! **And an identity claim needs a control, for the same reason.** A build that
//! never writes the tint into the frame at all satisfies every "nothing moved"
//! reading here. So each of those carries, in the same verdict, a case in which
//! something *must* move — the same machinery over an eye that does stand inside
//! the declared medium. An implementation that draws no tint anywhere fails on
//! the control rather than passing quietly.
//!
//! # The extent measured over the shipped world
//!
//! The water at `y = 34` occupies `x 60..63 × z 0..34`; its top face is at
//! `y = 35.0`, which is the boundary FR-2.4-S2's two heights straddle.

use std::error::Error;
use std::sync::Arc;

use mc_core::block::MediumTint;
use mc_render::camera::camera_view;
use mc_sim::camera::CameraPose;
use mc_testkit::frame::Rgba8Image;

use super::composite::Palette;
use super::content::ContentRoot;
use super::frames::{CAPTURE_SIZE, ReplayFrame, snapshot_in};
use super::medium::Strays;
use super::oracle::{self, CrossedSample, Sighted, Voxels};
use super::probe::{SAME_COLOR, pixel_color};
use super::{PreparedScene, prepare_scene_at};

/// Where the eye stands inside the sea and what it looks at. See this module's
/// header for the filter that admitted it and the ranking that chose it.
pub const EYE: [f32; 3] = [60.5, 34.5, 8.5];
pub const LOOK_AT: [f32; 3] = [28.5, 66.5, 8.5];

/// The same pose with the eye a block and a half higher, in the open air over
/// the sea's own top face at `y = 35.0`.
pub const DRY_EYE: [f32; 3] = [60.5, 36.5, 8.5];

/// The two heights FR-2.4-S2's boundary straddles: a hair under the sea's top
/// face and a hair over it.
pub const JUST_UNDER_THE_SURFACE: f32 = 34.98;
pub const JUST_OVER_THE_SURFACE: f32 = 35.02;

/// How far a pixel may sit from the colour predicted for it, in ΔE.
///
/// [`SAME_COLOR`] — the harness's own per-pixel default, so "this pixel is what
/// it should be" means here what it means in a golden comparison. The
/// superseded reading measured the worst any declared sample of this pose
/// stands from its own prediction at **ΔE 0.00**, because every surface it sees
/// is near enough to show its own texels.
pub const THE_SAME_COLOUR: f64 = SAME_COLOR;

/// One frame drawn over one content root, with everything a prediction over it
/// needs.
pub struct Shot {
    pub frame: Rgba8Image,
    pub prepared: PreparedScene,
    /// What the simulation's own resolver answered for the eye this was drawn
    /// from.
    pub tint: Option<MediumTint>,
}

impl Shot {
    /// Everything the ray through each declared sample met, over this shot's own
    /// world.
    ///
    /// # Errors
    ///
    /// Returns the registry's refusal for a block the world holds and it does
    /// not register.
    pub fn crossings(&self, eye: [f32; 3]) -> Result<Vec<CrossedSample>, Box<dyn Error>> {
        let voxels = Voxels {
            world: &self.prepared.world,
            registry: self.prepared.registry.as_ref(),
        };
        let pose = CameraPose {
            eye,
            target: LOOK_AT,
        };
        Ok(oracle::crossed_samples(&pose, CAPTURE_SIZE, &voxels)?)
    }

    /// Every declared sample this frame drew further than the tolerance from the
    /// colour its own crossing predicts, seen through the tint this shot
    /// resolved.
    ///
    /// # Errors
    ///
    /// Returns the prediction's own failure, or a sample outside the frame.
    pub fn straying(&self, eye: [f32; 3]) -> Result<Vec<String>, Box<dyn Error>> {
        let palette = Palette::of(
            &self.prepared.registry,
            &self.prepared.resolution,
            &self.prepared.texels,
        );
        let mut strayed = Strays::default();
        for sample in self.crossings(eye)? {
            note_if_off(&mut strayed, self, &palette, &sample)?;
        }
        Ok(strayed.named())
    }

    /// How many declared samples this shot's world predicts as sky.
    ///
    /// # Errors
    ///
    /// As [`crossings`](Self::crossings).
    pub fn sky_samples(&self, eye: [f32; 3]) -> Result<Vec<(u32, u32)>, Box<dyn Error>> {
        Ok(self
            .crossings(eye)?
            .into_iter()
            .filter(|(_, crossed)| crossed.sighted() == Sighted::Sky)
            .map(|(pixel, _)| pixel)
            .collect())
    }
}

/// Records `sample` in `strayed` when `shot` drew it away from the colour
/// `palette` predicts for its own crossing, seen through the tint that shot
/// resolved.
fn note_if_off(
    strayed: &mut Strays,
    shot: &Shot,
    palette: &Palette<'_>,
    sample: &CrossedSample,
) -> Result<(), Box<dyn Error>> {
    let (pixel, crossed) = sample;
    let drawn = pixel_color(&shot.frame, *pixel)?;
    let stands = palette.stands_from_through(crossed, drawn, shot.tint)?;
    if stands <= THE_SAME_COLOUR {
        return Ok(());
    }
    strayed.note(format!(
        "{pixel:?} looks at {} and drew {drawn:?}, ΔE {stands:.2} away",
        crossed.sighted().described()
    ));
    Ok(())
}

/// The declared pose drawn over the content root at `root`, from `eye`.
///
/// **The tint is resolved by the simulation's own resolver against the world
/// this frame is of**, through [`snapshot_in`], so a frame is untinted because
/// the eye stands in nothing that tints and never because a fixture said so.
///
/// `None` where the opt-in permitted the absence of a device.
///
/// # Errors
///
/// Returns the preparation, pipeline, resolver or capture failure.
pub fn drawn_from(
    root: &ContentRoot,
    eye: [f32; 3],
    named: &str,
) -> Result<Option<Shot>, Box<dyn Error>> {
    let prepared = prepare_scene_at(root.path())?;
    let Some(context) = super::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = super::frames::prepared_renderer(&context, &prepared)?;
    let scene = Arc::new(prepared.scene.clone());
    let snapshot = snapshot_in(&prepared, 0, camera_view(eye, LOOK_AT), &scene)?;
    let request = super::frames::request(&context, named)?;
    let frame = ReplayFrame {
        context: &context,
        renderer: &mut renderer,
        snapshot: &snapshot,
    }
    .capture(&request)?;
    Ok(Some(Shot {
        frame,
        tint: snapshot.tint,
        prepared,
    }))
}

/// How many pixels of two frames of the declared capture size differ.
#[must_use]
pub fn differing(one: &Rgba8Image, other: &Rgba8Image) -> usize {
    (0..CAPTURE_SIZE.height)
        .flat_map(|down| (0..CAPTURE_SIZE.width).map(move |across| (across, down)))
        .filter(|(across, down)| one.pixel(*across, *down) != other.pixel(*across, *down))
        .count()
}

/// How many pixels a frame of the declared capture size holds.
pub const PIXELS_IN_THE_FRAME: usize =
    (CAPTURE_SIZE.width as usize) * (CAPTURE_SIZE.height as usize);
