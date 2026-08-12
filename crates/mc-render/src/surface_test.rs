//! The surface policies the windowed client is built from: which format is
//! configured, what a resize does, what a failed acquire does, and what the
//! device is asked for.
//!
//! Every one of these is a pure function over plain values, which is why they
//! live here and not in the client. No window, no compositor and no adapter
//! exists in this test binary, and none of the decisions below needs one — the
//! client is left holding the translation from `winit` and `wgpu` vocabulary
//! into these types, and nothing else.
//!
//! # What each fixture is chosen to falsify
//!
//! Each function below has an obvious wrong implementation that a
//! typical-looking fixture would wave through, so the fixtures are picked
//! against those rather than against a typical case:
//!
//! - the offered-format list is non-sRGB first and carries **two** sRGB
//!   formats, so "return the first offered" answers 0 and "return the last
//!   sRGB" answers 2. Only "the first sRGB offered" answers 1, which is the
//!   value `spec.md` declares for exactly this list;
//! - the zero-dimension sizes are **asymmetric** — one zero width, one zero
//!   height — because a 0 x 0 fixture cannot tell "either dimension is zero"
//!   from "both dimensions are zero", and the second of those is a window that
//!   keeps drawing into a surface one pixel wide;
//! - the adapter that cannot execute an indirect draw satisfies the other two
//!   requirements, so a refusal that always names the first requirement in the
//!   list is caught;
//! - the depth policy is asked all three of its questions in one assertion, so
//!   neither a constant `true` nor a constant `false` survives it.
//!
//! # Exit statuses
//!
//! Two scenarios here end with "and exit with a non-zero status". A test cannot
//! portably take a machine's GPU away or lose a device on purpose, so what is
//! asserted is the pure half: the verdict the client reaches, and the exit code
//! [`crate::window::exit_code`] maps it to. The `main` that returns that code is
//! wiring; the decision is here, where it is counted and can be read.

use super::{
    AdapterFacts, DeviceRequest, DownlevelRequirement, FatalReason, FormatError, FrameAction,
    LimitsProfile, ResizeAction, StartupError, SurfaceErrorKind, SurfaceFormatFacts, SurfaceSize,
    depth_needs_reallocation, device_request, resize_action, select_surface_format,
    startup_verdict, surface_error_action,
};
use crate::window::{Ending, exit_code};

/// The formats a surface offers, in the order `spec.md` declares them.
///
/// The first is not sRGB and the last two are, which is what makes the answer
/// below distinguish three different implementations rather than one.
const OFFERED: [(&str, bool); 3] = [
    ("Bgra8Unorm", false),
    ("Bgra8UnormSrgb", true),
    ("Rgba8UnormSrgb", true),
];

/// Which of those three the client configures: the first sRGB one.
const FIRST_SRGB: usize = 1;

/// A surface with nothing sRGB on offer at all.
const NON_SRGB_ONLY: [&str; 2] = ["Bgra8Unorm", "Rgba8Unorm"];

/// The backends the client tries, in the order it tries them.
const BACKENDS: [&str; 3] = ["vulkan", "dx12", "gl"];

/// A machine on which nothing answered.
const NO_ADAPTERS: &[AdapterFacts] = &[];

/// What the draw path needs of an adapter, below `wgpu`'s downlevel defaults:
/// a compute pass to cull with, an indirect draw to issue, and vertex data read
/// from a storage buffer.
const REQUIRED_DOWNLEVEL: [DownlevelRequirement; 3] = [
    DownlevelRequirement::ComputeShaders,
    DownlevelRequirement::IndirectExecution,
    DownlevelRequirement::VertexStorage,
];

/// The size a resize scenario asks for.
const RESIZED: SurfaceSize = SurfaceSize {
    width: 1600,
    height: 900,
};

/// The size the depth attachment was last allocated at, before that resize.
const PREVIOUS: SurfaceSize = SurfaceSize {
    width: 1280,
    height: 720,
};

/// A minimised window, one dimension at a time. Deliberately not `0 x 0`.
const ZERO_WIDTH: SurfaceSize = SurfaceSize {
    width: 0,
    height: 900,
};
const ZERO_HEIGHT: SurfaceSize = SurfaceSize {
    width: 1600,
    height: 0,
};

#[test]
fn the_first_srgb_format_a_surface_offers_is_the_one_configured() {
    let offered = offered_formats(&OFFERED);

    assert_eq!(
        select_surface_format(&offered),
        Ok(FIRST_SRGB),
        "the first sRGB format offered is the one taken: a non-sRGB target would encode nothing \
         and a window would differ from every golden. This list opens with a non-sRGB format and \
         carries two sRGB ones, so taking the first offered or the last sRGB both answer a \
         different index"
    );
}

#[test]
fn a_surface_offering_no_srgb_format_is_refused_and_names_what_it_did_offer() {
    let offered = offered_formats(&NON_SRGB_ONLY.map(|name| (name, false)));

    let verdict = select_surface_format(&offered);

    let reported = reported(&verdict);
    assert_eq!(
        (
            verdict,
            NON_SRGB_ONLY.iter().all(|name| reported.contains(name))
        ),
        (
            Err(FormatError::NoSrgbFormat {
                offered: NON_SRGB_ONLY.iter().map(|&name| name.to_owned()).collect()
            }),
            true
        ),
        "a surface with nothing sRGB on offer is refused rather than configured anyway, and the \
         refusal names every format it did offer: `{reported}`"
    );
}

#[test]
fn a_startup_that_finds_no_adapter_names_every_backend_it_tried_and_ends_non_zero() {
    let tried = BACKENDS.iter().map(|&name| name.to_owned()).collect();

    let verdict = startup_verdict(NO_ADAPTERS, &device_request(), &backends());

    let reported = startup_report(&verdict);
    let code = exit_code_of(&verdict);
    assert_eq!(
        (
            verdict,
            BACKENDS.iter().all(|name| reported.contains(name)),
            code != 0
        ),
        (Err(StartupError::NoAdapter { tried }), true, true),
        "a player whose machine offered no adapter gets a report naming the backends that were \
         tried and a process that ends non-zero, never a window that opens and never draws: \
         `{reported}`, exit code {code}"
    );
}

#[test]
fn the_device_is_asked_for_no_optional_feature_and_no_limit_above_the_downlevel_defaults() {
    assert_eq!(
        device_request(),
        DeviceRequest {
            optional_features: [],
            limits: LimitsProfile::DownlevelDefaults,
            downlevel: REQUIRED_DOWNLEVEL.to_vec(),
        },
        "the client asks for nothing a low-end adapter cannot give. Two thirds of this is pinned \
         by the types — an empty optional-feature set is `[(); 0]` and `LimitsProfile` has no \
         variant that can spell a limit above the downlevel defaults — and the third is not: the \
         requirement list is a value, and a request that named none of them would ask for a \
         device that cannot run this draw path"
    );
}

#[test]
fn an_adapter_missing_one_requested_capability_is_refused_and_names_that_capability() {
    let adapter = adapter_without_indirect_execution();
    let named = adapter.name.clone();

    let verdict = startup_verdict(&[adapter], &device_request(), &backends());

    let reported = startup_report(&verdict);
    assert_eq!(
        (
            verdict,
            reported.contains(&named),
            reported.contains(&format!("{:?}", DownlevelRequirement::IndirectExecution))
        ),
        (
            Err(StartupError::UnmetRequirement {
                adapter: named.clone(),
                requirement: DownlevelRequirement::IndirectExecution,
            }),
            true,
            true
        ),
        "an adapter that cannot meet the request is refused by name, naming the one capability it \
         lacks rather than the request being quietly lowered to fit it. This adapter satisfies \
         the other two, so a refusal that always reports the first requirement is wrong here: \
         `{reported}`"
    );
}

#[test]
fn a_resize_to_a_frame_with_area_reconfigures_the_surface_at_that_size() {
    assert_eq!(
        resize_action(RESIZED),
        ResizeAction::Reconfigure(RESIZED),
        "a resize to {} x {} reconfigures the surface at exactly that size and the next frame is \
         drawn; `Reconfigure` is the only answer that carries a size a frame could be drawn at",
        RESIZED.width,
        RESIZED.height
    );
}

#[test]
fn a_depth_attachment_is_reallocated_unless_it_already_matches_the_frame() {
    assert_eq!(
        (
            depth_needs_reallocation(None, RESIZED),
            depth_needs_reallocation(Some(PREVIOUS), RESIZED),
            depth_needs_reallocation(Some(RESIZED), RESIZED),
        ),
        (true, true, false),
        "the terrain pass is recorded against a depth attachment of the frame's own size, so one \
         is allocated when there is none and when the one there is was made for another size. \
         The third question is what keeps the other two from being answered by a constant: an \
         attachment already at {} x {} is the one the pass wants, and reallocating it every frame \
         would put a texture creation on the frame path",
        RESIZED.width,
        RESIZED.height
    );
}

#[test]
fn a_surface_size_with_either_dimension_zero_is_neither_reconfigured_nor_drawn() {
    assert_eq!(
        [resize_action(ZERO_WIDTH), resize_action(ZERO_HEIGHT)],
        [ResizeAction::Skip, ResizeAction::Skip],
        "a minimised window reports a size with a zero dimension, and neither reconfiguration nor \
         a frame happens at that size however often a redraw is asked for. The two sizes are \
         asymmetric on purpose: a policy asking whether *both* dimensions are zero answers `Skip` \
         to 0 x 0 and keeps drawing into everything else"
    );
}

#[test]
fn a_frame_with_area_arriving_after_a_zero_dimension_one_reconfigures_the_surface() {
    assert_eq!(
        [resize_action(ZERO_HEIGHT), resize_action(RESIZED)],
        [ResizeAction::Skip, ResizeAction::Reconfigure(RESIZED)],
        "restoring a minimised window resumes drawing. The sequence is the assertion: the policy \
         is a function of the size in front of it and of nothing it remembers, so a `minimised` \
         flag that was set and never cleared — the failure this scenario is named for — is a \
         latch this cannot hold"
    );
}

#[test]
fn a_lost_or_outdated_surface_is_reconfigured_and_the_next_frame_continues() {
    assert_eq!(
        [
            surface_error_action(SurfaceErrorKind::Lost),
            surface_error_action(SurfaceErrorKind::Outdated)
        ],
        [FrameAction::Reconfigure, FrameAction::Reconfigure],
        "a surface reported lost or outdated is routine on a real machine — a laptop GPU switch, \
         a compositor restart — and the answer to both is to reconfigure and carry on with the \
         next frame"
    );
}

#[test]
fn a_lost_device_ends_the_client_non_zero_rather_than_being_retried() {
    let action = surface_error_action(SurfaceErrorKind::DeviceLost);

    let code = match action {
        FrameAction::Fatal(reason) => exit_code(&Ending::Frame(reason)),
        FrameAction::Render | FrameAction::Reconfigure | FrameAction::Skip => 0,
    };
    assert_eq!(
        (action, code != 0),
        (FrameAction::Fatal(FatalReason::DeviceLost), true),
        "a lost device is not a frame to retry: nothing the client can do brings it back, so it \
         is reported and the process ends non-zero. Retrying would spin forever on a window that \
         will never draw again"
    );
}

/// `SurfaceFormatFacts` for each `(name, is_srgb)` pair.
fn offered_formats(formats: &[(&str, bool)]) -> Vec<SurfaceFormatFacts> {
    formats
        .iter()
        .map(|&(name, is_srgb)| SurfaceFormatFacts {
            name: name.to_owned(),
            is_srgb,
        })
        .collect()
}

/// The backends the client tried, as the verdict takes them.
fn backends() -> Vec<String> {
    BACKENDS.iter().map(|&name| name.to_owned()).collect()
}

/// An adapter that runs compute shaders and reads vertices from storage, and
/// cannot execute an indirect draw.
///
/// It satisfies two of the three requested capabilities, so the one it is
/// missing is neither the first nor the last of the list — a refusal that
/// reported a fixed position rather than the capability actually absent would
/// name the wrong one.
fn adapter_without_indirect_execution() -> AdapterFacts {
    AdapterFacts {
        name: "Example Integrated Graphics".to_owned(),
        backend: "gl".to_owned(),
        downlevel: vec![
            DownlevelRequirement::ComputeShaders,
            DownlevelRequirement::VertexStorage,
        ],
    }
}

/// What a format verdict says for a human, whichever way it went.
fn reported(verdict: &Result<usize, FormatError>) -> String {
    match verdict {
        Ok(index) => format!("the format at index {index} was configured"),
        Err(failure) => failure.to_string(),
    }
}

/// What a startup verdict says for a human, whichever way it went.
fn startup_report(verdict: &Result<usize, StartupError>) -> String {
    match verdict {
        Ok(index) => format!("the adapter at index {index} was accepted"),
        Err(failure) => failure.to_string(),
    }
}

/// The exit code a failed startup verdict ends the client with, or 0 when it
/// did not fail at all.
fn exit_code_of(verdict: &Result<usize, StartupError>) -> u8 {
    verdict
        .as_ref()
        .err()
        .map_or(0, |failure| exit_code(&Ending::Startup(failure.clone())))
}
