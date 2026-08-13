//! What the client's adapter makes of the window's input: which key means what,
//! which keys mean nothing, what a lost focus does to the keys still held, and
//! whether pointer motion is the player looking around or the desktop's own
//! cursor.
//!
//! **These tests live here because this is where the key codes are.** The
//! binding table is data in `src/events.rs`, the one file of this crate that may
//! name the window library, and a table asserted against anything other than
//! real key codes would be asserting a copy of itself. `tests/winit_boundary.rs`
//! guards `src/` and nothing else, deliberately: a test that could not name the
//! library could not ask the question the table exists to answer.
//!
//! This is also the only crate that resolves both halves of the seam, which is
//! what lets two of these scenarios be asserted end to end rather than in two
//! halves that agree with each other by construction. A lost focus is a
//! `mc-render` policy (`FocusLost` → `ClearInput`) whose consequence is an
//! `mc-sim` one (`clear_held`), and pointer motion is admitted by one crate and
//! accumulated by the other. Each is one scenario and gets one test, spanning
//! both — the same shape `window_test.rs`'s own close-request test uses, where a
//! single scenario asserts the loop's action and the exit status together.
//!
//! **The table is asserted in exactly one place** (specification §"Table-driven
//! scenarios"): one test whose rows are the five declared entries, and no other
//! test in this workspace restates a row of it.
//!
//! Two of the four scenarios here assert that something is *unchanged*, and an
//! adapter that did nothing at all would satisfy both. Each therefore carries a
//! guard that the thing it is asking about happens at all: a bound key really
//! does reach the intent, and captured pointer motion really does reach the look
//! delta.

use mc_client::events::{bound_action, kind_of};
use mc_render::window::{CaptureState, LoopAction, accepts_pointer_motion, window_event_action};
use mc_sim::player::{InputState, MovementIntent};
use winit::event::WindowEvent;
use winit::keyboard::KeyCode;

/// An intent asking for nothing at all — no walk, no turn, no jump.
const NOTHING: MovementIntent = MovementIntent {
    forward: 0.0,
    strafe: 0.0,
    yaw_delta: 0.0,
    pitch_delta: 0.0,
    jump: false,
};

/// The binding table the specification declares, and the pending intent each of
/// its rows produces.
///
/// Forward is +1 and back is −1 on the same request, and strafe-right is +1
/// where strafe-left is −1, because the two pairs are opposite deflections of
/// one axis rather than four independent requests — which is what makes holding
/// both of a pair cancel rather than double.
const DECLARED_BINDINGS: [(KeyCode, MovementIntent); 5] = [
    (
        KeyCode::KeyW,
        MovementIntent {
            forward: 1.0,
            ..NOTHING
        },
    ),
    (
        KeyCode::KeyS,
        MovementIntent {
            forward: -1.0,
            ..NOTHING
        },
    ),
    (
        KeyCode::KeyA,
        MovementIntent {
            strafe: -1.0,
            ..NOTHING
        },
    ),
    (
        KeyCode::KeyD,
        MovementIntent {
            strafe: 1.0,
            ..NOTHING
        },
    ),
    (
        KeyCode::Space,
        MovementIntent {
            jump: true,
            ..NOTHING
        },
    ),
];

/// Keys no row of that table names.
///
/// Four rather than one, and chosen to sit around the table rather than far from
/// it: two letters adjacent to the bound ones on the keyboard, a modifier the
/// player will hold constantly for reasons this feature has nothing to do with,
/// and an arrow key a client might plausibly have bound instead.
const UNBOUND_KEYS: [KeyCode; 4] = [
    KeyCode::KeyQ,
    KeyCode::KeyE,
    KeyCode::ShiftLeft,
    KeyCode::ArrowUp,
];

/// The keys held when the window is taken away below.
///
/// Three bound keys that do not cancel each other, so the intent they produce
/// differs from [`NOTHING`] on all three of its fields — with W and S together
/// the walk request would be zero before anything was cleared, and clearing them
/// would assert nothing.
const HELD_WHEN_FOCUS_IS_LOST: [KeyCode; 3] = [KeyCode::KeyW, KeyCode::KeyA, KeyCode::Space];

/// The pointer motion delivered below, in raw device counts. Any non-zero
/// quantity would do; this is the one the look scenarios use.
const RAW_MOTION: f32 = 100.0;

/// What one pointer motion arriving in `capture` leaves in the pending intent.
///
/// The gate is the whole subject: the adapter is handed motion by the operating
/// system whatever the cursor is doing, and whether that motion is the player
/// looking around is [`accepts_pointer_motion`]'s answer and not the adapter's.
fn motion_seen_while(capture: CaptureState) -> MovementIntent {
    let mut input = InputState::default();
    if accepts_pointer_motion(capture) {
        input.look(RAW_MOTION, 0.0);
    }
    input.take_intent()
}

/// A pending intent with `keys` pressed and nothing released.
fn holding(keys: &[KeyCode]) -> InputState {
    let mut input = InputState::default();
    for key in keys {
        input.apply(bound_action(*key), true);
    }
    input
}

#[test]
fn every_key_the_declared_table_binds_records_its_own_row_in_the_pending_intent() {
    let mut wrong = Vec::new();

    for (key, expected) in DECLARED_BINDINGS {
        let recorded = holding(&[key]).take_intent();
        if recorded != expected {
            wrong.push(format!("{key:?} recorded {recorded:?}, not {expected:?}"));
        }
    }

    assert!(
        wrong.is_empty(),
        "the five rows are the whole table — W forward, S back, A strafe-left, D strafe-right, \
         Space jump — and each row records its own request and only its own. {} of the {} rows \
         recorded something else: {wrong:?}",
        wrong.len(),
        DECLARED_BINDINGS.len()
    );
}

#[test]
fn a_key_the_table_binds_nothing_to_leaves_the_pending_intent_as_it_was() {
    let mut input = holding(&[KeyCode::KeyW]);
    let before = input.take_intent();

    for key in UNBOUND_KEYS {
        input.apply(bound_action(key), true);
    }
    let after = input.take_intent();

    assert!(
        before != NOTHING,
        "the control this scenario needs: a bound key has to reach the pending intent at all, or \
         the intent below is unchanged because nothing here changes it. Holding W asked for \
         {before:?}"
    );
    assert_eq!(
        after, before,
        "a key the table does not name is not this game's input: pressing {UNBOUND_KEYS:?} while \
         W is held leaves the intent exactly as W left it, {before:?}. It became {after:?} — a \
         table that answered for keys it does not declare would walk the player every time a \
         modifier or a chat key went down"
    );
}

#[test]
fn losing_the_window_clears_every_key_the_player_was_holding() {
    let mut input = holding(&HELD_WHEN_FOCUS_IS_LOST);
    let held = input.take_intent();

    let action = window_event_action(&kind_of(&WindowEvent::Focused(false)));
    input.clear_held();

    assert!(
        held != NOTHING,
        "the control this scenario needs: the keys have to be held before losing focus can clear \
         them. {HELD_WHEN_FOCUS_IS_LOST:?} asked for {held:?}"
    );
    assert_eq!(
        (action, input.take_intent()),
        (LoopAction::ClearInput, NOTHING),
        "the key-up events for keys held when a window loses focus are delivered to whatever has \
         focus now and never arrive here, so the window going away is itself the release: the loop \
         is told to clear the input and every held key goes with it. This one answered {action:?} \
         and left {:?} behind — a client that kept them walks into a wall for as long as the player \
         is looking at another window, and is still walking when they come back",
        input.take_intent()
    );
}

#[test]
fn pointer_motion_that_arrives_while_the_cursor_is_free_adds_nothing_to_the_look() {
    let captured = motion_seen_while(CaptureState::Locked);

    let free = motion_seen_while(CaptureState::Uncaptured);

    assert!(
        captured != NOTHING,
        "the control this scenario needs: motion that arrives while the cursor *is* captured has \
         to reach the look delta, or the emptiness below is a client that never looks anywhere. \
         {RAW_MOTION} counts under a locked cursor asked for {captured:?}"
    );
    assert_eq!(
        free, NOTHING,
        "an uncaptured pointer belongs to the desktop: the player is moving a cursor over other \
         windows, and the same {RAW_MOTION} counts add nothing to the pending look. This left \
         {free:?} — a client that turned the camera anyway would spin the view while the player \
         was using another window, and would keep the spin waiting for them when they came back"
    );
}
