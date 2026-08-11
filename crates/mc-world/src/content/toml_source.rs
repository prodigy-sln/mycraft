//! A directory of TOML files, read as block definitions.
//!
//! One implementation of the definition-source port, and a dated one: MVP 2
//! replaces it with a scripting host through the same trait. Nothing above this
//! file learns that a definition was ever a file.

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use mc_core::block::source::{
    DefinitionFault, DefinitionSource, DefinitionSourceError, DefinitionStream,
};
use mc_core::block::{BlockDefinition, DefinitionOrigin};

use super::raw::{NAME_FIELD, RawBlockDefinition};

/// The subdirectory of a content root that block declarations live in.
const BLOCKS_DIRECTORY: &str = "blocks";

/// The extension a block declaration is written with.
const DECLARATION_EXTENSION: &str = "toml";

/// Block definitions read from the declaration files under a content root.
///
/// Construction is infallible and touches no disk: a root that does not exist is
/// not a programming error, it is something a mod author did, and it is reported
/// the same way every other content problem is — as a failure of the stream,
/// naming the path.
#[derive(Debug)]
pub struct TomlFileDefinitionSource {
    root: PathBuf,
}

impl TomlFileDefinitionSource {
    /// Definitions declared under `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Every declaration file under the root, in file-name order.
    ///
    /// The order is part of the contract rather than an implementation detail: a
    /// name declared twice is refused naming the first file and the second, and
    /// which is which is only well defined if the root is read in a fixed order.
    fn declaration_files(&self) -> Result<Vec<PathBuf>, DefinitionSourceError> {
        let declarations = self.root.join(BLOCKS_DIRECTORY);
        let listing =
            fs::read_dir(&declarations).map_err(|cause| unreadable(&declarations, &cause))?;
        let mut files = listing
            .map(|entry| entry.map(|found| found.path()))
            .collect::<Result<Vec<PathBuf>, _>>()
            .map_err(|cause| unreadable(&declarations, &cause))?;
        files.retain(|path| path.extension() == Some(OsStr::new(DECLARATION_EXTENSION)));
        files.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
        Ok(files)
    }
}

impl DefinitionSource for TomlFileDefinitionSource {
    fn origin(&self) -> DefinitionOrigin {
        origin_of(&self.root)
    }

    fn definitions(&self) -> DefinitionStream<'_> {
        match self.declaration_files() {
            Ok(files) => Box::new(files.into_iter().map(|file| definition_in(&file))),
            Err(unreadable) => Box::new(std::iter::once(Err(unreadable))),
        }
    }
}

/// A path as a label to quote back to whoever wrote the content.
fn origin_of(path: &Path) -> DefinitionOrigin {
    DefinitionOrigin::new(path.display().to_string())
}

/// A path that could not be listed or read, and why.
fn unreadable(path: &Path, cause: &impl fmt::Display) -> DefinitionSourceError {
    DefinitionSourceError::Unreadable {
        origin: origin_of(path),
        cause: cause.to_string(),
    }
}

/// The definition one declaration file holds.
fn definition_in(file: &Path) -> Result<BlockDefinition, DefinitionSourceError> {
    let origin = origin_of(file);
    let text = fs::read_to_string(file).map_err(|cause| unreadable(file, &cause))?;
    let declaration: RawBlockDefinition = toml::from_str(&text).map_err(|cause| {
        DefinitionSourceError::Malformed(DefinitionFault {
            origin: origin.clone(),
            block: declared_name_in(&text),
            field: None,
            cause: cause.to_string(),
        })
    })?;
    declaration
        .into_definition(&origin)
        .map_err(DefinitionSourceError::Malformed)
}

/// The name a file declares, read independently of whether the file as a whole
/// is acceptable.
///
/// A declaration carrying a field nobody recognises is refused before it becomes
/// a [`RawBlockDefinition`] at all, and a refusal that cannot say which block it
/// is about leaves a mod author reading the file by hand. `None` when the file is
/// not a table, which is the case where there is genuinely no name to report.
fn declared_name_in(text: &str) -> Option<String> {
    toml::from_str::<toml::Table>(text)
        .ok()?
        .get(NAME_FIELD)?
        .as_str()
        .map(str::to_owned)
}
