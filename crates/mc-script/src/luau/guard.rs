//! The latch that makes a limit mean something.
//!
//! A limit that merely raises an error once does not bound anything. Script
//! catches it — `pcall` is reachable and has to be — drops the error and starts
//! again, so `while true do pcall(f) end` re-enters the budget indefinitely and
//! the guarantee is decorative. Measured against a non-latching implementation,
//! exactly that happened: the protected call swallowed the abort and the chunk
//! returned normally.
//!
//! So the trip is **sticky**. Once the guard leaves [`Latch::Clear`], every
//! subsequent interrupt fails immediately without looking at anything, and no
//! script frame can make progress — including the frame that was going to catch
//! the error. The host clears the latch at the start of each guarded entry,
//! which is what keeps a budget per-invocation rather than a one-way ratchet
//! that stops the host after its first runaway mod.
//!
//! # Two limits, one latch
//!
//! The budget and the memory cap are checked on the same tick and trip the same
//! way. Memory is checked first: with a budget generous enough not to trip, an
//! allocation bomb has to be stopped by the cap, and a host that charged the
//! tick first would report whichever limit happened to be nearer. The first
//! reason to trip is the one reported — an entry stopped for memory that then
//! runs out of ticks while unwinding is still an allocation fault.
//!
//! One consequence is worth stating because it reads like a defect and is not:
//! after the latch trips, a script's own cleanup does not run. `pcall` handlers
//! do not execute, so content cannot rely on tidying up after an abort — build
//! and swap rather than mutating in place. Any hook that did run script after an
//! abort would re-open the hole the latch exists to close.

use std::cell::Cell;
use std::rc::Rc;

/// Why the guard stopped letting script run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Latch {
    /// Nothing has tripped; script may run.
    Clear,
    /// The entry used its whole call-and-loop budget.
    Budget,
    /// The entry allocated past what it is allowed to hold.
    Memory,
}

/// The state one guarded entry runs under, shared with the interrupt callback.
///
/// Cloning shares the state rather than copying it: the interrupt holds one
/// handle for the life of the backend and the host holds another, and both must
/// see the same trip.
#[derive(Debug, Clone)]
pub(crate) struct Guard {
    shared: Rc<Shared>,
}

#[derive(Debug)]
struct Shared {
    latch: Cell<Latch>,
    used: Cell<u64>,
    budget: Cell<u64>,
    /// The whole-state usage above which this entry has allocated too much.
    ///
    /// An absolute figure rather than a delta, because the interrupt reads an
    /// absolute one and comparing on every tick should cost a comparison. It is
    /// computed once per entry as the baseline the entry started from plus the
    /// configured cap, which is what makes the fault attributable to *this*
    /// invocation rather than to whatever the state was already holding.
    ceiling: Cell<usize>,
    /// The most script memory seen while this entry ran, for saying how much
    /// was held when it was stopped.
    observed: Cell<usize>,
}

impl Guard {
    /// A guard that has not tripped and admits nothing until an entry begins.
    pub(crate) fn new() -> Self {
        Self {
            shared: Rc::new(Shared {
                latch: Cell::new(Latch::Clear),
                used: Cell::new(0),
                budget: Cell::new(0),
                ceiling: Cell::new(0),
                observed: Cell::new(0),
            }),
        }
    }

    /// Opens a guarded entry with a whole fresh budget.
    ///
    /// Clearing the latch here is what makes the previous entry's abort a
    /// property of that entry rather than of the host.
    pub(crate) fn begin(&self, budget: u64, ceiling: usize) {
        self.shared.latch.set(Latch::Clear);
        self.shared.used.set(0);
        self.shared.budget.set(budget);
        self.shared.ceiling.set(ceiling);
        self.shared.observed.set(0);
    }

    /// The most script memory seen while the entry that just ran was running.
    pub(crate) fn observed(&self) -> usize {
        self.shared.observed.get()
    }

    /// How the entry that just ran ended.
    pub(crate) fn latch(&self) -> Latch {
        self.shared.latch.get()
    }

    /// Charges one interrupt tick, and reports whether script may continue.
    ///
    /// Called from inside the interrupt on a path measured at roughly the cost
    /// of the bare callback, so it does nothing but read and write cells. It
    /// must never call back into script: the backend's interrupt dispatch
    /// carries a recursion guard that silently continues on a re-entrant
    /// interrupt, which would make the trip below invisible.
    pub(crate) fn charge(&self, script_memory: usize) -> bool {
        if self.shared.latch.get() != Latch::Clear {
            return false;
        }
        self.shared.observed.set(script_memory);
        if script_memory > self.shared.ceiling.get() {
            self.shared.latch.set(Latch::Memory);
            return false;
        }
        let used = self.shared.used.get().saturating_add(1);
        self.shared.used.set(used);
        if used > self.shared.budget.get() {
            self.shared.latch.set(Latch::Budget);
            return false;
        }
        true
    }
}
