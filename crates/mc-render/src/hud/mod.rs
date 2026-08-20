//! Where a HUD layout's elements land on a render target.
//!
//! Pure, and deliberately **outside** `src/gpu/`. ADR-013 exempts that subtree
//! from the coverage denominator because a golden frame is the only thing that
//! can see it; this derivation is arithmetic over plain integers, so putting it
//! there would take it out of the denominator for no gain and leave it measured
//! by nothing.
//!
//! Nothing here draws. The composition produces a **plan** — the rectangles a
//! pass will paint, in the order it will paint them — and the pass that turns
//! one into pixels lives in `gpu/`.

mod held;
mod plan;
mod uniform;

pub use held::{HeldSwatch, INDICATOR_FACE, held_swatch};
pub use plan::{HudFrame, Painted, PaintedRect, compose};
pub use uniform::{HUD_UNIFORM_BYTES, HudUniform, MAX_HUD_RECTS, hud_uniform};
