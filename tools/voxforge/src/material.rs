//! The shared material table: one material per file, mirroring
//! `content/<mod>/blocks/`.
//!
//! The directory is read in **file-name sorted order**, and that sort is
//! contract rather than tidiness: a name declared twice is refused naming the
//! first file and the second, which is only well defined if the order is fixed.
//! `read_dir` is the one genuinely nondeterministic thing this tool touches.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

use crate::fault::{Fault, Origin};
use crate::format::dto::{MaterialDto, from_text};
use crate::name::MaterialKey;

/// A colour as a material declares it, `#rrggbb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Srgb8 {
    /// The red channel.
    pub red: u8,
    /// The green channel.
    pub green: u8,
    /// The blue channel.
    pub blue: u8,
}

/// How much light a material makes of its own, as a fraction from 0.0 to 1.0.
///
/// A fraction and not a level: there is no lighting model in this project at
/// all, so a 0–15 light level would be inventing an engine decision inside an
/// asset format.
#[derive(Debug, Clone, Copy)]
pub struct Emissive(f32);

impl Emissive {
    /// No self-illumination — what a material declaring no `emissive` means.
    pub const NONE: Self = Self(0.0);

    /// `fraction`, if it lies between 0.0 and 1.0 inclusive.
    #[must_use]
    pub fn new(fraction: f32) -> Option<Self> {
        (0.0..=1.0).contains(&fraction).then_some(Self(fraction))
    }

    /// The fraction, as declared.
    #[must_use]
    pub fn fraction(self) -> f32 {
        self.0
    }
}

/// What one material declares.
#[derive(Debug, Clone, Copy)]
pub struct Material {
    /// The material's colour.
    pub color: Srgb8,
    /// How much light it makes of its own.
    pub emissive: Emissive,
}

/// Every material declared under one directory.
#[derive(Debug, Clone)]
pub struct MaterialTable {
    directory: PathBuf,
    materials: BTreeMap<MaterialKey, Material>,
}

impl MaterialTable {
    /// A table holding `materials`, read from `directory`.
    pub fn new(directory: impl Into<PathBuf>, materials: BTreeMap<MaterialKey, Material>) -> Self {
        Self {
            directory: directory.into(),
            materials,
        }
    }

    /// The directory these materials were read from.
    ///
    /// Carried by the table because a palette naming a material nothing declares
    /// is refused naming the directory that was searched, and the model knows
    /// nothing about where materials came from.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// What `key` resolves to, or `None` where no file declares it.
    #[must_use]
    pub fn get(&self, key: &MaterialKey) -> Option<&Material> {
        self.materials.get(key)
    }

    /// How many materials the table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Whether the table holds no material at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

/// Every material declared under `directory`, read in file-name sorted order.
///
/// # Errors
///
/// Returns a [`Fault`] naming `directory` if it does not exist or declares no
/// material at all, or naming the file and field at fault if a declaration is
/// not acceptable.
pub fn load_materials(directory: &Path) -> Result<MaterialTable, Fault> {
    let origin = Origin::new(directory);
    let mut declared: BTreeMap<MaterialKey, Material> = BTreeMap::new();
    let mut sources: BTreeMap<MaterialKey, String> = BTreeMap::new();

    for file in declaration_files(directory, &origin)? {
        let (key, material) = read_declaration(&file)?;
        let spelling = file_name(&file);
        if let Some(first) = sources.get(&key) {
            return Err(Fault::about(
                origin.clone(),
                format!(
                    "the material `{key}` is declared twice: first by {first}, then by {spelling}",
                    key = key.as_str()
                ),
            ));
        }
        sources.insert(key.clone(), spelling);
        declared.insert(key, material);
    }

    if declared.is_empty() {
        return Err(Fault::about(
            origin,
            format!(
                "{} declares no material at all, so no palette entry could resolve",
                directory.display()
            ),
        ));
    }
    Ok(MaterialTable::new(directory, declared))
}

/// Every `.toml` file under `directory`, in file-name sorted order.
///
/// The sort is contract, not tidiness: a duplicate name is refused naming a
/// first file and a second, and which is which is only well defined under a
/// fixed order. `read_dir` promises none.
fn declaration_files(directory: &Path, origin: &Origin) -> Result<Vec<PathBuf>, Fault> {
    let entries = fs::read_dir(directory).map_err(|cause| {
        Fault::about(
            origin.clone(),
            format!(
                "{path} does not exist, or cannot be read as a materials directory: {cause}",
                path = directory.display()
            ),
        )
    })?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|kind| kind == "toml"))
        .collect();
    files.sort();
    Ok(files)
}

/// The key and material one file declares.
fn read_declaration(file: &Path) -> Result<(MaterialKey, Material), Fault> {
    let origin = Origin::new(file);
    let text = fs::read_to_string(file)
        .map_err(|cause| Fault::about(origin.clone(), cause.to_string()))?;
    let declared: MaterialDto = from_text(&text, &origin)?;
    let key = read_key(&declared, &origin)?;
    Ok((
        key,
        Material {
            color: read_color(&declared, &origin)?,
            emissive: read_emissive(&declared, &origin)?,
        },
    ))
}

/// The namespaced key a material file declares.
fn read_key(declared: &MaterialDto, origin: &Origin) -> Result<MaterialKey, Fault> {
    let name = declared
        .name
        .as_ref()
        .and_then(Value::as_str)
        .ok_or_else(|| {
            Fault::about(
                origin.clone(),
                "a material declares a namespaced `name`, written `namespace:path`",
            )
            .in_field("name")
        })?;
    MaterialKey::parse(name)
        .map_err(|cause| Fault::about(origin.clone(), cause.to_string()).in_field("name"))
}

/// The colour a material file declares.
///
/// `#rrggbb` and nothing else. Shorthand accepted is shorthand silently
/// mis-parsed, which is the HUD's own stated reason for the same rule.
fn read_color(declared: &MaterialDto, origin: &Origin) -> Result<Srgb8, Fault> {
    let refusal = |cause: String| Fault::about(origin.clone(), cause).in_field("color");
    let text = declared
        .color
        .as_ref()
        .and_then(Value::as_str)
        .ok_or_else(|| refusal("a material declares a `color`, written `#rrggbb`".to_owned()))?;
    let malformed = || {
        refusal(format!(
            "the colour `{text}` is not written `#rrggbb`, which is the only form a colour takes"
        ))
    };
    let digits = text.strip_prefix('#').ok_or_else(malformed)?;
    let channels: Vec<u8> = (0..3)
        .filter_map(|channel| digits.get(channel * 2..channel * 2 + 2))
        .filter_map(|pair| u8::from_str_radix(pair, 16).ok())
        .collect();
    match (digits.len(), channels.as_slice()) {
        (6, [red, green, blue]) => Ok(Srgb8 {
            red: *red,
            green: *green,
            blue: *blue,
        }),
        _ => Err(malformed()),
    }
}

/// How much light a material makes of its own, defaulting to none.
fn read_emissive(declared: &MaterialDto, origin: &Origin) -> Result<Emissive, Fault> {
    let Some(value) = declared.emissive.as_ref() else {
        return Ok(Emissive::NONE);
    };
    let refusal = || {
        Fault::about(
            origin.clone(),
            format!(
                "`emissive` is a fraction of self-illumination from 0.0 to 1.0, but is {value}"
            ),
        )
        .in_field("emissive")
    };
    let fraction = value.as_float().ok_or_else(refusal)?;
    Emissive::new(fraction as f32).ok_or_else(refusal)
}

/// A file's name as a refusal quotes it back.
fn file_name(file: &Path) -> String {
    file.file_name().map_or_else(
        || file.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}
