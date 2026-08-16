//! Quarantine acts on the pair, and on nothing wider.
//!
//! # Why the unit is the pair and not either half of it
//!
//! A component is a behaviour a mod attaches; a subject is what it attaches to.
//! Disabling a component everywhere would let one mod's mistake on one block
//! silence that mod on every block. Disabling a subject would let one mod's
//! mistake silence every *other* mod's behaviour on the block they share — which
//! is the interop case this project's composition model exists to support, where
//! a third party attaches to a subject whose author never heard of them.
//!
//! So the unit is the pair, and the two tests below are the two halves of that
//! claim rather than two versions of one test. One holds the subject fixed and
//! changes the component; the other holds the component fixed and changes the
//! subject. A host that keyed quarantine on the component alone passes the first
//! and fails the second; a host that keyed it on the subject alone does the
//! reverse. Neither test can see its own blind spot, which is why both exist.
//!
//! # What is asserted about the attachment that keeps working
//!
//! Its **returned value**, from a round after the other one was quarantined. A
//! host that invoked it and discarded the answer would satisfy an invocation
//! count, and an invocation count is what a host that stopped reporting results
//! also satisfies. Both are asserted; the value is the one that carries the
//! test.

use std::error::Error;
use std::num::{NonZeroU32, NonZeroU64};

use mc_script::{
    Attachment, ComponentName, DispatchReport, HostLimits, ScriptFunction, ScriptHost, ScriptValue,
    SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// How many consecutive faults quarantine an attachment.
const FAULT_THRESHOLD: u32 = 3;

/// A budget nothing here can approach; neither callback loops.
const BUDGET: u64 = 50_000;

/// A callback that raises every time it is invoked.
const ALWAYS_RAISES: &str = "return function()\n\terror('the furnace is jammed', 0)\nend\n";

/// A callback that works every time it is invoked.
const ALWAYS_WORKS: &str = "return function()\n\treturn 'still working'\nend\n";

/// What the working attachment returns, as the host renders it.
const WHAT_STILL_WORKS_RETURNS: &str = "text still working";

/// What the two attachments looked like after the broken one was quarantined.
#[derive(Debug, PartialEq, Eq)]
struct Isolation {
    broken_quarantined: bool,
    working_quarantined: bool,
    working_result: String,
    broken_invocations: u64,
    working_invocations: u64,
}

/// What isolation owes: one stopped, one untouched, and its answer still coming
/// back.
fn one_stopped_and_one_untouched() -> Isolation {
    Isolation {
        broken_quarantined: true,
        working_quarantined: false,
        working_result: WHAT_STILL_WORKS_RETURNS.to_owned(),
        broken_invocations: u64::from(FAULT_THRESHOLD),
        working_invocations: u64::from(FAULT_THRESHOLD) + 1,
    }
}

fn attachment(subject: &str, component: &str) -> Attachment {
    Attachment {
        subject: SubjectName::new(subject),
        component: ComponentName::new(component),
    }
}

fn host_at_the_named_threshold() -> Result<ScriptHost, Box<dyn Error>> {
    let limits = HostLimits {
        call_and_loop_budget: NonZeroU64::new(BUDGET).ok_or("the budget must not be zero")?,
        fault_threshold: NonZeroU32::new(FAULT_THRESHOLD)
            .ok_or("the fault threshold must not be zero")?,
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

/// Runs both attachments together until one is quarantined, then runs one more
/// round and reports what became of each.
///
/// The extra round is where the whole question lives: everything before it is
/// true of a host with no isolation at all.
fn after_quarantining_one_of(
    broken: &Attachment,
    working: &Attachment,
) -> Result<Isolation, Box<dyn Error>> {
    let mut host = host_at_the_named_threshold()?;
    let jams = callback_from(&mut host, "furnace.luau", ALWAYS_RAISES)?;
    let works = callback_from(&mut host, "hopper.luau", ALWAYS_WORKS)?;
    host.attach(broken.clone(), jams);
    host.attach(working.clone(), works);

    let round = [broken.clone(), working.clone()];
    for _ in 0..FAULT_THRESHOLD {
        host.dispatch(&round);
    }
    let afterwards = host.dispatch(&round);

    Ok(Isolation {
        broken_quarantined: host.is_quarantined(broken),
        working_quarantined: host.is_quarantined(working),
        working_result: result_for(&afterwards, working),
        broken_invocations: host.invocation_count(broken),
        working_invocations: host.invocation_count(working),
    })
}

const WHY_THE_OTHER_COMPONENT_ON_THE_SUBJECT_KEEPS_RUNNING: &str = "two mods attaching behaviour to one block is the ordinary case, not the exotic one, and \
     the whole reason this engine composes rather than subclasses is that a third party can \
     attach to a subject whose author never heard of them. A host that quarantined the subject \
     would let either of them break the other, and the mod that stopped working would have no \
     defect of its own to find. The result of the round *after* the quarantine is the \
     assertion, because everything before it is equally true of a host with no isolation at \
     all — and the invocation counts beside it separate a host that kept invoking from one \
     that kept reporting.";

#[test]
fn another_component_on_a_subject_keeps_running_after_one_of_them_is_quarantined() -> TestResult {
    let smelt = attachment("stone-furnace", "smelt");
    let vent = attachment("stone-furnace", "vent");

    assert_eq!(
        after_quarantining_one_of(&smelt, &vent)?,
        one_stopped_and_one_untouched(),
        "{WHY_THE_OTHER_COMPONENT_ON_THE_SUBJECT_KEEPS_RUNNING}"
    );
    Ok(())
}

const WHY_THE_SAME_COMPONENT_ELSEWHERE_KEEPS_RUNNING: &str = "one mod's behaviour is attached to many subjects, and the mistake that breaks it is \
     usually about one of them — a recipe missing for one block, a field absent on one entity. \
     A host that quarantined the component would take that mod out of the game everywhere \
     because of one subject, which is a far coarser punishment than the failure deserves and \
     one the operator sees as the mod being broken rather than as one attachment being \
     broken. This is the other half of the pair claim: a host keyed on the component alone \
     passes the sibling test next door and fails here.";

#[test]
fn the_same_component_on_another_subject_keeps_running_after_one_of_them_is_quarantined()
-> TestResult {
    let on_stone = attachment("stone-furnace", "smelt");
    let on_iron = attachment("iron-furnace", "smelt");

    assert_eq!(
        after_quarantining_one_of(&on_stone, &on_iron)?,
        one_stopped_and_one_untouched(),
        "{WHY_THE_SAME_COMPONENT_ELSEWHERE_KEEPS_RUNNING}"
    );
    Ok(())
}
