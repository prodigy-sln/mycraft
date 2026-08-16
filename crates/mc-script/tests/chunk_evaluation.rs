//! Evaluating a chunk is a guarded entry, and a chunk that will not stop is
//! stopped.
//!
//! The rule is categorical: there is no unbudgeted path from the engine into
//! script. Evaluating a chunk is such a path, so it carries the same budget and
//! the same abort as invoking a callback does. Nothing else in this crate's
//! scenarios reaches a chunk whose **top level** runs away — every chunk they
//! evaluate terminates — which is why two of the three tests here carry no
//! scenario of their own. Without them the first hostile script anybody
//! evaluates hangs the host, and hangs whatever test binary is holding it.
//!
//! # Two runaway shapes, not one
//!
//! The bare infinite loop is the obvious one. The second wraps its infinite loop
//! in a protected call inside another infinite loop, and it is the one that
//! decides whether the abort *latches*: an abort that is merely raised once is
//! caught by the protected call, and the outer loop starts again, so the budget
//! is re-entered indefinitely and bounds nothing. Measured against a
//! non-latching implementation, exactly that happened and the chunk returned
//! normally.
//!
//! That construction is covered elsewhere for callback invocation and nowhere
//! for chunk evaluation. Two entry points onto one mechanism are two things to
//! test: coverage of the first says the code ran, not that anything was checking
//! the second.
//!
//! # A note on how these fail
//!
//! If the abort is missing entirely, these tests do not fail — they hang, and
//! take the run with them. That is the failure mode they exist to remove from
//! everywhere else, and there is nowhere better for it to surface than here.
//!
//! # What a chunk-level fault may name
//!
//! A chunk runs before any attachment exists, so a fault from one names the
//! chunk and neither a subject nor a component. Those two fields being optional
//! is what this shape needs, and asserting they are empty is what stops a host
//! from filling them with whatever happened to be in hand.

use std::error::Error;
use std::num::NonZeroU64;

use mc_script::{ChunkName, FaultKind, HostLimits, ScriptFault, ScriptHost, ScriptValue};

type TestResult = Result<(), Box<dyn Error>>;

/// A call-and-loop budget small enough that a runaway chunk is stopped well
/// inside a test's own patience.
///
/// The budget counts calls and loop edges rather than instructions, so a bare
/// infinite loop spends one per iteration and reaches this in a few
/// milliseconds. Every other limit stays at its shipped default: this phase is
/// about the abort, and a second limit tripping first would answer a different
/// question.
const SMALL_BUDGET: u64 = 2_000;

/// A chunk whose top level never returns.
const RUNAWAY_CHUNK: &str = "while true do end";

/// A chunk whose top level never returns and which catches its own abort every
/// time round.
///
/// Without a latch the protected call swallows the abort, the outer loop starts
/// again, and the chunk runs forever inside a budget that was supposed to bound
/// it.
const RUNAWAY_CHUNK_THAT_CATCHES_ITS_OWN_ABORT: &str =
    "while true do pcall(function() while true do end end) end";

/// A chunk with a syntax error on its third line, and nothing wrong before it.
///
/// Two sound lines first, so the reported line is a fact about the chunk rather
/// than the only line there was.
const BROKEN_ON_LINE_THREE: &str = "local first = 1\nlocal second = 2\nlocal third = = 3\n";

/// A chunk that evaluates cleanly, for asking whether the host still works.
const A_SOUND_CHUNK: &str = "return 'recovered'";

/// What a fault says about where it came from.
///
/// A record rather than a handful of separate assertions, so one comparison
/// reports every field at once and a host that got two of them right is not
/// mistaken for one that got them all right.
#[derive(Debug, PartialEq, Eq)]
struct ChunkFault {
    kind: FaultKind,
    chunk: Option<String>,
    line: Option<u32>,
    names_a_subject: bool,
    names_a_component: bool,
}

fn describe(fault: &ScriptFault) -> ChunkFault {
    ChunkFault {
        kind: fault.kind,
        chunk: fault
            .origin
            .chunk
            .as_ref()
            .map(ChunkName::as_str)
            .map(str::to_owned),
        line: fault.line,
        names_a_subject: fault.subject.is_some(),
        names_a_component: fault.component.is_some(),
    }
}

/// An aborted chunk of the given name, carrying no line and no attachment.
fn aborted(chunk: &str) -> ChunkFault {
    ChunkFault {
        kind: FaultKind::BudgetExhausted,
        chunk: Some(chunk.to_owned()),
        line: None,
        names_a_subject: false,
        names_a_component: false,
    }
}

fn host_with_a_small_budget() -> Result<ScriptHost, Box<dyn Error>> {
    let budget = NonZeroU64::new(SMALL_BUDGET).ok_or("the configured budget must not be zero")?;
    let limits = HostLimits {
        call_and_loop_budget: budget,
        ..HostLimits::default()
    };
    match ScriptHost::with_limits(limits) {
        Ok(host) => Ok(host),
        Err(error) => {
            Err(format!("the host refused a small call-and-loop budget: {error:?}").into())
        }
    }
}

/// What an evaluation produced, as one comparable line.
fn outcome(evaluated: Result<ScriptValue, ScriptFault>) -> String {
    match evaluated {
        Ok(ScriptValue::Nil) => "nil".to_owned(),
        Ok(ScriptValue::Boolean(flag)) => format!("boolean {flag}"),
        Ok(ScriptValue::Integer(number)) => format!("integer {number}"),
        Ok(ScriptValue::Number(number)) => format!("number {number}"),
        Ok(ScriptValue::Text(text)) => text,
        Ok(ScriptValue::Table(_)) => "table".to_owned(),
        Ok(ScriptValue::Function(_)) => "function".to_owned(),
        Ok(ScriptValue::Opaque) => "opaque".to_owned(),
        Err(fault) => format!("fault: {fault}"),
    }
}

/// Aborts `source` under a small budget and reports what the host said, plus
/// what it did with a sound chunk afterwards.
fn abort_then_recover(
    chunk: &str,
    source: &str,
) -> Result<(ChunkFault, String, String), Box<dyn Error>> {
    let mut host = host_with_a_small_budget()?;
    let Err(fault) = host.evaluate(chunk, source) else {
        return Err(format!("`{chunk}` never returns, so evaluating it must not succeed").into());
    };
    let rendered = fault.to_string();
    let after = outcome(host.evaluate("sound-chunk", A_SOUND_CHUNK));
    Ok((describe(&fault), rendered, after))
}

#[test]
fn a_chunk_whose_top_level_never_returns_is_aborted_and_leaves_the_host_usable() -> TestResult {
    let (fault, rendered, after) = abort_then_recover("runaway.luau", RUNAWAY_CHUNK)?;

    assert_eq!(
        (fault, after.as_str()),
        (aborted("runaway.luau"), "recovered"),
        "nothing in the acceptance scenarios evaluates a chunk that does not come back, so \
         without this the first hostile script anybody loads hangs the server and there is no \
         test that would have said so. The abort has to name the chunk and leave the host able \
         to evaluate the next one. The host reported: {rendered}"
    );
    Ok(())
}

#[test]
fn a_chunk_that_catches_its_own_abort_in_a_loop_is_still_stopped() -> TestResult {
    let (fault, rendered, after) =
        abort_then_recover("catching.luau", RUNAWAY_CHUNK_THAT_CATCHES_ITS_OWN_ABORT)?;

    assert_eq!(
        (fault, after.as_str()),
        (aborted("catching.luau"), "recovered"),
        "an abort raised once and not latched is caught by the protected call inside this \
         chunk, and the outer loop simply starts again — measured, with the chunk returning \
         normally and the budget bounding nothing. The same shape is covered for callback \
         invocation and for nothing else, and one entry point is no evidence about the other. \
         The host reported: {rendered}"
    );
    Ok(())
}

#[test]
fn a_chunk_that_fails_to_compile_names_the_line_it_failed_on_and_leaves_the_host_usable()
-> TestResult {
    let mut host = host_with_a_small_budget()?;
    let Err(fault) = host.evaluate("furnace.luau", BROKEN_ON_LINE_THREE) else {
        return Err("a chunk with a syntax error on line 3 must not evaluate".into());
    };
    let rendered = fault.to_string();
    let after = outcome(host.evaluate("furnace.luau", A_SOUND_CHUNK));

    assert_eq!(
        (describe(&fault), after.as_str()),
        (
            ChunkFault {
                kind: FaultKind::Compilation,
                chunk: Some("furnace.luau".to_owned()),
                line: Some(3),
                names_a_subject: false,
                names_a_component: false,
            },
            "recovered"
        ),
        "the line is a typed field so that whoever reads this fault can be sent to a line \
         rather than to a string they have to parse — and so that a backend changing how it \
         spells its own messages cannot quietly take the locator away. A mod that fails to \
         load must also not take the ones after it down with it. The host reported: {rendered}"
    );
    Ok(())
}
