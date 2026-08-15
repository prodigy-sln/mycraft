//! VoxForge — an AI-authorable voxel model format, a CPU preview renderer and a
//! command-line tool over both.
//!
//! The feedback loop is the point: an agent describes a model as layered grid
//! art, renders it from every canonical angle, and sees whether it made the
//! thing it was asked for.
//!
//! Everything except three lines of `main.rs` lives here. A CLI whose behaviour
//! lived in its binary would earn the coverage exclusion the binary crates have,
//! and with it the blindness that exclusion brings.

pub mod cli;
pub mod fault;
pub mod format;
pub mod inspect;
pub mod material;
pub mod name;
pub mod render;
pub mod texture;
pub mod volume;
