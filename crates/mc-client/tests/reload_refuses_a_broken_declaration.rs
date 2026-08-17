//! What a mod author reads when the file they just saved will not load, and what
//! the game goes on serving while they fix it.
//!
//! # These are the refusals a launch already produces, reached through a reload
//!
//! Not one of them is new. A chunk that will not compile, a misspelled field, two
//! files claiming one name and an emptied `blocks/` are refused by
//! `mc_sim::content::load` today, naming the file, the block and the field, and a
//! reload's build stage is that same call on a worker. So these four are **controls
//! on the path** rather than tests of new vocabulary: they redden exactly if the
//! reload comes to reach content through something else, and a new fault type
//! written to satisfy one of them is the signal that it has.
//!
//! # No refusal's wording is spelled here
//!
//! Each expectation is asked of a second read of the same root, which reaches the
//! failure without going near the reload, and the reported text has to **end** in
//! it — so whatever framing a reload puts above a refusal, the sentence naming the
//! file survives to the person. A reworded refusal moves both sides together; a
//! *dropped* cause moves only the reported side, which is the asymmetry a
//! snapshotted string does not have. That defect is not hypothetical: this project
//! has shipped a report that flattened a typed failure at the moment it printed it,
//! with the file, the block and the field alive in the value and absent from the
//! terminal.
//!
//! What each scenario adds on top is the words it requires by name, because a
//! comparison against the whole chain would go on agreeing if the loader quietly
//! stopped filling one of them in.
//!
//! # Every one of them says what the game is still serving
//!
//! A refusal that lost the content would be a worse outcome than the broken file,
//! so each assertion carries the four blocks the client is still serving beside the
//! refusal. That half is what tells a refusal apart from a half-applied candidate.
//!
//! # What is not asserted here, and where it is held
//!
//! The client renders the refusal and hands it over; the frame path prints it. The
//! printing is `App`'s, which needs a real window and which nothing in this
//! workspace runs, so it is held by review exactly as the two recurring faults
//! beside it already are. What is asserted is the text where it crosses out of the
//! client's core, which is the value the printing is handed.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use reload::{AMBER_FILE, GRASS, STONE, STONE_FILE, shipped};
use reload_watch::{
    MISSPELLED_SOLID, NOT_A_CHUNK, Refusal, STONE_MISSPELLING_SOLID, a_client_on, block_path,
    declaration_named, named_in_order, naming, refusal, restating_raw, serving,
    the_four_shipped_blocks, the_loaders_own_words, until_settled,
};
use support::TestResult;

/// A second declaration claiming a name the shipped root already declares.
///
/// Its file sorts before `stone.luau`, so a refusal that names the two in file-name
/// order names this one first — which is the order the scenario is about and the
/// only one that is well defined.
const A_SECOND_STONE: &str =
    "return {\n\tname = \"base:stone\",\n\ttexture = \"base:stone\",\n\tsolid = true,\n}\n";

#[test]
fn a_declaration_that_will_not_compile_leaves_the_content_serving_and_names_the_file() -> TestResult
{
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = root.declaring_block(AMBER_FILE, NOT_A_CHUNK)?;
    let words = the_loaders_own_words(root.path())?;

    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (
            refusal(&crossed, &words, &naming(&[&declaration_named(AMBER_FILE)])),
            serving(&client)?
        ),
        (Refusal::NamedEverythingAsked, the_four_shipped_blocks()),
        "a file that never compiled returned no table, so there is no name to read out of it and \
         the whole file is what is named — with the compiler's own complaint carried underneath, \
         which is why the comparison is against a second read rather than against a diagnostic \
         spelled out here. The blocks beside it are what says a refused candidate cost the author \
         nothing but the save"
    );
    Ok(())
}

#[test]
fn a_misspelled_field_leaves_the_content_serving_and_names_the_file_block_and_field() -> TestResult
{
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating_raw(root, STONE_FILE, STONE_MISSPELLING_SOLID)?;
    let words = the_loaders_own_words(root.path())?;

    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (
            refusal(
                &crossed,
                &words,
                &naming(&[&declaration_named(STONE_FILE), STONE, MISSPELLED_SOLID])
            ),
            serving(&client)?
        ),
        (Refusal::NamedEverythingAsked, the_four_shipped_blocks()),
        "`{MISSPELLED_SOLID}` is the typo, and an author who reads only that a required field is \
         missing goes looking for the wrong line — the loader checks that every field it was given \
         is one it knows *before* it reads any of them, for exactly this reason. All three of the \
         file, the block and the word they typed have to reach them"
    );
    Ok(())
}

#[test]
fn two_files_claiming_one_block_leave_the_content_serving_and_name_both_in_file_name_order()
-> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = root.declaring_block(AMBER_FILE, A_SECOND_STONE)?;
    let words = the_loaders_own_words(root.path())?;
    let (first, second) = (declaration_named(AMBER_FILE), declaration_named(STONE_FILE));
    let both = naming(&[&first, &second, STONE]);

    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (
            refusal(&crossed, &words, &both),
            named_in_order(&crossed, &first, &second),
            serving(&client)?
        ),
        (
            Refusal::NamedEverythingAsked,
            true,
            the_four_shipped_blocks()
        ),
        "neither file is wrong on its own, so what the author needs is both of them and the name \
         they both claim — `this name is taken` sends them through every file they have. The order \
         is asked for separately because a search for each of two names cannot see which came \
         first, and file-name order is the only order a root is read in"
    );
    Ok(())
}

#[test]
fn a_blocks_directory_emptied_of_declarations_leaves_the_content_serving_and_names_the_root()
-> TestResult {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = root.declaring_no_blocks()?;
    let words = the_loaders_own_words(root.path())?;

    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let crossed = until_settled(&mut client);

    assert_eq!(
        (
            refusal(
                &crossed,
                &words,
                &naming(&[&root.path().display().to_string()])
            ),
            serving(&client)?
        ),
        (Refusal::NamedEverythingAsked, the_four_shipped_blocks()),
        "a root that declares nothing is refused about the root, because there is no block and no \
         field to be about — and the directory the client looked in is the whole of what the author \
         needs. The path reported is the one the run was given, which is what makes this the \
         refusal somebody who deleted a directory actually reads"
    );
    Ok(())
}
