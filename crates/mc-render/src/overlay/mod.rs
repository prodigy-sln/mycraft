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
//! **The wall clock is confined to [`clock`] and injected from there.** The
//! client reads no clock on the tick, the snapshot or the capture path, which is
//! what makes a replay identical at 30 and 300 frames a second — and the one
//! reading this overlay does need arrives through a port, so a test can drive
//! ten frames twenty milliseconds apart without waiting two hundred
//! milliseconds or trusting that it did.

pub mod clock;
mod state;

pub use state::{DebugOverlay, OverlayReadout, readout_lines};
