//! The three pipelines a terrain frame runs, and the bind groups they read.
//!
//! There is **one** render-pipeline builder and there is no second one. The
//! offscreen path and the windowed path differ in the colour format their
//! `TerrainPassConfig` carries and in nothing else, so a window and a golden
//! frame are drawn by the same pass by construction rather than by two struct
//! literals somebody keeps in step.
//!
//! **The two terrain draws are that same builder, parameterised.** A face that
//! stops all the light is drawn by the opaque pipeline and one that passes some
//! by the blended one, and the only things that differ are the colour target's
//! blend and whether the draw writes depth. That difference lives in
//! [`TerrainLayer`] here rather than in `TerrainPassConfig`, because
//! `pass.rs` says the colour target is the only thing **a caller** may choose —
//! and no caller chooses between these two, since both are always built from one
//! config and both are recorded on every frame.
//!
//! Both shaders are compiled from source that the build script has already
//! validated at the downlevel profile, so a shader that would not run on the
//! weakest declared adapter fails the build rather than the first draw.

use crate::pass::TerrainPassConfig;

use super::buffers::SceneBuffers;
use super::{color_format, depth_compare, depth_format};

/// The terrain vertex and fragment stages.
const TERRAIN_SOURCE: &str = include_str!("../../shaders/terrain.wgsl");

/// The compute cull and compaction pass.
const CULL_SOURCE: &str = include_str!("../../shaders/cull.wgsl");

/// Which of the two terrain draws a pipeline is built for.
///
/// Private, and it never reaches `TerrainPassConfig`: this is not a choice a
/// caller makes but a property of which half of the index buffer is being drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerrainLayer {
    /// Faces that stop all the light reaching them. Blends nothing and writes
    /// depth, exactly as the one terrain pipeline always did.
    Opaque,
    /// Faces that pass some of it. Blends source over destination and **does not
    /// write depth**: two translucent faces must both be able to reach a pixel
    /// the opaque pass left behind them, and a depth write would let whichever
    /// the compaction happened to order first discard the other.
    Translucent,
}

/// The pipelines and the bind groups they are recorded with.
#[derive(Debug)]
pub(super) struct Pipelines {
    pub(super) cull: wgpu::ComputePipeline,
    pub(super) terrain: wgpu::RenderPipeline,
    pub(super) blended_terrain: wgpu::RenderPipeline,
    pub(super) cull_group: wgpu::BindGroup,
    pub(super) frame_group: wgpu::BindGroup,
    pub(super) texture_group: wgpu::BindGroup,
}

impl Pipelines {
    /// Builds every pipeline against `buffers`.
    pub(super) fn new(
        device: &wgpu::Device,
        config: &TerrainPassConfig,
        buffers: &SceneBuffers,
    ) -> Self {
        let cull_layout = cull_bindings(device);
        let frame_layout = frame_bindings(device);
        let texture_layout = texture_bindings(device);
        let layouts = [Some(&frame_layout), Some(&texture_layout)];
        Self {
            cull: cull_pipeline(device, &cull_layout),
            terrain: terrain_pipeline(device, config, &layouts, TerrainLayer::Opaque),
            blended_terrain: terrain_pipeline(device, config, &layouts, TerrainLayer::Translucent),
            cull_group: cull_group(device, &cull_layout, buffers),
            frame_group: frame_group(device, &frame_layout, buffers),
            texture_group: texture_group(device, &texture_layout, buffers),
        }
    }
}

/// What the cull pass binds: the frame uniform, and the four storage buffers
/// that are its whole budget.
fn cull_bindings(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entries = [
        uniform_entry(0, wgpu::ShaderStages::COMPUTE),
        storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
        storage_entry(2, wgpu::ShaderStages::COMPUTE, false),
        storage_entry(3, wgpu::ShaderStages::COMPUTE, false),
        storage_entry(4, wgpu::ShaderStages::COMPUTE, false),
    ];
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mycraft cull bindings"),
        entries: &entries,
    })
}

/// What the two terrain stages bind: the frame uniform, and the section table
/// the vertex stage reconstructs world positions from.
///
/// **The uniform is visible to the fragment stage and the section table is
/// not.** The fragment stage reads the eye and the medium's tint out of the
/// uniform; it has no use for a section, and widening a storage binding it does
/// not read would spend one of the four per stage that the build enforces.
fn frame_bindings(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entries = [
        uniform_entry(0, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT),
        storage_entry(1, wgpu::ShaderStages::VERTEX, true),
    ];
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mycraft terrain frame bindings"),
        entries: &entries,
    })
}

/// What the fragment stage binds: the array texture and its sampler.
fn texture_bindings(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entries = [
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
    ];
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mycraft terrain texture bindings"),
        entries: &entries,
    })
}

/// One uniform-buffer binding.
const fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// One storage-buffer binding, read-only or read-write.
const fn storage_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    read_only: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// The cull pass's bind group.
fn cull_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffers: &SceneBuffers,
) -> wgpu::BindGroup {
    let entries = [
        bound(0, &buffers.frame),
        bound(1, &buffers.sections),
        bound(2, &buffers.visible),
        bound(3, &buffers.indices),
        bound(4, &buffers.args),
    ];
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mycraft cull"),
        layout,
        entries: &entries,
    })
}

/// The vertex stage's bind group.
fn frame_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffers: &SceneBuffers,
) -> wgpu::BindGroup {
    let entries = [bound(0, &buffers.frame), bound(1, &buffers.sections)];
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mycraft terrain frame"),
        layout,
        entries: &entries,
    })
}

/// The fragment stage's bind group.
fn texture_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffers: &SceneBuffers,
) -> wgpu::BindGroup {
    let entries = [
        wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&buffers.texture),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&buffers.sampler),
        },
    ];
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mycraft terrain textures"),
        layout,
        entries: &entries,
    })
}

/// One whole buffer, bound at `binding`.
fn bound(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

/// The compute pipeline that culls and compacts.
fn cull_pipeline(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mycraft cull"),
        source: wgpu::ShaderSource::Wgsl(CULL_SOURCE.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("mycraft cull"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("mycraft cull"),
                bind_group_layouts: &[Some(layout)],
                ..wgpu::PipelineLayoutDescriptor::default()
            }),
        ),
        module: &module,
        entry_point: Some("cull_sections"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

/// What a graphics debugger calls the pipeline that draws `drawing`.
///
/// Two names rather than one, because a frame that draws the wrong half is read
/// off a capture by which pipeline issued which draw.
const fn label_of(drawing: TerrainLayer) -> &'static str {
    match drawing {
        TerrainLayer::Opaque => "mycraft terrain",
        TerrainLayer::Translucent => "mycraft terrain blended",
    }
}

/// The render pipeline that draws one layer of terrain, in the one
/// configuration both paths build.
fn terrain_pipeline(
    device: &wgpu::Device,
    config: &TerrainPassConfig,
    layouts: &[Option<&wgpu::BindGroupLayout>],
    drawing: TerrainLayer,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mycraft terrain"),
        source: wgpu::ShaderSource::Wgsl(TERRAIN_SOURCE.into()),
    });
    let label = label_of(drawing);
    let targets = [Some(color_target(config, drawing))];
    let vertices = [Some(vertex_layout(config))];
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: layouts,
        ..wgpu::PipelineLayoutDescriptor::default()
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vertex_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertices,
        },
        primitive: primitive(config),
        depth_stencil: Some(depth_state(config, drawing)),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(fragment_state(&module, &targets)),
        multiview_mask: None,
        cache: None,
    })
}

/// The one colour target, in the format `config` declares.
///
/// The blended layer's `BlendState` is the HUD pass's verbatim, which is the one
/// blend this backend is already proven on. Its alpha component accumulates
/// coverage rather than scaling by itself, so compositing twice onto one pixel
/// leaves the attachment's own alpha meaning what it meant — which is what a
/// surface that is presented rather than read back needs, and costs a read-back
/// capture nothing.
const fn color_target(config: &TerrainPassConfig, drawing: TerrainLayer) -> wgpu::ColorTargetState {
    let blend = match drawing {
        TerrainLayer::Opaque => None,
        TerrainLayer::Translucent => Some(wgpu::BlendState {
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
    };
    wgpu::ColorTargetState {
        format: color_format(config),
        blend,
        write_mask: wgpu::ColorWrites::ALL,
    }
}

/// The fragment stage, writing the one colour target.
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

/// One packed vertex per step: eight bytes, read as two `u32`s because WGSL has
/// no sixty-four bit integer.
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Uint32x2,
    offset: 0,
    shader_location: 0,
}];

/// How the vertex buffer is stepped through.
const fn vertex_layout(config: &TerrainPassConfig) -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: config.vertex_stride as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    }
}

/// How triangles are assembled and which side of them is discarded.
const fn primitive(config: &TerrainPassConfig) -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: match config.front_face {
            crate::pass::FrontFace::Ccw => wgpu::FrontFace::Ccw,
        },
        cull_mode: match config.cull_mode {
            crate::pass::CullMode::Back => Some(wgpu::Face::Back),
        },
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
    }
}

/// How a fragment's depth is tested and written.
///
/// **Both layers test; only the opaque one writes.** Testing is what keeps an
/// opaque face in front of a translucent one from being drawn over — the blended
/// draw runs second in the same pass and reads the depth the first wrote, so a
/// hidden translucent face is discarded before it can blend into anything.
/// Writing is what the blended layer must not do: a translucent face that wrote
/// depth would discard whatever the compaction happened to order after it,
/// including a second translucent face standing behind it that is exactly what
/// a composition is made of.
fn depth_state(config: &TerrainPassConfig, drawing: TerrainLayer) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: depth_format(config),
        depth_write_enabled: Some(matches!(drawing, TerrainLayer::Opaque)),
        depth_compare: Some(depth_compare(config)),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}
