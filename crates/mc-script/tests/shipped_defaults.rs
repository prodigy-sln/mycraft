//! The limits a host runs under when nobody configures it, and what they do.
//!
//! Every other test in this crate configures the limit it is about, because a
//! production-sized memory cap cannot be tripped inside a test's time budget and
//! a test-sized one would refuse ordinary content. That leaves the shipped
//! values themselves constrained by nothing: a host whose defaults were absent,
//! zero or effectively unlimited passes the whole suite, and it is the default —
//! not the test value — that runs on a server.
//!
//! # The values, and what each is answerable to
//!
//! | limit | value | why that one |
//! |---|---|---|
//! | call-and-loop budget | 1,000,000 | The largest plausible **unsliceable** workload. A callback over budget has a mechanism — the queue, across rounds; a chunk over budget has none. A 64³ volume walked at one host call per cell is about 540,000 ticks, which this admits with room, and it refuses a workload an order of magnitude past that. |
//! | memory cap | 256 KiB | A **delta above the entry baseline**, so its floor is what a callback plausibly needs rather than what a state weighs. |
//! | memory backstop | 16 MiB | Must exceed the state's own baseline of 385,952 bytes plus the cap, i.e. 648,096. Roughly twenty-five times that leaves room for legitimately retained state across many attachments while staying a number an operator can reason about. It is the one value that decides when scripting stops working for everybody. |
//! | fault threshold | 3 | Three consecutive faults, the count reset by a success. |
//! | round bound | 64 | Invocations one round may perform. |
//! | pending bound | 256 | Four rounds' worth of queued work at that round bound. |
//! | retained print output | 256 KiB | Bytes of script output one host keeps, over its whole life. The same figure as the per-entry memory cap **on purpose**: the host-side copy of what content printed cannot then outgrow the allowance the chunk had to build it in, which is the only thing in reach that ties this number to something rather than to taste. It is stated as its own literal all the same — computed from the cap it would follow the cap wherever it went and bound nothing. |
//!
//! The budget counts calls and loop edges and not instructions — the interrupt
//! is emitted at seven opcodes, so a loop body of any size is free, a host call
//! costs one and a call within script costs two. Sizing it against how much code
//! a workload is, rather than against how many calls it makes, is wrong by the
//! size of every loop body in it.
//!
//! # Two ways to write these tests would prove nothing
//!
//! **"Finite and non-zero" cannot fail.** Every field is a `NonZero*`, so that
//! is a property of the type rather than of the value, and a test asserting it
//! stays green against a budget of one and a cap of one byte. The values are
//! asserted, and the expected ones are written here rather than read out of the
//! host, because an expectation sourced from the code under test agrees with
//! whatever that code happens to say.
//!
//! **Naming four limits stops covering the fifth, silently.** Three of the seven
//! arrived after the first four were settled, and they are the two most likely
//! to be wrong. So the expectation is built as a whole `HostLimits` **with no
//! `..` in it**: a seventh limit added later leaves this file unable to build,
//! which is loud, rather than quietly outside what anything checks.

use std::error::Error;
use std::num::{NonZeroU32, NonZeroU64, NonZeroUsize};

use mc_script::{
    Attachment, ChunkName, ComponentName, DispatchReport, FaultKind, HostLimits,
    PROVISIONAL_ROUND_BUDGET_CEILING, ScriptFunction, ScriptHost, ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// Interrupt ticks one entry into script may spend.
const DOCUMENTED_CALL_AND_LOOP_BUDGET: u64 = 1_000_000;

/// Bytes one entry may add above the baseline it started from.
const DOCUMENTED_MEMORY_CAP: usize = 256 * 1024;

/// Bytes the whole script state may reach, allocator-enforced.
const DOCUMENTED_MEMORY_BACKSTOP: usize = 16 * 1024 * 1024;

/// Consecutive faults that quarantine an attachment.
const DOCUMENTED_FAULT_THRESHOLD: u32 = 3;

/// Invocations one dispatch round may perform.
const DOCUMENTED_ROUND_BOUND: u32 = 64;

/// Entries of follow-up work that may be waiting at once.
const DOCUMENTED_PENDING_BOUND: u32 = 256;

/// Bytes of script output one host retains over its whole life.
///
/// Every `print` hands the host a string it keeps, and until the block loader
/// there was no production path by which content could reach that buffer at
/// all. A chunk can afford half a million such calls inside its budget, so what
/// is retained is bounded here or nowhere.
const DOCUMENTED_RETAINED_PRINT_BYTES: usize = 256 * 1024;

/// The ceiling stated for `round_bound × call_and_loop_budget`.
///
/// Written here as its own number so that the host's declaration of it is
/// compared against something rather than against itself.
const STATED_ROUND_BUDGET_CEILING: u64 = 64_000_000;

/// A callback whose loop never terminates.
const NEVER_RETURNS: &str = "return function()\n\twhile true do end\nend\n";

/// Iterations that fit inside the shipped budget with about a tenth of it to
/// spare, and that no substantially smaller budget admits.
///
/// A `for` loop of N iterations costs N ticks and its enclosing call one more:
/// measured, 9,000 iterations complete under a budget of 9,001 and are aborted
/// under 9,000. This is what makes the abort below evidence about *this* budget
/// rather than about any finite budget at all — a bare infinite loop is stopped
/// by a budget of ten, and a host that shipped one would look identical.
const A_LOAD_THE_SHIPPED_BUDGET_MUST_ADMIT: i64 = 900_000;

/// The seven documented values, as one record.
///
/// **The absence of `..` is the point.** Spelling every field is what makes a
/// limit added later a build failure here instead of a limit nothing asserts.
fn documented_limits() -> Result<HostLimits, Box<dyn Error>> {
    Ok(HostLimits {
        call_and_loop_budget: NonZeroU64::new(DOCUMENTED_CALL_AND_LOOP_BUDGET)
            .ok_or("the documented call-and-loop budget must not be zero")?,
        memory_cap: NonZeroUsize::new(DOCUMENTED_MEMORY_CAP)
            .ok_or("the documented memory cap must not be zero")?,
        memory_backstop: NonZeroUsize::new(DOCUMENTED_MEMORY_BACKSTOP)
            .ok_or("the documented memory backstop must not be zero")?,
        fault_threshold: NonZeroU32::new(DOCUMENTED_FAULT_THRESHOLD)
            .ok_or("the documented fault threshold must not be zero")?,
        round_bound: NonZeroU32::new(DOCUMENTED_ROUND_BOUND)
            .ok_or("the documented round bound must not be zero")?,
        pending_bound: NonZeroU32::new(DOCUMENTED_PENDING_BOUND)
            .ok_or("the documented pending bound must not be zero")?,
        retained_print_bytes: NonZeroUsize::new(DOCUMENTED_RETAINED_PRINT_BYTES)
            .ok_or("the documented retained-output bound must not be zero")?,
    })
}

/// A host nobody configured.
fn unconfigured_host() -> Result<ScriptHost, Box<dyn Error>> {
    ScriptHost::new().map_err(|error| format!("the host would not start: {error}").into())
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

/// A callback that adds up `iterations` numbers and returns the total.
fn chunk_that_counts_to(iterations: i64) -> String {
    format!(
        "return function()\n\
         \tlocal total = 0\n\
         \tfor index = 1, {iterations} do total = total + index end\n\
         \treturn total\n\
         end\n"
    )
}

/// The total that callback owes, derived here rather than read off a run.
fn total_of(iterations: i64) -> String {
    format!("integer {}", (1..=iterations).sum::<i64>())
}

/// What one invocation handed back, as one comparable line.
fn returned_by(report: &DispatchReport, attachment: &Attachment) -> String {
    match report.results.get(attachment) {
        Some(ScriptValue::Integer(number)) => format!("integer {number}"),
        Some(other) => format!("{other:?}"),
        None => "no result".to_owned(),
    }
}

/// What a fault says about itself, as one comparable record.
///
/// No cause: the text of a host-raised abort is the host's own prose and
/// pinning it here would make rewording it a failure. What has to be carried is
/// the kind and whose code it belongs to.
#[derive(Debug, PartialEq, Eq)]
struct InvocationFault {
    kind: FaultKind,
    chunk: Option<String>,
    subject: Option<String>,
    component: Option<String>,
}

/// The fault an aborted invocation should produce.
fn aborted(chunk: &str, subject: &str, component: &str) -> InvocationFault {
    InvocationFault {
        kind: FaultKind::BudgetExhausted,
        chunk: Some(chunk.to_owned()),
        subject: Some(subject.to_owned()),
        component: Some(component.to_owned()),
    }
}

fn describe_faults(report: &DispatchReport) -> Vec<InvocationFault> {
    report
        .faults
        .iter()
        .map(|fault| InvocationFault {
            kind: fault.kind,
            chunk: fault
                .origin
                .chunk
                .as_ref()
                .map(ChunkName::as_str)
                .map(str::to_owned),
            subject: fault
                .subject
                .as_ref()
                .map(SubjectName::as_str)
                .map(str::to_owned),
            component: fault
                .component
                .as_ref()
                .map(ComponentName::as_str)
                .map(str::to_owned),
        })
        .collect()
}

#[test]
fn a_host_given_no_configuration_reports_every_limit_at_the_value_documented_for_it() -> TestResult
{
    let host = unconfigured_host()?;

    assert_eq!(
        *host.limits(),
        documented_limits()?,
        "these seven numbers are what bounds a server nobody configured, and every other test in \
         this crate replaces the one it is about with a test-sized value — so nothing else here \
         constrains them. The comparison is over the whole record rather than over a few named \
         fields: three of the seven were added after the other four were settled, and a test naming \
         four would have stopped covering exactly those two without saying so. The expected \
         values are written in this file rather than read back from the host, because an \
         expectation taken from the code under test agrees with a budget of one as readily as \
         with the documented one."
    );
    Ok(())
}

#[test]
fn the_round_bound_and_the_budget_together_stay_within_the_ceiling_stated_for_the_pair()
-> TestResult {
    let host = unconfigured_host()?;
    let limits = host.limits();
    let ticks_one_round_may_spend =
        u64::from(limits.round_bound.get()).checked_mul(limits.call_and_loop_budget.get());

    assert_eq!(
        (
            PROVISIONAL_ROUND_BUDGET_CEILING,
            ticks_one_round_may_spend
                .is_some_and(|ticks| ticks <= PROVISIONAL_ROUND_BUDGET_CEILING),
        ),
        (STATED_ROUND_BUDGET_CEILING, true),
        "{WHY_THE_PAIR_NEEDS_A_STATED_CEILING} The pair spends \
         {ticks_one_round_may_spend:?} against a stated {PROVISIONAL_ROUND_BUDGET_CEILING}."
    );
    Ok(())
}

/// Why the pair is asserted against a stated number and not against itself.
const WHY_THE_PAIR_NEEDS_A_STATED_CEILING: &str = "one round may enter script `round_bound` times and each entry may spend a whole budget, so \
     the pair bounds what a single round can cost. The two constraints on the budget do not \
     covary — it must be large enough for the biggest unsliceable chunk and small enough that a \
     round of them fits somewhere — and nothing today can derive the pair jointly, because there \
     is no tick calling dispatch to derive it against. That impossibility is why the ceiling is \
     stated at the pair's present value rather than computed from it: it is not a bound anybody \
     measured, it is a bound that has to be raised on purpose. Asserting the pair is merely \
     non-zero would say nothing whatever, the product of two non-zero values being non-zero.";

/// Why the completing workload is asserted alongside the abort.
const WHY_A_STOPPED_RUNAWAY_IS_HALF_THE_CLAIM: &str = "a number reported by the host is not a number the host enforces: the budget is armed where a \
     host is built, and a host built without configuration has to arm the shipped one rather than \
     nothing. The runaway half is what says a callback that will not stop is stopped and named. \
     The counting half is what says it was stopped by *this* budget — a load sized at about nine \
     tenths of the documented budget completes under it and is aborted under anything much \
     smaller, so a host that shipped a token budget stops the runaway exactly as convincingly and \
     fails here, where a host that shipped the documented one does not. That total \
     is arithmetic done in this file, which is what says the loop ran to the end instead of being \
     cut short somewhere nothing reported.";

#[test]
fn a_callback_that_never_returns_is_stopped_under_the_shipped_budget_and_reported_as_exhausted()
-> TestResult {
    let mut host = unconfigured_host()?;
    let runaway = callback_from(&mut host, "furnace.luau", NEVER_RETURNS)?;
    let counting = callback_from(
        &mut host,
        "hopper.luau",
        &chunk_that_counts_to(A_LOAD_THE_SHIPPED_BUDGET_MUST_ADMIT),
    )?;
    let smelt = attachment("stone-furnace", "smelt");
    let vent = attachment("stone-hopper", "vent");
    host.attach(smelt.clone(), runaway);
    host.attach(vent.clone(), counting);

    let report = host.dispatch(&[smelt.clone(), vent.clone()]);

    assert_eq!(
        (
            describe_faults(&report),
            returned_by(&report, &smelt),
            returned_by(&report, &vent),
        ),
        (
            vec![aborted("furnace.luau", "stone-furnace", "smelt")],
            "no result".to_owned(),
            total_of(A_LOAD_THE_SHIPPED_BUDGET_MUST_ADMIT),
        ),
        "{WHY_A_STOPPED_RUNAWAY_IS_HALF_THE_CLAIM}"
    );
    Ok(())
}
