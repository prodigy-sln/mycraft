//! Where HUD element declarations come from, and how one that could not be
//! accepted is described.
//!
//! A source hands over elements one at a time and may fail part-way through,
//! which is what a reader of many files does and what a scripting host will do.
//! Nothing here names a file, a path, or a serialization format — swapping the
//! implementation at MVP 2 is meant to be a new type, not a redesign.
//!
//! [`HudFault`] is structurally parallel to
//! [`DefinitionFault`](crate::block::source::DefinitionFault) and deliberately
//! not a reuse of it: that type's field is named `block` and its `Display`
//! writes ", block `…`", which is the wrong vocabulary in the one message a
//! content author reads. The duplication is the accepted cost.

use std::fmt;

use thiserror::Error;

use super::element::{HudElement, HudOrigin};
use super::raw::RawHudElement;

/// A place HUD element declarations come from.
///
/// Its own port rather than a reuse of
/// [`DefinitionSource`](crate::block::source::DefinitionSource), which yields
/// block definitions: a parallel port is the only shape that keeps the two
/// loaders swappable independently when MVP 2 replaces the file reader with a
/// scripting host.
pub trait HudElementSource {
    /// Labels the source as a whole, for failures that are about the source
    /// rather than about any one declaration it yielded.
    fn origin(&self) -> HudOrigin;

    /// The elements this source declares, in declaration order.
    fn elements(&self) -> HudElementStream<'_>;
}

/// A fallible stream of elements.
///
/// A stream rather than a `Vec`, because a failure that arrives after some
/// elements have already been handed over is the case worth getting right, and
/// it is not expressible if the whole batch has to succeed to be produced.
pub type HudElementStream<'a> =
    Box<dyn Iterator<Item = Result<HudElement, HudElementSourceError>> + 'a>;

/// Why a source could not yield an element.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HudElementSourceError {
    #[error("{origin} could not be read: {cause}", origin = origin.as_str())]
    Unreadable { origin: HudOrigin, cause: String },
    #[error("{0}")]
    Malformed(HudFault),
}

/// Declarations held directly, in the order they were given, each attributed to
/// the origin beside it.
///
/// This is the port's second implementation, and it is what makes the seam real
/// rather than asserted.
///
/// It holds **declarations rather than checked elements**, which is where it
/// parts company with
/// [`InMemoryDefinitionSource`](crate::block::source::InMemoryDefinitionSource).
/// A source of already-accepted elements cannot express a declaration that the
/// model will refuse, so whoever wanted one would have to hand-build the fault
/// they claim the model produces — and would then be satisfied by a model that
/// produced no faults at all. Declarations are also the shape a Luau table
/// arrives in, so this implementation outlives the file reader.
#[derive(Debug, Clone)]
pub struct InMemoryHudSource {
    origin: HudOrigin,
    declarations: Vec<(HudOrigin, RawHudElement)>,
}

impl InMemoryHudSource {
    /// A source labelled `origin` handing over `declarations`, each attributed
    /// to the origin beside it.
    pub fn new(origin: HudOrigin, declarations: Vec<(HudOrigin, RawHudElement)>) -> Self {
        Self {
            origin,
            declarations,
        }
    }
}

impl HudElementSource for InMemoryHudSource {
    fn origin(&self) -> HudOrigin {
        self.origin.clone()
    }

    /// Checks each declaration as it is handed over, so that both
    /// implementations of this port check at the same layer and a fault reads
    /// the same whichever one produced it.
    fn elements(&self) -> HudElementStream<'_> {
        Box::new(self.declarations.iter().map(|(origin, declaration)| {
            declaration
                .clone()
                .into_element(origin)
                .map_err(HudElementSourceError::Malformed)
        }))
    }
}

/// One declaration that could not be accepted, described in the terms whoever
/// wrote it needs: where it was, which element it was, and which field was
/// wrong.
///
/// `element` is a plain `String` and not a
/// [`HudElementName`](crate::id::HudElementName) because a malformed
/// declaration may carry a name that does not parse, and quoting it back
/// verbatim is the whole point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudFault {
    pub origin: HudOrigin,
    pub element: Option<String>,
    pub field: Option<String>,
    pub cause: String,
}

impl fmt::Display for HudFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.origin.as_str())?;
        if let Some(element) = &self.element {
            write!(formatter, ", element `{element}`")?;
        }
        if let Some(field) = &self.field {
            write!(formatter, ", field `{field}`")?;
        }
        write!(formatter, ": {}", self.cause)
    }
}
