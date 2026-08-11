//! The harness must not depend on the code it verifies.
//!
//! The question is about the *resolved* graph, not the manifest: a renderer
//! reached through three intermediaries is still a renderer. So this walks
//! cargo's own resolution, from this crate's node outwards, following every
//! dependency kind.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::process::Command;

use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

/// The crate whose dependency closure is under inspection.
const HARNESS: &str = "mc-testkit";

/// The crates this harness exists to verify, and must therefore never reach.
const VERIFIED_CRATES: [&str; 3] = ["mc-render", "mc-client", "mc-server"];

/// A dependency the harness genuinely has, used to prove the walk resolved
/// something rather than silently producing an empty closure.
const KNOWN_DEPENDENCY: &str = "image";

/// Cargo's resolved workspace metadata.
///
/// Invoked through the `CARGO` variable cargo sets for test binaries, so the
/// same toolchain that built this test resolves the graph.
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

#[test]
fn the_harness_resolves_without_the_crates_it_exists_to_verify() -> TestResult {
    let metadata = resolved_metadata()?;
    let names = package_names(&metadata);
    let edges = dependency_edges(&metadata);

    let harness_id = names
        .iter()
        .find(|(_, name)| **name == HARNESS)
        .map(|(id, _)| *id)
        .ok_or("the harness package is absent from cargo metadata")?;

    let closure: BTreeSet<&str> = reachable_from(harness_id, &edges)
        .into_iter()
        .filter_map(|id| names.get(id).copied())
        .collect();

    assert!(
        closure.contains(KNOWN_DEPENDENCY),
        "the walk resolved nothing recognisable, so the check below would be vacuous: {closure:?}"
    );

    let reached: Vec<&str> = VERIFIED_CRATES
        .into_iter()
        .filter(|verified| closure.contains(verified))
        .collect();
    assert!(
        reached.is_empty(),
        "the harness must not depend on the code it verifies, but its resolved graph reaches {reached:?}"
    );
    Ok(())
}
