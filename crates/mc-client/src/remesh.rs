//! Turning an edit into a scene, on a thread that is neither the tick's nor the
//! frame's.
//!
//! **The worker hands back a finished scene rather than a bag of quads**, and
//! that is the whole shape of this module. It owns the meshed sections and the
//! texture resolution for the run, so it meshes, splices *and*
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

use mc_core::content::ContentSerial;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::texture::TextureResolution;
use mc_sim::replay::{PrepareError, SectionQuads, SpliceError, remesh, splice};
use mc_sim::world::{RemeshWork, SectionKey};
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
///
/// Three answers rather than two, because a batch can now be **correct and
/// useless**: a reload landing while it was in flight leaves it meshed against
/// content that has stopped serving, and drawing it would show a picture of a
/// content set nobody is playing.
#[derive(Debug)]
pub enum Remeshed {
    Scene(Arc<SceneGeometry>),
    /// The batch was meshed against content that is no longer serving, so its
    /// sections are still waiting to be meshed. See [`Stale`].
    Superseded(Stale),
    Failed(RemeshError),
}

/// Sections a discarded batch would have meshed, which are still stale.
///
/// A value that must be consumed rather than a list a caller may forget:
/// dropping these keys leaves those sections stale for the rest of the run with
/// no error anywhere. `#[must_use]` plus the denied `unused_variables` lint is
/// what makes ignoring one a build failure.
#[derive(Debug)]
#[must_use = "these sections are still stale; `Session::collect_remesh` is what hands them back"]
pub struct Stale {
    keys: Vec<SectionKey>,
}

impl Stale {
    /// The sections, taken, for the one thing that hands them back.
    ///
    /// The only way in or out. There is deliberately no borrowing accessor: one
    /// makes an assertion on *which* sections were discarded easy to write in
    /// place of one on their being meshed again.
    ///
    /// **`pub(crate)` with exactly one caller is load-bearing.** Forgetting the
    /// hand-back fails the build twice — `unused variable: stale` *and* `method
    /// into_keys is never used`, the second because dropping the call orphans this
    /// method's only consumer. Widening this to `pub` keeps the first diagnostic
    /// and silently loses the second.
    pub(crate) fn into_keys(self) -> Vec<SectionKey> {
        self.keys
    }
}

/// What one ask of the worker came to.
///
/// **Three answers, because the two absences want opposite repairs.** "Nothing has
/// finished yet" is repaired by asking again; "the worker is gone" never is, and no
/// amount of waiting turns one into the other. Reported as a single `None` they were
/// indistinguishable, so a worker thread that ended read as patience forever — a wait
/// that had already spent 15 s on 9 ms of work could not say which of the two it was
/// looking at, and neither could the frame path.
#[derive(Debug)]
pub enum Collecting {
    /// Nothing has finished since the last ask. The worker may still hold a batch —
    /// [`is_free`](Remesher::is_free) is what says whether it does.
    NothingYet,
    /// The channel is gone, so no batch will arrive now or later.
    WorkerGone,
    /// A batch finished, and this is what it came to.
    Finished(Remeshed),
}

/// The worker, and whether it is busy.
///
/// It holds no borrow of the simulation, the session or the renderer: a batch
/// arrives as an owned value and a scene comes back as one, which is what lets
/// the tick and the frame carry on while it runs.
#[derive(Debug)]
pub struct Remesher {
    batches: Sender<Message>,
    scenes: Receiver<Rebuilt>,
    busy: bool,
    /// The keys and the serial of the batch the worker currently holds.
    in_flight: Option<(Vec<SectionKey>, ContentSerial)>,
    /// Which accepted content set is serving now.
    ///
    /// Held here rather than beside the retained resolution: `Retained` is moved into
    /// the worker, and a serial the worker only updates when it dequeues would
    /// make the mismatch branch unreachable.
    serving: ContentSerial,
}

/// What the worker is sent, on one ordered channel so that a resolution retired
/// before a batch was queued is in place before that batch is meshed.
enum Message {
    Batch(RemeshWork),
    Retire(TextureResolution),
}

/// What the worker sends back: a scene, or the reason there is none.
enum Rebuilt {
    Scene(Arc<SceneGeometry>),
    Failed(RemeshError),
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
    pub fn spawn(retained: Retained, serving: ContentSerial) -> Self {
        let (batches, work) = channel::<Message>();
        let (finished, scenes) = channel::<Rebuilt>();
        drop(std::thread::spawn(move || {
            rebuild_batches(retained, &work, &finished);
        }));
        Self {
            batches,
            scenes,
            busy: false,
            in_flight: None,
            serving,
        }
    }

    /// Retires the resolution a scene is packed against, and records which
    /// content set is serving now.
    ///
    /// It travels on the **same ordered channel** the batches use, which is what
    /// makes "the worker was told before it meshed anything with it" true of the
    /// next batch without a handshake.
    pub fn retire(&mut self, resolution: TextureResolution, serial: ContentSerial) {
        self.serving = serial;
        drop(self.batches.send(Message::Retire(resolution)));
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
        let carried = (work.keys().collect::<Vec<_>>(), work.serial());
        self.busy = self.batches.send(Message::Batch(work)).is_ok();
        self.in_flight = self.busy.then_some(carried);
    }

    /// What the worker has for whoever asked.
    ///
    /// Never blocks: a frame that arrives before the worker does draws what it
    /// already had.
    pub fn collect(&mut self) -> Collecting {
        match self.scenes.try_recv() {
            Ok(finished) => {
                self.busy = false;
                Collecting::Finished(self.judged(finished))
            }
            Err(TryRecvError::Empty) => Collecting::NothingYet,
            Err(TryRecvError::Disconnected) => {
                self.busy = false;
                self.in_flight = None;
                Collecting::WorkerGone
            }
        }
    }

    /// What a finished batch is, given which content set is serving now.
    fn judged(&mut self, finished: Rebuilt) -> Remeshed {
        let carried = self.in_flight.take();
        match finished {
            Rebuilt::Failed(refused) => Remeshed::Failed(refused),
            Rebuilt::Scene(scene) => match carried {
                Some((keys, drained)) if drained != self.serving => {
                    Remeshed::Superseded(Stale { keys })
                }
                _ => Remeshed::Scene(scene),
            },
        }
    }
}

/// Everything a re-mesh needs for the rest of the run, which nothing retained
/// before.
///
/// The meshed sections are the list a re-meshed section is spliced back into and
/// the resolution is what a scene is packed against. Both are handed to the
/// worker rather than kept beside it: they are what it works on, and a copy on
/// each side is a second answer waiting to disagree.
///
/// **The whole retained list is re-packed on every batch, against whatever
/// resolution the worker currently holds.** That is what makes a section nobody
/// re-meshed draw the keys the content serving now declares, and it is the one
/// path on which quads outlive the resolution they were meshed under.
///
/// **No registry here, and that absence is load-bearing.** A batch carries the
/// registry its own world was resolved against, so meshing against a second
/// opinion is unspellable rather than checked.
#[derive(Debug)]
pub struct Retained {
    pub meshed: Vec<SectionQuads>,
    pub resolution: TextureResolution,
}

impl Retained {
    /// The scene that results from applying `work` to what is retained.
    ///
    /// # Errors
    ///
    /// Returns [`RemeshError`] naming which of the three steps refused.
    fn rebuilt(&mut self, work: &RemeshWork) -> Result<Arc<SceneGeometry>, RemeshError> {
        splice(&mut self.meshed, remesh(work)?)?;
        Ok(Arc::new(scene_of(&self.meshed, &self.resolution)?))
    }
}

/// Rebuilds every batch that arrives, until the sending end goes away.
fn rebuild_batches(mut retained: Retained, work: &Receiver<Message>, finished: &Sender<Rebuilt>) {
    while let Ok(message) = work.recv() {
        let batch = match message {
            Message::Retire(resolution) => {
                retained.resolution = resolution;
                continue;
            }
            Message::Batch(batch) => batch,
        };
        let rebuilt = match retained.rebuilt(&batch) {
            Ok(scene) => Rebuilt::Scene(scene),
            Err(refused) => Rebuilt::Failed(refused),
        };
        if finished.send(rebuilt).is_err() {
            return;
        }
    }
}
