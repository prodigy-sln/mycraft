//! When the host stops invoking an attachment, and what it takes to be counted
//! as broken enough to stop.
//!
//! # Three consecutive faults, and one success wipes the slate
//!
//! The budget bounds what a single invocation costs; quarantine bounds how often
//! a broken one repeats. They are different jobs, which is why the count is
//! **consecutive** and why a success resets it. A callback that alternates
//! failing and succeeding is therefore never quarantined, and that is accepted
//! rather than overlooked: its cost is already bounded, invocation by
//! invocation, by the limit that bounds every other invocation.
//!
//! # Every fault kind is one fault
//!
//! The count is over the outcome of an invocation, not over a kind. An
//! attachment that runs away, then allocates past what one invocation may hold,
//! then raises has failed three times running, and the host that counts those
//! separately never reaches a threshold at all — which is the shape a mod
//! failing three different ways actually has. That is what the mixed-kind test
//! below is for, and it is why it is written as three different failures rather
//! than three of the same one.
//!
//! # The pressure condition has to be *absent* here, and it is asserted
//!
//! Faults raised while the host itself is short of memory do not count toward
//! this total — a separate mechanism with its own tests. Which means a fixture
//! that accidentally put the host into that condition would make the scenarios
//! here measure nothing, and nothing in the assertions would say so. So the
//! tests that quarantine an attachment also assert the host is **not** under
//! memory pressure while they do it. The condition is written out here from its
//! own definition rather than asked of the host as a verdict: the state's
//! collected memory, plus what one invocation may add, against the ceiling the
//! whole state may reach.
//!
//! # Configured values
//!
//! The threshold is three, which is the figure every scenario names, and it is
//! configured rather than inherited so that this file says nothing about what
//! the host ships. The budget is fifty thousand: large enough that the
//! allocation bomb below is stopped by the memory cap rather than by its ticks —
//! the two limits mask each other and the masking is measured — and small enough
//! that a runaway loop ends in milliseconds. The memory limits stay at their
//! shipped defaults, and the bomb is sized against the cap rather than the other
//! way round.

use std::error::Error;
use std::num::{NonZeroU32, NonZeroU64};

use mc_script::{
    Attachment, ComponentName, DispatchReport, FaultKind, HostLimits, ScriptFunction, ScriptHost,
    ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// How many consecutive faults every scenario in this file names.
const FAULT_THRESHOLD: u32 = 3;

/// How many faults the recovering callback raises before it succeeds.
const FAULTS_BEFORE_IT_RECOVERS: u32 = 2;

/// How many rounds a quarantined attachment is left alone for.
const ROUNDS_AFTER_QUARANTINE: usize = 3;

/// A budget the allocation bomb below cannot reach and a runaway loop reaches
/// in milliseconds.
const BUDGET: u64 = 50_000;

/// How many buffers the bomb appends, and how large each one is.
///
/// Their product is twice the shipped cap, so the invocation is stopped partway
/// by the cap rather than finishing; a host that enforces nothing returns
/// promptly rather than taking the run down with it. **Buffers rather than
/// strings, and that is load-bearing**: the backend shares identical strings, so
/// a loop appending the same one allocates it once and this bomb would allocate
/// nothing at all.
const APPENDS_PAST_THE_CAP: usize = 128;
const APPENDED_BYTES: usize = 4096;

/// How the broken attachment reads when the host names it.
const THE_BROKEN_ATTACHMENT: &str = "stone-furnace/smelt";

/// A callback that raises every time it is invoked.
///
/// The second argument to `error` drops the position prefix, which keeps these
/// tests about how often a fault happened rather than about how a pre-1.0
/// dependency spells a location.
const ALWAYS_RAISES: &str = "return function()\n\terror('the furnace is jammed', 0)\nend\n";

/// A callback that raises on its first `failures` invocations and afterwards
/// returns how many times it has been invoked.
///
/// The count comes back as the result, so a round that merely happened is
/// distinguishable from a round whose callback actually ran to the end.
fn chunk_that_raises_then_counts(failures: u32) -> String {
    format!(
        "local calls = 0\n\
         return function()\n\
         \tcalls = calls + 1\n\
         \tif calls <= {failures} then error('the furnace is jammed', 0) end\n\
         \treturn calls\n\
         end\n"
    )
}

/// A callback that fails a different way on each of its first three
/// invocations: it runs away, then allocates past what one invocation may hold,
/// then raises.
///
/// The counter is an upvalue and survives every abort, which is what lets one
/// callback produce three different kinds in three consecutive rounds.
fn chunk_that_fails_three_different_ways(appends: usize, bytes: usize) -> String {
    format!(
        "local calls = 0\n\
         return function()\n\
         \tcalls = calls + 1\n\
         \tif calls == 1 then\n\
         \t\twhile true do end\n\
         \telseif calls == 2 then\n\
         \t\tlocal held = {{}}\n\
         \t\tfor index = 1, {appends} do held[index] = buffer.create({bytes}) end\n\
         \t\treturn #held\n\
         \tend\n\
         \terror('the furnace is jammed', 0)\n\
         end\n"
    )
}

/// What one round did to one attachment, as one comparable record.
#[derive(Debug, PartialEq, Eq)]
struct RoundOutcome {
    faults: Vec<FaultKind>,
    quarantined: Vec<String>,
}

/// A round that reported a fault and quarantined nobody.
fn faulted(kind: FaultKind) -> RoundOutcome {
    RoundOutcome {
        faults: vec![kind],
        quarantined: Vec::new(),
    }
}

/// A round that reported a fault and stopped invoking `attachment`.
fn faulted_and_quarantined(kind: FaultKind, attachment: &str) -> RoundOutcome {
    RoundOutcome {
        faults: vec![kind],
        quarantined: vec![attachment.to_owned()],
    }
}

fn outcome_of(report: &DispatchReport) -> RoundOutcome {
    RoundOutcome {
        faults: report.faults.iter().map(|fault| fault.kind).collect(),
        quarantined: report.quarantined.iter().map(named).collect(),
    }
}

/// Both halves of an attachment's identity, so a host that names one of them is
/// distinguishable from one that names both.
fn named(attachment: &Attachment) -> String {
    format!(
        "{}/{}",
        attachment.subject.as_str(),
        attachment.component.as_str()
    )
}

/// Whether the host's own memory leaves no room for one more invocation.
///
/// Stated here from the condition's own definition rather than asked of the host
/// as a verdict, so a host whose classification is wrong cannot also decide
/// whether the fixture was valid. While it holds, faults stop counting toward
/// quarantine — which is why every quarantine below asserts it does not.
fn under_memory_pressure(host: &ScriptHost) -> bool {
    let limits = host.limits();
    host.collected_memory_in_use()
        .saturating_add(limits.memory_cap.get())
        > limits.memory_backstop.get()
}

/// A host at the threshold every scenario names, and at a budget neither limit
/// below can reach by accident.
fn host_at_the_named_threshold() -> Result<ScriptHost, Box<dyn Error>> {
    let limits = HostLimits {
        call_and_loop_budget: NonZeroU64::new(BUDGET).ok_or("the budget must not be zero")?,
        fault_threshold: NonZeroU32::new(FAULT_THRESHOLD)
            .ok_or("the fault threshold must not be zero")?,
        ..HostLimits::default()
    };
    ScriptHost::with_limits(limits).map_err(|error| {
        format!("the host refused a threshold of {FAULT_THRESHOLD}: {error}").into()
    })
}

fn attachment(subject: &str, component: &str) -> Attachment {
    Attachment {
        subject: SubjectName::new(subject),
        component: ComponentName::new(component),
    }
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

/// What one invocation handed back, as one comparable line.
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

/// Runs `rounds` rounds over one attachment and reports what each one did.
fn rounds_over(host: &mut ScriptHost, attachment: &Attachment, rounds: u32) -> Vec<RoundOutcome> {
    (0..rounds)
        .map(|_| outcome_of(&host.dispatch(std::slice::from_ref(attachment))))
        .collect()
}

const WHY_THE_THIRD_FAULT_IS_THE_ONE_THAT_STOPS_IT: &str = "a callback that fails every time it runs costs the server a full budget every round \
     forever, and an operator who cannot see which attachment stopped cannot act on it — so \
     the report names the subject and the component, not one of them. The first two rounds \
     are asserted as well as the third: a host that quarantines on the first fault satisfies \
     `is_quarantined` just as well and turns one bad round into a permanent disabling with no \
     record of why. The host is asserted to be clear of memory pressure throughout, because \
     faults raised under it do not count at all and this whole scenario would then be \
     measuring nothing while every assertion about it still read plausibly.";

#[test]
fn an_attachment_whose_callback_fails_three_rounds_running_stops_being_invoked() -> TestResult {
    let mut host = host_at_the_named_threshold()?;
    let jams = callback_from(&mut host, "furnace.luau", ALWAYS_RAISES)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), jams);

    let rounds = rounds_over(&mut host, &smelt, FAULT_THRESHOLD);

    assert_eq!(
        (
            rounds,
            host.is_quarantined(&smelt),
            under_memory_pressure(&host)
        ),
        (
            vec![
                faulted(FaultKind::ScriptError),
                faulted(FaultKind::ScriptError),
                faulted_and_quarantined(FaultKind::ScriptError, THE_BROKEN_ATTACHMENT),
            ],
            true,
            false
        ),
        "{WHY_THE_THIRD_FAULT_IS_THE_ONE_THAT_STOPS_IT}"
    );
    Ok(())
}

const WHY_A_SUCCESS_WIPES_THE_SLATE: &str = "the count is consecutive, so a callback that recovers is a callback that is working and \
     the host has nothing left to act on. A host that counted faults cumulatively would \
     quarantine this one on its third *failure* however many successful rounds sat between \
     them, which over a long-running server is every attachment that has ever had a bad day. \
     The fourth round's returned value is the assertion rather than the fact of invocation: a \
     host that invoked it and discarded the answer would pass the weaker check, and the count \
     coming back is what says the callback ran to its end.";

#[test]
fn an_attachment_that_recovers_before_the_threshold_keeps_being_invoked() -> TestResult {
    let mut host = host_at_the_named_threshold()?;
    let source = chunk_that_raises_then_counts(FAULTS_BEFORE_IT_RECOVERS);
    let recovering = callback_from(&mut host, "furnace.luau", &source)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), recovering);

    rounds_over(&mut host, &smelt, FAULTS_BEFORE_IT_RECOVERS);
    let recovered = host.dispatch(std::slice::from_ref(&smelt));
    let afterwards = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (
            result_for(&recovered, &smelt),
            result_for(&afterwards, &smelt),
            host.is_quarantined(&smelt),
        ),
        ("integer 3".to_owned(), "integer 4".to_owned(), false),
        "{WHY_A_SUCCESS_WIPES_THE_SLATE}"
    );
    Ok(())
}

const WHY_THREE_DIFFERENT_FAILURES_ARE_STILL_THREE_FAILURES: &str = "the thing being counted is the outcome of an invocation, not a kind of outcome. A mod \
     that runs away one round, asks for more memory than it may hold the next, and raises on \
     the third has failed three times running — and a host keeping a tally per kind never \
     reaches any threshold at all, which is the ordinary shape of a broken callback rather \
     than an exotic one. The three kinds are asserted in order, so a host that reached the \
     threshold by producing the same kind three times is not mistaken for one that counted \
     three different ones. The host is clear of memory pressure throughout, which for this \
     scenario matters twice over: the allocation round is exactly the round a pressured host \
     would excuse.";

#[test]
fn an_attachment_that_fails_three_different_ways_running_is_stopped_the_same_way() -> TestResult {
    let mut host = host_at_the_named_threshold()?;
    let source = chunk_that_fails_three_different_ways(APPENDS_PAST_THE_CAP, APPENDED_BYTES);
    let broken = callback_from(&mut host, "furnace.luau", &source)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), broken);

    let rounds = rounds_over(&mut host, &smelt, FAULT_THRESHOLD);

    assert_eq!(
        (
            rounds,
            host.is_quarantined(&smelt),
            under_memory_pressure(&host)
        ),
        (
            vec![
                faulted(FaultKind::BudgetExhausted),
                faulted(FaultKind::Allocation),
                faulted_and_quarantined(FaultKind::ScriptError, THE_BROKEN_ATTACHMENT),
            ],
            true,
            false
        ),
        "{WHY_THREE_DIFFERENT_FAILURES_ARE_STILL_THREE_FAILURES}"
    );
    Ok(())
}

const WHY_QUARANTINE_HAS_TO_HOLD_RATHER_THAN_HAPPEN: &str = "quarantine that is reported once and quietly forgotten is worth nothing: the point is \
     the rounds afterwards, where the broken callback costs the server nothing at all. So \
     three further rounds are run and each is asked two questions — did this round invoke \
     anything, and has the count moved — because a host that invoked the callback and \
     discarded its fault would leave every fault-shaped assertion green while paying the full \
     cost every round. The count frozen at quarantine is the same number the threshold \
     names, by arithmetic rather than by observation: three rounds, one invocation each.";

#[test]
fn a_quarantined_attachment_is_left_alone_and_its_count_stays_where_it_stopped() -> TestResult {
    let mut host = host_at_the_named_threshold()?;
    let jams = callback_from(&mut host, "furnace.luau", ALWAYS_RAISES)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), jams);

    rounds_over(&mut host, &smelt, FAULT_THRESHOLD);
    let frozen = host.invocation_count(&smelt);
    let mut invocations = Vec::new();
    let mut counts = Vec::new();
    for _ in 0..ROUNDS_AFTER_QUARANTINE {
        invocations.push(host.dispatch(std::slice::from_ref(&smelt)).invocations);
        counts.push(host.invocation_count(&smelt));
    }

    assert_eq!(
        (frozen, invocations, counts, host.is_quarantined(&smelt)),
        (
            u64::from(FAULT_THRESHOLD),
            vec![0; ROUNDS_AFTER_QUARANTINE],
            vec![u64::from(FAULT_THRESHOLD); ROUNDS_AFTER_QUARANTINE],
            true
        ),
        "{WHY_QUARANTINE_HAS_TO_HOLD_RATHER_THAN_HAPPEN}"
    );
    Ok(())
}
