//! What a click asks the world for: which button asks for what, when a press is
//! allowed to ask at all, and how many times one press asks.
//!
//! Every scenario here is driven through the client's own dispatch in a process
//! that constructs no event loop, opens no window and acquires no GPU adapter. A
//! real `MouseInput` event crosses `events.rs` on every one of them, so the
//! translation from the window library's button into the vocabulary the client
//! decides in runs under test rather than being handed over already made.
//!
//! # The oracle is the report the tick hands back, because there is nothing else
//!
//! The session owns the simulation and hands out no borrow of it, so no test here
//! can read a block out of the world. What it can read is what the tick answers:
//! the cell one requested action changed, the block that cell held, and the block
//! it holds now. That is enough for everything below, and it is deliberately less
//! than a world reader — a reader would be an accessor onto what the session owns,
//! which is the one property the session's own header says it does not give away.
//!
//! # The aim, derived from the declared fixture and never guessed
//!
//! The harness world is a single chunk column, 16 blocks across in x and z, every
//! voxel of it open except the one solid layer at `y == 9`. The player spawns
//! standing on that layer at `(8.5, 10.0, 8.5)` facing +x, so the eye — 1.62 above
//! the feet — sits at `(8.5, 11.62, 8.5)` and a level ray meets nothing at all.
//! Every scenario that wants a block therefore aims *down*, and how far down is
//! the whole difficulty:
//!
//! [`AIM_DOWN_COUNTS`] raw counts of downward pointer motion is `280 × 0.0022 =
//! 0.616` radians, or 35.29° below level. The declared sensitivity is not exported
//! and is named here rather than read, which is a duplication the last test in
//! this file re-derives at runtime instead of trusting.
//!
//! At that angle the ray falls the 1.62 blocks to the floor's top face over a
//! horizontal run of `1.62 / tan 35.29° = 2.2885`, entering the solid layer at
//! `x = 10.7885` — the cell [`LOOKED_AT`], through its **upward** face, 2.804
//! blocks from the eye and well inside the reach of 5.0. It stays inside the layer
//! for a further 1.4127 blocks of x, crossing into `(11, 9, 8)` at 3.063 and
//! `(12, 9, 8)` at 4.288; it leaves the layer at 4.535 and the cell beyond is
//! entered at 5.513, past the reach. **Three cells, and the three are the point.**
//!
//! # Why not straight down, which is the obvious aim and the wrong one
//!
//! A ray aimed straight down takes the one cell under the player's feet and then
//! finds nothing, because everything below the floor layer is open. A press that
//! latched and re-fired on every tick would change **exactly one block** through
//! such an aim, "one press is one action" would be green against it, and the suite
//! would report a kill it never made. A shallower aim walks the ray along the
//! floor and gives a latching press somewhere further to go, which is what makes
//! the count mean something — and the last test asserts that it does rather than
//! assuming it.
//!
//! # Three scenarios assert that no block changed, and each carries its control
//!
//! A click that arrives while the cursor belongs to the desktop, a button coming
//! back up, and a click made before there is a world to click in all assert that
//! the tick reported nothing — and a client that never turns any click into a
//! request satisfies all three. So each runs the *same* click through the *same*
//! aim in the minimally different configuration that is supposed to work, in the
//! same test, and requires that run to have changed a block.
//!
//! **The two answers are "no report" and "a changed block", and not two
//! refusals.** Nothing here is declined by the world: the request is never made at
//! all, so there is no report of any kind for the tick to hand back. A refusal
//! would mean the click reached the world and the world said no, which is a
//! different failure with a different fix, and the assertions below are written so
//! a reader can tell the two apart.
//!
//! # No block in this file is named
//!
//! What a break leaves behind and what a place puts down are graded against their
//! own declared fixtures in `mc-sim`. What is asked here is *which cell* a button
//! reaches and whether the block standing in it is a different block afterwards,
//! so every assertion below is a cell and a difference. Spelling the harness's own
//! block names would tie these scenarios to a declaration they have no stake in.
//!
//! # The harness is included by path
//!
//! Not through `tests/support/mod.rs`, which links `support/frames.rs` and with it
//! the whole graphics stack — into a binary whose entire premise is that no
//! adapter is acquired (conductor ruling 56).

#[path = "support/input/mod.rs"]
mod input;

use std::error::Error;

use mc_render::window::CaptureState;
use mc_sim::action::EditReport;
use mc_sim::player::BlockPos;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use input::InputHarness;

type TestResult = Result<(), Box<dyn Error>>;

/// How far the pointer is pushed down before a click, in raw device counts.
///
/// +y is *down*, which is the screen's convention and the one the operating
/// system reports in. See the header for what this angle was chosen to reach and
/// what the obvious alternative would have hidden.
const AIM_DOWN_COUNTS: f64 = 280.0;

/// The cell that aim first meets: the nearest solid voxel along the ray.
const LOOKED_AT: BlockPos = BlockPos { x: 10, y: 9, z: 8 };

/// The cell a placement against that aim lands in — one step back out through the
/// upward face the ray came in by, which is the cell directly above [`LOOKED_AT`].
const AGAINST_ITS_UPWARD_FACE: BlockPos = BlockPos {
    x: LOOKED_AT.x,
    y: LOOKED_AT.y + 1,
    z: LOOKED_AT.z,
};

/// How many ticks follow a single click, with nothing further dispatched.
///
/// Far more than the one that spends it, so a press that was never cleared has
/// every opportunity to fire again — and three cells to fire into before the ray
/// runs out of world.
const TICKS_AFTER_ONE_CLICK: u32 = 10;

/// How many clicks the control makes, one per tick.
///
/// One per cell the ray crosses, so the control measures the whole run the aim
/// has rather than merely more than one of it.
const CLICKS_THE_CONTROL_MAKES: u32 = 3;

/// A platform that refuses every capture it is asked for.
///
/// The only route to a cursor the client does not hold from the very first ask,
/// which is the state the re-grab scenario is stated over.
const REFUSES_EVERYTHING: [CaptureState; 0] = [];

#[test]
fn a_left_click_breaks_the_block_the_player_is_looking_at() -> TestResult {
    let mut aimed = aimed_at_the_floor()?;
    aimed.click(MouseButton::Left);

    assert_eq!(
        change(aimed.edit()),
        Some((LOOKED_AT, true)),
        "the left button is how a player digs, and the block it digs out is the one the server \
         says they are looking at — {LOOKED_AT:?}, derived from the declared fixture and the \
         declared aim rather than read back from a run. A client that carries the press and never \
         spends it leaves the tick with nothing to report at all, which is a game where the mouse \
         does nothing whatever"
    );
    Ok(())
}

#[test]
fn a_right_click_places_a_block_against_the_face_the_player_is_looking_at() -> TestResult {
    let mut aimed = aimed_at_the_floor()?;
    aimed.click(MouseButton::Right);

    assert_eq!(
        change(aimed.edit()),
        Some((AGAINST_ITS_UPWARD_FACE, true)),
        "the right button is how a player builds, and the block it builds into is the cell on the \
         near side of the face the ray came in by — {AGAINST_ITS_UPWARD_FACE:?}, one step back out \
         through the upward face of {LOOKED_AT:?}. The cell is also what tells the two buttons \
         apart: a client that mapped both to a break would change {LOOKED_AT:?} here and satisfy \
         nothing this scenario is about"
    );
    Ok(())
}

#[test]
fn a_left_click_while_the_pointer_is_the_desktops_asks_the_platform_for_it_again() {
    let mut freed = InputHarness::granting(&REFUSES_EVERYTHING);
    freed.click(MouseButton::Left);

    assert_eq!(
        freed.grabs(),
        vec![
            CaptureState::Locked,
            CaptureState::Confined,
            CaptureState::Locked,
            CaptureState::Confined
        ],
        "a click is how a player takes the cursor back after Escape gave it away, so a press \
         arriving while the client holds no pointer has to re-enter the ladder at the top and walk \
         it again — a locked pointer asked for and refused, then a confined one. The first two \
         asks are the session being built and are what leaves it holding nothing; the second two \
         are the click. A client that only acted on a click it already had the cursor for is \
         unplayable after a single keypress, with every other scenario in this file still green. \
         It asked for {:?}",
        freed.grabs()
    );
}

#[test]
fn a_left_click_while_the_pointer_is_the_desktops_changes_no_block() -> TestResult {
    let mut held = aimed_at_the_floor()?;
    held.click(MouseButton::Left);
    let acted = held.edit();

    let mut freed = aimed_at_the_floor()?;
    freed.press(KeyCode::Escape);
    freed.click(MouseButton::Left);
    let declined = freed.edit();

    assert_eq!(
        change(acted),
        Some((LOOKED_AT, true)),
        "the control this scenario needs: the same click under a pointer the game holds has to \
         change a block, or the emptiness below is a client that turns no click into a request \
         ever"
    );
    assert_eq!(
        declined, None,
        "Escape gives the cursor back to the desktop, and the click that takes it again is the \
         player reaching for their own window — not a request to dig a hole where the cursor \
         happened to land. The tick that follows reports **nothing**: not a refusal, which would \
         mean the request reached the world and the world declined it, but no request at all. \
         This one reported {declined:?}. The capture is read before the ladder is walked, which is \
         the whole subtlety — by the time the tick runs, this client is holding the pointer again"
    );
    Ok(())
}

#[test]
fn a_mouse_button_coming_back_up_changes_no_block() -> TestResult {
    let mut pressed = aimed_at_the_floor()?;
    pressed.click(MouseButton::Left);
    let acted = pressed.edit();

    let mut lifted = aimed_at_the_floor()?;
    lifted.unclick(MouseButton::Left);
    let quiet = lifted.edit();

    assert_eq!(
        change(acted),
        Some((LOOKED_AT, true)),
        "the control this scenario needs: the same button, pressed rather than released, has to \
         change a block — or the emptiness below is a client that reads no mouse button at all"
    );
    assert_eq!(
        quiet, None,
        "a button coming back up is the player letting go of something they have already spent. \
         The tick that follows reports nothing at all — not a refusal, which would mean the \
         release had become a request the world then declined. This one reported {quiet:?}. A \
         client that acted on both transitions digs two blocks for every click the player makes, \
         and the second is wherever the first left the ray pointing"
    );
    Ok(())
}

#[test]
fn a_left_click_made_before_the_world_lands_changes_no_block_when_it_does() -> TestResult {
    let mut afterwards = InputHarness::started();
    afterwards.move_pointer(0.0, AIM_DOWN_COUNTS);
    afterwards.start_world()?;
    afterwards.click(MouseButton::Left);
    let acted = afterwards.edit();

    let mut beforehand = InputHarness::started();
    beforehand.move_pointer(0.0, AIM_DOWN_COUNTS);
    beforehand.click(MouseButton::Left);
    beforehand.start_world()?;
    let declined = beforehand.edit();

    assert_eq!(
        change(acted),
        Some((LOOKED_AT, true)),
        "the control this scenario needs: the identical drive with the click made a moment later, \
         once the world has landed, has to change a block. The two runs differ in one thing only — \
         which side of the world's arrival the click falls on — so the emptiness below is about \
         that and not about a client which never clicks"
    );
    assert_eq!(
        declined, None,
        "a click made while the world is still generating is a click at a loading screen, and \
         unlike a held key it is not something the player is still asking for when the world \
         appears. The first tick after it lands reports nothing at all — not a refusal, which \
         would mean the stale press had become a request. This one reported {declined:?}. The look \
         the player made while they waited *is* carried across, which is why both runs aim the \
         same way and why the difference between them cannot be that one of them was looking \
         somewhere else"
    );
    Ok(())
}

#[test]
fn a_single_left_click_changes_one_block_over_the_ten_ticks_that_follow_it() -> TestResult {
    let clicked_every_tick = changed_over_a_click_each_tick(CLICKS_THE_CONTROL_MAKES)?;

    let mut once = aimed_at_the_floor()?;
    once.click(MouseButton::Left);
    let clicked_once = changed_cells(once.edits(TICKS_AFTER_ONE_CLICK));

    assert!(
        clicked_every_tick.len() > 1,
        "the control this scenario needs, and the one thing that stops the count below being a \
         fact about the fixture. The same aim, clicked once on each of \
         {CLICKS_THE_CONTROL_MAKES} ticks, has to change more than one block — three, over the \
         three cells this ray crosses inside the floor layer. Aimed straight down there would be \
         one cell and nothing under it, a press that re-fired on every tick would still change \
         exactly one block, and the assertion below would pass against precisely the client it \
         exists to catch. It changed {clicked_every_tick:?}"
    );
    assert_eq!(
        clicked_once.len(),
        1,
        "one press is one action: the click is spent by the tick it lands in and the \
         {TICKS_AFTER_ONE_CLICK} ticks that follow it ask the world for nothing, so exactly one \
         block changes over all of them. It changed {clicked_once:?}. A press that is copied out \
         of the session rather than taken out of it fires again on every tick the player holds the \
         button down — which is auto-repeat nobody asked for, digging a tunnel out of a click \
         meant to break one block"
    );
    Ok(())
}

/// Every cell that changes when a client over the same aim clicks once before
/// each of `clicks` ticks.
///
/// The control the one-press scenario is read against: it measures how far the
/// declared aim can reach when a press is made on every tick, which is what a
/// press that never cleared itself would do on its own.
fn changed_over_a_click_each_tick(clicks: u32) -> Result<Vec<BlockPos>, Box<dyn Error>> {
    let mut harness = aimed_at_the_floor()?;
    Ok((0..clicks)
        .filter_map(|_| {
            harness.click(MouseButton::Left);
            changed_cell(harness.edit())
        })
        .collect())
}

/// A client over the declared floor, looking down at it along the derived aim,
/// with no click dispatched yet.
///
/// The look is dispatched and not yet spent: the tick that resolves the first
/// click is the tick that turns the view, and the action resolves against the
/// view that tick *ends* with, so every cell named in this file is the one the
/// aimed ray meets rather than the one a level ray would have.
///
/// It is fallible because the world is declared block by block against a registry
/// the harness builds, and either can refuse. The refusal is reported rather than
/// absorbed, so a fixture that failed to build cannot be mistaken for a click that
/// changed nothing.
fn aimed_at_the_floor() -> Result<InputHarness, Box<dyn Error>> {
    let mut harness = InputHarness::started();
    harness.start_world()?;
    harness.move_pointer(0.0, AIM_DOWN_COUNTS);
    Ok(harness)
}

/// What one tick's report says about the world: the cell it changed, and whether
/// the block standing there is now a different block.
///
/// `None` is a tick that asked the world for nothing — which is *not* a refusal.
/// The two are told apart deliberately: a refusal means a request was made and
/// declined, and every scenario above that expects emptiness expects no request.
fn change(report: Option<EditReport>) -> Option<(BlockPos, bool)> {
    match report? {
        EditReport::Changed { cell, from, to } => Some((cell, from != to)),
        EditReport::Refused(_) => None,
    }
}

/// The cell one report changed, if it changed one.
fn changed_cell(report: Option<EditReport>) -> Option<BlockPos> {
    change(report).map(|(cell, _)| cell)
}

/// Every cell a run of reports says a block changed in, in the order they were
/// reported.
///
/// Refusals contribute nothing: a refused request is an answer to a question that
/// was asked, and what is being counted here is the blocks that changed.
fn changed_cells(reports: Vec<EditReport>) -> Vec<BlockPos> {
    reports
        .into_iter()
        .filter_map(|report| changed_cell(Some(report)))
        .collect()
}
