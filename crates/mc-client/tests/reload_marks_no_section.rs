//! Which sections a reload leaves to be meshed again when a candidate changes
//! something **no frame shows**: none of them, and the control that says so
//! means anything.
//!
//! # This file is one half of a pair, and the seam is the question asked
//!
//! Every reading here is about a field `changes_geometry` deliberately excludes —
//! what a click does, what a swing can find, what a volume does to a body moving
//! through it, and what it does to the light of everything seen from inside it.
//! The other half, `reload_marks_sections.rs`, is about the fields the rule is
//! keyed on. **The pair was one file until it outgrew the size a test file is
//! allowed**, and the split is by that question rather than by outcome.
//!
//! # Every reading here asserts an absence, so two controls travel with them
//!
//! "This reload marked nothing" is satisfied in full by an implementation that
//! meshes nothing on any reload, and by a harness whose drain has stopped
//! reporting. Neither could be caught by any number of further absence readings.
//! So both controls live in this file rather than beside the marking scenarios
//! they borrow their positive half from:
//!
//! - `a_reload_that_changes_no_geometry_and_one_that_does_are_told_apart_on_one_instrument`
//!   drives two candidates through **one** session and grades the discrimination
//!   itself;
//! - `the_same_harness_marks_every_section_for_a_candidate_that_only_stops_drawing_stone`
//!   is the same client, the same root and the same reading over the one field
//!   whose whole subject is the picture.
//!
//! **Moving either of them into the other half would leave every reading here
//! resting on a control in a different binary**, which is the one thing this
//! split was not allowed to cost.
//!
//! # One instrument, and it is the set the frame path drains
//!
//! Every reading here is `Session::take_remesh_work`, which is what the client's
//! own frame path asks and what the re-mesh worker is handed. **The set is taken,
//! so it is read exactly once per reload** and held in a value; a second ask
//! would find it empty, and a scenario reading that would call it "nothing was
//! marked".
//!
//! # Each field is asserted on its own, and that is not repetition
//!
//! A key that learned one excluded field and not the others passes exactly the
//! scenarios about the ones it did not learn. So `targetable`, `breakable`,
//! `swimmable`, `move_resistance`, `swim_ascent` and the medium's colour and
//! reach each have a reading of their own, and `reload.rs`'s own doc comment
//! records that a newly declared field owes one beside them — the tint had none
//! for a while, and the mutation that should have caught it moved nothing at all.
//!
//! # The player stands in open air, and nothing they do writes to the world
//!
//! The spawn is above the landmark pillar's top, so no tick these scenarios
//! advance can edit a cell and no mark can arrive from anywhere but the reload.
//! Each scenario also drains before it reloads. Both are
//! `support/marks_sections.rs`, shared with the other half so the two suites
//! cannot launch into different worlds.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/marks_sections.rs"]
mod marks_sections;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_remesh.rs"]
mod reload_remesh;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use marks_sections::{a_client_over_the_shipped_world, require_nothing_outstanding};
use reload::{
    DIRT, Declaration, STONE, accepted, adoption, candidate, shipped_restating_stone,
    stone_that_is_not_solid,
};
use reload_content::{Run, run_of, serial_reported};
use reload_remesh::{Marking, every_section_once, marked, serial_serving};
use support::TestResult;

/// The resistance the medium scenarios below give stone.
///
/// Any value above zero would do — these scenarios are about what a reload draws
/// again, not about what the number does to a walk — so it is a plain one stated
/// against stone's silence, and stone stays solid so that nothing here is also a
/// solidity edit.
const A_RESISTANCE_WORTH_DECLARING: f32 = 3.0;

/// The ascent the medium scenario below gives stone.
///
/// **Not `9.0`**, which is what the loader supplies to a declaration that says
/// nothing: a candidate stating the default differs from the serving root in
/// nothing at all, and would leave the reading below green against a marking rule
/// that had learned the field. Stone stays solid and stays silent about
/// swimmability, so nothing here is also a solidity or a buoyancy edit.
const AN_ASCENT_WORTH_DECLARING: f32 = 4.0;

/// The medium colour and reach the scenario below gives stone.
///
/// **A pair, because the loader takes the two together or not at all**, and a
/// candidate stating one alone is a refused root rather than an edit. Neither
/// value has a default the loader could supply, so a stone that said nothing
/// about a tint and one that declares this pair differ in exactly the field the
/// reading is about. Stone stays solid and stays silent about everything else,
/// so nothing here is also a solidity, a buoyancy or a drawing edit.
const A_TINT_WORTH_DECLARING: &str = "#3A6EA5";
const A_REACH_WORTH_DECLARING: f32 = 12.0;

#[test]
fn a_candidate_touching_neither_solidity_nor_a_texture_key_leaves_no_section_to_mesh() -> TestResult
{
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let unbreakable = shipped_restating_stone(&Declaration::of(STONE).breakable(false))?;

    let answered = adoption(client.adopt(candidate(unbreakable.path())?));
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (answered, left_to_mesh),
        (accepted(DIRT), Marking::NoSectionAtAll),
        "`breakable` decides what a click does and nothing about what is drawn, so a reload that \
         changes it alone has no picture to correct. **This is satisfied by an implementation that \
         never meshes anything on any reload**, which is why it is read on the same instrument as \
         the scenario below it: the discrimination is that one of the two marks nothing and the \
         other marks the world"
    );
    Ok(())
}

#[test]
fn a_reload_that_changes_no_geometry_and_one_that_does_are_told_apart_on_one_instrument()
-> TestResult {
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let unbreakable = shipped_restating_stone(&Declaration::of(STONE).breakable(false))?;
    let softened = shipped_restating_stone(&stone_that_is_not_solid())?;

    let first = adoption(client.adopt(candidate(unbreakable.path())?));
    let after_the_first = marked(&mut client);
    let second = adoption(client.adopt(candidate(softened.path())?));
    let after_the_second = marked(&mut client);

    assert_eq!(
        (first, after_the_first, second, after_the_second),
        (
            accepted(DIRT),
            Marking::NoSectionAtAll,
            accepted(DIRT),
            every_section_once()
        ),
        "two candidates, one session, one instrument. **Without this pairing, an implementation \
         that meshes nothing on any reload satisfies both the no-section scenario and the \
         exactly-256 one** — the first because it is right and the second because nothing would be \
         comparing them. What is graded here is the discrimination itself: `breakable` moves \
         nothing and declared solidity moves everything"
    );
    Ok(())
}

#[test]
fn a_candidate_taking_stones_targetability_away_publishes_a_later_serial_and_marks_no_section()
-> TestResult {
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let unaimable = shipped_restating_stone(&Declaration::of(STONE).targetable(false))?;
    let launched = serial_serving(&client)?.get();

    let answered = client.adopt(candidate(unaimable.path())?);
    let published = serial_reported(&answered);
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (
            adoption(answered),
            run_of(launched, &[published]),
            left_to_mesh
        ),
        (
            accepted(DIRT),
            Run::EachLaterThanTheLast,
            Marking::NoSectionAtAll
        ),
        "what a swing can find changes not one pixel, so a reload that moves it alone has nothing \
         to draw again — and an implementation folding all five declaration properties into one \
         geometry key passes both scenarios above and fails only here. **The zero is asserted \
         beside the acceptance and the serial because a refused reload marks nothing either**, \
         and so does one that published no content at all: on its own, `NoSectionAtAll` is \
         satisfied by a reload that never happened"
    );
    Ok(())
}

#[test]
fn a_candidate_that_only_makes_stone_something_to_swim_in_marks_no_section() -> TestResult {
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let swimmable = shipped_restating_stone(&Declaration::of(STONE).swimmable(true))?;
    let launched = serial_serving(&client)?.get();

    let answered = client.adopt(candidate(swimmable.path())?);
    let published = serial_reported(&answered);
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (
            adoption(answered),
            run_of(launched, &[published]),
            left_to_mesh
        ),
        (
            accepted(DIRT),
            Run::EachLaterThanTheLast,
            Marking::NoSectionAtAll
        ),
        "whether a player can hold itself up in a block's volume decides what happens when they \
         walk into it and changes not one pixel of it, so there is nothing to draw again. **The \
         zero is asserted beside the acceptance and the serial because a refused reload marks \
         nothing either**, and so does one that published no content at all: on its own, \
         `NoSectionAtAll` is satisfied by a reload that never happened. The scenario below it on \
         this same instrument is what says the harness can still mark the world"
    );
    Ok(())
}

#[test]
fn a_candidate_that_only_makes_stone_slow_what_moves_through_it_marks_no_section() -> TestResult {
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let resistant = shipped_restating_stone(
        &Declaration::of(STONE).move_resistance(A_RESISTANCE_WORTH_DECLARING),
    )?;
    let launched = serial_serving(&client)?.get();

    let answered = client.adopt(candidate(resistant.path())?);
    let published = serial_reported(&answered);
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (
            adoption(answered),
            run_of(launched, &[published]),
            left_to_mesh
        ),
        (
            accepted(DIRT),
            Run::EachLaterThanTheLast,
            Marking::NoSectionAtAll
        ),
        "how much a volume slows what moves through it is a number the physics divides by, and a \
         still frame cannot show it — so an implementation that added either medium field to the \
         geometry key rebuilds all 256 sections for an edit that changes no picture, which is what \
         this reports. It is asserted separately from the buoyancy scenario above because a key \
         that learned one of the two and not the other passes exactly one of them"
    );
    Ok(())
}

#[test]
fn a_candidate_that_only_changes_how_fast_a_block_carries_a_swimmer_marks_no_section() -> TestResult
{
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let lifting =
        shipped_restating_stone(&Declaration::of(STONE).swim_ascent(AN_ASCENT_WORTH_DECLARING))?;
    let launched = serial_serving(&client)?.get();

    let answered = client.adopt(candidate(lifting.path())?);
    let published = serial_reported(&answered);
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (
            adoption(answered),
            run_of(launched, &[published]),
            left_to_mesh
        ),
        (
            accepted(DIRT),
            Run::EachLaterThanTheLast,
            Marking::NoSectionAtAll
        ),
        "how fast a volume carries a swimmer upward is a number the physics launches at, and a \
         still frame cannot show it — so an implementation that added the ascent to the geometry \
         key rebuilds all 256 sections every time an author retunes a rate nobody can see, and the \
         author reads the flicker as the reload having found something. It is asserted separately \
         from the two medium scenarios above because a key that learned one medium field and not \
         another passes exactly the scenarios about the ones it did not learn. **The serial and \
         the acceptance are asserted beside the zero because a refused reload marks nothing \
         either**, and so does one that published no content at all"
    );
    Ok(())
}

#[test]
fn a_candidate_that_only_changes_what_a_volume_does_to_the_light_marks_no_section() -> TestResult {
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let tinting = shipped_restating_stone(
        &Declaration::of(STONE).tint(A_TINT_WORTH_DECLARING, A_REACH_WORTH_DECLARING),
    )?;
    let launched = serial_serving(&client)?.get();

    let answered = client.adopt(candidate(tinting.path())?);
    let published = serial_reported(&answered);
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (
            adoption(answered),
            run_of(launched, &[published]),
            left_to_mesh
        ),
        (
            accepted(DIRT),
            Run::EachLaterThanTheLast,
            Marking::NoSectionAtAll
        ),
        "what a volume does to the light of everything seen from *inside* it is carried per frame \
         by the eye's own cell and by no face of any block, so no quad the mesher emits changes \
         and there is nothing to build again. An implementation that added either medium field to \
         the geometry key rebuilds all 256 sections every time an author retunes a colour or a \
         reach — invisible in every frame, and paid on every reload, which is the shape that \
         reaches a release rather than a review. Asserted separately from the three medium \
         scenarios above because a key that learned one of the fields and not the others passes \
         exactly the scenarios about the ones it did not learn"
    );
    Ok(())
}

#[test]
fn the_same_harness_marks_every_section_for_a_candidate_that_only_stops_drawing_stone() -> TestResult
{
    let mut client = a_client_over_the_shipped_world()?;
    require_nothing_outstanding(&mut client)?;
    let invisible = shipped_restating_stone(&Declaration::of(STONE).drawn(false))?;

    let answered = client.adopt(candidate(invisible.path())?);
    let left_to_mesh = marked(&mut client);

    assert_eq!(
        (adoption(answered), left_to_mesh),
        (accepted(DIRT), every_section_once()),
        "the control every reading in this file cannot supply for itself. Each of them asserts an \
         absence, and a reload path that came to mark nothing at all — or a harness whose drain \
         stopped reporting — satisfies all of them forever. This is the same client, the same \
         root and the same reading over the one field whose whole subject is the picture, so the \
         discrimination is which of two answers the marking gives rather than whether it can give \
         one. **It counted three medium scenarios and now guards four**: the tint joined them, \
         which is why this says what it guards rather than how many"
    );
    Ok(())
}
