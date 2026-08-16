//! Invoking a callback so that reporting its failure is not a second entry into
//! script.
//!
//! # Why the host does not simply call the callback
//!
//! Script can raise any value, including a table carrying a `__tostring`
//! metamethod — which is script. **Measured, the backend installs a message
//! handler for every protected call it makes, and that handler renders the
//! raised value**, so a host that calls a callback directly has already run the
//! mod's metamethod before it ever sees the error: unbudgeted, re-entrant, and
//! at a moment the mod chose.
//!
//! So the host holds one script-side function, `function(f, ...) return
//! pcall(f, ...) end`, **created before the sandbox is closed**, and invokes
//! every callback through it. A raised value then comes back as an ordinary
//! return value rather than as a propagating error, no message handler is
//! reached, and the host renders it with a formatter that matches on the value
//! and never consults a metamethod.
//!
//! # Two outcomes told apart by shape, and a third told apart by identity
//!
//! The trampoline **returns** `(false, value)` — the script raised, and the
//! value is rendered raw. The trampoline **cannot return** — an abort unwound
//! past it, which the guard already knows the reason for. The two are
//! distinguished structurally rather than by reading any message.
//!
//! The third is the guard being clear when the call still failed: no limit
//! tripped, so the refusal came from the backend on its own account. That one is
//! classified below.

use mlua::{Error, Lua, MultiValue, Value};

use crate::fault::FaultKind;

/// The source of the function every callback is invoked through.
///
/// It must be created before the sandbox is closed, which is why it lives here
/// as source rather than being built on demand.
pub(crate) const TRAMPOLINE_SOURCE: &str = "return function(callback, ...)\n\
                                            \treturn pcall(callback, ...)\n\
                                            end\n";

/// What the trampoline handed back.
pub(crate) enum Returned {
    /// The callback returned this.
    Value(Value),
    /// The callback raised this, already rendered without running script.
    Raised(String),
}

/// Reads the trampoline's own return values.
///
/// The first is whether the callback succeeded and the second is either its
/// result or what it raised. A trampoline that returned nothing at all is read
/// as a callback that returned nothing, which is what it would mean.
pub(crate) fn returned(mut values: MultiValue) -> Returned {
    let succeeded = matches!(values.pop_front(), Some(Value::Boolean(true)));
    let payload = values.pop_front().unwrap_or(Value::Nil);
    if succeeded {
        Returned::Value(payload)
    } else {
        Returned::Raised(super::translate::render(&payload))
    }
}

/// Builds the trampoline in `lua`. Call before closing the sandbox.
pub(crate) fn build(lua: &Lua) -> mlua::Result<mlua::Function> {
    lua.load(TRAMPOLINE_SOURCE)
        .set_name("host-trampoline")
        .eval()
}

/// What a refusal means when no limit of the host's tripped.
///
/// Decided from the error's **identity**, never from its text, and the two arms
/// are the whole rule rather than a case somebody handled and a case nobody did:
/// an allocation refusal is an allocation refusal, and everything the backend
/// can otherwise refuse is the script's own failure.
///
/// Reading the message instead fails in both directions at once. An allocation
/// refusal was measured to arrive carrying **no message at all**, so text
/// matching cannot see the case it most needs to; and a mod that raises an error
/// mentioning memory would have its own failure filed under the host's
/// condition, which quarantine and the memory-pressure rule each treat
/// differently.
pub(crate) fn classify_backend_error(error: &Error) -> FaultKind {
    match error {
        Error::MemoryError(_) => FaultKind::Allocation,
        _ => FaultKind::ScriptError,
    }
}

#[cfg(test)]
#[path = "trampoline_test.rs"]
mod tests;
