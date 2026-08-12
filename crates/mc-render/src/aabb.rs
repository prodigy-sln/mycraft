//! A world-space box, which is all the culling pass ever knows about a section.
//!
//! Deliberately data and nothing else. The frustum test that consumes it lives
//! beside the camera maths, and the same six-plane test is expressed a second
//! time in the cull shader — so a method here would be a third expression of it,
//! which is exactly one more than the design has room for.

/// An axis-aligned box in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}
