//! Which of a save's two records a declared opacity joins, stated as bytes on
//! both sides so that either revision moving is visible.
//!
//! # Why this reading has to state the bytes rather than compare two folds
//!
//! `format.rs` records the measurement: leaving a revision byte where it was
//! while its field list grew reddens **only** the guards that build the expected
//! bytes by hand. Every other witness in this repository compares one fold to
//! another, and a leading byte that moved in both is invisible to that — so a
//! green suite is no evidence a revision is right. Two hashes compared to each
//! other cannot say which list a new field joined either: appending the degree to
//! *both* lists moves both folds, which is exactly what a comparison of folds
//! reports as a correct change.
//!
//! # The two bytes have to move differently, and that is the whole scenario
//!
//! A degree of opacity is something a block **looks** like. It changes nothing
//! about standing on the block, building through it or breaking it, so a player
//! whose world was saved before the field existed has nothing to decide and
//! nothing to be warned about. That is what the appearance list is for, and its
//! byte moves.
//!
//! The behaviour byte must not. Every block of every save written before a
//! behaviour move reports as `changed` on its next load, which is what
//! `Acceptance::OnlyUnchangedBlocks` refuses — so routing a rendering field
//! through that byte would refuse a world for a change no still frame can show,
//! and would do it to every player at once. `drawn` and `occludes` are on the
//! appearance list for exactly this reason and the degree joins them.
//!
//! # The degree is folded as it was declared, and never as the byte a vertex
//! # carries
//!
//! `Opacity::quantised` exists for the renderer, which has eight bits to spend
//! and rounds into them. A save records what a **declaration said**, and two
//! declarations a quarter of a code value apart are two different declarations
//! however the renderer ends up drawing them. So the four bytes of the `f32` bit
//! pattern go into the record, exactly as the two numbers a medium states do —
//! and the fixture declares `0.25`, which is a degree no default produces and
//! which survives the round trip through both encodings so that neither can be
//! mistaken for the other.
//!
//! # The oracle shares no line with the writer
//!
//! Its own FNV-1a-64, its own length encoding, its own byte sequence — the same
//! arrangement `save_per_face_appearance.rs` and `save_declarations.rs` use, and
//! for the same reason: agreement between two implementations is evidence, and
//! agreement between one implementation and a constant taken out of it is not.

mod common;

use std::error::Error;

use common::persistence::{declaration_of, saved_requirements, world_at, world_holding};
use common::{FIXTURE_ORIGIN, TestResult};
use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin, Opacity};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use tempfile::TempDir;

/// The block this fixture declares, and the one texture key it wears on every
/// facing.
///
/// One key rather than six, because which facing carries which art is
/// `save_per_face_appearance.rs`'s subject and stating six here would make a
/// failure ambiguous between the two files.
const TINTED: &str = "fixture:tinted";
const TINTED_FACE: &str = "fixture:tinted_face";

/// The one cell the fixture world holds it at.
const A_CELL: mc_world::world::WorldPos = world_at(1, 1, 1);

/// Which revision of each field list this file states.
///
/// Written out here rather than read from the format, so that a revision bumped
/// without its list changing is a disagreement rather than a change both sides
/// make together. **The appearance list's number is the one this spec moves and
/// the behaviour list's is the one it must not**, and stating both as literals is
/// what lets one comparison say so.
const STATED_APPEARANCE_REVISION: u8 = 5;
const STATED_BEHAVIOUR_REVISION: u8 = 4;

/// What the fixture declares about how much light it stops.
///
/// A quarter, which is no default and no fold identity: a record that skipped
/// the field, one that folded a constant `1.0`, and one that folded the
/// quantised byte instead of the declared number all disagree with this.
const A_QUARTER: f32 = 0.25;

/// What the fixture declares about being seen and about hiding what is behind
/// it.
///
/// **Unequal on purpose**, so the byte sequence below can see them transposed,
/// and `occludes = false` because a block light passes through may not also hide
/// what lies beyond it — the pairing the loader refuses.
const DRAWN: bool = true;
const OCCLUDES: bool = false;

/// What the fixture declares about everything the behaviour list records.
///
/// Stated as constants rather than derived from solidity, for the reason
/// `common::registry_from` records: nothing has ever answered the three medium
/// questions, so deriving one from solidity would make every fixture in the crate
/// swimmable and no assertion here could see it.
const IS_SOLID: bool = false;
const REPLACEABLE: bool = false;
const BREAKABLE: bool = true;
const TARGETABLE: bool = true;
const SWIMMABLE: bool = false;
const MOVE_RESISTANCE: f32 = 0.0;
const SWIM_ASCENT: f32 = 9.0;

/// How the canonical encoding writes a `bool`, and how it writes an absent
/// optional value.
const FALSE_BYTE: u8 = 0x00;
const TRUE_BYTE: u8 = 0x01;
const NOTHING_BYTE: u8 = 0x00;

/// Where an FNV-1a 64 fold starts, and what it multiplies by.
///
/// The published constants, stated a second time on purpose. Reading them from
/// the crate under test would make a changed constant invisible here.
const STATED_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const STATED_PRIME: u64 = 0x0000_0100_0000_01b3;

/// How many bits of a length one byte of a variable-length length carries, and
/// the mask that takes them.
const LENGTH_PAYLOAD_BITS: u32 = 7;
const LENGTH_CONTINUES: u8 = 0b1000_0000;
const LENGTH_PAYLOAD: usize = 0b0111_1111;

/// What a save recorded about the fixture block: what it was declared to behave
/// like, and what it was declared to look like.
///
/// The two travel together in one comparison, because the whole scenario is that
/// one of them moved and the other did not — and a reading of either alone is
/// satisfied by an implementation that moved both.
#[derive(Debug, PartialEq, Eq)]
enum Recorded {
    Folds { behaviour: u64, appearance: u64 },
    NotNamed,
}

/// A registry holding the fixture block alone.
fn registry_holding_the_fixture() -> Result<BlockRegistry, Box<dyn Error>> {
    let opacity = Opacity::new(A_QUARTER).ok_or("a quarter is a degree of opacity")?;
    let declared = vec![Ok(BlockDefinition {
        name: BlockName::parse(TINTED)?,
        textures: FaceTextures::uniform(TextureKey::parse(TINTED_FACE)?),
        is_solid: IS_SOLID,
        replaceable: REPLACEABLE,
        breakable: BREAKABLE,
        breaks_into: None,
        drawn: DRAWN,
        occludes: OCCLUDES,
        targetable: TARGETABLE,
        swimmable: SWIMMABLE,
        move_resistance: MOVE_RESISTANCE,
        swim_ascent: SWIM_ASCENT,
        opacity,
        origin: DefinitionOrigin::new(FIXTURE_ORIGIN),
        tint: None,
    })];
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        DefinitionOrigin::new(FIXTURE_ORIGIN),
        declared,
    ))?;
    Ok(registry)
}

/// What a save of a world holding the fixture records about it.
///
/// # Errors
///
/// Returns an error if the registry is refused, if the world cannot be built, or
/// if the save cannot be written.
fn recorded() -> Result<Recorded, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let registry = registry_holding_the_fixture()?;
    let world = world_holding(&[(A_CELL, TINTED)], &registry)?;
    let required = saved_requirements(&directory, &world, &registry)?;
    Ok(match declaration_of(&required, TINTED) {
        Some((behaviour, appearance)) => Recorded::Folds {
            behaviour,
            appearance,
        },
        None => Recorded::NotNamed,
    })
}

/// The bytes the format states the fixture's declared behaviour is: the
/// revision, the name, the three flags, the absent residue, whether a swing can
/// find it, and last the three that say what its volume is to move through.
///
/// **No degree of opacity anywhere in it**, which is the half of this reading
/// that has to keep being true rather than become true.
fn stated_behaviour_bytes() -> Vec<u8> {
    let mut stated = vec![STATED_BEHAVIOUR_REVISION];
    push_text(&mut stated, TINTED);
    push_flag(&mut stated, IS_SOLID);
    push_flag(&mut stated, REPLACEABLE);
    push_flag(&mut stated, BREAKABLE);
    stated.push(NOTHING_BYTE);
    push_flag(&mut stated, TARGETABLE);
    push_flag(&mut stated, SWIMMABLE);
    push_number(&mut stated, MOVE_RESISTANCE);
    push_number(&mut stated, SWIM_ASCENT);
    stated
}

/// The bytes the format states the fixture's declared appearance is: the
/// revision, the name, the six keys in the order the six words are written, the
/// two flags that say whether the block is drawn and whether it hides what is
/// behind it, and last the degree of light it stops.
///
/// **Appended after `occludes` and never inserted among the keys.** The canonical
/// encoding writes a struct positionally, so a field placed among the existing
/// ones moves every byte after it and every save in existence would disagree for
/// a reason nobody declared — while the revision byte reported a change smaller
/// than the one that was made.
///
/// **The degree is no longer the last thing in the record**, and the marker
/// behind it is what says so: this fixture declares no medium, and the tag byte
/// for that absence is what every save in existence writes. Without it a degree
/// appended *after* the medium rather than before it would fold to the same
/// bytes as one appended before, which is precisely the sliding this file exists
/// to catch one field further along.
fn stated_appearance_bytes() -> Vec<u8> {
    let mut stated = vec![STATED_APPEARANCE_REVISION];
    push_text(&mut stated, TINTED);
    for _ in 0..6 {
        push_text(&mut stated, TINTED_FACE);
    }
    push_flag(&mut stated, DRAWN);
    push_flag(&mut stated, OCCLUDES);
    push_number(&mut stated, A_QUARTER);
    stated.push(NOTHING_BYTE);
    stated
}

/// `flag` as the canonical encoding writes it.
fn push_flag(stated: &mut Vec<u8>, flag: bool) {
    stated.push(if flag { TRUE_BYTE } else { FALSE_BYTE });
}

/// `number` as the canonical encoding writes it: the four bytes of its bit
/// pattern, least significant first, whatever its value.
fn push_number(stated: &mut Vec<u8>, number: f32) {
    stated.extend_from_slice(&number.to_bits().to_le_bytes());
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
fn a_declared_degree_joins_the_appearance_record_and_moves_only_that_revision() -> TestResult {
    let recorded = recorded()?;

    assert_eq!(
        recorded,
        Recorded::Folds {
            behaviour: folded_here(&stated_behaviour_bytes()),
            appearance: folded_here(&stated_appearance_bytes()),
        },
        "both records are stated as bytes and folded here, so this one comparison separates \
         four things nothing else in the workspace can tell apart: a degree that reached \
         neither record, one that reached the behaviour record instead, one that reached both, \
         and a revision byte moved on the wrong list. The appearance byte has to be {} and the \
         behaviour byte has to still be {} — a save whose behaviour record moved tells every \
         player holding these blocks that they behave differently, and refuses the world \
         outright for anybody who asked to be stopped if anything moved, over a change that is \
         a rendering number and nothing else",
        STATED_APPEARANCE_REVISION,
        STATED_BEHAVIOUR_REVISION
    );
    Ok(())
}
