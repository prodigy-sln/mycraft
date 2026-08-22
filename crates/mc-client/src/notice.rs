//! The words a clearing verdict becomes, and writing them for the player.
//!
//! Composing is separated from writing because **the composition must be
//! reachable with no device**. The reload's two sentences lived inside
//! `App::report_clearing`, behind a `wgpu::Surface` and a `winit::Window` that
//! nothing in this workspace constructs, so the exact words a moved player reads
//! were asserted by nothing at all. [`entering`](crate::notice::entering) and
//! [`reloading`](crate::notice::reloading) are total
//! functions of a `Copy` verdict — no window, no session, no `&self` — and the
//! `say_*` pair is the whole of what needs a running client.
//!
//! **The two moments keep their own wording.** A player moved by a reload watched
//! their cell become solid; one moved at entry did not witness anything, so the
//! entry sentence names the state found rather than an event. Unifying them would
//! tell one of the two about something that did not happen to them.

use mc_core::id::BlockName;
use mc_sim::world::Clearing;

#[cfg(test)]
#[path = "notice_test.rs"]
mod tests;

/// What both entry sentences open with.
///
/// **Declared as a const rather than written inline**, the idiom
/// [`CONTENT_NOT_TAKEN_UP`](crate::session::reload::CONTENT_NOT_TAKEN_UP)
/// already sets. It keeps the clause on a line of its own: a literal long enough
/// to wrap inside a `format!` lands across a `\` continuation, and
/// `crates/mc-client/tests/the_entry_sentence_is_said_once.rs` reads the source
/// as text to check that one place composes this and one place says it.
const ENTERED_INSIDE_SOLID_BLOCKS: &str = "you would have entered the world inside solid blocks";

/// How an entry refusal ends: the launch proceeds and the player is still in rock.
const LEFT_INSIDE_THEM: &str = "so you were left inside them";

/// What both reload sentences open with, character for character with what
/// `App::report_clearing` printed before this module existed.
const THE_RELOAD_MADE_YOUR_CELL_SOLID: &str = "the reload made your cell solid";

/// How a reload refusal ends.
const LEFT_WHERE_YOU_WERE: &str = "so you were left where you were";

/// What entry did about where the player stands, or `None` when it did nothing.
///
/// Every ordinary launch answers [`Clearing::Unneeded`], so composing anything
/// for it would put a line on every player's terminal on every run.
#[must_use]
pub fn entering(clearing: Clearing) -> Option<String> {
    match clearing {
        Clearing::Unneeded => None,
        Clearing::MovedTo(feet) => Some(format!(
            "mycraft: {ENTERED_INSIDE_SOLID_BLOCKS}, so you were moved to ({x}, {y}, {z})",
            x = feet.x,
            y = feet.y,
            z = feet.z
        )),
        Clearing::NoClearSpaceWithin { blocks } => Some(format!(
            "mycraft: {ENTERED_INSIDE_SOLID_BLOCKS} and nothing within {blocks} blocks is clear, \
             {LEFT_INSIDE_THEM}"
        )),
    }
}

/// What a content reload did about where the player stands, or `None` when it
/// did nothing.
///
/// The reach is the verdict's own field, never a literal: the search's `REACH` is
/// free to change and this sentence must move with it.
#[must_use]
pub fn reloading(clearing: Clearing) -> Option<String> {
    match clearing {
        Clearing::Unneeded => None,
        Clearing::MovedTo(feet) => Some(format!(
            "mycraft: {THE_RELOAD_MADE_YOUR_CELL_SOLID}, so you were moved to ({x}, {y}, {z})",
            x = feet.x,
            y = feet.y,
            z = feet.z
        )),
        Clearing::NoClearSpaceWithin { blocks } => Some(format!(
            "mycraft: {THE_RELOAD_MADE_YOUR_CELL_SOLID} and nothing within {blocks} blocks is \
             clear, {LEFT_WHERE_YOU_WERE}"
        )),
    }
}

/// What the line says about one block, and about more than one.
///
/// Two clauses rather than one that reads correctly for neither count. A player
/// with a single changed block is the common case and "these blocks no longer
/// behave" is wrong about their world; a player with nine is the case the line
/// exists for and the singular is wrong about theirs.
const NO_LONGER_BEHAVES: &str = "no longer behaves as it did when this world was saved";
const NO_LONGER_BEHAVE: &str = "no longer behave as they did when this world was saved";

/// How the line ends: the world is open regardless, and that is the half a
/// player acts on.
const LOADED_ANYWAY: &str = "and it was loaded anyway";

/// Which blocks a loaded save no longer agrees with the content about, or `None`
/// where it agrees about all of them.
///
/// **Every name, ascending, complete, and never truncated.** The refusal this
/// replaces printed both of its lists whole, so a line that named the first
/// three of nine would report *less* than the refusal did. A player whose mods
/// have changed acts on this by reading it, and a bounded list is one they cannot
/// act on.
///
/// `None` for an empty list, which is `entering`'s own rule: composing something
/// for the ordinary case would put a line on every player's terminal on every
/// run. **Behaviour only** — a retexture never reaches here, because a line after
/// every art edit is noise the one that matters would hide in.
#[must_use]
pub fn changed_blocks(changed: &[BlockName]) -> Option<String> {
    match changed {
        [] => None,
        [_] => Some(format!(
            "mycraft: {names} {NO_LONGER_BEHAVES}, {LOADED_ANYWAY}",
            names = named(changed)
        )),
        _ => Some(format!(
            "mycraft: {names} {NO_LONGER_BEHAVE}, {LOADED_ANYWAY}",
            names = named(changed)
        )),
    }
}

/// The names of `blocks`, each quoted, in the order they are held.
///
/// Quoted and comma-joined to match `LoadError::Unresolvable`'s own rendering of
/// the same kind of list: a player who meets both reads one convention.
fn named(blocks: &[BlockName]) -> String {
    blocks
        .iter()
        .map(|block| format!("`{}`", block.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Writes what entry did about where the player stands.
///
/// Straight to the error stream rather than through a reporting sink, which is
/// the convention this crate's other non-fatal notices already follow.
pub fn say_entering(clearing: Clearing) {
    if let Some(said) = entering(clearing) {
        eprintln!("{said}");
    }
}

/// Writes which blocks a loaded save no longer agrees with the content about.
///
/// Called where the load completes and **before a device is opened**, which is
/// the one thing about this notice that is not a copy of its siblings. The
/// clearing notices wait for a picture because "you were moved" needs a world to
/// have been moved in; "these blocks changed" is already true when the save has
/// been read, and saying it below the uploads would put the only scenario that
/// can see a client which composes the line and never prints it out of reach of
/// anything that runs without a display server.
pub fn say_changed_blocks(changed: &[BlockName]) {
    if let Some(said) = changed_blocks(changed) {
        eprintln!("{said}");
    }
}

/// Writes what a content reload did about where the player stands.
pub fn say_reloading(clearing: Clearing) {
    if let Some(said) = reloading(clearing) {
        eprintln!("{said}");
    }
}
