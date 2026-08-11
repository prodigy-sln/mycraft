//! Acquiring a device with no window, and the two ways that can fail.
//!
//! Both failures are provoked against real hardware rather than staged with a
//! hand-built error: a formatted error proves the formatting, not the trigger,
//! and "without panicking and without creating a window" is a claim about the
//! real path or about nothing. Every test here returns its failure instead of
//! panicking, so a panic anywhere in acquisition is a red test rather than a
//! passing one.

mod scene;

use mc_testkit::frame::gpu::{AcquireOptions, CaptureContext};
use mc_testkit::frame::{AcquireError, Backend, OptIns, wgpu};
use scene::TestResult;

/// A limit no adapter in existence offers, which is what makes wgpu's own
/// validation — not this crate's — reject the device request.
const IMPOSSIBLE_TEXTURE_DIMENSION: u32 = u32::MAX;

#[test]
fn an_acquired_device_reports_the_adapter_it_selected() -> TestResult {
    let context = scene::device_context()?;

    let provenance = context.provenance();

    assert!(
        !provenance.name.is_empty(),
        "the selected adapter must be named, got an empty name"
    );
    assert!(
        matches!(
            provenance.backend,
            Backend::Vulkan | Backend::Dx12 | Backend::Metal | Backend::Gl
        ),
        "a capture on this machine runs on a native backend, got {:?}",
        provenance.backend
    );
    Ok(())
}

#[test]
fn a_backend_with_no_adapters_fails_naming_every_backend_it_tried() -> TestResult {
    // Real zero-adapter enumeration on a machine that has a GPU: the browser
    // backend exists nowhere on native. `Backends::empty()` would enumerate
    // nothing either, but would leave nothing to name, making the assertion
    // degenerate.
    let options = AcquireOptions {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..AcquireOptions::default()
    };

    let error = CaptureContext::acquire(&OptIns::default(), &options)
        .err()
        .ok_or("no adapter answers on the browser backend on a native machine")?;

    let AcquireError::NoAdapter { tried } = &error else {
        return Err(format!("expected a no-adapter failure, got {error:?}").into());
    };
    assert_eq!(
        tried.as_slice(),
        [Backend::BrowserWebGpu].as_slice(),
        "the failure must name every backend it tried"
    );
    Ok(())
}

#[test]
fn a_device_request_past_the_adapters_limits_names_the_adapter_and_the_requirement() -> TestResult {
    let options = AcquireOptions {
        required_limits: wgpu::Limits {
            max_texture_dimension_2d: IMPOSSIBLE_TEXTURE_DIMENSION,
            ..wgpu::Limits::downlevel_defaults()
        },
        ..AcquireOptions::default()
    };

    let error = CaptureContext::acquire(&OptIns::default(), &options)
        .err()
        .ok_or("no adapter can offer a texture dimension of u32::MAX")?;

    let AcquireError::DeviceRejected {
        adapter,
        requirement,
    } = &error
    else {
        return Err(format!("expected a rejected device request, got {error:?}").into());
    };
    assert!(
        !adapter.is_empty(),
        "the failure must name the adapter that rejected the request"
    );
    assert_eq!(
        (requirement.limit, requirement.required),
        ("max_texture_dimension_2d", IMPOSSIBLE_TEXTURE_DIMENSION),
        "the failure must name the requirement the adapter could not meet"
    );
    Ok(())
}
