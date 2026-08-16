//! What the host takes out of a table script handed it, and what it refuses to
//! run while taking it.
//!
//! # Indexing a table is a call into script, and the caller does not choose when
//!
//! A table content hands back can carry a metatable whose `__index` is a
//! function, and that function is script. A host that reads a field the obvious
//! way runs it — on the host's own schedule, unasked, at whatever moment the
//! engine happened to want a value. Two things follow, and both are the mod's
//! choice rather than the host's: the metamethod runs code the host never
//! decided to run, and it learns **which fields the host asked for**, which is
//! the engine's internal reading order handed to a mod as an observable.
//!
//! So the host reads raw. Nothing else in this file is about anything else.
//!
//! # The probe, and why two of these tests differ by one statement
//!
//! One chunk defines a counter table, a supplied table whose `__index`
//! increments it, and a callback that hands the supplied table back the first
//! time it is invoked and afterwards performs one action and returns what the
//! counter reads. The two probe tests supply that one action — nothing at all,
//! or index the absent field — and nothing else about them differs.
//!
//! That is what makes the second the first's positive control. Without it, a
//! counter reading zero is equally consistent with a host that read raw and with
//! a probe that never fired at all: an increment nothing can trigger reads zero
//! forever. The present-field test is the control in the other direction — a
//! host that answered "absent" to everything would satisfy the raw-read test
//! perfectly and be useless.
//!
//! Everything lives in one chunk because each chunk is evaluated in its own
//! frozen environment and two chunks share nothing. The counter reaches the host
//! as a **returned value** and the metamethod's own answer reaches it through
//! the host's `print` — two host-side observables that do not lean on the very
//! method under test. An assertion that read `read_field` to check `read_field`
//! would prove nothing.
//!
//! # An interface decision taken here: absent is `None`, never `Some(Nil)`
//!
//! In script a field that was never set and a field set to nothing are one
//! state, so the host has exactly one honest answer for both and the return type
//! has a place to put it. `Some(ScriptValue::Nil)` would make the `Option` say
//! nothing — every read would be `Some` — and the caller that has to branch on
//! "the mod did not supply this" would be reading a value to find out.
//!
//! # The looping metamethod, and what its second half is really for
//!
//! A metamethod written to loop forever cannot tell anything on its own, and
//! this was measured rather than reasoned about: an ordinary indexed read runs
//! it, the interrupt charges it against whatever the last entry left behind, and
//! it comes back as an error the read swallows — so the read *completes*, the
//! attachment is fine afterwards, and a host doing precisely what this file
//! exists to forbid satisfies the scenario. The looping metamethod therefore
//! **says one line before it loops**, and that line not having been printed is
//! the witness the scenario's own wording does not supply.
//!
//! The rest of the scenario is what happens **afterwards** — the same attachment
//! runs 9,000 iterations under its 10,000 budget and returns the total. That is
//! deliberately tight: 9,000 iterations costs 9,001 ticks, measured on this
//! toolchain and recorded in `callback_budget.rs`, so under a thousand ticks
//! separate completing from being aborted and anything the host's read charged,
//! latched or left behind would show. The total is arithmetic performed here,
//! never a number read back from a run.

use std::error::Error;
use std::num::NonZeroU64;

use mc_script::{
    Attachment, ComponentName, DispatchReport, FaultKind, HostLimits, ScriptFunction, ScriptHost,
    ScriptTable, ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// The call-and-loop budget every test in this file runs under.
const BUDGET: u64 = 10_000;

/// Iterations that fit inside [`BUDGET`] with a few hundred ticks to spare and
/// fit inside no remainder of one at all.
const A_LOAD_THAT_NEEDS_A_WHOLE_BUDGET: i64 = 9_000;

/// A field the supplied table genuinely holds.
const A_FIELD_THE_TABLE_HAS: &str = "fuel";

/// What that field holds.
const WHAT_THAT_FIELD_HOLDS: &str = "still burning";

/// A field the supplied table does not hold, so that reading it is the moment a
/// metatable would get its turn.
const A_FIELD_THE_TABLE_LACKS: &str = "ash";

/// What the metatable offers for the field the table lacks, whether it offers it
/// through a function or through a second table.
const WHAT_THE_METATABLE_WOULD_SUPPLY: &str = "from the metatable";

/// What the metamethod that never returns says before it stops returning.
///
/// It is the whole reason that test can fail. Measured: a host reading the field
/// the ordinary way runs this metamethod, has it stopped by the interrupt, and
/// swallows the refusal — leaving a read that completed and an attachment that
/// works, which is the scenario satisfied by the implementation it forbids.
const WHAT_THE_LOOPING_METAMETHOD_SAYS_FIRST: &str = "the metamethod got its turn";

/// The probe's callback does nothing of its own, for the tests measuring the
/// host's read alone.
const NOTHING_ELSE: &str = "";

/// The one statement that separates the two probe tests: the chunk indexes the
/// same absent field on the same table, itself.
fn indexing_the_field_the_table_lacks() -> String {
    format!("print(supplied.{A_FIELD_THE_TABLE_LACKS})")
}

/// A chunk whose callback hands back a table with a counting `__index`, and
/// afterwards performs `action` and reports what the counter reads.
///
/// The counter, the metatable and the table share one chunk because they have
/// to: a per-chunk frozen environment means a second chunk can see none of them.
fn chunk_whose_callback_supplies_a_table_and_then(action: &str) -> String {
    format!(
        "local counter = {{ hits = 0 }}\n\
         local supplied = setmetatable({{ {A_FIELD_THE_TABLE_HAS} = '{WHAT_THAT_FIELD_HOLDS}' }}, {{\n\
         \t__index = function()\n\
         \t\tcounter.hits = counter.hits + 1\n\
         \t\treturn '{WHAT_THE_METATABLE_WOULD_SUPPLY}'\n\
         \tend,\n\
         }})\n\
         local first = true\n\
         return function()\n\
         \tif first then\n\
         \t\tfirst = false\n\
         \t\treturn supplied\n\
         \tend\n\
         \t{action}\n\
         \treturn counter.hits\n\
         end\n"
    )
}

/// A chunk whose callback hands back a table whose `__index` never returns, and
/// which afterwards does a load only a whole budget completes.
fn chunk_whose_callback_supplies_a_table_that_never_answers() -> String {
    format!(
        "local supplied = setmetatable({{}}, {{\n\
         \t__index = function()\n\
         \t\tprint('{WHAT_THE_LOOPING_METAMETHOD_SAYS_FIRST}')\n\
         \t\twhile true do end\n\
         \tend,\n\
         }})\n\
         local first = true\n\
         return function()\n\
         \tif first then\n\
         \t\tfirst = false\n\
         \t\treturn supplied\n\
         \tend\n\
         \tlocal total = 0\n\
         \tfor index = 1, {A_LOAD_THAT_NEEDS_A_WHOLE_BUDGET} do total = total + index end\n\
         \treturn total\n\
         end\n"
    )
}

/// A chunk whose callback hands back a table that reads through to a second one
/// rather than to a function.
fn chunk_whose_callback_supplies_a_table_reading_through_to_another() -> String {
    format!(
        "local behind = {{ {A_FIELD_THE_TABLE_LACKS} = '{WHAT_THE_METATABLE_WOULD_SUPPLY}' }}\n\
         local supplied = setmetatable(\n\
         \t{{ {A_FIELD_THE_TABLE_HAS} = '{WHAT_THAT_FIELD_HOLDS}' }},\n\
         \t{{ __index = behind }}\n\
         )\n\
         return function()\n\
         \treturn supplied\n\
         end\n"
    )
}

/// A host whose call-and-loop budget is the one this file names. Every other
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

/// The table an invocation handed back, for the host to read fields from.
fn table_from(
    report: &DispatchReport,
    attachment: &Attachment,
) -> Result<ScriptTable, Box<dyn Error>> {
    match report.results.get(attachment) {
        Some(ScriptValue::Table(supplied)) => Ok(supplied.clone()),
        other => {
            Err(format!("the callback was written to hand back a table, not {other:?}").into())
        }
    }
}

/// One value the host holds, rendered with its kind, so text and a number
/// spelling the same characters do not compare equal.
fn described(value: &ScriptValue) -> String {
    match value {
        ScriptValue::Nil => "nil".to_owned(),
        ScriptValue::Boolean(flag) => format!("boolean {flag}"),
        ScriptValue::Integer(number) => format!("integer {number}"),
        ScriptValue::Number(number) => format!("number {number}"),
        ScriptValue::Text(text) => format!("text {text}"),
        ScriptValue::Table(_) => "table".to_owned(),
        ScriptValue::Function(_) => "function".to_owned(),
        ScriptValue::Opaque => "opaque".to_owned(),
    }
}

/// What the host's read of a field answered.
///
/// A field the table does not hold reads `absent`, which is a different answer
/// from `nil` on purpose — see the note in this file's header.
fn read(value: Option<ScriptValue>) -> String {
    value
        .as_ref()
        .map_or_else(|| "absent".to_owned(), described)
}

/// What one invocation handed back, as one comparable line.
fn result_for(report: &DispatchReport, attachment: &Attachment) -> String {
    report
        .results
        .get(attachment)
        .map_or_else(|| "no result".to_owned(), described)
}

/// How text reads once the host holds it.
fn text(content: &str) -> String {
    format!("text {content}")
}

/// How the counter reads once the host holds it.
fn counter_reading(hits: i64) -> String {
    format!("integer {hits}")
}

/// The total the counting callback owes, derived here rather than observed.
fn total_of(iterations: i64) -> String {
    format!("integer {}", (1..=iterations).sum::<i64>())
}

/// What the probe observed: what the host's read answered, what the counter read
/// afterwards, and what the chunk printed along the way.
#[derive(Debug, PartialEq, Eq)]
struct RawRead {
    host_read: String,
    counter_afterwards: String,
    printed: Vec<String>,
}

/// Hands the host a table, has it read `field`, and reports what the metamethod
/// counter says about it afterwards.
///
/// `action` is the one statement the callback performs on its second invocation,
/// before reporting the counter.
fn probe(action: &str, field: &str) -> Result<RawRead, Box<dyn Error>> {
    let mut host = host_at_the_named_budget()?;
    let callback = callback_from(
        &mut host,
        "furnace.luau",
        &chunk_whose_callback_supplies_a_table_and_then(action),
    )?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), callback);

    let supplied = table_from(&host.dispatch(std::slice::from_ref(&smelt)), &smelt)?;
    let host_read = read(host.read_field(&supplied, field));
    let afterwards = host.dispatch(std::slice::from_ref(&smelt));

    Ok(RawRead {
        host_read,
        counter_afterwards: result_for(&afterwards, &smelt),
        printed: host.printed().to_vec(),
    })
}

#[test]
fn a_field_the_table_lacks_is_reported_absent_without_its_index_metamethod_running() -> TestResult {
    assert_eq!(
        probe(NOTHING_ELSE, A_FIELD_THE_TABLE_LACKS)?,
        RawRead {
            host_read: "absent".to_owned(),
            counter_afterwards: counter_reading(0),
            printed: Vec::new(),
        },
        "the metamethod is script, and a host that reaches it while reading a field has handed \
         the mod an unbudgeted call it chose the moment of — and told it which field the engine \
         wanted, which is the engine's own reading order becoming a mod's observable. A \
         non-raw read hands back the metamethod's answer instead of absence, so this reddens on \
         both halves at once: the counter reads 1 and the field the table does not have arrives \
         carrying a value nobody stored."
    );
    Ok(())
}

#[test]
fn a_field_the_table_genuinely_has_comes_back_with_its_value() -> TestResult {
    assert_eq!(
        probe(NOTHING_ELSE, A_FIELD_THE_TABLE_HAS)?,
        RawRead {
            host_read: text(WHAT_THAT_FIELD_HOLDS),
            counter_afterwards: counter_reading(0),
            printed: Vec::new(),
        },
        "this is the control in the direction the other tests cannot see. A host that answered \
         `absent` to every field would satisfy the raw-read test perfectly, run no metamethod \
         ever, and be unable to read a single thing content supplied it — which is the whole \
         point of holding a table handle. The value is asserted rather than merely its \
         presence, because a host reading the right field of the wrong table, or the wrong \
         field of the right one, is `Some` either way."
    );
    Ok(())
}

/// Why the counter reading zero next door means anything at all.
const WHY_THE_CHUNKS_OWN_READ_IS_THE_CONTROL: &str = "a probe that cannot fire reads zero forever, and a host that read raw and a chunk whose \
     metamethod was never reachable leave the counter saying exactly the same thing. The two \
     chunks differ by one statement — the host reads the field, or the host reads it and the \
     chunk reads it too — so an increment here is what proves the increment next door was \
     available and declined. *Exactly* once, not merely non-zero: a metamethod fired twice for \
     one indexing would mean the counter counts something other than what this claims. The \
     printed line is the second half, and it is the metamethod's own answer coming back rather \
     than a number somebody could have written by hand.";

#[test]
fn a_chunk_that_indexes_the_same_absent_field_itself_leaves_the_counter_reading_one() -> TestResult
{
    assert_eq!(
        probe(
            &indexing_the_field_the_table_lacks(),
            A_FIELD_THE_TABLE_LACKS
        )?,
        RawRead {
            host_read: "absent".to_owned(),
            counter_afterwards: counter_reading(1),
            printed: vec![WHAT_THE_METATABLE_WOULD_SUPPLY.to_owned()],
        },
        "{WHY_THE_CHUNKS_OWN_READ_IS_THE_CONTROL}"
    );
    Ok(())
}

/// Why the work that follows the read is the assertion, and not the read alone.
const WHY_THE_ATTACHMENT_HAS_TO_STILL_WORK: &str = "the read completing says nothing on its own, and that is measured rather than supposed: a \
     host reading this field the ordinary way runs the metamethod, has it stopped by the \
     interrupt on whatever the last entry left behind, swallows the refusal, and comes back \
     with a read that completed and an attachment that still works. So the metamethod speaks \
     before it loops, and its silence is what says it never got its turn. The rest is what the \
     attachment can do next: its work needs 9,001 ticks of a 10,000 budget, so under a thousand \
     separate completing from being aborted, and a host that charged the read to the \
     attachment, latched its guard, or left the state part-way through something aborts it \
     here. The total is arithmetic done in this file, so a host that invoked the callback and \
     threw the answer away fails too.";

#[test]
fn a_table_whose_index_never_returns_is_read_and_leaves_its_attachment_a_whole_budget() -> TestResult
{
    let mut host = host_at_the_named_budget()?;
    let callback = callback_from(
        &mut host,
        "furnace.luau",
        &chunk_whose_callback_supplies_a_table_that_never_answers(),
    )?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), callback);

    let supplied = table_from(&host.dispatch(std::slice::from_ref(&smelt)), &smelt)?;
    let host_read = read(host.read_field(&supplied, A_FIELD_THE_TABLE_LACKS));
    let afterwards = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (
            host_read,
            host.printed().to_vec(),
            result_for(&afterwards, &smelt),
            afterwards.faults.iter().map(|fault| fault.kind).collect(),
        ),
        (
            "absent".to_owned(),
            Vec::<String>::new(),
            total_of(A_LOAD_THAT_NEEDS_A_WHOLE_BUDGET),
            Vec::<FaultKind>::new(),
        ),
        "{WHY_THE_ATTACHMENT_HAS_TO_STILL_WORK}"
    );
    Ok(())
}

#[test]
fn a_field_that_exists_only_behind_the_metatable_is_reported_absent_as_well() -> TestResult {
    let mut host = host_at_the_named_budget()?;
    let callback = callback_from(
        &mut host,
        "hopper.luau",
        &chunk_whose_callback_supplies_a_table_reading_through_to_another(),
    )?;
    let vent = attachment("stone-hopper", "vent");
    host.attach(vent.clone(), callback);

    let supplied = table_from(&host.dispatch(std::slice::from_ref(&vent)), &vent)?;

    assert_eq!(
        (
            read(host.read_field(&supplied, A_FIELD_THE_TABLE_LACKS)),
            read(host.read_field(&supplied, A_FIELD_THE_TABLE_HAS)),
        ),
        ("absent".to_owned(), text(WHAT_THAT_FIELD_HOLDS)),
        "the counter probe cannot be built this way — a table cannot count — so the only \
         observable a metatable pointing at another table leaves is the value itself, and a \
         host that special-cased a function `__index` while still reading through a table one \
         would satisfy every other test here. What it would hand the engine is a field no mod \
         stored, arriving as though one had, which is precisely what the follow-up list the \
         cascade reads out of a returned table must never contain. The present field is read \
         through the same handle in the same test, so `absent` cannot be a handle that stopped \
         working."
    );
    Ok(())
}
