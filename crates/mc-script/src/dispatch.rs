//! Invoking the callbacks attached to a round's work, and reporting what
//! happened.

use std::collections::{BTreeMap, VecDeque};

use crate::fault::{FaultKind, ScriptFault, ScriptOrigin};
use crate::luau::handle::ScriptFunction;
use crate::luau::vm::{Outcome, Vm};
use crate::quarantine::Quarantine;
use crate::value::ScriptValue;
use crate::{Attachment, ChunkName, RoundIndex};

/// What one dispatch round did.
#[derive(Debug, Default)]
pub struct DispatchReport {
    /// Every attachment invoked, in the order it was entered.
    pub order: Vec<Attachment>,
    /// How many invocations this round performed.
    pub invocations: u32,
    /// What each attachment returned, for the ones that returned.
    ///
    /// An invocation that was stopped files nothing here: it did not return,
    /// and recording a value for it would report something that never happened.
    /// Keyed by attachment and holding the last result, because a round can
    /// invoke one attachment more than once.
    pub results: BTreeMap<Attachment, ScriptValue>,
    /// Everything that went wrong, in the order it went wrong.
    pub faults: Vec<ScriptFault>,
    /// Attachments this round stopped invoking.
    pub quarantined: Vec<Attachment>,
    /// Work carried into the next round.
    pub pending: u32,
}

/// What one round has in hand while it runs: where to invoke, what each
/// invocation may spend, and which round this is.
///
/// A record rather than three parameters threaded through, because three of
/// them are the same for every invocation in the round and only the attachment
/// changes.
struct Round<'a> {
    vm: &'a Vm,
    budget: u64,
    index: RoundIndex,
    /// How many consecutive faults stop an attachment being invoked.
    threshold: u32,
    /// How many invocations this round may perform.
    bound: u32,
    /// How many entries may be waiting at once.
    pending_bound: usize,
}

/// What one invocation handed back.
struct Returned {
    attachment: Attachment,
    value: ScriptValue,
}

/// What one invocation asked the host to run next.
struct Requested {
    requester: Attachment,
    wanted: Vec<Attachment>,
}

/// One piece of follow-up work, and who asked for it.
///
/// A struct rather than a bare pair, so a payload field stays additive whenever
/// one is earned. The payload itself is declined for now: the motivating case —
/// a job too large for one budget — is not closed by it, because the only
/// continuation script has is a closure upvalue and removing one motivating case
/// leaves that intact. **The first spec requiring cross-invocation continuation
/// must evaluate a queue payload before extending the callback return
/// convention.**
#[derive(Debug)]
struct PendingEntry {
    target: Attachment,
    /// Absent for work the engine seeded, present for work script asked for.
    /// It is what makes a cascade fault answerable: the blamed attachment is the
    /// one that asked, not the one that was queued.
    requested_by: Option<Attachment>,
}

/// Which callback belongs to which attachment, how often each has run, and
/// which have stopped running.
#[derive(Debug, Default)]
pub(crate) struct Registry {
    callbacks: BTreeMap<Attachment, ScriptFunction>,
    counts: BTreeMap<Attachment, u64>,
    quarantine: Quarantine,
    /// Work waiting to run, across rounds.
    pending: VecDeque<PendingEntry>,
    round: u32,
}

impl Registry {
    /// Registers `callback` against `attachment`, replacing whatever was there
    /// and lifting any quarantine.
    ///
    /// **Lifting is the point, not a convenience.** Reloading a mod somebody has
    /// just fixed *is* this operation, and a replace that left the attachment
    /// quarantined would silently fail at the one thing reloading a broken mod
    /// exists to do — the new callback would be registered and never invoked.
    ///
    /// The invocation count is deliberately left alone. It is cumulative
    /// telemetry about the **attachment**, so it resumes rather than resetting;
    /// the counter that resets is the consecutive-fault one, which is a
    /// different question with a different answer.
    pub(crate) fn attach(&mut self, attachment: Attachment, callback: ScriptFunction) {
        self.quarantine.lift(&attachment);
        self.callbacks.insert(attachment, callback);
    }

    /// Whether `attachment` has stopped being invoked.
    pub(crate) fn is_quarantined(&self, attachment: &Attachment) -> bool {
        self.quarantine.holds(attachment)
    }

    /// Lifts quarantine, leaving the callback in place, and reports whether the
    /// attachment was quarantined.
    pub(crate) fn release(&mut self, attachment: &Attachment) -> bool {
        self.quarantine.lift(attachment)
    }

    /// How often `attachment` has been invoked, over the whole life of the
    /// host.
    ///
    /// Cumulative telemetry about the attachment rather than about the callback
    /// currently registered against it, and a different counter from the
    /// consecutive-fault count. An attachment nobody registered has been
    /// invoked no times, which is an answer rather than an absence.
    pub(crate) fn invocation_count(&self, attachment: &Attachment) -> u64 {
        self.counts.get(attachment).copied().unwrap_or(0)
    }

    /// Drains one bounded round, seeded with `seed`.
    ///
    /// The seed is **appended** to whatever is still waiting rather than
    /// replacing it, so `dispatch(&[])` drains the residual of a previous round.
    /// Work that did not fit last time is not lost, and not re-requested either.
    pub(crate) fn run(
        &mut self,
        vm: &Vm,
        seed: &[Attachment],
        limits: &crate::HostLimits,
    ) -> DispatchReport {
        self.round = self.round.saturating_add(1);
        let round = Round {
            vm,
            budget: limits.call_and_loop_budget.get(),
            index: RoundIndex::new(self.round),
            threshold: limits.fault_threshold.get(),
            bound: limits.round_bound.get(),
            pending_bound: limits.pending_bound.get() as usize,
        };
        for target in seed {
            self.pending.push_back(PendingEntry {
                target: target.clone(),
                requested_by: None,
            });
        }

        let mut report = DispatchReport::default();
        while let Some(entry) = self.next_entry(report.invocations, round.bound) {
            self.invoke_into(&mut report, &entry.target, &round);
        }
        self.defer_what_is_left(&mut report, &round);
        report.pending = u32::try_from(self.pending.len()).unwrap_or(u32::MAX);
        report
    }

    /// The next entry to run, or nothing once the round has spent its bound.
    fn next_entry(&mut self, performed: u32, bound: u32) -> Option<PendingEntry> {
        if performed >= bound {
            return None;
        }
        self.pending.pop_front()
    }

    /// Reports work the round ended without reaching.
    ///
    /// **Reaching the bound is not itself the trigger** — a round whose last
    /// invocation empties the queue has lost nothing and says nothing. What is
    /// reported is that work is still waiting, and the attachment named is the
    /// **requester of the first entry that could not run**, not the entry
    /// itself: the entry is the victim and the requester is the answer to "whose
    /// cascade is this".
    ///
    /// It is reported eagerly, and it has to be. A cascade that will terminate
    /// in three more rounds and one that never will are indistinguishable at the
    /// end of this one — both hit the bound with work pending. The cost is
    /// recorded rather than hidden: a perfectly well-behaved terminating cascade
    /// emits one of these per round before it finishes. That is operator-facing
    /// noise the design accepts, and it is noise precisely because deferral
    /// loses nothing — which is exactly why it must not wear the same name as
    /// refusal, which does.
    fn defer_what_is_left(&mut self, report: &mut DispatchReport, round: &Round<'_>) {
        let Some(waiting) = self.pending.front() else {
            return;
        };
        let blamed = waiting.requested_by.clone();
        report.faults.push(cascade_fault(Cascade {
            round: round.index,
            kind: FaultKind::CascadeDeferred,
            blamed,
            refused_target: None,
            cause: DEFERRED,
        }));
    }

    /// Invokes one attachment, if anything is attached to it and it is still
    /// being invoked at all, and records what it did.
    fn invoke_into(
        &mut self,
        report: &mut DispatchReport,
        attachment: &Attachment,
        round: &Round<'_>,
    ) {
        let Some(callback) = self.callbacks.get(attachment).cloned() else {
            return;
        };
        // Skipped silently, and **without spending an invocation**. A host that
        // charged the round for work it declined to do would let a queue full of
        // quarantined targets crowd out everything still running.
        if self.quarantine.holds(attachment) {
            return;
        }
        let chunk = callback.origin_chunk().clone();
        self.enter(report, attachment);
        match round.vm.invoke(&callback, round.budget) {
            Outcome::Produced(value) => {
                let returned = Returned {
                    attachment: attachment.clone(),
                    value,
                };
                self.succeeded(report, returned, round);
            }
            Outcome::Failed { kind, line, cause } => {
                let raised = Raised {
                    attachment: attachment.clone(),
                    chunk,
                    round: round.index,
                    kind,
                    line,
                    cause,
                };
                self.record(report, raised, round.threshold);
            }
        }
    }

    /// Records that an invocation is about to happen.
    ///
    /// Separate from what the invocation *did*, because these three are true of
    /// every invocation whatever its outcome — including one that is about to be
    /// stopped, which still ran and still counts.
    fn enter(&mut self, report: &mut DispatchReport, attachment: &Attachment) {
        report.order.push(attachment.clone());
        report.invocations = report.invocations.saturating_add(1);
        let counted = self.counts.entry(attachment.clone()).or_insert(0);
        *counted = counted.saturating_add(1);
    }

    /// Records what an invocation returned and queues whatever it asked for
    /// next.
    ///
    /// The follow-up list is read out of the returned value **before** it is
    /// handed on, and read raw, because it is the one value in this design whose
    /// shape a mod chose.
    fn succeeded(&mut self, report: &mut DispatchReport, returned: Returned, round: &Round<'_>) {
        let wanted = round.vm.requested_follow_up(&returned.value);
        self.quarantine.succeeded(&returned.attachment);
        report
            .results
            .insert(returned.attachment.clone(), returned.value);
        let asked = Requested {
            requester: returned.attachment,
            wanted,
        };
        self.admit(report, asked, round);
    }

    /// Admits the follow-up work `requester` asked for, refusing what will not
    /// fit.
    ///
    /// **Why a second bound at all:** the round bound limits invocations and
    /// says nothing about queue length. A callback returning a fan-out of N
    /// grows the queue by N−1 per invocation, across unbounded rounds, and every
    /// entry is a host-side allocation — outside the memory cap, outside the
    /// allocator's ceiling, outside the budget, and outside quarantine, because
    /// the requester succeeds every time. Queueing only converts recursion into
    /// something countable once something counts it.
    ///
    /// Refused work is **named**, not dropped quietly: it will never run, and
    /// the consumer that makes that concrete is a neighbour notification, where
    /// a silent drop means a mod quietly does nothing for the rest of the
    /// session.
    fn admit(&mut self, report: &mut DispatchReport, asked: Requested, round: &Round<'_>) {
        for target in asked.wanted {
            let refused = self.admit_one(target, &asked.requester, round);
            report.faults.extend(refused);
        }
    }

    /// Admits one entry, or reports it refused because the queue is full.
    fn admit_one(
        &mut self,
        target: Attachment,
        requester: &Attachment,
        round: &Round<'_>,
    ) -> Option<ScriptFault> {
        if self.pending.len() < round.pending_bound {
            self.pending.push_back(PendingEntry {
                target,
                requested_by: Some(requester.clone()),
            });
            return None;
        }
        Some(cascade_fault(Cascade {
            round: round.index,
            kind: FaultKind::CascadeRefused,
            blamed: Some(requester.clone()),
            refused_target: Some(target),
            cause: REFUSED,
        }))
    }

    /// Records a fault against `attachment` and stops invoking it if this was
    /// the fault that reached the threshold.
    fn record(&mut self, report: &mut DispatchReport, raised: Raised, threshold: u32) {
        let (kind, attachment) = (raised.kind, raised.attachment.clone());
        report.faults.push(attribute(raised));
        if self.quarantine.faulted(&attachment, kind, threshold) {
            report.quarantined.push(attachment);
        }
    }
}

/// What a deferral says.
const DEFERRED: &str =
    "the round reached its invocation bound with follow-up work still waiting; it runs next round";

/// What a refusal says.
const REFUSED: &str = concat!(
    "the pending queue was full, so this follow-up work was not admitted ",
    "and will not run"
);

/// A fault about the round's admission control rather than about an invocation.
///
/// It names no chunk and no line: nothing failed inside script, so there is no
/// place in anybody's file to point at. The attachment it names is the requester
/// — for a refusal, alongside the target that was dropped, because for work that
/// will never run you have to know what was lost.
fn cascade_fault(cascade: Cascade) -> ScriptFault {
    ScriptFault {
        origin: ScriptOrigin {
            chunk: None,
            round: Some(cascade.round),
        },
        subject: cascade.blamed.as_ref().map(|blamed| blamed.subject.clone()),
        component: cascade.blamed.map(|blamed| blamed.component),
        kind: cascade.kind,
        line: None,
        refused_target: cascade.refused_target,
        cause: cascade.cause.to_owned(),
    }
}

/// What one cascade fault is about.
struct Cascade {
    round: RoundIndex,
    kind: FaultKind,
    /// The requester, which is who an operator has to look at.
    blamed: Option<Attachment>,
    /// What was dropped, for a refusal only.
    refused_target: Option<Attachment>,
    cause: &'static str,
}

/// One fault, before it is decided whom it belongs to.
struct Raised {
    attachment: Attachment,
    chunk: ChunkName,
    round: RoundIndex,
    kind: FaultKind,
    line: Option<u32>,
    cause: String,
}

/// Decides whether a fault names the attachment that was running.
///
/// It does for anything the invocation itself caused. It does **not** for
/// host-memory-pressure, because that invocation could have failed for a reason
/// that is not its own: naming a subject there files the blame against an author
/// who did nothing wrong, and an operator acting on it removes the wrong mod.
fn attribute(raised: Raised) -> ScriptFault {
    let blames_the_attachment = raised.kind != FaultKind::HostMemoryPressure;
    ScriptFault {
        origin: ScriptOrigin {
            chunk: blames_the_attachment.then_some(raised.chunk),
            round: Some(raised.round),
        },
        subject: blames_the_attachment.then_some(raised.attachment.subject),
        component: blames_the_attachment.then_some(raised.attachment.component),
        kind: raised.kind,
        line: raised.line,
        refused_target: None,
        cause: raised.cause,
    }
}
