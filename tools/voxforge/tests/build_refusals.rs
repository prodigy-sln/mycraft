//! A manifest the tool cannot honour refuses the build whole, naming what is
//! wrong.
//!
//! Every one of these is read off **stderr and the exit code together**. A
//! build that said the right words and exited zero would let the gate carry on
//! and grade the set the previous build left behind, which is the one thing the
//! whole-set rule exists to prevent — so `Refused` treats a successful exit as
//! *not refused at all*, whatever was printed.
//!
//! The tokens each refusal is required to name are values, not sentences:
//! a path, a key, a face name, a derived file name. Two exceptions are pinned
//! spellings and each says so where it is declared, because two refusals about
//! one field are told apart by their words and a mapping is decidable only
//! where its spelling is pinned.

#[path = "common/build.rs"]
mod build;
mod common;

use std::error::Error;

use common::TestResult;
use common::texture::GREY;

use build::{
    CUBE_MODEL, FIRST_KEY, MANIFEST_FILE, Refused, Root, built, built_from, entry, image_named,
    manifest, root_of_one_cube,
};

/// A manifest whose third line is not TOML at all.
const A_MANIFEST_THAT_IS_NOT_TOML: &str =
    "output = \"textures\"\nmaterials = \"materials\"\nthis is not toml\n";

/// Where a reader is told parsing stopped in that manifest.
const WHERE_PARSING_STOPS: &str = "line 3";

/// The six faces a manifest entry may select.
const FACES_A_MANIFEST_MAY_SELECT: [&str; 6] = ["front", "back", "left", "right", "top", "bottom"];

/// The sentence a namespaced id refuses a second separator with.
///
/// A pinned spelling, and it is the existing one: this refusal is the parse
/// rule every namespaced id in the project already carries, and a manifest that
/// invented its own words for it would be a second rule to keep in step.
const TOO_MANY_SEPARATORS: &str = "more than one namespace separator";

/// The word a refusal about a key that cannot be written down carries.
///
/// A pinned spelling. Both this refusal and the file-name one are about a key
/// and are raised in the same place, so the only thing separating them is what
/// they say: this one names the file the key cannot be recorded in, and the
/// other names the image file name the key would have produced.
const UNWRITABLE_TO_AN_INDEX: &str = "index";

/// A key whose image file name would carry a path separator.
const A_KEY_THAT_IS_NOT_A_FILE_NAME: &str = "base:a/b";

/// A key carrying a line break, spelled as a manifest spells one.
const A_KEY_CARRYING_A_LINE_BREAK: &str = "base:line\\nbreak";

/// The part of that key a refusal can quote back on one line.
const THE_READABLE_HALF_OF_IT: &str = "base:line";

/// What a build of a root whose manifest is `text` did.
struct Outcome {
    /// How it refused, and what its words left unnamed.
    verdict: Refused,
    /// Which images it left behind.
    images: Vec<String>,
    /// What it said, for a failure to quote.
    err: String,
}

/// What building the fixture root under manifest `text` did.
fn building(text: &str, expected: &[&str]) -> Result<Outcome, Box<dyn Error>> {
    let root = root_of_one_cube(GREY)?;
    root.holding(MANIFEST_FILE, text)?;
    let made = built(&root)?;
    Ok(Outcome {
        verdict: made.refusal(expected),
        images: made.images(),
        err: made.err.clone(),
    })
}

/// A manifest baking one face of the fixture cube against `key`.
fn baking(key: &str, face: &str) -> String {
    manifest(1, &[entry(key, CUBE_MODEL, face)])
}

#[test]
fn a_manifest_path_naming_no_file_refuses_the_build_naming_the_path_given() -> TestResult {
    let root = Root::bare()?;
    let nowhere = root.path().join("nowhere.toml");
    let spelled = nowhere.display().to_string();

    let made = built_from(&nowhere, &root.output())?;

    assert_eq!(
        (made.refusal(&[&spelled]), made.images()),
        (Refused::NamingEverything, Vec::new()),
        "the path quoted back is the one that was typed, not the directory it sits in or a \
         default the tool substituted — whoever ran this has a typo to find and the message is \
         where they find it. It said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_manifest_that_is_not_toml_refuses_the_build_reporting_where_parsing_stopped() -> TestResult {
    let made = building(
        A_MANIFEST_THAT_IS_NOT_TOML,
        &[MANIFEST_FILE, WHERE_PARSING_STOPS],
    )?;

    assert_eq!(
        (made.verdict, made.images),
        (Refused::NamingEverything, Vec::new()),
        "a manifest is a file somebody typed, so the refusal says which file and where in it \
         reading stopped. Nothing is written: an art build that emitted three images and then \
         found the fourth entry unreadable would leave a set that is neither the old one nor a \
         new one. It said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_model_file_that_does_not_exist_refuses_the_build_naming_the_path_and_the_key() -> TestResult {
    let text = manifest(
        1,
        &[
            entry(FIRST_KEY, CUBE_MODEL, "front"),
            entry("base:absent", "models/missing.mcvox", "top"),
        ],
    );

    let made = building(&text, &["missing.mcvox", "base:absent"])?;

    assert_eq!(
        (made.verdict, made.images),
        (Refused::NamingEverything, Vec::new()),
        "the model and the key both, because a manifest of seven entries naming one missing file \
         leaves an author reading seven lines to find out which one is wrong. The first entry is \
         perfectly good and is not written either — the set is all or nothing. It said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_face_that_is_not_one_of_the_six_refuses_the_build_naming_the_six_selectable() -> TestResult {
    let mut owed = vec!["side"];
    owed.extend(FACES_A_MANIFEST_MAY_SELECT);

    let made = building(&baking(FIRST_KEY, "side"), &owed)?;

    assert_eq!(
        (made.verdict, made.images),
        (Refused::NamingEverything, Vec::new()),
        "`side` is a word a block has and a model does not, so the refusal offers the six a \
         manifest may actually select. Six and not sixteen: the isometric views a preview can be \
         rendered from are not faces, and listing them here would send an author to write one. \
         It said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn two_entries_naming_one_texture_key_refuse_the_build_naming_the_key_stated_twice() -> TestResult {
    let text = manifest(
        1,
        &[
            entry(FIRST_KEY, CUBE_MODEL, "front"),
            entry(FIRST_KEY, CUBE_MODEL, "top"),
        ],
    );

    let made = building(&text, &[FIRST_KEY])?;

    assert_eq!(
        (made.verdict, made.images),
        (Refused::NamingEverything, Vec::new()),
        "one key names one image, so two entries claiming it is a manifest whose author meant \
         something the file cannot say. Letting the later entry win would bake whichever face \
         came second and say nothing at all. It said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_key_with_two_namespace_separators_refuses_the_build_reporting_that() -> TestResult {
    let made = building(
        &baking("base:grass:top", "front"),
        &["base:grass:top", TOO_MANY_SEPARATORS],
    )?;

    assert_eq!(
        (made.verdict, made.images),
        (Refused::NamingEverything, Vec::new()),
        "`base:grass:top` is what an author writes meaning a face of a block, and splitting on \
         the first separator would turn it into the plausible-looking path `grass:top` that \
         resolves to nothing. The rule is the project's own and the refusal is its own sentence, \
         not a second wording of it. It said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_key_whose_image_name_would_not_be_an_ordinary_file_name_refuses_the_build() -> TestResult {
    let owed = image_named(A_KEY_THAT_IS_NOT_A_FILE_NAME);

    let made = building(
        &baking(A_KEY_THAT_IS_NOT_A_FILE_NAME, "front"),
        &[A_KEY_THAT_IS_NOT_A_FILE_NAME, &owed],
    )?;

    assert_eq!(
        (made.verdict, made.images),
        (Refused::NamingEverything, Vec::new()),
        "a key has no character set, so deriving a file name from one is deriving a path from \
         unconstrained content text. The refusal names both the key and the name it would have \
         been written under — the second is what tells an author the rule rather than only that \
         they broke it, and it is also what tells this refusal apart from the one about a key \
         that cannot be recorded at all. It said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_key_carrying_a_line_break_refuses_the_build_as_unwritable_to_an_index() -> TestResult {
    let made = building(
        &baking(A_KEY_CARRYING_A_LINE_BREAK, "front"),
        &[THE_READABLE_HALF_OF_IT, UNWRITABLE_TO_AN_INDEX],
    )?;

    assert_eq!(
        (made.verdict, made.images),
        (Refused::NamingEverything, Vec::new()),
        "a key carrying a line break is a whole forged record: written down, `base:line` followed \
         by whatever the author put after the break is an index a client reads with a fold nobody \
         folded or a source nobody consumed. It is refused here, where an author can still see \
         their own manifest, as well as by the renderer that would emit it. It said: {err}",
        err = made.err
    );
    Ok(())
}
