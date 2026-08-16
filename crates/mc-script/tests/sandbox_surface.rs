//! What a content chunk can and cannot reach, enumerated rather than sampled.
//!
//! Every question here is asked **from inside a chunk**, through the chunk's own
//! environment, and never from Rust. Under a per-chunk frozen environment those
//! are different questions with different answers, and only the first one is
//! about content.
//!
//! # Why the reachable set is enumerated and not just spot-checked
//!
//! A deny list catches a capability being *reintroduced*. It cannot catch one
//! being *added* — by a backend upgrade, or by a library nobody thought to name
//! — because a list derived by asking "what should be removed?" never asks "what
//! is actually standing?". So the whole reachable set is compared against the
//! set the host declares, in both directions: a name reachable but undeclared is
//! a capability arriving unannounced, and a name declared but unreachable is a
//! declaration that has drifted from the thing it describes. The verdict names
//! both, because the failure message is the product here — set inequality on its
//! own tells nobody what to do.
//!
//! The declaration is a compile-time array of string literals, which is what
//! keeps this from being a tautology: it cannot be computed by asking the
//! running state the same question this test asks it.
//!
//! # Reading the whole chain, not one hop
//!
//! Closing the sandbox inserts a table between the chunk's environment and the
//! real globals, and that middle table has no keys of its own. An enumeration
//! that follows `__index` exactly once therefore reports the chunk's `_G` and
//! stops — a scan that has run out of places to look, wearing the clothes of a
//! clean answer. The walk below follows the chain to its end.

use std::collections::BTreeSet;
use std::error::Error;

use mc_script::{ScriptFault, ScriptHost, ScriptValue};

type TestResult = Result<(), Box<dyn Error>>;

/// The nine names one scenario requires to be unreachable.
const DENIED_LIBRARIES_AND_LOADERS: [&str; 9] = [
    "io",
    "os",
    "package",
    "require",
    "loadstring",
    "load",
    "dofile",
    "loadfile",
    "debug",
];

/// The five names the other denial scenario requires to be unreachable.
///
/// Kept apart from the nine above rather than merged into one list of fourteen,
/// because they are two scenarios and each has to be able to fail on its own.
///
/// # `gcinfo` is the fifth, and it is here for a different reason from the other
/// four
///
/// The first four are ways *around* isolation: two hand a chunk another chunk's
/// environment and two reach the collector. `gcinfo` grants no capability at
/// all — it only *reports* the size of the heap. It is denied because reading
/// the heap makes a script's behaviour depend on something no caller passed it.
///
/// **The deciding reason is determinism, not the side channel.** Worldgen is a
/// mod-provided callback that must be a pure function of `(seed, position)`, and
/// one that branches on heap size is not: the same seed regenerates a different
/// world, which loses a player's terrain rather than leaking a fact about the
/// server. Recorded here because the side-channel reason is the weaker one and
/// is the one somebody will later find unconvincing — whoever reads *that* as
/// the justification will conclude a heap-size reading is harmless and put it
/// back.
///
/// It is also a published surface the day it ships. Removing it afterwards
/// breaks third-party mods that read it, so the moment to decide is before
/// anybody can.
const DENIED_ENVIRONMENT_AND_COLLECTOR: [&str; 5] =
    ["getfenv", "setfenv", "collectgarbage", "newproxy", "gcinfo"];

/// The eleven names that must be reachable, each with the kind of value it is.
///
/// Asserting the *kind* and not merely the presence is what distinguishes a
/// working library from a name that has been left standing pointing at
/// something useless — a host that answered `true` to every one of these would
/// satisfy a presence check and nothing else.
const PERMITTED_ELEVEN: [(&str, &str); 11] = [
    ("math", "table"),
    ("string", "table"),
    ("table", "table"),
    ("pairs", "function"),
    ("tostring", "function"),
    ("pcall", "function"),
    ("select", "function"),
    ("type", "function"),
    ("coroutine", "table"),
    ("buffer", "table"),
    ("print", "function"),
];

/// A chunk that names every global it can reach, deduplicated and sorted, as one
/// comma-separated line.
///
/// It walks the `__index` chain to its end rather than taking one hop, and it
/// reads every table's own keys on the way. `tostring` on the key because a
/// table may be keyed by something that is not a string, and a name this test
/// cannot spell is still a name that was reachable.
const REACHABLE_GLOBALS_CHUNK: &str = r#"
local seen = {}
local names = {}
local count = 0
local visiting = _G
while type(visiting) == "table" do
    for key in pairs(visiting) do
        local name = tostring(key)
        if not seen[name] then
            seen[name] = true
            count = count + 1
            names[count] = name
        end
    end
    local meta = getmetatable(visiting)
    visiting = meta and rawget(meta, "__index") or nil
end
table.sort(names)
return table.concat(names, ",")
"#;

/// A chunk that fails to replace `print`, survives its own refusal, and calls
/// `print` anyway.
///
/// The refusal has to be caught, or the chunk aborts at the assignment and never
/// reaches the call that is the point of the scenario. What it returns is
/// whether the replacement succeeded, so one chunk witnesses both halves: the
/// replacement was refused, **and** the call that followed still reached the
/// host rather than a shadow the host cannot see.
const REPLACE_THEN_CALL_PRINT_CHUNK: &str = r#"
local replaced = pcall(function() print = function() end end)
print("from the chunk")
return tostring(replaced)
"#;

/// How the reachable set compared against the set the host declares.
///
/// A total verdict rather than two assertions a caller has to remember to write
/// in the right order. `ChunkReachedNothing` comes first because it explains
/// away everything after it: an enumeration that found no name at all agrees
/// with an empty declaration, and that agreement is the one shape this
/// comparison could reach while proving nothing.
#[derive(Debug, PartialEq, Eq)]
enum SurfaceVerdict {
    ExactlyTheDeclaredSet,
    ChunkReachedNothing,
    Differs {
        /// Reachable from a chunk and absent from the declaration: a capability
        /// that arrived without anybody deciding to admit it.
        reachable_but_undeclared: Vec<String>,
        /// Declared and not reachable: a declaration describing something that
        /// is no longer there.
        declared_but_unreachable: Vec<String>,
    },
}

fn new_host() -> Result<ScriptHost, Box<dyn Error>> {
    match ScriptHost::new() {
        Ok(host) => Ok(host),
        Err(error) => Err(format!("the host could not be constructed: {error:?}").into()),
    }
}

/// What an evaluation produced, as one comparable line.
///
/// A fault renders too, so a test expecting a value and handed a fault fails
/// with the fault in its diff rather than needing an unwrap — and a host that
/// could not run a chunk at all never reads the same as one that ran it and
/// returned nothing.
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

/// A chunk reporting the kind of value each of `names` holds, in order.
///
/// `type` is itself one of the names a scenario here asserts is available. If it
/// were missing this chunk would fault outright rather than report anything,
/// which is a louder failure than the one it was written to detect.
fn kind_report_chunk(names: &[&str]) -> String {
    let literals: Vec<String> = names.iter().map(|name| format!("\"{name}\"")).collect();
    format!(
        r#"local names = {{{}}}
local report = ""
for index = 1, #names do
    local name = names[index]
    if index > 1 then report = report .. ", " end
    report = report .. name .. "=" .. type(_G[name])
end
return report"#,
        literals.join(", ")
    )
}

/// The line `kind_report_chunk` must produce for `entries`, built from the same
/// table that drives the chunk.
fn expected_report(entries: &[(&str, &str)]) -> String {
    entries
        .iter()
        .map(|(name, kind)| format!("{name}={kind}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Every name in `entries`, for handing to `kind_report_chunk`.
fn names_of<'a>(entries: &[(&'a str, &str)]) -> Vec<&'a str> {
    entries.iter().map(|(name, _)| *name).collect()
}

/// The report a chunk of `names` produces against this host.
fn report_kinds(host: &mut ScriptHost, chunk: &str, names: &[&str]) -> String {
    outcome(host.evaluate(chunk, &kind_report_chunk(names)))
}

fn surface_verdict(reported: &str, declared: &[&str]) -> SurfaceVerdict {
    let reachable: BTreeSet<&str> = reported
        .split(',')
        .filter(|name| !name.is_empty())
        .collect();
    if reachable.is_empty() {
        return SurfaceVerdict::ChunkReachedNothing;
    }
    let declared: BTreeSet<&str> = declared.iter().copied().collect();
    let reachable_but_undeclared = named_only_in(&reachable, &declared);
    let declared_but_unreachable = named_only_in(&declared, &reachable);
    if reachable_but_undeclared.is_empty() && declared_but_unreachable.is_empty() {
        SurfaceVerdict::ExactlyTheDeclaredSet
    } else {
        SurfaceVerdict::Differs {
            reachable_but_undeclared,
            declared_but_unreachable,
        }
    }
}

fn named_only_in(left: &BTreeSet<&str>, right: &BTreeSet<&str>) -> Vec<String> {
    left.difference(right)
        .map(|name| (*name).to_owned())
        .collect()
}

#[test]
fn no_library_or_loader_the_host_denies_can_be_reached_from_a_chunk() -> TestResult {
    let mut host = new_host()?;
    let expected: Vec<(&str, &str)> = DENIED_LIBRARIES_AND_LOADERS
        .iter()
        .map(|name| (*name, "nil"))
        .collect();

    assert_eq!(
        report_kinds(&mut host, "denied-libraries", &DENIED_LIBRARIES_AND_LOADERS),
        expected_report(&expected),
        "these are the file system, the process, the loaders and the debug interface. Closing \
         the sandbox removes only some of them, so the host removes them itself — and the \
         report names each one so a single survivor is identifiable rather than hidden behind \
         a count"
    );
    Ok(())
}

#[test]
fn no_environment_or_collector_hook_the_host_denies_can_be_reached_from_a_chunk() -> TestResult {
    let mut host = new_host()?;
    let expected: Vec<(&str, &str)> = DENIED_ENVIRONMENT_AND_COLLECTOR
        .iter()
        .map(|name| (*name, "nil"))
        .collect();

    assert_eq!(
        report_kinds(
            &mut host,
            "denied-environment",
            &DENIED_ENVIRONMENT_AND_COLLECTOR
        ),
        expected_report(&expected),
        "these five survive the sandbox being closed. Two of them hand a chunk another chunk's \
         environment and two reach the collector — each a way around the isolation every other \
         guarantee here is built on. The fifth, `gcinfo`, is denied for a different reason and \
         the reason matters: it reports the size of the heap, and a worldgen callback that \
         branches on heap size is not a pure function of its seed and its position, so the same \
         seed regenerates a different world. That loses terrain rather than leaking a fact, and \
         it is why a reading that grants no capability is on this list at all"
    );
    Ok(())
}

#[test]
fn every_permitted_library_and_builtin_reaches_a_chunk_as_the_kind_of_value_it_should_be()
-> TestResult {
    let mut host = new_host()?;

    assert_eq!(
        report_kinds(&mut host, "permitted", &names_of(&PERMITTED_ELEVEN)),
        expected_report(&PERMITTED_ELEVEN),
        "a sandbox that removed everything would satisfy every denial check in this file and \
         be useless. These are what content is actually written against, and each has to \
         arrive as the library or the function it is supposed to be"
    );
    Ok(())
}

#[test]
fn the_globals_a_chunk_can_reach_are_exactly_the_ones_the_host_declares() -> TestResult {
    let mut host = new_host()?;
    let reported = outcome(host.evaluate("reachable-globals", REACHABLE_GLOBALS_CHUNK));

    assert_eq!(
        surface_verdict(&reported, &ScriptHost::PERMITTED_GLOBALS),
        SurfaceVerdict::ExactlyTheDeclaredSet,
        "the denial checks above catch a capability coming back; only this one catches a \
         capability arriving. A backend release that adds a global would otherwise be \
         reachable from every mod on the server with nothing in this suite able to see it. \
         The chunk reported: {reported}"
    );
    Ok(())
}

#[test]
fn a_chunk_that_fails_to_replace_print_still_has_its_own_call_recorded_at_the_host() -> TestResult {
    let mut host = new_host()?;
    let plain = outcome(host.evaluate("plain-print", "print(\"a plain call\")"));
    let shadowing = outcome(host.evaluate("shadowing-print", REPLACE_THEN_CALL_PRINT_CHUNK));
    let printed: Vec<&str> = host.printed().iter().map(String::as_str).collect();

    assert_eq!(
        (printed, plain.as_str(), shadowing.as_str()),
        (vec!["a plain call", "from the chunk"], "nil", "false"),
        "the host's `print` has to be the only one a chunk can reach. Installed after the \
         sandbox closes it is not: the assignment lands in a table nobody reads, the call \
         falls through to the backend's own `print`, and the mod writes to a file \
         descriptor outside every limit the host enforces — which is a capability escaping, \
         not a logging inconvenience"
    );
    Ok(())
}
