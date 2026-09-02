//! What the renderer is handed for a frame, and what it reports about it.
//!
//! The renderer keeps **no record of which tick it last drew**. That absence is
//! the design: with nothing stored there is no comparison that could refuse an
//! older snapshot or hold out for a newer one, so "reading a stale snapshot is
//! correct" is a fact about the shape of this module rather than a rule someone
//! has to follow. It is also why the statistics are a free function over a
//! snapshot and a projection instead of a method on something that could
//! remember.
//!
//! The snapshot type lives here, in the crate that *consumes* it, so that the
//! simulation never learns what a vertex is. The composition root builds one of
//! these per tick out of what the simulation published plus a clone of the scene
//! prepared once at startup.

use std::sync::Arc;

use mc_core::block::MediumTint;

use crate::camera::{CameraView, Frustum, Projection, view_projection, visible_sections};
use crate::geometry::scene::SceneGeometry;

/// One frame's input: which tick it is, where the camera is, and what to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainSnapshot {
    pub tick: u32,
    pub camera: CameraView,
    pub scene: Arc<SceneGeometry>,
    /// What the medium the eye stands in does to the light reaching it, as the
    /// simulation resolved it for this tick.
    ///
    /// **Carried, never worked out here.** Which block the eye is in is a
    /// question about the world and the registry, and this crate holds neither
    /// — answering it here is the seam.
    pub tint: Option<MediumTint>,
}

/// How far the scene has got, as the frame path is allowed to see it.
///
/// The world is generated and meshed on workers, and until that finishes there
/// is no geometry to draw. This is what the frame path is told instead of being
/// handed an empty scene: a window whose surface texture was acquired and left
/// unwritten shows whatever the driver last had there, which is worse than the
/// clear colour and looks like a crash rather than like waiting.
#[derive(Debug, Clone, PartialEq)]
pub enum ScenePhase {
    Preparing,
    Ready(Arc<SceneGeometry>),
}

/// What a frame has to record, given how far the scene has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameWork {
    ClearOnly,
    Terrain,
}

/// What the frame path does in `phase`.
///
/// A pure function over the phase, so the one branch that decides whether a
/// frame draws terrain is testable without a device and cannot acquire a third
/// answer by accident.
#[must_use]
pub const fn frame_work(phase: &ScenePhase) -> FrameWork {
    match phase {
        ScenePhase::Preparing => FrameWork::ClearOnly,
        ScenePhase::Ready(_) => FrameWork::Terrain,
    }
}

/// What one frame did.
///
/// `sections_admitted` is a **prediction** made by the pure frustum function,
/// and is named as one. What was actually drawn is the indirect arguments' index
/// count, which only a device can report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameStats {
    pub tick: u32,
    pub sections_submitted: u32,
    pub sections_admitted: u32,
    pub terrain_draw_calls: u32,
}

/// How many draw calls the terrain costs, whatever is in it.
///
/// **Two indirect draws, always: one per layer.** Visibility is decided on the
/// GPU and each draw's index count is the only field that varies, so the number
/// moves with neither what is in view, nor how many sections there are, nor how
/// much of the world passes light — a frame declaring no translucency issues the
/// second draw over zero indices rather than skipping it. A per-section call
/// would be a regression, and this constant is where that shows up in the
/// statistics: what it watches is that the count stays a property of the
/// pipeline rather than of the scene.
const TERRAIN_DRAW_CALLS: u32 = 2;

/// The statistics for drawing `snapshot` under `projection`.
///
/// Takes no history and holds none, so the same snapshot always yields the same
/// answer and an older tick is reported exactly as a newer one would be.
#[must_use]
pub fn frame_stats(snapshot: &TerrainSnapshot, projection: &Projection) -> FrameStats {
    let sections = snapshot.scene.sections();
    let frustum = Frustum::from_view_projection(&view_projection(&snapshot.camera, projection));
    FrameStats {
        tick: snapshot.tick,
        sections_submitted: sections.len() as u32,
        sections_admitted: visible_sections(&frustum, sections).len() as u32,
        terrain_draw_calls: TERRAIN_DRAW_CALLS,
    }
}
