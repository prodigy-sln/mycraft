//! The committed captures, compared against frames drawn from a tree whose sea
//! declares a tint.
//!
//! # This reading is the deliverable, not the expectation behind it
//!
//! No judged camera is submerged, so a correct implementation leaves every
//! committed capture byte-identical. That is an *expectation*; what stands
//! between this spec and a tint leaking into a dry frame is the comparison
//! actually being made. The scene revision deliberately does not move, and the
//! reason is this reading: a bump renames the set by deletion and a fresh mint,
//! the freshly minted set would be byte-identical to the deleted one, and the
//! comparison that proves the change safe would have been destroyed to make
//! room for one that compares the new frames only against themselves.
//!
//! # Its premise is asserted rather than assumed
//!
//! "No judged camera is submerged" is a fact about the world, the spawn and the
//! declared intent script, and all three can move. So each declared tick's own
//! camera cell is reported beside its capture's verdict: a camera standing in a
//! cell that holds a drawn block is **named, with its tick**, rather than
//! quietly making the comparison mean something else.
//!
//! # And the whole reading needs a control
//!
//! Every capture matching is exactly what a renderer that cannot tint at all
//! produces. So the verdict also states that the same tree **does** move a frame
//! whose eye is inside the declared sea — the pose `support/submerged.rs` holds,
//! over the tinting root and over one declaring nothing. Without that element
//! this reading passes about a renderer with no tint in it, which is the failure
//! it exists to make impossible.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_testkit::frame::{GoldenOutcome, OptIns};

use support::TestResult;
use support::content::{ContentRoot, SEA_FILE, shipped_copy};
use support::goldens::{DECLARED_TICKS, artifact_root, judged_over};
use support::medium::{REACHES_AT, TINT};
use support::oracle::Voxels;
use support::submerged::{EYE, differing, drawn_from};
use support::{frames, prepare_scene_at};

/// What the committed set came to against a tree whose sea declares a tint.
#[derive(Debug, PartialEq)]
struct Judged {
    /// Every declared capture that did not match, named with its tick.
    captures_that_did_not_match: Vec<String>,
    /// Every declared tick whose camera stands in a cell holding a drawn block,
    /// named with the block and the capture it belongs to.
    cameras_standing_in_a_drawn_cell: Vec<String>,
    /// Whether the same tree moves a frame whose eye *is* inside the declared
    /// sea.
    a_submerged_frame_over_the_same_tree_moves: bool,
}

#[test]
fn every_committed_capture_is_unmoved_by_a_sea_that_declares_a_tint() -> TestResult {
    require_the_goldens_are_not_being_minted()?;
    let Some(judged) = what_the_committed_set_comes_to()? else {
        return Ok(());
    };
    assert_eq!(
        judged,
        Judged {
            captures_that_did_not_match: Vec::new(),
            cameras_standing_in_a_drawn_cell: Vec::new(),
            a_submerged_frame_over_the_same_tree_moves: true,
        },
        "the declared walk wades and the eye never goes under, so a sea that declares a tint \
         reaching its full strength at {REACHES_AT} blocks leaves every committed capture byte \
         for byte where it was — which is why the scene revision does not move. The second \
         element is that premise, asserted rather than assumed: a camera that had come to stand \
         inside a drawn cell would make the first element mean something else, and is named with \
         its tick instead. The third is the control, and without it this reading passes about a \
         renderer that cannot tint anything at all"
    );
    Ok(())
}

/// Refuses a run that would mint the goldens rather than compare against them.
///
/// # Errors
///
/// Returns an error when `MYCRAFT_UPDATE_GOLDENS` is set — a run that writes the
/// committed set and then reports it as matching is a reading of its own output.
fn require_the_goldens_are_not_being_minted() -> Result<(), Box<dyn Error>> {
    if OptIns::from_environment().update_goldens {
        return Err(
            "this reading compares the captures committed before this spec's first \
                    implementation commit against frames drawn from a tree that declares a tint, \
                    and `MYCRAFT_UPDATE_GOLDENS` is set — so the run would write those captures \
                    and then report that they matched. That is a reading of its own output"
                .into(),
        );
    }
    Ok(())
}

/// Every declared capture's verdict, every declared camera's own cell, and the
/// control.
///
/// `None` where the opt-in permitted the absence of a device.
fn what_the_committed_set_comes_to() -> Result<Option<Judged>, Box<dyn Error>> {
    let tinted = a_sea_that_tints()?;
    let mut unmatched = Vec::new();
    for tick in DECLARED_TICKS {
        let Some(outcome) = judged_over(tinted.path(), tick, tick, artifact_root()?)? else {
            return Ok(None);
        };
        unmatched.extend(reported(tick, &outcome));
    }

    let plain = a_sea_declaring_nothing()?;
    let Some(under) = drawn_from(&tinted, EYE, "golden-control-tinted")? else {
        return Ok(None);
    };
    let Some(under_plain) = drawn_from(&plain, EYE, "golden-control-plain")? else {
        return Ok(None);
    };
    Ok(Some(Judged {
        captures_that_did_not_match: unmatched,
        cameras_standing_in_a_drawn_cell: standing_in_a_drawn_cell(
            &tinted,
            &declared_poses(&tinted)?,
        )?,
        a_submerged_frame_over_the_same_tree_moves: differing(&under.frame, &under_plain.frame) > 0,
    }))
}

/// One declared capture's tick, and where its camera stands.
type JudgedPose = (u16, [f32; 3]);

/// Where each declared capture's camera stands, taken from the simulation by
/// advancing it under the declared intent script.
fn declared_poses(root: &ContentRoot) -> Result<Vec<JudgedPose>, Box<dyn Error>> {
    let prepared = prepare_scene_at(root.path())?;
    let mut poses = Vec::new();
    for tick in DECLARED_TICKS {
        let pose = frames::player_pose(u32::from(tick), &prepared.world, &prepared.registry)?;
        poses.push((tick, pose.eye));
    }
    Ok(poses)
}

/// Every named camera standing in a cell that holds a drawn block, reported with
/// the capture it belongs to and the block it stands in.
///
/// **Read off the world and the registry rather than off a frame**, through the
/// same door `support::oracle` reads drawnness through, so a camera that had
/// come to stand under the sea is reported here whatever any picture looks like.
///
/// **The poses are a parameter and the declared ones are one value of it**,
/// which is what lets the reading below drive this same scan over cameras that
/// *are* inside terrain. An empty answer is what a clean set of cameras gives
/// and also what a scan that had stopped being able to look would give, and
/// nothing else here can tell those two apart.
fn standing_in_a_drawn_cell(
    root: &ContentRoot,
    poses: &[JudgedPose],
) -> Result<Vec<String>, Box<dyn Error>> {
    let prepared = prepare_scene_at(root.path())?;
    let voxels = Voxels {
        world: &prepared.world,
        registry: prepared.registry.as_ref(),
    };
    let mut standing = Vec::new();
    for (tick, eye) in poses {
        let cell = Vec3::from_array(*eye).floor().as_ivec3();
        if let Some(block) = voxels.drawn_block(cell)? {
            standing.push(format!(
                "the capture declared at tick {tick} draws from a camera whose own cell {cell:?} \
                 holds `{}`",
                block.as_str()
            ));
        }
    }
    Ok(standing)
}

/// How `tick`'s verdict reads to a person, or nothing at all when it matched.
///
/// **Never `{outcome:?}` for a mismatch.** A mismatch carries the per-pixel
/// failing mask, and debug-printing one buries the sentence a reader needs under
/// megabytes of booleans; `GoldenFailure`'s own `Display` says which golden, how
/// many pixels stood past the tolerance and where the evidence was written.
fn reported(tick: u16, outcome: &GoldenOutcome) -> Option<String> {
    match outcome {
        GoldenOutcome::Pass => None,
        GoldenOutcome::Failed(failure) => Some(format!("tick {tick}: {failure}")),
        other => Some(format!(
            "tick {tick}: the golden was minted rather than matched: {other:?}"
        )),
    }
}

#[test]
fn a_judged_camera_standing_in_a_drawn_cell_is_reported_with_its_tick_rather_than_passing()
-> TestResult {
    let tinted = a_sea_that_tints()?;
    let declared = declared_poses(&tinted)?;
    let sunk: Vec<JudgedPose> = declared
        .iter()
        .map(|(tick, eye)| (*tick, [eye[0], SUNK_TO, eye[2]]))
        .collect();

    let named = standing_in_a_drawn_cell(&tinted, &sunk)?;
    assert_eq!(
        (named.len(), named.iter().all(|said| said.contains("tick"))),
        (declared.len(), true),
        "the scan above answers an empty list for the declared cameras, and an empty list is what          a scan that had stopped being able to look also answers. Driven over the same cameras          dropped to y = {SUNK_TO}, which the shipped world fills with terrain everywhere the          declared walk goes, it has to name every one of them and say which capture each belongs          to. What it reported: {named:?}"
    );
    Ok(())
}

/// The height the control drops each declared camera to.
///
/// Well under the shipped world's surface everywhere the declared walk reaches,
/// and above the world's floor, so every dropped camera lands inside terrain
/// rather than outside the world — which the scan would report as nothing drawn
/// and would make this control agree with a scan that could not look.
const SUNK_TO: f32 = 16.5;

/// A copy of the shipped root whose sea declares [`TINT`] at [`REACHES_AT`], and
/// one whose sea declares no tint at all.
fn a_sea_that_tints() -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?.whose_block_declares(SEA_FILE, Some((TINT, REACHES_AT)))
}
fn a_sea_declaring_nothing() -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?.whose_block_declares(SEA_FILE, None)
}
