//! Whether a part's declared states and its layers agree.
//!
//! A flicker is different voxels rather than a transform, so a state is only
//! real if art belongs to it. All four refusals here are about the same thing
//! from different sides: art that would never be drawn, or a state that would
//! draw nothing.
//!
//! Checked after the part tree, because a repeated part name sends every layer
//! naming it to the first of the two and would earn the second a true but
//! misleading complaint about having no art.

use crate::fault::{Fault, Origin};
use crate::format::{Part, StateName};

/// Checks that every part's layers and declared states account for each other.
///
/// # Errors
///
/// Returns a [`Fault`] naming the part and the states at fault when a layer
/// belongs to a state nobody declared, a layer under a stateful part names no
/// state, a declared state has no art, or a part has no art at all.
pub fn check(parts: &[Part], origin: &Origin) -> Result<(), Fault> {
    for part in parts {
        check_part(part, origin)?;
    }
    Ok(())
}

/// The four ways one part's states and layers can disagree.
fn check_part(part: &Part, origin: &Origin) -> Result<(), Fault> {
    let name = part.name.as_str();
    if part.layers.is_empty() {
        return Err(refusal(
            origin,
            name,
            format!("the part `{name}` declares no layer at all, so it contributes no voxel"),
        ));
    }
    if part.states.is_empty() {
        return Ok(());
    }
    check_layers_name_a_declared_state(part, origin)?;
    check_every_declared_state_has_art(part, origin)
}

/// Refuses a layer naming a state the part never declared, or naming none.
fn check_layers_name_a_declared_state(part: &Part, origin: &Origin) -> Result<(), Fault> {
    let name = part.name.as_str();
    for key in part.layers.keys() {
        let Some(state) = key.state.as_ref() else {
            return Err(refusal(
                origin,
                name,
                format!(
                    "a layer of the part `{name}` names no state, but the part declares {declared} — a layer belonging to all of them or to none is not something this format can mean",
                    declared = quoted(&part.states)
                ),
            ));
        };
        if !part.states.contains(state) {
            return Err(refusal(
                origin,
                name,
                format!(
                    "a layer of the part `{name}` belongs to the state `{spelled}`, which the part does not declare — it declares {declared}",
                    spelled = state.as_str(),
                    declared = quoted(&part.states)
                ),
            ));
        }
    }
    Ok(())
}

/// Refuses a declared state no layer belongs to.
fn check_every_declared_state_has_art(part: &Part, origin: &Origin) -> Result<(), Fault> {
    let name = part.name.as_str();
    for state in &part.states {
        let arted = part
            .layers
            .keys()
            .any(|key| key.state.as_ref() == Some(state));
        if !arted {
            return Err(refusal(
                origin,
                name,
                format!(
                    "the part `{name}` declares the state `{spelled}`, but no layer belongs to it, so that state would assemble to nothing",
                    spelled = state.as_str()
                ),
            ));
        }
    }
    Ok(())
}

/// A refusal about one part.
fn refusal(origin: &Origin, part: &str, cause: String) -> Fault {
    Fault::about(origin.clone(), cause).in_part(part)
}

/// Several state names, quoted and joined the way a sentence reads them.
pub(crate) fn quoted(states: &[StateName]) -> String {
    states
        .iter()
        .map(|state| format!("`{}`", state.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}
