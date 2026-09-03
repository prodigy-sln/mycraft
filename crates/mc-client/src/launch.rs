//! Deciding which world a launch plays, and preparing that world for the
//! renderer.
//!
//! **The split from [`startup`](crate::startup) is by responsibility rather than
//! by line count.** `startup` turns a content root into a drawable *generated*
//! scene — the capture pipeline every golden frame is shot through, which reads
//! no save and must not learn how. What is here is the other question: which
//! world a *player* is handed, which is the save's answer, and the preparation of
//! that world.
//!
//! # Why two entry points rather than a branch inside one
//!
//! [`prepare_launch`](crate::launch::prepare_launch) and
//! [`prepare_scene`](crate::startup::prepare_scene) both
//! turn a content root into geometry, and having two of anything on the golden
//! path is exactly how the images once drifted: evidence gathered from a pipeline
//! the product does not run does not transfer to the one a player launches. The
//! objection is answered structurally rather than by care — both doors share one
//! mesher, one definition of the texture key set and one packer, so the only
//! thing that can differ between them is which world's blocks went in — and then
//! it is asserted, byte for byte, by a test comparing the two.
//!
//! The alternative, a save-aware branch inside `prepare_scene`, is ruled out: a
//! stray save in a capture's working directory would then change what a committed
//! image shows, for a reason no reader of the diff could see.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

use mc_core::block::BlockRegistry;
use mc_core::content::LayerAssignment;
use mc_core::hud::HudLayout;
use mc_core::id::{BlockName, TextureKey};
use mc_render::geometry::scene::SceneGeometry;
use mc_render::texture::TextureResolution;
use mc_sim::action::default_held_block;
use mc_sim::content::LoadedContent;
use mc_sim::persistence::{Launching, simulation_at_launch};
use mc_sim::replay::SectionQuads;
use mc_sim::simulation::{PublishedContent, Seated, Simulation};
use mc_sim::world::Clearing;
use mc_world::persistence::Acceptance;

use mc_render::texture::supplied::SuppliedTexels;

use crate::content::ContentView;
use crate::notice::Notices;
use crate::startup::{PreparationError, scene_of};
use crate::textures::{built_set, refusal_for};

/// Where the client keeps its save, relative to the directory it was started in
/// — the same convention the content root follows.
const SAVE_PATH: [&str; 2] = ["saves", "world.mcw"];

/// What one launch produces: the packed scene, the layers it was packed against,
/// the meshed sections that same scene was packed *from*, the simulation of the
/// world it plays, the block a client holds in it, and the content both were read
/// from.
///
/// **There is no `world` field, and that absence is load-bearing.** The geometry
/// the renderer is shown and the meshed sections a later edit splices into are two
/// answers to one question, and the wiring that picks them lives in the frame
/// path, where no guard may live. Carrying no world here is what makes a second
/// world unspellable at the seam rather than merely checked for: there is nothing
/// else in scope to mesh or to pack.
///
/// The layers are carried alongside rather than discarded because a scene records
/// which array *layer* each corner draws from and never which key that layer came
/// from, so uploading the array texture needs both halves.
///
/// The meshed sections are carried because an edit re-meshes the sections it
/// touched and puts them back where they were, so whatever re-meshes has to hold
/// the list it is putting them back into — and `splice` is positional, so it must
/// be *this* list and not a second whole-world mesh of the same world.
///
/// **The clearing verdict does not weaken the absence above.** What made a
/// `world` field dangerous is that there were two answers to one question and
/// the frame path would have had to pick; a [`Clearing`] is a `Copy` verdict
/// about what already happened, with no second candidate anywhere in scope to
/// confuse it with, and the one thing done with it is saying it out loud.
///
/// The registry is shared rather than handed over, because the simulation holds
/// one for the whole run — every edit resolves the name it writes against the
/// same registry the world was resolved against — and the caller keeps reading the
/// same one. The HUD elements the same content root declared travel with it for
/// the same reason: both are what the shipped content says, and neither is
/// something the frame path may decide.
#[derive(Debug)]
pub struct PreparedLaunch {
    /// The content root this launch was prepared from, so the run can watch it.
    ///
    /// **Handed back rather than kept by whoever spawned the preparation.** The
    /// root a client watches must be the root it is playing, and the only place
    /// those are the same value by construction is here.
    pub root: PathBuf,
    pub scene: SceneGeometry,
    pub resolution: TextureResolution,
    pub meshed: Vec<SectionQuads>,
    pub simulation: Simulation,
    /// What seating the player in that simulation did about where they stood.
    pub clearing: Clearing,
    pub holding: BlockName,
    pub registry: Arc<BlockRegistry>,
    pub hud: Arc<HudLayout>,
}

/// The worker preparing a launch, until there is a window to draw what it
/// produces.
pub type PreparationHandle = JoinHandle<Result<PreparedLaunch, PreparationError>>;

/// What a run is started with, before there is a window to draw it in.
///
/// **One value rather than two arguments**, because the two are handed together
/// from the process's own environment all the way to the frame path and a caller
/// holding one without the other has not made a mistake this signature should
/// let it express.
///
/// **The texels are read here and never again.** The renderer takes them at
/// construction and holds them for the whole run: the built set is a pre-build
/// artefact that does not change while the client runs, so a reload appending a
/// key finds either art that was already read or no art at all. A supply the
/// reload path carried would be a value that can arrive empty, and a world
/// drawing its baked art would go back to generated colours the moment somebody
/// saved a block file.
pub struct Starting {
    /// The level-zero texels the content root's built art offers.
    pub texels: SuppliedTexels,
    /// The worker turning that root into a drawable world.
    pub preparation: PreparationHandle,
    /// Where every non-fatal notice this run writes goes.
    ///
    /// Carried here rather than passed beside it, for the reason the two fields
    /// above are: this is what a run is started with, and a caller holding a sink
    /// without the run it belongs to has not made a mistake this signature should
    /// let them express.
    pub notices: Notices,
}

impl std::fmt::Debug for Starting {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Starting")
            .field("texels", &self.texels)
            .finish_non_exhaustive()
    }
}

/// Judges the built set under `root`, then starts preparing the launch it
/// belongs to.
///
/// **The set is judged before the window opens and before a world is generated**,
/// so a contributor who has not run the art build reads one sentence naming the
/// command rather than waiting out a world they will not be shown.
///
/// # Errors
///
/// Returns the refusal the set's verdict becomes — see [`refusal_for`] — and
/// [`PreparationError::TextureSetUnreadable`] where the set admits no verdict at
/// all or offers an image no layer can be filled from.
pub fn start(
    root: PathBuf,
    save: PathBuf,
    accepting: Acceptance,
    notices: &Notices,
) -> Result<Starting, PreparationError> {
    let (verdict, texels) = built_set(&root)?;
    if let Some(refused) = refusal_for(&verdict) {
        return Err(refused);
    }
    // Taken before the texels are handed on, so the notice and the array texture
    // answer out of one decode rather than out of two reads of the set.
    let covered = texels.keys();
    Ok(Starting {
        texels,
        preparation: spawn_preparation(
            root,
            save,
            accepting,
            Reporting {
                covered,
                notices: notices.clone(),
            },
        ),
        notices: notices.clone(),
    })
}

/// Where this client keeps its save.
///
/// **Not checked for existence**, unlike
/// [`shipped_content`](crate::startup::shipped_content), and the asymmetry is the
/// point: content that is not there is somebody running the binary from the wrong
/// directory, and a save that is not there is a first launch.
#[must_use]
pub fn save_path() -> PathBuf {
    SAVE_PATH.iter().collect()
}

/// The simulation a launch plays, and the block a client holds in it.
///
/// **The one statement of "which world, and which block"**, callable with no
/// device and no window present. It decides nothing itself: which world is
/// `mc-sim`'s answer, and which block is the simulation's own policy.
///
/// The seed is passed through rather than read here, so a launch has no hidden
/// input and nothing generates a world before `mc-sim` has said one is wanted.
///
/// # Errors
///
/// Returns [`PreparationError::Launch`] when the save is there and cannot be
/// read, or when there is no save and no world could be generated in its place,
/// and [`PreparationError::NothingToPlace`] when the content registers no solid
/// block for a player to place.
pub fn simulation_to_play(
    save: &Path,
    launching: Launching,
) -> Result<(Seated, BlockName), PreparationError> {
    let holding =
        default_held_block(&launching.registry).ok_or(PreparationError::NothingToPlace)?;
    Ok((simulation_at_launch(save, launching)?, holding))
}

/// The simulation a launch plays, with what the save it read disagreed with the
/// content about already said out loud.
///
/// **Said here, where the load has completed and no device has been opened.** It
/// is a statement about a file rather than about a world the player has been put
/// into, so it does not wait for a picture the way the clearing notice does —
/// `notice::say_entering` sits below the frame path's uploads because "you were
/// moved" needs a world to have been moved in, and "these blocks changed" is
/// already true the moment the save has been read.
///
/// That is not tidiness: it is what puts the only scenario able to see a client
/// which composes the line and never prints it — a run of the shipped binary — in
/// reach of a suite with no display server. Moving this below the uploads for
/// symmetry with its sibling would take that scenario out of reach again.
///
/// **A function of its own rather than four statements inside the sequence
/// above**, because deciding which world a launch plays and telling the player
/// what reading its save found are one step from the caller's side and the
/// sequence has no business being able to do the first without the second.
///
/// # Errors
///
/// Returns whatever [`simulation_to_play`] refuses.
fn played_and_reported(
    save: &Path,
    launching: Launching,
    notices: &Notices,
) -> Result<(Seated, BlockName), PreparationError> {
    let (seated, holding) = simulation_to_play(save, launching)?;
    crate::notice::say_changed_blocks(&seated.changed, notices);
    Ok((seated, holding))
}

/// Starts preparing a launch on a worker, and hands back the handle to collect it
/// through.
///
/// The caller polls the handle rather than joining it, so the frame path never
/// waits: a window that is drawing the clear colour is a window that is visibly
/// working.
///
/// **The thread is why this is not the entry point a test should use.** A
/// `std::thread` spawned inside a `rayon` pool's `install` does not inherit that
/// pool, so a caller that wanted to decide how many workers mesh the world would
/// silently get the global one. Anything asking that question calls
/// [`prepare_launch`] directly, on its own thread.
///
/// **`covered` is passed in rather than read here**, because the set was already
/// judged by the caller: a second read to answer the same question is a second
/// opinion the notice could disagree with the array texture over.
pub fn spawn_preparation(
    root: PathBuf,
    save: PathBuf,
    accepting: Acceptance,
    saying: Reporting,
) -> PreparationHandle {
    std::thread::spawn(move || {
        let Reporting { covered, notices } = saying;
        let prepared = prepare_launch(&root, &save, accepting, &notices)?;
        // Said here rather than inside `prepare_launch` because this is where both
        // halves are: the preparation is what says which keys content declares,
        // and the set covering them was judged before this worker was spawned.
        // `prepare_launch` is handed a root and never a set.
        crate::notice::say_stand_ins(&declared_keys(&prepared.resolution), &covered, &notices);
        Ok(prepared)
    })
}

/// Every texture key the content a launch read declares.
///
/// **Read off the layer assignment rather than out of the registry**, which is
/// what `tests/client_derives_no_layer_assignment.rs` refuses and refuses for a
/// reason that outranks this notice: a client that builds its own key set has two
/// participants numbering layers separately, and since a layer index rides inside
/// every packed vertex, inserting one block then textures the whole world wrong
/// with no error anywhere. The assignment states which keys the *serving* content
/// names, so honouring it answers this question and derives nothing.
#[must_use]
pub fn declared_keys(resolution: &TextureResolution) -> BTreeSet<TextureKey> {
    resolution
        .layers()
        .entries()
        .map(|(key, _)| key.clone())
        .collect()
}

/// Collects a finished preparation, translating a worker that panicked into an
/// error rather than a second panic here.
///
/// # Errors
///
/// Returns whatever the preparation failed with, or
/// [`PreparationError::WorkerLost`] when the worker did not survive to report
/// anything at all.
pub fn collect(handle: PreparationHandle) -> Result<PreparedLaunch, PreparationError> {
    handle.join().unwrap_or(Err(PreparationError::WorkerLost))
}

/// What a preparation says about the art it read, and where it says it.
///
/// **One value rather than two arguments**, on the rule
/// `standards/global/code-quality.md` §2 states: the covered keys and the sink
/// are the saying half of a preparation and travel together, and a caller holding
/// one without the other cannot do anything with it.
#[derive(Debug)]
pub struct Reporting {
    /// The texture keys the built set covers, judged before this worker started.
    pub covered: BTreeSet<TextureKey>,
    /// Where the notice about the rest of them goes.
    pub notices: Notices,
}

/// The registry, the resolution and the published content the root at `root`
/// declares.
///
/// Asked of the simulation, which is what reads a content root. The HUD comes
/// back with it because a crosshair the content declares is content exactly as a
/// block is, and the two are refused together. A launch has spent no layers,
/// which is a fact rather than a decision, and passing it here is what makes the
/// property visible at the call.
///
/// The resolution is asked of the registry and never of what was just meshed: a
/// layer index rides inside every packed vertex, so a key set read off the played
/// world would let a save renumber the array texture.
///
/// # Errors
///
/// Returns whatever reading the content root refused with.
fn content_of(
    root: &Path,
) -> Result<(Arc<BlockRegistry>, TextureResolution, PublishedContent), PreparationError> {
    let LoadedContent {
        registry,
        hud,
        resolved,
    } = mc_sim::content::load(root, &LayerAssignment::none())?;
    let resolution = ContentView::of(&resolved).into_resolution();
    Ok((
        Arc::new(registry),
        resolution,
        PublishedContent::first(resolved, hud),
    ))
}

/// Reads `root`, asks `mc-sim` which world this launch plays, meshes that world
/// and packs one scene for the renderer.
///
/// Runs on the calling thread. No device, no window and no working-directory
/// change: `save` is an explicit path, as [`simulation_to_play`]'s already is.
///
/// **The order of the four steps below is the whole of it.** Which world is
/// established first and everything after it is done for that world, so the
/// geometry the renderer is shown and the simulation the player walks in are the
/// same world by construction — not by a check somewhere that they match. A
/// launch that meshed before it asked has already built the wrong world, and the
/// only repairs available then are to build a second one or to hand over the
/// first.
///
/// **Nothing here generates**, and the seed goes past this function rather than
/// into it: a world is derived from it only where the launch decision says there
/// is no save to resume. So a resume does no worldgen at all — not worldgen
/// whose product is discarded, which looks the same from the outside and is the
/// shape this order exists to rule out.
///
/// # Errors
///
/// Returns [`PreparationError`] for any step that could not complete. A failed
/// mesh fails the launch: half a world is not a picture anybody should be shown.
pub fn prepare_launch(
    root: &Path,
    save: &Path,
    accepting: Acceptance,
    notices: &Notices,
) -> Result<PreparedLaunch, PreparationError> {
    let (registry, resolution, content) = content_of(root)?;
    let hud = Arc::clone(&content.hud);

    let (seated, holding) = played_and_reported(
        save,
        Launching {
            seed: mc_sim::REPLAY_SEED,
            registry: Arc::clone(&registry),
            content,
            accepting,
        },
        notices,
    )?;

    let meshed = seated.simulation.world().mesh()?;

    Ok(PreparedLaunch {
        root: root.to_path_buf(),
        scene: scene_of(&meshed, &resolution)?,
        resolution,
        meshed,
        clearing: seated.clearing,
        simulation: seated.simulation,
        holding,
        registry,
        hud,
    })
}
