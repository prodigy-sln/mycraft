//! The scripting backend is named in one directory of this crate and nowhere
//! else.
//!
//! The crate's public surface is the port — the host, its values, its faults and
//! its dispatch report — and the backend's own types stay behind it. That is not
//! isolation theatre: the backend is pre-1.0, breaking minors are routine, and a
//! second backend is deferred rather than rejected, so a vendor type on a public
//! signature is a migration this crate could not perform without breaking every
//! consumer. An unenforced litmus is a claim, so this is the enforcement.
//!
//! It lands **before** the backend does. A guard written after the code it
//! guards has never once been observed refusing anything, and there is no run of
//! it that says whether it works.
//!
//! # Two roots, and the second is the one that gets forgotten
//!
//! `src/` is the obvious tree. `tests/` is scanned too, because the hostile-mod
//! harness is the code most likely to reach for the backend directly — a harness
//! that built its own VM would be verifying the host against a copy of the
//! thing it is supposed to be watching, and a guard that looked only at `src/`
//! would see none of it.
//!
//! Each root's contribution is counted **on its own**. "The scan read more than
//! zero files" is vacuous the moment one root is large: a root that contributes
//! nothing leaves the total healthy and the absence check green over a tree
//! nothing read.
//!
//! # What is passed over, and how
//!
//! Two exemptions, both compared **segment by segment against the whole path
//! relative to the crate root** — never against a bare file name. That is the
//! trap `crates/mc-client/tests/seam_boundaries.rs` records: a name-only
//! exemption for `vm.rs` silently excuses a `tests/support/hostile/vm.rs`, which
//! is precisely what a leak would be called.
//!
//! 1. Anything under `src/luau/` — the adapter, which exists to name the
//!    backend. Note that this is `src/luau/` and not "any directory called
//!    `luau`": a harness directory of that name would be a leak wearing the
//!    adapter's clothes.
//! 2. This file, whose needle would otherwise be its own hit.
//!
//! Sibling `*_test.rs` unit files are **not** exempt, which is a deliberate
//! divergence from the other text guards in this repository. Those skip test
//! code because their invariants are about production behaviour; this one is
//! about which code may hold a vendor type, and a unit test under `src/` holding
//! one is the same leak as its module holding one. The adapter's own siblings
//! are already covered by the first exemption.
//!
//! Doc comments are stripped before matching, on the usual reasoning: prose and
//! rustdoc examples about the backend are documentation, not use.
//!
//! # The control is what makes this meaningful on its first run
//!
//! An empty tree scans clean, so the real check below says nothing on its own
//! until the crate has a backend to leak. The second test is what carries the
//! file: it points the same scan at a tree that *does* leak, in four places at
//! once, and requires each to be reported while the two exempt files are passed
//! over.

use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The directories walked, each relative to the crate root.
const ROOTS: [&str; 2] = ["src", "tests"];

/// The spelling whose presence in production text is the offence.
const NEEDLE: &str = "mlua";

/// The one path prefix allowed to name it, as path segments.
const ADAPTER_PREFIX: [&str; 2] = ["src", "luau"];

/// This file, relative to the crate root and spelled with `/` on every platform.
const GUARD_FILE: &str = "tests/mlua_containment.rs";

/// What a scan of both roots found.
#[derive(Debug, Default)]
struct Scan {
    /// How many files each root contributed, kept apart so a root that went
    /// unread is distinguishable from a root that is merely clean.
    read_per_root: BTreeMap<&'static str, usize>,
    hits: Vec<String>,
}

/// What a scan amounts to.
///
/// A total verdict rather than a boolean, because "nothing named it" and "a root
/// went unread" are different facts and only the first is good news — and an
/// `is_empty()` on the hit list cannot tell them apart.
#[derive(Debug, PartialEq, Eq)]
enum ContainmentVerdict {
    /// Both roots contributed source, and none of it outside the adapter names
    /// the backend.
    EveryMluaReferenceIsUnderLuauDir,
    /// Every place outside the adapter that names it.
    LeakedOutside(Vec<String>),
    /// These roots contributed no file at all, so whatever lives under them went
    /// unscanned and no verdict about them is available.
    ScanFoundNoFiles(Vec<&'static str>),
}

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Where a file sits relative to the crate root, spelled with `/` on every
/// platform so an exemption can be written once and compared whole.
fn relative_spelling(path: &Path, crate_root: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .strip_prefix(crate_root)?
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/"))
}

/// Whether a file is passed over, judged on its whole path and never on its
/// name.
fn is_exempt(relative: &str) -> bool {
    if relative == GUARD_FILE {
        return true;
    }
    let segments: Vec<&str> = relative.split('/').collect();
    segments.len() > ADAPTER_PREFIX.len()
        && segments
            .iter()
            .zip(ADAPTER_PREFIX.iter())
            .all(|(segment, wanted)| segment == wanted)
}

fn is_rust_source(path: &Path) -> bool {
    path.extension().and_then(OsStr::to_str) == Some("rs")
}

/// A file's text with its doc comments removed.
fn production_text(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("///") && !trimmed.starts_with("//!")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read(
    path: &Path,
    crate_root: &Path,
    root: &'static str,
    scanned: &mut Scan,
) -> Result<(), Box<dyn Error>> {
    let relative = relative_spelling(path, crate_root)?;
    if is_exempt(&relative) {
        return Ok(());
    }
    let text = production_text(&fs::read_to_string(path)?);
    *scanned.read_per_root.entry(root).or_default() += 1;
    if text.contains(NEEDLE) {
        scanned.hits.push(format!("{relative} names `{NEEDLE}`"));
    }
    Ok(())
}

fn walk(
    directory: &Path,
    crate_root: &Path,
    root: &'static str,
    scanned: &mut Scan,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, crate_root, root, scanned)?;
        } else if is_rust_source(&path) {
            read(&path, crate_root, root, scanned)?;
        }
    }
    Ok(())
}

/// Reads every Rust source under both roots and reports each place outside the
/// adapter that names the backend.
///
/// A root that does not exist contributes no file rather than an error, which
/// leaves the per-root count — and not an I/O failure — to report a directory
/// that has moved or gone.
fn scan(crate_root: &Path) -> Result<Scan, Box<dyn Error>> {
    let mut scanned = Scan::default();
    for root in ROOTS {
        scanned.read_per_root.entry(root).or_default();
        let directory = crate_root.join(root);
        if directory.is_dir() {
            walk(&directory, crate_root, root, &mut scanned)?;
        }
    }
    Ok(scanned)
}

/// The unread-root check comes first because it explains away any answer that
/// would follow it.
fn verdict(scanned: &Scan) -> ContainmentVerdict {
    let unread: Vec<&'static str> = scanned
        .read_per_root
        .iter()
        .filter(|(_, read)| **read == 0)
        .map(|(root, _)| *root)
        .collect();
    if !unread.is_empty() {
        return ContainmentVerdict::ScanFoundNoFiles(unread);
    }
    if scanned.hits.is_empty() {
        return ContainmentVerdict::EveryMluaReferenceIsUnderLuauDir;
    }
    ContainmentVerdict::LeakedOutside(scanned.hits.clone())
}

#[test]
fn no_source_outside_the_adapter_directory_names_the_scripting_backend() -> TestResult {
    let scanned = scan(&crate_root())?;

    assert_eq!(
        verdict(&scanned),
        ContainmentVerdict::EveryMluaReferenceIsUnderLuauDir,
        "a vendor type reachable from this crate's public surface is a backend swap that cannot \
         be performed and a breaking minor that cannot be absorbed. The verdict names any root \
         it read nothing under, so a tree this scan stopped looking at reports as a refusal \
         rather than as the same clean answer an obedient crate gives"
    );
    Ok(())
}

/// The control for the check above, in six directions at once.
///
/// A walk that broke, a filter that matched nothing, or an exemption that grew
/// would all report a clean crate forever — including on the day the backend
/// leaks. So the fixture leaks in four places and is exempt in two, and each is
/// a different way of getting this wrong:
///
/// - a plain module of `src/`, the obvious case;
/// - a sibling unit-test file, which the other text guards in this repository
///   would skip and this one must not;
/// - a harness file wearing the adapter's own file name, which a name-only
///   exemption would excuse;
/// - a `tests/luau/` directory, which an exemption on any segment called `luau`
///   rather than on the `src/luau/` prefix would excuse;
/// - the adapter itself, which must be passed over;
/// - a copy of this guard file, whose own needle must not be its own hit.
///
/// The expected hit count is derived from the fixture — six files, two exempt —
/// so a leak silently stopping being reported reddens here rather than passing.
#[test]
fn the_same_scan_reports_a_leak_wherever_it_sits_and_passes_over_the_adapter() -> TestResult {
    let fixture = tempfile::tempdir()?;
    a_tree_that_leaks_the_backend(fixture.path())?;

    let scanned = scan(fixture.path())?;
    let reported = |file: &str| scanned.hits.iter().any(|hit| hit.starts_with(file));
    let leaked = matches!(verdict(&scanned), ContainmentVerdict::LeakedOutside(_));

    assert_eq!(
        (
            leaked,
            scanned.hits.len(),
            reported("src/host.rs"),
            reported("src/limits_test.rs"),
            reported("tests/support/hostile/vm.rs"),
            reported("tests/luau/probe.rs"),
        ),
        (true, 4, true, true, true, true),
        "the scan has to reach into both roots, report every file outside `src/luau/` that names \
         the backend whatever that file is called and wherever it sits, and pass over exactly \
         two: the adapter, whose job it is, and this guard, whose needle would be its own hit. \
         Four of the six files leak, so four is the count: {:?}",
        scanned.hits
    );
    Ok(())
}

/// The offending tree the control above scans, written under `root`.
fn a_tree_that_leaks_the_backend(root: &Path) -> Result<(), Box<dyn Error>> {
    let sources = root.join("src");
    let adapter = sources.join("luau");
    let harness = root.join("tests/support/hostile");
    let disguised = root.join("tests/luau");
    fs::create_dir_all(&adapter)?;
    fs::create_dir_all(&harness)?;
    fs::create_dir_all(&disguised)?;
    fs::write(adapter.join("vm.rs"), "use mlua::Lua;\n")?;
    fs::write(sources.join("host.rs"), "pub fn state() -> mlua::Lua {}\n")?;
    fs::write(
        sources.join("limits_test.rs"),
        "let state = mlua::Lua::new();\n",
    )?;
    fs::write(harness.join("vm.rs"), "let hostile = mlua::Lua::new();\n")?;
    fs::write(
        disguised.join("probe.rs"),
        "let probe = mlua::Lua::new();\n",
    )?;
    fs::write(root.join(GUARD_FILE), "const NEEDLE: &str = \"mlua\";\n")?;
    Ok(())
}
