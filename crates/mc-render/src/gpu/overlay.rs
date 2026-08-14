//! The debug overlay's text, painted with `egui`.
//!
//! **This is the only file in the workspace that may name an `egui::` or
//! `egui_wgpu::` path**, and the litmus is that egui disappearing changes one
//! file. A confinement scan is not what holds that — `mc-render/CLAUDE.md`
//! reserves egui for debug and tooling UI, and the reason it is enforceable at
//! all is that the whole of what crosses this boundary is an
//! [`OverlayReadout`]: four plain readings, derived and formatted by a pure
//! module the coverage denominator counts. Nothing above this file knows a
//! toolkit is involved, and nothing in it decides what the overlay says.
//!
//! # Third pass, loading what the first two left
//!
//! Terrain, then the HUD content declared, then this. The colour attachment is
//! `LoadOp::Load` and there is no depth attachment, so the overlay composites
//! over whatever was drawn under it — including an element a content root
//! declared over the whole screen. That pass order is what makes "content
//! cannot obscure the overlay" true by construction, and it is asserted anyway,
//! because true-by-construction is a claim about today's construction.
//!
//! # No `egui-winit`, and the overlay is why that is affordable
//!
//! Taking it would re-arm RUSTSEC-2026-0192 through `winit/default` and name
//! winit in a second client file, which `crates/mc-client/tests/winit_boundary.rs`
//! fails the build over. It is not needed: the overlay accepts no input, so the
//! [`egui::RawInput`] it runs on is built here from the target's own size and
//! carries no events at all. What a window integration exists to forward, this
//! overlay has nothing to do with.
//!
//! # Predictable rather than pretty
//!
//! [`egui_wgpu::RendererOptions::PREDICTABLE`] rather than `default()`, which is
//! a deliberate departure in two respects. `default()` enables **dithering** —
//! noise applied to values falling between two 8-bit steps, which exists to hide
//! banding in gradients. Four lines of text have no gradients, so all it could
//! contribute here is nondeterminism, and this crate's whole verification story
//! is frames a second machine can reproduce. It also turns on software texture
//! filtering, which the same option set exists to provide: target hardware spans
//! an RTX 4090 and an Intel UHD 770, and the glyph atlas should sample the same
//! way on both.
//!
//! **What is deliberately not claimed:** none of that makes rasterised text
//! safe to commit. Drivers disagree about glyphs, and a golden holding one makes
//! whatever rasterised it the ground truth every other machine must reproduce.
//! No declared capture is taken with the overlay shown, and the scenario that
//! grades this overlay's pixels is a difference between two frames.

use egui_wgpu::RendererOptions;

use crate::overlay::{OverlayReadout, readout_lines};
use crate::pass::TerrainPassConfig;

use super::hud_pass::paintable;
use super::{RecordTarget, color_format};

/// Where the readout sits, in pixels from the target's top-left corner.
///
/// Top-left rather than centred or anchored to an edge: the screen centre is
/// reserved for the crosshair, and an instrument that moved when the window
/// resized would be one more thing to account for while reading it.
const READOUT_INSET: f32 = 8.0;

/// One physical pixel per egui point.
///
/// The target's size is in physical pixels and the readout is engine tooling
/// rather than something scaled for a player, so the two spaces are the same
/// space and no conversion has to be right.
const PHYSICAL_PIXELS: f32 = 1.0;

/// The `egui` context and its wgpu renderer, kept across frames.
///
/// Held rather than rebuilt per frame because the font atlas is built on first
/// use and uploaded once: a context built per frame would rasterise every glyph
/// again for every frame the overlay is shown.
pub(super) struct OverlayPass {
    context: egui::Context,
    renderer: egui_wgpu::Renderer,
}

/// Named, not dumped: `egui_wgpu::Renderer` is not [`Debug`] and the enclosing
/// [`FrameRenderer`](super::FrameRenderer) is, so the derive cannot be used.
///
/// Nothing is lost by declining to describe it. A pipeline handle, a vertex
/// buffer and a glyph atlas print as opaque identifiers on the far side of a
/// driver, and this pass holds no state a reader could act on — what it paints
/// arrives as an argument on every call.
impl std::fmt::Debug for OverlayPass {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OverlayPass")
    }
}

impl OverlayPass {
    /// Builds the renderer against the colour format `config` declares, which is
    /// the format of the target every other pass of the frame writes into.
    ///
    /// Taken from the same `config` the terrain and HUD passes are built from
    /// rather than passed separately: three passes writing one attachment have to
    /// agree about its format, and a second way of saying it is a second answer
    /// waiting to disagree.
    pub(super) fn new(device: &wgpu::Device, config: &TerrainPassConfig) -> Self {
        Self {
            context: egui::Context::default(),
            renderer: egui_wgpu::Renderer::new(
                device,
                color_format(config),
                RendererOptions::PREDICTABLE,
            ),
        }
    }

    /// Records the pass painting `readout`'s lines onto `target`.
    ///
    /// Called only for a frame whose overlay is being shown; a hidden overlay is
    /// an absent readout at the call site rather than an empty one here, so this
    /// never paints nothing.
    pub(super) fn record(&mut self, target: RecordTarget<'_>, readout: &OverlayReadout) {
        if !paintable(target.size) {
            return;
        }
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [target.size.width, target.size.height],
            pixels_per_point: PHYSICAL_PIXELS,
        };
        let mut output = self
            .context
            .run_ui(input_for(&screen), |ui| state_lines(ui, readout));
        let primitives = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);

        self.upload(&target, &output.textures_delta);
        self.paint(target, &primitives, &screen);
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        // Both halves have been applied to the renderer, and `TexturesDelta`
        // asserts on drop that somebody did exactly that. Freeing after the pass
        // rather than before it is epaint's own order: a texture this frame
        // stopped using may still be sampled by the draw that frame recorded.
        output.textures_delta.clear();
    }

    /// Gives the renderer whatever `delta` says the glyph atlas now holds.
    ///
    /// In practice one upload on the first frame the overlay is shown and nothing
    /// afterwards, since the atlas grows only when a glyph nothing has drawn yet
    /// appears — but a frame that skipped this would sample an atlas missing the
    /// character it was about to draw.
    /// One texture may carry several deltas in a frame — a whole replacement and
    /// then a patch — so the batches are flattened rather than nested, and every
    /// delta is applied in the order epaint recorded it.
    fn upload(&mut self, target: &RecordTarget<'_>, delta: &egui::TexturesDelta) {
        let images = delta
            .set
            .iter()
            .flat_map(|(id, batch)| batch.iter().map(move |image| (*id, image)));
        for (id, image) in images {
            self.renderer
                .update_texture(target.device, target.queue, id, image);
        }
    }

    /// Records the draw of `primitives` into `target`'s colour attachment.
    fn paint(
        &mut self,
        target: RecordTarget<'_>,
        primitives: &[egui::ClippedPrimitive],
        screen: &egui_wgpu::ScreenDescriptor,
    ) {
        // egui writes its vertices through the queue and returns command buffers
        // only for paint callbacks, which this overlay has none of. Submitting
        // them here rather than handing them back leaves the caller's encoder the
        // only thing it has to submit, and submission order puts these first
        // regardless.
        let prepared = self.renderer.update_buffers(
            target.device,
            target.queue,
            target.encoder,
            primitives,
            screen,
        );
        target.queue.submit(prepared);

        let attachments = [Some(loaded(target.color))];
        let pass = target
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mycraft debug overlay"),
                color_attachments: &attachments,
                ..wgpu::RenderPassDescriptor::default()
            });
        // `egui_wgpu::Renderer::render` takes a pass that outlives its borrows.
        // The pass is dropped at the end of this function either way, so what is
        // forgotten is a lifetime the scope already enforces.
        let mut pass = pass.forget_lifetime();
        self.renderer.render(&mut pass, primitives, screen);
    }
}

/// Lays out one line per reading the overlay published, in the order it published
/// them.
///
/// Monospaced, which is the one presentation decision made here rather than
/// inherited: the lines are columns of numbers whose labels are padded to align,
/// and a proportional font throws that alignment away. What the lines *say* is
/// decided in the pure module that formats them — this only puts them on a
/// screen.
fn state_lines(ui: &mut egui::Ui, readout: &OverlayReadout) {
    for line in readout_lines(readout) {
        ui.monospace(line);
    }
}

/// The input one frame of a non-interactive overlay runs on: how big the screen
/// is, and no events.
///
/// A fresh value per frame rather than one carried across them. There is no
/// state to accumulate — nothing is hovered, focused, dragged or typed into —
/// and an input that remembered a previous frame would be state this overlay
/// cannot be asked about.
fn input_for(screen: &egui_wgpu::ScreenDescriptor) -> egui::RawInput {
    let [width, height] = screen.size_in_pixels;
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(READOUT_INSET, READOUT_INSET),
            #[expect(
                clippy::cast_precision_loss,
                reason = "a target extent large enough to lose precision as f32 is far past what \
                          any adapter will allocate, and the readout's own position is what this \
                          feeds"
            )]
            egui::vec2(width as f32, height as f32),
        )),
        ..egui::RawInput::default()
    }
}

/// The colour attachment, keeping whatever the terrain and HUD passes left in it.
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
