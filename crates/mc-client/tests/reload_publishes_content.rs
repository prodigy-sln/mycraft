//! What an accepted reload publishes, what a refused one leaves alone, and how a
//! reader tells one from the other.
//!
//! # A publisher whose serial never moves is what these scenarios exist to catch
//!
//! Three of the readings below are satisfied by a client that publishes nothing at
//! all — "the serial a reader last observed is the serial it still observes" and
//! "a refused candidate leaves the content exactly as it was" are both true of a
//! channel nobody ever writes to. That is the shape this project has already paid
//! for once: an observation taken through a channel the operation does not update
//! cannot see the operation.
//!
//! So each of them carries its discriminating half **in the same run**. The
//! scenario about a reader that has not looked goes on to accept a candidate and
//! require the serial to move; the scenario about a refusal goes on to correct the
//! mistake and require the same; and no expectation anywhere states a serial's
//! absolute value, so none of them can be satisfied by a counter that was already
//! sitting on the number the fixture happened to write down.
//!
//! # Nothing here states which number a launch publishes under
//!
//! Every claim is a relation — moved, distinct, unchanged — and `Run` is the
//! verdict those relations come to. A change to where the counter starts moves no
//! expectation in this file.
//!
//! # The HUD travels with the blocks because it is refused with them
//!
//! One content root is read for both, all or nothing, so applying the blocks while
//! leaving the HUD behind is the partial application the scripting host's
//! invariants call a Blocker. The reading here is the value where it crosses the
//! boundary: the widened element is in the layout the client is publishing. That
//! it reaches a *drawn* frame is a separate instrument
//! (`reload_hud_reaches_the_frame.rs`) and neither covers the other's half.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::sync::Arc;

use mc_sim::simulation::PublishedContent;

use input::InputHarness;
use reload::{
    AMBER, AMBER_FILE, DIRT, Declaration, GRASS, GRASS_FILE, STONE, WATER, accepted, adoption,
    amber, declaring, holding_blocks_it_does_not_declare, shipped,
};
use reload_content::{
    BERYL, BERYL_FILE, NOTHING_IS_SERVING, Publishing, Run, SHIPPED_CROSSBAR_EXTENT,
    WIDENED_CROSSBAR_EXTENT, candidate_against, crossbar_extent, fresh_layers, publishing, run_of,
    serial_reported, shipped_widening_the_crossbar,
};
use reload_world::{floor_of, playing, standing};
use support::{TestResult, content_root};

/// How many ticks a scenario advances to show that advancing alone publishes no
/// content. Long enough that a client republishing per tick would have moved.
const A_WHILE: u32 = 30;

/// Every block the root a reload declares states, in registration order — which is
/// file-name order — and whether each is solid.
///
/// Listed rather than read back out of the published content, for the reason the
/// texture keys are listed: a fixture that discovered them would go on passing over
/// content that had stopped declaring one. Each declares `texture` equal to `name`,
/// `base:water` is the shipped root's one non-solid block, and `base:amber`'s file
/// sorts ahead of every other — which is what puts it first.
const DECLARED_WITH_AMBER: [(&str, bool); 5] = [
    (AMBER, true),
    (DIRT, true),
    (GRASS, true),
    (STONE, true),
    (WATER, false),
];

/// Everything a reader can see of what a client publishes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Published {
    at: Publishing,
    blocks: Vec<(String, bool)>,
}

/// How the serial one reload reported stands against the serial the client then
/// publishes, and against the serial it published before.
///
/// **A total verdict over relations, so nothing states a number.** Each way of
/// failing has an arm that names it, which is what makes an assertion against the
/// good arm reject a publisher that reported a serial and stored another, and one
/// whose counter never moved at all.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Together {
    /// The reload reported a serial, the client publishes that same serial, and it
    /// is later than the one standing before.
    PublishedUnderTheSerialItReported,
    /// The reload reported one serial and the client publishes another.
    Disagreed { reported: Option<u32>, serving: u32 },
    /// Nothing moved: what is published is what was published before.
    NothingMoved(u32),
}

#[test]
fn an_accepted_candidate_publishes_the_content_a_reader_draws_with_under_a_serial_of_its_own()
-> TestResult {
    let mut client = a_client_over(STONE)?;
    let before = published_state(&client)?;
    let with_amber = declaring(shipped()?, AMBER_FILE, &amber())?;
    let candidate = candidate_against(&with_amber, client.content())?;

    let answered = client.adopt(candidate);
    let reported = serial_reported(&answered);
    let verdict = adoption(answered);
    let after = published_state(&client)?;
    let stands = together(&before, reported, &after);

    assert_eq!(
        (verdict, after.blocks, stands),
        (
            accepted(AMBER),
            declared(&DECLARED_WITH_AMBER),
            Together::PublishedUnderTheSerialItReported
        ),
        "a reader draws from the content the simulation publishes, so a swap that changed the \
         registry and published nothing leaves the picture drawn from content that is no longer \
         serving — with no error anywhere. What crosses the boundary has to be the candidate's own \
         blocks *and* the serial the acceptance reported, or a reader cannot tell which content set \
         it is holding"
    );
    Ok(())
}

#[test]
fn a_reader_that_has_not_looked_goes_on_seeing_the_content_serial_it_last_observed() -> TestResult {
    let mut client = a_client_over(STONE)?;
    let observed = serving(&client)?;
    let when_it_looked = publishing(Some(Arc::clone(&observed)))?.serial;

    client.ticks(A_WHILE);
    let after_ticks = publishing(client.content())?.serial;
    let with_amber = declaring(shipped()?, AMBER_FILE, &amber())?;
    let candidate = candidate_against(&with_amber, client.content())?;
    let verdict = adoption(client.adopt(candidate));
    let asked_again = publishing(client.content())?.serial;
    let still_held = publishing(Some(observed))?.serial;

    assert_eq!(
        (
            verdict,
            after_ticks,
            still_held,
            run_of(when_it_looked, &[Some(asked_again)])
        ),
        (
            accepted(AMBER),
            when_it_looked,
            when_it_looked,
            Run::EachLaterThanTheLast
        ),
        "content is observed by asking, not by being told, so thirty ticks with nothing accepted \
         publish nothing and the reader that already looked keeps exactly what it was handed. The \
         last element is what stops this passing over a client that publishes nothing ever: one \
         accepted candidate has to move what a fresh ask returns, while leaving the value the \
         earlier reader is still holding alone"
    );
    Ok(())
}

#[test]
fn two_accepted_candidates_are_published_under_two_serials_a_reader_can_tell_apart() -> TestResult {
    let mut client = a_client_over(STONE)?;
    let launch = published_state(&client)?.at.serial;
    let with_amber = declaring(shipped()?, AMBER_FILE, &amber())?;
    let one_key = candidate_against(&with_amber, client.content())?;
    let first = client.adopt(one_key);

    let with_both = declaring(shipped()?, AMBER_FILE, &amber())?
        .declaring_block(BERYL_FILE, &Declaration::of(BERYL).text())?;
    let two_keys = candidate_against(&with_both, client.content())?;
    let second = client.adopt(two_keys);
    let serials = [serial_reported(&first), serial_reported(&second)];

    assert_eq!(
        (adoption(first), adoption(second), run_of(launch, &serials)),
        (accepted(AMBER), accepted(AMBER), Run::EachLaterThanTheLast),
        "two saves are two reloads, and a reader holding a serial has to be able to say which of \
         them it is holding. A counter that moved once and then stuck, or one that reported the \
         same number twice, leaves a re-mesh worker unable to tell a batch meshed against the \
         content now serving from one meshed against the content before it — which is a stale \
         picture with nothing reporting it"
    );
    Ok(())
}

#[test]
fn a_refused_candidate_leaves_the_published_content_and_its_serial_exactly_as_they_were()
-> TestResult {
    let mut client = a_client_over(GRASS)?;
    let before = published_state(&client)?;
    let dropping_a_held_block = shipped()?.not_declaring_blocks(&[GRASS_FILE])?;
    let turned_away = candidate_against(&dropping_a_held_block, client.content())?;

    let refused = adoption(client.adopt(turned_away));
    let after_the_refusal = published_state(&client)?;
    let corrected = candidate_against(&shipped()?, client.content())?;
    let took_it_up = adoption(client.adopt(corrected));
    let after_the_acceptance = published_state(&client)?;

    assert_eq!(
        (
            refused,
            after_the_refusal,
            took_it_up,
            run_of(before.at.serial, &[Some(after_the_acceptance.at.serial)])
        ),
        (
            holding_blocks_it_does_not_declare(&[GRASS]),
            before,
            accepted(DIRT),
            Run::EachLaterThanTheLast
        ),
        "no part of a refused candidate may reach a reader: not a block, not a layer, and not a \
         serial — a serial that moved for a refusal tells a re-mesh worker its batch is stale and \
         makes it mesh the whole world again for nothing. The corrected save at the end is what \
         stops this passing over a client that publishes nothing at all, which would leave the \
         content 'exactly as it was' forever"
    );
    Ok(())
}

#[test]
fn a_candidate_identical_to_the_content_serving_leaves_the_layers_and_publishes_a_later_serial()
-> TestResult {
    let mut client = a_client_over(STONE)?;
    let before = published_state(&client)?;
    let identical = candidate_against(&shipped()?, client.content())?;

    let verdict = adoption(client.adopt(identical));
    let after = published_state(&client)?;

    assert_eq!(
        (
            verdict,
            after.at.layers.clone(),
            run_of(before.at.serial, &[Some(after.at.serial)])
        ),
        (accepted(DIRT), fresh_layers()?, Run::EachLaterThanTheLast),
        "the author saved a file and changed nothing in it, which is an ordinary thing to do and \
         costs the session no layer: the four keys stay exactly where they were. It still publishes, \
         and under a later serial — without that half the scenario is satisfied by an \
         implementation that noticed the content was identical and skipped the whole attempt, and \
         then nothing here would be about a reload at all"
    );
    Ok(())
}

#[test]
fn a_widened_crosshair_declaration_reaches_the_published_layout_at_its_declared_extent()
-> TestResult {
    let mut client = a_client_over(STONE)?;
    let before = crossbar_of(&client)?;
    let widened = shipped_widening_the_crossbar()?;
    let candidate = candidate_against(&widened, client.content())?;

    let verdict = adoption(client.adopt(candidate));
    let after = crossbar_of(&client)?;

    assert_eq!(
        (verdict, before, after),
        (
            accepted(DIRT),
            Some(SHIPPED_CROSSBAR_EXTENT),
            Some(WIDENED_CROSSBAR_EXTENT)
        ),
        "one content root is read for the blocks and the HUD together and refused for both at once, \
         so a reload that applied the blocks and left the HUD behind is exactly the partial \
         application that is forbidden. The extent standing before the reload is in the same \
         comparison, so a client publishing no layout at all fails here rather than reading as a \
         client whose HUD happened not to change"
    );
    Ok(())
}

/// A client playing a one-column floor of `floor`, with the shipped content root
/// serving.
fn a_client_over(floor: &'static str) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(&content_root()?, standing(), |registry| {
        floor_of(registry, floor)
    })?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// The content `client` is publishing.
///
/// # Errors
///
/// Returns an error where the client publishes none, which is a client with no
/// world rather than one a reader could be handed anything from.
fn serving(client: &InputHarness) -> Result<Arc<PublishedContent>, Box<dyn Error>> {
    client.content().ok_or_else(|| NOTHING_IS_SERVING.into())
}

/// The extent the layout `client` is publishing states for the crossbar.
///
/// # Errors
///
/// Returns an error where the client publishes nothing, which is a client with no
/// HUD rather than one whose crossbar states an extent.
fn crossbar_of(client: &InputHarness) -> Result<Option<[u32; 2]>, Box<dyn Error>> {
    let published = serving(client)?;
    Ok(crossbar_extent(&published.hud))
}

/// Everything a reader can see of what `client` publishes.
///
/// # Errors
///
/// Returns an error where the client publishes nothing.
fn published_state(client: &InputHarness) -> Result<Published, Box<dyn Error>> {
    let published = serving(client)?;
    Ok(Published {
        at: publishing(Some(Arc::clone(&published)))?,
        blocks: published
            .resolved
            .blocks()
            .map(|block| (block.name.as_str().to_owned(), block.is_solid))
            .collect(),
    })
}

/// The blocks a declared table states, as a published content set spells them.
fn declared(table: &[(&str, bool)]) -> Vec<(String, bool)> {
    table
        .iter()
        .map(|(name, is_solid)| ((*name).to_owned(), *is_solid))
        .collect()
}

/// How the serial `reported` stands against what `after` publishes and what
/// `before` published.
fn together(before: &Published, reported: Option<u32>, after: &Published) -> Together {
    if reported != Some(after.at.serial) {
        return Together::Disagreed {
            reported,
            serving: after.at.serial,
        };
    }
    if after.at.serial == before.at.serial {
        return Together::NothingMoved(after.at.serial);
    }
    Together::PublishedUnderTheSerialItReported
}
