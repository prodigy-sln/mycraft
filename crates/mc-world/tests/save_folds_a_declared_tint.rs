//! Which of a save's two records a declared medium joins, stated as bytes on
//! both sides so that either revision moving is visible — and what a block
//! declaring no medium at all writes there.
//!
//! # Why this reading has to state the bytes rather than compare two folds
//!
//! `format.rs` records the measurement: leaving a revision byte where it was
//! while its field list grew reddens **only** the guards that build the expected
//! bytes by hand. Every other witness in this repository compares one fold to
//! another, and a leading byte that moved in both is invisible to that — so a
//! green suite is no evidence a revision is right. Two hashes compared to each
//! other cannot say which list a new field joined either: appending the medium
//! to *both* lists moves both folds, which is what a comparison of folds reports
//! as a correct change.
//!
//! # The two bytes have to move differently, and that is the whole scenario
//!
//! What colour a block is seen through from inside is something it **looks**
//! like. It changes nothing about standing on the block, building through it or
//! breaking it, so a player whose world was saved before the fields existed has
//! nothing to decide about it. That is what the appearance list is for, and its
//! byte moves.
//!
//! The behaviour byte must not. Every block of every save written before a
//! behaviour move reports as `changed` on its next load, which is what
//! `Acceptance::OnlyUnchangedBlocks` refuses — so routing a rendering field
//! through that byte would refuse a world for a change no still frame taken from
//! outside the block can show, and would do it to every player at once.
//!
//! # One optional over the pair, not two fields
//!
//! The two declared fields are stated together or not at all, so the record
//! carries **one** optional value holding both. That is the loader's rule
//! expressed in the record's own shape rather than restated beside it, and it is
//! what makes the tag byte meaningful: a single `0x00` separates "this block
//! declares no medium" from every colour at every distance, black at any
//! distance included. Two optionals would admit a record shape the loader
//! refuses, and nothing downstream could tell which of the two absences it was
//! looking at.
//!
//! # The untinted case is a byte, and it is the byte every existing world writes
//!
//! Every declaration written before these fields existed carries no tint, so the
//! absent marker below is what the whole of shipped content and every save in
//! existence records. Stating it by hand is the only thing that can see a `None`
//! encoded as a colour of zeros at a distance of zero — which would fold
//! identically to a block somebody deliberately declared black at no distance,
//! and which the loader refuses anybody from declaring at all.
//!
//! # The oracle shares no line with the writer
//!
//! Its own FNV-1a-64, its own length encoding, its own byte sequence — the same
//! arrangement `save_folds_a_declared_opacity.rs` and `save_per_face_appearance.rs`
//! use, and for the same reason: agreement between two implementations is
//! evidence, and agreement between one implementation and a constant taken out of
//! it is not.

mod common;

use std::error::Error;

use common::persistence::{
    SAVE_FILE, declaration_of, produced_from, saved_requirements, world_at, world_holding,
};
use common::{FIXTURE_ORIGIN, TestResult};
use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin, MediumTint, Opacity};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_world::persistence::Acceptance;
use tempfile::TempDir;

/// The block this fixture declares, and the one texture key it wears on every
/// facing.
///
/// One key rather than six, because which facing carries which art is
/// `save_per_face_appearance.rs`'s subject and stating six here would make a
/// failure ambiguous between the two files.
const SUBMERGING: &str = "fixture:submerging";
const SUBMERGING_FACE: &str = "fixture:submerging_face";

/// The one cell the fixture world holds it at.
const A_CELL: mc_world::world::WorldPos = world_at(1, 1, 1);

/// Which revision of each field list this file states.
///
/// Written out here rather than read from the format, so that a revision bumped
/// without its list changing is a disagreement rather than a change both sides
/// make together. **The appearance list's number is the one this spec moves and
/// the behaviour list's is the one it must not.**
const STATED_APPEARANCE_REVISION: u8 = 5;
const STATED_BEHAVIOUR_REVISION: u8 = 4;

/// The revision the appearance list stands at before this spec moves it.
///
/// The control's own number: a record folded over the grown list under the old
/// byte has to disagree with what the save wrote, or nothing in this repository
/// would report a revision that stayed put while its list changed.
const THE_APPEARANCE_REVISION_BEFORE_THIS_SPEC: u8 = 4;

/// What the fixture declares its medium to be.
///
/// Three channels no two of which are equal, so a record writing them in another
/// order is reported, and none of them zero or `0xFF`, so a record writing a
/// constant is reported too. The distance is a fraction rather than a whole
/// number, which is what catches a record folding an integer where the engine
/// keeps an `f32`.
const THE_CHANNELS: [u8; 3] = [0x3A, 0x6E, 0xA5];
const THE_DISTANCE: f32 = 12.5;

/// What the fixture declares about how much light it stops.
///
/// A quarter, which is no default and no fold identity, and which sits directly
/// before the medium in the record — so a medium written one field too early is
/// a disagreement in the degree as well.
const A_QUARTER: f32 = 0.25;

/// What the fixture declares about being seen and about hiding what is behind
/// it.
///
/// **Unequal on purpose**, so the byte sequence below can see them transposed,
/// and `occludes = false` because a block light passes through may not also hide
/// what lies beyond it.
const DRAWN: bool = true;
const OCCLUDES: bool = false;

/// What the fixture declares about everything the behaviour list records.
///
/// Stated as constants rather than derived from solidity: nothing has ever
/// answered the three medium questions, so deriving one from solidity would make
/// every fixture in the crate swimmable and no assertion here could see it.
const IS_SOLID: bool = false;
const REPLACEABLE: bool = false;
const BREAKABLE: bool = true;
const TARGETABLE: bool = true;
const SWIMMABLE: bool = false;
const MOVE_RESISTANCE: f32 = 0.0;
const SWIM_ASCENT: f32 = 9.0;

/// How the canonical encoding writes a `bool`, and how it writes an absent
/// optional value and a present one.
const FALSE_BYTE: u8 = 0x00;
const TRUE_BYTE: u8 = 0x01;
const NOTHING_BYTE: u8 = 0x00;
const SOMETHING_BYTE: u8 = 0x01;

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

/// A registry holding the fixture block alone, declaring `tint`.
fn registry_holding_the_fixture(tint: Option<MediumTint>) -> Result<BlockRegistry, Box<dyn Error>> {
    let opacity = Opacity::new(A_QUARTER).ok_or("a quarter is a degree of opacity")?;
    let declared = vec![Ok(BlockDefinition {
        name: BlockName::parse(SUBMERGING)?,
        textures: FaceTextures::uniform(TextureKey::parse(SUBMERGING_FACE)?),
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
        tint,
    })];
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        DefinitionOrigin::new(FIXTURE_ORIGIN),
        declared,
    ))?;
    Ok(registry)
}

/// The medium the tinted fixture declares.
///
/// # Errors
///
/// Returns an error if the distance is not one the engine keeps, which would
/// make the fixture's own subject unstatable.
fn the_declared_medium() -> Result<MediumTint, Box<dyn Error>> {
    MediumTint::new(THE_CHANNELS, THE_DISTANCE)
        .ok_or_else(|| "a finite distance greater than zero is a medium the engine keeps".into())
}

/// What a save of a world holding the fixture, declaring `tint`, records about
/// it.
///
/// # Errors
///
/// Returns an error if the registry is refused, if the world cannot be built, or
/// if the save cannot be written.
fn recorded(tint: Option<MediumTint>) -> Result<Recorded, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let registry = registry_holding_the_fixture(tint)?;
    let world = world_holding(&[(A_CELL, SUBMERGING)], &registry)?;
    let required = saved_requirements(&directory, &world, &registry)?;
    Ok(match declaration_of(&required, SUBMERGING) {
        Some((behaviour, appearance)) => Recorded::Folds {
            behaviour,
            appearance,
        },
        None => Recorded::NotNamed,
    })
}

/// The bytes the format states the fixture's declared behaviour is.
///
/// **No colour and no distance anywhere in it**, which is the half of this
/// reading that has to keep being true rather than become true.
fn stated_behaviour_bytes() -> Vec<u8> {
    let mut stated = vec![STATED_BEHAVIOUR_REVISION];
    push_text(&mut stated, SUBMERGING);
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

/// The bytes the format states the fixture's declared appearance is, under
/// `revision` and declaring `tint`.
///
/// **Appended after the degree and never inserted.** The canonical encoding
/// writes a struct positionally, so a field placed among the existing ones moves
/// every byte after it and every save in existence would disagree for a reason
/// nobody declared — while the revision byte reported a change smaller than the
/// one that was made.
///
/// The optional is one tag byte, then the three declared channel bytes with no
/// length in front of them, then the four bytes of the distance's bit pattern.
fn stated_appearance_bytes(revision: u8, tint: Option<([u8; 3], f32)>) -> Vec<u8> {
    let mut stated = vec![revision];
    push_text(&mut stated, SUBMERGING);
    for _ in 0..6 {
        push_text(&mut stated, SUBMERGING_FACE);
    }
    push_flag(&mut stated, DRAWN);
    push_flag(&mut stated, OCCLUDES);
    push_number(&mut stated, A_QUARTER);
    match tint {
        None => stated.push(NOTHING_BYTE),
        Some((channels, distance)) => {
            stated.push(SOMETHING_BYTE);
            stated.extend_from_slice(&channels);
            push_number(&mut stated, distance);
        }
    }
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
/// The second implementation. It shares no line with the one under test, which
/// is the only reason its agreement is evidence of anything.
fn folded_here(stated: &[u8]) -> u64 {
    stated.iter().fold(STATED_OFFSET_BASIS, |folded, byte| {
        (folded ^ u64::from(*byte)).wrapping_mul(STATED_PRIME)
    })
}

/// The folds a save of the untinted fixture has to record.
fn the_untinted_folds() -> Recorded {
    Recorded::Folds {
        behaviour: folded_here(&stated_behaviour_bytes()),
        appearance: folded_here(&stated_appearance_bytes(STATED_APPEARANCE_REVISION, None)),
    }
}

#[test]
fn a_declared_medium_joins_the_appearance_record_and_moves_only_that_revision() -> TestResult {
    let recorded = recorded(Some(the_declared_medium()?))?;

    assert_eq!(
        recorded,
        Recorded::Folds {
            behaviour: folded_here(&stated_behaviour_bytes()),
            appearance: folded_here(&stated_appearance_bytes(
                STATED_APPEARANCE_REVISION,
                Some((THE_CHANNELS, THE_DISTANCE)),
            )),
        },
        "both records are stated as bytes and folded here, so this one comparison separates \
         five things nothing else in the workspace can tell apart: a medium that reached \
         neither record, one that reached the behaviour record instead, one that reached both, \
         one written among the existing fields rather than after them, and a revision byte \
         moved on the wrong list. The appearance byte has to be {} and the behaviour byte has \
         to still be {} — a save whose behaviour record moved tells every player holding these \
         blocks that they behave differently, and refuses the world outright for anybody who \
         asked to be stopped if anything moved, over a colour that is only ever seen from \
         inside a block they are standing in",
        STATED_APPEARANCE_REVISION,
        STATED_BEHAVIOUR_REVISION
    );
    Ok(())
}

/// What a save of the untinted fixture recorded, and what reopening it at the
/// strictest acceptance there is produced.
///
/// # Errors
///
/// Returns an error if the registry is refused, if the world cannot be built, if
/// the save cannot be written, or if the two worlds cannot be compared.
fn written_and_reopened() -> Result<(Recorded, String), Box<dyn Error>> {
    let directory = TempDir::new()?;
    let registry = registry_holding_the_fixture(None)?;
    let world = world_holding(&[(A_CELL, SUBMERGING)], &registry)?;
    let required = saved_requirements(&directory, &world, &registry)?;
    let reopened = produced_from(
        &directory.path().join(SAVE_FILE),
        &registry,
        Acceptance::OnlyUnchangedBlocks,
        &world,
    )?;
    let recorded = declaration_of(&required, SUBMERGING).map_or(
        Recorded::NotNamed,
        |(behaviour, appearance)| Recorded::Folds {
            behaviour,
            appearance,
        },
    );
    Ok((recorded, reopened))
}

#[test]
fn a_block_declaring_no_medium_records_one_absent_marker_and_reopens_unchanged() -> TestResult {
    assert_eq!(
        written_and_reopened()?,
        (the_untinted_folds(), common::persistence::AGREES.to_owned()),
        "every declaration written before these fields existed carries no medium, so this is \
         what the whole of shipped content and every world anybody has saved records — and it \
         has to be **one byte**. A `None` written as a colour of zeros at a distance of zero \
         would fold identically to a block somebody declared black at no distance, which is a \
         declaration the loader refuses outright; the two would then be indistinguishable on \
         disk and a later reader could not tell which it was holding. The reopen is asked at \
         the strictest acceptance there is, so a record that came out unstable across a write \
         and a read is a refusal here rather than a silent pass"
    );
    Ok(())
}

#[test]
fn the_appearance_record_folded_under_todays_revision_disagrees_rather_than_passing() -> TestResult
{
    let folded_under = |revision| {
        folded_here(&stated_appearance_bytes(
            revision,
            Some((THE_CHANNELS, THE_DISTANCE)),
        ))
    };
    let Recorded::Folds { appearance, .. } = recorded(Some(the_declared_medium()?))? else {
        return Err("a save of a world holding the fixture has to name it".into());
    };

    assert_eq!(
        (
            appearance == folded_under(STATED_APPEARANCE_REVISION),
            appearance == folded_under(THE_APPEARANCE_REVISION_BEFORE_THIS_SPEC),
        ),
        (true, false),
        "the same record read against two revision bytes, and both halves are the assertion. A \
         revision left at {} while the list it is folded over grew is invisible to every \
         witness in this repository that compares one fold to another — both folds move, both \
         comparisons agree, and a save written before the change reports its blocks as \
         unchanged when they are not. The first half says the by-hand sequence is the one the \
         format writes; the second says that sequence is actually **sensitive** to the leading \
         byte rather than merely carrying it, which is what a comparison against the new \
         revision alone cannot state",
        THE_APPEARANCE_REVISION_BEFORE_THIS_SPEC
    );
    Ok(())
}
