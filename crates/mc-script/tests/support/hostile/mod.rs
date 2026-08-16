//! The hostile-mod harness: six named cases, the evidence each needs the host
//! to have produced, and a verdict with three values.
//!
//! # Nothing here decides anything the host decides
//!
//! This is the harness's one design constraint and the reason a sibling text
//! guard watches this directory. Every case runs against a host built with **no
//! configuration at all**, so the budget that stops the infinite loop, the cap
//! that stops the bomb and the two bounds that stop the cascade are the ones the
//! host ships. The names the escape case probes come from
//! `ScriptHost::DENIED_GLOBALS`, generated into the script rather than
//! transcribed. The evidence each case requires is the host's own `FaultKind`.
//!
//! A harness holding any of those itself would report all six contained on the
//! day the host's enforcement was deleted — and every scenario about the harness
//! runs *through* the harness, so nothing else in this suite could see it.
//! `crates/mc-script/tests/harness_boundaries.rs` is what keeps it that way.
//!
//! # What the workloads are sized against
//!
//! Two of the six could take the machine down instead of failing, so neither is
//! written as an unbounded loop over an allocation: the bomb asks for a fixed
//! multiple of the host's own cap, and the cascade stops requesting after a
//! round's worth of invocations and is then drained. Against a host that
//! enforces nothing they return, visibly and quickly, rather than running until
//! something outside this suite intervenes. The infinite loop is the one case
//! that genuinely cannot be bounded from this side — its whole claim is that the
//! host stops it — which is why the scenario driving it is bounded by the test
//! runner instead.
//!
//! # The verdict is three-valued, and that is the point
//!
//! `sandbox-escape` and `hostile-index` are contained *by producing no fault*,
//! so "did anything fault?" is not a verdict this harness can give. And a case
//! whose script never compiled produces no fault either — which is why a
//! failure to compile is named rather than folded into either of the other two.
//! A harness that stopped running must never read like one that ran clean.
#![allow(dead_code)]

mod exercise;
mod scripts;

use mc_script::{FaultKind, ScriptHost};

pub use exercise::{EscapeProbe, probe_denied_globals};

use exercise::Observed;

/// What a hostile case needs the host to have done for it to count as
/// contained.
///
/// Declared per case rather than inferred, because the four fault kinds are not
/// interchangeable — a memory bomb stopped for exhausting its tick budget has
/// measured the wrong mechanism — and because two of the six are contained by
/// nothing being reported at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainmentEvidence {
    /// The host reported a fault of exactly this kind.
    FaultReported(FaultKind),
    /// Every global the host declares denied was unavailable to the script that
    /// went looking for it.
    EveryDeniedGlobalUnavailable,
    /// The `__index` a mod hung on a table it handed the host never ran.
    MetamethodNotInvoked,
}

/// What running one case amounts to.
///
/// Three values and not two. `Uncontained` is a case that ran and did not
/// produce its evidence; `NotExercised` is a case that never ran at all. Folding
/// the second into `Contained` would let a typo retire a hostile shape from the
/// suite in silence, and folding it into `Uncontained` would report a host
/// defect where there is a script defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseOutcome {
    Contained,
    Uncontained,
    NotExercised,
}

/// One hostile mod: what it is called, and what containing it requires.
///
/// The script itself is not public. What a caller may do is ask for the six, or
/// supply a case of its own through [`HostileCase::from_source`] — which is how
/// the two outcomes the six cannot reach against a working host are driven.
#[derive(Debug, Clone)]
pub struct HostileCase {
    /// What this shape of bad mod is called.
    pub name: &'static str,
    /// What the host must have done for it to be contained.
    pub requires: ContainmentEvidence,
    shape: Shape,
}

/// Which hostile script a case runs, and therefore how it is driven.
///
/// The shape decides the exercise rather than the evidence deciding it, because
/// two cases needing the same evidence can still need different driving: a
/// runaway cascade has to be drained afterwards and a plain fault does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    NeverReturns,
    AllocatesPastTheCap,
    ProbesEveryDeniedGlobal,
    RaisesAnError,
    RequestsFollowUpForever,
    SuppliesATableThatCounts,
    /// A script the caller supplied, driven as an ordinary callback.
    Supplied(&'static str),
}

/// What one case did, under the name it did it under.
///
/// The name travels with the outcome because both scenarios about a failed case
/// require it to be named: a run reporting *that* something was uncontained
/// without saying *which* shape leaves a reader no better off than a run that
/// said nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReport {
    pub name: &'static str,
    pub outcome: CaseOutcome,
}

impl HostileCase {
    /// A case whose script the caller supplies.
    ///
    /// The six below cannot produce `Uncontained` or `NotExercised` against a
    /// working host — that is what containing them means — so the only way to
    /// exercise those two verdicts is to hand the harness a case that fails on
    /// purpose. It is judged by exactly the code path the six go through.
    pub fn from_source(
        name: &'static str,
        requires: ContainmentEvidence,
        source: &'static str,
    ) -> Self {
        Self {
            name,
            requires,
            shape: Shape::Supplied(source),
        }
    }

    pub(crate) fn shape(&self) -> Shape {
        self.shape
    }
}

/// The six shapes a bad mod takes, each named for what it does rather than for
/// the mechanism that stops it.
pub fn hostile_cases() -> [HostileCase; 6] {
    [
        faulting("infinite-loop", FaultKind::BudgetExhausted, NEVER_RETURNS),
        faulting("memory-bomb", FaultKind::Allocation, ALLOCATES),
        a_mod_that_goes_looking_for_what_was_taken_away(),
        faulting("faulting-callback", FaultKind::ScriptError, RAISES),
        faulting("runaway-cascade", FaultKind::CascadeRefused, CASCADES),
        a_mod_that_hangs_a_metatable_on_what_it_hands_back(),
    ]
}

/// Spelled once here so the six above fit on one line apiece and read as the
/// list they are.
const NEVER_RETURNS: Shape = Shape::NeverReturns;
const ALLOCATES: Shape = Shape::AllocatesPastTheCap;
const RAISES: Shape = Shape::RaisesAnError;
const CASCADES: Shape = Shape::RequestsFollowUpForever;

/// A case contained by the host reporting a fault of one particular kind.
fn faulting(name: &'static str, kind: FaultKind, shape: Shape) -> HostileCase {
    HostileCase {
        name,
        requires: ContainmentEvidence::FaultReported(kind),
        shape,
    }
}

/// The escape case, which is contained by every name it looks for being gone.
fn a_mod_that_goes_looking_for_what_was_taken_away() -> HostileCase {
    HostileCase {
        name: "sandbox-escape",
        requires: ContainmentEvidence::EveryDeniedGlobalUnavailable,
        shape: Shape::ProbesEveryDeniedGlobal,
    }
}

/// The hostile-index case, which is contained by its metamethod never running.
fn a_mod_that_hangs_a_metatable_on_what_it_hands_back() -> HostileCase {
    HostileCase {
        name: "hostile-index",
        requires: ContainmentEvidence::MetamethodNotInvoked,
        shape: Shape::SuppliesATableThatCounts,
    }
}

/// Runs one hostile case against `host` and reports what became of it.
///
/// The host is left running: every case here is one a server has to survive, and
/// a harness that needed a fresh host per case would be unable to say whether
/// surviving one leaves the next one possible.
pub fn run(host: &mut ScriptHost, case: &HostileCase) -> CaseReport {
    CaseReport {
        name: case.name,
        outcome: judge(case.requires, &exercise::observe(host, case)),
    }
}

/// Whether what was observed is the evidence the case declared.
///
/// The compile failure is decided first because it explains away everything
/// after it: a script that never ran produces no fault, reaches no global and
/// runs no metamethod, which is indistinguishable from perfect containment on
/// every other axis.
///
/// Anything else — an observation that cannot answer the evidence declared, a
/// fault of the wrong kind, a metamethod that ran — is `Uncontained`. The case
/// was exercised and what the host owed did not appear.
fn judge(requires: ContainmentEvidence, observed: &Observed) -> CaseOutcome {
    match (requires, observed) {
        (_, Observed::DidNotCompile) => CaseOutcome::NotExercised,
        (ContainmentEvidence::FaultReported(kind), Observed::Faults(reported))
            if reported.contains(&kind) =>
        {
            CaseOutcome::Contained
        }
        (ContainmentEvidence::EveryDeniedGlobalUnavailable, Observed::Escape(probed))
            if probed.every_denied_global_is_gone() =>
        {
            CaseOutcome::Contained
        }
        (ContainmentEvidence::MetamethodNotInvoked, Observed::Metamethod { invoked: false }) => {
            CaseOutcome::Contained
        }
        _ => CaseOutcome::Uncontained,
    }
}
