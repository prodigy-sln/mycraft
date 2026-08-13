//! Where a camera stands and what it looks at.
//!
//! Its own type rather than the renderer's `CameraView`: this crate never learns
//! what a view matrix is, and the composition root converts. Three duplicated
//! coordinates are the cheap half of that trade.
//!
//! It lives at the crate root rather than under `replay` because the replay is
//! no longer the only thing that produces one — a player's own state implies a
//! camera, and that has nothing to do with a scripted scene.

/// Where the camera stands and what it looks at.
///
/// `[f32; 3]` rather than a vector type on purpose: the client passes these
/// straight to the renderer's view constructor, and a vector type here would
/// push a conversion into the crate ADR-013 exists to keep empty.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose {
    pub eye: [f32; 3],
    pub target: [f32; 3],
}
