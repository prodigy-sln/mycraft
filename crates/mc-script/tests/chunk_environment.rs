//! Each chunk gets its own frozen environment, and neither it nor anything it
//! calls can write through it.
//!
//! Closing the backend's sandbox is not enough on its own. It makes the *base*
//! globals readonly and then hands the running thread a **writable** child table
//! to play in — which is the whole point of that mechanism, and which leaves an
//! assignment to a new global name succeeding. The host therefore evaluates each
//! chunk against a fresh table of its own, frozen, reading through to the
//! sandboxed globals.
//!
//! # Why a readonly table and not an `__newindex` hook
//!
//! An `__newindex` hook is bypassed by `rawset`, which is reachable. A readonly
//! table is not, because the refusal is enforced where the write lands rather
//! than by a metamethod the write can decline to trigger. One of the tests below
//! is exactly that case, and it is why the choice is settled by measurement
//! instead of taste.
//!
//! # Both halves of a scenario are the test
//!
//! Three of these have a second chunk, and it is not decoration. "The
//! assignment was rejected" passes just as well against a host that froze
//! nothing and merely raised an error at the right moment; what distinguishes
//! the two is whether a *later* chunk still sees the original. So each of those
//! tests asserts the refusal and the later observation together.
//!
//! # Why the refusal message is asserted and not only the fact of a fault
//!
//! Every one of these chunks calls or assigns through something that would also
//! fault if it were simply missing — a chunk calling a `rawset` that is not
//! there raises a script error too. A test satisfied by "some error happened"
//! would be green against a host that had removed the very capability the
//! scenario exists to prove is guarded. Matching the refusal tells the two
//! apart.
//!
//! One measured wrinkle, so an assertion is not written to expect more than a
//! correct host gives: a refusal raised at the assignment site carries the
//! chunk's name and line as a prefix, while one raised inside a builtin carries
//! no prefix at all — the error comes from the C function rather than from the
//! call site. Both are conformant, so the chunk's identity is asserted through
//! the fault's own typed field, which carries it either way, and the message is
//! matched on the part that is actually about the refusal.

use std::error::Error;

use mc_script::{ChunkName, FaultKind, ScriptFault, ScriptHost, ScriptValue};

type TestResult = Result<(), Box<dyn Error>>;

/// The name every chunk here is evaluated under.
///
/// One name, because the chunk's identity is what several of these assert and a
/// per-test name would make the expected values differ for a reason that has
/// nothing to do with what is being checked.
const CHUNK: &str = "content-chunk";

/// What a refusal to write through a frozen table says.
///
/// Matched as a fragment rather than as the whole message, because the prefix
/// naming chunk and line is present on some of these refusals and absent on
/// others, for a reason that is about where the error was raised rather than
/// about whether the write was refused.
const REFUSED: &str = "attempt to modify a readonly table";

fn new_host() -> Result<ScriptHost, Box<dyn Error>> {
    match ScriptHost::new() {
        Ok(host) => Ok(host),
        Err(error) => Err(format!("the host could not be constructed: {error:?}").into()),
    }
}

/// What an evaluation produced, as one comparable line.
///
/// A fault renders too, so a test expecting a value and handed a fault fails
/// with the fault in its diff rather than needing an unwrap.
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

/// The chunk a fault names, as a plain string.
fn chunk_of(fault: &ScriptFault) -> Option<&str> {
    fault.origin.chunk.as_ref().map(ChunkName::as_str)
}

/// How a refused write reads, as the three facts every one of these asserts:
/// the kind of fault, the chunk it names, and whether the message is about the
/// refusal rather than about something being missing.
#[derive(Debug, PartialEq, Eq)]
struct Refusal {
    kind: FaultKind,
    chunk: Option<String>,
    says_the_table_is_readonly: bool,
}

/// What the host must report for every write refused by a frozen environment.
fn a_refused_write() -> Refusal {
    Refusal {
        kind: FaultKind::ScriptError,
        chunk: Some(CHUNK.to_owned()),
        says_the_table_is_readonly: true,
    }
}

/// Evaluates `source` and describes the refusal it must have produced.
///
/// Returns the fault's own rendering alongside, so an assertion that fails has
/// the whole message in front of whoever reads it rather than a bare `false`.
fn refusal_from(host: &mut ScriptHost, source: &str) -> Result<(Refusal, String), Box<dyn Error>> {
    let Err(fault) = host.evaluate(CHUNK, source) else {
        return Err(format!("`{source}` must be refused by a frozen environment").into());
    };
    let refusal = Refusal {
        kind: fault.kind,
        chunk: chunk_of(&fault).map(str::to_owned),
        says_the_table_is_readonly: fault.cause.contains(REFUSED),
    };
    Ok((refusal, fault.to_string()))
}

#[test]
fn a_chunk_cannot_add_a_global_of_its_own() -> TestResult {
    let mut host = new_host()?;

    let (refusal, rendered) = refusal_from(&mut host, "newname = 1")?;

    assert_eq!(
        refusal,
        a_refused_write(),
        "closing the backend's sandbox leaves this assignment succeeding, because the thread \
         is handed a writable table of its own. A mod that can add a global can collide with \
         every other mod that reads one. The host reported: {rendered}"
    );
    Ok(())
}

#[test]
fn a_chunk_declares_a_local_and_returns_its_value_to_the_host() -> TestResult {
    let mut host = new_host()?;

    assert_eq!(
        outcome(host.evaluate(CHUNK, "local count = 42\nreturn count")),
        "integer 42",
        "freezing the environment must not make the environment useless: ordinary script — a \
         local, a value, a return — has to work, and a whole number has to come back as a \
         whole number rather than as something the engine has to round"
    );
    Ok(())
}

#[test]
fn a_chunk_cannot_replace_a_shared_library_function_for_the_chunks_that_follow_it() -> TestResult {
    let mut host = new_host()?;

    let (refusal, rendered) = refusal_from(&mut host, "string.format = function() end")?;
    let later = outcome(host.evaluate("later-chunk", "return string.format('%d', 7)"));

    assert_eq!(
        (refusal, later.as_str()),
        (a_refused_write(), "7"),
        "the libraries are shared by every mod on the server. One mod replacing `string.format` \
         changes what every other mod's calls do, and the mod that breaks is not the mod that \
         did it. The refusal alone would pass against a host that froze nothing and merely \
         errored — the later chunk still getting `7` is what says otherwise. The host \
         reported: {rendered}"
    );
    Ok(())
}

#[test]
fn a_chunk_cannot_smuggle_a_global_past_the_freeze_with_a_raw_write() -> TestResult {
    let mut host = new_host()?;

    let (refusal, rendered) = refusal_from(&mut host, "rawset(_G, 'smuggled', 1)")?;
    let later = outcome(host.evaluate("later-chunk", "return type(_G.smuggled)"));

    assert_eq!(
        (refusal, later.as_str()),
        (a_refused_write(), "nil"),
        "this is the case that decides the mechanism. A freeze built on an assignment hook is \
         bypassed by exactly this call, because a raw write is defined as the one that does not \
         trigger the hook; a frozen table refuses it because the refusal lives where the write \
         lands. The host reported: {rendered}"
    );
    Ok(())
}

/// A chunk that reaches one level above its own environment and tries to plant
/// a name there, by both routes a chunk has.
///
/// The environment is frozen and so is its metatable — but the table that
/// metatable points *at* is the one every chunk reads through, and freezing the
/// first two does not freeze the third. Both attempts are caught so the chunk
/// survives to report what happened, because whether the write was refused is
/// the fact under test rather than an accident of where the chunk stopped.
const PLANT_ONE_LEVEL_UP_CHUNK: &str = r#"
local above = getmetatable(_G).__index
local by_raw_write = pcall(function() rawset(above, 'smuggled_raw', 1) end)
local by_assignment = pcall(function() above.smuggled_assigned = 1 end)
return tostring(by_raw_write) .. "," .. tostring(by_assignment)
"#;

/// A chunk reporting whether either planted name reached it.
const READ_THE_PLANTED_NAMES_CHUNK: &str =
    "return type(_G.smuggled_raw) .. ',' .. type(_G.smuggled_assigned)";

/// Carries no scenario of its own, and is here because the scenario nearest to
/// it cannot see this.
///
/// That scenario writes `rawset(_G, 'smuggled', 1)`, which lands on the chunk's
/// own frozen environment and is refused. This writes the same verb to the table
/// one level above it — same name, same call, one hop apart — and against a host
/// that froze only the environment and its metatable, the write succeeds and
/// every later chunk reads the planted name. The scenario stays green
/// throughout, which is the single-witness shape: one test standing between a
/// code path and silence, reaching it by only one route.
///
/// Both routes are tried because they are refused by different machinery, and a
/// host could plausibly stop one and not the other.
#[test]
fn a_chunk_cannot_plant_a_global_in_the_table_every_other_chunk_reads_through() -> TestResult {
    let mut host = new_host()?;

    let planting = outcome(host.evaluate(CHUNK, PLANT_ONE_LEVEL_UP_CHUNK));
    let later = outcome(host.evaluate("later-chunk", READ_THE_PLANTED_NAMES_CHUNK));

    assert_eq!(
        (planting.as_str(), later.as_str()),
        ("false,false", "nil,nil"),
        "the environment a chunk gets is frozen, and so is the metatable behind it — but the \
         table that metatable reads through is shared by every chunk on the server, and \
         freezing two of the three leaves the third writable. A mod that plants a name there \
         has added to the global environment exactly as if nothing were frozen at all. The \
         second chunk is the half that matters: a refusal that did not stick would still let \
         the name through"
    );
    Ok(())
}

#[test]
fn a_chunk_cannot_reach_behind_a_shared_metatable_to_change_what_every_string_does() -> TestResult {
    let mut host = new_host()?;

    let (refusal, rendered) = refusal_from(&mut host, "getmetatable('').__index = {}")?;
    let later = outcome(host.evaluate("later-chunk", "return ('hello'):upper()"));

    assert_eq!(
        (refusal, later.as_str()),
        (a_refused_write(), "HELLO"),
        "every string in every mod shares one metatable, so writing through it is the widest \
         reach a chunk has: it changes what indexing a string means everywhere at once, \
         including inside the host's own calls. The host reported: {rendered}"
    );
    Ok(())
}
