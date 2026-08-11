//! Where block definitions come from, expressed so that this crate never learns
//! where that is.
//!
//! A source hands over definitions one at a time and may fail part-way through,
//! which is what a reader of many files does and what a scripting host will do.
//! Nothing here names a file, a path, or a serialization format — swapping the
//! implementation is meant to be a new type, not a redesign.

use std::fmt;

use thiserror::Error;

use super::definition::{BlockDefinition, DefinitionOrigin};

/// A place block definitions come from.
pub trait DefinitionSource {
    /// Labels the source as a whole, for failures that are about the source
    /// rather than about any one definition it yielded.
    fn origin(&self) -> DefinitionOrigin;

    /// The definitions this source declares, in declaration order.
    fn definitions(&self) -> DefinitionStream<'_>;
}

/// A fallible stream of definitions.
///
/// A stream rather than a `Vec`, because a failure that arrives after some
/// definitions have already been handed over is the case worth getting right,
/// and it is not expressible if the whole batch has to succeed to be produced.
pub type DefinitionStream<'a> =
    Box<dyn Iterator<Item = Result<BlockDefinition, DefinitionSourceError>> + 'a>;

/// One definition a source could not produce, described in the terms whoever
/// wrote it needs: where it was, which block it was, and which field was wrong.
///
/// `block` is a plain `String` and not a [`BlockName`](crate::id::BlockName)
/// because a malformed definition may carry a name that does not parse, and
/// quoting it back verbatim is the whole point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionFault {
    pub origin: DefinitionOrigin,
    pub block: Option<String>,
    pub field: Option<String>,
    pub cause: String,
}

impl fmt::Display for DefinitionFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.origin.as_str())?;
        if let Some(block) = &self.block {
            write!(formatter, ", block `{block}`")?;
        }
        if let Some(field) = &self.field {
            write!(formatter, ", field `{field}`")?;
        }
        write!(formatter, ": {}", self.cause)
    }
}

/// Why a source could not yield a definition.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DefinitionSourceError {
    #[error("{origin} could not be read: {cause}", origin = origin.as_str())]
    Unreadable {
        origin: DefinitionOrigin,
        cause: String,
    },
    #[error("{0}")]
    Malformed(DefinitionFault),
}

/// Definitions held directly, in the order they were given.
///
/// This is the port's second implementation, and it is what makes the seam real
/// rather than asserted. It is also the only programmatic way to populate a
/// registry at all — there is no public registration call — so it is production
/// code, not a test fixture.
#[derive(Debug, Clone)]
pub struct InMemoryDefinitionSource {
    origin: DefinitionOrigin,
    definitions: Vec<Result<BlockDefinition, DefinitionSourceError>>,
}

impl InMemoryDefinitionSource {
    /// A source labelled `origin` yielding `definitions`, failures included.
    pub fn new(
        origin: DefinitionOrigin,
        definitions: Vec<Result<BlockDefinition, DefinitionSourceError>>,
    ) -> Self {
        Self {
            origin,
            definitions,
        }
    }
}

impl DefinitionSource for InMemoryDefinitionSource {
    fn origin(&self) -> DefinitionOrigin {
        self.origin.clone()
    }

    fn definitions(&self) -> DefinitionStream<'_> {
        Box::new(self.definitions.iter().cloned())
    }
}
