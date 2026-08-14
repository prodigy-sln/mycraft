//! How the debug overlay is shown and hidden, and the one thing that must not
//! change when it is.
//!
//! Every scenario here is driven through the client's own dispatch in a process
//! that constructs no event loop, opens no window and acquires no GPU adapter.
//! That the harness they drive it through stays that way is a text guard, in
//! `tests/seam_boundaries.rs`, rather than anything this file can assert.
//!
//! # The toggle is a binding, not a key, and this file is where that is asked
//!
//! Every action in this game is remappable, so a key spelled directly into the
//! code that acts on it is a bug rather than a shortcut. What makes that a claim
//! something can grade is the pair below: the key the declared table names has to
//! toggle, **and** the same key has to stop toggling once the table names another
//! one. Either half alone is satisfied by a hardcoded key — the first trivially,
//! the second by a client that never toggles at all — which is why both are here
//! and why each carries the control that rules the other's failure out.
//!
//! # The bindings are constructed here and never interrogated
//!
//! A scenario builds a `Bindings` value and hands it over; nothing asks it what a
//! key means. Asking would be this suite answering its own question, and the
//! client's own table would have nothing left watching it. This is also why the
//! construction sits in *this* file rather than in the harness: the harness may
//! not spell the client's key vocabulary at all, and a scenario file may.
//!
//! # Nothing here is asserted against a written-down visibility
//!
//! Every assertion is a *difference between two clients* — one that was handed a
//! keystroke and one that was handed nothing, otherwise identical. So none of
//! them depends on which state the overlay starts in, which is a separate claim
//! belonging to the phase that paints it, and none of them would go quietly green
//! if that default ever moved.
//!
//! # The replay scenario compares whole runs, not final positions
//!
//! A movement key reaches the world through a tick; showing an instrument does
//! not reach the world at all. What that means concretely is that a fixed
//! sequence of input has to leave the *same* world behind whether or not somebody
//! was watching the overlay while it ran — every tick's edit report and every
//! tick's published snapshot, in order, not merely the same end position. Two
//! runs that diverged and came back would satisfy a final-position comparison.
//!
//! **The toggle key is pressed and deliberately never released**, which is the
//! one place that scenario's fixture had to be chosen rather than written down. A
//! press immediately followed by a release nets out in a held-key accumulator
//! *before the first tick spends anything*, so a toggle that also reached the
//! player as a movement would leave both runs identical and the comparison would
//! report agreement it had not earned. Held for the whole run, that same leak is
//! held across every tick of it.
//!
//! # The harness is included by path
//!
//! Not through `tests/support/mod.rs`, which links `support/frames.rs` and with it
//! the whole graphics stack — into a binary whose entire premise is that no
//! adapter is acquired (conductor ruling 56).

#[path = "support/input/mod.rs"]
mod input;

use std::error::Error;

use mc_client::session::{Bindings, KeyKind};
use mc_sim::action::EditReport;
use mc_sim::simulation::SimSnapshot;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use input::InputHarness;

type TestResult = Result<(), Box<dyn Error>>;

/// The key the declared table binds the overlay's toggle to.
const DECLARED_TOGGLE: KeyCode = KeyCode::F3;

/// The key a table that moved the toggle binds it to instead, in the two
/// vocabularies the two sides of the seam spell it in.
///
/// Both are named because they are different facts: one is what a *table* says,
/// and one is what the operating system reports when somebody presses the key.
/// A test that could only spell one of them could not tell a binding that moved
/// from a key that was never translated.
const REBOUND_TOGGLE: KeyCode = KeyCode::F7;
const REBOUND_TO: KeyKind = KeyKind::F7;

/// A table with the overlay's toggle moved off the key the declared one names.
const REBOUND: Bindings = Bindings::with_overlay_toggle(REBOUND_TO);

/// The key the replay walks with, which the table binds to a movement and never
/// to the overlay.
const FORWARD: KeyCode = KeyCode::KeyW;

/// How far the pointer is pushed down before the replay's click, in raw device
/// counts.
///
/// +y is down, which is the screen's convention and the one the operating system
/// reports in. Level with the horizon the harness's floor is not in reach at all,
/// so a replay that never looked down would ask the world for nothing and would
/// have no world state to compare.
const AIM_DOWN_COUNTS: f64 = 280.0;

/// How many tick steps the replay takes, and which of them the walk is let go on.
///
/// The release is inside the run rather than at the end of it so the replay is a
/// *sequence* rather than one held key: a client that spent the overlay's
/// keystroke somewhere on the movement path would have every remaining tick to
/// diverge.
const REPLAY_TICKS: u32 = 12;
const RELEASE_AT: u32 = 6;

/// One tick of a run: what the action it carried did to the world, and whatever
/// it published.
type Step = (Option<EditReport>, Option<SimSnapshot>);

/// Whether a client started with the declared table and handed `dispatched` is
/// showing its overlay.
fn declared_client_showing(dispatched: impl FnOnce(&mut InputHarness)) -> bool {
    let mut client = InputHarness::started();
    dispatched(&mut client);
    client.overlay_visible()
}

/// The same for a client started with `bindings`.
fn client_showing(bindings: Bindings, dispatched: impl FnOnce(&mut InputHarness)) -> bool {
    let mut client = InputHarness::bound(bindings);
    dispatched(&mut client);
    client.overlay_visible()
}

/// The fixed replay, run on `harness`, reported tick by tick.
///
/// Look down, walk, dig, keep walking, let go, keep ticking. The same calls in the
/// same order every time, so the only thing that can differ between two runs of it
/// is what the client did with them.
fn replay(harness: &mut InputHarness) -> Vec<Step> {
    harness.move_pointer(0.0, AIM_DOWN_COUNTS);
    harness.press(FORWARD);
    harness.click(MouseButton::Left);
    (0..REPLAY_TICKS)
        .map(|tick| {
            if tick == RELEASE_AT {
                harness.release(FORWARD);
            }
            let (edited, published) = harness.step();
            (edited, published.map(|snapshot| *snapshot))
        })
        .collect()
}

/// Refuses to go on unless the two runs about to be compared are worth comparing.
///
/// Two preconditions, and neither is the scenario's claim. One run has to be
/// showing the overlay and the other not, or the comparison is of the same run
/// twice and would agree whatever the toggle did; and the replay has to have
/// changed a block, or "the same resulting world state" is a statement about two
/// runs that did nothing to any world.
///
/// # Errors
///
/// Returns an error naming which of the two failed and what was seen instead.
fn require_worth_comparing(
    watched: &InputHarness,
    unwatched: &InputHarness,
    run: &[Step],
) -> Result<(), Box<dyn Error>> {
    if !watched.overlay_visible() || unwatched.overlay_visible() {
        return Err(format!(
            "one of these two runs has to be showing the overlay and the other not, or what is \
             compared is the same run twice. Showing it: {watched} and {unwatched}",
            watched = watched.overlay_visible(),
            unwatched = unwatched.overlay_visible()
        )
        .into());
    }
    if !run
        .iter()
        .any(|(edited, _)| matches!(edited, Some(EditReport::Changed { .. })))
    {
        return Err(format!(
            "the replay has to change the world, or 'the same resulting world state' is a claim \
             about two runs that touched no world. It reported: {:?}",
            run.iter().map(|(edited, _)| edited).collect::<Vec<_>>()
        )
        .into());
    }
    Ok(())
}

#[test]
fn pressing_the_key_the_declared_table_binds_the_toggle_to_changes_the_overlays_visibility() {
    let untouched = declared_client_showing(|_| {});
    let pressed = declared_client_showing(|client| client.press(DECLARED_TOGGLE));

    assert_ne!(
        pressed, untouched,
        "the overlay is the instrument somebody diagnosing this engine reaches for, and one press \
         of the key the declared table names is how they reach it. A client that never changed its \
         visibility has an overlay nobody can get at — which is indistinguishable, from outside, \
         from not having built one. It showed the overlay either way: {pressed}"
    );
}

#[test]
fn releasing_the_key_bound_to_the_toggle_leaves_the_overlays_visibility_where_the_press_left_it() {
    let untouched = declared_client_showing(|_| {});
    let pressed = declared_client_showing(|client| client.press(DECLARED_TOGGLE));
    let let_go = declared_client_showing(|client| {
        client.press(DECLARED_TOGGLE);
        client.release(DECLARED_TOGGLE);
    });

    assert_ne!(
        pressed, untouched,
        "the control this scenario needs: the press has to change the visibility, or the sameness \
         below is a client whose overlay never moves and the release is being credited for it"
    );
    assert_eq!(
        let_go, pressed,
        "a release is the player letting go of a key whose whole effect has already happened, so \
         it changes nothing. A client that acted on both transitions would make one \
         press-and-release two changes and leave the overlay exactly where it started — a key that \
         appears to do nothing, and that appears to do nothing however many times it is pressed"
    );
}

#[test]
fn pressing_the_key_a_rebound_table_puts_the_toggle_on_changes_the_overlays_visibility() {
    let untouched = client_showing(REBOUND, |_| {});
    let pressed = client_showing(REBOUND, |client| client.press(REBOUND_TOGGLE));

    assert_ne!(
        pressed, untouched,
        "the toggle is an *action* the table names a key for, so a table naming a different key \
         has to toggle from that key. A client that read the key directly would leave this one \
         doing nothing, and the player who remapped it would have no way to open the overlay at \
         all — while the scenario above stayed green"
    );
}

#[test]
fn pressing_the_declared_toggle_key_after_the_toggle_moved_leaves_the_overlays_visibility_alone() {
    let untouched = client_showing(REBOUND, |_| {});
    let on_its_new_key = client_showing(REBOUND, |client| client.press(REBOUND_TOGGLE));
    let on_the_key_it_left = client_showing(REBOUND, |client| client.press(DECLARED_TOGGLE));

    assert_ne!(
        on_its_new_key, untouched,
        "the control this scenario needs: the key this table *does* name has to toggle, or the \
         sameness below is a client that toggles from no key whatever and the assertion is about \
         nothing"
    );
    assert_eq!(
        on_the_key_it_left, untouched,
        "a binding that moved is a binding the old key no longer has, and this is the half a \
         hardcoded key fails: a client that toggled from the declared key *as well* would satisfy \
         every other scenario in this file while the remapping did nothing but add a second key"
    );
}

#[test]
fn a_fixed_replay_reaches_the_same_world_state_with_the_overlay_shown_and_with_it_hidden()
-> TestResult {
    let mut watched = InputHarness::started();
    watched.start_world()?;
    watched.press(DECLARED_TOGGLE);
    let with_the_overlay = replay(&mut watched);

    let mut unwatched = InputHarness::started();
    unwatched.start_world()?;
    let without_it = replay(&mut unwatched);
    require_worth_comparing(&watched, &unwatched, &with_the_overlay)?;

    assert_eq!(
        with_the_overlay, without_it,
        "watching an instrument is not playing the game. The overlay's toggle is a client-side \
         answer that never reaches the simulation, so a fixed sequence of input leaves the \
         identical world behind either way — every tick's edit and every tick's published player, \
         in order. A toggle that leaked into the tick would make a replay depend on whether \
         somebody had the overlay open while it ran, which is the property that makes a replay \
         evidence at all"
    );
    Ok(())
}
