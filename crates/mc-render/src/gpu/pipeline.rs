//! The two pipelines a terrain frame runs, and the bind groups they read.
//!
//! There is **one** render-pipeline builder and there is no second one. The
//! offscreen path and the windowed path differ in the colour format their
//! `TerrainPassConfig` carries and in nothing else, so a window and a golden
//! frame are drawn by the same pass by construction rather than by two struct
//! literals somebody keeps in step.
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

/// The pipelines and the bind groups they are recorded with.
#[derive(Debug)]
pub(super) struct Pipelines {
    pub(super) cull: wgpu::ComputePipeline,
    pub(super) terrain: wgpu::RenderPipeline,
    pub(super) cull_group: wgpu::BindGroup,
    pub(super) frame_group: wgpu::BindGroup,
    pub(super) texture_group: wgpu::BindGroup,
}

impl Pipelines {
    /// Builds both pipelines against `buffers`.
    pub(super) fn new(
        device: &wgpu::Device,
        config: &TerrainPassConfig,
        buffers: &SceneBuffers,
    ) -> Self {
        let cull_layout = cull_bindings(device);
        let frame_layout = frame_bindings(device);
        let texture_layout = texture_bindings(device);
        Self {
            cull: cull_pipeline(device, &cull_layout),
            terrain: terrain_pipeline(
                device,
                config,
                &[Some(&frame_layout), Some(&texture_layout)],
            ),
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

/// What the vertex stage binds: the frame uniform, and the section table it
/// reconstructs world positions from.
fn frame_bindings(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entries = [
        uniform_entry(0, wgpu::ShaderStages::VERTEX),
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

/// The render pipeline that draws terrain, in the one configuration both paths
/// build.
fn terrain_pipeline(
    device: &wgpu::Device,
    config: &TerrainPassConfig,
    layouts: &[Option<&wgpu::BindGroupLayout>],
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mycraft terrain"),
        source: wgpu::ShaderSource::Wgsl(TERRAIN_SOURCE.into()),
    });
    let targets = [Some(color_target(config))];
    let vertices = [Some(vertex_layout(config))];
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("mycraft terrain"),
        bind_group_layouts: layouts,
        ..wgpu::PipelineLayoutDescriptor::default()
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mycraft terrain"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vertex_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertices,
        },
        primitive: primitive(config),
        depth_stencil: Some(depth_state(config)),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(fragment_state(&module, &targets)),
        multiview_mask: None,
        cache: None,
    })
}

/// The one colour target, in the format `config` declares.
const fn color_target(config: &TerrainPassConfig) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format: color_format(config),
        blend: None,
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
fn depth_state(config: &TerrainPassConfig) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: depth_format(config),
        depth_write_enabled: Some(true),
        depth_compare: Some(depth_compare(config)),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}
