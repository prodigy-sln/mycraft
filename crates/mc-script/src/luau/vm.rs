//! Constructing the script state, and running one guarded entry into it.
//!
//! # The construction order is load-bearing, not stylistic
//!
//! Everything the host takes away or installs happens **before** the sandbox is
//! closed. Afterwards the running thread reads through a child table, so a write
//! to a global lands in the child and the read falls straight through to an
//! untouched parent: setting a denied global to `nil` returns success and
//! removes nothing, and a host `print` installed late is bypassed entirely.
//!
//! That last one is an escape rather than a logging inconvenience. The backend's
//! own `print` writes to C's `stdout` — a different buffer from the host's,
//! flushed at process exit — so a chunk that falls through to it writes to raw
//! file descriptor 1, outside every limit and every log the host controls. The
//! capability the sandbox was supposed to have removed is back, and nothing in
//! the host can see it happen.
//!
//! # Closing the sandbox is not enough, twice over
//!
//! It removes five of the denied names and leaves the rest standing: `os`,
//! `require`, `loadstring`, `debug`, `getfenv`, `setfenv`, `collectgarbage`,
//! `newproxy` and `gcinfo`. Measured. Four of those were not the four this
//! design originally expected and `gcinfo` was on nobody's list at all, which is
//! why the permitted side is enumerated exactly rather than sampled — a list
//! derived by asking what should be removed keeps missing whatever nobody
//! thought to name.
//!
//! And it leaves the sandboxed globals table itself writable, so it is frozen
//! here. Without that, a chunk reaches it through its own environment's
//! metatable and plants a name every later chunk reads.

use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::rc::Rc;

use mlua::{Error, Function, Lua, MultiValue, Value, VmState};

use crate::fault::FaultKind;
use crate::luau::guard::{Guard, Latch};
use crate::luau::handle::{IsolationUnit, ScriptFunction, ScriptTable};
use crate::luau::print_sink::PrintSink;
use crate::luau::trampoline::{self, Returned};
use crate::luau::{env, translate};
use crate::value::{FieldNames, ScriptValue};
use crate::{Attachment, ChunkName, ComponentName, HostError, SubjectName};

/// How a guarded entry into script ended, before the host attributes it.
///
/// Two outcomes and no third, so a caller cannot be handed a state meaning
/// neither — an entry that produced nothing and failed at nothing would be a
/// shape the host would have to invent an answer for.
pub(crate) enum Outcome {
    /// The entry returned this.
    Produced(ScriptValue),
    /// The entry did not return, for this reason.
    Failed {
        kind: FaultKind,
        /// The line the backend named, where it named one.
        line: Option<u32>,
        cause: String,
    },
}

impl Outcome {
    /// Re-files a failure as the host's own condition when the invocation was
    /// entered while there was no room for it.
    ///
    /// The invocation could have failed for a reason that is not its own, and
    /// there is no way to tell from the failure which it was — so the honest
    /// answer is the one that does not accuse anybody. **The cost is real and
    /// accepted**: while the condition holds, a genuinely looping mod is excused
    /// too, and under sustained pressure quarantine is inactive for everyone.
    /// That failure is loud — a slow server an operator notices and acts on —
    /// where the alternative is silent and misdirected: an innocent mod
    /// permanently disabled with the blame filed against the wrong author,
    /// ending with the operator removing the wrong mod. Quarantine functioning
    /// would not help either way, because what raised the baseline lives in
    /// closure upvalues that survive it.
    fn under_host_pressure(self, pressured: bool) -> Self {
        match self {
            Self::Failed { .. } if pressured => Self::Failed {
                kind: FaultKind::HostMemoryPressure,
                line: None,
                cause: PRESSURED.to_owned(),
            },
            other => other,
        }
    }
}

/// One attachment identity out of a follow-up list, read raw.
fn identity(list: &mlua::Table, slot: usize) -> Option<Attachment> {
    let Ok(Value::Table(entry)) = list.raw_get::<Value>(slot) else {
        return None;
    };
    let subject = entry.raw_get::<String>("subject").ok()?;
    let component = entry.raw_get::<String>("component").ok()?;
    Some(Attachment {
        subject: SubjectName::new(subject),
        component: ComponentName::new(component),
    })
}

/// What the host says about a failure it will not attribute.
const PRESSURED: &str = concat!(
    "the state had no room for this invocation's whole memory allowance before it began, ",
    "so this failure may not be the running attachment's own"
);

/// The script state, its guard, and everything the host installed in it.
#[derive(Debug)]
pub(crate) struct Vm {
    lua: Lua,
    guard: Guard,
    printed: Rc<RefCell<PrintSink>>,
    /// The one script-side protected call every callback is invoked through.
    trampoline: Function,
    /// Which state this is, stamped onto every handle taken out of it.
    unit: IsolationUnit,
    /// How much one entry may add above the baseline it started from.
    cap: usize,
    /// The absolute ceiling the whole state may reach.
    backstop: usize,
}

/// The two memory limits a state is built with.
///
/// A record rather than two arguments, so the pair that has to be consistent
/// travels together.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Memory {
    /// What one entry may add above its entry baseline.
    pub(crate) cap: usize,
    /// The absolute ceiling the whole state may reach.
    pub(crate) backstop: usize,
}

impl Vm {
    /// A state with the denied names removed, the host's `print` installed, the
    /// sandbox closed over both, the interrupt armed and the allocator capped.
    pub(crate) fn new(
        denied: &[&str],
        memory: Memory,
        retained_print_bytes: NonZeroUsize,
    ) -> Result<Self, HostError> {
        let lua = Lua::new();
        let printed = Rc::new(RefCell::new(PrintSink::new(retained_print_bytes)));
        let guard = Guard::new();
        let trampoline = prepare(&lua, denied, &printed, &guard).map_err(HostError::backend)?;

        // Measured once the state is built and sandboxed, because that is what
        // it holds before any content runs.
        let baseline = lua.used_memory();
        if memory.backstop <= baseline.saturating_add(memory.cap) {
            return Err(HostError::unusable_memory(
                memory.backstop,
                baseline,
                memory.cap,
            ));
        }
        lua.set_memory_limit(memory.backstop)
            .map_err(HostError::backend)?;

        Ok(Self {
            lua,
            guard,
            printed,
            trampoline,
            unit: IsolationUnit::mint(),
            cap: memory.cap,
            backstop: memory.backstop,
        })
    }

    /// Where this entry's allocation must stop, measured from what the state
    /// holds right now.
    ///
    /// A delta above the entry baseline rather than an absolute, which is what
    /// makes the fault attributable to this invocation instead of to whatever
    /// the state was already holding on its behalf.
    fn ceiling(&self) -> usize {
        self.lua.used_memory().saturating_add(self.cap)
    }

    /// Ends the incremental cycle in progress, then sweeps.
    ///
    /// **Two calls, not one, and it is measured.** After an abort at a 1 MiB cap
    /// the state still held 1,434,679 B and the next half-megabyte allocation
    /// failed outright; after two collections usage was back to exactly its
    /// baseline and the same allocation returned. It runs only on the allocation
    /// path, so ordinary dispatch pays nothing.
    fn reclaim(&self) -> mlua::Result<()> {
        self.lua.gc_collect()?;
        self.lua.gc_collect()
    }

    /// What the state holds once everything unreachable has been collected.
    ///
    /// A collection that fails leaves the figure higher than it might have been,
    /// which is the conservative direction for every decision made on it: it can
    /// report the host short of memory when a collection would have cleared it,
    /// and it cannot hide a shortage that is real. The path where a failed
    /// collection changes what an operator should do reports it in the fault
    /// instead.
    pub(crate) fn collected_memory_in_use(&self) -> usize {
        let _collection = self.reclaim();
        self.lua.used_memory()
    }

    /// Whether this invocation could fail for a reason that is not its own.
    ///
    /// The condition is *derived*, not tuned: an invocation may add
    /// [`cap`](Memory::cap) above the baseline, and if that would carry the state
    /// past the absolute backstop then the allocation this invocation is entitled
    /// to does not fit. There is no constant to choose, defend or re-choose when
    /// the backstop moves — it is the literal statement of the thing being
    /// detected rather than a proxy for it. It is justified on **correctness**,
    /// because it removes a parameter, and never on security.
    ///
    /// **Only a collected reading may conclude.** Over a megabyte of garbage was
    /// measured surviving until an explicit collection, so a raw reading reports
    /// a shortage caused by memory nothing is holding and condemns the host to
    /// permanent pressure. The raw reading is still worth taking first, because
    /// it is free and it settles the ordinary case: if there is room *without*
    /// collecting, there is room.
    fn under_pressure(&self) -> bool {
        if self.lua.used_memory().saturating_add(self.cap) <= self.backstop {
            return false;
        }
        self.collected_memory_in_use().saturating_add(self.cap) > self.backstop
    }

    /// Invokes one callback through the trampoline, under a whole fresh budget.
    pub(crate) fn invoke(&self, callback: &ScriptFunction, budget: u64) -> Outcome {
        let pressured = self.under_pressure();
        self.guard.begin(budget, self.ceiling());
        let called = self
            .trampoline
            .call::<MultiValue>((callback.handle().clone(),));
        let ended = match called {
            Ok(values) => match trampoline::returned(values) {
                Returned::Value(value) => {
                    Outcome::Produced(translate::value(value, self.unit, callback.origin_chunk()))
                }
                Returned::Raised(cause) => Outcome::Failed {
                    kind: FaultKind::ScriptError,
                    line: None,
                    cause,
                },
            },
            Err(error) => self.failed_by(&error),
        };
        // Applied to **every** way an invocation can fail, not only to the ones
        // the host stopped. Under pressure the allocation that fails is often
        // refused by the allocator and caught by the trampoline's own protected
        // call, so it arrives as an ordinary raised value — which is exactly the
        // path a reader expects to be attributable and here is not.
        ended.under_host_pressure(pressured)
    }

    /// Classifies an entry that did not return, from the guard rather than from
    /// the error's text.
    ///
    /// An entry that raised comes back with the guard still clear; one the host
    /// stopped comes back with the guard holding the reason it stopped. Reading
    /// the message instead would tie this to how a pre-1.0 dependency spells
    /// itself — and on the allocation path there is no message to read.
    fn failed_by(&self, error: &Error) -> Outcome {
        match self.guard.latch() {
            Latch::Budget => Outcome::Failed {
                kind: FaultKind::BudgetExhausted,
                line: None,
                cause: ABORTED.to_owned(),
            },
            Latch::Memory => Outcome::Failed {
                kind: FaultKind::Allocation,
                line: None,
                cause: allocation_cause(self.cap, self.guard.observed(), self.reclaim().is_err()),
            },
            Latch::Clear => failed(
                trampoline::classify_backend_error(error),
                &translate::message_of(error),
            ),
        }
    }

    /// Reads one field of a script table without running any script.
    ///
    /// **A raw read, and the rawness is the whole point.** An ordinary indexed
    /// read consults `__index`, which is script the table's author chose — so a
    /// host reading a field the ordinary way runs a mod's code on the host's own
    /// schedule, unbudgeted, at a moment the mod picked, and hands back whatever
    /// that code decided to say. Reading raw means a metatable can neither run
    /// on the host's schedule nor observe which fields the host looked at.
    ///
    /// Absent and present-but-nothing are one state in script, so they get one
    /// answer here. Spending the `Option` on telling them apart would make every
    /// read `Some` and leave it saying nothing.
    pub(crate) fn read_field(&self, table: &ScriptTable, field: &str) -> Option<ScriptValue> {
        match table.handle().raw_get::<Value>(field) {
            Ok(Value::Nil) | Err(_) => None,
            Ok(value) => Some(translate::value(value, self.unit, table.origin_chunk())),
        }
    }

    /// The keys a script table holds, without running any script to find them.
    ///
    /// **Raw, and against a different metamethod than [`Self::read_field`].** A
    /// named read is exposed to `__index`; an enumeration is exposed to `__iter`,
    /// `__pairs` and `__len`. `Table::pairs` walks the table itself rather than
    /// asking it how to be walked, which is what keeps a mod's code off the
    /// host's schedule here — verified rather than assumed: a table whose
    /// `__len` reports zero still enumerates every key it holds, which is the
    /// half that is actually reachable, since `Table::len` *does* consult the
    /// metamethod and would report a full declaration as carrying nothing.
    ///
    /// **The bound binds inside the walk.** Filling a vector and measuring it
    /// afterwards has already made the allocation the bound exists to refuse, so
    /// the walk stops at the first key past `most` and carries none of them back.
    ///
    /// A key that is not a string is rendered by the same rendering `print`
    /// uses, never skipped: a table may be keyed by anything, and a key nobody
    /// intended to write is exactly the one an unrecognised-field check must
    /// still see.
    pub(crate) fn field_names(&self, table: &ScriptTable, most: NonZeroUsize) -> FieldNames {
        let allowed = most.get();
        // One past the allowance and no further: enough to tell "exactly the
        // allowance" from "more than it", and never the whole of a table the
        // bound exists to refuse.
        let walked: Result<Vec<String>, Error> = table
            .handle()
            .pairs::<Value, Value>()
            .take(allowed.saturating_add(1))
            .map(|pair| pair.map(|(key, _)| translate::render(&key)))
            .collect();
        // Converting a key to `Value` cannot fail, so an error here is the walk
        // itself giving up. Reporting the table as over the bound refuses the
        // declaration, where handing back the keys gathered so far would accept
        // it while having quietly lost the rest — and a lost key is precisely
        // the typo this enumeration exists to catch.
        let Ok(mut names) = walked else {
            return FieldNames::MoreThanAllowed { allowed };
        };
        if names.len() > allowed {
            return FieldNames::MoreThanAllowed { allowed };
        }
        names.sort();
        FieldNames::Enumerated(names)
    }

    /// The follow-up work a callback's return value asks for.
    ///
    /// A callback requests work by returning a table with a `follow_up` field
    /// holding an array of `{ subject, component }` entries. Anything else is a
    /// result and requests nothing.
    ///
    /// **Every read here is raw** — the field, each slot, and both identity
    /// fields. This is the one place the host reads a value script chose the
    /// shape of, and reading any part of it the ordinary way would let a
    /// metatable on the returned table run on the host's schedule, between
    /// invocations, outside the guard that was just cleared. An entry that is
    /// not two strings is passed over rather than guessed at.
    pub(crate) fn requested_follow_up(&self, produced: &ScriptValue) -> Vec<Attachment> {
        let ScriptValue::Table(table) = produced else {
            return Vec::new();
        };
        let Ok(Value::Table(list)) = table.handle().raw_get::<Value>("follow_up") else {
            return Vec::new();
        };
        (1..=list.raw_len())
            .filter_map(|slot| identity(&list, slot))
            .collect()
    }

    /// The lines content has printed since the host last collected them.
    pub(crate) fn take_printed(&self) -> Vec<String> {
        self.printed.borrow_mut().drain()
    }

    /// How many printed lines the host was handed and did not keep.
    pub(crate) fn dropped_print_lines(&self) -> u64 {
        self.printed.borrow().dropped()
    }

    /// Compiles and runs one chunk under a whole fresh budget.
    pub(crate) fn evaluate(&self, name: &str, source: &str, budget: u64) -> Outcome {
        let environment = match env::frozen(&self.lua) {
            Ok(environment) => environment,
            Err(error) => return failed(FaultKind::ScriptError, &translate::message_of(&error)),
        };
        let compiled = self
            .lua
            .load(source)
            .set_name(name)
            .set_environment(environment)
            .into_function();
        let compiled = match compiled {
            Ok(compiled) => compiled,
            Err(error) => return failed(FaultKind::Compilation, &translate::message_of(&error)),
        };

        self.guard.begin(budget, self.ceiling());
        match compiled.call::<Value>(()) {
            Ok(value) => {
                Outcome::Produced(translate::value(value, self.unit, &ChunkName::new(name)))
            }
            Err(error) => self.failed_by(&error),
        }
    }
}

/// Everything that has to happen before the sandbox closes, in the order it has
/// to happen in.
fn prepare(
    lua: &Lua,
    denied: &[&str],
    printed: &Rc<RefCell<PrintSink>>,
    guard: &Guard,
) -> mlua::Result<Function> {
    install_print(lua, printed)?;
    let trampoline = trampoline::build(lua)?;
    for name in denied {
        lua.globals().set(*name, Value::Nil)?;
    }
    lua.sandbox(true)?;
    lua.globals().set_readonly(true);
    arm_interrupt(lua, guard);
    Ok(trampoline)
}

/// Installs the only `print` a chunk can reach.
fn install_print(lua: &Lua, printed: &Rc<RefCell<PrintSink>>) -> mlua::Result<()> {
    let sink = Rc::clone(printed);
    let print = lua.create_function(move |_, arguments: MultiValue| {
        let rendered = translate::render_all(&arguments.into_iter().collect::<Vec<_>>());
        sink.borrow_mut().record(rendered);
        Ok(())
    })?;
    lua.globals().set("print", print)
}

/// Arms the interrupt that charges the budget and refuses once latched.
///
/// The callback does nothing but read and write cells. It must never call back
/// into script: the backend's interrupt dispatch carries a recursion guard that
/// silently continues on a re-entrant interrupt, which would make a trip taken
/// inside it invisible.
fn arm_interrupt(lua: &Lua, guard: &Guard) {
    let charged = guard.clone();
    lua.set_interrupt(move |lua| {
        if charged.charge(lua.used_memory()) {
            Ok(VmState::Continue)
        } else {
            Err(mlua::Error::runtime(ABORTED))
        }
    });
}

/// What the host raises from inside the interrupt when an entry may not
/// continue.
///
/// It is a message script never sees the inside of — the latch stops every frame
/// that could catch it — and it is replaced by the host's own account of the
/// abort before anything reads it.
const ABORTED: &str = "script exceeded a limit the host enforces";

fn failed(kind: FaultKind, message: &str) -> Outcome {
    let (line, cause) = translate::split_location(message);
    Outcome::Failed { kind, line, cause }
}

/// What the host says about an allocation it stopped.
///
/// **Composed here rather than passed through, because there is nothing to pass
/// through.** Measured, the underlying error is literally `MemoryError("<nil>")`
/// — no line and no message — and a traceback taken afterwards is empty because
/// the stack has already unwound. The raw formatter is right for a value script
/// raised and, applied to this one, renders nothing at all: the fault would name
/// its subject and its component and then say literally nothing about why. An
/// empty cause and a cause that was never populated read identically.
///
/// It states the configured cap, so a formatter emitting a constant string
/// reddens the day the cap changes, and the usage seen when the entry was
/// stopped, so a reader can tell "asked for slightly too much" from "asked for
/// far too much".
/// A collection that failed is reported rather than discarded, because it is the
/// difference between one invocation having gone wrong and the host being unable
/// to give the memory back — which is what the next invocation runs into.
fn allocation_cause(cap: usize, observed: usize, unreclaimed: bool) -> String {
    let stated = format!(
        "script allocated more than the {cap} bytes one invocation may hold above the memory it \
         started with; {observed} bytes were in use when it was stopped"
    );
    if unreclaimed {
        format!("{stated}, and the host could not collect it afterwards")
    } else {
        stated
    }
}
