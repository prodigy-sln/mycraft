//! Turning an edit into a scene, on a thread that is neither the tick's nor the
//! frame's.
//!
//! **The worker hands back a finished scene rather than a bag of quads**, and
//! that is the whole shape of this module. It owns the meshed sections, the
//! texture layers and the registry for the run, so it meshes, splices *and*
//! packs, and the frame path's entire share of an edit is one `upload_scene`.
//! Splicing and packing on the frame path instead would put a whole-scene
//! rebuild — every section packed again and a fresh byte pass over some eleven
//! thousand vertices — on the render thread once per click, which is exactly what
//! `mc-render`'s own rules forbid.
//!
//! **A failed batch drops and is reported; it never fails the run.** This is the
//! opposite of the rule for preparation, deliberately: preparation has no
//! previous picture, so half a world is nothing anybody should be shown, whereas
//! a re-mesh that fails still has the picture it had a moment ago. The sections
//! that batch carried stay stale until something dirties them again, which is a
//! wrong picture rather than no picture — and it is the lesser of the two.
//!
//! **One batch is in flight at a time.** Edits made while the worker is busy
//! accumulate in the world's own dirty set, because nothing drains it until the
//! worker is free, and the set is keyed per section — so a player digging
//! continuously through a slow batch collects sections to re-mesh rather than
//! batches to run.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use mc_core::block::BlockRegistry;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::texture::TextureLayers;
use mc_sim::replay::{PrepareError, SectionQuads, SpliceError, remesh, splice};
use mc_sim::world::RemeshWork;
use thiserror::Error;

use crate::startup::{PreparationError, scene_of};

/// Why an edit could not be turned into a scene.
#[derive(Debug, Error)]
pub enum RemeshError {
    #[error("a section could not be meshed again after an edit")]
    Mesh(#[from] PrepareError),
    #[error("a re-meshed section has no place in the prepared scene")]
    Splice(#[from] SpliceError),
    #[error("the re-meshed sections could not be packed into a scene")]
    Pack(#[from] PreparationError),
}

/// What a finished batch came to.
pub type Remeshed = Result<Arc<SceneGeometry>, RemeshError>;

/// The worker, and whether it is busy.
///
/// It holds no borrow of the simulation, the session or the renderer: a batch
/// arrives as an owned value and a scene comes back as one, which is what lets
/// the tick and the frame carry on while it runs.
#[derive(Debug)]
pub struct Remesher {
    batches: Sender<RemeshWork>,
    scenes: Receiver<Remeshed>,
    busy: bool,
}

impl Remesher {
    /// Starts a worker owning everything a re-mesh needs for the rest of the
    /// run.
    ///
    /// **The join handle is deliberately dropped.** The worker's loop ends when
    /// the sending end goes away, which is when this value is dropped, and there
    /// is nothing for a shutting-down client to collect from it — the only thing
    /// it produces is a scene nobody will draw. Keeping a handle to join would be
    /// a field read by nothing, on the one path where waiting is exactly what a
    /// closing window must not do.
    #[must_use]
    pub fn spawn(retained: Retained) -> Self {
        let (batches, work) = channel::<RemeshWork>();
        let (finished, scenes) = channel::<Remeshed>();
        drop(std::thread::spawn(move || {
            rebuild_batches(retained, &work, &finished);
        }));
        Self {
            batches,
            scenes,
            busy: false,
        }
    }

    /// Whether a batch may be handed over, which it may not while one is still
    /// running.
    #[must_use]
    pub const fn is_free(&self) -> bool {
        !self.busy
    }

    /// Hands `work` to the worker.
    ///
    /// A worker that has gone away leaves this free again rather than busy
    /// forever: nothing will arrive to clear the flag, and a client that stopped
    /// asking would stop showing edits with no report of why.
    pub fn submit(&mut self, work: RemeshWork) {
        self.busy = self.batches.send(work).is_ok();
    }

    /// The scene the last batch produced, if it has finished.
    ///
    /// Never blocks: a frame that arrives before the worker does draws what it
    /// already had.
    pub fn collect(&mut self) -> Option<Remeshed> {
        match self.scenes.try_recv() {
            Ok(finished) => {
                self.busy = false;
                Some(finished)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.busy = false;
                None
            }
        }
    }
}

/// Everything a re-mesh needs for the rest of the run, which nothing retained
/// before.
///
/// The meshed sections are the list a re-meshed section is spliced back into,
/// the layers are what a scene is packed against, and the registry is what a
/// section's blocks are meshed through. All three are handed to the worker
/// rather than kept beside it: they are what it works on, and a copy on each
/// side is a second answer waiting to disagree.
#[derive(Debug)]
pub struct Retained {
    pub meshed: Vec<SectionQuads>,
    pub layers: TextureLayers,
    pub registry: Arc<BlockRegistry>,
}

impl Retained {
    /// The scene that results from applying `work` to what is retained.
    ///
    /// # Errors
    ///
    /// Returns [`RemeshError`] naming which of the three steps refused.
    fn rebuilt(&mut self, work: &RemeshWork) -> Result<Arc<SceneGeometry>, RemeshError> {
        splice(&mut self.meshed, remesh(work, &self.registry)?)?;
        Ok(Arc::new(scene_of(&self.meshed, &self.layers)?))
    }
}

/// Rebuilds every batch that arrives, until the sending end goes away.
fn rebuild_batches(
    mut retained: Retained,
    work: &Receiver<RemeshWork>,
    finished: &Sender<Remeshed>,
) {
    while let Ok(batch) = work.recv() {
        if finished.send(retained.rebuilt(&batch)).is_err() {
            return;
        }
    }
}
