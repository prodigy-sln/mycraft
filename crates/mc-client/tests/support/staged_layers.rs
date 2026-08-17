//! Reaching a layer assignment that is deliberately **not** the lexicographic one,
//! the only way a session can reach one.
//!
//! # Why a pair list is no longer the way, and why that is an improvement
//!
//! `ResolvedContent::stating` used to take arbitrary `(TextureKey, u16)` pairs, so
//! a fixture could state any assignment at all — including ones no session could
//! ever hold, such as a sparse one whose `spent` would then silently lie. It now
//! takes a `LayerAssignment`, which is constructible by `none` and `appending`
//! alone.
//!
//! **Staged appends are how a non-lexicographic assignment arises in production**,
//! so building one that way makes a falsifier assert something a running session
//! can actually be in. A key already live keeps the layer it holds and a new one
//! takes the next unspent index, so appending `{example:zinc}` and then
//! `{example:amber, example:zinc}` leaves zinc on 0 and amber on 1 — the reverse of
//! their sorted order, which is exactly what a reader that re-derived its layers
//! from a sort would get wrong.
//!
//! # The stages are derived from the table, never written beside it
//!
//! A scenario states one table of key-to-layer pairs and both the fixture and the
//! expectation come out of it. Stage `n` is every key whose layer is at most `n`,
//! so the key the table puts on layer `n` is the one that takes it. Nothing has to
//! be kept in step, and a table somebody edits moves the assignment with it.
//!
//! # What this cannot reach, stated because it is the one thing that would be a
//! finding
//!
//! A layer set that is **not** dense from zero. Staging hands out `0, 1, 2, …` in
//! order, so a table naming layers `0` and `2` with nothing on `1` is unreachable
//! and [`assigned`] refuses it by name rather than quietly producing something
//! else. A falsifier that genuinely needs such a set is a finding about the
//! constructor rule and not something to work around here.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::error::Error;

use mc_core::content::LayerAssignment;
use mc_core::id::TextureKey;

/// The assignment a session reaches by handing out `table`'s layers in ascending
/// order, one append per layer.
///
/// # Errors
///
/// Returns an error if a key does not parse, if `table` names a layer twice or
/// leaves a gap — see this module's header for why a gap is a finding rather than
/// a fixture to fix — or if the appends do not fit a session's budget.
pub fn assigned(table: &[(&str, u16)]) -> Result<LayerAssignment, Box<dyn Error>> {
    require_dense(table)?;
    let mut assignment = LayerAssignment::none();
    for stage in 0..layer_count(table) {
        assignment = assignment.appending(&up_to(table, stage)?)?;
    }
    Ok(assignment)
}

/// Every key `table` puts on a layer at or below `stage`.
///
/// # Errors
///
/// Returns an error if a key is not a namespaced id.
fn up_to(table: &[(&str, u16)], stage: u16) -> Result<BTreeSet<TextureKey>, Box<dyn Error>> {
    let mut keys = BTreeSet::new();
    for (key, _) in table.iter().filter(|(_, layer)| *layer <= stage) {
        keys.insert(TextureKey::parse(key)?);
    }
    Ok(keys)
}

/// How many layers `table` hands out.
fn layer_count(table: &[(&str, u16)]) -> u16 {
    u16::try_from(table.len()).unwrap_or(u16::MAX)
}

/// Refuses unless `table` names each of the layers `0..table.len()` exactly once.
///
/// # Errors
///
/// Returns an error naming what it found, because a table with a gap or a repeat
/// is one staged appends cannot reproduce and a fixture that produced something
/// near it would be grading a different assignment from the one a scenario states.
fn require_dense(table: &[(&str, u16)]) -> Result<(), Box<dyn Error>> {
    let stated: BTreeSet<u16> = table.iter().map(|(_, layer)| *layer).collect();
    let dense: BTreeSet<u16> = (0..layer_count(table)).collect();
    if stated == dense {
        return Ok(());
    }
    Err(format!(
        "staged appends hand out layers 0, 1, 2 and so on in order, so a table has to name each of \
         {dense:?} exactly once, and this one names {stated:?}. A falsifier that genuinely needs a \
         layer set with a gap in it cannot be built through the assignment's own constructors at \
         all, which is a finding about that rule rather than a fixture to work around"
    )
    .into())
}
