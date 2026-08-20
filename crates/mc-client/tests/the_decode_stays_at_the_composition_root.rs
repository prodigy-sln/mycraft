//! Where a PNG may be turned into texels, and where a path may be named.
//!
//! # One decision, two halves, and only together do they say anything
//!
//! `mc-render` has no `std::fs`, no `PathBuf` and no image decoder anywhere in
//! `src/`, and this spec does not give it one: it gains the ability to be
//! *handed* texels for a key. The read and the decode land in `mc-client`, which
//! is already the composition root and already the only crate that builds
//! `TextureLayers`. That is the whole of the boundary — and each half is easy to
//! keep by accident and easy to lose without anyone noticing, because losing
//! either one compiles, runs and draws exactly the same picture.
//!
//! The two failures are different mistakes. A filesystem type in the renderer is
//! a crate that can go on to *read* what it was supposed to be handed, which is
//! how a texture pack becomes a renderer feature instead of a client one. A
//! second file in the client naming the decoder is a vendor swap that has to be
//! made in two places, in a crate whose other confined dependencies are each
//! confined to one.
//!
//! # These are text scans over `src/`, and the needles are paths not words
//!
//! `image::` and `PathBuf` are spelled with enough of their surroundings to be a
//! type or a crate rather than a variable somebody called `image` or a doc
//! comment mentioning a path. That is a real distinction and it is why the
//! needles carry their separators.
//!
//! # Each absence is paired with a control, as a separate test function
//!
//! A scan pointed at a directory that no longer exists, or one whose walk
//! stopped descending, reports nothing found — which is the same answer as a
//! boundary that is being kept. So each scan is also run over a fixture written
//! for the purpose that *does* contain the needle, and has to report it. Split
//! into separate test functions deliberately: as one test, "the control failed
//! while the real assertion still passed" is not something a run can show you.
//!
//! **The verdicts are enumerated, not absences.** The client's scan states the
//! one file that may name the decoder rather than a count, so a decoder that
//! moved somewhere else is a different answer rather than the same one.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The one file of the client that may name the image decoder.
///
/// Stated as the path a reader would go and open, `/`-separated so the
/// expectation reads the same on either platform.
const THE_DECODERS_FILE: &str = "crates/mc-client/src/textures/decode.rs";

/// How the image decoder is named in Rust, with the separator that makes it a
/// crate path rather than a word.
const THE_DECODER: &str = "image::";

/// The filesystem types the renderer may not name.
///
/// `PathBuf` rather than `Path`, because `Path` appears inside `std::path` in
/// prose and inside unrelated identifiers; a `PathBuf` is the owned type a
/// crate that had started keeping filenames would reach for. `std::fs` carries
/// its own separator for the same reason.
const FILESYSTEM_TYPES: [&str; 3] = ["std::fs", "PathBuf", "std::path::"];

/// A fixture written for the controls: a Rust file naming every needle above.
///
/// Written into a temporary directory rather than committed, because a committed
/// one under either crate's `src/` would be found by the scans it exists to
/// control.
const A_FILE_THAT_NAMES_THEM: &str =
    "use image::ImageReader;\nuse std::fs;\nuse std::path::PathBuf;\nfn reads(at: PathBuf) {}\n";
const ITS_NAME: &str = "names-them.rs";

#[test]
fn exactly_one_file_of_the_client_names_the_image_decoder() -> TestResult {
    let root = repository_root()?;

    let naming = files_naming(
        &root.join("crates").join("mc-client").join("src"),
        &[THE_DECODER],
    )?;

    assert_eq!(
        as_repository_paths(&root, &naming)?,
        vec![THE_DECODERS_FILE.to_owned()],
        "the client reads the built set and decodes its PNGs, and it does that in one file so \
         that swapping the decoder is one edit and so that every other file of the composition \
         root stays a file about content rather than about a format. This is stated as the path \
         rather than as a count: a decoder that moved is a different answer, not the same one"
    );
    Ok(())
}

#[test]
fn the_same_scan_reports_a_file_that_does_name_the_image_decoder() -> TestResult {
    let holding = a_directory_naming_them()?;

    let naming = files_naming(holding.path(), &[THE_DECODER])?;

    assert_eq!(
        naming.len(),
        1,
        "this is the control for the reading above, and without it a scan pointed at a directory \
         that had moved would report one file forever — or none, and read as a boundary being \
         kept. It looked at {} and found {naming:?}",
        holding.path().display()
    );
    Ok(())
}

#[test]
fn no_file_of_the_renderer_names_a_filesystem_type() -> TestResult {
    let root = repository_root()?;

    let naming = files_naming(
        &root.join("crates").join("mc-render").join("src"),
        &FILESYSTEM_TYPES,
    )?;

    assert_eq!(
        as_repository_paths(&root, &naming)?,
        Vec::<String>::new(),
        "the renderer is handed texels and never reads them, which is what keeps a texture pack \
         a client-side thing and keeps this crate testable without a filesystem. A path type \
         here is the first step of the other design, and it compiles and draws the same picture \
         — so nothing but this reports it"
    );
    Ok(())
}

#[test]
fn the_same_scan_reports_a_file_that_does_name_a_filesystem_type() -> TestResult {
    let holding = a_directory_naming_them()?;

    let naming = files_naming(holding.path(), &FILESYSTEM_TYPES)?;

    assert_eq!(
        naming.len(),
        1,
        "this is the control for the reading above. An absence that cannot be turned into a \
         presence is an absence nobody can trust: it looked at {} and found {naming:?}",
        holding.path().display()
    );
    Ok(())
}

/// The repository's own root, located upwards from this crate.
///
/// **Located here rather than through the shared harness**, and that is the
/// point of it: this file asks a question about the *tree*, so it links nothing
/// but the standard library and stays compilable while the crate it scans is
/// halfway through a change.
///
/// # Errors
///
/// Returns an error if the manifest directory has no grandparent.
fn repository_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the crate manifest directory has no repository root above it")?
        .to_owned())
}

/// A temporary directory holding one Rust file that names every needle.
///
/// # Errors
///
/// Returns the failure to create or write it.
fn a_directory_naming_them() -> Result<tempfile::TempDir, Box<dyn Error>> {
    let holding = tempfile::TempDir::new()?;
    fs::write(holding.path().join(ITS_NAME), A_FILE_THAT_NAMES_THEM)?;
    Ok(holding)
}

/// Every `.rs` file under `at` naming any of `needles`, in path order.
///
/// # Errors
///
/// Returns the failure to read the tree, which for a directory this suite names
/// is a scan that can no longer look rather than a boundary being kept.
fn files_naming(at: &Path, needles: &[&str]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut found = Vec::new();
    let mut walking = vec![at.to_owned()];
    while let Some(directory) = walking.pop() {
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            visit(path, needles, &mut walking, &mut found)?;
        }
    }
    found.sort();
    Ok(found)
}

/// Descends into `path` if it is a directory, and records it if it is a Rust
/// file naming one of `needles`.
fn visit(
    path: PathBuf,
    needles: &[&str],
    walking: &mut Vec<PathBuf>,
    found: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    if path.is_dir() {
        walking.push(path);
        return Ok(());
    }
    if !path.extension().is_some_and(|kind| kind == "rs") {
        return Ok(());
    }
    let written = fs::read_to_string(&path)?;
    if needles.iter().any(|needle| written.contains(needle)) {
        found.push(path);
    }
    Ok(())
}

/// `found`, spelled as paths under the repository with `/` separators.
///
/// # Errors
///
/// Returns a failure when a path is not under the repository or is not UTF-8,
/// both of which mean the reading below would be about a tree nobody described.
fn as_repository_paths(root: &Path, found: &[PathBuf]) -> Result<Vec<String>, Box<dyn Error>> {
    found
        .iter()
        .map(|path| {
            let under = path.strip_prefix(root)?;
            Ok(under
                .to_str()
                .ok_or("a file under the repository has a name that is not UTF-8")?
                .replace('\\', "/"))
        })
        .collect()
}
