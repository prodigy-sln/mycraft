//! What the host reports when a callback raises, and what it refuses to run
//! while reporting it.
//!
//! # Reporting a fault must not be a second entry into script
//!
//! Script can raise any value, including a table carrying a `__tostring`
//! metamethod — which is script. A host that renders a raised value the obvious
//! way runs that metamethod, on the host's own schedule, at exactly the moment
//! it is reporting that mod's failure. It is unbudgeted, it is re-entrant, and
//! the mod chooses when it happens.
//!
//! It is not hypothetical and it is not avoided by declining to call `tostring`
//! in host code: **measured, the backend installs a message handler for every
//! protected call it makes, and that handler renders the error value** — so a
//! host that simply calls the callback has already run the metamethod before it
//! ever sees the error. The mechanism that avoids it is a script-side protected
//! call the host invokes callbacks through, so a raised value comes back as an
//! ordinary return value and no message handler is ever reached.
//!
//! Which is why the counter probe below is the construction and a metamethod
//! that loops forever is not an acceptable substitute for it: a host that *did*
//! invoke a looping metamethod would have it aborted by the budget and would
//! still report a fault, so the looping version is green either way.
//!
//! # The probe, and why the two probe tests differ by one statement
//!
//! One chunk defines a counter table, a raised table whose `__tostring`
//! increments it, and a callback that performs one action the first time it is
//! invoked and afterwards returns what the counter reads. The two probe tests
//! supply that one action — raise the table, or convert it to a string — and
//! nothing else about them differs.
//!
//! That is what makes the second the first's positive control. Without it, a
//! counter reading zero is equally consistent with a host that never rendered
//! the raised value and with a probe that never fired at all: an increment
//! nothing can trigger reads zero forever.
//!
//! Everything has to live in one chunk because each chunk is evaluated in its
//! own frozen environment and two chunks share nothing. The counter reaches the
//! host as a **returned value**, and the conversion's own output reaches it
//! through the host's `print` — both host-side observables that need no way to
//! read a field of a script table, which does not exist yet.
//!
//! # A fault names the chunk that defined the callback
//!
//! Not the round it happened in. The round is where the engine was; the chunk is
//! where the author has to look, and an invocation fault is the most common
//! fault this system produces. Two chunks faulting in one round is the shape
//! that catches a host naming the round's chunk, the last chunk evaluated, or
//! nothing at all.

use std::error::Error;

use mc_script::{
    Attachment, ChunkName, ComponentName, DispatchReport, FaultKind, ScriptFunction, ScriptHost,
    ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// A callback that raises text of its own.
///
/// The second argument to `error` suppresses the position prefix the backend
/// would otherwise splice onto the front of the message. That keeps this test's
/// assertion about *the text script raised* rather than about how a pre-1.0
/// dependency spells a location — which is a fact the typed `line` field carries
/// and which `chunk_evaluation.rs` already pins.
const RAISES_TEXT: &str = "return function()\n\terror('the furnace is jammed', 0)\nend\n";

/// A second callback raising different text, for telling two faults apart.
const RAISES_OTHER_TEXT: &str = "return function()\n\terror('the hopper is stuck', 0)\nend\n";

/// A callback that works, for asking whether the host still serves afterwards.
const RETURNS_TEXT: &str = "return function()\n\treturn 'still serving'\nend\n";

/// The action the probe's callback takes the first time it is invoked, when the
/// probe is measuring whether the **host** rendered the raised table.
const RAISE_THE_TABLE: &str = "error(raised)";

/// The same, when the probe is measuring whether the metamethod fires at all.
const CONVERT_THE_TABLE_TO_TEXT: &str = "print(tostring(raised))";

/// A chunk whose callback performs `action` once and afterwards reports what the
/// metamethod counter reads.
///
/// The counter, the raised table and the callback share one chunk because they
/// have to: a per-chunk environment means a second chunk can see none of them.
fn chunk_whose_callback_first(action: &str) -> String {
    format!(
        "local counter = {{ hits = 0 }}\n\
         local raised = setmetatable({{}}, {{\n\
         \t__tostring = function()\n\
         \t\tcounter.hits = counter.hits + 1\n\
         \t\treturn 'rendered'\n\
         \tend,\n\
         }})\n\
         local first = true\n\
         return function()\n\
         \tif first then\n\
         \t\tfirst = false\n\
         \t\t{action}\n\
         \tend\n\
         \treturn counter.hits\n\
         end\n"
    )
}

/// What a fault says about itself, as one comparable record.
///
/// A record rather than a handful of separate assertions, so one comparison
/// reports every field at once and a host that got three of them right is not
/// mistaken for one that got them all right.
#[derive(Debug, PartialEq, Eq)]
struct InvocationFault {
    kind: FaultKind,
    chunk: Option<String>,
    subject: Option<String>,
    component: Option<String>,
    cause: String,
    names_a_round: bool,
}

fn host() -> Result<ScriptHost, Box<dyn Error>> {
    ScriptHost::new().map_err(|error| format!("the host refused to start: {error}").into())
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
            cause: fault.cause.clone(),
            names_a_round: fault.origin.round.is_some(),
        })
        .collect()
}

/// Whether every fault in a report was raised in the same dispatch round.
///
/// A report carrying one fault, or none, agrees with itself — so this says
/// nothing on its own and is only ever asserted beside the faults themselves.
/// What it rules out is a host that satisfied "two chunks, two names" by running
/// the two callbacks in two rounds, which is a different claim from the one the
/// scenario makes.
fn all_faults_share_one_round(report: &DispatchReport) -> bool {
    let rounds: Vec<_> = report
        .faults
        .iter()
        .map(|fault| fault.origin.round)
        .collect();
    rounds.windows(2).all(|pair| pair.first() == pair.last())
}

/// A fault a raising callback should produce.
fn raised(chunk: &str, subject: &str, component: &str, cause: &str) -> InvocationFault {
    InvocationFault {
        kind: FaultKind::ScriptError,
        chunk: Some(chunk.to_owned()),
        subject: Some(subject.to_owned()),
        component: Some(component.to_owned()),
        cause: cause.to_owned(),
        names_a_round: true,
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

/// What the probe observed: how the first round ended, what the counter read
/// afterwards, and what content printed along the way.
#[derive(Debug, PartialEq, Eq)]
struct MetamethodProbe {
    first_round: Vec<FaultKind>,
    counter_afterwards: String,
    printed: Vec<String>,
}

/// Runs the probe chunk for `action` over two rounds and reports what it saw.
fn probe(action: &str) -> Result<MetamethodProbe, Box<dyn Error>> {
    let mut host = host()?;
    let callback = callback_from(
        &mut host,
        "furnace.luau",
        &chunk_whose_callback_first(action),
    )?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), callback);

    let first = host.dispatch(std::slice::from_ref(&smelt));
    let second = host.dispatch(std::slice::from_ref(&smelt));

    Ok(MetamethodProbe {
        first_round: first.faults.iter().map(|fault| fault.kind).collect(),
        counter_afterwards: result_for(&second, &smelt),
        printed: host.printed().to_vec(),
    })
}

#[test]
fn a_callback_that_raises_is_reported_against_its_attachment_with_the_text_it_raised() -> TestResult
{
    let mut host = host()?;
    let jams = callback_from(&mut host, "furnace.luau", RAISES_TEXT)?;
    let works = callback_from(&mut host, "hopper.luau", RETURNS_TEXT)?;
    let smelt = attachment("stone-furnace", "smelt");
    let vent = attachment("stone-hopper", "vent");
    host.attach(smelt.clone(), jams);
    host.attach(vent.clone(), works);

    let faulted = host.dispatch(std::slice::from_ref(&smelt));
    let afterwards = host.dispatch(std::slice::from_ref(&vent));

    assert_eq!(
        (describe_faults(&faulted), result_for(&afterwards, &vent)),
        (
            vec![raised(
                "furnace.luau",
                "stone-furnace",
                "smelt",
                "the furnace is jammed"
            )],
            "text still serving".to_owned()
        ),
        "an operator reading this fault has to be able to act on it, which means all three of \
         who failed, what they said, and where their code lives. The text is what the script \
         raised and nothing else — a cause that arrives empty and a cause that was never \
         populated read identically, and this is the one fault kind whose text comes from \
         script rather than from the host. Control returning to the caller is the other half: \
         the round after a raise still invokes another attachment and still hands back what it \
         returned."
    );
    Ok(())
}

#[test]
fn a_raised_table_is_reported_without_its_string_metamethod_ever_running() -> TestResult {
    assert_eq!(
        probe(RAISE_THE_TABLE)?,
        MetamethodProbe {
            first_round: vec![FaultKind::ScriptError],
            counter_afterwards: "integer 0".to_owned(),
            printed: Vec::new(),
        },
        "the metamethod is script, and running it while reporting a fault hands a mod an \
         unbudgeted, re-entrant call into the host at a moment the mod chooses. Measured, the \
         backend's own message handler renders the raised value before the host sees the error, \
         so a host that merely calls the callback has already run it — the counter reads 1 and \
         nothing about the fault looks wrong. The fault still has to be reported: a host that \
         avoids the metamethod by dropping the failure on the floor is a worse answer than the \
         one this test exists to reject."
    );
    Ok(())
}

#[test]
fn a_chunk_that_converts_the_same_table_itself_leaves_the_counter_reading_one() -> TestResult {
    assert_eq!(
        probe(CONVERT_THE_TABLE_TO_TEXT)?,
        MetamethodProbe {
            first_round: Vec::new(),
            counter_afterwards: "integer 1".to_owned(),
            printed: vec!["rendered".to_owned()],
        },
        "this is what makes the counter reading zero next door mean something. A probe that \
         cannot fire reads zero forever, and the two chunks differ by exactly one statement — \
         raise the table, or convert it — so a zero here and a zero there would say the \
         metamethod is unreachable rather than that the host declined to run it. The printed \
         line is the second half: it is the metamethod's own return value coming back, which a \
         counter alone cannot distinguish from an increment somebody wrote by hand."
    );
    Ok(())
}

/// Why one round's faults have to name two different files.
const WHY_EACH_FAULT_NAMES_ITS_OWN_CHUNK: &str = "the chunk reaches the fault from the callback that failed, not from the round it failed \
     in — so two callbacks defined by two chunks and failing in one round name two different \
     files. A host that stamps the round's own bookkeeping onto the fault names one file for \
     both, or none for either, and either way the most common fault in the system sends its \
     author nowhere. Both faults still belong to one round, which is what stops this passing \
     against a host that ran them in two.";

#[test]
fn two_callbacks_faulting_in_one_round_each_name_the_chunk_that_defined_them() -> TestResult {
    let mut host = host()?;
    let furnace = callback_from(&mut host, "furnace.luau", RAISES_TEXT)?;
    let hopper = callback_from(&mut host, "hopper.luau", RAISES_OTHER_TEXT)?;
    let smelt = attachment("stone-furnace", "smelt");
    let vent = attachment("stone-hopper", "vent");
    host.attach(smelt.clone(), furnace);
    host.attach(vent.clone(), hopper);

    let report = host.dispatch(&[smelt, vent]);

    assert_eq!(
        (
            describe_faults(&report),
            all_faults_share_one_round(&report)
        ),
        (
            vec![
                raised(
                    "furnace.luau",
                    "stone-furnace",
                    "smelt",
                    "the furnace is jammed"
                ),
                raised("hopper.luau", "stone-hopper", "vent", "the hopper is stuck"),
            ],
            true
        ),
        "{WHY_EACH_FAULT_NAMES_ITS_OWN_CHUNK}"
    );
    Ok(())
}
