//! What the golden set is called, that it is all there, and what happens when
//! it is not.
//!
//! Nothing here needs a device, and that is the point: a capture id is a string
//! derived from a tick and a revision, and the golden lifecycle judges an image
//! it is handed. Both are decisions, so both are testable in the configuration
//! where `wgpu` is not in this crate's dependency graph at all.
//!
//! # Why the revision is in the id
//!
//! Ambient occlusion will one day narrow the merge predicate and invalidate
//! every committed frame. Carrying the scene revision in the id turns that from
//! a silent re-shoot into a **rename**: the commit shows added and removed PNGs
//! instead of a modified binary blob nobody can read, and a bumped revision with
//! no new goldens fails as a *missing* golden naming the path it looked for.
//!
//! The inventory check below is the other half of that. Renaming the set only
//! helps if the previous set has to go, so `goldens/` is asserted to hold
//! exactly the directories the current revision declares — an orphaned
//! revision fails the gate rather than lingering beside its replacement.
//!
//! # Placement
//!
//! `tasks.md` names a sibling `capture_test.rs`. Every property here is
//! reachable from the public API, and `docs/technical/testing.md` binds the
//! choice: *"Test through the public API in `tests/` by default; write a
//! private-access unit test only where the property genuinely has no public
//! surface."* Nothing here needs private access. No signature is affected
//! either way.

use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use mc_render::capture::{
    DECLARED_CAPTURE_TICKS, HUD_CAPTURE_TICKS, SCENE_REVISION, capture_id, declared_capture_ids,
};
use mc_testkit::frame::{
    AdapterProvenance, Backend, CaptureId, GoldenFailureReason, GoldenOutcome, GoldenSettings,
    OptIns, Rgba8Image, Thresholds, verify_against_golden,
};
use serde_json::Value;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// A revision nobody has captured anything for.
///
/// **Derived from the current revision rather than written down, and that is the
/// whole point of it.** It used to be the literal `r2` — the next revision the
/// spec named — and the day the scene revision became `r2` the two would have
/// been the same string: the test below would have compared a list of ids
/// against itself, found every one of them repeated, and failed for a reason
/// with nothing to do with whatever had moved the revision. Suffixing the
/// current one cannot collide with it however far it advances, and no directory
/// can be committed under a name the id functions will only ever produce for
/// this test.
///
/// The suffix is spelled in the alphabet a capture id admits — lowercase letters
/// and `_` — so this is a revision the id functions accept and not one they
/// refuse, which is what keeps the scenario about a *missing* golden rather than
/// about a rejected name.
fn a_revision_nothing_was_captured_for() -> String {
    format!("{SCENE_REVISION}_uncaptured")
}

/// The tick whose id the missing-golden refusal is asked for.
///
/// A **declared** capture tick rather than any valid number. The refusal is
/// about a declared id whose directory does not exist, so asking it about a tick
/// nothing declares would leave the scenario asserted against a capture the
/// inventory never looks for.
const TICK: u16 = 59;

/// The filename the golden lifecycle reads inside a capture's directory.
const GOLDEN_FILE: &str = "default.png";

/// How many captures a revision declares: one per declared terrain tick plus one
/// per declared HUD tick.
///
/// **Summed from the two declaration constants rather than taken from
/// `declared_capture_ids` itself**, which is the function under test. Its own
/// length would make the assertion below self-referential — an implementation
/// that declared *nothing* would answer `(0, 0, 0)` and satisfy an expectation of
/// `(0, 0, 0)` — and it would be exactly the "expected quantity copied from a run
/// of the code under test" that `testing.md` §2 refuses. Two constants can only
/// go wrong by a declaration being deleted, which is a different commit.
const DECLARED_CAPTURES: usize = DECLARED_CAPTURE_TICKS.len() + HUD_CAPTURE_TICKS.len();

/// Opt-ins with the update variable unset, stated rather than read.
///
/// `MYCRAFT_UPDATE_GOLDENS` is what the scenario says must be unset, and no
/// test in this project sets or unsets an environment variable — `set_var` is
/// `unsafe` in edition 2024. Constructing the value says the same thing and
/// keeps the assertion true even for a developer who is mid-re-shoot.
const NOT_UPDATING: OptIns = OptIns {
    allow_no_gpu: false,
    update_goldens: false,
};

#[test]
fn the_capture_ids_of_a_second_scene_revision_all_carry_it_and_none_repeats_the_first() -> TestResult
{
    let uncaptured = a_revision_nothing_was_captured_for();
    let first = declared_capture_ids(SCENE_REVISION)?;
    let second = declared_capture_ids(&uncaptured)?;

    let carrying = second.iter().filter(|id| id.contains(&uncaptured)).count();
    let repeated = first
        .iter()
        .zip(&second)
        .filter(|(one, other)| one == other)
        .count();

    assert_eq!(
        (second.len(), carrying, repeated),
        (DECLARED_CAPTURES, DECLARED_CAPTURES, 0),
        "a revision has to reach every declared capture's id and rename all of them: an id \
         that ignored the revision it was asked for would leave the new set colliding with \
         the old one, which is exactly the silent re-shoot the revision exists to prevent. \
         Every declared capture, terrain and HUD alike — a set that renamed only some of \
         itself is the same collision arriving through half the ids. \
         `{SCENE_REVISION}` gave {first:?} and `{uncaptured}` gave {second:?}"
    );
    Ok(())
}

#[test]
fn a_revision_whose_goldens_were_never_captured_fails_naming_the_path_it_looked_for() -> TestResult
{
    let workspace = TempDir::new()?;
    let (settings, looked_for) = settings_for_an_uncaptured_revision(workspace.path())?;

    let outcome = verify_against_golden(&a_frame()?, &an_adapter(), &settings);

    let GoldenOutcome::Failed(failure) = outcome else {
        return Err(format!("a revision with no goldens must fail, got {outcome:?}").into());
    };
    let reported = failure.to_string();
    let minted = looked_for.exists();
    assert!(
        matches!(failure.reason, GoldenFailureReason::MissingGolden { .. })
            && reported.contains(&looked_for.display().to_string())
            && !minted,
        "bumping the revision without capturing its goldens has to fail naming the path it \
         looked for, and must not mint one on the way past — a harness that captured a \
         replacement would make every future frame its own ground truth. It reported \
         `{reported}` and the golden at `{}` was {}",
        looked_for.display(),
        if minted { "written" } else { "still absent" }
    );
    Ok(())
}

/// The lifecycle's settings for a revision nothing was ever captured for, and
/// the golden path they will send it looking for.
///
/// Both come back together because the path is not a second guess at what the
/// settings imply — it is built from the same capture id and the same root, so
/// the assertion cannot pass by agreeing with a path the test made up.
fn settings_for_an_uncaptured_revision(
    workspace: &Path,
) -> Result<(GoldenSettings, PathBuf), Box<dyn Error>> {
    let capture = CaptureId::new(&capture_id(TICK, &a_revision_nothing_was_captured_for())?)?;
    let golden_root = workspace.join("goldens");
    let looked_for = golden_root.join(capture.as_str()).join(GOLDEN_FILE);
    let settings = GoldenSettings {
        golden_root,
        artifact_root: workspace.join("artifacts"),
        capture,
        thresholds: Thresholds::default(),
        opt_ins: NOT_UPDATING,
    };
    Ok((settings, looked_for))
}

#[test]
fn the_committed_goldens_are_exactly_the_directories_the_current_revision_declares() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("goldens");
    let mut committed = entries_of(&root)?;
    let mut declared = declared_capture_ids(SCENE_REVISION)?;
    committed.sort();
    declared.sort();

    assert_eq!(
        committed,
        declared,
        "`{}` has to hold exactly the captures revision `{SCENE_REVISION}` declares: a set \
         left behind by a previous revision keeps a frame nothing compares against, and a \
         directory for a capture nobody declared is a golden no test reads",
        root.display()
    );
    Ok(())
}

/// The revision the frames this build supersedes were shot under.
///
/// **Written out rather than derived, and it is the one constant here that has
/// to be.** Every other reading in this file asks whether the committed set
/// agrees with whatever revision the library currently names, which is a
/// question a set that never moved answers just as happily as one that did. This
/// asks the other question — whether the set moved *off* the revision whose
/// frames were shot under a contract this build changes — and a derived name
/// could only ever be the current one, which is the collision rather than the
/// check.
const SUPERSEDED_REVISION: &str = "r3";

#[test]
fn the_committed_goldens_carry_none_of_the_names_the_superseded_frames_were_shot_under()
-> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("goldens");
    let mut committed = entries_of(&root)?;
    let mut declared = declared_capture_ids(SCENE_REVISION)?;
    let superseded = declared_capture_ids(SUPERSEDED_REVISION)?;
    committed.sort();
    declared.sort();
    let stale: Vec<&String> = committed
        .iter()
        .filter(|name| superseded.contains(name))
        .collect();

    assert_eq!(
        (committed.clone(), stale.len()),
        (declared, 0),
        "the physics the scripted walk runs under is declared by content, and this build \
         changes it — so the committed frames describe a contract the current one is not, and \
         every capture directory has to be renamed off `{SUPERSEDED_REVISION}`. Both halves \
         are asserted together because neither carries the other: a set left on the old \
         revision satisfies nothing, and an *emptied* `goldens/` would satisfy 'none of them \
         are stale' forever. `{}` holds {committed:?} and {} of those are still \
         `{SUPERSEDED_REVISION}` names",
        root.display(),
        stale.len()
    );
    Ok(())
}

/// Everything `directory` holds, by name. A directory that is not there holds
/// nothing — which is a state the assertion is entitled to report rather than
/// an error that hides what was expected.
fn entries_of(directory: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let read = match fs::read_dir(directory) {
        Ok(read) => read,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut names = Vec::new();
    for entry in read {
        names.push(entry?.file_name().to_string_lossy().into_owned());
    }
    Ok(names)
}

/// A frame to hand the lifecycle. Its contents decide nothing here: the golden
/// it would be compared against does not exist.
fn a_frame() -> Result<Rgba8Image, Box<dyn Error>> {
    Ok(Rgba8Image::from_rgba(1, 1, vec![0, 0, 0, 255])?)
}

/// The provenance a golden would have been written with, had one been written.
fn an_adapter() -> AdapterProvenance {
    AdapterProvenance::new("test", Backend::Other, None)
}

/// The name the harness writes a golden's provenance under.
///
/// **Written out here, and it is the one hand-duplicated string in this file.**
/// `mc_testkit`'s own path helper is `pub(crate)`, so a reader of the set has no
/// public door to it; `mc-testkit/tests/support/mod.rs` carries the same literal
/// for the same reason. What keeps the two honest is that the writer's side is
/// pinned by `golden_update.rs`, which asserts the field this reads out of a
/// sidecar the harness actually wrote.
const GOLDEN_SIDECAR: &str = "default.provenance.json";

/// The field a sidecar names its own capture in.
const CAPTURE_FIELD: &str = "capture";

/// What the sidecars under a golden root say about the captures they belong to.
///
/// **A total verdict rather than a list of strays.** `assert!(faults.is_empty())`
/// cannot tell a clean set from a scan that opened nothing — a root that had
/// moved, a directory listing that came back empty, a loop that stopped
/// visiting — and every one of those answers "nothing wrong" exactly as loudly
/// as a correct set does. Both arms carry how many sidecars were read, so a scan
/// that looked at none fails on the count before its verdict is ever weighed.
#[derive(Debug, PartialEq, Eq)]
enum Provenance {
    /// Every directory's sidecar names that directory.
    EverySidecarNamesItsOwnCapture { sidecars_read: usize },
    /// The ones that do not, each named with what is wrong with it.
    Disagreeing {
        sidecars_read: usize,
        faults: Vec<String>,
    },
}

#[test]
fn every_committed_sidecar_names_the_capture_of_the_directory_holding_it() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("goldens");
    let declared = declared_capture_ids(SCENE_REVISION)?;

    assert_eq!(
        provenance_under(&root)?,
        Provenance::EverySidecarNamesItsOwnCapture {
            sidecars_read: declared.len(),
        },
        "a sidecar is the only record of which capture a committed frame is of, and it is the \
         half of a re-shoot that no other reading can see: the mint writes nothing for a capture \
         that still matches, this file's inventory compares directory *names*, and the golden \
         comparison reads *pixels* — so all three agree while a sidecar still names the revision \
         it was shot under. Measured on the 2026-08-27 re-shoot, a `git mv` passed every one of \
         them with two directories carrying stale ids. The count is `declared_capture_ids`' own \
         length rather than a number written here, so a set that grows moves it and a scan that \
         read nothing cannot pass. `{}` holds {} declared captures",
        root.display(),
        declared.len()
    );
    Ok(())
}

#[test]
fn that_same_scan_reports_a_sidecar_naming_another_capture_and_one_naming_none() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sidecars-a-scan-has-to-report");

    assert_eq!(
        provenance_under(&root)?,
        Provenance::Disagreeing {
            sidecars_read: 2,
            faults: vec![
                "`a-directory-with-no-sidecar` has no `default.provenance.json` beside its frame"
                    .to_owned(),
                "`a-sidecar-naming-another-capture`'s sidecar names \
                 `a-capture-this-directory-is-not`"
                    .to_owned(),
                "`a-sidecar-stating-no-capture`'s sidecar states no `capture` at all".to_owned(),
            ],
        },
        "a scan asserting only that a committed set is clean goes green forever the day it stops \
         being able to look, so the same scan is driven over three directories committed to be \
         wrong in three different ways. **All three have to be told apart**, because they are \
         three different defects and a reader has to know which: a sidecar that went missing in a \
         rename, one whose field a format change dropped, and one carrying an id from the \
         revision before. The count of two is the two sidecars that exist to be opened, so a scan \
         that opened the wrong number is a failure of this reading as well"
    );
    Ok(())
}

/// What every directory under `root` says about the capture it holds.
///
/// Directories are walked in name order so a verdict reads the same way twice,
/// and a sidecar that cannot be opened is a **fault rather than an error**: a
/// missing record is one of the three things this reading exists to report, and
/// propagating it would stop the scan at the first one instead of naming them
/// all.
fn provenance_under(root: &Path) -> Result<Provenance, Box<dyn Error>> {
    let mut directories = entries_of(root)?;
    directories.sort();
    let mut sidecars_read = 0;
    let mut faults = Vec::new();
    for directory in directories {
        let Ok(stated) = fs::read_to_string(root.join(&directory).join(GOLDEN_SIDECAR)) else {
            faults.push(format!(
                "`{directory}` has no `{GOLDEN_SIDECAR}` beside its frame"
            ));
            continue;
        };
        sidecars_read += 1;
        let named: Value = serde_json::from_str(&stated)?;
        match named.get(CAPTURE_FIELD).and_then(Value::as_str) {
            Some(capture) if capture == directory => {}
            Some(capture) => faults.push(format!("`{directory}`'s sidecar names `{capture}`")),
            None => faults.push(format!(
                "`{directory}`'s sidecar states no `{CAPTURE_FIELD}` at all"
            )),
        }
    }
    Ok(if faults.is_empty() {
        Provenance::EverySidecarNamesItsOwnCapture { sidecars_read }
    } else {
        Provenance::Disagreeing {
            sidecars_read,
            faults,
        }
    })
}
