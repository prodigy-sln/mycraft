//! Preparing the replay's render input, end to end, from the composition root.
//!
//! **This lives here because it is the only place it can.** The scenarios these
//! fixtures serve compare *packed vertex bytes*, which only the renderer
//! produces, over a world that only the simulation generates — and neither of
//! those two crates may resolve the other, in any dependency kind, including a
//! dev-dependency. The client is the crate that depends on both, so it is where
//! a test that needs both belongs. The lines it exercises live in the renderer
//! and are counted there.
//!
//! **Nothing here restates the pipeline; it calls the client's.** This module
//! used to carry its own generate → mesh → resolve → pack → assemble sequence
//! beside `mc_client::startup`'s, with nothing asserting the two agreed — so the
//! goldens, which are this spec's only automated evidence that the renderer draws
//! the right picture, were shot through a path the product does not run
//! (conductor ruling 38). Everything below now goes through
//! [`mc_client::startup::prepare_scene`], the one statement of that sequence.
//!
//! **It is [`mc_client::startup::prepare_scene`] and never `spawn_preparation`.**
//! The byte-determinism scenarios decide how many `rayon` workers mesh the world
//! by preparing inside a pool, and a `std::thread` spawned inside `install` does
//! not inherit that pool — measured at 1 worker inside and 16 on the spawned
//! thread. Going through the spawning entry point would leave both worker counts
//! meshing on the global pool and comparing bytes that could never disagree.
//! `prepare_scene` runs on the calling thread, so the pool the caller built is
//! the pool `mesh_all`'s `par_iter` sees.
//!
//! The content root is located from the repository rather than taken from
//! `mc_client::startup::content_root`: that answer is relative to the process's
//! working directory, and a test binary starts in its own package directory
//! rather than where the shipped binary starts.

// Each test binary links this whole module and uses a subset of it.
#![allow(dead_code)]

/// Content roots built from the shipped one, for scenarios about what a root
/// declares and about what it stops declaring.
pub mod content;
/// Rendering one tick of the replay offscreen, at the declared capture size.
pub mod frames;
/// Judging a rendered capture against a committed golden, shared by the two
/// golden binaries so the mint path and the verify path cannot differ.
pub mod goldens;
/// Rendering one tick of the replay with a HUD over it, through the frame call
/// the windowed client makes.
pub mod hud_frames;
/// An independent prediction of what the player's camera sees, marched through
/// the world's own voxels.
pub mod oracle;
/// Rendering one waiting frame with the debug overlay's readout over it, in the
/// one fixture module that may carry a readout and shoots no golden.
pub mod overlay_frames;
/// An independent prediction of where a content root's HUD declarations land and
/// what they paint, sharing no code with the composition it judges.
pub mod prediction;
/// Assertions about a captured frame that come from nowhere near the renderer.
pub mod probe;
/// Reading a rendered swatch against the colours the texture behind it is made
/// of.
pub mod swatch;

use std::error::Error;
use std::path::{Path, PathBuf};

use mc_core::block::BlockRegistry;
use mc_render::window::{Ending, report};
use mc_world::content::LuauFileDefinitionSource;

/// The replay's assembled scene and the texture layers its blocks resolved to, as
/// the client's own startup defines them.
///
/// Re-exported rather than redeclared: a second type with the same two fields is
/// how the two pipelines drifted apart in the first place.
pub use mc_client::startup::PreparedScene;

/// The error type every test in this suite propagates with `?`.
pub type TestResult = Result<(), Box<dyn Error>>;

/// The render input one preparation of the replay produces.
///
/// The packed vertex buffer and the section table, as the bytes that reach the
/// GPU. There is no separate index buffer to compare: indices are compacted on
/// the GPU from the section table's `first_quad` and `quad_count`, so the
/// section table *is* the index-side render input on the CPU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderInput {
    pub vertices: Vec<u8>,
    pub sections: Vec<u8>,
}

/// The repository's own root, located upwards from the crate this test binary
/// was built for.
///
/// # Errors
///
/// Returns an error if the manifest directory has no grandparent.
pub fn repository_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the crate manifest directory has no repository root above it")?
        .to_owned())
}

/// The directory the shipped content is read from, located from the repository
/// rather than from the working directory this test binary happens to start in.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located.
pub fn content_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(repository_root()?.join("content").join("base"))
}

/// A registry holding exactly what this repository ships as content.
///
/// The preparation below builds its own from [`content_root`], as the client
/// does; this is for a test that needs the registry itself rather than a scene.
///
/// # Errors
///
/// Returns an error if the content root cannot be read or does not apply.
pub fn content_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(content_root()?))?;
    Ok(registry)
}

/// The render input the replay world produces, prepared from scratch.
///
/// Runs on the calling thread, so a caller that prepared inside a `rayon` pool
/// meshed the world on that pool's workers.
///
/// # Errors
///
/// Returns an error if the world cannot be generated, meshed, packed or
/// assembled.
pub fn prepare() -> Result<RenderInput, Box<dyn Error>> {
    let prepared = prepare_scene()?;
    Ok(RenderInput {
        vertices: prepared.scene.vertex_bytes(),
        sections: prepared.scene.section_bytes(),
    })
}

/// The replay's scene, prepared through the pipeline the client runs at startup —
/// the same function, not the same steps.
///
/// # Errors
///
/// Returns an error if the content cannot be read, or the world cannot be
/// generated, meshed, packed or assembled.
pub fn prepare_scene() -> Result<PreparedScene, Box<dyn Error>> {
    Ok(mc_client::startup::prepare_scene(&content_root()?)?)
}

/// What a run ending in `ending` writes to the error stream, captured whole.
///
/// **Captured from the shipped reporting rather than composed here.** A refusal
/// is the only thing a mod author with a broken file ever gets, and a suite that
/// rendered one of its own would agree with itself while the client printed a
/// single sentence — which is exactly the state these scenarios exist to leave.
///
/// # Errors
///
/// Returns an error if the sink refuses the bytes, which a `Vec` does not, or if
/// what was written is not text.
pub fn reported(ending: &Ending) -> Result<String, Box<dyn Error>> {
    let mut sink = Vec::new();
    report(ending, &mut sink)?;
    Ok(String::from_utf8(sink)?)
}

/// What the client writes when preparing a scene from the content root at `root`
/// is refused.
///
/// The seam a refusal is observable at without a GPU or a display server: the
/// client's own preparation produces the failure, the shipped reporting turns it
/// into text, and the text is what a person reads.
///
/// # Errors
///
/// Returns an error if the root was accepted. A fixture whose declaration was
/// taken is not a fixture a refusal can be read from, and a test searching an
/// empty string for what it should contain would be reporting the fixture rather
/// than the printing.
pub fn refusal_printed_over(root: &Path) -> Result<String, Box<dyn Error>> {
    match mc_client::startup::prepare_scene(root) {
        Ok(_) => Err(format!(
            "this scenario needs the content root at {} to refuse the run, and it prepared a \
             scene instead. There is no refusal to read, so every word this asks for would be \
             missing for a reason that has nothing to do with what is printed",
            root.display()
        )
        .into()),
        Err(refused) => reported(&Ending::failed(&refused, "")),
    }
}
