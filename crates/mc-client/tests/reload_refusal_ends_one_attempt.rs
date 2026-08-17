//! A refusal ends one attempt and not the watching, costs the author nothing they
//! had, and is said once however many saves meet it.
//!
//! # The three of these are the halves that stop a refusal path being a dead end
//!
//! A reload that refuses correctly and then never accepts anything again is worse
//! than one that never watched: the author fixes their file, saves, and nothing
//! happens. So one scenario here is the fix landing, one is what the refusal left
//! alone, and one is the terminal not filling up with the same sentence while they
//! work.
//!
//! # Nothing the author had is spent by a refusal, and "nothing" is three things
//!
//! The four blocks, the layer the next new texture key will take, and the block in
//! their hand. The candidate the refusal scenario hands over **would have changed
//! all three if it had been accepted** — it declares a new solid block whose file
//! sorts first, which is a fifth registration, a fifth texture key and a different
//! block in the hand — and it is refused for a *second* file beside it. So a
//! half-applied candidate is caught three times over rather than being invisible in
//! a refusal that reads correctly.
//!
//! # Deduplication compares the text a person reads, and the text is why
//!
//! A refusal is an error chain with no equality to compare, and the thing that
//! matters is what somebody reads on the terminal — so what is remembered is the
//! rendering, exactly as the recurring re-mesh fault beside it already does. Two
//! structurally different refusals that render identically are reported once, which
//! is accepted.
//!
//! **What that makes load-bearing is the depth of the rendering.** Every refusal
//! that comes out of a content root shares its outermost sentence, so a
//! deduplication comparing only that would report the first broken file of a session
//! and then go silent for the rest of it — the author fixes one fault, meets
//! another, and is told nothing. The second half of the last scenario is that case:
//! a *different* refusal, in the same file, after five of the first.

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
use reload::{AMBER, AMBER_FILE, DIRT, GRASS, STONE_FILE, amber, shipped};
use reload_content::{BERYL_FILE, THE_NEXT_UNUSED_LAYER, fresh_layers, layers_beside, publishing};
use reload_watch::{
    MISSPELLED_SOLID, NOT_A_CHUNK, Refusal, STONE_MISSPELLING_SOLID, a_client_on, block_path,
    declaration_named, ended, naming, refusal, refusal_said, restating_raw, serving,
    the_four_shipped_blocks, the_loaders_own_words, until_settled,
};
use support::TestResult;
use support::content::{BLOCK_DIRECTORY, ContentRoot};

/// How many successive saves meet one refusal before the count is read.
///
/// The scenario's own number. What matters is that it is more than one: a
/// deduplication that only ever compared against nothing reports this many.
const SAVES_MEETING_ONE_REFUSAL: usize = 5;

#[test]
fn a_refused_candidate_leaves_the_blocks_the_layers_and_the_block_in_the_hand_alone() -> TestResult
{
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = a_new_block_beside_a_broken_file(root)?;
    let words = the_loaders_own_words(root.path())?;

    reports.changed(&[block_path(&root, BERYL_FILE)])?;
    let crossed = until_settled(&mut client);
    let published = publishing(client.content())?;

    assert_eq!(
        (
            refusal(&crossed, &words, &naming(&[&declaration_named(BERYL_FILE)])),
            serving(&client)?,
            published.layers,
            published.spent,
            held(&client)
        ),
        (
            Refusal::NamedEverythingAsked,
            the_four_shipped_blocks(),
            fresh_layers()?,
            THE_NEXT_UNUSED_LAYER,
            Some(DIRT.to_owned())
        ),
        "the save that was refused also declared a new solid block sorting ahead of everything, so \
         a candidate applied in part would show up three ways: a fifth block registered, a fifth \
         texture key holding layer {THE_NEXT_UNUSED_LAYER}, and `{AMBER}` in the hand instead of \
         `{DIRT}`. None of them moves, and the layer count is the one that would leak the session's \
         budget away silently — a refused candidate that spent a layer takes it from every reload \
         that follows"
    );
    Ok(())
}

#[test]
fn a_corrected_declaration_is_taken_up_by_the_next_attempt_after_a_refusal() -> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = root.declaring_block(AMBER_FILE, NOT_A_CHUNK)?;

    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let refused = until_settled(&mut client);
    let root = corrected(root, AMBER_FILE, &amber().text())?;
    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let taken_up = until_settled(&mut client);

    assert_eq!(
        (
            refusal_said(&refused).is_some(),
            ended(&taken_up),
            publishing(client.content())?.layers,
            held(&client)
        ),
        (
            true,
            reload_watch::taken_up_once(),
            layers_beside(&[(AMBER, THE_NEXT_UNUSED_LAYER)])?,
            Some(AMBER.to_owned())
        ),
        "a refusal ends one attempt and not the watching, which is the whole of what makes fixing a \
         file a save rather than a relaunch. The layer the corrected candidate's new key takes is \
         asserted beside it because it is the same layer the refused one would have taken — a \
         refusal that had spent it would put this key on the one after, and nothing else in the \
         spec compares those two numbers"
    );
    Ok(())
}

#[test]
fn five_saves_meeting_one_refusal_are_reported_once_and_the_next_one_that_differs_is_reported()
-> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating_raw(root, STONE_FILE, STONE_MISSPELLING_SOLID)?;
    let words = the_loaders_own_words(root.path())?;
    let crossed = saving_again(&mut client, &reports, &root)?;

    let root = restating_raw(root, STONE_FILE, NOT_A_CHUNK)?;
    let differs = the_loaders_own_words(root.path())?;
    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let after = until_settled(&mut client);

    assert_eq!(
        (
            refusal(&crossed, &words, &naming(&[MISSPELLED_SOLID])),
            refusal(&after, &differs, &naming(&[&declaration_named(STONE_FILE)]))
        ),
        (Refusal::NamedEverythingAsked, Refusal::NamedEverythingAsked),
        "an author fixing a file saves it many times, and being told the same thing on every save \
         buries the one message that changed. So the same refusal is stated once — and the next one \
         that reads differently is stated, which is what makes the first half evidence rather than \
         silence. The second fault is in the *same file* as the first, so a deduplication comparing \
         only the sentence a content refusal opens with reports the first and swallows this one"
    );
    Ok(())
}

/// `root` with a well-formed new block declared beside a file that will not load.
///
/// The pair is the point: the candidate would register a fifth block, append a fifth
/// texture key and change the block in the hand, and it is refused for the *other*
/// file — so a candidate applied in part has three separate ways of showing up.
///
/// # Errors
///
/// Returns an error if the root already declares either file, or if a write fails.
fn a_new_block_beside_a_broken_file(root: ContentRoot) -> Result<ContentRoot, Box<dyn Error>> {
    root.declaring_block(AMBER_FILE, &amber().text())?
        .declaring_block(BERYL_FILE, NOT_A_CHUNK)
}

/// The block this client would place, as text.
fn held(client: &InputHarness) -> Option<String> {
    client.held_block().map(|block| block.as_str().to_owned())
}

/// `root` with the declaration in `file_name` corrected to text that loads.
///
/// The write is the author saving again; the file is already there, which is what
/// tells this apart from declaring one.
///
/// # Errors
///
/// Returns an error if the root does not hold the file, or if the write fails.
fn corrected(
    root: ContentRoot,
    file_name: &str,
    declaration: &str,
) -> Result<ContentRoot, Box<dyn Error>> {
    let declared = root.path().join(BLOCK_DIRECTORY).join(file_name);
    if !declared.is_file() {
        return Err(format!(
            "this fixture has to correct `{BLOCK_DIRECTORY}/{file_name}`, and the root does not \
             hold it — what it would build is a declaration nobody had got wrong"
        )
        .into());
    }
    fs::write(&declared, declaration)?;
    Ok(root)
}

/// Every boundary crossed over five saves of one broken file.
///
/// Each save is its own report and its own attempt; what the run is counting is how
/// many times the person was told.
///
/// # Errors
///
/// Returns an error if a report cannot be delivered.
fn saving_again(
    client: &mut InputHarness,
    reports: &reload_watch::Reports,
    root: &ContentRoot,
) -> Result<Vec<Option<reload_watch::Attempt>>, Box<dyn Error>> {
    let mut crossed = Vec::new();
    for _ in 0..SAVES_MEETING_ONE_REFUSAL {
        reports.changed(&[block_path(root, STONE_FILE)])?;
        crossed.extend(until_settled(client));
    }
    Ok(crossed)
}
