//! The single description of the terrain pass, which both paths construct.
//!
//! A windowed frame and a captured frame are only comparable if they were drawn
//! by the same pass, and the failure that matters here is not a wrong picture
//! but a **divergent** one: the goldens keep passing while the window shows
//! something else, and no automated check is looking at the window. So there is
//! one descriptor, one function that fills it in, and the colour target is the
//! only thing a caller may choose. Two constructors writing out two struct
//! literals would put the two paths one careless edit apart; here the divergence
//! is not something to be careful about, because there is no second literal to
//! be careless in.
//!
//! Every setting except the colour format is expressed as a type with **one**
//! variant. That is deliberate rather than incomplete: "always `Depth32Float`",
//! "always back-face culling", "always counter-clockwise", "always `Less`" are
//! decisions the architecture already made, and a type that cannot spell the
//! alternative turns "the two paths agree" from a test result into a fact. The
//! day one of them genuinely needs to vary, adding a variant is a visible act
//! that arrives with its own reason — which is the same argument
//! `LimitsProfile` is built on.

use crate::color::{CLEAR_COLOR_SRGB, srgb8_to_linear};

/// The format of a pass's colour target.
///
/// Only sRGB formats appear: the hardware performs the encode on write, and a
/// non-sRGB target would produce a picture the goldens never recorded. Which is
/// why surface selection refuses to configure one at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorFormat {
    Rgba8UnormSrgb,
    Bgra8UnormSrgb,
}

/// The format of the depth attachment the renderer owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthFormat {
    Depth32Float,
}

/// Which side of a triangle is discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullMode {
    Back,
}

/// Which winding faces the viewer.
///
/// If culling ever turns out inverted, this is the setting that changes — never
/// the corner order the geometry builder emits, which is pinned by a property
/// test that re-winding would break while making the picture look right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontFace {
    Ccw,
}

/// How a fragment's depth is tested against the attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthCompare {
    Less,
}

/// How many bytes one vertex occupies in the vertex buffer.
///
/// One packed `u64`. Named here because it is a pass setting the two paths must
/// agree on, and derived from the packing rather than chosen.
const PACKED_VERTEX_STRIDE: u32 = 8;

/// The format a captured frame is allocated and read back in.
///
/// The capture harness owns this fact; this crate must target the same format or
/// the readback reads a texture the pass never wrote. The two are asserted to
/// agree rather than trusted to, which is the only way round the harness being a
/// dev-dependency: it cannot be named from production code without making the
/// test harness a runtime dependency of the client.
const OFFSCREEN_COLOR_FORMAT: ColorFormat = ColorFormat::Rgba8UnormSrgb;

/// Every setting of the terrain pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainPassConfig {
    pub color_format: ColorFormat,
    pub depth_format: DepthFormat,
    pub clear_color_linear: [f64; 3],
    pub cull_mode: CullMode,
    pub front_face: FrontFace,
    pub depth_compare: DepthCompare,
    pub vertex_stride: u32,
}

impl TerrainPassConfig {
    /// The pass a capture is drawn with.
    #[must_use]
    pub fn offscreen() -> Self {
        Self::targeting(OFFSCREEN_COLOR_FORMAT)
    }

    /// The pass a window is drawn with, into a surface of `color_format`.
    #[must_use]
    pub fn windowed(color_format: ColorFormat) -> Self {
        Self::targeting(color_format)
    }

    /// The one terrain pass, pointed at `color_format`.
    fn targeting(color_format: ColorFormat) -> Self {
        Self {
            color_format,
            depth_format: DepthFormat::Depth32Float,
            clear_color_linear: srgb8_to_linear(CLEAR_COLOR_SRGB),
            cull_mode: CullMode::Back,
            front_face: FrontFace::Ccw,
            depth_compare: DepthCompare::Less,
            vertex_stride: PACKED_VERTEX_STRIDE,
        }
    }
}

#[cfg(test)]
#[path = "pass_test.rs"]
mod tests;
