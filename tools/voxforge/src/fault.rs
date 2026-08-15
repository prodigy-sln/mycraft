//! What a refusal says.
//!
//! One struct for every refusal this tool makes, shaped after
//! `mc_core::block::source::DefinitionFault` field for field — including why
//! `part` is a plain `String`: a malformed declaration may carry a name that
//! does not parse, and quoting it back verbatim is the point.
//!
//! The consumer of these messages is an AI agent repairing its own input from
//! the message alone, so every refusal names where it happened, which element it
//! is about, and what specifically was wrong.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

/// The file, or the directory, a refusal is about.
///
/// A directory is a legitimate origin: a duplicate material name spans two
/// files, so the failure belongs to the source rather than to either file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Origin(PathBuf);

impl Origin {
    /// The origin at `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// The path this origin points at.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.display())
    }
}

/// Which layer a refusal is about.
///
/// Two indices, because they answer different questions and either can be the
/// only one available. `declaration` is where the `[[layers]]` table sits in the
/// document, counted from zero, and always exists. `plane` is the coordinate the
/// layer declared, which a layer whose index is missing or unreadable has not
/// got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerRef {
    /// Position of the `[[layers]]` table in the document, counted from zero.
    pub declaration: usize,
    /// The plane the layer declares.
    ///
    /// `i32` rather than TOML's `i64`: a plane is bounded by the 64-per-axis
    /// rule, and a declared index too large to hold here is refused for that
    /// reason anyway. The value exactly as written always reaches the reader
    /// through [`Fault::cause`], which is the field that has to carry it.
    pub plane: Option<i32>,
}

impl LayerRef {
    /// A layer identified only by where it was declared.
    pub fn declared(declaration: usize) -> Self {
        Self {
            declaration,
            plane: None,
        }
    }

    /// A layer that declared the plane `plane`.
    pub fn at_plane(declaration: usize, plane: i32) -> Self {
        Self {
            declaration,
            plane: Some(plane),
        }
    }
}

impl fmt::Display for LayerRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "layer {}", self.declaration)?;
        match self.plane {
            Some(plane) => write!(formatter, " (plane {plane})"),
            None => Ok(()),
        }
    }
}

/// One thing this tool refused to do, in the terms whoever wrote the file needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// The file, or the directory, the refusal is about.
    pub origin: Origin,
    /// The part, exactly as it was spelled, when the refusal is about one.
    pub part: Option<String>,
    /// The layer, when the refusal is about one.
    pub layer: Option<LayerRef>,
    /// The field at fault, when one field is at fault.
    pub field: Option<String>,
    /// What was wrong, in words, naming every value the reader needs.
    pub cause: String,
}

impl Fault {
    /// A refusal about `origin` as a whole.
    pub fn about(origin: Origin, cause: impl Into<String>) -> Self {
        Self {
            origin,
            part: None,
            layer: None,
            field: None,
            cause: cause.into(),
        }
    }

    /// The same refusal, attributed to `field`.
    #[must_use]
    pub fn in_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// The same refusal, attributed to `part`.
    #[must_use]
    pub fn in_part(mut self, part: impl Into<String>) -> Self {
        self.part = Some(part.into());
        self
    }

    /// The same refusal, attributed to `layer`.
    #[must_use]
    pub fn in_layer(mut self, layer: LayerRef) -> Self {
        self.layer = Some(layer);
        self
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.origin)?;
        if let Some(part) = &self.part {
            write!(formatter, ", part `{part}`")?;
        }
        if let Some(layer) = self.layer {
            write!(formatter, ", {layer}")?;
        }
        if let Some(field) = &self.field {
            write!(formatter, ", field `{field}`")?;
        }
        write!(formatter, ": {}", self.cause)
    }
}

impl Error for Fault {}
