//! A root that is good for one thing and bad for another is a root that failed,
//! and a declaration that misbehaves is contained by the limits the host ships.
//!
//! # The HUD is refused with the blocks, and that is the Blocker-class half
//!
//! One call reads a content root and it reads both halves of it, so a candidate
//! whose HUD declaration is refused is a candidate that is refused — blocks
//! included. Applying the blocks and leaving the HUD behind *is* the partial
//! application the scripting host's invariants call a Blocker, and it is invisible
//! in the refusal itself: what says the blocks were not applied is that the edit
//! made beside the broken HUD did not land. So that scenario edits a block
//! declaration **and** breaks a HUD declaration in one save, and the block's
//! solidity is the other half of the assertion.
//!
//! # The three limits are the host's own, and each fixture's shape is a constraint
//! no assertion here enforces
//!
//! A declaration that loops, one that allocates past the per-entry cap and one that
//! raises an error of its own are stopped by the call-and-loop budget, the memory cap
//! and the fault path the scripting host already ships. **Which limit each fixture
//! actually trips is held by the code that builds it and by
//! `crates/mc-world/tests/luau_declaration_guard.rs`**, which asserts for each of
//! these chunks that the limit it names is the one that fired and that the other one
//! is not — a memory bomb under a small enough budget dies of ticks and reports the
//! wrong limit while passing every count written against it. Nothing in this file
//! can see that, and it says so rather than implying otherwise.
//!
//! What these three add over the launch-time suite is the reload: the process is
//! still running, the previous content is still serving, and the ticks go on being
//! advanced while a declaration was busy trying to hang the run.
//!
//! # The loop is the one that would have taken the server with it
//!
//! A build that ran round the side of the host does not fail the looping scenario —
//! it never finishes it, and takes the run with it. That is why the assertion beside
//! the refusal is the tick count: the simulation went on advancing while a
//! declaration spent a million loop edges on a worker.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::fs;

use input::InputHarness;
use reload::{
    AMBER, AMBER_FILE, Declaration, GRASS, STONE, STONE_FILE, restating, shipped,
    stone_that_is_not_solid,
};
use reload_content::CROSSBAR_FILE;
use reload_watch::{
    Refusal, a_client_on, block_path, declaration_named, hud_path, naming, refusal, serving,
    solidity_of, the_four_shipped_blocks, the_loaders_own_words, until_settled,
};
use reload_world::published_tick;
use support::TestResult;
use support::content::{ContentRoot, HUD_DIRECTORY};

/// A crossbar declaration stating an extent with nothing in it.
///
/// Every field is the shipped one; only `size` differs, and an element nought wide
/// is a rectangle covering no pixel — which the model refuses rather than drawing
/// nothing.
const A_CROSSBAR_OF_NO_WIDTH: &str = "name = \"base:crosshair-horizontal\"\nanchor = \"center\"\nsize = [0, 1]\ndraw = \
     \"fill\"\ncolor = \"#FFFFFFFF\"\noutline = \"#000000FF\"\n";

/// A declaration whose top level never returns.
///
/// The table below it would register perfectly well; the loop in front of it is the
/// only thing wrong with it, which is what makes the budget the thing being asserted
/// rather than the declaration.
const A_LOOP_THAT_NEVER_RETURNS: &str = "while true do end\n";

/// A declaration that allocates far past anything one entry may hold.
///
/// Each appended string carries the loop index so every one of them is distinct and
/// therefore separately allocated: the backend interns strings, and without the
/// index a thousand appends are a thousand references to one string that no cap can
/// stop. A thousand appends of 4 KiB is about 4 MiB against the 256 KiB an entry may
/// add.
const AN_ALLOCATION_PAST_THE_CAP: &str = "local held = {}\n\
     for index = 1, 1024 do held[index] = string.rep('x', 4096) .. index end\n";

/// What a declaration that refuses to load says about itself.
///
/// This fixture's own words, so the needle below is something an author wrote rather
/// than something the engine produced.
const RAISED_BY_THE_DECLARATION: &str = "this declaration is not ready to be loaded";

#[test]
fn a_refused_hud_declaration_refuses_the_block_declarations_beside_it() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating(root, STONE_FILE, &stone_that_is_not_solid())?;
    let root = breaking_the_crossbar(root)?;
    let words = the_loaders_own_words(root.path())?;

    reports.changed(&[hud_path(&root, CROSSBAR_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (
            refusal(&crossed, &words, &naming(&[CROSSBAR_FILE])),
            solidity_of(&client, STONE)?
        ),
        (Refusal::NamedEverythingAsked, Some(true)),
        "the same save carries a block edit and a broken HUD element, and the whole root is what \
         failed: stone is still solid because the candidate was refused entire. A reload that \
         applied the blocks and left the HUD behind reads as a success to everybody looking at the \
         blocks, and it is the partial application invariant 7 calls a Blocker — this is the only \
         instrument that would see it"
    );
    Ok(())
}

#[test]
fn a_declaration_that_loops_is_refused_naming_its_file_while_the_simulation_advances() -> TestResult
{
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = root.declaring_block(AMBER_FILE, &after(A_LOOP_THAT_NEVER_RETURNS))?;
    let words = the_loaders_own_words(root.path())?;
    let before = advanced(&client)?;

    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let crossed = until_settled(&mut client);
    let boundaries = u32::try_from(crossed.len())?;

    assert_eq!(
        (
            refusal(&crossed, &words, &naming(&[&declaration_named(AMBER_FILE)])),
            advanced(&client)? - before == boundaries,
            serving(&client)?
        ),
        (
            Refusal::NamedEverythingAsked,
            true,
            the_four_shipped_blocks()
        ),
        "a declaration that never returns is aborted at the call-and-loop budget the engine ships, \
         on a worker, and the run does not notice: the ticks advanced across the attempt are the \
         ticks the run crossed. A build that evaluated round the side of the host does not fail \
         this test — it never finishes it, and takes the game with it, which is the failure this \
         whole limit exists to keep off a server"
    );
    Ok(())
}

#[test]
fn a_declaration_that_allocates_past_the_memory_cap_is_refused_naming_its_file() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = root.declaring_block(AMBER_FILE, &after(AN_ALLOCATION_PAST_THE_CAP))?;
    let words = the_loaders_own_words(root.path())?;

    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (
            refusal(&crossed, &words, &naming(&[&declaration_named(AMBER_FILE)])),
            serving(&client)?
        ),
        (Refusal::NamedEverythingAsked, the_four_shipped_blocks()),
        "a declaration allocating past what one entry may add is stopped by the cap, the process \
         is still running and the content the author had is still serving. **Which limit this \
         fixture trips is not asserted here** — under a small enough budget the same bomb dies of \
         ticks and reports the wrong one — and `mc-world`'s own guard is what pins it, by naming \
         the cap and its byte count and denying the budget"
    );
    Ok(())
}

#[test]
fn a_declaration_that_raises_is_refused_naming_its_file_and_what_it_raised() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = root.declaring_block(AMBER_FILE, &a_declaration_that_raises())?;
    let words = the_loaders_own_words(root.path())?;

    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (
            refusal(
                &crossed,
                &words,
                &naming(&[&declaration_named(AMBER_FILE), RAISED_BY_THE_DECLARATION])
            ),
            serving(&client)?
        ),
        (Refusal::NamedEverythingAsked, the_four_shipped_blocks()),
        "a declaration may refuse itself, and what it said is the whole of why: the sentence is \
         this fixture's own, written into the chunk, so a refusal that carried only 'the \
         declaration failed' fails here. An author who reads their own words back knows they \
         reached the loader at all"
    );
    Ok(())
}

/// `chunk` in front of a declaration that would register perfectly well.
///
/// So the thing wrong with the file is the chunk and nothing else — a fixture whose
/// table was also broken could be refused for either.
fn after(chunk: &str) -> String {
    format!("{chunk}{}", Declaration::of(AMBER).text())
}

/// A declaration that raises an error of its own before it declares anything.
fn a_declaration_that_raises() -> String {
    after(&format!("error('{RAISED_BY_THE_DECLARATION}')\n"))
}

/// `root` with the crossbar declaration restated as an element of no width.
///
/// # Errors
///
/// Returns an error if the root does not declare the crossbar, or if the write
/// fails.
fn breaking_the_crossbar(root: ContentRoot) -> Result<ContentRoot, Box<dyn Error>> {
    let declared = root.path().join(HUD_DIRECTORY).join(CROSSBAR_FILE);
    if !declared.is_file() {
        return Err(format!(
            "this fixture has to break `{HUD_DIRECTORY}/{CROSSBAR_FILE}` in the root the client is \
             playing, and that root does not declare it. A root that never had a crosshair is not \
             a root whose crosshair an author broke"
        )
        .into());
    }
    fs::write(&declared, A_CROSSBAR_OF_NO_WIDTH)?;
    Ok(root)
}

/// How many ticks this client has published.
///
/// # Errors
///
/// Returns an error where it has published nothing.
fn advanced(client: &InputHarness) -> Result<u32, Box<dyn Error>> {
    let published = client
        .published()
        .ok_or("this fixture's client has published no tick to count")?;
    Ok(published_tick(&published))
}
