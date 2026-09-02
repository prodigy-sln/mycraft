//! What a declaration says about the colour its block is seen through from
//! inside, how far it lets an eye see, and what the registry keeps of both.
//!
//! # Two claims, and the second is not a strength written twice
//!
//! `tint` says what colour a medium carries what is seen through it toward;
//! `tint_distance` says how far a surface stands before it is drawn wholly at
//! that colour. The colour states no alpha, because how strongly the medium acts
//! is the distance's job — a colour carrying one as well would be two fields
//! answering one question, and the loader would have to decide which of the two
//! numbers an author meant.
//!
//! # Both dialects, because shipped content already speaks both
//!
//! `content/base/materials/*.toml` writes `#rrggbb` in lowercase and
//! `content/base/hud/*.toml` writes `#RRGGBBAA` in uppercase, each behind a
//! reader whose refusal claims its own form is the only one a colour takes. Both
//! claims are false about this tree. This field accepts both forms in either
//! case, which is what keeps it from being a third rule, and the three spellings
//! below have to register as one value — a save folds the declared bytes, so two
//! spellings of one colour hashing apart would tell every player holding that
//! block that it was retextured.
//!
//! # The fixtures declare a tint where no other field could have supplied one
//!
//! There is no bit on a declaration this field could plausibly be derived from,
//! but there is one it could plausibly be *gated* on: a loader that kept a tint
//! only for a block light passes through would satisfy every reading about
//! water and drop the tint of every opaque block. So one fixture below states
//! `opacity = 1.0` beside its tint and requires both — what a block looks like
//! from outside and what it does to a view from inside are separate claims, and
//! the specification says so outright rather than leaving an implementer to
//! guess.
//!
//! # Read as bytes and bits
//!
//! The colour is compared as the three channel bytes and the distance as its
//! `f32` bits, which is exact. A loader that rounded a channel, re-ordered two
//! of them, or re-scaled the distance on the way in has nowhere to land that a
//! value comparison would forgive — and `-0.0` retained where `0.0` was meant is
//! invisible to every comparison but this one.
//!
//! # A refusal can never satisfy a comparison here
//!
//! Every reading answers [`WhatTheRootRegistered`], whose refused arm carries the
//! refusal's own words. A reading that propagated a refusal with `?` would end
//! before its assertion ran — which matters most in exactly the state this file
//! is authored in, where neither field is recognised at all and every root here
//! is refused for a reason that has nothing to do with the values it states.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, ASH, ASH_FILE, QUARTZ, declaration_of, raw_field, text_field,
};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::content::LuauFileDefinitionSource;
use tempfile::TempDir;

/// The key a declaration states the colour of the medium it is in.
const TINT_FIELD: &str = "tint";

/// The key a declaration states how far that medium lets an eye see in.
const TINT_DISTANCE_FIELD: &str = "tint_distance";

/// The key a declaration states how much light it stops in.
const OPACITY_FIELD: &str = "opacity";

/// The key a declaration states hiding its neighbours in, and the value every
/// fixture here states it as.
///
/// Stated outright rather than left to the block's own solidity, so that no
/// fixture below accidentally declares the contradiction between an occluding
/// block and a degree under one that the loader already refuses.
const OCCLUDES_FIELD: &str = "occludes";
const HIDING_NOTHING: &str = "false";

/// A blue, in the six-digit form `content/base/materials/*.toml` writes.
///
/// Deliberately not a colour any engine constant supplies: it is neither the
/// sky's `#87CEEB`, nor black, nor white, so a loader answering from anything
/// but the declaration cannot reach it.
const A_BLUE: &str = "'#3A6EA5'";

/// The same blue written in lowercase, which is how shipped material files
/// spell one.
const THE_SAME_BLUE_IN_LOWERCASE: &str = "'#3a6ea5'";

/// The same blue again with an alpha of `FF`, which is how shipped HUD files
/// spell one.
const THE_SAME_BLUE_WITH_A_FULL_ALPHA: &str = "'#3A6EA5FF'";

/// The three channel bytes all three spellings mean.
const THE_BLUES_CHANNELS: [u8; 3] = [0x3A, 0x6E, 0xA5];

/// A brown, for the second block of the pair.
///
/// Neither channel-wise equal to the blue nor any permutation of it, so a loader
/// that resolved both blocks from one declaration answers the same triple twice
/// and is reported, and one that shuffled the channels on the way in is reported
/// too.
const A_BROWN: &str = "'#8A4400'";

/// The three channel bytes it means.
const THE_BROWNS_CHANNELS: [u8; 3] = [0x8A, 0x44, 0x00];

/// How far the blue lets an eye see, and how far the brown does.
///
/// Two distances rather than one, and neither a multiple of the other, so a
/// loader carrying one block's distance into the other has nowhere to hide.
const TWELVE_BLOCKS: &str = "12.0";
const THREE_BLOCKS: &str = "3.0";

/// A degree that stops all the light, stated rather than left out.
const STOPPING_EVERYTHING: &str = "1.0";

/// A degree that lets half of it through, which is what a medium an eye can
/// stand inside actually declares.
const PASSING_HALF_THE_LIGHT: &str = "0.5";

/// The degree an unstated `opacity` means, written here rather than read from
/// the loader.
const AN_UNSTATED_DEGREE: f32 = 1.0;

/// What one registered block was declared to be.
///
/// The name, the degree it stops light at, and the tint — one record rather than
/// three readings, so a loader that kept the tint and lost the degree is not
/// mistaken for one that kept both. The degree travels because the tint is the
/// one field on this declaration something might plausibly *gate* on it.
#[derive(Debug, PartialEq, Eq)]
struct Registered {
    name: String,
    degree_bits: u32,
    /// The three declared channel bytes and the distance's `f32` bits, or
    /// nothing where the block declares no tint.
    ///
    /// One `Option` over the pair rather than two, because the two fields are
    /// stated together or not at all: a shape admitting a colour without a
    /// distance would be a shape the loader has to refuse and this reading could
    /// still describe.
    tint: Option<([u8; 3], u32)>,
}

/// What a content root did with the declarations it was handed.
///
/// **Two arms, and the refused one carries words.** A reading that could only
/// answer "these blocks registered" cannot distinguish a root that was refused
/// from one that registered nothing, and a refusal is the answer every fixture
/// here gets on a tree where neither field is recognised.
#[derive(Debug, PartialEq, Eq)]
enum WhatTheRootRegistered {
    /// Accepted, and these are the blocks it holds, in the order they were
    /// asked for.
    Blocks(Vec<Registered>),
    /// Refused, rendered as it renders itself.
    Refused(String),
}

/// A root whose one declaration is [`AMBER`] — seen, hiding nothing — stating
/// each of `fields` and otherwise well formed.
///
/// The shape every fixture here takes: a declaration that would register, and
/// the lines that are the subject stated on top of it. Handing an empty slice
/// writes the same declaration with those lines left out, which is what makes
/// the absent-field reading a reading of *this* fixture minus two lines rather
/// than of a different one.
fn root_of(directory: &TempDir, fields: &[(&str, &str)]) -> Result<PathBuf, Box<dyn Error>> {
    content_root(directory, &[(AMBER_FILE, amber_stating(fields))])
}

/// [`AMBER`]'s declaration, stating each of `fields`.
fn amber_stating(fields: &[(&str, &str)]) -> String {
    declaration_of(&stated_fields(AMBER, fields))
}

/// A well-formed declaration of `name` stating each of `fields`.
fn stated_fields(name: &str, fields: &[(&str, &str)]) -> Vec<String> {
    let mut declared = vec![
        text_field("name", name),
        text_field("texture", QUARTZ),
        raw_field("solid", "false"),
        raw_field(OCCLUDES_FIELD, HIDING_NOTHING),
    ];
    for (field, stated) in fields {
        declared.push(raw_field(field, stated));
    }
    declared
}

/// A root declaring two blocks, each stating its own colour and distance.
fn root_declaring_two(
    directory: &TempDir,
    one: (&str, &str),
    other: (&str, &str),
) -> Result<PathBuf, Box<dyn Error>> {
    let declaring = |name: &str, (colour, distance): (&str, &str)| {
        declaration_of(&stated_fields(
            name,
            &[
                (TINT_FIELD, colour),
                (TINT_DISTANCE_FIELD, distance),
                (OPACITY_FIELD, PASSING_HALF_THE_LIGHT),
            ],
        ))
    };
    content_root(
        directory,
        &[
            (AMBER_FILE, declaring(AMBER, one)),
            (ASH_FILE, declaring(ASH, other)),
        ],
    )
}

/// What the root at `root` registered for each of `names`, in that order.
fn what_registered(root: &Path, names: &[&str]) -> WhatTheRootRegistered {
    let mut registry = BlockRegistry::new();
    if let Err(refused) = registry.apply(&LuauFileDefinitionSource::new(root)) {
        return WhatTheRootRegistered::Refused(refused.to_string());
    }
    let mut registered = Vec::new();
    for name in names {
        match BlockName::parse(name).ok().and_then(|parsed| {
            registry.resolve(&parsed).ok().map(|definition| Registered {
                name: (*name).to_owned(),
                degree_bits: definition.opacity.get().to_bits(),
                tint: definition
                    .tint
                    .map(|held| (held.color(), held.distance().to_bits())),
            })
        }) {
            Some(block) => registered.push(block),
            None => {
                return WhatTheRootRegistered::Refused(format!(
                    "the root was accepted and does not hold `{name}`"
                ));
            }
        }
    }
    WhatTheRootRegistered::Blocks(registered)
}

/// One accepted [`AMBER`], stopping `degree` of the light and carrying `tint`.
fn amber_carrying(degree: f32, tint: Option<([u8; 3], f32)>) -> WhatTheRootRegistered {
    WhatTheRootRegistered::Blocks(vec![Registered {
        name: AMBER.to_owned(),
        degree_bits: degree.to_bits(),
        tint: tint.map(|(channels, distance)| (channels, distance.to_bits())),
    }])
}

/// The three spellings of one colour, each read from a root of its own.
fn what_each_spelling_registered(
    spellings: [&str; 3],
) -> Result<Vec<WhatTheRootRegistered>, Box<dyn Error>> {
    let mut read = Vec::with_capacity(spellings.len());
    for spelling in spellings {
        let directory = TempDir::new()?;
        let root = root_of(
            &directory,
            &[
                (TINT_FIELD, spelling),
                (TINT_DISTANCE_FIELD, TWELVE_BLOCKS),
                (OPACITY_FIELD, PASSING_HALF_THE_LIGHT),
            ],
        )?;
        read.push(what_registered(&root, &[AMBER]));
    }
    Ok(read)
}

#[test]
fn one_colour_written_three_ways_registers_as_one_pair_of_values() -> TestResult {
    let read = what_each_spelling_registered([
        A_BLUE,
        THE_SAME_BLUE_IN_LOWERCASE,
        THE_SAME_BLUE_WITH_A_FULL_ALPHA,
    ])?;

    let one_pair = amber_carrying(0.5, Some((THE_BLUES_CHANNELS, 12.0)));
    assert_eq!(
        read,
        vec![
            amber_carrying(0.5, Some((THE_BLUES_CHANNELS, 12.0))),
            amber_carrying(0.5, Some((THE_BLUES_CHANNELS, 12.0))),
            one_pair,
        ],
        "shipped content already writes colours in two dialects, each behind a reader whose \
         refusal says its own form is the only one — so an author copying either file has to \
         get a block that works. The three spellings are compared against one expectation \
         rather than against each other, because three spellings agreeing tells you nothing \
         about whether any of them is the colour that was declared. Compared as channel bytes \
         and distance bits, which is exact: a loader that upper-cased its way to the wrong \
         nibble, dropped the alpha by truncating the string rather than by reading it, or \
         re-scaled the distance has nowhere to land"
    );
    Ok(())
}

#[test]
fn a_declaration_stating_neither_field_registers_carrying_no_tint_at_all() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_of(&directory, &[(OPACITY_FIELD, PASSING_HALF_THE_LIGHT)])?;

    assert_eq!(
        what_registered(&root, &[AMBER]),
        amber_carrying(0.5, None),
        "every declaration written before these fields existed says nothing about them, and \
         every one of them has to go on meaning what it always meant. There is no default tint \
         and no default distance anywhere in the engine, so the absence is `None` and not a \
         colourless tint at some distance — a default would be an engine constant standing in \
         for content, which invariant 1 forbids. This block passes half the light, which is \
         exactly the shape a loader gating the tint on translucency would hand a colour to, so \
         the absence here is a claim rather than an accident"
    );
    Ok(())
}

#[test]
fn two_blocks_stating_two_media_are_each_registered_with_their_own() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring_two(&directory, (A_BLUE, TWELVE_BLOCKS), (A_BROWN, THREE_BLOCKS))?;

    assert_eq!(
        what_registered(&root, &[AMBER, ASH]),
        WhatTheRootRegistered::Blocks(vec![
            Registered {
                name: AMBER.to_owned(),
                degree_bits: 0.5f32.to_bits(),
                tint: Some((THE_BLUES_CHANNELS, 12.0f32.to_bits())),
            },
            Registered {
                name: ASH.to_owned(),
                degree_bits: 0.5f32.to_bits(),
                tint: Some((THE_BROWNS_CHANNELS, 3.0f32.to_bits())),
            },
        ]),
        "one declaration per block is the whole premise of a content root, and a medium is \
         exactly the kind of thing a reader might reasonably hold in one place for the whole \
         root — it is a rendering property, and the frame carries one tint at a time. Both \
         blocks are read in one comparison so that a loader carrying one block's medium into \
         the other is reported rather than half-reported, and the two colours share no channel \
         and the two distances are not multiples, so a loader that resolved both from one \
         declaration has nowhere to hide"
    );
    Ok(())
}

#[test]
fn a_block_that_stops_all_the_light_may_still_declare_what_it_looks_like_from_inside() -> TestResult
{
    let directory = TempDir::new()?;
    let root = root_of(
        &directory,
        &[
            (TINT_FIELD, A_BLUE),
            (TINT_DISTANCE_FIELD, TWELVE_BLOCKS),
            (OPACITY_FIELD, STOPPING_EVERYTHING),
        ],
    )?;

    assert_eq!(
        what_registered(&root, &[AMBER]),
        amber_carrying(AN_UNSTATED_DEGREE, Some((THE_BLUES_CHANNELS, 12.0))),
        "what a block looks like from outside and what it does to a view from inside are \
         separate claims, and this is the declaration that proves the loader keeps them apart. \
         The tempting wrong rule is to gate the tint on translucency — a medium is something \
         you see through, so surely an opaque block has no view from inside — and it is wrong \
         because the block's own faces are back-facing along every ray that leaves an eye \
         standing in it. Such a block draws the whole frame at its declared colour, which is a \
         thing a mod author may want and the engine has no business refusing. Neither field is \
         refused and neither is dropped, which are two different ways of getting this wrong"
    );
    Ok(())
}
