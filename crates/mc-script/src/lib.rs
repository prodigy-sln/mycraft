//! Luau scripting host: sandboxed VMs, engine bindings, the definition registry, and hot reload.
//!
//! # The vocabulary this crate is built on
//!
//! A **subject** is an opaque namespaced identity the host stores and never
//! interprets — a block name, eventually. A **component** is an opaque
//! namespaced identity under which a callback is registered against a subject.
//! The `(subject, component)` pair is an [`Attachment`], and it is the unit of
//! budget, of fault counting and of quarantine: one mod's broken attachment on
//! a block stops acting while another mod's behaviour on the same block keeps
//! working.
//!
//! Every identity here is a newtype over what it wraps, **stored and compared
//! and never parsed**. The host is forbidden from reading structure into a
//! namespace — deriving "which mod is this" from a component name is exactly
//! the interpretation these types exist to refuse.
//!
//! # Nothing in this crate may panic
//!
//! A bad mod never takes down the server, and a panic in the host is a mod
//! taking down the server through the host. Every failure a script can cause is
//! a [`ScriptFault`] — a value returned to the caller, never an unwind. The
//! crate-root denial below is what makes that a build error rather than a rule
//! somebody remembers: `unwrap`, `expect` and `panic!` are hard errors here at
//! plain `cargo check`, not only under a lint-gated build.
//!
//! It covers this library and its sibling unit-test modules. Each integration
//! test under `tests/` is its own crate root and is not reached by it; those are
//! covered by the workspace lint table promoted to errors at the quality gate.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod dispatch;
mod fault;
mod host;
mod limits;
mod luau;
mod quarantine;
mod value;

pub use dispatch::DispatchReport;
pub use fault::{FaultKind, ScriptFault, ScriptOrigin};
pub use host::{HostError, ScriptHost};
pub use limits::{HostLimits, PROVISIONAL_ROUND_BUDGET_CEILING};
pub use luau::handle::{IsolationUnit, ScriptFunction, ScriptTable};
pub use value::ScriptValue;

/// The identity of something a callback can be attached to.
///
/// Opaque and namespaced. The host stores it, compares it and hands it back in
/// faults; it never splits it, resolves it or reads a mod out of it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectName(String);

impl SubjectName {
    /// The subject spelled `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The spelling, for rendering it back to whoever wrote it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The identity of a behaviour attached to a subject.
///
/// Opaque and namespaced, on the same terms as [`SubjectName`]. Two mods
/// declaring a component of the same name against the same subject are the
/// same attachment, which is why the pair is the unit of isolation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentName(String);

impl ComponentName {
    /// The component spelled `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The spelling, for rendering it back to whoever wrote it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A component attached to a subject: the unit of budget, fault counting and
/// quarantine.
///
/// Quarantine acts on the pair and never on the component alone. A component
/// broken on one subject keeps being invoked on every other subject it is
/// attached to, and every other component on the broken subject keeps being
/// invoked — strictly finer isolation than disabling either half.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Attachment {
    /// What the behaviour is attached to.
    pub subject: SubjectName,
    /// The behaviour attached to it.
    pub component: ComponentName,
}

/// What a chunk of script was called when it was evaluated.
///
/// It is a label rather than a path: the host is handed source and a name to
/// report it under, and never opens a file. Whatever a mod author will
/// recognise is the right value, because this is what a fault names when it
/// tells them where to look.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkName(String);

impl ChunkName {
    /// The chunk labelled `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The label, for rendering it back to whoever wrote it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Which dispatch round something happened in.
///
/// Rounds are bounded by a configured invocation count, so this counts rounds
/// and not invocations and stays small.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoundIndex(u32);

impl RoundIndex {
    /// The round numbered `index`.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// The number, for rendering and for comparing two rounds.
    pub fn get(self) -> u32 {
        self.0
    }
}
