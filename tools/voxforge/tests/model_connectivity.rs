//! What holds together, what floats, and what mirrors.
//!
//! This file is the reason `inspect` exists. The preview loop is good at
//! proportion and orientation and unreliable for structure — measured, on this
//! spec's own reference model, where a pair of armrests cantilevered on nothing
//! survived ten rendered views and a contact sheet and was caught only by
//! reading the document by hand. A part with something behind it looks
//! supported from every angle a camera can take. Face connectivity is decidable
//! where a picture is not, and nothing else in this tool reports it.
//!
//! Three things the fixtures below are built to separate, because each is a way
//! of being right for the wrong reason:
//!
//! - **6-connected, not 18 or 26.** The two-group fixture touches at an *edge*,
//!   so an implementation that counted diagonal neighbours reports one
//!   component where the spec asks for two.
//! - **Every axis, not two of them.** The single-component fixture is a chain
//!   that turns once on `x`, once on `y` and once on `z`, so a neighbour set
//!   missing any one direction breaks it into pieces.
//! - **The assembled model, not the parts.** The torch is two parts and one
//!   component; a per-part answer reports two, and would be wrong about every
//!   model that has parts at all.
//!
//! The gapped row and the closed row are one voxel apart on purpose, in the
//! shape this suite already uses for the 64/65 span pair: "some voxel floats"
//! and "none does" are each other's only positive control, and a routine
//! answering either one unconditionally passes exactly one of them.

mod common;

use common::{TestResult, at, inspected, torch};
use voxforge::format::Axis;
use voxforge::inspect::{Component, ExitCode, Floating, Observation, Report, SymmetryVerdict};

/// Two groups of unequal size touching only along an edge.
///
/// `(1, 0, 0)` and `(2, 1, 0)` differ on two axes at once, so they share an
/// edge and no face. Nothing else in either group comes within a face of the
/// other, which makes this two components under a 6-connected reading and one
/// under an 18- or 26-connected one. The sizes are 3 and 2 rather than equal so
/// that a report which found the right *number* of groups but attributed the
/// voxels to the wrong one still fails.
const EDGE_TOUCH: &str = r#"schema = 1
name = "base:edge_touch"
scale = 16
size = [3, 2, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
"w" = "base:oak_plank"
"." = "empty"

[[layers]]
y = 0
grid = """
ww.
w..
"""

[[layers]]
y = 1
grid = """
..w
..w
"""
"#;

/// Four voxels in a chain that turns on all three axes.
///
/// `(0,0,0)–(1,0,0)` joins on `x`, `(1,0,0)–(1,1,0)` on `y` and
/// `(1,1,0)–(1,1,1)` on `z`, each by exactly one shared face. Drop any one of
/// the six face directions from the neighbour set and this model falls apart.
const THREE_AXIS_CHAIN: &str = r#"schema = 1
name = "base:chain"
scale = 16
size = [2, 2, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
"w" = "base:oak_plank"
"." = "empty"

[[layers]]
y = 0
grid = """
ww
..
"""

[[layers]]
y = 1
grid = """
.w
.w
"""
"#;

/// A joined pair and one voxel clear of everything.
const GAPPED_ROW: &str = r#"schema = 1
name = "base:gapped"
scale = 16
size = [4, 1, 1]
origin = [0, 0, 0]
slice = "y"

[palette]
"w" = "base:oak_plank"
"." = "empty"

[[layers]]
y = 0
grid = """
ww.w
"""
"#;

/// The gapped row with its gap closed, and nothing else changed.
const CLOSED_ROW: &str = r#"schema = 1
name = "base:closed"
scale = 16
size = [4, 1, 1]
origin = [0, 0, 0]
slice = "y"

[palette]
"w" = "base:oak_plank"

[[layers]]
y = 0
grid = """
wwww
"""
"#;

/// A joined pair and two voxels clear of everything, at different distances.
const TWO_DETACHED: &str = r#"schema = 1
name = "base:scattered"
scale = 16
size = [6, 1, 1]
origin = [0, 0, 0]
slice = "y"

[palette]
"w" = "base:oak_plank"
"." = "empty"

[[layers]]
y = 0
grid = """
ww.w.w
"""
"#;

/// A model that mirrors about its `x` midplane and about neither other axis.
///
/// Five voxels: a three-wide bar on `x`, one voxel behind its centre and one
/// above it. The bar makes it symmetric on `x`; the voxel behind breaks `z` and
/// the voxel above breaks `y`, so a check that mirrored the wrong axis cannot
/// answer "symmetric" by accident.
const MIRRORED: &str = r#"schema = 1
name = "base:mirrored"
scale = 16
size = [3, 2, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
"w" = "base:oak_plank"
"." = "empty"

[[layers]]
y = 0
grid = """
www
.w.
"""

[[layers]]
y = 1
grid = """
.w.
...
"""
"#;

/// The mirrored model with its `+x` arm removed, and nothing else changed.
///
/// Removing the arm also narrows the model, so the midplane moves with it: this
/// is asymmetric about the box it actually occupies, not merely about the one
/// it used to.
const LOPSIDED: &str = r#"schema = 1
name = "base:lopsided"
scale = 16
size = [3, 2, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
"w" = "base:oak_plank"
"." = "empty"

[[layers]]
y = 0
grid = """
ww.
.w.
"""

[[layers]]
y = 1
grid = """
.w.
...
"""
"#;

/// The mirrored model with its `+x` arm repainted, and nothing else changed.
const MISMATCHED_ARMS: &str = r#"schema = 1
name = "base:mismatched"
scale = 16
size = [3, 2, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
"w" = "base:oak_plank"
"f" = "base:flame"
"." = "empty"

[[layers]]
y = 0
grid = """
wwf
.w.
"""

[[layers]]
y = 1
grid = """
.w.
...
"""
"#;

/// What a report said about one observation, when asked for exactly one.
///
/// An enumerated verdict rather than an `Option`: a report that never observed
/// the thing, and one that observed it twice so that any single reading of it
/// is arbitrary, are different failures — and neither may be able to read the
/// same as a verdict.
#[derive(Debug, PartialEq, Eq)]
enum Reported<T> {
    /// Observed once, saying this.
    Once(T),
    /// Not observed at all.
    Never,
    /// Observed more than once.
    Repeatedly,
}

impl<T> Reported<T> {
    /// The same verdict, reading `change` out of what was observed.
    fn map<U>(self, change: impl FnOnce(T) -> U) -> Reported<U> {
        match self {
            Self::Once(observed) => Reported::Once(change(observed)),
            Self::Never => Reported::Never,
            Self::Repeatedly => Reported::Repeatedly,
        }
    }
}

/// One verdict, from the first two matches a search found.
fn exactly_one<T>(first: Option<T>, second: Option<T>) -> Reported<T> {
    match (first, second) {
        (Some(only), None) => Reported::Once(only),
        (Some(_), Some(_)) => Reported::Repeatedly,
        (None, _) => Reported::Never,
    }
}

/// What `report` observed about how the model holds together.
fn connectivity_of(report: &Report) -> Reported<(Vec<Component>, Floating)> {
    let mut found = report
        .observations()
        .iter()
        .filter_map(|observation| match observation {
            Observation::Connectivity {
                components,
                floating,
            } => Some((components.clone(), floating.clone())),
            Observation::Symmetry { .. } => None,
        });
    exactly_one(found.next(), found.next())
}

/// Every face-connected group `report` observed.
fn components_of(report: &Report) -> Reported<Vec<Component>> {
    connectivity_of(report).map(|(components, _)| components)
}

/// Which voxels `report` observed to share a face with nothing.
fn floating_of(report: &Report) -> Reported<Floating> {
    connectivity_of(report).map(|(_, floating)| floating)
}

/// What `report` observed about mirroring across `axis`.
fn symmetry_on(report: &Report, axis: Axis) -> Reported<SymmetryVerdict> {
    let mut found = report
        .observations()
        .iter()
        .filter_map(|observation| match observation {
            Observation::Symmetry {
                axis: about,
                verdict,
            } if *about == axis => Some(*verdict),
            _ => None,
        });
    exactly_one(found.next(), found.next())
}

#[test]
fn two_groups_sharing_no_face_are_reported_as_two_components_without_failing() -> TestResult {
    // Counted off the grid text: three voxels in the lower group, two in the
    // upper, and the lowest voxel of each in ascending x, y, z order. Exit 0 is
    // half the scenario — a model held together only at an edge is a fact worth
    // reporting and not, on this tool's own evidence, a fault.
    let report = inspected(EDGE_TOUCH)?;

    assert_eq!(
        (components_of(&report), report.exit_code()),
        (
            Reported::Once(vec![
                Component {
                    voxels: 3,
                    lowest: at(0, 0, 0),
                },
                Component {
                    voxels: 2,
                    lowest: at(2, 1, 0),
                },
            ]),
            ExitCode::Success
        ),
        "these two groups touch along an edge and never across a face, so a 6-connected reading finds two — and finding two is an observation, not a defect"
    );
    Ok(())
}

#[test]
fn a_model_joined_across_every_axis_is_reported_as_one_component() -> TestResult {
    let report = inspected(THREE_AXIS_CHAIN)?;

    assert_eq!(
        components_of(&report),
        Reported::Once(vec![Component {
            voxels: 4,
            lowest: at(0, 0, 0),
        }]),
        "the chain joins on x, then y, then z, so one component is the answer only if all six face directions are neighbours"
    );
    Ok(())
}

#[test]
fn a_voxel_touching_no_other_is_reported_by_position_and_as_a_group_of_one() -> TestResult {
    // The row is `ww.w`: the pair at x 0 and 1 is one group, and the voxel at
    // x 3 is clear of everything. The spec's own reasoning is that the two
    // reports can never disagree — a voxel touching nothing *is* a component of
    // size 1 — so both are read here rather than only the one named "floating".
    let report = inspected(GAPPED_ROW)?;

    assert_eq!(
        (floating_of(&report), components_of(&report)),
        (
            Reported::Once(Floating::Detached(vec![at(3, 0, 0)])),
            Reported::Once(vec![
                Component {
                    voxels: 2,
                    lowest: at(0, 0, 0),
                },
                Component {
                    voxels: 1,
                    lowest: at(3, 0, 0),
                },
            ])
        ),
        "an author needs to know where the loose voxel is, not merely that one exists"
    );
    Ok(())
}

#[test]
fn a_model_whose_every_voxel_has_a_neighbour_reports_nothing_floating() -> TestResult {
    let report = inspected(CLOSED_ROW)?;

    assert_eq!(
        floating_of(&report),
        Reported::Once(Floating::NoneDetached),
        "this is the gapped row with its gap closed, so the verdict has to move with the one voxel that changed"
    );
    Ok(())
}

#[test]
fn a_two_part_torch_correctly_attached_is_reported_as_one_component() -> TestResult {
    // 40 voxels of handle and 96 of flame, from the two declared extents, and
    // they do not overlap: the handle occupies y 0..9 and the flame y 10..15,
    // meeting across the plane between. The flame is also the part whose art
    // sits in negative x and z before the model is normalised, so a count of
    // 136 in one group says the translation kept every voxel as well.
    let report = inspected(&torch())?;

    assert_eq!(
        components_of(&report),
        Reported::Once(vec![Component {
            voxels: 136,
            lowest: at(0, 10, 0),
        }]),
        "connectivity is computed on the assembled model: two parts that meet across a face are one thing, and a per-part answer would call every jointed model broken"
    );
    Ok(())
}

#[test]
fn a_model_mirroring_about_its_x_midplane_is_reported_as_symmetric_on_x() -> TestResult {
    let report = inspected(MIRRORED)?;

    assert_eq!(
        symmetry_on(&report, Axis::X),
        Reported::Once(SymmetryVerdict::Mirrored),
        "the bar mirrors about x while the voxel above and the one behind break y and z, so answering `symmetric` here means the x midplane and no other"
    );
    Ok(())
}

#[test]
fn a_model_that_does_not_mirror_about_its_x_midplane_is_reported_as_not_symmetric() -> TestResult {
    let report = inspected(LOPSIDED)?;

    assert_eq!(
        symmetry_on(&report, Axis::X),
        Reported::Once(SymmetryVerdict::NotMirrored),
        "one arm shorter than the other is the whole difference from the mirrored fixture, and it is the difference the verdict has to see"
    );
    Ok(())
}

/// No scenario states this. FR-6.3-S3 declares exactly one floating voxel, so a
/// report that found the *first* loose voxel and stopped — a `find` where a
/// `filter` belonged — satisfies it, and the order of a list with one element
/// in it is not an order.
///
/// Both matter here specifically. The defect this whole file exists for was a
/// **pair** of armrests attached by one face each; a report that named one of
/// them would have been read as "one thing to fix" and left the other in place.
#[test]
fn every_voxel_touching_no_other_is_reported_in_ascending_order() -> TestResult {
    let report = inspected(TWO_DETACHED)?;

    assert_eq!(
        floating_of(&report),
        Reported::Once(Floating::Detached(vec![at(3, 0, 0), at(5, 0, 0)])),
        "two loose voxels are two findings, and a list whose order nothing declares is a list an author cannot diff against the next run"
    );
    Ok(())
}

/// No scenario states this either. The spec says "mirror-symmetric" and leaves
/// open whether a model whose *shape* mirrors while its materials do not is
/// symmetric. This file answers: it is not — a chair with an oak arm and an iron
/// arm is not a mirrored chair — and that answer is a design decision, so it is
/// pinned somewhere a reader can find it rather than left to whoever writes the
/// mirror loop.
///
/// It is the only test in this suite that can see a material-blind comparison:
/// every other symmetry fixture is one material throughout.
#[test]
fn a_model_whose_shape_mirrors_but_whose_materials_do_not_is_not_symmetric() -> TestResult {
    let report = inspected(MISMATCHED_ARMS)?;

    assert_eq!(
        symmetry_on(&report, Axis::X),
        Reported::Once(SymmetryVerdict::NotMirrored),
        "the two arms occupy mirrored positions and are made of different things, which is a difference a report about symmetry ought to have seen"
    );
    Ok(())
}
