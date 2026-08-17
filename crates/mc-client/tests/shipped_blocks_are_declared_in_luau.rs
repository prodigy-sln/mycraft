//! What the base game ships, read through the door a player's launch goes
//! through.
//!
//! The four blocks, their order, their solidity, the one that may be built
//! through, and the one a client puts in a player's hand. None of that is new —
//! it is what the game already did — so a test asserting only those facts would
//! have been green before a line of this feature was written, and green
//! afterwards, and would have said nothing about either.
//!
//! **So every reading here also says which file declared the block**, and that
//! is what makes these tests able to fail today. A definition carries the origin
//! it was declared by, the registry hands it back, and a block declared by
//! `dirt.toml` is not the block this feature is about. The scenario's own
//! assertion is kept exactly as it is worded and the declaring file is asserted
//! beside it, in the same comparison, so a swap that registered the same four
//! blocks out of the old reader is reported rather than accepted.
//!
//! **A file name and never a whole path.** An origin renders with the
//! platform's own separators, so an expectation spelling one out would be a
//! test that passes on one operating system and fails on the other for a reason
//! that has nothing to do with content.

mod support;

use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;

use mc_client::launch::prepare_launch;
use mc_core::block::{BlockDefinition, BlockId, BlockRegistry, DefinitionOrigin};
use mc_world::persistence::Acceptance;
use support::{TestResult, content, content_root};

/// What the base game declares, in the order file names put it in, as a reader
/// of `content/base/blocks/` would write it down.
///
/// Written out rather than read off the shipped root, because a fixture derived
/// from the thing under test agrees with it whatever it becomes. Only `water` is
/// non-solid and only `water` may be built through, which is the whole of the
/// difference between the four.
const SHIPPED: [Declared; 4] = [
    Declared {
        name: "base:dirt",
        file: "dirt.luau",
        solid: true,
        replaceable: false,
    },
    Declared {
        name: "base:grass",
        file: "grass.luau",
        solid: true,
        replaceable: false,
    },
    Declared {
        name: "base:stone",
        file: "stone.luau",
        solid: true,
        replaceable: false,
    },
    Declared {
        name: "base:water",
        file: "water.luau",
        solid: false,
        replaceable: true,
    },
];

/// The block a client is expected to put in the player's hand, and the file
/// that has to have declared it: the first solid block in registration order.
const IN_HAND: Declared = SHIPPED[0];

/// The block file every solid declaration is stripped down to, for the root
/// that registers nothing anybody could place.
const THE_ONLY_NON_SOLID_ONE: &str = "water";

/// One line of the expected reading: a block, the file that declared it, and
/// the two facts about it that differ across the shipped four.
#[derive(Debug, Clone, Copy)]
struct Declared {
    name: &'static str,
    file: &'static str,
    solid: bool,
    replaceable: bool,
}

/// What a launch came to, so that a launch which refused and a launch which
/// held the wrong block are the same failed comparison rather than one failure
/// and one propagated error.
#[derive(Debug, PartialEq, Eq)]
enum Launched {
    /// The blocks it registered, in registration order, each with the file that
    /// declared it.
    Registered(Vec<String>),
    /// It held this block, declared by this file.
    Holding(String),
    /// It refused because nothing the content declares could be placed.
    RefusedForHavingNothingToPlace,
    /// It refused for some other reason, rendered as it renders itself.
    RefusedOtherwise(String),
}

#[test]
fn the_shipped_content_declares_four_blocks_in_luau_with_water_alone_soft_and_replaceable()
-> TestResult {
    let save = tempfile::tempdir()?;

    let launched = launched_over(&content_root()?, &save.path().join("world.mcw"))?;

    assert_eq!(
        launched,
        Launched::Registered(SHIPPED.iter().map(reading_of).collect()),
        "these four blocks, in this order, with water alone soft and water alone replaceable, are \
         the world the base game has always made — and the point of this feature is that they are \
         now declared in the language a mod author writes. A reading that names the right blocks \
         out of the wrong files is the swap not having happened"
    );
    Ok(())
}

#[test]
fn a_launch_puts_the_first_solid_block_the_declarations_name_into_the_players_hand() -> TestResult {
    let save = tempfile::tempdir()?;

    let launched = held_over(&content_root()?, &save.path().join("world.mcw"))?;

    assert_eq!(
        launched,
        Launched::Holding(format!("{} declared by {}", IN_HAND.name, IN_HAND.file)),
        "the block a client holds is the first solid one in registration order, and registration \
         order is the order file names sort in — which is the rule the modding pages teach a mod \
         author and the only thing that decides what they start the game holding"
    );
    Ok(())
}

#[test]
fn a_content_root_declaring_nothing_solid_refuses_to_start_rather_than_opening_a_window()
-> TestResult {
    let root =
        content::shipped_copy()?.declaring_only_the_block_file_named(THE_ONLY_NON_SOLID_ONE)?;
    let save = tempfile::tempdir()?;

    let launched = launched_over(root.path(), &save.path().join("world.mcw"))?;

    assert_eq!(
        launched,
        Launched::RefusedForHavingNothingToPlace,
        "a window that opens over content nothing can be placed from is a window a player can do \
         nothing in, and it says so nowhere. The refusal names the rule instead, so whoever wrote \
         the content learns that the block a client holds is the first solid one"
    );
    Ok(())
}

/// What preparing a launch from `root` registered, or why it would not.
///
/// # Errors
///
/// Returns an error if the temporary save path cannot be used.
fn launched_over(root: &Path, save: &Path) -> Result<Launched, Box<dyn Error>> {
    Ok(
        match prepare_launch(root, save, Acceptance::OnlyUnchangedBlocks) {
            Ok(prepared) => Launched::Registered(registered_in(&prepared.registry)),
            Err(refused) => refusal(&refused),
        },
    )
}

/// What preparing a launch from `root` put in the player's hand, or why it
/// would not.
///
/// # Errors
///
/// Returns an error if the held block is not one the registry can resolve —
/// which would mean a launch handing a player a name nothing declared.
fn held_over(root: &Path, save: &Path) -> Result<Launched, Box<dyn Error>> {
    let prepared = match prepare_launch(root, save, Acceptance::OnlyUnchangedBlocks) {
        Ok(prepared) => prepared,
        Err(refused) => return Ok(refusal(&refused)),
    };
    let held = prepared.registry.resolve(&prepared.holding)?;
    Ok(Launched::Holding(format!(
        "{} declared by {}",
        held.name.as_str(),
        declaring_file(&held.origin)
    )))
}

/// Why a preparation refused, told apart from every other reason it could have.
fn refusal(refused: &mc_client::startup::PreparationError) -> Launched {
    match refused {
        mc_client::startup::PreparationError::NothingToPlace => {
            Launched::RefusedForHavingNothingToPlace
        }
        other => Launched::RefusedOtherwise(other.to_string()),
    }
}

/// Every block `registry` holds, in the order it registered them.
fn registered_in(registry: &BlockRegistry) -> Vec<String> {
    (0..registry.registered_count())
        .map(|id| {
            registry
                .definition(BlockId::from_raw(id as u32))
                .map_or_else(|failure| failure.to_string(), described)
        })
        .collect()
}

/// One registered block, as this suite reads it.
fn described(definition: &BlockDefinition) -> String {
    reading(
        definition.name.as_str(),
        &declaring_file(&definition.origin),
        definition.is_solid,
        definition.replaceable,
    )
}

/// One expected block, written the same way.
fn reading_of(declared: &Declared) -> String {
    reading(
        declared.name,
        declared.file,
        declared.solid,
        declared.replaceable,
    )
}

/// The one spelling both sides of the comparison are written in.
fn reading(name: &str, file: &str, solid: bool, replaceable: bool) -> String {
    format!("{name} declared by {file}, solid {solid}, replaceable {replaceable}")
}

/// The file an origin points at, by its name alone.
///
/// The whole origin is a path and renders with the platform's separators; its
/// last component is the same word everywhere, which is what a portable
/// expectation can be written against.
fn declaring_file(origin: &DefinitionOrigin) -> String {
    Path::new(origin.as_str())
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(origin.as_str())
        .to_owned()
}
