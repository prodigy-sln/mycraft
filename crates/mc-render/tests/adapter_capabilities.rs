//! The one assumption the GPU-driven draw shape rests on, checked before any
//! pipeline exists.
//!
//! Terrain draws through a compute pass that culls sections, compacts their
//! indices and writes an indirect argument buffer a single
//! `draw_indexed_indirect` then consumes. Three *downlevel capabilities* —
//! not optional device features — have to be present for that shape to exist at
//! all: compute shaders, indirect execution, and storage buffers readable from
//! the vertex stage.
//!
//! This is deliberately the first thing phase 4 asserts. Discovering it at the
//! windowed client's device request would put the discovery after two GPU
//! phases had been built on it, and `crates/mc-render/CLAUDE.md` says what a
//! shortfall costs: a fallback path needs its own golden frame, which is a spec
//! escalation rather than something implementation absorbs.
//!
//! Every adapter the primary backends offer is checked, not merely the one a
//! capture would pick. The declared hardware range runs from an RTX 4090 to an
//! Intel UHD 770 and the harness's ranking is free to choose either, so an
//! adapter that falls short is a fact worth knowing whichever one it is.
//!
//! This carries no acceptance scenario. It is a checked assumption, recorded as
//! such in `test-map.md`.

use std::error::Error;

use mc_testkit::frame::{OptIns, wgpu};

type TestResult = Result<(), Box<dyn Error>>;

/// What the compute cull pass, the indirect draw and the vertex stage need.
const REQUIRED: wgpu::DownlevelFlags = wgpu::DownlevelFlags::COMPUTE_SHADERS
    .union(wgpu::DownlevelFlags::INDIRECT_EXECUTION)
    .union(wgpu::DownlevelFlags::VERTEX_STORAGE);

/// The backends a capture and the client both run on.
const BACKENDS: wgpu::Backends = wgpu::Backends::PRIMARY;

#[test]
fn every_adapter_this_machine_offers_can_run_the_gpu_driven_terrain_draw() -> TestResult {
    let Some(adapters) = adapters()? else {
        return Ok(());
    };

    let short: Vec<String> = adapters.iter().filter_map(shortfall).collect();

    assert!(
        short.is_empty(),
        "the terrain draw needs {REQUIRED:?} of every adapter it may run on, and {} of {} fall \
         short: {}",
        short.len(),
        adapters.len(),
        short.join("; ")
    );
    Ok(())
}

/// What `adapter` is missing, or `None` when it is missing nothing.
fn shortfall(adapter: &wgpu::Adapter) -> Option<String> {
    let missing = REQUIRED.difference(adapter.get_downlevel_capabilities().flags);
    (!missing.is_empty()).then(|| format!("`{}` lacks {missing:?}", adapter.get_info().name))
}

/// Every adapter the primary backends enumerate, or `None` when there are none
/// and the opt-in permitted saying so.
///
/// No window and no surface is created: the instance is built through
/// `new_without_display_handle`, whose name is the guarantee.
fn adapters() -> Result<Option<Vec<wgpu::Adapter>>, Box<dyn Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: BACKENDS,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = pollster::block_on(instance.enumerate_adapters(BACKENDS));

    if !adapters.is_empty() {
        return Ok(Some(adapters));
    }
    if OptIns::from_environment().allow_no_gpu {
        eprintln!("skipping the capability assertion: no adapter on {BACKENDS:?}, permitted");
        return Ok(None);
    }
    Err(format!("no adapter answered on {BACKENDS:?}, and nothing permitted skipping").into())
}
