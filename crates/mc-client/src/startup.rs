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

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::content::{LayerAssignment, LayerBudget};
use mc_core::hud::{HudLayout, HudLoadError};
use mc_core::id::TextureKey;
use mc_render::geometry::scene::{SceneError, SceneGeometry};
use mc_render::geometry::{GeometryError, SectionOrigin, build_section_geometry};
use mc_render::gpu::RendererError;
use mc_render::texture::TextureResolution;
use mc_render::texture::supplied::SuppliedTexels;
use mc_sim::content::{ContentError, LoadedContent};
use mc_sim::persistence::LaunchError;
use mc_sim::replay::{PrepareError, ReplayWorld, SectionQuads, WorldGenError, mesh_all};
use mc_world::persistence::{Acceptance, LoadError};
use thiserror::Error;

use crate::content::ContentView;
use crate::textures::{TextureSetError, built_set, refusal_for};

/// What a player types to load a save whose blocks are no longer what they were.
///
/// **One spelling, named once**, because the parse that accepts it and the
/// refusal that tells a player about it have to agree: a message quoting a flag
/// nothing accepts is worse than no message at all, since it reads as a way out
/// and is not one.
const LOAD_CHANGED_BLOCKS: &str = "--load-changed-blocks";

/// What a contributor runs to produce the texture set a launch reads.
///
/// **One spelling, named once**, for the reason `LOAD_CHANGED_BLOCKS` carries:
/// a message quoting a command nothing accepts reads as a way out and is not
/// one. `README.md` and `docs/modding/voxel-models.md` quote this same string,
/// and `pub` is what lets a test compare the four rather than compare the
/// constant with a copy of itself.
pub const BUILD_THE_TEXTURE_SET: &str = "cargo run -p voxforge -- build content/base/textures.toml";

/// What one preparation of the replay produces: the packed scene, the resolution
/// its blocks were packed against, and the world and registry both were built
/// from.
///
/// The resolution is carried alongside rather than discarded because a scene
/// records which array *layer* each corner draws from and never which key that
/// layer came from, so uploading the array texture needs both halves — and a
/// section re-packed later needs the declarations as well.
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
/// The HUD elements the same content root declared are carried for a fourth
/// reason: they are read here, beside the blocks, because both are what the
/// shipped content says and neither is something the frame path may decide. The
/// layout is shared rather than handed over so that the composition a frame
/// performs and the declarations a run was started with cannot be two things.
#[derive(Debug)]
pub struct PreparedScene {
    pub scene: SceneGeometry,
    pub resolution: TextureResolution,
    pub meshed: Vec<SectionQuads>,
    pub world: ReplayWorld,
    pub registry: Arc<BlockRegistry>,
    pub hud: Arc<HudLayout>,
    /// The level-zero texels the content root's built art offers, one entry per
    /// key it covers.
    ///
    /// Carried for the same reason the resolution is: a scene records which
    /// array *layer* each corner draws from and never what fills that layer, so
    /// whoever gives the array texture to a device needs both halves. A key the
    /// set covers nothing for is filled from the texture generated for it, which
    /// is a mod author's first block and never a refusal.
    pub texels: SuppliedTexels,
}

/// Why a preparation — of the generated scene here, or of a launch in
/// [`crate::launch`] — could not be completed.
///
/// **One enum for both paths**, because `--load-changed-blocks` is spelled in one
/// constant here: the parse that accepts the flag and the refusal that advertises
/// it must not be able to disagree, and the refusal is interpolated inside this
/// type's own `Display`.
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
    /// The content root's HUD declarations were refused.
    ///
    /// **This stops the launch rather than degrading it**, which is the same
    /// trade every variant here makes and is worth stating because the obvious
    /// alternative — start anyway, with no HUD — is the one that costs a player
    /// the most. A window that opens with a silently missing crosshair tells
    /// nobody why; a refusal quoting the file, the element and the field tells a
    /// content author exactly what to fix. There is no error screen and no
    /// HUD-less mode to fall back to.
    ///
    /// The refusal underneath already names all three, so this adds no sentence
    /// of its own: a message restating what its cause says would be a second
    /// place for the two to disagree.
    #[error("the shipped HUD declarations could not be read")]
    Hud(#[from] HudLoadError),
    #[error("the replay world could not be generated")]
    WorldGen(#[from] WorldGenError),
    #[error("the replay world could not be meshed")]
    Mesh(#[from] PrepareError),
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
    /// **The sentence a player whose blocks have changed needs is not in this
    /// message, and it used to be.** It is asked for separately, through
    /// [`way_out`](Self::way_out), and said after the whole chain rather than
    /// wrapped around the front of it. A way out is not a cause: a report is a
    /// failure and every failure under it, so a message appending advice to its
    /// own source strands that advice at the top of the report — before the
    /// refusal it answers — and has the source read out twice besides.
    ///
    /// Transparent, because this level knows nothing the launch does not. The
    /// text a player reads is unchanged to the byte on the one path that offers
    /// a way out, leading `". "` included.
    #[error(transparent)]
    Launch(#[from] LaunchError),
    #[error(
        "the content registers no solid block, so a player would have nothing to place; the \
         block a client holds is the first solid one in registration order"
    )]
    NothingToPlace,
    #[error("the thread preparing the replay ended without producing a scene or an error")]
    WorkerLost,
    /// The content root needs more array-texture layers than a session has.
    ///
    /// Transparent for the same reason [`Launch`](Self::Launch) is: the sentence
    /// naming the count, the bound and the way out is declared once, beside the
    /// budget, and this level knows nothing to add to it. Unreachable from a
    /// launch, which spends nothing and would need a root declaring more than
    /// the budget's worth of texture keys — it is here because the type has to be
    /// total over what the one content door can refuse.
    #[error(transparent)]
    Layers(#[from] LayerBudget),
    /// The content root states art and nothing has been built from it.
    ///
    /// **This and [`TextureSetImageMissing`](Self::TextureSetImageMissing) must
    /// never come to be said the same way.** *The build step was not run* is a
    /// refusal one command clears: nothing about the content is wrong, its
    /// derived half simply is not there. *This key was never authored* is the
    /// ordinary state of a mod author's first block and costs them a generated
    /// texture rather than a launch, so it is never a refusal at all. A client
    /// that said one sentence for both would send whoever forgot to run the
    /// build looking for a declaration they never wrote.
    #[error("the generated texture set is not there; run `{BUILD_THE_TEXTURE_SET}`")]
    TextureSetAbsent,
    #[error(
        "the generated texture set is stale against its sources; run `{BUILD_THE_TEXTURE_SET}`"
    )]
    TextureSetStale,
    /// A source the set was built from is no longer where the index recorded it.
    ///
    /// **Its own variant rather than an `Option<PathBuf>` inside the one above**,
    /// because a `thiserror` format string cannot render a field conditionally
    /// and a message ending in `built from ``None``` is worse than two variants.
    ///
    /// The field is `missing` and not `source`: `thiserror` reads a field of that
    /// name as the error's cause, and the variant does not compile with it.
    #[error(
        "the generated texture set was built from `{missing}`, which is no longer there; run \
         `{BUILD_THE_TEXTURE_SET}`",
        missing = missing.display()
    )]
    TextureSetSourceMissing { missing: PathBuf },
    #[error(
        "the generated texture set names the art for `{key}` as `{image}`, which is not there; run \
         `{BUILD_THE_TEXTURE_SET}`",
        key = key.as_str(),
        image = image.display()
    )]
    TextureSetImageMissing { key: TextureKey, image: PathBuf },
    /// The set could not be read at all, which is not one of its verdicts.
    ///
    /// Transparent for the reason [`Launch`](Self::Launch) is: the refusal
    /// underneath names the file and the record at fault, and a sentence here
    /// restating it would be a second place for the two to disagree.
    #[error(transparent)]
    TextureSetUnreadable(#[from] TextureSetError),
}

impl PreparationError {
    /// What to tell a player beyond what this refusal already says — the way out
    /// where there is one, and the empty string where there is not.
    ///
    /// **A way out is not a link in the causal chain**, which is why it is asked
    /// for separately rather than carried in a message. A cause says what
    /// happened; `--load-changed-blocks` says what to do about it, so it belongs
    /// after the whole chain rather than at the top of it, where a message
    /// wrapping its own source would strand it before the refusal it answers.
    ///
    /// The sentence itself is unchanged and still spelled in exactly one place,
    /// so the parse that accepts the flag and the refusal that advertises it
    /// cannot disagree.
    #[must_use]
    pub fn way_out(&self) -> String {
        match self {
            Self::Launch(failure) => way_out_of(failure),
            _ => String::new(),
        }
    }
}

/// The content directory this run reads, checked to exist before anything is
/// started that would fail on it later.
///
/// **The directory is the simulation's to resolve**, not this crate's: a client
/// that works out for itself where content lives is a client that can go on to
/// read it. This asks and translates the answer into the failure a player sees.
///
/// # Errors
///
/// Returns [`PreparationError::NoContentRoot`] naming the path when it is not
/// there, which is the failure somebody running the binary from the wrong
/// directory gets — and it names the directory rather than a missing block.
pub fn shipped_content() -> Result<PathBuf, PreparationError> {
    mc_sim::content::shipped_directory().map_err(|root| PreparationError::NoContentRoot { root })
}

impl From<ContentError> for PreparationError {
    /// **Flattened rather than carried whole.** A caller matching on "the blocks
    /// were refused" keeps matching now that something else is what reads them,
    /// and the refusal a player sees is the reader's own rather than a wrapper
    /// around it. A third variant here would be a second name for a failure that
    /// already had one.
    fn from(error: ContentError) -> Self {
        match error {
            ContentError::Blocks(refusal) => Self::Content(refusal),
            ContentError::Hud(refusal) => Self::Hud(refusal),
            ContentError::Layers(refusal) => Self::Layers(refusal),
        }
    }
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
/// `root` is a parameter rather than [`shipped_content`]'s answer because that answer
/// is relative to the process's working directory: the binary starts in the
/// repository root and a test binary does not.
///
/// # Errors
///
/// Returns [`PreparationError`] for any step that could not complete. A failed
/// mesh fails the replay: half a world is not a picture anybody should be shown.
pub fn prepare_scene(root: &Path) -> Result<PreparedScene, PreparationError> {
    // First, and before a world is generated: a contributor who has not run the
    // art build is told which command to run rather than waiting out a world
    // they will not be shown.
    let (verdict, texels) = built_set(root)?;
    if let Some(refused) = refusal_for(&verdict) {
        return Err(refused);
    }
    // Asked of the simulation, which is what reads a content root. A fault here
    // fails the preparation rather than being noted and skipped: see
    // `PreparationError::Hud`.
    let LoadedContent {
        registry,
        hud,
        resolved,
        // A capture has spent no layers either, and it must produce the same
        // assignment a launch does — one door, one answer.
    } = mc_sim::content::load(root, &LayerAssignment::none())?;

    let world = ReplayWorld::generate(mc_sim::REPLAY_SEED, &registry)?;
    let meshed = mesh_all(&world, &registry)?;
    let resolution = ContentView::of(&resolved).into_resolution();

    Ok(PreparedScene {
        scene: scene_of(&meshed, &resolution)?,
        resolution,
        meshed,
        world,
        registry: Arc::new(registry),
        hud: Arc::new(hud),
        texels,
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
/// **The resolution is a parameter and is never re-resolved here**, and the reason
/// has changed even though the rule has not. The registry *does* change
/// mid-session now — hot reload replaces it whole — but a layer already assigned
/// keeps its index for the run, because that index rides inside every packed
/// vertex. So a re-mesh splicing its sections into this list still needs the same
/// layers the rest of the scene was packed against, and a second opinion about
/// them is now something that can genuinely exist rather than something that
/// could not.
///
/// # Errors
///
/// Returns [`PreparationError::Geometry`] when a section cannot be packed and
/// [`PreparationError::Scene`] when the packed sections cannot be assembled.
pub fn scene_of(
    meshed: &[SectionQuads],
    resolution: &TextureResolution,
) -> Result<SceneGeometry, PreparationError> {
    let geometry = meshed
        .iter()
        .map(|section| {
            build_section_geometry(
                &section.quads,
                SectionOrigin::new(section.origin),
                resolution,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SceneGeometry::assemble(geometry)?)
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

/// What the frame path composes until the worker above lands: the HUD of a
/// client that has not read its content yet.
///
/// **Built through the loader rather than constructed**, and asked for rather
/// than built here. Loading a layout is the simulation's, on the same terms as
/// reading a content root: a client that can construct one has a door into a
/// layout of its own, and the reason there is only one such door is that no
/// engine may put an element into a layout from Rust. A source declaring nothing
/// is a valid, empty answer, which is the one place HUD loading diverges from
/// block registration.
///
/// # Errors
///
/// Returns [`HudLoadError`] if a source declaring nothing is ever refused —
/// which is the one thing HUD loading is specified never to do. It is an error
/// rather than a panic for the reason [`SetupError::FormatVanished`] is: this
/// runs on a player's startup path, and a client that cannot start should say so
/// rather than abort.
///
/// [`SetupError::FormatVanished`]: crate::surface_setup::SetupError::FormatVanished
pub fn empty_hud() -> Result<Arc<HudLayout>, HudLoadError> {
    Ok(Arc::new(mc_sim::content::hud_before_content_is_read()?))
}
