//! What `inspect` states as fact about a model, and what it grades as a defect.
//!
//! Two things are being pinned here that nothing else in this suite pins.
//!
//! **The bounding box is inclusive.** A `4³` model reports its far corner as
//! `(3, 3, 3)`, not `(4, 4, 4)`. Two conventions now coexist in this workspace —
//! `world-format.md`'s section planes are exclusive — so the one an author reads
//! off a report is worth a test rather than a comment.
//!
//! **A count is the second witness the assembled-cells path never had.** Phase 3
//! recorded that the model-wide translation is asserted by exactly one test,
//! because its own scenarios are about *placement* and read the placement map
//! rather than the art. So the solid cube below is deliberately built from two
//! parts, one of which sits wholly in negative `z` before the model is
//! normalised: an assembler that dropped its negative cells still reports a
//! plausible model, and only a count and a far corner can see that half of it
//! has gone.

mod common;

use std::error::Error;
use std::path::Path;

use common::{TestResult, all_named, at, document_file, inspected, solid_y_layers, unnamed};
use tempfile::TempDir;
use voxforge::fault::Fault;
use voxforge::inspect::{Bounds, Defect, ExitCode, MaterialCount, Report, inspect_document};
use voxforge::name::MaterialKey;
use voxforge::volume::StateSelection;

/// A model holding ten voxels of one material and six of another.
///
/// One part of `[4, 2, 2]`, filled to all sixteen cells so that the two counts
/// have to add up to the declared extent: `8 + 2` of `a` against `6` of `b`,
/// counted off the grid text rather than off a run. The two totals are unequal
/// on purpose, so a reading that swapped the materials cannot pass.
const MIXED_MATERIALS: &str = r#"schema = 1
name = "base:mixed"
scale = 16
size = [4, 2, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
"a" = "base:oak_plank"
"b" = "base:flame"

[[layers]]
y = 0
grid = """
aaaa
aaaa
"""

[[layers]]
y = 1
grid = """
aabb
bbbb
"""
"#;

/// A document declaring a schema this tool does not understand.
///
/// It names a model and declares a part, so a loader that reported statistics
/// about a document it had refused would have something to report — which is
/// exactly the answer the scenario forbids.
const UNREADABLE_SCHEMA: &str = r#"schema = 2
name = "base:future"
scale = 16
size = [2, 1, 1]
origin = [0, 0, 0]
slice = "y"

[palette]
"w" = "base:oak_plank"

[[layers]]
y = 0
grid = """
ww
"""
"#;

/// A palette declaring one entry the art uses and one it never spells.
const ONE_UNUSED_ENTRY: &str = r#"schema = 1
name = "base:spare"
scale = 16
size = [2, 1, 1]
origin = [0, 0, 0]
slice = "y"

[palette]
"w" = "base:oak_plank"
"u" = "base:flame"

[[layers]]
y = 0
grid = """
ww
"""
"#;

/// A palette every entry of which the art spells — including the empty marker.
///
/// The empty marker is the interesting half. A resolved grid holds nothing at
/// all where the marker was, so an implementation deciding "used" from the
/// assembled volume's materials cannot see that `.` was spelled, and reports it
/// as unused. That is a defect, and a non-zero exit, on a document with nothing
/// wrong with it.
///
/// The art is also the most *observable* thing this file declares: two
/// components, one floating voxel and no mirror symmetry on `x`. All three are
/// observations, so the exit code below has to stay 0 in their presence — which
/// is what makes this test the guard on the severity partition as well.
const EVERY_ENTRY_USED: &str = r#"schema = 1
name = "base:whole"
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

/// A solid `4 × 4 × 4` of one material, assembled from two parts.
///
/// `shell` pivots on its own `z = 2` and attaches at `core`'s `(0, 0, 0)`, so
/// its translation is `(0, 0, −2)` and its art occupies `z −2..−1` before the
/// model is normalised. `core` occupies `z 0..1`. The assembled model is
/// therefore `z −2..1`, four deep, and normalising adds `(0, 0, 2)` to
/// everything.
///
/// One part would have been simpler and would have graded nothing: with every
/// cell already non-negative the translation is the identity. Here an assembler
/// that discarded its negative cells reports 32 voxels in a `4 × 4 × 2` box,
/// both of which this file's first test reads.
fn solid_cube() -> String {
    format!(
        r#"schema = 1
name = "base:cube"
scale = 16
slice = "y"

[palette]
"w" = "base:oak_plank"

[[parts]]
name = "core"
size = [4, 4, 2]
origin = [0, 0, 0]

[[parts]]
name = "shell"
size = [4, 4, 2]
origin = [0, 0, 2]
attach = {{ to = "core", at = [0, 0, 0] }}
{core}{shell}"#,
        core = solid_y_layers("core", (4, 4, 2), 'w'),
        shell = solid_y_layers("shell", (4, 4, 2), 'w'),
    )
}

/// The count of `voxels` voxels of the material `key` names.
///
/// # Errors
///
/// Returns an error when `key` is not a namespaced material key, which would
/// otherwise arrive as a mismatch about the wrong thing entirely.
fn count(key: &str, voxels: usize) -> Result<MaterialCount, Box<dyn Error>> {
    Ok(MaterialCount {
        material: MaterialKey::parse(key)?,
        voxels,
    })
}

/// The refusal inspecting the document at `path` earns.
///
/// # Errors
///
/// Returns an error when a report came back instead. The message names the
/// facts that were stated, because "reported the failure and nothing about the
/// model" is a claim about what was *not* said, and a bare "expected Err" would
/// leave a reader unable to see what leaked.
fn inspection_refusal(path: &Path) -> Result<Fault, Box<dyn Error>> {
    match inspect_document(path, &StateSelection::default()) {
        Ok(report) => Err(unwanted(&report).into()),
        Err(fault) => Ok(fault),
    }
}

/// What a report about an unreadable document wrongly said.
fn unwanted(report: &Report) -> String {
    format!(
        "this document must be refused, but a report came back stating {filled} filled voxel(s), bounds {bounds:?}, {materials} material(s) and {defects} defect(s)",
        filled = report.stats().filled,
        bounds = report.stats().bounds,
        materials = report.stats().materials.len(),
        defects = report.defects().len()
    )
}

#[test]
fn a_solid_four_voxel_cube_reports_sixty_four_voxels_and_a_far_corner_of_three() -> TestResult {
    // Both numbers come off the fixture text: two solid 4 × 4 × 2 parts that do
    // not overlap are 64 voxels, and a box four voxels deep on every axis with
    // its near corner at the origin has its far corner at 3 when the corner is
    // inclusive. An exclusive maximum would answer (4, 4, 4) for the same model.
    let report = inspected(&solid_cube())?;

    assert_eq!(
        (report.stats().filled, report.stats().bounds),
        (
            64,
            Bounds::Spanning {
                lowest: at(0, 0, 0),
                highest: at(3, 3, 3),
            }
        ),
        "the far corner is a voxel the model has, not the first one past it — and every voxel that went in is still there"
    );
    Ok(())
}

#[test]
fn a_model_of_two_materials_reports_how_many_voxels_each_one_fills() -> TestResult {
    // Eight `a` on the lower layer and two more on the upper is 10; the upper
    // layer's remaining six cells are `b`. Ascending by key puts `base:flame`
    // before `base:oak_plank`, so the order is part of the expectation rather
    // than something a reader has to guess.
    let report = inspected(MIXED_MATERIALS)?;

    assert_eq!(
        report.stats().materials,
        vec![count("base:flame", 6)?, count("base:oak_plank", 10)?],
        "a per-material count is what tells an author which of two materials they used more of, so it has to name both and it has to be in a declared order"
    );
    Ok(())
}

#[test]
fn a_document_that_cannot_be_loaded_reports_the_refusal_and_no_statistics() -> TestResult {
    let directory = TempDir::new()?;
    let path = document_file(&directory, UNREADABLE_SCHEMA)?;

    let fault = inspection_refusal(&path)?;

    assert_eq!(
        (
            fault.origin.as_path(),
            unnamed(&fault, &["schema 2", "highest supported schema is 1"]),
        ),
        (path.as_path(), all_named()),
        "the refusal has to survive the inspector rather than being turned into a report of zeroes about a model nobody has; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_palette_entry_no_grid_spells_is_a_defect_and_sets_a_non_zero_exit() -> TestResult {
    let report = inspected(ONE_UNUSED_ENTRY)?;

    assert_eq!(
        (report.defects(), report.exit_code()),
        (
            [Defect::UnusedPaletteEntry { key: b'u' }].as_slice(),
            ExitCode::Defective
        ),
        "`w` is spelled and `u` is not, so exactly one entry is at fault and the exit has to say so"
    );
    Ok(())
}

#[test]
fn a_palette_every_grid_spells_carries_no_defect_and_exits_zero() -> TestResult {
    let expected: &[Defect] = &[];
    let report = inspected(EVERY_ENTRY_USED)?;

    assert_eq!(
        (report.defects(), report.exit_code()),
        (expected, ExitCode::Success),
        "the empty marker is spelled by the art as much as the material is, and the model's floating voxel and broken symmetry are observations, which never reach the exit code"
    );
    Ok(())
}
