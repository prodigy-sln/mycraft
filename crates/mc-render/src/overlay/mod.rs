//! The debug overlay's own state: what it shows, and the four readings it
//! publishes.
//!
//! Engine-owned tooling, deliberately unlike the HUD one directory over. A HUD
//! element is a declaration in a content root and the engine knows only how to
//! fill a rectangle; this is an instrument for whoever is diagnosing the engine
//! *while* a content root misbehaves, so no declaration reaches it and no
//! declaration can hide it.
//!
//! Pure, and deliberately **outside** `src/gpu/`. ADR-013 exempts that subtree
//! from the coverage denominator because a golden frame is the only thing that
//! can see it; everything here is arithmetic and formatting over plain values,
//! so putting it there would take it out of the denominator for no gain. What
//! paints these lines needs a device and lives in `gpu/`.
//!
//! **No clock reaches this module.** A frame time arrives already measured, from
//! the client's frame path, which reads [`mc_render::time::clock`](crate::time::clock)
//! once a frame and spends the same interval into simulation ticks. So the rate
//! shown here and the time the world spent are one reading rather than two that
//! could disagree — and a test drives ten frames twenty milliseconds apart by
//! handing over twenty milliseconds, without waiting two hundred and without
//! trusting that it did.

mod state;

pub use state::{DebugOverlay, OverlayReadout, readout_lines};
