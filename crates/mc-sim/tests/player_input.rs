//! What a client accumulates between two ticks: the keys it is holding, and the
//! pointer motion that has arrived since the last intent went out.
//!
//! **The two kinds of input are kept for different lengths of time, and that is
//! the subject here.** A held key is a *state*: it contributes to every tick
//! between the press and the release, so submitting an intent must not forget
//! it. Pointer motion is a *quantity*: it arrived once, so the tick it feeds
//! consumes it and the tick after it starts from nothing. An accumulator that
//! got either the wrong way round is smooth, total and reproducible while being
//! wrong — a drained key stops the player mid-walk on the tick after the press,
//! and a look delta that is never drained turns the camera further on every tick
//! that follows one flick of the pointer.
//!
//! **Nothing below is read off a run.** The declared constants are the look
//! sensitivity (0.0022 radians per raw pointer count), the walk speed and the
//! tick duration, and every expected figure is written as the arithmetic over
//! them rather than as the number it comes to. The angle 100 counts produce is
//! `100 × 0.0022` and is spelled that way.
//!
//! **The comparison epsilon is derived from both directions**
//! (`standards/global/testing.md` §2). The arithmetic here is two `f32`
//! multiplications and one addition around 0.22, whose error is a couple of ulps
//! — about 3 × 10⁻⁸ — while the smallest defect these tests must still catch is
//! an implementation that keeps only the last of two motions, which is 0.11 out.
//! The specification's declared 1 × 10⁻⁴ sits three orders above the one and
//! three below the other, so floating-point noise cannot fail these tests and no
//! defect they are written for can pass them. Where *unchanged* is the claim the
//! comparison is on bits instead, which is both its exact form and the form
//! `clippy::float_cmp` has no quarrel with.
//!
//! The binding table itself is not here. A key no row names cannot reach
//! [`InputState::press`], which takes an action rather than a key, so the
//! scenarios about *which key means what* are asserted where the table lives and
//! against real key codes — `crates/mc-client/tests/window_input.rs`.

mod support;

use glam::Vec3;
use mc_sim::player::{InputState, PlayerAction, PlayerState, advance_player};

use support::solidity::Ground;

/// How far two figures this feature calls equal may differ, in radians or in
/// blocks. The specification's declared comparison epsilon; the module header
/// derives why it fits this file's arithmetic from both directions.
const EPSILON: f32 = 1e-4;

/// How much of a turn one raw pointer count asks for, in radians. Declared.
const LOOK_SENSITIVITY: f32 = 0.0022;

/// The pointer motion the scenarios below deliver, in raw device counts, and the
/// half of it that two events split between them.
const FULL_MOTION: f32 = 100.0;
const HALF_MOTION: f32 = 50.0;

/// The angle that motion asks for, in radians.
///
/// Written as the product of the two declarations rather than as the 0.22 it
/// comes to, so that a sensitivity changed in the specification changes this
/// expectation with it and a sensitivity changed in the *implementation* does
/// not.
const DECLARED_ANGLE: f32 = FULL_MOTION * LOOK_SENSITIVITY;

/// The angle a client that kept only the last of two motions would report.
const ONLY_THE_LAST: f32 = HALF_MOTION * LOOK_SENSITIVITY;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast a walk carries the player, in blocks per second. Declared.
const WALK_SPEED: f32 = 4.5;

/// How long a key is held for below, in ticks, and how far one walk of that
/// length covers.
const HELD_TICKS: u32 = 60;
const WALKED: f32 = WALK_SPEED * TICK_DURATION * HELD_TICKS as f32;

/// The topmost solid voxel of the flat floor the walks happen on, and where its
/// top face — and so a standing player's feet — therefore is.
const FLOOR_SURFACE: i32 = 63;
const FLOOR_TOP: f32 = (FLOOR_SURFACE + 1) as f32;

/// Where the player's feet start.
///
/// Off-lattice on both horizontal axes and different on each, so a walk that
/// went out along the axis it did not mean has nowhere to hide.
const START: Vec3 = Vec3::new(10.5, FLOOR_TOP, 3.5);

/// The flat floor every walk below happens on.
fn floor() -> Ground {
    Ground::Flat {
        surface: FLOOR_SURFACE,
    }
}

/// A player standing still on that floor, facing +x.
fn standing() -> PlayerState {
    PlayerState {
        position: START,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    }
}

/// Where `ticks` ticks of whatever `input` is holding leave the player.
///
/// The intent is taken afresh every tick, which is what a client does and what
/// makes the held keys' survival part of what is being asked: an accumulator
/// that forgot a key on submission would walk exactly one tick.
fn walk_holding(input: &mut InputState, ticks: u32) -> PlayerState {
    let floor = floor();
    let mut state = standing();
    for _ in 0..ticks {
        let intent = input.take_intent();
        state = advance_player(state, &intent, &floor);
    }
    state
}

/// A horizontal position as the integers its floats are. "Unchanged" means the
/// same value, not a nearly equal one.
fn horizontally(state: &PlayerState) -> (u32, u32) {
    (state.position.x.to_bits(), state.position.z.to_bits())
}

#[test]
fn releasing_one_of_two_held_keys_takes_only_its_own_contribution_out() {
    let mut input = InputState::default();
    input.press(PlayerAction::Forward);
    input.press(PlayerAction::StrafeRight);

    input.release(PlayerAction::Forward);

    let intent = input.take_intent();
    assert!(
        intent.forward.abs() <= EPSILON && (intent.strafe - 1.0).abs() <= EPSILON,
        "a release clears the released key's contribution and leaves every other key exactly where \
         it was, so letting go of forward while strafe-right is still down asks for a pure sideways \
         walk: forward 0 and strafe 1. This intent asked for forward {} and strafe {} — a release \
         that cleared nothing keeps walking the player forward after the key came up, and one that \
         cleared everything drops the key the player is still holding",
        intent.forward,
        intent.strafe
    );
}

#[test]
fn holding_forward_and_back_together_walks_the_player_nowhere() {
    let mut opposed = InputState::default();
    opposed.press(PlayerAction::Forward);
    opposed.press(PlayerAction::Back);
    let mut forward_only = InputState::default();
    forward_only.press(PlayerAction::Forward);

    let held_both = walk_holding(&mut opposed, HELD_TICKS);
    let held_forward = walk_holding(&mut forward_only, HELD_TICKS);

    assert!(
        (held_forward.position.x - (START.x + WALKED)).abs() <= EPSILON,
        "the control this scenario needs: one key alone walks {WALKED} blocks along +x in \
         {HELD_TICKS} ticks, so a player that stayed put below stayed put because the two keys \
         cancelled and not because nothing here walks at all. Forward alone reached {}",
        held_forward.position.x
    );
    assert_eq!(
        horizontally(&held_both),
        horizontally(&standing()),
        "the two keys are opposite requests of equal size, so holding both asks for no walk at all \
         and the player stays at ({}, {}) for all {HELD_TICKS} ticks. It ended at ({}, {}) — a \
         client that took the later key, or that summed their magnitudes instead of their signed \
         requests, walks a player who is asking to stand still",
        START.x,
        START.z,
        held_both.position.x,
        held_both.position.z
    );
}

#[test]
fn a_hundred_counts_of_pointer_motion_to_the_right_turn_the_view_by_the_declared_angle() {
    let mut input = InputState::default();

    input.look(FULL_MOTION, 0.0);

    let yaw_delta = input.take_intent().yaw_delta;
    assert!(
        (yaw_delta - DECLARED_ANGLE).abs() <= EPSILON,
        "the sensitivity is what turns device counts into an angle: {FULL_MOTION} counts to the \
         right ask for {FULL_MOTION} × {LOOK_SENSITIVITY} = {DECLARED_ANGLE} radians of yaw, and \
         the sign is positive because +x on the screen turns the same way the yaw grows. This \
         intent asked for {yaw_delta}, and a delta of {FULL_MOTION} is a client handing the \
         simulation raw hardware counts"
    );
}

#[test]
fn a_hundred_counts_of_pointer_motion_down_the_screen_lower_the_view_by_the_declared_angle() {
    let mut input = InputState::default();

    input.look(0.0, FULL_MOTION);

    let pitch_delta = input.take_intent().pitch_delta;
    assert!(
        (pitch_delta + DECLARED_ANGLE).abs() <= EPSILON,
        "screen +y points *down* and a positive pitch looks *up*, so the pitch takes the opposite \
         sign to the count: {FULL_MOTION} counts down the screen ask for {} radians. This intent \
         asked for {pitch_delta}, and getting the sign wrong here is an inverted look that is \
         otherwise indistinguishable from a correct one",
        -DECLARED_ANGLE
    );
}

#[test]
fn the_look_delta_a_tick_carried_is_gone_from_the_tick_after_it() {
    let mut input = InputState::default();
    input.look(FULL_MOTION, FULL_MOTION);

    let submitted = input.take_intent();
    let next = input.take_intent();

    assert!(
        submitted.yaw_delta.abs() > EPSILON && submitted.pitch_delta.abs() > EPSILON,
        "the control this scenario needs: the tick that consumed the motion has to have carried it, \
         or the emptiness below is an accumulator that never accumulated. It carried yaw {} and \
         pitch {}",
        submitted.yaw_delta,
        submitted.pitch_delta
    );
    assert!(
        next.yaw_delta.abs() <= EPSILON && next.pitch_delta.abs() <= EPSILON,
        "pointer motion is a quantity that arrived once, not a state that is held: the tick it fed \
         consumed it, so the next intent asks for no turn at all until more motion arrives. This \
         one asked for yaw {} and pitch {} — a delta that is never drained turns the camera further \
         on every tick after a single flick of the pointer, for as long as the player does not \
         touch it again",
        next.yaw_delta,
        next.pitch_delta
    );
}

#[test]
fn two_motions_arriving_before_one_tick_are_submitted_as_their_sum() {
    let mut input = InputState::default();

    input.look(HALF_MOTION, 0.0);
    input.look(HALF_MOTION, 0.0);

    let yaw_delta = input.take_intent().yaw_delta;
    assert!(
        (yaw_delta - DECLARED_ANGLE).abs() <= EPSILON,
        "the pointer reports far more often than the tick does, so an intent carries everything \
         that arrived since the last one: two motions of {HALF_MOTION} counts are one turn of \
         {DECLARED_ANGLE} radians. This intent asked for {yaw_delta}; {ONLY_THE_LAST} is a client \
         that overwrote the first motion with the second and quietly loses most of a fast flick"
    );
}
