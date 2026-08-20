//! Shared primitives: namespaced ids, math types, registry contracts, event definitions. No I/O, depends on nothing else in the workspace.
//!
//! The block registry lives here rather than beside the voxel world because the
//! scripting host must populate the same registry, and dependencies flow inward:
//! a registry owned by the world crate would drag chunk storage and worldgen into
//! the scripting host's graph.

pub mod art;
pub mod block;
pub mod content;
pub mod hash;
pub mod hud;
pub mod id;
