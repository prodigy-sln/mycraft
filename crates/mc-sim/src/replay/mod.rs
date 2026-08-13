//! The scripted replay: a fixed world, a declared intent script, and the render
//! input they produce.
//!
//! The world is a pure function of a seed and the script a pure function of a
//! tick index. That is not a stylistic preference: a golden frame is a claim
//! about what a camera saw, and the claim is only checkable if the world, the
//! inputs and the meshing order are all reproducible from the same two numbers
//! on any machine.
//!
//! **The camera is no longer among them, and that is the point.** A camera pose
//! used to be a function of the tick index here — an orbit, which could be asked
//! for tick 59 directly. What a frame is shot through now is the camera the
//! *simulation publishes*, which is reached by advancing the script's intents
//! from the spawn and cannot be asked for out of order. Reproducibility comes
//! from the script and the world being declared, not from the path being a
//! formula.

pub mod contract;
pub mod height;
pub mod patch;
pub mod prepare;
pub mod script;
pub mod solid;
pub mod spawn;
pub mod world;

pub use crate::camera::CameraPose;
/// `Extent` lives in `mc-world` now, because that is where the world it
/// describes lives and mc-world may not depend on mc-sim. It is re-exported from
/// the path it was declared at so its committed consumers keep compiling.
pub use mc_world::world::Extent;
pub use patch::{SpliceError, remesh, splice};
pub use prepare::{PrepareError, SectionQuads, mesh_all};
pub use script::{SCRIPT_TICKS, TickError, TickIndex, scripted_intent};
pub use solid::{BlockVolume, SolidVoxels};
pub use spawn::{SpawnError, simulation_for, spawn};
pub use world::{ReplayWorld, WorldGenError};
