//! The frame the client draws: terrain, and then the HUD content declared.
//!
//! [`FrameRenderer`] is the client's only frame call. It wraps the terrain pass
//! unchanged — [`TerrainRenderer::record_terrain`] stays public and byte-
//! identical because it *is* "the HUD stage not run at all" that the zero-
//! element comparisons measure against, and it is what the committed terrain
//! goldens are shot through.
//!
//! [`FrameRenderer::compose_hud`] is the single HUD composition entry point and
//! is public for one reason: a frame test needs to compose onto an arbitrary
//! cleared target, and the sky-blue terrain clear cannot supply the black
//! backdrop an alpha composite is stated against.
//!
//! # Order, and what it makes true
//!
//! Terrain, then the HUD, then the debug overlay. Each pass after the first
//! loads the colour attachment rather than clearing it and carries no depth
//! attachment, so what content declared is composited over the world rather than
//! tested against it — and the overlay is composited over both. **That last
//! ordering is load-bearing rather than tidy:** the overlay is the instrument
//! somebody diagnoses a misbehaving content root with, so it has to be the last
//! thing to write a pixel, and an element declared over the whole target is
//! painted underneath it.

use crate::geometry::scene::SceneGeometry;
use crate::hud::{HudFrame, compose};
use crate::overlay::OverlayReadout;
use crate::pass::TerrainPassConfig;
use crate::snapshot::{FrameStats, ScenePhase, TerrainSnapshot};
use crate::texture::TextureLayers;

use super::hud_pass::{HudPass, paintable};
use super::overlay::OverlayPass;
use super::{FrameError, RecordTarget, RendererError, TerrainRenderer};

/// Everything one frame shows: the world, the HUD over it, and the debug overlay
/// over that.
#[derive(Debug)]
pub struct FrameSnapshot<'a> {
    pub terrain: &'a TerrainSnapshot,
    /// The elements content declared, which may be none at all.
    pub hud: &'a HudFrame,
    /// What the debug overlay publishes when it is being shown, and `None` when
    /// it is hidden — which is every frame of a client nobody has asked for it
    /// on.
    ///
    /// **An `Option` rather than a readout that is sometimes blank**, because
    /// "the overlay is not being shown" and "the overlay is showing nothing" are
    /// different frames and only one of them is a frame this client draws. It is
    /// the whole of what crosses into the file that paints it.
    pub overlay: Option<&'a OverlayReadout>,
}

/// The terrain pass, the HUD pass over it and the overlay pass over that, with
/// the texture layers the HUD resolves a swatch through.
#[derive(Debug)]
pub struct FrameRenderer {
    terrain: TerrainRenderer,
    hud: HudPass,
    /// Built whether or not the overlay is ever shown, because a pipeline and a
    /// glyph atlas built on the first keypress would be built on the render
    /// thread while somebody waits for a frame.
    overlay: OverlayPass,
    /// Kept from [`upload_textures`](Self::upload_textures): `compose_hud` takes
    /// no layers, because the client uploads them once at preparation and the
    /// HUD samples the same array texture terrain does.
    layers: TextureLayers,
    compositions: u64,
}

impl FrameRenderer {
    /// Builds the terrain pass `config` describes, and the HUD pass over it.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError`] when a resource cannot be built to the declared
    /// capacities.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &TerrainPassConfig,
    ) -> Result<Self, RendererError> {
        let terrain = TerrainRenderer::new(device, queue, config)?;
        let hud = HudPass::new(device, config, terrain.array_texture());
        let overlay = OverlayPass::new(device, config);
        Ok(Self {
            terrain,
            hud,
            overlay,
            layers: TextureLayers::default(),
            compositions: 0,
        })
    }

    /// Which layer of the array texture each texture key occupies, as the last
    /// [`upload_textures`](Self::upload_textures) left it.
    ///
    /// Borrowed from the renderer rather than copied into whoever composes a
    /// frame: the layers a swatch is looked for in have to be the layers the
    /// array texture was filled from, and a second copy is a second answer
    /// waiting to disagree with this one.
    #[must_use]
    pub const fn texture_layers(&self) -> &TextureLayers {
        &self.layers
    }

    /// Fills the array texture, one layer per resolved texture key.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::TextureLayerOutOfRange`] when a key resolved to
    /// a layer the array does not hold.
    pub fn upload_textures(
        &mut self,
        queue: &wgpu::Queue,
        layers: &TextureLayers,
    ) -> Result<(), RendererError> {
        self.terrain.upload_textures(queue, layers)?;
        self.layers = layers.clone();
        Ok(())
    }

    /// Uploads a scene's vertices and section table.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::SceneTooLarge`] when the scene does not fit the
    /// buffers.
    pub fn upload_scene(
        &mut self,
        queue: &wgpu::Queue,
        scene: &SceneGeometry,
    ) -> Result<(), RendererError> {
        self.terrain.upload_scene(queue, scene)
    }

    /// The client's only frame call: the terrain pass, then `compose_hud`
    /// exactly once, then the debug overlay when one is being shown.
    ///
    /// **The overlay goes last, and that is the whole of why content cannot
    /// obscure it.** Each pass loads the colour attachment the previous one left,
    /// so the last thing recorded is the last thing to write a pixel — an element
    /// a content root declared over the entire target is painted before this and
    /// composited under it.
    ///
    /// # Errors
    ///
    /// Returns whatever the terrain pass or the HUD pass reported. The overlay
    /// adds no failure of its own: it paints text it was handed onto a target the
    /// two passes before it already wrote to.
    pub fn record_frame(
        &mut self,
        mut target: RecordTarget<'_>,
        phase: &ScenePhase,
        frame: &FrameSnapshot<'_>,
    ) -> Result<FrameStats, FrameError> {
        let stats = self
            .terrain
            .record_terrain(reborrow(&mut target), phase, frame.terrain)?;
        self.compose_hud(reborrow(&mut target), frame.hud)?;
        if let Some(readout) = frame.overlay {
            self.overlay.record(target, readout);
        }
        Ok(stats)
    }

    /// The single HUD composition entry point.
    ///
    /// The composition is counted whether or not the target has any area to
    /// paint into: what this counts is that the client asked for a HUD, and a
    /// minimised window is not the client changing its mind.
    ///
    /// # Errors
    ///
    /// Returns the recording failure the HUD pass reported.
    pub fn compose_hud(
        &mut self,
        target: RecordTarget<'_>,
        hud: &HudFrame,
    ) -> Result<(), FrameError> {
        self.compositions = self.compositions.saturating_add(1);
        if !paintable(target.size) {
            return Ok(());
        }
        let planned = compose(hud, target.size, &self.layers);
        self.hud.record(target, &planned);
        Ok(())
    }

    /// How many times [`compose_hud`](Self::compose_hud) has run.
    #[must_use]
    pub const fn hud_compositions(&self) -> u64 {
        self.compositions
    }
}

/// `target` borrowed again, so one frame can record two passes into one encoder.
fn reborrow<'a>(target: &'a mut RecordTarget<'_>) -> RecordTarget<'a> {
    RecordTarget {
        device: target.device,
        queue: target.queue,
        encoder: &mut *target.encoder,
        color: target.color,
        size: target.size,
    }
}
