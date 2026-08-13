//! Why a save could not be written, and why one could not be read.
//!
//! Both enums are `Debug + Clone + PartialEq + Error`, and [`SaveError`] is
//! `Eq` besides — [`LoadError`] cannot be, for the reason recorded at the type
//! itself. That set is why an I/O failure is recorded as a path and an
//! [`ErrorKind`] rather than as a [`std::io::Error`], none of whose derives
//! exist. The translation happens where the failure is raised, so no foreign
//! error type crosses this module's edge.
//!
//! **The same translation is what collapses every decode failure into one
//! variant.** Which way a library declined a sequence of bytes is not part of
//! anything this crate promises: the encoder is treated as working, and a test
//! asserting how it classifies a corrupt input would be a test of somebody
//! else's release notes. Collapsing it here makes that true in the type system
//! rather than by convention — there is nowhere for a caller to reach a library
//! error, because the library error does not survive the boundary.
//!
//! What *is* ours is every check over an already-decoded value, and each one
//! names the thing that was wrong: a component, a name, a count.

use std::io::ErrorKind;
use std::path::PathBuf;

use mc_core::id::{BlockName, NamespacedIdError};
use thiserror::Error;

use crate::column::ColumnError;
use crate::section::{ImportError, SectionError};
use crate::world::{WorldError, WorldPos};

/// Why a world could not be saved.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SaveError {
    #[error("{path} is a directory, not a file a save can be written to", path = path.display())]
    PathIsDirectory { path: PathBuf },
    #[error("{component} is a file, not a directory", component = component.display())]
    NotADirectory { component: PathBuf },
    #[error(
        "the world holds `{name}`, which the registry it is being saved against does not declare",
        name = name.as_str()
    )]
    UnknownBlock { name: BlockName },
    /// More distinct names than a save's table can address.
    ///
    /// A format limit rather than a plausible one: a world reaches the memory a
    /// registry of this many blocks would need long before it reaches the
    /// identifier's width. It is refused by name because the alternative is an
    /// identifier that quietly aliases another block's.
    #[error("a save's table holds at most {supported} names, and this world needs {found}")]
    TooManyNames { found: usize, supported: usize },
    #[error("{path} could not be written: {kind:?}", path = path.display())]
    Io { path: PathBuf, kind: ErrorKind },
    /// A section could not describe what it holds.
    ///
    /// An internal invariant and nothing a caller did, carried rather than
    /// swallowed for the reason [`crate::world::WorldError`] carries the same
    /// thing: a save that quietly wrote a section it could not read would be the
    /// one failure this whole path exists to prevent.
    #[error(transparent)]
    Section(#[from] SectionError),
}

/// Why a save could not be read.
///
/// **`Eq` is the one derive this enum cannot carry**, and the exception is
/// forced rather than chosen: [`NotFinite`](Self::NotFinite) has to name the
/// value it refused, that value is an `f32`, and `f32` is not `Eq` because a NaN
/// is not equal to itself. Carrying the value is what the requirement asks for,
/// so `PartialEq` is what this enum can honestly promise. Every other error in
/// this crate keeps all four.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum LoadError {
    /// Nothing is at the path.
    ///
    /// **Distinct from [`Unreadable`](Self::Unreadable), and the distinction is
    /// load-bearing.** A launch decides whether to generate a world by branching
    /// on exactly this: a collapsed pair would make a missing save generate a new
    /// world over one that merely could not be opened.
    #[error("{path} does not exist", path = path.display())]
    Missing { path: PathBuf },
    #[error("{path} could not be read: {kind:?}", path = path.display())]
    Unreadable { path: PathBuf, kind: ErrorKind },
    /// The file does not begin the way a save does.
    ///
    /// Carries the leading bytes that were there, and names the ones a save
    /// begins with — a player pointed at the wrong file learns nothing from
    /// being told it could not be read, and a great deal from being shown what
    /// was found beside what was expected. A file of no bytes at all reports the
    /// same refusal carrying nothing, because something is there and generating
    /// a world over it would write across whatever is left.
    #[error("{found:?} is not how a save begins — a save begins `MYCRAFT\\x1A`")]
    NotASave { found: Vec<u8> },
    /// The save declares a version of this format that this build cannot read.
    ///
    /// Both numbers, because only both together say what to do: the one found
    /// says which build wrote the file and the one supported says which build
    /// can read it.
    #[error("this save declares format version {found}, and this build reads version {supported}")]
    UnsupportedVersion { found: u16, supported: u16 },
    /// The save's bytes are not a save this build can make sense of.
    ///
    /// One variant and only a path, deliberately: the decoder is a library this
    /// crate treats as working, so *which* way it refused is not part of any
    /// contract here and there is nowhere for a caller to reach it.
    #[error("{path} is not a save this build can read", path = path.display())]
    Malformed { path: PathBuf },
    /// The save's table holds text where a block name belongs.
    ///
    /// Quoting the text back is the whole of the diagnosis — it is the only
    /// thing that says *which* entry — and the reason beside it says what is
    /// wrong with it.
    #[error("`{text}` is not a block name a save may hold")]
    MalformedName {
        text: String,
        #[source]
        source: NamespacedIdError,
    },
    /// The save's table names the same block twice.
    ///
    /// Two answers to one question, with every cell in the save pointing at one
    /// of them by number: whichever entry a reader kept, half the world would be
    /// read against a declaration the file itself disagrees with.
    #[error("this save's table names `{name}` twice", name = name.as_str())]
    DuplicateName { name: BlockName },
    /// A stored coordinate or angle is not a finite number.
    ///
    /// Refused at the boundary because it is the last place it is still one
    /// value with a name: carried into a simulation it reaches a velocity, a
    /// collision box and a camera before anything looks wrong on screen.
    #[error("this save records {value} for the player's {axis}, which is not a finite number")]
    NotFinite { axis: &'static str, value: f32 },
    /// The registry cannot answer for the blocks this save names.
    ///
    /// **Both lists in one refusal, and that is the point.** Missing and changed
    /// are refused together, in one pass, in one report — the complete statement
    /// of what is wrong rather than the first thing that was. They stay separate
    /// fields because the outcomes differ: a missing name is never loadable, a
    /// changed one is loadable with the player's acceptance, and a player who
    /// supplied acceptance and was still refused has to be able to see that it
    /// was the missing half that turned them away.
    ///
    /// Neither list is empty when this is produced, and never both.
    #[error(
        "this save needs blocks this registry cannot answer for — missing: {missing}; changed: {changed}",
        missing = named(missing),
        changed = named(changed)
    )]
    Unresolvable {
        missing: Vec<BlockName>,
        changed: Vec<BlockName>,
    },
    /// A palette entry names a table position the table does not hold.
    ///
    /// Names how many entries the table does hold beside the identifier that
    /// was not one of them, because the two together say whether the file was
    /// cut short or written against a table that is not the one it carries.
    #[error("this save's table holds {table_len} names, and a stored block names entry {id}")]
    UnknownNameId { id: u32, table_len: usize },
    /// A voxel names a palette position its section's palette does not have.
    ///
    /// **Names the world position**, which is the whole reason this refusal is
    /// raised here rather than left to the section importer: the importer knows
    /// the index and the palette's length but not which of four thousand voxels
    /// carried it, and a position is the only part of that a player can look at.
    #[error(
        "the cell at ({x}, {y}, {z}) names palette position {index}, and its section's palette holds {palette_len}",
        x = at.x,
        y = at.y,
        z = at.z
    )]
    UnknownCellEntry {
        at: WorldPos,
        index: u16,
        palette_len: usize,
    },
    /// The save's column list does not fill the footprint it declares.
    ///
    /// Both counts, because either one alone leaves a reader guessing which of
    /// the two the file got wrong.
    #[error("a footprint of {expected} columns cannot be filled by {found}")]
    WrongColumnCount { expected: usize, found: usize },
    /// The file carries bytes after the world record.
    ///
    /// Named by the offset the save should have ended at, because that is the
    /// one thing that says *where* the file stopped being a save.
    #[error("this save should have ended at byte {should_have_ended_at}, and does not")]
    TrailingBytes { should_have_ended_at: u64 },
    /// A section a save stored cannot be built back.
    #[error(transparent)]
    Section(#[from] ImportError),
    /// A column a save stored cannot be stacked back.
    #[error(transparent)]
    Column(#[from] ColumnError),
    /// The world a save stored cannot be assembled back.
    #[error(transparent)]
    World(#[from] WorldError),
}

/// `names` as a reader sees them: comma-separated, or the word for none.
fn named(names: &[BlockName]) -> String {
    if names.is_empty() {
        return "none".to_owned();
    }
    names
        .iter()
        .map(|name| format!("`{}`", name.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}
