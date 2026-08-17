//! Which sections a reload left to mesh again, what a batch of them came to, and
//! what the worker made of one.
//!
//! # The dirty set is *taken*, so reading it twice reads it once
//!
//! `Session::take_remesh_work` drains. Every reading below therefore happens
//! exactly once per scenario and is held in a value, and a scenario wanting a
//! before and an after takes two separate readings around the thing it is judging.
//! A fixture that asked twice would find an empty set the second time and a
//! scenario reading it would call that "nothing was marked" — which is the
//! channel-blindness this spec has already paid for once, with the sign flipped.
//!
//! # Every verdict here is total, so an assertion against the good arm rejects
//! the answers that mean "there was nothing to look at"
//!
//! [`Marking`] has an arm for a set that is not the shipped world's, [`Collected`]
//! has one for nothing arriving at all, and [`Meshed`] has one for a batch that
//! never existed. `assert!(..is_some())` cannot tell any of those from the
//! property being asserted.
//!
//! # How long a collect waits, and why the direction of its failure is what makes
//! the bound safe
//!
//! A re-mesh runs on a worker thread, so a collect has to be polled. The wait is
//! denominated in the one declared quantity a batch's cost can be expressed in —
//! the mesher's per-section budget of 200 µs (`crates/mc-render/CLAUDE.md`) times
//! the sections a whole-world batch carries — and then widened by a margin that is
//! not a measurement. **Every assertion made against it is a *presence*: that a
//! superseded batch was reported, that a scene arrived, that a failure was named.**
//! A window too short therefore fails loudly on [`Collected::StillMeshing`]
//! rather than passing over an absence, which is the one direction a generous bound
//! is safe in.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names a batch that carries its own registry and a three-armed `Remeshed`,
//! neither of which the implementation has written yet, exactly as
//! [`crate::reload`] and [`crate::reload_content`] do. A binary including this must
//! declare `mod support;`, the input harness and [`crate::reload_world`] as well.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use mc_client::content::ContentView;
use mc_client::remesh::{Collecting, Remeshed, Remesher, Retained};
use mc_client::session::reload::Remeshing;
use mc_core::block::BlockRegistry;
use mc_core::content::ContentSerial;
use mc_render::texture::TextureLayers;
use mc_render::window::rendered;
use mc_sim::player::PlayerState;
use mc_sim::replay::world::FOOTPRINT_COLUMNS;
use mc_sim::replay::{SectionQuads, remesh};
use mc_sim::world::{RemeshWork, SectionKey, World};
use mc_world::column::SECTIONS_PER_COLUMN;
use mc_world::mesh::{Facing, Quad};
use mc_world::world::VoxelWorld;
use winit::event::MouseButton;

use crate::input::InputHarness;
use crate::reload_world::{AIM_AT_THE_FAR_CELL, AIM_ON_TO_THE_NEAR_CELL, Edit, edit, playing};

/// One section of one column, as a value a scenario can compare and print.
pub type Section = (i32, i32, usize);

/// How many sections the shipped world stacks.
///
/// **Derived from the two declarations it is the product of** — four columns
/// square, sixteen sections each — so the 256 this phase's counts turn on appears
/// nowhere as a number somebody would have to keep in step by hand. A world built
/// to a different footprint fails the comparison loudly instead of agreeing over
/// fewer sections.
pub const EVERY_SECTION_OF_THE_SHIPPED_WORLD: usize =
    (FOOTPRINT_COLUMNS * FOOTPRINT_COLUMNS * SECTIONS_PER_COLUMN) as usize;

/// The most a collect waits for the worker to finish a batch.
///
/// **A test bound with a number of its own, and not the mesher's production budget
/// times a count.** An earlier form was `200 µs × 256 × 20`, derived from the
/// declared per-section budget in `crates/mc-render/CLAUDE.md` — which is *production
/// policy about a shipped machine*, not a statement about how slow this one may be
/// while the suite runs. It came to 1.02 s and a coverage-instrumented run of 1 191
/// tests lost a 256-section batch inside it.
///
/// **Derived from both directions, per `testing.md` §2.**
///
/// - *From below, measured:* a whole-world batch did **not** finish inside 1.02 s
///   under `cargo llvm-cov nextest` over the full workspace, while the same batch
///   meshes in about 35 ms uninstrumented (256 sections at the benchmarked ~136 µs).
///   So the floor is above 1.02 s, and how far above is unknown because the run that
///   exceeded it never completed — which is the argument for a wide multiple rather
///   than a tight one.
/// - *From above:* nothing. Every assertion made against this bound is a **presence**
///   — a scene arrived, a batch was superseded, a failure was named — so a longer
///   wait can only turn a red into a green that was always true. And it costs
///   **nothing on a passing run**: the loop returns the moment the worker answers, so
///   the bound is only ever spent by a run that was going to fail anyway.
/// - *The smallest difference it must still catch:* a worker that never answers at
///   all, which no amount of waiting turns into an answer.
///
/// It was not reached by loosening until green: the failing run was read first, its
/// four agreeing elements identified the missing one as the scene alone, and this
/// bound is fifteen times the measured insufficiency.
const A_COLLECT_MAY_NOT_OUTLAST: Duration = Duration::from_secs(15);

/// How long a collect pauses between asks, so the worker has the machine.
const BETWEEN_POLLS: Duration = Duration::from_millis(1);

/// What a fixture says when it was asked to read something out of a client that
/// publishes no content at all.
pub const NOTHING_IS_SERVING: &str = "this fixture's client publishes no content, so there are no layers to draw with and no \
     serial a batch could have been drained under";

/// What a fixture says when a scenario needed a batch and the client had none.
pub const NOTHING_WAS_LEFT_TO_MESH: &str = "this scenario needs the client to have been left something to mesh again, and it was left \
     nothing at all";

/// How long a collect waits for the worker before it gives up.
const fn a_batchs_patience() -> Duration {
    A_COLLECT_MAY_NOT_OUTLAST
}

/// A client playing the world `blocks_of` builds against the root at `root`, with
/// the player at `spawn`.
///
/// **No watcher.** The scenarios this serves hand the client a candidate through
/// its own door; the ones about what a report carries attach a watch instead.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if
/// the content declares no solid block at all.
pub fn a_client_over(
    root: &Path,
    spawn: PlayerState,
    blocks_of: impl FnOnce(&BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>>,
) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, spawn, blocks_of)?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// Breaks the cell the spawn's look meets first, and says what that did.
pub fn breaking_the_far_cell(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_AT_THE_FAR_CELL);
    client.click(MouseButton::Left);
    edit(client.edit())
}

/// Builds what the client is holding against the nearer of the two aimed-at
/// cells, from the spawn's own level look.
pub fn placing_over_the_near_cell(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_AT_THE_FAR_CELL);
    placing_over_the_near_cell_after_the_far_aim(client)
}

/// The same, for a client whose look is already aimed at the further cell.
///
/// **Raw counts accumulate into the pitch**, so a run that has already broken the
/// far cell must add only the difference between the two aims. Asking for the
/// whole of the nearer aim again would carry the look past the declared pitch limit
/// and clamp it, which is a third aim nothing derived.
pub fn placing_over_the_near_cell_after_the_far_aim(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_ON_TO_THE_NEAR_CELL);
    client.click(MouseButton::Right);
    edit(client.edit())
}

/// Which sections a reload left to be meshed again.
///
/// **A total verdict**, so an assertion against the whole-world arm rejects a
/// reload that marked nothing, one that marked a subset, and one that marked a
/// section twice — the last of which is what a dirty set that stopped being a set
/// would produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marking {
    /// Nothing at all was left to mesh again.
    NoSectionAtAll,
    /// Every section of the shipped world, each exactly once.
    EverySectionOfTheShippedWorld { marked: usize },
    /// Some other set, and how it differs from the shipped world's.
    Sections {
        marked: usize,
        distinct: usize,
        missing: Vec<Section>,
        beyond: Vec<Section>,
    },
}

/// Every section of the shipped world, as the set a whole-world mark produces.
///
/// Built from the footprint's own declarations rather than from a read of a world,
/// so it is an independent statement of what "every section" means.
#[must_use]
pub fn every_section_of_the_shipped_world() -> BTreeSet<Section> {
    let across = i32::try_from(FOOTPRINT_COLUMNS).unwrap_or(0);
    let stacked = usize::try_from(SECTIONS_PER_COLUMN).unwrap_or(0);
    (0..across)
        .flat_map(move |x| (0..across).map(move |z| (x, z)))
        .flat_map(move |(x, z)| (0..stacked).map(move |index| (x, z, index)))
        .collect()
}

/// What `client` was left to mesh again, taken once.
pub fn marked(client: &mut InputHarness) -> Marking {
    let Some(work) = client.take_remesh_work() else {
        return Marking::NoSectionAtAll;
    };
    marking_of(&keys_of(&work))
}

/// What a key list amounts to as a [`Marking`].
#[must_use]
pub fn marking_of(keys: &[SectionKey]) -> Marking {
    let named: Vec<Section> = keys.iter().copied().map(section_of).collect();
    let distinct: BTreeSet<Section> = named.iter().copied().collect();
    let whole = every_section_of_the_shipped_world();
    if named.len() == distinct.len() && distinct == whole {
        return Marking::EverySectionOfTheShippedWorld {
            marked: named.len(),
        };
    }
    Marking::Sections {
        marked: named.len(),
        distinct: distinct.len(),
        missing: whole.difference(&distinct).copied().collect(),
        beyond: distinct.difference(&whole).copied().collect(),
    }
}

/// The whole shipped world, marked once each, for a scenario to compare against.
#[must_use]
pub const fn every_section_once() -> Marking {
    Marking::EverySectionOfTheShippedWorld {
        marked: EVERY_SECTION_OF_THE_SHIPPED_WORLD,
    }
}

/// Which sections `work` will mesh, in the order the dirty set holds them.
#[must_use]
pub fn keys_of(work: &RemeshWork) -> Vec<SectionKey> {
    work.keys().collect()
}

/// One re-mesh key as a comparable triple.
#[must_use]
pub const fn section_of(key: SectionKey) -> Section {
    (key.column.x, key.column.z, key.index)
}

/// What one finished batch came to.
///
/// **A total verdict**, so a scenario expecting a discarded batch cannot be
/// satisfied by a scene, by a failure, or by nothing arriving inside the window
/// this waits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Collected {
    /// A scene was handed over, holding this many sections.
    Scene { sections: usize },
    /// The batch was meshed against content that is no longer serving.
    ///
    /// **Which sections it would have meshed is deliberately not carried here.**
    /// The only defect their identity could catch on this side of the seam is a
    /// worker that recorded the wrong keys for the batch in flight, and that is
    /// already caught *through its effect* by the scenario about the hand-back,
    /// which compares what ends up waiting against a set captured before the batch
    /// was submitted. Carrying them here would re-prove that fact through the same
    /// bookkeeping — and an accessor that hands a test the value it is about is how
    /// an assertion on the value gets written in place of one on the consequence.
    Superseded,
    /// The batch could not be turned into a scene, and this is what it said.
    Failed { said: String },
    /// Nothing came back, and the worker is still holding the batch it was given.
    ///
    /// This is the only one of the two absences a longer wait could turn into an
    /// answer.
    StillMeshing,
    /// The channel is gone, so no batch will arrive now or later.
    ///
    /// **Said by the collect and no longer inferred**, so it arrives immediately
    /// rather than after the patience is spent: waiting is the repair for a worker
    /// that has not finished and no repair at all for one that has gone.
    TheWorkerIsGone,
    /// The patience was spent, nothing arrived, and the worker holds nothing.
    ///
    /// **A fixture fault rather than a product one.** A channel that had gone would
    /// have said so on the first ask, so what is left here is a batch nobody handed
    /// over — read it as "this scenario submitted nothing", never as a broken client.
    NothingWasEverHandedOver,
}

/// What `remesher` handed back, waited for.
pub fn collected(remesher: &mut Remesher) -> Collected {
    let started = Instant::now();
    while started.elapsed() < a_batchs_patience() {
        match remesher.collect() {
            Collecting::Finished(finished) => return verdict_of(&finished),
            // Immediate, and it ends the run: nothing will arrive later, so spending
            // the rest of the patience would only delay the report.
            Collecting::WorkerGone => return Collected::TheWorkerIsGone,
            Collecting::NothingYet => thread::sleep(BETWEEN_POLLS),
        }
    }
    // The patience is spent. A worker that is still busy has the batch; one that is
    // free was never handed anything, because a channel that had gone would have said
    // so on the first ask above.
    if remesher.is_free() {
        Collected::NothingWasEverHandedOver
    } else {
        Collected::StillMeshing
    }
}

/// One answer from the worker as a [`Collected`].
fn verdict_of(finished: &Remeshed) -> Collected {
    match finished {
        Remeshed::Scene(scene) => Collected::Scene {
            sections: scene.sections().len(),
        },
        // The sections go unread: nothing on this side of the seam may hand them
        // back, and the scenario that is about them asserts through what ends up
        // waiting instead.
        Remeshed::Superseded(_) => Collected::Superseded,
        Remeshed::Failed(failure) => Collected::Failed {
            said: rendered(failure),
        },
    }
}

/// A scene of every section one column stacks, for a scenario to compare against.
#[must_use]
pub const fn a_scene_of_one_column() -> Collected {
    Collected::Scene {
        sections: SECTIONS_PER_COLUMN as usize,
    }
}

/// What one collect *through the client* came to.
///
/// **The difference from [`Collected`] is which side of the seam the reading is
/// taken on, and it is the whole reason this type exists.** `Collected` reads
/// `Remesher::collect`, which is where the staleness comparison is made; this reads
/// `Session::collect_remesh`, which is where a discarded batch's sections are *put
/// back*. A scenario about the hand-back that read the first would have to make the
/// call itself, and a frame path that dropped the keys would satisfy it exactly —
/// measured at 77 of 77 green before this type existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handled {
    /// A scene was handed up for the device, holding this many sections.
    Scene {
        sections: usize,
    },
    /// A batch was discarded, and its sections put back among those waiting.
    Discarded,
    /// The batch could not be turned into a scene, and this is what it said.
    Failed {
        said: String,
    },
    /// The three absences, told apart: see [`Collected`] for what each means.
    StillMeshing,
    TheWorkerIsGone,
    NothingWasEverHandedOver,
}

/// What `client` made of whatever `remesher` has finished, waited for.
///
/// **Nothing here puts a section back.** The hand-back happens inside the client's
/// own collect, which is what makes a scenario reading this an assertion about the
/// client rather than an agreement between two callers of one function.
pub fn handled(client: &mut InputHarness, remesher: &mut Remesher) -> Handled {
    let started = Instant::now();
    while started.elapsed() < a_batchs_patience() {
        match client.collect_remesh(remesher) {
            Remeshing::NothingYet => thread::sleep(BETWEEN_POLLS),
            Remeshing::Show(scene) => {
                return Handled::Scene {
                    sections: scene.sections().len(),
                };
            }
            Remeshing::Discarded => return Handled::Discarded,
            Remeshing::Report(failure) => {
                return Handled::Failed {
                    said: rendered(&failure),
                };
            }
            Remeshing::WorkerGone => return Handled::TheWorkerIsGone,
        }
    }
    if remesher.is_free() {
        Handled::NothingWasEverHandedOver
    } else {
        Handled::StillMeshing
    }
}

/// What the worker made of a batch it was expected to be unable to pack.
///
/// The failure's own words are judged against the block whose texture has no
/// layer rather than carried into an expectation, so no scenario states a sentence
/// the renderer owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reported {
    /// It failed, and what it said names that block.
    FailedNamingTheBlock,
    /// It failed without naming it, saying this.
    FailedWithoutNamingIt { said: String },
    /// It did not fail at all.
    DidNot(Collected),
}

/// What `finished` amounts to, against the `block` a failure has to name.
#[must_use]
pub fn reported(finished: Collected, block: &str) -> Reported {
    match finished {
        Collected::Failed { said } if said.contains(block) => Reported::FailedNamingTheBlock,
        Collected::Failed { said } => Reported::FailedWithoutNamingIt { said },
        other => Reported::DidNot(other),
    }
}

/// The sections and the layers a launch over `blocks` would have retained.
///
/// **Meshed here through the world's own whole-world mesh**, which is the call a
/// launch makes, so the list a re-meshed section is spliced back into is the one
/// the product would have held.
///
/// # Errors
///
/// Returns an error if the world does not resolve against `registry`, or if it
/// cannot be meshed.
pub fn retained_at_launch(
    blocks: VoxelWorld,
    registry: Arc<BlockRegistry>,
    layers: TextureLayers,
) -> Result<Retained, Box<dyn Error>> {
    Ok(Retained {
        meshed: World::new(blocks, registry)?.mesh()?,
        layers,
    })
}

/// The layers a reader builds out of what `client` is publishing.
///
/// # Errors
///
/// Returns an error where the client publishes nothing.
pub fn layers_serving(client: &InputHarness) -> Result<TextureLayers, Box<dyn Error>> {
    let published = client.content().ok_or(NOTHING_IS_SERVING)?;
    Ok(ContentView::of(&published.resolved).into_layers())
}

/// The serial the content `client` is publishing was published under.
///
/// # Errors
///
/// Returns an error where the client publishes nothing.
pub fn serial_serving(client: &InputHarness) -> Result<ContentSerial, Box<dyn Error>> {
    Ok(client.content().ok_or(NOTHING_IS_SERVING)?.serial)
}

/// One section as it was meshed: where its near corner sits, and the faces it
/// shows.
///
/// The origin travels with the quads because a re-mesh that placed a section one
/// block from where the whole-world mesh put it would draw a world subtly sheared,
/// and a comparison over the quads alone could not see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshedSection {
    pub origin: [i32; 3],
    pub quads: Vec<Quad>,
}

/// Every section a mesh produced, keyed by which section it is.
pub type Sections = BTreeMap<Section, MeshedSection>;

/// What meshing a batch came to.
///
/// **A total verdict and never a propagated error**: a batch that refused to mesh
/// fails the comparison naming what it said instead of ending the test before its
/// assertion ran, and a batch that never existed has an arm rather than arriving
/// as an empty map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Meshed {
    /// Every section the batch named, meshed.
    Sections(Sections),
    /// The batch could not be meshed, and this is what it said.
    Refused { said: String },
    /// Nothing was left to mesh, so there was no batch.
    NoBatch,
}

/// What meshing whatever `client` was left to mesh came to, taken once.
///
/// **The batch is meshed through no registry of this fixture's own.** A batch
/// carries the registry the world that produced it was resolved against, so a
/// section meshed here is meshed against the content the client is serving and
/// there is no second opinion to hand in.
pub fn meshed(client: &mut InputHarness) -> Meshed {
    let Some(work) = client.take_remesh_work() else {
        return Meshed::NoBatch;
    };
    meshed_of(&work)
}

/// The same, for a batch a scenario is holding.
#[must_use]
pub fn meshed_of(work: &RemeshWork) -> Meshed {
    match remesh(work) {
        Ok(sections) => Meshed::Sections(as_sections(sections)),
        Err(refused) => Meshed::Refused {
            said: rendered(&refused),
        },
    }
}

/// Every section of `blocks`, meshed against `registry`.
///
/// The independent oracle a batch's own meshing is compared against: it shares no
/// batch, no dirty set and no registry with the client under test, and it is the
/// same whole-world mesh a launch produces.
///
/// # Errors
///
/// Returns an error if the world does not resolve against `registry`, or if it
/// cannot be meshed.
pub fn meshed_against(
    blocks: VoxelWorld,
    registry: Arc<BlockRegistry>,
) -> Result<Meshed, Box<dyn Error>> {
    Ok(Meshed::Sections(as_sections(
        World::new(blocks, registry)?.mesh()?,
    )))
}

/// A meshed list keyed by which section each entry is.
fn as_sections(meshed: Vec<SectionQuads>) -> Sections {
    meshed
        .into_iter()
        .map(|section| {
            (
                (section.column.x, section.column.z, section.section_index),
                MeshedSection {
                    origin: section.origin,
                    quads: section.quads,
                },
            )
        })
        .collect()
}

/// How many of a block's faces a mesh shows, or what that mesh came to instead.
///
/// **A total verdict**, so a count of zero cannot stand in for a batch that
/// refused or one that never existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Faces {
    Showing(usize),
    Refused { said: String },
    NoBatch,
}

/// How many faces `meshed` shows for `block`.
#[must_use]
pub fn faces_shown(meshed: &Meshed, block: &str) -> Faces {
    counting(meshed, |sections| faces_of(sections, block))
}

/// How many of `block`'s faces in `meshed` point along `facing`.
#[must_use]
pub fn faces_shown_facing(meshed: &Meshed, block: &str, facing: Facing) -> Faces {
    counting(meshed, |sections| faces_facing(sections, block, facing))
}

/// Whatever `count` makes of a mesh's sections, or the arm the mesh came to.
fn counting(meshed: &Meshed, count: impl FnOnce(&Sections) -> usize) -> Faces {
    match meshed {
        Meshed::Sections(sections) => Faces::Showing(count(sections)),
        Meshed::Refused { said } => Faces::Refused { said: said.clone() },
        Meshed::NoBatch => Faces::NoBatch,
    }
}

/// How many faces `sections` shows for `block`.
#[must_use]
pub fn faces_of(sections: &Sections, block: &str) -> usize {
    quads_of(sections, block).count()
}

/// How many of `block`'s faces in `sections` point along `facing`.
#[must_use]
pub fn faces_facing(sections: &Sections, block: &str, facing: Facing) -> usize {
    quads_of(sections, block)
        .filter(|quad| quad.facing == facing)
        .count()
}

/// Every quad of `sections` that holds `block`.
fn quads_of<'a>(sections: &'a Sections, block: &'a str) -> impl Iterator<Item = &'a Quad> {
    sections
        .values()
        .flat_map(|section| section.quads.iter())
        .filter(move |quad| quad.block.as_str() == block)
}

/// Whatever a mesh came to as its sections, or an error naming what it came to
/// instead.
///
/// For the guards that need the sections themselves rather than a verdict.
///
/// # Errors
///
/// Returns an error unless the batch was meshed.
pub fn sections_meshed(meshed: Meshed) -> Result<Sections, Box<dyn Error>> {
    match meshed {
        Meshed::Sections(sections) => Ok(sections),
        other => Err(format!(
            "this fixture needs the batch to have been meshed before it can read faces out of it, \
             and it came to {other:?}"
        )
        .into()),
    }
}

/// Fails with `explanation` unless `holds`.
///
/// A fixture that does not have the property an assertion rests on is a broken
/// fixture rather than a failed behaviour, and it says so before the assertion
/// runs.
///
/// # Errors
///
/// Returns `explanation` when `holds` is false.
pub fn require(holds: bool, explanation: String) -> Result<(), Box<dyn Error>> {
    if holds {
        Ok(())
    } else {
        Err(explanation.into())
    }
}
