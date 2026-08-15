//! The harness the command-line tests drive the tool through, and the verdicts
//! they grade its effects with.
//!
//! Two ways in, and they answer different questions. [`invoke`] calls the
//! library's `run` over two in-memory writers, which is where the tool's
//! behaviour lives and where almost everything is graded. [`built_binary`] runs
//! the **shipped executable** as a real subprocess, which is the only thing that
//! can say whether `main` consults any of it — testing a decision does not test
//! that the application asks for it, and a `main` that ignored the library
//! entirely would leave every in-process assertion green.
//!
//! Every verdict here is an enumerated answer rather than a boolean or an
//! absence, for the reason `testing.md` §2 gives: "no file was written" and "the
//! right file was written" must never compare equal, and neither must "the
//! pre-existing file survived" and "the check can no longer look at it".

use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use voxforge::cli::run;
use voxforge::inspect::ExitCode;

/// The line prefix an inspect report states its filled-voxel count behind.
///
/// The count is the first whitespace-separated token after it, so the report is
/// free to say `filled 22 voxels` or `filled 22`; what is contract is that the
/// number is findable without reading English.
pub const FILLED_PREFIX: &str = "filled ";

/// Everything one in-process invocation produced.
#[derive(Debug)]
pub struct Invocation {
    /// What the tool answered.
    pub code: ExitCode,
    /// Everything it wrote to stdout.
    pub out: String,
    /// Everything it wrote to stderr.
    pub err: String,
}

/// What `voxforge <arguments…>` does, in this process.
///
/// The program name is prepended here, so a caller writes only the arguments it
/// cares about.
///
/// # Errors
///
/// Returns an error when either stream holds bytes that are not UTF-8 — a
/// diagnostic nobody can read is a failure of the tool, not of the test.
pub fn invoke(arguments: &[&str]) -> Result<Invocation, Box<dyn Error>> {
    let mut argv: Vec<OsString> = vec![OsString::from("voxforge")];
    argv.extend(arguments.iter().map(|argument| OsString::from(*argument)));
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(argv, &mut out, &mut err);
    Ok(Invocation {
        code,
        out: String::from_utf8(out)?,
        err: String::from_utf8(err)?,
    })
}

/// What a preview invocation answered, beside whatever landed at its output
/// path.
///
/// The bytes are read with a defaulting read rather than a propagating one: a
/// tool that wrote no file at all must reach the assertion as an empty picture
/// and be graded there, not disappear into an I/O error the test reports as its
/// own failure.
#[derive(Debug)]
pub struct Rendered {
    /// What the tool answered.
    pub code: ExitCode,
    /// What sits at the output path, empty when nothing does.
    pub image: Vec<u8>,
}

/// How a subprocess ended.
///
/// A three-valued verdict rather than an `Option<i32>`: a process killed by a
/// signal has no code at all, and that must not read as either answer.
#[derive(Debug, PartialEq, Eq)]
pub enum Exited {
    /// Successfully.
    Zero,
    /// With a failing status.
    NonZero(i32),
    /// Carrying no status at all.
    WithoutACode,
}

/// Everything one run of the shipped executable produced.
#[derive(Debug)]
pub struct Subprocess {
    /// How it ended.
    pub exit: Exited,
    /// Everything it wrote to stdout.
    pub out: String,
    /// Everything it wrote to stderr.
    pub err: String,
}

/// What the **built** `voxforge` binary does with `arguments`.
///
/// The path comes from `CARGO_BIN_EXE_voxforge`, which Cargo sets for an
/// integration test to the executable it has just built, so this always runs
/// today's binary rather than whatever is on the path.
///
/// # Errors
///
/// Returns an error when the executable could not be started, and when either
/// stream holds bytes that are not UTF-8.
pub fn built_binary(arguments: &[&str]) -> Result<Subprocess, Box<dyn Error>> {
    let finished = Command::new(env!("CARGO_BIN_EXE_voxforge"))
        .args(arguments)
        .output()?;
    let exit = match finished.status.code() {
        Some(0) => Exited::Zero,
        Some(code) => Exited::NonZero(code),
        None => Exited::WithoutACode,
    };
    Ok(Subprocess {
        exit,
        out: String::from_utf8(finished.stdout)?,
        err: String::from_utf8(finished.stderr)?,
    })
}

/// What sits at a requested output path once the tool has run.
#[derive(Debug, PartialEq, Eq)]
pub enum Written {
    /// Nothing at all: no file is there.
    Nothing,
    /// Exactly the bytes the library encodes for what was asked for.
    ThePicture,
    /// A file holding something else.
    SomethingElse {
        /// How many bytes it holds.
        bytes: usize,
    },
}

/// Whether `path` holds `expected`.
#[must_use]
pub fn written(path: &Path, expected: &[u8]) -> Written {
    let Ok(found) = fs::read(path) else {
        return Written::Nothing;
    };
    if found == expected {
        return Written::ThePicture;
    }
    Written::SomethingElse { bytes: found.len() }
}

/// What became of a file that was already at the requested output path.
///
/// The three failing answers are separate because they accuse different code:
/// `Truncated` is a file opened before the work was done, `Rewritten` is a
/// picture replaced by one nobody asked for, and `Deleted` is a path cleared on
/// the way out.
#[derive(Debug, PartialEq, Eq)]
pub enum Survival {
    /// Byte for byte what it was.
    Untouched,
    /// Still there and shorter than it was.
    Truncated {
        /// How many bytes it held.
        was: usize,
        /// How many it holds now.
        now: usize,
    },
    /// Still there, no shorter, and holding other bytes.
    Rewritten {
        /// How many bytes it held.
        was: usize,
        /// How many it holds now.
        now: usize,
    },
    /// Gone.
    Deleted,
}

/// What became of the file that held `before` at `path`.
#[must_use]
pub fn survival(path: &Path, before: &[u8]) -> Survival {
    let Ok(found) = fs::read(path) else {
        return Survival::Deleted;
    };
    if found == before {
        return Survival::Untouched;
    }
    if found.len() < before.len() {
        return Survival::Truncated {
            was: before.len(),
            now: found.len(),
        };
    }
    Survival::Rewritten {
        was: before.len(),
        now: found.len(),
    }
}

/// What a report says about how many voxels are filled.
#[derive(Debug, PartialEq, Eq)]
pub enum Filled {
    /// That many.
    Voxels(usize),
    /// No line of the report states a count at all.
    Unstated,
    /// A line states one, and it is not a number.
    Unreadable(String),
}

/// How many voxels `report` says are filled.
#[must_use]
pub fn filled_in(report: &str) -> Filled {
    let stated = report
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(FILLED_PREFIX));
    let Some(stated) = stated else {
        return Filled::Unstated;
    };
    let count = stated.split_whitespace().next().unwrap_or_default();
    count
        .parse::<usize>()
        .map_or_else(|_| Filled::Unreadable(count.to_owned()), Filled::Voxels)
}

/// Every one of `expected` that `text` does not contain.
///
/// Owned answers so that a token built from a temporary path compares against
/// [`nothing_missing`] without a lifetime standing in the way. Empty is the
/// passing answer, and a failure prints exactly which token was absent.
#[must_use]
pub fn unnamed_in(text: &str, expected: &[&str]) -> Vec<String> {
    expected
        .iter()
        .filter(|token| !text.contains(*token))
        .map(|token| (*token).to_owned())
        .collect()
}

/// Nothing — what [`unnamed_in`] answers for a text that named everything.
#[must_use]
pub fn nothing_missing() -> Vec<String> {
    Vec::new()
}

/// One material file's text.
///
/// `emissive` is spelled rather than computed so that a fraction reaches the
/// file as TOML writes a fraction: `1.0` and not `1`, which is an integer.
#[must_use]
pub fn material_file(key: &str, colour: &str, emissive: &str) -> String {
    format!("name = \"{key}\"\ncolor = \"{colour}\"\nemissive = {emissive}\n")
}

/// Writes `files` into a directory called `named` under `directory`, and hands
/// its path back.
///
/// Named rather than fixed because the redirection scenario needs two material
/// directories alive at once, each declaring the same key differently.
///
/// # Errors
///
/// Returns the I/O failure when the directory or a file cannot be written.
pub fn materials_at(
    directory: &TempDir,
    named: &str,
    files: &[(&str, String)],
) -> Result<PathBuf, Box<dyn Error>> {
    let root = directory.path().join(named);
    fs::create_dir_all(&root)?;
    for (file, text) in files {
        fs::write(root.join(file), text)?;
    }
    Ok(root)
}

/// Writes `text` as a document called `named` inside `directory`, and hands its
/// path back.
///
/// # Errors
///
/// Returns the I/O failure when the file cannot be written.
pub fn document_at(
    directory: &TempDir,
    named: &str,
    text: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let path = directory.path().join(named);
    fs::write(&path, text)?;
    Ok(path)
}

/// A path inside the repository, from this crate's own manifest.
///
/// Not the working directory: a test's working directory is the package root,
/// and the committed models and materials live two levels above it.
#[must_use]
pub fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}
