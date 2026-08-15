//! The shared material table: one material per file, read in file-name sorted
//! order.
//!
//! The sort is contract rather than tidiness. A name declared twice is refused
//! naming a first file and a second, and which is which is only well defined if
//! the directory is read in a fixed order — `read_dir` is the one genuinely
//! nondeterministic thing this tool touches, so the fixture below deliberately
//! writes its two files in the *opposite* order from the one the refusal must
//! report them in.

mod common;

use std::error::Error;
use std::path::Path;

use common::{Mention, TestResult, all_named, directory_holding, mentioned_in_order, unnamed};
use tempfile::TempDir;
use voxforge::fault::Fault;
use voxforge::material::{Srgb8, load_materials};
use voxforge::name::MaterialKey;

/// A flame: a colour and a self-illumination that is neither of the two fixed
/// points of the transfer function.
const FLAME: &str = r##"name     = "base:flame"
color    = "#ff9a3c"
emissive = 0.8
"##;

/// A plank: a colour and no `emissive` at all.
const PLANK: &str = r##"name  = "base:oak_plank"
color = "#8b5a2b"
"##;

/// A flame whose self-illumination is outside the fraction it has to be.
const OVERBRIGHT: &str = r##"name     = "base:flame"
color    = "#ff9a3c"
emissive = 1.5
"##;

/// A flame carrying a field nobody reads.
const GLOWING: &str = r##"name  = "base:flame"
color = "#ff9a3c"
glow  = 3
"##;

/// A plank whose colour is missing the `#` that makes it a colour.
const BARE_COLOUR: &str = r#"name  = "base:oak_plank"
color = "8b5a2b"
"#;

/// The refusal `directory` earns.
fn refused(directory: &Path) -> Result<Fault, Box<dyn Error>> {
    match load_materials(directory) {
        Ok(table) => Err(format!(
            "this materials directory must be refused, but loaded {} material(s)",
            table.len()
        )
        .into()),
        Err(fault) => Ok(fault),
    }
}

#[test]
fn a_material_file_resolves_its_key_to_the_colour_and_emissive_it_declares() -> TestResult {
    let directory = TempDir::new()?;
    let materials = directory_holding(&directory, &[("flame.toml", FLAME)])?;

    let table = load_materials(&materials)?;
    let flame = table
        .get(&MaterialKey::parse("base:flame")?)
        .ok_or("the flame material was not resolved at all")?;

    // #ff9a3c read as three pairs of hex digits, by hand from the fixture text.
    assert_eq!(
        flame.color,
        Srgb8 {
            red: 0xff,
            green: 0x9a,
            blue: 0x3c
        },
        "a declared colour reaches the table unchanged"
    );
    // The tolerance is above the error of rounding TOML's f64 to f32 (at most
    // 0.8 × 2⁻²⁴ ≈ 4.8e-8) and far below any difference this test must catch.
    let emissive = flame.emissive.fraction();
    assert!(
        (emissive - 0.8).abs() <= f32::EPSILON,
        "a declared emissive of 0.8 must resolve as 0.8, but resolved as {emissive}"
    );
    Ok(())
}

#[test]
fn a_material_file_omitting_emissive_resolves_to_none_at_all() -> TestResult {
    let directory = TempDir::new()?;
    let materials = directory_holding(&directory, &[("oak_plank.toml", PLANK)])?;

    let table = load_materials(&materials)?;
    let plank = table
        .get(&MaterialKey::parse("base:oak_plank")?)
        .ok_or("the plank material was not resolved at all")?;

    let emissive = plank.emissive.fraction();
    assert!(
        emissive.abs() <= f32::EPSILON,
        "a material that says nothing about emissive makes no light of its own, but reported {emissive}"
    );
    Ok(())
}

#[test]
fn a_colour_written_without_its_hash_is_refused_naming_the_value_and_the_form() -> TestResult {
    let directory = TempDir::new()?;
    let materials = directory_holding(&directory, &[("oak_plank.toml", BARE_COLOUR)])?;

    let fault = refused(&materials)?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["8b5a2b", "#rrggbb"]),
        ),
        (Some("color"), all_named()),
        "shorthand accepted is shorthand silently mis-parsed, so the accepted form is spelled out; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn an_emissive_above_one_is_refused_naming_the_value_and_the_range() -> TestResult {
    let directory = TempDir::new()?;
    let materials = directory_holding(&directory, &[("flame.toml", OVERBRIGHT)])?;

    let fault = refused(&materials)?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["is 1.5", "0.0", "1.0"]),
        ),
        (Some("emissive"), all_named()),
        "emissive is a fraction of self-illumination and not a light level, so its range is part of the refusal; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_material_file_carrying_an_unrecognised_field_is_refused_by_name() -> TestResult {
    let directory = TempDir::new()?;
    let materials = directory_holding(&directory, &[("flame.toml", GLOWING)])?;

    let fault = refused(&materials)?;

    assert_eq!(
        (fault.field.as_deref(), unnamed(&fault, &["glow"])),
        (Some("glow"), all_named()),
        "a field nobody reads is a typo the author cannot see any other way; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn one_name_declared_by_two_files_is_refused_naming_them_in_file_name_order() -> TestResult {
    let directory = TempDir::new()?;
    // Written zulu first, so that a refusal reporting them in creation order or
    // in whatever order the filesystem hands them back would name them the
    // wrong way round.
    let materials = directory_holding(&directory, &[("zulu.toml", FLAME), ("alpha.toml", FLAME)])?;

    let fault = refused(&materials)?;

    assert_eq!(
        (
            fault.origin.as_path(),
            mentioned_in_order(&fault, &["alpha.toml", "zulu.toml"]),
        ),
        (materials.as_path(), Mention::Ordered),
        "a duplicate spans two files, so the failure belongs to the directory and names both in the order it read them; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_materials_directory_holding_no_material_is_refused_naming_the_directory() -> TestResult {
    let directory = TempDir::new()?;
    let materials = directory_holding(&directory, &[])?;

    let fault = refused(&materials)?;

    assert_eq!(
        (
            fault.origin.as_path(),
            unnamed(&fault, &["declares no material"])
        ),
        (materials.as_path(), all_named()),
        "an empty table resolves nothing, so it is refused rather than left to fail per palette entry; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_materials_directory_that_does_not_exist_is_refused_naming_the_path() -> TestResult {
    let directory = TempDir::new()?;
    let absent = directory.path().join("no-such-directory");

    let fault = refused(&absent)?;

    assert_eq!(
        (fault.origin.as_path(), unnamed(&fault, &["does not exist"])),
        (absent.as_path(), all_named()),
        "a mistyped --materials is the likeliest way to reach this, and the path is what the author has to correct; cause was: {}",
        fault.cause
    );
    Ok(())
}
