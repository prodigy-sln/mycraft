//! Whether the declared parts form a tree.
//!
//! Checked before any layer is read, and the order inside is load-bearing twice
//! over. Two parts under one name make every `attach` and every layer naming it
//! ambiguous, so that is refused first — otherwise the layers all land on the
//! first of the two and the second earns a *true* but misleading complaint about
//! having no art. And a self-attachment is refused before cycles are looked for,
//! so a mistyped parent reads as a mistyped parent rather than as a one-part
//! cycle.

use std::collections::{BTreeMap, BTreeSet};

use crate::fault::{Fault, Origin};
use crate::format::Part;

/// Checks that `parts` form a tree rooted at the one part declaring no `attach`.
///
/// # Errors
///
/// Returns a [`Fault`] naming the parts at fault for a repeated name, a part
/// attached to itself, an attachment naming a part nobody declares, no root at
/// all, more than one root, or a cycle.
pub fn check(parts: &[Part], origin: &Origin) -> Result<(), Fault> {
    check_names_are_unique(parts, origin)?;
    check_none_is_its_own_parent(parts, origin)?;
    check_every_parent_exists(parts, origin)?;
    check_exactly_one_root(parts, origin)
}

/// Refuses a name two parts share.
fn check_names_are_unique(parts: &[Part], origin: &Origin) -> Result<(), Fault> {
    let mut seen = BTreeSet::new();
    for part in parts {
        let name = part.name.as_str();
        if !seen.insert(name) {
            return Err(Fault::about(
                origin.clone(),
                format!(
                    "two parts are declared under the name `{name}`, which leaves every `attach` and every layer naming it ambiguous"
                ),
            )
            .in_part(name));
        }
    }
    Ok(())
}

/// Refuses a part that hangs off itself.
fn check_none_is_its_own_parent(parts: &[Part], origin: &Origin) -> Result<(), Fault> {
    for part in parts {
        let name = part.name.as_str();
        let hangs_off_itself = part
            .attach
            .as_ref()
            .is_some_and(|attach| attach.to.as_str() == name);
        if hangs_off_itself {
            return Err(Fault::about(
                origin.clone(),
                format!("the part `{name}` names itself as its own parent"),
            )
            .in_part(name));
        }
    }
    Ok(())
}

/// Refuses an attachment naming a part the document does not declare.
fn check_every_parent_exists(parts: &[Part], origin: &Origin) -> Result<(), Fault> {
    let declared: BTreeSet<&str> = parts.iter().map(|part| part.name.as_str()).collect();
    for part in parts {
        let Some(attach) = part.attach.as_ref() else {
            continue;
        };
        let parent = attach.to.as_str();
        if !declared.contains(parent) {
            let name = part.name.as_str();
            return Err(Fault::about(
                origin.clone(),
                format!(
                    "the part `{name}` attaches to `{parent}`, which the document does not declare"
                ),
            )
            .in_part(name));
        }
    }
    Ok(())
}

/// Refuses a set of parts with no single root, naming what is wrong with it.
fn check_exactly_one_root(parts: &[Part], origin: &Origin) -> Result<(), Fault> {
    let roots: Vec<&str> = parts
        .iter()
        .filter(|part| part.attach.is_none())
        .map(|part| part.name.as_str())
        .collect();
    match roots.as_slice() {
        [_root] => check_all_reachable(parts, origin),
        [] => Err(Fault::about(origin.clone(), no_root(parts))),
        many => Err(Fault::about(
            origin.clone(),
            format!(
                "{names} each declare no `attach`, but exactly one part is the root of a model",
                names = quoted(many)
            ),
        )),
    }
}

/// What to say when every part hangs off another one.
///
/// Every part having a parent that exists *forces* a cycle — there is no
/// arrangement of finitely many parts where each has a declared parent and none
/// of them loops. So "no root" and "there is a cycle" are one condition with two
/// halves, and the refusal states both rather than making the author discover
/// the second after repairing the first.
fn no_root(parts: &[Part]) -> String {
    let looping: Vec<&str> = parts.iter().map(|part| part.name.as_str()).collect();
    format!(
        "no part is the root: every part declares an `attach`, so the attachments close into a cycle among {names}",
        names = quoted(&looping)
    )
}

/// Refuses parts that hang off one another out of the root's reach.
fn check_all_reachable(parts: &[Part], origin: &Origin) -> Result<(), Fault> {
    let children = children_by_parent(parts);
    let mut reached = BTreeSet::new();
    let mut pending: Vec<&str> = parts
        .iter()
        .filter(|part| part.attach.is_none())
        .map(|part| part.name.as_str())
        .collect();
    while let Some(name) = pending.pop() {
        if !reached.insert(name) {
            continue;
        }
        pending.extend(children.get(name).into_iter().flatten().copied());
    }
    let stranded: Vec<&str> = parts
        .iter()
        .map(|part| part.name.as_str())
        .filter(|name| !reached.contains(name))
        .collect();
    if stranded.is_empty() {
        return Ok(());
    }
    Err(Fault::about(
        origin.clone(),
        format!(
            "{names} hang off one another out of the root's reach, so the attachments close into a cycle",
            names = quoted(&stranded)
        ),
    ))
}

/// Which parts hang off each part.
fn children_by_parent(parts: &[Part]) -> BTreeMap<&str, Vec<&str>> {
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for part in parts {
        if let Some(attach) = part.attach.as_ref() {
            children
                .entry(attach.to.as_str())
                .or_default()
                .push(part.name.as_str());
        }
    }
    children
}

/// Several names, quoted and joined the way a sentence reads them.
fn quoted(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}
