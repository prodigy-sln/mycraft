//! Array-texture layers a reload produced, before the device has been given them.
//!
//! **This module exists to make one omission impossible rather than to move any
//! data.** A reload that appends a layer publishes content whose blocks are drawn
//! from it, and if the frame path forgets the upload the world draws the new block
//! from whatever that layer held before — indefinitely, with nothing reporting it.
//! Measured: deleting the upload outright left 234 of 234 `mc-client` tests green,
//! because nothing in this workspace constructs the frame path.
//!
//! So the layers leave the client's core wrapped, and `Unuploaded::uploaded_to`
//! is the only way to obtain an owned `TextureLayers` from one. The re-mesh
//! worker's retirement needs an owned value, so **the upload cannot be skipped on
//! the way there without failing the build.**
//!
//! What this does not catch: `TextureLayers` is `Clone`, so cloning the borrow
//! below and retiring that compiles. The realistic defect is an omission or a
//! reorder during a refactor rather than a deliberate bypass, and the compiler
//! covers exactly that case.

use mc_render::gpu::{FrameRenderer, RendererError};
use mc_render::texture::TextureLayers;

/// Layers the device has not been given yet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "the device has not been given these layers; `uploaded_to` is what gives them"]
pub struct Unuploaded {
    stated: TextureLayers,
}

impl Unuploaded {
    /// Wraps the layers an accepted candidate's content states.
    pub const fn of(stated: TextureLayers) -> Self {
        Self { stated }
    }

    /// What the layers are, for whoever is asking rather than uploading.
    ///
    /// A borrow, so reading them is not a way out of the obligation.
    pub const fn stated(&self) -> &TextureLayers {
        &self.stated
    }

    /// Uploads them and hands them on.
    ///
    /// The only way to an owned value, which is what makes forgetting the upload a
    /// build failure rather than a wrong picture.
    ///
    /// # Errors
    ///
    /// Returns whatever the renderer refused.
    pub fn uploaded_to(
        self,
        renderer: &mut FrameRenderer,
        queue: &wgpu::Queue,
    ) -> Result<TextureLayers, RendererError> {
        renderer.upload_textures(queue, &self.stated)?;
        Ok(self.stated)
    }
}
