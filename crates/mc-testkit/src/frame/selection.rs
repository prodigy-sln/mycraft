//! Which adapter to prefer, whether it can do what is asked of it, and what a
//! failed acquisition means.
//!
//! Every decision here is a pure function over plain values. That is not tidiness:
//! ranking two adapters would otherwise need a two-adapter machine, and deciding
//! what a failure means would otherwise need an environment variable a test may
//! not set. Both are asserted here instead, on this machine, with no device in
//! the process.

use thiserror::Error;

use super::optins::{ALLOW_NO_GPU, OptIns};
use super::report::Backend;

/// What kind of device an adapter is, as the driver reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Discrete,
    Integrated,
    Virtual,
    Cpu,
    Other,
}

/// One enumerated adapter, reduced to the facts this crate decides on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterDescription {
    pub name: String,
    pub backend: Backend,
    pub kind: AdapterKind,
    pub driver_description: String,
}

/// The device capabilities this harness depends on.
///
/// One 2D colour target is all it renders, so one limit is all it asks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterLimits {
    pub max_texture_dimension_2d: u32,
}

/// A capability the harness needs and the adapter does not offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsatisfiedLimit {
    pub limit: &'static str,
    pub required: u32,
    pub available: u32,
}

/// Why no usable device could be obtained.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AcquireError {
    #[error("no adapter could be acquired on any of the backends tried: {tried:?}")]
    NoAdapter { tried: Vec<Backend> },
    #[error(
        "the adapter `{adapter}` rejected the device request: it offers \
         {limit} {available}, and {required} was required",
        limit = requirement.limit,
        available = requirement.available,
        required = requirement.required
    )]
    DeviceRejected {
        adapter: String,
        requirement: UnsatisfiedLimit,
    },
    /// The adapter refused the device for something other than the one limit
    /// this harness asks about.
    ///
    /// [`UnsatisfiedLimit`] can only name a capability the harness models, and
    /// it models exactly one — the 2D texture dimension of its single colour
    /// target. A refusal for any other reason has no requirement to name, so it
    /// carries the driver's own words instead of a fabricated limit.
    #[error("the adapter `{adapter}` refused a device: {cause}")]
    DeviceUnavailable { adapter: String, cause: String },
}

/// The announcement that a capture was skipped rather than run.
///
/// It carries the name of the opt-in that permitted the skip, because a skip
/// nobody can attribute is indistinguishable from a test that quietly verified
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipNotice {
    message: String,
}

impl SkipNotice {
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The notice for an acquisition that failed while the opt-in was set.
    ///
    /// The `expect` here and on the three items below states the seam as a
    /// compile condition: these decisions exist to be called by the GPU layer,
    /// so with the feature off they have no caller outside a test. `expect`
    /// rather than `allow`, so a caller appearing in the core turns the
    /// annotation into a warning instead of leaving it to rot.
    #[cfg_attr(all(not(test), not(feature = "gpu")), expect(dead_code))]
    fn for_failure(cause: &AcquireError) -> Self {
        Self {
            message: format!("capture skipped because {ALLOW_NO_GPU} is set: {cause}"),
        }
    }
}

/// What to do about an attempt to acquire an adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(all(not(test), not(feature = "gpu")), expect(dead_code))]
pub(crate) enum AcquisitionVerdict {
    /// Run the capture on this adapter.
    Use(AdapterDescription),
    /// Report the capture as skipped, saying why.
    Skip(SkipNotice),
    /// Fail, because a skip was never asked for.
    Fail(AcquireError),
}

/// Picks the adapter to render on, or `None` when none was enumerated.
///
/// Ranking is `Discrete > Integrated > Virtual > Other > Cpu`, ties broken by
/// enumeration order. **`Cpu` ranks last, below `Other`**: `Cpu` is the only
/// kind that definitively means a software rasteriser, while `Other` is what
/// real hardware reports on GL/ANGLE and on some Vulkan drivers. Ranking
/// `Other` below `Cpu` would silently mint goldens from a software rasteriser
/// on such a machine, and nobody would notice until cross-adapter drift
/// appeared.
#[must_use]
pub fn select_preferred(candidates: &[AdapterDescription]) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .min_by_key(|(_, candidate)| preference(candidate.kind))
        .map(|(index, _)| index)
}

/// How much this harness would rather not use an adapter. Lower is better.
const fn preference(kind: AdapterKind) -> u8 {
    match kind {
        AdapterKind::Discrete => 0,
        AdapterKind::Integrated => 1,
        AdapterKind::Virtual => 2,
        AdapterKind::Other => 3,
        AdapterKind::Cpu => 4,
    }
}

/// The first capability `available` does not cover, or `None` when it covers
/// everything required.
///
/// A limit met exactly is satisfied: the bound is what the adapter offers.
///
/// `pub(crate)` rather than public: the harness's public surface is capture,
/// compare and verify, and this is an error-detail helper the GPU layer calls
/// when a device request comes back rejected.
#[cfg_attr(all(not(test), not(feature = "gpu")), expect(dead_code))]
pub(crate) fn unsatisfied_limit(
    required: &AdapterLimits,
    available: &AdapterLimits,
) -> Option<UnsatisfiedLimit> {
    (required.max_texture_dimension_2d > available.max_texture_dimension_2d).then_some(
        UnsatisfiedLimit {
            limit: "max_texture_dimension_2d",
            required: required.max_texture_dimension_2d,
            available: available.max_texture_dimension_2d,
        },
    )
}

/// Decides what an acquisition attempt means, given what the caller opted in to.
///
/// A failure is a failure by default. Turning it into a skip takes an explicit
/// opt-in, and the resulting notice names that opt-in — a silent skip would let
/// the gate go green while verifying nothing.
///
/// `pub(crate)` rather than public, for the reason given on
/// [`unsatisfied_limit`]: this is internal policy, and the GPU layer's
/// acquisition path is the only caller it will ever have.
#[cfg_attr(all(not(test), not(feature = "gpu")), expect(dead_code))]
pub(crate) fn classify_acquisition(
    outcome: Result<AdapterDescription, AcquireError>,
    opt_ins: &OptIns,
) -> AcquisitionVerdict {
    match outcome {
        Ok(adapter) => AcquisitionVerdict::Use(adapter),
        Err(cause) if opt_ins.allow_no_gpu => {
            AcquisitionVerdict::Skip(SkipNotice::for_failure(&cause))
        }
        Err(cause) => AcquisitionVerdict::Fail(cause),
    }
}

#[cfg(test)]
mod tests {
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
    fn a_failed_acquisition_with_the_opt_in_skips_with_a_warning_naming_the_variable() -> TestResult
    {
        let outcome = Err(AcquireError::NoAdapter {
            tried: vec![Backend::Vulkan],
        });

        let verdict = classify_acquisition(outcome, &allowing_no_gpu());

        let AcquisitionVerdict::Skip(notice) = verdict else {
            return Err(
                format!("the opt-in downgrades the failure to a skip, got {verdict:?}").into(),
            );
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
            return Err(
                format!("an acquired adapter is used, not skipped, got {verdict:?}").into(),
            );
        };
        assert_eq!(
            selected.name, HARDWARE,
            "the opt-in permits a skip; it never causes one"
        );
        Ok(())
    }
}
