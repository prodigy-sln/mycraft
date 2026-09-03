//! The four sentences a clearing verdict becomes, asserted with no device in
//! reach.
//!
//! `entering` and `reloading` are total functions of a `Copy` enum: no window, no
//! surface, no `App`, no session. That is the whole point of them living here.
//! The reload's two sentences have been composed since PRO-918 inside
//! `App::report_clearing` (`src/app/reload.rs:96`), which sits behind a
//! `wgpu::Surface` and a `winit::Window` that **nothing in this workspace
//! constructs** — so until this file existed, the exact words a player reads when
//! a reload moved them were asserted by nothing at all. Repeating that shape for
//! the entry sentences would have given FR-2's whole promise no observable.
//!
//! # The reload's two are transcribed from the shipped print, not from memory
//!
//! Read out of `crates/mc-client/src/app/reload.rs:96-115` on 2026-08-18, before
//! anything in phase 2 edited it. Both are written there across a `\`
//! continuation, which strips the newline *and* the following line's indentation,
//! so the shipped text is one line:
//!
//! ```text
//! "mycraft: the reload made your cell solid, so you were moved to \
//!  ({x}, {y}, {z})"
//! ```
//!
//! **"Character-identical" is a claim about that text and this file is the only
//! place it is checked.** A reload sentence reworded while these tests are edited
//! to match has lost the thing they are here for; the words on a player's
//! terminal are the artefact, and moving both sides together proves nothing.
//!
//! # `Display` on `f32` is what the expected coordinates were written against
//!
//! `10.0_f32` renders as `10`, and every expected string below depends on it.
//! That formatting is **inherited** from `app/reload.rs:100-106` rather than
//! chosen here, and it is why **no width or precision specifier may be added**:
//! `{:.1}` renders `10.0`, which reddens FR-2.1-S1 against an otherwise correct
//! implementation and gives no hint why. `an_entry_sentence_renders_a_whole_number…`
//! below is the hint.
//!
//! # The reach in a refusal is interpolated, never a literal
//!
//! `Clearing::NoClearSpaceWithin` carries `blocks`
//! (`crates/mc-sim/src/world/clearing.rs:53`), so the `8` in FR-2.1-S3's expected
//! sentence is that field arriving in the text. A hardcoded `8` satisfies that
//! scenario and is caught by `both_refusals_name_the_reach…`, which asks with a
//! reach of 3.
//!
//! # The stand-in readings, and why the empty answer is the one that matters
//!
//! `stand_ins` replaced a constant printed on every launch before the content
//! root had been read: it named no key and read identically whether every
//! declared key was covered or none was. So the reading that says the sentence is
//! *absent* for a fully covered set is the one carrying the defect, and it is
//! worth nothing beside a composer that never speaks — which is why the readings
//! naming keys sit in the same file. An over-eager composer naming every declared
//! key fails all four; an inert one fails the two that name keys.
//!
//! The fourth reading is the one none of the three scenarios reach: a key the set
//! covers that no block declares. `declared.difference(covered)` passes it and
//! `symmetric_difference` names it, and the three scenario readings above are
//! green for both.

use std::collections::BTreeSet;
use std::error::Error;

use glam::Vec3;
use mc_core::id::{BlockName, TextureKey};
use mc_sim::world::Clearing;

use super::recording::Recorder;
use super::{
    changed_blocks, entering, reloading, say_changed_blocks, say_entering, say_reloading,
    say_stand_ins, stand_ins,
};

/// What the readings that parse a block name propagate with `?`.
///
/// The clearing readings above take no name and stay infallible; a name that is
/// not a namespaced id is a broken fixture rather than a claim about the
/// composer, so it is reported as an error and never asserted about.
type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_player_moved_at_entry_is_told_they_would_have_entered_inside_solid_blocks_and_where_they_were_put()
 {
    let said = entering(Clearing::MovedTo(Vec3::new(12.5, 10.0, 12.5)));

    assert_eq!(
        said.as_deref(),
        Some(
            "mycraft: you would have entered the world inside solid blocks, so you were moved to \
             (12.5, 10, 12.5)"
        ),
        "a player who quit inside water and relaunched after it was declared solid appears \
         somewhere they did not save, and the sentence is the whole of what tells them that was \
         the game doing its job rather than a bug: what happened, and where they now are"
    );
}

#[test]
fn a_player_who_needed_no_moving_at_entry_is_told_nothing_about_where_they_were_placed() {
    let said = entering(Clearing::Unneeded);

    assert_eq!(
        said, None,
        "every ordinary launch answers `Unneeded`, so anything composed here is a line on the \
         terminal of every player on every run — and a message that appears when nothing happened \
         teaches its reader to stop reading the ones that mean something"
    );
}

#[test]
fn a_player_with_nothing_clear_within_reach_is_told_they_were_left_inside_the_solid_blocks() {
    let said = entering(Clearing::NoClearSpaceWithin { blocks: 8 });

    assert_eq!(
        said.as_deref(),
        Some(
            "mycraft: you would have entered the world inside solid blocks and nothing within 8 \
             blocks is clear, so you were left inside them"
        ),
        "the launch proceeds and the player is still inside rock; this sentence is the only thing \
         in the run that says so, and without it being stuck is a broken game rather than an \
         explained state they can edit their way out of"
    );
}

#[test]
fn a_player_moved_by_a_reload_is_told_the_reload_made_their_cell_solid_and_never_the_entry_sentence()
 {
    let said = [
        reloading(Clearing::MovedTo(Vec3::new(12.5, 10.0, 12.5))),
        reloading(Clearing::NoClearSpaceWithin { blocks: 8 }),
    ];

    assert_eq!(
        said,
        [
            Some(
                "mycraft: the reload made your cell solid, so you were moved to (12.5, 10, 12.5)"
                    .to_owned()
            ),
            Some(
                "mycraft: the reload made your cell solid and nothing within 8 blocks is clear, \
                 so you were left where you were"
                    .to_owned()
            ),
        ],
        "a player who was already playing watched their cell become solid, and one who is \
         arriving did not witness anything — so the reload keeps its own wording, character for \
         character with what it printed before this module existed"
    );
}

/// A hardcoded `8` composes FR-2.1-S3's sentence exactly and is wrong.
///
/// The reach is `Clearing::NoClearSpaceWithin { blocks }`, and the search's
/// `REACH` (`crates/mc-sim/src/world/clearing.rs:38`) is free to change. Asking
/// with 3 is what makes the interpolation observable at all; both refusals are
/// asked because the reload's carries the same field and neither may quietly
/// become a literal.
#[test]
fn both_refusals_name_the_reach_the_verdict_carries_rather_than_a_literal_eight() {
    let three = Clearing::NoClearSpaceWithin { blocks: 3 };

    assert_eq!(
        [entering(three), reloading(three)],
        [
            Some(
                "mycraft: you would have entered the world inside solid blocks and nothing within \
                 3 blocks is clear, so you were left inside them"
                    .to_owned()
            ),
            Some(
                "mycraft: the reload made your cell solid and nothing within 3 blocks is clear, \
                 so you were left where you were"
                    .to_owned()
            ),
        ],
        "a sentence stating a reach the verdict does not carry tells a player the game looked 8 \
         blocks when it looked 3, and it reads as true because the two agree today"
    );
}

/// The formatting trap from the other side, so a red FR-2.1-S1 says *why*.
///
/// Every coordinate a clearing produces is a cell centre, so `y` is always whole
/// (`centre_of`, `crates/mc-sim/src/world/clearing.rs:120-127`, adds `0.5` to `x`
/// and `z` only). `Display` on `f32` renders a whole number without its
/// fractional part, and the sentences were written against that. A `{:.1}` added
/// for tidiness renders `(4.0, 70.0, -3.0)` and reddens two tests whose message
/// is about the wording; this one is about the number.
#[test]
fn an_entry_sentence_renders_a_whole_number_coordinate_without_a_trailing_zero() {
    let said = entering(Clearing::MovedTo(Vec3::new(4.0, 70.0, -3.0)));

    assert_eq!(
        said.as_deref(),
        Some(
            "mycraft: you would have entered the world inside solid blocks, so you were moved to \
             (4, 70, -3)"
        ),
        "the coordinates a player reads are the ones they type into a debug overlay or a chat \
         message; a trailing zero on every axis is noise, and a precision specifier is how it \
         arrives"
    );
}

/// The two pairs being unified, in the direction FR-2.1-S4 cannot see.
///
/// S4 catches `reloading` returning the entry wording. What it cannot catch is
/// one function grown a parameter with both call sites pointed at it — and this
/// cannot catch that either, **because with the four texts still distinct there
/// is nothing observable left to catch.** What this does hold is the observable
/// half: no verdict is told in the same words at the two moments.
///
/// **`Unneeded` is deliberately outside the claim.** Both moments compose nothing
/// for it, so `None == None` is agreement rather than unification, and asserting
/// otherwise would redden a correct implementation.
#[test]
fn no_clearing_verdict_is_told_in_the_same_words_at_entry_and_at_reload() {
    let moved = Clearing::MovedTo(Vec3::new(1.5, 2.0, 3.5));
    let refused = Clearing::NoClearSpaceWithin { blocks: 5 };

    let told_alike: Vec<Clearing> = [moved, refused]
        .into_iter()
        .filter(|verdict| entering(*verdict) == reloading(*verdict))
        .collect();

    assert_eq!(
        told_alike,
        Vec::new(),
        "a player told `the reload made your cell solid` about a launch is told about a change \
         they did not witness, and one told the entry wording mid-session is told they just \
         arrived; the verdict named above is the one whose two moments have collapsed into one \
         sentence"
    );
}

/// Three names, which no fixture in this suite's launches produces.
///
/// The saves those launches read hold one changed block or two, so the join is
/// only ever exercised at a single separator. Three is where a fold that dropped
/// the last name, emitted a trailing separator, or reversed the order stops being
/// invisible — and none of those is reachable with two.
#[test]
fn every_changed_block_is_named_in_the_order_it_was_given_separated_once_each() -> TestResult {
    let said = changed_blocks(&[
        BlockName::parse("mod:copper")?,
        BlockName::parse("mod:iron")?,
        BlockName::parse("mod:tin")?,
    ]);

    assert_eq!(
        said.as_deref(),
        Some(
            "mycraft: `mod:copper`, `mod:iron`, `mod:tin` no longer behave as they did when this \
             world was saved, and it was loaded anyway"
        ),
        "a player whose mods have changed acts on this line by reading it, so it names every block \
         and never the first few. The refusal it replaces printed both of its lists whole, and a \
         line that reported less than the refusal did would be a step backwards"
    );
    Ok(())
}

/// The singular, which is the common case and the one a plural sentence is wrong
/// about.
#[test]
fn one_changed_block_is_told_in_the_singular() -> TestResult {
    let said = changed_blocks(&[BlockName::parse("mod:copper")?]);

    assert_eq!(
        said.as_deref(),
        Some(
            "mycraft: `mod:copper` no longer behaves as it did when this world was saved, and it \
             was loaded anyway"
        ),
        "one block is what a player usually has, and `blocks no longer behave` is wrong about \
         their world. A single sentence that read correctly for the plural is the shape this \
         catches"
    );
    Ok(())
}

/// The keys `written` names, as the set a launch compares.
///
/// # Errors
///
/// Returns the parse failure of a fixture key that is not a namespaced id.
fn keys(written: &[&str]) -> Result<BTreeSet<TextureKey>, Box<dyn Error>> {
    written
        .iter()
        .map(|key| TextureKey::parse(key).map_err(Into::into))
        .collect()
}

/// A launch whose built set covers everything says nothing at all, which is the
/// half the constant this replaces got wrong.
#[test]
fn a_built_set_covering_every_declared_key_composes_no_line() -> TestResult {
    let declared = keys(&["base:dirt", "base:stone", "base:water"])?;

    assert_eq!(
        stand_ins(&declared, &declared),
        None,
        "every key content declared has baked art and draws it, so there is nothing anybody can \
         act on and a line here would be on every player's terminal on every run — which is \
         exactly what the constant this replaces was. A composer that names the declared keys \
         without consulting the built set prints all three"
    );
    Ok(())
}

/// Three uncovered names beside one that is covered, asked out of order.
///
/// Three is where a fold that dropped the last name, left a trailing separator or
/// kept the order it was asked in stops being invisible, and the covered key is
/// what keeps "name every declared key" from reading correctly.
#[test]
fn every_key_the_built_set_left_uncovered_is_named_in_ascending_order() -> TestResult {
    let declared = keys(&["mod:tin", "mod:copper", "mod:iron", "mod:zinc"])?;
    let covered = keys(&["mod:iron"])?;

    assert_eq!(
        stand_ins(&declared, &covered).as_deref(),
        Some(
            "mycraft: `mod:copper`, `mod:tin`, `mod:zinc` draw generated stand-ins because \
             nothing has baked them, and that is not a failure"
        ),
        "a mod author has to go and bake each of these, so each has to be named — a bounded list \
         is one they cannot act on, which is `changed_blocks`'s rule on reasoning that applies \
         here word for word. `mod:iron` has art and is not among them"
    );
    Ok(())
}

/// The control: a key the set covers is never named, and the singular is the
/// case a mod author's first block is in.
#[test]
fn a_key_the_built_set_covers_is_not_named_among_the_stand_ins() -> TestResult {
    let declared = keys(&["mod:copper", "mod:iron"])?;
    let covered = keys(&["mod:iron"])?;

    assert_eq!(
        stand_ins(&declared, &covered).as_deref(),
        Some(
            "mycraft: `mod:copper` draws a generated stand-in because nothing has baked it, and \
             that is not a failure"
        ),
        "`mod:iron` has baked art and draws it, so naming it would send an author looking for a \
         file that is already there. A composer naming every declared key reports both and reads \
         perfectly well while doing it, which is why this reading exists"
    );
    Ok(())
}

/// A set covering a key nothing declares is not a stand-in and is not anybody's
/// business.
///
/// The reading none of the three scenarios reach: `symmetric_difference` in place
/// of `difference` names `mod:spare` here and satisfies all three of them.
#[test]
fn a_covered_key_no_block_declares_is_not_named_either() -> TestResult {
    let declared = keys(&["mod:copper"])?;
    let covered = keys(&["mod:copper", "mod:spare"])?;

    assert_eq!(
        stand_ins(&declared, &covered),
        None,
        "the line is about keys that draw a stand-in, and a key no block declares draws nothing \
         at all. Naming it would send an author looking for the block that uses it, and there is \
         none"
    );
    Ok(())
}

/// What a caller who supplied a sink reads back, exactly.
///
/// **The point of the sink is that this is answerable at all.** Every one of
/// these lines went to the process error stream by name, so a harness could not
/// read one, a caller could not route one elsewhere, and nothing could silence
/// them. Four of the crate's nine are reachable by a direct call and are asserted
/// here; `tests/no_notice_names_the_error_stream.rs` covers all nine, including
/// the five that need a window.
#[test]
fn every_notice_a_caller_asks_for_is_read_back_off_the_sink_they_supplied() -> TestResult {
    let (recorder, notices) = Recorder::listening();

    say_entering(Clearing::MovedTo(Vec3::new(12.5, 10.0, 12.5)), &notices);
    say_reloading(Clearing::NoClearSpaceWithin { blocks: 8 }, &notices);
    say_changed_blocks(&[BlockName::parse("mod:copper")?], &notices);
    say_stand_ins(&keys(&["mod:tin"])?, &BTreeSet::new(), &notices);

    assert_eq!(
        recorder.said(),
        "mycraft: you would have entered the world inside solid blocks, so you were moved to \
         (12.5, 10, 12.5)\n\
         mycraft: the reload made your cell solid and nothing within 8 blocks is clear, so you \
         were left where you were\n\
         mycraft: `mod:copper` no longer behaves as it did when this world was saved, and it was \
         loaded anyway\n\
         mycraft: `mod:tin` draws a generated stand-in because nothing has baked it, and that is \
         not a failure\n",
        "this is the whole capability the sink exists for: a caller hands one in and reads back \
         exactly what a person at the terminal would have read, in the order it was said. Four \
         lines, four calls, nothing else"
    );
    Ok(())
}

/// The control: a notice whose composer answered `None` writes nothing.
///
/// **It carries a line that *is* said**, which is what keeps it from passing for
/// the wrong reason. An implementation that wrote nothing at all satisfies "these
/// three said nothing" perfectly well, and this reading rejects it because the
/// fourth call is missing from the comparison. One that dropped the `if let Some`
/// and wrote unconditionally fails on the three empty ones.
#[test]
fn a_notice_with_nothing_to_say_puts_nothing_on_the_sink() -> TestResult {
    let (recorder, notices) = Recorder::listening();
    let covered = keys(&["base:dirt"])?;

    say_entering(Clearing::Unneeded, &notices);
    say_reloading(Clearing::Unneeded, &notices);
    say_stand_ins(&covered, &covered, &notices);
    say_changed_blocks(&[BlockName::parse("mod:copper")?], &notices);

    assert_eq!(
        recorder.said(),
        "mycraft: `mod:copper` no longer behaves as it did when this world was saved, and it was \
         loaded anyway\n",
        "three of these four had nothing to say and the fourth did, so the sink holds one line. \
         Every ordinary launch takes the first three paths, and a client that wrote on them would \
         put three lines on every player's terminal on every run"
    );
    Ok(())
}

/// A panic while the sink is held must not silence every later notice.
///
/// **This is the spec's own defect one level up.** A `Mutex` guarding a byte sink
/// has no invariant a panic can corrupt, so treating a poisoned lock as fatal
/// would mean one unrelated panic stops the client telling anybody anything —
/// which is exactly the failure mode the sink was introduced to end.
#[test]
fn a_notice_is_still_said_after_a_panic_poisoned_the_sink() {
    let (recorder, notices) = Recorder::listening();
    let held = notices.clone();
    let panicked = std::thread::spawn(move || {
        // A panic this project's lints permit. `panic!`, `panic_any`, `unwrap`,
        // `expect` and slice indexing are all denied workspace-wide, and this
        // reading needs a real panic *while the lock is held* — that is what
        // poisons it, and a poisoned lock is the whole subject.
        held.with(|_| Vec::<u8>::new().remove(0));
    })
    .join();

    notices.say("mycraft: said after the poisoning");

    assert_eq!(
        (panicked.is_err(), recorder.said()),
        (true, "mycraft: said after the poisoning\n".to_owned()),
        "the first element is the premise: a lock nobody poisoned would make this reading pass \
         about nothing. A client that propagated the poisoning would go quiet for the rest of the \
         run over a panic that had nothing to do with its sink"
    );
}
