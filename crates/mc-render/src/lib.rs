//! wgpu renderer: chunk meshing, GPU-driven terrain draw, texture arrays, UI compositing.
//!
//! The crate is split in two by the `gpu` Cargo feature. Everything at this
//! level is a pure function over plain values — quads become vertices and
//! indices, vertices are bit-packed, sections are assembled into a scene — and
//! is unit-tested with no device anywhere. [`gpu`] holds the half that needs
//! one, and is the only place `wgpu::` may be named.

pub mod aabb;
pub mod camera;
pub mod capture;
pub mod color;
pub mod geometry;
pub mod pass;
pub mod snapshot;
pub mod surface;
pub mod texture;
pub mod window;

#[cfg(feature = "gpu")]
pub mod gpu;
