//! The scripted replay: a fixed world, a fixed orbit, and the render input they
//! produce.
//!
//! Everything here is a pure function of a seed or of a tick index. That is not
//! a stylistic preference: a golden frame is a claim about what a camera saw,
//! and the claim is only checkable if the world, the pose and the meshing order
//! are all reproducible from the same two numbers on any machine.

pub mod camera;
pub mod contract;
pub mod height;
pub mod prepare;
pub mod world;

pub use camera::{CameraPose, TickError, TickIndex, pose};
pub use prepare::{PrepareError, SectionQuads, mesh_all};
pub use world::{ReplayWorld, WorldGenError};
