//! No Rust source names a block the base game ships.
//!
//! Invariant 1 in test form. The base game is a mod, so the engine may know the
//! *shape* of a block definition and nothing about any particular block; the
//! moment a name appears in Rust, the base game has a privilege a third-party mod
//! does not.
//!
//! The scan reads every `.rs` file under a crate's `src/` and looks at its
//! **production text**: the file minus every `#[cfg(test)]` item and minus every
//! doc comment. Unit tests are inline `#[cfg(test)] mod tests` blocks
//! (`docs/technical/testing.md`) and a rustdoc example is a doc test, so both are
//! test code that happens to live in a production file, and both may say the
//! names out loud. So may tests under `tests/`, which are not scanned at all —
//! which is why this one can.
//!
//! Finding where a `#[cfg(test)]` item ends means counting braces, and a `{`
//! inside a string or a comment does not open a block. Hence the state machine
//! below rather than a line filter. It refuses to guess: a walk that does not end
//! balanced fails the scan rather than silently swallowing the rest of a file,
//! because *that* failure mode would leave the check green forever.

mod common;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use common::{TestResult, repository_root};
use tempfile::TempDir;

/// The blocks this repository ships as content.
const SHIPPED_NAMES: [&str; 5] = [
    "base:air",
    "base:stone",
    "base:dirt",
    "base:grass",
    "base:water",
];

/// The attribute that marks the item after it as test-only.
const CFG_TEST: &[u8] = b"#[cfg(test)]";

/// What a scan of one directory tree found.
#[derive(Debug, Default)]
struct Scan {
    files_read: usize,
    hits: Vec<String>,
}

/// Where a walk is in a file. A `{` opens a block only in `Code`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Lexical {
    #[default]
    Code,
    Comment,
    DocComment,
    StringLiteral,
}

/// One pass over one file, keeping the bytes outside every `#[cfg(test)]` item
/// and outside every doc comment.
///
/// What it produces is text to search, not compilable Rust — the brace that
/// closed a skipped item is kept, which matters to a compiler and not to a
/// substring search.
#[derive(Debug, Default)]
struct ProductionText {
    kept: Vec<u8>,
    depth: usize,
    lexical: Lexical,
    escaped: bool,
    pending_cfg_test: bool,
    test_item_opened_at: Option<usize>,
}

impl ProductionText {
    /// Consumes one byte — or the whole `#[cfg(test)]` attribute — and reports
    /// how many bytes that was.
    fn step(&mut self, rest: &[u8]) -> usize {
        let Some(&byte) = rest.first() else { return 1 };
        match self.lexical {
            Lexical::Code => return self.in_code(rest, byte),
            Lexical::StringLiteral => self.in_string(byte),
            Lexical::Comment | Lexical::DocComment => self.end_comment_at_newline(byte),
        }
        self.keep(byte);
        1
    }

    fn in_code(&mut self, rest: &[u8], byte: u8) -> usize {
        if rest.starts_with(CFG_TEST) {
            self.pending_cfg_test = true;
            return CFG_TEST.len();
        }
        match byte {
            b'/' if rest.starts_with(b"//") => self.begin_comment(rest),
            b'"' => self.lexical = Lexical::StringLiteral,
            b'{' => self.open_brace(),
            b'}' => self.close_brace(),
            // `#[cfg(test)] use ...;` attaches to an item with no block of its
            // own, so the attribute is spent rather than left waiting for the
            // next `{` it meets.
            b';' => self.pending_cfg_test = false,
            _ => {}
        }
        self.keep(byte);
        1
    }

    /// A doc comment is documentation and its examples are doc tests, so its
    /// text is not production source. An ordinary comment is.
    fn begin_comment(&mut self, rest: &[u8]) {
        self.lexical = if rest.starts_with(b"///") || rest.starts_with(b"//!") {
            Lexical::DocComment
        } else {
            Lexical::Comment
        };
    }

    fn end_comment_at_newline(&mut self, byte: u8) {
        if byte == b'\n' {
            self.lexical = Lexical::Code;
        }
    }

    fn in_string(&mut self, byte: u8) {
        if self.escaped {
            self.escaped = false;
            return;
        }
        match byte {
            b'\\' => self.escaped = true,
            b'"' => self.lexical = Lexical::Code,
            _ => {}
        }
    }

    /// The depth an item opened at is what says where it closes, so that is what
    /// is remembered rather than a nesting count of its own.
    fn open_brace(&mut self) {
        if self.pending_cfg_test && self.test_item_opened_at.is_none() {
            self.test_item_opened_at = Some(self.depth);
        }
        self.pending_cfg_test = false;
        self.depth += 1;
    }

    fn close_brace(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        if self.test_item_opened_at == Some(self.depth) {
            self.test_item_opened_at = None;
        }
    }

    fn keep(&mut self, byte: u8) {
        if self.test_item_opened_at.is_some() || self.lexical == Lexical::DocComment {
            return;
        }
        self.kept.push(byte);
    }

    /// The production text, or an explanation of why this walk cannot be trusted.
    fn finish(self) -> Result<String, String> {
        if self.depth != 0 || self.test_item_opened_at.is_some() {
            return Err(format!(
                "the walk ended {} brace(s) deep, so it lost track of the file. Reporting no hits \
                 from here would mean 'the scanner gave up', which is indistinguishable from \
                 'nothing is wrong' — and stays green forever",
                self.depth
            ));
        }
        Ok(String::from_utf8_lossy(&self.kept).into_owned())
    }
}

/// A file's text with every `#[cfg(test)]` item and every doc comment removed.
fn production_text(source: &str) -> Result<String, String> {
    let bytes = source.as_bytes();
    let mut walk = ProductionText::default();
    let mut index = 0;
    while index < bytes.len() {
        index += walk.step(bytes.get(index..).unwrap_or_default());
    }
    walk.finish()
}

/// Reads every Rust source under `root` and reports each place a shipped block
/// name appears in one's production text.
fn scan_for_shipped_names(root: &Path) -> Result<Scan, Box<dyn Error>> {
    let mut scan = Scan::default();
    scan_directory(root, &mut scan)?;
    Ok(scan)
}

fn scan_directory(directory: &Path, scan: &mut Scan) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            scan_directory(&path, scan)?;
        } else if is_rust_source(&path) {
            scan_file(&path, scan)?;
        }
    }
    Ok(())
}

fn is_rust_source(path: &Path) -> bool {
    path.extension().and_then(OsStr::to_str) == Some("rs")
}

fn scan_file(path: &Path, scan: &mut Scan) -> Result<(), Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    let text = match production_text(&source) {
        Ok(text) => text,
        Err(why) => return Err(format!("{}: {why}", path.display()).into()),
    };
    scan.files_read += 1;
    for name in SHIPPED_NAMES {
        if text.contains(name) {
            scan.hits.push(format!("{} names `{name}`", path.display()));
        }
    }
    Ok(())
}

/// Every crate's production source directory.
fn source_directories() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut directories = Vec::new();
    for entry in fs::read_dir(repository_root()?.join("crates"))? {
        let sources = entry?.path().join("src");
        if sources.is_dir() {
            directories.push(sources);
        }
    }
    Ok(directories)
}

/// A scan of a single file written into a temporary directory.
fn scan_of(source: &str) -> Result<(TempDir, Scan), Box<dyn Error>> {
    let directory = TempDir::new()?;
    fs::write(directory.path().join("blocks.rs"), source)?;
    let scanned = scan_for_shipped_names(directory.path())?;
    Ok((directory, scanned))
}

#[test]
fn no_production_rust_source_names_a_block_the_base_game_ships() -> TestResult {
    let mut scanned = Scan::default();
    for directory in source_directories()? {
        let found = scan_for_shipped_names(&directory)?;
        scanned.files_read += found.files_read;
        scanned.hits.extend(found.hits);
    }

    assert!(
        scanned.files_read > 0,
        "the scan read no Rust source at all, so the check below would be vacuous"
    );
    assert!(
        scanned.hits.is_empty(),
        "a block's name belongs to content, never to the engine: {:?}",
        scanned.hits
    );
    Ok(())
}

/// A guard rather than a scenario, and the reason the check above cannot go
/// quiet. A scan whose directory walk or whose matcher broke would report nothing
/// forever — including on the day the invariant it guards is actually violated.
/// The fixture is nested one directory deep on purpose: a walk that stopped at
/// the top level would otherwise still look healthy here.
#[test]
fn the_scan_reports_a_source_that_does_name_a_block_the_base_game_ships() -> TestResult {
    let directory = TempDir::new()?;
    let nested = directory.path().join("nested");
    fs::create_dir_all(&nested)?;
    fs::write(
        nested.join("blocks.rs"),
        "const FILL: &str = \"base:grass\";\n",
    )?;

    let scanned = scan_for_shipped_names(directory.path())?;

    assert!(
        !scanned.hits.is_empty(),
        "a source that does name a shipped block must be reported, or this scan proves nothing"
    );
    Ok(())
}

/// The second half of that guard, now that the filter is a walk rather than a
/// file name. The walk must skip the test item and *not a byte more*: one that
/// lost the closing brace — to a `{` inside a string, say — would swallow every
/// remaining line of the file, leaving the real check above green while scanning
/// nothing. So the fixture puts a shipped name on both sides of a test module
/// whose body contains an unbalanced brace in a string literal.
#[test]
fn a_name_inside_a_test_module_is_skipped_and_one_after_it_is_still_found() -> TestResult {
    let (_directory, scanned) = scan_of(concat!(
        "#[cfg(test)]\n",
        "mod tests {\n",
        "    const NAMED_IN_A_TEST: &str = \"base:dirt\";\n",
        "    fn opening_brace_in_a_string() -> &'static str { \"{\" }\n",
        "}\n",
        "const NAMED_IN_PRODUCTION: &str = \"base:stone\";\n",
    ))?;

    assert!(
        scanned.hits.len() == 1 && scanned.hits.join(" ").contains("base:stone"),
        "the test module's name is test code and must be skipped; the one after it is production \
         source and must still be found. Exactly one hit, and it is the second: {:?}",
        scanned.hits
    );
    Ok(())
}

/// Why a rustdoc example may say `base:stone` out loud. A doc example is a doc
/// test — test code that happens to live in a production file — and the most
/// natural example for a namespaced-id type is the real namespace. This asserts
/// the scan agrees, so the rule stays a decision rather than a trap someone
/// rediscovers by turning the suite red.
#[test]
fn a_name_in_a_doc_example_is_not_a_hardcoded_block_name() -> TestResult {
    let (_directory, scanned) = scan_of(concat!(
        "/// ```\n",
        "/// let name = BlockName::parse(\"base:water\")?;\n",
        "/// ```\n",
        "pub fn parse_a_name() {}\n",
    ))?;

    assert!(
        scanned.hits.is_empty(),
        "a doc example is a doc test, so naming a shipped block in one is not the engine knowing \
         about it: {:?}",
        scanned.hits
    );
    Ok(())
}
