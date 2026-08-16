//! The hostile-mod harness decides nothing the host decides.
//!
//! # The failure this file exists to prevent
//!
//! Every scenario about the harness runs **through** the harness, so no other
//! test in this suite can see it cheating. A harness that carries its own copy
//! of the deny list, sets its own budget, or arms a second interrupt agrees with
//! the host by construction: it would report all six hostile cases contained on
//! the day the host's own enforcement was deleted, and the run would read
//! exactly as it reads now.
//!
//! `crates/mc-client/tests/seam_boundaries.rs` records the identical failure in
//! its own words — *"A harness that gated its own pointer motion on the capture
//! policy would agree with the client by construction and pass a three-needle
//! scan while the client's own gate was deleted."* This is that shape, one crate
//! over and one level up: there the harness must not ask the policy, here it
//! must not **be** the policy.
//!
//! So the deny list the harness probes is `ScriptHost::DENIED_GLOBALS`, the
//! faults it requires are the host's own `FaultKind`s, and the limits that stop
//! the runaway cases are the ones the host ships — the harness constructs no
//! limits at all, which is why every spelling of one is an offence below.
//!
//! # The needles are generated from the host's declaration
//!
//! The deny-list needles are built from `ScriptHost::DENIED_GLOBALS` rather than
//! written out, because a guard that carried its own transcription of the list
//! would be committing, in the file that forbids it, the exact offence it
//! forbids — and would stop matching the day the host added a name. What is
//! written by hand is the shorter list of spellings that are a re-implementation
//! whatever they are used for: a limits record, a host built with limits of
//! somebody's choosing, the three non-zero types those limits are made of, and a
//! second interrupt.
//!
//! The host's latch is not among them and cannot be: it is private to the crate,
//! so a test binary could not build a second one if it tried. The needle that
//! remains is the reachable half of that concern.
//!
//! # What is read, and how the exemption is compared
//!
//! One root — this crate's whole `tests/` tree — with everything outside
//! `tests/support/hostile/` passed over, judged **segment by segment against the
//! whole path** and never on a bare file name. Both halves matter. The rest of
//! the suite may name a denied global and build a limits record freely:
//! `sandbox_surface.rs` is a list of those names on purpose and
//! `callback_memory.rs` is nothing but configured limits. And a file-name
//! comparison would place the exemption on precisely the name a harness file
//! re-implementing a policy would be given, which is what the control's
//! `tests/support/hostile/sandbox_surface.rs` is for.
//!
//! The count is kept **per root** rather than as a total, so that a second root
//! added later cannot go unread while the total stays healthy — the failure
//! `crates/mc-script/tests/mlua_containment.rs` records for the same walk.
//!
//! # The control is what makes this meaningful at all
//!
//! A harness that names nothing scans clean, and so does a walk that broke, a
//! needle that stopped matching, and an exemption that grew to cover the tree.
//! The second test points the same scan at a tree that *does* re-implement the
//! host's policy — every needle committed once, so a mistyped one is caught here
//! rather than standing unwatched forever — and requires each to be reported
//! while the two files that are allowed to name a policy are passed over.

use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use mc_script::ScriptHost;

type TestResult = Result<(), Box<dyn Error>>;

/// The directories walked, each relative to the crate root.
const ROOTS: [&str; 1] = ["tests"];

/// The one path prefix that is read, as path segments.
const HARNESS_PREFIX: [&str; 3] = ["tests", "support", "hostile"];

/// The spellings that are a re-implementation of the host's limits whatever they
/// are used for.
///
/// A harness that names any of these has stopped running what the host ships and
/// started running numbers of its own — at which point the six cases are stopped
/// by the harness's configuration rather than by the host's defaults, and the
/// defaults themselves have nothing exercising them.
const A_SECOND_SOURCE_OF_POLICY: [&str; 6] = [
    "HostLimits {",
    "with_limits",
    "NonZeroU64",
    "NonZeroUsize",
    "NonZeroU32",
    "set_interrupt",
];

/// What a scan of the roots found.
#[derive(Debug, Default)]
struct Scan {
    /// How many files each root contributed, kept apart so a root that went
    /// unread is distinguishable from a root that is merely clean.
    read_per_root: BTreeMap<&'static str, usize>,
    hits: Vec<String>,
}

/// What a scan amounts to.
///
/// A total verdict rather than a boolean: "the harness re-implements nothing"
/// and "the harness was never read" are different facts, only one of them is
/// good news, and an `is_empty()` on the hit list cannot tell them apart.
#[derive(Debug, PartialEq, Eq)]
enum HarnessVerdict {
    /// The harness was read, and every policy it applies comes from the host.
    EveryPolicyComesFromTheHost,
    /// Every place the harness states a policy of its own.
    ReimplementsPolicy(Vec<String>),
    /// These roots contributed no file at all, so nothing was judged.
    ScanFoundNoFiles(Vec<&'static str>),
}

/// Every spelling whose presence in the harness is the offence.
///
/// The deny-list half is generated from the host's own declaration, quoted, so
/// that it matches a Rust string literal naming a denied global and not the
/// prose or the generated script text that legitimately mentions one.
fn needles() -> Vec<String> {
    ScriptHost::DENIED_GLOBALS
        .iter()
        .map(|name| format!("\"{name}\""))
        .chain(
            A_SECOND_SOURCE_OF_POLICY
                .iter()
                .map(|spelling| (*spelling).to_owned()),
        )
        .collect()
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

/// Whether a file is read at all, judged on its whole path and never on its
/// name.
fn is_harness_source(relative: &str) -> bool {
    let segments: Vec<&str> = relative.split('/').collect();
    segments.len() > HARNESS_PREFIX.len()
        && segments
            .iter()
            .zip(HARNESS_PREFIX.iter())
            .all(|(segment, wanted)| segment == wanted)
}

fn is_rust_source(path: &Path) -> bool {
    path.extension().and_then(OsStr::to_str) == Some("rs")
}

/// A file's text with its doc comments removed, on the usual reasoning: prose
/// about a denied global or about a limit is documentation, not use.
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
    if !is_harness_source(&relative) {
        return Ok(());
    }
    let text = production_text(&fs::read_to_string(path)?);
    *scanned.read_per_root.entry(root).or_default() += 1;
    for needle in needles() {
        if text.contains(&needle) {
            scanned.hits.push(format!("{relative} states `{needle}`"));
        }
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

/// Reads every harness source under the roots and reports each place one states
/// a policy of its own.
///
/// A root that does not exist contributes no file rather than an error, which
/// leaves the per-root count — and not an I/O failure — to report a harness
/// directory that has moved or gone.
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
fn verdict(scanned: &Scan) -> HarnessVerdict {
    let unread: Vec<&'static str> = scanned
        .read_per_root
        .iter()
        .filter(|(_, read)| **read == 0)
        .map(|(root, _)| *root)
        .collect();
    if !unread.is_empty() {
        return HarnessVerdict::ScanFoundNoFiles(unread);
    }
    if scanned.hits.is_empty() {
        return HarnessVerdict::EveryPolicyComesFromTheHost;
    }
    HarnessVerdict::ReimplementsPolicy(scanned.hits.clone())
}

const WHY_THE_HARNESS_MAY_STATE_NO_POLICY_OF_ITS_OWN: &str = "the harness is the only witness the six hostile cases have, and a harness that states \
     the policy itself is a witness to its own testimony: give it its own deny list and it \
     reports every name gone whatever the host removes; give it its own budget and it stops \
     the runaway itself; give it its own interrupt and the host's need never fire. All five \
     scenarios of this feature would stay green through the deletion of the enforcement they \
     exist to prove. The verdict names any root it read nothing under, so a harness directory \
     that moved reports as a refusal rather than as the same clean answer an obedient harness \
     gives.";

#[test]
fn the_hostile_harness_states_no_deny_list_no_limit_and_no_interrupt_of_its_own() -> TestResult {
    let scanned = scan(&crate_root())?;

    assert_eq!(
        verdict(&scanned),
        HarnessVerdict::EveryPolicyComesFromTheHost,
        "{WHY_THE_HARNESS_MAY_STATE_NO_POLICY_OF_ITS_OWN}"
    );
    Ok(())
}

/// The control for the check above, in five directions at once.
///
/// A walk that broke, a needle that matches nothing, or an exemption that grew
/// would all report an obedient harness forever — including on the day it stops
/// being one. So the fixture states a policy in two places and is passed over in
/// two more, and each is a different way of getting this wrong:
///
/// - a harness file naming **every** needle the guard carries, because a needle
///   no fixture ever commits is a needle nobody has watched match anything:
///   mistype one and it reports a clean harness for as long as it stands there;
/// - a harness file wearing the name of a suite file that is allowed to name
///   these things, which a bare-name exemption would excuse — and which is
///   exactly what a harness re-implementing the sandbox surface would be called;
/// - the suite file it is named after, which must be passed over;
/// - a file sitting beside the harness directory whose *name* begins with the
///   directory's, which a string-prefix comparison would read into the harness.
///
/// The expected count is derived from the needle list rather than transcribed,
/// so a needle added without a fixture line to catch it reddens here.
#[test]
fn the_same_scan_reports_a_harness_file_that_states_a_policy_wherever_it_sits() -> TestResult {
    let fixture = tempfile::tempdir()?;
    a_harness_that_states_the_policy_itself(fixture.path())?;

    let scanned = scan(fixture.path())?;
    let reported = |file: &str| scanned.hits.iter().any(|hit| hit.starts_with(file));
    let states_its_own_policy = matches!(verdict(&scanned), HarnessVerdict::ReimplementsPolicy(_));

    assert_eq!(
        (
            states_its_own_policy,
            scanned.read_per_root.get("tests").copied(),
            scanned.hits.len(),
            reported("tests/support/hostile/probe.rs"),
            reported("tests/support/hostile/sandbox_surface.rs"),
        ),
        (true, Some(2), needles().len() + 1, true, true),
        "the scan has to reach into the harness directory, report every spelling of a policy it \
         finds there — one hit per needle, so a needle that has stopped matching anything is \
         caught here rather than passing silently — report a harness file whatever it is called, \
         and pass over the two files that are allowed to name a policy: the suite file whose name \
         it borrowed, and the file beside the directory whose name merely starts the same way: \
         {:?}",
        scanned.hits
    );
    Ok(())
}

/// The offending tree the control above scans, written under `root`.
///
/// The first file states every policy the guard knows how to look for, one per
/// line, so the expected hit count is the needle list's own length.
fn a_harness_that_states_the_policy_itself(root: &Path) -> Result<(), Box<dyn Error>> {
    let harness = root.join("tests/support/hostile");
    let suite = root.join("tests");
    fs::create_dir_all(&harness)?;
    fs::create_dir_all(suite.join("support"))?;
    let first_denied_name = ScriptHost::DENIED_GLOBALS.first().copied().unwrap_or("io");
    let a_copy_of_the_deny_list = format!("const DENIED: [&str; 1] = [\"{first_denied_name}\"];\n");
    fs::write(harness.join("probe.rs"), needles().join("\n"))?;
    fs::write(harness.join("sandbox_surface.rs"), &a_copy_of_the_deny_list)?;
    fs::write(suite.join("sandbox_surface.rs"), &a_copy_of_the_deny_list)?;
    fs::write(
        suite.join("support/hostile_probe.rs"),
        &a_copy_of_the_deny_list,
    )?;
    Ok(())
}
