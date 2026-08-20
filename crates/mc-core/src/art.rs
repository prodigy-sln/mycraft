//! The index a built texture set carries, and the fold it records.
//!
//! # Why the format lives here
//!
//! `voxforge` writes the index and `mc-client` reads it, and neither may depend
//! on the other: `crates/` never depends on `tools/`, and that is mechanically
//! asserted. Two hand-rolled parsers agreeing forever is the defect that
//! arrangement exists to make unspellable — not a value that is wrong, but two
//! values each computed correctly that do not match — so the format is one
//! parse/render pair in the crate both sides already depend on.
//!
//! Nothing here opens a file. The writer renders a `String` and writes it; the
//! reader reads the bytes and parses them; both hand the bytes they read to
//! [`folded_sources`]. That is what keeps this crate free of I/O while still
//! being the only place the agreement is written down.
//!
//! # The shape of an index
//!
//! ```text
//! mycraft-texture-set 1
//! fold 00008f14e45fceea
//! source models/grass-block.mcvox
//! source materials/dirt.toml
//! key base__grass_top.png base:grass_top
//! ```
//!
//! The magic is line 1 and the fold is line 2, sixteen lowercase hex digits.
//! `source` lines follow in fold order, and every path is relative to the
//! manifest's own directory, `/`-separated — which is what lets a copied
//! content root re-fold to the same value somewhere else on disk.
//!
//! A `key` record is written **image first, key last**, because a
//! [`TextureKey`] may contain whitespace and an image file name may not: only
//! one of the two can be the rest of the line, and it has to be the one with no
//! character set imposed on it.
//!
//! # Why a control character is refused on both sides
//!
//! A key has no character set, so `base:a` followed by a newline and
//! `fold 0000000000000000` is a spellable declaration whose rendered index a
//! reader would accept with a fold nobody folded. Refusing it only on parse
//! leaves a writer that can emit the forgery; refusing it only on render leaves
//! a reader that believes one. Both sides refuse it.

use crate::hash::fnv_1a_64;
use crate::id::TextureKey;

/// The file an index is written as, inside a set's output directory.
///
/// Shared rather than spelled on each side: two string literals that have to
/// match are the same hazard the format itself is here to remove.
pub const INDEX_FILE_NAME: &str = "index.txt";

/// The line every index begins with.
const MAGIC: &str = "mycraft-texture-set 1";

/// The word a fold record begins with.
const FOLD_RECORD: &str = "fold";

/// The word a source record begins with.
const SOURCE_RECORD: &str = "source";

/// The word an entry record begins with.
const KEY_RECORD: &str = "key";

/// How many hex digits a fold is written as.
const FOLD_DIGITS: usize = 16;

/// The line an index states its fold on.
const FOLD_LINE: usize = 2;

/// The line the first record after the fold occupies.
const FIRST_RECORD_LINE: usize = 3;

/// The field a refusal names when a texture key is at fault.
const KEY_FIELD: &str = "key";

/// The field a refusal names when a source path is at fault.
const SOURCE_FIELD: &str = "source";

/// The field a refusal names when an image file name is at fault.
const IMAGE_FIELD: &str = "image";

/// The extension every image in a set is written under.
const IMAGE_EXTENSION: &str = ".png";

/// One texture key, and the image file the set holds its art in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    /// The key a block declaration names.
    pub key: TextureKey,
    /// The file, within the set's directory, holding that key's art.
    pub image: String,
}

/// Why an index could not be read, or could not be written down.
///
/// Every arm carries the line it is about. On the parse side that is the line
/// as read; on the render side it is the line a reader would later meet the
/// record at, since the same convention in both directions is worth more than a
/// different one per direction — and the offending text travels with it,
/// because at render time the file does not exist yet and a number alone is
/// nothing an author can act on.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IndexError {
    /// The first line is not the one an index begins with.
    #[error("this is not a texture set index — its first line is `{first_line}`, not `{MAGIC}`")]
    NotAnIndex {
        /// The first line, exactly as it was found.
        first_line: String,
    },
    /// A record this format does not know, or one it cannot read.
    #[error("line {line}: `{word}` does not begin a record a texture set index may carry")]
    UnknownRecord {
        /// Where the record sits.
        line: usize,
        /// The leading word of that line.
        word: String,
    },
    /// The fold is not sixteen lowercase hex digits.
    #[error(
        "line {line}: a texture set index states its fold as `{FOLD_RECORD} ` and {FOLD_DIGITS} lowercase hex digits"
    )]
    MalformedFold {
        /// Where the fold belongs.
        line: usize,
    },
    /// A path an index may not record.
    #[error(
        "line {line}: `{path}` is not a path an index may record — every one is relative to the content root, `/`-separated, and names no parent directory"
    )]
    UnsafePath {
        /// Where the path was recorded.
        line: usize,
        /// The path, exactly as it was written.
        path: String,
    },
    /// A control character, which would forge or break a record.
    #[error(
        "line {line}: the {field} `{spelled}` carries a control character, and an index is written one record to a line"
    )]
    ControlCharacter {
        /// Where the record sits, or would sit.
        line: usize,
        /// Which of the record's fields carries it.
        field: &'static str,
        /// The offending text, so an author can find it.
        spelled: String,
    },
    /// One key recorded twice, offering a reader two images for it.
    #[error("line {line}: `{key}` is named twice, and an index offers one image per key", key = key.as_str())]
    DuplicateKey {
        /// Where the second mention sits.
        line: usize,
        /// The key stated twice.
        key: TextureKey,
    },
}

/// What a texture set was built from, and what art it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureSetIndex {
    /// The fold over every source, in the order they are recorded.
    fold: u64,
    /// The sources, in fold order.
    sources: Vec<String>,
    /// One entry per key the set covers.
    entries: Vec<IndexEntry>,
}

impl TextureSetIndex {
    /// The index stating `fold` over `sources`, holding `entries`.
    ///
    /// # Errors
    ///
    /// Returns [`IndexError::ControlCharacter`] where a source path, an image
    /// name or a key carries one — rendering it would emit a file whose records
    /// are not the ones this index states.
    pub fn stating(
        fold: u64,
        sources: Vec<String>,
        entries: Vec<IndexEntry>,
    ) -> Result<Self, IndexError> {
        for (at, source) in sources.iter().enumerate() {
            control_free(source, FIRST_RECORD_LINE + at, SOURCE_FIELD)?;
        }
        let after_sources = FIRST_RECORD_LINE + sources.len();
        for (at, entry) in entries.iter().enumerate() {
            control_free(&entry.image, after_sources + at, IMAGE_FIELD)?;
            control_free(entry.key.as_str(), after_sources + at, KEY_FIELD)?;
        }
        Ok(Self {
            fold,
            sources,
            entries,
        })
    }

    /// The index as the text a set carries it in.
    #[must_use]
    pub fn rendered(&self) -> String {
        // Padded to sixteen digits: a fold with a high zero byte written without
        // padding is a shorter line that a strict reader refuses, and the value
        // it would parse as is a different one.
        let mut text = format!("{MAGIC}\n{FOLD_RECORD} {fold:016x}\n", fold = self.fold);
        for source in &self.sources {
            text.push_str(&format!("{SOURCE_RECORD} {source}\n"));
        }
        for entry in &self.entries {
            text.push_str(&format!(
                "{KEY_RECORD} {image} {key}\n",
                image = entry.image,
                key = entry.key.as_str()
            ));
        }
        text
    }

    /// The index `text` states.
    ///
    /// # Errors
    ///
    /// Returns the [`IndexError`] naming the line and what is wrong with it.
    pub fn parse(text: &str) -> Result<Self, IndexError> {
        let mut lines = text.lines();
        let first = lines.next().unwrap_or_default();
        if first != MAGIC {
            return Err(IndexError::NotAnIndex {
                first_line: first.to_owned(),
            });
        }
        let fold = fold_stated(lines.next().unwrap_or_default())?;
        let mut read = Records::default();
        for (at, line) in lines.enumerate() {
            read.record(line, FIRST_RECORD_LINE + at)?;
        }
        Ok(Self {
            fold,
            sources: read.sources,
            entries: read.entries,
        })
    }

    /// The value this index records over its sources.
    #[must_use]
    pub const fn fold(&self) -> u64 {
        self.fold
    }

    /// Every source, in the order they were folded.
    #[must_use]
    pub fn sources(&self) -> &[String] {
        &self.sources
    }

    /// Every key the set covers, with the image holding its art.
    #[must_use]
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }
}

/// The records read so far, as one line at a time reaches them.
#[derive(Debug, Default)]
struct Records {
    /// The sources, in the order they were recorded.
    sources: Vec<String>,
    /// The entries, in the order they were recorded.
    entries: Vec<IndexEntry>,
}

impl Records {
    /// Reads one record, at line `at`.
    fn record(&mut self, line: &str, at: usize) -> Result<(), IndexError> {
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        match word {
            SOURCE_RECORD => self.source(rest, at),
            KEY_RECORD => self.entry(rest, at),
            // Refused rather than passed over: a record type a future writer
            // adds and this reader ignores is a reader quietly using half an
            // index, and a blank line is the same thing spelled with nothing.
            _ => Err(IndexError::UnknownRecord {
                line: at,
                word: word.to_owned(),
            }),
        }
    }

    /// Reads one `source` record.
    fn source(&mut self, path: &str, at: usize) -> Result<(), IndexError> {
        control_free(path, at, SOURCE_FIELD)?;
        recordable_path(path, at)?;
        self.sources.push(path.to_owned());
        Ok(())
    }

    /// Reads one `key` record.
    fn entry(&mut self, rest: &str, at: usize) -> Result<(), IndexError> {
        let Some((image, spelled)) = rest.split_once(' ') else {
            return Err(unreadable_entry(at));
        };
        control_free(image, at, IMAGE_FIELD)?;
        recordable_path(image, at)?;
        control_free(spelled, at, KEY_FIELD)?;
        let Ok(key) = TextureKey::parse(spelled) else {
            return Err(unreadable_entry(at));
        };
        if self.entries.iter().any(|held| held.key == key) {
            return Err(IndexError::DuplicateKey { line: at, key });
        }
        self.entries.push(IndexEntry {
            key,
            image: image.to_owned(),
        });
        Ok(())
    }
}

/// The refusal a `key` record nobody can read earns.
///
/// A `key` record whose key is not a namespaced id has no arm of its own: the
/// six are fixed, and a seventh no test could construct would be an arm nobody
/// has read. It is reported as the record it is — a line beginning `key` that
/// this format cannot read — rather than as a key, because the line's leading
/// word is what a reader can see is at fault and the text after it is precisely
/// the part that means nothing.
fn unreadable_entry(at: usize) -> IndexError {
    IndexError::UnknownRecord {
        line: at,
        word: KEY_RECORD.to_owned(),
    }
}

/// The value a fold line states.
fn fold_stated(line: &str) -> Result<u64, IndexError> {
    let malformed = || IndexError::MalformedFold { line: FOLD_LINE };
    let Some(digits) = line.strip_prefix(&format!("{FOLD_RECORD} ")) else {
        return Err(malformed());
    };
    if digits.len() != FOLD_DIGITS
        || !digits
            .bytes()
            .all(|digit| matches!(digit, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(malformed());
    }
    // The digits are already exactly sixteen of the sixteen this accepts, so the
    // conversion cannot fail; the mapping is here because no total form of it
    // exists, not because a failure is expected.
    u64::from_str_radix(digits, 16).ok().ok_or_else(malformed)
}

/// Refuses `text` where it carries a character an index cannot hold.
fn control_free(text: &str, at: usize, field: &'static str) -> Result<(), IndexError> {
    if text.chars().any(|held| held.is_ascii_control()) {
        return Err(IndexError::ControlCharacter {
            line: at,
            field,
            spelled: text.to_owned(),
        });
    }
    Ok(())
}

/// Refuses `path` where a reader resolving it against a content root would
/// leave that root, or would have to guess at a separator this format does not
/// write.
fn recordable_path(path: &str, at: usize) -> Result<(), IndexError> {
    let refused = path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || names_a_drive(path)
        // A component, never a substring: `a..b.toml` is an ordinary file name
        // and refusing it would be a refusal no author could act on.
        || path.split('/').any(|part| part == "..");
    if refused {
        return Err(IndexError::UnsafePath {
            line: at,
            path: path.to_owned(),
        });
    }
    Ok(())
}

/// Whether `path` begins with a Windows drive letter.
///
/// Judged here rather than by `Path::is_absolute`, which answers differently on
/// each platform: an index written on Windows is read on Linux and the rule has
/// to be the same one on both.
fn names_a_drive(path: &str) -> bool {
    let mut letters = path.chars();
    matches!(
        (letters.next(), letters.next()),
        (Some(first), Some(':')) if first.is_ascii_alphabetic()
    )
}

/// The fold over `sources`, each a recorded path and the bytes read from it.
///
/// The sequence, which both sides of the index depend on: for each source in
/// order, the recorded path as UTF-8 bytes preceded by its length as a
/// little-endian `u64`, then the file's bytes preceded by theirs.
///
/// **Length prefixes rather than separators**, so a file holding whatever byte
/// a separator used cannot forge a boundary: without them, the source `ab`
/// holding nothing and the source `a` holding `b` fold over the same two bytes.
#[must_use]
pub fn folded_sources(sources: &[(&str, &[u8])]) -> u64 {
    let mut folding: Vec<u8> = Vec::new();
    for (path, bytes) in sources {
        length_prefixed(&mut folding, path.as_bytes());
        length_prefixed(&mut folding, bytes);
    }
    fnv_1a_64(&folding)
}

/// Appends `bytes` to `into`, preceded by its length as a little-endian `u64`.
fn length_prefixed(into: &mut Vec<u8>, bytes: &[u8]) {
    into.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    into.extend_from_slice(bytes);
}

/// Whether `name` is a name a set's image may be written under.
///
/// `[A-Za-z0-9._-]+` followed by `.png`, which is the one rule both sides of the
/// index apply and neither invents. voxforge derives the name from a texture key
/// and refuses a build whose derived name this rejects; a reader takes the name
/// **from the index** rather than deriving it a second time, and refuses one this
/// rejects — the same reason the format itself lives in this crate.
///
/// A key has no character set, so deriving a file name from one is deriving a
/// path from unconstrained content text. That is a correctness and
/// reproducibility rule rather than a threat model: a build whose output
/// location depends on punctuation in a key is not reproducible, and a name
/// carrying a separator will not round-trip through the index a client parses.
///
/// `.` and `..` are excluded by the extension rather than by a second test —
/// neither ends in `.png` — so there is no separate branch for them to rot in.
#[must_use]
pub fn is_an_ordinary_image_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(IMAGE_EXTENSION) else {
        return false;
    };
    !stem.is_empty()
        && stem
            .chars()
            .all(|held| held.is_ascii_alphanumeric() || matches!(held, '.' | '_' | '-'))
}
