//! MyCraft client: a window, and the scripted replay drawn in it.
//!
//! The composition root, and the only crate that resolves both halves of the
//! snapshot seam. `mc-sim` publishes a world and a camera path, `mc-render` turns
//! quads into packed vertices and draws them, and neither may name the other — so
//! everything that needs both lives here.
//!
//! # Why this is a library and not only a binary
//!
//! [`startup::prepare_scene`] is the pipeline that turns shipped content into the
//! bytes the GPU draws, and **the goldens are shot through it.** A binary crate
//! cannot be imported by its own integration tests, so before this file existed
//! the tests carried their own copy of that pipeline: the goldens verified one
//! sequence and the window ran another, with nothing asserting the two agreed.
//! The goldens are this spec's only automated evidence that the renderer draws
//! the right picture, and evidence gathered from a path the product does not run
//! does not transfer to the thing a player launches. One path, imported by both,
//! is what makes it transfer.
//!
//! This crate holds no policy. Every decision the client makes — which surface
//! format to configure, whether a size is drawable, whether a failed acquire
//! recovers or is fatal, how a run ends — is a pure function in `mc-render` with
//! a test that never opened a window, and that is what makes ADR-013's wholesale
//! exclusion of this crate from the coverage denominator honest rather than
//! convenient.

/// The frame path: acquire, record, present, advance one tick.
pub mod app;
/// Which key asks for what. Private, and re-exported through [`session`]:
/// nothing outside this crate may ask the table what a key means.
mod bindings;
/// The window and the event loop, and the only module that may name `winit`.
pub mod events;
/// Adapter facts, the startup verdict, and opening a device.
pub mod gpu_startup;
/// Turning an edit into a scene, off the tick thread and off the frame thread.
pub mod remesh;
/// The client's input dispatch, drivable with no window and no adapter.
pub mod session;
/// Turning shipped content into a scene the renderer can draw.
pub mod startup;
/// Configuring a surface for the window it came from, and why one cannot be.
pub mod surface_setup;
