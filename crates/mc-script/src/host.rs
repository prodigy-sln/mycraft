//! The host: what the engine holds, and the only way into script.

use std::fmt;
use std::num::NonZeroUsize;

use crate::Attachment;
use crate::dispatch::{DispatchReport, Registry};
use crate::fault::{FaultKind, ScriptFault, ScriptOrigin};
use crate::limits::HostLimits;
use crate::luau::handle::{ScriptFunction, ScriptTable};
use crate::luau::vm::{Memory, Outcome, Vm};
use crate::value::{FieldNames, ScriptValue};

/// The host could not be built. Never a script fault: nothing content does
/// produces one of these, and everything content does produces a
/// [`ScriptFault`] instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError {
    cause: String,
}

impl HostError {
    /// A failure the scripting backend reported while the host was being set up.
    pub(crate) fn backend(cause: impl fmt::Display) -> Self {
        Self {
            cause: cause.to_string(),
        }
    }

    /// The two memory limits cannot both hold.
    ///
    /// Refused at construction rather than discovered at the first invocation. A
    /// backstop that does not clear the state's own baseline plus one
    /// invocation's cap means the very first callback is already short of
    /// memory, so every allocation fault such a host reported would be about its
    /// own configuration rather than about the mod that happened to be running.
    pub(crate) fn unusable_memory(backstop: usize, baseline: usize, cap: usize) -> Self {
        Self {
            cause: format!(
                "the absolute memory backstop of {backstop} bytes leaves no room for one \
                 invocation: the state holds {baseline} bytes before any content runs, and each \
                 invocation may add {cap}"
            ),
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "the scripting host could not start: {}",
            self.cause
        )
    }
}

impl std::error::Error for HostError {}

/// A sandboxed, budgeted script host.
///
/// It owns one script state and everything that bounds what content may do
/// inside it. It is `!Send` and stays that way — the state is pinned to the
/// thread that made it, which is both simpler and better for determinism than
/// the alternative.
#[derive(Debug)]
pub struct ScriptHost {
    vm: Vm,
    limits: HostLimits,
    printed: Vec<String>,
    registry: Registry,
}

impl ScriptHost {
    /// The capabilities removed before any content runs.
    ///
    /// **Public because the harness that proves they are gone must check this
    /// list rather than a copy of it.** A harness carrying its own copy of these
    /// names agrees with the host by construction: delete the removal and both
    /// still say the same thing.
    ///
    /// Five of these are removed by closing the backend's sandbox and the rest
    /// are not — `os`, `require`, `loadstring`, `debug`, `getfenv`, `setfenv`,
    /// `collectgarbage`, `newproxy` and `gcinfo` survive it, so the host removes
    /// all fourteen itself and does not rely on which half is which.
    ///
    /// `gcinfo` is denied for **determinism**, not because reading the heap size
    /// leaks anything worth having. It makes script's behaviour a function of
    /// the collector's state rather than of its own inputs, and world generation
    /// is the caller that cannot survive that: a generator branching on heap
    /// size returns different terrain for one seed, which loses a world rather
    /// than leaking a number. `collectgarbage` is denied for the same reason
    /// from the other direction — one reads the collector, one drives it.
    pub const DENIED_GLOBALS: [&'static str; 14] = [
        "io",
        "os",
        "package",
        "require",
        "loadstring",
        "load",
        "dofile",
        "loadfile",
        "debug",
        "getfenv",
        "setfenv",
        "collectgarbage",
        "newproxy",
        "gcinfo",
    ];

    /// Every name a content chunk can reach, declared rather than discovered.
    ///
    /// This is the list nobody had written down, and writing it down is the
    /// point. A deny list catches a capability being *reintroduced* and cannot
    /// catch one being *added*: a backend release that ships a new global would
    /// otherwise be reachable from every mod on the server with nothing able to
    /// notice. Compared against what a chunk can actually reach, in both
    /// directions.
    ///
    /// It is written out by hand and must stay that way. Computed by asking the
    /// running state the same question the comparison asks it, it would agree
    /// with itself forever.
    ///
    /// Enumerating it is what found `gcinfo`, which nobody had thought to name
    /// and which is now denied above. That is the whole argument for keeping
    /// this list: a denied set derived by asking what should be removed keeps
    /// missing exactly what nobody thought of.
    pub const PERMITTED_GLOBALS: [&'static str; 31] = [
        "_G",
        "_VERSION",
        "assert",
        "bit32",
        "buffer",
        "coroutine",
        "error",
        "getmetatable",
        "integer",
        "ipairs",
        "math",
        "next",
        "pairs",
        "pcall",
        "print",
        "rawequal",
        "rawget",
        "rawlen",
        "rawset",
        "select",
        "setmetatable",
        "string",
        "table",
        "tonumber",
        "tostring",
        "type",
        "typeof",
        "unpack",
        "utf8",
        "vector",
        "xpcall",
    ];

    /// A host under the shipped defaults.
    ///
    /// # Errors
    ///
    /// Fails if the scripting backend could not be started or sandboxed.
    pub fn new() -> Result<Self, HostError> {
        Self::with_limits(HostLimits::default())
    }

    /// A host under `limits`.
    ///
    /// # Errors
    ///
    /// Fails if the scripting backend could not be started or sandboxed.
    pub fn with_limits(limits: HostLimits) -> Result<Self, HostError> {
        let memory = Memory {
            cap: limits.memory_cap.get(),
            backstop: limits.memory_backstop.get(),
        };
        Ok(Self {
            vm: Vm::new(&Self::DENIED_GLOBALS, memory, limits.retained_print_bytes)?,
            limits,
            printed: Vec::new(),
            registry: Registry::default(),
        })
    }

    /// Registers `callback` against `attachment`.
    pub fn attach(&mut self, attachment: Attachment, callback: ScriptFunction) {
        self.registry.attach(attachment, callback);
    }

    /// Runs one dispatch round over `seed`.
    ///
    /// Each invocation is a guarded entry with a whole fresh budget, so one
    /// attachment exhausting its budget costs the next one nothing. An
    /// attachment nothing is attached to is passed over.
    pub fn dispatch(&mut self, seed: &[Attachment]) -> DispatchReport {
        let report = self.registry.run(&self.vm, seed, &self.limits);
        self.printed.extend(self.vm.take_printed());
        report
    }

    /// Whether `attachment` has stopped being invoked.
    pub fn is_quarantined(&self, attachment: &Attachment) -> bool {
        self.registry.is_quarantined(attachment)
    }

    /// Lifts quarantine, leaving the callback in place, and reports whether the
    /// attachment was quarantined.
    ///
    /// Not `detach`: nothing unloads mods yet, so there is no operation for
    /// taking a callback away — only for letting one run again.
    pub fn release(&mut self, attachment: &Attachment) -> bool {
        self.registry.release(attachment)
    }

    /// Reads a field from a script-supplied table without invoking script.
    ///
    /// A table a mod handed the engine can carry a metatable, and an ordinary
    /// indexed read consults it — which means running the mod's code on the
    /// host's schedule, outside any budget, at a moment the mod chose, and
    /// believing whatever it returns. This reads raw instead, so a metatable can
    /// neither run on the host's schedule nor observe which fields were read.
    ///
    /// `None` covers both a field the table does not have and one holding
    /// nothing, because script does not distinguish them either.
    pub fn read_field(&self, table: &ScriptTable, field: &str) -> Option<ScriptValue> {
        self.vm.read_field(table, field)
    }

    /// Which fields a script-supplied table holds, without invoking script.
    ///
    /// The other half of [`Self::read_field`], and the half a caller needs to
    /// tell a **typo from an absence**: a host that can read `solid` but cannot
    /// ask what fields exist has no way to distinguish a declaration that left a
    /// field out from one that misspelled it, so a misspelling becomes a
    /// silently lost declaration.
    ///
    /// Enumeration meets `__iter`, `__pairs` and `__len` rather than the
    /// `__index` a named read meets, and it consults none of them. The names
    /// come back **sorted lexicographically** — see [`FieldNames`] for why that
    /// happens here rather than at the caller.
    ///
    /// `most` bounds what this will copy out, and it is a parameter because the
    /// number is the caller's policy: this crate holds no opinion about how many
    /// fields any particular kind of declaration may carry. A table holding more
    /// answers [`FieldNames::MoreThanAllowed`] without carrying any of them, so
    /// the bound is reached before the allocation rather than after it.
    pub fn field_names(&self, table: &ScriptTable, most: NonZeroUsize) -> FieldNames {
        self.vm.field_names(table, most)
    }

    /// What the script state holds once everything unreachable has been
    /// collected.
    ///
    /// The collected figure rather than the raw one, and the difference is not
    /// cosmetic: over a megabyte of garbage was measured surviving until an
    /// explicit collection, so a raw reading answers with memory nothing is
    /// holding. Anything deciding whether the host is short of memory has to ask
    /// this rather than the cheap question.
    pub fn collected_memory_in_use(&self) -> usize {
        self.vm.collected_memory_in_use()
    }

    /// How often `attachment` has been invoked over the life of this host.
    pub fn invocation_count(&self, attachment: &Attachment) -> u64 {
        self.registry.invocation_count(attachment)
    }

    /// What this host is willing to spend on script.
    pub fn limits(&self) -> &HostLimits {
        &self.limits
    }

    /// What content has printed, in the order it printed it.
    ///
    /// **Unbounded, script-controlled text**, on the same terms as
    /// [`ScriptFault::cause`]: a mod chooses both the content and the length,
    /// and whoever routes this to a log inherits both.
    pub fn printed(&self) -> &[String] {
        &self.printed
    }

    /// How many printed lines the host was handed and did not keep.
    ///
    /// Reported rather than left to be inferred from a short record, because
    /// [`printed`](Self::printed) alone cannot tell "the mod printed nothing"
    /// from "the host stopped keeping what the mod printed" — and a record that
    /// cannot tell those apart is an absence that reads as agreement. Counted
    /// over the host's whole life, on the same terms as the allowance in
    /// [`HostLimits::retained_print_bytes`].
    pub fn dropped_print_lines(&self) -> u64 {
        self.vm.dropped_print_lines()
    }

    /// Evaluates a chunk in its own frozen environment, under a budget.
    ///
    /// Evaluating is an entry into script like any other, so it is guarded like
    /// any other: the rule that there is no unbudgeted path from the engine into
    /// script is categorical, and a chunk whose top level never returns would
    /// otherwise hang the host with nothing able to stop it.
    ///
    /// A fault from here names the chunk and neither a subject nor a component,
    /// because a chunk runs before any attachment exists.
    ///
    /// # Errors
    ///
    /// Returns a fault if the chunk could not be compiled, raised an error, or
    /// was stopped for exceeding a limit.
    ///
    /// A fault carries four optional identities and a cause, which puts it over
    /// the size at which returning one by value is flagged as a large `Err`.
    /// Boxing it is declined: the shape of that type is a decision — flat rather
    /// than split, with the kind, the line and the refused target as typed
    /// fields instead of text — and a heuristic about move size does not get to
    /// overturn a decision about what a fault *is*. Nothing here is on a hot
    /// path: no tick calls into this crate, and a fault is by construction the
    /// rare branch. Scoped to this function rather than the crate, so a later
    /// one where boxing *would* be right is still told so.
    #[allow(clippy::result_large_err)]
    pub fn evaluate(&mut self, name: &str, source: &str) -> Result<ScriptValue, ScriptFault> {
        let budget = self.limits.call_and_loop_budget.get();
        let outcome = self.vm.evaluate(name, source, budget);
        self.printed.extend(self.vm.take_printed());
        match outcome {
            Outcome::Produced(value) => Ok(value),
            Outcome::Failed { kind, line, cause } => Err(chunk_fault(name, kind, line, cause)),
        }
    }
}

/// A fault raised by evaluating a chunk.
///
/// The chunk's name comes from what the caller asked the chunk to be called, not
/// from parsing it back out of the backend's message — a refusal raised inside a
/// builtin carries no prefix at all, and a fault that could name its chunk only
/// when the backend happened to spell it would lose the name exactly where it is
/// least obvious.
fn chunk_fault(name: &str, kind: FaultKind, line: Option<u32>, cause: String) -> ScriptFault {
    ScriptFault {
        origin: ScriptOrigin {
            chunk: Some(crate::ChunkName::new(name)),
            round: None,
        },
        subject: None,
        component: None,
        kind,
        line,
        refused_target: None,
        cause,
    }
}
