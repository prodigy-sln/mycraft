//! Every frame this repository has committed, redrawn from a tree whose sea
//! declares a tint and compared against the bytes on disk with no tolerance at
//! all.
//!
//! # This strengthens `a_dry_judged_frame_is_unmoved_by_a_declared_sea_tint`; it
//! does not repeat it
//!
//! That file is FR-3.1's reading and it carries the control — a submerged frame
//! over the same tree has to move — without which any "nothing moved" claim
//! passes about a renderer with no tint in it. **Borrowing that control here
//! would read as strength and be a second copy of it through the same code
//! path**, so it is named rather than repeated: the control lives next door and
//! this reading rests on it.
//!
//! What is *not* next door is either of the two things below, and both are
//! defects this spec could actually ship.
//!
//! **The tolerance.** That reading judges through the golden lifecycle, whose
//! default thresholds are ΔE 2.0 per pixel over a 0.01% area budget — 92 wrong
//! pixels of 921 600. Two leaks pass it: a tint reaching up to ninety-two pixels
//! at any strength, and a tint reaching **every** pixel at a strength under
//! ΔE 2.0. The second is the shape a dry frame would actually leak in, because a
//! strength that failed to reach the literal zero applies to the whole frame at
//! once. The stronger claim is available and the spec asks for it: where the eye
//! stands in nothing that declares a medium the strength written into the frame
//! record is the literal `0.0`, and `mix(a, b, 0.0)` returns `a` bit-exactly
//! under every form a backend compiles it into — no branch, no second code path,
//! and so no bit of a dry frame a correct implementation may move.
//!
//! **The fourth capture.** `crates/mc-render/goldens/` holds four directories.
//! Three are terrain captures shot through `record_terrain`; the fourth is the
//! HUD capture shot through `record_frame`, the one call the windowed client
//! makes. FR-3.1's reading walks `DECLARED_TICKS`, which is the three — so the
//! capture the HUD lives in is judged by `hud_goldens.rs` under the same
//! tolerance and by nothing else. The reading below walks the set the capture
//! library declares, terrain and HUD alike, and checks that count back against
//! the directories that exist.
//!
//! # Measured, on the tree this was written against
//!
//! All four captures redraw byte-identical to their committed blobs — 0 of
//! 3 686 400 channel bytes apart on each. That is what makes the claim usable
//! rather than aspirational, and it is a property of this adapter as well as of
//! the renderer: a machine whose driver rasterises differently would fail this
//! before it failed the tolerance reading, and the right response there is to
//! say so rather than to loosen this.

mod support;

use std::error::Error;

use support::TestResult;
use support::committed_captures::{
    Committed, Difference, Frames, declared, directories_committed, frames_against,
    the_declared_ids_agree, the_golden_update_opt_in_is_set,
};
use support::content::{ContentRoot, SEA_FILE, shipped_copy};
use support::medium::{REACHES_AT, TINT};

/// What the committed set came to when every capture was redrawn and judged
/// against its own committed frame.
#[derive(Debug, PartialEq)]
struct Unmoved {
    /// Whether the run reading these blobs may also be rewriting them.
    the_golden_update_opt_in_was_set: bool,
    /// Whether the captures redrawn here are the ones the capture library
    /// declares, rather than a list this file keeps.
    the_captures_judged_are_the_librarys_own: bool,
    /// How many capture directories stand under the golden root.
    frames_committed_on_disk: usize,
    /// What redrawing every one of them came to.
    redrawn: Frames,
}

#[test]
fn every_committed_capture_is_drawn_byte_for_byte_as_the_frame_committed_for_it() -> TestResult {
    let captures = declared()?;
    let judged: Vec<(Committed, Committed)> = captures
        .iter()
        .map(|capture| (capture.clone(), capture.clone()))
        .collect();

    let Some(redrawn) = frames_against(a_sea_that_tints()?.path(), &judged)? else {
        return Ok(());
    };
    assert_eq!(
        Unmoved {
            the_golden_update_opt_in_was_set: the_golden_update_opt_in_is_set(),
            the_captures_judged_are_the_librarys_own: the_declared_ids_agree()?,
            frames_committed_on_disk: directories_committed()?,
            redrawn,
        },
        nothing_moved(captures.len()),
        "no capture this repository commits is shot from an eye inside anything that declares a \
         medium, so a sea declaring a tint reaching its full strength at {REACHES_AT} blocks \
         reaches no pixel of any of them — and every one has to draw the bytes already committed \
         for it, with none of the ΔE 2.0 over 92 pixels the golden lifecycle would forgive. A \
         tint that reached the whole frame faintly is inside that budget and outside this. The \
         count on disk stands beside the count redrawn because a capture nothing here judged is a \
         frame this reading is silent about, and the opt-in is named because a run minting \
         goldens underneath this comparison would be reading blobs it had just written"
    );
    Ok(())
}

#[test]
fn that_same_comparison_reports_a_capture_drawn_against_another_captures_committed_frame()
-> TestResult {
    let captures = declared()?;
    let (first, second) = two_captures_of_different_ticks(&captures)?;

    let root = a_sea_that_tints()?;
    let Some(redrawn) = frames_against(root.path(), &[(first.clone(), second.clone())])? else {
        return Ok(());
    };
    assert_eq!(
        redrawn,
        Frames::Moved {
            compared: 1,
            moved: vec![(
                second.id.clone(),
                Difference::BytesTheCommittedFrameDoesNotHold,
            )],
        },
        "a comparison that always answers `matching` reads exactly like a set that never moved, \
         and it is the answer this reading would get from a decoder handing back the frame it was \
         just given, from a blob opened out of the wrong directory, or from a loop that compared \
         nothing. Without this the reading above would go green the day it stopped being able to \
         look — which is the day it is needed. So the same comparator is driven over `{}`'s frame \
         against `{}`'s committed bytes — two captures of two different ticks of one walk, which \
         cannot be one picture — and it has to name the capture it was judged against rather than \
         pass",
        first.id,
        second.id
    );
    Ok(())
}

/// Two of the committed captures whose ticks differ, so the frames they draw
/// cannot be one picture.
///
/// **A fixture guard rather than an assertion**, because a comparator driven
/// over two captures that happened to be the same picture would report
/// `matching` and be right to — which is the reading passing for a reason that
/// has nothing to do with what it is about.
///
/// # Errors
///
/// Returns an error when the revision declares fewer than two captures, or when
/// the first two are of one tick.
fn two_captures_of_different_ticks(
    captures: &[Committed],
) -> Result<(&Committed, &Committed), Box<dyn Error>> {
    let [first, second, ..] = captures else {
        return Err(format!(
            "this reading needs two committed captures of two different ticks to judge one \
             against the other's frame, and the current revision declares {}",
            captures.len()
        )
        .into());
    };
    if first.tick == second.tick {
        return Err(format!(
            "this reading needs the two captures it crosses to be of different ticks, so that \
             they cannot be the same picture, and `{}` and `{}` are both at tick {}",
            first.id, second.id, first.tick
        )
        .into());
    }
    Ok((first, second))
}

/// The verdict the committed set owes: nothing minting, the library's own list,
/// as many frames on disk as were redrawn, and every one drawn as committed.
fn nothing_moved(captures: usize) -> Unmoved {
    Unmoved {
        the_golden_update_opt_in_was_set: false,
        the_captures_judged_are_the_librarys_own: true,
        frames_committed_on_disk: captures,
        redrawn: Frames::EveryCommittedFrameIsDrawnByteForByte { compared: captures },
    }
}

/// A copy of the shipped root whose sea declares [`TINT`] at [`REACHES_AT`].
///
/// The tree the scenario names, rather than the shipped one: before the sea's
/// own declaration lands these two are different trees, and a reading over the
/// shipped root would be about a world with no medium declared in it at all.
fn a_sea_that_tints() -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?.whose_block_declares(SEA_FILE, Some((TINT, REACHES_AT)))
}
