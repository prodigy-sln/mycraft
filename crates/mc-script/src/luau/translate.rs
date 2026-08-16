//! Turning what the backend says into what the host reports.
//!
//! Vendor error translation belongs at the adapter, and this is the whole of it.
//! Two things are pulled out of a backend message and neither is left in the
//! text: the **line**, which becomes a typed field, and the **traceback**, which
//! is dropped.
//!
//! The line is typed because the alternative is asserting on a substring of a
//! string a pre-1.0 dependency formats however it likes. Whoever reads a fault
//! needs to be sent to a line; a backend renaming its own prefix should not be
//! able to take that away silently.

use mlua::{Error, Value};

use crate::ChunkName;
use crate::luau::handle::{IsolationUnit, ScriptFunction, ScriptTable};
use crate::value::ScriptValue;

/// Where a traceback starts in a backend message.
const TRACEBACK: &str = "\nstack traceback:";

/// What an error says, taken from the variant that carries it rather than from
/// how the backend renders itself.
///
/// Rendering an error yields the message with a word for its kind in front —
/// `syntax error: `, `runtime error: ` — which is presentation, and reading the
/// location out of *that* means depending on two pieces of vendor formatting
/// where one will do. The kind is already known from the variant, so only the
/// message is taken.
pub(crate) fn message_of(error: &Error) -> String {
    match error {
        Error::SyntaxError { message, .. } | Error::RuntimeError(message) => message.clone(),
        Error::CallbackError { cause, .. } => message_of(cause),
        other => other.to_string(),
    }
}

/// A backend message with its location prefix and traceback removed, plus the
/// line the prefix named.
///
/// Messages arrive in two shapes and both are conformant. One raised at the
/// assignment site carries `[string "name"]:N: ` in front; one raised inside a
/// builtin carries nothing at all, because the error comes from the C function
/// rather than from the call site.
pub(crate) fn split_location(message: &str) -> (Option<u32>, String) {
    let body = message
        .split(TRACEBACK)
        .next()
        .unwrap_or(message)
        .trim_end();
    match parse_prefix(body) {
        Some((line, rest)) => (Some(line), rest.to_owned()),
        None => (None, body.to_owned()),
    }
}

/// The `[string "name"]:N: ` prefix, as the line it names and what follows it.
fn parse_prefix(body: &str) -> Option<(u32, &str)> {
    let after_source = body.strip_prefix("[string \"")?;
    let (_, after_quote) = after_source.split_once("\"]:")?;
    let (digits, rest) = after_quote.split_once(':')?;
    let line = digits.parse().ok()?;
    Some((line, rest.trim_start()))
}

/// What the host sees when script hands a value back.
///
/// A value the host has no vocabulary for becomes `Opaque` rather than being
/// forced into one it does not fit: the engine can carry it, hand it back, and
/// never has to pretend it knows what it is.
pub(crate) fn value(from: Value, unit: IsolationUnit, chunk: &ChunkName) -> ScriptValue {
    match from {
        Value::Nil => ScriptValue::Nil,
        Value::Boolean(flag) => ScriptValue::Boolean(flag),
        Value::Integer(number) => ScriptValue::Integer(number),
        Value::Number(number) => ScriptValue::Number(number),
        Value::String(text) => ScriptValue::Text(text.to_string_lossy()),
        Value::Table(handle) => ScriptValue::Table(ScriptTable::new(handle, unit, chunk.clone())),
        Value::Function(handle) => {
            ScriptValue::Function(ScriptFunction::new(handle, unit, chunk.clone()))
        }
        _ => ScriptValue::Opaque,
    }
}

/// How a value reads when the host has to render it without running script.
///
/// **Never `tostring`.** That honours `__tostring`, which is script, which means
/// the host would be running a mod's code on the host's own schedule at exactly
/// the moment it is reporting that mod's failure. A table renders as the fact
/// that it is a table; that is less informative than a mod author might like and
/// it is the only rendering that cannot be turned into a second entry into
/// script.
pub(crate) fn render(value: &Value) -> String {
    match value {
        Value::Nil => "nil".to_owned(),
        Value::Boolean(flag) => flag.to_string(),
        Value::Integer(number) => number.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.to_string_lossy(),
        Value::Table(_) => "a table raised by script".to_owned(),
        Value::Function(_) => "a function raised by script".to_owned(),
        _ => "a value raised by script".to_owned(),
    }
}

/// Renders the arguments of one `print` call, raw and space-separated.
///
/// Raw for the same reason a raised value is rendered raw: routing content's
/// output through `__tostring` would let a mod run code inside the host's own
/// logging path.
pub(crate) fn render_all(arguments: &[Value]) -> String {
    arguments.iter().map(render).collect::<Vec<_>>().join("\t")
}
