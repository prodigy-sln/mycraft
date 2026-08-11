//! Reading block definitions off a content root.
//!
//! This module is the only place in the workspace that knows a definition is
//! spelled in TOML and lives in a file. Everything above it sees the
//! `DefinitionSource` port and nothing else, which is what lets MVP 2 swap this
//! reader for a scripting host without the registry noticing.

mod raw;
mod toml_source;

pub use toml_source::TomlFileDefinitionSource;
