//! What a collect came to, on either side of the seam, and how long it waits.
//!
//! # A collect waits without a bound, because there is no direction to derive one
//! from
//!
//! A re-mesh runs on a worker thread, so a collect has to be polled. It polls
//! **until the worker answers**. That is the conclusion of the same three-part
//! derivation a wall-clock constant would need — with one of the three empty and
//! another unmeasurable, which is what says no constant exists to be chosen.
//!
//! - *From below, measured.* An earlier bound of 1.02 s — the mesher's declared
//!   per-section budget times the sections a whole-world batch carries — lost a
//!   256-section batch under coverage instrumentation over the whole workspace. It
//!   was replaced by 15 s, fifteen times the measured insufficiency, and that lost
//!   one too. Four complete instrumented runs then timed the same collect at
//!   **2.677 s, 6.965 s, 10.081 s and 10.151 s**, against **63.838 ms** for the
//!   same collect on an idle machine. Those four spread 3.79× among themselves and
//!   **none of them is the failure**: a run that exceeds the bound is censored at
//!   it and known only as *longer than the bound*. So the real spread is at least
//!   5.6×, the observed maximum sits 1.48× *under* a bound the true tail is known
//!   to pass through, and every further sample measures the body again. A floor
//!   cannot be derived from data that never contains the case being bounded.
//! - *From above: nothing.* Every assertion made against this wait is a
//!   **presence** — a scene arrived, a batch was superseded, a failure was named.
//!   A longer wait can therefore only turn a red into a green that was always
//!   true, and it costs nothing on a passing run, because the loop returns the
//!   moment the worker answers. Nothing pushes down on the number at all.
//! - *The smallest difference it must still catch:* a worker that never answers,
//!   which no amount of waiting turns into an answer — so patience is not the
//!   instrument for that one either.
//!
//! **A quantity whose floor cannot be measured and whose ceiling is nothing is not
//! a constant, and picking one anyway is how a threshold gets loosened until it is
//! green.** Both bounds this file has carried failed that way, the second after
//! being widened fifteenfold, and each failure read as the property being false
//! when what had happened was that the machine was busy.
//!
//! The two absences are told apart by structure instead of by patience.
//! [`Remesher::is_free`] says whether a batch is outstanding — it is written only
//! by the thread that submits and collects, never by the worker — so a fixture that
//! submitted nothing is answered *immediately* with
//! [`Collected::NothingWasEverHandedOver`] rather than after a wait, which is
//! strictly more diagnostic than a patience was. The verdict that used to mean
//! "this machine was slow" no longer exists to be mistaken for "the property is
//! false".
//!
//! # What has a witness here and what does not
//!
//! The waiting itself is graded on both loops, and it took a batch big enough to
//! still be meshing when the first ask arrives. Reading a *small* batch back proves
//! nothing about the wait: the two-section batch the supersession scenarios submit
//! has already answered by the time they ask, so inverting either loop's
//! [`Remesher::is_free`] guard leaves every one of them green. It is the whole-world
//! batch — 256 sections, measured at 40 to 53 polls of a millisecond and never
//! fewer than 40 over twenty runs — that reaches the arm at all, and exactly one
//! test on each loop is what turns that inversion red.
//!
//! [`Collected::TheWorkerIsGone`] and [`Handled::TheWorkerIsGone`] are **reached by
//! nothing here**, and neither is [`Handled::Failed`]. Constructing a gone worker
//! means panicking the worker thread, which is not worth manufacturing for a fixture
//! arm — recorded rather than chased, so that a future reader counts the defences
//! that exist and not the arms that merely look tested.
//!
//! **What has a witness elsewhere is the *wording*, and not the state.** An earlier
//! version of this paragraph said these arms had "no witness at all", which was true
//! of both halves until PRO-949 and is now true of one. `report::said_about` decides
//! what a player is told about a [`Remeshing`] verdict as a function of that verdict
//! instead of inside a redraw, so `crates/mc-client/src/app/report_test.rs` asserts
//! the sentence a stopped worker gets them: the `WorkerGone` arm answering nothing
//! reddens exactly that reading. Nothing has moved on the state — no fixture can hold
//! a live [`Remesher`] whose worker has died, because both ends of the channel are
//! the `Remesher` itself — so these two arms stay unreached and the wiring that would
//! carry a real occurrence to that sentence stays unasserted. Half-closed, not
//! closed.
//!
//! What the unbounded wait costs is that a worker which never answers *and* never
//! dies wedges the run rather than reddening it. A panicking worker does not: its
//! sending end drops, and the next ask says [`Collected::TheWorkerIsGone`] at once.
//! The wedge is bounded at the runner layer instead, in `.config/nextest.toml`,
//! because the two bounds are different acts — exceeding a runner bound says *this
//! run is broken, stop it*, which is true whatever the number, where exceeding an
//! assertion bound says *the property is false*, which is a lie when the machine
//! was merely slow.
//!

use std::thread;
use std::time::Duration;

use mc_client::remesh::{Collecting, Remeshed, Remesher};
use mc_client::session::reload::Remeshing;
use mc_render::window::rendered;
use mc_world::column::SECTIONS_PER_COLUMN;

use crate::input::InputHarness;

/// How long a collect pauses between asks, so the worker has the machine.
///
/// The only duration here. What bounds the waiting is the worker answering, not a
/// clock — see this module's own account of why no clock is derivable.
const BETWEEN_POLLS: Duration = Duration::from_millis(1);

/// What one finished batch came to.
///
/// **A total verdict**, so a scenario expecting a discarded batch cannot be
/// satisfied by a scene, by a failure, or by either of the two ways nothing
/// arrives.
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
    /// The channel is gone, so no batch will arrive now or later.
    ///
    /// **Said by the collect rather than inferred from a wait running out**, which
    /// is what makes it immediate: waiting is the repair for a worker that has not
    /// finished and no repair at all for one that has gone.
    TheWorkerIsGone,
    /// Nothing arrived and the worker holds nothing, so nothing ever will.
    ///
    /// **A fixture fault rather than a product one, and immediate.** A channel that
    /// had gone would have said so on the same ask, and a worker holding a batch
    /// reports busy, so what is left here is a batch nobody handed over — read it as
    /// "this scenario submitted nothing", never as a broken client. It used to cost
    /// a whole patience to say so.
    NothingWasEverHandedOver,
}

/// What `remesher` handed back, waited for until it answers.
pub fn collected(remesher: &mut Remesher) -> Collected {
    loop {
        let asked = remesher.collect();
        match asked {
            Collecting::Finished(finished) => return verdict_of(&finished),
            Collecting::WorkerGone => return Collected::TheWorkerIsGone,
            // A free worker was handed nothing, so asking again cannot change the
            // answer; a busy one has the batch, and only it decides when.
            Collecting::NothingYet if remesher.is_free() => {
                return Collected::NothingWasEverHandedOver;
            }
            Collecting::NothingYet => thread::sleep(BETWEEN_POLLS),
        }
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
    /// The two absences, told apart: see [`Collected`] for what each means.
    TheWorkerIsGone,
    NothingWasEverHandedOver,
}

/// What `client` made of whatever `remesher` has finished, waited for.
///
/// **Nothing here puts a section back.** The hand-back happens inside the client's
/// own collect, which is what makes a scenario reading this an assertion about the
/// client rather than an agreement between two callers of one function.
pub fn handled(client: &mut InputHarness, remesher: &mut Remesher) -> Handled {
    loop {
        let asked = client.collect_remesh(remesher);
        match asked {
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
            // The same two absences [`collected`] tells apart, told apart the same
            // way: the client's collect passes both through unchanged.
            Remeshing::NothingYet if remesher.is_free() => {
                return Handled::NothingWasEverHandedOver;
            }
            Remeshing::NothingYet => thread::sleep(BETWEEN_POLLS),
        }
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
