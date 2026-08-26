//! A frame that spends several ticks spends each input the right number of
//! times: the ones the player is *still making* on every tick, the ones they made
//! *once* exactly once.
//!
//! # This is a hold, not a repair
//!
//! Nothing here was broken. A frame used to be one tick, so "a look delta applied
//! three times" was unspellable and the drains that make it right —
//! `InputState::take_intent` draining the pointer motion while keeping the held
//! keys, and the session taking its pending action rather than reading it — were
//! already written this way for their own reasons. What changed is that a frame
//! can now spend fifteen ticks, and those two drains went from *correct* to
//! *load-bearing* with nothing watching them. A look applied three times is a 3×
//! sensitivity spike that only appears on a machine that stutters; a click applied
//! three times digs three holes from one press.
//!
//! So these three scenarios were green before the fix and are green after it, and
//! that is the point: they are what makes the next change to either drain fail
//! here instead of in somebody's game. Recorded plainly rather than presented as
//! regression evidence.
//!
//! # Three ticks in one frame, and why three
//!
//! Small enough to sit far inside the catch-up bound, so nothing below is
//! measuring the clamp; large enough that "once" and "per tick" differ by a
//! factor a reader can see rather than by a rounding.
//!
//! # The harness is included by path
//!
//! Not through `tests/support/mod.rs`, which links `support/frames.rs` and with it
//! the whole graphics stack — into a binary whose entire premise is that no
//! adapter is acquired (conductor ruling 56).

#[path = "support/input/mod.rs"]
mod input;

use std::error::Error;
use std::time::Duration;

use glam::Vec3;
use mc_sim::action::EditReport;
use mc_sim::player::{BlockPos, TICK_QUANTUM};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use input::InputHarness;

type TestResult = Result<(), Box<dyn Error>>;

/// The key the declared table binds to walking forward.
const FORWARD: KeyCode = KeyCode::KeyW;

/// How many ticks the frame under test spends. See the header.
const TICKS_IN_ONE_FRAME: u32 = 3;

/// The elapsed time a frame has to carry to spend that many.
const A_FRAME_OF_THREE_TICKS: Duration = Duration::from_nanos(3 * 16_666_667);

/// The pointer motion one frame carries, in raw device counts.
///
/// A twentieth of what the pointer scenarios drive, chosen so that applying it
/// three times is still a turn a camera can be asked about rather than a spin
/// past the pitch clamp — the scenario is about a *count* of applications, and a
/// clamped answer would hide a difference by saturating it.
const MOTION_COUNTS: f64 = 10.0;

/// How far the pointer is pushed down before a click, in raw device counts.
///
/// The same aim `click_dispatch.rs` uses, and the cell it reaches is that file's
/// derivation: +y is down, which is the screen's convention and the one the
/// operating system reports in.
const AIM_DOWN_COUNTS: f64 = 280.0;

/// The cell that aim first meets: the nearest solid voxel along the ray.
const LOOKED_AT: BlockPos = BlockPos { x: 10, y: 9, z: 8 };

/// How many further frames follow the one that carries the click.
///
/// Enough that a press which was never cleared has every opportunity to fire
/// again, and enough cells left along the ray for it to fire *into* — a run that
/// re-fired against an already-broken cell would report a refusal rather than a
/// second change, and would read as this scenario passing.
const FRAMES_AFTER_THE_CLICK: u32 = 5;

#[test]
fn a_frame_that_spends_three_ticks_turns_the_view_by_one_pointer_motion_once() -> TestResult {
    let over_three_ticks = facing_after(A_FRAME_OF_THREE_TICKS, MOTION_COUNTS)?;
    let over_one_tick = facing_after(TICK_QUANTUM, MOTION_COUNTS)?;
    let unturned = facing_after(TICK_QUANTUM, 0.0)?;

    assert_ne!(
        over_one_tick, unturned,
        "the control this scenario is read under: {MOTION_COUNTS} raw counts has to turn the \
         camera at all, or the equality below is two runs that ignored the pointer"
    );
    assert_eq!(
        over_three_ticks, over_one_tick,
        "a pointer motion is something the player did once, and the first tick of a frame drains \
         it. A frame that spent it on each of its {TICKS_IN_ONE_FRAME} ticks would turn the view \
         three times as far — a sensitivity that triples exactly when the machine stutters, which \
         is when a player can least afford it"
    );
    Ok(())
}

#[test]
fn a_frame_that_spends_three_ticks_carrying_one_break_request_breaks_one_voxel() -> TestResult {
    let mut aimed = aimed_at_the_floor()?;
    aimed.click(MouseButton::Left);

    let in_the_frame = changed_cell(aimed.frame(A_FRAME_OF_THREE_TICKS).0);
    let afterwards: Vec<BlockPos> = (0..FRAMES_AFTER_THE_CLICK)
        .filter_map(|_| changed_cell(aimed.frame(TICK_QUANTUM).0))
        .collect();

    assert_eq!(
        (in_the_frame, afterwards.as_slice()),
        (Some(LOOKED_AT), [].as_slice()),
        "one press is one action however many ticks the frame that carries it spends. The tick \
         that spends it *takes* the request rather than reading it, so the other \
         {} ticks of that frame carry none and the {FRAMES_AFTER_THE_CLICK} frames after it carry \
         none either. A client that copied the request out would dig a hole per tick — three from \
         one click on a stuttering machine, and a tunnel from a click on a hanging one",
        TICKS_IN_ONE_FRAME - 1
    );
    Ok(())
}

#[test]
fn a_held_walk_key_applies_to_every_tick_a_frame_spends() -> TestResult {
    let from = standing_still()?;
    let one_tick = walked_over(TICK_QUANTUM)?;
    let three_ticks = walked_over(A_FRAME_OF_THREE_TICKS)?;

    let one = (one_tick - from).x;
    let three = (three_ticks - from).x;

    assert!(
        one > 0.0,
        "the control this scenario is read under: one tick of a held key has to walk the player \
         forward, or the ratio below is nought over nought. It walked {one}"
    );
    assert!(
        (three - f64::from(TICKS_IN_ONE_FRAME) as f32 * one).abs() < one / 2.0,
        "a key the player is *still holding* is input for every tick a frame spends, which is the \
         opposite of the pointer motion above: the accumulator keeps its held keys and drains only \
         the look. So a frame of {TICKS_IN_ONE_FRAME} ticks walks {TICKS_IN_ONE_FRAME} ticks' \
         worth — a client that applied the walk once per frame runs slow by exactly the factor the \
         frame is long, which is the original defect wearing the other sign. One tick walked \
         {one}, the frame walked {three}"
    );
    Ok(())
}

/// Where a client holding the walk key stands before any frame is drawn.
fn standing_still() -> Result<Vec3, Box<dyn Error>> {
    Ok(walking()?
        .published()
        .ok_or("a client over a started world publishes where its player is standing")?
        .player
        .position)
}

/// Where one frame of `took` leaves a client holding the walk key.
fn walked_over(took: Duration) -> Result<Vec3, Box<dyn Error>> {
    let mut harness = walking()?;
    Ok(harness
        .frame(took)
        .1
        .ok_or("a frame over a started world publishes where its player is standing")?
        .player
        .position)
}

/// A client over the declared floor, holding the walk key and not yet given a
/// frame.
fn walking() -> Result<InputHarness, Box<dyn Error>> {
    let mut harness = InputHarness::started();
    harness.start_world()?;
    harness.press(FORWARD);
    Ok(harness)
}

/// Which way the camera faces after one frame of `took` carrying `counts` of
/// horizontal pointer motion.
///
/// The facing the *renderer* is handed — `target - eye` off the published pose —
/// rather than a yaw or a look delta, so a client that stopped consulting either
/// while still drawing the same wrong picture has nothing to hide behind.
fn facing_after(took: Duration, counts: f64) -> Result<Vec3, Box<dyn Error>> {
    let mut harness = InputHarness::started();
    harness.start_world()?;
    harness.move_pointer(counts, 0.0);
    let published = harness
        .frame(took)
        .1
        .ok_or("a frame over a started world publishes the pose it left the camera in")?;
    Ok(Vec3::from(published.camera.target) - Vec3::from(published.camera.eye))
}

/// A client over the declared floor, looking down at it along the derived aim,
/// with no click dispatched yet.
fn aimed_at_the_floor() -> Result<InputHarness, Box<dyn Error>> {
    let mut harness = InputHarness::started();
    harness.start_world()?;
    harness.move_pointer(0.0, AIM_DOWN_COUNTS);
    Ok(harness)
}

/// The cell one report says a block changed in, if it says one changed.
///
/// A refusal contributes nothing: it means a request was made and the world
/// declined it, which is a different answer from no request at all, and the
/// scenario above counts the blocks that changed.
fn changed_cell(report: Option<EditReport>) -> Option<BlockPos> {
    match report? {
        EditReport::Changed { cell, from, to } if from != to => Some(cell),
        EditReport::Changed { .. } | EditReport::Refused(_) => None,
    }
}
