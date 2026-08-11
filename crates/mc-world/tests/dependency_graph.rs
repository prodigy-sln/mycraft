//! The crate every other crate depends on must not inherit a file-format parser.
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

/// The definition-file parser. It belongs to the loader and to nothing else.
const PARSER: &str = "toml";

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
fn the_crate_owning_the_registry_contract_resolves_no_definition_file_parser() -> TestResult {
    let metadata = resolved_metadata()?;
    let closure = resolved_closure(CONTRACT_CRATE, &metadata)?;

    assert!(
        closure.contains(CONTRACT_KNOWN_DEPENDENCY),
        "the walk resolved nothing recognisable, so the check below would be vacuous: {closure:?}"
    );
    assert!(
        !closure.contains(PARSER),
        "the registry contract must not know what a definition file is spelled in, \
         but its resolved graph reaches `{PARSER}`: {closure:?}"
    );
    Ok(())
}

/// A guard rather than a scenario, and the reason the check above cannot go quiet.
/// The day someone deletes the loader and hard-codes definitions in Rust — the
/// exact regression this feature exists to prevent — `mc-core` would still be
/// parser-free and the scenario above would still pass, cheerfully, forever.
#[test]
fn the_crate_owning_the_content_loader_does_resolve_a_definition_file_parser() -> TestResult {
    let metadata = resolved_metadata()?;
    let closure = resolved_closure(LOADER_CRATE, &metadata)?;

    assert!(
        closure.contains(PARSER),
        "the loader is what reads definition files, so it must resolve `{PARSER}`; \
         if it no longer does, definitions are coming from somewhere else: {closure:?}"
    );
    Ok(())
}
