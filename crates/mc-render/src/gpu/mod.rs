//! The GPU-resident half of the renderer: the only subtree in which `wgpu::`
//! may be named.
//!
//! The boundary is a Cargo feature rather than a convention. `gpu` is default-on
//! and carries the `wgpu` dependency, so `--no-default-features` removes it from
//! the resolved dependency graph and a `use wgpu::` written anywhere outside
//! this subtree stops compiling. `crates/mc-render/CLAUDE.md` asks for exactly
//! that split — anything expressible as a pure function is unit-tested normally
//! and gets no coverage exemption; only what needs a device does — and this
//! module is where "needs a device" is written down.
//!
//! It is also the coverage denominator's edge (ADR-013): code here is verified
//! by golden frames, code beside it by unit tests.
//!
//! # The shape of a frame
//!
//! Terrain is **one indirect draw call**, whatever is in it. A compute pass
//! decides which sections survive the frustum and compacts their indices into a
//! destination index buffer, incrementing the indirect arguments' index count as
//! it goes; the render pass then issues a single `draw_indexed_indirect` over
//! whatever that count came to. A per-section draw loop would be a regression
//! and is what the draw-call figure in the frame statistics counts.
//!
//! # Everything is allocated once, at capacity
//!
//! `MAX_SECTIONS` and `MAX_QUADS` are the sizes of the buffers below, and
//! `SceneGeometry::assemble` refuses a scene that would not fit in them. That is
//! why it is the only capacity gate in the renderer: by the time a scene reaches
//! this module its buffers already exist and it is known to fit. Uploading a
//! scene therefore writes bytes and allocates nothing, which is what keeps a
//! re-mesh off the allocator.

mod buffers;
mod depth;
mod hud;
mod hud_pass;
mod overlay;
mod pipeline;
mod readback;
mod record;

use thiserror::Error;

use crate::camera::projection_for;
use crate::geometry::scene::SceneGeometry;
use crate::pass::{ColorFormat, DepthCompare, DepthFormat, TerrainPassConfig};
use crate::snapshot::{FrameStats, ScenePhase, TerrainSnapshot, frame_stats};
use crate::surface::SurfaceSize;
use crate::texture::TextureLayers;

use buffers::SceneBuffers;
use depth::DepthAttachment;
use hud_pass::ArrayTexture;
use pipeline::Pipelines;

pub use hud::{FrameRenderer, FrameSnapshot};

/// Everything one frame is recorded into and through.
///
/// A named parameter group rather than five loose arguments: the values it
/// carries are part of the binding contract, and a caller that has an encoder
/// but no matching colour view has not made a mistake this signature should let
/// it express. The queue belongs here for the same reason the encoder does —
/// recording a frame writes the per-frame uniform and returns the indirect
/// arguments to their declared state, and neither of those goes through an
/// encoder.
#[derive(Debug)]
pub struct RecordTarget<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub color: &'a wgpu::TextureView,
    pub size: SurfaceSize,
}

/// Why a frame could not be recorded or read back.
///
/// Every variant reaches the caller. Nothing here panics: a dropped frame is
/// recoverable and a crash is not, and this is the module where that rule
/// matters most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FrameError {
    #[error("no depth attachment can be allocated for a {}x{} frame", size.width, size.height)]
    DepthAllocation { size: SurfaceSize },
    #[error("the {stage} could not be read back from the device")]
    Readback { stage: &'static str },
    #[error("the terrain pass could not be recorded: the {stage} is not ready")]
    Recording { stage: &'static str },
}

/// Why a renderer could not be built or filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RendererError {
    #[error("texture layer {layer} is outside the {capacity} the array texture holds")]
    TextureLayerOutOfRange { layer: u16, capacity: u16 },
    #[error("a scene of {found} {resource} exceeds the {capacity} its buffer holds")]
    SceneTooLarge {
        resource: &'static str,
        found: usize,
        capacity: usize,
    },
}

/// The terrain pass: its pipelines, its buffers, its array texture and the depth
/// attachment it owns.
#[derive(Debug)]
pub struct TerrainRenderer {
    config: TerrainPassConfig,
    pipelines: Pipelines,
    buffers: SceneBuffers,
    depth: DepthAttachment,
    /// How many sections the uploaded scene holds, or `None` while no scene has
    /// been uploaded at all. A frame asked for terrain before then is refused
    /// rather than drawn empty: an empty picture and a picture of a world that
    /// has not arrived are the same frame, and only one of them is a defect.
    sections: Option<u32>,
}

impl TerrainRenderer {
    /// Builds the pass `config` describes, with every buffer at capacity.
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
        let buffers = SceneBuffers::new(device, queue)?;
        let pipelines = Pipelines::new(device, config, &buffers);
        Ok(Self {
            config: *config,
            pipelines,
            buffers,
            depth: DepthAttachment::default(),
            sections: None,
        })
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
        self.buffers.write_textures(queue, layers)
    }

    /// The array texture this pass samples, for a second pass that has to sample
    /// the same texels.
    ///
    /// Private to this module and the passes under it, and lent rather than
    /// handed over: the HUD's swatch of a block must be the texture that block is
    /// drawn with, and the only way to guarantee that is for there to be one array
    /// texture rather than two filled from the same layers.
    const fn array_texture(&self) -> ArrayTexture<'_> {
        ArrayTexture {
            view: &self.buffers.texture,
            sampler: &self.buffers.sampler,
        }
    }

    /// Uploads a scene's vertices and section table.
    ///
    /// Called at preparation, never per frame: the replay meshes once and orbits
    /// a camera around what it produced.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::SceneTooLarge`] when the scene does not fit the
    /// buffers, which `SceneGeometry::assemble` has already ruled out.
    pub fn upload_scene(
        &mut self,
        queue: &wgpu::Queue,
        scene: &SceneGeometry,
    ) -> Result<(), RendererError> {
        self.buffers.write_scene(queue, scene)?;
        self.sections = Some(scene.sections().len() as u32);
        Ok(())
    }

    /// Records the compute cull pass and the single indirect terrain draw, or a
    /// bare clear while the scene is still being prepared.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::DepthAllocation`] for a frame with no area, and
    /// [`FrameError::Recording`] when terrain was asked for before a scene was
    /// uploaded.
    pub fn record_terrain(
        &mut self,
        target: RecordTarget<'_>,
        phase: &ScenePhase,
        snapshot: &TerrainSnapshot,
    ) -> Result<FrameStats, FrameError> {
        record::record(self, target, record::Frame { phase, snapshot })
    }

    /// The indirect arguments' index count, as it stands after submission.
    ///
    /// The **observed** figure a frame drew, against the prediction the pure
    /// frustum function makes. Test-path only: it maps a buffer and waits, which
    /// no frame may do.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::Readback`] when the device did not hand the buffer
    /// over.
    pub fn read_drawn_index_count(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<u32, FrameError> {
        readback::drawn_index_count(&self.buffers, readback::Gpu { device, queue })
    }

    /// The per-section visibility flags the compute pass wrote, in section-index
    /// order. Test-path only, for the same reason.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::Readback`] when the device did not hand the buffer
    /// over.
    pub fn read_visible_sections(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<u32>, FrameError> {
        readback::visible_sections(
            &self.buffers,
            readback::Gpu { device, queue },
            self.sections.unwrap_or_default(),
        )
    }

    /// The statistics for `snapshot` at `size`, computed by the pure function.
    ///
    /// `sections_admitted` is a prediction and is named as one; the observation
    /// is [`read_drawn_index_count`](Self::read_drawn_index_count).
    fn stats_for(snapshot: &TerrainSnapshot, size: SurfaceSize) -> FrameStats {
        frame_stats(snapshot, &projection_for(size))
    }
}

/// The depth format `config` declares, as wgpu spells it.
const fn depth_format(config: &TerrainPassConfig) -> wgpu::TextureFormat {
    match config.depth_format {
        DepthFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
    }
}

/// The colour format `config` declares, as wgpu spells it.
const fn color_format(config: &TerrainPassConfig) -> wgpu::TextureFormat {
    match config.color_format {
        ColorFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        ColorFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
    }
}

/// The depth comparison `config` declares, as wgpu spells it.
const fn depth_compare(config: &TerrainPassConfig) -> wgpu::CompareFunction {
    match config.depth_compare {
        DepthCompare::Less => wgpu::CompareFunction::Less,
    }
}

/// The clear colour `config` declares, in the linear space wgpu takes it in.
///
/// The target is sRGB and the hardware performs the encode on write, so a value
/// specified in sRGB here would be encoded a second time and come back lighter
/// than the colour anybody chose — wrong in an invisible direction, which is why
/// the conversion happens once, in the pure layer, and this only unpacks it.
const fn clear_color(config: &TerrainPassConfig) -> wgpu::Color {
    let [red, green, blue] = config.clear_color_linear;
    wgpu::Color {
        r: red,
        g: green,
        b: blue,
        a: 1.0,
    }
}
