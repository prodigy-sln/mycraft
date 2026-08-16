//! What the host reports when an invocation could have failed for a reason that
//! is not its own, and why those failures are not counted against anybody.
//!
//! # The condition, stated exactly
//!
//! One state serves every attachment, and a closure holds its upvalues, so a mod
//! can retain memory across invocations with no state API at all — a table in a
//! closure, appended to. Nothing here bounds that: the per-invocation cap bounds
//! what one entry *adds*, not what the state already holds on somebody's behalf.
//! As the state approaches the ceiling, every other mod's ordinary allocations
//! start failing, and each failure is charged to whoever happened to be running.
//!
//! The host detects that at entry, and the condition is derived rather than
//! chosen:
//!
//! > the state's collected memory, plus what one invocation may add, no longer
//! > fits below the ceiling the whole state may reach
//!
//! — *this invocation could fail for a reason that is not its own*. There is no
//! fraction and no constant to pick, defend or re-pick when the ceiling moves.
//! While it holds, a fault is reported as host memory pressure, carries **no
//! subject and no component**, and does not count toward the consecutive-fault
//! total. Naming an attachment on such a fault would file the blame against an
//! author who did nothing wrong, and an operator acting on it removes the wrong
//! mod.
//!
//! # The reading has to be a collected one
//!
//! Measured: 1,434,679 bytes of garbage survived until an explicit collection.
//! Deciding pressure against a raw reading would report pressure caused by
//! memory nothing is holding, and condemn the host to permanent "pressure" it
//! could never leave. So the fixture reads the collected figure, and reading it
//! before each ballast round is also what makes this fixture converge rather
//! than stall: after a collection, what is free below the ceiling is exactly
//! what the reading says it is.
//!
//! # The cost this excuses is named rather than hidden
//!
//! While pressure holds, an attachment whose own retention raised the baseline
//! is immune to quarantine, and a genuinely looping mod is not quarantined
//! either — which is the second test here, and it is the scenario that reconciles
//! this rule with the one saying three faults of any three kinds still
//! quarantine. That cost was weighed and taken: the excused failure is loud — a
//! slow server an operator notices — where the alternative is silent and
//! misdirected, ending with the operator removing a mod that did nothing.
//! Quarantine would not have helped in any case: retention lives in closure
//! upvalues, which survive it.
//!
//! # Fixture construction is the constraint no assertion can enforce
//!
//! Every assertion below is satisfied by a host under pressure and by a fixture
//! that failed to establish any. So the tests assert the condition **holds**,
//! computed here from its own definition rather than asked of the host, before
//! and after the rounds they measure — and the ordinary quarantine tests assert
//! it does **not**, because a fixture that established pressure permanently
//! would make those scenarios measure nothing at all.
//!
//! **The retention is buffers, not strings, and that is load-bearing.** The
//! backend shares identical strings, so a loop retaining the same one allocates
//! it once and this fixture would raise nothing while every count in it still
//! read plausibly.

use std::error::Error;
use std::num::{NonZeroU32, NonZeroU64, NonZeroUsize};

use mc_script::{
    Attachment, ComponentName, FaultKind, HostLimits, ScriptFault, ScriptFunction, ScriptHost,
    ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// What one invocation may add above the baseline it started from — the figure
/// both scenarios name.
const MEMORY_CAP: usize = 1024 * 1024;

/// The ceiling the whole state may reach.
///
/// Above the empty state's own baseline plus the cap, so the host agrees to
/// start; low enough that a fixture can genuinely fill the room below it in a
/// bounded number of rounds.
const MEMORY_BACKSTOP: usize = 2048 * 1024;

/// A budget a runaway loop reaches in milliseconds and the ballast cannot
/// approach.
const BUDGET: u64 = 50_000;

/// How many consecutive faults would quarantine an attachment, if any of these
/// faults counted.
const FAULT_THRESHOLD: u32 = 3;

/// What the attachment under test asks for on every invocation.
const A_MODEST_ALLOCATION: usize = 64 * 1024;

/// How much the ballast retains per invocation.
///
/// Smaller than the modest allocation on purpose: the fill stops on the round
/// that leaves less room than the modest allocation needs, so the room left
/// afterwards is between one increment short of it and none at all — never
/// enough for the invocation under test, and always enough for the host's own
/// working room while it runs one.
const RETAINED_PER_INVOCATION: usize = 16 * 1024;

/// The most ballast rounds the fixture will run before declaring itself unable
/// to establish the condition it needs.
const FILL_ROUNDS_ALLOWED: usize = 256;

/// A callback that retains what it allocates, in a table its closure holds.
///
/// This is the retention the design names as the thing no per-invocation limit
/// can reach, used here to build the state it produces.
fn chunk_that_retains(bytes: usize) -> String {
    format!(
        "local kept = {{}}\n\
         return function()\n\
         \tkept[#kept + 1] = buffer.create({bytes})\n\
         \treturn #kept\n\
         end\n"
    )
}

/// A callback that allocates a modest amount and retains none of it.
fn chunk_that_allocates(bytes: usize) -> String {
    format!(
        "return function()\n\
         \tlocal held = buffer.create({bytes})\n\
         \treturn buffer.len(held)\n\
         end\n"
    )
}

/// A callback whose loop never terminates.
const NEVER_RETURNS: &str = "return function()\n\twhile true do end\nend\n";

/// What a fault said about whose failure it was.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedFault {
    kind: FaultKind,
    subject: Option<String>,
    component: Option<String>,
}

/// A fault the host raised about its own condition, blaming nobody.
fn blamed_on_nobody() -> ObservedFault {
    ObservedFault {
        kind: FaultKind::HostMemoryPressure,
        subject: None,
        component: None,
    }
}

fn described(fault: &ScriptFault) -> ObservedFault {
    ObservedFault {
        kind: fault.kind,
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
    }
}

fn attachment(subject: &str, component: &str) -> Attachment {
    Attachment {
        subject: SubjectName::new(subject),
        component: ComponentName::new(component),
    }
}

fn host_at_the_named_limits() -> Result<ScriptHost, Box<dyn Error>> {
    let limits = HostLimits {
        call_and_loop_budget: NonZeroU64::new(BUDGET).ok_or("the budget must not be zero")?,
        memory_cap: NonZeroUsize::new(MEMORY_CAP).ok_or("the cap must not be zero")?,
        memory_backstop: NonZeroUsize::new(MEMORY_BACKSTOP)
            .ok_or("the backstop must not be zero")?,
        fault_threshold: NonZeroU32::new(FAULT_THRESHOLD)
            .ok_or("the fault threshold must not be zero")?,
        ..HostLimits::default()
    };
    ScriptHost::with_limits(limits)
        .map_err(|error| format!("the host refused these limits: {error}").into())
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

/// Whether `bytes` more would still fit below the ceiling, measured after a
/// collection.
fn fits(host: &ScriptHost, bytes: usize) -> bool {
    host.collected_memory_in_use().saturating_add(bytes) <= host.limits().memory_backstop.get()
}

/// Whether the host's own memory leaves no room for one more invocation.
///
/// Written from the condition's own definition rather than asked of the host, so
/// that a host whose classification is wrong does not also get to decide whether
/// the fixture was valid.
fn under_memory_pressure(host: &ScriptHost) -> bool {
    !fits(host, host.limits().memory_cap.get())
}

/// Retains memory in the ballast's closure until a modest allocation will no
/// longer fit below the ceiling.
///
/// It fails loudly rather than returning a host that is not under pressure: a
/// fixture that quietly established nothing would leave every assertion in this
/// file green and measuring nothing.
fn fill(host: &mut ScriptHost, ballast: &Attachment) -> Result<(), Box<dyn Error>> {
    for _ in 0..FILL_ROUNDS_ALLOWED {
        if !fits(host, A_MODEST_ALLOCATION) {
            return Ok(());
        }
        host.dispatch(std::slice::from_ref(ballast));
    }
    Err(format!(
        "the fixture could not fill the state to within {A_MODEST_ALLOCATION} bytes of its \
         {MEMORY_BACKSTOP}-byte ceiling in {FILL_ROUNDS_ALLOWED} rounds: it reached {} bytes, \
         and every assertion that follows would have measured a host under no pressure at all",
        host.collected_memory_in_use()
    )
    .into())
}

/// A host filled to the point of pressure, with `source` attached to the
/// attachment under test.
///
/// Both chunks are evaluated **before** the filling starts, because evaluating
/// one is itself an entry into script that allocates, and a chunk that cannot be
/// compiled against a full state would fail the fixture rather than the
/// mechanism.
fn host_under_pressure(source: &str) -> Result<(ScriptHost, Attachment), Box<dyn Error>> {
    let mut host = host_at_the_named_limits()?;
    let retaining = callback_from(
        &mut host,
        "ballast.luau",
        &chunk_that_retains(RETAINED_PER_INVOCATION),
    )?;
    let under_test = callback_from(&mut host, "furnace.luau", source)?;
    let ballast = attachment("host-ballast", "retain");
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(ballast.clone(), retaining);
    host.attach(smelt.clone(), under_test);

    fill(&mut host, &ballast)?;
    Ok((host, smelt))
}

/// What the attachment under test met over the rounds it was invoked for.
#[derive(Debug, PartialEq, Eq)]
struct Measured {
    pressure_before: bool,
    pressure_after: bool,
    faults: Vec<ObservedFault>,
    invocations: u64,
    quarantined: bool,
}

/// Invokes `smelt` for the threshold's worth of rounds and reports what
/// happened, with the condition read on both sides of them.
fn rounds_under_pressure(host: &mut ScriptHost, smelt: &Attachment) -> Measured {
    let pressure_before = under_memory_pressure(host);
    let mut faults = Vec::new();
    for _ in 0..FAULT_THRESHOLD {
        let report = host.dispatch(std::slice::from_ref(smelt));
        faults.extend(report.faults.iter().map(described));
    }
    Measured {
        pressure_before,
        pressure_after: under_memory_pressure(host),
        faults,
        invocations: host.invocation_count(smelt),
        quarantined: host.is_quarantined(smelt),
    }
}

const WHY_A_PRESSURED_FAULT_NAMES_NOBODY: &str = "the mod under test asks for sixty-four kilobytes and asks for nothing else, and it \
     fails because the state is full of somebody else's retention. A host that reported that \
     against its subject and its component would send an operator to remove the one mod that \
     was behaving, and three such rounds would quarantine it — permanently disabling an \
     innocent attachment on the strength of a condition it did not cause and cannot fix. So \
     the fault names neither half of the attachment, and none of the three counts. The \
     condition is asserted on both sides of the rounds because every other assertion here is \
     equally satisfied by a fixture that established no pressure at all.";

#[test]
fn a_modest_allocation_that_fails_under_a_full_state_is_blamed_on_nobody() -> TestResult {
    let (mut host, smelt) = host_under_pressure(&chunk_that_allocates(A_MODEST_ALLOCATION))?;

    let measured = rounds_under_pressure(&mut host, &smelt);

    assert_eq!(
        measured,
        Measured {
            pressure_before: true,
            pressure_after: true,
            faults: vec![blamed_on_nobody(); FAULT_THRESHOLD as usize],
            invocations: u64::from(FAULT_THRESHOLD),
            quarantined: false,
        },
        "{WHY_A_PRESSURED_FAULT_NAMES_NOBODY}"
    );
    Ok(())
}

const WHY_EVEN_A_LOOPING_MOD_ESCAPES_QUARANTINE_HERE: &str = "this is the case the rule costs, and it is written down rather than discovered: while \
     the host is short of memory a genuinely broken callback — one whose loop never \
     terminates, three rounds running — is not quarantined either. The alternative is a host \
     deciding, in exactly the condition where it cannot tell whose failure it is looking at, \
     to permanently disable an attachment. Its cost is bounded anyway: every one of these \
     invocations was stopped by its budget. The invocation count and the fault count are \
     asserted beside the quarantine, because `not quarantined` is also true of an attachment \
     that was never invoked and of one that never failed, and neither of those would be \
     measuring this rule. The fault *kind* is deliberately not asserted: the scenario is about \
     what is counted, and pinning what such a fault is called would decide a question it does \
     not ask.";

#[test]
fn an_attachment_looping_under_a_full_state_is_still_not_quarantined_for_it() -> TestResult {
    let (mut host, smelt) = host_under_pressure(NEVER_RETURNS)?;

    let measured = rounds_under_pressure(&mut host, &smelt);

    assert_eq!(
        (
            measured.pressure_before,
            measured.pressure_after,
            measured.faults.len(),
            measured.invocations,
            measured.quarantined,
        ),
        (
            true,
            true,
            FAULT_THRESHOLD as usize,
            u64::from(FAULT_THRESHOLD),
            false
        ),
        "{WHY_EVEN_A_LOOPING_MOD_ESCAPES_QUARANTINE_HERE}"
    );
    Ok(())
}
