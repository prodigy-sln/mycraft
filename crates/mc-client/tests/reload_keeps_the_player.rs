//! The player crosses a reload exactly where the ticks put them, and moving
//! exactly as fast.
//!
//! # The reading is taken one tick *after* the swap, and that is the whole of it
//!
//! A swap publishes no tick of its own — it is what happens between two of them,
//! and a scenario in this suite pins that. So the snapshot standing when a
//! candidate is taken up is the one the *previous* `advance` stored, and **nothing
//! the swap does to the player can change what it holds.** A comparison taken
//! there is unfalsifiable: it reads a value written before the thing it is judging
//! ran. Measured, not reasoned — setting the player's position to the origin
//! inside the swap left the earlier form of this file green.
//!
//! What closes it is one tick, advanced in both clients after the candidate lands.
//! The swap writes the player's own fields; the tick after it publishes them; and
//! the two clients are comparable at that tick because **neither of them is
//! standing on the block the candidate changed**:
//!
//! - The walking scenario walks on a floor of grass and the candidate takes
//!   `base:stone`'s solidity away. Stone is the *ceiling*, so the swap cannot move
//!   either player and the tick after it is the same tick for both — unless the
//!   swap moved one of them, which is the claim.
//! - The falling scenario is in free fall two blocks above the floor. One tick of
//!   falling is one tick of falling whatever the floor is declared to be, and the
//!   two runs part company later, when one of them lands and the other does not.
//!
//! **A player standing on the changed block would break this in the other
//! direction** — the tick after the swap would drop them, legitimately, and the
//! comparison would go red against a *correct* client. That is the over-tight
//! assertion `testing.md` §2 names, and it is why the floor here is grass.
//!
//! # The oracle is a second run and never a number copied out of the first
//!
//! Both scenarios run two clients side by side over the same world, driven by the
//! same script, and hand a candidate to only one of them. A position copied from a
//! green run would have been worse than useless: gravity acts on every tick and
//! walking moves the player on every tick, so "exactly where they were" is not
//! what a correct client produces, it is what a frozen one produces.
//!
//! **The player is moving at the swap, and that is a fixture requirement rather
//! than a detail.** Two different mutations need it, and they need different parts
//! of it:
//!
//! - *The oracle advanced one tick too few* is caught by the walk. A player
//!   standing still publishes the same position every tick, so a short oracle would
//!   agree with the run it judges. Measured: one tick of walking is 0.072 blocks in
//!   x, and yaw, pitch and height are bit-identical across it — **only the two
//!   horizontal axes carry that signal.** Comparing orientation alone would retire
//!   this defence entirely.
//! - *The swap zeroes the velocity* is caught only while the velocity is not
//!   already zero. The difference the tick after the swap is exactly the velocity
//!   standing before it, so a reading taken at rest reports nothing.
//!
//! Each scenario guards its own half.
//!
//! # Velocity is asked about a player nobody moved
//!
//! The falling scenario is about a player who was **not** cleared out of anything:
//! the candidate takes solidity *away*, so no cell their box overlaps becomes solid
//! and no clearing move can be what left their velocity where it is. A scenario
//! that let those two paths overlap would be unable to say which it was watching.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::path::Path;

use mc_core::id::BlockName;
use mc_sim::simulation::{SimSnapshot, Simulation};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use input::InputHarness;
use reload::{
    Adoption, GRASS, STONE, STONE_FILE, adoption, candidate, restating, shipped,
    stone_that_is_not_solid,
};
use reload_world::{
    AIM_AT_THE_CEILING, Edit, SPAWN, at_rest, falling_from, floor_of, floor_under_a_ceiling,
    moving_at, playing, resting, standing, standing_and_facing,
};
use support::{TestResult, content_root};

/// How many raw device counts the walking scenario turns by before it sets off,
/// so that the orientation it compares is one somebody moved.
const TURN_ACROSS: f64 = 120.0;
const TURN_DOWN: f64 = 80.0;

/// How many ticks the walking scenario spends walking. Well inside the floor:
/// twenty ticks at the declared walk speed is about a block and a half, against
/// seven blocks of margin to the nearest edge.
const WALKING_TICKS: u32 = 20;

/// Where the falling scenario drops from, and how many ticks it falls before the
/// candidate arrives. Two blocks of fall are a fraction of a block at any
/// plausible gravity, so the player is still well above the floor and still
/// gathering speed.
const DROPPED_FROM: f32 = 12.0;
const FALLING_TICKS: u32 = 6;

/// The one tick that carries what the swap wrote into what a reader can see.
const THE_TICK_AFTER_THE_SWAP: u32 = 1;

/// How many ticks both falling clients advance after the comparison, so that one
/// has plainly landed and the other plainly has not. A second of falling covers
/// far more than the two blocks between the drop and the floor.
const AFTERWARDS: u32 = 60;

#[test]
fn a_reload_leaves_the_player_where_the_same_ticks_with_no_reload_would_have_put_them() -> TestResult
{
    let serving = content_root()?;
    let mut reloading = a_client_walking_under_a_stone_ceiling(&serving)?;
    let mut untouched = a_client_walking_under_a_stone_ceiling(&serving)?;
    walked(&mut reloading);
    walked(&mut untouched);

    let root = restating(shipped()?, STONE_FILE, &stone_that_is_not_solid())?;
    let answered = adoption(reloading.adopt(candidate(root.path())?));
    require_admitted(&answered)?;

    reloading.ticks(THE_TICK_AFTER_THE_SWAP);
    untouched.ticks(THE_TICK_AFTER_THE_SWAP);
    let across_the_swap = standing_and_facing(&published(&reloading)?);
    let oracle = standing_and_facing(&published(&untouched)?);
    require_moved(oracle.0)?;

    assert_eq!(
        (
            across_the_swap,
            found_the_ceiling(&broke_upward(&mut untouched)),
            found_the_ceiling(&broke_upward(&mut reloading))
        ),
        (oracle, true, false),
        "a swap is not a landing, a launch or a teleport: where the player is standing and which \
         way they are looking are the ticks' answer, and the reload has nothing to say about \
         either. Both are read a tick *after* the candidate landed, because a swap publishes no \
         tick of its own — read at the swap, this compares a snapshot written before the thing it \
         is judging ran, and a swap that dropped the player at the origin goes unnoticed. The \
         ceiling one of them can still break and the other's look passes straight through is what \
         says the candidate was taken up at all"
    );
    Ok(())
}

#[test]
fn a_reload_while_the_player_is_falling_leaves_their_velocity_where_the_tick_left_it() -> TestResult
{
    let serving = content_root()?;
    let mut reloading = a_client_falling(&serving)?;
    let mut untouched = a_client_falling(&serving)?;
    reloading.ticks(FALLING_TICKS);
    untouched.ticks(FALLING_TICKS);
    require_still_falling(&untouched, moving_at(&published(&untouched)?))?;

    let root = restating(shipped()?, STONE_FILE, &stone_that_is_not_solid())?;
    let answered = adoption(reloading.adopt(candidate(root.path())?));
    require_admitted(&answered)?;

    reloading.ticks(THE_TICK_AFTER_THE_SWAP);
    untouched.ticks(THE_TICK_AFTER_THE_SWAP);
    let across_the_swap = moving_at(&published(&reloading)?);
    let oracle = moving_at(&published(&untouched)?);

    reloading.ticks(AFTERWARDS);
    untouched.ticks(AFTERWARDS);

    assert_eq!(
        (across_the_swap, held_up(&reloading)?, held_up(&untouched)?),
        (oracle, false, true),
        "a reload is neither a landing nor a launch, so a player half way through a fall goes on \
         falling at the speed the tick left them at. Zeroing it would drop them out of the sky the \
         moment somebody saved a file, and setting it would throw them. The speed is read the tick \
         *after* the candidate landed, because that is the first tick a swap could have written \
         into — and the difference it would show is exactly the speed standing before it, which is \
         why a reading taken at rest reports nothing. The floor one of them lands on and the other \
         falls through says the candidate was taken up at all"
    );
    Ok(())
}

/// A client standing on a floor of grass with a whole layer of stone overhead.
///
/// **Grass under the feet and stone over the head, and the two may not be
/// exchanged.** The candidate takes stone's solidity away, so a player standing on
/// stone would begin to fall on the tick this scenario compares — legitimately,
/// which would put the comparison red against a correct client.
fn a_client_walking_under_a_stone_ceiling(root: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, standing(), |registry| {
        floor_under_a_ceiling(registry, GRASS, STONE)
    })?;
    Ok(playing_client(simulation, holding))
}

/// A client dropped over a floor of stone, at rest and about to fall.
fn a_client_falling(root: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, falling_from(DROPPED_FROM), |registry| {
        floor_of(registry, STONE)
    })?;
    Ok(playing_client(simulation, holding))
}

/// A started client already playing what it was handed.
fn playing_client(simulation: Simulation, holding: BlockName) -> InputHarness {
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client
}

/// The one script both walking clients are driven by.
///
/// Written once and handed to each, so the two runs differ in the candidate and in
/// nothing else — a script spelled twice is two places for the oracle to stop
/// being one. The key is never released, so both go on walking through the tick
/// after the swap.
fn walked(client: &mut InputHarness) {
    client.move_pointer(TURN_ACROSS, TURN_DOWN);
    client.press(KeyCode::KeyW);
    client.ticks(WALKING_TICKS);
}

/// What a break aimed at the ceiling does.
///
/// A whole layer overhead, so this reaches it wherever the walk ended up. Before
/// the candidate the look stops at the stone; after it, stone stops anything from
/// stopping a look, and the ray leaves the world through the space above.
fn broke_upward(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_AT_THE_CEILING);
    client.click(MouseButton::Left);
    edit_of(client)
}

/// Whether one break found anything at all to break.
///
/// The ceiling is the only thing over a player's head, so "found something" and
/// "the ceiling is still solid" are one answer here — and `NoTarget` is what a
/// look through a block content no longer calls solid comes back with.
fn found_the_ceiling(broke: &Edit) -> bool {
    !matches!(broke, Edit::NoTarget)
}

/// One tick of the client, reported as what its action did.
fn edit_of(client: &mut InputHarness) -> Edit {
    reload_world::edit(client.edit())
}

/// Whatever the client has published.
///
/// Taken by value rather than by pointer: what a client published is a tick, a
/// pose and a player state, all plain values, and a scenario comparing two runs
/// wants the values rather than the cells they were read out of.
///
/// # Errors
///
/// Returns an error where it has published nothing, which is a client with no
/// world rather than one standing anywhere.
fn published(client: &InputHarness) -> Result<SimSnapshot, Box<dyn Error>> {
    client
        .published()
        .map(|published| *published)
        .ok_or_else(|| "this fixture's client has published no tick to compare".into())
}

/// Whether the world is still holding the client's player up.
///
/// # Errors
///
/// Returns an error where the client has published nothing.
fn held_up(client: &InputHarness) -> Result<bool, Box<dyn Error>> {
    Ok(resting(&published(client)?).1)
}

/// Refuses unless the walk moved the player off the spawn.
///
/// A player who never moved publishes the same position on every tick, and an
/// oracle advanced one tick too few would then agree with the run it is judging.
fn require_moved(stood: [u32; 3]) -> Result<(), Box<dyn Error>> {
    if stood != SPAWN.to_array().map(f32::to_bits) {
        return Ok(());
    }
    Err(NEVER_MOVED.into())
}

/// What a fixture that never walked its player is told.
const NEVER_MOVED: &str = "this fixture has to walk the player off the spawn before a swap is \
                           asked to leave them alone, and they are still standing on it. A \
                           comparison against a player who never moved cannot tell one run from \
                           another advanced a tick less";

/// Refuses unless the player is still in the air and still gathering speed.
///
/// A velocity of nothing is what a swap that zeroed it would produce, and the
/// difference the tick after the swap is exactly the velocity standing before it —
/// so a reading taken at rest could not report that mutation at all.
fn require_still_falling(client: &InputHarness, moving: [u32; 3]) -> Result<(), Box<dyn Error>> {
    if held_up(client)? {
        return Err(ALREADY_LANDED.into());
    }
    if moving == at_rest() {
        return Err(ALREADY_AT_REST.into());
    }
    Ok(())
}

/// What a fixture whose player had already landed is told.
const ALREADY_LANDED: &str = "this fixture has to leave the player in the air when the candidate \
                              arrives, and the floor is already holding them up";

/// What a fixture whose player was not moving is told.
const ALREADY_AT_REST: &str = "this fixture has to leave the player moving when the candidate \
                               arrives, and they are at rest — a swap that zeroed their velocity \
                               would leave this reading exactly as it is";

/// Refuses unless the candidate was admitted at all.
fn require_admitted(answered: &Adoption) -> Result<(), Box<dyn Error>> {
    if matches!(answered, Adoption::Accepted { .. }) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the candidate to be admitted, and the client answered {answered:?}. \
         The two runs would then differ in nothing and the comparison would be about neither"
    )
    .into())
}
