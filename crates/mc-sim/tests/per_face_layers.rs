//! Six facing keys spend layers out of the one budget single keys spend, and
//! running out refuses the load whole.
//!
//! An array-texture layer is a scarce, session-lifetime resource: eight bits of
//! every packed vertex carry a layer index, so one session hands out 256 of them
//! and a relaunch is the only thing that gives any back. Before this feature a
//! block cost at most one; now it costs up to six. Nothing about the budget
//! changes — the refusal is the one that already existed, reached by a new route —
//! and these are the readings that say so.
//!
//! # Every number here is arithmetic, and none of it is read back off a run
//!
//! **Five** is six facings naming five distinct keys: the declaration that gives
//! `up` and `down` one key and its four sides four others. **256** is 250 already
//! spent plus six introduced. **257 against 256** is 251 plus six, against the
//! budget a session has. Each is computed in the test from the fixture beside it,
//! because a count snapshotted from the first green run records whatever the code
//! did that day and can never fail for the right reason afterwards.
//!
//! # A session near its budget is built by appending, never by construction
//!
//! Reaching 250 assigned layers organically takes two hundred and fifty reloads.
//! [`assigning`] gets there through `LayerAssignment::appending`, the only door
//! into the type, over the keys the shipped root declares plus synthetic ones
//! namespaced to sort after them — so the shipped four stay on the layers a launch
//! would have given them, and the block this fixture adds is the only thing
//! introducing anything.
//!
//! **The fixture counts its own keys and never asks the result what it spent.**
//! Asking the value under test whether it spent what it was told to spend would
//! make a broken bound report itself as a broken fixture, and the scenarios
//! grading that bound would error out instead of failing.
//!
//! # The all-or-nothing half is the one a passing implementation gets wrong
//!
//! A refusal that had already handed out four of the six layers it needed would
//! leave the session with four layers gone and nothing drawing them, and every
//! other reading in this file would still agree. So the refusal is followed by
//! two questions the assignment can only answer if nothing happened to it: does it
//! still hold every pair it held, and does the next key it meets still take the
//! layer that was next.

#[path = "support/roots.rs"]
mod roots;
mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use mc_core::content::LayerAssignment;
use mc_core::id::TextureKey;
use mc_sim::content::ContentError;
use support::TestResult;
use tempfile::TempDir;

/// The texture keys the shipped root declares, ascending.
///
/// Written out rather than read back, and checked against the shipped root by
/// [`the_shipped_root_spends_one_layer_per_key`] below, so that a root which grew
/// one more key fails there instead of moving every expectation in this file.
///
/// **Eight, not four, and `base:grass` is not among them.** The grass block
/// declares a key per facing: `base:grass_top` upward, `base:dirt` downward and
/// four side keys of its own, so the name `base:grass` is a *block* name and is
/// no longer a texture key at all. Dirt and stone still state one key for all six
/// of their facings.
const SHIPPED_KEYS: [&str; 8] = [
    "base:dirt",
    "base:grass_side_east",
    "base:grass_side_north",
    "base:grass_side_south",
    "base:grass_side_west",
    "base:grass_top",
    "base:stone",
    "base:water",
];

/// How many array-texture layers one session may assign.
///
/// **Written out rather than read from `LAYERS_A_SESSION_MAY_ASSIGN`.** That
/// constant is the declaration under test, and an expectation assembled from it
/// reads back whatever it became. The value is the packed vertex's eight-bit
/// layer field — two to the eighth.
const A_SESSIONS_BUDGET: usize = 256;

/// The namespace the synthetic keys that fill a session's budget are declared in,
/// chosen to sort after `base:`.
const FILLER_NAMESPACE: &str = "zz";

/// The block every fixture here adds, and the file it arrives in.
const AMBER: &str = "example:amber";
const AMBER_FILE: &str = "amber.luau";

/// The six keys a block declaring `up` and `down` alike states, in the order the
/// six words are written.
///
/// Five distinct values across six facings, which is where the five in the first
/// scenario comes from — counted below, never written as a digit.
const SHARED_TOP_AND_BOTTOM: [&str; 6] = [
    "example:ash",
    "example:ash",
    "example:basalt",
    "example:chert",
    "example:diorite",
    "example:gabbro",
];

/// Six keys no other declaration names, one per facing, all six distinct.
const SIX_UNASSIGNED_KEYS: [&str; 6] = [
    "example:quartz",
    "example:ash",
    "example:basalt",
    "example:chert",
    "example:diorite",
    "example:gabbro",
];

/// The six facing words, in the order a declaration writes them.
const SIX_FACINGS: [&str; 6] = ["up", "down", "north", "south", "east", "west"];

/// What reading a content root against the layers a session has already spent
/// came to.
///
/// **A total verdict and never a `Result` propagated out of a test.** A read that
/// was supposed to be refused then fails on its own comparison, naming what it
/// produced, instead of ending the test before its assertion ran — and a refusal
/// that was supposed to be a read does the same in the other direction.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Reading {
    /// The root was read, and the session has now spent this many layers.
    Spent(u16),
    /// The root needs more layers than the session has left.
    OverBudget {
        needed: usize,
        spent: usize,
        states_the_budget: bool,
    },
    /// Refused for some other reason, rendered.
    RefusedOtherwise(String),
}

/// What reading `root` against `spent` came to.
///
/// The budget refusal is read off the variant that carries it rather than by
/// walking the chain and downcasting: the counts are the subject, and a chain walk
/// would report a refusal from some other layer under the same arm.
fn reading(root: &Path, spent: &LayerAssignment) -> Reading {
    match mc_sim::content::load(root, spent) {
        Ok(loaded) => Reading::Spent(loaded.resolved.layers().spent()),
        Err(ContentError::Layers(budget)) => Reading::OverBudget {
            needed: budget.needed,
            spent: budget.spent,
            states_the_budget: budget.to_string().contains(&A_SESSIONS_BUDGET.to_string()),
        },
        Err(other) => Reading::RefusedOtherwise(other.to_string()),
    }
}

/// A declaration of [`AMBER`] stating `keys` against the six facings.
fn amber_stating(keys: [&str; 6]) -> String {
    let facings: String = SIX_FACINGS
        .into_iter()
        .zip(keys)
        .map(|(word, key)| format!("\t\t{word} = \"{key}\",\n"))
        .collect();
    format!(
        "return {{\n\
         \tname = \"{AMBER}\",\n\
         \ttexture = {{\n{facings}\t}},\n\
         \tsolid = true,\n\
         }}\n"
    )
}

/// A content root declaring [`AMBER`] and nothing else.
///
/// Bare rather than a copy of the shipped one, because "content spending no
/// layers" has to mean this block's keys and no others. A root declaring no HUD is
/// a valid, empty answer, which is what makes a root of one file readable at all.
///
/// # Errors
///
/// Returns an error if the directory cannot be written.
fn root_declaring_only_amber(
    directory: &TempDir,
    keys: [&str; 6],
) -> Result<PathBuf, Box<dyn Error>> {
    let root = directory.path().to_owned();
    let blocks = root.join(roots::BLOCK_DIRECTORY);
    fs::create_dir_all(&blocks)?;
    fs::write(blocks.join(AMBER_FILE), amber_stating(keys))?;
    Ok(root)
}

/// How many distinct values `keys` holds.
fn distinct(keys: [&str; 6]) -> usize {
    keys.into_iter().collect::<BTreeSet<_>>().len()
}

/// An assignment over the shipped keys and enough synthetic ones to have
/// spent `layers` in all.
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
             spent nothing, and it assembled {count} — so the session it builds is not the one the \
             scenario names",
            count = keys.len()
        )
        .into());
    }
    Ok(LayerAssignment::none().appending(&keys)?)
}

/// Which layer each key of `assignment` holds.
fn layers_of(assignment: &LayerAssignment) -> BTreeMap<String, u16> {
    assignment
        .entries()
        .map(|(key, layer)| (key.as_str().to_owned(), layer))
        .collect()
}

/// The layer `assignment` would give a key it has never seen.
///
/// # Errors
///
/// Returns an error if the key does not parse, if appending is refused, or if the
/// assignment turned out to hold that key already — in which case the reading
/// would be about a layer it kept rather than one it handed out.
fn the_next_layer_handed_out(assignment: &LayerAssignment) -> Result<u16, Box<dyn Error>> {
    let unseen = TextureKey::parse("zzz:unseen")?;
    if assignment.layer_of(&unseen).is_some() {
        return Err("this fixture's probe key has to be one the assignment has never seen".into());
    }
    let mut introducing = BTreeSet::new();
    introducing.insert(unseen.clone());
    assignment
        .appending(&introducing)?
        .layer_of(&unseen)
        .ok_or_else(|| "appending a key has to give it a layer".into())
}

#[test]
fn the_shipped_root_spends_one_layer_per_key_it_declares() -> TestResult {
    let root = roots::shipped()?;

    let read = reading(root.path(), &LayerAssignment::none());

    assert_eq!(
        read,
        Reading::Spent(SHIPPED_KEYS.len() as u16),
        "every arithmetic expectation below adds six to a number that assumes the shipped root \
         declares exactly these keys. A root that grew one more has to fail here, where the \
         message says what happened, rather than by moving three unrelated counts by one"
    );
    Ok(())
}

#[test]
fn six_facing_keys_naming_five_distinct_values_spend_five_layers() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring_only_amber(&directory, SHARED_TOP_AND_BOTTOM)?;

    let read = reading(&root, &LayerAssignment::none());

    assert_eq!(
        read,
        Reading::Spent(distinct(SHARED_TOP_AND_BOTTOM) as u16),
        "a block naming one key against both `up` and `down` costs five layers and not six: two \
         blocks may share a texture and so may two facings of one block. Counting facings instead \
         of keys would make the commonest declaration there is — a grass block — a sixth more \
         expensive than it needs to be, out of a budget nothing gives back"
    );
    Ok(())
}

#[test]
fn six_unassigned_facing_keys_added_to_content_spending_250_layers_spend_256() -> TestResult {
    let already_spent = A_SESSIONS_BUDGET - SIX_UNASSIGNED_KEYS.len();
    let spent = assigning(already_spent)?;
    let root = roots::shipped()?.declaring(AMBER_FILE, &amber_stating(SIX_UNASSIGNED_KEYS))?;

    let read = reading(root.path(), &spent);

    assert_eq!(
        read,
        Reading::Spent((already_spent + SIX_UNASSIGNED_KEYS.len()) as u16),
        "six facing keys spend six layers out of the same budget a single key spends one from. \
         There is no second pool and no per-block allowance: the last layer a session has is the \
         last layer, whether the block that wanted it declared one key or six"
    );
    Ok(())
}

#[test]
fn six_unassigned_facing_keys_added_to_content_spending_251_layers_refuse_the_load() -> TestResult {
    let already_spent = A_SESSIONS_BUDGET - SIX_UNASSIGNED_KEYS.len() + 1;
    let spent = assigning(already_spent)?;
    let root = roots::shipped()?.declaring(AMBER_FILE, &amber_stating(SIX_UNASSIGNED_KEYS))?;

    let read = reading(root.path(), &spent);

    assert_eq!(
        read,
        Reading::OverBudget {
            needed: already_spent + SIX_UNASSIGNED_KEYS.len(),
            spent: already_spent,
            states_the_budget: true,
        },
        "one layer past the budget refuses the whole load, and what the author is told has to \
         carry all three numbers they can act on: how many this content needs, how many are \
         already gone, and how many a session has at all. Without the last of the three the only \
         reading left is that their content is too big, which is not what happened"
    );
    Ok(())
}

#[test]
fn a_load_refused_for_want_of_layers_leaves_every_assigned_layer_holding_its_key() -> TestResult {
    let already_spent = A_SESSIONS_BUDGET - SIX_UNASSIGNED_KEYS.len() + 1;
    let spent = assigning(already_spent)?;
    let before = layers_of(&spent);
    let root = roots::shipped()?.declaring(AMBER_FILE, &amber_stating(SIX_UNASSIGNED_KEYS))?;

    let refused = reading(root.path(), &spent);
    let after = layers_of(&spent);
    let next = the_next_layer_handed_out(&spent)?;

    assert_eq!(
        (refused, after, next),
        (
            Reading::OverBudget {
                needed: already_spent + SIX_UNASSIGNED_KEYS.len(),
                spent: already_spent,
                states_the_budget: true,
            },
            before,
            already_spent as u16
        ),
        "the refusal is all or nothing, and the two readings after it are what say so. A load \
         that had handed out four of the six layers it needed before running out would leave four \
         layers spent on a block that never registered — invisible, unrecoverable without a \
         relaunch, and it would still satisfy every count in this file. The layer the next key \
         takes is the one that was next before the refusal, so nothing was consumed and nothing \
         was renumbered"
    );
    Ok(())
}
