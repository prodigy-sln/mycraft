//! Guard. The texture-set index, from both sides of it.
//!
//! # Why the format lives here and why this file is its whole grading
//!
//! `voxforge` writes the index and `mc-client` reads it, and neither may depend
//! on the other. Two hand-rolled parsers agreeing forever is precisely what
//! that arrangement exists to make unspellable, so the format is one
//! parse/render pair in the crate they both already depend on — and the pair is
//! graded **here**, through the published surface, rather than through either
//! consumer.
//!
//! Every refusal is an enumerated arm carrying the line it is about, and there
//! is one test per arm. An enumerated error with an arm no test constructs is
//! an arm nobody has read: it can be renamed, reordered or left unreachable and
//! nothing says so.
//!
//! # What the control characters are for
//!
//! A texture key has no character set — `namespaced.rs` says so outright — so a
//! key may carry a newline. `base:a\nfold 0000000000000000` is therefore a
//! spellable manifest entry whose rendered index a reader parses with a forged
//! fold. That is why the refusal is on **both** sides and why it is graded from
//! both in one test: refusing it only on parse leaves a writer able to emit the
//! forgery, and refusing it only on render leaves a reader that believes one.

use std::error::Error;

use mc_core::art::{IndexEntry, IndexError, TextureSetIndex};
use mc_core::id::{NamespacedIdError, TextureKey};

/// The error type every test in this file propagates with `?`.
type TestResult = Result<(), Box<dyn Error>>;

/// The line every index begins with.
const MAGIC: &str = "mycraft-texture-set 1";

/// A fold whose top two bytes are zero.
///
/// Deliberately not a value with sixteen significant digits: a renderer that
/// formatted the fold without padding emits `8f14e45fceea`, which is twelve
/// digits and which `parse` refuses — and only a value with a high zero byte
/// can say so.
const A_FOLD_WITH_LEADING_ZEROS: u64 = 0x0000_8f14_e45f_ceea;

/// That fold as an index states it.
const A_FOLD_LINE: &str = "fold 00008f14e45fceea";

/// The sources a round-tripped index records.
///
/// `a..b.toml` is here on purpose. The rule refuses a `..` **component**, and a
/// substring check would refuse this perfectly ordinary file name — a refusal
/// no author could act on, about a file they are allowed to have.
const ROUND_TRIP_SOURCES: [&str; 3] = [
    "textures.toml",
    "models/grass-block.mcvox",
    "materials/a..b.toml",
];

/// The keys a round-tripped index records, each with its image.
///
/// One key carries a space, which is the whole reason a `key` record is written
/// image first and key last: a texture key may contain whitespace and an image
/// file name may not, so only one of the two can be the rest of the line.
const ROUND_TRIP_ENTRIES: [(&str, &str); 3] = [
    ("base:stone", "base__stone.png"),
    ("base:grass top", "base__grass_top.png"),
    ("base:dirt", "base__dirt.png"),
];

/// Texts whose first line is not the one an index begins with.
const TEXTS_THAT_ARE_NOT_AN_INDEX: [&str; 3] =
    ["", "mycraft-texture-set 2", "# the base game's textures"];

/// Fold lines an index may not carry, each of them line 2 of its text.
///
/// The last is not malformed so much as absent: line 2 of an index is its fold,
/// and a text that puts something else there has none. It is refused through
/// the same arm because "this index states no fold I can read" is one answer,
/// and splitting it would add an arm nothing else needs.
const FOLD_LINES_NO_INDEX_MAY_CARRY: [&str; 6] = [
    "fold 8F14E45FCEEA167A",
    "fold 8f14e45fceea167",
    "fold 8f14e45fceea167ab",
    "fold zzzzzzzzzzzzzzzz",
    "fold",
    "source models/a.mcvox",
];

/// A key that renders as a whole second record.
///
/// `base:a` is a perfectly good texture key and the newline after it is legal
/// content text, so a manifest can spell this, and a renderer that did not
/// refuse it would emit an index whose next line reads `fold 0000000000000000`.
const THE_FORGING_KEY: &str = "base:a\nfold 0000000000000000";

/// Paths no index may record, on a `source` line or as an image name.
///
/// Both an absolute POSIX path and an absolute Windows one, because a rule
/// about paths tested only in one platform's spelling goes green on the other
/// against the very literal it exists to catch.
const PATHS_NO_INDEX_MAY_RECORD: [&str; 5] = [
    "/etc/passwd",
    "C:/Windows/system32",
    "models\\grass-block.mcvox",
    "../outside/dirt.toml",
    "",
];

/// The entries `stated` names, as an index holds them.
fn entries(stated: &[(&str, &str)]) -> Result<Vec<IndexEntry>, NamespacedIdError> {
    stated
        .iter()
        .map(|(key, image)| {
            Ok(IndexEntry {
                key: TextureKey::parse(key)?,
                image: (*image).to_owned(),
            })
        })
        .collect()
}

/// `stated` as the owned paths an index holds.
fn sources(stated: &[&str]) -> Vec<String> {
    stated.iter().map(|path| (*path).to_owned()).collect()
}

/// An index text carrying `records` after the magic and a well-formed fold.
fn index_of(records: &[&str]) -> String {
    let mut text = format!("{MAGIC}\n{A_FOLD_LINE}\n");
    for record in records {
        text.push_str(record);
        text.push('\n');
    }
    text
}

/// What parsing an index whose second line is `line` answers.
fn parsed_with_fold_line(line: &str) -> Result<TextureSetIndex, IndexError> {
    TextureSetIndex::parse(&format!("{MAGIC}\n{line}\nsource models/a.mcvox\n"))
}

/// The refusal parsing `text` earns, and nothing where it parsed.
fn refusal(text: &str) -> Option<IndexError> {
    TextureSetIndex::parse(text).err()
}

/// The refusal `spelled`, carrying a control character in `field` on that line,
/// earns.
///
/// A named helper because the three members are the whole content of that
/// answer, and four of them written out inline hide which is which.
fn carrying_one(line: usize, field: &'static str, spelled: &str) -> Option<IndexError> {
    Some(IndexError::ControlCharacter {
        line,
        field,
        spelled: spelled.to_owned(),
    })
}

#[test]
fn an_index_that_is_rendered_and_parsed_again_states_the_same_fold_sources_and_entries()
-> TestResult {
    let stated = TextureSetIndex::stating(
        A_FOLD_WITH_LEADING_ZEROS,
        sources(&ROUND_TRIP_SOURCES),
        entries(&ROUND_TRIP_ENTRIES)?,
    )?;

    let read = TextureSetIndex::parse(&stated.rendered());

    assert_eq!(
        read.as_ref(),
        Ok(&stated),
        "the two halves of this format are one contract, and a round trip is the only thing that \
         says so: a renderer that stopped padding the fold, reordered a record's two tokens or \
         dropped a source would each leave one half correct and the pair broken. The index that \
         went in states {sources:?} and {entries} entries",
        sources = stated.sources(),
        entries = stated.entries().len()
    );
    Ok(())
}

#[test]
fn a_first_line_that_is_not_the_magic_is_refused_naming_it() {
    let refused: Vec<Option<IndexError>> = TEXTS_THAT_ARE_NOT_AN_INDEX
        .iter()
        .map(|text| refusal(text))
        .collect();

    let owed: Vec<Option<IndexError>> = TEXTS_THAT_ARE_NOT_AN_INDEX
        .iter()
        .map(|text| {
            Some(IndexError::NotAnIndex {
                first_line: (*text).to_owned(),
            })
        })
        .collect();

    assert_eq!(
        refused, owed,
        "a text whose first line is not `{MAGIC}` is not an index, and the refusal quotes the \
         line it found instead — including the empty one, which is what an empty file offers and \
         what a reader would otherwise be told nothing about"
    );
}

#[test]
fn an_unknown_leading_word_is_refused_naming_the_line_and_the_word() {
    let refused = [
        refusal(&index_of(&["folds 0000000000000000"])),
        refusal(&index_of(&[
            "source models/a.mcvox",
            "image base__a.png base:a",
        ])),
        refusal(&index_of(&[""])),
    ];

    assert_eq!(
        refused,
        [
            Some(IndexError::UnknownRecord {
                line: 3,
                word: "folds".to_owned()
            }),
            Some(IndexError::UnknownRecord {
                line: 4,
                word: "image".to_owned()
            }),
            Some(IndexError::UnknownRecord {
                line: 3,
                word: String::new()
            }),
        ],
        "a record this format does not know is refused rather than passed over. A future record \
         type silently ignored is a reader quietly using half an index, and a blank line is the \
         same thing spelled with nothing"
    );
}

#[test]
fn a_fold_that_is_not_sixteen_hex_digits_is_refused() {
    let refused: Vec<Option<IndexError>> = FOLD_LINES_NO_INDEX_MAY_CARRY
        .iter()
        .map(|line| parsed_with_fold_line(line).err())
        .collect();

    let owed =
        vec![Some(IndexError::MalformedFold { line: 2 }); FOLD_LINES_NO_INDEX_MAY_CARRY.len()];

    assert_eq!(
        refused, owed,
        "the fold is sixteen lowercase hex digits on line 2 and nothing else. A truncated one \
         parses as a smaller number that compares unequal to every real fold forever, with no \
         message anywhere; an uppercase one would make two spellings of one value; and a line 2 \
         that is not a fold at all leaves the reader nothing to compare"
    );
}

#[test]
fn a_path_that_is_absolute_or_carries_a_parent_component_is_refused() {
    let mut refused: Vec<Option<IndexError>> = PATHS_NO_INDEX_MAY_RECORD
        .iter()
        .map(|path| refusal(&index_of(&[&format!("source {path}")])))
        .collect();
    refused.push(refusal(&index_of(&["key ../x.png base:x"])));

    let mut owed: Vec<Option<IndexError>> = PATHS_NO_INDEX_MAY_RECORD
        .iter()
        .map(|path| {
            Some(IndexError::UnsafePath {
                line: 3,
                path: (*path).to_owned(),
            })
        })
        .collect();
    owed.push(Some(IndexError::UnsafePath {
        line: 3,
        path: "../x.png".to_owned(),
    }));

    assert_eq!(
        refused, owed,
        "every path an index records is resolved by a reader against the content root it was \
         given, so one that escapes that root, names a drive or spells a separator this format \
         does not use is refused where it is written down rather than where it is opened. An \
         image name is a path by the same rule"
    );
}

#[test]
fn a_key_or_source_carrying_a_control_character_is_refused_on_render_and_on_parse() -> TestResult {
    let forging_key = entries(&[(THE_FORGING_KEY, "base__a.png")])?;
    let rendered = [
        TextureSetIndex::stating(0, sources(&["models/a.mcvox"]), forging_key).err(),
        TextureSetIndex::stating(0, sources(&["models/a\u{9}.mcvox"]), Vec::new()).err(),
    ];
    let parsed = [
        refusal(&index_of(&["source models/a\u{7}.mcvox"])),
        refusal(&index_of(&["key base__a.png base:a\u{7}b"])),
    ];

    assert_eq!(
        (rendered, parsed),
        (
            [
                carrying_one(4, "key", THE_FORGING_KEY),
                carrying_one(3, "source", "models/a\u{9}.mcvox")
            ],
            [
                carrying_one(3, "source", "models/a\u{7}.mcvox"),
                carrying_one(3, "key", "base:a\u{7}b")
            ]
        ),
        "a key carrying a newline is a spellable manifest entry that forges a whole record: \
         rendered, `base:a` followed by `fold 0000000000000000` is an index a reader accepts with \
         a fold nobody folded. Refusing it on one side only leaves either a writer that can emit \
         the forgery or a reader that believes one, so both are graded here. Each refusal quotes \
         the offending text as well as the line, because at render time that line is in a file \
         that does not exist yet and a number alone is nothing an author can act on"
    );
    Ok(())
}

#[test]
fn an_index_naming_one_key_twice_is_refused_naming_the_key() -> TestResult {
    let twice = index_of(&[
        "key base__a.png base:a",
        "key base__b.png base:b",
        "key other.png base:a",
    ]);

    assert_eq!(
        refusal(&twice),
        Some(IndexError::DuplicateKey {
            line: 5,
            key: TextureKey::parse("base:a")?,
        }),
        "an index naming one key twice offers a reader two images for it, and whichever it takes \
         is arbitrary. The build refuses a manifest that states a key twice; this is the other \
         door into the same state, through a file on disk that a build never wrote"
    );
    Ok(())
}
