//! What the host does when there is no room left in the queue.
//!
//! # Why a second bound exists at all
//!
//! The round bound limits invocations per round and says nothing whatever about
//! queue length. A callback returning a fan-out of N identities grows the queue
//! by N−1 for every invocation, sixty-four invocations a round, across as many
//! rounds as the caller runs — and every one of those entries is a host-side
//! allocation. It sits outside the script memory cap, outside the allocator's
//! ceiling, outside the call-and-loop budget, and outside quarantine, because
//! the attachment doing it succeeds every single time. Converting recursion
//! depth into queue length is only an improvement once something bounds the
//! queue.
//!
//! # Refusal is not deferral, and the difference is the whole point
//!
//! Deferred work is progressing: it runs next round, nothing is missing, and the
//! fault names only who asked. Refused work is **dropped and will never run**,
//! so the fault names the requester *and* what was refused — for something that
//! is gone you have to know what it was. The consumer that makes this concrete
//! is a neighbour notification: a full queue silently drops one, a furnace never
//! learns its neighbour changed, and the failure an operator sees is content
//! that quietly does nothing rather than a server that is slow. One fault kind
//! for both would leave an operator unable to tell "wait" from "something is
//! lost".
//!
//! The refused attachment is a typed field on the fault, never text spliced into
//! its cause. A structured fact buried in a string formatted by a pre-1.0
//! dependency leaves substring matching as the only assertion available.
//!
//! # Configured values
//!
//! A pending bound of eight, which is the figure the scenario names and the only
//! place in this specification where the queue is meant to fill; a round bound
//! of 64, high enough that everything admitted also runs, so a missing
//! invocation means refusal rather than a round that ended early. A
//! call-and-loop budget of 10,000. The closing witness lowers the pending bound
//! to one and the fault threshold to one, because its question is what a refusal
//! does to the fault total of the attachment it names.

use std::error::Error;
use std::num::{NonZeroU32, NonZeroU64};

use mc_script::{
    Attachment, ComponentName, DispatchReport, FaultKind, HostLimits, ScriptFault, ScriptFunction,
    ScriptHost, ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// What one invocation may spend.
const BUDGET: u64 = 10_000;

/// How many invocations one round may perform. Comfortably above everything
/// this file admits, so nothing here ends a round early.
const ROUND_BOUND: u32 = 64;

/// How many consecutive faults stop an attachment being invoked.
const FAULT_THRESHOLD: u32 = 3;

/// How many entries the queue may hold, and how many the callback asks for.
///
/// Everything below is derived from this pair and from nothing the host reports:
/// the queue admits as many as it holds, everything past that — twelve minus
/// eight of them — is refused, and the round performs one invocation for the
/// requester plus one for each admitted entry.
const PENDING_BOUND: u32 = 8;
const REQUESTED: u32 = 12;
const ADMITTED: u32 = PENDING_BOUND;
const INVOCATIONS: u32 = 1 + ADMITTED;

/// The pending bound and threshold the closing witness runs under: room for one
/// entry, and one fault enough to quarantine.
const WITNESS_PENDING_BOUND: u32 = 1;
const WITNESS_THRESHOLD: u32 = 1;

/// A callback that asks for nothing and returns quietly.
const ALWAYS_COLLECTS: &str = "return function()\n\treturn 'collected'\nend\n";

fn attachment(subject: &str, component: &str) -> Attachment {
    Attachment {
        subject: SubjectName::new(subject),
        component: ComponentName::new(component),
    }
}

/// The collectors the requester asks for, named so their order is readable in a
/// failure and their identities are distinct.
fn collectors(count: u32) -> Vec<Attachment> {
    (1..=count)
        .map(|index| attachment("ash-bin", &format!("collect-{index:02}")))
        .collect()
}

/// How an attachment reads when a test names one.
fn named(attachment: &Attachment) -> String {
    format!(
        "{}/{}",
        attachment.subject.as_str(),
        attachment.component.as_str()
    )
}

fn names(order: &[Attachment]) -> Vec<String> {
    order.iter().map(named).collect()
}

/// One reported fault, as the three facts a cascade fault is judged on.
#[derive(Debug, PartialEq, Eq)]
struct Reported {
    kind: FaultKind,
    blamed: Option<String>,
    refused: Option<String>,
}

fn reported(faults: &[ScriptFault]) -> Vec<Reported> {
    faults
        .iter()
        .map(|fault| Reported {
            kind: fault.kind,
            blamed: match (&fault.subject, &fault.component) {
                (Some(subject), Some(component)) => {
                    Some(format!("{}/{}", subject.as_str(), component.as_str()))
                }
                _ => None,
            },
            refused: fault.refused_target.as_ref().map(named),
        })
        .collect()
}

/// A refusal naming who asked and what was turned away.
fn refusal(requester: &Attachment, target: &Attachment) -> Reported {
    Reported {
        kind: FaultKind::CascadeRefused,
        blamed: Some(named(requester)),
        refused: Some(named(target)),
    }
}

/// The table a callback returns to request follow-up work.
fn follow_up_of(targets: &[Attachment]) -> String {
    let entries: Vec<String> = targets
        .iter()
        .map(|target| {
            format!(
                "{{ subject = '{}', component = '{}' }}",
                target.subject.as_str(),
                target.component.as_str()
            )
        })
        .collect();
    format!("{{ follow_up = {{ {} }} }}", entries.join(", "))
}

/// A callback that asks for `targets` every time it runs.
fn chunk_requesting(targets: &[Attachment]) -> String {
    let request = follow_up_of(targets);
    format!("return function()\n\treturn {request}\nend\n")
}

fn host_with(pending_bound: u32, threshold: u32) -> Result<ScriptHost, Box<dyn Error>> {
    let limits = HostLimits {
        call_and_loop_budget: NonZeroU64::new(BUDGET).ok_or("the budget must not be zero")?,
        fault_threshold: NonZeroU32::new(threshold)
            .ok_or("the fault threshold must not be zero")?,
        round_bound: NonZeroU32::new(ROUND_BOUND).ok_or("the round bound must not be zero")?,
        pending_bound: NonZeroU32::new(pending_bound)
            .ok_or("the pending bound must not be zero")?,
        ..HostLimits::default()
    };
    ScriptHost::with_limits(limits)
        .map_err(|error| format!("the host refused to start: {error}").into())
}

fn callback_from(
    host: &mut ScriptHost,
    chunk: &str,
    source: &str,
) -> Result<ScriptFunction, Box<dyn Error>> {
    match host.evaluate(chunk, source) {
        Ok(ScriptValue::Function(callback)) => Ok(callback),
        Ok(other) => {
            Err(format!("`{chunk}` was written to return a function, not {other:?}").into())
        }
        Err(fault) => Err(format!("`{chunk}` did not evaluate: {fault}").into()),
    }
}

/// Attaches a quiet, undemanding callback to each of `targets`.
fn attach_collectors(host: &mut ScriptHost, targets: &[Attachment]) -> TestResult {
    for target in targets {
        let callback = callback_from(host, "ash.luau", ALWAYS_COLLECTS)?;
        host.attach(target.clone(), callback);
    }
    Ok(())
}

/// What a round did with more work than it had room for.
#[derive(Debug, PartialEq, Eq)]
struct Refusal {
    order: Vec<String>,
    invocations: u32,
    pending: u32,
    faults: Vec<Reported>,
    refused_ran: u64,
}

/// What the round did, gathered into the one record the assertion compares.
///
/// The refused entries are asked for their invocation counts here rather than
/// inferred from the round's order: refusal means dropped, and a host that
/// quietly re-queued them for a later round reports the same faults now.
fn observed(host: &ScriptHost, report: &DispatchReport, asked_for: &[Attachment]) -> Refusal {
    Refusal {
        order: names(&report.order),
        invocations: report.invocations,
        pending: report.pending,
        faults: reported(&report.faults),
        refused_ran: asked_for
            .iter()
            .skip(ADMITTED as usize)
            .map(|target| host.invocation_count(target))
            .sum(),
    }
}

/// The refusals expected, in the order the work was asked for: the tail of the
/// list past what the queue had room for is what does not fit.
fn refusals(requester: &Attachment, asked_for: &[Attachment], admitted: u32) -> Vec<Reported> {
    asked_for
        .iter()
        .skip(admitted as usize)
        .map(|target| refusal(requester, target))
        .collect()
}

const WHY_A_FULL_QUEUE_NAMES_WHAT_IT_DROPPED: &str = "four faults of the right kind is the assertion a host that refused all twelve also \
     satisfies, which is why the eight admitted are asserted to have run: the round's order \
     names each of them, in the order they were asked for, and the invocation total is one \
     for the requester plus one apiece. The refused four are asserted to have run no times at \
     all, because refusal means dropped rather than delayed, and a host that quietly \
     re-queued them would report the same four faults. Each fault names two attachments, and \
     the second one is the whole reason this kind exists separately from deferral — for work \
     that is gone, an operator needs to know what is gone. Both come from typed fields, so \
     neither assertion is matching a substring of a message a dependency formats however it \
     likes.";

#[test]
fn follow_up_work_past_the_pending_bound_is_refused_and_each_refusal_names_what_was_dropped()
-> TestResult {
    let mut host = host_with(PENDING_BOUND, FAULT_THRESHOLD)?;
    let smelt = attachment("stone-furnace", "smelt");
    let asked_for = collectors(REQUESTED);
    attach_collectors(&mut host, &asked_for)?;
    let source = chunk_requesting(&asked_for);
    let requester = callback_from(&mut host, "furnace.luau", &source)?;
    host.attach(smelt.clone(), requester);

    let report = host.dispatch(std::slice::from_ref(&smelt));

    let mut ran = vec![named(&smelt)];
    ran.extend(asked_for.iter().take(ADMITTED as usize).map(named));
    assert_eq!(
        observed(&host, &report, &asked_for),
        Refusal {
            order: ran,
            invocations: INVOCATIONS,
            pending: 0,
            faults: refusals(&smelt, &asked_for, ADMITTED),
            refused_ran: 0,
        },
        "{WHY_A_FULL_QUEUE_NAMES_WHAT_IT_DROPPED}"
    );
    Ok(())
}

/// What a refusal did to the attachment it named.
#[derive(Debug, PartialEq, Eq)]
struct AfterRefusal {
    first_round: Vec<Reported>,
    second_round_invocations: u32,
    quarantined: bool,
    counted: u64,
}

const WHY_A_REFUSAL_IS_NOT_A_FAULTING_INVOCATION: &str = "the counting unit for quarantine is the outcome of an invocation, and a refusal is not \
     one: the requesting callback ran to the end and returned, and the fault is a property of \
     what the queue had room for. The threshold here is one and the queue holds one, so a \
     host routing a refusal through the same bookkeeping as a raised error quarantines the \
     requester after a single round, and the second round invokes nothing. Running a second \
     round is what makes that visible — the reset a successful invocation performs would \
     otherwise hide it at any higher threshold.";

#[test]
fn work_refused_for_want_of_room_does_not_count_against_the_attachment_that_asked_for_it()
-> TestResult {
    let mut host = host_with(WITNESS_PENDING_BOUND, WITNESS_THRESHOLD)?;
    let smelt = attachment("stone-furnace", "smelt");
    let asked_for = collectors(WITNESS_PENDING_BOUND + 1);
    attach_collectors(&mut host, &asked_for)?;
    let source = chunk_requesting(&asked_for);
    let requester = callback_from(&mut host, "furnace.luau", &source)?;
    host.attach(smelt.clone(), requester);

    let first = host.dispatch(std::slice::from_ref(&smelt));
    let second = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        AfterRefusal {
            first_round: reported(&first.faults),
            second_round_invocations: second.invocations,
            quarantined: host.is_quarantined(&smelt),
            counted: host.invocation_count(&smelt),
        },
        AfterRefusal {
            first_round: refusals(&smelt, &asked_for, WITNESS_PENDING_BOUND),
            second_round_invocations: 1 + WITNESS_PENDING_BOUND,
            quarantined: false,
            counted: 2,
        },
        "{WHY_A_REFUSAL_IS_NOT_A_FAULTING_INVOCATION}"
    );
    Ok(())
}
