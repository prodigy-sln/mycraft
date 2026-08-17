//! The crate every other crate depends on must not inherit a way of reading a
//! declaration.
//!
//! **There are two of those now and there used to be one.** A block declaration
//! is a chunk a scripting host evaluates and a HUD declaration is a document a
//! parser reads, so a guard naming only the parser would have gone on passing
//! while saying nothing at all about the way block definitions actually arrive.
//! Both are named below, and the crate owning the registry contract must reach
//! neither.
//!
//! The question is about the *resolved* graph, not the manifest: a parser reached
//! through three intermediaries is still a parser, and a `[dev-dependencies]`
//! entry is still an edge cargo resolves. So this walks cargo's own resolution
//! outwards from a crate's node, following every dependency kind.
//!
//! The walk is copied from `mc-testkit`'s own invariant test rather than shared:
//! this crate may not depend on the harness, and sixty lines do not justify a
//! crate that exists only to hold them.

mod common;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::process::Command;

use common::TestResult;
use serde_json::Value;

/// The crate that owns the registry contract, and therefore the one every other
/// crate — including MVP 2's scripting host — will depend on.
const CONTRACT_CRATE: &str = "mc-core";

/// The crate that owns the reader turning a content root into definitions.
const LOADER_CRATE: &str = "mc-world";

/// How a HUD declaration is read, and how a block declaration is.
///
/// Both belong to the loader and to nothing else. The parser is named as the HUD
/// format's rather than as "the declaration format's", because block
/// declarations stopped being a document the day they became a chunk — and a
/// constant still calling it the definition-file parser would have this guard
/// passing for a reason that is no longer true.
const HUD_PARSER: &str = "toml";
const DECLARATION_EVALUATOR: &str = "mlua";

/// A dependency the contract crate genuinely has, so that a walk which resolved
/// nothing cannot pass the check below by finding nothing.
const CONTRACT_KNOWN_DEPENDENCY: &str = "thiserror";

/// Cargo's resolved workspace metadata.
///
/// Invoked through the `CARGO` variable cargo sets for test binaries, so the same
/// toolchain that built this test resolves the graph.
fn resolved_metadata() -> Result<Value, Box<dyn Error>> {
    let cargo = std::env::var("CARGO")?;
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed: {stderr}").into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn package_names(metadata: &Value) -> BTreeMap<&str, &str> {
    metadata
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|package| Some((package.get("id")?.as_str()?, package.get("name")?.as_str()?)))
        .collect()
}

fn dependency_edges(metadata: &Value) -> BTreeMap<&str, Vec<&str>> {
    metadata
        .get("resolve")
        .and_then(|resolve| resolve.get("nodes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|node| {
            let id = node.get("id")?.as_str()?;
            let edges = node
                .get("deps")?
                .as_array()?
                .iter()
                .filter_map(|dependency| dependency.get("pkg")?.as_str())
                .collect();
            Some((id, edges))
        })
        .collect()
}

fn reachable_from<'a>(root: &'a str, edges: &BTreeMap<&'a str, Vec<&'a str>>) -> BTreeSet<&'a str> {
    let mut seen = BTreeSet::new();
    let mut pending = VecDeque::from([root]);
    while let Some(node) = pending.pop_front() {
        if !seen.insert(node) {
            continue;
        }
        for next in edges.get(node).into_iter().flatten() {
            pending.push_back(*next);
        }
    }
    seen
}

/// Every crate `package` resolves, directly or through any number of hops.
fn resolved_closure<'a>(
    package: &str,
    metadata: &'a Value,
) -> Result<BTreeSet<&'a str>, Box<dyn Error>> {
    let names = package_names(metadata);
    let edges = dependency_edges(metadata);
    let root = names
        .iter()
        .find(|(_, name)| **name == package)
        .map(|(id, _)| *id)
        .ok_or_else(|| format!("`{package}` is absent from cargo metadata"))?;
    Ok(reachable_from(root, &edges)
        .into_iter()
        .filter_map(|id| names.get(id).copied())
        .collect())
}

#[test]
fn the_crate_owning_the_registry_contract_reads_no_declaration_in_either_way() -> TestResult {
    let metadata = resolved_metadata()?;
    let closure = resolved_closure(CONTRACT_CRATE, &metadata)?;

    assert!(
        closure.contains(CONTRACT_KNOWN_DEPENDENCY),
        "the walk resolved nothing recognisable, so the checks below would be vacuous: {closure:?}"
    );
    assert_eq!(
        (
            closure.contains(HUD_PARSER),
            closure.contains(DECLARATION_EVALUATOR)
        ),
        (false, false),
        "the registry contract must not know what a declaration is written in — neither the \
         document a HUD is nor the chunk a block is — and its resolved graph reaches one of \
         them: {closure:?}"
    );
    Ok(())
}

/// A guard rather than a scenario, and the reason the check above cannot go quiet.
/// The day someone deletes the loader and hard-codes definitions in Rust — the
/// exact regression this feature exists to prevent — `mc-core` would still be
/// parser-free and the scenario above would still pass, cheerfully, forever.
///
/// **Both are asked for, and the evaluator is the half that would go missing.**
/// The loader kept resolving the parser through the swap because the HUD format
/// did not change, so the parser alone can no longer tell a loader that reads
/// block declarations from one that has stopped.
#[test]
fn the_crate_owning_the_content_loader_reads_declarations_in_both_ways() -> TestResult {
    let metadata = resolved_metadata()?;
    let closure = resolved_closure(LOADER_CRATE, &metadata)?;

    assert_eq!(
        (
            closure.contains(HUD_PARSER),
            closure.contains(DECLARATION_EVALUATOR)
        ),
        (true, true),
        "the loader is what reads declarations: a HUD through `{HUD_PARSER}` and a block through \
         `{DECLARATION_EVALUATOR}`. If it no longer resolves one of them, declarations of that \
         kind are coming from somewhere else: {closure:?}"
    );
    Ok(())
}
