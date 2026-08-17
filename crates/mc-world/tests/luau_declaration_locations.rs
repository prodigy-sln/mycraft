//! Where a refusal about a chunk points, and what it manages to say about why.
//!
//! A declaration that will not compile, or that raises before it returns, is
//! still a file somebody has to open and fix. The scripting host cannot supply
//! that on its own: it is handed a chunk *name* as a label and never opens
//! anything, so what it can report back is the label it was given. The path is
//! the loader's to own, and this suite is what holds it to owning it.
//!
//! # Why the chunk name and the origin are deliberately different
//!
//! The loader hands the host the file's own name — `amber.luau` — and builds the
//! origin itself from the whole path. That split is what makes the requirement
//! falsifiable at all: a loader passing the full path as the chunk name would
//! make the origin and the label coincide, and every assertion below would be
//! two copies of one decision agreeing with each other. The additional test at
//! the foot of this file is what keeps the two apart, because nothing in the
//! scenarios themselves can see the label the host was given.
//!
//! # Comparing a whole path here, and only here
//!
//! Everywhere else in these suites an origin is judged by the name of the file
//! it points at. That reading cannot serve here: `amber.luau` and
//! `<root>/blocks/amber.luau` both contain `amber.luau`, so the check that is
//! right elsewhere is precisely the one that cannot tell the two apart. The
//! expectation is therefore the whole path — built with `Path::join` and never
//! spelled with a separator, so it stays portable.

mod common;
mod luau_common;

use std::error::Error;

use common::{TestResult, content_root};
use luau_common::{
    AMBER_FILE, Attribution, amber_after, attribution_of, declaration_label, declarations_label,
    fault_from, naming_the_file_alone,
};
use tempfile::TempDir;

/// A declaration file whose third line is not Luau at all.
///
/// The two lines above it are ordinary, so the compiler has somewhere to get to
/// before it fails and the line it names is not simply the first one.
const A_FILE_THAT_IS_NOT_VALID_LUAU: &str = "local intended = 'example:amber'\n\
     local textured = 'example:quartz'\n\
     local broken = =\n\
     return { name = intended, texture = textured, solid = true }\n";

/// The text that makes [`A_FILE_THAT_IS_NOT_VALID_LUAU`] not Luau.
///
/// The expected line is found by looking for this in the fixture rather than
/// written down beside it, so that editing the chunk moves the expectation with
/// it instead of leaving a number behind that no longer means anything.
const THE_BROKEN_TEXT: &str = "local broken = =";

/// What the raising declaration says when it stops itself.
///
/// Distinctive enough that no part of the host's own vocabulary contains it, so
/// a refusal quoting it is quoting the mod and not itself.
const THE_ERROR_THE_CHUNK_RAISES: &str = "amber cannot be declared before its texture exists";

/// A declaration that stops itself before it returns, with a message of its own.
fn a_declaration_that_raises() -> String {
    amber_after(&format!("error('{THE_ERROR_THE_CHUNK_RAISES}')\n"))
}

/// A declaration that asks the host what it is called, and declares the answer.
///
/// The only route by which the chunk name the host was given is observable from
/// outside: a message the backend positions carries the label in front of it, so
/// a chunk that catches its own error and puts the text where a name belongs
/// gets the label quoted straight back in the refusal.
fn a_declaration_naming_itself_after_what_the_host_calls_it() -> String {
    "local _, raised = pcall(function() error('probe') end)\n\
     return { name = raised, texture = 'example:quartz', solid = true }\n"
        .to_owned()
}

/// Which line of `chunk` holds `marker`, counting from one as a compiler does.
fn line_holding(chunk: &str, marker: &str) -> Result<u32, Box<dyn Error>> {
    let index = chunk
        .lines()
        .position(|line| line.contains(marker))
        .ok_or_else(|| format!("this fixture no longer holds `{marker}`"))?;
    Ok(u32::try_from(index + 1)?)
}

/// What a refusal about a chunk that would not compile said.
///
/// One record rather than separate assertions, so a loader that located the file
/// but said nothing useful about why is not mistaken for one that did both.
#[derive(Debug, PartialEq, Eq)]
struct CompileRefusal {
    attribution: Attribution,
    names_the_broken_line: bool,
}

/// What a refusal about a chunk that stopped itself said, on the same terms.
#[derive(Debug, PartialEq, Eq)]
struct RaisedRefusal {
    attribution: Attribution,
    names_what_the_chunk_raised: bool,
}

/// What the host was told to call the chunk, as the refusal let it slip.
#[derive(Debug, PartialEq, Eq)]
struct ChunkNameSeen {
    the_files_own_name: bool,
    the_whole_path_to_it: bool,
}

#[test]
fn a_chunk_that_will_not_compile_is_located_by_its_path_and_not_by_the_name_the_host_was_given()
-> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(AMBER_FILE, A_FILE_THAT_IS_NOT_VALID_LUAU.to_owned())],
    )?;

    let fault = fault_from(&root)?;

    assert_eq!(
        fault.origin.as_str(),
        declaration_label(&root, AMBER_FILE),
        "the scripting host is handed a label and never opens a file, so the label it reports \
         back is the label it was given — `amber.luau`, which is not something a mod author with \
         several content roots can open. The path is the loader's own and has to be built by the \
         loader; lifting the origin out of the host's fault instead produces the bare name and \
         fails here"
    );
    Ok(())
}

#[test]
fn a_declaration_file_that_is_not_valid_luau_is_refused_naming_the_line_the_compiler_named()
-> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(AMBER_FILE, A_FILE_THAT_IS_NOT_VALID_LUAU.to_owned())],
    )?;
    let broken_line = line_holding(A_FILE_THAT_IS_NOT_VALID_LUAU, THE_BROKEN_TEXT)?;

    let fault = fault_from(&root)?;

    assert_eq!(
        CompileRefusal {
            attribution: attribution_of(&fault, AMBER_FILE),
            names_the_broken_line: fault.cause.contains(&format!("line {broken_line}")),
        },
        CompileRefusal {
            attribution: naming_the_file_alone(),
            names_the_broken_line: true,
        },
        "the line is the whole value of a compiler diagnostic to whoever has to fix the file, and \
         the host carries it as a field of its own precisely so that it survives a backend that \
         renames its own message prefix. A refusal composed from the fault's kind and cause alone \
         drops it, and a mod author is told their file is not Luau and not told where: {fault:?}"
    );
    Ok(())
}

#[test]
fn a_chunk_that_raises_before_returning_is_refused_naming_the_error_it_raised() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(&directory, &[(AMBER_FILE, a_declaration_that_raises())])?;

    let fault = fault_from(&root)?;

    assert_eq!(
        RaisedRefusal {
            attribution: attribution_of(&fault, AMBER_FILE),
            names_what_the_chunk_raised: fault.cause.contains(THE_ERROR_THE_CHUNK_RAISES),
        },
        RaisedRefusal {
            attribution: naming_the_file_alone(),
            names_what_the_chunk_raised: true,
        },
        "the declaration this fixture wraps would register; the line in front of it stops itself \
         on purpose, which is how a mod author says a declaration cannot be made. What they said \
         is the only thing that explains the refusal, so it has to reach the refusal — and there \
         is no block to attribute it to, because the chunk never returned one: {fault:?}"
    );
    Ok(())
}

#[test]
fn the_host_is_told_to_call_a_chunk_by_its_file_name_and_not_by_its_path() -> TestResult {
    let directory = TempDir::new()?;
    let root = content_root(
        &directory,
        &[(
            AMBER_FILE,
            a_declaration_naming_itself_after_what_the_host_calls_it(),
        )],
    )?;

    let fault = fault_from(&root)?;
    let quoted = fault.block.clone().unwrap_or_default();

    assert_eq!(
        ChunkNameSeen {
            the_files_own_name: quoted.contains(AMBER_FILE),
            the_whole_path_to_it: quoted.contains(&declarations_label(&root)),
        },
        ChunkNameSeen {
            the_files_own_name: true,
            the_whole_path_to_it: false,
        },
        "this is what keeps the origin assertion in this file honest. If the loader handed the \
         host the whole path as the chunk name, the origin it reports and the label the host was \
         given would coincide and that assertion could never fail — two copies of one decision \
         agreeing with each other. Nothing in the scenarios can see the label, so this does: the \
         chunk catches its own error and declares the positioned message as its name, and the \
         refusal quotes it back: {fault:?}"
    );
    Ok(())
}
