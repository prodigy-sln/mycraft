//! How parts hang off one another, and what a set of parts that is not a tree
//! is told.
//!
//! The arithmetic under test is
//! `parent_position + attach.at + p − child.origin`, and the two fixtures that
//! grade it are built so that a wrong implementation lands somewhere visibly
//! else rather than somewhere that happens to agree:
//!
//! - **The torch subtracts a non-zero child origin from a non-zero attachment
//!   point on every axis it can.** `at = [1, 10, 1]` against `origin =
//!   [2, 0, 2]` means an implementation that *added* the origin puts the
//!   flame's pivot at `(5, 10, 5)` rather than on the handle's `(1, 10, 1)`,
//!   and the two are not each other's mirror.
//! - **The chain is two links deep and its three parts differ on every axis.**
//!   A chain that applied only the nearest attachment would put `crown` 5 along
//!   y and nothing along x; one that applied only the root's would put it 4
//!   along x and nothing along y. Both are rejected by one expected value
//!   because the correct answer is the *sum* of two displacements that share no
//!   axis.
//!
//! Every part in these fixtures is solid, which is a constraint no assertion
//! can enforce: the assembled volume is normalised onto the art, so art short
//! of its own declared corners would make every expected coordinate a second
//! calculation rather than the extent the fixture states.

mod common;

use common::{TestResult, all_named, assembled, at, refusal, solid_y_layers, torch, unnamed};
use glam::IVec3;
use voxforge::format::PartName;
use voxforge::volume::StateSelection;

/// Three parts in a two-link chain, each a different shape on every axis.
///
/// `bough` hangs 4 along x off `arm`, and `crown` hangs 5 along y off `bough`,
/// so `crown`'s displacement from `arm` is the sum of two vectors sharing no
/// axis.
fn chain() -> String {
    format!(
        r#"schema = 1
name = "base:bough"
scale = 16
slice = "y"

[palette]
"w" = "base:oak_plank"

[[parts]]
name = "arm"
size = [3, 2, 4]
origin = [0, 0, 0]

[[parts]]
name = "bough"
size = [2, 4, 3]
origin = [0, 0, 0]
attach = {{ to = "arm", at = [4, 0, 0] }}

[[parts]]
name = "crown"
size = [4, 3, 2]
origin = [0, 0, 0]
attach = {{ to = "bough", at = [0, 5, 0] }}
{arm}{bough}{crown}"#,
        arm = solid_y_layers("arm", (3, 2, 4), 'w'),
        bough = solid_y_layers("bough", (2, 4, 3), 'w'),
        crown = solid_y_layers("crown", (4, 3, 2), 'w'),
    )
}

/// A document declaring `parts`, each of them a solid one-voxel part.
///
/// The layers are generated from the same names, so every part here declares
/// art — a part with no layer at all is a different refusal, and a fixture that
/// earned two would grade neither.
fn document(parts: &[(&str, &str)]) -> String {
    let declarations: String = parts
        .iter()
        .map(|(name, attach)| unit_part(name, attach))
        .collect();
    let layers: String = parts
        .iter()
        .map(|(name, _)| solid_y_layers(name, (1, 1, 1), 'w'))
        .collect();
    format!(
        r#"schema = 1
name = "base:probe"
scale = 16
slice = "y"

[palette]
"w" = "base:oak_plank"
{declarations}{layers}"#
    )
}

/// One `[[parts]]` table of a single voxel, carrying `attach` verbatim.
fn unit_part(name: &str, attach: &str) -> String {
    format!("\n[[parts]]\nname = \"{name}\"\nsize = [1, 1, 1]\norigin = [0, 0, 0]\n{attach}")
}

/// An attachment to `parent` at the parent's own origin.
fn attached_to(parent: &str) -> String {
    format!("attach = {{ to = \"{parent}\", at = [0, 0, 0] }}\n")
}

/// Two parts declared under one name, with the two layers naming that name on
/// planes of their own.
///
/// Written out rather than generated because the planes matter: two layers on
/// one plane would be refused as a repeated index, which is a true complaint
/// about a document whose actual defect is the repeated *name*, and a test red
/// for the wrong reason reports nothing when the right one is fixed.
const REPEATED_PART_NAME: &str = r#"schema = 1
name = "base:probe"
scale = 16
slice = "y"

[palette]
"w" = "base:oak_plank"

[[parts]]
name = "handle"
size = [1, 1, 1]
origin = [0, 0, 0]

[[parts]]
name = "flame"
size = [1, 2, 1]
origin = [0, 0, 0]
attach = { to = "handle", at = [0, 1, 0] }

[[parts]]
name = "flame"
size = [1, 2, 1]
origin = [0, 0, 0]
attach = { to = "handle", at = [0, 1, 0] }

[[layers]]
part = "handle"
y = 0
grid = """
w
"""

[[layers]]
part = "flame"
y = 0
grid = """
w
"""

[[layers]]
part = "flame"
y = 1
grid = """
w
"""
"#;

#[test]
fn a_child_pivot_lands_on_the_parent_local_position_its_attachment_names() -> TestResult {
    // The flame's origin is [2, 0, 2] and it attaches at the handle's [1, 10, 1],
    // so the flame's translation is (1, 10, 1) − (2, 0, 2) = (−1, 10, −1) and the
    // model's lowest art sits at (−1, 0, −1). Normalising adds (1, 0, 1), which
    // puts both of these voxels at (2, 10, 2).
    let volume = assembled(&torch(), &StateSelection::default())?;
    let pivot = volume.placed(&PartName::new("flame"), at(2, 0, 2));
    let socket = volume.placed(&PartName::new("handle"), at(1, 10, 1));

    assert_eq!(
        (pivot, socket),
        (Some(IVec3::new(2, 10, 2)), Some(IVec3::new(2, 10, 2))),
        "a child's origin lands exactly on the parent-local position its `at` names, which adding the origin instead of subtracting it would put at (5, 10, 5)"
    );
    Ok(())
}

#[test]
fn a_two_level_chain_accumulates_both_attachments_rather_than_only_one() -> TestResult {
    // arm sits at the pre-normalisation origin, bough at (4, 0, 0) and crown at
    // (4, 0, 0) + (0, 5, 0). No part reaches below zero on any axis, so
    // normalisation moves nothing and crown's own (0, 0, 0) is at (4, 5, 0).
    let volume = assembled(&chain(), &StateSelection::default())?;
    let root = volume.placed(&PartName::new("arm"), at(0, 0, 0));
    let leaf = volume.placed(&PartName::new("crown"), at(0, 0, 0));

    assert_eq!(
        (root, leaf),
        (Some(IVec3::ZERO), Some(IVec3::new(4, 5, 0))),
        "a grandchild carries both displacements: applying only one would answer (0, 5, 0) or (4, 0, 0)"
    );
    Ok(())
}

#[test]
fn an_attachment_naming_a_part_the_document_does_not_declare_is_refused_naming_both() -> TestResult
{
    let fault = refusal(&document(&[
        ("handle", ""),
        ("flame", &attached_to("wick")),
    ]))?;

    assert_eq!(
        (
            fault.part.as_deref(),
            unnamed(&fault, &["`wick`", "`flame`"]),
        ),
        (Some("flame"), all_named()),
        "the repair is either to declare `wick` or to correct the spelling, and both need the pair named; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_document_in_which_every_part_declares_an_attachment_is_refused_for_having_no_root()
-> TestResult {
    let fault = refusal(&document(&[
        ("arm", &attached_to("bough")),
        ("bough", &attached_to("crown")),
        ("crown", &attached_to("arm")),
    ]))?;

    assert_eq!(
        unnamed(&fault, &["no part is the root"]),
        all_named(),
        "parts form a tree rooted at the one part declaring no `attach`, and a document offering none has nothing to hang the rest off; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn two_parts_declaring_no_attachment_are_refused_naming_both() -> TestResult {
    let fault = refusal(&document(&[("arm", ""), ("bough", "")]))?;

    assert_eq!(
        unnamed(
            &fault,
            &["`arm`", "`bough`", "exactly one part is the root"],
        ),
        all_named(),
        "two candidate roots is two models in one file, and the author has to be told which two parts are competing; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn two_parts_attached_to_each_other_are_refused_naming_both_in_the_cycle() -> TestResult {
    let fault = refusal(&document(&[
        ("arm", &attached_to("bough")),
        ("bough", &attached_to("arm")),
    ]))?;

    assert_eq!(
        unnamed(&fault, &["`arm`", "`bough`", "cycle"]),
        all_named(),
        "a cycle is repaired by breaking one of its links, so the parts on it are named rather than merely counted; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_part_attached_to_itself_is_refused_as_its_own_parent() -> TestResult {
    let fault = refusal(&document(&[
        ("handle", ""),
        ("flame", &attached_to("flame")),
    ]))?;

    assert_eq!(
        (
            fault.part.as_deref(),
            unnamed(&fault, &["`flame`", "its own parent"]),
        ),
        (Some("flame"), all_named()),
        "a self-attachment is a mistyped parent rather than a cycle between two parts, and saying so is what sends the author to the right line; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn two_parts_declared_under_one_name_are_refused_naming_it() -> TestResult {
    let fault = refusal(REPEATED_PART_NAME)?;

    assert_eq!(
        (
            fault.part.as_deref(),
            unnamed(&fault, &["`flame`", "two parts"]),
        ),
        (Some("flame"), all_named()),
        "a repeated name makes every `attach` and every layer naming it ambiguous, so it is refused before either is read; cause was: {}",
        fault.cause
    );
    Ok(())
}
