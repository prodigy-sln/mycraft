//! Gathering what the adapters on this machine report, and opening a device on
//! the one the verdict picks.
//!
//! **Nothing here decides anything.** Enumeration is I/O over a nondeterministic
//! environment, so this module reduces each adapter to plain facts and hands them
//! to `mc_render::surface::startup_verdict`, which is a pure function with a test.
//! The same shape as the capture harness's own acquisition, and for the same
//! reason: a decision reached inside an `if` next to a driver call is a decision
//! nobody can check.
//!
//! This runs **before a window is opened**. A player whose machine cannot draw
//! this gets a message and a non-zero exit, never a window that opens and then
//! shows nothing.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use mc_render::surface::{
    AdapterFacts, DownlevelRequirement, StartupError, device_request, startup_verdict,
};
use mc_render::window::Ending;
use pollster::block_on;

/// The backends the client asks for.
///
/// The primaries — Vulkan, DirectX 12, Metal. The GL backend cannot execute the
/// indirect draw this renderer is built on, so including it would mean
/// enumerating adapters only to refuse them by name.
const BACKENDS: wgpu::Backends = wgpu::Backends::PRIMARY;

/// The label the device carries in a driver capture.
const DEVICE_LABEL: &str = "mycraft client";

/// An open device, and everything reached through the same instance.
///
/// The instance is kept because the surface is created from it later, once a
/// window exists to create one for; the adapter because the surface's
/// capabilities are reported against it.
#[derive(Debug)]
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    lost: Arc<AtomicBool>,
}

impl Gpu {
    /// Whether the device has been reported lost.
    ///
    /// An acquire cannot say so on its own — `wgpu` reports a lost surface and a
    /// lost device through the same `Lost`, and separates them only through the
    /// callback this flag is set from. The frame path asks this to tell a
    /// compositor restart, which is recoverable, from a device that is gone,
    /// which is not.
    pub fn is_device_lost(&self) -> bool {
        self.lost.load(Ordering::Relaxed)
    }
}

/// Enumerates the adapters this machine offers and opens a device on the first
/// one that meets the request.
///
/// # Errors
///
/// Returns [`Ending::Startup`] carrying [`StartupError::NoAdapter`] naming the
/// backends tried when nothing answered, and [`StartupError::UnmetRequirement`]
/// when what answered cannot run this draw path. An adapter that met the
/// requirements and then would not open a device is reported with the driver's own
/// words, which no variant of `StartupError` can hold.
pub fn open() -> Result<Gpu, Ending> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: BACKENDS,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = block_on(instance.enumerate_adapters(BACKENDS));
    let facts: Vec<AdapterFacts> = adapters.iter().map(facts_of).collect();

    let request = device_request();
    let index = startup_verdict(&facts, &request, &backends_tried()).map_err(Ending::Startup)?;
    let adapter = adapters
        .get(index)
        // Unreachable: the verdict answers with an index into the very slice it
        // was handed. Reported as "nothing answered" rather than panicked on,
        // because this is the startup path of a player's game.
        .ok_or_else(|| {
            Ending::Startup(StartupError::NoAdapter {
                tried: backends_tried(),
            })
        })?;

    open_device(&instance, adapter, name_at(&facts, index))
}

/// Creates the surface a window presents through.
///
/// It lives here rather than in the event adapter so that the window library and
/// the graphics API are never named in the same file: the adapter hands over its
/// window and gets back something it only ever passes on.
///
/// # Errors
///
/// Returns whatever `wgpu` reports when the platform will not give a surface for
/// this window.
pub fn create_surface<T>(
    instance: &wgpu::Instance,
    target: T,
) -> Result<wgpu::Surface<'static>, wgpu::CreateSurfaceError>
where
    T: Into<wgpu::SurfaceTarget<'static>>,
{
    instance.create_surface(target)
}

/// Opens a device on `adapter`, and arms the callback that says when it is gone.
fn open_device(
    instance: &wgpu::Instance,
    adapter: &wgpu::Adapter,
    named: String,
) -> Result<Gpu, Ending> {
    let descriptor = wgpu::DeviceDescriptor {
        label: Some(DEVICE_LABEL),
        required_limits: wgpu::Limits::downlevel_defaults(),
        ..wgpu::DeviceDescriptor::default()
    };
    let (device, queue) =
        block_on(adapter.request_device(&descriptor)).map_err(|cause| Ending::Failed {
            report: format!(
                "the adapter `{named}` reported every capability this client needs and then would \
                 not open a device: {cause}"
            ),
        })?;

    let lost = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&lost);
    device.set_device_lost_callback(move |_, _| flag.store(true, Ordering::Relaxed));

    Ok(Gpu {
        instance: instance.clone(),
        adapter: adapter.clone(),
        device,
        queue,
        lost,
    })
}

/// What the adapter the verdict picked calls itself, for a report to name.
fn name_at(facts: &[AdapterFacts], index: usize) -> String {
    facts.get(index).map_or_else(
        || "an adapter this machine no longer reports".to_owned(),
        |adapter| adapter.name.clone(),
    )
}

/// One adapter, reduced to the facts the verdict decides over.
fn facts_of(adapter: &wgpu::Adapter) -> AdapterFacts {
    let reported = adapter.get_info();
    AdapterFacts {
        name: reported.name,
        backend: format!("{:?}", reported.backend).to_lowercase(),
        downlevel: downlevel_of(&adapter.get_downlevel_capabilities().flags),
    }
}

/// Which of the capabilities this client needs an adapter reports.
///
/// Only the three are translated. A capability nothing asks for is a capability
/// no refusal could name, so carrying the rest would be a list nobody reads.
fn downlevel_of(flags: &wgpu::DownlevelFlags) -> Vec<DownlevelRequirement> {
    [
        (
            wgpu::DownlevelFlags::COMPUTE_SHADERS,
            DownlevelRequirement::ComputeShaders,
        ),
        (
            wgpu::DownlevelFlags::INDIRECT_EXECUTION,
            DownlevelRequirement::IndirectExecution,
        ),
        (
            wgpu::DownlevelFlags::VERTEX_STORAGE,
            DownlevelRequirement::VertexStorage,
        ),
    ]
    .into_iter()
    .filter(|(flag, _)| flags.contains(*flag))
    .map(|(_, requirement)| requirement)
    .collect()
}

/// The backends that were asked for, by name.
///
/// Derived from the same constant the instance is built with, so a failure names
/// what was actually tried rather than a second list somebody kept in step by
/// hand.
fn backends_tried() -> Vec<String> {
    wgpu::Backend::ALL
        .into_iter()
        .filter(|backend| BACKENDS.contains(wgpu::Backends::from(*backend)))
        .map(|backend| format!("{backend:?}").to_lowercase())
        .collect()
}
