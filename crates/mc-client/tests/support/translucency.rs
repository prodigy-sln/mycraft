//! A world of flat-coloured panes standing one behind another, declared through
//! a content root and drawn offscreen.
//!
//! # What is a fixture here and what is the shipped path
//!
//! Two things are the fixture's: the **declarations** (a temporary content root
//! holding nothing but the blocks a reading names) and the **geometry** (a
//! handful of quads placed by hand, because no seed generates a world with one
//! pane four blocks behind another). Everything between them is the client's
//! own: `mc_sim::content::load` reads the root, `resolved_from` folds it into
//! what a participant that only draws receives, `ContentView::of` turns that
//! into the resolution a packer is handed, `build_section_geometry` packs the
//! quads against it and `TerrainRenderer` draws them. A declared degree
//! therefore travels the whole way a shipped one does; only the world it lands
//! in is chosen rather than generated.
//!
//! # Every layer is one flat colour, and that is the point
//!
//! A blend expectation is an exact triple, and it is only exact if the two
//! colours going into it are. A generated layer holds two colours on a
//! checkerboard and the shipped art holds three to six, so a pixel drawn from
//! one may legitimately be either of them — which turns every expectation below
//! into a band and buys nothing, because what these readings are about is the
//! *arithmetic between two layers* rather than the variety inside one. A flat
//! layer also removes minification from the question entirely: nearest
//! magnification, linear minification and mip interpolation all answer the same
//! byte for a texture of one colour, so the same expectation holds at any
//! distance.
//!
//! **It is not a form no caller supplies.** A flat 16 x 16 PNG is an ordinary
//! image and `decode::texels_of` builds exactly this value out of one. What the
//! fixture skips is the file, not the shape.
//!
//! # The declaration is written out here rather than built
//!
//! `support/reload.rs`'s `Declaration` builder is reached by `#[path]` from a
//! test binary rather than declared in this module tree, so a module inside
//! `support/` cannot see it — and every binary saying `mod support;` compiles
//! this file. The chunk below is written directly instead, which is what
//! `an_unauthored_key_draws_a_generated_texture.rs` already does for the same
//! reason. It is four lines and one of them is the subject.
//!
//! # Where the numbers come from, and the trap they exist to catch
//!
//! The colour target is `Rgba8UnormSrgb`. The hardware **decodes to linear,
//! blends, and re-encodes**, so an expectation computed on the stored bytes is
//! not merely imprecise — for this fixture's own colours it stands **ΔE 15.60**
//! from the correct answer at a half blend and **ΔE 9.72** at a quarter, both
//! well past the ΔE 6 these readings call two colours the same. Every expected
//! triple here goes through [`super::art::composited`], which does the
//! arithmetic in linear light through a transfer pair written from
//! IEC 61966-2-1 and shared with nothing in the draw path.
//!
//! # The palette, and the separation it was chosen for
//!
//! Three block colours, measured pairwise against each other, against the sky
//! and against every composite these readings name. **The closest two colours
//! any reading has to tell apart stand ΔE 15.40 apart** — the sky
//! `(135, 206, 235)` and a quarter of the pane over it, `(168, 189, 208)` —
//! which is what [`TELLS_THEM_APART`] is derived against. The next closest are
//! the pane's own colour against two panes composited over the wall at ΔE 25.06,
//! and one pane over the wall against two at ΔE 25.38.
//!
//! No assertion can enforce that a fixture's colours stand far enough apart, so
//! [`super::pixel_census::require_told_apart`] asserts it on every run rather
//! than leaving it to this paragraph.

use std::error::Error;
use std::ops::Range;
use std::sync::Arc;

use mc_client::content::ContentView;
use mc_core::content::LayerAssignment;
use mc_core::id::{BlockName, TextureKey};
use mc_render::camera::camera_view;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::geometry::{SectionOrigin, build_section_geometry};
use mc_render::gpu::{RecordTarget, TerrainRenderer, TerrainTextures};
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_render::surface::SurfaceSize;
use mc_render::texture::TextureResolution;
use mc_render::texture::sampler::TERRAIN_SAMPLER;
use mc_render::texture::supplied::SuppliedTexels;
use mc_testkit::frame::gpu::{CaptureContext, CaptureRequest, draw_fn};
use mc_testkit::frame::{CaptureId, Rgba8Image, validate_frame_size};
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};

use super::content::shipped_copy;

/// The frame these readings are drawn into.
///
/// Square, and large enough that every region below covers thousands of pixels
/// against the hundred a scenario asks for. It is not the declared capture size,
/// deliberately: nothing here is a golden and nothing here is shot through the
/// player's camera.
pub const FRAME: SurfaceSize = SurfaceSize {
    width: 256,
    height: 256,
};

/// How many pixels one of these frames holds.
pub const PIXELS_IN_THE_FRAME: u64 = (FRAME.width as u64) * (FRAME.height as u64);

/// Where the eye stands and what it looks at.
///
/// Straight down `-Z` at the middle of the section, far enough back that the
/// whole 16-block face fits inside the frame with sky around it. Every pane is a
/// `+Z` face, so every one of them turns its normal at this eye and none is
/// back-face culled.
pub const EYE: [f32; 3] = [8.0, 8.0, 40.0];
pub const LOOK_AT: [f32; 3] = [8.0, 8.0, 0.0];

/// How far a rendered pixel may sit from the colour it is expected to be, in ΔE.
///
/// **Derived from both directions and never loosened until green.**
///
/// The floor is what a correct frame can still be off by. Three terms, all
/// measured over this fixture's own palette: the layer's texel spread is
/// **0.00**, because every layer is one flat colour; the declared degree is
/// quantised to a byte before it reaches a fragment, which moves the composite
/// by at most **ΔE 0.47** (that is the two-pane case — the others move by
/// 0.00); and the hardware's own rounding through an 8-bit sRGB attachment is
/// worth at most a code value or two, which over these colours is **ΔE 2.68**
/// at the very worst. That puts the floor near **ΔE 3.2**.
///
/// The ceiling is half the closest separation any reading has to preserve. The
/// closest pair is the sky against a quarter of the pane over it at **ΔE
/// 15.40**, and two tolerance balls stay disjoint only while the tolerance is
/// under half of that — **ΔE 7.70**.
///
/// **6.0** sits in that bracket, comfortably above the floor and comfortably
/// under the ceiling. Whoever widens it is eating into 1.7 ΔE of headroom, not
/// nine: past 7.70 a pixel can belong to two named colours at once and the
/// census stops meaning anything.
pub const TELLS_THEM_APART: f64 = 6.0;

/// One block a fixture root declares, and what fills the layer it draws from.
#[derive(Debug, Clone, Copy)]
pub struct Declared {
    /// The block's name, which is also the texture key it declares.
    pub block: &'static str,
    /// The one colour its whole layer is filled with.
    pub colour: [u8; 3],
    /// The alpha every texel of that layer carries.
    ///
    /// `255` for every block but the one reading that is about a texture's own
    /// alpha reaching the sampler.
    pub alpha: u8,
    /// The degree of opacity the declaration states.
    pub opacity: f32,
}

impl Declared {
    /// An opaque block of `block`, drawn in `colour`.
    #[must_use]
    pub const fn opaque(block: &'static str, colour: [u8; 3]) -> Self {
        Self {
            block,
            colour,
            alpha: 255,
            opacity: 1.0,
        }
    }

    /// The same block declaring `opacity`.
    #[must_use]
    pub const fn at(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// The same block whose every texel carries `alpha`.
    #[must_use]
    pub const fn textured_at_alpha(mut self, alpha: u8) -> Self {
        self.alpha = alpha;
        self
    }
}

/// One face standing in the section's `x`–`y` plane at a stated depth.
///
/// `plane` is the emitting voxel's own `z`, so the face itself sits at
/// `plane + 1` — the asymmetry `geometry::mod`'s header states. A larger `plane`
/// is therefore *nearer* the eye, which stands at `z = 40`.
#[derive(Debug, Clone)]
pub struct Pane {
    pub block: &'static str,
    pub plane: u32,
    pub x: Range<u32>,
    pub y: Range<u32>,
}

/// One drawn frame, and what filled the layers it was drawn from.
#[derive(Debug)]
pub struct Shot {
    pub frame: Rgba8Image,
    /// The texels every layer was filled with, so that a reading takes a
    /// layer's colour from what fills it rather than from the frame it is
    /// judging.
    pub texels: SuppliedTexels,
    /// Every key the resolution assigned a layer, in the order it assigned
    /// them.
    pub keys: Vec<TextureKey>,
}

/// The frame `panes` draw when `declared` is what the content root says, or
/// `None` when the opt-in permitted the absence of a device.
///
/// # Errors
///
/// Returns the root's own refusal, the packing failure, or the capture failure.
/// A refusal here is a fixture that declared something the loader will not keep,
/// which is a broken fixture rather than a failed behaviour.
pub fn drawn(declared: &[Declared], panes: &[Pane]) -> Result<Option<Shot>, Box<dyn Error>> {
    let root = declaring(declared)?;
    let loaded = mc_sim::content::load(root.path(), &LayerAssignment::none())?;
    let resolution = ContentView::of(&loaded.resolved).into_resolution();
    let texels = flat_layers(declared)?;
    let scene = Arc::new(SceneGeometry::assemble(vec![build_section_geometry(
        &quads(panes)?,
        SectionOrigin::new([0, 0, 0]),
        &resolution,
    )?])?);

    let Some(context) = super::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = uploaded(&context, &resolution, &texels, &scene)?;
    Ok(Some(Shot {
        frame: captured(&context, &mut renderer, &scene)?,
        texels,
        keys: resolution
            .layers()
            .entries()
            .map(|(key, _layer)| key.clone())
            .collect(),
    }))
}

/// A renderer holding `scene` and the array texture `texels` fills.
fn uploaded(
    context: &CaptureContext,
    resolution: &TextureResolution,
    texels: &SuppliedTexels,
    scene: &Arc<SceneGeometry>,
) -> Result<TerrainRenderer, Box<dyn Error>> {
    let mut renderer = TerrainRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
        &TerrainTextures {
            supplied: texels,
            sampler: TERRAIN_SAMPLER,
        },
    )?;
    renderer.upload_textures(context.queue(), resolution.layers())?;
    renderer.upload_scene(context.queue(), scene)?;
    Ok(renderer)
}

/// A content root declaring `declared` and nothing else.
///
/// **`occludes = false` is stated on every block that passes light, and that is
/// not decoration.** `occludes` falls back to `solid`, and these blocks are
/// solid, so a translucent one that said nothing would resolve to a block that
/// both passes light and hides what lies behind it — which the loader refuses,
/// taking the whole root with it. The refusal would be correct and the fixture
/// would be the thing at fault.
fn declaring(declared: &[Declared]) -> Result<super::content::ContentRoot, Box<dyn Error>> {
    let mut root = shipped_copy()?.declaring_no_blocks()?;
    for block in declared {
        let file = format!("{}.luau", block.block.replace(':', "__"));
        root = root.declaring_block(&file, &chunk_for(block))?;
    }
    Ok(root)
}

/// The Luau chunk one of these blocks is declared by.
///
/// `occludes` is stated only where the degree is below one. Stating it on an
/// ordinary opaque block would be a fixture quietly claiming the wall does not
/// hide what stands behind it, which is a different world from the one these
/// readings are about.
fn chunk_for(block: &Declared) -> String {
    let occludes = if block.opacity < 1.0 {
        "	occludes = false,
"
    } else {
        ""
    };
    format!(
        "return {{
	name = \"{name}\",
	texture = \"{name}\",
	solid = true,
         {occludes}	opacity = {opacity:?},
}}
",
        name = block.block,
        opacity = block.opacity,
    )
}

/// A layer of one colour for every block `declared` names.
fn flat_layers(declared: &[Declared]) -> Result<SuppliedTexels, Box<dyn Error>> {
    let mut stated = Vec::with_capacity(declared.len());
    for block in declared {
        let [red, green, blue] = block.colour;
        let texels = vec![
            [red, green, blue, block.alpha];
            (mc_core::content::TEXTURE_EDGE * mc_core::content::TEXTURE_EDGE) as usize
        ];
        stated.push((TextureKey::parse(block.block)?, texels));
    }
    Ok(SuppliedTexels::stating(stated))
}

/// Every pane as the one quad it is.
///
/// A `+Z` facing takes its plane coordinates in `x` primary and `y` secondary,
/// which is `PLANE_AXES`' own row for it.
fn quads(panes: &[Pane]) -> Result<Vec<Quad>, Box<dyn Error>> {
    panes
        .iter()
        .map(|pane| {
            Ok(Quad {
                facing: Facing::PosZ,
                plane: pane.plane,
                origin: PlanePos {
                    primary: pane.x.start,
                    secondary: pane.y.start,
                },
                extent: PlaneExtent {
                    primary: pane.x.end - pane.x.start,
                    secondary: pane.y.end - pane.y.start,
                },
                block: BlockName::parse(pane.block)?,
            })
        })
        .collect()
}

/// The frame `scene` draws through the declared eye.
fn captured(
    context: &CaptureContext,
    renderer: &mut TerrainRenderer,
    scene: &Arc<SceneGeometry>,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let snapshot = TerrainSnapshot {
        tick: 0,
        camera: camera_view(EYE, LOOK_AT),
        scene: Arc::clone(scene),
        // The panes are quads placed by hand with no world behind them, and the
        // declared eye stands well clear of every one of them.
        tint: None,
    };
    let phase = ScenePhase::Ready(Arc::clone(scene));
    let mut ran = false;
    let image;
    {
        let mut work = draw_fn(|encoder, color| {
            let target = RecordTarget {
                device: context.device(),
                queue: context.queue(),
                encoder,
                color,
                size: FRAME,
            };
            renderer.record_terrain(target, &phase, &snapshot)?;
            ran = true;
            Ok(())
        });
        image = context.capture(&request(context)?, &mut work)?.image;
    }
    if !ran {
        return Err(DRAW_WORK_NEVER_RAN.into());
    }
    Ok(image)
}

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so every pixel read back      would be about a target nothing drew into";

/// A request for one of these frames.
fn request(context: &CaptureContext) -> Result<CaptureRequest, Box<dyn Error>> {
    let maximum = context.limits().max_texture_dimension_2d;
    let size = validate_frame_size(FRAME.width, FRAME.height, maximum)?;
    Ok(CaptureRequest::new(
        CaptureId::new("declared-opacity")?,
        size,
    ))
}
