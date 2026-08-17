//! What a reload published: which layer each texture key holds, the serial it
//! was published under, how many layers the session has spent, and the HUD that
//! travelled with it.
//!
//! # Every number here is derived from a declared list, never written down
//!
//! The shipped root declares four blocks and each declares `texture` equal to
//! `name`, so its four texture keys are its four block names — listed in
//! [`SHIPPED_KEYS`] rather than read back from a registry, for the reason
//! [`crate::reload`] lists the block names: a fixture that discovered them would
//! go on passing over a root that had stopped declaring one.
//!
//! The layer a fresh assignment gives each of them is that key's position in the
//! sorted list, and the layer a *new* key takes is the list's length. Neither is
//! spelled as a digit anywhere below: `4` and `5` do not appear in this module,
//! and a reader who wants to know why `base:amber` takes layer four is meant to
//! find [`THE_NEXT_UNUSED_LAYER`] rather than a literal.
//!
//! # No absolute serial is stated, and that is deliberate
//!
//! Nothing here says what number a launch publishes under. What the scenarios ask
//! is whether a serial *moved*, whether two reloads got *distinct* ones, and
//! whether a refusal left it *where it was* — all of which are relations between
//! observed values. [`Run`] is the verdict those relations come to, so a change
//! to where the counter starts moves no expectation in this suite.
//!
//! # An assignment is compared as a map, never as a list
//!
//! `LayerAssignment::entries` states its pairs in whatever order the assignment
//! holds them, and which order that is — key-ascending or layer-ascending — is
//! not something a reader may depend on. So every comparison below is over a
//! `BTreeMap`: total in the keys it holds, blind to their order.
//!
//! `spent` is carried beside that map because it is the one value `live.len()`
//! would get wrong — a retired layer is spent and is not live — and stating it
//! here means a defect in that distinction reads as one wrong number rather than
//! as several wrong layers further downstream.
//!
//! # A session that has already spent its budget is built through the real
//! constructor
//!
//! [`spent_all_but_one`] and [`spent_all`] reach 255 and 256 assigned layers by
//! appending that many keys through `LayerAssignment::appending` — the only door
//! into the type — rather than by constructing one. Reaching the same state
//! organically would take two hundred and fifty reloads. The synthetic keys are
//! namespaced so that every one of them sorts *after* all four shipped keys,
//! which is what keeps the shipped four on the layers a launch would have given
//! them and keeps every expectation below derived rather than read out of the
//! value under test.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names types the implementation has not written yet, exactly as
//! [`crate::reload`] does, and a module declared in `support/mod.rs` is compiled
//! into every binary that says `mod support;`. A binary including this must
//! declare `mod support;` and the reload fixture as well.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::sync::Arc;

use mc_core::content::{LAYERS_A_SESSION_MAY_ASSIGN, LayerAssignment};
use mc_core::hud::HudLayout;
use mc_core::id::TextureKey;
use mc_sim::content::LoadedContent;
use mc_sim::reload::ReloadRefusal;
use mc_sim::simulation::{Accepted, PublishedContent};

use crate::reload::{DIRT, GRASS, STONE, WATER};
use crate::support::content::{ContentRoot, HUD_DIRECTORY, shipped_copy};

/// The four texture keys the shipped root declares, in ascending order — which
/// is the order a fresh assignment hands layers out in.
///
/// Listed rather than read, and asserted ascending by [`fresh_layers`] so that a
/// later edit reordering it cannot silently move every expectation below.
pub const SHIPPED_KEYS: [&str; 4] = [DIRT, GRASS, STONE, WATER];

/// The layer a key introduced over the shipped four takes, which is also how many
/// layers a launch over the shipped root spends.
///
/// **Derived from the list and never written as a digit.** It is the count of
/// keys a launch assigns, so it is also the first index nothing holds.
pub const THE_NEXT_UNUSED_LAYER: u16 = SHIPPED_KEYS.len() as u16;

/// The namespace the synthetic keys that fill a session's budget are declared in.
///
/// Chosen to sort after `base:`, so a fixture that has spent nearly the whole
/// budget still leaves the shipped four on the layers a launch would have given
/// them.
const FILLER_NAMESPACE: &str = "zz";

/// A second block declared for the first time, and the file it arrives in.
///
/// Needed wherever one new texture key is not enough — a candidate introducing two
/// of them at once, and a session that accepts two candidates in a row.
pub const BERYL: &str = "base:beryl";
pub const BERYL_FILE: &str = "beryl.luau";

/// The HUD element a reload widens to show that the HUD is applied with the blocks —
/// see `tests/reload_hud_reaches_the_frame.rs` — and the file it is declared in.
pub const CROSSBAR: &str = "base:crosshair-horizontal";
pub const CROSSBAR_FILE: &str = "crosshair-horizontal.toml";

/// The extent the shipped declaration states, and the extent an author widens it
/// to.
///
/// Both written out, because the edit under test *is* the difference between
/// them: a fixture deriving either from the file it edits could not say which one
/// it meant.
pub const SHIPPED_CROSSBAR_EXTENT: [u32; 2] = [9, 1];
pub const WIDENED_CROSSBAR_EXTENT: [u32; 2] = [21, 1];

/// The crossbar declaration with its extent widened and nothing else touched.
///
/// Written out rather than produced by editing the shipped text, so a reader can
/// see the whole of what the author saved. Every field is the shipped one; only
/// `size` differs.
pub const WIDENED_CROSSBAR: &str = "name = \"base:crosshair-horizontal\"\nanchor = \"center\"\nsize = [21, 1]\ndraw = \
     \"fill\"\ncolor = \"#FFFFFFFF\"\noutline = \"#000000FF\"\n";

/// The serial one reload reported, or nothing where it was refused or there was
/// no simulation to hand a candidate to.
///
/// Taken by reference so that the verdict a scenario compares can still be read
/// out of the same answer by [`crate::reload::adoption`], which is phase one's
/// and carries no serial.
#[must_use]
pub fn serial_reported(answered: &Option<Result<Accepted, ReloadRefusal>>) -> Option<u32> {
    match answered {
        Some(Ok(accepted)) => Some(accepted.serial.get()),
        Some(Err(_)) | None => None,
    }
}

/// How the serials a session published stand against each other.
///
/// **A total verdict over relations, so no expectation states a number.** Every
/// way of a serial failing to move has an arm of its own, which is what makes an
/// assertion against the good arm reject a publisher whose counter never moves —
/// the failure this whole group of scenarios exists to close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Run {
    /// Each was later than the one before it, and the first was later than the
    /// serial the launch published.
    EachLaterThanTheLast,
    /// Two of them are the same number, so a reader cannot tell one publication
    /// from the other.
    Repeated(Vec<u32>),
    /// One went backwards, or one is the serial the launch already published.
    OutOfOrder { launch: u32, published: Vec<u32> },
    /// One of the publications reported no serial at all.
    OneReportedNothing,
}

/// What the run `published` makes, against the `launch` serial it began from.
#[must_use]
pub fn run_of(launch: u32, published: &[Option<u32>]) -> Run {
    let Some(serials) = published.iter().copied().collect::<Option<Vec<u32>>>() else {
        return Run::OneReportedNothing;
    };
    let mut seen = BTreeSet::new();
    if !serials.iter().all(|serial| seen.insert(*serial)) {
        return Run::Repeated(serials);
    }
    let ascending = serials.windows(2).all(ascends);
    if !ascending || serials.first().is_some_and(|first| *first <= launch) {
        return Run::OutOfOrder {
            launch,
            published: serials,
        };
    }
    Run::EachLaterThanTheLast
}

/// Whether a pair of adjacent serials goes up.
fn ascends(pair: &[u32]) -> bool {
    matches!(pair, [before, after] if before < after)
}

/// What a client is publishing: the serial, the layer every live key holds, and
/// how many layers the session has spent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publishing {
    pub serial: u32,
    pub layers: BTreeMap<String, u16>,
    pub spent: u16,
}

/// What `content` states, as a [`Publishing`].
///
/// # Errors
///
/// Returns an error when there is no published content at all, which is a client
/// with no simulation rather than one publishing anything a scenario could read.
pub fn publishing(content: Option<Arc<PublishedContent>>) -> Result<Publishing, Box<dyn Error>> {
    let published =
        content.ok_or("this fixture's client publishes no content, so it has no serial to read")?;
    Ok(Publishing {
        serial: published.serial.get(),
        layers: assigned(&published),
        spent: published.resolved.layers().spent(),
    })
}

/// The layer every live key of `published` holds.
///
/// Read through `ResolvedContent::layer_assignment`, which is what `ContentView`
/// reads on the way to the packer — so a scenario asserting a layer here is
/// asserting the value a frame is drawn against and not a second accessor beside
/// it.
fn assigned(published: &PublishedContent) -> BTreeMap<String, u16> {
    published
        .resolved
        .layer_assignment()
        .map(|(key, layer)| (key.as_str().to_owned(), layer))
        .collect()
}

/// The layers a launch over the shipped root hands out: each key of
/// [`SHIPPED_KEYS`] on its own position in that list.
///
/// # Errors
///
/// Returns an error unless the list is strictly ascending. The whole of this
/// module's arithmetic is that a fresh assignment numbers ascending keys from
/// zero, and a list somebody reordered would move every expectation below at once
/// with nothing saying so.
pub fn fresh_layers() -> Result<BTreeMap<String, u16>, Box<dyn Error>> {
    let ascending = SHIPPED_KEYS
        .windows(2)
        .all(|pair| matches!(pair, [before, after] if before < after));
    if !ascending {
        return Err(format!(
            "the four shipped texture keys have to be listed in ascending order, because every \
             expectation in this module is a key's position in that list, and they are listed \
             {SHIPPED_KEYS:?}"
        )
        .into());
    }
    Ok(SHIPPED_KEYS
        .iter()
        .zip(0..)
        .map(|(key, layer)| ((*key).to_owned(), layer))
        .collect())
}

/// The layers a launch hands out, with `extra` added at the layers stated.
///
/// # Errors
///
/// Returns an error unless [`SHIPPED_KEYS`] is ascending, or if `extra` names a
/// key the launch already assigned — an expectation restating one of the four
/// could no longer say the four were left alone.
pub fn layers_beside(extra: &[(&str, u16)]) -> Result<BTreeMap<String, u16>, Box<dyn Error>> {
    let mut layers = fresh_layers()?;
    for (key, layer) in extra {
        if layers.insert((*key).to_owned(), *layer).is_some() {
            return Err(format!(
                "`{key}` is one of the four keys a launch already assigns, so stating it again \
                 here would build an expectation that cannot say whether the launch's four were \
                 left where they were"
            )
            .into());
        }
    }
    Ok(layers)
}

/// The layers a launch hands out with the key at `retired` taken out and nothing
/// renumbered.
///
/// That is the whole of what "appended, never renumbered" means, stated as an
/// expectation: the fresh map, one entry lighter, every other key on the layer it
/// held.
///
/// # Errors
///
/// Returns an error unless [`SHIPPED_KEYS`] is ascending, or if the launch never
/// assigned `retired`.
pub fn layers_without(retired: &str) -> Result<BTreeMap<String, u16>, Box<dyn Error>> {
    let mut layers = fresh_layers()?;
    if layers.remove(retired).is_none() {
        return Err(format!(
            "`{retired}` is not one of the four keys a launch assigns, so taking it out of the \
             expectation would leave the expectation the launch's own"
        )
        .into());
    }
    Ok(layers)
}

/// An assignment that has spent every layer of the session's budget but one.
///
/// # Errors
///
/// Returns an error if a synthetic key does not parse, if the fixture would not
/// hold the count it is named for, or if appending is refused — which is the
/// budget turning away a fixture that is supposed to fit inside it.
pub fn spent_all_but_one() -> Result<LayerAssignment, Box<dyn Error>> {
    assigning(LAYERS_A_SESSION_MAY_ASSIGN - 1)
}

/// An assignment that has spent every layer of the session's budget.
///
/// # Errors
///
/// Returns an error if a synthetic key does not parse, if the fixture would not
/// hold the count it is named for, or if appending is refused.
pub fn spent_all() -> Result<LayerAssignment, Box<dyn Error>> {
    assigning(LAYERS_A_SESSION_MAY_ASSIGN)
}

/// An assignment over the four shipped keys and enough synthetic ones to have
/// spent `layers` in all.
///
/// **The guard counts the keys and never the result's own `spent`.** Asking the
/// value under test whether it spent what it was told to spend would make a
/// broken bound report itself as a broken fixture, and the scenarios that grade
/// that bound would error out instead of failing.
///
/// # Errors
///
/// Returns an error if a key does not parse, if the key set is not the size this
/// asked for, or if appending is refused.
fn assigning(layers: usize) -> Result<LayerAssignment, Box<dyn Error>> {
    let mut keys = BTreeSet::new();
    for key in SHIPPED_KEYS {
        keys.insert(TextureKey::parse(key)?);
    }
    for filler in 0..layers - SHIPPED_KEYS.len() {
        keys.insert(TextureKey::parse(&format!(
            "{FILLER_NAMESPACE}:filler{filler:04}"
        ))?);
    }
    if keys.len() != layers {
        return Err(format!(
            "this fixture has to hand {layers} distinct texture keys to an assignment that has \
             spent nothing, and it assembled {count} — so the session it builds is not the one \
             the scenario names",
            count = keys.len()
        )
        .into());
    }
    Ok(LayerAssignment::none().appending(&keys)?)
}

/// The candidate the content root at `root` builds against the layers the content
/// in `serving` has already spent.
///
/// **The layers come from what the client is publishing and never from
/// `LayerAssignment::none`.** That is what a reload's build stage reads, and a
/// fixture reading anything else would hand the client a candidate no running
/// session could have produced.
///
/// It takes exactly what `Session::content()` hands back, so a scenario writes
/// `candidate_against(&root, client.content())` and the one place a client with no
/// world is turned into a failure is here.
///
/// # Errors
///
/// Returns an error where nothing is being published, and whichever reader refused
/// the root — the budget refusal included.
pub fn candidate_against(
    root: &ContentRoot,
    serving: Option<Arc<PublishedContent>>,
) -> Result<LoadedContent, Box<dyn Error>> {
    let published = serving.ok_or(NOTHING_IS_SERVING)?;
    Ok(mc_sim::content::load(
        root.path(),
        published.resolved.layers(),
    )?)
}

/// What a fixture says when it was asked to read something out of a client that
/// is publishing no content at all — which is a client with no world rather than
/// one serving anything a scenario could read.
pub const NOTHING_IS_SERVING: &str = "this fixture's client publishes no content, so there are no spent layers to read a candidate \
     against and no serial to compare";

/// What reading a content root against the layers a session has already spent
/// came to.
///
/// **A total verdict and never a `Result` propagated out of a test.** A read that
/// was supposed to be refused then fails on the comparison, naming the layers it
/// produced, instead of ending the test before its assertion ran — and a refusal
/// that was supposed to be a read does the same in the other direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    /// The root was read, and its assignment states these layers.
    Read(BTreeMap<String, u16>),
    /// The root was refused, and the deepest cause said this.
    ///
    /// The deepest cause and not the whole chain, because that sentence is the one
    /// a page quotes and the one a scenario about wording is written about.
    /// Whatever framing sits above it belongs to `documented_refusals.rs`, which
    /// compares a real run against a real page.
    Refused { said: String },
}

/// What reading `root` against the layers `spent` already holds came to.
///
/// # Errors
///
/// Returns an error only if the layers a read produced cannot be spelled, never
/// for a refusal — a refusal is one of the two verdicts.
pub fn reading(root: &ContentRoot, spent: &LayerAssignment) -> Result<Reading, Box<dyn Error>> {
    match mc_sim::content::load(root.path(), spent) {
        Ok(loaded) => Ok(Reading::Read(
            loaded
                .resolved
                .layer_assignment()
                .map(|(key, layer)| (key.as_str().to_owned(), layer))
                .collect(),
        )),
        Err(refused) => Ok(Reading::Refused {
            said: deepest_cause(&refused),
        }),
    }
}

/// What the last thing under `refused` says — the sentence nothing else caused.
fn deepest_cause(refused: &dyn Error) -> String {
    let mut said = refused.to_string();
    let mut beneath = refused.source();
    while let Some(cause) = beneath {
        said = cause.to_string();
        beneath = cause.source();
    }
    said
}

/// A copy of the shipped root whose crossbar declaration states the widened
/// extent.
///
/// # Errors
///
/// Returns an error if the root cannot be copied, if the write fails, or if the
/// shipped root does not declare the crossbar — a root that never declared it is
/// not a root whose declaration an author widened.
pub fn shipped_widening_the_crossbar() -> Result<ContentRoot, Box<dyn Error>> {
    let copied = shipped_copy()?;
    let declared = copied.path().join(HUD_DIRECTORY).join(CROSSBAR_FILE);
    if !declared.is_file() {
        return Err(format!(
            "this fixture has to widen `{HUD_DIRECTORY}/{CROSSBAR_FILE}` in a copy of the shipped \
             content root, and the shipped root does not declare it. What it would build is a root \
             that gained a crosshair rather than one whose crosshair an author widened"
        )
        .into());
    }
    fs::write(&declared, WIDENED_CROSSBAR)?;
    Ok(copied)
}

/// The extent `layout` states for the crossbar, or nothing where it declares no
/// such element.
///
/// **`None` rather than a default**, so a layout that lost the element and one
/// that states an extent of its own are told apart by shape.
#[must_use]
pub fn crossbar_extent(layout: &HudLayout) -> Option<[u32; 2]> {
    layout
        .elements()
        .iter()
        .find(|element| element.name.as_str() == CROSSBAR)
        .map(|element| element.size)
}
