//! Publishing what the simulation is at, without waiting on whoever is reading
//! it.
//!
//! Reading a stale snapshot is correct; stalling the simulation to serve a
//! reader is not. Publication is therefore a pointer swap: a reader holds an
//! `Arc` of whatever was current when it looked, and the next publish replaces
//! the pointer rather than the contents.
//!
//! **That the held snapshot cannot change underneath its holder is a property of
//! the type, not of a compiler error.** It holds a tick, a pose and a player
//! state, all plain values, and it must stay that way — a field carrying a
//! `Mutex` or an `AtomicU32` would silently reopen the hole while every test here
//! still passed.
//!
//! **`advance` takes `&mut self` because of what a tick mutates, not to fend off
//! a race.** A tick assigns `self.player` — a plain field sitting *beside* the
//! `ArcSwap` rather than inside it — and now also edits the world, and an
//! `&self` method can do neither. `arc_swap` does offer `rcu` on `&self`; what
//! rules it out is its retry semantics, since a tick's effect is not confined to
//! the swapped cell and a re-run closure would step the player, and break a
//! block, more than once for one tick number.
//!
//! **The `!Sync` half of that argument no longer holds, and the conclusion
//! survives without it.** The world used to be a `Box<dyn Solidity + Send>`,
//! which left this struct `Send` but not `Sync`, so no two threads could hold
//! `&Simulation` at all. It is a concrete `World` now — a block store, a bitset
//! and an `Arc<BlockRegistry>`, all `Sync` — so `Simulation` is `Send + Sync`
//! and the borrow is doing real work again. Read the `&mut` as recording that a
//! tick mutates state outside the published cell. `latest` still takes `&self`,
//! so readers are unaffected.
//!
//! **The tick counter is free-running and never wraps.** 120 is the length of the
//! declared *intent script*, not a period of the simulation: a windowed client
//! runs for as long as its window is open, and a counter that restarted would
//! republish an old tick number to everything downstream that reads one.

use std::fmt;
use std::sync::Arc;

use arc_swap::ArcSwap;

use mc_core::content::{ContentSerial, ResolvedContent};
use mc_core::hud::HudLayout;
use mc_core::id::BlockName;

use crate::camera::CameraPose;
use crate::content::LoadedContent;
use crate::player::{PlayerState, advance_player, eye_pose};
use crate::reload::ReloadRefusal;
use crate::world::action::{EditReport, TickIntent, resolve};
use crate::world::{Clearing, RemeshWork, SectionKey, World, clearing, reload};

/// What the simulation publishes: which tick it is at, where the camera is, and
/// everything it knows about the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimSnapshot {
    pub tick: u32,
    pub camera: CameraPose,
    pub player: PlayerState,
}

/// The tick a simulation publishes before any intent has been submitted.
const FIRST_TICK: u32 = 0;

/// The content a reader draws with, and which accepted set it is.
///
/// **A second published value beside the snapshot, not a field inside it.**
/// `SimSnapshot` is `Copy` and holds plain values, and nothing needs the
/// correlation: a re-mesh batch carries its own serial, and a reader that wants
/// the content asks for it.
#[derive(Debug)]
pub struct PublishedContent {
    pub serial: ContentSerial,
    pub resolved: ResolvedContent,
    /// The HUD the same root declared.
    ///
    /// **It travels because it is refused with the blocks**, so applying one and
    /// not the other is the partial application invariant 7 calls a Blocker.
    pub hud: Arc<HudLayout>,
}

impl PublishedContent {
    /// The content a launch publishes, under the first serial.
    #[must_use]
    pub fn first(resolved: ResolvedContent, hud: HudLayout) -> Self {
        Self {
            serial: ContentSerial::FIRST,
            resolved,
            hud: Arc::new(hud),
        }
    }
}

/// What taking up a candidate settled, for whoever holds the answers it
/// replaces.
///
/// No `Eq`, because [`Clearing`] carries a position and `Vec3` has none.
#[derive(Debug, Clone, PartialEq)]
pub struct Accepted {
    /// The serial the accepted content was published under.
    pub serial: ContentSerial,
    /// What the swap did about a player the new solidity left inside a block.
    ///
    /// Travels out with the rest so it can reach the one place that prints for a
    /// person: a verdict computed and dropped satisfies nothing.
    pub clearing: Clearing,
    /// The block a client holds under the content now serving.
    ///
    /// **Re-derived and not preserved**: it is a policy over the registry rather
    /// than something the player accumulated, and re-deriving it is what lets a
    /// block a mod author has just declared be one they can go and place.
    pub holding: BlockName,
}

/// A simulation of a world, and what seating the player in it did about where
/// they were going to stand.
///
/// The verdict travels out beside the simulation because a verdict computed and
/// dropped satisfies nothing: the one place that prints for a person is a long
/// way from the one place that seats a player.
#[derive(Debug)]
pub struct Seated {
    /// The simulation the player was seated in.
    pub simulation: Simulation,
    /// What entry did about a player whose box covered a solid cell.
    pub clearing: Clearing,
    /// The blocks the save this simulation was loaded from records differently
    /// from what the content now declares them to be, ascending.
    ///
    /// **Not something seating decides**, which is why [`seat`] leaves it empty
    /// and the launch that read a save fills it in: it is a statement about a
    /// file, and it travels here because this is the rail that already reaches
    /// the one place that prints for a person. Empty for a generated world and
    /// for a save that still agrees with the content.
    pub changed: Vec<BlockName>,
}

/// Seats the player at `spawn` in `world` serving `content`, moving them clear
/// of solid blocks first if their box covers any.
///
/// **The one public way to put a player into a world.** `Simulation`'s
/// constructor is private to this crate, so every door — a resume, a first
/// launch, a golden capture, a fixture — passes through here and is asked the
/// clearing question. What the compiler holds is that no caller *outside* this
/// crate can skip it; a second seating path added inside it would not be held
/// at all, which is what `crates/mc-sim/tests/one_way_seats_a_player.rs` reads
/// these sources for.
///
/// **The clearing is applied before the simulation exists**, so the first
/// snapshot ever published already shows the player where they were put and no
/// frame is ever drawn of them inside rock.
///
/// Infallible, and deliberately: [`Clearing::NoClearSpaceWithin`] is a verdict
/// rather than a refusal, so a player nothing clear was found for is still
/// seated — where they were — and still told.
#[must_use]
pub fn seat(mut spawn: PlayerState, world: World, content: PublishedContent) -> Seated {
    let clearing = clearing::clear_the_player(&mut spawn, &world, world.extent());
    Seated {
        simulation: Simulation::new(spawn, world, content),
        clearing,
        // Seating a player says nothing about a save, and this door is crossed by
        // launches that read none. Whoever loaded one puts what it reported here.
        changed: Vec::new(),
    }
}

/// The simulation's own state, and the snapshot it publishes.
pub struct Simulation {
    published: ArcSwap<SimSnapshot>,
    /// The content a reader draws with, replaced whole when a candidate is
    /// accepted. A reader observes it by asking rather than by being told.
    content: ArcSwap<PublishedContent>,
    player: PlayerState,
    /// The world a tick resolves the player's motion against, and edits.
    ///
    /// Concrete and not a trait object. A trait object would re-open the hole
    /// [`World`] closes: any implementor could be *asked* to keep its block
    /// store and its collision view in agreement, and that is precisely the
    /// thing that must not be askable.
    world: World,
}

impl Simulation {
    /// A simulation of `world` serving `content`, with the player at `spawn` and
    /// its first snapshot already published.
    ///
    /// The spawn's own snapshot exists before any intent is submitted, because
    /// the state before the first tick is a state a reader can be shown — the
    /// frame drawn while nothing has been asked for yet is drawn from it.
    ///
    /// `content` is taken at construction so that a simulation is never in a
    /// state where it has a world and nothing to draw it with.
    #[must_use]
    fn new(spawn: PlayerState, world: World, content: PublishedContent) -> Self {
        Self {
            published: ArcSwap::from_pointee(SimSnapshot {
                tick: FIRST_TICK,
                camera: eye_pose(&spawn),
                player: spawn,
            }),
            content: ArcSwap::from_pointee(content),
            player: spawn,
            world,
        }
    }

    /// The content a reader draws with, and which accepted set it is.
    ///
    /// **Observed by asking, never by being told.** A reader that has not looked
    /// since the last accept goes on seeing what it last observed, which is what
    /// keeps this an arrangement rather than a callback.
    #[must_use]
    pub fn content(&self) -> Arc<PublishedContent> {
        self.content.load_full()
    }

    /// Advances one tick under `intent` and publishes the result.
    ///
    /// **Movement and look are applied first, and the action is resolved against
    /// the state the tick ends with.** So the ray is cast from the eye the tick
    /// arrived at, along the orientation it has already turned and limited, and
    /// the snapshot this publishes shows the edit and the view that produced it
    /// together.
    ///
    /// The camera is derived from the player the tick produced rather than
    /// carried beside it, so there is nothing to keep in step: an eye that moved
    /// is an eye whose player moved.
    ///
    /// Returns nothing when the tick asked for no action — which is almost every
    /// tick, and is not a refusal. Deliberately not `#[must_use]`: every caller
    /// that only walks the player ignores it.
    pub fn advance(&mut self, intent: impl Into<TickIntent>) -> Option<EditReport> {
        let intent = intent.into();
        self.player = advance_player(self.player, &intent.movement, &self.world);
        let report = intent
            .action
            .as_ref()
            .map(|action| resolve(action, &self.player, &mut self.world));
        self.published.store(Arc::new(SimSnapshot {
            tick: self.published.load().tick.saturating_add(1),
            camera: eye_pose(&self.player),
            player: self.player,
        }));
        report
    }

    /// Takes up `candidate` as the content this simulation serves.
    ///
    /// **The seam splits where the borrow does.** Admission and the swap need
    /// only the world and are [`world::reload`](crate::world::reload)'s, a child
    /// of the module owning the write privilege; what needs the player is here.
    ///
    /// Called after [`advance`](Self::advance) has published its tick, so the
    /// change is in force from the next one. `pub(crate)` because the door a
    /// driver goes through is [`crate::reload::adopt_at_tick_boundary`].
    ///
    /// # Errors
    ///
    /// Returns [`ReloadRefusal`] with this simulation exactly as it was.
    pub(crate) fn adopt(&mut self, candidate: LoadedContent) -> Result<Accepted, ReloadRefusal> {
        let LoadedContent {
            registry,
            hud,
            resolved,
        } = candidate;
        // Admission first, so a refusal returns before anything is published and
        // the content a reader holds is untouched by construction.
        let adopted = reload::adopt_candidate(&mut self.world, Arc::new(registry))?;
        let clearing =
            clearing::clear_the_player(&mut self.player, &self.world, self.world.extent());
        let serial = self.content().serial.next();
        self.content.store(Arc::new(PublishedContent {
            serial,
            resolved,
            hud: Arc::new(hud),
        }));
        Ok(Accepted {
            serial,
            holding: adopted.holding,
            clearing,
        })
    }

    /// What has to be re-meshed for this simulation's edits to be seen, or
    /// nothing when there have been none since it was last asked.
    ///
    /// **An owned, `Send` batch and never a borrow**, which is what lets a
    /// re-mesh run on a thread of its own without pinning the tick behind it —
    /// the same property the session that owns this simulation already has.
    pub fn take_remesh_work(&mut self) -> Option<RemeshWork> {
        let serial = self.content().serial;
        self.world.take_remesh_work(serial)
    }

    /// Records `keys` as needing to be meshed again.
    ///
    /// The one caller is a batch discarded for having been meshed against content
    /// that stopped serving; without this those sections stay stale for the rest
    /// of the run.
    pub fn mark_for_remesh(&mut self, keys: Vec<SectionKey>) {
        self.world.mark_for_remesh(keys);
    }

    /// Whatever was published most recently.
    #[must_use]
    pub fn latest(&self) -> Arc<SimSnapshot> {
        self.published.load_full()
    }

    /// The world this simulation owns.
    ///
    /// Read-only, and that is a property of the type rather than of this
    /// signature: the one function that writes anything is private to
    /// `mc_sim::world`, so a `&World` is a view and can be nothing else.
    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }
}

/// A world's blocks would bury whatever a panic message was about, so what is
/// shown is what a reader of one can use: which tick was published and where the
/// player is.
impl fmt::Debug for Simulation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Simulation")
            .field("published", &self.published.load())
            .field("player", &self.player)
            .finish_non_exhaustive()
    }
}
