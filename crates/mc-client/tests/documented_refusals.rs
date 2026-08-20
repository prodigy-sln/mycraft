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
//! preparation, over the three declarations these pages are written about: a block
//! declaration carrying a field nobody recognises, one stating a value longer than
//! a declaration may state, and a HUD declaration stating an extent of zero. All
//! three are rendered through the shipped reporting. A guard that spelled the
//! expected refusal out here would be comparing the documentation against a third
//! copy of somebody's belief about the program.
//!
//! **The third arrived because a substring check cannot see rendering.** The
//! declared-text refusal shipped with a line continuation that never landed, so a
//! mod author read a message with a hole punched through it, and the test that
//! covers that bound asks only whether both quantities are *mentioned* — which is
//! true of a well-formed message and a malformed one alike. Quoting the refusal on
//! a page puts it under the comparison below, which is line for line, and needs no
//! assertion of its own: the page is the oracle.
//!
//! **A mod author can trip roughly fourteen refusals and the pages quote three of
//! them.** That gap is real and is filed rather than closed here: expanding this
//! run twelve-fold is the change that introduces defects at the moment there is
//! least appetite to look for them, and the quoting and the prose want writing
//! together rather than one retrofitted onto the other.
//!
//! # The four a reload puts in front of an author, added with the reload page
//!
//! Four more are produced for `docs/modding/hot-reload.md`, on the rule the page was
//! written to: **a refusal an author meets by making an ordinary mistake is quoted
//! and held; one they meet because their filesystem is unusual is described.** They
//! are a saved typo, a deleted declaration the running world still holds, a root
//! that declares nothing solid, and a session out of texture layers. A root that
//! cannot be watched at all and a builder thread that died are neither ordinary nor
//! an authoring mistake, so the page states them in prose and this recogniser
//! deliberately passes over them.
//!
//! **They are produced through a running client rather than through a door**, because
//! a reload refusal only exists while a simulation is running: a watch reports a
//! change, a boundary collects the attempt, and the words come out of
//! `Session::take_reload_report`. The producing half of this file now lives in
//! `support/printed_refusals.rs`, and every part of the line it composes is
//! production's own — the sentence above the chain included, since
//! `mc_client::session::reload::CONTENT_NOT_TAKEN_UP` is declared where a test can
//! ask for it. **A scan stood in for that constant for one commit** and is recorded
//! in that module's header rather than kept: an instrument that exists because a
//! string had two homes retires when it has one.
//!
//! **Seven or eight of fourteen rather than three, and the filed gap above stands
//! for the rest.**
//!
//! # The two an entry writes, added with the offline half of the reload page
//!
//! A player a save recorded inside solid rock is told where they were put, or that
//! nothing within reach was clear and they were left in it. Neither is a refusal
//! — the launch proceeds either way — and both are quoted where the reload page
//! answers for a solidity change made while the game was off. **The recogniser does
//! not know the difference and must not**: it is the prefix the reporting writes, so
//! whatever a page shows under it is held to what a run produces. Both come out of
//! `mc_client::notice::entering`, over a verdict a launch over a real save arrived
//! at; `support/printed_refusals.rs` records why the fixture decides the world and
//! never the sentence.
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
//! # A dependency bump reddens the HUD half of this, and that is the guard working
//!
//! The HUD declaration's refusal ends in the TOML parser's five-line caret
//! diagnostic, quoted verbatim in the pages this reads. A minor version bump of that
//! parser restyles the diagnostic and this guard goes red — **diagnose that as the
//! dependency moving under the documentation, never as a flake.** The blast radius
//! is the pages that quote it and this file.
//!
//! **That is now true of the HUD half alone.** A block declaration is no longer
//! parsed by anything: it is a chunk the scripting host evaluates, and what a page
//! quotes for one carrying a field nobody recognises is a refusal MyCraft writes
//! itself — origin, block, field, cause. It moves when this project decides it
//! moves, and never underneath it. The narrowing is stated rather than dropped
//! because whoever next meets a red here needs to know which half a parser can
//! reach.

#[path = "support/built_set_refusals.rs"]
mod built_set_refusals;
#[path = "support/entry.rs"]
mod entry;
#[path = "support/input/mod.rs"]
mod input;
#[path = "support/per_facing_refusals.rs"]
mod per_facing_refusals;
#[path = "support/persistence.rs"]
mod persistence;
#[path = "support/printed_refusals.rs"]
mod printed_refusals;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_watch.rs"]
mod reload_watch;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::cmp::Reverse;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use printed_refusals::{UNRECOGNISED_FIELD, normalised, printed_refusals};
use support::TestResult;

/// The first thing the reporting writes, and therefore the whole of the
/// recogniser: a quoted refusal is a fenced block whose first line begins here.
const REFUSAL_PREFIX: &str = "mycraft: ";

/// How a fenced block opens and closes.
const FENCE: &str = "```";

/// The extension a page is written with.
const PAGE_EXTENSION: &str = "md";

/// The directory of pages a mod author reads, below the repository root.
const PAGES: [&str; 2] = ["docs", "modding"];

/// The same typo, one letter further from anything the loader knows — what a page
/// left quoting a field name that has moved on would carry.
const STALE_FIELD: &str = "slidey";

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
         promise with nothing showing it kept. What the client writes today, for the roots and \
         runs these pages are about, is:\n\n{}\n",
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

/// The page a mod author writes a block declaration against, below [`PAGES`].
const BLOCKS_PAGE: &str = "blocks-items.md";

/// The page a mod author reads about the built texture set on, below [`PAGES`].
const MODELS_PAGE: &str = "voxel-models.md";

/// The six fields a declaration may state, in the order the guide introduces
/// them.
///
/// Written out here rather than read from the loader, which is the point: an
/// expectation derived from the value under test agrees with whatever that value
/// becomes. `documented_refusals` is the one guard where the page and the program
/// are compared against each other rather than each against a third copy, and this
/// list is the page's own order.
const FIELDS_IN_THE_ORDER_THE_GUIDE_STATES: [&str; 6] = [
    "name",
    "texture",
    "solid",
    "replaceable",
    "breakable",
    "breaks_into",
];

/// What a page says about a set of refusals the client raises.
///
/// **Four arms rather than a count**, so that "the page states them all" and "the
/// page states none" can never compare equal, and so that a scan which stopped
/// finding quotations reports that rather than borrowing the good answer.
#[derive(Debug, PartialEq, Eq)]
enum GuideListing {
    /// Every refusal raised is quoted, and the fields the page's quotations name
    /// run in the order the guide states them.
    ///
    /// **The ordering half is only meaningful where the refusals name a
    /// declaration field.** A set of refusals that name none — the ones a built
    /// texture set raises — satisfies it vacuously, and what the verdict carries
    /// for those is the coverage half. Said here rather than split into two
    /// verdicts, because a second enum whose arms differed by one would be two
    /// things a reader has to tell apart.
    EveryRefusalIsQuotedInFieldOrder,
    /// This refusal is raised and the page does not carry it.
    NotQuoted { refusal: String },
    /// A quotation naming this field is quoted after one naming a field the guide
    /// introduces later.
    OutOfFieldOrder { field: String, after: String },
    /// The page was read and quotes no refusal at all.
    NoRefusalIsQuoted,
}

#[test]
fn the_modding_guide_states_every_per_facing_refusal_in_the_recognised_field_order() -> TestResult {
    let raised = per_facing_refusals::per_facing_refusals()?;

    let listing = listing_of(&pages()?.join(BLOCKS_PAGE), &raised)?;

    assert_eq!(
        listing,
        GuideListing::EveryRefusalIsQuotedInFieldOrder,
        "a mod author meets these seven by writing a texture table slightly wrong, which is what \
         everybody does on their first one, and each has to be findable on the page they are \
         already reading — quoted from a real run rather than described, so that what they see on \
         their terminal is what they can search the page for. The field order is the page's own: \
         a refusal about `texture` filed after one about `breakable` is a page a reader scans past"
    );
    Ok(())
}

#[test]
fn the_voxel_model_guide_states_every_refusal_a_built_texture_set_raises() -> TestResult {
    let raised = built_set_refusals::built_set_refusals()?;

    let listing = listing_of(&pages()?.join(MODELS_PAGE), &raised)?;

    assert_eq!(
        listing,
        GuideListing::EveryRefusalIsQuotedInFieldOrder,
        "a contributor who has not run the art build reads one of these six and then goes \
         looking for it here. The scan above asks only that what a page quotes is real; without \
         this one a page quoting three of the six passes, and an instrument that checks every \
         quotation is real while nothing checks every real thing is quoted reports its own blind \
         spot as zero"
    );
    Ok(())
}

/// What `page` makes of the refusals in `raised`.
///
/// # No positive control, and this is what that leaves undetected
///
/// A `listing_of` that came to report every raised refusal as quoted would answer
/// [`GuideListing::EveryRefusalIsQuotedInFieldOrder`] forever, and both guards
/// above with it — the other arms rule out a scan that *cannot* look, never one
/// that looks and always approves. **Both guards share this and neither
/// introduced it**: the per-facing listing has never had a control either, so one
/// test closes two. It was built and measured, and is absent only for this file's
/// size limit; the split that would make room is unrelated scope, for `main`
/// between specs.
///
/// # Errors
///
/// Returns an error if the page cannot be read, or if nothing was raised for it to
/// be compared against — neither is one of the four verdicts, for the reason
/// [`verdict_over`] refuses to let a vanished directory borrow one.
fn listing_of(page: &Path, raised: &[String]) -> Result<GuideListing, Box<dyn Error>> {
    if raised.is_empty() {
        return Err("no per-facing refusal was raised for the guide to be compared against".into());
    }
    let quoted = quoted_refusals_in(&fs::read_to_string(page)?);
    if quoted.is_empty() {
        return Ok(GuideListing::NoRefusalIsQuoted);
    }
    if let Some(refusal) = raised.iter().find(|refusal| !quoted.contains(refusal)) {
        return Ok(GuideListing::NotQuoted {
            refusal: refusal.clone(),
        });
    }
    Ok(fields_in_order(&quoted))
}

/// Whether the fields the page's quotations name run in the guide's own order.
///
/// A quotation whose field is not one of the six is passed over rather than
/// ranked: `slid` is the misspelling one of them is *about*, and giving it a
/// position would rank a word the guide never introduces.
fn fields_in_order(quoted: &[String]) -> GuideListing {
    let mut furthest: Option<(usize, String)> = None;
    for refusal in quoted {
        let Some((at, field)) = ranked_field(refusal) else {
            continue;
        };
        match &furthest {
            Some((reached, after)) if at < *reached => {
                return GuideListing::OutOfFieldOrder {
                    field,
                    after: after.clone(),
                };
            }
            _ => furthest = Some((at, field)),
        }
    }
    GuideListing::EveryRefusalIsQuotedInFieldOrder
}

/// Where in the guide's own order the field `refusal` blames sits, and what it is
/// called.
fn ranked_field(refusal: &str) -> Option<(usize, String)> {
    FIELDS_IN_THE_ORDER_THE_GUIDE_STATES
        .into_iter()
        .enumerate()
        .find(|(_, field)| refusal.contains(&format!("field `{field}`")))
        .map(|(at, field)| (at, field.to_owned()))
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
