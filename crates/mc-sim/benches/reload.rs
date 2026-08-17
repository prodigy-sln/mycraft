//! The hot-reload latency benchmark, and the budget check whose verdict is its
//! exit code.
//!
//! Run it with:
//!
//! ```text
//! cargo bench -p mc-sim --bench reload
//! ```
//!
//! **Required at this spec's own validation and at MVP exit verification, and
//! nowhere else** — the same two points, and for the same reason, as the meshing
//! benchmark next door in `mc-world`. A wall-clock threshold is not deterministic,
//! and a gate that goes red on a slower machine is a gate people learn to waive.
//! What the deterministic gate carries instead is the *identity* half: that the
//! candidate build and the re-mesh both run off the tick thread. Duration is
//! measured here, deliberately by somebody who can account for the machine.
//!
//! It follows the meshing benchmark's three steps, in this order:
//!
//! 1. **Asserts the work**, before a single timing is taken. A build that read no
//!    declaration would benchmark superbly, and so would a mesh that emitted
//!    nothing. Both expectations are **derived** — the block count from the
//!    declaration files on disk, the section count from the footprint the world
//!    reports — never snapshotted from a run of the code being measured.
//! 2. **Measures and reports with criterion.**
//! 3. **Judges its own means** against the engine's share of the budget, printing
//!    every number it used.
//!
//! **Why the budget here is 850 ms and not one second.** The end-to-end target is
//! one second from an author's save to a visible change, and **150 ms of it is
//! spent before the engine is told anything** — that is the settling window a save
//! is allowed, declared once in `mc_world::content::watch`. So the engine's share
//! is the remainder, and it is derived from the two constants rather than written
//! as a third number that would drift from both.
//!
//! **What is measured, and what is left to the identity half.** The two costly
//! steps between a save and a visible change are the candidate build — the whole
//! content root read again through the same door a launch uses — and the
//! whole-world re-mesh a geometry change forces. The tick-boundary swap resolves
//! solidity over the same footprint and is bounded by the same arithmetic; the
//! texture upload and the draw are a device's, which no benchmark in this
//! workspace can time.
//!
//! `MYCRAFT_SKIP_PERF_BUDGET` waives step 3 and nothing else. Step 1 still runs and
//! can still fail, so a waived run never means that nothing was verified.

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use criterion::Criterion;

use mc_core::block::BlockRegistry;
use mc_core::content::LayerAssignment;
use mc_sim::replay::{ReplayWorld, mesh_all};
use mc_world::content::watch::SETTLING_WINDOW;

/// The end-to-end target: a save reaches the world inside this.
const END_TO_END_TARGET: Duration = Duration::from_secs(1);

/// What the engine may spend of it — the target less the window a save settles in.
///
/// Derived rather than written, so it cannot disagree with either constant it is
/// made of.
const ENGINE_SHARE: Duration = END_TO_END_TARGET.saturating_sub(SETTLING_WINDOW);

/// The environment variable that waives the verdict and nothing else.
const SKIP: &str = "MYCRAFT_SKIP_PERF_BUDGET";

/// Where the shipped declarations live, below the content root.
const BLOCKS_DIRECTORY: &str = "blocks";

/// The extension a block declaration carries.
const BLOCK_DECLARATION: &str = "luau";

/// How many sections one column of the replay world holds.
const SECTIONS_PER_COLUMN: usize = 16;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(refused) => {
            eprintln!("mycraft: the reload benchmark could not run: {refused}");
            ExitCode::FAILURE
        }
    }
}

/// Asserts the work, measures it, then judges the two means.
///
/// # Errors
///
/// Returns an error if the shipped root cannot be read, if the world cannot be
/// generated, or if either workload is not what it was expected to be.
fn run() -> Result<ExitCode, Box<dyn Error>> {
    let root = shipped_root();
    if !root.is_dir() {
        return Err(format!("no content root at {}", root.display()).into());
    }

    let (declared, sections) = assert_the_work(&root)?;
    println!(
        "reload workload: {declared} declarations, {sections} sections meshed; \
         engine share {engine} ms of a {target} ms target after a {window} ms window",
        engine = ENGINE_SHARE.as_millis(),
        target = END_TO_END_TARGET.as_millis(),
        window = SETTLING_WINDOW.as_millis()
    );

    let build = measured("candidate build", || build_once(&root));
    let remesh = measured("whole-world re-mesh", || remesh_once(&root));
    report(&root)?;

    Ok(verdict(build, remesh))
}

/// The shipped content root, from the repository rather than from the working
/// directory.
///
/// **The relative spelling is the production one and is taken from production**,
/// which is why `shipped_directory` is asked even though its answer is a path this
/// process cannot reach: both of its arms carry the same relative root, and a
/// benchmark restating `content/base` would be a second copy of a value the loader
/// already owns. A bench runs with its own crate as the working directory, so it is
/// joined onto the repository root above this crate.
fn shipped_root() -> PathBuf {
    let relative = mc_sim::content::shipped_directory().unwrap_or_else(|root| root);
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .unwrap_or(manifest)
        .join(relative)
}

/// The work each step must be worth, established independently of either step.
///
/// # Errors
///
/// Returns an error if the declaration directory cannot be read, or if a step did
/// not do the work the count above says it should have.
fn assert_the_work(root: &Path) -> Result<(usize, usize), Box<dyn Error>> {
    let declared = declaration_files(root)?;
    let content = mc_sim::content::load(root, &LayerAssignment::none())?;
    let registered = content.registry.registered_count();
    if registered != declared {
        return Err(format!(
            "the build registered {registered} blocks and the root holds {declared} declaration \
             files, so the timing below would be over the wrong workload"
        )
        .into());
    }

    let world = ReplayWorld::generate(mc_sim::REPLAY_SEED, &content.registry)?;
    let columns = world.columns().count();
    let expected = columns * SECTIONS_PER_COLUMN;
    let meshed = mesh_all(&world, &content.registry)?.len();
    if meshed != expected {
        return Err(format!(
            "the mesh produced {meshed} sections and the world holds {columns} columns of \
             {SECTIONS_PER_COLUMN}, which is {expected}"
        )
        .into());
    }
    Ok((declared, meshed))
}

/// How many block declarations the root holds, counted on disk.
///
/// The independent oracle for the build's workload: it shares no code with the
/// loader whose output it checks.
///
/// # Errors
///
/// Returns an error if the declaration directory cannot be read.
fn declaration_files(root: &Path) -> Result<usize, Box<dyn Error>> {
    let directory: PathBuf = root.join(BLOCKS_DIRECTORY);
    let mut declared = 0_usize;
    for entry in fs::read_dir(&directory)? {
        let path = entry?.path();
        if path.extension().and_then(OsStr::to_str) == Some(BLOCK_DECLARATION) {
            declared += 1;
        }
    }
    Ok(declared)
}

/// Reads the whole root, as a reload's candidate build does.
fn build_once(root: &Path) {
    let content = mc_sim::content::load(root, &LayerAssignment::none());
    drop(black_box(content));
}

/// Meshes every section of the world, as a reload that changed what is drawn does.
fn remesh_once(root: &Path) {
    let Ok(content) = mc_sim::content::load(root, &LayerAssignment::none()) else {
        return;
    };
    mesh_and_discard(&content.registry);
}

/// The mesh itself, split out so the step above stays inside two nesting levels.
fn mesh_and_discard(registry: &BlockRegistry) {
    let Ok(world) = ReplayWorld::generate(mc_sim::REPLAY_SEED, registry) else {
        return;
    };
    drop(black_box(mesh_all(&world, registry)));
}

/// This run's own mean for `work`, over enough repetitions to be worth reading.
///
/// **The number that decides anything.** Criterion returns no estimate to a caller
/// and documents its own report files as private, so a verdict built on those
/// breaks silently on an upgrade — the meshing benchmark records the same reasoning
/// and this follows it rather than inventing a second policy.
fn measured(work: &str, mut once: impl FnMut()) -> Duration {
    /// How many repetitions the mean is taken over.
    const REPETITIONS: u32 = 10;

    let started = Instant::now();
    for _ in 0..REPETITIONS {
        once();
    }
    let mean = started.elapsed() / REPETITIONS;
    println!(
        "{work}: mean {:.1} ms over {REPETITIONS} runs",
        millis(mean)
    );
    mean
}

/// Criterion's own measurement and report, which gates nothing.
///
/// # Errors
///
/// Returns an error if the root stops being readable between the two.
fn report(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut criterion = Criterion::default();
    let mut group = criterion.benchmark_group("reload");
    group.bench_function("candidate build", |bencher| {
        bencher.iter(|| build_once(root));
    });
    group.bench_function("whole-world re-mesh", |bencher| {
        bencher.iter(|| remesh_once(root));
    });
    group.finish();
    criterion.final_summary();
    Ok(())
}

/// Whether the two means fit the engine's share, printing every number used.
fn verdict(build: Duration, remesh: Duration) -> ExitCode {
    let spent = build + remesh;
    println!(
        "reload budget: build {build:.1} ms + re-mesh {remesh:.1} ms = {spent:.1} ms against \
         {share:.1} ms",
        build = millis(build),
        remesh = millis(remesh),
        spent = millis(spent),
        share = millis(ENGINE_SHARE)
    );
    if std::env::var_os(SKIP).is_some() {
        println!("reload budget: waived by {SKIP}; the workload assertions still ran");
        return ExitCode::SUCCESS;
    }
    if spent > ENGINE_SHARE {
        eprintln!(
            "mycraft: a reload's engine share is over budget by {over:.1} ms",
            over = millis(spent.saturating_sub(ENGINE_SHARE))
        );
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// `elapsed` in milliseconds, for printing.
fn millis(elapsed: Duration) -> f64 {
    elapsed.as_secs_f64() * 1000.0
}
