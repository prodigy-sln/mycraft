//! Voxel world: palette-compressed chunk storage, lighting, worldgen, and redb-backed persistence.
//!
//! This crate is also where the engine is allowed to touch a disk. The block
//! registry contract lives in `mc-core`, which performs no I/O; the reader that
//! turns a content root into definitions lives here, behind the same port a
//! scripting host will implement later.

pub mod column;
pub mod content;
pub mod mesh;
pub mod section;
pub mod world;
