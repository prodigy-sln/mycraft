//! Reading a content root: the blocks it declares and the HUD it declares.
//!
//! This module is the only place in the workspace that knows what a declaration
//! is spelled in and that it lives in a file. Everything above it sees the
//! `DefinitionSource` and `HudSource` ports and nothing else, which is what let
//! the block reader be swapped for one that evaluates script without the
//! registry noticing — the swap this module has now been through.
//!
//! **Blocks are Luau and the HUD is TOML**, and the asymmetry is deliberate
//! rather than unfinished: a block declaration is code that runs, so a block may
//! compute what it declares, while the HUD's format stays declarative until a
//! spec decides otherwise. `toml` is still a dependency of this crate for that
//! second reader alone.

mod hud_toml_source;
mod luau_declaration;
mod luau_source;
/// Whether a content root has changed, behind a port, with the one file that
/// names a filesystem-watching vendor behind that.
pub mod watch;

pub use hud_toml_source::TomlFileHudSource;
pub use luau_source::{LuauFileDefinitionSource, Printed};
