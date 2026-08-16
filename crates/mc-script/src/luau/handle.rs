//! Handles on values that live inside the script state.
//!
//! These are the two `ScriptValue` variants the host cannot copy out. They are
//! opaque on purpose: the engine may hold one, hand it back and ask the host to
//! do something with it, and may not reach through it. That is what keeps the
//! backend's own types off this crate's public surface, and it is what makes
//! discarding a state safe — nothing outside holds a live reference into it.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use mlua::{Function, Table};

use crate::ChunkName;

/// Mints a distinct tag for each script state ever created in this process.
static NEXT_UNIT: AtomicU64 = AtomicU64::new(1);

/// Which script state a handle came from.
///
/// Opaque, compared and never parsed. Exactly one value exists at a time today,
/// and that is the point rather than an argument against it.
///
/// **It is here for reload.** Reloading builds a candidate registry in a
/// *scratch* state and its whole job is substituting a scratch-state callback
/// for a live one. A handle that cannot say which state it came from makes that
/// substitution unverifiable, in the one path whose partial application this
/// crate calls a Blocker. The tag costs one field now and makes the day a second
/// state exists a change behind the adapter rather than at every consumer.
///
/// **It is not justified by the modding API's exemption from "no abstraction
/// before three concrete uses."** That exemption is scoped to the published
/// scripting surface content is written against, and does not reach an
/// engine-internal handle consumed by sibling crates. Recording that reason here
/// would be worse than recording none: whoever reads the standard correctly
/// would find the justification void and delete the field.
///
/// It is minted **per state**, so deriving it from a handle's own address or
/// from the chunk that produced it is wrong in a way nothing today can see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IsolationUnit(u64);

impl IsolationUnit {
    /// A tag for a newly created script state.
    pub(crate) fn mint() -> Self {
        Self(NEXT_UNIT.fetch_add(1, Ordering::Relaxed))
    }
}

/// A table the script produced, held for the host to read fields from without
/// running script.
#[derive(Clone)]
pub struct ScriptTable {
    handle: Table,
    unit: IsolationUnit,
    origin_chunk: ChunkName,
}

impl ScriptTable {
    pub(crate) fn new(handle: Table, unit: IsolationUnit, origin_chunk: ChunkName) -> Self {
        Self {
            handle,
            unit,
            origin_chunk,
        }
    }

    /// Which script state this came from.
    pub fn unit(&self) -> IsolationUnit {
        self.unit
    }

    /// The chunk this table came out of.
    ///
    /// Carried for the same reason a callback carries one: a function read out
    /// of this table has to be able to say where it was defined, or a fault from
    /// invoking it names no file. Stamped at the one moment it is known for
    /// free.
    pub(crate) fn origin_chunk(&self) -> &ChunkName {
        &self.origin_chunk
    }

    /// The backend table, for the adapter that reads it.
    pub(crate) fn handle(&self) -> &Table {
        &self.handle
    }
}

/// Identifies which value inside the script state this stands for, and nothing
/// about its contents.
///
/// Rendering the contents would mean reading the table, and reading a table
/// script supplied can run a metamethod — so a handle that described itself
/// would be a second, unguarded entry into script hiding inside a debug
/// formatter.
impl fmt::Debug for ScriptTable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "ScriptTable({:?} from `{}` in {:?})",
            self.handle.to_pointer(),
            self.origin_chunk.as_str(),
            self.unit
        )
    }
}

/// A function the script produced, which the host may invoke under a budget.
#[derive(Clone)]
pub struct ScriptFunction {
    handle: Function,
    unit: IsolationUnit,
    origin_chunk: ChunkName,
}

impl ScriptFunction {
    pub(crate) fn new(handle: Function, unit: IsolationUnit, origin_chunk: ChunkName) -> Self {
        Self {
            handle,
            unit,
            origin_chunk,
        }
    }

    /// Which script state this came from.
    pub fn unit(&self) -> IsolationUnit {
        self.unit
    }

    /// The chunk that defined this callback.
    ///
    /// Stamped when the handle is made, which is the only moment it is known for
    /// free. A fault raised by invoking this reaches its author through this
    /// name — not through the round it happened in, which says where the engine
    /// was rather than where the code is.
    pub(crate) fn origin_chunk(&self) -> &ChunkName {
        &self.origin_chunk
    }

    /// The backend function, for the adapter that invokes it.
    pub(crate) fn handle(&self) -> &Function {
        &self.handle
    }
}

/// Identifies which function inside the script state this stands for, for the
/// same reason [`ScriptTable`]'s does and with the same restraint.
impl fmt::Debug for ScriptFunction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "ScriptFunction({:?} from `{}` in {:?})",
            self.handle.to_pointer(),
            self.origin_chunk.as_str(),
            self.unit
        )
    }
}
