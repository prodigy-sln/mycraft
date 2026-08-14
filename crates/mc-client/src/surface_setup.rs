//! Configuring a surface for the window it came from: which of the formats it
//! offers is taken, what the renderer calls that format, and the configuration
//! the two produce.
//!
//! **Split out of the frame path rather than sitting beside it.** Configuring a
//! surface happens once, before a frame is ever drawn, and it answers a different
//! question from "what does this frame show" — so a reader chasing a format
//! decision does not read past the frame path to find it, and neither grows by
//! the other's changes.
//!
//! Every decision here is somebody else's. Which format is configured is
//! `mc_render::surface::select_surface_format`, a pure function with a test that
//! never opened a window; what is left is carrying its answer to the graphics
//! API, which is the whole reason this crate holds no coverage of its own.

use mc_core::hud::HudLoadError;
use mc_render::gpu::RendererError;
use mc_render::pass::ColorFormat;
use mc_render::surface::{FormatError, SurfaceFormatFacts, SurfaceSize, select_surface_format};
use thiserror::Error;

use crate::gpu_startup::Gpu;

/// Why the client could not be built around the window it was given.
#[derive(Debug, Error)]
pub enum SetupError {
    #[error("the surface offers no format this client can present through")]
    Format(#[from] FormatError),
    #[error(
        "the surface's first sRGB format is `{name}`, which this renderer has no pass \
         configuration for"
    )]
    UnsupportedFormat { name: String },
    #[error("the surface reported no default configuration for this adapter")]
    NoDefaultConfiguration,
    #[error(
        "the surface's format list no longer holds the format at index {index} that was chosen \
         from it"
    )]
    FormatVanished { index: usize },
    #[error("the terrain pass could not be built")]
    Renderer(#[from] RendererError),
    #[error("the HUD a client composes before it has read its content could not be built")]
    Hud(#[from] HudLoadError),
}

/// Which of the formats a surface offers is configured.
pub(crate) fn chosen_format(
    offered: &[wgpu::TextureFormat],
) -> Result<wgpu::TextureFormat, SetupError> {
    let facts: Vec<SurfaceFormatFacts> = offered
        .iter()
        .map(|format| SurfaceFormatFacts {
            name: format!("{format:?}"),
            is_srgb: format.is_srgb(),
        })
        .collect();
    let index = select_surface_format(&facts)?;
    offered
        .get(index)
        .copied()
        // Unreachable: the index came from the list this one was built from. It is
        // an error rather than a panic because this is a player's startup path.
        .ok_or(SetupError::FormatVanished { index })
}

/// The pass's colour target, as the renderer spells it.
pub(crate) fn color_format(format: wgpu::TextureFormat) -> Result<ColorFormat, SetupError> {
    match format {
        wgpu::TextureFormat::Rgba8UnormSrgb => Ok(ColorFormat::Rgba8UnormSrgb),
        wgpu::TextureFormat::Bgra8UnormSrgb => Ok(ColorFormat::Bgra8UnormSrgb),
        other => Err(SetupError::UnsupportedFormat {
            name: format!("{other:?}"),
        }),
    }
}

/// The surface's own default configuration, pointed at the format that was
/// chosen rather than the one it would have picked.
pub(crate) fn configuration_for(
    surface: &wgpu::Surface<'static>,
    gpu: &Gpu,
    size: SurfaceSize,
    format: wgpu::TextureFormat,
) -> Result<wgpu::SurfaceConfiguration, SetupError> {
    let mut configuration = surface
        .get_default_config(&gpu.adapter, size.width, size.height)
        .ok_or(SetupError::NoDefaultConfiguration)?;
    configuration.format = format;
    configuration.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    Ok(configuration)
}
