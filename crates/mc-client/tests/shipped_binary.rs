//! The shipped executable, run as a real subprocess.
//!
//! Every other test of the reporting calls the library — it renders a failure, or
//! asks what `report` writes to a sink it was handed. All of that grades what the
//! library decides and nothing about whether the binary asks. **A `main` that
//! dropped its `report` call entirely, or wrote the refusal to standard output,
//! leaves every one of them green**, and so does the scan in
//! `tests/reporting_seam.rs`: nothing there composes a report, because nothing
//! there prints one either. `testing.md` §2 records the measured version of this
//! shape twice over — a client submitting a default intent every tick with 406 of
//! 406 tests passing, and `tools/voxforge/tests/binary.rs`'s own header, where a
//! `main` gutted to ignore its library left 123 of 125 green.
//!
//! So this runs `CARGO_BIN_EXE_mc-client`, the executable Cargo has just built,
//! and grades it through the process boundary: the real streams and the real exit
//! status.
//!
//! **Three of the four readings need no device and no display server**, which is
//! what makes them cheap enough to have. `run` asks for the content root first and
//! returns on that refusal before it spawns the preparation and before it opens a
//! device; and the changed-blocks *notice* is written from the preparation worker,
//! above any device, so a bounded read of the stream catches it either way.
//!
//! **The fourth needs one, and the reason is structural rather than incidental** —
//! see "The refusing side" below. It is called out here so nobody reads this file's
//! old promise of a device-free suite and takes the new reading for a mistake.
//!
//! # What it does not witness, and this must not be over-read
//!
//! It says nothing about the guidance a site supplies **on the content-root path**,
//! whose way out is empty by construction. A test that closes a real hole is exactly
//! when somebody is most tempted to read it as closing the one beside it.
//!
//! # The changed-blocks reading, and why it is a process rather than a call
//!
//! **`mc_client::notice::changed_blocks` returning the right `Option<String>`
//! while nothing calls its `say_changed_blocks` sibling leaves every other test of
//! that line green.** All of them — `tests/changed_blocks_named_on_the_error_stream.rs`
//! and `src/notice_test.rs` alike — reach the composer through a launch or
//! directly, which is agreement between two copies of one decision:
//! `prepare_launch` can stop saying it out loud and the pure function it
//! duplicates still answers correctly. That is `testing.md` §2's *policy is not
//! wiring*, and its measured instance lives in this same mechanism — 191 tests
//! green against a `RegistryVerdict::refuses` that ignored its argument. Nothing
//! short of a real process can see it, so **do not "simplify" the reading below
//! into a library call**: that is the trap it exists to escape.
//!
//! **The line is reachable here only because it is said before a device is
//! opened.** It is written from the preparation worker, which touches no GPU and
//! no window, while the main thread is still enumerating adapters. A launch that
//! *succeeds* goes on to open a window, so the child is killed the moment the line
//! arrives rather than waited out — that is why this reading spawns and reads
//! instead of calling `output()`. Emitting the line below the frame path's uploads,
//! where the clearing notice sits, would put it back out of reach.
//!
//! # The refusing side is the same hole with the sign flipped, and it needs a device
//!
//! `--refuse-changed-blocks` is graded everywhere else through `simulation_to_play`,
//! which is a library call: **the binary could stop honouring the argument entirely
//! and every one of those readings would stay green**, because none of them looks at
//! what the process does with its own `argv`. That is not a hypothesis — the
//! measured version of it is one paragraph up, where deleting the `say_*` call left
//! 1 384 of 1 385 green. It is also the highest-consequence path in this feature:
//! the argument exists because loading a doubtful world and quitting rewrites the
//! hashes that made it doubtful, so somebody who asks for the world to be left shut
//! and has it opened anyway loses the exact thing they were protecting.
//!
//! **Unlike the other two readings, that one needs a device, and it cannot not.** A
//! refusal over the *save* is discovered on the preparation worker and surfaces only
//! where the frame path collects it — `App::collect_preparation`, inside a redraw —
//! so the child has to reach a window before it can say anything about the save at
//! all. It ends itself once it does — a refused launch is a refused launch — so
//! nothing has to be killed on the passing path. Three consequences, all deliberate:
//!
//! - A machine with no device answers [`Answered::NeverGotAsFarAsReadingTheSave`],
//!   which is a **failing** verdict rather than a pass. A test that skipped quietly
//!   there would go on reporting nothing the day the argument stopped working.
//! - **It is still bounded, and `output()` is not good enough.** Measured: with the
//!   binary mutated to ignore its own `argv`, the child loads the world and its
//!   window never closes, so `output()` waited **606 seconds** and the run had to be
//!   killed by hand. Bounded, that same mutation answers
//!   [`Answered::LoadedAndNamedTheBlockAsANotice`] in about a second and names the
//!   defect. A hang gets blamed on the machine; a red names the mechanism.
//! - It is the only reading in this workspace that covers the way-out sentence
//!   reaching a real terminal, because `App::redraw` is the one production line that
//!   emits it. `docs/technical/testing.md` recorded that line as uncovered, and this
//!   is what changed it.

use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use mc_client::startup::PreparationError;
use mc_core::id::BlockName;
use mc_render::window::rendered;
use mc_sim::persistence::LaunchError;
use mc_world::persistence::LoadError;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// How long the child is given to say the line before the reading gives up.
///
/// Generous against its real cost — the child reads four block declarations and a
/// 65 KB save before it says anything — because this bound exists to turn a
/// missing line into a failure rather than to measure anything. A run that wedges
/// gets blamed on the machine; a run that fails names the defect.
const PATIENCE: Duration = Duration::from_secs(20);

/// The clause the line carries whatever blocks it names, and so what the reading
/// below waits for on the stream.
///
/// A fragment rather than the whole sentence: waiting for the exact text would
/// make a *wrong* line indistinguishable from no line at all, and the failure
/// message could then not show what was actually written.
///
/// **Short of the verb's ending on purpose.** The line reads `behaves` for one
/// block and `behave` for more than one, so a fragment carrying the `s` waits
/// forever on the plural line the run below actually produces — and a wait that
/// times out is reported as the client never printing.
const THE_CLAUSE: &str = "no longer behave";

/// The whole line the child has to write, for the save and content below.
///
/// Written out rather than composed from the client's own pieces, on
/// `notice_test.rs`'s rule: what a player reads is the artefact, and a test that
/// assembled it the way the client does would agree with the client about a
/// rewording neither of them noticed.
const NAMES_ALL_FOUR: &str = "mycraft: `base:dirt`, `base:grass`, `base:stone`, `base:water` no \
                              longer behave as they did when this world was saved, and it was \
                              loaded anyway";

/// The save the child is given, relative to the repository root: written before
/// this repository's blocks were Luau, and never regenerated.
const OLDER_SAVE: [&str; 5] = [
    "crates",
    "mc-world",
    "tests",
    "fixtures",
    "world_saved_against_the_toml_declarations.mcw",
];

/// Where the client reads its content and its save from, relative to the
/// directory it was started in.
const CONTENT: [&str; 2] = ["content", "base"];
const SAVE: [&str; 2] = ["saves", "world.mcw"];

/// The blocks the committed save and the shipped content disagree about,
/// ascending — which is every block the save holds, because the list a behaviour
/// fold goes over has grown since it was written.
const THE_CHANGED_BLOCKS: [&str; 4] = ["base:dirt", "base:grass", "base:stone", "base:water"];

/// What a player types to have such a save refused rather than loaded.
///
/// Spelled out rather than read from the client, because what is under test here is
/// whether the *process* honours this exact text — a reading that took the client's
/// own constant would agree with a binary that had quietly changed it.
const REFUSE_CHANGED_BLOCKS: &str = "--refuse-changed-blocks";

/// How a subprocess ended.
///
/// Three-valued rather than a boolean: a process killed by a signal carries no
/// status at all, and that must not read as "it refused".
#[derive(Debug, PartialEq, Eq)]
enum Exited {
    /// Successfully.
    Zero,
    /// With a failing status, whichever one — the mapping from ending to status
    /// is graded where it lives and is not this test's subject.
    NonZero,
    /// Carrying no status at all.
    WithoutACode,
}

#[test]
fn the_shipped_binary_started_away_from_its_content_says_why_on_its_error_stream() -> TestResult {
    let elsewhere = tempfile::tempdir()?;

    let finished = Command::new(env!("CARGO_BIN_EXE_mc-client"))
        .current_dir(elsewhere.path())
        .output()?;
    let said = String::from_utf8(finished.stderr)?;
    let printed = String::from_utf8(finished.stdout)?;

    // Built by refusing the same way the client does and rendering it through the
    // shipped renderer, never by pasting what a run was observed to say. The
    // looked-for directory is assembled from its two components, so it spells
    // itself the way this platform spells a path.
    let refusal = rendered(&PreparationError::NoContentRoot {
        root: ["content", "base"].iter().collect::<PathBuf>(),
    });
    let expected = format!("mycraft: {refusal}\n");

    assert_eq!(
        (
            said.as_str(),
            exit_of(&finished.status),
            printed.contains(&refusal),
            printed.is_empty()
        ),
        (expected.as_str(), Exited::NonZero, false, false),
        "the shipped binary has to reach the reporting, write the whole refusal and nothing else \
         to the stream a mod author reads refusals on, and end with a status a shell can act on. \
         A silent error stream is also what a binary that never reported produces, and a refusal \
         on standard output is one a person piping the client's output past a pager would lose. \
         What it printed was:\n{printed}"
    );
    Ok(())
}

#[test]
fn the_shipped_binary_over_a_save_whose_blocks_behave_differently_names_them_on_its_error_stream()
-> TestResult {
    let game = a_game_directory_holding_the_older_save()?;

    let (found, everything) = the_line_the_binary_wrote(game.path())?;

    assert_eq!(
        found.as_deref(),
        Some(NAMES_ALL_FOUR),
        "the built binary — not a library call — has to reach the saying and write the whole line \
         to the stream a player reads notices on. A client that composes the sentence and never \
         says it out loud is what this reading exists for, and every other test of that line stays \
         green through it. Nothing matching `{THE_CLAUSE}` inside {PATIENCE:?} is that failure. \
         What it wrote was:\n{}",
        everything.join("\n")
    );
    Ok(())
}

#[test]
fn neither_run_of_the_binary_touches_the_save_it_read() -> TestResult {
    let game = a_game_directory_holding_the_older_save()?;
    let save = game.path().join(SAVE.iter().collect::<PathBuf>());
    let before = fs::read(&save)?;

    let _ = the_line_the_binary_wrote(game.path())?;
    let after_the_killed_run = fs::read(&save)?;
    let _ = what_the_binary_said(game.path(), Some(REFUSE_CHANGED_BLOCKS))?;

    assert_eq!(
        (
            after_the_killed_run == before,
            fs::read(&save)? == before,
            lock_files_beside_the_save(game.path())?
        ),
        (true, true, Vec::<String>::new()),
        "**this is the property `{REFUSE_CHANGED_BLOCKS}` exists for**, so it is checked rather than \
         argued. Somebody asks for a doubtful world to be left shut precisely because opening it and \
         quitting rewrites the hashes that made it doubtful, and a refusing run that wrote anything \
         would destroy the evidence it was asked to preserve. The accepting run is here for a second \
         reason: it is killed mid-startup on every pass, and a test that could corrupt its own \
         fixture would make the *next* run fail for reasons nothing to do with it. Both are safe for \
         reasons a reader can check rather than take on trust — a save is opened read-only and holds \
         no lock (`opened_with_length` calls `File::open` and nothing else), and the only write is \
         `ending_after_saving`, which saves on a clean close alone and so is reached by neither a \
         killed child nor a refused launch"
    );
    Ok(())
}

/// What the child said about the save it was told to refuse.
///
/// **A total verdict rather than a search for a substring.** "It did not print the
/// notice" is satisfied by a process that never read the save at all, and by one
/// that could not open a device, and by one started in the wrong directory — so
/// each of those is a verdict of its own and only one of them passes. An absence
/// assertion here would go green forever the day the child stopped getting as far
/// as its save.
#[derive(Debug, PartialEq, Eq)]
enum Answered {
    /// The whole refusal, character for character, with the way out at the end of
    /// it — what a player who asked to be turned away reads.
    RefusedNamingTheBlockAndTheWayOut,
    /// It loaded the world and wrote the notice instead. **This is the failure this
    /// reading exists for**: the argument reached no decision, and no library-level
    /// test can see it.
    LoadedAndNamedTheBlockAsANotice,
    /// It was still running when the reading gave up on it.
    ///
    /// **Bounded rather than waited out, and this variant is why.** A child that
    /// honours the argument refuses and ends; one that ignores it opens a window and
    /// runs until somebody closes it. `Command::output()` waits for that forever —
    /// measured at 606 seconds under exactly the mutation this reading exists for —
    /// and a wedged run reports nothing about which mechanism broke, while a red one
    /// names it.
    NeverEndedOnItsOwn,
    /// It never named the save, so whatever it refused happened before the save was
    /// read — no content root, an unbuilt texture set, or no device on this machine.
    NeverGotAsFarAsReadingTheSave,
    /// It named the save and said something other than the refusal above.
    SaidSomethingElseAboutTheSave,
}

#[test]
fn the_shipped_binary_told_to_refuse_a_changed_save_leaves_it_shut_and_says_why() -> TestResult {
    let game = a_game_directory_holding_the_older_save()?;

    let said = what_the_binary_said(game.path(), Some(REFUSE_CHANGED_BLOCKS))?;

    assert_eq!(
        answered_by(&said, &the_refusal_a_shut_world_reads()?),
        Answered::RefusedNamingTheBlockAndTheWayOut,
        "a player who passed `{REFUSE_CHANGED_BLOCKS}` asked for a world whose blocks have moved to \
         be left shut, and this is the only reading in the workspace that grades whether the \
         *process* honoured them. It has to name {THE_CHANGED_BLOCKS:?}, end with the argument to \
         drop, and exit with a status a shell can act on. `LoadedAndNamedTheBlockAsANotice` is the \
         argument reaching no decision, which no library-level reading can see; \
         `NeverGotAsFarAsReadingTheSave` on a machine with no device is this reading being unable \
         to look rather than finding nothing. What it wrote was:\n{}",
        said.text
    );
    Ok(())
}

/// Which of the five `said` is, judged against `expected`.
///
/// The order of the questions is the order of the answers, and the first one is the
/// failure this reading is for: a child that wrote the *notice* loaded a world it was
/// told to leave shut, whatever else its stream says or fails to say. How it ended is
/// folded in here rather than asserted beside it, so one enumerated verdict covers
/// the whole answer and no other verdict can pass for it.
fn answered_by(said: &Said, expected: &str) -> Answered {
    if said.text.contains(THE_CLAUSE) {
        return Answered::LoadedAndNamedTheBlockAsANotice;
    }
    match said.ended {
        Ended::HadToBeKilled => return Answered::NeverEndedOnItsOwn,
        Ended::OnItsOwn(Exited::NonZero) => {}
        Ended::OnItsOwn(_) => return Answered::SaidSomethingElseAboutTheSave,
    }
    let save: PathBuf = SAVE.iter().collect();
    if !said.text.contains(&save.display().to_string()) {
        return Answered::NeverGotAsFarAsReadingTheSave;
    }
    if said.text == expected {
        return Answered::RefusedNamingTheBlockAndTheWayOut;
    }
    Answered::SaidSomethingElseAboutTheSave
}

/// Everything a run of the binary wrote to its error stream, and how it ended.
#[derive(Debug)]
struct Said {
    text: String,
    ended: Ended,
}

/// Whether a child finished inside the reading's patience.
#[derive(Debug, PartialEq, Eq)]
enum Ended {
    /// It ended by itself, with this status.
    OnItsOwn(Exited),
    /// It was still running when the deadline passed, and was killed.
    HadToBeKilled,
}

/// Runs the built client in `game`, optionally with `argument`, and reads its error
/// stream until the child ends or [`PATIENCE`] runs out.
///
/// **Bounded rather than `output()`.** A child that ignores its argument never ends,
/// and a wedged gate reports nothing about which mechanism broke.
///
/// # Errors
///
/// Returns an error if the child cannot be spawned, its error stream cannot be taken,
/// or it cannot be waited on.
fn what_the_binary_said(game: &Path, argument: Option<&str>) -> Result<Said, Box<dyn Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mc-client"));
    command
        .current_dir(game)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(argument) = argument {
        command.arg(argument);
    }
    let mut child = command.spawn()?;
    let lines = lines_of(&mut child)?;
    let (everything, ended_on_its_own) = until_it_stopped(&lines);
    let ended = if ended_on_its_own {
        Ended::OnItsOwn(exit_of(&child.wait()?))
    } else {
        drop(child.kill());
        drop(child.wait());
        Ended::HadToBeKilled
    };
    Ok(Said {
        text: line_by_line(&everything),
        ended,
    })
}

/// Every line the child wrote, and whether its stream ended before the deadline.
///
/// A stream that ends is a child that has exited or closed it; a deadline that passes
/// is a child still running. Those are different answers and the caller needs both.
fn until_it_stopped(lines: &Receiver<String>) -> (Vec<String>, bool) {
    let deadline = Instant::now() + PATIENCE;
    let mut everything = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match lines.recv_timeout(left) {
            Ok(line) => everything.push(line),
            Err(RecvTimeoutError::Disconnected) => return (everything, true),
            Err(RecvTimeoutError::Timeout) => return (everything, false),
        }
    }
}

/// The lines put back together the way the child wrote them, so a whole-stream
/// comparison is a comparison of what a person reads.
fn line_by_line(everything: &[String]) -> String {
    everything
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<Vec<_>>()
        .concat()
}

/// The whole line the child has to write, built by refusing the way the client
/// refuses and rendering it through the shipped reporting.
///
/// **Rendered rather than pasted**, which is the rule the content-root reading above
/// follows: a refusal written out here would be a second copy of the client's own
/// decision about what to say, and the two could disagree with nothing to notice.
/// The way out is appended by asking the failure for it, exactly as `App::redraw`
/// does, so a `way_out` that stopped supplying the sentence moves this expectation
/// and the child's output together. The save is named the way the client names it —
/// the relative path it reads, spelled by whichever platform is running.
///
/// # Errors
///
/// Returns an error if a changed block's name is not a namespaced id.
fn the_refusal_a_shut_world_reads() -> Result<String, Box<dyn Error>> {
    let refused = PreparationError::Launch(LaunchError::Load {
        save: SAVE.iter().collect(),
        source: Box::new(LoadError::Unresolvable {
            missing: Vec::new(),
            changed: THE_CHANGED_BLOCKS
                .iter()
                .map(|name| BlockName::parse(name))
                .collect::<Result<Vec<_>, _>>()?,
        }),
    });
    Ok(format!(
        "mycraft: {rendered}{way_out}
",
        rendered = rendered(&refused),
        way_out = refused.way_out()
    ))
}

/// A directory laid out the way a person's game directory is: the shipped content
/// under `content/base`, and the committed pre-Luau save where the client looks
/// for one.
///
/// The content is **copied** rather than linked, and the copy stays current
/// because the built set is judged by a fold over its sources rather than by
/// timestamps. The save is copied rather than written, because a save this suite
/// wrote against the declarations under test would agree with them by
/// construction and could report nothing.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located or the copy fails.
fn a_game_directory_holding_the_older_save() -> Result<TempDir, Box<dyn Error>> {
    let game = TempDir::new()?;
    let repository = repository_root()?;
    copy_tree(
        &repository.join(CONTENT.iter().collect::<PathBuf>()),
        &game.path().join(CONTENT.iter().collect::<PathBuf>()),
    )?;
    let save = game.path().join(SAVE.iter().collect::<PathBuf>());
    fs::create_dir_all(
        save.parent()
            .ok_or("the save path has no directory above it")?,
    )?;
    fs::copy(
        repository.join(OLDER_SAVE.iter().collect::<PathBuf>()),
        &save,
    )?;
    Ok(game)
}

/// Runs the built client in `game` and reads its error stream until it says the
/// line, then kills it.
///
/// Hands back what it was looking for and **everything it read on the way**, so a
/// run that never said it fails with the child's own output in the message rather
/// than with an empty absence.
///
/// # Errors
///
/// Returns an error if the child cannot be spawned or its error stream cannot be
/// taken.
fn the_line_the_binary_wrote(game: &Path) -> Result<(Option<String>, Vec<String>), Box<dyn Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mc-client"))
        .current_dir(game)
        // Never read, so never piped: a pipe nobody drains is a child that blocks
        // once it fills, which would be this reading's own hang.
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let lines = lines_of(&mut child)?;
    let read = until_it_said(&lines, THE_CLAUSE);
    // Killed rather than waited out: this launch succeeds, and a client whose
    // launch succeeds goes on to open a window and take the pointer. Both answers
    // are dropped deliberately — a child that has already gone is the outcome this
    // asks for, and a failure to reap it says nothing about the line.
    drop(child.kill());
    drop(child.wait());
    Ok(read)
}

/// The child's error stream, one line at a time, off a thread so that reading it
/// can be given up on.
///
/// # Errors
///
/// Returns an error if the stream was already taken.
fn lines_of(child: &mut Child) -> Result<Receiver<String>, Box<dyn Error>> {
    let stream = child
        .stderr
        .take()
        .ok_or("the child was spawned without an error stream to read")?;
    let (send, receive) = mpsc::channel();
    // Stops of its own accord when the reader hangs up, which is what the `take_while`
    // is: a send into a dropped receiver is the end of this thread's work.
    std::thread::spawn(move || {
        BufReader::new(stream)
            .lines()
            .map_while(Result::ok)
            .take_while(|line| send.send(line.clone()).is_ok())
            .for_each(drop);
    });
    Ok(receive)
}

/// Every line read until one contains `clause`, and that line where there was
/// one.
///
/// Bounded by [`PATIENCE`] rather than by the stream ending, because the stream
/// does not end: the child is still running when the line arrives.
fn until_it_said(lines: &Receiver<String>, clause: &str) -> (Option<String>, Vec<String>) {
    let deadline = Instant::now() + PATIENCE;
    let mut everything = Vec::new();
    let mut found = None;
    while found.is_none() {
        let left = deadline.saturating_duration_since(Instant::now());
        let Ok(line) = lines.recv_timeout(left) else {
            // Both ways of running out are the same answer: the line was not said.
            // A stream that ended is a child that exited without saying it, and a
            // deadline that passed is one still running and still silent.
            return (None, everything);
        };
        found = line.contains(clause).then(|| line.clone());
        everything.push(line);
    }
    (found, everything)
}

/// Whatever sits beside the save that is not the save itself.
///
/// A save is a plain file read with `File::open`, so this is expected to be
/// empty; it is asked rather than assumed because a lock left behind is exactly
/// the kind of damage that shows up as an unrelated failure one run later.
///
/// # Errors
///
/// Returns an error if the directory cannot be read.
fn lock_files_beside_the_save(game: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(game.join(SAVE[0]))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name != SAVE[1] {
            found.push(name);
        }
    }
    found.sort();
    Ok(found)
}

/// The repository's own root, located upwards from this crate.
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

/// Copies every file and directory under `from` into `into`.
///
/// # Errors
///
/// Returns an error if a directory cannot be created or a file cannot be copied.
fn copy_tree(from: &Path, into: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(into)?;
    for entry in fs::read_dir(from)? {
        let source = entry?.path();
        let Some(name) = source.file_name() else {
            continue;
        };
        let destination = into.join(name);
        if source.is_dir() {
            copy_tree(&source, &destination)?;
        } else {
            fs::copy(&source, &destination)?;
        }
    }
    Ok(())
}

/// How `status` ended, without pinning which failing code it chose.
fn exit_of(status: &ExitStatus) -> Exited {
    match status.code() {
        Some(0) => Exited::Zero,
        Some(_) => Exited::NonZero,
        None => Exited::WithoutACode,
    }
}
