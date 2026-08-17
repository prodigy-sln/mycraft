//! A candidate takes effect between two ticks, and never during one.
//!
//! # A break either side of the swap is the sharpest instrument this has
//!
//! A tick that was already resolved cannot be un-resolved by content arriving
//! afterwards, and a tick that has not run yet has to run under the content now
//! serving. A swap taken part way through a tick makes one of those two wrong,
//! and the two are told apart here by asking for the same thing twice: a break of
//! stone before the candidate lands, which succeeds, and one after it, which the
//! candidate has made impossible.
//!
//! **The two breaks are aimed at different cells and they have to be.** A break
//! empties the cell the ray stopped at, so a second break down the same line
//! finds nothing there and would be refused for having no target — a refusal that
//! says nothing at all about what the candidate declared. The second aim is
//! steeper and meets a cell the first one never touched.
//!
//! # The tick counter is read as a difference and never as a value
//!
//! What tick a run happens to be at when a candidate arrives is the fixture's
//! business. What the scenario is about is that the swap published nothing of its
//! own and that the tick after it is the tick before it plus one — so both are
//! read as offsets from the tick that was current when the candidate was handed
//! over.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::path::Path;

use winit::event::MouseButton;

use input::InputHarness;
use reload::{
    Adoption, Declaration, STONE, STONE_FILE, adoption, candidate, restating, shipped,
    stone_that_is_not_solid,
};
use reload_world::{
    AIM_AT_THE_FAR_CELL, AIM_ON_TO_THE_NEAR_CELL, Edit, THE_FAR_CELL, edit, floor_of, playing,
    published_tick, resting, standing,
};
use support::{TestResult, content_root};

/// How many ticks a run advances before a candidate is handed over, so that the
/// tick the swap is measured against is not the first one.
const SETTLED_AFTER: u32 = 3;

/// What the tick counter is expected to have done by the time each reading is
/// taken: nothing at the swap itself, and one step at the tick after it.
const NOTHING_YET: i64 = 0;
const ONE_STEP: i64 = 1;

#[test]
fn a_break_asked_for_before_the_swap_succeeds_and_one_asked_for_after_it_is_refused() -> TestResult
{
    let mut client = a_client_playing(&content_root()?)?;
    client.move_pointer(0.0, AIM_AT_THE_FAR_CELL);
    client.click(MouseButton::Left);
    let earlier = edit(client.edit());

    let root = restating(
        shipped()?,
        STONE_FILE,
        &Declaration::of(STONE).breakable(false),
    )?;
    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;

    client.move_pointer(0.0, AIM_ON_TO_THE_NEAR_CELL);
    client.click(MouseButton::Left);
    let later = edit(client.edit());

    assert_eq!(
        (earlier, later),
        (Edit::Emptied(THE_FAR_CELL), Edit::Indestructible),
        "the earlier tick was resolved under the content that was serving and its answer is final; \
         the later one runs under the content the candidate brought. A swap taken part way through \
         a tick makes one of those two wrong — either the player's dig is undone by a file they \
         saved after asking for it, or an edit they made a moment ago is still being answered \
         under content nobody is serving any more"
    );
    Ok(())
}

#[test]
fn the_tick_after_an_accepted_candidate_is_the_tick_before_it_plus_one() -> TestResult {
    let mut client = a_client_playing(&content_root()?)?;
    client.ticks(SETTLED_AFTER);
    let before = tick_of(&client)?;

    let root = restating(shipped()?, STONE_FILE, &stone_that_is_not_solid())?;
    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    let at_the_swap = tick_of(&client)?;

    client.ticks(1);
    let after = tick_of(&client)?;

    assert_eq!(
        (
            i64::from(at_the_swap) - i64::from(before),
            i64::from(after) - i64::from(before),
            still_held_up(&client)?
        ),
        (NOTHING_YET, ONE_STEP, false),
        "a swap is something that happens between two ticks, so it publishes no tick of its own \
         and the next one carries on from where the last left off. The candidate has taken stone's \
         solidity away, and that is in force on the later tick and not before it — which is what \
         makes the counter's step a step rather than a tick the run skipped or ran twice"
    );
    Ok(())
}

/// A client playing a floor of stone, with the content root at `root` serving.
fn a_client_playing(root: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, standing(), |registry| floor_of(registry, STONE))?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// Which tick the client has published.
///
/// # Errors
///
/// Returns an error where the client has published none, which is a client with
/// no world rather than one at a tick.
fn tick_of(client: &InputHarness) -> Result<u32, Box<dyn Error>> {
    let published = client
        .published()
        .ok_or("this fixture's client has published no tick to count from")?;
    Ok(published_tick(&published))
}

/// Whether the world is still holding the player up.
///
/// # Errors
///
/// Returns an error where the client has published no tick.
fn still_held_up(client: &InputHarness) -> Result<bool, Box<dyn Error>> {
    let published = client
        .published()
        .ok_or("this fixture's client has published no tick to stand in")?;
    Ok(resting(&published).1)
}

/// Refuses unless the candidate was admitted at all.
fn require_admitted(answered: &Adoption) -> Result<(), Box<dyn Error>> {
    if matches!(answered, Adoption::Accepted { .. }) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the candidate to be admitted, and the client answered {answered:?}. \
         Nothing would then have crossed a tick boundary for the comparison to be about"
    )
    .into())
}
