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
    fn for_failure(cause: &AcquireError) -> Self {
        Self {
            message: format!("capture skipped because {ALLOW_NO_GPU} is set: {cause}"),
        }
    }
}

/// What to do about an attempt to acquire an adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), expect(dead_code))]
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
/// Its production caller is the device request in the GPU layer, which does not
/// exist yet. `expect` rather than `allow`, so the annotation becomes a warning
/// the moment that caller arrives.
#[cfg_attr(not(test), expect(dead_code))]
fn unsatisfied_limit(
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
/// Its production caller is the GPU layer's acquisition path, which does not
/// exist yet; see [`unsatisfied_limit`] on the annotation.
#[cfg_attr(not(test), expect(dead_code))]
fn classify_acquisition(
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
#[path = "selection_test.rs"]
mod tests;
