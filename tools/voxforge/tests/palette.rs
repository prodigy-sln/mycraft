//! What a palette character means, and what a grid naming one nobody declared
//! is told.
//!
//! A palette is the only place a material key appears in a document, so it is
//! also the only place the namespaced id rule reaches the art. The rule itself
//! is reused rather than reimplemented — `mc_core::id::NamespacedId` already
//! produces the diagnostics this project's block loader documents, and a second
//! implementation would be a second opinion about what a namespaced id is.

mod common;

use std::path::Path;

use common::{
    FIXTURE_FILE, TestResult, all_named, directory_holding, loaded, refusal, shown, unnamed,
};
use tempfile::TempDir;
use voxforge::format::{FilledCell, Voxel};
use voxforge::material::load_materials;
use voxforge::name::MaterialKey;

/// The material every accepted fixture here paints with.
const MATERIAL: &str = "base:oak_plank";

/// The empty marker and one material — the palette a fixture declares when the
/// palette itself is not the thing under test.
const OAK_PALETTE: &str = r#""." = "empty"
"w" = "base:oak_plank""#;

/// A named part whose grid spells a character the palette does not declare.
const UNDECLARED_CHARACTER: &str = r#"schema = 1
name = "base:probe"
scale = 16
slice = "y"

[palette]
"." = "empty"
"w" = "base:oak_plank"

[[parts]]
name = "handle"
size = [4, 3, 2]
origin = [0, 0, 0]

[[layers]]
part = "handle"
y = 0
grid = """
....
..q.
"""
"#;

/// A document whose `[palette]` table declares nothing at all.
const EMPTY_PALETTE: &str = r#"schema = 1
name = "base:probe"
scale = 16
size = [4, 3, 2]
origin = [0, 0, 0]
slice = "y"

[palette]

[[layers]]
y = 0
grid = """
....
....
"""
"#;

/// A one-part model sliced on `y`, carrying `palette` and one `y = 0` layer of
/// `rows`.
fn model(palette: &str, rows: &[&str]) -> String {
    let grid = rows.join("\n");
    format!(
        r#"schema = 1
name = "base:probe"
scale = 16
size = [4, 3, 2]
origin = [0, 0, 0]
slice = "y"

[palette]
{palette}

[[layers]]
y = 0
grid = """
{grid}
"""
"#
    )
}

/// The cell at `x`, `y`, `z` made of the fixture material.
fn cell(x: u32, y: u32, z: u32) -> Result<FilledCell, Box<dyn std::error::Error>> {
    Ok(FilledCell {
        position: Voxel { x, y, z },
        material: MaterialKey::parse(MATERIAL)?,
    })
}

#[test]
fn a_grid_fills_every_mapped_cell_and_leaves_every_empty_marker_holding_nothing() -> TestResult {
    let document = loaded(&model(OAK_PALETTE, &["w.w.", ".w.w"]))?;
    let part = document
        .parts
        .first()
        .ok_or("the document declared no part at all")?;

    // Four filled characters in the fixture, at the four positions the y-slice
    // mapping sends them to. The empty positions are absent from this list,
    // which is what makes it an assertion about both halves of the palette.
    assert_eq!(
        part.filled_cells(),
        vec![
            cell(0, 0, 0)?,
            cell(1, 0, 1)?,
            cell(2, 0, 0)?,
            cell(3, 0, 1)?
        ],
        "every `w` is a voxel of that material and every `.` is no voxel at all"
    );
    Ok(())
}

#[test]
fn a_palette_key_of_more_than_one_character_is_refused_naming_the_key() -> TestResult {
    let fault = refusal(&model(
        r#""." = "empty"
"ww" = "base:oak_plank""#,
        &["....", "...."],
    ))?;

    assert_eq!(
        unnamed(&fault, &["`ww`", "one character"]),
        all_named(),
        "a grid is a character per cell, so a two-character key could never be spelled; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_grid_character_the_palette_does_not_declare_is_refused_at_its_position() -> TestResult {
    let fault = refusal(UNDECLARED_CHARACTER)?;

    assert_eq!(
        (
            fault.part.as_deref(),
            fault.layer.map(|found| found.declaration),
            unnamed(&fault, &["`q`", "row 1", "column 2", "palette"]),
        ),
        (Some("handle"), Some(0), all_named()),
        "an undeclared character is repaired by opening the file at that cell, so all four coordinates are named; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_palette_entry_no_grid_uses_is_reported_as_unused() -> TestResult {
    let document = loaded(&model(
        r#""." = "empty"
"w" = "base:oak_plank"
"h" = "base:iron""#,
        &["w...", "...."],
    ))?;

    // Exactly one of the three declared keys is absent from the grid text
    // above; the other two are both spelled in it.
    assert_eq!(
        document.unused_palette_keys(),
        vec![b'h'],
        "an entry nothing spells is a defect the inspector grades, not a reason to refuse the document"
    );
    Ok(())
}

#[test]
fn a_palette_declaring_no_entry_at_all_is_refused_naming_it() -> TestResult {
    let fault = refusal(EMPTY_PALETTE)?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["palette", "no entry"]),
        ),
        (Some("palette"), all_named()),
        "a palette with nothing in it can spell no grid, so it is refused rather than left to fail per character; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_palette_value_carrying_no_namespace_is_refused_naming_the_value() -> TestResult {
    let fault = refusal(&model(
        r#""." = "empty"
"w" = "oak_plank""#,
        &["w...", "...."],
    ))?;

    assert_eq!(
        unnamed(&fault, &["oak_plank", "no namespace"]),
        all_named(),
        "a material key is namespaced by the same rule a block name is, and the same diagnostic reaches the author; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_palette_naming_a_material_no_file_declares_is_refused_naming_the_directory() -> TestResult {
    let directory = TempDir::new()?;
    let materials = directory_holding(
        &directory,
        &[(
            "oak_plank.toml",
            "name  = \"base:oak_plank\"\ncolor = \"#8b5a2b\"\n",
        )],
    )?;
    let table = load_materials(&materials)?;
    let document = loaded(&model(
        r#""." = "empty"
"h" = "base:iron""#,
        &["h...", "...."],
    ))?;

    let fault = document
        .bind_materials(&table)
        .err()
        .ok_or("a palette naming an undeclared material must be refused")?;

    let searched = shown(&materials);
    assert_eq!(
        (
            fault.origin.as_path(),
            unnamed(&fault, &["base:iron", searched.as_str()]),
        ),
        (Path::new(FIXTURE_FILE), all_named()),
        "the author needs both the key that was not found and the directory that was searched for it; cause was: {}",
        fault.cause
    );
    Ok(())
}
