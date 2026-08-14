//! The HUD elements a client draws, and the single door they come in through.
//!
//! Structurally parallel to [`BlockRegistry`](crate::block::BlockRegistry), with
//! one deliberate difference: a source that declares nothing is a **valid, empty
//! answer** here, where for blocks it is a refusal. A world with no blocks cannot
//! be rendered or played; a game with no HUD is merely bare, and a mod that ships
//! none is ordinary.

use thiserror::Error;

use super::element::{HudElement, HudOrigin};
use super::source::{HudElementSource, HudElementSourceError};
use crate::id::HudElementName;

/// The HUD elements a running client draws, in the order the source declared
/// them.
///
/// A layout starts from a [`HudElementSource`] and there is no way to hand it an
/// element from Rust: [`load`](HudLayout::load) is the only door in, exactly as
/// it is for blocks. An engine that shipped a HUD element of its own would not
/// compile, rather than being caught by a test someone has to remember to keep.
#[derive(Debug)]
pub struct HudLayout {
    elements: Vec<HudElement>,
}

impl HudLayout {
    /// Registers every element the source yields, or none of them.
    ///
    /// Atomicity is structural rather than careful: everything fallible happens
    /// while a staging buffer is filled, and the layout is constructed from that
    /// buffer only once it is complete, so there is no point at which a
    /// half-loaded layout could exist to be handed back.
    ///
    /// A source that yields nothing loads as an empty layout. That is the one
    /// place this diverges from block registration, and it is FR-1.5's whole
    /// point.
    ///
    /// # Errors
    ///
    /// Returns [`HudLoadError::Source`] if the source fails while yielding, and
    /// [`HudLoadError::AlreadyDeclared`] if two declarations claim one name.
    pub fn load(source: &dyn HudElementSource) -> Result<Self, HudLoadError> {
        let mut staged: Vec<HudElement> = Vec::new();
        for yielded in source.elements() {
            let element = yielded?;
            reject_if_already_declared(&element, &staged)?;
            staged.push(element);
        }
        Ok(Self { elements: staged })
    }

    /// The registered elements, in the order the source declared them.
    pub fn elements(&self) -> &[HudElement] {
        &self.elements
    }
}

/// Refuses a name the batch under load has already claimed, naming both places
/// that declared it.
///
/// Both origins, because "this name is taken" sends a content author looking
/// through every file they have; "these two files both declare it" sends them to
/// the two that matter.
fn reject_if_already_declared(
    element: &HudElement,
    staged: &[HudElement],
) -> Result<(), HudLoadError> {
    match staged.iter().find(|earlier| earlier.name == element.name) {
        Some(earlier) => Err(HudLoadError::AlreadyDeclared {
            name: element.name.clone(),
            first: earlier.origin.clone(),
            second: element.origin.clone(),
        }),
        None => Ok(()),
    }
}

/// Why a layout refused to load.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HudLoadError {
    #[error(
        "`{name}` is already declared: stated by {first} and again by {second}",
        name = name.as_str(),
        first = first.as_str(),
        second = second.as_str()
    )]
    AlreadyDeclared {
        name: HudElementName,
        first: HudOrigin,
        second: HudOrigin,
    },
    #[error(transparent)]
    Source(#[from] HudElementSourceError),
}
