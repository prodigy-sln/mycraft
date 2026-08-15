//! Where the assembled model sits once every part is placed, and how big it is
//! allowed to get.
//!
//! Two facts are graded here and they pull in opposite directions. Normalisation
//! is **model-wide**: a part reaching below the model's lowest voxel drags the
//! whole model up with it, so a fixture whose parts all start at their own zero
//! could not tell a model-wide translation from a per-part one. And the 64-voxel
//! limit is about the *assembled* box rather than any declared `size`, so both
//! span fixtures declare parts of 32 — comfortably legal on their own — and put
//! the whole question in the attachment.
//!
//! The span fixtures are one voxel apart on purpose. 64 must load and 65 must
//! not, and an off-by-one in the comparison passes whichever of the two it is
//! written against, so neither is graded without the other.

mod common;

use std::path::Path;

use common::{
    FIXTURE_FILE, TestResult, all_named, assembled, assembly_refusal, at, positions_of,
    solid_y_layers, torch, unnamed,
};
use glam::IVec3;
use voxforge::format::{Extent, PartName};
use voxforge::volume::StateSelection;

/// What the torch's handle is painted with.
const HANDLE_MATERIAL: &str = "base:oak_plank";

/// What the torch's flame is painted with.
const FLAME_MATERIAL: &str = "base:flame";

/// A model whose only art is the empty marker.
///
/// The palette itself is not empty — `w` is declared and never spelled — so the
/// refusal has to be about the assembled art rather than about the palette.
const NOTHING_FILLED: &str = r#"schema = 1
name = "base:probe"
scale = 16
size = [2, 2, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
"." = "empty"
"w" = "base:oak_plank"

[[layers]]
y = 0
grid = """
..
..
"""
"#;

/// Two parts whose combined art reaches `hub`'s own origin from below on every
/// axis, by a different distance on each.
///
/// `arm`'s pivot is `[3, 4, 5]` and it attaches at `hub`'s `[0, 0, 0]`, so its
/// translation is `(−3, −4, −5)` and its art spans `x −3..−2`, `y −4..−3`,
/// `z −5..−4`. `hub` spans `0..1` on every axis, so the model's lowest art is
/// `(−3, −4, −5)`: a translation that used the wrong corner, or that ran on one
/// axis only, cannot land on `(0, 0, 0)` by accident.
fn spread() -> String {
    format!(
        r#"schema = 1
name = "base:spread"
scale = 16
slice = "y"

[palette]
"w" = "base:oak_plank"

[[parts]]
name = "hub"
size = [2, 2, 2]
origin = [0, 0, 0]

[[parts]]
name = "arm"
size = [2, 2, 2]
origin = [3, 4, 5]
attach = {{ to = "hub", at = [0, 0, 0] }}
{hub}{arm}"#,
        hub = solid_y_layers("hub", (2, 2, 2), 'w'),
        arm = solid_y_layers("arm", (2, 2, 2), 'w'),
    )
}

/// Two 32-voxel bars, the second attached `reach` along x from the first.
///
/// `left` spans `x 0..31`; `right` spans `x reach..reach + 31`. At `reach = 32`
/// the assembled model is exactly 64 wide and at `reach = 33` it is 65.
fn span(reach: i32) -> String {
    format!(
        r#"schema = 1
name = "base:span"
scale = 16
slice = "y"

[palette]
"w" = "base:oak_plank"

[[parts]]
name = "left"
size = [32, 1, 1]
origin = [0, 0, 0]

[[parts]]
name = "right"
size = [32, 1, 1]
origin = [0, 0, 0]
attach = {{ to = "left", at = [{reach}, 0, 0] }}
{left}{right}"#,
        left = solid_y_layers("left", (32, 1, 1), 'w'),
        right = solid_y_layers("right", (32, 1, 1), 'w'),
    )
}

#[test]
fn a_part_reaching_below_the_model_moves_the_whole_model_rather_than_only_itself() -> TestResult {
    // The flame's lowest voxel sits at the pre-normalisation position
    // (−1, 10, −1) and the handle's at (0, 0, 0). Normalising the model adds
    // (1, 0, 1) to both, so the flame's lands on x = 0 and z = 0 while the
    // handle's moves to (1, 0, 1) — which is exactly what a per-part
    // normalisation, putting both at their own zero, would destroy.
    let volume = assembled(&torch(), &StateSelection::default())?;
    let reaching = volume.placed(&PartName::new("flame"), at(0, 0, 0));
    let anchored = volume.placed(&PartName::new("handle"), at(0, 0, 0));

    assert_eq!(
        (reaching, anchored),
        (Some(IVec3::new(0, 10, 0)), Some(IVec3::new(1, 0, 1))),
        "a pivot lets a part reach into negative space, and the model as a whole is what translates back"
    );
    Ok(())
}

#[test]
fn an_assembled_model_reports_a_bounding_box_starting_at_the_origin() -> TestResult {
    // The whole box, and how much art is inside it. The minimum corner alone is
    // what the scenario names, and on its own it cannot fail: an assembler that
    // never translated its cells drops every negative one on the conversion to
    // an unsigned position, so `arm` vanishes and `hub` — already at 0..1 — is
    // left reporting a minimum of (0, 0, 0) for a model that has lost half its
    // voxels. Both halves of this expectation come off the fixture text: the box
    // spans x −3..1, y −4..1 and z −5..1, so its far corner is 4, 5, 6 once the
    // near one is the origin; and two solid 2 × 2 × 2 parts that do not overlap
    // are 8 + 8 voxels.
    let volume = assembled(&spread(), &StateSelection::default())?;

    assert_eq!(
        (volume.filled_bounds(), volume.filled().len()),
        (Some((at(0, 0, 0), at(4, 5, 6))), 16),
        "the assembled model is normalised onto its own art, so its lowest filled voxel is the origin on every axis at once — and every voxel that went in is still there"
    );
    Ok(())
}

/// No scenario states this. It exists because the torch is read by three tests
/// and **all three read `placed()`**, which answers out of the placement map —
/// so the largest model in this file, and the only one where a part's art
/// straddles the boundary the translation moves it across, has its actual voxel
/// data asserted by nothing.
///
/// `spread()` witnesses the same code path one test above, and on every defect
/// either fixture can presently name, it witnesses it first. The two are not
/// interchangeable all the same: `spread()`'s parts sit wholly on one side of
/// the boundary, so a defect there deletes a part outright, while the torch's
/// flame loses three of its twelve outer columns and still reports a plausible
/// shape with the right minimum corner. Plausible-but-thinner is the failure
/// this file could not otherwise see, and "I could not think of a defect that
/// hides there" is the reasoning that has already lost twice in this spec.
#[test]
fn a_model_translated_off_negative_coordinates_keeps_every_voxel_of_every_part() -> TestResult {
    // Both parts are solid, so each contributes exactly the product of its
    // declared extent: the handle is 2 × 10 × 2 = 40 and the flame is
    // 4 × 6 × 4 = 96. They cannot overwrite one another — the handle occupies
    // y 0..9 and the flame y 10..15 — so the two counts are independent, and
    // being unequal they also refuse a reading that swapped the materials.
    let volume = assembled(&torch(), &StateSelection::default())?;

    assert_eq!(
        (
            positions_of(&volume, HANDLE_MATERIAL)?.len(),
            positions_of(&volume, FLAME_MATERIAL)?.len(),
        ),
        (40, 96),
        "translating the model must move every voxel, not discard the ones that were reaching below the origin"
    );
    Ok(())
}

#[test]
fn a_model_whose_assembled_art_is_empty_is_refused_naming_the_model() -> TestResult {
    let fault = assembly_refusal(NOTHING_FILLED, &StateSelection::default())?;

    assert_eq!(
        (
            fault.origin.as_path(),
            unnamed(&fault, &["base:probe", "no filled voxel"]),
        ),
        (Path::new(FIXTURE_FILE), all_named()),
        "a model that assembles to nothing is a document whose art never landed, not a legitimate empty answer; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn an_attachment_spreading_the_model_past_the_axis_limit_is_refused_naming_the_axis() -> TestResult
{
    let fault = assembly_refusal(&span(33), &StateSelection::default())?;

    assert_eq!(
        unnamed(&fault, &["axis x", "65", "limit is 64"]),
        all_named(),
        "neither part declares more than 32, so the axis and the assembled figure are the only things telling the author what to shorten; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn an_attachment_spreading_the_model_to_exactly_the_axis_limit_assembles() -> TestResult {
    // 32 voxels from x = 0 and 32 more from x = 32: 64 on x, and one voxel on
    // each of the other two axes, counted off the declared sizes rather than
    // read from a run.
    let volume = assembled(&span(32), &StateSelection::default())?;

    assert_eq!(
        volume.extent(),
        Extent { x: 64, y: 1, z: 1 },
        "64 is the limit and not one past it, so a model reaching it exactly is a legal model"
    );
    Ok(())
}
