//! What a save records as a block's appearance, now that a block has six
//! facings.
//!
//! A save records two folds per block — what it is to stand on, and what it looks
//! like — and the split is what stops a retexture and a rebalance being the same
//! event. This file is about the second of the two once "what it looks like" is
//! six keys rather than one: a block whose `north` alone changed looks different
//! and must record differently, and the value it records is a fold over a byte
//! sequence stated in the design rather than a number anybody read off a run.
//!
//! # The expected value is built here, byte by byte, and never copied
//!
//! The appearance is the format's own field list — a revision byte, the block's
//! name, its six keys in the order `up`, `down`, `north`, `south`, `east`,
//! `west`, and then whether the block is drawn and whether it occludes — each
//! variable-length field carrying its own length in front of it, so that
//! `("ab", "c")` and `("a", "bc")` cannot fold identically. The fold is
//! FNV-1a-64 from the published constants, written out again here. **No number in
//! this file came from a run of the code under test, and nothing here calls the
//! fold it is judging**: a test calling the function under test is agreement
//! between two copies of one decision.
//!
//! # The two flags are appended after the keys, and the fixture is what can see
//! their order
//!
//! `drawn` and `occludes` are appearance because a block that stopped being drawn
//! is still the same block to stand on. They are *appended* because the canonical
//! encoding writes a struct positionally, so a field placed among the existing
//! ones moves every byte after it.
//!
//! **The fixture below states the two differently, and that is the whole of what
//! makes their order visible here.** A block stating them alike folds the same
//! two bytes whichever way round they are written, so a fold that emitted
//! `occludes` first, or emitted one of them twice, or answered from a constant
//! rather than from the declaration, agrees with such a fixture exactly. Drawn
//! and see-through is a real combination and not a contrivance built to break a
//! tie: it is what a pane of glass is, and what `base:water` is.
//!
//! The other witness for that order is `base:water` itself, in
//! `src/persistence/format_test.rs` — the one *shipped* block whose two flags
//! disagree. Two witnesses and not one, because that one is a fact about the base
//! game's current opinion rather than about the format: the day anybody declares
//! water's `drawn` and `occludes` alike, or stops shipping water, it goes silent
//! without failing. This fixture cannot, because nothing outside this file
//! decides what it says.
//!
//! Sixty-four bits over sixty-odd bytes is not something a person derives on
//! paper, so what is derived is the **relation** — a recorded hash equals what the
//! stated bytes fold to — not the digits.
//!
//! # Why six distinct keys, and why they are not in alphabetical order
//!
//! An appearance folded over six keys in the wrong order still folds six keys, and
//! six keys that happen to be sorted make the declared order and the sorted order
//! the same order. The fixture below states six pairwise-distinct keys whose
//! `Face::ALL` order is **not** their alphabetical order, so a fold that sorted
//! them, or that walked them by any other route, disagrees with this oracle.
//!
//! # What is deliberately not here
//!
//! Anything about an older save. A save written before this format is
//! `shipped_declarations_and_an_older_save.rs`, which owns the committed pre-spec
//! fixture — the only thing in this repository that is genuinely a save from
//! before, and the only honest witness for what happens when one is loaded.

mod common;

use std::error::Error;

use common::persistence::{declaration_of, saved_requirements, world_at, world_holding};
use common::{FIXTURE_ORIGIN, TestResult};
use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use tempfile::TempDir;

/// The block every fixture here declares.
const PINNED: &str = "fixture:andesite";

/// The one cell the fixture world holds it at.
const A_CELL: mc_world::world::WorldPos = world_at(1, 1, 1);

/// Six keys, one per facing, in the order the six words are written.
///
/// Pairwise distinct, none of them the block's own name, and **deliberately not
/// in alphabetical order**: a fold that sorted the keys before folding them agrees
/// with any fixture whose declared order is already sorted.
const SIX_KEYS: [&str; 6] = [
    "fixture:quartz",
    "fixture:ash",
    "fixture:gabbro",
    "fixture:basalt",
    "fixture:diorite",
    "fixture:chert",
];

/// The facing whose key one of the two declarations changes, and the key it
/// changes to.
const CHANGED_FACING: usize = 2;
const A_DIFFERENT_NORTH: &str = "fixture:andesite_reworked";

/// Which revision of the appearance field list this file states.
///
/// Written out here rather than read from the format, so that a revision bumped
/// without the fields changing is a disagreement rather than a change both sides
/// make together. It is the appearance list's own revision and not the behaviour
/// list's, and the two are different numbers: a single number shared between the
/// two lists would report every block in every save as behaving differently the
/// moment a texture key or a rendering flag joined this one.
const STATED_APPEARANCE_REVISION: u8 = 3;

/// What the fixture block declares about being seen, stated once and read by both
/// the declaration and the oracle.
///
/// **The two are deliberately unequal**, which is what lets the byte sequence
/// below see them transposed, see one of them folded twice, and see a fold that
/// answers `true` from a constant instead of from the declaration. See the module
/// header for why that job is this fixture's rather than a shipped block's.
///
/// One constant per flag rather than a literal in each place: two literals that
/// drifted apart would leave the oracle folding a block the fixture never
/// declared, and the comparison would fail for a reason that is about neither.
const DRAWN: bool = true;
const OCCLUDES: bool = false;

/// How the canonical encoding writes a `bool`.
const FALSE_BYTE: u8 = 0x00;
const TRUE_BYTE: u8 = 0x01;

/// Where an FNV-1a 64 fold starts, and what it multiplies by.
///
/// The published constants, stated a second time on purpose. Reading them from the
/// crate under test would make a changed constant invisible here.
const STATED_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const STATED_PRIME: u64 = 0x0000_0100_0000_01b3;

/// How the canonical encoding writes the length that prefixes every text: seven
/// bits to a byte, low group first, the top bit set on every byte but the last.
const LENGTH_PAYLOAD_BITS: u32 = 7;
const LENGTH_CONTINUES: u8 = 0b1000_0000;
const LENGTH_PAYLOAD: usize = 0b0111_1111;

/// What a save recorded for one block: its behaviour fold and its appearance
/// fold.
///
/// A record compared whole rather than two readings, so a save that moved both
/// when it should have moved one is not mistaken for a save that moved the right
/// one.
#[derive(Debug, PartialEq, Eq)]
enum Recorded {
    /// The save names the block and recorded these two folds.
    Folds { behaviour: u64, appearance: u64 },
    /// The save does not name the block at all.
    NotNamed,
}

/// A registry holding [`PINNED`] alone, its six facings holding `keys`.
///
/// # Errors
///
/// Returns an error if a name or a key is not a namespaced id, or if the registry
/// refuses the batch.
fn registry_texturing(keys: [&str; 6]) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut parsed = Vec::with_capacity(keys.len());
    for key in keys {
        parsed.push(TextureKey::parse(key)?);
    }
    let stated: [TextureKey; 6] = parsed.try_into().map_err(|wrong: Vec<TextureKey>| {
        format!(
            "six facings need six keys, and this fixture assembled {count}",
            count = wrong.len()
        )
    })?;
    let declared = vec![Ok(BlockDefinition {
        name: BlockName::parse(PINNED)?,
        textures: FaceTextures::stating(stated),
        is_solid: true,
        replaceable: false,
        breakable: true,
        breaks_into: None,
        drawn: DRAWN,
        occludes: OCCLUDES,
        targetable: true,
        swimmable: false,
        move_resistance: 0.0,
        origin: DefinitionOrigin::new(FIXTURE_ORIGIN),
    })];
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        DefinitionOrigin::new(FIXTURE_ORIGIN),
        declared,
    ))?;
    Ok(registry)
}

/// What a save of a world holding [`PINNED`] records about it, against a registry
/// texturing it with `keys`.
///
/// # Errors
///
/// Returns an error if the registry is refused, if the world cannot be built, or
/// if the save cannot be written or read back.
fn recorded(keys: [&str; 6]) -> Result<Recorded, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let registry = registry_texturing(keys)?;
    let world = world_holding(&[(A_CELL, PINNED)], &registry)?;
    let required = saved_requirements(&directory, &world, &registry)?;
    Ok(match declaration_of(&required, PINNED) {
        Some((behaviour, appearance)) => Recorded::Folds {
            behaviour,
            appearance,
        },
        None => Recorded::NotNamed,
    })
}

/// [`SIX_KEYS`] with the `north` facing holding `key` instead of its own.
fn six_keys_with_north(key: &'static str) -> [&'static str; 6] {
    let mut keys = SIX_KEYS;
    keys[CHANGED_FACING] = key;
    keys
}

/// The bytes the format states a block's declared appearance is: the revision,
/// the name, the six keys in the order the six words are written, and then the
/// two flags that say whether the block is drawn and whether it hides what is
/// behind it.
fn stated_appearance_bytes(name: &str, keys: [&str; 6]) -> Vec<u8> {
    let mut stated = vec![STATED_APPEARANCE_REVISION];
    push_text(&mut stated, name);
    for key in keys {
        push_text(&mut stated, key);
    }
    push_flag(&mut stated, DRAWN);
    push_flag(&mut stated, OCCLUDES);
    stated
}

/// `flag` as the canonical encoding writes it.
fn push_flag(stated: &mut Vec<u8>, flag: bool) {
    stated.push(if flag { TRUE_BYTE } else { FALSE_BYTE });
}

/// `text` as the canonical encoding writes it: its length, then its bytes.
fn push_text(stated: &mut Vec<u8>, text: &str) {
    push_length(stated, text.len());
    stated.extend_from_slice(text.as_bytes());
}

/// `length` as the canonical encoding writes it.
fn push_length(stated: &mut Vec<u8>, length: usize) {
    let mut remaining = length;
    while remaining > LENGTH_PAYLOAD {
        stated.push(low_bits_of(remaining) | LENGTH_CONTINUES);
        remaining >>= LENGTH_PAYLOAD_BITS;
    }
    stated.push(low_bits_of(remaining));
}

/// The low seven bits of `value`, which are a byte by construction.
fn low_bits_of(value: usize) -> u8 {
    (value & LENGTH_PAYLOAD) as u8
}

/// `stated` folded with this file's own FNV-1a-64.
///
/// The second implementation. It shares no line with the one under test, which is
/// the only reason its agreement is evidence of anything.
fn folded_here(stated: &[u8]) -> u64 {
    stated.iter().fold(STATED_OFFSET_BASIS, |folded, byte| {
        (folded ^ u64::from(*byte)).wrapping_mul(STATED_PRIME)
    })
}

#[test]
fn two_blocks_differing_only_in_their_north_key_record_different_appearances() -> TestResult {
    let one = recorded(SIX_KEYS)?;
    let other = recorded(six_keys_with_north(A_DIFFERENT_NORTH))?;

    assert_eq!(
        (
            behaviour_of(&one) == behaviour_of(&other),
            appearance_of(&one) == appearance_of(&other),
            matches!(one, Recorded::Folds { .. })
        ),
        (true, false, true),
        "the two declarations differ in one facing's key and in nothing else, so the recorded \
         appearance has to move and the recorded behaviour has to stand still. A fold that read \
         only `up` — or only the key a block used to have — would leave a player's world reporting \
         a block as unchanged after its sides were redrawn, which is the whole of what recording an \
         appearance is for"
    );
    Ok(())
}

#[test]
fn an_unchanged_declaration_records_the_appearance_the_stated_byte_sequence_folds_to() -> TestResult
{
    let stated = folded_here(&stated_appearance_bytes(PINNED, SIX_KEYS));

    let recorded = recorded(SIX_KEYS)?;

    assert_eq!(
        appearance_of(&recorded),
        Some(stated),
        "what a save records as a block's appearance is the format's stated field list folded \
         with FNV-1a-64, and this is that same fold arrived at without calling it. The standard \
         library's hasher is unspecified and moves with the toolchain, so a save written with it \
         would report every block as changed after an unrelated compiler upgrade — a report a \
         player cannot act on and learns to ignore. **The fixture is drawn and see-through, so \
         this comparison also grades the order of the two flags**: a fold emitting `occludes` \
         first agrees with every block that states them alike and disagrees here"
    );
    Ok(())
}

/// The behaviour fold in `recorded`, or nothing where the save does not name the
/// block.
fn behaviour_of(recorded: &Recorded) -> Option<u64> {
    match recorded {
        Recorded::Folds { behaviour, .. } => Some(*behaviour),
        Recorded::NotNamed => None,
    }
}

/// The appearance fold in `recorded`, or nothing where the save does not name the
/// block.
fn appearance_of(recorded: &Recorded) -> Option<u64> {
    match recorded {
        Recorded::Folds { appearance, .. } => Some(*appearance),
        Recorded::NotNamed => None,
    }
}
