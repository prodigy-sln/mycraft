//! The graphs this crate's seams are made of.
//!
//! Two invariants live here. The first is the GPU seam: the pure half of the
//! renderer must build with no GPU API in the graph. The second is the
//! simulation/renderer seam: neither crate may resolve the other, while the
//! composition root that wires them together must resolve both.
//!
//! Both are questions about the *resolved* graph rather than about a manifest:
//! `wgpu` reached through three intermediaries is still `wgpu`, and a
//! `[dev-dependencies]` entry is still an edge cargo resolves — which is exactly
//! how a default-featured dev-dependency on the capture harness would drag a GPU
//! API back in while looking like a feature bug in this crate.
//!
//! **They are asked of cargo two different ways, and the reason is a measured
//! fact rather than a preference.** `cargo metadata`'s `resolve` is
//! **workspace-unified**, and `--manifest-path <member>` does not escape that:
//! once `mc-client` depended on this crate with default features, the `gpu`
//! feature was re-enabled on this crate's node under `--no-default-features`
//! too. The earlier belief that scoping the manifest was enough had only ever
//! been checked while `mc-client` had no dependencies at all, so it was
//! measuring a workspace with nothing to unify with.
//!
//! So:
//!
//! - **Feature questions go to `cargo tree -p <package>`**, which resolves
//!   features as if that package alone were being built. Measured in this
//!   workspace, with the client's dependency in place: no `wgpu` line under
//!   `--no-default-features`, 93 of them without it. `cargo clippy -p mc-render
//!   --no-default-features` — which is what the gate's stage 2b actually builds
//!   — agrees.
//! - **Reachability questions go to `cargo metadata`'s resolved graph**, walked
//!   breadth-first. Unification changes which *features* are on, so it cannot
//!   invent the non-optional path edge between two workspace members, which is
//!   the whole of what the seam tests ask.
//!
//! Each assertion of an absence is paired with a test asserting the presence
//! that proves the same instrument can see the thing at all. Split into separate
//! test functions deliberately: as one test, "the control failed while the real
//! assertion still passed" is not something a test run can show you happening.
//!
//! The metadata walk is copied from `mc-world`'s and `mc-testkit`'s own
//! invariant tests rather than shared: this crate may not depend on the harness,
//! and sixty lines do not justify a crate that exists only to hold them.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

/// The crate whose dependency closure is under inspection.
const RENDERER: &str = "mc-render";

/// The crate that owns the world, the tick and the camera path, and that the
/// renderer must never name.
const SIMULATION: &str = "mc-sim";

/// The composition root, which is where the two are allowed to meet.
const COMPOSITION_ROOT: &str = "mc-client";

/// The GPU API the pure layer must be able to build without.
const GPU_API: &str = "wgpu";

/// A dependency this crate genuinely has in *both* configurations — it is where
/// `Quad` comes from, and geometry building is the pure layer's whole job. It is
/// asserted present so that a walk which resolved nothing cannot satisfy the
/// absence check below by finding nothing at all.
const KNOWN_DEPENDENCY: &str = "mc-world";

/// Which feature selection cargo should resolve the graph under.
#[derive(Debug, Clone, Copy)]
enum Features {
    /// The crate as everything else consumes it, GPU layer included.
    Default,
    /// The pure layer alone.
    NoDefault,
}

/// Cargo's resolution, rooted at `package`'s own manifest.
///
/// Invoked through the `CARGO` variable cargo sets for test binaries, so the
/// same toolchain that built this test resolves the graph. It takes no feature
/// selection: this resolve is workspace-unified whatever is passed, so a flag
/// here would be a knob that looks like it does something. Feature questions go
/// to [`feature_scoped_closure`] instead.
fn resolved_metadata(package: &str) -> Result<Value, Box<dyn Error>> {
    let cargo = std::env::var("CARGO")?;
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(package);
    let manifest = crate_root.join("Cargo.toml");
    let manifest = manifest
        .to_str()
        .ok_or("that crate's manifest path is not valid UTF-8")?
        .to_owned();

    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked"])
        .args(["--manifest-path", &manifest])
        .current_dir(&crate_root)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed: {stderr}").into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Every crate this one resolves under `features`, as cargo resolves them for a
/// build of this package alone.
///
/// `--edges all` so that build- and dev-dependencies count, which is the point:
/// a dev-dependency is an edge that can carry a GPU API back in. `--prefix none
/// --format {p}` prints one package per line, name first.
fn feature_scoped_closure(features: Features) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let cargo = std::env::var("CARGO")?;
    let mut command = Command::new(cargo);
    command
        .args(["tree", "--locked", "--package", RENDERER])
        .args(["--edges", "all", "--prefix", "none", "--format", "{p}"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")));
    if matches!(features, Features::NoDefault) {
        command.arg("--no-default-features");
    }

    let output = command.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo tree failed: {stderr}").into());
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect())
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
fn the_renderer_resolves_no_gpu_api_without_its_default_features() -> TestResult {
    let closure = feature_scoped_closure(Features::NoDefault)?;

    assert!(
        closure.iter().any(|name| name == KNOWN_DEPENDENCY),
        "the walk resolved nothing recognisable — it never reached `{KNOWN_DEPENDENCY}` — \
         so the check below would pass by finding nothing: {closure:?}"
    );
    assert!(
        !closure.iter().any(|name| name == GPU_API),
        "the pure layer must build with no GPU API in the graph, but resolving this crate \
         without its default features still reaches `{GPU_API}`: {}",
        own_crates_of(&closure)
    );
    Ok(())
}

/// The control for the assertion above, and the reason it cannot go quiet.
///
/// A walk pointed at the wrong package, an output format that stopped yielding
/// names, or a `gpu` feature that stopped pulling the GPU API at all would each
/// leave the absence above trivially true, forever. Only asking the same
/// instrument for the configuration in which the API *must* be present says so.
#[test]
fn the_renderer_resolves_the_gpu_api_with_its_default_features() -> TestResult {
    let closure = feature_scoped_closure(Features::Default)?;

    assert!(
        closure.iter().any(|name| name == GPU_API),
        "the default feature selection is the one that draws, so it must resolve `{GPU_API}`; \
         if it no longer does, the seam is measuring two identical configurations: {}",
        own_crates_of(&closure)
    );
    Ok(())
}

/// The renderer reads a snapshot and never reaches into world storage, and the
/// simulation never learns what a vertex is. Both halves are asserted, because
/// an edge in either direction breaks the seam and the two are different
/// mistakes: the renderer naming the simulation makes the transport swap a
/// rewrite, and the simulation naming the renderer puts a GPU-shaped type
/// upstream of everything.
#[test]
fn neither_the_simulation_nor_the_renderer_resolves_the_other() -> TestResult {
    let rendering = resolved_metadata(RENDERER)?;
    let simulating = resolved_metadata(SIMULATION)?;

    let renders = resolved_closure(RENDERER, &rendering)?;
    let simulates = resolved_closure(SIMULATION, &simulating)?;

    assert!(
        renders.contains(KNOWN_DEPENDENCY) && simulates.contains(KNOWN_DEPENDENCY),
        "both walks have to reach `{KNOWN_DEPENDENCY}`, which both crates genuinely \
         depend on, or a walk that resolved nothing would satisfy the absences below by \
         finding nothing at all: {:?} / {:?}",
        own_crates(&renders),
        own_crates(&simulates)
    );
    assert!(
        !renders.contains(SIMULATION) && !simulates.contains(RENDERER),
        "the snapshot seam runs one way through the composition root, so neither crate \
         may resolve the other: {:?} / {:?}",
        own_crates(&renders),
        own_crates(&simulates)
    );
    Ok(())
}

/// The crates worth naming in a failure about the GPU seam: this project's own,
/// and anything from the GPU API's own family. The other 150 are a page nobody
/// reads.
fn own_crates_of(closure: &BTreeSet<String>) -> String {
    closure
        .iter()
        .filter(|name| name.starts_with("mc-") || name.starts_with(GPU_API))
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

/// This project's own crates in a closure, which is all a seam between two of
/// them is ever about — the rest is a page of third-party names nobody reads.
fn own_crates<'a>(closure: &BTreeSet<&'a str>) -> Vec<&'a str> {
    closure
        .iter()
        .copied()
        .filter(|name| name.starts_with("mc-"))
        .collect()
}

/// The positive control for the pair above. Two crates that simply do not
/// resolve each other prove nothing about a seam — a renderer nobody wired to a
/// simulation would satisfy it forever. The composition root is where they are
/// wired together, so its graph is where both have to be present.
#[test]
fn the_composition_root_resolves_both_the_simulation_and_the_renderer() -> TestResult {
    let metadata = resolved_metadata(COMPOSITION_ROOT)?;
    let closure = resolved_closure(COMPOSITION_ROOT, &metadata)?;

    assert!(
        closure.contains(SIMULATION) && closure.contains(RENDERER),
        "the client drives the simulation and hands the renderer what it publishes, so \
         both belong in its resolved graph: {:?}",
        own_crates(&closure)
    );
    Ok(())
}
