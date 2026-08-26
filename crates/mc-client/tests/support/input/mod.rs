//! The client's own input dispatch, driven with no event loop, no window and no
//! graphics device.
//!
//! # Every method here is one call into the client's dispatch
//!
//! That is the whole design constraint, and it is the single most likely way the
//! feature this harness serves fails. A harness that read a key table itself, or
//! decided for itself whether pointer motion counts, or drained the accumulator
//! and stepped the simulation, would agree with the client by construction: every
//! scenario written against it would be green while the client submitted nothing.
//! So nothing below matches on a key, weighs a capture, or builds an intent —
//! each method hands a value to `mc_client::events` and stops. A sibling text
//! guard, `tests/seam_boundaries.rs`, is what keeps it that way.
//!
//! # Why it enters where it does
//!
//! Real window and device events are constructed and dispatched whole, so the
//! client's own translation of them runs under test. The keyboard is the one
//! exception and it is the library's doing rather than a choice: a key event
//! cannot be built outside the window library, so the keyboard enters one level
//! below, at the reduction to a key code and a pressed flag.
//!
//! # The world it drives is declared, not empty
//!
//! Gravity acts on every tick, so a world that answered "nothing is solid" would
//! put the player in free fall and turn every comparison against a no-input
//! control into a comparison of two falls. The fixture is a floor to stand on and
//! nothing else — see [`world`].

// Each scenario binary links the whole harness and drives a subset of it.
#![allow(dead_code)]

/// The clock the client reads once a frame, moved by hand rather than waited on.
mod clock;
/// The platform the session asks for the pointer, recording what it was asked.
mod platform;
/// The world a driven tick resolves the player's motion against.
mod world;

use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use mc_client::events::{dispatch_device_event, dispatch_key, dispatch_window_event};
use mc_client::remesh::Remesher;
use mc_client::session::reload::{ReloadReport, Remeshing};
use mc_client::session::{Bindings, PointerAsk, Session, ending_after_saving};
use mc_core::id::BlockName;
use mc_render::overlay::OverlayReadout;
use mc_render::window::{CaptureState, Ending};
use mc_sim::action::EditReport;
use mc_sim::content::LoadedContent;
use mc_sim::player::TICK_QUANTUM;
use mc_sim::reload::{ContentReload, ReloadRefusal};
use mc_sim::simulation::{Accepted, PublishedContent, SimSnapshot, Simulation};
use mc_sim::world::RemeshWork;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::keyboard::KeyCode;

use clock::DrivenClock;
use platform::{PointerLog, RecordingPlatform};

/// A client's dispatch, driven with no window and no adapter.
#[derive(Debug)]
pub struct InputHarness {
    session: Session,
    log: PointerLog,
    /// What this client's frame path reads, moved by hand before every frame.
    ///
    /// **The harness owns the clock, and the client owns what a reading buys.**
    /// A frame here costs the elapsed time the scenario names, and how many ticks
    /// that is worth is the client's answer travelling back out — a harness that
    /// worked it out and asked for that many ticks would be agreeing with itself
    /// about the very conversion these scenarios are written to watch.
    clock: DrivenClock,
}

impl InputHarness {
    /// A session over a platform granting exactly `granted` and refusing
    /// everything else, with no world yet.
    ///
    /// The session asks for its first capture while it is being built, so the
    /// asks this records begin before any event has been dispatched.
    #[must_use]
    pub fn granting(granted: &[CaptureState]) -> Self {
        let (platform, log) = RecordingPlatform::granting(granted);
        Self {
            session: Session::new(Box::new(platform)),
            log,
            clock: DrivenClock::default(),
        }
    }

    /// The common case: a platform that grants a locked pointer.
    #[must_use]
    pub fn started() -> Self {
        Self::granting(&[CaptureState::Locked])
    }

    /// The same, with the client deciding what `bindings` says rather than what
    /// the declared table does.
    ///
    /// **The bindings arrive already built and are never inspected here.** A
    /// harness that spelled which key means what would be translating on the
    /// client's behalf, and the table it was supposed to be watching would have
    /// nothing left watching it — which is the failure the sibling text guard
    /// exists to keep out of this directory. What is handed over is an opaque
    /// value a scenario constructed.
    #[must_use]
    pub fn bound(bindings: Bindings) -> Self {
        let (platform, log) = RecordingPlatform::granting(&[CaptureState::Locked]);
        Self {
            session: Session::bound(Box::new(platform), bindings),
            log,
            clock: DrivenClock::default(),
        }
    }

    /// Puts a player on the declared ground plane and hands it to the session.
    ///
    /// # Errors
    ///
    /// Returns the refusal if the declared world does not build. Fallible
    /// because the world is now declared block by block against a declared
    /// registry rather than answered by a predicate, and a fixture that is wrong
    /// about itself is worth hearing about rather than absorbing.
    pub fn start_world(&mut self) -> Result<(), Box<dyn Error>> {
        let (simulation, holding) = world::ground_plane()?;
        self.session.attach_simulation(simulation, holding);
        Ok(())
    }

    /// Hands the session a world to play that the caller declared, and the block
    /// a place request over it names.
    ///
    /// The same entry [`start_world`](Self::start_world) uses, with the
    /// declaration supplied rather than taken from this harness's own floor: a
    /// scenario about what survives a quit has to write a save against the
    /// registry it will be read against, and that registry is the caller's to
    /// build.
    pub fn play(&mut self, simulation: Simulation, holding: BlockName) {
        self.session.attach_simulation(simulation, holding);
    }

    /// Hands the client a candidate content set to take up.
    ///
    /// Forwarded, deciding nothing: whether the candidate is admitted, what the
    /// swap replaces and which block the player is left holding are the client's
    /// answers travelling back out, and a harness that judged any of them would
    /// agree with itself while the client swapped nothing at all.
    ///
    /// `None` when there is no world yet, exactly as a tick step with nothing to
    /// advance publishes nothing.
    pub fn adopt(&mut self, candidate: LoadedContent) -> Option<Result<Accepted, ReloadRefusal>> {
        self.session.adopt_content(candidate)
    }

    /// Hands the session the reload it drives at every tick boundary from now on.
    ///
    /// Forwarded, deciding nothing. **When an attempt begins, how many a burst of
    /// changes becomes and what a refused one reports are the client's answers
    /// travelling back out**, and a harness that drove the reload itself would be
    /// the one thing left calling it — green for good while the client's own tick
    /// stopped.
    pub fn attach_reload(&mut self, reload: ContentReload) {
        self.session.attach_reload(reload);
    }

    /// Whatever the last tick boundary made of a content change, taken once.
    ///
    /// Forwarded, deciding nothing, and take-once because that is the shape the
    /// client's own report has: a scenario counting attempts has to be able to tell
    /// a boundary that reported something from the one after it, and a reading that
    /// left the report in place would count one attempt on every tick that
    /// followed it.
    pub fn take_reload_report(&mut self) -> Option<ReloadReport> {
        self.session.take_reload_report()
    }

    /// What the client was left to mesh again, taken once.
    ///
    /// Forwarded, deciding nothing. **Which sections a reload left behind is the
    /// client's answer travelling back out**, and the set is *taken* — so a
    /// harness that remembered what it had handed over would report a batch the
    /// client had already drained, and a scenario reading it would call that a
    /// section marked twice.
    pub fn take_remesh_work(&mut self) -> Option<RemeshWork> {
        self.session.take_remesh_work()
    }

    /// What the client made of whatever the re-mesh worker has finished.
    ///
    /// Forwarded, deciding nothing. **There is deliberately no forward for putting
    /// a section back**, and its absence is the point: what a discarded batch means
    /// is the client's decision, and a harness offering the hand-back as a call of
    /// its own is what lets a scenario about it make that call itself — green for
    /// good while the frame path dropped the keys and left those sections stale for
    /// the rest of the run.
    pub fn collect_remesh(&mut self, remesher: &mut Remesher) -> Remeshing {
        self.session.collect_remesh(remesher)
    }

    /// Whatever content the simulation is publishing, asked for rather than
    /// pushed.
    ///
    /// Forwarded, deciding nothing. **A reader observes by asking**, which is
    /// what makes a reader that has not looked since the last accepted candidate
    /// go on seeing what it last observed — and a harness that remembered what it
    /// had handed over would answer itself while the simulation published nothing
    /// at all.
    ///
    /// `None` when there is no world yet, exactly as a tick step with nothing to
    /// advance publishes nothing.
    #[must_use]
    pub fn content(&self) -> Option<Arc<PublishedContent>> {
        self.session.content()
    }

    /// Whatever the simulation has published, without advancing anything.
    ///
    /// [`tick`](Self::tick) reports the same value after taking a step, which is
    /// no use to a scenario about what a *swap* published: a step would move the
    /// tick counter this is being read to compare.
    #[must_use]
    pub fn published(&self) -> Option<Arc<SimSnapshot>> {
        self.session.latest()
    }

    /// What this run reports once whatever it was playing has been saved to
    /// `save`.
    ///
    /// Forwarded, deciding nothing: whether an ending saves at all, and what a
    /// failed write does to it, are the client's answers and are exactly what
    /// the scenarios ask about.
    pub fn quit(&self, ending: Ending, save: &Path) -> Ending {
        ending_after_saving(Some(&self.session), ending, save)
    }

    /// A key going down.
    pub fn press(&mut self, key: KeyCode) {
        dispatch_key(&mut self.session, key, true);
    }

    /// The same key coming back up.
    pub fn release(&mut self, key: KeyCode) {
        dispatch_key(&mut self.session, key, false);
    }

    /// The pointer moving by a number of raw device counts.
    ///
    /// A real device event handed to the whole device-event entry, so the pair
    /// the client destructures out of it is destructured under test: the two
    /// axes are one type and one call apart, and a swap between them or a
    /// flipped vertical sign is an upside-down world that nothing here would
    /// notice if the harness handed the two numbers over already separated.
    ///
    /// It is dispatched whatever the client is doing with the cursor. Whether
    /// motion arriving now is the player looking around is the client's question
    /// to answer, and a harness that asked it first would agree with the client
    /// by construction and go on agreeing after the client stopped asking.
    pub fn move_pointer(&mut self, raw_x: f64, raw_y: f64) {
        dispatch_device_event(
            &mut self.session,
            &DeviceEvent::MouseMotion {
                delta: (raw_x, raw_y),
            },
        );
    }

    /// A mouse button going down.
    ///
    /// A real `MouseInput` event handed to the whole window-event entry, so the
    /// client's own translation of the library's button runs under test. Driving
    /// the session directly would hand it a button already spelled in the
    /// vocabulary the client decides in — which is the one thing on this path
    /// the adapter is responsible for, and nothing would be left watching it.
    ///
    /// The device is the library's own placeholder: which device a click came
    /// from is not something this client reads, and a real window would report
    /// one this test has no way to predict.
    pub fn click(&mut self, button: MouseButton) {
        self.mouse(button, ElementState::Pressed);
    }

    /// The same button coming back up.
    pub fn unclick(&mut self, button: MouseButton) {
        self.mouse(button, ElementState::Released);
    }

    /// One mouse button transition, whichever way it goes.
    fn mouse(&mut self, button: MouseButton, state: ElementState) {
        let _ = dispatch_window_event(
            &mut self.session,
            &WindowEvent::MouseInput {
                device_id: DeviceId::dummy(),
                state,
                button,
            },
        );
    }

    /// The window telling the client it no longer has focus.
    ///
    /// A real `Focused(false)` handed to the whole window-event entry rather than
    /// a direct call to the session's own clear: what a lost window *means* is
    /// the client's translation to make, and a harness that made it here would
    /// leave `kind_of` and the loop's own policy asserted by nothing.
    pub fn lose_focus(&mut self) {
        let _ = dispatch_window_event(&mut self.session, &WindowEvent::Focused(false));
    }

    /// The block a place request over this run would name, as the client's own
    /// core reports it.
    ///
    /// Forwarded and nothing more, for the reason every other method here is:
    /// what the client holds is the client's answer, and a harness that
    /// remembered what it handed over would agree with itself while the core
    /// reported nothing at all.
    #[must_use]
    pub fn held_block(&self) -> Option<BlockName> {
        self.session.held_block()
    }

    /// Whether the client is showing its debug overlay, as the client's own core
    /// reports it.
    ///
    /// Forwarded and nothing more, for the reason every other method here is: a
    /// harness that remembered which keys it had dispatched and worked the answer
    /// out would agree with itself while the client showed nothing at all.
    #[must_use]
    pub fn overlay_visible(&self) -> bool {
        self.session.overlay_visible()
    }

    /// What this client's overlay publishes for whoever paints it, as the
    /// client's own core answers it.
    ///
    /// **Forwarded and never assembled**, for the reason every other method here
    /// is one call: a harness that decided when there is a reading to publish, or
    /// what it holds, would agree with itself while the client published nothing
    /// — and a frame drawn from the answer would be a frame of a client that does
    /// not exist. Nothing here constructs a readout; this is the client's own,
    /// carried.
    #[must_use]
    pub fn overlay_readout(&self) -> Option<OverlayReadout> {
        self.session.overlay_readout()
    }

    /// The frame rate this client's overlay is reporting, and nothing else it
    /// publishes.
    ///
    /// **A scalar rather than the readout, because of a guard next door.**
    /// `tests/overlay_over_content.rs` refuses to let any suite that can record a
    /// frame of this product name `OverlayReadout` — rasterised glyphs must not
    /// reach a committed golden, and four files are exempt from that. A scenario
    /// whose whole question is "does the rate on screen agree with the time the
    /// world spent" needs one number rather than a readout, and taking it here
    /// rather than there is what keeps the exemption list at four.
    ///
    /// Forwarded and never assembled, for the reason every other method here is
    /// one call: what the overlay reports is the client's answer, and a harness
    /// that worked the rate out from the intervals it had handed over would agree
    /// with itself while the client reported nothing at all.
    #[must_use]
    pub fn overlay_frame_rate(&self) -> Option<f64> {
        self.session.overlay_readout().map(|shown| shown.frame_rate)
    }

    /// One frame that took `took`, and both halves of what the client made of it:
    /// what an action any of its ticks carried did to the world, and whatever the
    /// simulation published.
    ///
    /// **This is the only way anything here advances the world**, and it hands
    /// over elapsed time rather than a number of ticks. How many ticks a frame
    /// buys is the whole question this harness exists to let a scenario ask, so a
    /// harness that answered it would be answering itself.
    pub fn frame(&mut self, took: Duration) -> (Option<EditReport>, Option<Arc<SimSnapshot>>) {
        self.clock.move_on(took);
        let edited = self.session.advance_frame(&self.clock);
        (edited, self.session.latest())
    }

    /// What `frames` frames of `took` each leave published, in order.
    ///
    /// Shorter than `frames` exactly when some of them published nothing, so a
    /// caller that expected a snapshot per frame can see that it did not get one
    /// rather than reading a repeat of the last.
    pub fn frames(&mut self, frames: u32, took: Duration) -> Vec<Arc<SimSnapshot>> {
        (0..frames).filter_map(|_| self.frame(took).1).collect()
    }

    /// One frame that took exactly one tick quantum, and whatever it left
    /// published.
    ///
    /// The common case: a scenario about what a key does wants one tick's worth
    /// of motion and does not care how the time arrived. It is spelled as the
    /// quantum rather than as a tick because a tick is not something this side of
    /// the seam can ask for any more.
    ///
    /// `None` when no world has been started: a tick with nothing to advance
    /// publishes nothing, which is a state the client is genuinely in for the
    /// first frames of every run.
    pub fn tick(&mut self) -> Option<Arc<SimSnapshot>> {
        self.frame(TICK_QUANTUM).1
    }

    /// What `ticks` tick steps publish, in order.
    ///
    /// Shorter than `ticks` exactly when some of them had no world to advance,
    /// so a caller that expected a snapshot per step can see that it did not get
    /// one rather than reading a repeat of the last.
    pub fn ticks(&mut self, ticks: u32) -> Vec<Arc<SimSnapshot>> {
        (0..ticks).filter_map(|_| self.tick()).collect()
    }

    /// One tick step, and what the action it carried did to the world.
    ///
    /// `None` is every tick that asked the world for nothing — which is a tick
    /// with no world to advance, a tick no click preceded, and a click the
    /// client declined to spend. Those three are told apart by what the caller
    /// dispatched and by the platform log, not by this.
    ///
    /// The report is the client's own answer travelling back out, not something
    /// this harness worked out: what a click became is exactly the question the
    /// scenarios ask, so a harness that resolved it would answer itself.
    pub fn edit(&mut self) -> Option<EditReport> {
        self.frame(TICK_QUANTUM).0
    }

    /// What `ticks` tick steps report, in the order they reported it.
    ///
    /// A tick that asked for nothing contributes nothing, so the length is the
    /// number of requests the run resolved and never the number of ticks — which
    /// is what makes "one press is one action" a thing to count rather than a
    /// thing to describe.
    pub fn edits(&mut self, ticks: u32) -> Vec<EditReport> {
        (0..ticks).filter_map(|_| self.edit()).collect()
    }

    /// One tick step, and **both** halves of what it left behind: what the action
    /// it carried did to the world, and whatever the simulation published.
    ///
    /// [`tick`](Self::tick) drops the report and [`edit`](Self::edit) drops the
    /// snapshot, so a scenario about the whole resulting state of a run cannot use
    /// either — asking for both would take two tick steps and compare a run
    /// against a different run. This is one step, reported whole.
    pub fn step(&mut self) -> (Option<EditReport>, Option<Arc<SimSnapshot>>) {
        self.frame(TICK_QUANTUM)
    }

    /// The captures the platform was asked for, in order.
    ///
    /// Derived from the ask log by filtering rather than counted beside it, so
    /// there is one record of what happened and a released pointer can never be
    /// miscounted as a captured one.
    #[must_use]
    pub fn grabs(&self) -> Vec<CaptureState> {
        self.log
            .asks()
            .into_iter()
            .filter_map(|ask| match ask {
                PointerAsk::Grab(capture) => Some(capture),
                PointerAsk::Release | PointerAsk::CursorVisible(_) => None,
            })
            .collect()
    }
}
