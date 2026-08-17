//! Which entries under `blocks/` are declarations, and in what order the loader
//! reads them.
//!
//! Until now the loader took whatever `blocks/` held, in whatever order the
//! filesystem handed it back. Both halves of that are content-facing contracts
//! rather than incidentals: a mod author keeps notes, textures and
//! subdirectories beside their declarations, and registration order decides
//! which block ends up in a player's hand and which of two files declaring one
//! name is named "first" when the pair is refused.
//!
//! # The order fixture has to disagree with three things, not one
//!
//! Asserting a sort is unusually easy to do vacuously, because two of the three
//! orders a directory of declarations has tend to agree with the answer by
//! accident.
//!
//! * **The filesystem's own listing order.** Measured on this project's
//!   platform: NTFS hands entries back in its own case-insensitive name order,
//!   so a fixture of lowercase names is *already* sorted when it arrives and a
//!   loader that never sorts passes. The third file below exists only to break
//!   that: `_` sorts after every letter under NTFS's collation and before every
//!   lowercase letter under the byte ordering every plausible Rust sort uses, so
//!   the listing order and the required order genuinely differ. Every sort an
//!   implementation might reach for — over `OsStr`, over the whole path, over a
//!   lowercased rendering — puts `_cobalt.luau` first; only *no* sort does not.
//! * **The block-name order.** Two of the files declare the other's name, so a
//!   loader that sorted by what a declaration calls itself rather than by the
//!   file it is in reads them the other way round.
//! * **The creation order.** Recorded rather than relied on: on NTFS it is not
//!   observable at all, which is exactly why the spec's original two-file
//!   fixture could not falsify anything.

mod common;
mod luau_common;

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use common::{TestResult, content_root};
use luau_common::{
    AMBER_FILE, BLOCKS_DIRECTORY, Refusal, declaration_label, declarations_label, declaring,
    refusal_of, registration_order_or_refusal,
};
use tempfile::TempDir;

/// The declaration file whose name sorts before every letter, and which the
/// filesystem nevertheless hands back last.
///
/// See the module note: without it this suite cannot tell a loader that sorts
/// from one that repeats whatever the directory said.
const COBALT_FILE: &str = "_cobalt.luau";

/// The declaration file that declares the *other* file's block.
const ZINC_FILE: &str = "zinc.luau";

/// A subdirectory of `blocks/` named as though it were a declaration.
const A_DIRECTORY_NAMED_LIKE_A_DECLARATION: &str = "nested.luau";

/// A subdirectory of `blocks/` holding a declaration one level too deep.
const A_SUBDIRECTORY: &str = "nested";

/// A file under `blocks/` that is not a declaration, whatever is inside it.
const A_FILE_THAT_IS_NOT_A_DECLARATION: &str = "notes.txt";

/// A root whose file-name order, block-name order and listing order all
/// disagree.
///
/// Created deliberately in an order that is none of the three, so that a reader
/// cannot mistake the expectation for a transcript of how the fixture was
/// written.
fn a_root_whose_orders_disagree(directory: &TempDir) -> Result<PathBuf, Box<dyn Error>> {
    content_root(
        directory,
        &[
            (ZINC_FILE, declaring("example:amber")),
            (COBALT_FILE, declaring("example:cobalt")),
            (AMBER_FILE, declaring("example:zinc")),
        ],
    )
}

#[test]
fn declarations_register_in_file_name_order_rather_than_in_the_order_the_directory_lists_them()
-> TestResult {
    let directory = TempDir::new()?;
    let root = a_root_whose_orders_disagree(&directory)?;

    let registered = registration_order_or_refusal(&root);

    assert_eq!(
        registered,
        Ok(vec![
            "example:cobalt".to_owned(),
            "example:zinc".to_owned(),
            "example:amber".to_owned()
        ]),
        "registration order is the sorted order of the file names — `_cobalt.luau`, \
         `amber.luau`, `zinc.luau` — and the blocks those files declare are deliberately not in \
         that order. A loader that sorted by the name a declaration gives itself reads amber, \
         cobalt, zinc; a loader that never sorts reads whatever the directory said, which on this \
         platform is amber, zinc, cobalt. Only the file-name sort produces the list above"
    );
    Ok(())
}

#[test]
fn only_the_luau_files_directly_under_the_declarations_directory_are_declarations() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(&directory, &[(AMBER_FILE, declaring("example:amber"))])?;
    let declarations = root.join(BLOCKS_DIRECTORY);
    // Perfectly good Luau, in a file that is not a declaration file. Filling it
    // with prose instead would let a loader that reads every entry pass by
    // failing to parse it, which is a different rule than the one under test.
    fs::write(
        declarations.join(A_FILE_THAT_IS_NOT_A_DECLARATION),
        declaring("example:notes"),
    )?;
    let nested = declarations.join(A_SUBDIRECTORY);
    fs::create_dir_all(&nested)?;
    fs::write(nested.join("hidden.luau"), declaring("example:hidden"))?;

    let registered = registration_order_or_refusal(&root);

    assert_eq!(
        registered,
        Ok(vec!["example:amber".to_owned()]),
        "a mod author keeps notes and working files beside their declarations, and a loader that \
         reads every entry either registers blocks nobody declared or refuses a root over a file \
         that was never a declaration. `notes.txt` holds a declaration that would register \
         perfectly well if it were named `.luau`, and `nested/hidden.luau` is one directory too \
         deep — neither is under `blocks/` as a `.luau` file, so neither is read"
    );
    Ok(())
}

#[test]
fn a_content_root_with_no_declarations_directory_is_refused_naming_that_directory() -> TestResult {
    let directory = TempDir::new()?;
    // The root itself, with nothing in it: no `blocks/` at all, which is what a
    // mod author has the first time they point the loader at the wrong place.
    let root = directory.path().to_owned();

    let refusal = refusal_of(&root);

    assert_eq!(
        refusal,
        Refusal::Unreadable {
            path: declarations_label(&root)
        },
        "the refusal has to send a mod author to the directory they did not create, which is \
         `blocks/` under their root and not the root itself — those are different mistakes with \
         different fixes. An unreadable path carries no block, which is the other half of what \
         this scenario asks and what the verdict above rejects every alternative to"
    );
    Ok(())
}

#[test]
fn a_directory_named_like_a_declaration_is_refused_naming_its_path() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(&directory, &[(AMBER_FILE, declaring("example:amber"))])?;
    fs::create_dir_all(
        root.join(BLOCKS_DIRECTORY)
            .join(A_DIRECTORY_NAMED_LIKE_A_DECLARATION),
    )?;

    let refusal = refusal_of(&root);

    assert_eq!(
        refusal,
        Refusal::Unreadable {
            path: declaration_label(&root, A_DIRECTORY_NAMED_LIKE_A_DECLARATION)
        },
        "the root holds one perfectly good declaration, so a loader that quietly skipped the \
         directory would register `example:amber` and refuse nothing — which is why the good \
         declaration is there. A name ending in `.luau` is how a mod author says `this is a \
         declaration`, so an entry wearing that name and not being a file is worth saying out \
         loud rather than passing over in silence"
    );
    Ok(())
}
