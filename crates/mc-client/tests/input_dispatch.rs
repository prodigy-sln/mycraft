//! What a keystroke does once the client has it: whether it reaches the player
//! at all, and whether each declared binding moves the player the way it says.
//!
//! Every scenario here is driven through the client's own dispatch in a process
//! that constructs no event loop, opens no window and acquires no GPU adapter.
//! That the harness they drive it through stays that way is a text guard, in
//! `tests/seam_boundaries.rs`, rather than anything this file can assert.
//!
//! # Three of these assert an absence, and each carries its own control
//!
//! A released key, an unbound key and a key lost with the window all assert that
//! the player ended up where a player who pressed nothing ends up — and a client
//! that dropped every key alike would satisfy all three. So each of the three
//! runs the *same* key through the *same* ticks without the thing it is denying,
//! in the same test, and requires that run to have moved the player. An adapter
//! that did nothing whatever fails all three on that control rather than passing
//! them on the assertion.
//!
//! # Nothing here is asserted against a written-down coordinate
//!
//! Gravity acts on every tick and a walk covers a fixed fraction of a block on
//! each of them, so any position spelled in this file would be a number copied
//! out of a run of the code it is judging — and would stay green ever after,
//! including on the day the walk stops. Every assertion below is a *difference
//! between two runs of the same harness*, one that dispatched a key and one that
//! dispatched nothing over the same number of ticks, so the oracle is independent
//! of the walk speed, the gravity, the jump speed and the tick duration, none of
//! which this feature touches.
//!
//! # The declared basis
//!
//! Yaw 0 faces +x and yaw +π/2 faces +z (`crates/mc-sim/src/player/mod.rs`), and
//! the fixture spawns the player at yaw 0. So forward is +x, the right hand of it
//! is +z, and the player's left is −z. That frame is what "left" and "right" mean
//! in the table below, and it is declared here rather than inferred from which
//! way the code happened to go.
//!
//! # Why an axis a row does not name is compared exactly
//!
//! On an axis a row does not name, the walk asks for a displacement of exactly
//! zero — a distance with no sign, which leaves the coordinate untouched rather
//! than recomputed. The vertical is genuinely recomputed on every tick of a walk,
//! but both runs recompute it from the same vertical velocity and resolve it onto
//! the same block face of the same declared floor, so it is the same value.
//! Measured against the arithmetic path, the error on an unnamed axis is
//! therefore zero, while the smallest difference this table has to catch is a
//! whole tick of walk. The bar sits above the one and far below the other, and a
//! tolerance here would only widen the gap a mis-mapped row could fall into.
//!
//! # The control the table carries
//!
//! Four of the five rows assert that two axes did *not* move, and a client that
//! did nothing at all would satisfy all four. What stops that is the table being
//! one test: every axis an unmoved assertion watches is an axis some other row is
//! required to have moved, so a dispatch that reached the player on no axis fails
//! all five rows rather than passing four of them.
//!
//! # The harness is included by path
//!
//! Not through `tests/support/mod.rs`, which links `support/frames.rs` and with it
//! the whole graphics stack — into a binary whose entire premise is that no
//! adapter is acquired (conductor ruling 56).

#[path = "support/input/mod.rs"]
mod input;

use std::error::Error;

use glam::Vec3;
use winit::keyboard::KeyCode;

use input::InputHarness;

type TestResult = Result<(), Box<dyn Error>>;

/// How many ticks a held key is held for.
///
/// A tick walks a fraction of a block, so twenty is far enough for the direction
/// of the walk to be unmistakable, and the fixture is a plain floor with no wall
/// to stop it short of them.
const HELD_TICKS: u32 = 20;

/// The key the declared table binds to walking forward.
///
/// Spelled here as well as in the table below, because the two say different
/// things: the table *declares* the binding, and this is the key the scenarios
/// that are not about the table reach for when they need one that does something.
const FORWARD: KeyCode = KeyCode::KeyW;

/// A key no row of the declared table names.
///
/// A letter beside the bound ones rather than far from them, because a table that
/// answered for its neighbours is the plausible failure — not one that answered
/// for a function key.
const UNBOUND: KeyCode = KeyCode::KeyQ;

/// How many tick steps are taken before the world lands.
///
/// More than one, so the input has to survive being carried rather than merely
/// arriving late.
const TICKS_BEFORE_THE_WORLD: u32 = 3;

/// One axis of the world.
#[derive(Debug, Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    /// This axis's component of `displacement`.
    const fn of(self, displacement: Vec3) -> f32 {
        match self {
            Self::X => displacement.x,
            Self::Y => displacement.y,
            Self::Z => displacement.z,
        }
    }

    /// The two axes this one is not.
    const fn others(self) -> [Self; 2] {
        match self {
            Self::X => [Self::Y, Self::Z],
            Self::Y => [Self::X, Self::Z],
            Self::Z => [Self::X, Self::Y],
        }
    }
}

/// One row of the declared binding table: the key, where holding it takes the
/// player, and when that is read.
#[derive(Debug, Clone, Copy)]
struct Row {
    key: KeyCode,
    axis: Axis,
    /// Which way along that axis, as a sign.
    toward: f32,
    /// Which of the published ticks the row is read at, counting from zero.
    at_tick: usize,
    what: &'static str,
}

/// The five keys the client declares a binding for, and what each of them does.
///
/// The four walks are read at the last of the held ticks, where the difference
/// from the control has had every tick to accumulate. The jump is read at the
/// first, because that is where a jump is unambiguously *up*: the arc it buys
/// comes back down again, and a row read after it landed would assert nothing.
const DECLARED_ROWS: [Row; 5] = [
    Row {
        key: KeyCode::KeyW,
        axis: Axis::X,
        toward: 1.0,
        at_tick: HELD_TICKS as usize - 1,
        what: "walked along the direction the player faces",
    },
    Row {
        key: KeyCode::KeyS,
        axis: Axis::X,
        toward: -1.0,
        at_tick: HELD_TICKS as usize - 1,
        what: "walked against the direction the player faces",
    },
    Row {
        key: KeyCode::KeyA,
        axis: Axis::Z,
        toward: -1.0,
        at_tick: HELD_TICKS as usize - 1,
        what: "strafed to the facing direction's left",
    },
    Row {
        key: KeyCode::KeyD,
        axis: Axis::Z,
        toward: 1.0,
        at_tick: HELD_TICKS as usize - 1,
        what: "strafed to the facing direction's right",
    },
    Row {
        key: KeyCode::Space,
        axis: Axis::Y,
        toward: 1.0,
        at_tick: 0,
        what: "left the ground on the very first tick",
    },
];

impl Row {
    /// What is wrong with the displacement this row produced, if anything.
    fn fault_in(self, displacement: Vec3) -> Option<String> {
        let along = self.axis.of(displacement) * self.toward;
        let across = self.axis.others().map(|axis| axis.of(displacement));
        if along > 0.0 && across.iter().all(|distance| *distance == 0.0) {
            return None;
        }
        Some(format!(
            "{:?} should have {} and moved the player on no other axis; against the no-input \
             control it displaced them by {displacement:?}",
            self.key, self.what
        ))
    }
}

#[test]
fn a_dispatched_forward_key_displaces_the_player_the_next_tick_publishes() -> TestResult {
    let idle = one_tick(|_| {})?;
    let forward = one_tick(|harness| harness.press(FORWARD))?;

    assert_ne!(
        forward, idle,
        "a client wired to none of its own input publishes the same player either way, which is \
         what makes a keystroke that never arrives invisible. Dispatching the key bound to \
         forward and taking one tick has to publish a player somewhere the same tick does not \
         publish them when nothing was dispatched"
    );
    Ok(())
}

#[test]
fn every_declared_binding_moves_the_player_along_its_own_axis_and_no_other() -> TestResult {
    let control = walked(|_| {})?;
    let mut wrong = Vec::new();

    for row in DECLARED_ROWS {
        let held = walked(|harness| harness.press(row.key))?;
        let displacement = at(&held, row.at_tick)? - at(&control, row.at_tick)?;
        if let Some(fault) = row.fault_in(displacement) {
            wrong.push(fault);
        }
    }

    assert!(
        wrong.is_empty(),
        "the five rows are the whole declared table — W forward, S back, A left, D right, Space \
         up — and each moves the player its own way and no other way. A row that reached the \
         wrong axis walks the player sideways every time they ask to go forward, and a row that \
         reached no axis is a key the player presses and the game never sees. {} of the {} rows \
         went somewhere else: {wrong:?}",
        wrong.len(),
        DECLARED_ROWS.len()
    );
    Ok(())
}

#[test]
fn a_key_released_before_the_first_tick_walks_the_player_nowhere() -> TestResult {
    let control = walked(|_| {})?;
    let held = walked(|harness| harness.press(FORWARD))?;
    let released = walked(|harness| {
        harness.press(FORWARD);
        harness.release(FORWARD);
    })?;

    assert_ne!(
        held, control,
        "the control this scenario needs: the same key, held and not let go of, has to walk the \
         player somewhere — or the sameness below is a client that walks nowhere ever"
    );
    assert_eq!(
        released, control,
        "a key the player let go of is a key they have stopped asking with, and every one of the \
         {HELD_TICKS} ticks that follow the release leaves them exactly where the same ticks \
         leave a player who pressed nothing at all. A client that read the press and not the \
         release walks on for as long as the game is open, with nothing the player can do to \
         stop it"
    );
    Ok(())
}

#[test]
fn a_key_the_binding_table_names_no_action_for_walks_the_player_nowhere() -> TestResult {
    let control = walked(|_| {})?;
    let bound = walked(|harness| harness.press(FORWARD))?;
    let unbound = walked(|harness| harness.press(UNBOUND))?;

    assert_ne!(
        bound, control,
        "the control this scenario needs: a key the table *does* name has to reach the player, or \
         the sameness below is a dispatch that drops every key alike"
    );
    assert_eq!(
        unbound, control,
        "{UNBOUND:?} is not this game's input: no row of the declared table names it, so holding \
         it for {HELD_TICKS} ticks leaves the player exactly where holding nothing leaves them. A \
         table that answered for keys it does not declare would walk the player every time they \
         reached for a modifier or typed into a chat box"
    );
    Ok(())
}

#[test]
fn losing_the_window_drops_the_key_the_player_was_holding() -> TestResult {
    let control = walked(|_| {})?;
    let held = walked(|harness| harness.press(FORWARD))?;
    let lost = walked(|harness| {
        harness.press(FORWARD);
        harness.lose_focus();
    })?;

    assert_ne!(
        held, control,
        "the control this scenario needs: the key has to be reaching the player before losing the \
         window can be shown to take it away"
    );
    assert_eq!(
        lost, control,
        "the key-up for a key held when the window goes away is delivered to whatever has focus \
         now and never arrives here, so the window going away is itself the release: the \
         {HELD_TICKS} ticks that follow leave the player where pressing nothing leaves them. A \
         client that kept the key walks into a wall for as long as the player is looking at \
         another window, and is still walking when they come back"
    );
    Ok(())
}

#[test]
fn a_key_dispatched_before_the_world_lands_reaches_the_player_at_its_first_tick() -> TestResult {
    let mut early = InputHarness::started();
    early.press(FORWARD);
    let published_before_the_world = early.ticks(TICKS_BEFORE_THE_WORLD);
    early.start_world()?;
    let first = early
        .tick()
        .ok_or("a tick over a started world publishes a snapshot")?;

    let displacement = first.player.position - one_tick(|_| {})?;

    assert!(
        published_before_the_world.is_empty(),
        "the precondition this scenario is stated under: the {TICKS_BEFORE_THE_WORLD} tick steps \
         before the world lands have to have had nothing to advance, or the input below was \
         never carried across anything. They published {} snapshots",
        published_before_the_world.len()
    );
    assert!(
        displacement.x > 0.0,
        "a key pressed while the world is still loading is a key the player is holding when it \
         arrives: the first tick after the world lands walks them along the direction they face, \
         exactly as it would had the press come afterwards. This one displaced them by \
         {displacement:?} — a client that spent the input on the ticks that had nothing to \
         advance drops whatever the player did while they waited"
    );
    Ok(())
}

/// A client over the declared ground plane, with `dispatched` delivered to it
/// before any tick is taken.
///
/// A closure rather than a key, because what separates the scenarios in this file
/// is a *sequence* — a press, a press and a release, a press and a lost window —
/// and a second parameter per sequence would end in a helper nobody can read.
/// Dispatching nothing is the control every assertion here is read against: the
/// same harness, the same world, the same number of ticks, and no input at all.
///
/// It is fallible because the world is declared block by block against a registry
/// the harness builds, and either can refuse. Nothing about what the scenarios
/// below claim depends on that — the refusal is reported here and never absorbed,
/// so a fixture that failed to build cannot be mistaken for a player that did not
/// move.
fn over_the_ground(
    dispatched: impl FnOnce(&mut InputHarness),
) -> Result<InputHarness, Box<dyn Error>> {
    let mut harness = InputHarness::started();
    harness.start_world()?;
    dispatched(&mut harness);
    Ok(harness)
}

/// Where one tick leaves the player.
fn one_tick(dispatched: impl FnOnce(&mut InputHarness)) -> Result<Vec3, Box<dyn Error>> {
    let published = over_the_ground(dispatched)?
        .tick()
        .ok_or("a tick over a started world publishes a snapshot")?;
    Ok(published.player.position)
}

/// Where the player stands at each of [`HELD_TICKS`] ticks that follow
/// `dispatched`.
fn walked(dispatched: impl FnOnce(&mut InputHarness)) -> Result<Vec<Vec3>, Box<dyn Error>> {
    let published = over_the_ground(dispatched)?.ticks(HELD_TICKS);
    if published.len() != HELD_TICKS as usize {
        return Err(format!(
            "{HELD_TICKS} tick steps over a started world publish {HELD_TICKS} snapshots, not {}",
            published.len()
        )
        .into());
    }
    Ok(published
        .iter()
        .map(|snapshot| snapshot.player.position)
        .collect())
}

/// Where the player stood at one of those ticks.
fn at(walk: &[Vec3], tick: usize) -> Result<Vec3, Box<dyn Error>> {
    Ok(*walk
        .get(tick)
        .ok_or("the tick this row is read at was not published")?)
}
