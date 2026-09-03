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

use std::collections::BTreeSet;
use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};

use mc_core::id::{BlockName, TextureKey};
use mc_sim::world::Clearing;

#[cfg(test)]
#[path = "notice_test.rs"]
mod tests;

/// Where this client's non-fatal notices are written.
///
/// **The caller supplies it and every notice goes through it**, which is the
/// whole of what the nine `eprintln!` sites this replaces could not offer: a
/// harness could not read them, a caller could not route them elsewhere, and
/// nothing could silence them. Silencing needs no feature of its own — a sink
/// that discards is one a caller already knows how to build, and
/// [`discarding`](Self::discarding) is that spelling.
///
/// # Why it is shared rather than borrowed
///
/// The reported *ending* takes a borrowed `&mut dyn Write`, and this was ruled to
/// be the same sink rather than a second one. It cannot be the same *borrow*: two
/// of the notices are written from the preparation worker, `std::thread::spawn`
/// requires `Send + 'static`, and [`spawn_preparation`](crate::launch::spawn_preparation)
/// returns the handle so a scoped thread is not available either. Moving those
/// two notices below the worker would put them behind a device and take both
/// subprocess readings of them with it, which is trading evidence for tidiness.
/// So the handle is shared and the `dyn Write` is unchanged.
#[derive(Clone)]
pub struct Notices {
    sink: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl std::fmt::Debug for Notices {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Notices").finish_non_exhaustive()
    }
}

impl Notices {
    /// Notices written to `sink`.
    #[must_use]
    pub fn writing_to(sink: Box<dyn Write + Send>) -> Self {
        Self {
            sink: Arc::new(Mutex::new(sink)),
        }
    }

    /// Notices nobody reads.
    ///
    /// What silencing this client costs: one call. A harness that wants the run
    /// and not the commentary supplies this and nothing else changes.
    #[must_use]
    pub fn discarding() -> Self {
        Self::writing_to(Box::new(std::io::sink()))
    }

    /// Writes `line` and a line break.
    ///
    /// **A poisoned lock is recovered rather than propagated.** The mutex guards
    /// a byte sink, which has no invariant a panic can corrupt, so treating
    /// poisoning as fatal would let one unrelated panic silently disable every
    /// later notice — this module's own defect, reintroduced by its fix.
    pub fn say(&self, line: &str) {
        let mut sink = self.sink.lock().unwrap_or_else(PoisonError::into_inner);
        // The one place in this crate where a failure is swallowed, and the only
        // place it is right: there is nowhere to report a failure to report, and
        // a notice must never be more fatal than the thing it describes.
        let _written = writeln!(sink, "{line}");
    }

    /// Hands the sink to `write`, for the one caller that needs the stream
    /// itself.
    ///
    /// [`mc_render::window::report`] takes a `&mut dyn Write`, and the ending it
    /// says goes to the same place the notices do. This is what makes that one
    /// place rather than two.
    pub fn with<T>(&self, write: impl FnOnce(&mut dyn Write) -> T) -> T {
        let mut sink = self.sink.lock().unwrap_or_else(PoisonError::into_inner);
        write(&mut **sink)
    }
}

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

/// What the line says about one uncovered key, and about more than one.
///
/// Two clauses on [`changed_blocks`]'s reasoning: a mod author's first block is
/// one key and the plural is wrong about it, and a root missing a dozen is what
/// the line exists for.
const DRAWS_A_STAND_IN: &str = "draws a generated stand-in because nothing has baked it";
const DRAW_STAND_INS: &str = "draw generated stand-ins because nothing has baked them";

/// How the line ends: the launch is not refused for this, and that is the half a
/// mod author acts on.
const NOT_A_FAILURE: &str = "and that is not a failure";

/// Which of the texture keys the content declares the built set left uncovered,
/// or `None` where it covers all of them.
///
/// **The comparison is here rather than at the call site**, which is the whole of
/// what the sentence this replaces was missing: it was a constant printed before
/// the content root had been read, so it named no key and read identically
/// whether every declared key was covered or none was.
///
/// Every uncovered key, ascending, complete and never truncated —
/// [`changed_blocks`]'s rule, on reasoning that applies here word for word. A mod
/// author has to go and bake each missing key, so each has to be named.
///
/// A key the set covers that nothing declares is not named: it draws nothing at
/// all, so there is no block for an author to go looking for.
#[must_use]
pub fn stand_ins(
    declared: &BTreeSet<TextureKey>,
    covered: &BTreeSet<TextureKey>,
) -> Option<String> {
    let uncovered: Vec<&TextureKey> = declared.difference(covered).collect();
    match uncovered.as_slice() {
        [] => None,
        [_] => Some(format!(
            "mycraft: {keys} {DRAWS_A_STAND_IN}, {NOT_A_FAILURE}",
            keys = quoted(&uncovered)
        )),
        _ => Some(format!(
            "mycraft: {keys} {DRAW_STAND_INS}, {NOT_A_FAILURE}",
            keys = quoted(&uncovered)
        )),
    }
}

/// The `keys`, each quoted, in the order they are held.
///
/// [`named`]'s convention, for the same reason: an author who meets both lists
/// reads one.
fn quoted(keys: &[&TextureKey]) -> String {
    keys.iter()
        .map(|key| format!("`{}`", key.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Writes what entry did about where the player stands.
///
/// Through the caller's sink rather than straight to the error stream, which is
/// the convention every one of this crate's non-fatal notices now follows.
pub fn say_entering(clearing: Clearing, notices: &Notices) {
    if let Some(said) = entering(clearing) {
        notices.say(&said);
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
pub fn say_changed_blocks(changed: &[BlockName], notices: &Notices) {
    if let Some(said) = changed_blocks(changed) {
        notices.say(&said);
    }
}

/// Writes which of the keys content declared the built set left uncovered.
///
/// Called on the preparation worker and **before a device is opened**, for
/// [`say_changed_blocks`]'s reason: which keys have art is settled the moment the
/// content and the set beside it have been read, and saying it below the uploads
/// would put it behind a display server.
pub fn say_stand_ins(
    declared: &BTreeSet<TextureKey>,
    covered: &BTreeSet<TextureKey>,
    notices: &Notices,
) {
    if let Some(said) = stand_ins(declared, covered) {
        notices.say(&said);
    }
}

/// Writes what a content reload did about where the player stands.
pub fn say_reloading(clearing: Clearing, notices: &Notices) {
    if let Some(said) = reloading(clearing) {
        notices.say(&said);
    }
}

/// A sink a test can read back, shared with the [`Notices`] holding it.
#[cfg(test)]
pub(crate) mod recording {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex, PoisonError};

    use super::Notices;

    /// Everything written to it, readable while the writer still holds it.
    #[derive(Clone, Default)]
    pub(crate) struct Recorder {
        written: Arc<Mutex<Vec<u8>>>,
    }

    impl Recorder {
        /// A recorder, and the notices writing into it.
        pub(crate) fn listening() -> (Self, Notices) {
            let recorder = Self::default();
            let notices = Notices::writing_to(Box::new(recorder.clone()));
            (recorder, notices)
        }

        /// Everything written so far, as the text a person would read.
        pub(crate) fn said(&self) -> String {
            let written = self.written.lock().unwrap_or_else(PoisonError::into_inner);
            String::from_utf8_lossy(&written).into_owned()
        }
    }

    impl Write for Recorder {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let mut written = self.written.lock().unwrap_or_else(PoisonError::into_inner);
            written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
