//! What a declaration says about how much light its block stops, and what the
//! registry keeps of it.
//!
//! # The field states a degree, and both ends of it are declarations
//!
//! `1.0` stops all the light — an ordinary block, and what a declaration saying
//! nothing about the matter means. `0.0` stops none. Both bounds are inclusive,
//! so neither end is a value the loader may treat as a mistake: a block declared
//! at exactly `1.0` is the commonest thing anybody writes on purpose, and one at
//! exactly `0.0` is a pane of glass with no glass in it. The pair below is what
//! says a floor written `> 0.0` or a ceiling written `< 1.0` takes a legal
//! declaration away.
//!
//! # Solidity travels in every comparison, and it is not decoration
//!
//! For the reason it travels in `luau_declaration_medium.rs`: a loader that
//! answered this question **from the wrong field** has to have somewhere to be
//! wrong. Every fixture here declares a solidity that disagrees with the answer a
//! derivation would give — a solid block that lets light through, a non-solid one
//! that stops it — so a loader deriving opacity from `solid` cannot pass any
//! comparison in this file by accident.
//!
//! # Every fixture states `occludes` outright, and the first draft of this file
//! # did not
//!
//! **`occludes` is one of the three fields whose absence means the block's own
//! solidity**, so `solid = true` with no `occludes` line is a declaration that
//! *states* occlusion as surely as one that spells it — the registry holds
//! `true` either way and the mesher reads that. A degree below one beside it is
//! the contradiction FR-1.3-S1 refuses, so the fixtures below were asking the
//! loader to register a declaration the loader is required to refuse, and every
//! one of them was about a block nobody could ship.
//!
//! It was invisible while the field was unrecognised, because a root refused for
//! *any* reason fails these comparisons the same way. It surfaced the moment the
//! loader could read the field, and only then. What it cost is recorded here
//! rather than repaired quietly: a fixture that supplies a value in a form the
//! product reads differently than its author intended is correct-looking on the
//! page, and there is no assertion that can catch it — only reading the loader.
//!
//! Stating `occludes = false` costs the control above nothing. The block is
//! still solid while letting light through, so a loader deriving the degree from
//! solidity still answers `1.0` and is still reported.
//!
//! # The degree is read by its bits
//!
//! **Because the one thing a value comparison cannot see is `-0.0` retained
//! where `0.0` was meant**, and a save folds this number by its bits: two
//! declarations meaning the same thing would hash differently and tell every
//! player holding either block that it no longer looks as it did. The
//! normalisation that prevents it already exists for the two numbers a medium
//! states; what has no witness is whether opacity still reaches it once that
//! reader has been given a ceiling. The signed-zero fixture below is that
//! witness.
//!
//! # A refusal can never satisfy a comparison here
//!
//! Every reading answers [`WhatTheRootRegistered`], whose refused arm carries the
//! refusal's own words. A reading that propagated a refusal with `?` would end
//! before its assertion ran, and a test that never reached its assertion has not
//! shown it was checking the right thing — which matters most in exactly the
//! state this file is authored in, where the field is not recognised at all and
//! every root here is refused for a reason that has nothing to do with the value
//! it states.

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

/// The key a declaration states how much light it stops in.
const OPACITY_FIELD: &str = "opacity";

/// The key a declaration states hiding its neighbours in, and the value every
/// fixture here states it as.
///
/// **Stated and never left out.** See this file's header: an absent `occludes`
/// means the block's own solidity, so leaving it out of a solid fixture declares
/// the very contradiction the loader refuses.
const OCCLUDES_FIELD: &str = "occludes";
const HIDING_NOTHING: &str = "false";

/// A degree halfway between stopping all the light and stopping none, written
/// the way Luau writes a fraction.
///
/// Deliberately not a value an unstated field could produce: a loader that never
/// read the field answers `1.0` and cannot reach this by accident.
const HALFWAY: &str = "0.5";

/// A quarter and three quarters, the two the pair fixture states.
///
/// Two fractions rather than a fraction and a whole number, and neither of them
/// the other's complement under any operation the loader performs — so a loader
/// that resolved both blocks from one declaration answers the same value twice
/// and is reported, and one that inverted the degree on the way in is reported
/// too.
const A_QUARTER: &str = "0.25";
const THREE_QUARTERS: &str = "0.75";

/// Stopping no light at all, stated rather than left out.
///
/// The value a floor written `> 0.0` takes away, and the one a loader confusing
/// "stated zero" with "said nothing" answers `1.0` for — which is a block a mod
/// author declared invisible and got a wall.
const STOPPING_NOTHING: &str = "0.0";

/// Stopping all of it, stated rather than left out.
///
/// The value a ceiling written `< 1.0` takes away. It is also what an absent
/// field means, which is the whole reason it needs a fixture of its own: the two
/// are the same number reached by two routes, and a loader that refused the
/// stated one would leave every author who spelled out their opaque block with a
/// content root that will not load.
const STOPPING_EVERYTHING: &str = "1.0";

/// Stopping all of it written the way Luau writes a whole number.
///
/// Both `ScriptValue` variants a declared number can arrive in are stated
/// somewhere in this file, because the host carries an integer and a fraction as
/// separate variants and a reader taking one of them refuses half the values the
/// modding guide shows.
const STOPPING_EVERYTHING_AS_AN_INTEGER: &str = "1";

/// Zero written with a sign in front of it, which Luau retains.
const STOPPING_SIGNED_NOTHING: &str = "-0.0";

/// The degree an unstated field means, written here rather than read from the
/// loader: an expectation derived from the value under test agrees with whatever
/// that value becomes.
const AN_UNSTATED_DEGREE: f32 = 1.0;

/// What one registered block was declared to be.
///
/// The name, the solidity that is deliberately not what the degree is derived
/// from, and the degree itself by its bits. One record rather than three
/// readings, so a loader that got the name and the solidity right and the degree
/// wrong is not mistaken for one that got all three right.
#[derive(Debug, PartialEq, Eq)]
struct Registered {
    name: String,
    solid: bool,
    degree_bits: u32,
}

/// What a content root did with the declarations it was handed.
///
/// **Two arms, and the refused one carries words.** A reading that could only
/// answer "these blocks registered" cannot distinguish a root that was refused
/// from one that registered nothing, and a refusal is the answer every fixture
/// here gets on a tree where the field is not recognised. The refusal renders
/// itself so that a failure names the reason rather than reporting an absence.
#[derive(Debug, PartialEq, Eq)]
enum WhatTheRootRegistered {
    /// Accepted, and these are the blocks it holds, in the order they were
    /// asked for.
    Blocks(Vec<Registered>),
    /// Refused, rendered as it renders itself.
    Refused(String),
}

/// A root whose one declaration is [`AMBER`], solid or not as `solid` says, and
/// stating the opacity `stated` where one is given.
///
/// The shape every fixture here takes: a declaration that would register, and one
/// line in it that is the subject. Handing `None` writes the same declaration
/// with that line left out, which is what makes the absent-field reading a
/// reading of *this* fixture minus one line rather than of a different one.
///
/// `occludes = false` is stated outright rather than left to the block's own
/// solidity, for the reason this file's header records — without it, every solid
/// fixture below asks the loader to accept a contradiction.
fn root_stating(
    directory: &TempDir,
    solid: bool,
    stated: Option<&str>,
) -> Result<PathBuf, Box<dyn Error>> {
    let mut fields = vec![
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", if solid { "true" } else { "false" }),
        raw_field(OCCLUDES_FIELD, HIDING_NOTHING),
    ];
    if let Some(degree) = stated {
        fields.push(raw_field(OPACITY_FIELD, degree));
    }
    content_root(directory, &[(AMBER_FILE, declaration_of(&fields))])
}

/// A root declaring two blocks, each stating its own degree.
fn root_declaring_two(
    directory: &TempDir,
    one: &str,
    other: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let declaring = |name: &str, solid: bool, degree: &str| {
        declaration_of(&[
            text_field("name", name),
            text_field("texture", QUARTZ),
            raw_field("solid", if solid { "true" } else { "false" }),
            raw_field(OCCLUDES_FIELD, HIDING_NOTHING),
            raw_field(OPACITY_FIELD, degree),
        ])
    };
    content_root(
        directory,
        &[
            (AMBER_FILE, declaring(AMBER, true, one)),
            (ASH_FILE, declaring(ASH, false, other)),
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
                solid: definition.is_solid,
                degree_bits: definition.opacity.get().to_bits(),
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

/// One accepted [`AMBER`], solid as `solid` says and holding `degree`.
fn amber_at(solid: bool, degree: f32) -> WhatTheRootRegistered {
    WhatTheRootRegistered::Blocks(vec![Registered {
        name: AMBER.to_owned(),
        solid,
        degree_bits: degree.to_bits(),
    }])
}

#[test]
fn a_declaration_stating_half_a_degree_registers_at_exactly_that_degree() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(&directory, true, Some(HALFWAY))?;

    assert_eq!(
        what_registered(&root, &[AMBER]),
        amber_at(true, 0.5),
        "this is the whole of what a mod author gets for writing the line: a block the registry \
         holds at the degree they stated. The block is declared **solid** while stating that \
         light passes through it, so a loader answering this question from `solid` — the only \
         bit on the declaration that was ever about what a block does to what is behind it — \
         gives `1.0` here and is reported. The degree is compared by its bits, which is exact: \
         a loader that rounded, clamped or re-scaled the number on the way in has nowhere to \
         land that a value comparison would forgive"
    );
    Ok(())
}

#[test]
fn a_declaration_that_states_no_degree_registers_stopping_all_the_light() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(&directory, false, None)?;

    assert_eq!(
        what_registered(&root, &[AMBER]),
        amber_at(false, AN_UNSTATED_DEGREE),
        "every declaration written before this field existed says nothing about it, and every \
         one of them has to go on meaning what it always meant. The default is a **constant** \
         and never `whatever you wrote for solid`: this block is declared non-solid, so a \
         loader deriving the degree from solidity makes it invisible — and it would make every \
         non-solid block in the game invisible, which is a content root that loads and a world \
         nobody can see"
    );
    Ok(())
}

#[test]
fn two_blocks_stating_two_degrees_are_each_registered_at_their_own() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring_two(&directory, A_QUARTER, THREE_QUARTERS)?;

    assert_eq!(
        what_registered(&root, &[AMBER, ASH]),
        WhatTheRootRegistered::Blocks(vec![
            Registered {
                name: AMBER.to_owned(),
                solid: true,
                degree_bits: 0.25f32.to_bits(),
            },
            Registered {
                name: ASH.to_owned(),
                solid: false,
                degree_bits: 0.75f32.to_bits(),
            },
        ]),
        "one declaration per block is the whole premise of a content root, and a degree is the \
         first thing on this declaration a reader might reasonably hold in one place for the \
         whole root — it is a rendering number, and the layer a block draws from is shared. \
         Both blocks are read in one comparison so that a loader carrying one block's degree \
         into the other is reported rather than half-reported, and the two values are neither \
         equal nor each other's complement, so a loader that resolved both from one \
         declaration, or that inverted the degree on the way in, has nowhere to hide"
    );
    Ok(())
}

#[test]
fn a_degree_at_either_end_of_the_range_registers_and_the_root_is_refused_neither_time() -> TestResult
{
    let stopping_nothing = TempDir::new()?;
    let stopping_everything = TempDir::new()?;
    let nothing = root_stating(&stopping_nothing, true, Some(STOPPING_NOTHING))?;
    let everything = root_stating(&stopping_everything, false, Some(STOPPING_EVERYTHING))?;

    assert_eq!(
        (
            what_registered(&nothing, &[AMBER]),
            what_registered(&everything, &[AMBER]),
        ),
        (amber_at(true, 0.0), amber_at(false, 1.0)),
        "both bounds are inclusive, so both ends are declarations rather than mistakes, and \
         each has its own way of being taken away: a floor written `> 0.0` refuses the pane \
         with no glass in it, and a ceiling written `< 1.0` refuses every author who spelled \
         out the opaque block they could have left unstated. The two are read in one comparison \
         because a loader that admitted one end and refused the other is the likely defect and \
         a reading of either end alone cannot see it. Each root is a root of its own, so \
         `refused neither time` is a statement about two loads and not about one"
    );
    Ok(())
}

#[test]
fn a_degree_of_signed_nothing_is_retained_as_the_unsigned_zero_a_save_folds() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_stating(&directory, true, Some(STOPPING_SIGNED_NOTHING))?;

    assert_eq!(
        what_registered(&root, &[AMBER]),
        amber_at(true, 0.0),
        "a save folds this number by its bits, and the two zeroes have different ones — so two \
         declarations meaning the same thing would hash apart and tell every player holding \
         either block that it no longer looks as it did. The normalisation already exists for \
         the two numbers a medium states; what nothing else here watches is whether this field \
         still reaches it once that reader has been given a ceiling to check as well as a \
         floor. Compared by bits, because `-0.0 == 0.0` is true and this is the one reading \
         that must not agree"
    );
    Ok(())
}

#[test]
fn a_degree_written_as_a_whole_number_registers_as_the_same_degree_a_fraction_does() -> TestResult {
    let integer = TempDir::new()?;
    let fraction = TempDir::new()?;
    let whole = root_stating(&integer, true, Some(STOPPING_EVERYTHING_AS_AN_INTEGER))?;
    let pointed = root_stating(&fraction, true, Some(STOPPING_EVERYTHING))?;

    assert_eq!(
        (
            what_registered(&whole, &[AMBER]),
            what_registered(&pointed, &[AMBER]),
        ),
        (amber_at(true, 1.0), amber_at(true, 1.0)),
        "Luau writes `1` as an integer and `1.0` as a number and the host carries the two as \
         separate values, so a reader taking one of them refuses half of what a mod author will \
         type — and refuses it for a reason nobody can read off their own line. The two \
         spellings are compared against one expectation rather than against each other, because \
         two spellings agreeing tells you nothing about whether either is the degree that was \
         declared"
    );
    Ok(())
}
