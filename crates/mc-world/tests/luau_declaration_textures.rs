//! What a declaration's `texture` field may say, and which key each of a
//! block's six faces ends up drawing from.
//!
//! A block states its texture either as **one string** — the form every
//! declaration in this repository used before there was another one — or as a
//! **table naming a key against each of the six facings** `up`, `down`, `north`,
//! `south`, `east` and `west`. The two forms are the same field, and a block that
//! states one string is a block whose six facings all hold that key. There is no
//! third form and no partial one: a table that is not exactly the six is refused,
//! and those refusals are `luau_declaration_texture_refusals.rs`.
//!
//! # The one-string form is a control here, not a scenario that had to be won
//!
//! It works today, it goes on working, and the reason it is asserted beside the
//! table form is that it is the whole of what makes this change picture-neutral:
//! all four blocks this repository ships state their texture as one string, so
//! every key they resolve to is the key they already resolved to. A failure here
//! and nowhere else says the uniform form was broken while the table form was
//! being written — the one regression this phase can produce that nobody would
//! otherwise be looking for.
//!
//! # A texture key is never the block's own name, and here it is never a
//! neighbour's either
//!
//! The suite-wide rule is [`luau_common`]'s: a fixture whose texture equals its
//! name leaves a loader that read the name into both fields green. The
//! six-different-keys fixture needs a second rule on top of it, because the
//! failure it is written against is subtler — a resolver answering `up`'s key for
//! every facing agrees with any fixture whose six keys are not **pairwise
//! distinct**. So [`six_distinct_keys`] refuses a list that repeats a key or that
//! names the block, as a fixture failure rather than leaving it to an assertion
//! that could not see it.
//!
//! The one place a key is deliberately repeated is the scenario about exactly
//! that: `up` and `down` naming one key while the four sides name four others.
//! There the repetition is the subject, and it is also where the layer budget
//! gets its arithmetic — six facings naming five distinct keys.
//!
//! # A metatable decides nothing about which facings were stated
//!
//! The loader reads a declaration's fields raw, and the two guards at the end of
//! this file extend that property one level down into the texture table. One
//! declaration's metatable offers a facing the table does not hold; another hides
//! a name the table does hold. Neither may be believed — a table that could decide
//! what the loader is allowed to notice about it could decide it was well formed.
//! Both are asserted as a **blame**, not as a sentence: the wording of a
//! per-facing refusal is the refusals suite's subject and is held there against
//! the modding guide, and a second copy of it here would be a second place to
//! disagree.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, Blamed, QUARTZ, SIX_FACINGS, blamed_by, blaming, declaration_of,
    facing_table, facings_of, raw_field, registry_from, text_field,
};
use tempfile::TempDir;

/// The field a texture is stated in.
const TEXTURE_FIELD: &str = "texture";

/// Six texture keys, one per facing, pairwise distinct and none of them the
/// block's own name.
///
/// Listed in [`SIX_FACINGS`] order, which is the order they are written against
/// the six words.
const SIX_KEYS: [&str; 6] = [
    "example:quartz",
    "example:ash",
    "example:basalt",
    "example:chert",
    "example:diorite",
    "example:gabbro",
];

/// The key `up` and `down` share where a declaration names one key twice, and the
/// four the sides name beside it.
const SHARED_BY_TOP_AND_BOTTOM: &str = "example:ash";
const FOUR_SIDE_KEYS: [&str; 4] = [
    "example:basalt",
    "example:chert",
    "example:diorite",
    "example:gabbro",
];

/// The facing a metatable offers a key for without the table holding one.
const WITHHELD_FACING: &str = "up";

/// The name a metatable hides while the table really does hold it.
const HIDDEN_NAME: &str = "top";

/// What reading a content root made of a block's six facings.
///
/// **Total rather than fallible**, for the reason [`Blamed`] is: a scenario about
/// which key a facing holds has to fail on its own comparison when a loader
/// refuses a root it should have accepted. Propagating that with `?` ends the
/// test before its assertion ever runs, and a test that never reached its
/// assertion has not shown it was checking the right thing.
#[derive(Debug, PartialEq, Eq)]
enum Resolved {
    /// The root registered, and each facing holds the key on its line.
    Facings(Vec<String>),
    /// The root was refused, rendered as it renders itself.
    Refused(String),
}

/// What `root` resolved [`AMBER`]'s six facings to.
fn resolved(root: &Path) -> Resolved {
    match registry_from(root).and_then(|registry| facings_of(&registry, AMBER)) {
        Ok(facings) => Resolved::Facings(facings),
        Err(refused) => Resolved::Refused(refused.to_string()),
    }
}

/// A root holding one declaration file, written from `fields`.
fn root_declaring(directory: &TempDir, fields: &[String]) -> Result<PathBuf, Box<dyn Error>> {
    content_root(directory, &[(AMBER_FILE, declaration_of(fields))])
}

/// A declaration stating `texture` as `field`, with the other two required fields
/// correctly stated around it.
fn declaring_texture(field: String) -> Vec<String> {
    vec![text_field("name", AMBER), field, raw_field("solid", "true")]
}

/// The six words paired with `keys`, in [`SIX_FACINGS`] order.
///
/// # Errors
///
/// Returns an error unless the six keys are pairwise distinct and none of them is
/// the block's own name — see this module's header for why a fixture repeating a
/// key could not see the defect it exists to catch.
fn six_distinct_keys(
    keys: [&'static str; 6],
) -> Result<Vec<(&'static str, &'static str)>, Box<dyn Error>> {
    let mut sorted = keys.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != keys.len() {
        return Err(format!(
            "these six facing keys have to be pairwise distinct, or a loader answering one \
             facing's key for every facing agrees with the fixture: {keys:?}"
        )
        .into());
    }
    if keys.contains(&AMBER) {
        return Err(format!(
            "no facing key here may be `{AMBER}`, the block's own name, or a loader reading the \
             name into a facing is right about that facing: {keys:?}"
        )
        .into());
    }
    Ok(SIX_FACINGS.into_iter().zip(keys).collect())
}

/// The six words paired with `keys`, whatever those keys are.
///
/// The unguarded half of [`six_distinct_keys`], for the one fixture whose subject
/// is a key stated twice.
fn keys_against_the_six(keys: [&'static str; 6]) -> Vec<(&'static str, &'static str)> {
    SIX_FACINGS.into_iter().zip(keys).collect()
}

/// What each facing is expected to hold, given `keys` in [`SIX_FACINGS`] order.
fn holding(keys: [&str; 6]) -> Resolved {
    Resolved::Facings(
        SIX_FACINGS
            .into_iter()
            .zip(keys)
            .map(|(word, key)| format!("{word} = {key}"))
            .collect(),
    )
}

/// The six keys a declaration naming one key against `up` and `down` resolves to.
fn shared_at_top_and_bottom() -> [&'static str; 6] {
    [
        SHARED_BY_TOP_AND_BOTTOM,
        SHARED_BY_TOP_AND_BOTTOM,
        FOUR_SIDE_KEYS[0],
        FOUR_SIDE_KEYS[1],
        FOUR_SIDE_KEYS[2],
        FOUR_SIDE_KEYS[3],
    ]
}

/// A metatable whose `__index` answers `key` for any name the table does not
/// hold.
fn an_index_supplying(key: &str) -> String {
    format!("{{\n\t__index = function(_, _) return '{key}' end,\n}}")
}

/// A metatable whose `__iter` reports exactly `shown`, whatever the table holds.
///
/// It reads each key back raw, so what it reports is the table's own value for a
/// key it chose to admit to — a metamethod lying about the values as well would
/// leave it open which lie the loader believed.
fn an_iter_reporting(shown: &[&str]) -> String {
    let listed = shown
        .iter()
        .map(|key| format!("'{key}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n\
         \t__iter = function(self)\n\
         \t\tlocal shown = {{ {listed} }}\n\
         \t\tlocal position = 0\n\
         \t\treturn function()\n\
         \t\t\tposition = position + 1\n\
         \t\t\tlocal key = shown[position]\n\
         \t\t\tif key then return key, rawget(self, key) end\n\
         \t\t\treturn nil\n\
         \t\tend\n\
         \tend,\n\
         }}"
    )
}

/// A root whose texture table states `facings` and carries `metatable`.
///
/// The metatable goes on the **inner** table rather than on the declaration,
/// because what these two guards are about is how the facings are read; the outer
/// table's own metatable is already answered for by `luau_declaration_keys.rs`.
fn root_whose_texture_table_carries(
    directory: &TempDir,
    facings: &[(&str, &str)],
    metatable: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let stated: String = facings
        .iter()
        .map(|(word, key)| format!("\t{word} = '{key}',\n"))
        .collect();
    let chunk = format!(
        "local textures = {{\n{stated}}}\n\
         setmetatable(textures, {metatable})\n\
         return {{\n\
         \tname = '{AMBER}',\n\
         \ttexture = textures,\n\
         \tsolid = true,\n\
         }}\n"
    );
    content_root(directory, &[(AMBER_FILE, chunk)])
}

#[test]
fn a_texture_stated_as_one_string_registers_that_key_on_all_six_facings() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &declaring_texture(text_field(TEXTURE_FIELD, QUARTZ)),
    )?;

    let facings = resolved(&root);

    assert_eq!(
        facings,
        holding([QUARTZ; 6]),
        "one string is six facings holding one key, and every block this repository ships states \
         its texture that way. This is the control that says the picture did not move while the \
         table form was being written: a resolver that only understands tables leaves all four \
         shipped blocks drawing nothing at all"
    );
    Ok(())
}

#[test]
fn a_table_of_six_different_keys_registers_each_facing_with_the_key_written_against_it()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &declaring_texture(facing_table(&six_distinct_keys(SIX_KEYS)?)),
    )?;

    let facings = resolved(&root);

    assert_eq!(
        facings,
        holding(SIX_KEYS),
        "each facing holds the key written against its own word, and the six are pairwise \
         distinct so that a resolver answering `up`'s key for all six has nowhere to be right. \
         This is the assertion the whole per-face feature rests on: a grass block is a block whose \
         top, bottom and sides differ, and if they cannot differ here they cannot differ anywhere"
    );
    Ok(())
}

#[test]
fn a_key_named_against_both_up_and_down_is_registered_against_both() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &declaring_texture(facing_table(&keys_against_the_six(
            shared_at_top_and_bottom(),
        ))),
    )?;

    let facings = resolved(&root);

    assert_eq!(
        facings,
        holding(shared_at_top_and_bottom()),
        "the same key against two facings is the declaration a mod author writes on their first \
         block — dirt above and below, four sides of something else — and nothing about holding \
         six keys may turn naming one of them twice into a refusal"
    );
    Ok(())
}

#[test]
fn a_declarations_metatable_supplies_no_facing_it_did_not_state() -> TestResult {
    let directory = TempDir::new()?;
    let five: Vec<(&str, &str)> = six_distinct_keys(SIX_KEYS)?
        .into_iter()
        .filter(|(word, _)| *word != WITHHELD_FACING)
        .collect();
    let root = root_whose_texture_table_carries(&directory, &five, &an_index_supplying(QUARTZ))?;

    let blamed = blamed_by(&root, AMBER_FILE);

    assert_eq!(
        blamed,
        Blamed::Declaration(blaming(AMBER, TEXTURE_FIELD)),
        "a texture table one facing short is one facing short whatever its `__index` would \
         answer. Believing that metamethod hands a declaration the power to decide what the \
         loader is allowed to notice about it — and the block would then draw a facing nobody \
         wrote, which is the silent loss the raw reads exist to prevent"
    );
    Ok(())
}

#[test]
fn a_declarations_metatable_hides_no_facing_it_did_state() -> TestResult {
    let directory = TempDir::new()?;
    let mut stated = six_distinct_keys(SIX_KEYS)?;
    stated.push((HIDDEN_NAME, QUARTZ));
    let root =
        root_whose_texture_table_carries(&directory, &stated, &an_iter_reporting(&SIX_FACINGS))?;

    let blamed = blamed_by(&root, AMBER_FILE);

    assert_eq!(
        blamed,
        Blamed::Declaration(blaming(AMBER, TEXTURE_FIELD)),
        "the six words are all there and a seventh is too, and an `__iter` reporting only the six \
         must not hide it. The same property as the guard above from the other side: a table that \
         can hide a name it holds can hide the very name that is wrong with it"
    );
    Ok(())
}
