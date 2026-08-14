//! Content roots holding HUD declarations, written as real files.
//!
//! The thing under test is precisely the reading of a directory — what is picked
//! up, in what order, and what happens when there is nothing to read — so every
//! fixture here writes real files into a temporary directory. A mocked
//! filesystem would assert nothing about any of that.
//!
//! **Creation order is a fixture constraint no assertion can enforce.**
//! [`hud_content_root`] writes its files in the order it is given them, and the
//! tests whose scenario depends on that say so at the call site. A fixture built
//! in file-name order cannot falsify a claim about file-name order.
//!
//! Origins are compared by the *name* of the file or directory they point at,
//! never by a whole path — a path renders with OS-specific separators and an
//! assertion on one would be a Windows-only or Unix-only test.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use mc_core::hud::{HudLayout, HudLoadError};
use mc_world::content::TomlFileHudSource;
use tempfile::TempDir;

/// The subdirectory of a content root that HUD declarations live in.
pub const HUD_DIRECTORY: &str = "hud";

/// The final component of the root [`root_with_a_file_named_hud`] builds.
///
/// Distinctive on purpose: it is the needle a refusal that genuinely names the
/// path it could not read has to contain, and no message that merely says the
/// HUD could not be loaded can contain it by accident.
pub const ROOT_WITH_A_FILE_NAMED_HUD: &str = "root-with-a-file-named-hud";

/// The final component of the root [`root_without_a_hud_directory`] builds.
pub const ROOT_WITHOUT_A_HUD_DIRECTORY: &str = "root-without-a-hud-directory";

/// The final component of the root [`root_with_an_empty_hud_directory`] builds.
pub const ROOT_WITH_AN_EMPTY_HUD_DIRECTORY: &str = "root-with-an-empty-hud-directory";

/// The text of a well-formed HUD declaration file naming `name` — a centred
/// white fill, the smallest declaration the spec's table accepts.
///
/// Every field the model requires and none it does not, so a root built from
/// these fails only for the reason a fixture deliberately introduced.
#[must_use]
pub fn hud_file(name: &str) -> String {
    format!(
        "name = \"{name}\"\nanchor = \"center\"\nsize = [9, 1]\ndraw = \"fill\"\n\
         color = \"#FFFFFFFF\"\n"
    )
}

/// The element name this suite's declaration for `stem.toml` states.
#[must_use]
pub fn declared_by(stem: &str) -> String {
    format!("base:{stem}")
}

/// A content root inside `directory` whose `hud/` holds `declarations`, written
/// **in the order given**, and holding nothing else whatsoever.
///
/// # Errors
///
/// Returns an error if the directory or any file cannot be written.
pub fn hud_content_root(
    directory: &TempDir,
    declarations: &[(&str, String)],
) -> Result<PathBuf, Box<dyn Error>> {
    let root = directory.path().to_owned();
    let declared = root.join(HUD_DIRECTORY);
    fs::create_dir_all(&declared)?;
    for (file_name, body) in declarations {
        fs::write(declared.join(file_name), body)?;
    }
    Ok(root)
}

/// A content root that exists, is readable, and holds no `hud/` directory at
/// all.
///
/// It declares a block, so the root is a real content root rather than an empty
/// directory — a HUD that loaded only because there was nothing anywhere would
/// be a different fact from the one the scenario states.
///
/// # Errors
///
/// Returns an error if the directory or the file cannot be written.
pub fn root_without_a_hud_directory(directory: &TempDir) -> Result<PathBuf, Box<dyn Error>> {
    let root = directory.path().join(ROOT_WITHOUT_A_HUD_DIRECTORY);
    let blocks = root.join("blocks");
    fs::create_dir_all(&blocks)?;
    fs::write(
        blocks.join("stone.toml"),
        "name = \"base:stone\"\ntexture = \"base:stone\"\nsolid = true\n",
    )?;
    Ok(root)
}

/// A content root whose `hud/` directory exists and holds no file at all.
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
pub fn root_with_an_empty_hud_directory(directory: &TempDir) -> Result<PathBuf, Box<dyn Error>> {
    let root = directory.path().join(ROOT_WITH_AN_EMPTY_HUD_DIRECTORY);
    fs::create_dir_all(root.join(HUD_DIRECTORY))?;
    Ok(root)
}

/// A content root where `hud` is a **regular file** rather than a directory.
///
/// # Errors
///
/// Returns an error if the directory or the file cannot be written.
pub fn root_with_a_file_named_hud(directory: &TempDir) -> Result<PathBuf, Box<dyn Error>> {
    let root = directory.path().join(ROOT_WITH_A_FILE_NAMED_HUD);
    fs::create_dir_all(&root)?;
    fs::write(
        root.join(HUD_DIRECTORY),
        "this is a file, not a directory\n",
    )?;
    Ok(root)
}

/// Every element name the HUD declarations under `root` register, in the order
/// the layout holds them.
///
/// A fresh source is constructed per call, so loading the same root twice cannot
/// be satisfied by anything one source object remembered.
///
/// # Errors
///
/// Returns the refusal's own message if the root does not load. A scenario about
/// what a root registers has learned nothing from a root that was refused.
pub fn registered_names(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    match HudLayout::load(&TomlFileHudSource::new(root)) {
        Ok(layout) => Ok(layout
            .elements()
            .iter()
            .map(|element| element.name.as_str().to_owned())
            .collect()),
        Err(error) => Err(format!("this content root must load, but was refused: {error}").into()),
    }
}

/// The refusal loading the HUD declarations under `root` produced.
///
/// # Errors
///
/// Fails if the root loaded, because an assertion about a refusal that never
/// happened is vacuous.
pub fn refusal(root: &Path) -> Result<HudLoadError, Box<dyn Error>> {
    match HudLayout::load(&TomlFileHudSource::new(root)) {
        Ok(layout) => Err(format!(
            "this content root must not load, or the assertion below is vacuous, but it \
             registered {} element(s)",
            layout.elements().len()
        )
        .into()),
        Err(error) => Ok(error),
    }
}
