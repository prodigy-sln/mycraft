//! What script hands back, in terms the engine can hold.

use crate::luau::handle::{ScriptFunction, ScriptTable};

/// A value produced by script.
///
/// Scalars are copied out; a table and a function are handles onto values that
/// stay inside the script state, because copying either would mean either
/// running script or reaching into it.
///
/// `Opaque` is deliberate rather than a gap. A value this crate has no
/// vocabulary for can still be carried and handed back, and forcing it into a
/// variant it does not fit would be the host claiming to know something it does
/// not.
#[derive(Debug, Clone)]
pub enum ScriptValue {
    /// No value.
    Nil,
    /// A boolean.
    Boolean(bool),
    /// A whole number.
    Integer(i64),
    /// A number that is not whole.
    Number(f64),
    /// Text, copied out of the script state.
    Text(String),
    /// A handle on a table the script produced.
    Table(ScriptTable),
    /// A handle on a function the script produced.
    Function(ScriptFunction),
    /// Something the host holds without interpreting.
    Opaque,
}

/// The keys a script table holds **in its own right**.
///
/// The answer to "what fields does this declaration carry?", which is a
/// different question from "what does this field hold" and meets a different
/// metamethod. A host that can read a named field but cannot ask which fields
/// exist can never tell a typo from an absence, so a misspelled key becomes a
/// silently lost declaration.
///
/// The names are **sorted lexicographically**, here rather than at the caller.
/// Lua leaves the key order of a table's hash part unspecified, so the state's
/// own order carries no information at all — returning it would be handing back
/// noise for a caller to render into a refusal nobody can quote. No caller can
/// want an order that does not exist, and a caller that forgot to sort produces
/// text that changes between runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldNames {
    /// Every key the table holds, sorted, no more than the caller allowed.
    Enumerated(Vec<String>),
    /// The table holds more keys than the caller allowed, and none of them are
    /// carried here.
    ///
    /// Reported rather than truncated: a list silently cut to the allowance
    /// describes a table of a hundred thousand keys as though it held the
    /// handful that fit, and the allocation the bound exists to refuse has
    /// already happened by the time anyone could notice.
    MoreThanAllowed {
        /// The bound the caller passed, which the table exceeded.
        allowed: usize,
    },
}
