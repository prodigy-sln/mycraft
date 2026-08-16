//! The client binary cannot reach the scripting host, in the resolved graph and
//! across every dependency kind.
//!
//! The question is about the *resolved* graph rather than about any manifest: a
//! crate reached through three intermediaries is still reached, and a
//! dev-dependency edge compiles just as much code as a normal one. So the walk
//! reads `cargo metadata` and follows `resolve.nodes[].deps` — which cargo emits
//! with every kind already folded in — rather than reading `Cargo.toml` (direct
//! dependencies only) or `Cargo.lock` (every workspace member, which would make
//! the assertion vacuously false).
//!
//! This is a **third** walker of that metadata, and the duplication is
//! deliberate on the reasoning `crates/mc-testkit/tests/workspace_layering.rs`
//! already records for the second: an integration test is its own crate, the
//! invariants are independent, and a `tests/` module carried by several files
//! would be the same amount of code in a less obvious place.
//!
//! # An absence is not a verdict
//!
//! "The closure does not contain the host" is exactly what a walk that resolved
//! nothing reports, and exactly what a workspace with no such crate reports.
//! Both are answers the walk could not give, and neither is good news, so the
//! walk returns an enumerated verdict instead of a boolean. Asserting the exact
//! verdict rejects every other one *including* the two that mean "I could not
//! look", which is what makes a vanished package redden here for free.
//!
//! Two of the three checks below are that guard read out loud: one doctors the
//! metadata so the host is absent altogether and requires a refusal rather than
//! a clean report, and one asks the same walk about a dependency the client
//! genuinely has. The second is not decoration — the walk seeds itself with its
//! own root unconditionally, so "the closure contains the client" would be true
//! of a walk that followed no edge at all.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::process::Command;

use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

/// The binary whose closure is under inspection.
const CLIENT: &str = "mc-client";

/// The crate the client must not resolve, in any dependency kind.
const SCRIPTING_HOST: &str = "mc-script";

/// A dependency the client genuinely has, one step out from its root.
///
/// The window and the event loop: the client cannot open a window without it,
/// so an edge to it disappearing means the walk broke rather than that the
/// client changed.
const A_DEPENDENCY_THE_CLIENT_HAS: &str = "winit";

/// What the walk saw about one package's presence in another's resolved
/// closure.
///
/// The two refusals come first when the verdict is decided, because each
/// explains away any answer that would follow it: a closure is only evidence
/// about packages that were in the metadata to begin with.
#[derive(Debug, PartialEq, Eq)]
enum ClosureVerdict {
    /// The package the walk starts from is not in the resolved metadata, so
    /// there is no closure to report on.
    RootAbsentFromTheWorkspace,
    /// The package being looked for is not in the resolved metadata, so its
    /// absence from the closure says nothing about the closure.
    ProbeAbsentFromTheWorkspace,
    ProbeOutsideTheClosure,
    ProbeInsideTheClosure,
}

/// Cargo's resolved workspace metadata, obtained through the `CARGO` variable
/// cargo sets for test binaries so that the toolchain which built this test is
/// the one that resolves it.
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

/// Every resolved edge, keyed by package id.
///
/// `resolve.nodes[].deps` is cargo's own resolution with normal, build and dev
/// dependencies already merged, which is the whole reason the walk reads it
/// rather than any manifest.
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

fn id_named<'a>(names: &BTreeMap<&'a str, &'a str>, package: &str) -> Option<&'a str> {
    names
        .iter()
        .find(|(_, name)| **name == package)
        .map(|(id, _)| *id)
}

/// Whether `probe` is reachable from `root` in the resolved graph.
fn closure_verdict(metadata: &Value, root: &str, probe: &str) -> ClosureVerdict {
    let names = package_names(metadata);
    let Some(root_id) = id_named(&names, root) else {
        return ClosureVerdict::RootAbsentFromTheWorkspace;
    };
    if id_named(&names, probe).is_none() {
        return ClosureVerdict::ProbeAbsentFromTheWorkspace;
    }
    let closure: BTreeSet<&str> = reachable_from(root_id, &dependency_edges(metadata))
        .into_iter()
        .filter_map(|reached| names.get(reached).copied())
        .collect();
    if closure.contains(probe) {
        ClosureVerdict::ProbeInsideTheClosure
    } else {
        ClosureVerdict::ProbeOutsideTheClosure
    }
}

/// The same metadata with every trace of `package` taken out of it.
///
/// Doctoring the real document rather than hand-writing one, so the fixture has
/// the shape cargo actually emits: a hand-built stub would agree with whatever
/// the walk expects to find and prove nothing about the walk's behaviour on the
/// real thing.
///
/// It refuses if the package was not there to remove, which is what keeps this
/// fixture from silently becoming a no-op the day the crate is renamed.
fn metadata_without(metadata: &Value, package: &str) -> Result<Value, Box<dyn Error>> {
    let removed = ids_of(metadata, package);
    if removed.is_empty() {
        return Err(format!(
            "`{package}` is not in the resolved metadata, so removing it changes nothing and \
             the check below would prove nothing"
        )
        .into());
    }
    let mut doctored = metadata.clone();
    let packages = doctored
        .get_mut("packages")
        .ok_or("the resolved metadata carries no package array")?;
    drop_entries(packages, &removed)?;
    let nodes = doctored
        .get_mut("resolve")
        .and_then(|resolve| resolve.get_mut("nodes"))
        .ok_or("the resolved metadata carries no resolve node array")?;
    drop_entries(nodes, &removed)?;
    Ok(doctored)
}

/// Every package id the metadata gives the name `package`.
fn ids_of(metadata: &Value, package: &str) -> BTreeSet<String> {
    package_names(metadata)
        .into_iter()
        .filter(|(_, name)| *name == package)
        .map(|(id, _)| id.to_owned())
        .collect()
}

/// Drops every entry of `entries` whose `id` is one of `removed`.
fn drop_entries(entries: &mut Value, removed: &BTreeSet<String>) -> Result<(), Box<dyn Error>> {
    entries
        .as_array_mut()
        .ok_or("the resolved metadata holds something other than an array here")?
        .retain(|entry| {
            entry
                .get("id")
                .and_then(Value::as_str)
                .is_none_or(|id| !removed.contains(id))
        });
    Ok(())
}

#[test]
fn the_client_binary_resolves_nothing_that_reaches_the_scripting_host() -> TestResult {
    let metadata = resolved_metadata()?;

    assert_eq!(
        closure_verdict(&metadata, CLIENT, SCRIPTING_HOST),
        ClosureVerdict::ProbeOutsideTheClosure,
        "the client is untrusted code running on a player's machine and the scripting host is \
         the server's enforcement of what a mod may do. An edge between them — through any \
         intermediary, in any dependency kind — puts the enforcement inside the thing it is \
         enforcing against"
    );
    Ok(())
}

#[test]
fn a_walk_that_cannot_see_the_scripting_host_at_all_refuses_instead_of_reporting_the_client_clean()
-> TestResult {
    let metadata = resolved_metadata()?;
    let doctored = metadata_without(&metadata, SCRIPTING_HOST)?;

    assert_eq!(
        closure_verdict(&doctored, CLIENT, SCRIPTING_HOST),
        ClosureVerdict::ProbeAbsentFromTheWorkspace,
        "a package the walk cannot find is not a package the client does not reach. Reporting \
         those two the same way is how this check would go green forever the day the crate is \
         renamed, moved out of the workspace, or dropped from the resolution"
    );
    Ok(())
}

#[test]
fn the_same_walk_reports_a_dependency_the_client_genuinely_has_as_present() -> TestResult {
    let metadata = resolved_metadata()?;

    assert_eq!(
        closure_verdict(&metadata, CLIENT, A_DEPENDENCY_THE_CLIENT_HAS),
        ClosureVerdict::ProbeInsideTheClosure,
        "the walk seeds itself with its own root, so a closure containing the client proves \
         nothing about whether a single edge was followed. A dependency one step out is the \
         only thing that says the graph was traversed at all"
    );
    Ok(())
}
