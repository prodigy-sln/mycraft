//! Getting a device with no window, and turning what the driver says into the
//! plain facts the core decides on.
//!
//! Nothing here decides anything. It creates an instance, enumerates, maps each
//! adapter into a description, asks [`select_preferred`] which one to use, asks
//! [`classify_acquisition`] what the outcome means, and does what it is told.
//! That is the whole point of the seam: ranking two adapters would otherwise
//! need a two-adapter machine, and deciding what a failure means would otherwise
//! need an environment variable a test may not set.
//!
//! No window, no surface, no display handle is created at any point. The
//! instance is built through `new_without_display_handle`, whose name is itself
//! the guarantee.

use pollster::block_on;

use crate::frame::optins::OptIns;
use crate::frame::report::{AdapterProvenance, Backend};
use crate::frame::selection::{
    AcquireError, AcquisitionVerdict, AdapterDescription, AdapterKind, AdapterLimits, SkipNotice,
    classify_acquisition, select_preferred, unsatisfied_limit,
};

/// The label the device carries in a graphics debugger.
const DEVICE_LABEL: &str = "mycraft frame capture";

/// Which backends to try, and what the device must be able to do.
#[derive(Debug, Clone)]
pub struct AcquireOptions {
    pub backends: wgpu::Backends,
    pub required_limits: wgpu::Limits,
}

impl Default for AcquireOptions {
    /// The primary backends, and the downlevel limits.
    ///
    /// The harness renders one 2D colour target, which every downlevel device
    /// covers — asking for more would reject adapters that can do everything
    /// this crate needs.
    fn default() -> Self {
        Self {
            backends: wgpu::Backends::PRIMARY,
            required_limits: wgpu::Limits::downlevel_defaults(),
        }
    }
}

/// What an attempt to acquire a device produced.
#[derive(Debug)]
pub enum Acquisition {
    /// A device to capture on. Boxed so the two variants are of comparable size.
    Ready(Box<CaptureContext>),
    /// No device, and an opt-in that permitted saying so instead of failing.
    Skipped(SkipNotice),
}

/// A device, its queue, and what is known about the adapter behind them.
///
/// The instance and adapter handles are deliberately **not** retained: wgpu's
/// `Device` and `Queue` hold their own reference to the instance internals, so
/// keeping dead handles alongside them would be two fields nothing ever reads.
#[derive(Debug)]
pub struct CaptureContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    provenance: AdapterProvenance,
    limits: AdapterLimits,
}

impl CaptureContext {
    /// Acquires a device on the first usable adapter, or says why it could not.
    ///
    /// Failure is failure by default. `MYCRAFT_ALLOW_NO_GPU` — read into
    /// `opt_ins` by the caller, never from the environment here — is what turns
    /// it into an announced skip instead, and the notice names the variable that
    /// permitted it.
    ///
    /// # Errors
    ///
    /// Returns [`AcquireError::NoAdapter`] naming every backend it tried when
    /// nothing was enumerated, or [`AcquireError::DeviceRejected`] /
    /// [`AcquireError::DeviceUnavailable`] naming the adapter when one was found
    /// but would not open a device.
    pub fn acquire(
        opt_ins: &OptIns,
        options: &AcquireOptions,
    ) -> Result<Acquisition, AcquireError> {
        let opened = open(options);
        match classify_acquisition(described(&opened), opt_ins) {
            // `Use` arises only from an `Ok` outcome, so this arm never drops a
            // device on the floor.
            AcquisitionVerdict::Use(_) => {
                opened.map(|(context, _)| Acquisition::Ready(Box::new(context)))
            }
            AcquisitionVerdict::Skip(notice) => {
                eprintln!("{}", notice.message());
                Ok(Acquisition::Skipped(notice))
            }
            AcquisitionVerdict::Fail(cause) => Err(cause),
        }
    }

    /// Which adapter produced everything this context captures.
    #[must_use]
    pub const fn provenance(&self) -> &AdapterProvenance {
        &self.provenance
    }

    /// What the device can do, as the facts the core validates against.
    ///
    /// A capture size is checked against this rather than against a guess: the
    /// maximum texture dimension is a device fact, and `validate_frame_size`
    /// takes it as a parameter precisely so that the check stays pure.
    #[must_use]
    pub const fn limits(&self) -> AdapterLimits {
        self.limits
    }

    /// The device the caller's draw work builds its pipelines on.
    ///
    /// Draw work receives an encoder and a view, neither of which can create a
    /// pipeline, so a caller drawing anything more than a clear needs this. It
    /// is a fact the layer already holds, not a decision.
    #[must_use]
    pub const fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The queue the caller's draw work uploads through.
    ///
    /// Public for the same reason [`device`](Self::device) is, and the argument
    /// there is the argument here: draw work receives an encoder and a view,
    /// neither of which can write a buffer or a texture. A caller whose pass
    /// needs a per-frame uniform, or an indirect argument zeroed before the
    /// dispatch that fills it, has nowhere else to put it. It is a fact the
    /// layer already holds, not a decision.
    #[must_use]
    pub const fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

/// A device that opened, alongside the description of the adapter it came from.
///
/// The description is carried rather than rebuilt because
/// [`classify_acquisition`] decides over a description and an error, not over a
/// device — and reconstructing one from the context would mean inventing a value
/// for the `kind` the context has no reason to keep.
type Opened = (CaptureContext, AdapterDescription);

/// What the acquisition attempt amounted to, as the pure decision sees it.
fn described(opened: &Result<Opened, AcquireError>) -> Result<AdapterDescription, AcquireError> {
    match opened {
        Ok((_, description)) => Ok(description.clone()),
        Err(cause) => Err(cause.clone()),
    }
}

/// Enumerates, picks, and opens — the whole I/O half of acquisition.
fn open(options: &AcquireOptions) -> Result<Opened, AcquireError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: options.backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = block_on(instance.enumerate_adapters(options.backends));
    let candidates: Vec<AdapterDescription> = adapters.iter().map(description_of).collect();

    // Nothing enumerated, or nothing the ranking would take: either way this
    // machine offered no adapter on the backends asked for.
    let (adapter, description) = select_preferred(&candidates)
        .and_then(|index| Some((adapters.get(index)?, candidates.get(index)?)))
        .ok_or_else(|| AcquireError::NoAdapter {
            tried: backends_tried(options.backends),
        })?;

    open_device(adapter, description.clone(), options)
}

/// Requests a device from the chosen adapter.
fn open_device(
    adapter: &wgpu::Adapter,
    description: AdapterDescription,
    options: &AcquireOptions,
) -> Result<Opened, AcquireError> {
    let descriptor = wgpu::DeviceDescriptor {
        label: Some(DEVICE_LABEL),
        required_limits: options.required_limits.clone(),
        ..wgpu::DeviceDescriptor::default()
    };
    let (device, queue) = block_on(adapter.request_device(&descriptor))
        .map_err(|cause| rejection(&description.name, &cause, adapter.limits(), options))?;

    let context = CaptureContext {
        device,
        queue,
        // `new` is what normalises an adapter that reported no driver string to
        // the literal `unknown`, so an empty description never reaches a report.
        provenance: AdapterProvenance::new(
            &description.name,
            description.backend,
            Some(description.driver_description.as_str()),
        ),
        limits: limits_of(&adapter.limits()),
    };
    Ok((context, description))
}

/// Works out which requirement a refused device request fell short of.
///
/// The refusal itself carries no structured limit, so the shortfall is
/// recomputed from what the adapter offers against what was asked for. When
/// that comes back empty the adapter refused for a reason this harness does not
/// model, and the driver's own words are reported rather than a limit invented
/// to fill the field.
fn rejection(
    adapter: &str,
    cause: &wgpu::RequestDeviceError,
    available: wgpu::Limits,
    options: &AcquireOptions,
) -> AcquireError {
    unsatisfied_limit(&limits_of(&options.required_limits), &limits_of(&available)).map_or_else(
        || AcquireError::DeviceUnavailable {
            adapter: adapter.to_owned(),
            cause: cause.to_string(),
        },
        |requirement| AcquireError::DeviceRejected {
            adapter: adapter.to_owned(),
            requirement,
        },
    )
}

/// One enumerated adapter, reduced to the facts the ranking decides on.
fn description_of(adapter: &wgpu::Adapter) -> AdapterDescription {
    let reported = adapter.get_info();
    AdapterDescription {
        name: reported.name,
        backend: backend_of(reported.backend),
        kind: kind_of(reported.device_type),
        driver_description: reported.driver_info,
    }
}

/// The limits this harness cares about, out of everything a device reports.
const fn limits_of(limits: &wgpu::Limits) -> AdapterLimits {
    AdapterLimits {
        max_texture_dimension_2d: limits.max_texture_dimension_2d,
    }
}

/// Every backend in `backends`, in the crate's own vocabulary.
///
/// A failure has to name what it tried, and "the flags you passed" is not an
/// answer a reader can act on.
fn backends_tried(backends: wgpu::Backends) -> Vec<Backend> {
    wgpu::Backend::ALL
        .into_iter()
        .filter(|backend| backends.contains(wgpu::Backends::from(*backend)))
        .map(backend_of)
        .collect()
}

const fn backend_of(backend: wgpu::Backend) -> Backend {
    match backend {
        wgpu::Backend::Vulkan => Backend::Vulkan,
        wgpu::Backend::Dx12 => Backend::Dx12,
        wgpu::Backend::Metal => Backend::Metal,
        wgpu::Backend::Gl => Backend::Gl,
        wgpu::Backend::BrowserWebGpu => Backend::BrowserWebGpu,
        wgpu::Backend::Noop => Backend::Other,
    }
}

const fn kind_of(device_type: wgpu::DeviceType) -> AdapterKind {
    match device_type {
        wgpu::DeviceType::DiscreteGpu => AdapterKind::Discrete,
        wgpu::DeviceType::IntegratedGpu => AdapterKind::Integrated,
        wgpu::DeviceType::VirtualGpu => AdapterKind::Virtual,
        wgpu::DeviceType::Cpu => AdapterKind::Cpu,
        wgpu::DeviceType::Other => AdapterKind::Other,
    }
}
