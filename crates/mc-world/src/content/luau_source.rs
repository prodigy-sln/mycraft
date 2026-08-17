//! A directory of Luau chunks, evaluated as block definitions.
//!
//! The second implementation of the definition-source port, and the one the
//! game runs on: a declaration is **code that runs**, not a document that is
//! parsed, so a block may compute what it declares. Nothing above this file
//! learns that a definition was ever a chunk.
//!
//! # Evaluating a declaration is an entry into script, and is guarded like one
//!
//! A declaration file is the first production path that runs mod-authored code,
//! so every declaration goes through [`ScriptHost::evaluate`] under the limits
//! the engine ships — the call-and-loop budget, the per-entry memory cap, the
//! sandbox and the frozen environment. None of that machinery is written here,
//! and that is the point: a loader that read a file and ran it round the side of
//! the host would satisfy every scenario about fields and would hang the server
//! on the first declaration that looped.
//!
//! Fields are read **raw**, by the same [`ScriptHost::read_field`] the host uses
//! everywhere else. A declaration's own metatable therefore never runs on the
//! host's schedule, never observes which fields were looked at, and cannot
//! supply a field the declaration did not state in its own right.
//!
//! # One host per read, and no handle outlives its file
//!
//! [`DefinitionSource::definitions`] takes `&self` and [`ScriptHost::evaluate`]
//! takes `&mut self`, so a source cannot simply hold a host. Holding one behind
//! interior mutability would make two overlapping streams a re-entrant borrow,
//! which panics — and a panic on a path content reaches is exactly what the
//! scripting host's own invariants forbid. So a host is built inside the call,
//! used for every file, and dropped before the call returns. The failure is
//! unexpressible rather than avoided, and the source itself holds nothing but a
//! path and a record of what content printed.

use std::cell::RefCell;
use std::fmt;
use std::fs;
use std::io;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};

use mc_core::block::source::{
    DefinitionFault, DefinitionSource, DefinitionSourceError, DefinitionStream,
};
use mc_core::block::{BlockDefinition, DefinitionOrigin};
use mc_script::{ScriptFault, ScriptHost, ScriptValue};

use super::luau_declaration::checked_declaration;

/// The subdirectory of a content root that block declarations live in.
pub(super) const BLOCKS_DIRECTORY: &str = "blocks";

/// The extension that makes a file under [`BLOCKS_DIRECTORY`] a declaration.
pub(super) const DECLARATION_EXTENSION: &str = "luau";

/// How many declarations one content root may hold.
///
/// A directory listing is an allocation whose size a mod author chooses, and
/// nothing else bounds it: the format this loader replaces borrowed its limits
/// from a parser and a filesystem, and this one has none until they are said out
/// loud. Far above any real content root and far below anything that costs the
/// process.
const DECLARATIONS_A_ROOT_MAY_HOLD: usize = 4_096;

/// How many bytes one declaration file may hold.
///
/// A declaration is read into memory in full before a line of it is evaluated,
/// so this is the other content-supplied quantity the loader would otherwise
/// take on trust.
const BYTES_A_DECLARATION_MAY_HOLD: usize = 256 * 1024;

/// What a declaration that did not return a table is told.
const NOT_A_DECLARATION: &str =
    "a declaration chunk must return a table stating the block's fields, and this one did not";

/// What an entry wearing a declaration's name without being a file is told.
///
/// Refused rather than passed over: naming a thing `*.luau` is how a mod author
/// says *this is a declaration*, so an entry that says it and is not one is a
/// mistake worth reporting. Skipping it silently would register whatever else
/// the directory held and leave the author's own statement unanswered.
const NOT_A_FILE: &str = "an entry named like a declaration must be a file, and this one is not";

/// What content printed while a source was read, and whether the host kept all
/// of it.
///
/// **One value rather than a record beside a count**, and that is the whole
/// design. The host bounds what it retains and reports how many lines it stopped
/// keeping, because "the mod printed nothing" and "the host stopped keeping what
/// the mod printed" are different facts. Handing on the lines and leaving the
/// count to be asked for separately would re-open that distinction at the
/// boundary a failed load is actually read from: a truncated record would look
/// exactly like a declaration that printed that much and stopped. A caller
/// cannot reach the lines here without meeting the answer.
///
/// **Script-controlled text**, at whatever length a mod chose: whoever routes
/// this to a log inherits both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Printed {
    /// Everything the declarations printed.
    Whole(Vec<String>),
    /// The earliest lines the host kept, and how many it stopped keeping.
    ///
    /// The earliest rather than the latest: the first line a chunk printed is
    /// what locates a failed load, and the millionth is not. `dropped` cannot be
    /// zero, so a record that says it was truncated always says by how much.
    Truncated {
        /// The lines kept, in the order they were printed.
        kept: Vec<String>,
        /// How many lines the host was handed and did not keep.
        dropped: NonZeroU64,
    },
}

impl Printed {
    /// The record for `kept` lines with `dropped` refused, which is `Whole`
    /// exactly when nothing was refused.
    fn of(kept: Vec<String>, dropped: u64) -> Self {
        match NonZeroU64::new(dropped) {
            None => Self::Whole(kept),
            Some(dropped) => Self::Truncated { kept, dropped },
        }
    }
}

/// Block definitions declared by the Luau chunks under a content root.
///
/// Construction is infallible and touches no disk: a root that does not exist is
/// not a programming error, it is something a mod author did, and it is reported
/// the same way every other content problem is — as a failure of the stream,
/// naming the path.
#[derive(Debug)]
pub struct LuauFileDefinitionSource {
    root: PathBuf,
    /// What content printed while this source was **last** read.
    ///
    /// Replaced rather than extended on each read. A tally kept across every
    /// read of a source's life would grow without bound as an author saved a
    /// file over and over, while reporting each time that the mod printed more
    /// than it did.
    printed: RefCell<Printed>,
}

impl LuauFileDefinitionSource {
    /// Definitions declared under `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            printed: RefCell::new(Printed::Whole(Vec::new())),
        }
    }

    /// What content printed while this source was last read, and whether the
    /// host kept all of it. See [`Printed`].
    #[must_use]
    pub fn printed(&self) -> Printed {
        self.printed.borrow().clone()
    }

    /// Where this source's declarations live.
    fn declarations(&self) -> PathBuf {
        self.root.join(BLOCKS_DIRECTORY)
    }

    /// Every declaration under the declarations directory, in file-name order.
    ///
    /// The order is the loader's own and not the filesystem's, because it is a
    /// content-facing contract: it decides which block reaches a player's hand
    /// and which of two files declaring one name is named first when the pair is
    /// refused. A directory listing is in whatever order the filesystem keeps —
    /// on NTFS its own case-insensitive name order, elsewhere often the order
    /// entries were created — and none of those is a thing a mod author can see
    /// or reason about.
    ///
    /// Sorting happens **before** each entry is checked to be a file, so that a
    /// root holding two offenders refuses the same one on every run.
    fn declaration_files(&self) -> Result<Vec<PathBuf>, DefinitionSourceError> {
        let mut named = self.entries_named_as_declarations()?;
        // **Counted before anything is asked about any single entry**, which is
        // the whole point of a bound on how many there are: a loader that
        // checked each entry to be a file first would make four thousand and
        // ninety-seven filesystem calls to reach a refusal whose reason was
        // known from the length of the listing.
        if named.len() > DECLARATIONS_A_ROOT_MAY_HOLD {
            return Err(too_many_declarations(&self.declarations(), named.len()));
        }
        named.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
        for entry in &named {
            confirmed_file(entry)?;
        }
        Ok(named)
    }

    /// Every entry of the declarations directory whose name says it is a
    /// declaration, unordered and not yet known to be a file.
    fn entries_named_as_declarations(&self) -> Result<Vec<PathBuf>, DefinitionSourceError> {
        let declarations = self.declarations();
        let listing =
            fs::read_dir(&declarations).map_err(|cause| unreadable(&declarations, &cause))?;
        listing
            .map(|entry| entry.map(|found| found.path()))
            .filter(worth_reading)
            .collect::<Result<Vec<PathBuf>, _>>()
            .map_err(|cause| unreadable(&declarations, &cause))
    }

    /// Every declaration the root holds, read through one host that does not
    /// outlive this call.
    fn read_declarations(&self) -> Vec<Result<BlockDefinition, DefinitionSourceError>> {
        let files = match self.declaration_files() {
            Ok(files) => files,
            Err(refusal) => return vec![Err(refusal)],
        };
        // A host that will not start is not the content's fault, and must not
        // be reported as a malformed declaration.
        let mut host = match ScriptHost::new() {
            Ok(host) => host,
            Err(cause) => return vec![Err(unreadable(&self.declarations(), &cause))],
        };
        let mut read = Vec::with_capacity(files.len());
        for file in &files {
            read.push(definition_in(&mut host, file));
        }
        // Taken together, because the count is what tells a truncated record
        // from a short one and asking for it separately is how the two come
        // apart again.
        self.printed.replace(Printed::of(
            host.printed().to_vec(),
            host.dropped_print_lines(),
        ));
        read
    }
}

impl DefinitionSource for LuauFileDefinitionSource {
    fn origin(&self) -> DefinitionOrigin {
        origin_of(&self.root)
    }

    fn definitions(&self) -> DefinitionStream<'_> {
        Box::new(self.read_declarations().into_iter())
    }
}

/// Whether a listed entry is one the loader should go on to read.
///
/// An entry the listing could not produce at all answers `true`, so that
/// collecting reports it rather than passing over a directory this cannot see.
/// Only entries that were read and are not named as declarations are dropped.
fn worth_reading(entry: &io::Result<PathBuf>) -> bool {
    match entry {
        Ok(path) => names_a_declaration(path),
        Err(_) => true,
    }
}

/// A content root holding more declarations than it may.
///
/// Blames the directory rather than any file in it, because the mistake is the
/// directory's and no single declaration is at fault. Both quantities are stated
/// so a reader can tell one file too many from a hundred thousand.
fn too_many_declarations(declarations: &Path, found: usize) -> DefinitionSourceError {
    DefinitionSourceError::Malformed(DefinitionFault {
        origin: origin_of(declarations),
        block: None,
        field: None,
        cause: format!(
            "this directory holds {found} declarations, and a content root may hold at most \
             {DECLARATIONS_A_ROOT_MAY_HOLD}"
        ),
    })
}

/// Nothing, once `file` is small enough to be read.
///
/// **Measured from the directory entry before the file is opened.** A loader
/// that read first and checked after would hand a 300 KiB file to the scripting
/// host and report whatever the compiler said about it — a true statement about
/// the wrong problem, sending its author to edit text that was never going to be
/// read.
fn within_the_size_bound(file: &Path) -> Result<(), DefinitionSourceError> {
    let found = fs::metadata(file).map_err(|cause| unreadable(file, &cause))?;
    let bytes = usize::try_from(found.len()).unwrap_or(usize::MAX);
    if bytes <= BYTES_A_DECLARATION_MAY_HOLD {
        return Ok(());
    }
    Err(DefinitionSourceError::Malformed(DefinitionFault {
        origin: origin_of(file),
        block: None,
        field: None,
        cause: format!(
            "this declaration holds {bytes} bytes, and a declaration file may hold at most \
             {BYTES_A_DECLARATION_MAY_HOLD}"
        ),
    }))
}

/// Nothing, once `entry` is known to be a file.
///
/// # Errors
///
/// Returns the refusal naming `entry` where it could not be read at all, or
/// where it wears a declaration's name without being a file.
fn confirmed_file(entry: &Path) -> Result<(), DefinitionSourceError> {
    let found = fs::metadata(entry).map_err(|cause| unreadable(entry, &cause))?;
    if found.is_file() {
        return Ok(());
    }
    Err(unreadable(entry, &NOT_A_FILE))
}

/// Whether `path`'s name says it holds a declaration.
///
/// The name alone, which is all a listing knows. Whether the thing wearing it is
/// really a file is a separate question, asked separately, because the two have
/// different answers: a name that does not claim to be a declaration is passed
/// over in silence, and one that claims it and is not is refused.
fn names_a_declaration(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == DECLARATION_EXTENSION)
}

/// A path as a label to quote back to whoever wrote the content.
fn origin_of(path: &Path) -> DefinitionOrigin {
    DefinitionOrigin::new(path.display().to_string())
}

/// A path that could not be listed or read, and why.
fn unreadable(path: &Path, cause: &impl fmt::Display) -> DefinitionSourceError {
    DefinitionSourceError::Unreadable {
        origin: origin_of(path),
        cause: cause.to_string(),
    }
}

/// The definition one declaration file holds.
fn definition_in(
    host: &mut ScriptHost,
    file: &Path,
) -> Result<BlockDefinition, DefinitionSourceError> {
    within_the_size_bound(file)?;
    let source = fs::read_to_string(file).map_err(|cause| unreadable(file, &cause))?;
    let evaluated = host
        .evaluate(&chunk_name_of(file), &source)
        .map_err(|fault| chunk_refusal(file, &fault))?;
    let ScriptValue::Table(declaration) = evaluated else {
        return Err(DefinitionSourceError::Malformed(DefinitionFault {
            origin: origin_of(file),
            block: None,
            field: None,
            cause: NOT_A_DECLARATION.to_owned(),
        }));
    };
    checked_declaration(host, &declaration, &origin_of(file))
        .map_err(DefinitionSourceError::Malformed)
}

/// What the host is asked to call a chunk.
///
/// The file's own name, which is what a mod author recognises. It is a label
/// and not a path: the host never opens a file, and it reports back only what
/// it was told.
fn chunk_name_of(file: &Path) -> String {
    file.file_name().map_or_else(
        || file.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// A chunk that would not compile, raised, or was stopped for exceeding a
/// limit.
///
/// **The origin is the loader's own**, built from the path it opened rather than
/// lifted out of the fault. The host is handed a label and never opens anything,
/// so what it can report back is the label it was given — `amber.luau`, which is
/// not a thing a mod author with several content roots can go and open. Only the
/// loader knows which file that was.
///
/// The cause is composed from the fault's **typed fields** rather than from its
/// whole rendering, which opens by naming the chunk — and a definition fault
/// renders its own origin already, so splicing one into the other would state
/// the location twice.
fn chunk_refusal(file: &Path, fault: &ScriptFault) -> DefinitionSourceError {
    DefinitionSourceError::Malformed(DefinitionFault {
        origin: origin_of(file),
        block: None,
        field: None,
        cause: faulted_cause(fault),
    })
}

/// Why a chunk failed, and where in the chunk the backend said so.
///
/// The line is carried as a typed field of the fault precisely so that it
/// survives a backend that rewords its own message, so it is stated here rather
/// than left inside whatever text the backend produced. A fault that named no
/// line says nothing about one instead of inventing a zero.
fn faulted_cause(fault: &ScriptFault) -> String {
    let kind = fault.kind.as_str();
    fault.line.map_or_else(
        || format!("{kind}: {}", fault.cause),
        |line| format!("{kind}: line {line}: {}", fault.cause),
    )
}
