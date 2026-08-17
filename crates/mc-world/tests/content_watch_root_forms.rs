//! A saved declaration is content whatever form the content root was handed over
//! in.
//!
//! # The defect this exists for, and why 1 188 tests were green over it
//!
//! `notify` reports the paths the platform gives it, and on Windows those are
//! absolute. The relevance rule asks `path.strip_prefix(root.join("blocks"))`. So a
//! watcher constructed over a **relative** root is handed absolute paths, every
//! `strip_prefix` fails, and every save a player makes is classified as not being
//! content. The reload is then inert: nothing is ever begun, and no refusal is
//! printed either, because from the domain's point of view nothing changed.
//!
//! The shipped client hands over exactly that: `mc_sim::content::shipped_directory`
//! is `["content", "base"].iter().collect()` — relative — and it travels unchanged
//! to `watching_shipped_content`. **Every automated test over this path used a
//! `tempfile` directory**, which is absolute, so the two forms always agreed and the
//! suite could not see it. The owner found it by playing.
//!
//! This is `standards/global/testing.md`'s *policy is not wiring* one layer down:
//! the wiring landed and the thing it wires is a no-op. What hid it is a fixture
//! that supplied its input in a form no caller uses.
//!
//! # Three forms of one directory, and none of them is redundant
//!
//! The three are the same directory, watched three ways: absolute, relative to
//! wherever this binary was started, and the relative one spelled with a leading
//! `./`, forward slashes and a trailing separator — the way a path written by hand
//! in a configuration file or typed on a command line looks. **A later reader will
//! see three near-identical roots and want to delete two.** What each is for:
//!
//! - the absolute form is what every existing test uses, so it is the control that
//!   says the fixture works at all;
//! - the relative form is what the shipped client actually hands over;
//! - the dotted form is what says the repair is a rule about *paths* rather than a
//!   special case for one spelling.
//!
//! **A path's spelling is not the caller's to know.** A port whose contract holds
//! for one spelling of its argument has no contract; it has a habit.
//!
//! # What is deliberately not among the three
//!
//! The `\\?\` verbatim form `fs::canonicalize` produces on Windows. No caller in
//! this tree hands one over — the shipped root is relative and every fixture root
//! is a plain absolute path — so asserting on it would pin behaviour nothing in the
//! spec states, against a vendor that reports the plain form. It is also the form
//! the *wrong* repair produces, and this file rejects that repair by construction
//! instead: the root is handed to the **adapter** in each form, so absolutising it
//! at a call site changes nothing here.
//!
//! # A real filesystem, and how to read a failure
//!
//! This is the second test in this crate that touches one, for the same reason the
//! first does: the paths the platform reports are the thing under test and no double
//! can produce them. A save is given fifteen seconds to arrive; paths then go on
//! being collected for two settling windows, because one save is several events and
//! the first report is not necessarily the last.
//!
//! A form reporting **nothing at all** is the watcher not watching. A form reporting
//! paths the rule calls nothing is the defect above, and the paths are printed so
//! whoever reads the failure can see which spelling arrived.

mod common;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use common::{TestResult, repository_root};
use mc_world::content::watch::{
    ContentChanges, ContentWatch, NotifyContentWatch, declares_content,
};

/// Where a scratch root goes: under the repository's own build directory.
///
/// **Two properties, both required.** It is gitignored, so a directory left behind
/// by a killed run is not something anybody has to notice; and it is on the same
/// volume as this crate, which is what makes a relative spelling of it *exist* — a
/// system temporary directory is often on another drive, and no relative path
/// crosses volumes on Windows.
const SCRATCH: [&str; 2] = ["target", "content-root-forms"];

/// The declaration directory a watched root has to have before it is watched, so
/// that the only event a run produces is the save it makes.
const BLOCKS: &str = "blocks";

/// A directory one level below that one, which no loader reads.
///
/// The rule's own doc comment says a declaration nested one directory deeper is not
/// content, "because it is not something either loader reads" — and **nothing in the
/// suite witnessed that** until this run wrote a file there. Measured: a mutation
/// letting the rule walk every ancestor left ten candidate tests green, because the
/// shipped root holds no nested declaration and no fixture wrote one.
const NESTED: &str = "experiments";

/// A well-formed declaration, saved as an author would save one.
const DECLARATION: &str =
    "return {\n\tname = 'example:probe',\n\ttexture = 'example:probe',\n\tsolid = true,\n}\n";

/// How long a real save is given to reach the watch.
///
/// **A maximum, polled to rather than slept through**, and a test bound with a number
/// of its own — `SETTLING_WINDOW` is production policy about an editor's save, not a
/// statement about how slow this machine may be while the suite runs.
///
/// - *From below:* it has to exceed a platform watcher's own latency plus a debounce
///   under load. The sibling suite's real-filesystem test has used fifteen seconds
///   for the same question since it was written, and no run has ever spent it.
/// - *From above:* nothing about correctness. Every assertion made against this is a
///   **presence**, and the loop returns the moment a content path arrives — so the
///   bound is only ever spent by a run that was going to fail, which is the one
///   direction where waiting is free.
const WAITING_FOR_A_SAVE: Duration = Duration::from_secs(15);

/// How long a boundary waits before asking again.
const BETWEEN_ASKS: Duration = Duration::from_millis(5);

/// What a watch over one form of a root made of a saved declaration.
///
/// **A total verdict.** An assertion against the first arm rejects a watch that
/// reported nothing, one whose reports the rule calls nothing, and a root that could
/// not be watched at all — where `assert!(reported.iter().any(..))` inverted would
/// be a weak green over any of the three.
#[derive(Debug, PartialEq, Eq)]
enum Reported {
    /// The declaration directly under `blocks/` is content and the one nested a
    /// directory deeper is not — which is what the loaders read and what they do not.
    OnlyTheDeclarationADirectoryDeepIsContent,
    /// The rule called the nested declaration content as well.
    TheNestedDeclarationToo,
    /// Both paths arrived and the rule calls neither of them content. These are they.
    PathsTheRuleCallsNothing(Vec<String>),
    /// One or both paths never arrived inside the window a save is given, so nothing
    /// above could be said about a classification.
    ///
    /// **Both halves are decided on paths that demonstrably arrived**, which is what
    /// keeps the nested half from being an absence with a window: it is not "the
    /// nested path was never called content", it is "the nested path arrived and was
    /// not".
    NotBothPathsArrived { arrived: Vec<String> },
    /// The root could not be watched at all.
    CouldNotWatch { directory: String },
}

#[test]
fn a_saved_declaration_is_content_whatever_form_the_root_was_handed_over_in() -> TestResult {
    let scratch = Scratch::under(&repository_root()?)?;
    let forms = the_three_forms_of(scratch.path())?;

    let reported = [
        reported_over(&forms.absolute, scratch.path(), "probe-absolute.luau")?,
        reported_over(&forms.relative, scratch.path(), "probe-relative.luau")?,
        reported_over(&forms.dotted, scratch.path(), "probe-dotted.luau")?,
    ];

    assert_eq!(
        reported,
        [
            Reported::OnlyTheDeclarationADirectoryDeepIsContent,
            Reported::OnlyTheDeclarationADirectoryDeepIsContent,
            Reported::OnlyTheDeclarationADirectoryDeepIsContent
        ],
        "these are one directory watched three ways, and a saved declaration is a saved declaration \
         in all three. The shipped client hands over the relative form — `content/base`, assembled \
         from two components and never made absolute — so a rule that only works for the form a \
         `tempfile` fixture produces leaves hot reload doing nothing at all in the shipped game, \
         with no attempt begun and no refusal printed. The forms were: absolute `{absolute}`, \
         relative `{relative}`, dotted `{dotted}`",
        absolute = forms.absolute.display(),
        relative = forms.relative.display(),
        dotted = forms.dotted.display()
    );
    Ok(())
}

/// One directory, spelled three ways.
struct Forms {
    absolute: PathBuf,
    relative: PathBuf,
    dotted: PathBuf,
}

/// The three spellings of `directory`, each checked to name it.
///
/// **The check is the fixture's own guard and it is not optional**: a relative
/// spelling that resolved somewhere else would make every form below fail for a
/// reason that has nothing to do with the rule under test. It is done by asking the
/// filesystem, which is the only authority on whether two spellings are one
/// directory.
///
/// # Errors
///
/// Returns an error if no relative spelling exists — which is a scratch directory
/// on another volume than this crate — or if the three do not name one directory.
fn the_three_forms_of(directory: &Path) -> Result<Forms, Box<dyn Error>> {
    let absolute = directory.to_path_buf();
    let relative = relative_from(&std::env::current_dir()?, &absolute)?;
    let dotted = dotted_spelling(&relative);
    let named: Vec<PathBuf> = [&absolute, &relative, &dotted]
        .into_iter()
        .map(fs::canonicalize)
        .collect::<Result<_, _>>()?;
    let Some((first, beside)) = named.split_first() else {
        return Err("this fixture resolved none of the three spellings it was asked for".into());
    };
    if beside.iter().any(|found| found != first) {
        return Err(format!(
            "this fixture has to hand one directory over in three spellings, and the three name \
             different places: {named:?}. Every form below would then be refused for a reason about \
             the fixture rather than about the rule"
        )
        .into());
    }
    Ok(Forms {
        absolute,
        relative,
        dotted,
    })
}

/// `target` spelled relative to `from`.
///
/// **Never by changing the working directory.** A `set_current_dir` is
/// process-global and this suite runs in parallel, so a test that chdirs corrupts
/// its neighbours and the corruption reads as a flake. The relative spelling is
/// computed instead, from the two absolute paths.
///
/// # Errors
///
/// Returns an error when the two share no prefix at all, which on Windows is two
/// different volumes — there is no relative spelling then, and a fixture that
/// pretended otherwise would be testing a path that names nothing.
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

/// The same relative path with a leading `./`, forward slashes and a trailing
/// separator — the way a path written by hand looks.
fn dotted_spelling(relative: &Path) -> PathBuf {
    let forward = relative.display().to_string().replace('\\', "/");
    PathBuf::from(format!("./{forward}/"))
}

/// What a watch constructed over `form` made of two declarations saved into
/// `directory`: one where the loaders read, and one a directory deeper.
///
/// The saves are written through the directory's own path: what varies between runs is
/// the form the *watch* was given, never how the file was written. An author's editor
/// does not know what form the game was started with.
///
/// # Errors
///
/// Returns an error if either declaration cannot be written.
fn reported_over(form: &Path, directory: &Path, stem: &str) -> Result<Reported, Box<dyn Error>> {
    let declared = format!("{stem}.luau");
    let mut watching = NotifyContentWatch::watching(form);
    fs::write(directory.join(BLOCKS).join(&declared), DECLARATION)?;
    fs::write(
        directory.join(BLOCKS).join(NESTED).join(&declared),
        DECLARATION,
    )?;
    Ok(classified(&mut watching, form, &declared))
}

/// Everything `watching` reported, sorted by what the relevance rule makes of the two
/// saves.
///
/// **Polled until both saves have arrived**, and then classified — which is what keeps
/// the nested half from being an absence with a window of its own. What is asked is not
/// "was the nested path never called content" but "the nested path arrived, and was
/// it", so only paths that demonstrably arrived are judged.
///
/// One save is several filesystem events, so a run that classified the first report
/// would read a fraction of what arrived; and a run that collected for a fixed stretch
/// after it would turn a maximum into an equality against machine load.
fn classified(watching: &mut dyn ContentWatch, root: &Path, declared: &str) -> Reported {
    let started = Instant::now();
    let mut arrived: Vec<PathBuf> = Vec::new();
    while started.elapsed() < WAITING_FOR_A_SAVE {
        match watching.changes() {
            ContentChanges::Nothing => {}
            ContentChanges::Unwatchable { directory, .. } => {
                return Reported::CouldNotWatch {
                    directory: directory.display().to_string(),
                };
            }
            ContentChanges::Changed(paths) => arrived.extend(paths),
        }
        if both_saves_arrived(&arrived, declared) {
            break;
        }
        thread::sleep(BETWEEN_ASKS);
    }
    verdict_over(&arrived, root, declared)
}

/// Whether both of the run's saves are among the paths reported so far.
fn both_saves_arrived(arrived: &[PathBuf], declared: &str) -> bool {
    reported_inside(arrived, declared, BLOCKS).is_some()
        && reported_inside(arrived, declared, NESTED).is_some()
}

/// The reported path for `declared` sitting directly inside a directory called
/// `inside`, if one arrived.
///
/// **Matched on the two names rather than on a path this test built**, because the
/// vendor reports the caller's own spelling of the root with its own prefix on the
/// front — so a comparison against a path assembled here would answer no for reasons
/// that have nothing to do with what arrived.
fn reported_inside<'a>(
    arrived: &'a [PathBuf],
    declared: &str,
    inside: &str,
) -> Option<&'a PathBuf> {
    arrived.iter().find(|path| {
        path.file_name().and_then(OsStr::to_str) == Some(declared)
            && path
                .parent()
                .and_then(Path::file_name)
                .and_then(OsStr::to_str)
                == Some(inside)
    })
}

/// What a set of reported paths amounts to, over the two saves the run made.
fn verdict_over(arrived: &[PathBuf], root: &Path, declared: &str) -> Reported {
    let Some(direct) = reported_inside(arrived, declared, BLOCKS) else {
        return Reported::NotBothPathsArrived {
            arrived: spelled(arrived),
        };
    };
    let Some(nested) = reported_inside(arrived, declared, NESTED) else {
        return Reported::NotBothPathsArrived {
            arrived: spelled(arrived),
        };
    };
    match (
        calls_it_content(root, direct),
        calls_it_content(root, nested),
    ) {
        (true, false) => Reported::OnlyTheDeclarationADirectoryDeepIsContent,
        (_, true) => Reported::TheNestedDeclarationToo,
        (false, false) => Reported::PathsTheRuleCallsNothing(spelled(arrived)),
    }
}

/// Reported paths as a report reads them.
fn spelled(arrived: &[PathBuf]) -> Vec<String> {
    arrived
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

/// Whether the relevance rule calls `reported` a content declaration under `root`.
///
/// **The one place this file names the rule**, so that the repair — the adapter
/// reporting paths relative to the root it was constructed with, and the rule
/// becoming a predicate over a root-relative path — lands in one line here rather
/// than in three assertions.
fn calls_it_content(root: &Path, reported: &Path) -> bool {
    declares_content(root, reported)
}

/// A uniquely named directory under the repository's build directory, with the
/// declaration directory already in it, removed when this is dropped.
///
/// Removed on the way out **including on a panic**, which is what `Drop` buys over
/// a line at the end of the test: an assertion that fires is exactly when a
/// left-behind directory would accumulate.
struct Scratch {
    directory: PathBuf,
}

impl Scratch {
    /// A scratch root under `repository`, and its `blocks/` directory.
    ///
    /// The name carries the process id and a timestamp, so two runs of this suite —
    /// or two crates' suites at once — never share one.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    fn under(repository: &Path) -> Result<Self, Box<dyn Error>> {
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
        fs::create_dir_all(directory.join(BLOCKS).join(NESTED))?;
        Ok(Self { directory })
    }

    /// Where this root sits.
    fn path(&self) -> &Path {
        &self.directory
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // A scratch directory that outlives its run is litter rather than evidence:
        // everything a failure needs is in the assertion's own message.
        drop(fs::remove_dir_all(&self.directory));
    }
}
