//! The texture resolution a reload produced, before the device has been given it.
//!
//! **This module exists to make one omission impossible rather than to move any
//! data.** A reload that appends a layer publishes content whose blocks are drawn
//! from it, and if the frame path forgets the upload the world draws the new block
//! from whatever that layer held before — indefinitely, with nothing reporting it.
//! Measured: deleting the upload outright left 234 of 234 `mc-client` tests green,
//! because nothing in this workspace constructs the frame path.
//!
//! So it leaves the client's core wrapped, and `Unuploaded::uploaded_to` is the
//! only way to obtain an owned `TextureResolution` from one. The re-mesh
//! worker's retirement needs an owned value, so **the upload cannot be skipped on
//! the way there without failing the build.**
//!
//! What this does not catch: `TextureResolution` is `Clone`, so cloning the
//! borrow below and retiring that compiles. The realistic defect is an omission or a
//! reorder during a refactor rather than a deliberate bypass, and the compiler
//! covers exactly that case.

use mc_render::gpu::{FrameRenderer, RendererError};
use mc_render::texture::TextureResolution;

/// A resolution the device has not been given yet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "the device has not been given this resolution; `uploaded_to` is what gives it"]
pub struct Unuploaded {
    stated: TextureResolution,
}

impl Unuploaded {
    /// Wraps what an accepted candidate's content states.
    pub const fn of(stated: TextureResolution) -> Self {
        Self { stated }
    }

    /// What it states, for whoever is asking rather than uploading.
    ///
    /// A borrow, so reading it is not a way out of the obligation.
    pub const fn stated(&self) -> &TextureResolution {
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
    ) -> Result<TextureResolution, RendererError> {
        renderer.upload_textures(queue, &self.stated)?;
        Ok(self.stated)
    }
}
