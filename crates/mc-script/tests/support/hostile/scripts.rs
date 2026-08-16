//! The hostile scripts, written against the host's own declarations.
//!
//! Nothing here states a limit or a name of its own. The bomb's size comes from
//! the host's cap, the cascade's fan-out from the host's two queue bounds, and
//! the escape probe's list of names from `ScriptHost::DENIED_GLOBALS` — which is
//! why that constant is public. A script built from a transcription of any of
//! them would keep saying the same thing after the host stopped enforcing it.

use mc_script::ScriptHost;

use super::{HostileCase, Shape};

/// The subject every hostile case attaches under. Its component is the case's
/// own name, so one case's fault total and quarantine state are its own.
pub(crate) const HOSTILE_SUBJECT: &str = "hostile-mod";

/// The field the hostile table does not hold, so that reading it is the moment a
/// metatable would get its turn.
pub(crate) const A_FIELD_THE_TABLE_LACKS: &str = "ash";

/// What the hostile `__index` says if it ever runs.
///
/// A metamethod that only counted could not tell a host that read raw from a
/// probe that was never reachable. This reaches the host through `print`, which
/// is a second observable and one the harness does not have to trust the script
/// for.
const WHAT_THE_METAMETHOD_SAYS: &str = "the metamethod got its turn";

/// How big each piece the bomb allocates is.
///
/// Small enough that the interrupt watches usage climb over many ticks rather
/// than the allocator refusing one huge jump — the shape the cap was measured
/// against.
const PIECE_BYTES: usize = 4096;

/// How far past the host's own cap the bomb asks for.
///
/// Bounded rather than endless, and that is deliberate: against a host with no
/// cap at all an endless bomb takes the run down with it and reports nothing,
/// where this one returns.
const CAPS_ASKED_FOR: usize = 4;

/// A callback whose loop never terminates.
const NEVER_RETURNS: &str = "return function()\n\twhile true do end\nend\n";

/// A callback that raises an error of its own.
const RAISES_AN_ERROR: &str = "return function()\n\terror('this mod is broken')\nend\n";

/// The script this case runs, built against `host`'s own declarations.
pub(crate) fn source_of(case: &HostileCase, host: &ScriptHost) -> String {
    match case.shape() {
        Shape::NeverReturns => NEVER_RETURNS.to_owned(),
        Shape::RaisesAnError => RAISES_AN_ERROR.to_owned(),
        Shape::AllocatesPastTheCap => allocates_past_the_cap(host.limits().memory_cap.get()),
        Shape::ProbesEveryDeniedGlobal => probes_every_denied_global(),
        Shape::RequestsFollowUpForever => requests_follow_up_forever(host, case.name),
        Shape::SuppliesATableThatCounts => supplies_a_table_that_counts(),
        Shape::Supplied(source) => source.to_owned(),
    }
}

/// A callback that allocates a fixed multiple of the host's own cap, in pieces.
///
/// **Buffers rather than strings, and it is load-bearing.** The backend shares
/// identical strings, so a bomb built from one repeated string allocates it once
/// and grows the state by almost nothing — measured — while every count in a
/// test written against it still reads plausibly. A buffer is never interned and
/// never shared.
fn allocates_past_the_cap(cap: usize) -> String {
    let pieces = cap.saturating_mul(CAPS_ASKED_FOR).div_ceil(PIECE_BYTES);
    format!(
        "return function()\n\
         \tlocal kept = {{}}\n\
         \tfor index = 1, {pieces} do kept[index] = buffer.create({PIECE_BYTES}) end\n\
         \treturn #kept\n\
         end\n"
    )
}

/// A chunk that asks, from inside its own environment, about every global the
/// host declares denied, and reports both what it was asked and what answered.
///
/// It reports the names it was asked about as well as the survivors, so that a
/// harness quietly narrowing its probe is visible from outside rather than
/// reporting a clean escape. The read is an ordinary indexed one — the chain a
/// content chunk really reads through — because a raw read of the chunk's own
/// environment would find nothing whatever the host had left standing.
pub(crate) fn probes_every_denied_global() -> String {
    let names: Vec<String> = ScriptHost::DENIED_GLOBALS
        .iter()
        .map(|name| format!("'{name}'"))
        .collect();
    format!(
        "local names = {{{}}}\n\
         local standing = {{}}\n\
         for index = 1, #names do\n\
         \tif _G[names[index]] ~= nil then standing[#standing + 1] = names[index] end\n\
         end\n\
         return table.concat(names, ',') .. '|' .. table.concat(standing, ',')\n",
        names.join(", ")
    )
}

/// A callback that asks for a fan-out of follow-up work on every invocation of
/// its first round, then stops asking.
///
/// It stops so that the queue can be drained afterwards and the host handed on
/// to the next case empty. A cascade that never stopped asking would leave every
/// later case running behind somebody else's queue, which is a fixture defect
/// wearing the clothes of a containment failure.
fn requests_follow_up_forever(host: &ScriptHost, name: &str) -> String {
    let limits = host.limits();
    let asks_for = limits.round_bound.get();
    let entry = format!("{{ subject = '{HOSTILE_SUBJECT}', component = '{name}' }}");
    let request = vec![entry; fan_out_for(host)].join(", ");
    format!(
        "local calls = 0\n\
         return function()\n\
         \tcalls = calls + 1\n\
         \tif calls > {asks_for} then return calls end\n\
         \treturn {{ follow_up = {{ {request} }} }}\n\
         end\n"
    )
}

/// How many entries each invocation asks for, derived from the host's own two
/// bounds.
///
/// One invocation consumes one entry and adds this many, so a round of
/// `round_bound` invocations leaves `round_bound * (fan_out - 1)` more waiting
/// than it took. That has to exceed `pending_bound` for the queue to reach the
/// point where the host refuses, and it is doubled so the refusal happens in the
/// middle of the round rather than on its last invocation.
fn fan_out_for(host: &ScriptHost) -> usize {
    let limits = host.limits();
    let per_invocation = limits
        .pending_bound
        .get()
        .div_ceil(limits.round_bound.get());
    usize::try_from(per_invocation)
        .unwrap_or(usize::MAX)
        .saturating_mul(2)
        .saturating_add(1)
}

/// A callback that hands back a table whose `__index` counts and speaks, and
/// afterwards reports what the counter reads.
///
/// The counter, the metatable and the table share one chunk because they have
/// to: each chunk is evaluated in its own frozen environment, so a second chunk
/// can see none of them.
fn supplies_a_table_that_counts() -> String {
    format!(
        "local counter = {{ hits = 0 }}\n\
         local supplied = setmetatable({{}}, {{\n\
         \t__index = function()\n\
         \t\tprint('{WHAT_THE_METAMETHOD_SAYS}')\n\
         \t\tcounter.hits = counter.hits + 1\n\
         \t\treturn 'a value no mod stored'\n\
         \tend,\n\
         }})\n\
         local first = true\n\
         return function()\n\
         \tif first then\n\
         \t\tfirst = false\n\
         \t\treturn supplied\n\
         \tend\n\
         \treturn counter.hits\n\
         end\n"
    )
}
