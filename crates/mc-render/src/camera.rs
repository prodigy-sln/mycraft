//! Where the camera is, what it can see, and which sections are worth drawing.
//!
//! The frustum test here is the CPU's expression of a decision the GPU makes for
//! real: the cull pass reads the same six planes out of a uniform buffer and
//! applies the same half-space test per section. Having it twice is the point —
//! one scenario asserts the two agree, and a compute shader is not something a
//! unit test can reason about on its own.
//!
//! **Depth runs 0..1 and clip-space y is up.** `glam` offers three projection
//! conventions and only one of them is wgpu's: `opengl` puts NDC z in −1..1, and
//! `vulkan` puts clip-space y *down*, which is a vertically mirrored world — the
//! exact defect the golden probes and their flip control exist to catch. So
//! `directx` it is, whose name says nothing useful about which API it suits.
//!
//! **The planes are deliberately not normalised.** Nothing here asks how far a
//! box is from a plane; every question is which side of it the box is on, and
//! the sign of `normal · p + offset` is unchanged by scaling the pair. Skipping
//! the normalisation removes a division, and with it the degenerate case where a
//! zero-length normal would quietly produce NaN and admit everything.

use glam::{Mat4, Vec3, Vec4};

use crate::aabb::Aabb;
use crate::geometry::scene::SectionRecord;
use crate::surface::SurfaceSize;

/// Where the camera sits and what it looks at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraView {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
}

/// The camera looking from `eye` at `target`.
///
/// World up is `+Y` and is not a parameter: a replay that could roll the camera
/// would need a scenario saying what rolling means, and there is none. This
/// lives here rather than in the client so that the conversion from the plain
/// arrays a caller holds is inside the counted, tested part of the renderer.
#[must_use]
pub fn camera_view(eye: [f32; 3], target: [f32; 3]) -> CameraView {
    CameraView {
        eye: Vec3::from_array(eye),
        target: Vec3::from_array(target),
        up: Vec3::Y,
    }
}

/// The lens: how much of the world reaches the frame, and how far.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection {
    pub fov_y_radians: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

/// The lens the replay is seen through, at a frame of `size`.
///
/// The field of view, the near plane and the far plane are the replay's declared
/// values; only the aspect ratio comes from the target. It is a function rather
/// than a constant because the aspect does, and it lives here rather than at each
/// call site because a frame drawn under one lens and judged under another is a
/// disagreement that reads as a culling bug: the frustum a test predicts the
/// visible set from has to be the frustum the pass culled by.
#[must_use]
pub fn projection_for(size: SurfaceSize) -> Projection {
    Projection {
        fov_y_radians: FIELD_OF_VIEW_DEGREES.to_radians(),
        aspect: size.width as f32 / size.height as f32,
        near: NEAR_PLANE,
        far: FAR_PLANE,
    }
}

/// How much of the world the replay's lens takes in, vertically.
const FIELD_OF_VIEW_DEGREES: f32 = 60.0;

/// The nearest and furthest a fragment may be and still be drawn.
///
/// The near plane is half a block: the orbit never enters the terrain, and a
/// nearer one would spend depth precision on a range nothing occupies. The far
/// plane clears the orbit's own diameter with room to spare.
const NEAR_PLANE: f32 = 0.5;
const FAR_PLANE: f32 = 512.0;

/// The matrix taking a world position to clip space.
#[must_use]
pub fn view_projection(view: &CameraView, projection: &Projection) -> Mat4 {
    let lens = glam::camera::rh::proj::directx::perspective(
        projection.fov_y_radians,
        projection.aspect,
        projection.near,
        projection.far,
    );
    lens * glam::camera::rh::view::look_at_mat4(view.eye, view.target, view.up)
}

/// One half-space: everything with `normal · p + offset >= 0` is inside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub offset: f32,
}

/// The six half-spaces a camera can see through, in the order near, far, left,
/// right, bottom, top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum([Plane; 6]);

impl Frustum {
    /// The frustum of the camera `matrix` describes.
    ///
    /// Each plane is one of the clip-space inequalities rearranged into world
    /// space. A point is drawn when `0 <= z <= w` and `|x| <= w` and `|y| <= w`,
    /// and each of those six comparisons is a row of the matrix combined with
    /// the `w` row — which is why the near plane is the `z` row alone, and would
    /// be `z + w` under the OpenGL convention this project does not use.
    #[must_use]
    pub fn from_view_projection(matrix: &Mat4) -> Self {
        let (x, y, z, w) = (matrix.row(0), matrix.row(1), matrix.row(2), matrix.row(3));
        Self([
            half_space(z),
            half_space(w - z),
            half_space(w + x),
            half_space(w - x),
            half_space(w + y),
            half_space(w - y),
        ])
    }

    /// The six half-spaces, in the order near, far, left, right, bottom, top.
    ///
    /// The cull shader tests the same six against the same boxes, and reads them
    /// out of a uniform buffer this is the source of. Exposed rather than
    /// rebuilt on that side: a second extraction of the same planes from the
    /// same matrix is the duplication FR-2.2-S2 would then be comparing, instead
    /// of the two tests it exists to compare.
    #[must_use]
    pub const fn planes(&self) -> &[Plane; 6] {
        &self.0
    }

    /// Whether any part of `aabb` could be drawn.
    ///
    /// Conservative in the one direction that is safe: a box straddling a plane
    /// is admitted, and a box that clears every plane's own test is admitted
    /// even in the corner cases where it lies outside the frustum itself. The
    /// cost is a section drawn that contributes nothing; the alternative would
    /// be a hole in the world.
    #[must_use]
    pub fn admits(&self, aabb: &Aabb) -> bool {
        let (min, max) = (Vec3::from_array(aabb.min), Vec3::from_array(aabb.max));
        self.0.iter().all(|plane| {
            // The corner furthest along the plane's normal. If even that one is
            // behind the plane, every other corner is too.
            let furthest = Vec3::select(plane.normal.cmpge(Vec3::ZERO), max, min);
            plane.normal.dot(furthest) + plane.offset >= 0.0
        })
    }
}

/// One clip-space inequality as a world-space half-space.
fn half_space(coefficients: Vec4) -> Plane {
    Plane {
        normal: coefficients.truncate(),
        offset: coefficients.w,
    }
}

/// The indices of the sections `frustum` admits, ascending.
///
/// Indices rather than records: this is what the visible-set buffer holds, and
/// the compute pass writes the same numbers for the same sections.
#[must_use]
pub fn visible_sections(frustum: &Frustum, sections: &[SectionRecord]) -> Vec<u32> {
    sections
        .iter()
        .enumerate()
        .filter(|(_, section)| frustum.admits(&section.aabb))
        .map(|(index, _)| index as u32)
        .collect()
}

#[cfg(test)]
#[path = "camera_test.rs"]
mod tests;
