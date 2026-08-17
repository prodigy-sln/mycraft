//! How many declarations a content root may hold, how large one of them may be,
//! and which refusal wins when a root breaks a bound and holds a broken file
//! too.
//!
//! # Both of these are content-supplied quantities, and neither had a bound
//!
//! A directory listing is an allocation whose size a mod author chooses, and a
//! declaration file is read into memory in full before a line of it is
//! evaluated. Under the format this loader replaces, the TOML parser and the
//! filesystem supplied the practical limits; under this one nothing does until
//! it is said out loud.
//!
//! # What these tests are about is the **order** the checks run in
//!
//! Two of them would be satisfied by a loader that bounds nothing and merely
//! happens to refuse, which is why each fixture breaks a bound *and* holds
//! something else worth refusing:
//!
//! * A root of one declaration too many, one of which is not valid Luau at all.
//!   A loader that reads first and counts afterwards refuses the broken file and
//!   never mentions the count — and has already spent the whole listing doing
//!   it, which is the cost the bound exists to pre-empt.
//! * A file well past the size bound whose first line is a syntax error. A
//!   loader that opens the file and hands it to the scripting host reports the
//!   syntax error faithfully, which is a true statement about the wrong problem:
//!   the file was never going to be read, and saying so is what tells its author
//!   to look at its size rather than at its text.
//!
//! So each of those two asserts the refusal it must give **and** the refusal it
//! must not, and the second half is derived rather than written down: the broken
//! declaration is loaded once on its own, in a file small enough to be read, and
//! the reason that produces is what the oversized file's refusal must not
//! contain.
//!
//! # The accepting side is half of every bound
//!
//! A bound stated only from the refusing side leaves `>` and `>=`
//! indistinguishable, so exactly 4,096 declarations and a file of exactly
//! 256 KiB each have a test of their own. Both state the fixture's measured size
//! in the same comparison as the outcome, because a padding helper that quietly
//! produced 262,143 bytes would make the accepting test pass for a reason
//! nothing here would report.

mod common;
mod luau_common;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, BLOCKS_DIRECTORY, Refusal, declaration_label, declarations_label, declaring,
    mentioning, refusal_and_cause, registration_order_or_refusal, well_formed_amber,
};
use tempfile::TempDir;

/// How many declaration files one content root may hold.
const DECLARATIONS_A_ROOT_MAY_HOLD: usize = 4_096;

/// One more than that, which is the smallest root the bound refuses.
const ONE_DECLARATION_TOO_MANY: usize = DECLARATIONS_A_ROOT_MAY_HOLD + 1;

/// How many bytes one declaration file may hold.
const BYTES_A_DECLARATION_FILE_MAY_HOLD: usize = 256 * 1024;

/// A file size far enough past the bound that "slightly over" and "far over"
/// are visibly different quantities in the refusal.
const A_FILE_WELL_PAST_THAT: usize = 300 * 1024;

/// A declaration file that will not compile, and whose first line is where it
/// goes wrong.
///
/// Deliberately short. Its refusal is captured from a root small enough to be
/// read, and that captured text is the needle the oversized file's refusal must
/// not carry — so nothing here writes down a word of the backend's diagnostic.
const A_DECLARATION_THAT_WILL_NOT_COMPILE: &str = "local broken = =\n";

/// What the invalid declaration is called where a root holds one beside its
/// good ones.
const A_BROKEN_FILE: &str = "broken.luau";

/// A subdirectory wearing a declaration's name, placed in a root that is
/// already over the count bound.
///
/// Named to sort first under every ordering, so that a loader checking entries
/// to be files before it counts them meets this one immediately.
const A_DIRECTORY_NAMED_LIKE_A_DECLARATION: &str = "_nested.luau";

/// The file the `position`-th generated declaration is written to.
///
/// Zero-padded so that file-name order and numeric order are the same order,
/// which is what lets the accepting test state the whole registration order it
/// expects rather than only how many blocks arrived.
fn generated_file(position: usize) -> String {
    format!("block_{position:04}.luau")
}

/// The block the `position`-th generated declaration declares.
fn generated_name(position: usize) -> String {
    format!("example:block_{position:04}")
}

/// The names a root of `count` generated declarations registers, in order.
///
/// Arithmetic in this file rather than a list read back from a run: an expected
/// order taken from the loader agrees with whatever order the loader produces.
fn every_generated_name(count: usize) -> Vec<String> {
    (0..count).map(generated_name).collect()
}

/// A content root holding `count` well-formed declarations and nothing else.
fn root_of_declarations(directory: &TempDir, count: usize) -> Result<PathBuf, Box<dyn Error>> {
    let root = directory.path().to_owned();
    let declarations = root.join(BLOCKS_DIRECTORY);
    fs::create_dir_all(&declarations)?;
    for position in 0..count {
        fs::write(
            declarations.join(generated_file(position)),
            declaring(&generated_name(position)),
        )?;
    }
    Ok(root)
}

/// How many entries `root`'s declarations directory holds, counted off the disk.
///
/// Asserted alongside every outcome below, because a fixture builder that wrote
/// one file fewer than it was asked for would make a count-bound test pass for a
/// reason no assertion here would otherwise report.
fn entries_under(root: &Path) -> Result<usize, Box<dyn Error>> {
    Ok(fs::read_dir(root.join(BLOCKS_DIRECTORY))?.count())
}

/// How many bytes `file` holds under `root`, read off the disk.
fn bytes_of(root: &Path, file: &str) -> Result<usize, Box<dyn Error>> {
    Ok(usize::try_from(
        fs::metadata(root.join(BLOCKS_DIRECTORY).join(file))?.len(),
    )?)
}

/// `chunk` followed by a trailing comment, padded so the whole is exactly
/// `bytes` long.
///
/// A comment rather than more code: the subject is how large a file is, and
/// padding it with statements would make the fixture also a test of how much
/// work a chunk may do. A comment after a `return` is still a legal chunk,
/// because a comment is not a statement.
fn padded_to(chunk: &str, bytes: usize) -> Result<String, Box<dyn Error>> {
    let opener = "\n-- ";
    let filler = bytes
        .checked_sub(chunk.len() + opener.len() + 1)
        .ok_or("this chunk is already longer than the size it is being padded to")?;
    Ok(format!("{chunk}{opener}{}\n", "x".repeat(filler)))
}

/// The two quantities a refusal about the declaration count owes its reader.
fn the_count_and_its_bound() -> [String; 2] {
    [
        ONE_DECLARATION_TOO_MANY.to_string(),
        DECLARATIONS_A_ROOT_MAY_HOLD.to_string(),
    ]
}

/// The two quantities a refusal about a file's size owes its reader.
fn the_size_and_its_bound() -> [String; 2] {
    [
        A_FILE_WELL_PAST_THAT.to_string(),
        BYTES_A_DECLARATION_FILE_MAY_HOLD.to_string(),
    ]
}

/// Those quantities as needles, sorted the way [`mentioning`] answers.
fn both_stated(quantities: &[String; 2]) -> Vec<String> {
    let mut stated = quantities.to_vec();
    stated.sort();
    stated
}

/// What became of a root that is over the count bound and holds a broken file.
#[derive(Debug, PartialEq, Eq)]
struct OverTheCount {
    entries: usize,
    refused_as: Refusal,
    quantities_named: Vec<String>,
}

/// What became of an oversized file, and of the same broken declaration in a
/// file small enough to be read.
#[derive(Debug, PartialEq, Eq)]
struct OverTheSize {
    the_readable_copy_was_refused_as: Refusal,
    the_readable_copy_gave_a_reason: bool,
    bytes_on_disk: usize,
    refused_as: Refusal,
    quantities_named: Vec<String>,
    names_what_the_readable_copy_named: bool,
}

/// What a root over the count bound answered, beside what it owes.
///
/// `intruder` is the entry the refusal must **not** name — the thing a loader
/// checking in the wrong order would report instead of the count.
fn over_the_count(
    root: &Path,
    intruder: &str,
) -> Result<(OverTheCount, OverTheCount), Box<dyn Error>> {
    let quantities = the_count_and_its_bound();
    let needles: [&str; 3] = [&quantities[0], &quantities[1], intruder];
    let (refused_as, cause) = refusal_and_cause(root);
    Ok((
        OverTheCount {
            entries: entries_under(root)?,
            refused_as,
            quantities_named: mentioning(&cause, &needles),
        },
        OverTheCount {
            entries: ONE_DECLARATION_TOO_MANY,
            refused_as: Refusal::Malformed {
                path: declarations_label(root),
                block: None,
            },
            quantities_named: both_stated(&quantities),
        },
    ))
}

/// What the two copies of one broken declaration answered, beside what they
/// owe.
fn over_the_size(small: &Path, large: &Path) -> Result<(OverTheSize, OverTheSize), Box<dyn Error>> {
    let (the_readable_copy_was_refused_as, compiler_said) = refusal_and_cause(small);
    let quantities = the_size_and_its_bound();
    let needles: [&str; 2] = [&quantities[0], &quantities[1]];
    let (refused_as, cause) = refusal_and_cause(large);
    Ok((
        OverTheSize {
            the_readable_copy_was_refused_as,
            the_readable_copy_gave_a_reason: !compiler_said.is_empty(),
            bytes_on_disk: bytes_of(large, AMBER_FILE)?,
            refused_as,
            quantities_named: mentioning(&cause, &needles),
            names_what_the_readable_copy_named: !compiler_said.is_empty()
                && cause.contains(&compiler_said),
        },
        OverTheSize {
            the_readable_copy_was_refused_as: Refusal::Malformed {
                path: declaration_label(small, AMBER_FILE),
                block: None,
            },
            the_readable_copy_gave_a_reason: true,
            bytes_on_disk: A_FILE_WELL_PAST_THAT,
            refused_as: Refusal::Malformed {
                path: declaration_label(large, AMBER_FILE),
                block: None,
            },
            quantities_named: both_stated(&quantities),
            names_what_the_readable_copy_named: false,
        },
    ))
}

/// Why the broken file is in the count fixture at all.
const WHY_THE_COUNT_MUST_BEAT_EVALUATION: &str = "the count is the reason this root cannot be read, and the broken file is there to prove it: a \
     loader that evaluates first and counts afterwards refuses `broken.luau` with a perfectly \
     accurate diagnostic, having already listed, opened and compiled its way through four \
     thousand files to reach it — which is the whole cost the bound exists to pre-empt. The \
     refusal points at the directory, because the mistake is the directory's and no single \
     declaration in it is at fault, and it states both quantities so a reader can tell one file \
     too many from a hundred thousand.";

/// Why the count fixture is built a second time with a directory in it.
const WHY_THE_COUNT_MUST_BEAT_THE_FILE_TYPE_CHECK: &str = "the sibling test proves the count beats *evaluation*; this one proves it beats the file-type \
     check, which happens one step earlier and once per entry. The directory sorts first under \
     every ordering, so a loader that asks the filesystem about each entry before it counts them \
     meets this one immediately and refuses it by name — four thousand and ninety-seven metadata \
     calls into the work the count was supposed to make unnecessary. No scenario words this, and \
     without it the check order is pinned at only one of its two boundaries.";

/// Why the same broken declaration is loaded twice.
const WHY_THE_SIZE_MUST_BEAT_WHAT_THE_FILE_HOLDS: &str = "the same broken declaration is loaded twice — once in a file small enough to be read, once \
     padded past the size bound — and the first run's reason is what the second run's refusal \
     must not carry. Nothing here writes down a word of the compiler's diagnostic, so a backend \
     that rewords its own messages moves both halves together. Size has to be taken from the \
     directory listing **before** the file is opened: a loader that reads first reports the \
     syntax error faithfully, which is a true statement about the wrong problem and sends an \
     author to edit a file that was never going to be read. The small root's own refusal is \
     asserted beside it, because if the broken declaration were not refusable at all the needle \
     would be empty and this test would pass over any implementation whatever.";

#[test]
fn a_root_holding_more_declarations_than_allowed_is_refused_on_the_count_and_not_on_a_broken_file()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_of_declarations(&directory, DECLARATIONS_A_ROOT_MAY_HOLD)?;
    fs::write(
        root.join(BLOCKS_DIRECTORY).join(A_BROKEN_FILE),
        A_DECLARATION_THAT_WILL_NOT_COMPILE,
    )?;

    let (answered, owed) = over_the_count(&root, A_BROKEN_FILE)?;

    assert_eq!(answered, owed, "{WHY_THE_COUNT_MUST_BEAT_EVALUATION}");
    Ok(())
}

#[test]
fn a_root_over_the_count_is_refused_before_any_entry_is_checked_to_be_a_file() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_of_declarations(&directory, DECLARATIONS_A_ROOT_MAY_HOLD)?;
    fs::create_dir_all(
        root.join(BLOCKS_DIRECTORY)
            .join(A_DIRECTORY_NAMED_LIKE_A_DECLARATION),
    )?;

    let (answered, owed) = over_the_count(&root, A_DIRECTORY_NAMED_LIKE_A_DECLARATION)?;

    assert_eq!(
        answered, owed,
        "{WHY_THE_COUNT_MUST_BEAT_THE_FILE_TYPE_CHECK}"
    );
    Ok(())
}

#[test]
fn a_declaration_file_past_the_size_bound_is_refused_on_its_size_and_not_on_what_it_holds()
-> TestResult {
    let readable = TempDir::new()?;
    let small = content_root(
        &readable,
        &[(AMBER_FILE, A_DECLARATION_THAT_WILL_NOT_COMPILE.to_owned())],
    )?;
    let oversized = TempDir::new()?;
    let large = content_root(
        &oversized,
        &[(
            AMBER_FILE,
            padded_to(A_DECLARATION_THAT_WILL_NOT_COMPILE, A_FILE_WELL_PAST_THAT)?,
        )],
    )?;

    let (answered, owed) = over_the_size(&small, &large)?;

    assert_eq!(
        answered, owed,
        "{WHY_THE_SIZE_MUST_BEAT_WHAT_THE_FILE_HOLDS}"
    );
    Ok(())
}

#[test]
fn a_root_holding_exactly_as_many_declarations_as_are_allowed_registers_every_one() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_of_declarations(&directory, DECLARATIONS_A_ROOT_MAY_HOLD)?;

    assert_eq!(
        (entries_under(&root)?, registration_order_or_refusal(&root)),
        (
            DECLARATIONS_A_ROOT_MAY_HOLD,
            Ok(every_generated_name(DECLARATIONS_A_ROOT_MAY_HOLD)),
        ),
        "the accepting side of the count bound, and a bound stated only from the refusing side \
         leaves `>` and `>=` indistinguishable. It is also the one place this suite finds out \
         whether a full root survives at all: every declaration in it is evaluated through one \
         script state, so four thousand compiled chunks and their residue accumulate against an \
         absolute memory ceiling that no per-declaration limit can see. A root refused here says \
         nothing about the count bound and everything about that ceiling, which is why the whole \
         registration order is compared rather than merely how many blocks arrived"
    );
    Ok(())
}

#[test]
fn a_declaration_file_of_exactly_the_size_allowed_registers_the_block_it_declares() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(
            AMBER_FILE,
            padded_to(&well_formed_amber(), BYTES_A_DECLARATION_FILE_MAY_HOLD)?,
        )],
    )?;

    assert_eq!(
        (
            bytes_of(&root, AMBER_FILE)?,
            registration_order_or_refusal(&root)
        ),
        (
            BYTES_A_DECLARATION_FILE_MAY_HOLD,
            Ok(vec![AMBER.to_owned()])
        ),
        "the accepting side of the size bound. The measured size is asserted in the same \
         comparison as the outcome because the boundary is the whole point: a padding helper that \
         produced one byte fewer would leave a loader spelling the check `>=` looking exactly as \
         correct as one spelling it `>`, and nothing else here would notice"
    );
    Ok(())
}
