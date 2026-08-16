//! The environment one chunk is evaluated against.
//!
//! Closing the backend's sandbox does not satisfy the requirement on its own,
//! and the reason is that it is doing something else: it makes the *base*
//! globals readonly and then hands the running thread a **writable child** table
//! to work in, which is exactly what a sandbox is for — protect the host's
//! globals, let the script have its own. Measured under it, `newname = 1` from a
//! content chunk is allowed. The requirement is that it is refused.
//!
//! So each chunk is loaded against a fresh table of its own, frozen, whose
//! metatable reads through to the sandboxed globals and whose `_G` is itself.
//!
//! # Three tables have to be frozen, not one
//!
//! Freezing the environment stops a chunk writing to its own globals. Freezing
//! its metatable stops `getmetatable(_G).__index = {}` repointing what the
//! environment reads through. Neither stops a chunk taking the table the
//! metatable points *at* — `getmetatable(_G).__index` hands it straight over —
//! and writing a name into that, which every chunk on the server reads through.
//! Measured: both `rawset` and plain assignment succeeded there, and a later
//! chunk read the planted name. The third freeze is applied where the backend's
//! sandbox is closed, once for the whole state.
//!
//! # A frozen table rather than an assignment hook
//!
//! A hook on assignment is bypassed by `rawset`, whose definition is that it
//! does not trigger one, and `rawset` is reachable from content. A frozen table
//! refuses the write where it lands instead of asking a metamethod the write can
//! decline to consult. Settled by measurement rather than by preference.
//!
//! # Per chunk rather than shared, and this is containment
//!
//! With one environment shared between chunks, a mod writing
//! `print = function() end` silences every other mod's `print` — measured at
//! zero host calls from a second chunk. Plain assignment and a chunk's own
//! environment are what an environment *is*, so the answer is not to forbid the
//! write but to give each chunk somewhere private to make it.

use mlua::{Lua, Table};

/// A fresh frozen environment reading through to the sandboxed globals.
///
/// The chunk sees `_G` as this table, so a chunk that reads a global gets the
/// shared library through `__index` and a chunk that writes one is refused.
pub(crate) fn frozen(lua: &Lua) -> mlua::Result<Table> {
    let environment = lua.create_table()?;
    let metatable = lua.create_table()?;
    metatable.set("__index", lua.globals())?;
    environment.set("_G", &environment)?;
    metatable.set_readonly(true);
    environment.set_metatable(Some(metatable))?;
    environment.set_readonly(true);
    Ok(environment)
}
