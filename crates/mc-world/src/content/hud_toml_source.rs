//! A directory of TOML files, read as HUD element declarations.
//!
//! One implementation of the element-source port, and a dated one: MVP 2
//! replaces it with a scripting host through the same trait. Nothing above this
//! file learns that a declaration was ever a file.
//!
//! The declaration is handed to the model as a **key-and-value list**, not
//! deserialized into a struct. `mc-core`'s resolved dependency graph is asserted
//! to reach no serialization format, so the model owns its own untyped value and
//! its own unknown-field rule; a struct with `deny_unknown_fields` here would
//! move that decision back out of the model and into whichever format happened
//! to be reading. It would also have to be written again for Luau tables at
//! MVP 2, where a declaration is not a `toml::Value` either.

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use mc_core::hud::source::{HudElementSource, HudElementSourceError, HudElementStream, HudFault};
use mc_core::hud::{DeclaredValue, HudElement, HudOrigin, RawHudElement};

/// The subdirectory of a content root that HUD declarations live in.
pub(super) const HUD_DIRECTORY: &str = "hud";

/// The extension a HUD declaration is written with.
pub(super) const DECLARATION_EXTENSION: &str = "toml";

/// HUD elements read from the declaration files under a content root.
///
/// Construction is infallible and touches no disk: a root that declares no HUD
/// is not a programming error, it is something a mod author did, and what it
/// means is decided when the directory is read.
#[derive(Debug)]
pub struct TomlFileHudSource {
    root: PathBuf,
}

impl TomlFileHudSource {
    /// Elements declared under `root`, which is a content root and not the
    /// `hud/` directory inside it.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Every declaration file directly under `hud/`, in file-name order, or
    /// none at all where the root declares no HUD.
    ///
    /// The order is part of the contract rather than an implementation detail:
    /// a name declared twice is refused naming the first file and the second,
    /// and which is which is only well defined if the root is read in a fixed
    /// order. Two loads of one root agree for the same reason.
    ///
    /// The search is one directory deep. A declaration below `hud/` is not
    /// found, so where a file sits is not something a content author has to
    /// reason about.
    fn declaration_files(&self) -> Result<Vec<PathBuf>, HudElementSourceError> {
        let declarations = self.root.join(HUD_DIRECTORY);
        let listing = match fs::read_dir(&declarations) {
            Ok(listing) => listing,
            // A root that declares no HUD is a valid, empty answer — unlike
            // `blocks/`, where declaring nothing is an error. Anything else that
            // cannot be listed is a fault, which is what keeps a *regular file*
            // named `hud` from degrading into "zero elements" and hiding a
            // mis-shaped content root.
            Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(cause) => return Err(unreadable(&declarations, &cause)),
        };
        let mut files = listing
            .map(|entry| entry.map(|found| found.path()))
            .collect::<Result<Vec<PathBuf>, _>>()
            .map_err(|cause| unreadable(&declarations, &cause))?;
        files.retain(|path| path.extension() == Some(OsStr::new(DECLARATION_EXTENSION)));
        files.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
        Ok(files)
    }
}

impl HudElementSource for TomlFileHudSource {
    fn origin(&self) -> HudOrigin {
        origin_of(&self.root)
    }

    fn elements(&self) -> HudElementStream<'_> {
        match self.declaration_files() {
            Ok(files) => Box::new(files.into_iter().map(|file| element_in(&file))),
            Err(unreadable) => Box::new(std::iter::once(Err(unreadable))),
        }
    }
}

/// A path as a label to quote back to whoever wrote the content.
fn origin_of(path: &Path) -> HudOrigin {
    HudOrigin::new(path.display().to_string())
}

/// A path that could not be listed or read, and why.
fn unreadable(path: &Path, cause: &impl fmt::Display) -> HudElementSourceError {
    HudElementSourceError::Unreadable {
        origin: origin_of(path),
        cause: cause.to_string(),
    }
}

/// The element one declaration file holds.
fn element_in(file: &Path) -> Result<HudElement, HudElementSourceError> {
    let origin = origin_of(file);
    let text = fs::read_to_string(file).map_err(|cause| unreadable(file, &cause))?;
    let table: toml::Table = toml::from_str(&text).map_err(|cause| {
        HudElementSourceError::Malformed(HudFault {
            origin: origin.clone(),
            // A file that is not TOML has no fields to read a name out of, so
            // the path is all there is to say — and it is what a content author
            // needs, because the whole file is what is wrong.
            element: None,
            field: None,
            cause: cause.to_string(),
        })
    })?;
    let fields = table
        .into_iter()
        .map(|(key, value)| (key, declared(&value)))
        .collect();
    RawHudElement::new(fields)
        .into_element(&origin)
        .map_err(HudElementSourceError::Malformed)
}

/// A TOML value as the model's own untyped value.
///
/// Total by construction: a value the model has no use for keeps the name TOML
/// calls it by, so a refusal can report what it found rather than that something
/// was dropped.
fn declared(value: &toml::Value) -> DeclaredValue {
    match value {
        toml::Value::String(spelled) => DeclaredValue::Text(spelled.clone()),
        toml::Value::Integer(number) => DeclaredValue::Integer(*number),
        toml::Value::Float(number) => DeclaredValue::Decimal(*number),
        toml::Value::Boolean(stated) => DeclaredValue::Boolean(*stated),
        toml::Value::Array(stated) => DeclaredValue::List(stated.iter().map(declared).collect()),
        other => DeclaredValue::Opaque(format!("a {}", other.type_str())),
    }
}
