//! The one size type the renderer speaks.
//!
//! A frame's dimensions arrive from three directions — a window's inner size, a
//! capture request, and the depth attachment the renderer allocates for itself —
//! and the first draft of this design carried two types for them. Two size types
//! is one conversion nobody performs at the moment it matters, so there is one
//! here and `mc_testkit::frame::FrameSize` stays the harness's own, converted at
//! the test boundary and nowhere else.
//!
//! Surface-format selection, the resize policy, the surface-error mapping and
//! what the device is asked for are this module's too. Every one of them is a
//! pure function over plain values: no `wgpu::` type is nameable here, so the
//! client is left translating its vocabulary into these types and holds no
//! decision of its own.

use thiserror::Error;

/// How large a frame is, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceSize {
    pub width: u32,
    pub height: u32,
}

impl SurfaceSize {
    /// Whether a frame of this size has any pixels in it at all.
    ///
    /// A minimised window reports a zero in **one** dimension as readily as in
    /// both, so this asks about either. The distinction is not academic: a policy
    /// that asked whether both were zero would keep drawing into a surface one
    /// pixel wide.
    const fn has_area(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// What a surface says about one format it offers.
///
/// The name is carried as text rather than as a `wgpu` enum because this module
/// may not name one, and because the refusal below has to be readable by whoever
/// is holding the machine that offered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceFormatFacts {
    pub name: String,
    pub is_srgb: bool,
}

/// Why no surface format could be configured.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FormatError {
    #[error(
        "this surface offers no sRGB format, and a non-sRGB target would draw a picture no golden \
         ever recorded; it offers {}",
        offered.join(", ")
    )]
    NoSrgbFormat { offered: Vec<String> },
}

/// Which of the formats a surface `offered` the window is configured with.
///
/// The first sRGB one. The hardware performs the sRGB encode on write, so a
/// non-sRGB target would put a differently-lit picture on screen than the one the
/// goldens hold — which is why the alternative is a refusal rather than a
/// fallback.
///
/// # Errors
///
/// Returns [`FormatError::NoSrgbFormat`], naming every format offered, when none
/// of them is sRGB.
pub fn select_surface_format(offered: &[SurfaceFormatFacts]) -> Result<usize, FormatError> {
    offered
        .iter()
        .position(|format| format.is_srgb)
        .ok_or_else(|| FormatError::NoSrgbFormat {
            offered: offered.iter().map(|format| format.name.clone()).collect(),
        })
}

/// What a newly reported surface size asks the client to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAction {
    Skip,
    Reconfigure(SurfaceSize),
}

/// What to do about a surface size of `requested`.
///
/// **Stateless, and that is the behaviour rather than the implementation.** A
/// window that was minimised and restored resumes drawing because this function
/// remembers nothing — there is no `minimised` flag that could be set and never
/// cleared, which is the failure the restore scenario is named for.
#[must_use]
pub const fn resize_action(requested: SurfaceSize) -> ResizeAction {
    if requested.has_area() {
        ResizeAction::Reconfigure(requested)
    } else {
        ResizeAction::Skip
    }
}

/// Whether a depth attachment last allocated at `current` still serves a frame of
/// `requested`.
///
/// The terrain pass is recorded against a depth attachment of the frame's own
/// size, so a resize needs a new one before the next pass is recorded. `None` is
/// a renderer that has not allocated one yet.
#[must_use]
pub const fn depth_needs_reallocation(
    current: Option<SurfaceSize>,
    requested: SurfaceSize,
) -> bool {
    match current {
        Some(had) => had.width != requested.width || had.height != requested.height,
        None => true,
    }
}

/// What went wrong acquiring a surface texture, in the renderer's own vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceErrorKind {
    Lost,
    Outdated,
    Timeout,
    OutOfMemory,
    DeviceLost,
    Other,
}

/// Why a run cannot continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FatalReason {
    DeviceLost,
    OutOfMemory,
}

/// What the frame path does about the surface texture it just asked for.
///
/// `Render` is the answer to an acquire that *succeeded*, which is why
/// [`surface_error_action`] never returns it: the client reaches it on the path
/// where there was no error to map. Keeping both halves in one enum is what lets
/// the frame path have a single match rather than a bool and a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameAction {
    Render,
    Reconfigure,
    Skip,
    Fatal(FatalReason),
}

/// What a failed acquire of `kind` asks the frame path to do.
///
/// A lost or outdated surface is routine on a real machine — a laptop GPU
/// switch, a compositor restart — and reconfiguring is what recovers from it. A
/// lost device is not: nothing the client can do brings it back, so retrying
/// would spin forever on a window that will never draw again.
#[must_use]
pub const fn surface_error_action(kind: SurfaceErrorKind) -> FrameAction {
    match kind {
        SurfaceErrorKind::Lost | SurfaceErrorKind::Outdated => FrameAction::Reconfigure,
        SurfaceErrorKind::Timeout | SurfaceErrorKind::Other => FrameAction::Skip,
        SurfaceErrorKind::OutOfMemory => FrameAction::Fatal(FatalReason::OutOfMemory),
        SurfaceErrorKind::DeviceLost => FrameAction::Fatal(FatalReason::DeviceLost),
    }
}

/// Which set of device limits is asked for.
///
/// One variant, deliberately: there is no way to *spell* a limit above `wgpu`'s
/// downlevel defaults here, so raising one means adding a variant — a visible
/// act that arrives with its own reason, rather than a number somebody nudged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitsProfile {
    DownlevelDefaults,
}

/// A downlevel capability the terrain draw path cannot work without.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownlevelRequirement {
    ComputeShaders,
    IndirectExecution,
    VertexStorage,
}

/// What the client asks a device for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRequest {
    pub optional_features: [(); 0],
    pub limits: LimitsProfile,
    pub downlevel: Vec<DownlevelRequirement>,
}

/// The request the client makes of every adapter, on every machine.
///
/// Nothing optional and nothing above the downlevel defaults, so an Intel UHD
/// answers it as readily as an RTX 4090. The three downlevel entries are the
/// draw path written out: a compute pass to cull with, an indirect draw to issue
/// it, and vertex data read from a storage buffer.
#[must_use]
pub fn device_request() -> DeviceRequest {
    DeviceRequest {
        optional_features: [],
        limits: LimitsProfile::DownlevelDefaults,
        downlevel: vec![
            DownlevelRequirement::ComputeShaders,
            DownlevelRequirement::IndirectExecution,
            DownlevelRequirement::VertexStorage,
        ],
    }
}

/// What one adapter reports about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterFacts {
    pub name: String,
    pub backend: String,
    pub downlevel: Vec<DownlevelRequirement>,
}

/// Why the client cannot start.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StartupError {
    #[error(
        "no usable GPU adapter answered, so there is nothing to draw with; the backends tried were \
         {}",
        tried.join(", ")
    )]
    NoAdapter { tried: Vec<String> },
    #[error(
        "the adapter `{adapter}` cannot provide {requirement:?}, which the terrain draw path needs \
         and which this client will not lower its request to do without"
    )]
    UnmetRequirement {
        adapter: String,
        requirement: DownlevelRequirement,
    },
}

/// Which of the `adapters` the client starts on, given what it is asking for.
///
/// The first one that meets every requirement. When none does, the refusal names
/// the first adapter offered and the first capability *that adapter* lacks —
/// never a lowered request, and never a window that opens and then draws
/// nothing.
///
/// # Errors
///
/// Returns [`StartupError::NoAdapter`] naming `backends_tried` when `adapters` is
/// empty, and [`StartupError::UnmetRequirement`] when none of them satisfies
/// `request`.
pub fn startup_verdict(
    adapters: &[AdapterFacts],
    request: &DeviceRequest,
    backends_tried: &[String],
) -> Result<usize, StartupError> {
    let mut refusal = None;
    for (index, adapter) in adapters.iter().enumerate() {
        let Some(requirement) = first_unmet(adapter, request) else {
            return Ok(index);
        };
        refusal = refusal.or(Some(StartupError::UnmetRequirement {
            adapter: adapter.name.clone(),
            requirement,
        }));
    }
    Err(refusal.unwrap_or(StartupError::NoAdapter {
        tried: backends_tried.to_vec(),
    }))
}

/// The first capability `request` asks for that `adapter` does not carry.
///
/// The *first requested* one rather than a fixed position in either list, so an
/// adapter missing only the middle capability is refused by the name of that one.
fn first_unmet(adapter: &AdapterFacts, request: &DeviceRequest) -> Option<DownlevelRequirement> {
    request
        .downlevel
        .iter()
        .find(|required| !adapter.downlevel.contains(required))
        .copied()
}

#[cfg(test)]
#[path = "surface_test.rs"]
mod tests;
