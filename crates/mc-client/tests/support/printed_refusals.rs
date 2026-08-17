//! What the client actually writes when it turns a content root away — the launch
//! refusals and the reload ones — each produced by a real run.
//!
//! # Why this is a module of its own
//!
//! `tests/documented_refusals.rs` does two separable things: it produces what the
//! client writes, and it compares that against what the modding pages quote. The
//! producing half is what grows — a launch refusal here, a reload refusal there —
//! and the comparing half is the guard. The file's own header names that seam
//! ("compared against a real run, never against a second copy"), so the split
//! follows the seam rather than a line count.
//!
//! # The launch four and the reload four are produced differently, and they have to
//! be
//!
//! A launch refusal is what `prepare_scene` returns for a root, rendered through the
//! shipped reporting: no client is running, so the failure is the door's own answer.
//! A **reload** refusal only exists while a simulation is running, and the text a
//! person reads for one is composed in the frame path. So each of the four below is
//! driven through a real client — a watch reports a change, a boundary collects the
//! attempt, and the words are taken out of `Session::take_reload_report`, which is
//! the product's own output rather than a rendering this module performed.
//!
//! # The sentence above the chain is production's, and it was not always
//!
//! `App::report_reload` writes `mycraft: {CONTENT_NOT_TAKEN_UP}: {reason}`, and
//! [`CONTENT_NOT_TAKEN_UP`] is declared beside `ReloadReport` — the only place the
//! sentence is both printed and askable. So **every part of a reload refusal's line
//! comes from production**: the chain from the client's own report, the joiner from
//! `Ending::failed_under`, the prefix from `mc_render::window::report`, and now the
//! sentence itself.
//!
//! **It was a copy here for one commit, and the copy was held by a scan** requiring
//! the spelling to appear in the source that printed it. Both are gone. The reason
//! they existed is worth keeping: a page held to a fixture's copy of a sentence is a
//! page holding hands with a test, and neither would notice the program disagreeing
//! with both. The scan was the weaker instrument standing in until the constant
//! existed, which is the same relationship `client_names_no_content_door.rs` records
//! with the dependency-closure guard it stands in for.
//!
//! What this still does not close is `App` ceasing to print at all, which needs a
//! window — the residual `main.rs` accepts and `shipped_binary.rs` closes for the
//! launch path.

// Each binary that includes this drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::time::Instant;

use mc_client::session::reload::CONTENT_NOT_TAKEN_UP;
use mc_render::window::Ending;

use crate::input::InputHarness;
use crate::reload::{
    AMBER_FILE, DIRT, DIRT_FILE, Declaration, GRASS, GRASS_FILE, STONE, STONE_FILE, WATER,
    WATER_FILE, amber, declaring, restating, shipped,
};
use crate::reload_content::spent_all;
use crate::reload_watch::{
    Attempt, Reports, STONE_MISSPELLING_SOLID, a_client_on, a_client_over, block_path, boundary,
    may_cross_another, pause_between_boundaries, restating_raw, watch,
};
use crate::reload_world::{floor_of, floor_under_a_ceiling, playing_serving, standing};
use crate::support::{self, content};

/// The root a person running the client from their game directory is given, and
/// therefore the root a quoted refusal names.
///
/// Assembled from its two components rather than written as one string, so it
/// spells itself the way the platform running this spells a path.
const SHIPPED_ROOT: [&str; 2] = ["content", "base"];

/// The block declaration file the pages are written about.
pub const BLOCK_FILE: &str = "amber.luau";

/// A field no loader recognises, spelled close enough to a real one to be the typo
/// a mod author actually makes.
pub const UNRECOGNISED_FIELD: &str = "slid";

/// A block declaration whose three well-formed fields sit beside one nobody
/// recognises.
///
/// A chunk that returns a table, because that is what a declaration is now. It is
/// written with single quotes for the same reason every other Luau fixture in this
/// suite is: a declaration inside a Rust string literal with every escape doubled is
/// unreadable at exactly the moment somebody has to read it.
const CARRYING_AN_UNRECOGNISED_FIELD: &str = "return {\n\tname = 'example:amber',\n\ttexture = 'example:amber',\n\tsolid = true,\n\tslid = \
     true,\n}\n";

/// A well-formed block declaration naming a texture key no other declaration names.
///
/// Well formed on purpose: what refuses it is the session having no layer left for
/// the key it declares, and a declaration with a second thing wrong with it would be
/// refused for that first.
const CARRYING_A_NEW_TEXTURE_KEY: &str =
    "return {\n\tname = 'example:amber',\n\ttexture = 'example:amber',\n\tsolid = true,\n}\n";

/// What this guard says when the session it read a root against turned out to have a
/// layer left after all.
const THE_BUDGET_WAS_NOT_SPENT: &str = "this guard needs a content root that cannot fit in the layers this session has spent, and it \
     was read instead. There is no refusal for a page to quote";

/// The namespace every declaration in this file gives its ids, and how many
/// characters of it that is.
///
/// The length is measured off the text rather than written down, so the arithmetic
/// below stays right if the namespace is ever renamed.
const NAMESPACE: &str = "example:";

/// How many characters of declared text a declaration may state, and the length the
/// fixture below states.
///
/// Written here rather than read from the loader, which is the whole point: an
/// expectation derived from the value under test agrees with it whatever it becomes.
/// One past the bound is the smallest fixture that trips it, and the smallest is what
/// keeps the refusal a page has to quote readable.
const CHARACTERS_A_DECLARATION_MAY_STATE: usize = 256;
const ONE_CHARACTER_PAST_THE_BOUND: usize = CHARACTERS_A_DECLARATION_MAY_STATE + 1;

/// The HUD declaration file the pages are written about.
const HUD_FILE: &str = "malformed-readout.toml";

/// A HUD declaration every other field of which is well formed, stating an extent of
/// zero.
const REFUSED_HUD_DECLARATION: &str = "name = \"example:malformed-readout\"\nanchor = \"center\"\nsize = [0, 4]\ndraw = \"fill\"\n\
     color = \"#FFFFFFFF\"\n";

/// Every refusal a page may quote, each as a person running from their own game
/// directory reads it.
///
/// **Eight roots and not one**, because each is refused whole: a root carrying two
/// mistakes is refused for whichever the loader reaches first, and the second refusal
/// would be one no run ever prints.
///
/// The order is load-bearing in one place only: the launch refusal over a
/// misspelled field is first, because the drift control in the binary alters *that*
/// one to build a page nobody's run produces.
///
/// # Errors
///
/// Returns an error if a fixture root cannot be built, if a root that must refuse is
/// accepted, or if a client cannot be started.
pub fn printed_refusals() -> Result<Vec<String>, Box<dyn Error>> {
    let blocks =
        content::shipped_copy()?.declaring_block(BLOCK_FILE, CARRYING_AN_UNRECOGNISED_FIELD)?;
    let overlong = content::shipped_copy()?
        .declaring_block(BLOCK_FILE, &stating_a_texture_past_the_bound())?;
    let hud = content::shipped_with(HUD_FILE, REFUSED_HUD_DECLARATION)?;
    Ok(vec![
        as_read_from_a_game_directory(&blocks)?,
        as_read_from_a_game_directory(&overlong)?,
        as_read_from_a_game_directory(&hud)?,
        over_the_layer_budget()?,
        a_reload_over_a_broken_declaration()?,
        a_reload_dropping_a_block_the_world_holds()?,
        a_reload_declaring_nothing_solid()?,
        a_reload_over_the_layer_budget()?,
    ])
}

/// A block declaration stating a texture key one character longer than a declared
/// value may be.
///
/// **The overlong value is `texture` and not `name`, and that is a decision about the
/// page rather than about the loader.** Both fields are checked by one bound through
/// one rendering, so either trips the same refusal down the same path; what differs
/// is what the refusal then quotes back. A declaration whose *name* is 257 characters
/// is refused naming itself, so the line a page has to carry holds a 257-character
/// block id — unreadable, and it teaches a mod author nothing that the count in the
/// cause does not already say. With the texture overlong the block still names itself
/// `example:amber` and the whole refusal fits on a line somebody can read.
///
/// The value is assembled by the chunk rather than written out, which a declaration
/// may do because a declaration is code that ran.
fn stating_a_texture_past_the_bound() -> String {
    let padding = ONE_CHARACTER_PAST_THE_BOUND - NAMESPACE.chars().count();
    format!(
        "return {{\n\
         \tname = 'example:amber',\n\
         \ttexture = '{NAMESPACE}' .. string.rep('q', {padding}),\n\
         \tsolid = true,\n\
         }}\n"
    )
}

/// What the client writes when a content root needs more array-texture layers than
/// the session reading it has left.
///
/// **Produced differently from the three above, and it has to be.** Those are
/// refusals a *launch* meets, so the client's own preparation produces each of them
/// and each names the root it was given. A session's layer budget can only be
/// exhausted by a session that has already spent it, which no launch has — so the
/// root is read against an assignment that has, and what came back is rendered
/// through the same shipped reporting.
///
/// # Errors
///
/// Returns an error if the root was read, which is a session with layers left rather
/// than one out of them.
fn over_the_layer_budget() -> Result<String, Box<dyn Error>> {
    let root = content::shipped_copy()?.declaring_block(BLOCK_FILE, CARRYING_A_NEW_TEXTURE_KEY)?;
    let spent = crate::reload_content::spent_all()?;
    let refused = mc_sim::content::load(root.path(), &spent)
        .err()
        .ok_or(THE_BUDGET_WAS_NOT_SPENT)?;
    Ok(normalised(&support::reported(&Ending::failed(
        &refused, "",
    ))?))
}

/// What the client writes for the content root at `root`, with the fixture's own
/// temporary path rewritten to the root a person runs against.
///
/// # Errors
///
/// Returns an error if the root was accepted, or if what was written does not name
/// the fixture root — in which case the rewrite below would be a silent no-op and the
/// text compared against the pages would be one no page could ever carry.
fn as_read_from_a_game_directory(root: &content::ContentRoot) -> Result<String, Box<dyn Error>> {
    let printed = support::refusal_printed_over(root.path())?;
    Ok(normalised(&rewritten(&printed, root.path())?))
}

/// The author saves a typo into the declaration they were editing.
///
/// The most common of the four by a distance, and the only one whose words name a
/// file — so it is the only reload refusal the fixture-root rewrite has anything to
/// do.
///
/// # Errors
///
/// Returns an error if the fixture cannot be built, if no refusal was reported, or if
/// what was reported does not name the fixture root.
fn a_reload_over_a_broken_declaration() -> Result<String, Box<dyn Error>> {
    let root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    let root = restating_raw(root, STONE_FILE, STONE_MISSPELLING_SOLID)?;
    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let said = refusal_reported(&mut client)?;
    as_a_person_reads_it(&rewritten(&said, root.path())?)
}

/// The author deletes two declarations the running world still holds.
///
/// Two rather than one, because the refusal names **every** such block in ascending
/// order and a page quoting one block cannot show that. The world holds grass and
/// stone because the client stands on a grass floor under a stone ceiling.
///
/// # Errors
///
/// Returns an error if the fixture cannot be built or no refusal was reported.
fn a_reload_dropping_a_block_the_world_holds() -> Result<String, Box<dyn Error>> {
    let root = shipped()?;
    let (mut client, reports) = a_client_over(&root, standing(), |registry| {
        floor_under_a_ceiling(registry, GRASS, STONE)
    })?;
    let root = root.not_declaring_blocks(&[GRASS_FILE, STONE_FILE])?;
    reports.changed(&[block_path(&root, GRASS_FILE)])?;
    let said = refusal_reported(&mut client)?;
    as_a_person_reads_it(&said)
}

/// The author takes the solidity off every block there is.
///
/// # Errors
///
/// Returns an error if the fixture cannot be built or no refusal was reported.
fn a_reload_declaring_nothing_solid() -> Result<String, Box<dyn Error>> {
    let mut root = shipped()?;
    let (mut client, reports) = a_client_on(&root, GRASS)?;
    for (file, block) in [
        (DIRT_FILE, DIRT),
        (GRASS_FILE, GRASS),
        (STONE_FILE, STONE),
        (WATER_FILE, WATER),
    ] {
        root = restating(root, file, &Declaration::of(block).solid(false))?;
    }
    reports.changed(&[block_path(&root, STONE_FILE)])?;
    let said = refusal_reported(&mut client)?;
    as_a_person_reads_it(&said)
}

/// A session that has spent every layer it has meets a block declaring a new key.
///
/// **A different string from the launch spelling of the same sentence**, and that is
/// the reason it is produced here as well: a reload wraps it in two more layers —
/// the reload's own outer sentence and the content door's — so a page quoting the
/// launch text for a reload would be quoting something no run prints.
///
/// # Errors
///
/// Returns an error if the fixture cannot be built or no refusal was reported.
fn a_reload_over_the_layer_budget() -> Result<String, Box<dyn Error>> {
    let root = shipped()?;
    let (mut client, reports) = a_client_having_spent_every_layer(&root)?;
    let root = declaring(root, AMBER_FILE, &amber())?;
    reports.changed(&[block_path(&root, AMBER_FILE)])?;
    let said = refusal_reported(&mut client)?;
    as_a_person_reads_it(&said)
}

/// A client on a floor of grass whose session has already spent its whole budget,
/// watching the root it plays.
///
/// # Errors
///
/// Returns an error if the root does not read against a spent assignment, if the
/// world does not build, or if the content declares no solid block.
fn a_client_having_spent_every_layer(
    root: &content::ContentRoot,
) -> Result<(InputHarness, Reports), Box<dyn Error>> {
    let (simulation, holding) = playing_serving(
        root.path(),
        standing(),
        |registry| floor_of(registry, GRASS),
        &spent_all()?,
    )?;
    let (watching, reports) = watch();
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client.attach_reload(mc_sim::reload::ContentReload::watching(
        root.path().to_owned(),
        Box::new(watching),
    ));
    Ok((client, reports))
}

/// The words the first boundary that reported anything refused with.
///
/// **The product's own output**, taken out of the report the frame path reads rather
/// than rendered here from an error this module happened to hold: what a page quotes
/// has to be what a run produced, and a rendering performed here would be a second
/// copy of the client's own decision about what to say.
///
/// # Errors
///
/// Returns an error if no boundary reported anything inside the run's patience, or if
/// the candidate was taken up — a fixture that meant to be refused and was accepted
/// has no refusal for a page to quote.
fn refusal_reported(client: &mut InputHarness) -> Result<String, Box<dyn Error>> {
    let started = Instant::now();
    while may_cross_another(started) {
        match boundary(client) {
            None => pause_between_boundaries(),
            Some(Attempt::Refused { said }) => return Ok(said),
            Some(Attempt::TakenUp) => {
                return Err(
                    "this fixture's candidate was taken up, so there is no refusal for a \
                            page to quote"
                        .into(),
                );
            }
        }
    }
    Err("no boundary reported anything, so this fixture produced no refusal".into())
}

/// One reload refusal as a person reads it: the shipped prefix, the sentence the
/// client knows, and the chain the report carried.
///
/// **Nothing here is spelled twice.** The sentence is the client's own constant, the
/// joiner between it and the chain is `Ending::failed_under`'s, and the prefix is
/// `report`'s — so rewording any of the three moves this text and the pages with it.
///
/// # Errors
///
/// Returns an error if the reporting cannot be written to a sink.
fn as_a_person_reads_it(chain: &str) -> Result<String, Box<dyn Error>> {
    let ending = Ending::failed_under(CONTENT_NOT_TAKEN_UP, &Layer(chain.to_owned()));
    Ok(normalised(&support::reported(&ending)?))
}

/// A failure whose message is a chain already rendered, so that the client's own
/// `Ending` vocabulary can compose the line above it.
///
/// It carries no source: the chain beneath the sentence has already been walked by
/// the client, and walking it twice would insert the joiner twice.
#[derive(Debug)]
struct Layer(String);

impl fmt::Display for Layer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for Layer {}

/// `said` with the fixture's own temporary path rewritten to the root a person runs
/// against.
///
/// # Errors
///
/// Returns an error if `said` does not name the fixture root, in which case the
/// rewrite would be a silent no-op and the text compared against the pages would name
/// a directory that exists for a hundred milliseconds.
fn rewritten(said: &str, fixture: &std::path::Path) -> Result<String, Box<dyn Error>> {
    let named = fixture.display().to_string();
    if !said.contains(&named) {
        return Err(format!(
            "this guard has to rewrite the fixture root out of a refusal before comparing it with \
             a page, and what was written does not name the root it was given. What was written \
             was:\n{said}"
        )
        .into());
    }
    let shipped: PathBuf = SHIPPED_ROOT.iter().collect();
    Ok(said.replace(&named, &shipped.display().to_string()))
}

/// A text in the one spelling both sides of a comparison are held to: no trailing
/// whitespace on any line, no blank lines at the end, and path separators written the
/// same way on every platform.
///
/// Leading whitespace is left alone, because the caret diagnostic's own indentation is
/// part of what a page has to get right.
#[must_use]
pub fn normalised(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .replace('\\', "/")
}
