//! Writing a world to a save file, and asking a save what it needs.
//!
//! A save names blocks the only way a save may — by their namespaced name — and
//! carries **no registry-local identity**: no runtime id, no registration order,
//! and nothing about which registry instance resolved it. All three are
//! reassigned the moment the block set changes, so a save that stored one would
//! be rewritten by an update nobody asked to change it.
//!
//! It does record what each of those blocks was *declared* to be, which is
//! derived from definitions and from none of those three. A name resolving proves
//! *a* block exists under it, not that it is the block the world was built from —
//! a mod updated, forked, or replaced by another claiming the same name all load
//! silently against a name-only check.
//!
//! # Where the boundary is
//!
//! **The library decodes and we validate; the line is where the error is
//! raised.** The encoder's job is turning bytes into typed values and it is
//! treated as working: a widely-used decoder has had orders of magnitude more
//! adversarial attention than anything one feature can produce, and a test
//! asserting how it classifies a corrupt input would be a test of somebody else's
//! release notes. So `postcard` is nameable **only inside this module**, its
//! records are converted to this crate's own types at this module's edge, and
//! every one of its refusals collapses into [`LoadError::Malformed`].
//!
//! Everything after decoding is ours, and every one of those checks names the
//! value that was wrong: a name the registry does not hold, a component of a path
//! that is a file, more distinct names than a table can address.
//!
//! Every length, count and identifier read out of a save is attacker-controlled.
//! Nothing here is indexed without a bounds check, no reader ends the process,
//! and **no declared length drives an allocation ahead of the bytes behind it
//! arriving** — the decoder reads into a fixed scratch buffer and refuses a
//! length that will not fit. The encoder bounds bytes *read* rather than the
//! memory they expand into, so the file's own length is checked against
//! `MAX_SAVE_BYTES` before it is decoded at all; that check is what converts a
//! read bound into a memory bound, and it is ours rather than the library's.

mod error;
mod format;
mod read;
mod table;
mod write;

pub use error::{LoadError, SaveError};
pub use format::{DefinitionHash, SaveNameId, SavedPlayer};
pub use read::reader::{
    RequiredBlock, SaveRequirements, requirements, saved_player, stored_world_data,
};
pub use read::world::{LoadedWorld, load_world};
pub use table::{Acceptance, RegistryVerdict, resolve};
pub use write::{replace_atomically, save_world, write_save};
