//! Every refusal the modding pages quote is a refusal the client actually writes.
//!
//! A page that shows a mod author what a broken declaration looks like is making a
//! promise about text on a terminal, and nothing in a documentation tree can keep
//! that promise on its own. The failure this guards against is not hypothetical: a
//! page here described a refusal naming the file, the block and the field while the
//! program printed one sentence and threw the rest away — the page was right about
//! the intent and wrong about the artefact, and no test in the workspace could tell.
//!
//! # What counts as a quoted refusal, and why it is recognised this way
//!
//! **A fenced code block whose first line begins `mycraft: `.** That prefix is not a
//! convention anybody has to maintain — it is the first thing the reporting writes,
//! so the recogniser is derived from the artefact rather than agreed with an author.
//! Two consequences are deliberate: a page that adds a refusal is guarded the moment
//! it is pasted, with nothing to remember; and a page that *stops* quoting one
//! changes the verdict to [`Verdict::NoQuotedRefusalWasFound`] rather than passing
//! silently on an empty set.
//!
//! # An enumerated verdict, not an absence
//!
//! A scan that read no page, whose walk broke, or whose recogniser stopped matching
//! reports "nothing wrong" just as loudly as a clean tree does. So the answer is one
//! of three verdicts and each test compares the whole of it, which rejects the other
//! two — including the one meaning "there was nothing to check" — for free.
//!
//! Reading nothing at all is not one of the three. A directory that has moved, or
//! one holding no page, fails the scan rather than choosing the verdict nearest to
//! it: `NoQuotedRefusalWasFound` is a statement about pages that were read, and
//! letting a vanished directory borrow it would be the same conflation one level up.
//!
//! # Compared against a real run, never against a second copy
//!
//! The texts a quoted block is matched against come from the client's own
//! preparation, over the two declarations these pages are written about: a block
//! declaration carrying a field nobody recognises, and a HUD declaration stating an
//! extent of zero. Both are rendered through the shipped reporting. A guard that
//! spelled the expected refusal out here would be comparing the documentation
//! against a third copy of somebody's belief about the program.
//!
//! # Two normalisations, both stated rather than assumed
//!
//! - **The fixture root becomes the shipped one.** A refusal names the declaration
//!   by the path the run was given, and a run over a temporary copy names that copy.
//!   A page cannot quote a directory that exists for a hundred milliseconds, so the
//!   fixture root's own path is rewritten to the root a person actually runs against
//!   before anything is compared. The rewrite is checked rather than hoped for: a
//!   refusal that does not name the fixture root fails the scan.
//! - **Path separators are compared in `/` spelling.** The same refusal reads
//!   `content/base/blocks/amber.toml` on one platform and `content\base\...` on
//!   another, and a page can only carry one of the two. Everything else — the caret
//!   diagnostic's own indentation included — is compared exactly, line for line.
//!
//! # A dependency bump reddens this, and that is the guard working
//!
//! The refusal for an unrecognised field is the TOML parser's five-line caret
//! diagnostic, quoted verbatim in the pages this reads. A minor version bump of that
//! parser restyles the diagnostic and this guard goes red — **diagnose that as the
//! dependency moving under the documentation, never as a flake.** The blast radius
//! is the pages that quote it and this file.

mod support;

use std::cmp::Reverse;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use support::{TestResult, content};

/// The first thing the reporting writes, and therefore the whole of the
/// recogniser: a quoted refusal is a fenced block whose first line begins here.
const REFUSAL_PREFIX: &str = "mycraft: ";

/// How a fenced block opens and closes.
const FENCE: &str = "```";

/// The extension a page is written with.
const PAGE_EXTENSION: &str = "md";

/// The directory of pages a mod author reads, below the repository root.
const PAGES: [&str; 2] = ["docs", "modding"];

/// The root a person running the client from their game directory is given, and
/// therefore the root a quoted refusal names.
///
/// Assembled from its two components rather than written as one string, so it
/// spells itself the way the platform running this spells a path.
const SHIPPED_ROOT: [&str; 2] = ["content", "base"];

/// The block declaration file the pages are written about.
const BLOCK_FILE: &str = "amber.toml";

/// A field no loader recognises, spelled close enough to a real one to be the typo
/// a mod author actually makes.
const UNRECOGNISED_FIELD: &str = "slid";

/// The same typo, one letter further from anything the loader knows — what a page
/// left quoting a field name that has moved on would carry.
const STALE_FIELD: &str = "slidey";

/// A block declaration whose three well-formed fields sit beside one nobody
/// recognises.
const CARRYING_AN_UNRECOGNISED_FIELD: &str =
    "name = \"example:amber\"\ntexture = \"example:amber\"\nsolid = true\nslid = true\n";

/// The HUD declaration file the pages are written about.
const HUD_FILE: &str = "malformed-readout.toml";

/// A HUD declaration every other field of which is well formed, stating an extent
/// of zero.
const REFUSED_HUD_DECLARATION: &str = "name = \"example:malformed-readout\"\nanchor = \"center\"\nsize = [0, 4]\ndraw = \"fill\"\n\
     color = \"#FFFFFFFF\"\n";

/// What the scan came to.
///
/// Three answers rather than a list and a count, because "every page agrees" and
/// "no page said anything" must never compare equal — and because a mismatch that
/// named only the drifted side would leave whoever fixes it guessing at the other.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Every fenced refusal under the pages is text the client writes.
    EveryQuotedRefusalIsTheRefusalPrinted,
    /// This page quotes this, and the client writes that.
    Mismatch { quoted: String, produced: String },
    /// The pages were read and none of them quotes a refusal at all.
    NoQuotedRefusalWasFound,
}

#[test]
fn every_refusal_the_modding_pages_quote_is_a_refusal_the_client_prints() -> TestResult {
    let printed = printed_refusals()?;

    let verdict = verdict_over(&pages()?, &printed)?;

    assert_eq!(
        verdict,
        Verdict::EveryQuotedRefusalIsTheRefusalPrinted,
        "a page quoting a refusal the client does not write sends a mod author looking for text \
         that will never reach their terminal, and a page quoting none leaves the all-or-nothing \
         promise with nothing showing it kept. What the client writes today, for the two \
         declarations these pages are about, is:\n\n{}\n",
        printed.join("\n\n")
    );
    Ok(())
}

/// The control for the guard above, in the direction that matters most.
///
/// The drift is a **field name three lines down**, never the first line: a
/// comparison that stopped at the sentence the refusal opens with would accept this
/// page forever, and that sentence is precisely the part of a refusal a page is
/// least likely to get wrong. The altered text is derived from the printed refusal
/// rather than written out, so this fixture cannot fall behind what the client says.
#[test]
fn a_quoted_refusal_altered_to_text_the_client_never_prints_is_reported_with_both_sides()
-> TestResult {
    let printed = printed_refusals()?;
    let refusal = the_block_refusal(&printed)?;
    let pages = tempfile::tempdir()?;
    let drifted = a_page_naming_a_field_the_client_never_names(pages.path(), &refusal)?;

    let verdict = verdict_over(pages.path(), &printed)?;

    assert_eq!(
        verdict,
        Verdict::Mismatch {
            quoted: drifted,
            produced: refusal
        },
        "whoever has to repair a page needs both halves of the disagreement in front of them — \
         what the page claims and what the program says — because a guard reporting only that they \
         differ leaves the repair to be guessed at, and a guessed repair is how a page comes to \
         quote text nobody has run"
    );
    Ok(())
}

/// The pages under a documentation tree that quotes no refusal at all.
///
/// The fixture names the prefix twice outside any fence — once inside a code span
/// and once as a bare line of prose — so a recogniser that scanned lines rather than
/// fenced blocks would report a quotation here and fail. Prose *about* a refusal is
/// not a quotation of one: nothing holds it to the artefact, and treating it as
/// quoted would put a sentence somebody wrote by hand under a comparison it was
/// never going to survive.
#[test]
fn a_page_that_quotes_no_refusal_at_all_is_reported_as_quoting_none() -> TestResult {
    let printed = printed_refusals()?;
    let pages = tempfile::tempdir()?;
    a_page_quoting_no_refusal(pages.path())?;

    let verdict = verdict_over(pages.path(), &printed)?;

    assert_eq!(
        verdict,
        Verdict::NoQuotedRefusalWasFound,
        "a documentation tree that shows a mod author no refusal at all is not a tree in \
         agreement with the program — it is one with nothing to agree about, and the two must \
         never read the same. Agreement reported over an empty set is how a page that quietly \
         stopped quoting a refusal would go on passing"
    );
    Ok(())
}

/// The control the other two cannot supply: that a page quoting the printed text
/// **verbatim** is accepted, over a set of blocks that is not empty.
///
/// Without it, nothing in this file has ever watched the agreement verdict come out
/// of a real comparison — a recogniser that reported a mismatch for every block
/// would leave the drift control green and the real tree red, and the redness would
/// read as a documentation problem rather than as a broken guard.
///
/// Two pages and both printed refusals, which is the shape the real tree is about to
/// take, and each page carries a second fenced block that is a declaration rather
/// than a refusal — so a recogniser that treated every fenced block as a quotation
/// is caught here rather than by whoever next adds an example to a page.
#[test]
fn pages_quoting_the_printed_refusals_verbatim_agree_and_their_other_blocks_are_passed_over()
-> TestResult {
    let printed = printed_refusals()?;
    let pages = tempfile::tempdir()?;
    pages_quoting_verbatim(pages.path(), &printed)?;

    let verdict = verdict_over(pages.path(), &printed)?;

    assert_eq!(
        verdict,
        Verdict::EveryQuotedRefusalIsTheRefusalPrinted,
        "a refusal pasted out of a real run is what a page is supposed to carry, and a guard that \
         could not recognise one would make the correct page unwritable — which is the state where \
         somebody deletes the guard instead of the drift"
    );
    Ok(())
}

/// The refusals the client writes for the two declarations these pages are about,
/// each as a person running from their own game directory reads it.
fn printed_refusals() -> Result<Vec<String>, Box<dyn Error>> {
    let blocks =
        content::shipped_copy()?.declaring_block(BLOCK_FILE, CARRYING_AN_UNRECOGNISED_FIELD)?;
    let hud = content::shipped_with(HUD_FILE, REFUSED_HUD_DECLARATION)?;
    Ok(vec![
        as_read_from_a_game_directory(&blocks)?,
        as_read_from_a_game_directory(&hud)?,
    ])
}

/// What the client writes for the content root at `root`, with the fixture's own
/// temporary path rewritten to the root a person runs against.
///
/// # Errors
///
/// Returns an error if the root was accepted, or if what was written does not name
/// the fixture root — in which case the rewrite below would be a silent no-op and
/// the text compared against the pages would be one no page could ever carry.
fn as_read_from_a_game_directory(root: &content::ContentRoot) -> Result<String, Box<dyn Error>> {
    let printed = support::refusal_printed_over(root.path())?;
    let fixture = root.path().display().to_string();
    if !printed.contains(&fixture) {
        return Err(format!(
            "this guard has to rewrite the fixture root out of the refusal before comparing it \
             with a page, and what was written does not name the root it was given. Comparing it \
             unchanged would hold every page to text naming a directory that exists for a \
             hundred milliseconds. What was written was:\n{printed}"
        )
        .into());
    }
    let shipped: PathBuf = SHIPPED_ROOT.iter().collect();
    Ok(normalised(
        &printed.replace(&fixture, &shipped.display().to_string()),
    ))
}

/// The first of the printed refusals — the one about a block declaration.
///
/// # Errors
///
/// Returns an error if there is none, since a control comparing a drifted quotation
/// against nothing would be asserting nothing.
fn the_block_refusal(printed: &[String]) -> Result<String, Box<dyn Error>> {
    printed
        .first()
        .cloned()
        .ok_or_else(|| "the client wrote no refusal for a block declaration to drift from".into())
}

/// Where the pages a mod author reads live.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located above this crate.
fn pages() -> Result<PathBuf, Box<dyn Error>> {
    Ok(PAGES
        .iter()
        .fold(support::repository_root()?, |below, part| below.join(part)))
}

/// What the pages under `directory` quote, judged against what the client writes.
///
/// # Errors
///
/// Returns an error if there is nothing to compare against, if the directory cannot
/// be walked, or if it holds no page — none of the three verdicts is an answer a
/// scan that could not look is entitled to.
fn verdict_over(directory: &Path, printed: &[String]) -> Result<Verdict, Box<dyn Error>> {
    if printed.is_empty() {
        return Err("the client wrote no refusal for any page to be compared against".into());
    }
    let mut quoted = Vec::new();
    let read = pages_under(directory)?;
    if read.is_empty() {
        return Err(format!(
            "no page was read under {}, so nothing below could be said about what the pages quote",
            directory.display()
        )
        .into());
    }
    for page in &read {
        quoted.extend(quoted_refusals_in(&fs::read_to_string(page)?));
    }
    Ok(judged(&quoted, printed))
}

/// The verdict a set of quotations and a set of printed refusals come to.
fn judged(quoted: &[String], printed: &[String]) -> Verdict {
    let Some(drifted) = quoted.iter().find(|block| !printed.contains(block)) else {
        return if quoted.is_empty() {
            Verdict::NoQuotedRefusalWasFound
        } else {
            Verdict::EveryQuotedRefusalIsTheRefusalPrinted
        };
    };
    Verdict::Mismatch {
        quoted: drifted.clone(),
        produced: nearest_to(drifted, printed),
    }
}

/// The printed refusal `quoted` most nearly is — the one it agrees with for
/// longest, so the report shows where the two part company.
fn nearest_to(quoted: &str, printed: &[String]) -> String {
    printed
        .iter()
        .min_by_key(|candidate| Reverse(agreed_characters(quoted, candidate)))
        .cloned()
        .unwrap_or_default()
}

/// How far two texts agree from their beginnings.
fn agreed_characters(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(from_left, from_right)| from_left == from_right)
        .count()
}

/// Every page under `directory`, in a settled order so the block a mismatch names
/// is the same one whatever order the filesystem hands its entries back in.
///
/// # Errors
///
/// Returns an error if the directory, or one below it, cannot be read.
fn pages_under(directory: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut found = Vec::new();
    collect_pages(directory, &mut found)?;
    found.sort();
    Ok(found)
}

fn collect_pages(directory: &Path, found: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_pages(&path, found)?;
        } else if path.extension() == Some(OsStr::new(PAGE_EXTENSION)) {
            found.push(path);
        }
    }
    Ok(())
}

/// The refusals one page quotes: the fenced blocks whose first line begins with the
/// prefix the reporting writes, in the order they appear.
///
/// A block's info string is not read — `text`, `console` or nothing at all are the
/// same block — because the recogniser is derived from what the program writes and
/// an info string is something an author chooses.
fn quoted_refusals_in(page: &str) -> Vec<String> {
    let mut quoted = Vec::new();
    let mut lines = page.lines();
    while let Some(line) = lines.next() {
        if !is_fence(line) {
            continue;
        }
        let block = lines
            .by_ref()
            .take_while(|line| !is_fence(line))
            .collect::<Vec<_>>()
            .join("\n");
        if block.starts_with(REFUSAL_PREFIX) {
            quoted.push(normalised(&block));
        }
    }
    quoted
}

/// Whether a line opens or closes a fenced block.
fn is_fence(line: &str) -> bool {
    line.trim_start().starts_with(FENCE)
}

/// A text in the one spelling both sides of a comparison are held to: no trailing
/// whitespace on any line, no blank lines at the end, and path separators written
/// the same way on every platform.
///
/// Leading whitespace is left alone, because the caret diagnostic's own indentation
/// is part of what a page has to get right.
fn normalised(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .replace('\\', "/")
}

/// A page quoting the printed refusal with one field renamed, written under
/// `directory`, and the drifted text it now carries.
///
/// # Errors
///
/// Returns an error if the rename changed nothing — the page would then quote the
/// refusal correctly, the scan would rightly agree with it, and the mismatch this
/// control expects would be missing for a reason about the fixture rather than about
/// the guard.
fn a_page_naming_a_field_the_client_never_names(
    directory: &Path,
    refusal: &str,
) -> Result<String, Box<dyn Error>> {
    let drifted = refusal.replace(UNRECOGNISED_FIELD, STALE_FIELD);
    if drifted == refusal {
        return Err(format!(
            "this control has to alter the refusal into text the client does not write, and \
             renaming `{UNRECOGNISED_FIELD}` changed nothing in it. What it would write is a page \
             quoting the refusal correctly. The refusal was:\n{refusal}"
        )
        .into());
    }
    fs::write(
        directory.join("README.md"),
        format!("# When you get it wrong\n\nWhat you read:\n\n```text\n{drifted}\n```\n"),
    )?;
    Ok(drifted)
}

/// A page that names the reporting's prefix twice outside any fence and quotes no
/// refusal, written under `directory`.
fn a_page_quoting_no_refusal(directory: &Path) -> Result<(), Box<dyn Error>> {
    fs::write(
        directory.join("README.md"),
        format!(
            "# Blocks\n\nA declaration the loader will not accept stops the client, and what it \
             says begins with the program's own name:\n\n{REFUSAL_PREFIX}the shipped content could \
             not be read\n\nso `{REFUSAL_PREFIX}` is how you know the line is the client's. An \
             example declaration:\n\n```toml\nname = \"example:amber\"\nsolid = true\n```\n"
        ),
    )?;
    Ok(())
}

/// One page per printed refusal, each quoting it verbatim beside a fenced block that
/// is a declaration rather than a refusal, written under `directory`.
fn pages_quoting_verbatim(directory: &Path, printed: &[String]) -> Result<(), Box<dyn Error>> {
    for (index, refusal) in printed.iter().enumerate() {
        fs::write(
            directory.join(format!("page-{index}.md")),
            format!(
                "# All-or-nothing loading\n\nA declaration like this one:\n\n```toml\nname = \
                 \"example:amber\"\nsolid = true\n```\n\nis refused whole, and what you read \
                 is:\n\n```text\n{refusal}\n```\n"
            ),
        )?;
    }
    Ok(())
}
