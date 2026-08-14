//! The HUD pass: its pipeline, the uniform it reads, and the one draw it
//! records.
//!
//! # A second render pass, loading what the first one left
//!
//! The colour attachment is `LoadOp::Load` and there is **no depth attachment**.
//! A HUD element is not in the world and has nothing to be occluded by; giving
//! it a depth test would mean either clearing depth the terrain pass just wrote
//! or testing against it, and both are ways for a crosshair to disappear behind
//! a block.
//!
//! **The pass is recorded whether or not the plan holds anything.** Skipping it
//! for an empty layout would be cheaper by one pass, and it would also make "a
//! frame declaring no elements is the frame the terrain pass alone draws"
//! unfalsifiable against a HUD pass that clears — the early return, not the load
//! op, would be what preserved the picture. A zero-instance draw is legal and
//! costs nothing worth the loss of that falsifier.
//!
//! # Blending is the hardware's, in linear light
//!
//! `SrcAlpha`/`OneMinusSrcAlpha` over an `Rgba8UnormSrgb` target: the hardware
//! decodes the destination, blends in linear light and re-encodes on write.
//! That is why `#FFFFFF80` over `#000000FF` reads back as **188** per channel
//! and not the 128 the hex digits suggest. Colours arrive already decoded from
//! `src/hud/uniform.rs`; nothing in this file converts one.

use crate::hud::{HUD_UNIFORM_BYTES, HudUniform, PaintedRect};
use crate::pass::TerrainPassConfig;
use crate::surface::SurfaceSize;

use super::{RecordTarget, color_format};

/// The screen-space vertex and fragment stages.
const HUD_SOURCE: &str = include_str!("../../shaders/hud.wgsl");

/// How many vertices one rectangle is drawn by: two triangles, no index buffer.
const VERTICES_PER_RECT: u32 = 6;

/// The array texture a textured element's swatch is sampled from, and the
/// sampler it is read through.
///
/// The two travel together because a bind group needs both and because they are
/// one decision: which texels a swatch shows depends on the filtering as much as
/// on the layer, and the answer is the terrain's for both.
#[derive(Debug, Clone, Copy)]
pub(super) struct ArrayTexture<'a> {
    pub(super) view: &'a wgpu::TextureView,
    pub(super) sampler: &'a wgpu::Sampler,
}

/// The HUD pass's pipeline, the uniform it reads and the group binding it.
#[derive(Debug)]
pub(super) struct HudPass {
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
}

impl HudPass {
    /// Builds the pass against the colour format `config` declares, reading
    /// `textures` for the swatch a textured element samples.
    ///
    /// The uniform is allocated once, at capacity, exactly as every other buffer
    /// in this module is: composing a frame writes bytes and allocates nothing.
    ///
    /// **The array texture is the terrain's own, borrowed rather than copied.**
    /// A swatch of a block has to be the texture that block draws with, and a
    /// second array filled from the same layers would be a second answer waiting
    /// to disagree with the first. The view outlives every upload into it, so the
    /// bind group built here stays valid for the life of the pass.
    pub(super) fn new(
        device: &wgpu::Device,
        config: &TerrainPassConfig,
        textures: ArrayTexture<'_>,
    ) -> Self {
        let layout = bindings(device);
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mycraft hud"),
            size: HUD_UNIFORM_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let entries = bound(&uniform, textures);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mycraft hud"),
            layout: &layout,
            entries: &entries,
        });
        Self {
            pipeline: pipeline(device, config, &layout),
            uniform,
            group,
        }
    }

    /// Records the pass painting `planned` onto `target`.
    ///
    /// One draw call whatever the plan holds, and one that is issued even when
    /// the plan is empty — see this module's note on why the pass is not skipped.
    pub(super) fn record(&self, target: RecordTarget<'_>, planned: &[PaintedRect]) {
        let HudUniform { bytes, rects } = crate::hud::hud_uniform(planned, target.size);
        target.queue.write_buffer(&self.uniform, 0, &bytes);

        let attachments = [Some(loaded(target.color))];
        let mut pass = target
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mycraft hud"),
                color_attachments: &attachments,
                ..wgpu::RenderPassDescriptor::default()
            });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.draw(0..VERTICES_PER_RECT, 0..rects);
    }
}

/// Whether a target has pixels the pass could paint into.
///
/// A minimised window reports a zero extent, and a render pass over a target of
/// no area is a validation error rather than a frame nobody sees.
pub(super) fn paintable(size: SurfaceSize) -> bool {
    size.width > 0 && size.height > 0
}

/// The colour attachment, keeping whatever the terrain pass left in it.
const fn loaded(view: &wgpu::TextureView) -> wgpu::RenderPassColorAttachment<'_> {
    wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        },
    }
}

/// What the pass binds: the one uniform carrying the target's size and the
/// rectangles, and the array texture a textured rectangle samples. No storage
/// buffer, so the four-per-stage budget is untouched.
///
/// The uniform is visible to the vertex stage alone. A layer index reaches the
/// fragment stage as a flat vertex output rather than by binding the uniform
/// twice, so nothing here has to be visible to both.
fn bindings(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entries = declared_bindings();
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mycraft hud bindings"),
        entries: &entries,
    })
}

/// The three the pass declares, in the order the shader numbers them.
const fn declared_bindings() -> [wgpu::BindGroupLayoutEntry; 3] {
    [
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
    ]
}

/// What the group binds those three to.
fn bound<'a>(
    uniform: &'a wgpu::Buffer,
    textures: ArrayTexture<'a>,
) -> [wgpu::BindGroupEntry<'a>; 3] {
    [
        wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::TextureView(textures.view),
        },
        wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::Sampler(textures.sampler),
        },
    ]
}

/// The render pipeline the HUD is composed by.
fn pipeline(
    device: &wgpu::Device,
    config: &TerrainPassConfig,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mycraft hud"),
        source: wgpu::ShaderSource::Wgsl(HUD_SOURCE.into()),
    });
    let targets = [Some(blended_target(config))];
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("mycraft hud"),
        bind_group_layouts: &[Some(layout)],
        ..wgpu::PipelineLayoutDescriptor::default()
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mycraft hud"),
        layout: Some(&pipeline_layout),
        vertex: vertex_state(&module),
        primitive: screen_space(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(fragment_state(&module, &targets)),
        multiview_mask: None,
        cache: None,
    })
}

/// The vertex stage, reading no vertex buffer at all: a rectangle's corners come
/// from the vertex index and its four numbers from the uniform.
fn vertex_state(module: &wgpu::ShaderModule) -> wgpu::VertexState<'_> {
    wgpu::VertexState {
        module,
        entry_point: Some("vertex_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        buffers: &[],
    }
}

/// The fragment stage, writing the one blended colour target.
fn fragment_state<'a>(
    module: &'a wgpu::ShaderModule,
    targets: &'a [Option<wgpu::ColorTargetState>],
) -> wgpu::FragmentState<'a> {
    wgpu::FragmentState {
        module,
        entry_point: Some("fragment_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        targets,
    }
}

/// The colour target, blending source over destination.
///
/// The alpha channel accumulates coverage rather than being scaled by itself, so
/// compositing twice onto the same pixel leaves the attachment's own alpha
/// meaning what it meant before — which matters for a surface that is presented
/// rather than read back.
const fn blended_target(config: &TerrainPassConfig) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format: color_format(config),
        blend: Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        write_mask: wgpu::ColorWrites::ALL,
    }
}

/// How the two triangles of a rectangle are assembled.
///
/// Nothing is culled: a screen-space quad has no back to face away, and a cull
/// mode here would turn a winding mistake into an element that silently does not
/// draw.
const fn screen_space() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
    }
}
