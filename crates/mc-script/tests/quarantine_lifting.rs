//! The two ways a quarantined attachment comes back, and what the host keeps
//! across both of them.
//!
//! # Attaching over an attachment is what reloading a fixed mod *is*
//!
//! An author whose callback was quarantined fixes it and reloads. Reloading is
//! this operation and nothing else: a new callback registered against the
//! identity that already carries one. A host that replaced the callback and left
//! the quarantine standing would look correct at every step and silently fail at
//! the one thing reloading a broken mod exists to do — the fix would be
//! installed and never run, with nothing to say why.
//!
//! So the assertion is the **returned value of the new callback**, never the
//! fact that a round invoked something. The two callbacks below return different
//! text for exactly that reason: a host that kept the old one and reported
//! success passes every weaker check, and the text is what tells the two hosts
//! apart.
//!
//! # Releasing is the operator's half of the same rule
//!
//! Release lifts the quarantine and leaves the callback where it was — the
//! operator saying *try that one again* without anything being reloaded. There
//! is no matching detach: nothing unloads mods yet, and a method with no caller
//! is a commitment with no consumer.
//!
//! # The invocation count resumes; it does not reset
//!
//! It is cumulative telemetry about the **attachment**, answering *how often has
//! this been asked to run*, and it is a different counter from the
//! consecutive-fault count that already resets on a success. Two counters, two
//! questions. A host that reset this one on release or on replace would erase the
//! evidence of the very episode an operator is looking into, and the number
//! would silently mean something different after a reload than before it. Each
//! test below therefore asserts the count across the lift as well as the
//! behaviour.

use std::error::Error;
use std::num::{NonZeroU32, NonZeroU64};

use mc_script::{
    Attachment, ComponentName, DispatchReport, HostLimits, ScriptFunction, ScriptHost, ScriptValue,
    SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// How many consecutive faults quarantine an attachment.
const FAULT_THRESHOLD: u32 = 3;

/// A budget nothing here can approach; no callback in this file loops.
const BUDGET: u64 = 50_000;

/// A callback that raises every time it is invoked.
const ALWAYS_RAISES: &str = "return function()\n\terror('the furnace is jammed', 0)\nend\n";

/// The callback registered first, and the one that must not answer afterwards.
const THE_ORIGINAL_CALLBACK: &str = "return function()\n\treturn 'the original callback'\nend\n";

/// The callback registered over it.
const THE_REPLACEMENT_CALLBACK: &str =
    "return function()\n\treturn 'the replacement callback'\nend\n";

/// What each of them returns, as the host renders it.
const THE_ORIGINAL_ANSWER: &str = "text the original callback";
const THE_REPLACEMENT_ANSWER: &str = "text the replacement callback";

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

/// Runs the rounds it takes to quarantine `attachment`.
fn quarantine(host: &mut ScriptHost, attachment: &Attachment) {
    for _ in 0..FAULT_THRESHOLD {
        host.dispatch(std::slice::from_ref(attachment));
    }
}

/// What a lifted quarantine looked like: whether the lift reported one, whether
/// the host still holds one, what came back afterwards, and the count across it.
#[derive(Debug, PartialEq, Eq)]
struct Lifted {
    reported_a_quarantine: bool,
    still_quarantined: bool,
    answered_afterwards: String,
    count_at_quarantine: u64,
    count_afterwards: u64,
}

const WHY_A_RELEASED_ATTACHMENT_RESUMES_ITS_COUNT: &str = "release is the operator saying `try that one again`, so the next round has to invoke it \
     — and the count has to carry on from where it stopped rather than starting over. It is \
     cumulative telemetry about the attachment, a different counter from the consecutive-fault \
     tally that already resets on a success, and a host that reset it here would erase the \
     history of the episode the operator is looking into. Exactly one greater than the frozen \
     count, so a host that invoked it twice or not at all is visible either way. The \
     never-quarantined attachment is the control on the return value: a `release` that always \
     answered `true` would tell an operator it had undone something it had not.";

#[test]
fn releasing_a_quarantined_attachment_puts_it_back_in_the_next_round() -> TestResult {
    let mut host = host_at_the_named_threshold()?;
    let jams = callback_from(&mut host, "furnace.luau", ALWAYS_RAISES)?;
    let works = callback_from(&mut host, "hopper.luau", THE_ORIGINAL_CALLBACK)?;
    let smelt = attachment("stone-furnace", "smelt");
    let vent = attachment("stone-hopper", "vent");
    host.attach(smelt.clone(), jams);
    host.attach(vent.clone(), works);

    quarantine(&mut host, &smelt);
    let frozen = host.invocation_count(&smelt);
    let released = host.release(&smelt);
    let never_quarantined = host.release(&vent);
    host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (
            released,
            never_quarantined,
            host.is_quarantined(&smelt),
            host.invocation_count(&smelt),
            frozen,
        ),
        (
            true,
            false,
            false,
            u64::from(FAULT_THRESHOLD) + 1,
            u64::from(FAULT_THRESHOLD)
        ),
        "{WHY_A_RELEASED_ATTACHMENT_RESUMES_ITS_COUNT}"
    );
    Ok(())
}

const WHY_THE_REPLACEMENT_HAS_TO_BE_THE_ONE_THAT_ANSWERS: &str = "attaching over an attachment that already carries a callback is how a mod's behaviour \
     is changed, and the only way to tell a host that replaced it from one that kept the old \
     one is to ask what came back. Both callbacks return, both return promptly, and a host \
     that ignored the new registration would report a perfectly ordinary successful round — \
     so the returned text is the assertion and the fact of invocation is not. The count \
     carries across the replacement for the same reason it carries across a release: it \
     belongs to the attachment, not to whichever callback happens to be registered.";

#[test]
fn a_callback_attached_over_another_is_the_one_that_answers_next_round() -> TestResult {
    let mut host = host_at_the_named_threshold()?;
    let original = callback_from(&mut host, "furnace.luau", THE_ORIGINAL_CALLBACK)?;
    let replacement = callback_from(&mut host, "furnace.luau", THE_REPLACEMENT_CALLBACK)?;
    let smelt = attachment("stone-furnace", "smelt");

    host.attach(smelt.clone(), original);
    let before = host.dispatch(std::slice::from_ref(&smelt));
    host.attach(smelt.clone(), replacement);
    let after = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (
            result_for(&before, &smelt),
            result_for(&after, &smelt),
            host.invocation_count(&smelt),
        ),
        (
            THE_ORIGINAL_ANSWER.to_owned(),
            THE_REPLACEMENT_ANSWER.to_owned(),
            2
        ),
        "{WHY_THE_REPLACEMENT_HAS_TO_BE_THE_ONE_THAT_ANSWERS}"
    );
    Ok(())
}

const WHY_ATTACHING_OVER_A_QUARANTINE_LIFTS_IT: &str = "this is hot reload in miniature: the author whose callback was quarantined fixes it and \
     reloads, and reloading is exactly a new callback registered against an identity that \
     already carries one. A host that installed the fix and left the quarantine standing is \
     the worst shape this mechanism has — the operator sees the reload succeed, the mod stays \
     silent, and nothing anywhere says why. Both halves are the test: no longer quarantined \
     when asked, and the replacement's own answer coming back from the next round rather than \
     the round merely happening.";

#[test]
fn a_callback_attached_over_a_quarantined_one_lifts_the_quarantine_and_answers() -> TestResult {
    let mut host = host_at_the_named_threshold()?;
    let jams = callback_from(&mut host, "furnace.luau", ALWAYS_RAISES)?;
    let replacement = callback_from(&mut host, "furnace.luau", THE_REPLACEMENT_CALLBACK)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), jams);

    quarantine(&mut host, &smelt);
    let reported_a_quarantine = host.is_quarantined(&smelt);
    let count_at_quarantine = host.invocation_count(&smelt);
    host.attach(smelt.clone(), replacement);
    let still_quarantined = host.is_quarantined(&smelt);
    let afterwards = host.dispatch(std::slice::from_ref(&smelt));
    let observed = Lifted {
        reported_a_quarantine,
        still_quarantined,
        answered_afterwards: result_for(&afterwards, &smelt),
        count_at_quarantine,
        count_afterwards: host.invocation_count(&smelt),
    };

    assert_eq!(
        observed,
        Lifted {
            reported_a_quarantine: true,
            still_quarantined: false,
            answered_afterwards: THE_REPLACEMENT_ANSWER.to_owned(),
            count_at_quarantine: u64::from(FAULT_THRESHOLD),
            count_afterwards: u64::from(FAULT_THRESHOLD) + 1,
        },
        "{WHY_ATTACHING_OVER_A_QUARANTINE_LIFTS_IT}"
    );
    Ok(())
}
