//! The block declaration the first-block walkthrough shows a mod author is a
//! declaration that loads.
//!
//! `docs/modding/README.md` is the page a person who has never read this
//! codebase starts at, and what it shows them is a file to type out. A page
//! whose example does not load teaches somebody to write a file that is refused
//! and then leaves them to work out which of the two is wrong — theirs or the
//! page's — with the page the only thing they have to go on.
//!
//! # What counts as the walkthrough's declaration
//!
//! **A fenced block whose info string says the language declarations are written
//! in.** That is not a convention somebody has to remember: it is what a page
//! marks a file's contents with so a reader knows what they are looking at, and a
//! walkthrough that shows a declaration without saying what language it is in has
//! a worse problem than this guard.
//!
//! **Every such block, not the first one.** A page that grows a second example is
//! a page a mod author will type the second example out of. The consequence is a
//! constraint on whoever writes the page and it is stated here rather than
//! discovered: a fenced block tagged as a declaration is a *whole* declaration
//! that loads, and a fragment of one is shown without that tag.
//!
//! # The name is read out of the page, never written down here
//!
//! What the declaration registers is compared against the name that same quoted
//! text states, parsed out of it by reading the page rather than by evaluating
//! it. Writing an expected name here would hold the walkthrough to whatever
//! example this file was written beside, and the page would become unwritable the
//! first time somebody renamed the block it teaches.
//!
//! # An enumerated verdict, not an absence
//!
//! A page that quotes no declaration at all is not a page in agreement with the
//! loader — it is a page with nothing to agree about, and reporting agreement
//! over an empty set is how a walkthrough that quietly stopped showing an example
//! would go on passing.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use mc_core::block::{BlockId, BlockRegistry};
use mc_world::content::LuauFileDefinitionSource;

type TestResult = Result<(), Box<dyn Error>>;

/// The page a mod author starts at, below the repository root.
const WALKTHROUGH: [&str; 3] = ["docs", "modding", "README.md"];

/// How a fenced block opens and closes, and what its info string says when what
/// it holds is a block declaration.
const FENCE: &str = "```";
const DECLARATION_LANGUAGE: &str = "luau";

/// The subdirectory of a content root a declaration is written into, and the
/// file the quoted one is written as.
const BLOCKS: &str = "blocks";
const QUOTED_AS: &str = "amber.luau";

/// The field a declaration states its own name in.
const NAME_FIELD: &str = "name";

/// A declaration a page could correctly show, and one it could wrongly show.
///
/// The second states everything but its solidity, which is a page one letter's
/// worth of editing away from the first and reads exactly as convincingly.
const A_DECLARATION_THAT_LOADS: &str =
    "return {\n\tname = 'example:amber',\n\ttexture = 'example:amber',\n\tsolid = true,\n}\n";
const A_DECLARATION_THAT_IS_REFUSED: &str =
    "return {\n\tname = 'example:amber',\n\ttexture = 'example:amber',\n}\n";

/// What the walkthrough's declarations came to.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Every declaration the page quotes registers a block under the name that
    /// same quotation states.
    EveryQuotedDeclarationRegistersUnderTheNameItStates,
    /// This quotation did something else — it was refused, it states no name of
    /// its own to compare against, or what registered is not what it states.
    Disagreed { quoted: String, happened: String },
    /// The page was read and quotes no declaration at all.
    NoDeclarationWasQuoted,
}

#[test]
fn the_declaration_the_first_block_walkthrough_shows_registers_the_block_it_states() -> TestResult {
    let verdict = verdict_over(&fs::read_to_string(walkthrough()?)?);

    assert_eq!(
        verdict,
        Verdict::EveryQuotedDeclarationRegistersUnderTheNameItStates,
        "this page is the whole of what somebody with no knowledge of this codebase has to go on, \
         and the file it tells them to type is the first thing they will ever write in this \
         engine's own language. An example that does not load leaves them unable to tell whether \
         they mistyped it or the page is wrong"
    );
    Ok(())
}

/// The vacuity control, and the reason this guard is worth building at all.
///
/// A walkthrough that stops showing a declaration, or a recogniser that stops
/// matching one, finds nothing to check — which must not read the same as finding
/// everything in order. The fixture names the language twice outside any fence,
/// once in prose and once in a code span, so a recogniser that scanned for the
/// word rather than for a fenced block reports a quotation here and fails.
#[test]
fn a_walkthrough_quoting_no_declaration_at_all_is_reported_as_quoting_none() -> TestResult {
    let page = format!(
        "# Your first block\n\nBlock declarations are written in {DECLARATION_LANGUAGE}, and a \
         file ending `.{DECLARATION_LANGUAGE}` under `blocks/` is one. Where the HUD is \
         concerned:\n\n{FENCE}toml\nname = \"example:readout\"\n{FENCE}\n"
    );

    let verdict = verdict_over(&page);

    assert_eq!(
        verdict,
        Verdict::NoDeclarationWasQuoted,
        "a page showing a mod author no declaration at all has nothing for this to agree with, \
         and agreement reported over an empty set is how a walkthrough that lost its example goes \
         on passing"
    );
    Ok(())
}

/// The control the two above cannot supply: that a page whose quoted
/// declaration really does load is accepted, over a set of quotations that is
/// not empty.
///
/// Without it nothing here has ever watched the agreement verdict come out of a
/// real load. A recogniser that reported a disagreement for every block would
/// leave both tests above green and the real page red, and that redness would
/// read as a documentation problem rather than as a broken guard — which is the
/// state in which somebody deletes the guard instead of the drift.
///
/// The fixture carries a second fenced block that is not a declaration, so a
/// recogniser treating every fenced block as one is caught here rather than by
/// whoever next adds an example to the page.
#[test]
fn a_walkthrough_quoting_a_declaration_that_loads_agrees_and_its_other_blocks_are_passed_over()
-> TestResult {
    let page = format!(
        "# Your first block\n\nWrite this into `content/base/blocks/amber.luau`:\n\n\
         {FENCE}{DECLARATION_LANGUAGE}\n{A_DECLARATION_THAT_LOADS}{FENCE}\n\nThe HUD is declared \
         elsewhere and in another format:\n\n{FENCE}toml\nname = \"example:readout\"\n{FENCE}\n"
    );

    let verdict = verdict_over(&page);

    assert_eq!(
        verdict,
        Verdict::EveryQuotedDeclarationRegistersUnderTheNameItStates,
        "a declaration pasted out of a file that runs is what this page is supposed to carry, and \
         a guard that could not recognise one would make the correct page unwritable"
    );
    Ok(())
}

/// The control in the direction that matters most: a page whose example does not
/// load is reported, with both the quotation and what happened to it.
///
/// The example is broken by leaving out a field rather than by mistyping the
/// language, so what is caught is a declaration that reads perfectly well to
/// somebody skimming the page — which is the only kind that survives review.
#[test]
fn a_walkthrough_quoting_a_declaration_the_loader_refuses_is_reported_with_both_sides() -> TestResult
{
    let page = format!(
        "# Your first block\n\n{FENCE}{DECLARATION_LANGUAGE}\n{A_DECLARATION_THAT_IS_REFUSED}\
         {FENCE}\n"
    );

    let verdict = verdict_over(&page);

    let reported = match &verdict {
        Verdict::Disagreed { quoted, happened } => (
            quoted == A_DECLARATION_THAT_IS_REFUSED,
            happened.contains("solid"),
        ),
        _ => (false, false),
    };
    assert_eq!(
        (
            reported,
            verdict != Verdict::EveryQuotedDeclarationRegistersUnderTheNameItStates
        ),
        ((true, true), true),
        "whoever has to repair the page needs the example that failed and the reason in front of \
         them, because a guard reporting only that something is wrong leaves the repair to be \
         guessed at: {verdict:?}"
    );
    Ok(())
}

/// Where the walkthrough lives.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located above this crate.
fn walkthrough() -> Result<PathBuf, Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("the crate manifest directory has no repository root above it")?
        .to_owned();
    Ok(WALKTHROUGH
        .iter()
        .fold(root, |below, part| below.join(part)))
}

/// What the declarations `page` quotes come to when they are loaded.
fn verdict_over(page: &str) -> Verdict {
    let quoted = quoted_declarations_in(page);
    if quoted.is_empty() {
        return Verdict::NoDeclarationWasQuoted;
    }
    for declaration in quoted {
        if let Some(disagreement) = judged(&declaration) {
            return disagreement;
        }
    }
    Verdict::EveryQuotedDeclarationRegistersUnderTheNameItStates
}

/// What `declaration` did, or nothing where it did what the page promises.
fn judged(declaration: &str) -> Option<Verdict> {
    let disagreed = |happened: String| {
        Some(Verdict::Disagreed {
            quoted: declaration.to_owned(),
            happened,
        })
    };
    let Some(stated) = name_stated_in(declaration) else {
        return disagreed(format!(
            "it states no `{NAME_FIELD}` of its own, so there is nothing to hold what registers to"
        ));
    };
    match registered_from(declaration) {
        Err(refused) => disagreed(format!("it was refused: {refused}")),
        Ok(registered) if registered == vec![stated.clone()] => None,
        Ok(registered) => disagreed(format!(
            "it states `{stated}` and what registered was {registered:?}"
        )),
    }
}

/// Every block a content root holding `declaration` alone registers.
///
/// # Errors
///
/// Returns an error if the root cannot be written, or if the declaration is
/// refused.
fn registered_from(declaration: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let blocks = root.path().join(BLOCKS);
    fs::create_dir_all(&blocks)?;
    fs::write(blocks.join(QUOTED_AS), declaration)?;
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root.path().to_owned()))?;
    Ok((0..registry.registered_count())
        .map(|id| {
            registry
                .definition(BlockId::from_raw(id as u32))
                .map_or_else(
                    |failure| failure.to_string(),
                    |definition| definition.name.as_str().to_owned(),
                )
        })
        .collect())
}

/// The name `declaration` states for itself, read out of its text.
///
/// Read rather than evaluated, so that what registers is compared against an
/// answer this side of the loader. A declaration may compute a value — that is
/// what makes it code rather than a document — but the one a walkthrough shows
/// somebody typing states its name outright, and a name assembled by a page's
/// first example would be teaching the wrong lesson anyway.
fn name_stated_in(declaration: &str) -> Option<String> {
    declaration
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix(NAME_FIELD))
        .filter_map(|rest| rest.trim_start().strip_prefix('='))
        .find_map(|value| quoted_text_in(value.trim()))
}

/// What is inside the quotes of `value`, in either spelling Luau accepts.
fn quoted_text_in(value: &str) -> Option<String> {
    let quote = value
        .chars()
        .next()
        .filter(|mark| *mark == '\'' || *mark == '"')?;
    value[quote.len_utf8()..]
        .split(quote)
        .next()
        .map(str::to_owned)
}

/// The declarations one page quotes: the fenced blocks whose info string says
/// they are written in the declaration language, in the order they appear.
fn quoted_declarations_in(page: &str) -> Vec<String> {
    let mut quoted = Vec::new();
    let mut lines = page.lines();
    while let Some(line) = lines.next() {
        let opening = line.trim_start();
        if !opening.starts_with(FENCE) {
            continue;
        }
        let held = lines
            .by_ref()
            .take_while(|line| !line.trim_start().starts_with(FENCE))
            .collect::<Vec<_>>();
        if opening.trim_start_matches('`') == DECLARATION_LANGUAGE {
            quoted.push(format!("{}\n", held.join("\n")));
        }
    }
    quoted
}
