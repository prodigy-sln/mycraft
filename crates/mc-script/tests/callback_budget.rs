//! The call-and-loop budget, charged to one invocation and to nothing else.
//!
//! # What the budget counts
//!
//! Calls and loop edges, not instructions. The interrupt is emitted at seven
//! opcodes, so the body of a loop is free and a thousand straight-line
//! statements cost one tick. Measured on this toolchain: a loop of 9,000
//! iterations completes under a budget of 9,001 and is aborted under 9,000, and
//! that measurement is what sizes the work below. It is also why the field is not
//! called an instruction budget — anyone sizing it against VM instructions is
//! wrong by the size of the loop body.
//!
//! # Every test here is easy to pass for the wrong reason
//!
//! An abort is easy. What is hard, and what these tests are actually about, is
//! that the budget belongs to *one invocation*: an attachment that exhausted one
//! must not leave the next attachment, or its own next round, running on a
//! remainder. So the work that follows an abort is deliberately sized to be
//! impossible under any remainder and comfortable under a whole budget — 9,000
//! iterations against a budget of 10,000. A host that hands out what is left
//! aborts it; a host that hands out a whole budget returns 40,504,500.
//!
//! Every expected total below is arithmetic performed here rather than a number
//! copied from a run of the host. A snapshotted count records whatever the code
//! happened to do the day it was written.
//!
//! # The protected call is the one that decides whether any of this holds
//!
//! An abort that is merely raised is caught by script and the loop starts again,
//! which was measured against a non-latching implementation: the invocation
//! returned normally and the budget bounded nothing. So the callback in the last
//! test is written to *report* a caught abort rather than to spin quietly — if
//! the abort is catchable it returns a count and this test sees a result where
//! it demanded a fault, which is the difference between an abort that bounds and
//! an abort that annoys.

use std::error::Error;
use std::num::NonZeroU64;

use mc_script::{
    Attachment, ChunkName, ComponentName, DispatchReport, FaultKind, HostLimits, ScriptFunction,
    ScriptHost, ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// The budget every scenario in this file names.
const BUDGET: u64 = 10_000;

/// Iterations that fit inside `BUDGET` with room to spare, and cannot fit inside
/// whatever is left of it after an abort.
const A_LOAD_THAT_NEEDS_A_WHOLE_BUDGET: i64 = 9_000;

/// Iterations well inside the budget, for the case that simply works.
const A_LOAD_THAT_COMFORTABLY_FITS: i64 = 1_000;

/// A callback whose loop never terminates.
const NEVER_RETURNS: &str = "return function()\n\twhile true do end\nend\n";

/// A callback that catches its own abort and reports having caught it.
///
/// Reporting is the point. A callback that merely spins inside its protected
/// call cannot tell a host that latched the abort from a host that let the
/// protected call swallow it and then ran out of patience somewhere else.
const CATCHES_ITS_OWN_ABORT_AND_REPORTS_IT: &str = "return function()\n\
     \tlocal caught = 0\n\
     \twhile true do\n\
     \t\tlocal ok = pcall(function() while true do end end)\n\
     \t\tif not ok then\n\
     \t\t\tcaught = caught + 1\n\
     \t\t\treturn caught\n\
     \t\tend\n\
     \tend\n\
     end\n";

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

/// A callback that never returns the first time it is invoked and counts every
/// time after.
fn chunk_that_runs_away_once_then_counts_to(iterations: i64) -> String {
    format!(
        "local first = true\n\
         return function()\n\
         \tif first then\n\
         \t\tfirst = false\n\
         \t\twhile true do end\n\
         \tend\n\
         \tlocal total = 0\n\
         \tfor index = 1, {iterations} do total = total + index end\n\
         \treturn total\n\
         end\n"
    )
}

/// The total the counting callback owes, derived here rather than observed.
fn total_of(iterations: i64) -> String {
    format!("integer {}", (1..=iterations).sum::<i64>())
}

/// What a fault says about itself, as one comparable record.
///
/// No cause: the text of a host-raised abort is the host's own prose, and
/// pinning it here would make rewording it a test failure. What the fault has to
/// carry is who it belongs to and where their code lives.
#[derive(Debug, PartialEq, Eq)]
struct InvocationFault {
    kind: FaultKind,
    chunk: Option<String>,
    subject: Option<String>,
    component: Option<String>,
    names_a_round: bool,
}

/// The fault an aborted invocation should produce.
fn aborted(chunk: &str, subject: &str, component: &str) -> InvocationFault {
    InvocationFault {
        kind: FaultKind::BudgetExhausted,
        chunk: Some(chunk.to_owned()),
        subject: Some(subject.to_owned()),
        component: Some(component.to_owned()),
        names_a_round: true,
    }
}

/// A host whose call-and-loop budget is the one the scenarios name. Every other
/// limit stays at its shipped default: a second limit tripping first would
/// answer a different question.
fn host_at_the_named_budget() -> Result<ScriptHost, Box<dyn Error>> {
    let budget = NonZeroU64::new(BUDGET).ok_or("the configured budget must not be zero")?;
    let limits = HostLimits {
        call_and_loop_budget: budget,
        ..HostLimits::default()
    };
    ScriptHost::with_limits(limits)
        .map_err(|error| format!("the host refused a budget of {BUDGET}: {error}").into())
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
            names_a_round: fault.origin.round.is_some(),
        })
        .collect()
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

#[test]
fn a_callback_whose_loop_never_terminates_is_aborted_and_the_fault_names_its_attachment()
-> TestResult {
    let mut host = host_at_the_named_budget()?;
    let runaway = callback_from(&mut host, "furnace.luau", NEVER_RETURNS)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), runaway);

    let report = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (describe_faults(&report), result_for(&report, &smelt)),
        (
            vec![aborted("furnace.luau", "stone-furnace", "smelt")],
            "no result".to_owned()
        ),
        "a callback that will not stop is the first hostile shape anybody meets, and stopping \
         it is worth nothing to an operator who cannot tell which mod to remove. The fault \
         names the subject, the component and the file. It also carries no result: an \
         invocation that was stopped did not return, and a host that files a value for it \
         under the attachment's name is reporting something that never happened."
    );
    Ok(())
}

#[test]
fn a_callback_that_finishes_its_loop_inside_the_budget_returns_its_result_and_reports_no_fault()
-> TestResult {
    let mut host = host_at_the_named_budget()?;
    let counting = callback_from(
        &mut host,
        "furnace.luau",
        &chunk_that_counts_to(A_LOAD_THAT_COMFORTABLY_FITS),
    )?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), counting);

    let report = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (result_for(&report, &smelt), describe_faults(&report)),
        (total_of(A_LOAD_THAT_COMFORTABLY_FITS), Vec::new()),
        "the budget has to be invisible to content that stays inside it, and the total is what \
         says the loop actually ran to the end rather than being cut short somewhere the host \
         forgot to report. It is arithmetic done here, not a number read off a run: an expected \
         value copied from the first green run records whatever the host happened to do that \
         day."
    );
    Ok(())
}

/// Why the second attachment's result is the assertion and not its invocation.
const WHY_THE_SECOND_GETS_A_WHOLE_BUDGET: &str = "one mod exhausting its budget must cost the next mod nothing. The second attachment's \
     work needs almost the whole budget, so a host that hands it the remainder of the first \
     one's aborts it and a host that hands it a fresh budget returns the total — which is why \
     the assertion is on the returned value and not merely on the second having been invoked. \
     A host that invoked it and threw the answer away would pass the weaker check.";

#[test]
fn the_second_attachment_of_a_round_gets_a_whole_budget_after_the_first_exhausts_one() -> TestResult
{
    let mut host = host_at_the_named_budget()?;
    let runaway = callback_from(&mut host, "furnace.luau", NEVER_RETURNS)?;
    let counting = callback_from(
        &mut host,
        "hopper.luau",
        &chunk_that_counts_to(A_LOAD_THAT_NEEDS_A_WHOLE_BUDGET),
    )?;
    let smelt = attachment("stone-furnace", "smelt");
    let vent = attachment("stone-hopper", "vent");
    host.attach(smelt.clone(), runaway);
    host.attach(vent.clone(), counting);

    let report = host.dispatch(&[smelt.clone(), vent.clone()]);

    assert_eq!(
        (
            report.order.clone(),
            report.invocations,
            describe_faults(&report),
            result_for(&report, &vent),
        ),
        (
            vec![smelt.clone(), vent.clone()],
            2,
            vec![aborted("furnace.luau", "stone-furnace", "smelt")],
            total_of(A_LOAD_THAT_NEEDS_A_WHOLE_BUDGET),
        ),
        "{WHY_THE_SECOND_GETS_A_WHOLE_BUDGET}"
    );
    Ok(())
}

#[test]
fn an_attachment_aborted_in_one_round_is_granted_a_whole_budget_in_the_next() -> TestResult {
    let mut host = host_at_the_named_budget()?;
    let callback = callback_from(
        &mut host,
        "furnace.luau",
        &chunk_that_runs_away_once_then_counts_to(A_LOAD_THAT_NEEDS_A_WHOLE_BUDGET),
    )?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), callback);

    let aborted_round = host.dispatch(std::slice::from_ref(&smelt));
    let next_round = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (
            describe_faults(&aborted_round),
            describe_faults(&next_round),
            result_for(&next_round, &smelt),
        ),
        (
            vec![aborted("furnace.luau", "stone-furnace", "smelt")],
            Vec::new(),
            total_of(A_LOAD_THAT_NEEDS_A_WHOLE_BUDGET),
        ),
        "the budget is per invocation, which means the abort belongs to the invocation and not \
         to the attachment. A host that never re-arms turns one runaway round into an \
         attachment that is finished forever — a quarantine nobody asked for, applied on the \
         first offence, with no record that it happened. The second round's work is sized so \
         that only a whole budget completes it."
    );
    Ok(())
}

#[test]
fn a_callback_that_catches_its_own_abort_is_still_aborted_and_never_regains_control() -> TestResult
{
    let mut host = host_at_the_named_budget()?;
    let catching = callback_from(
        &mut host,
        "furnace.luau",
        CATCHES_ITS_OWN_ABORT_AND_REPORTS_IT,
    )?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), catching);

    let report = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (describe_faults(&report), result_for(&report, &smelt)),
        (
            vec![aborted("furnace.luau", "stone-furnace", "smelt")],
            "no result".to_owned()
        ),
        "a protected call is reachable from content and has to be, so an abort that script can \
         catch bounds nothing: measured against a non-latching implementation the invocation \
         caught it, carried on, and returned normally. This callback reports a caught abort by \
         returning the number of them, so an abort that is merely raised shows up here as a \
         result rather than as a fault — the difference between the invocation being stopped \
         and the invocation being inconvenienced."
    );
    Ok(())
}
