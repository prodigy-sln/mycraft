//! `tools/` may depend inward on `crates/`; nothing in `crates/` may depend on
//! `tools/`.
//!
//! Like `dependency_graph.rs`, the question is about the *resolved* graph and
//! not about any single manifest: a tool reached through three intermediaries
//! is still a tool. The metadata plumbing below is duplicated from that file
//! rather than shared — an integration test is its own crate, the two
//! invariants are independent, and a `tests/` module carried by both files
//! would be the same amount of code in a less obvious place.
//!
//! The walk reads `cargo metadata`, so this file creates no build edge onto
//! `tools/` and can live under `crates/` without being the very thing it
//! forbids.
//!
//! **Both directions are asserted, and the second is not decoration.** "No
//! engine crate reaches the tool" is vacuously true of a workspace where the
//! tool does not exist, and equally true of one where it exists but is joined
//! to nothing. The inward edge has to be *observed* for the outward absence to
//! carry any information.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::process::Command;

use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

/// The tool crate under `tools/`. Nothing under `crates/` may resolve through
/// it, and it must itself resolve inward onto the engine.
const TOOL: &str = "voxforge";

/// Every package this walk inspects, paired with a dependency it genuinely
/// has — `None` where the crate declares none at all, in which case its whole
/// resolved closure is the crate itself.
///
/// The pairs are the positive control. `reachable_from` inserts its own root
/// unconditionally, so "the closure contains the crate" proves nothing about
/// whether the walk resolved anything; a dependency one step out does.
const INSPECTED: [(&str, Option<&str>); 11] = [
    ("mc-core", Some("thiserror")),
    ("mc-world", Some("mc-core")),
    ("mc-script", Some("mlua")),
    ("mc-proto", None),
    ("mc-net", None),
    ("mc-sim", Some("rayon")),
    ("mc-render", Some("glam")),
    ("mc-client", Some("winit")),
    ("mc-server", None),
    ("mc-testkit", Some("image")),
    (TOOL, Some("mc-core")),
];

/// What the walk saw about one package's reach onto the tool.
#[derive(Debug, PartialEq, Eq)]
enum ToolReach {
    /// The package is not in the resolved metadata at all. An answer the walk
    /// could not give must never read the same as a clean one.
    PackageAbsent,
    ResolvesWithoutTheTool,
    ResolvesThroughTheTool,
}

/// What the walk saw about one package's own dependencies.
#[derive(Debug, PartialEq, Eq)]
enum ClosureVerdict {
    PackageAbsent,
    ReachesItsNamedDependency,
    MissesItsNamedDependency,
    ClosureIsTheCrateAlone,
    ClosureIsNotTheCrateAlone,
}

/// Cargo's resolved workspace metadata, through the `CARGO` variable cargo
/// sets for test binaries, so the toolchain that built this test resolves it.
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

/// The resolved closure of each inspected package, keyed by package name.
/// A package missing from the workspace is simply absent from the map, which
/// is what makes an unresolvable crate distinguishable from a clean one.
fn resolved_closures(metadata: &Value) -> BTreeMap<&str, BTreeSet<&str>> {
    let names = package_names(metadata);
    let edges = dependency_edges(metadata);
    names
        .iter()
        .filter(|(_, name)| INSPECTED.iter().any(|(inspected, _)| inspected == *name))
        .map(|(id, name)| {
            let closure = reachable_from(id, &edges)
                .into_iter()
                .filter_map(|reached| names.get(reached).copied())
                .collect();
            (*name, closure)
        })
        .collect()
}

fn tool_reach(closure: Option<&BTreeSet<&str>>) -> ToolReach {
    match closure {
        None => ToolReach::PackageAbsent,
        Some(closure) if closure.contains(TOOL) => ToolReach::ResolvesThroughTheTool,
        Some(_) => ToolReach::ResolvesWithoutTheTool,
    }
}

fn closure_verdict(closure: Option<&BTreeSet<&str>>, dependency: Option<&str>) -> ClosureVerdict {
    let Some(closure) = closure else {
        return ClosureVerdict::PackageAbsent;
    };
    match dependency {
        Some(dependency) if closure.contains(dependency) => {
            ClosureVerdict::ReachesItsNamedDependency
        }
        Some(_) => ClosureVerdict::MissesItsNamedDependency,
        None if closure.len() == 1 => ClosureVerdict::ClosureIsTheCrateAlone,
        None => ClosureVerdict::ClosureIsNotTheCrateAlone,
    }
}

#[test]
fn no_engine_crate_resolves_through_the_tool_crate() -> TestResult {
    let metadata = resolved_metadata()?;
    let closures = resolved_closures(&metadata);

    assert!(
        closures.contains_key(TOOL),
        "`{TOOL}` is absent from the resolved workspace, so every verdict below would read clean for want of anything to reach"
    );

    let engine = || INSPECTED.iter().filter(|(package, _)| *package != TOOL);
    let observed: BTreeMap<&str, ToolReach> = engine()
        .map(|(package, _)| (*package, tool_reach(closures.get(package))))
        .collect();
    let expected: BTreeMap<&str, ToolReach> = engine()
        .map(|(package, _)| (*package, ToolReach::ResolvesWithoutTheTool))
        .collect();

    assert_eq!(
        observed, expected,
        "the dependency direction between crates/ and tools/ is not what the workspace claims"
    );
    Ok(())
}

#[test]
fn every_inspected_package_resolves_a_dependency_it_genuinely_has() -> TestResult {
    let metadata = resolved_metadata()?;
    let closures = resolved_closures(&metadata);

    let observed: BTreeMap<&str, ClosureVerdict> = INSPECTED
        .iter()
        .map(|(package, dependency)| {
            (
                *package,
                closure_verdict(closures.get(package), *dependency),
            )
        })
        .collect();
    let expected: BTreeMap<&str, ClosureVerdict> = INSPECTED
        .iter()
        .map(|(package, dependency)| match dependency {
            Some(_) => (*package, ClosureVerdict::ReachesItsNamedDependency),
            None => (*package, ClosureVerdict::ClosureIsTheCrateAlone),
        })
        .collect();

    assert_eq!(
        observed, expected,
        "the walk did not resolve what each inspected package genuinely depends on, so a clean direction verdict would prove nothing"
    );
    Ok(())
}
