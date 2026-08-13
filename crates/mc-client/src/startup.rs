//! Turning the shipped content into something the renderer can draw, off the
//! thread that draws it.
//!
//! Generating the world, meshing it and packing every section takes long enough
//! to be visible, and doing it before the window opens would show the player a
//! frozen desktop instead of a window that is obviously waiting. So it runs on a
//! worker and the frame path draws the clear colour until it lands — which is the
//! whole reason `ScenePhase` has two variants rather than being an
//! `Option<SceneGeometry>` nobody looks at.
//!
//! Nothing here touches a GPU or a window. It is the same pipeline in the same
//! order every time, because a golden frame is a claim about what a camera saw
//! and the claim is only checkable if the bytes it saw are reproducible.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::id::{BlockName, NamespacedIdError, TextureKey};
use mc_render::geometry::scene::{SceneError, SceneGeometry};
use mc_render::geometry::{GeometryError, SectionOrigin, build_section_geometry};
use mc_render::gpu::RendererError;
use mc_render::texture::TextureLayers;
use mc_sim::action::default_held_block;
use mc_sim::persistence::{LaunchError, simulation_at_launch};
use mc_sim::replay::{PrepareError, ReplayWorld, SectionQuads, WorldGenError, mesh_all};
use mc_sim::simulation::Simulation;
use mc_world::content::TomlFileDefinitionSource;
use mc_world::persistence::{Acceptance, LoadError};
use thiserror::Error;

/// Where the shipped content is looked for, relative to the directory the client
/// was started in.
const CONTENT_ROOT: [&str; 2] = ["content", "base"];

/// Where the client keeps its save, relative to the directory it was started in
/// — the same convention [`CONTENT_ROOT`] follows.
const SAVE_PATH: [&str; 2] = ["saves", "world.mcw"];

/// What a player types to load a save whose blocks are no longer what they were.
///
/// **One spelling, named once**, because the parse that accepts it and the
/// refusal that tells a player about it have to agree: a message quoting a flag
/// nothing accepts is worse than no message at all, since it reads as a way out
/// and is not one.
const LOAD_CHANGED_BLOCKS: &str = "--load-changed-blocks";

/// What one preparation of the replay produces: the packed scene, the texture
/// layers its blocks resolved to, and the world and registry both were built
/// from.
///
/// The layers are carried alongside rather than discarded because a scene records
/// which array *layer* each corner draws from and never which key that layer came
/// from, so uploading the array texture needs both halves.
///
/// The world and the registry are carried for a different reason: the player's
/// spawn is derived from the world, so the simulation cannot exist until this
/// does. Handing them back rather than generating a second world in the
/// composition root is what keeps the world a frame is drawn of and the world a
/// player walks on the same world.
///
/// The registry is shared rather than handed over, because the simulation holds
/// one for the whole run — every edit resolves the name it writes against the
/// same registry the world was resolved against — and the caller keeps reading
/// the same one.
/// The meshed sections are carried for a third reason again: an edit re-meshes
/// the sections it touched and puts them back where they were, so whatever
/// re-meshes has to hold the list it is putting them back into. Nothing retained
/// it before — [`prepare_scene`] computed it, packed it and dropped it — and a
/// second whole-world mesh to recover it would be both slow and a second answer
/// to a question the first one already answered.
#[derive(Debug)]
pub struct PreparedScene {
    pub scene: SceneGeometry,
    pub layers: TextureLayers,
    pub meshed: Vec<SectionQuads>,
    pub world: ReplayWorld,
    pub registry: Arc<BlockRegistry>,
}

/// The worker preparing the replay, until there is a window to draw what it
/// produces.
pub type PreparationHandle = JoinHandle<Result<PreparedScene, PreparationError>>;

/// What a run was started with, before there is a window to draw it in: the
/// worker generating the world, and what the player said about loading a save
/// whose blocks have changed.
///
/// **One value rather than two arguments carried side by side through three
/// layers.** Both are decided before the window opens and both are spent at the
/// same moment — when the preparation lands and a world has to be chosen — so
/// they travel together.
#[derive(Debug)]
pub struct Launch {
    pub preparation: PreparationHandle,
    pub accepting: Acceptance,
}

/// Why the replay could not be prepared.
///
/// Every one of these fails the run rather than degrading it: a world that could
/// not be meshed has no picture to show, and a window drawing the clear colour
/// forever is indistinguishable from one that is still working.
#[derive(Debug, Error)]
pub enum PreparationError {
    #[error(
        "no content was found at `{}`; the client reads the shipped blocks from that directory \
         relative to where it was started",
        root.display()
    )]
    NoContentRoot { root: PathBuf },
    #[error("the shipped content could not be read")]
    Content(#[from] RegistryError),
    #[error("the replay world could not be generated")]
    WorldGen(#[from] WorldGenError),
    #[error("the replay world could not be meshed")]
    Mesh(#[from] PrepareError),
    #[error("a quad named a block whose name is not a texture key")]
    TextureKey(#[from] NamespacedIdError),
    #[error("a meshed section could not be packed into vertices")]
    Geometry(#[from] GeometryError),
    #[error("the packed sections could not be assembled into one scene")]
    Scene(#[from] SceneError),
    #[error("the prepared scene could not be given to the device")]
    Upload(#[from] RendererError),
    /// The launch could not decide which world to play.
    ///
    /// **The generated world failing to place a player arrives here too**, as
    /// `LaunchError::Spawn`, which is why this type carries no spawn variant of
    /// its own: deciding which world a launch plays and placing the player in it
    /// are one answer, and two variants for it would be two things a reader has
    /// to tell apart where nothing constructs the second.
    ///
    /// **The message a player is shown for a save whose blocks have changed is
    /// this one**, and it is the whole user interface the decision has in this
    /// build: it has to name every changed block and say exactly what to pass to
    /// load anyway. The refusal underneath already names the file and every
    /// block; what is added here is the sentence that says what to do about it.
    ///
    /// **The way out is offered only where it is one.** A save refused for a
    /// block the registry does not hold, or for bytes that are not a save, is
    /// not something the flag can load — telling a player to pass it there would
    /// send them round the same refusal a second time.
    #[error("{0}{way_out}", way_out = way_out_of(.0))]
    Launch(#[from] LaunchError),
    #[error(
        "the content registers no solid block, so a player would have nothing to place; the \
         block a client holds is the first solid one in registration order"
    )]
    NothingToPlace,
    #[error("the thread preparing the replay ended without producing a scene or an error")]
    WorkerLost,
}

/// The content directory the client reads, checked to exist before anything is
/// started that would fail on it later.
///
/// # Errors
///
/// Returns [`PreparationError::NoContentRoot`] naming the path when it is not
/// there, which is the failure somebody running the binary from the wrong
/// directory gets — and it names the directory rather than a missing block.
pub fn content_root() -> Result<PathBuf, PreparationError> {
    let root = CONTENT_ROOT.iter().collect::<PathBuf>();
    if root.is_dir() {
        Ok(root)
    } else {
        Err(PreparationError::NoContentRoot { root })
    }
}

/// Where this client keeps its save.
///
/// **Not checked for existence**, unlike [`content_root`], and the asymmetry is
/// the point: content that is not there is somebody running the binary from the
/// wrong directory, and a save that is not there is a first launch.
#[must_use]
pub fn save_path() -> PathBuf {
    SAVE_PATH.iter().collect()
}

/// What the player said about loading a save whose blocks have changed.
///
/// One flag, parsed by hand, with **no default in the domain value**: absent
/// means the save is refused if anything about its blocks has changed, and a
/// player who wants it loaded anyway says so per run. An environment variable
/// would persist invisibly across launches, which is the "defaulting to yes"
/// this decision exists to rule out.
#[must_use]
pub fn acceptance_from(args: impl Iterator<Item = String>) -> Acceptance {
    // The first argument is the program's own name, which is not something the
    // player typed and is not an answer to anything.
    if args.skip(1).any(|argument| argument == LOAD_CHANGED_BLOCKS) {
        Acceptance::ChangedBlocksToo
    } else {
        Acceptance::OnlyUnchangedBlocks
    }
}

/// The simulation a launch plays, and the block a client holds in it.
///
/// **The wiring the composition root does when the preparation lands**, lifted
/// out of the frame path so that which world a launch plays can be asked with no
/// device and no window present. It decides nothing itself: which world is
/// `mc-sim`'s answer, and which block is the simulation's own policy.
///
/// # Errors
///
/// Returns [`PreparationError::Launch`] when the save is there and cannot be
/// read, and [`PreparationError::NothingToPlace`] when the content registers no
/// solid block for a player to place.
pub fn simulation_to_play(
    generated: &ReplayWorld,
    registry: Arc<BlockRegistry>,
    save: &Path,
    accepting: Acceptance,
) -> Result<(Simulation, BlockName), PreparationError> {
    let holding = default_held_block(&registry).ok_or(PreparationError::NothingToPlace)?;
    Ok((
        simulation_at_launch(save, generated, registry, accepting)?,
        holding,
    ))
}

/// What to tell a player about `failure` beyond what it already says — the way
/// out where there is one, and nothing where there is not.
///
/// A save whose blocks have only been *redeclared* is loadable data, and whether
/// it should be loaded is the player's judgement to make; every other refusal is
/// about a world that cannot be built at all, and no flag changes that.
fn way_out_of(failure: &LaunchError) -> String {
    if refused_only_for_changed_blocks(failure) {
        format!(
            ". Those blocks are no longer what they were when this world was saved; pass \
             `{LOAD_CHANGED_BLOCKS}` to load it anyway"
        )
    } else {
        String::new()
    }
}

/// Whether the player saying yes is all that stands between `failure` and a
/// world.
///
/// A missing name is deliberately not one of these: acceptance never covers it,
/// so a save refused for both would be refused again by the same list.
fn refused_only_for_changed_blocks(failure: &LaunchError) -> bool {
    let LaunchError::Load { source, .. } = failure else {
        return false;
    };
    matches!(
        source.as_ref(),
        LoadError::Unresolvable { missing, changed } if missing.is_empty() && !changed.is_empty()
    )
}

/// Starts preparing the replay on a worker, and hands back the handle to collect
/// it through.
///
/// The caller polls the handle rather than joining it, so the frame path never
/// waits: a window that is drawing the clear colour is a window that is visibly
/// working.
///
/// **The thread is why this is not the entry point a test should use.** A
/// `std::thread` spawned inside a `rayon` pool's `install` does not inherit that
/// pool, so a caller that wanted to decide how many workers mesh the world would
/// silently get the global one. Anything asking that question calls
/// [`prepare_scene`] directly, on its own thread.
pub fn spawn_preparation(root: PathBuf) -> PreparationHandle {
    std::thread::spawn(move || prepare_scene(&root))
}

/// Collects a finished preparation, translating a worker that panicked into an
/// error rather than a second panic here.
///
/// # Errors
///
/// Returns whatever the preparation failed with, or
/// [`PreparationError::WorkerLost`] when the worker did not survive to report
/// anything at all.
pub fn collect(handle: PreparationHandle) -> Result<PreparedScene, PreparationError> {
    handle.join().unwrap_or(Err(PreparationError::WorkerLost))
}

/// Generates the replay world, meshes it, resolves its textures and packs every
/// section into one scene, reading its content from `root`.
///
/// **This is the one statement of that sequence, and it is public so that the
/// suites which shoot the goldens run it rather than a copy of it.** Golden frames
/// are the only automated evidence that the renderer draws the right picture, and
/// evidence gathered from a second pipeline does not transfer to the one a player
/// launches. It runs on the calling thread, which is what lets a caller decide
/// with `rayon` how many workers mesh the world.
///
/// `root` is a parameter rather than [`content_root`]'s answer because that answer
/// is relative to the process's working directory: the binary starts in the
/// repository root and a test binary does not.
///
/// # Errors
///
/// Returns [`PreparationError`] for any step that could not complete. A failed
/// mesh fails the replay: half a world is not a picture anybody should be shown.
pub fn prepare_scene(root: &Path) -> Result<PreparedScene, PreparationError> {
    let mut registry = BlockRegistry::new();
    registry.apply(&TomlFileDefinitionSource::new(root.to_owned()))?;

    let world = ReplayWorld::generate(mc_sim::REPLAY_SEED, &registry)?;
    let meshed = mesh_all(&world, &registry)?;
    let layers = TextureLayers::resolve(&texture_keys(&meshed)?);

    Ok(PreparedScene {
        scene: scene_of(&meshed, &layers)?,
        layers,
        meshed,
        world,
        registry: Arc::new(registry),
    })
}

/// Packs meshed sections into one scene the renderer can be handed.
///
/// **The half of the preparation an edit repeats**, which is why it is a
/// function rather than four statements inside the sequence above: a re-mesh
/// splices its sections back into the meshed list and then needs exactly this,
/// and a second spelling of it would be a second answer to what the renderer is
/// shown.
///
/// It touches no device, opens no window and spawns no thread, so what a scene
/// is made of can be asserted with none of the three present.
///
/// **The layers are a parameter and are never re-resolved here.** They are
/// assigned from the *initially meshed* quads' keys; resolving over every
/// registered block instead would insert a key at position 0 and shift every
/// layer index, which rewrites every committed golden frame.
///
/// # Errors
///
/// Returns [`PreparationError::Geometry`] when a section cannot be packed and
/// [`PreparationError::Scene`] when the packed sections cannot be assembled.
pub fn scene_of(
    meshed: &[SectionQuads],
    layers: &TextureLayers,
) -> Result<SceneGeometry, PreparationError> {
    let geometry = meshed
        .iter()
        .map(|section| {
            build_section_geometry(&section.quads, SectionOrigin::new(section.origin), layers)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SceneGeometry::assemble(geometry)?)
}

/// Every texture key the meshed world's quads reference.
fn texture_keys(meshed: &[SectionQuads]) -> Result<BTreeSet<TextureKey>, NamespacedIdError> {
    meshed
        .iter()
        .flat_map(|section| section.quads.iter())
        .map(|quad| TextureKey::parse(quad.block.as_str()))
        .collect()
}

/// A scene holding nothing, which is what the frame path draws from until the
/// worker above lands.
///
/// It exists so the snapshot handed to the renderer is the same shape in both
/// phases; the phase, not the emptiness, is what decides whether terrain is
/// drawn.
pub fn empty_scene() -> Arc<SceneGeometry> {
    Arc::new(SceneGeometry::default())
}
