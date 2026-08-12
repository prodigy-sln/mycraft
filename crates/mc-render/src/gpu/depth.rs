//! The depth attachment the renderer owns.
//!
//! The capture harness supplies a colour target and nothing else — "the harness
//! supplies a canvas and never a scene" — so depth is this crate's to allocate,
//! and allocating it here rather than asking the harness for it is what keeps
//! the captured path and the windowed path drawing through the same pass.
//!
//! It is cached and reallocated only when the frame changes size. Reallocating
//! every frame would be a per-frame texture creation on the render path; never
//! reallocating would attach a texture of the wrong size, which wgpu refuses at
//! the point the pass begins rather than at the point the size changed.

use crate::pass::TerrainPassConfig;
use crate::surface::{SurfaceSize, depth_needs_reallocation};

use super::FrameError;

/// The depth texture a frame is drawn against, and the size it was made for.
#[derive(Debug, Default)]
pub(super) struct DepthAttachment {
    allocated: Option<(SurfaceSize, wgpu::TextureView)>,
}

impl DepthAttachment {
    /// The view for a frame of `size`, allocating one if the cached view is for
    /// a different size or there is none.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::DepthAllocation`] for a frame with no area. A
    /// zero-width or zero-height attachment is what a minimised window asks for,
    /// and it is a refusal rather than a device-level failure because the
    /// decision that a frame that small is not drawable belongs on this side.
    pub(super) fn view_for(
        &mut self,
        device: &wgpu::Device,
        size: SurfaceSize,
        config: &TerrainPassConfig,
    ) -> Result<&wgpu::TextureView, FrameError> {
        if size.width == 0 || size.height == 0 {
            return Err(FrameError::DepthAllocation { size });
        }
        // The policy is pure and lives beside the rest of the resize behaviour,
        // where a test reaches it without a device. What stays here is the
        // allocation it decides on, which is the only half that needs one — and
        // there is no second copy of the rule to drift from this one.
        if depth_needs_reallocation(self.allocated.as_ref().map(|(had, _)| *had), size) {
            self.allocated = Some((size, allocate(device, size, config)));
        }
        self.allocated
            .as_ref()
            .map(|(_, view)| view)
            // Unreachable: the branch above assigns whenever the cached size is
            // absent or different, so by here the option is filled.
            .ok_or(FrameError::DepthAllocation { size })
    }
}

/// A fresh depth texture's view, at `size`.
fn allocate(
    device: &wgpu::Device,
    size: SurfaceSize,
    config: &TerrainPassConfig,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mycraft terrain depth"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: super::depth_format(config),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
