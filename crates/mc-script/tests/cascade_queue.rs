//! What a callback does when it asks for more work, and what the host does with
//! the asking.
//!
//! # The return convention
//!
//! A callback that wants more work done returns a table carrying a `follow_up`
//! field holding an array of attachment identities:
//!
//! ```lua
//! return { follow_up = { { subject = "stone-furnace", component = "smelt" } } }
//! ```
//!
//! A callback returning anything else is returning a result and requesting
//! nothing, which is every other test in this crate and stays true.
//!
//! **Every part of that is read raw** — the field, each array slot, and each
//! `subject` and `component` — so a metatable a mod hung on the returned table
//! can neither run on the host's schedule nor observe which parts the host
//! looked at. The witness for the field itself already exists next door: the
//! probe in `raw_field_reads.rs` hands back a table whose `__index` counts its
//! own invocations, and a host reading `follow_up` the ordinary way increments
//! that counter and reddens a test that has nothing to do with cascades.
//!
//! # Queued, never entered inline
//!
//! Follow-up work is appended to a queue the round drains; a callback never
//! re-enters dispatch. This is the load-bearing decision rather than a
//! convenience. Synchronous re-entry turns an unbounded script cascade into
//! Rust stack growth, and a stack overflow is an abort — the one outcome this
//! crate exists to make unreachable, arrived at by the mechanism a depth counter
//! notices last. Queueing converts recursion depth into queue length, which is a
//! number the host can count and refuse.
//!
//! # Configured values
//!
//! A call-and-loop budget of 10,000, a pending bound of 256 and a fault
//! threshold of three, which are the figures the scenarios name. The round bound
//! is per-test: 64 where a round has to be filled exactly, and four where the
//! question is what a skipped entry costs against it. The memory limits stay at
//! their shipped defaults — nothing here allocates, and a second limit tripping
//! first would answer a different question.

use std::error::Error;
use std::num::{NonZeroU32, NonZeroU64};

use mc_script::{
    Attachment, ComponentName, DispatchReport, FaultKind, HostLimits, ScriptFault, ScriptFunction,
    ScriptHost, ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// What one invocation may spend.
const BUDGET: u64 = 10_000;

/// How many entries of follow-up work may wait at once. Far above anything this
/// file queues: refusal is the sibling file's question, not this one's.
const PENDING_BOUND: u32 = 256;

/// How many consecutive faults stop an attachment being invoked.
const FAULT_THRESHOLD: u32 = 3;

/// The round bound the scenarios name.
const ROUND_BOUND: u32 = 64;

/// A round bound small enough that a handful of skipped entries would fill it,
/// if skipping cost anything.
const NARROW_ROUND_BOUND: u32 = 4;

/// How many quarantined entries sit at the head of the queue in the skip test.
///
/// One fewer than the narrow bound, so a host charging a skip against the bound
/// runs the requester, burns the rest of the round on entries it had already
/// decided not to invoke, and reaches no collector at all.
const SKIPPED_ENTRIES: usize = NARROW_ROUND_BOUND as usize - 1;

/// A callback that raises every time. The second argument to `error` drops the
/// position prefix, which keeps this about how often rather than about how a
/// pre-1.0 dependency spells a location.
const ALWAYS_RAISES: &str = "return function()\n\terror('the vent is blocked', 0)\nend\n";

/// A callback that asks for nothing and returns quietly.
const ALWAYS_COLLECTS: &str = "return function()\n\treturn 'collected'\nend\n";

fn attachment(subject: &str, component: &str) -> Attachment {
    Attachment {
        subject: SubjectName::new(subject),
        component: ComponentName::new(component),
    }
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

fn kinds(faults: &[ScriptFault]) -> Vec<FaultKind> {
    faults.iter().map(|fault| fault.kind).collect()
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

/// A callback that asks for `targets` and returns nothing else.
fn chunk_requesting(targets: &[Attachment]) -> String {
    let request = follow_up_of(targets);
    format!("return function()\n\treturn {request}\nend\n")
}

/// A callback that asks for itself again until it has run `invocations` times,
/// then returns how many times that was.
///
/// The count comes back as the result, so a round that merely reported a number
/// is distinguishable from one whose callback actually ran to the end.
fn chunk_requesting_itself_until(target: &Attachment, invocations: u32) -> String {
    let request = follow_up_of(std::slice::from_ref(target));
    format!(
        "local calls = 0\n\
         return function()\n\
         \tcalls = calls + 1\n\
         \tif calls < {invocations} then return {request} end\n\
         \treturn calls\n\
         end\n"
    )
}

fn host_bounded_at(round_bound: u32) -> Result<ScriptHost, Box<dyn Error>> {
    let limits = HostLimits {
        call_and_loop_budget: NonZeroU64::new(BUDGET).ok_or("the budget must not be zero")?,
        fault_threshold: NonZeroU32::new(FAULT_THRESHOLD)
            .ok_or("the fault threshold must not be zero")?,
        round_bound: NonZeroU32::new(round_bound).ok_or("the round bound must not be zero")?,
        pending_bound: NonZeroU32::new(PENDING_BOUND)
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

/// What one invocation handed back, as one comparable line carrying the kind as
/// well as the quantity.
fn result_for(report: &DispatchReport, attachment: &Attachment) -> String {
    match report.results.get(attachment) {
        Some(ScriptValue::Nil) => "nil".to_owned(),
        Some(ScriptValue::Boolean(flag)) => format!("boolean {flag}"),
        Some(ScriptValue::Integer(number)) => format!("integer {number}"),
        Some(ScriptValue::Number(number)) => format!("number {number}"),
        Some(ScriptValue::Text(text)) => format!("text {text}"),
        Some(ScriptValue::Table(_)) => "table".to_owned(),
        Some(ScriptValue::Function(_)) => "function".to_owned(),
        Some(ScriptValue::Opaque) => "opaque".to_owned(),
        None => "no result".to_owned(),
    }
}

/// Attaches a callback that always raises to `broken` and runs it until the host
/// stops invoking it.
fn quarantine(host: &mut ScriptHost, broken: &Attachment) -> TestResult {
    let jams = callback_from(host, "vent.luau", ALWAYS_RAISES)?;
    host.attach(broken.clone(), jams);
    for _ in 0..FAULT_THRESHOLD {
        host.dispatch(std::slice::from_ref(broken));
    }
    Ok(())
}

/// Attaches a quiet, undemanding callback to each of `collectors`.
fn attach_collectors(host: &mut ScriptHost, collectors: &[Attachment]) -> TestResult {
    for collector in collectors {
        let callback = callback_from(host, "ash.luau", ALWAYS_COLLECTS)?;
        host.attach(collector.clone(), callback);
    }
    Ok(())
}

const WHY_THE_FOLLOW_UP_COMES_AFTER_THE_RETURN: &str = "only the first attachment is seeded, so every later entry in the order is follow-up \
     work being honoured rather than the caller asking twice. The shape is what makes the \
     order say more than that: the requester asks for two, and the first of those two asks \
     for a third. Drained from a queue, the pair the requester asked for run before the one \
     asked for later — the order below. Entered from inside the invocation that asked, the \
     third runs before the requester's second, because the walk goes deeper rather than \
     wider. That difference is the whole decision: recursion here is Rust stack growth, and \
     an unbounded cascade reaches a stack overflow, which is a process abort — the one \
     outcome this crate exists to make unreachable, and the one a depth counter notices \
     last.";

#[test]
fn requested_follow_up_work_is_entered_only_after_its_requester_has_returned() -> TestResult {
    let mut host = host_bounded_at(ROUND_BOUND)?;
    let smelt = attachment("stone-furnace", "smelt");
    let vent = attachment("stone-furnace", "vent");
    let flue = attachment("stone-furnace", "flue");
    let ash = attachment("ash-bin", "scrape");
    let asks_for_both = chunk_requesting(&[vent.clone(), flue.clone()]);
    let asks_for_one = chunk_requesting(std::slice::from_ref(&ash));
    let requester = callback_from(&mut host, "furnace.luau", &asks_for_both)?;
    let middle = callback_from(&mut host, "vent.luau", &asks_for_one)?;
    host.attach(smelt.clone(), requester);
    host.attach(vent.clone(), middle);
    attach_collectors(&mut host, &[flue.clone(), ash.clone()])?;

    let report = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        names(&report.order),
        vec![named(&smelt), named(&vent), named(&flue), named(&ash)],
        "{WHY_THE_FOLLOW_UP_COMES_AFTER_THE_RETURN}"
    );
    Ok(())
}

/// A round that filled itself exactly, and what was left behind it.
#[derive(Debug, PartialEq, Eq)]
struct ExactFill {
    invocations: u32,
    pending: u32,
    faults: Vec<FaultKind>,
    result: String,
    counted: u64,
    drained_afterwards: u32,
}

const WHY_FILLING_A_ROUND_IS_NOT_ITSELF_A_FAULT: &str = "reaching the bound cannot be what raises a cascade fault, or every busy round would \
     report one. The trigger is work left over, and here there is none: the last invocation \
     returned a value instead of asking for more, so the queue is empty at the moment the \
     count reaches the bound. The result and the invocation count are both here because a \
     host that stopped one short and reported the bound anyway satisfies a count on its own — \
     the callback's own tally is a script-side oracle that shares nothing with the host's. \
     The empty round afterwards is what says the queue is genuinely empty rather than \
     reported empty.";

#[test]
fn a_cascade_that_exactly_fills_a_round_completes_inside_it_without_a_cascade_fault() -> TestResult
{
    let mut host = host_bounded_at(ROUND_BOUND)?;
    let smelt = attachment("stone-furnace", "smelt");
    let callback = callback_from(
        &mut host,
        "furnace.luau",
        &chunk_requesting_itself_until(&smelt, ROUND_BOUND),
    )?;
    host.attach(smelt.clone(), callback);

    let filled = host.dispatch(std::slice::from_ref(&smelt));
    let afterwards = host.dispatch(&[]);

    assert_eq!(
        ExactFill {
            invocations: filled.invocations,
            pending: filled.pending,
            faults: kinds(&filled.faults),
            result: result_for(&filled, &smelt),
            counted: host.invocation_count(&smelt),
            drained_afterwards: afterwards.invocations,
        },
        ExactFill {
            invocations: ROUND_BOUND,
            pending: 0,
            faults: Vec::new(),
            result: format!("integer {ROUND_BOUND}"),
            counted: u64::from(ROUND_BOUND),
            drained_afterwards: 0,
        },
        "{WHY_FILLING_A_ROUND_IS_NOT_ITSELF_A_FAULT}"
    );
    Ok(())
}

/// Runs the round the skip test observes: a requester asking for the
/// quarantined entries first and the collectors behind them.
fn round_over_a_queue_headed_by(
    host: &mut ScriptHost,
    requester_of: &Attachment,
    quarantined: &Attachment,
    collectors: &[Attachment],
) -> Result<DispatchReport, Box<dyn Error>> {
    let mut requested = vec![quarantined.clone(); SKIPPED_ENTRIES];
    requested.extend(collectors.iter().cloned());
    let callback = callback_from(host, "furnace.luau", &chunk_requesting(&requested))?;
    host.attach(requester_of.clone(), callback);
    Ok(host.dispatch(std::slice::from_ref(requester_of)))
}

/// What became of a round whose queue held a quarantined attachment.
#[derive(Debug, PartialEq, Eq)]
struct Skipped {
    order: Vec<String>,
    invocations: u32,
    pending: u32,
    faults: Vec<FaultKind>,
    still_quarantined: bool,
    quarantined_invocations: u64,
}

const WHY_A_QUARANTINED_TARGET_COSTS_THE_ROUND_NOTHING: &str = "the round bound is four and the queue holds three quarantined entries ahead of three \
     runnable ones, so a host charging a skip against the bound runs the requester and \
     nothing else, then reports as deferred the work it had already decided never to do. A \
     fault count alone cannot separate those two hosts: neither reports a fault for the \
     quarantined attachment. What separates them is which attachments appear in the round's \
     order and how many invocations it admitted. The frozen invocation count is the third \
     witness — it says the skipped attachment was not merely unreported but genuinely never \
     entered. This is also the only path that reaches quarantine through the queue rather \
     than through a seed the caller supplied.";

#[test]
fn follow_up_work_naming_a_quarantined_attachment_is_skipped_without_a_fault_or_an_invocation()
-> TestResult {
    let mut host = host_bounded_at(NARROW_ROUND_BOUND)?;
    let smelt = attachment("stone-furnace", "smelt");
    let vent = attachment("stone-furnace", "vent");
    let collectors = ["scrape", "sweep", "bag"].map(|task| attachment("ash-bin", task));
    quarantine(&mut host, &vent)?;
    attach_collectors(&mut host, &collectors)?;

    let report = round_over_a_queue_headed_by(&mut host, &smelt, &vent, &collectors)?;

    let mut ran = vec![named(&smelt)];
    ran.extend(collectors.iter().map(named));
    assert_eq!(
        Skipped {
            order: names(&report.order),
            invocations: report.invocations,
            pending: report.pending,
            faults: kinds(&report.faults),
            still_quarantined: host.is_quarantined(&vent),
            quarantined_invocations: host.invocation_count(&vent),
        },
        Skipped {
            order: ran,
            invocations: NARROW_ROUND_BOUND,
            pending: 0,
            faults: Vec::new(),
            still_quarantined: true,
            quarantined_invocations: u64::from(FAULT_THRESHOLD),
        },
        "{WHY_A_QUARANTINED_TARGET_COSTS_THE_ROUND_NOTHING}"
    );
    Ok(())
}
