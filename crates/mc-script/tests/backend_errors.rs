//! The third way an entry into script can end: the backend refuses while the
//! host's own limits are still untouched.
//!
//! # Two outcomes were enumerated and there are three
//!
//! An entry into script either comes back with a value, or does not come back
//! because a limit tripped. The scheme this crate is built on reads those two
//! structurally — a callback that raised returns through the host's protected
//! call, and a latched abort cannot return at all — and there is a third state
//! the guard admits: the call fails while the guard is still **clear**. No limit
//! tripped, so the refusal came from the backend rather than from anything this
//! host enforces.
//!
//! Nothing in the acceptance scenarios reaches it. Left to a fallback arm it
//! becomes the branch that decides what an operator is told about a failure
//! nobody predicted, chosen by whichever arm happened to be last.
//!
//! # What is reachable from here, and what is not
//!
//! Measured on this toolchain: a chunk whose top level exhausts the call stack
//! reaches exactly this state — the backend refuses in about three milliseconds,
//! the guard never tripped, and no budget was near exhaustion. That is a genuine
//! end-to-end witness that the arm is reached and produces a whole fault rather
//! than a panic or an invented one.
//!
//! What a call-stack refusal cannot witness is the *classification*. The arm
//! distinguishes an allocation refusal from everything else, and a call-stack
//! refusal is "everything else" — so it answers the same way a host classifying
//! from the error's text would.
//!
//! # The allocation half, which needed the backstop to exist
//!
//! The other half is now reachable and the third test takes it. A single
//! allocation larger than the whole state is allowed to reach never lands, so
//! the usage the interrupt watches never climbs and no tick can latch — the
//! guard is still clear when the call comes back failed, and the only thing left
//! that can tell an allocation refusal from a script's own error is the identity
//! of the error the backend raised. A host that stopped consulting that and
//! answered "the script failed" for everything reddens here, which is the wiring
//! witness the classification did not have.
//!
//! It has to arrive through chunk evaluation rather than through a callback:
//! measured, the host's protected call catches this refusal exactly as it
//! catches a call-stack one, so behind a callback it is an ordinary raised value
//! and never reaches this arm at all.
//!
//! # A callback reaches this differently, and that is worth knowing
//!
//! Also measured: the host's protected call **catches** a call-stack refusal, so
//! a callback that exhausts the stack comes back as an ordinary raised value and
//! is attributed to its attachment. The second test pins that, because the
//! alternative — a refusal escaping the protected call — would arrive at the
//! third arm with no attachment to name, and an operator would be told a mod
//! failed without being told which.

use std::error::Error;
use std::num::NonZeroUsize;

use mc_script::{
    Attachment, ChunkName, ComponentName, DispatchReport, FaultKind, HostLimits, ScriptFault,
    ScriptFunction, ScriptHost, ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// A chunk whose top level recurses until the backend runs out of call stack.
///
/// The recursive call is not in tail position, so each level is a frame that
/// stays. The refusal is reported against line 2, which is where the call is.
const EXHAUSTS_THE_CALL_STACK: &str = "local function deeper(depth)\n\
     \tlocal reached = deeper(depth + 1)\n\
     \treturn reached\n\
     end\n\
     return deeper(1)\n";

/// The same runaway recursion, behind a callback.
const CALLBACK_EXHAUSTS_THE_CALL_STACK: &str = "local function deeper(depth)\n\
     \tlocal reached = deeper(depth + 1)\n\
     \treturn reached\n\
     end\n\
     return function()\n\
     \treturn deeper(1)\n\
     end\n";

/// A chunk that evaluates cleanly, for asking whether the host still works.
const A_SOUND_CHUNK: &str = "return 'recovered'";

/// A chunk asking for more in one allocation than the state may ever hold.
///
/// One request rather than many, deliberately: the interrupt only looks between
/// two ticks and this builtin runs to completion between them, so nothing the
/// host enforces per tick can see this coming. Whatever refuses it is the
/// allocator, on its own account.
const ALLOCATES_PAST_THE_BACKSTOP_IN_ONE_GO: &str =
    "local held = string.rep('x', 8 * 1024 * 1024)\nreturn #held\n";

/// Twice what unbounded recursion costs before the backend gives up on it, so
/// that the C stack is what runs out first. See `host` below for why this
/// number is a constraint rather than a preference.
const A_CAP_THE_CALL_STACK_RUNS_OUT_BEFORE: usize = 4 * 1024 * 1024;

/// Room above that cap, so nothing here meets the allocator's own ceiling.
const A_CEILING_WELL_ABOVE_THAT: usize = 16 * 1024 * 1024;

/// Bytes of script allocation one entry may add above its entry baseline.
const MEMORY_CAP: usize = 1024 * 1024;

/// The absolute ceiling the state may reach, with room above the cap so that
/// the refusal below is the allocator's rather than the interrupt's.
const MEMORY_BACKSTOP: usize = 1792 * 1024;

/// What a chunk-level fault says about itself.
#[derive(Debug, PartialEq, Eq)]
struct ChunkFault {
    kind: FaultKind,
    chunk: Option<String>,
    line: Option<u32>,
}

/// What an invocation fault says about itself.
#[derive(Debug, PartialEq, Eq)]
struct InvocationFault {
    kind: FaultKind,
    chunk: Option<String>,
    subject: Option<String>,
    component: Option<String>,
}

/// A host generous enough that the call stack runs out before anything the host
/// enforces does.
///
/// **Both limits have to be out of the way and the memory one is the subtle
/// half.** Unbounded recursion is not free: each frame that stays is script
/// memory, and measured, the backend refuses the recursion only after it has
/// grown the state by **1,911,289 bytes**. Under the shipped cap — a quarter of
/// a megabyte — the host stops it for memory long before the stack runs out and
/// reports an allocation fault, which is the correct answer to a different
/// question and leaves this file measuring nothing. So the cap here is 4 MiB,
/// twice what the recursion costs.
///
/// **That is a real constraint on the arm below, not a detail of this fixture.**
/// The backend refusing while the guard is still clear is reachable *only*
/// where a script can exhaust the C stack before its memory allowance, which
/// takes a cap of roughly two megabytes. Tidying these numbers down looks
/// harmless and silently deletes the only end-to-end witness the arm has.
fn host() -> Result<ScriptHost, Box<dyn Error>> {
    let limits = HostLimits {
        memory_cap: NonZeroUsize::new(A_CAP_THE_CALL_STACK_RUNS_OUT_BEFORE)
            .ok_or("the cap must not be zero")?,
        memory_backstop: NonZeroUsize::new(A_CEILING_WELL_ABOVE_THAT)
            .ok_or("the backstop must not be zero")?,
        ..HostLimits::default()
    };
    ScriptHost::with_limits(limits)
        .map_err(|error| format!("the host refused to start: {error}").into())
}

/// A host whose absolute ceiling is low enough that one oversized request
/// cannot be served, and whose budget stays generous for the same reason as
/// above.
fn host_under_a_low_ceiling() -> Result<ScriptHost, Box<dyn Error>> {
    let limits = HostLimits {
        memory_cap: NonZeroUsize::new(MEMORY_CAP).ok_or("the cap must not be zero")?,
        memory_backstop: NonZeroUsize::new(MEMORY_BACKSTOP)
            .ok_or("the backstop must not be zero")?,
        ..HostLimits::default()
    };
    ScriptHost::with_limits(limits)
        .map_err(|error| format!("the host refused a ceiling of {MEMORY_BACKSTOP}: {error}").into())
}

fn attachment(subject: &str, component: &str) -> Attachment {
    Attachment {
        subject: SubjectName::new(subject),
        component: ComponentName::new(component),
    }
}

fn chunk_name(fault: &ScriptFault) -> Option<String> {
    fault
        .origin
        .chunk
        .as_ref()
        .map(ChunkName::as_str)
        .map(str::to_owned)
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
            chunk: chunk_name(fault),
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

/// What an evaluation produced, as one comparable line.
fn outcome(evaluated: Result<ScriptValue, ScriptFault>) -> String {
    match evaluated {
        Ok(ScriptValue::Text(text)) => text,
        Ok(other) => format!("{other:?}"),
        Err(fault) => format!("fault: {fault}"),
    }
}

#[test]
fn a_chunk_that_exhausts_the_call_stack_is_reported_whole_and_leaves_the_host_usable() -> TestResult
{
    let mut host = host()?;
    let Err(fault) = host.evaluate("deep.luau", EXHAUSTS_THE_CALL_STACK) else {
        return Err("recursion with no base case must not evaluate successfully".into());
    };
    let rendered = fault.to_string();
    let afterwards = outcome(host.evaluate("sound.luau", A_SOUND_CHUNK));

    assert_eq!(
        (
            ChunkFault {
                kind: fault.kind,
                chunk: chunk_name(&fault),
                line: fault.line,
            },
            afterwards.as_str()
        ),
        (
            ChunkFault {
                kind: FaultKind::ScriptError,
                chunk: Some("deep.luau".to_owned()),
                line: Some(2),
            },
            "recovered"
        ),
        "this is the one construction that reaches the backend's own refusal while every limit \
         the host enforces is still untouched, and it is the only end-to-end evidence that the \
         arm handling that state is reached at all. It has to produce a whole fault — the kind, \
         the chunk and the line the backend named — rather than a shrug, and it has to leave \
         the host able to evaluate the next mod. The host reported: {rendered}"
    );
    Ok(())
}

#[test]
fn a_callback_that_exhausts_the_call_stack_is_reported_against_its_attachment() -> TestResult {
    let mut host = host()?;
    let runaway = callback_from(&mut host, "furnace.luau", CALLBACK_EXHAUSTS_THE_CALL_STACK)?;
    let smelt = attachment("stone-furnace", "smelt");
    host.attach(smelt.clone(), runaway);

    let report = host.dispatch(std::slice::from_ref(&smelt));

    assert_eq!(
        describe_faults(&report),
        vec![InvocationFault {
            kind: FaultKind::ScriptError,
            chunk: Some("furnace.luau".to_owned()),
            subject: Some("stone-furnace".to_owned()),
            component: Some("smelt".to_owned()),
        }],
        "the host's protected call catches a call-stack refusal — measured — so this arrives as \
         an ordinary raised value and is attributed like one. That is the whole point of \
         invoking callbacks through it: the alternative is a refusal unwinding past the host \
         with no attachment attached to it, and an operator told that a mod failed without \
         being told which mod."
    );
    Ok(())
}

const WHY_AN_OVERSIZED_REQUEST_IS_THE_OTHER_HALF: &str = "this is the arm's allocation half, and until the absolute ceiling existed nothing could \
     reach it. The request is one the interrupt cannot see coming and one that never lands, \
     so the usage the host watches never moves and no limit of its own trips — which leaves \
     the identity of the backend's own error as the only thing that can tell an allocation \
     refusal from a mod's own failure. Both directions matter and both are silent: a host \
     that stopped consulting that identity reports the state running out of memory as the \
     running mod's error, and a host that read the message instead would find nothing to \
     read, because this refusal was measured to arrive carrying no text at all. It also has \
     to leave the host able to load the next mod.";

#[test]
fn one_allocation_larger_than_the_state_may_hold_is_reported_as_an_allocation_refusal() -> TestResult
{
    let mut host = host_under_a_low_ceiling()?;

    let refused = host.evaluate("greedy.luau", ALLOCATES_PAST_THE_BACKSTOP_IN_ONE_GO);
    let described = match &refused {
        Ok(value) => (None, format!("{value:?}")),
        Err(fault) => (Some(fault.kind), format!("{:?}", chunk_name(fault))),
    };
    let afterwards = outcome(host.evaluate("sound.luau", A_SOUND_CHUNK));

    assert_eq!(
        (described, afterwards.as_str()),
        (
            (
                Some(FaultKind::Allocation),
                format!("{:?}", Some("greedy.luau".to_owned()))
            ),
            "recovered"
        ),
        "{WHY_AN_OVERSIZED_REQUEST_IS_THE_OTHER_HALF}"
    );
    Ok(())
}
