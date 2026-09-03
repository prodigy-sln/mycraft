//! What a mod author reads when the client will not accept their blocks.
//!
//! Six shapes of refusal and one root that is fine, all asked the same way: the
//! client's own preparation is given a real content root, and what it refuses
//! with is written out through the client's own reporting into a sink this file
//! holds. **The assertion is on the text in that sink**, because the text is the
//! whole of the capability — a refusal that names the file, the block and the
//! field inside an error value nobody prints leaves an author bisecting their
//! content one file at a time.
//!
//! **Not every part is there every time, and that is the contract.** A duplicate
//! name is about two files and no field; an empty `blocks/` is about the root and
//! neither; a file that never compiles has no field to read a name out of. So
//! three of the six ask for the absence of a part as firmly as the others ask for
//! its presence — a refusal inventing a block name it does not know would be the
//! same defect pointing the other way.
//!
//! **Each expectation is compared whole as well as searched.** Searching alone
//! cannot tell a clean rendering from one that appended a separator to an empty
//! layer, and comparing alone would go on agreeing if the loader quietly stopped
//! filling in the block or the field — so the words a scenario is about are asked
//! for one by one *and* the whole of what was written is compared against the
//! refusal the loader itself produced.
//!
//! Reached without a GPU and without a display server: the preparation is the
//! seam a test can hold, and a refused declaration is only collected at the first
//! redraw of a real run.

mod support;

use std::error::Error;
use std::path::Path;

use mc_core::block::RegistryError;
use mc_core::block::source::DefinitionSourceError;
use mc_render::window::Ending;

use support::{TestResult, content};

/// The file every block declaration below is written into.
///
/// Distinctive on purpose: it is the needle a refusal that genuinely names the
/// declaration it could not accept has to carry, and no message that merely says
/// the content could not be read can hold it by accident.
const REFUSED_FILE: &str = "amber.luau";

/// A second file declaring the same block as the first.
///
/// It sorts *before* `amber.luau`, so a refusal about the pair names this one as
/// the first declaration and that one as the second — which is only well defined
/// because a content root is read in file-name order.
const SECOND_FILE: &str = "amber-copy.luau";

/// The name a well-formed declaration below gives itself.
///
/// An `example:` namespace rather than `base:`, because a fixture borrowing a
/// shipped block's name would be the test describing the engine in terms of the
/// content it ships.
const REFUSED_BLOCK: &str = "example:amber";

/// A name that is not a namespaced id: two separators, so there is neither one
/// namespace nor one path in it.
const NOT_A_NAMESPACED_ID: &str = "example:amber:top";

/// A field no loader recognises, spelled close enough to a real one to be the
/// typo a mod author actually makes.
const UNRECOGNISED_FIELD: &str = "slid";

/// The field a declaration names itself in.
const NAME_FIELD: &str = "name";

/// A declaration well formed apart from the name it gives itself.
const NAMED_WRONGLY: &str =
    "return {\n\tname = 'example:amber:top',\n\ttexture = 'example:amber',\n\tsolid = true,\n}\n";

/// A declaration whose three well-formed fields sit beside one nobody
/// recognises.
const CARRYING_AN_UNRECOGNISED_FIELD: &str = "return {\n\tname = 'example:amber',\n\ttexture = 'example:amber',\n\tsolid = true,\n\tslid = \
     true,\n}\n";

/// A declaration with nothing wrong with it, written twice so that what is wrong
/// is the pair rather than either one of them.
const WELL_FORMED: &str =
    "return {\n\tname = 'example:amber',\n\ttexture = 'example:amber',\n\tsolid = true,\n}\n";

/// A file that is not a chunk at all, so nothing ever returns a table and there
/// is no field to read a name out of: the whole file is what is wrong.
///
/// It is broken as *syntax* rather than by returning the wrong thing, so what
/// reaches the author is the compiler's own complaint carried out through the
/// loader — which is the case where a fixture is least able to guess the wording
/// and most needs to read it off the run.
const NOT_A_CHUNK: &str = "this is not a chunk at all\n";

/// How a refusal spells the block it is about, and how it spells the field.
///
/// Named here so that the scenarios requiring *no* block and *no* field have
/// something to look for the absence of that a path or a sentence cannot hold by
/// accident — unlike the bare words, which the prose around them contains.
const BLOCK_CLAUSE: &str = ", block `";
const FIELD_CLAUSE: &str = ", field `";

/// What the client says above a refusal that came out of the content root: the
/// layer the rest of the chain hangs from, and the whole of what a mod author
/// reads today.
const CONTENT_REFUSAL: &str = "the shipped content could not be read";

#[test]
fn a_block_named_wrongly_is_refused_naming_the_file_the_block_and_the_field() -> TestResult {
    let root = content::shipped_copy()?.declaring_block(REFUSED_FILE, NAMED_WRONGLY)?;
    let whole = everything_written_for(&content::block_refusal_over(root.path())?);

    let said = support::refusal_printed_over(root.path())?;

    assert_eq!(
        (
            said.contains(&declaration_path(REFUSED_FILE)),
            said.contains(NOT_A_NAMESPACED_ID),
            said.contains(NAME_FIELD),
            said.as_str(),
        ),
        (true, true, true, whole.as_str()),
        "a mod author who mistyped a block's name reads the file it is in, the name as they wrote \
         it, and the field it sits under. The three are asked for one by one because the whole \
         comparison beside them would go on agreeing if the loader stopped filling any of them \
         in; the whole comparison is asked because `{NAME_FIELD}` is a word a sentence about \
         namespaces holds by accident"
    );
    Ok(())
}

#[test]
fn a_block_carrying_an_unrecognised_field_is_refused_naming_that_field() -> TestResult {
    let root =
        content::shipped_copy()?.declaring_block(REFUSED_FILE, CARRYING_AN_UNRECOGNISED_FIELD)?;
    let whole = everything_written_for(&content::block_refusal_over(root.path())?);

    let said = support::refusal_printed_over(root.path())?;

    assert_eq!(
        (
            said.contains(&declaration_path(REFUSED_FILE)),
            said.contains(REFUSED_BLOCK),
            said.contains(UNRECOGNISED_FIELD),
            said.as_str(),
        ),
        (true, true, true, whole.as_str()),
        "which slot carries the field name is not the author's business and is not asked here — \
         it moved when the reader did, from inside a parser's own diagnostic to the typed field \
         beside it, and neither is a fact a mod author has any use for. What is asked is that the \
         file, the block and the word `{UNRECOGNISED_FIELD}` all reach the person who typed it"
    );
    Ok(())
}

#[test]
fn two_files_declaring_one_block_are_refused_naming_both_files_and_the_name() -> TestResult {
    let root = content::shipped_copy()?
        .declaring_block(REFUSED_FILE, WELL_FORMED)?
        .declaring_block(SECOND_FILE, WELL_FORMED)?;
    let whole = everything_written_for(&content::block_refusal_over(root.path())?);

    let said = support::refusal_printed_over(root.path())?;

    assert_eq!(
        (
            said.contains(&declaration_path(REFUSED_FILE)),
            said.contains(&declaration_path(SECOND_FILE)),
            said.contains(REFUSED_BLOCK),
            said.contains(FIELD_CLAUSE),
            said.as_str(),
        ),
        (true, true, true, false, whole.as_str()),
        "neither file is wrong on its own, so what a refusal has to hand over is both of them and \
         the name they both claim — `this name is taken` sends an author through every file they \
         have. There is no field at fault here and none is invented: a refusal naming one would \
         send them to a line that is fine"
    );
    Ok(())
}

#[test]
fn a_root_declaring_no_block_at_all_is_refused_naming_the_root_and_nothing_else() -> TestResult {
    let root = content::shipped_copy()?.declaring_no_blocks()?;
    let whole = everything_written_for(&content::block_refusal_over(root.path())?);

    let said = support::refusal_printed_over(root.path())?;

    assert_eq!(
        (
            said.contains(&root.path().display().to_string()),
            said.contains(BLOCK_CLAUSE),
            said.contains(FIELD_CLAUSE),
            said.as_str(),
        ),
        (true, false, false, whole.as_str()),
        "a root that declares nothing is refused about the root, which is the only thing there is \
         to say: there is no block and no field, and a refusal that named either would be naming \
         something nobody wrote. What the author needs is the directory the client looked in"
    );
    Ok(())
}

#[test]
fn a_block_file_that_will_not_compile_is_refused_naming_the_file_and_the_reason() -> TestResult {
    let root = content::shipped_copy()?.declaring_block(REFUSED_FILE, NOT_A_CHUNK)?;
    let refused = content::block_refusal_over(root.path())?;
    let reason = stated_reason_in(&refused)?;
    let whole = everything_written_for(&refused);

    let said = support::refusal_printed_over(root.path())?;

    assert_eq!(
        (
            said.contains(&declaration_path(REFUSED_FILE)),
            said.contains(&reason),
            said.contains(BLOCK_CLAUSE),
            said.contains(FIELD_CLAUSE),
            said.as_str(),
        ),
        (true, true, false, false, whole.as_str()),
        "the reason comes from the run rather than from this file: a test spelling a compiler's \
         complaint out by hand would be asserting that wording rather than that any of it reached \
         the author. A file that never compiles returned no table, so there is no name to read out \
         of it — the whole file is what is named, and no block and no field are"
    );
    Ok(())
}

#[test]
fn a_missing_content_root_is_refused_naming_the_directory_that_was_looked_for() -> TestResult {
    let looked_for = Path::new("content").join("base").display().to_string();
    let refused = match mc_client::startup::shipped_content() {
        Ok(found) => return Err(no_refusal_from(&found.display().to_string())),
        Err(refused) => refused,
    };
    let whole = format!("mycraft: {refused}\n");

    let said = support::reported(&Ending::failed(&refused))?;

    assert_eq!(
        (said.contains(&looked_for), said.as_str()),
        (true, whole.as_str()),
        "somebody running the client from the wrong directory is told which directory was looked \
         in. There is nothing beneath this refusal, so what is written ends with its own sentence \
         — a separator and an empty layer after it would be the rendering describing a cause that \
         does not exist"
    );
    Ok(())
}

#[test]
fn the_shipped_content_registers_one_block_for_each_declaration_and_says_nothing() -> TestResult {
    let root = support::content_root()?;
    let declared = content::block_declarations_in(&root.join(content::BLOCK_DIRECTORY))?.len();

    let prepared = mc_client::startup::prepare_scene(&root);
    let (registered, said) = match &prepared {
        Ok(scene) => (
            scene.registry.registered_count(),
            support::reported(&Ending::Closed)?,
        ),
        Err(refused) => (0, support::reported(&Ending::failed(refused))?),
    };

    assert_eq!(
        (registered, said.as_str()),
        (declared, ""),
        "the count is derived by counting the declaration files rather than written down, so a \
         block added to the shipped game changes it correctly and no number here can be one \
         snapshotted from a run. A root the client accepts is a root it says nothing about, which \
         is why what it said is compared beside the count"
    );
    Ok(())
}

/// The whole of what the client writes for a run the content root refused,
/// derived from the refusal the registry itself produced rather than restated
/// here.
fn everything_written_for(refused: &RegistryError) -> String {
    format!("mycraft: {CONTENT_REFUSAL}: {refused}\n")
}

/// How a declaration file is written into a refusal — as a path, so the needle is
/// spelled the way this platform spells one.
fn declaration_path(file_name: &str) -> String {
    Path::new(content::BLOCK_DIRECTORY)
        .join(file_name)
        .display()
        .to_string()
}

/// The reason the run gave, read out of `refused` rather than spelled here.
///
/// # Errors
///
/// Returns an error if the refusal is not about a declaration that could not be
/// read, or if it carries no reason — in either case the scenario has no reason
/// to look for, and searching for an empty one would find it everywhere.
fn stated_reason_in(refused: &RegistryError) -> Result<String, Box<dyn Error>> {
    let RegistryError::Source(DefinitionSourceError::Malformed(fault)) = refused else {
        return Err(format!(
            "this scenario needs a declaration that could not be read, and what was refused \
             was: {refused}"
        )
        .into());
    };
    if fault.cause.is_empty() {
        return Err("the declaration was refused and no reason was given for it".into());
    }
    Ok(fault.cause.clone())
}

/// What to say when a scenario needing a refusal did not get one.
fn no_refusal_from(found: &str) -> Box<dyn Error> {
    format!(
        "this scenario needs the client to find no content root beside it, and it found one at \
         {found}. A test binary runs in its own package directory, where the shipped root is \
         deliberately not reachable"
    )
    .into()
}
