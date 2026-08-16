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
