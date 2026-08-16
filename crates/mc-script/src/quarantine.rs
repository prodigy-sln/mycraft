//! Which attachments have stopped being invoked, and how they got there.
//!
//! The budget bounds what one invocation costs. This bounds how often a broken
//! one repeats: three **consecutive** faults and the attachment stops being
//! invoked at all.
//!
//! # Consecutive, and reset by a success
//!
//! A callback that alternates failing and succeeding is never quarantined, and
//! that is accepted rather than overlooked — its cost is already bounded, one
//! invocation at a time, by the budget and the memory cap. Quarantine exists for
//! the callback that is simply broken, not for the one that is unreliable.
//!
//! # Per attachment, never per component
//!
//! The unit is the `(subject, component)` pair. A component broken on one
//! subject keeps being invoked on every other subject it is attached to, and
//! every other component on the broken subject keeps being invoked. Containment
//! costs three faults per subject for a component broken everywhere, and each of
//! those faults is itself bounded.
//!
//! # What does not count
//!
//! A fault raised while the host itself is short of memory does not count, and
//! neither does a fault about the round's admission control. Both are conditions
//! of the host rather than outcomes of the invocation, and counting them
//! quarantines whichever mod happened to be running.

use std::collections::{BTreeMap, BTreeSet};

use crate::Attachment;
use crate::fault::FaultKind;

/// The consecutive-fault count for each attachment, and the set that has
/// stopped being invoked.
#[derive(Debug, Default)]
pub(crate) struct Quarantine {
    consecutive: BTreeMap<Attachment, u32>,
    stopped: BTreeSet<Attachment>,
}

impl Quarantine {
    /// Whether `attachment` has stopped being invoked.
    pub(crate) fn holds(&self, attachment: &Attachment) -> bool {
        self.stopped.contains(attachment)
    }

    /// Records a successful invocation, which resets the count.
    pub(crate) fn succeeded(&mut self, attachment: &Attachment) {
        self.consecutive.remove(attachment);
    }

    /// Records a faulting invocation and reports whether it was the one that
    /// stopped the attachment.
    ///
    /// Only a fault the invocation itself caused is counted. A fault carrying
    /// the host's own condition leaves the count exactly as it was — it neither
    /// advances toward quarantine nor forgives what came before, because it says
    /// nothing about the callback either way.
    pub(crate) fn faulted(
        &mut self,
        attachment: &Attachment,
        kind: FaultKind,
        threshold: u32,
    ) -> bool {
        if !counts_against_the_attachment(kind) {
            return false;
        }
        let seen = self.consecutive.entry(attachment.clone()).or_insert(0);
        *seen = seen.saturating_add(1);
        if *seen >= threshold && self.stopped.insert(attachment.clone()) {
            return true;
        }
        false
    }

    /// Lifts quarantine and forgets what led to it, reporting whether the
    /// attachment was quarantined.
    ///
    /// The count is cleared alongside, because leaving it standing would
    /// re-quarantine on the next single fault — which would make releasing a
    /// callback somebody has just fixed a one-invocation reprieve rather than a
    /// fresh start.
    pub(crate) fn lift(&mut self, attachment: &Attachment) -> bool {
        self.consecutive.remove(attachment);
        self.stopped.remove(attachment)
    }
}

/// Whether a fault of this kind says anything about the callback that produced
/// it.
///
/// Cascade faults are about the round's admission control and the invocation
/// they name completed and returned; host-memory-pressure faults are about the
/// state the whole host is in. Counting either quarantines the mod that happened
/// to be running when the host had a problem of its own.
fn counts_against_the_attachment(kind: FaultKind) -> bool {
    match kind {
        FaultKind::BudgetExhausted
        | FaultKind::Allocation
        | FaultKind::ScriptError
        | FaultKind::Compilation => true,
        FaultKind::CascadeRefused | FaultKind::CascadeDeferred | FaultKind::HostMemoryPressure => {
            false
        }
    }
}
