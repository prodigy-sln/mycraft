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

use glam::Vec3;
use mc_sim::world::Clearing;

use super::{entering, reloading};

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
