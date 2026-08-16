//! The scripting backend, and the only place in this crate that may name it.
//!
//! # The port
//!
//! Everything outside this directory speaks in [`ScriptHost`](crate::ScriptHost),
//! [`ScriptValue`](crate::ScriptValue), [`ScriptTable`](crate::ScriptTable),
//! [`ScriptFunction`](crate::ScriptFunction),
//! [`ScriptFault`](crate::ScriptFault) and
//! [`HostLimits`](crate::HostLimits). The backend's own `Lua`, `Function`,
//! `Table`, `Value` and `Error` stay behind this module and appear on no public
//! signature.
//!
//! That is not isolation for its own sake. The backend is pre-1.0 and breaking
//! minor releases are routine, and a second backend is deferred rather than
//! rejected — so a vendor type on a public signature is a migration this crate
//! could not perform without breaking every consumer of it. It costs
//! approximately nothing, because it is the crate's public API declining to leak
//! rather than a layer added on top of one.
//!
//! The port is shaped around what the host needs — *evaluate this chunk under a
//! budget; invoke this attachment; tell me how it ended* — and deliberately not
//! around the backend's surface, which is the shape that would have to be
//! reproduced by whatever replaced it.
//!
//! **The rule is mechanised rather than asserted**: a text guard walks this
//! crate's `src` and `tests` roots and reports an enumerated verdict naming
//! every file outside this directory that mentions the backend, with a positive
//! control over a tree that does leak. An unenforced litmus is a claim.
//!
//! # What is measured here, and what to re-measure on an upgrade
//!
//! Five backend behaviours this design rests on are not covered by any stability
//! promise: which globals closing the sandbox leaves standing, the child
//! environment it hands the running thread, the interrupt's error propagation,
//! how allocation failure is reported, and the `[string "name"]:N:` prefix a
//! message carries. All five are observed only inside this directory, which is
//! what makes an upgrade a re-measurement rather than a rewrite.

pub(crate) mod env;
pub(crate) mod guard;
pub(crate) mod handle;
pub(crate) mod trampoline;
pub(crate) mod translate;
pub(crate) mod vm;
