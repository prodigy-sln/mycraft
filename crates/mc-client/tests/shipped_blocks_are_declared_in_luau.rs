//! What the base game ships, read through the door a player's launch goes
//! through.
//!
//! The four blocks, their order, what stops a player, what is drawn, what hides
//! what is behind it, what a swing can find, what may be broken, the one that
//! may be built through, and the one a client puts in a player's hand.
//!
//! **The six facts about a block are read as one line each, and the four lines
//! are compared as one ordered list.** Six separate assertions cannot see a
//! field that stopped being read at all, and a comparison that *filtered* the
//! observed blocks by the ones it expected could not see a fifth block arrive —
//! this project has twice shipped a hand-maintained list that stayed green while
//! the thing it mirrored grew, both times because the comparison filtered. So a
//! missing block, an extra block, a reordering and a changed field are four
//! distinct failures here.
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
use mc_client::notice::Notices;
use mc_core::block::{BlockDefinition, BlockId, BlockRegistry, DefinitionOrigin};
use mc_world::persistence::Acceptance;
use support::{TestResult, content, content_root};

/// What the base game declares, in the order file names put it in, as a reader
/// of `content/base/blocks/` would write it down.
///
/// Written out rather than read off the shipped root, because a fixture derived
/// from the thing under test agrees with it whatever it becomes.
///
/// **Water is the whole of the difference between the four, and it is no longer
/// one difference but five.** It alone is soft, it alone may be built through,
/// it alone cannot be broken, and it alone is drawn without hiding what stands
/// behind it. The other three say nothing at all about being drawn, hiding or
/// being aimed at, so each of those reads back as whatever that declaration says
/// about `solid` — which for those three is `true` three times over. Writing the
/// derived values out here rather than deriving them is the point: a reader that
/// came to answer every one of the three from `solid` would agree with a table
/// that derived them the same way, and disagree with this one over water.
const SHIPPED: [Declared; 4] = [
    Declared {
        name: "base:dirt",
        file: "dirt.luau",
        solid: true,
        replaceable: false,
        breakable: true,
        drawn: true,
        occludes: true,
        targetable: true,
    },
    Declared {
        name: "base:grass",
        file: "grass.luau",
        solid: true,
        replaceable: false,
        breakable: true,
        drawn: true,
        occludes: true,
        targetable: true,
    },
    Declared {
        name: "base:stone",
        file: "stone.luau",
        solid: true,
        replaceable: false,
        breakable: true,
        drawn: true,
        occludes: true,
        targetable: true,
    },
    Declared {
        name: "base:water",
        file: "water.luau",
        solid: false,
        replaceable: true,
        breakable: false,
        drawn: true,
        occludes: false,
        targetable: true,
    },
];

/// The block a client is expected to put in the player's hand, and the file
/// that has to have declared it: the first solid block in registration order.
const IN_HAND: Declared = SHIPPED[0];

/// The block file every solid declaration is stripped down to, for the root
/// that registers nothing anybody could place.
const THE_ONLY_NON_SOLID_ONE: &str = "water";

/// One line of the expected reading: a block, the file that declared it, and
/// the six facts a declaration states or leaves to its `solid`.
#[derive(Debug, Clone, Copy)]
struct Declared {
    name: &'static str,
    file: &'static str,
    solid: bool,
    replaceable: bool,
    breakable: bool,
    drawn: bool,
    occludes: bool,
    targetable: bool,
}

/// One line of a reading, owned, so that the expectation and the observation are
/// the same shape and are compared field by field.
///
/// **A struct rather than a formatted line.** Both sides used to be rendered to
/// one string, which compares the whole of a block's reading as this does but
/// reports a disagreement as two sentences a reader has to diff by eye. Named
/// fields put the changed one in the failure output.
#[derive(Debug, PartialEq, Eq)]
struct Reading {
    name: String,
    file: String,
    solid: bool,
    replaceable: bool,
    breakable: bool,
    drawn: bool,
    occludes: bool,
    targetable: bool,
}

/// What a launch came to, so that a launch which refused and a launch which
/// held the wrong block are the same failed comparison rather than one failure
/// and one propagated error.
#[derive(Debug, PartialEq, Eq)]
enum Launched {
    /// The blocks it registered, in registration order, each with the file that
    /// declared it.
    Registered(Vec<Reading>),
    /// A block it registered that could not be read back at all.
    Unreadable(String),
    /// It held this block, declared by this file.
    Holding(String),
    /// It refused because nothing the content declares could be placed.
    RefusedForHavingNothingToPlace,
    /// It refused for some other reason, rendered as it renders itself.
    RefusedOtherwise(String),
}

#[test]
fn the_shipped_content_declares_four_blocks_with_water_alone_soft_unbreakable_and_seen_through()
-> TestResult {
    let save = tempfile::tempdir()?;

    let launched = launched_over(&content_root()?, &save.path().join("world.mcw"))?;

    assert_eq!(
        launched,
        Launched::Registered(SHIPPED.iter().map(reading_of).collect()),
        "these four blocks, in this order, out of these four files, are the world the base game \
         ships. Three of them state nothing about being drawn, hiding what is behind them or \
         being aimed at, and so read back as the `solid` each of them states; water states all \
         three and is the one block where they come apart — soft, unbreakable, drawn, and hiding \
         nothing behind it. A reading that names the right blocks out of the wrong files is the \
         declaration not having moved; a reading that answers water's three from its solidity is \
         the split not having happened"
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
        match prepare_launch(
            root,
            save,
            Acceptance::OnlyUnchangedBlocks,
            &Notices::discarding(),
        ) {
            Ok(prepared) => registered_in(&prepared.registry),
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
    let prepared = match prepare_launch(
        root,
        save,
        Acceptance::OnlyUnchangedBlocks,
        &Notices::discarding(),
    ) {
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
///
/// A block the registry will not read back stops the reading rather than
/// becoming one more line of it: a definition that could not be fetched says
/// nothing about what any declaration states, and rendering the failure as a
/// line of the list would put it where a reader looks for a block.
fn registered_in(registry: &BlockRegistry) -> Launched {
    let mut readings = Vec::new();
    for id in 0..registry.registered_count() {
        match registry.definition(BlockId::from_raw(id as u32)) {
            Ok(definition) => readings.push(described(definition)),
            Err(failure) => return Launched::Unreadable(failure.to_string()),
        }
    }
    Launched::Registered(readings)
}

/// One registered block, as this suite reads it.
fn described(definition: &BlockDefinition) -> Reading {
    Reading {
        name: definition.name.as_str().to_owned(),
        file: declaring_file(&definition.origin),
        solid: definition.is_solid,
        replaceable: definition.replaceable,
        breakable: definition.breakable,
        drawn: definition.drawn,
        occludes: definition.occludes,
        targetable: definition.targetable,
    }
}

/// One expected block, written the same way.
fn reading_of(declared: &Declared) -> Reading {
    Reading {
        name: declared.name.to_owned(),
        file: declared.file.to_owned(),
        solid: declared.solid,
        replaceable: declared.replaceable,
        breakable: declared.breakable,
        drawn: declared.drawn,
        occludes: declared.occludes,
        targetable: declared.targetable,
    }
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
