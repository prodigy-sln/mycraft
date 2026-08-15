//! What a `.mcvox` document must declare, and what it is told when it does not.
//!
//! Everything here is decided before a single grid is read. That ordering is
//! the point of several of these fixtures rather than an implementation detail:
//! a part declaring 65 voxels on an axis is refused **without its layer art
//! needing to exist**, so the over-large fixtures below deliberately declare no
//! layer at all. A loader that read grids first would have to refuse them for
//! the wrong reason.
//!
//! The refusals are graded on what they *say*. The consumer of these messages
//! is an agent repairing its own file from the message alone, so a refusal that
//! does not name the axis, the value or the field is a defect even when it
//! correctly refuses.

mod common;

use std::path::Path;

use common::{FIXTURE_FILE, TestResult, all_named, document_file, loaded, refusal, unnamed};
use tempfile::TempDir;
use voxforge::format::{Extent, load_document};

/// The header a well-formed one-part door declares.
///
/// Sliced on `z`, so each layer is a front elevation: with an extent of
/// `[4, 3, 2]` that makes every layer 3 rows of 4 columns.
const DOOR_HEADER: &str = r#"schema = 1
name = "base:door_oak"
scale = 16
size = [4, 3, 2]
origin = [0, 0, 0]
slice = "z"
"#;

/// A palette naming the empty marker and one material.
const PALETTE: &str = r#"
[palette]
"." = "empty"
"w" = "base:oak_plank"
"#;

/// Everything below a door's header: a palette and the one layer at `z = 0`.
/// `z = 1` is left undeclared, which is a legitimate empty slab.
const DOOR_BODY: &str = r#"
[palette]
"." = "empty"
"w" = "base:oak_plank"

[[layers]]
z = 0
grid = """
wwww
wwww
wwww
"""
"#;

/// A torch in the explicit form, whose one part carries a field nobody reads.
const TORCH_WITH_STRAY_PART_FIELD: &str = r#"schema = 1
name = "base:torch"
scale = 16
slice = "y"

[palette]
"." = "empty"
"w" = "base:oak_plank"

[[parts]]
name = "handle"
size = [2, 3, 2]
origin = [0, 0, 0]
wobble = true

[[layers]]
part = "handle"
y = 0
grid = """
ww
ww
"""
"#;

/// The implicit single-part form: a top-level `size` and no `[[parts]]` table.
/// Sliced on `y`, so a `[8, 16, 3]` extent makes each layer 3 rows of 8.
const IMPLICIT_FORM: &str = r#"schema = 1
name = "base:door_oak"
scale = 16
size = [8, 16, 3]
origin = [0, 0, 0]
slice = "y"

[palette]
"." = "empty"
"w" = "base:oak_plank"

[[layers]]
y = 0
grid = """
wwwwwwww
........
........
"""
"#;

/// A second part table, for the document that declares both forms at once.
const PARTS_TABLE: &str = r#"
[[parts]]
name = "panel"
size = [4, 3, 2]
origin = [0, 0, 0]
"#;

/// A door document under `header`.
fn door(header: &str) -> String {
    format!("{header}{DOOR_BODY}")
}

/// The door's header with `line` written as `replacement` instead.
fn header_where(line: &str, replacement: &str) -> String {
    DOOR_HEADER.replace(line, replacement)
}

/// A document declaring `size` and no layer art whatsoever.
fn sized(size: &str) -> String {
    format!(
        r#"schema = 1
name = "base:door_oak"
scale = 16
size = {size}
origin = [0, 0, 0]
slice = "y"
{PALETTE}"#
    )
}

#[test]
fn a_document_declaring_every_required_field_loads_from_disk_reporting_its_name() -> TestResult {
    let directory = TempDir::new()?;
    let path = document_file(&directory, &door(DOOR_HEADER))?;

    let model = load_document(&path)?;

    assert_eq!(
        model.name.as_str(),
        "base:door_oak",
        "a document declaring a schema, a name, a scale, a slice, a palette and a layer is a model"
    );
    Ok(())
}

#[test]
fn a_document_declaring_a_newer_schema_is_refused_naming_it_and_the_supported_one() -> TestResult {
    let fault = refusal(&door(&header_where("schema = 1", "schema = 2")))?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["schema 2", "highest supported schema is 1"]),
        ),
        (Some("schema"), all_named()),
        "an unreadable schema must say which one was asked for and which is the newest understood; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_top_level_field_the_loader_does_not_recognise_is_refused_by_name() -> TestResult {
    let fault = refusal(&door(&format!("{DOOR_HEADER}hinge_side = \"left\"\n")))?;

    assert_eq!(
        (fault.field.as_deref(), unnamed(&fault, &["hinge_side"])),
        (Some("hinge_side"), all_named()),
        "a silently ignored typo is a debugging trap, so the field itself is named; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_document_omitting_schema_is_refused_naming_the_missing_field() -> TestResult {
    let fault = refusal(&door(&header_where("schema = 1\n", "")))?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["schema", "required"]),
        ),
        (Some("schema"), all_named()),
        "the field that was not declared is the one to name; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_document_that_is_not_well_formed_toml_is_refused_naming_the_file_and_line() -> TestResult {
    let fault = refusal("schema = 1\nname = \"base:door_oak\"\nscale = = 16\n")?;

    assert_eq!(
        (fault.origin.as_path(), unnamed(&fault, &["line 3"])),
        (Path::new(FIXTURE_FILE), all_named()),
        "a syntax error is repaired by opening the file at the line, so the line is part of the refusal; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn an_unrecognised_field_in_a_part_is_refused_naming_the_field_and_the_part() -> TestResult {
    let fault = refusal(TORCH_WITH_STRAY_PART_FIELD)?;

    assert_eq!(
        (
            fault.part.as_deref(),
            fault.field.as_deref(),
            unnamed(&fault, &["wobble"]),
        ),
        (Some("handle"), Some("wobble"), all_named()),
        "a part carrying a field nobody recognises is named as well as the field; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn an_unrecognised_field_in_a_layer_is_refused_naming_the_field_and_the_layer() -> TestResult {
    let fault = refusal(&format!(
        r#"{DOOR_HEADER}{PALETTE}
[[layers]]
z = 0
grid = """
wwww
wwww
wwww
"""

[[layers]]
z = 1
wobble = true
grid = """
wwww
wwww
wwww
"""
"#
    ))?;

    assert_eq!(
        (
            fault.layer.map(|layer| layer.declaration),
            fault.field.as_deref(),
            unnamed(&fault, &["wobble"]),
        ),
        (Some(1), Some("wobble"), all_named()),
        "the second declared layer is the one at fault, and a layer is identified by where it was declared; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_namespaced_name_is_reported_exactly_as_it_was_written() -> TestResult {
    let model = loaded(&door(DOOR_HEADER))?;

    assert_eq!(
        model.name.as_str(),
        "base:door_oak",
        "a name that satisfies the namespaced id rule is carried through unchanged"
    );
    Ok(())
}

#[test]
fn a_name_carrying_two_separators_is_refused_naming_the_extra_one() -> TestResult {
    let fault = refusal(&door(&header_where(
        r#"name = "base:door_oak""#,
        r#"name = "base:door:oak""#,
    )))?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(
                &fault,
                &["base:door:oak", "more than one namespace separator"],
            ),
        ),
        (Some("name"), all_named()),
        "the namespaced id rule is reused, so its own diagnostic is what reaches the author; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_name_carrying_no_namespace_is_refused_naming_what_is_missing() -> TestResult {
    let fault = refusal(&door(&header_where(
        r#"name = "base:door_oak""#,
        r#"name = "door_oak""#,
    )))?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["door_oak", "no namespace"]),
        ),
        (Some("name"), all_named()),
        "a name with no colon at all is refused for the namespace, not for the shape; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_top_level_size_with_no_parts_table_loads_as_one_part_of_that_extent() -> TestResult {
    let model = loaded(IMPLICIT_FORM)?;

    assert_eq!(
        (model.parts.len(), model.parts.first().map(|part| part.size),),
        (1, Some(Extent { x: 8, y: 16, z: 3 })),
        "the implicit form is one part, and its extent is the size the document declared"
    );
    Ok(())
}

#[test]
fn declaring_a_top_level_size_and_a_parts_table_is_refused_naming_both_forms() -> TestResult {
    let fault = refusal(&format!("{}{PARTS_TABLE}", door(DOOR_HEADER)))?;

    assert_eq!(
        unnamed(&fault, &["size", "parts", "never both"]),
        all_named(),
        "the two forms may not be mixed, and a refusal that named only one would leave the author guessing which to delete; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn declaring_neither_a_top_level_size_nor_a_parts_table_is_refused_naming_both_forms() -> TestResult
{
    let fault = refusal(&door(&header_where("size = [4, 3, 2]\n", "")))?;

    assert_eq!(
        unnamed(&fault, &["size", "parts", "neither"]),
        all_named(),
        "a document with no geometry at all is told both ways of declaring some; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_model_thirty_two_voxels_tall_at_scale_sixteen_is_two_blocks_tall() -> TestResult {
    let model = loaded(&format!(
        r#"schema = 1
name = "base:door_oak"
scale = 16
size = [2, 32, 2]
origin = [0, 0, 0]
slice = "y"
{PALETTE}
[[layers]]
y = 0
grid = """
ww
ww
"""
"#
    ))?;

    // 32 voxels over 16 voxels per block edge is 2 — arithmetic, never a figure
    // read off a run. The tolerance sits above the error of an exact binary
    // division (which is none) and far below the one-block granularity any
    // wrong answer would miss by.
    let height = model.height_in_blocks();
    assert!(
        (height - 2.0).abs() < 1e-9,
        "a 32-voxel model at scale 16 is 2 blocks tall, but was reported as {height}"
    );
    Ok(())
}

#[test]
fn a_scale_of_zero_is_refused_naming_the_field_and_the_value() -> TestResult {
    let fault = refusal(&door(&header_where("scale = 16", "scale = 0")))?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["at least 1", "is 0"]),
        ),
        (Some("scale"), all_named()),
        "a scale of zero divides nothing, and the refusal quotes the value back; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_negative_scale_is_refused_naming_the_field_and_the_value() -> TestResult {
    let fault = refusal(&door(&header_where("scale = 16", "scale = -16")))?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["at least 1", "is -16"]),
        ),
        (Some("scale"), all_named()),
        "a negative scale is quoted back as written, which a typed parse would have thrown away; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_declared_extent_of_sixty_five_is_refused_naming_the_axis_and_the_limit() -> TestResult {
    let fault = refusal(&sized("[65, 4, 4]"))?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["axis x", "is 65", "64"]),
        ),
        (Some("size"), all_named()),
        "the offending axis is what the author has to edit, so naming the value alone is not enough; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_declared_extent_of_zero_is_refused_naming_the_axis_and_the_value() -> TestResult {
    let fault = refusal(&sized("[4, 0, 4]"))?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["axis y", "is 0", "at least 1"]),
        ),
        (Some("size"), all_named()),
        "an extent of zero is a part with no voxels in it, and the axis is named; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_negative_declared_extent_is_refused_naming_the_axis_and_the_value() -> TestResult {
    let fault = refusal(&sized("[4, -1, 4]"))?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["axis y", "is -1", "at least 1"]),
        ),
        (Some("size"), all_named()),
        "minus one has to reach the author as minus one, which is why the declared size is not read as three unsigned integers; cause was: {}",
        fault.cause
    );
    Ok(())
}
