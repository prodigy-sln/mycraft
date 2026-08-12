//! Every buffer and texture the terrain pass reads or writes, allocated once at
//! the capacities the scene may not exceed.
//!
//! Nothing here is sized to a scene. `SceneGeometry::assemble` refuses a scene
//! over `MAX_SECTIONS` or `MAX_QUADS`, and these are the buffers those two
//! numbers are the capacities *of* — so uploading a scene writes bytes into
//! storage that already exists. A re-mesh therefore costs a queue write and
//! never an allocation, and there is no frame on which a buffer is created.
//!
//! **The array texture is sRGB, and that is load-bearing.** A texel sampled from
//! an `Rgba8UnormSrgb` texture is decoded to linear, and the colour target
//! encodes it back on write, so the byte a capture reads back is the byte the
//! placeholder generator produced. An `Rgba8Unorm` texture would skip the decode
//! and the frame would come back lighter than any declared mean colour —
//! plausible-looking, and wrong in the direction nothing notices.

use mc_core::id::TextureKey;

use crate::geometry::scene::{MAX_QUADS, MAX_SECTIONS, SceneGeometry};
use crate::geometry::vertex::MAX_LAYER;
use crate::texture::TextureLayers;
use crate::texture::placeholder::{PLACEHOLDER_SIZE, placeholder_texels};

use super::RendererError;

/// How many corners one quad has.
const CORNERS_PER_QUAD: u64 = 4;

/// How many indices one quad is drawn by.
pub(super) const INDICES_PER_QUAD: u32 = 6;

/// Bytes in one packed vertex.
const VERTEX_BYTES: u64 = 8;

/// Bytes in one index.
const INDEX_BYTES: u64 = 4;

/// Bytes in one section table record: origin, first quad, quad count, and the
/// box's two corners. Declared by `SceneGeometry::section_bytes`; `cull.wgsl`'s
/// `Section` struct is written against the same layout, field by field.
const SECTION_BYTES: u64 = 44;

/// Bytes in one visibility flag.
const FLAG_BYTES: u64 = 4;

/// How many layers the array texture holds: every index the packed vertex's
/// layer field can express.
const TEXTURE_LAYERS: u32 = MAX_LAYER + 1;

/// Bytes in one RGBA8 texel.
const TEXEL_BYTES: u32 = 4;

/// The five `u32`s of `wgpu::util::DrawIndexedIndirectArgs`, in the order the
/// device reads them.
///
/// Exactly one of them varies: the compute pass raises `index_count` by six per
/// quad it compacts. `instance_count` is 1 and `first_instance` is 0 so that
/// neither `MULTI_DRAW_INDIRECT` nor `INDIRECT_FIRST_INSTANCE` is required of
/// the device — the optional-feature set stays empty, which is a requirement in
/// its own right.
const INDIRECT_ARGS: [u32; 5] = [0, 1, 0, 0, 0];

/// Bytes in the indirect argument buffer.
const ARGS_BYTES: u64 = 20;

/// Bytes in the per-frame uniform: a `mat4x4<f32>` and six `vec4<f32>` planes.
const FRAME_UNIFORM_BYTES: u64 = 64 + 96;

/// Everything the pass binds.
#[derive(Debug)]
pub(super) struct SceneBuffers {
    pub(super) frame: wgpu::Buffer,
    pub(super) vertices: wgpu::Buffer,
    pub(super) sections: wgpu::Buffer,
    pub(super) visible: wgpu::Buffer,
    pub(super) indices: wgpu::Buffer,
    pub(super) args: wgpu::Buffer,
    pub(super) texture: wgpu::TextureView,
    pub(super) sampler: wgpu::Sampler,
}

impl SceneBuffers {
    /// Allocates every buffer at capacity and leaves the device in the state a
    /// frame that drew nothing would have left it.
    ///
    /// The indirect arguments and the visibility flags are written here as well
    /// as per frame, so that reading either one before the first frame reports a
    /// declared value rather than whatever the allocator handed over.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError`] when a declared capacity cannot be built.
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self, RendererError> {
        let texture = array_texture(device);
        let buffers = Self {
            frame: uniform(device, "frame", FRAME_UNIFORM_BYTES),
            vertices: vertex_buffer(device),
            sections: storage(device, "sections", SECTION_BYTES * MAX_SECTIONS as u64),
            visible: readable_storage(device, "visible", FLAG_BYTES * MAX_SECTIONS as u64),
            indices: index_buffer(device),
            args: indirect(device),
            texture: texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..wgpu::TextureViewDescriptor::default()
            }),
            sampler: sampler(device),
        };
        buffers.reset_draw(queue);
        queue.write_buffer(
            &buffers.visible,
            0,
            &vec![0; (FLAG_BYTES * MAX_SECTIONS as u64) as usize],
        );
        Ok(buffers)
    }

    /// Writes the indirect arguments back to their pre-frame state.
    ///
    /// The compute pass raises `index_count` with an atomic add, so it has to
    /// start each frame at zero — this is the CPU half of that, and it is why
    /// nothing needs a second dispatch or a prefix sum.
    pub(super) fn reset_draw(&self, queue: &wgpu::Queue) {
        let bytes: Vec<u8> = INDIRECT_ARGS
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        queue.write_buffer(&self.args, 0, &bytes);
    }

    /// Uploads `scene`'s packed vertices and section table.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::SceneTooLarge`] when the scene does not fit,
    /// which assembly has already ruled out — this is the second half of that
    /// refusal, kept because a buffer overrun is the one failure here that would
    /// otherwise be a device-level crash rather than a reported error.
    pub(super) fn write_scene(
        &self,
        queue: &wgpu::Queue,
        scene: &SceneGeometry,
    ) -> Result<(), RendererError> {
        let vertices = scene.vertex_bytes();
        let sections = scene.section_bytes();
        fits(vertices.len(), self.vertices.size(), "vertex bytes")?;
        fits(sections.len(), self.sections.size(), "section bytes")?;
        queue.write_buffer(&self.vertices, 0, &vertices);
        queue.write_buffer(&self.sections, 0, &sections);
        Ok(())
    }

    /// Writes each resolved key's placeholder texels into the layer it occupies.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::TextureLayerOutOfRange`] when a key resolved to
    /// a layer the array does not hold.
    pub(super) fn write_textures(
        &self,
        queue: &wgpu::Queue,
        layers: &TextureLayers,
    ) -> Result<(), RendererError> {
        for (key, layer) in layers.entries() {
            self.write_layer(queue, key, layer)?;
        }
        Ok(())
    }

    /// Writes one key's placeholder texels into the layer it occupies.
    fn write_layer(
        &self,
        queue: &wgpu::Queue,
        key: &TextureKey,
        layer: u16,
    ) -> Result<(), RendererError> {
        if u32::from(layer) >= TEXTURE_LAYERS {
            return Err(RendererError::TextureLayerOutOfRange {
                layer,
                // Bounded by the packed field's width, which is 8 bits.
                capacity: TEXTURE_LAYERS as u16,
            });
        }
        let texels: Vec<u8> = placeholder_texels(key, PLACEHOLDER_SIZE)
            .into_iter()
            .flatten()
            .collect();
        queue.write_texture(
            layer_origin(&self.texture, layer),
            &texels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(PLACEHOLDER_SIZE * TEXEL_BYTES),
                rows_per_image: Some(PLACEHOLDER_SIZE),
            },
            one_layer(),
        );
        Ok(())
    }
}

/// Fails unless `found` bytes fit in `capacity`.
fn fits(found: usize, capacity: u64, resource: &'static str) -> Result<(), RendererError> {
    if found as u64 <= capacity {
        return Ok(());
    }
    Err(RendererError::SceneTooLarge {
        resource,
        found,
        // The capacity is a compile-time product of two constants and fits.
        capacity: capacity as usize,
    })
}

/// Where one array layer's texels are copied to.
fn layer_origin(view: &wgpu::TextureView, layer: u16) -> wgpu::TexelCopyTextureInfo<'_> {
    wgpu::TexelCopyTextureInfo {
        texture: view.texture(),
        mip_level: 0,
        origin: wgpu::Origin3d {
            x: 0,
            y: 0,
            z: u32::from(layer),
        },
        aspect: wgpu::TextureAspect::All,
    }
}

/// The extent of exactly one array layer.
const fn one_layer() -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: PLACEHOLDER_SIZE,
        height: PLACEHOLDER_SIZE,
        depth_or_array_layers: 1,
    }
}

/// The array texture every block's placeholder occupies a layer of.
fn array_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mycraft terrain textures"),
        size: wgpu::Extent3d {
            width: PLACEHOLDER_SIZE,
            height: PLACEHOLDER_SIZE,
            depth_or_array_layers: TEXTURE_LAYERS,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

/// The sampler every terrain fragment reads through.
///
/// Nearest and repeating. Nearest because a placeholder texture is sixteen
/// texels of deliberate pattern and filtering would blur it towards its own mean
/// — which is the very value the probes cluster against, so a filtered frame
/// would agree with them for a reason that has nothing to do with the texture
/// being right. Repeating because a merged quad's texture coordinates run in
/// whole blocks, so a face four blocks wide shows the texture four times.
fn sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("mycraft terrain sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..wgpu::SamplerDescriptor::default()
    })
}

/// The packed vertex buffer, at the capacity assembly enforces.
fn vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    allocate(
        device,
        "vertices",
        VERTEX_BYTES * CORNERS_PER_QUAD * MAX_QUADS as u64,
        wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    )
}

/// The destination index buffer the compute pass compacts into.
fn index_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    allocate(
        device,
        "indices",
        INDEX_BYTES * u64::from(INDICES_PER_QUAD) * MAX_QUADS as u64,
        wgpu::BufferUsages::INDEX | wgpu::BufferUsages::STORAGE,
    )
}

/// The indirect arguments the one terrain draw reads.
fn indirect(device: &wgpu::Device) -> wgpu::Buffer {
    allocate(
        device,
        "indirect args",
        ARGS_BYTES,
        wgpu::BufferUsages::INDIRECT
            | wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
    )
}

/// A uniform buffer of `size` bytes.
fn uniform(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    allocate(
        device,
        label,
        size,
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    )
}

/// A storage buffer of `size` bytes.
fn storage(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    allocate(
        device,
        label,
        size,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    )
}

/// A storage buffer of `size` bytes that can also be copied out for a readback.
fn readable_storage(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    allocate(
        device,
        label,
        size,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
    )
}

/// One buffer, labelled so a graphics debugger names it.
fn allocate(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("mycraft terrain {label}")),
        size,
        usage,
        mapped_at_creation: false,
    })
}
