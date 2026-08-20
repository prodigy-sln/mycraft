//! Guard. Both hashes a save records about every shipped block, against an
//! FNV-1a-64 this file computes for itself.
//!
//! # Why this file was written before the fold moved crates
//!
//! [`fnv_1a_64`](super::fnv_1a_64) has since left this module for
//! `mc_core::hash`, so that a texture baker and a client can share one value
//! forever. The move was meant to change no stored hash at all — and a guard
//! written *after* it could only have agreed with whatever the move produced.
//!
//! So these two were authored deliberately against the tree as it stood
//! **before** the move, run green there, and are green still. That is the claim
//! they buy: not that these values are right, which a guard written afterwards
//! would say just as loudly, but that they did not move.
//!
//! They are also indifferent to *where* the fold lives, and that was measured
//! rather than assumed: a private copy left behind in this module and called
//! from here reddened nothing in the workspace at all. Deliberate — the move is
//! held by the compiler and by a reader, never by a test, and these guards hold
//! the **value** instead.
//!
//! # Why both halves, and what happened to the appearance one
//!
//! A block records two hashes and the split is deliberate: what it is to stand
//! on, and what it looks like. The phase that gave an appearance six texture
//! keys instead of one has since landed, so the appearance half below states six
//! keys under a revision of its own — **revised here by that phase's test author
//! and by nobody else**, which is what the note this paragraph replaces asked
//! for.
//!
//! What was bought by writing both halves before the fold moved crates survives
//! the revision, and it is the behaviour half: it is still the fold it was, over
//! the fields it was, under revision 1, and it has been green across two changes
//! it had every opportunity to move under.
//!
//! # The revision byte is per field list, and that is load-bearing
//!
//! One number shared between the two lists would move every **behaviour** fold in
//! existence the moment the appearance list gained a field — every save in the
//! world reporting every block as behaving differently, over a texture key. So
//! the two are stated separately here as they are stated separately in the
//! format, and the behaviour constant staying at 1 while the appearance one is 2
//! is the whole of what says the bump reached only the list that grew.
//!
//! # The oracle, and why the same arithmetic is written twice
//!
//! Each guard computes its expected value from nothing: the byte sequence the
//! format's own field list states, built here field by field, folded here by a
//! second FNV-1a-64 written out from the published constants.
//!
//! **No number in this file was taken from a run of the code under test, and
//! nothing here calls the fold it is judging.** A test calling the function
//! under test is agreement between two copies of one decision, which is the
//! failure this project has met more often than any other. Two independent
//! copies of the arithmetic is not duplication here, it is the instrument: the
//! one under test cannot move without this one disagreeing.
//!
//! # What makes them non-vacuous, given neither could be red first
//!
//! They are green from the moment they are written, so falsifiability has to
//! come from mutation instead. Two of this phase's are the evidence, and both
//! redden both guards: changing the offset basis by one, and returning the basis
//! without folding at all. The outcomes are recorded in the phase's own record.
//!
//! # What they do not assert
//!
//! Not the concrete values. Sixty-four bits folded over forty-odd bytes is not
//! something a person derives by hand, so what is derived here is the
//! **relation** — a recorded hash equals what the stated bytes fold to. Nor
//! anything about a save file: no file is written and no encoder is called on
//! this path.
//!
//! The shipped set is **named** below rather than counted off the registry,
//! because a registry that loaded nothing at all would satisfy a claim about
//! "every shipped block" by having none.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use mc_core::block::{BlockDefinition, BlockId, BlockRegistry};
use mc_core::content::Face;
use mc_core::id::BlockName;

use crate::content::LuauFileDefinitionSource;

use super::{appearance_of, behaviour_of};

/// The error type these guards propagate with `?`.
type GuardResult = Result<(), Box<dyn Error>>;

/// The blocks this repository ships, stated rather than discovered.
///
/// An empty registry answers "every shipped block" vacuously, and so does one
/// that lost a file. Every name here must resolve, and every block the registry
/// holds must appear here, because the two maps below are compared whole.
const SHIPPED_BLOCKS: [&str; 4] = ["base:dirt", "base:grass", "base:stone", "base:water"];

/// How many directories above this crate's manifest the repository root sits.
const CRATE_DEPTH: usize = 2;

/// Which revision of each of the format's two field lists these guards state.
///
/// Written out here rather than read from the module under test, so that a
/// revision bumped without the fields changing is a disagreement rather than a
/// change both sides make together.
///
/// **Two numbers and not one.** The appearance list gained five fields and says
/// so in its own byte; the behaviour list gained nothing and its byte has not
/// moved. A single shared constant would have moved both, and the behaviour half
/// of this file is the instrument that would have reported it.
const STATED_BEHAVIOUR_REVISION: u8 = 1;
const STATED_APPEARANCE_REVISION: u8 = 2;

/// Where an FNV-1a 64 fold starts, and what it multiplies by.
///
/// The published constants, stated a second time on purpose. Reading them from
/// the module under test would make a changed constant invisible here, which is
/// exactly the mutation these guards are answerable for.
const STATED_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const STATED_PRIME: u64 = 0x0000_0100_0000_01b3;

/// How the canonical encoding writes a `bool`, and how it writes whether an
/// optional field is present.
const FALSE_BYTE: u8 = 0x00;
const TRUE_BYTE: u8 = 0x01;
const ABSENT_BYTE: u8 = 0x00;
const PRESENT_BYTE: u8 = 0x01;

/// How the canonical encoding writes the length that prefixes every text: seven
/// bits to a byte, low group first, the top bit set on every byte but the last.
const LENGTH_PAYLOAD_BITS: u32 = 7;
const LENGTH_CONTINUES: u8 = 0b1000_0000;
const LENGTH_PAYLOAD: usize = 0b0111_1111;

#[test]
fn every_shipped_blocks_recorded_behaviour_is_the_fold_an_independent_oracle_computes()
-> GuardResult {
    let registry = shipped_registry()?;

    let recorded = recorded_over(&registry, |definition| behaviour_of(definition).get())?;
    let stated = stated_over(&registry, stated_behaviour_bytes)?;

    assert_eq!(
        recorded, stated,
        "what a save records as a block's declared behaviour is the format's stated field list \
         folded with FNV-1a-64, and this is that same fold arrived at without calling it. The \
         pair being equal is what says the value did not move when the fold changed crates — a \
         claim only a guard that existed before the move can make"
    );
    Ok(())
}

#[test]
fn every_shipped_blocks_recorded_appearance_is_the_fold_an_independent_oracle_computes()
-> GuardResult {
    let registry = shipped_registry()?;

    let recorded = recorded_over(&registry, |definition| appearance_of(definition).get())?;
    let stated = stated_over(&registry, stated_appearance_bytes)?;

    assert_eq!(
        recorded, stated,
        "an appearance is six texture keys under a revision of its own, and this states them all \
         — the revision, the name, then one key per facing in the order the faces are declared in. \
         Every shipped block states one string, so all six are equal here; what this half is \
         answerable for is the shape of the record and the revision it carries, and the guard for \
         the *order* is over six distinct keys in `tests/save_per_face_appearance.rs`"
    );
    Ok(())
}

/// A registry holding exactly what this repository ships as content.
///
/// The declarations are read off the disk they ship on, rather than restated
/// here, because "every shipped block" is otherwise a claim about a copy: a
/// hand-written definition drifting from `content/base/blocks/` would leave both
/// guards green about blocks nobody ships.
fn shipped_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(content_root()?))?;
    Ok(registry)
}

/// Where the shipped content lives, located from this crate's manifest and never
/// from the directory a test binary happens to start in.
fn content_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(CRATE_DEPTH)
        .ok_or("the crate manifest directory has no repository root above it")?
        .join("content")
        .join("base"))
}

/// What the format records under `record` for every block the registry holds,
/// keyed by name.
///
/// Enumerated off the registry rather than off [`SHIPPED_BLOCKS`], so that a
/// block the repository grew and this file does not name makes the comparison
/// below fail instead of passing unnoticed.
fn recorded_over(
    registry: &BlockRegistry,
    record: fn(&BlockDefinition) -> u64,
) -> Result<BTreeMap<String, u64>, Box<dyn Error>> {
    let mut recorded = BTreeMap::new();
    for position in 0..registry.registered_count() {
        let definition = registry.definition(BlockId::from_raw(u32::try_from(position)?))?;
        recorded.insert(definition.name.as_str().to_owned(), record(definition));
    }
    Ok(recorded)
}

/// What the stated bytes fold to for every block this repository is known to
/// ship, keyed by name.
///
/// Resolved by name, so a shipped block that stopped loading is a failure to
/// resolve rather than one fewer entry nobody misses.
fn stated_over(
    registry: &BlockRegistry,
    state: fn(&BlockDefinition) -> Vec<u8>,
) -> Result<BTreeMap<String, u64>, Box<dyn Error>> {
    let mut stated = BTreeMap::new();
    for name in SHIPPED_BLOCKS {
        let definition = registry.resolve(&BlockName::parse(name)?)?;
        stated.insert(name.to_owned(), folded_here(&state(definition)));
    }
    Ok(stated)
}

/// The bytes the format states a block's declared behaviour is.
///
/// The field list written out in its declared order: the input version, the
/// name, the three flags, then the residue as an optional name. The origin is
/// deliberately not among them — it is derived from a file path, and folding it
/// would make a save refuse to load from a second checkout.
fn stated_behaviour_bytes(definition: &BlockDefinition) -> Vec<u8> {
    let mut stated = vec![STATED_BEHAVIOUR_REVISION];
    push_text(&mut stated, definition.name.as_str());
    push_flag(&mut stated, definition.is_solid);
    push_flag(&mut stated, definition.replaceable);
    push_flag(&mut stated, definition.breakable);
    match definition.breaks_into.as_ref() {
        None => stated.push(ABSENT_BYTE),
        Some(residue) => {
            stated.push(PRESENT_BYTE);
            push_text(&mut stated, residue.as_str());
        }
    }
    stated
}

/// The bytes the format states a block's declared appearance is.
///
/// The name is in this list as well as in the behaviour one, which is what stops
/// a block's two hashes being swapped for each other and stops one block's
/// appearance colliding with another's behaviour.
///
/// **Six keys, in `Face::ALL` order**, and the order is stated by walking that
/// array rather than by naming the six words here: a second authored copy of a
/// declaration order is a row nobody checked. Every block this repository ships
/// states one string, so all six of these keys are equal today — which is exactly
/// why the *order* has a witness of its own, over six distinct keys, in
/// `tests/save_per_face_appearance.rs`.
fn stated_appearance_bytes(definition: &BlockDefinition) -> Vec<u8> {
    let mut stated = vec![STATED_APPEARANCE_REVISION];
    push_text(&mut stated, definition.name.as_str());
    for face in Face::ALL {
        push_text(&mut stated, definition.textures.at(face).as_str());
    }
    stated
}

/// `text` as the canonical encoding writes it: its length, then its bytes.
///
/// The length prefix is what makes `("ab", "c")` and `("a", "bc")` fold to
/// different values, so it is stated here rather than skipped as noise.
fn push_text(stated: &mut Vec<u8>, text: &str) {
    push_length(stated, text.len());
    stated.extend_from_slice(text.as_bytes());
}

/// `length` as the canonical encoding writes it.
///
/// Written for every length rather than only the one byte every name in this
/// repository needs, so that a name grown past 127 bytes is folded correctly
/// here instead of quietly disagreeing.
fn push_length(stated: &mut Vec<u8>, length: usize) {
    let mut remaining = length;
    while remaining > LENGTH_PAYLOAD {
        stated.push(low_bits_of(remaining) | LENGTH_CONTINUES);
        remaining >>= LENGTH_PAYLOAD_BITS;
    }
    stated.push(low_bits_of(remaining));
}

/// `flag` as the canonical encoding writes it.
fn push_flag(stated: &mut Vec<u8>, flag: bool) {
    stated.push(if flag { TRUE_BYTE } else { FALSE_BYTE });
}

/// The low seven bits of `value`, which are a byte by construction.
fn low_bits_of(value: usize) -> u8 {
    (value & LENGTH_PAYLOAD) as u8
}

/// `stated` folded with this file's own FNV-1a-64.
///
/// The second implementation. It shares no line with the one under test, which
/// is the only reason its agreement is evidence of anything.
fn folded_here(stated: &[u8]) -> u64 {
    stated.iter().fold(STATED_OFFSET_BASIS, |folded, byte| {
        (folded ^ u64::from(*byte)).wrapping_mul(STATED_PRIME)
    })
}
