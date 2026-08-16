//! What happens when a cascade outlives the round it started in.
//!
//! # Two cascades that look identical from inside round one
//!
//! A cascade that never terminates and a cascade of exactly two hundred are
//! locally indistinguishable at the moment the first round fills: both reached
//! the bound with work still queued, and neither can be told apart without
//! running the very work the bound exists to refuse. So the report is **eager** —
//! a round that ends with work pending says so immediately rather than waiting
//! to find out whether the work eventually finishes.
//!
//! The consequence is recorded here so nobody later reads it as a defect: a
//! perfectly well-behaved terminating cascade emits one cascade-deferred fault
//! per round it spans before completing normally. That is operator-facing noise
//! the specification permits, and it is noise precisely because deferral loses
//! nothing — the work runs, one round later. It is also why deferral must not
//! wear the same name as refusal, which loses the work outright.
//!
//! # Whom a deferral blames
//!
//! The **requester of the first entry that could not run**, never the entry
//! itself. The entry did nothing; something asked for it. The non-terminating
//! test below bounces between two attachments precisely so that the blamed one
//! is not the one the caller seeded — a host that names the seed passes a
//! single-attachment cascade and fails here.
//!
//! # The queue outlives the round
//!
//! A seed is **appended** to whatever is already waiting, and a dispatch with an
//! empty seed drains the residual. That is what lets two hundred invocations
//! finish across four rounds without the caller re-seeding anything, and it is
//! what the four-round test asserts by seeding exactly once.
//!
//! # Configured values
//!
//! A call-and-loop budget of 10,000, a pending bound of 256 and a fault
//! threshold of three, which are the figures the scenarios name; a round bound
//! of 64 for the scenarios and of two for the witness at the end, where the
//! question is what a deferral does to an attachment's fault total rather than
//! how many invocations fit. The memory limits stay at their shipped defaults.

use std::error::Error;
use std::num::{NonZeroU32, NonZeroU64};

use mc_script::{
    Attachment, ComponentName, DispatchReport, FaultKind, HostLimits, ScriptFault, ScriptFunction,
    ScriptHost, ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// What one invocation may spend.
const BUDGET: u64 = 10_000;

/// How many entries of follow-up work may wait at once.
const PENDING_BOUND: u32 = 256;

/// How many consecutive faults stop an attachment being invoked.
const FAULT_THRESHOLD: u32 = 3;

/// The round bound the scenarios name.
const ROUND_BOUND: u32 = 64;

/// How long the terminating cascade is.
const CASCADE_LENGTH: u32 = 200;

/// How many rounds of the terminating cascade fill their bound exactly, and how
/// many invocations are left for the one after them.
///
/// Both derived from the two literals above and never read back off a run: three
/// full rounds of 64 leave 8, and 64 + 64 + 64 + 8 is 200.
const FULL_ROUNDS: u32 = 3;
const LAST_ROUND: u32 = CASCADE_LENGTH - ROUND_BOUND * FULL_ROUNDS;

/// The round bound and threshold the closing witness runs under, and how long
/// its cascade is: two rounds of two, so a deferral happens and the cascade
/// still terminates.
const WITNESS_ROUND_BOUND: u32 = 2;
const WITNESS_THRESHOLD: u32 = 1;
const WITNESS_CASCADE: u32 = 4;

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

/// A deferral naming who asked for the work that did not fit.
fn deferred(requester: &Attachment) -> Reported {
    Reported {
        kind: FaultKind::CascadeDeferred,
        blamed: Some(named(requester)),
        refused: None,
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

/// A callback that asks for `targets` every single time it runs.
fn chunk_requesting_forever(targets: &[Attachment]) -> String {
    let request = follow_up_of(targets);
    format!("return function()\n\treturn {request}\nend\n")
}

/// A callback that asks for itself again until it has run `invocations` times,
/// then returns how many times that was.
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

fn host_bounded_at(round_bound: u32, threshold: u32) -> Result<ScriptHost, Box<dyn Error>> {
    let limits = HostLimits {
        call_and_loop_budget: NonZeroU64::new(BUDGET).ok_or("the budget must not be zero")?,
        fault_threshold: NonZeroU32::new(threshold)
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

/// Which of two alternating attachments performed the round's last invocation.
///
/// The pair bounces, starting from the seed, so an even bound ends on the one
/// the seed asked for and an odd one ends on the seed itself. Derived rather
/// than transcribed, so changing the bound cannot silently move the answer.
fn ran_last<'a>(seed: &'a Attachment, other: &'a Attachment, bound: u32) -> &'a Attachment {
    if bound.is_multiple_of(2) { other } else { seed }
}

/// What each round did, as invocations performed and work left waiting.
fn per_round(rounds: &[DispatchReport]) -> Vec<(u32, u32)> {
    rounds
        .iter()
        .map(|round| (round.invocations, round.pending))
        .collect()
}

/// Every fault raised across a run of rounds, in the order they were raised.
fn every_kind(rounds: &[DispatchReport]) -> Vec<FaultKind> {
    rounds
        .iter()
        .flat_map(|round| round.faults.iter().map(|fault| fault.kind))
        .collect()
}

/// How a round that could not finish its work ended.
#[derive(Debug, PartialEq, Eq)]
struct Overflow {
    invocations: u32,
    pending: u32,
    faults: Vec<Reported>,
}

const WHY_AN_ENDLESS_CASCADE_ENDS_THE_ROUND_AND_NAMES_ITS_REQUESTER: &str = "the round has to end and control has to come back, which is the half of this scenario \
     that an unbounded host reports as a wedged run rather than as a red one. The other half \
     is whom it names. The two attachments ask for each other, so with an even bound the last \
     invocation of the round belongs to the one the seed asked for and not to the seed — a \
     host that blames the seed, or that blames the entry which could not run instead of \
     whoever asked for it, reports a fault of the right kind naming the wrong attachment, and \
     an operator sent to the wrong mod is worse served than one sent nowhere. The absent \
     refused target is the asymmetry: nothing was dropped, so there is nothing to name.";

#[test]
fn a_cascade_that_never_terminates_ends_its_round_and_blames_whoever_asked_for_what_is_left()
-> TestResult {
    let mut host = host_bounded_at(ROUND_BOUND, FAULT_THRESHOLD)?;
    let smelt = attachment("stone-furnace", "smelt");
    let vent = attachment("stone-furnace", "vent");
    let asks_for_vent = chunk_requesting_forever(std::slice::from_ref(&vent));
    let asks_for_smelt = chunk_requesting_forever(std::slice::from_ref(&smelt));
    let furnace = callback_from(&mut host, "furnace.luau", &asks_for_vent)?;
    let flue = callback_from(&mut host, "vent.luau", &asks_for_smelt)?;
    host.attach(smelt.clone(), furnace);
    host.attach(vent.clone(), flue);

    let report = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        Overflow {
            invocations: report.invocations,
            pending: report.pending,
            faults: reported(&report.faults),
        },
        Overflow {
            invocations: ROUND_BOUND,
            pending: 1,
            faults: vec![deferred(ran_last(&smelt, &vent, ROUND_BOUND))],
        },
        "{WHY_AN_ENDLESS_CASCADE_ENDS_THE_ROUND_AND_NAMES_ITS_REQUESTER}"
    );
    Ok(())
}

/// How a cascade that outlived three rounds settled.
#[derive(Debug, PartialEq, Eq)]
struct Settled {
    per_round: Vec<(u32, u32)>,
    total: u32,
    counted: u64,
    result: String,
    faults: Vec<FaultKind>,
    drained_afterwards: u32,
}

/// What a run of rounds did, gathered into the one record the assertion
/// compares.
fn settled(
    host: &ScriptHost,
    spanned: &[DispatchReport],
    afterwards: &DispatchReport,
    requester: &Attachment,
) -> Result<Settled, Box<dyn Error>> {
    let last = spanned.last().ok_or("no round was run")?;
    Ok(Settled {
        per_round: per_round(spanned),
        total: spanned.iter().map(|round| round.invocations).sum(),
        counted: host.invocation_count(requester),
        result: result_for(last, requester),
        faults: every_kind(spanned),
        drained_afterwards: afterwards.invocations,
    })
}

/// The four rounds this cascade is expected to take, written from the two
/// figures the scenario names.
fn four_bounded_rounds() -> Vec<(u32, u32)> {
    let mut rounds = vec![(ROUND_BOUND, 1); FULL_ROUNDS as usize];
    rounds.push((LAST_ROUND, 0));
    rounds
}

const WHY_TWO_HUNDRED_INVOCATIONS_TAKE_FOUR_ROUNDS: &str = "sixty-four, sixty-four, sixty-four and eight, which is two hundred — every one of those \
     numbers written from the two the scenario names rather than read back off a run. The \
     per-round breakdown is what makes the total mean anything: a host that ran all two \
     hundred in one round reports the same total and has no bound at all, and a host that \
     reported two hundred while running fewer is caught by the callback's own tally, which is \
     a script-side count sharing no code with the host's. The pending figure after each round \
     is the residual the next one drains, and the caller seeds exactly once — so a host that \
     dropped the queue at a round boundary does nothing from the second round on. Three \
     deferrals and no other fault is the noise this design accepts, written as a list so that \
     four of them, none of them, or one of another kind is visible rather than tolerated.";

#[test]
fn a_cascade_of_two_hundred_invocations_finishes_across_four_bounded_rounds() -> TestResult {
    let mut host = host_bounded_at(ROUND_BOUND, FAULT_THRESHOLD)?;
    let smelt = attachment("stone-furnace", "smelt");
    let source = chunk_requesting_itself_until(&smelt, CASCADE_LENGTH);
    let callback = callback_from(&mut host, "furnace.luau", &source)?;
    host.attach(smelt.clone(), callback);

    let mut spanned = vec![host.dispatch(std::slice::from_ref(&smelt))];
    for _ in 0..FULL_ROUNDS {
        spanned.push(host.dispatch(&[]));
    }
    let afterwards = host.dispatch(&[]);

    assert_eq!(
        settled(&host, &spanned, &afterwards, &smelt)?,
        Settled {
            per_round: four_bounded_rounds(),
            total: CASCADE_LENGTH,
            counted: u64::from(CASCADE_LENGTH),
            result: format!("integer {CASCADE_LENGTH}"),
            faults: vec![FaultKind::CascadeDeferred; FULL_ROUNDS as usize],
            drained_afterwards: 0,
        },
        "{WHY_TWO_HUNDRED_INVOCATIONS_TAKE_FOUR_ROUNDS}"
    );
    Ok(())
}

/// What a deferral did to the attachment it named.
#[derive(Debug, PartialEq, Eq)]
struct AfterDeferral {
    first_round: Vec<Reported>,
    second_round_invocations: u32,
    quarantined: bool,
    counted: u64,
    result: String,
}

const WHY_A_DEFERRAL_IS_NOT_A_FAULTING_INVOCATION: &str = "quarantine counts the outcomes of invocations, and a deferral is not one: the \
     invocation completed and returned, and what was reported is a property of the round's \
     admission control rather than of anything the callback did. The threshold here is one, \
     so a host counting deferrals toward it quarantines this attachment on the strength of a \
     single round and the second round performs no invocations at all — which is why a second \
     round is run rather than the flag read after the first. The threshold the rest of this \
     file names would hide the same defect behind the reset a successful invocation performs, \
     since the blamed requester succeeds dozens of times per round and its faults are never \
     consecutive.";

#[test]
fn work_deferred_to_a_later_round_does_not_count_against_the_attachment_that_asked_for_it()
-> TestResult {
    let mut host = host_bounded_at(WITNESS_ROUND_BOUND, WITNESS_THRESHOLD)?;
    let smelt = attachment("stone-furnace", "smelt");
    let source = chunk_requesting_itself_until(&smelt, WITNESS_CASCADE);
    let callback = callback_from(&mut host, "furnace.luau", &source)?;
    host.attach(smelt.clone(), callback);

    let first = host.dispatch(std::slice::from_ref(&smelt));
    let second = host.dispatch(&[]);

    assert_eq!(
        AfterDeferral {
            first_round: reported(&first.faults),
            second_round_invocations: second.invocations,
            quarantined: host.is_quarantined(&smelt),
            counted: host.invocation_count(&smelt),
            result: result_for(&second, &smelt),
        },
        AfterDeferral {
            first_round: vec![deferred(&smelt)],
            second_round_invocations: WITNESS_ROUND_BOUND,
            quarantined: false,
            counted: u64::from(WITNESS_CASCADE),
            result: format!("integer {WITNESS_CASCADE}"),
        },
        "{WHY_A_DEFERRAL_IS_NOT_A_FAULTING_INVOCATION}"
    );
    Ok(())
}
