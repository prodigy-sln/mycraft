//! The render input the replay produces is byte-identical across runs and
//! across thread counts.
//!
//! A committed golden frame is only worth anything if the frame's inputs are the
//! same every time. Two things can quietly break that, and each has a test here.
//! A world or a mesh that depends on anything but the seed breaks it between
//! runs. Parallel meshing that collects into anything unordered breaks it
//! between thread counts — and it breaks it *invisibly*, because on the machine
//! the goldens were captured on the worker count never changed.
//!
//! Both tests compare bytes rather than structures. Byte serialisation is
//! explicitly little-endian throughout, so "identical bytes" is a statement
//! about the buffers that reach the GPU and not about a host's word order.
//!
//! There is no separate index buffer to compare here — see `RenderInput`.

mod support;

use std::error::Error;

use mc_sim::TICK_COUNT;
use mc_sim::simulation::Simulation;
use rayon::ThreadPoolBuilder;

use support::{RenderInput, TestResult, prepare};

/// The ticks the two runs are compared at: the first, the halfway point and the
/// last, which are the three the replay declares captures for.
const SAMPLED_TICKS: [u32; 3] = [0, 60, 119];

/// The two worker counts the meshing is run under.
const ONE_WORKER: usize = 1;
const EIGHT_WORKERS: usize = 8;

#[test]
fn two_runs_of_the_replay_produce_byte_identical_render_input_at_every_sampled_tick() -> TestResult
{
    let first = run_to_the_end()?;
    let second = run_to_the_end()?;

    assert!(
        first.len() == SAMPLED_TICKS.len()
            && first.iter().all(|(_, input)| !input.vertices.is_empty()),
        "each run has to reach all {} sampled ticks with a non-empty vertex buffer, or two \
         runs that produced nothing would compare equal: {:?}",
        SAMPLED_TICKS.len(),
        first.iter().map(|(tick, _)| *tick).collect::<Vec<_>>()
    );
    assert_eq!(
        first, second,
        "the same seed run to the same tick has to pack the same bytes"
    );
    Ok(())
}

#[test]
fn meshing_the_replay_on_one_worker_and_on_eight_produces_the_same_bytes() -> TestResult {
    let single = on_workers(ONE_WORKER)?;
    let many = on_workers(EIGHT_WORKERS)?;

    assert!(
        !single.vertices.is_empty(),
        "the preparation packed no vertex at all, so both worker counts produced nothing \
         and would agree about it"
    );
    assert_eq!(
        single, many,
        "meshing order has to be section index order however many workers share the \
         work; collecting into anything unordered lets the worker count decide the bytes"
    );
    Ok(())
}

/// One whole run of the replay: prepare the scene once, then advance tick by
/// tick to the end, keeping the render input at each declared capture tick.
fn run_to_the_end() -> Result<Vec<(u32, RenderInput)>, Box<dyn Error>> {
    let input = prepare()?;
    let simulation = Simulation::new();
    let mut sampled = Vec::new();

    for _ in 0..TICK_COUNT {
        let tick = simulation.latest().tick;
        if SAMPLED_TICKS.contains(&tick) {
            sampled.push((tick, input.clone()));
        }
        simulation.advance();
    }
    Ok(sampled)
}

/// The render input prepared with the meshing confined to `workers` threads.
///
/// **`prepare` runs on the thread `install` runs it on, which is what confines
/// the meshing at all.** It calls `mc_client::startup::prepare_scene` directly
/// rather than the entry point that spawns a worker: a `std::thread` spawned
/// inside `install` does not inherit the pool, so that route would mesh both
/// worker counts on the global pool and compare bytes that could never disagree.
///
/// The failure is flattened to its own message before it crosses back out of the
/// pool. `install` returns its closure's value from another thread, so that value
/// has to be `Send`, and a boxed error is not — requiring `Send + Sync` of it
/// instead would push a bound onto every error type in the preparation chain for
/// the benefit of a test that only ever reads the success side. What a failure
/// here has to do is fail this test with something legible, and a message does
/// that.
fn on_workers(workers: usize) -> Result<RenderInput, Box<dyn Error>> {
    let pool = ThreadPoolBuilder::new().num_threads(workers).build()?;
    let prepared = pool.install(|| prepare().map_err(|failure| failure.to_string()));
    Ok(prepared?)
}
