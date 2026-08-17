//! What a content change becomes: the client's share of taking up a content set
//! read while the game was running.
//!
//! The client decides none of it. Whether a candidate is admitted, what the swap
//! replaces and which block a player is left holding are the simulation's
//! answers, reached through the one door [`mc_sim::reload`] publishes.

use std::sync::Arc;

use mc_render::geometry::scene::SceneGeometry;
use mc_render::window::rendered;
use mc_sim::content::LoadedContent;
use mc_sim::reload::{ContentReload, ReloadRefusal, ReloadStep};
use mc_sim::simulation::{Accepted, PublishedContent, Simulation};
use mc_sim::world::{Clearing, SectionKey};

use crate::content::ContentView;
use crate::remesh::{Collecting, RemeshError, Remeshed, Remesher};
use crate::upload::Unuploaded;

use super::Session;

/// What one finished re-mesh batch left for the frame path to do.
///
/// `Discarded` is its own arm rather than folded into `NothingYet`: two arms
/// reading as "no work" would let a worker that had not finished satisfy a
/// scenario about a discard.
#[derive(Debug)]
pub enum Remeshing {
    /// The worker has not finished a batch.
    NothingYet,
    /// A scene to upload.
    Show(Arc<SceneGeometry>),
    /// A batch meshed against content that has stopped serving. **Its sections are
    /// already back among the ones waiting to be meshed.**
    Discarded,
    /// A batch that could not be turned into a scene.
    Report(RemeshError),
    /// The re-mesh worker is gone, so no edit will be shown for the rest of the run.
    ///
    /// Its own arm rather than folded into `NothingYet` for the reason
    /// [`Collecting`] states: waiting is the repair for one of them and no repair at
    /// all for the other.
    WorkerGone,
}

/// What a person reads above a refused reload's own chain.
///
/// **Declared here rather than at the `eprintln!` that writes it**, which lives in a
/// file nothing in this workspace can run — so this is the only place the sentence
/// can be both printed and asked for.
///
/// **It is quoted verbatim by `docs/modding/hot-reload.md`, and a guard holds the page
/// to what a run prints.** Rewording it takes that page with it: change both, or the
/// guard reddens and tells you which. That is the whole reason the string is not
/// written at its call site.
pub const CONTENT_NOT_TAKEN_UP: &str = "the content root could not be taken up";

/// What one tick boundary did about the content root, for whoever reports it and
/// draws what it produced.
#[derive(Debug)]
pub enum ReloadReport {
    /// A candidate was refused, in the words a person reads — the whole chain,
    /// rendered by the one renderer.
    Refused(String),
    /// A candidate was taken up: this is the content now serving, and these are
    /// the array-texture layers it states.
    ///
    /// Built here rather than in the frame path so that the appended layer is a
    /// value something can read: nothing in this workspace constructs a window,
    /// so a layer assembled in `App` is unassertable. Handed over as
    /// [`Unuploaded`] because nothing in this workspace can check that the frame
    /// path uploads them either.
    Accepted {
        content: Arc<PublishedContent>,
        layers: Unuploaded,
        /// What the swap did about a player the new solidity left inside a block,
        /// on its way to the one place that says so to a person.
        clearing: Clearing,
    },
}

impl Session {
    /// Takes up `candidate` as the content this session now plays.
    ///
    /// **The block a client holds is written back here and nowhere else.** A
    /// re-derivation that never reached this field would leave the client
    /// displaying and placing the block it held before, with the simulation
    /// perfectly correct about which one it should be.
    ///
    /// `None` when there is no world yet, exactly as [`tick`](Session::tick) has
    /// nothing to advance then.
    ///
    /// # Errors
    ///
    /// Returns whatever the simulation turned the candidate away with, having
    /// changed nothing.
    pub fn adopt_content(
        &mut self,
        candidate: LoadedContent,
    ) -> Option<Result<Accepted, ReloadRefusal>> {
        let answered = mc_sim::reload::adopt_at_tick_boundary(self.simulation.as_mut()?, candidate);
        if let Ok(accepted) = &answered {
            self.holding = Some(accepted.holding.clone());
        }
        Some(answered)
    }

    /// Watches a content root from now on, taking up what it declares when it
    /// changes.
    pub fn attach_reload(&mut self, reload: ContentReload) {
        self.reload = Some(reload);
    }

    /// What the last tick boundary did about the content root, once.
    ///
    /// Take-once, the shape `pending_action` already has: a report read twice
    /// would print a refusal on every frame after the save that caused it.
    #[must_use]
    pub fn take_reload_report(&mut self) -> Option<ReloadReport> {
        self.reload_report.take()
    }

    /// Crosses one tick boundary of the content root, stashing what it did.
    ///
    /// **Called from [`tick`](Session::tick) after the tick has been advanced**,
    /// so a candidate is taken up between two ticks and never during one. There
    /// is nothing to do while the world is still generating: a swap *is* a tick
    /// boundary, and there is no boundary until a simulation exists.
    pub(super) fn cross_reload_boundary(&mut self) {
        let Some((reload, simulation)) = self.reload.as_mut().zip(self.simulation.as_mut()) else {
            return;
        };
        self.reload_report = match reload.at_tick_boundary(simulation) {
            ReloadStep::Nothing => None,
            ReloadStep::Refused(refused) => Some(ReloadReport::Refused(rendered(&refused))),
            ReloadStep::Accepted(accepted) => {
                self.holding = Some(accepted.holding.clone());
                let content = simulation.content();
                Some(ReloadReport::Accepted {
                    layers: Unuploaded::of(ContentView::of(&content.resolved).into_layers()),
                    clearing: accepted.clearing,
                    content,
                })
            }
        };
    }

    /// Collects whatever the re-mesh worker has finished, handing a discarded
    /// batch's sections back before returning.
    ///
    /// The hand-back happens here rather than in a caller because dropping those
    /// sections leaves them stale for the whole run with no error anywhere.
    /// [`Remeshing::Discarded`] carries nothing for the same reason.
    ///
    /// **Do not also submit from here.** Handed-back sections would go straight
    /// into flight and `take_remesh_work` would find nothing, which is the only
    /// observable the discard has.
    pub fn collect_remesh(&mut self, remesher: &mut Remesher) -> Remeshing {
        match remesher.collect() {
            Collecting::NothingYet => Remeshing::NothingYet,
            Collecting::WorkerGone => Remeshing::WorkerGone,
            Collecting::Finished(Remeshed::Scene(scene)) => Remeshing::Show(scene),
            Collecting::Finished(Remeshed::Failed(failure)) => Remeshing::Report(failure),
            Collecting::Finished(Remeshed::Superseded(stale)) => {
                self.mark_for_remesh(stale.into_keys());
                Remeshing::Discarded
            }
        }
    }

    /// Records `keys` as needing to be meshed again.
    ///
    /// Private: offered as a call of its own, a fixture makes the hand-back
    /// itself and stops grading whether the product does.
    fn mark_for_remesh(&mut self, keys: Vec<SectionKey>) {
        if let Some(simulation) = self.simulation.as_mut() {
            simulation.mark_for_remesh(keys);
        }
    }

    /// The content the simulation is publishing, for whoever draws with it.
    ///
    /// An owned `Arc` rather than a borrow, the same no-borrow shape
    /// [`held_block`](Session::held_block) has: a reader is handed a value it
    /// cannot ask the session a second question through.
    ///
    /// `None` for exactly as long as there is no simulation.
    #[must_use]
    pub fn content(&self) -> Option<Arc<PublishedContent>> {
        self.simulation.as_ref().map(Simulation::content)
    }
}
