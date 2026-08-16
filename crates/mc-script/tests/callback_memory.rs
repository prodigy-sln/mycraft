//! What one entry into script may allocate, what happens when it asks for more,
//! and what the host does with the memory afterwards.
//!
//! # The cap is a delta above the baseline the entry started from
//!
//! One state serves every attachment, so the only usage figure available is the
//! whole state's. A cap read as an absolute would therefore charge each
//! invocation for every byte every other mod is holding, and the first fault
//! would name whoever happened to be running. The cap is a delta above the
//! entry's own baseline instead, which is what makes an allocation fault
//! attributable to the attachment that caused it.
//!
//! # Two limits that mask each other, and the masking is measured
//!
//! Filling a megabyte costs far more interrupt ticks than the 10,000 the
//! call-and-loop scenarios name, so under that budget an allocation bomb is
//! stopped for **budget exhaustion** and every test in this file goes green
//! having measured the wrong mechanism. The budget here is a million for that
//! reason and for no other: it has to be a number the work below cannot reach,
//! so that the only limit left to stop the bomb is the one these tests are
//! about. A file like this one passing under a nominal budget says nothing.
//!
//! # The bombs are bounded, and that is not timidity
//!
//! A bomb written as `while true do` allocates until *something* stops it —
//! and if nothing does, the something is the machine. Against a host with no
//! cap the test would then take the run down with it rather than failing, which
//! reports nothing about which mechanism is missing. Each bomb below therefore
//! asks for a fixed multiple of the cap: a host that enforces the cap stops it
//! partway and reports a fault, and a host that enforces nothing *returns* —
//! visibly, quickly, and with the machine intact.
//!
//! # Why the backstop sits where it does
//!
//! It has to be high enough that the bomb latches on the enforced cap rather
//! than dying at the allocator, and low enough that the reclamation test can
//! tell a host that gave the memory back from one that did not. Measured: after
//! an abort at a 1 MiB cap the state holds about 1.43 MB until something
//! collects, and the next callback's 512 KiB lands at about 1.96 MB. A backstop
//! of 1.75 MiB sits between those, so an uncollected host refuses that
//! allocation and a collected one serves it out of a baseline nine tenths
//! smaller.
//!
//! Every expected quantity below is arithmetic performed here — the byte counts
//! the scripts are told to allocate — never a number read back from a run.

use std::error::Error;
use std::num::{NonZeroU64, NonZeroUsize};

use mc_script::{
    Attachment, ChunkName, ComponentName, DispatchReport, FaultKind, HostLimits, ScriptFunction,
    ScriptHost, ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// The cap every scenario in this file names.
const MEMORY_CAP: usize = 1024 * 1024;

/// The absolute ceiling the whole state may reach — see the note above on where
/// it sits and why.
const MEMORY_BACKSTOP: usize = 1792 * 1024;

/// A second cap, used only to show that the cause names the cap it was given
/// rather than a number somebody typed into a format string.
const A_SMALLER_CAP: usize = 512 * 1024;

/// A backstop that leaves the same kind of room above the smaller cap.
const A_SMALLER_BACKSTOP: usize = 1280 * 1024;

/// A backstop with no room above the cap at all, for the refusal.
///
/// Above what an empty state costs — so the state itself can be built — and far
/// below that baseline plus the cap, which is the relation the host checks.
const A_BACKSTOP_WITH_NO_ROOM: usize = 768 * 1024;

/// A call-and-loop budget the work below cannot reach. See the note above.
const BUDGET: u64 = 1_000_000;

/// How much each append asks for. Small enough that the interrupt sees the
/// usage climb past the cap rather than the allocator refusing one huge jump.
const APPENDED_BYTES: usize = 4096;

/// Appends that would take the state to four times the cap if nothing stopped
/// them.
const APPENDS_PAST_THE_CAP: usize = 1024;

/// How many times the protected bomb picks itself up and starts again.
const RETRIES: usize = 5;

/// A structure comfortably inside the cap.
const A_STRUCTURE_INSIDE_THE_CAP: usize = 64 * 1024;

/// What the callback after an allocation fault asks for, which it can only get
/// if the stopped callback's memory came back.
const THE_MEMORY_THE_NEXT_CALLBACK_NEEDS: usize = 512 * 1024;

/// A callback that appends a *distinct* string of about `bytes` to a table,
/// until it is stopped or runs out of appends to make.
///
/// **The index on the end is the whole fixture and removing it silently voids
/// every test in this file.** The backend interns strings, so a thousand
/// appends of `string.rep('x', 4096)` are a thousand references to **one**
/// string: measured, that grew the state by 120,629 bytes against a cap of
/// 1,048,576 and returned normally. No host can stop an invocation that never
/// allocates, so the tests here would have been unpassable by any
/// implementation — a count-based assertion satisfied by a fixture measuring
/// the wrong workload, which is the one failure no assertion can catch.
/// Concatenating the loop index makes every string distinct and therefore
/// separately allocated: measured, the same loop then grows the state by
/// 1,445,562 bytes.
fn chunk_that_appends(appends: usize, bytes: usize) -> String {
    format!(
        "return function()\n\
         \tlocal held = {{}}\n\
         \tfor index = 1, {appends} do held[index] = string.rep('x', {bytes}) .. index end\n\
         \treturn #held\n\
         end\n"
    )
}

/// The same bomb built out of binary buffers rather than strings.
///
/// A buffer is never interned and never shared, so this route to the cap shares
/// nothing with the one above but the cap itself.
fn chunk_that_appends_buffers(appends: usize, bytes: usize) -> String {
    format!(
        "return function()\n\
         \tlocal held = {{}}\n\
         \tfor index = 1, {appends} do held[index] = buffer.create({bytes}) end\n\
         \treturn #held\n\
         end\n"
    )
}

/// A callback that allocates one structure of `bytes` and reports its size.
fn chunk_that_allocates(bytes: usize) -> String {
    format!(
        "return function()\n\
         \tlocal held = string.rep('x', {bytes})\n\
         \treturn #held\n\
         end\n"
    )
}

/// A callback that runs the same bomb inside a protected call and reports how
/// many times it caught the failure and carried on.
///
/// Reporting is the whole construction. An allocation failure that reaches
/// script as an ordinary catchable error was measured to be defeated exactly
/// this way — the handler drops the table, the collector takes it back, and the
/// next round starts from nothing — so a callback that merely spun could not
/// tell that host from one that stopped it. This one hands the count back, so a
/// catchable failure arrives here as a **result** where a fault was demanded.
///
/// The inner loop is the fixture above, index and all, for the reason recorded
/// there: without the index it allocates nothing, nothing fails, and the count
/// this reports is zero however the host behaves.
fn chunk_that_retries_its_bomb(retries: usize, appends: usize, bytes: usize) -> String {
    format!(
        "return function()\n\
         \tlocal caught = 0\n\
         \tfor round = 1, {retries} do\n\
         \t\tlocal ok = pcall(function()\n\
         \t\t\tlocal held = {{}}\n\
         \t\t\tfor index = 1, {appends} do held[index] = string.rep('x', {bytes}) .. index end\n\
         \t\tend)\n\
         \t\tif not ok then caught = caught + 1 end\n\
         \tend\n\
         \treturn caught\n\
         end\n"
    )
}

/// What a fault says about itself, as one comparable record.
#[derive(Debug, PartialEq, Eq)]
struct InvocationFault {
    kind: FaultKind,
    chunk: Option<String>,
    subject: Option<String>,
    component: Option<String>,
}

/// The fault an invocation stopped for allocating too much should produce.
fn stopped_for_memory(chunk: &str, subject: &str, component: &str) -> InvocationFault {
    InvocationFault {
        kind: FaultKind::Allocation,
        chunk: Some(chunk.to_owned()),
        subject: Some(subject.to_owned()),
        component: Some(component.to_owned()),
    }
}

/// What an allocation fault manages to say about why it happened.
///
/// The kind is optional because a host that stopped nothing reports no fault at
/// all, and "there was nothing to read" is an answer this record has to be able
/// to give rather than a reason to stop early.
#[derive(Debug, PartialEq, Eq)]
struct StatedCause {
    kind: Option<FaultKind>,
    says_nothing: bool,
    names_the_cap: bool,
}

/// What an allocation fault owes whoever reads it.
fn states_the_cap() -> StatedCause {
    StatedCause {
        kind: Some(FaultKind::Allocation),
        says_nothing: false,
        names_the_cap: true,
    }
}

/// What a round that stopped nothing has to say for itself.
fn stopped_nothing() -> StatedCause {
    StatedCause {
        kind: None,
        says_nothing: true,
        names_the_cap: false,
    }
}

/// A host at `cap`, `backstop`, and a budget the work here cannot reach.
fn host_at(cap: usize, backstop: usize) -> Result<ScriptHost, Box<dyn Error>> {
    ScriptHost::with_limits(limits_at(cap, backstop)?)
        .map_err(|error| format!("the host refused a cap of {cap} bytes: {error}").into())
}

fn limits_at(cap: usize, backstop: usize) -> Result<HostLimits, Box<dyn Error>> {
    Ok(HostLimits {
        call_and_loop_budget: NonZeroU64::new(BUDGET).ok_or("the budget must not be zero")?,
        memory_cap: NonZeroUsize::new(cap).ok_or("the cap must not be zero")?,
        memory_backstop: NonZeroUsize::new(backstop).ok_or("the backstop must not be zero")?,
        ..HostLimits::default()
    })
}

/// A host at the cap every scenario in this file names.
fn host_at_the_named_cap() -> Result<ScriptHost, Box<dyn Error>> {
    host_at(MEMORY_CAP, MEMORY_BACKSTOP)
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

/// How a size the script was told to allocate reads when it comes back.
fn allocated(bytes: usize) -> String {
    format!("integer {bytes}")
}

const WHY_A_STOPPED_ALLOCATION_NAMES_ITS_ATTACHMENT: &str = "an allocation a mod cannot pay for is the second hostile shape anybody meets, and \
     stopping it is worth nothing to an operator who cannot tell whose it was. The fault \
     names the subject, the component and the file that defined the callback. It also carries \
     no result: an invocation the host stopped did not return, and filing a value under its \
     name reports something that never happened. Under a nominal budget this test passes \
     while measuring the wrong limit — the bomb is stopped for exhausting its ticks and the \
     kind reads `BudgetExhausted` — which is why the budget here is a million.";

#[test]
fn a_callback_that_allocates_past_the_cap_is_stopped_and_the_fault_names_its_attachment()
-> TestResult {
    let mut host = host_at_the_named_cap()?;
    let source = chunk_that_appends(APPENDS_PAST_THE_CAP, APPENDED_BYTES);
    let bomb = callback_from(&mut host, "furnace.luau", &source)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), bomb);

    let report = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (describe_faults(&report), result_for(&report, &smelt)),
        (
            vec![stopped_for_memory("furnace.luau", "stone-furnace", "smelt")],
            "no result".to_owned()
        ),
        "{WHY_A_STOPPED_ALLOCATION_NAMES_ITS_ATTACHMENT}"
    );
    Ok(())
}

#[test]
fn a_callback_that_allocates_well_inside_the_cap_completes_and_returns_its_result() -> TestResult {
    let mut host = host_at_the_named_cap()?;
    let source = chunk_that_allocates(A_STRUCTURE_INSIDE_THE_CAP);
    let modest = callback_from(&mut host, "hopper.luau", &source)?;
    let vent = attachment("stone-hopper", "vent");
    host.attach(vent.clone(), modest);

    let report = host.dispatch(std::slice::from_ref(&vent));

    assert_eq!(
        (result_for(&report, &vent), describe_faults(&report)),
        (allocated(A_STRUCTURE_INSIDE_THE_CAP), Vec::new()),
        "the cap has to be invisible to content that stays inside it, and the size coming back \
         is what says the structure was really built rather than the invocation being cut short \
         somewhere the host forgot to report. A cap read as an absolute rather than as a delta \
         above the entry's baseline fails here the moment anything else in the state is holding \
         memory, which is the ordinary condition of a server that has been running for an hour."
    );
    Ok(())
}

const WHY_THE_MEMORY_HAS_TO_BE_BACK_BY_THE_NEXT_INVOCATION: &str = "a cap that stops a mod and then leaves its megabyte standing has converted one bad \
     invocation into a permanently poorer host, and the next mod pays for it. Measured: \
     without an explicit collection the state still held about 1.43 MB after the abort and \
     the following 512 KiB was refused — the backstop here sits between those two figures \
     precisely so that this test can tell the two hosts apart. The second attachment's \
     result is the assertion rather than its invocation: a host that invoked it and threw \
     the answer away would pass the weaker check.";

#[test]
fn the_memory_a_stopped_callback_held_is_back_in_time_for_the_next_one() -> TestResult {
    let mut host = host_at_the_named_cap()?;
    let bomb_source = chunk_that_appends(APPENDS_PAST_THE_CAP, APPENDED_BYTES);
    let bomb = callback_from(&mut host, "furnace.luau", &bomb_source)?;
    let next_source = chunk_that_allocates(THE_MEMORY_THE_NEXT_CALLBACK_NEEDS);
    let next = callback_from(&mut host, "hopper.luau", &next_source)?;
    let smelt = attachment("stone-furnace", "smelt");
    let vent = attachment("stone-hopper", "vent");
    host.attach(smelt.clone(), bomb);
    host.attach(vent.clone(), next);

    let report = host.dispatch(&[smelt.clone(), vent.clone()]);

    assert_eq!(
        (describe_faults(&report), result_for(&report, &vent)),
        (
            vec![stopped_for_memory("furnace.luau", "stone-furnace", "smelt")],
            allocated(THE_MEMORY_THE_NEXT_CALLBACK_NEEDS)
        ),
        "{WHY_THE_MEMORY_HAS_TO_BE_BACK_BY_THE_NEXT_INVOCATION}"
    );
    Ok(())
}

const WHY_A_PROTECTED_BOMB_IS_STILL_STOPPED: &str = "a protected call is reachable from content and has to be, so an allocation failure that \
     script can catch bounds nothing: measured, a bomb wrapped this way went round ten times \
     and returned normally, because each caught failure dropped the table and the collector \
     handed the memory straight back. This callback reports how many failures it caught, so a \
     host whose cap is merely a catchable error arrives here with a **result** — the count — \
     where a fault was demanded. That is the difference between an invocation being stopped \
     and an invocation being inconvenienced.";

#[test]
fn a_callback_that_catches_its_own_allocation_failure_is_still_stopped() -> TestResult {
    let mut host = host_at_the_named_cap()?;
    let source = chunk_that_retries_its_bomb(RETRIES, APPENDS_PAST_THE_CAP, APPENDED_BYTES);
    let stubborn = callback_from(&mut host, "furnace.luau", &source)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), stubborn);

    let report = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (describe_faults(&report), result_for(&report, &smelt)),
        (
            vec![stopped_for_memory("furnace.luau", "stone-furnace", "smelt")],
            "no result".to_owned()
        ),
        "{WHY_A_PROTECTED_BOMB_IS_STILL_STOPPED}"
    );
    Ok(())
}

const WHY_A_SECOND_ROUTE_TO_THE_CAP_IS_WORTH_HAVING: &str = "every other test in this file reaches the cap by allocating strings, so the cap has one \
     witness and one allocator behind it. A buffer is the engine-facing half of the same \
     surface — binary, never interned, never shared — and the design records it as fully \
     visible to this accounting on the strength of a measurement rather than of a mechanism. \
     If it were not, a mod would have an unmetered heap sitting in plain sight in the \
     permitted set, and every string-based test here would keep passing while it did.";

#[test]
fn a_bomb_built_from_binary_buffers_is_stopped_by_the_same_cap() -> TestResult {
    let mut host = host_at_the_named_cap()?;
    let source = chunk_that_appends_buffers(APPENDS_PAST_THE_CAP, APPENDED_BYTES);
    let bomb = callback_from(&mut host, "furnace.luau", &source)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), bomb);

    let report = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        (describe_faults(&report), result_for(&report, &smelt)),
        (
            vec![stopped_for_memory("furnace.luau", "stone-furnace", "smelt")],
            "no result".to_owned()
        ),
        "{WHY_A_SECOND_ROUTE_TO_THE_CAP_IS_WORTH_HAVING}"
    );
    Ok(())
}

/// Runs the bomb at `cap` and reports what its fault managed to say.
fn cause_stated_at(cap: usize, backstop: usize) -> Result<StatedCause, Box<dyn Error>> {
    let mut host = host_at(cap, backstop)?;
    let source = chunk_that_appends(APPENDS_PAST_THE_CAP, APPENDED_BYTES);
    let bomb = callback_from(&mut host, "furnace.luau", &source)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), bomb);

    let report = host.dispatch(std::slice::from_ref(&smelt));
    let Some(fault) = report.faults.first() else {
        return Ok(stopped_nothing());
    };
    Ok(StatedCause {
        kind: Some(fault.kind),
        says_nothing: fault.cause.trim().is_empty(),
        names_the_cap: fault.cause.contains(&cap.to_string()),
    })
}

const WHY_THE_CAUSE_HAS_TO_NAME_THE_CAP: &str = "the failure underneath this one carries no message and no line at all — measured, it is \
     literally an empty memory error — so a fault that passes it through names its subject \
     and its component and then says nothing whatever about why. An empty cause and a cause \
     that was never populated read identically to whoever is trying to fix the mod, which is \
     why the host composes this one. Non-emptiness alone would be true of any format string, \
     so the cap is what has to appear, **as its byte count in decimal**: the same bomb is run \
     at two different caps here, and a formatter emitting a constant string can satisfy at \
     most one of them.";

#[test]
fn an_allocation_fault_states_the_cap_it_exceeded_rather_than_saying_nothing() -> TestResult {
    let observed = vec![
        cause_stated_at(MEMORY_CAP, MEMORY_BACKSTOP)?,
        cause_stated_at(A_SMALLER_CAP, A_SMALLER_BACKSTOP)?,
    ];

    assert_eq!(
        observed,
        vec![states_the_cap(), states_the_cap()],
        "{WHY_THE_CAUSE_HAS_TO_NAME_THE_CAP}"
    );
    Ok(())
}

/// Whether a host with these two limits agreed to start.
fn started_at(cap: usize, backstop: usize) -> Result<&'static str, Box<dyn Error>> {
    match ScriptHost::with_limits(limits_at(cap, backstop)?) {
        Ok(_) => Ok("started"),
        Err(_) => Ok("refused"),
    }
}

#[test]
fn a_host_whose_backstop_leaves_no_room_above_its_cap_refuses_to_start() -> TestResult {
    let observed = (
        started_at(MEMORY_CAP, A_BACKSTOP_WITH_NO_ROOM)?,
        started_at(MEMORY_CAP, MEMORY_BACKSTOP)?,
    );

    assert_eq!(
        observed,
        ("refused", "started"),
        "a backstop that does not clear the empty state's own baseline plus the cap puts the \
         host into memory pressure from its first invocation: every fault it ever reports is \
         about its own configuration rather than about the mod that was running, and every \
         later test of that condition measures nothing. It is a configuration error rather \
         than a script fault — no mod caused it and no mod can fix it — so it belongs where \
         the host is built. The second half is the control: a host that refused every pair \
         would satisfy the first half and be useless."
    );
    Ok(())
}
