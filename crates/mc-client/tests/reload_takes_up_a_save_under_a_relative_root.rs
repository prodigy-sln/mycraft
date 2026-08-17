//! A save under a content root given as a relative path reaches the running
//! client, exactly as one under an absolute path does.
//!
//! # This is the owner's session, written down
//!
//! The shipped client hands `watching_shipped_content` a **relative** root —
//! `mc_sim::content::shipped_directory` is `["content", "base"].iter().collect()`,
//! and it travels through `PreparedLaunch::root` unchanged. Every automated test
//! over this path, this suite's own real-filesystem one included, used a `tempfile`
//! directory instead, which is absolute. So the form the game actually runs in was
//! the one form nothing exercised, and hot reload was inert in the shipped client
//! while 1 188 tests were green: the vendor reports absolute paths, the relevance
//! rule strips the root as given, every save is classified as not content, and no
//! attempt is begun and no refusal printed.
//!
//! `crates/mc-world/tests/content_watch_root_forms.rs` is the same defect at the
//! port, over three spellings of one directory. This is it at the shipped shape:
//! two clients, one watching a root the way a `tempfile` fixture spells it and one
//! watching it the way the game does, and **the difference between them is the whole
//! assertion.**
//!
//! # Both halves are asserted, and neither is an absence
//!
//! What is compared is the attempt list *and* what the swap changed — stone's
//! declared solidity, read out of the content the client is publishing. A run that
//! reported an attempt and swapped nothing satisfies the first half alone;
//! `assert!(!attempts.is_empty())` satisfies neither honestly.
//!
//! # Two directories, and that is not an accident
//!
//! Each form gets its own copy of the shipped content, because the save this makes
//! *is* the edit under test: a second run over a directory whose `stone.luau` had
//! already been rewritten would read stone as non-solid before any reload happened,
//! and the comparison would be green over a client that never noticed anything.
//!
//! # A real filesystem, and what a failure costs
//!
//! A save is given fifteen seconds, the bound this crate's other real-filesystem
//! test already uses. While the defect stands, the relative half spends all fifteen
//! of them — that is the shape of the red rather than a hang, and the assertion is
//! what says so. Once the repair lands both halves finish in well under a second.
//!
//! # Why the scratch root is under `target/`
//!
//! A relative spelling has to *exist*, and on Windows no relative path crosses
//! volumes — a system temporary directory is routinely on another drive from the
//! repository. Under the repository's own build directory it is on the same volume
//! and gitignored. **The working directory is never changed to get there**:
//! `set_current_dir` is process-global and this suite runs in parallel, so a chdir
//! corrupts its neighbours and the corruption reads as a flake. The relative
//! spelling is computed from the two absolute paths instead.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use mc_sim::reload::watching_shipped_content;

use input::InputHarness;
use reload::{GRASS, STONE, STONE_FILE, stone_that_is_not_solid};
use reload_watch::{
    AN_ATTEMPT_MAY_NOT_OUTLAST, Attempt, boundary, ended, solidity_of, taken_up_once,
};
use reload_world::{floor_of, playing, standing};
use support::TestResult;
use support::content::BLOCK_DIRECTORY;

/// Where a scratch content root goes, below the repository root.
const SCRATCH: [&str; 2] = ["target", "reload-root-forms"];

/// The least a real save is given to reach the client.
///
/// **A floor under a bound the run derives for itself**, not the whole of it: see
/// [`patience_after`]. Fifteen seconds is what this suite's other real-filesystem test
/// has always given a save, and no run of it has spent that.
const AT_LEAST_FOR_A_SAVE: Duration = Duration::from_secs(15);

/// How many times the control's own time-to-report the second half is given.
///
/// **The bound is derived from this machine, in this run, and that is the point.** A
/// literal was measured wrong here once: fifteen seconds was enough alone (the whole
/// test takes about 3.5 s) and not enough inside a full concurrent run, where one half
/// spent all fifteen. The absolute half does *the same work at the same moment* — a
/// real watcher, a real debounce, a real build — so how long it took is the only honest
/// scale for how long the relative half should be given.
///
/// - *From below:* twenty times a control that itself carries the machine's load.
/// - *From above:* nothing. A passing run returns at its first report and never spends
///   this; under the defect this exists to catch, **no** patience produces an attempt,
///   because the save is classified as not content forever. So generosity cannot green
///   anything.
const TIMES_THE_CONTROL_TOOK: u32 = 20;

/// How long a boundary waits for the next one.
const BETWEEN_BOUNDARIES: Duration = Duration::from_millis(5);

/// What one run made of the save it wrote, and how long it waited for it.
///
/// The timing is kept out of [`TookUp`] so the two halves stay comparable as whole
/// values: what is asserted is what each run made of its save, and a duration would
/// make every comparison a timing assertion.
struct Run {
    took_up: TookUp,
    reported_after: Option<Duration>,
}

/// What one run made of the save it wrote.
#[derive(Debug, PartialEq, Eq)]
struct TookUp {
    /// Every attempt that ended, in order.
    attempts: Vec<Attempt>,
    /// What the content now serving says about stone's solidity, or nothing where it
    /// no longer declares stone at all.
    stone_is_solid: Option<bool>,
}

#[test]
fn a_save_under_a_relative_content_root_is_taken_up_exactly_as_one_under_an_absolute_root()
-> TestResult {
    let repository = support::repository_root()?;
    let for_absolute = Scratch::holding_the_shipped_content(&repository)?;
    let for_relative = Scratch::holding_the_shipped_content(&repository)?;
    let relative = relative_from(&std::env::current_dir()?, for_relative.path())?;
    require_one_directory(for_relative.path(), &relative)?;

    let control = took_up(
        for_absolute.path(),
        for_absolute.path(),
        AT_LEAST_FOR_A_SAVE,
    )?;
    let watched = took_up(&relative, for_relative.path(), patience_after(&control))?;

    assert_eq!(
        [control.took_up, watched.took_up],
        [a_save_taken_up(), a_save_taken_up()],
        "this is the capability, and the second half of the comparison is the shipped client: the \
         root a player's game watches is `content/base`, relative, and the root every fixture \
         watched was absolute. A watch given the relative form is handed absolute paths by the \
         platform, the relevance rule strips a prefix that does not match, and the save is \
         classified as not being content — so nothing is begun, nothing is refused, and the author \
         is told nothing at all. The absolute half is the control that says this fixture works. The \
         relative root was `{relative}`",
        relative = relative.display()
    );
    Ok(())
}

/// One run's worth of state, for a client watching `form` and playing the content at
/// `root`.
///
/// **The client plays through the directory's own path and watches through `form`.**
/// What varies is the form the *watch* was constructed with; how the content was
/// loaded and how the editor wrote the file are not the subject, and an author's
/// editor does not know what form the game was started with.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, if the
/// save cannot be written, or if the client publishes no content to read stone out
/// of.
fn took_up(form: &Path, root: &Path, patience: Duration) -> Result<Run, Box<dyn Error>> {
    let (simulation, holding) = playing(root, standing(), |registry| floor_of(registry, GRASS))?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client.attach_reload(watching_shipped_content(form.to_owned()));

    fs::write(
        root.join(BLOCK_DIRECTORY).join(STONE_FILE),
        stone_that_is_not_solid().text(),
    )?;
    let started = Instant::now();
    let crossed = crossing_at_a_human_pace(&mut client, patience);
    let attempts = ended(&crossed);

    Ok(Run {
        reported_after: (!attempts.is_empty()).then(|| started.elapsed()),
        took_up: TookUp {
            attempts,
            stone_is_solid: solidity_of(&client, STONE)?,
        },
    })
}

/// How long the second half is given, derived from what the first half took.
///
/// The floor applies when the control was quick, and when it reported nothing at all —
/// in which case the assertion is about the control and this number does not matter.
fn patience_after(control: &Run) -> Duration {
    control.reported_after.map_or(AT_LEAST_FOR_A_SAVE, |took| {
        (took * TIMES_THE_CONTROL_TOOK).max(AT_LEAST_FOR_A_SAVE)
    })
}

/// One save, taken up, with the edit it carried now serving.
fn a_save_taken_up() -> TookUp {
    TookUp {
        attempts: taken_up_once(),
        stone_is_solid: Some(false),
    }
}

/// Boundaries crossed at a human pace until an attempt has ended and the run has
/// gone on long enough to see a second one.
fn crossing_at_a_human_pace(client: &mut InputHarness, patience: Duration) -> Vec<Option<Attempt>> {
    let started = Instant::now();
    let mut crossed = Vec::new();
    let mut reported = None;
    while waiting(started, reported, patience) {
        let attempt = boundary(client);
        if attempt.is_some() {
            reported = reported.or_else(|| Some(Instant::now()));
        }
        crossed.push(attempt);
        thread::sleep(BETWEEN_BOUNDARIES);
    }
    crossed
}

/// Whether a run that started at `started` and first reported at `reported` has more
/// to wait for.
fn waiting(started: Instant, reported: Option<Instant>, patience: Duration) -> bool {
    match reported {
        None => started.elapsed() < patience,
        Some(first) => first.elapsed() < AN_ATTEMPT_MAY_NOT_OUTLAST,
    }
}

/// `target` spelled relative to `from`, computed rather than reached by chdir.
///
/// # Errors
///
/// Returns an error when the two share no prefix, which on Windows is two volumes:
/// there is no relative spelling then.
fn relative_from(from: &Path, target: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let shared = from
        .components()
        .zip(target.components())
        .take_while(|(here, there)| here == there)
        .count();
    if shared == 0 {
        return Err(format!(
            "no relative spelling of {target} exists from {from}: they share no prefix, which is \
             two volumes. This fixture needs its scratch root on the same volume as the crate",
            target = target.display(),
            from = from.display()
        )
        .into());
    }
    let mut spelled = PathBuf::new();
    for _ in 0..from.components().count() - shared {
        spelled.push("..");
    }
    spelled.extend(target.components().skip(shared));
    Ok(spelled)
}

/// Refuses unless two spellings name one directory.
///
/// The fixture's own guard: a relative spelling that resolved somewhere else would
/// fail the comparison for a reason that has nothing to do with the client, and the
/// only authority on whether two spellings are one directory is the filesystem.
///
/// # Errors
///
/// Returns an error if either spelling cannot be resolved, or if they differ.
fn require_one_directory(absolute: &Path, relative: &Path) -> Result<(), Box<dyn Error>> {
    let named = fs::canonicalize(absolute)?;
    let spelled = fs::canonicalize(relative)?;
    if named != spelled {
        return Err(format!(
            "this fixture has to watch the very directory it is saving into, and the relative \
             spelling {spelled:?} names something other than {named:?}"
        )
        .into());
    }
    Ok(())
}

/// A uniquely named directory under the repository's build directory holding a copy
/// of the shipped content, removed when this is dropped.
///
/// Removed on a panic too, which is what `Drop` buys over a line at the end: an
/// assertion that fires is exactly when a left-behind tree would accumulate.
struct Scratch {
    directory: PathBuf,
}

impl Scratch {
    /// A scratch root under `repository`, holding what the repository ships as
    /// content.
    ///
    /// **A copy and never the shipped root itself**: this fixture saves a rewritten
    /// `stone.luau`, and the shipped declarations are what every golden frame and
    /// every layer expectation in this spec rests on.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or the content cannot be
    /// copied.
    fn holding_the_shipped_content(repository: &Path) -> Result<Self, Box<dyn Error>> {
        let stamped = format!(
            "{pid}-{nanos}",
            pid = std::process::id(),
            nanos = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_nanos()
        );
        let directory = SCRATCH
            .iter()
            .fold(repository.to_path_buf(), |below, part| below.join(part))
            .join(stamped);
        fs::create_dir_all(&directory)?;
        copy_tree(&support::content_root()?, &directory)?;
        Ok(Self { directory })
    }

    /// Where this root sits.
    fn path(&self) -> &Path {
        &self.directory
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.directory));
    }
}

/// Copies everything under `from` into `into`, directories included.
///
/// # Errors
///
/// Returns an error if a directory cannot be read or created, or a file cannot be
/// copied.
fn copy_tree(from: &Path, into: &Path) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(from)? {
        let path = entry?.path();
        let Some(name) = path.file_name() else {
            continue;
        };
        let beside = into.join(name);
        if path.is_dir() {
            fs::create_dir_all(&beside)?;
            copy_tree(&path, &beside)?;
        } else {
            fs::copy(&path, &beside)?;
        }
    }
    Ok(())
}
