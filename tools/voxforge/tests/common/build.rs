//! The harness the `voxforge build` tests drive the tool through, and the
//! verdicts they grade its effects with.
//!
//! **Declared by path from the build tests alone rather than from
//! `common/mod.rs`**, so that a test binary with no interest in an art build
//! does not link `mc-core`'s index parser to get at a document fixture.
//!
//! Every run goes through [`crate::common::cli::invoke`], which is the shipped
//! command line over two in-memory writers. That is deliberate and it is the
//! only route these tests take: a manifest is what a mod author writes and
//! `voxforge build <manifest>` is what they type, so a fixture handing a
//! `Manifest` value straight to a builder would assert about a world the
//! product does not inhabit.
//!
//! Every verdict here is an **enumerated** answer. "No index was written" and
//! "an index naming no keys was written" must never compare equal — that pair
//! is the whole of what keeps the zero-entry scenario from passing for a build
//! that writes nothing at all — and neither must "the previous set survived"
//! and "the check can no longer look at it".

// Each build test binary links this whole module and uses a subset of it.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use mc_core::art::{INDEX_FILE_NAME, TextureSetIndex};
use mc_core::content::TEXTURE_EDGE;
use mc_core::hash::fnv_1a_64;
use tempfile::TempDir;
use voxforge::inspect::ExitCode;

use crate::common::cli::invoke;
use crate::common::texture::{BLACK, GRADIENT, HIGH, LOW, Tone, WHITE, material_text, model};

/// The file a manifest is written as, inside a content root.
///
/// The shipped root carries one at `content/base/textures.toml`, and a build is
/// given that path — so a fixture spells it the same way.
pub const MANIFEST_FILE: &str = "textures.toml";

/// Where the shipped content root sits, relative to the repository.
const SHIPPED_ROOT: &str = "content/base";

/// A content root a build is run against.
///
/// A real temporary directory rather than a mock of one: the thing under test
/// is what lands on disk and what survives a refusal, and a mocked filesystem
/// asserts nothing about either.
#[derive(Debug)]
pub struct Root {
    /// The directory itself, removed when the test ends.
    directory: TempDir,
}

impl Root {
    /// A root holding nothing yet.
    ///
    /// # Errors
    ///
    /// Returns the I/O failure when the directory cannot be made.
    pub fn bare() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            directory: TempDir::new()?,
        })
    }

    /// A copy of the shipped content root.
    ///
    /// A copy and not the tree itself, because a build writes into its root and
    /// a test must not write into the repository. The copy is also what says
    /// the recorded paths are relative to the manifest's own directory: an
    /// index recording absolute paths re-folds against nothing here.
    ///
    /// # Errors
    ///
    /// Returns the I/O failure when the tree cannot be copied.
    pub fn shipped() -> Result<Self, Box<dyn Error>> {
        let made = Self::bare()?;
        copied(
            &crate::common::cli::repository_path(SHIPPED_ROOT),
            made.path(),
        )?;
        // The set is derived and ignored by git, but it is *present* on any
        // machine that has run a build — and the gate runs one. A copy carrying
        // it would find its output current, do no work, and leave every
        // assertion in this suite grading somebody else's build.
        drop(fs::remove_dir_all(made.output()));
        Ok(made)
    }

    /// The root itself.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// The manifest path a build is given.
    #[must_use]
    pub fn manifest(&self) -> PathBuf {
        self.path().join(MANIFEST_FILE)
    }

    /// The directory a build writes its set into.
    #[must_use]
    pub fn output(&self) -> PathBuf {
        self.path().join("textures")
    }

    /// The same root, now holding `text` at `relative`.
    ///
    /// # Errors
    ///
    /// Returns the I/O failure when the file or the directories above it cannot
    /// be written.
    pub fn holding(&self, relative: &str, text: &str) -> Result<&Self, Box<dyn Error>> {
        let path = self.path().join(relative);
        if let Some(above) = path.parent() {
            fs::create_dir_all(above)?;
        }
        fs::write(&path, text)?;
        Ok(self)
    }

    /// The same root, now declaring one material file per tone of `palette`.
    ///
    /// # Errors
    ///
    /// Returns the I/O failure when a file cannot be written.
    pub fn painted(&self, palette: &[Tone]) -> Result<&Self, Box<dyn Error>> {
        for tone in palette {
            let file = tone.key.rsplit(':').next().unwrap_or(tone.key);
            self.holding(&format!("materials/{file}.toml"), &material_text(*tone))?;
        }
        Ok(self)
    }

    /// The same root, now carrying one block file per key of `declared`.
    ///
    /// The files are Luau because that is what the scan reads, and the scan
    /// reads them as **text**: an art build that started a script host to find
    /// out which keys are used would let a broken block declaration refuse a
    /// texture bake that has nothing to do with it.
    ///
    /// # Errors
    ///
    /// Returns the I/O failure when a file cannot be written.
    pub fn declaring(&self, declared: &[&str]) -> Result<&Self, Box<dyn Error>> {
        for (at, key) in declared.iter().enumerate() {
            let text = format!("return {{\n\tname = \"{key}\",\n\ttexture = \"{key}\",\n}}\n");
            self.holding(&format!("blocks/declared_{at}.luau"), &text)?;
        }
        Ok(self)
    }
}

/// Copies the tree at `from` into `into`, directories and all.
fn copied(from: &Path, into: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(into)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let landing = into.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copied(&entry.path(), &landing)?;
        } else {
            fs::copy(entry.path(), landing)?;
        }
    }
    Ok(())
}

/// One entry of a manifest, as its three fields.
#[derive(Debug, Clone, Copy)]
pub struct Entry<'a> {
    /// The texture key the entry bakes to.
    pub key: &'a str,
    /// The model the face is taken from, relative to the manifest.
    pub model: &'a str,
    /// Which of that model's six faces.
    pub face: &'a str,
}

/// That entry.
#[must_use]
pub const fn entry<'a>(key: &'a str, model: &'a str, face: &'a str) -> Entry<'a> {
    Entry { key, model, face }
}

/// A manifest naming `entries`, at that many pixels per voxel.
///
/// The key is written into the file verbatim, so a caller states a TOML escape
/// where it wants one — which is the only way a key carrying a line break
/// reaches the tool the way an author would spell it.
#[must_use]
pub fn manifest(pixels_per_voxel: u32, entries: &[Entry<'_>]) -> String {
    let mut text = format!(
        "output           = \"textures\"\nmaterials        = \"materials\"\n\
         blocks           = \"blocks\"\npixels_per_voxel = {pixels_per_voxel}\n"
    );
    for stated in entries {
        text.push_str(&format!(
            "\n[[texture]]\nkey   = \"{key}\"\nmodel = \"{model}\"\nface  = \"{face}\"\n",
            key = stated.key,
            model = stated.model,
            face = stated.face
        ));
    }
    text
}

/// A `scale`-cubed model painted from `palette`, whose voxel at `x` takes
/// whatever `tone_of` answers.
///
/// Cubed because a face set is by definition a block's six faces, and painted
/// by `x` alone because that is what makes one pair of a face's edges disagree
/// while the other pair, and two whole faces, stay uniform.
#[must_use]
pub fn cube(scale: u32, palette: &[Tone], tone_of: &dyn Fn(u32) -> Option<Tone>) -> String {
    model((scale, scale, scale), scale, palette, &|x, _, _| tone_of(x))
}

/// A model of `size` declaring `scale`, filled entirely with `tone`.
#[must_use]
pub fn block_of(size: (u32, u32, u32), scale: u32, tone: Tone) -> String {
    model(size, scale, &[tone], &|_, _, _| Some(tone))
}

/// A cube of the edge a block texture has, filled entirely with `tone`.
///
/// Every face of it is flat, so every face tiles: the largest step within a row
/// is zero and so is the step across the wrap.
#[must_use]
pub fn uniform_cube(tone: Tone) -> String {
    let edge = TEXTURE_EDGE;
    block_of((edge, edge, edge), edge, tone)
}

/// The tone a ramped cube shows at column `x`.
///
/// Four equal steps of 85 across sixteen columns, because `255 / 3 = 85`
/// exactly. Within a row the largest step is 85 and across the wrap it is 255,
/// which is the only shape that makes an edge disagree by more than the face's
/// own interior — a fixture whose interior steps saturated could never fail
/// that leg at all.
#[must_use]
pub fn ramp(x: u32) -> Option<Tone> {
    match x {
        0..=3 => Some(BLACK),
        4..=7 => Some(LOW),
        8..=11 => Some(HIGH),
        _ => Some(WHITE),
    }
}

/// A cube of the block edge painted by [`ramp`].
///
/// Its `front`, `back`, `top` and `bottom` faces each run the ramp across the
/// image and do not tile; its `left` and `right` faces see one slab apiece and
/// are flat, so they do.
#[must_use]
pub fn ramped_cube() -> String {
    cube(TEXTURE_EDGE, &GRADIENT, &ramp)
}

/// The file name a key's image is written under.
///
/// The rule restated as this suite's own oracle: a key's colon becomes two
/// underscores and the name gains `.png`. Restated rather than imported,
/// because a test that asked the tool what it named a file would agree with
/// whatever the tool did.
#[must_use]
pub fn image_named(key: &str) -> String {
    format!("{name}.png", name = key.replace(':', "__"))
}

/// One entry a manifest on disk states: its key, its model and its face.
pub type Stated = (String, String, String);

/// Every entry a manifest states, in the order it states them.
///
/// Read from the file rather than restated in a test, so that "seven entries"
/// is the manifest's own length and not a number repeated beside it, and so
/// that a scenario naming one entry's three fields can say whether the shipped
/// manifest really carries it.
///
/// # Errors
///
/// Returns an error when the manifest cannot be read or is not TOML.
pub fn entries_stated(manifest: &Path) -> Result<Vec<Stated>, Box<dyn Error>> {
    let text = fs::read_to_string(manifest)?;
    let read: toml::Value = toml::from_str(&text)?;
    let stated = read
        .get("texture")
        .and_then(toml::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    Ok(stated.iter().map(field_triple).collect())
}

/// The three fields one `[[texture]]` table states, empty where it states none.
fn field_triple(entry: &toml::Value) -> Stated {
    let read = |field: &str| {
        entry
            .get(field)
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    (read("key"), read("model"), read("face"))
}

/// The keys a manifest states, in the order it states them.
///
/// # Errors
///
/// Returns an error when the manifest cannot be read or is not TOML.
pub fn keys_stated(manifest: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(entries_stated(manifest)?
        .into_iter()
        .map(|(key, _, _)| key)
        .collect())
}

/// The model the shared fixture root holds, as a manifest names it.
pub const CUBE_MODEL: &str = "models/cube.mcvox";

/// The two keys the shared fixture manifest bakes.
pub const FIRST_KEY: &str = "base:one";
/// The second of them.
pub const SECOND_KEY: &str = "base:two";

/// A root holding one uniform cube of the edge a block texture has, that tone's
/// material, and a block file declaring both fixture keys — but no manifest.
///
/// The edge comes from [`TEXTURE_EDGE`] rather than from a sixteen written here,
/// because a model whose scale times the manifest's pixels per voxel is not that
/// edge is refused, and a fixture spelling its own sixteen would go on passing
/// through a change to the contract it is supposed to bake against.
///
/// # Errors
///
/// Returns the I/O failure when the root cannot be written.
pub fn root_of_one_cube(tone: Tone) -> Result<Root, Box<dyn Error>> {
    let root = Root::bare()?;
    root.holding(CUBE_MODEL, &uniform_cube(tone))?
        .painted(&[tone])?
        .declaring(&[FIRST_KEY, SECOND_KEY])?;
    Ok(root)
}

/// A manifest baking two of that cube's faces, at one pixel per voxel.
#[must_use]
pub fn two_faces_of_the_cube() -> String {
    manifest(
        1,
        &[
            entry(FIRST_KEY, CUBE_MODEL, "front"),
            entry(SECOND_KEY, CUBE_MODEL, "top"),
        ],
    )
}

/// Everything one `voxforge build` run answered and left behind.
#[derive(Debug)]
pub struct Build {
    /// What the tool answered.
    pub code: ExitCode,
    /// Everything it wrote to stdout.
    pub out: String,
    /// Everything it wrote to stderr.
    pub err: String,
    /// Every file in the output directory afterwards, by name.
    pub written: BTreeMap<String, Vec<u8>>,
}

/// What an index says about the set beside it, as one answer.
#[derive(Debug, PartialEq, Eq)]
pub enum Index {
    /// It names these keys, in the order it states them.
    Naming(Vec<String>),
    /// There is no index at all.
    Absent,
    /// There is one and it does not parse, saying this.
    Unreadable(String),
}

impl Index {
    /// The same answer with its keys ascending.
    ///
    /// The order an index states its keys in is not something any scenario
    /// fixes, so a test about *which* keys it names sorts both sides rather
    /// than pinning an order the format never promised.
    #[must_use]
    pub fn sorted(self) -> Self {
        let Self::Naming(mut named) = self else {
            return self;
        };
        named.sort();
        Self::Naming(named)
    }
}

/// The value an index records, or why there is none.
#[derive(Debug, PartialEq, Eq)]
pub enum Fold {
    /// That value.
    Recorded(u64),
    /// There is no index at all.
    Absent,
    /// There is one and it does not parse, saying this.
    Unreadable(String),
}

/// How a build that was supposed to be refused actually ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// With a failing exit, its words naming everything they had to.
    NamingEverything,
    /// With a failing exit, and these are absent from its words.
    Missing(Vec<String>),
    /// Successfully, having left this many files behind.
    Completed(usize),
}

impl Build {
    /// Which images the build left behind, by file name, ascending.
    #[must_use]
    pub fn images(&self) -> Vec<String> {
        self.written
            .keys()
            .filter(|name| name.ends_with(".png"))
            .cloned()
            .collect()
    }

    /// Every file the build left, by name, as its length and a fold of its
    /// bytes.
    ///
    /// A fold rather than the bytes themselves, and only because a failure has
    /// to be readable: comparing two whole output directories byte for byte
    /// prints tens of kilobytes of PNG per side and buries the one file that
    /// differs. The length travels with it so that a truncation is visible as
    /// one, and the fold is `mc-core`'s published one, which nothing in this
    /// suite is asserting about.
    #[must_use]
    pub fn fingerprints(&self) -> BTreeMap<String, (usize, u64)> {
        fingerprinted(&self.written)
    }

    /// The index the build left, as one answer.
    #[must_use]
    pub fn index(&self) -> Index {
        match self.parsed_index() {
            None => Index::Absent,
            Some(Err(unreadable)) => Index::Unreadable(unreadable),
            Some(Ok(read)) => Index::Naming(
                read.entries()
                    .iter()
                    .map(|entry| entry.key.as_str().to_owned())
                    .collect(),
            ),
        }
    }

    /// The value the index records, as one answer.
    #[must_use]
    pub fn recorded_fold(&self) -> Fold {
        match self.parsed_index() {
            None => Fold::Absent,
            Some(Err(unreadable)) => Fold::Unreadable(unreadable),
            Some(Ok(read)) => Fold::Recorded(read.fold()),
        }
    }

    /// How the build refused, and which of `expected` its words leave unnamed.
    ///
    /// The refusal is read off **stderr**, which is where this tool puts every
    /// one it makes, and off the exit code, because a build that said the right
    /// words and exited zero would let the gate carry on and test against a
    /// stale set.
    #[must_use]
    pub fn refusal(&self, expected: &[&str]) -> Refused {
        if self.code != ExitCode::Defective {
            return Refused::Completed(self.written.len());
        }
        let missing: Vec<String> = expected
            .iter()
            .filter(|token| !self.err.contains(*token))
            .map(|token| (*token).to_owned())
            .collect();
        if missing.is_empty() {
            return Refused::NamingEverything;
        }
        Refused::Missing(missing)
    }

    /// The index as `mc-core` reads it, and nothing where none was written.
    fn parsed_index(&self) -> Option<Result<TextureSetIndex, String>> {
        let bytes = self.written.get(INDEX_FILE_NAME)?;
        let Ok(text) = std::str::from_utf8(bytes) else {
            return Some(Err("the index is not UTF-8".to_owned()));
        };
        Some(TextureSetIndex::parse(text).map_err(|cause| cause.to_string()))
    }
}

/// What an edit did to the value the index records.
///
/// Three answers and not two: an edit that moved the fold and a build that
/// left no readable index at all must never compare equal, or a scenario about
/// staleness passes for a build that stopped writing.
#[derive(Debug, PartialEq, Eq)]
pub enum Recorded {
    /// A different value than before.
    Moved,
    /// The same value.
    Stayed,
    /// One of the two builds recorded no value this test could read.
    Unavailable,
}

/// What `before` and `after` say an edit between them did.
#[must_use]
pub fn recorded(before: &Fold, after: &Fold) -> Recorded {
    let (Fold::Recorded(was), Fold::Recorded(now)) = (before, after) else {
        return Recorded::Unavailable;
    };
    if was == now {
        return Recorded::Stayed;
    }
    Recorded::Moved
}

/// What `voxforge build <root>/textures.toml` does.
///
/// # Errors
///
/// Returns an error when either stream holds bytes that are not UTF-8, or when
/// the output directory exists and cannot be read.
pub fn built(root: &Root) -> Result<Build, Box<dyn Error>> {
    built_from(&root.manifest(), &root.output())
}

/// What `voxforge build <manifest>` does, reading `output` afterwards.
///
/// # Errors
///
/// Returns an error when either stream holds bytes that are not UTF-8, or when
/// the output directory exists and cannot be read.
pub fn built_from(manifest: &Path, output: &Path) -> Result<Build, Box<dyn Error>> {
    let spelled = manifest.display().to_string();
    let answered = invoke(&["build", &spelled])?;
    Ok(Build {
        code: answered.code,
        out: answered.out,
        err: answered.err,
        written: files_in(output)?,
    })
}

/// `files` by name, as each one's length and a fold of its bytes.
#[must_use]
pub fn fingerprinted(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, (usize, u64)> {
    files
        .iter()
        .map(|(name, bytes)| (name.clone(), (bytes.len(), fnv_1a_64(bytes))))
        .collect()
}

/// Every file directly inside `directory`, by name, and nothing where the
/// directory is not there.
///
/// A missing directory answers an empty map rather than an error: a build that
/// wrote nothing at all has to reach the assertion as an empty set of files and
/// be graded there, not disappear into an I/O failure the test reports as its
/// own.
///
/// # Errors
///
/// Returns the I/O failure when the directory is there and an entry inside it
/// cannot be read.
pub fn files_in(directory: &Path) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Ok(BTreeMap::new());
    };
    let mut found = BTreeMap::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            found.insert(
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path())?,
            );
        }
    }
    Ok(found)
}
