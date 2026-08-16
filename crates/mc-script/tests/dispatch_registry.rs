//! What the host keeps between rounds: which state a handle came from, and how
//! often an attachment has been invoked.
//!
//! Neither of these carries an acceptance scenario, and both arrive with the
//! registry this phase builds. A mechanism that lands without a test is a
//! mechanism nobody can show working, and this crate is already carrying more of
//! those than it should.
//!
//! # The isolation unit, and exactly what this test can and cannot say
//!
//! A handle onto a value inside the script state carries an opaque tag saying
//! *which* state it came from. Today one state exists, so the tag has one value,
//! and this test can only observe that handles taken from one host agree with
//! each other. **It cannot observe disagreement, because there is nothing to
//! disagree with yet.** That limit is stated here rather than papered over: the
//! test earns its place by pinning the tag to the *state* rather than to the
//! handle or to the chunk, which are the two implementations that look correct
//! while one state exists and are wrong the moment a second one appears.
//!
//! The reason the tag exists at all is hot reload. Reloading builds a candidate
//! registry in a scratch state and then substitutes a scratch-state function for
//! a live one; a handle that cannot say where it came from makes that
//! substitution unverifiable, in the one path whose partial application this
//! crate calls a Blocker. It is deliberately **not** justified by the
//! published-modding-API exemption from "no abstraction before three uses" —
//! that exemption is scoped to the surface content writes against and does not
//! reach engine-internal handles. Whoever reads the standard correctly and finds
//! no justification recorded will delete the field, so the justification is
//! recorded.
//!
//! # The invocation count
//!
//! Cumulative telemetry about the attachment. It answers *how often has this
//! been asked to run*, which is a different question from the consecutive-fault
//! count quarantine keeps, and the two are separate counters for that reason.
//! What this test pins is that it counts, that it counts per round, and that an
//! attachment nobody registered reads zero rather than being absent, missing or
//! a panic.

use std::error::Error;

use mc_script::{Attachment, ComponentName, ScriptFunction, ScriptHost, ScriptValue, SubjectName};

type TestResult = Result<(), Box<dyn Error>>;

/// A chunk yielding a callback the host can register.
const RETURNS_A_CALLBACK: &str = "return function()\n\treturn 'invoked'\nend\n";

/// A chunk yielding a table, which is the other handle the host hands out.
const RETURNS_A_TABLE: &str = "return { fuel = 4 }\n";

fn host() -> Result<ScriptHost, Box<dyn Error>> {
    ScriptHost::new().map_err(|error| format!("the host refused to start: {error}").into())
}

fn attachment(subject: &str, component: &str) -> Attachment {
    Attachment {
        subject: SubjectName::new(subject),
        component: ComponentName::new(component),
    }
}

/// The function a chunk returns, or an account of why it did not return one.
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

#[test]
fn every_handle_taken_from_one_host_reports_the_same_isolation_unit() -> TestResult {
    let mut host = host()?;
    let from_one_chunk = callback_from(&mut host, "furnace.luau", RETURNS_A_CALLBACK)?;
    let from_another_chunk = callback_from(&mut host, "hopper.luau", RETURNS_A_CALLBACK)?;
    let table = match host.evaluate("recipes.luau", RETURNS_A_TABLE) {
        Ok(ScriptValue::Table(table)) => table,
        Ok(other) => return Err(format!("`recipes.luau` returns a table, not {other:?}").into()),
        Err(fault) => return Err(format!("`recipes.luau` did not evaluate: {fault}").into()),
    };

    assert_eq!(
        [from_another_chunk.unit(), table.unit()],
        [from_one_chunk.unit(), from_one_chunk.unit()],
        "the tag says which script state a handle came from, and there is exactly one state \
         today — so the only thing observable now is that handles agree, and the only way to get \
         this wrong is to derive the tag from something other than the state. A tag taken from \
         the handle itself, or from the chunk that produced it, passes every use this crate \
         makes of it today and is wrong the first time hot reload substitutes a scratch-state \
         function for a live one. Three handles from two chunks and two kinds, so both wrong \
         derivations are visible here."
    );
    Ok(())
}

#[test]
fn an_attachments_invocation_count_rises_once_for_each_round_it_is_invoked_in() -> TestResult {
    let mut host = host()?;
    let callback = callback_from(&mut host, "furnace.luau", RETURNS_A_CALLBACK)?;
    let smelt = attachment("stone-furnace", "smelt");
    let never_registered = attachment("stone-furnace", "vent");
    host.attach(smelt.clone(), callback);

    let mut counted = Vec::new();
    for _ in 0..3 {
        host.dispatch(std::slice::from_ref(&smelt));
        counted.push(host.invocation_count(&smelt));
    }

    assert_eq!(
        (counted, host.invocation_count(&never_registered)),
        (vec![1, 2, 3], 0),
        "this is the counter an operator reads to answer `is this callback running at all`, and \
         the counter quarantine's own bookkeeping is later asserted to resume from rather than \
         reset. Read after every round rather than once at the end, so a counter that jumps, \
         that counts rounds instead of invocations, or that is written only when a round ends \
         is distinguishable from one that counts. An attachment nobody registered reads zero \
         rather than being missing."
    );
    Ok(())
}
