//! Adapter preference, limit checking, and what a failed acquisition means.
//!
//! None of this needs a device. Selection is a pure ranking over an enumerated
//! candidate list, limit checking is arithmetic over two DTOs, and
//! classification is a decision over a `Result` and two booleans — which is the
//! only reason a two-adapter machine and an environment variable a test may not
//! set are both unnecessary here.

use super::{
    AcquireError, AcquisitionVerdict, AdapterDescription, AdapterKind, AdapterLimits, Backend,
    OptIns, classify_acquisition, select_preferred, unsatisfied_limit,
};

/// The error type these tests propagate with `?`.
type TestResult = Result<(), Box<dyn std::error::Error>>;

const HARDWARE: &str = "NVIDIA GeForce RTX 4090";
const SOFTWARE: &str = "Microsoft Basic Render Driver";
const UNKNOWN_KIND: &str = "Intel(R) UHD Graphics 770";

/// What this harness asks of a device: one 2D colour target, comfortably inside
/// wgpu's downlevel defaults.
const REQUIRED_DIMENSION: u32 = 8192;

fn adapter(name: &str, kind: AdapterKind) -> AdapterDescription {
    AdapterDescription {
        name: name.to_owned(),
        backend: Backend::Vulkan,
        kind,
        driver_description: "unknown".to_owned(),
    }
}

fn limits(max_texture_dimension_2d: u32) -> AdapterLimits {
    AdapterLimits {
        max_texture_dimension_2d,
    }
}

fn allowing_no_gpu() -> OptIns {
    OptIns {
        allow_no_gpu: true,
        update_goldens: false,
    }
}

#[test]
fn a_hardware_adapter_is_chosen_over_a_software_one() {
    // The software adapter is enumerated first on purpose: preference has to
    // beat enumeration order for this to pass.
    let candidates = [
        adapter(SOFTWARE, AdapterKind::Cpu),
        adapter(HARDWARE, AdapterKind::Discrete),
    ];

    assert_eq!(
        select_preferred(&candidates),
        Some(1),
        "a discrete adapter outranks a software rasteriser wherever it appears \
         in the list"
    );
}

#[test]
fn an_adapter_of_unknown_kind_outranks_a_software_rasteriser() {
    // `Cpu` is the only kind that definitively means a software rasteriser;
    // `Other` is what real hardware reports on GL/ANGLE and some Vulkan
    // drivers. Ranking `Other` last would mint goldens from lavapipe on such a
    // machine, and nobody would notice until cross-adapter drift appeared.
    let candidates = [
        adapter(SOFTWARE, AdapterKind::Cpu),
        adapter(UNKNOWN_KIND, AdapterKind::Other),
    ];

    assert_eq!(
        select_preferred(&candidates),
        Some(1),
        "an unknown kind is hardware until proven otherwise; `Cpu` is not"
    );
}

#[test]
fn an_empty_candidate_list_selects_nothing() {
    let candidates: [AdapterDescription; 0] = [];

    assert_eq!(
        select_preferred(&candidates),
        None,
        "there is no preferred adapter among none"
    );
}

#[test]
fn a_limit_the_adapter_exactly_meets_is_not_reported() {
    let required = limits(REQUIRED_DIMENSION);
    let available = limits(REQUIRED_DIMENSION);

    assert!(
        unsatisfied_limit(&required, &available).is_none(),
        "a limit met exactly is satisfied; the bound is what the adapter offers"
    );
}

#[test]
fn a_limit_beyond_the_adapter_is_reported_with_both_numbers() -> TestResult {
    let required = limits(u32::MAX);
    let available = limits(REQUIRED_DIMENSION);

    let unsatisfied = unsatisfied_limit(&required, &available)
        .ok_or("a requirement no adapter can meet must be reported")?;

    assert_eq!(
        (unsatisfied.required, unsatisfied.available),
        (u32::MAX, REQUIRED_DIMENSION),
        "the report carries what was asked for and what the adapter offers, so \
         the rejection can say why"
    );
    Ok(())
}

#[test]
fn a_failed_acquisition_without_the_opt_in_is_an_error_rather_than_a_skip() {
    let outcome = Err(AcquireError::NoAdapter {
        tried: vec![Backend::Vulkan, Backend::Dx12],
    });

    let verdict = classify_acquisition(outcome, &OptIns::default());

    assert!(
        matches!(verdict, AcquisitionVerdict::Fail(_)),
        "a silent skip would let the gate go green while verifying nothing, \
         got {verdict:?}"
    );
}

#[test]
fn a_failed_acquisition_with_the_opt_in_skips_with_a_warning_naming_the_variable() -> TestResult {
    let outcome = Err(AcquireError::NoAdapter {
        tried: vec![Backend::Vulkan],
    });

    let verdict = classify_acquisition(outcome, &allowing_no_gpu());

    let AcquisitionVerdict::Skip(notice) = verdict else {
        return Err(format!("the opt-in downgrades the failure to a skip, got {verdict:?}").into());
    };
    assert!(
        notice.message().contains("MYCRAFT_ALLOW_NO_GPU"),
        "a skip has to announce which opt-in produced it, got `{}`",
        notice.message()
    );
    Ok(())
}

#[test]
fn a_successful_acquisition_with_the_opt_in_set_still_runs_the_capture() -> TestResult {
    let outcome = Ok(adapter(HARDWARE, AdapterKind::Discrete));

    let verdict = classify_acquisition(outcome, &allowing_no_gpu());

    let AcquisitionVerdict::Use(selected) = verdict else {
        return Err(format!("an acquired adapter is used, not skipped, got {verdict:?}").into());
    };
    assert_eq!(
        selected.name, HARDWARE,
        "the opt-in permits a skip; it never causes one"
    );
    Ok(())
}
