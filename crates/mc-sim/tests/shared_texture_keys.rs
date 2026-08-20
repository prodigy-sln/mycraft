//! Two blocks may draw from one texture, and it costs one layer rather than two.
//!
//! An array-texture layer is a scarce, session-lifetime resource: eight bits of
//! every packed vertex carry a layer index, so one session hands out 256 of them
//! and only a relaunch gives any back. What decides how many a content root
//! spends is the set of **keys its blocks declare**, and never the set of blocks
//! — a rule that was unobservable while every block in existence declared its
//! texture equal to its own name, because under that convention the two sets are
//! the same size.
//!
//! # The number is counted, never written as a digit
//!
//! Each reading counts the distinct keys of its own fixture and compares the
//! load against that count. A digit copied out of a run records whatever the
//! loader did that day.
//!
//! # The second reading is the first one's control
//!
//! "Two blocks spend one layer" is satisfied by a loader that reports one layer
//! for anything at all. The pair below differs in exactly one character — the
//! second block's declared key — so the two readings share a fixture shape and
//! disagree only about the thing under test.

#[path = "support/roots.rs"]
mod roots;
mod support;

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use mc_core::content::LayerAssignment;
use support::TestResult;
use tempfile::TempDir;

/// The two blocks these roots declare, and the files they arrive in.
///
/// Different names, deliberately: the whole subject is that a name is not a key.
const FIRST_BLOCK: &str = "example:amber";
const FIRST_FILE: &str = "amber.luau";
const SECOND_BLOCK: &str = "example:cobalt";
const SECOND_FILE: &str = "cobalt.luau";

/// The one key both blocks declare in the first reading.
///
/// It is neither block's name, so a loader counting names rather than keys has
/// nothing to accidentally agree with.
const SHARED_KEY: &str = "example:gold";

/// The key the second block declares in the control, instead of sharing.
const A_SECOND_KEY: &str = "example:quartz";

/// What reading a content root against the layers a session has already spent
/// came to.
///
/// A total verdict and never a `Result` propagated out of a test: a read that
/// was supposed to succeed then fails on its own comparison, naming what it
/// produced, instead of ending the test before its assertion ran.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Reading {
    /// The root was read, and the session has now spent this many layers.
    Spent(u16),
    /// Refused, rendered.
    Refused(String),
}

/// What reading `root` against a session that has spent nothing came to.
fn reading(root: &Path) -> Reading {
    match mc_sim::content::load(root, &LayerAssignment::none()) {
        Ok(loaded) => Reading::Spent(loaded.resolved.layers().spent()),
        Err(refused) => Reading::Refused(refused.to_string()),
    }
}

/// A declaration of `block` stating `key` as its whole texture.
fn declaring(block: &str, key: &str) -> String {
    format!(
        "return {{\n\
         \tname = \"{block}\",\n\
         \ttexture = \"{key}\",\n\
         \tsolid = true,\n\
         }}\n"
    )
}

/// A content root declaring the two blocks above and nothing else.
///
/// Bare rather than a copy of the shipped one, because "content spending no
/// layers" has to mean these blocks' keys and no others. A root declaring no HUD
/// is a valid, empty answer, which is what makes a root of two files readable.
///
/// # Errors
///
/// Returns an error if the directory cannot be written.
fn root_declaring(directory: &TempDir, keys: [&str; 2]) -> Result<PathBuf, Box<dyn Error>> {
    let root = directory.path().to_owned();
    let blocks = root.join(roots::BLOCK_DIRECTORY);
    fs::create_dir_all(&blocks)?;
    fs::write(blocks.join(FIRST_FILE), declaring(FIRST_BLOCK, keys[0]))?;
    fs::write(blocks.join(SECOND_FILE), declaring(SECOND_BLOCK, keys[1]))?;
    Ok(root)
}

/// How many distinct values `keys` holds.
fn distinct(keys: [&str; 2]) -> u16 {
    let counted = keys.into_iter().collect::<BTreeSet<_>>().len();
    u16::try_from(counted).unwrap_or(u16::MAX)
}

#[test]
fn two_differently_named_blocks_declaring_one_texture_key_spend_one_layer() -> TestResult {
    let directory = TempDir::new()?;
    let keys = [SHARED_KEY, SHARED_KEY];
    let root = root_declaring(&directory, keys)?;

    let read = reading(&root);

    assert_eq!(
        read,
        Reading::Spent(distinct(keys)),
        "what a content root spends is one layer per distinct key its blocks declare, and never \
         one per block. These two are called {FIRST_BLOCK} and {SECOND_BLOCK} and both draw from \
         {SHARED_KEY}: charging per block would spend a second layer on a texture that is already \
         in the array, out of a budget nothing gives back until the client is restarted"
    );
    Ok(())
}

#[test]
fn two_blocks_declaring_two_texture_keys_spend_two_layers() -> TestResult {
    let directory = TempDir::new()?;
    let keys = [SHARED_KEY, A_SECOND_KEY];
    let root = root_declaring(&directory, keys)?;

    let read = reading(&root);

    assert_eq!(
        read,
        Reading::Spent(distinct(keys)),
        "the control for the reading above. Without it, a loader that reported one layer for any \
         root at all — or that folded two distinct keys together and drew both blocks from one \
         texture — would satisfy the sharing scenario and be caught nowhere"
    );
    Ok(())
}
