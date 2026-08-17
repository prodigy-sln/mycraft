//! Taking up a content set that was read while the simulation was running.
//!
//! A candidate is a content set built from a root and not yet accepted. This is
//! the door a driver offers one through, and the vocabulary an offer is turned
//! away in.
//!
//! **Two stages refuse, and the split is forced by what each can see.** The
//! build stage reads the root and needs no vocabulary of its own — everything
//! wrong with the *content* already arrives as a
//! [`ContentError`]. The admission stage runs
//! against the running world and refuses the two things only the world knows.
//!
//! A refusal leaves the world untouched by construction: the build stage never
//! touches the simulation, and the admission stage settles everything fallible
//! before the swap.

use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

use thiserror::Error;

use mc_core::content::LayerAssignment;
use mc_core::id::BlockName;
use mc_world::content::watch::{
    ContentChanges, ContentWatch, NotifyContentWatch, declares_content,
};

use crate::content::{ContentError, LoadedContent};
use crate::simulation::{Accepted, Simulation};

/// Why a candidate was turned away.
///
/// **Deliberately not [`PartialEq`]**: [`Content`](Self::Content) carries an
/// error chain that has no equality to derive from.
#[derive(Debug, Error)]
pub enum ReloadRefusal {
    /// Everything the content root itself was refused for — a chunk that will
    /// not compile, a misspelled field, two files claiming one name, an emptied
    /// declaration directory, a refused HUD declaration, a declaration that
    /// looped past the budget or allocated past the memory cap, and a candidate
    /// needing more layers than the session has.
    ///
    /// **No wording of its own beyond naming the stage.** Every one of those
    /// already arrives as a `ContentError` over a fault that names the file, the
    /// block and the field, and the same refusals a launch produces are the ones
    /// a reload produces.
    #[error("the content root could not be read")]
    Content(#[from] ContentError),

    /// The root could not be watched at all.
    ///
    /// Not a build refusal — nothing was read — but it travels here so that
    /// "reported once" comes from the same deduplication rather than a second
    /// reporting channel.
    #[error(
        "the content root {directory} cannot be watched, so edits to it will not \
         be noticed: {cause}",
        directory = directory.display()
    )]
    RootUnwatchable { directory: PathBuf, cause: String },

    /// The thread building the candidate ended without producing one.
    #[error("the thread building the candidate ended without producing one or a refusal")]
    BuilderLost,
    /// Cells of the running world hold blocks this content does not declare.
    ///
    /// Every one of them, ascending. Nothing can go in a cell whose block no
    /// longer exists, and that is not a judgement to make on the author's
    /// behalf — the save path already refuses a missing name for the same
    /// reason.
    #[error(
        "the world holds {held} that this content does not declare",
        held = named(blocks)
    )]
    BlocksTheWorldHolds { blocks: Vec<BlockName> },

    /// The content registers no solid block at all.
    ///
    /// The sentence is `PreparationError::NothingToPlace`'s, unchanged to the
    /// byte: two wordings are two places for one decision to disagree.
    #[error(
        "the content registers no solid block, so a player would have nothing to place; the \
         block a client holds is the first solid one in registration order"
    )]
    NothingToPlace,
}

/// Takes up `candidate` as the content `simulation` now serves, at the boundary
/// between two ticks.
///
/// Call it after the tick it follows has been published: a tick answers every
/// question it asks from one content set.
///
/// # Errors
///
/// Returns [`ReloadRefusal`] with the simulation exactly as it was.
pub fn adopt_at_tick_boundary(
    simulation: &mut Simulation,
    candidate: LoadedContent,
) -> Result<Accepted, ReloadRefusal> {
    simulation.adopt(candidate)
}

/// How a candidate is built from a root and the layers a session has spent.
///
/// A function rather than a closure so [`ContentReload`] stays `Debug` and
/// `Send`; the shipped one is [`crate::content::load`].
pub type CandidateBuild = fn(&Path, &LayerAssignment) -> Result<LoadedContent, ContentError>;

/// What one tick boundary did about the content root.
#[must_use]
#[derive(Debug)]
pub enum ReloadStep {
    Nothing,
    /// Emitted only when it differs from the last refusal reported.
    Refused(ReloadRefusal),
    Accepted(Accepted),
}

/// Watching a content root, and taking up what it declares when it changes.
///
/// **The coalescing is a single flag and the order is the whole of it.** On a
/// relevant change the flag is set; at a boundary with nothing in flight and the
/// flag set it is *cleared and then* the build starts, so a change arriving during
/// a build sets it again and the boundary after that build starts exactly one
/// further attempt. A queue would run one build per save and publish one serial
/// per save for a single edit; refusing would drop an edit silently.
#[derive(Debug)]
pub struct ContentReload {
    root: PathBuf,
    watch: Box<dyn ContentWatch>,
    build: CandidateBuild,
    /// A change has been seen and no attempt has begun for it yet.
    pending: bool,
    in_flight: Option<JoinHandle<Result<LoadedContent, ContentError>>>,
    /// The last refusal reported, rendered whole. A recurring refusal is stated
    /// once however many attempts meet it.
    reported: Option<String>,
}

impl ContentReload {
    /// Watches `root` through `watch`, building candidates through the one
    /// content door.
    #[must_use]
    pub fn watching(root: PathBuf, watch: Box<dyn ContentWatch>) -> Self {
        Self::building(root, watch, crate::content::load)
    }

    /// The same, building candidates through `build` instead.
    ///
    /// Exists so a builder that does not survive is reachable at all: a thread
    /// that dies is not a state writing files can produce.
    #[must_use]
    pub fn building(root: PathBuf, watch: Box<dyn ContentWatch>, build: CandidateBuild) -> Self {
        Self {
            root,
            watch,
            build,
            pending: false,
            in_flight: None,
            reported: None,
        }
    }

    /// Asks the watch what changed, collects a finished build, and starts one if
    /// this boundary is where one should start.
    ///
    /// Called once per tick, **after** the tick has been advanced.
    pub fn at_tick_boundary(&mut self, simulation: &mut Simulation) -> ReloadStep {
        if let Some(unwatchable) = self.noticed() {
            return self.reporting(unwatchable);
        }
        let step = self.collected(simulation);
        self.begin_a_build(simulation);
        step
    }

    /// Records whether anything relevant changed, and reports a root that cannot
    /// be watched at all.
    fn noticed(&mut self) -> Option<ReloadRefusal> {
        match self.watch.changes() {
            ContentChanges::Nothing => None,
            ContentChanges::Changed(paths) => {
                self.pending |= paths.iter().any(|path| declares_content(&self.root, path));
                None
            }
            ContentChanges::Unwatchable { directory, cause } => {
                Some(ReloadRefusal::RootUnwatchable { directory, cause })
            }
        }
    }

    /// Whatever a finished build came to, or nothing while one is still running.
    fn collected(&mut self, simulation: &mut Simulation) -> ReloadStep {
        if !self.in_flight.as_ref().is_some_and(JoinHandle::is_finished) {
            return ReloadStep::Nothing;
        }
        let Some(finished) = self.in_flight.take() else {
            return ReloadStep::Nothing;
        };
        match finished.join() {
            Err(_) => self.reporting(ReloadRefusal::BuilderLost),
            Ok(Err(refused)) => self.reporting(refused.into()),
            Ok(Ok(candidate)) => match simulation.adopt(candidate) {
                Ok(accepted) => {
                    self.reported = None;
                    ReloadStep::Accepted(accepted)
                }
                Err(refused) => self.reporting(refused),
            },
        }
    }

    /// Starts one build where this boundary is where one should start.
    ///
    /// **Cleared before the build starts**, so a change arriving while it runs is
    /// remembered rather than absorbed. Nothing begins before a simulation has
    /// published a tick: a swap *is* a tick boundary, and there is no boundary
    /// yet.
    fn begin_a_build(&mut self, simulation: &Simulation) {
        if !self.pending || self.in_flight.is_some() {
            return;
        }
        self.pending = false;
        let root = self.root.clone();
        let spent = simulation.content().resolved.layers().clone();
        let build = self.build;
        self.in_flight = Some(std::thread::spawn(move || build(&root, &spent)));
    }

    /// `refused` as a step, unless the last one reported said the same thing.
    ///
    /// The comparison is over the **whole rendered chain** and not the top
    /// sentence: every content refusal shares its outer two layers, so comparing
    /// those would report the first broken file of a session and go silent for
    /// every later one.
    fn reporting(&mut self, refused: ReloadRefusal) -> ReloadStep {
        let said = rendered(&refused);
        if self.reported.as_ref() == Some(&said) {
            return ReloadStep::Nothing;
        }
        self.reported = Some(said);
        ReloadStep::Refused(refused)
    }
}

/// A failure and everything beneath it, as one string.
///
/// `mc-sim` may not name the renderer, so the walk is here rather than borrowed
/// from the one place a client renders a failure for a person.
fn rendered(failure: &dyn std::error::Error) -> String {
    let mut said = failure.to_string();
    let mut beneath = failure.source();
    while let Some(cause) = beneath {
        said.push_str(": ");
        said.push_str(&cause.to_string());
        beneath = cause.source();
    }
    said
}

/// Watches the shipped content root.
///
/// **The one door a client goes through**, which is what keeps a client's own
/// sources naming nothing that reads or watches a content root.
#[must_use]
pub fn watching_shipped_content(root: PathBuf) -> ContentReload {
    let watch = NotifyContentWatch::watching(&root);
    ContentReload::watching(root, Box::new(watch))
}

/// Block names quoted, comma separated, the last two joined by `and`.
fn named(blocks: &[BlockName]) -> String {
    let quoted: Vec<String> = blocks
        .iter()
        .map(|block| format!("`{name}`", name = block.as_str()))
        .collect();
    match quoted.split_last() {
        None => "no block".to_owned(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{first} and {last}", first = rest.join(", ")),
    }
}
