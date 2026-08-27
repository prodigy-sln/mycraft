//! The three properties a declaration may state about the *medium* its volume
//! is, and what it means by leaving them out.
//!
//! `swimmable` says a player can hold itself up in the volume; `move_resistance`
//! says how much the volume slows what moves through it; `swim_ascent` says how
//! fast it lifts a swimmer who asks to rise. They are the fields whose absence
//! means a **constant** in a file that already carries three whose absence means
//! whatever the same declaration said about `solid`, and that difference is the
//! whole reason this file sits beside `luau_declaration_properties.rs` rather
//! than inside it.
//!
//! # The third one's default is not the other two's
//!
//! `swimmable` absent is `false` and `move_resistance` absent is `0.0`, and both
//! of those are the value a volume contributes when it is not there at all. A
//! `swim_ascent` absent is **`9.0`** — the speed the player's own jump leaves
//! the ground at — which is what keeps a declaration written before this field
//! existed lifting exactly as it did, and is emphatically not what an empty cell
//! contributes. Every reading below therefore states the ascent it expects even
//! where the fixture is about something else: `9.0` is a number a loader that
//! never read the field cannot answer by accident once any fixture states
//! another one.
//!
//! # Absence means a constant, never solidity
//!
//! `drawn`, `occludes` and `targetable` default to `solid` because one bit used
//! to answer those three questions, so a declaration written before they existed
//! is still stating them. **Nothing has ever answered these two**, so a derived
//! default here would invent a claim no author made — and would make every solid
//! block in existence swimmable. The fixture that can see that is the **solid**
//! one stating neither field: it is the only shape in this file that reddens
//! against a loader routing either name through `defaulting_to_solidity`.
//!
//! # `move_resistance` is the first number a declaration may state
//!
//! Luau writes `4` as an integer and `4.5` as a number, and the host carries the
//! two as separate `ScriptValue` variants. A reader accepting one of them refuses
//! half the values the modding guide shows, so both are stated here against the
//! same expectation — which is what makes the two fixtures distinct paths rather
//! than one restated.
//!
//! # The retained width is the width the physics divides by
//!
//! What a declaration states is kept as the `f32` a tick multiplies with, and the
//! save fold serialises that number **by its bits**. So `-0.0` and `0.0` are one
//! declaration's worth of difference away from telling every player their blocks
//! changed, and `==` cannot see it: `-0.0 == 0.0` is true in IEEE-754. The
//! normalisation is asserted through [`Retained`], which compares bits, and every
//! other reading here compares the value.
//!
//! # Every reading is total
//!
//! A root stating a field the loader has no meaning for is *refused*, so a
//! reading that propagated a refusal with `?` would end each of these tests
//! before its assertion ever ran — and a test that never reached its assertion
//! has not shown it was checking the right thing. Both readings below answer with
//! the refusal instead, so one comparison judges either outcome.

mod common;
mod luau_common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::{TestResult, content_root};
use luau_common::{
    AMBER, AMBER_FILE, QUARTZ, declaration_of, raw_field, registry_from, text_field,
};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use tempfile::TempDir;

/// The key a declaration states being something a player can swim in.
const SWIMMABLE_FIELD: &str = "swimmable";

/// The key a declaration states how much its volume slows movement in.
const MOVE_RESISTANCE_FIELD: &str = "move_resistance";

/// The key a declaration states how fast its volume lifts a swimmer in.
const SWIM_ASCENT_FIELD: &str = "swim_ascent";

/// How fast a volume lifts a swimmer when its declaration says nothing about it.
///
/// The speed the player's own jump leaves the ground at, written out here rather
/// than read from the loader: an expectation derived from the value under test
/// agrees with whatever that value becomes.
const AN_UNSTATED_ASCENT: f32 = 9.0;

/// An ascent written the way Luau writes a fraction, and slower than an unstated
/// one so that a loader which never read the field cannot answer it.
const AN_ASCENT_WRITTEN_AS_A_NUMBER: &str = "3.5";

/// The same magnitude one greater, written the way Luau writes a whole number,
/// so both `ScriptValue` variants a declared number can arrive in are stated.
const AN_ASCENT_WRITTEN_AS_AN_INTEGER: &str = "4";

/// An ascent of nothing at all, stated rather than left out.
///
/// The one value that separates "stated zero" from "said nothing": a loader
/// treating this field's zero as its silence answers [`AN_UNSTATED_ASCENT`] and
/// gives a mod author a volume that flings a swimmer upward where they declared
/// one that lifts nobody.
const AN_ASCENT_OF_NOTHING: &str = "0";

/// Zero written with a sign in front of it, which Luau retains.
const AN_ASCENT_OF_SIGNED_NOTHING: &str = "-0.0";

/// A resistance written the way Luau writes a whole number.
const A_RESISTANCE_WRITTEN_AS_AN_INTEGER: &str = "4";

/// The same magnitude written the way Luau writes a fraction, so that the two
/// `ScriptValue` variants a declaration can reach the host through are both
/// stated against an expectation only one of them satisfies by accident.
const A_RESISTANCE_WRITTEN_AS_A_NUMBER: &str = "4.5";

/// A resistance of nothing at all, stated rather than left out.
///
/// The value whose acceptance a floor written `> 0` rather than `>= 0` would
/// take away, and the one every declaration written before this field existed
/// means by its silence.
const A_RESISTANCE_OF_NOTHING: &str = "0";

/// A resistance far past any speed the engine moves anything at.
///
/// Still a finite number with a well-defined answer, so it registers rather than
/// being clamped to a ceiling nobody can derive. This is the half of "that exact
/// value" that can fail: `0` is satisfied by a loader that never read the field.
const A_RESISTANCE_BEYOND_ANY_SCALE: &str = "1e30";

/// Zero written with a sign in front of it, which Luau retains.
const A_RESISTANCE_OF_SIGNED_NOTHING: &str = "-0.0";

/// What a declaration said about the medium its volume is, beside the solidity
/// the two of them are deliberately not derived from.
///
/// A record rather than four readings, so one comparison reports all four at
/// once and a loader that resolved three of them correctly is not mistaken for
/// one that resolved them all correctly. Solidity travels in every comparison for
/// the reason it travels in `luau_declaration_properties.rs`: a loader that
/// answered the three medium questions *from the wrong field* has to have
/// somewhere to be wrong.
#[derive(Debug, PartialEq)]
struct Medium {
    solid: bool,
    swimmable: bool,
    move_resistance: f32,
    swim_ascent: f32,
}

/// What a declaration's resistance was retained as, by its bits, beside the
/// swimmability stated with it.
///
/// **Bits rather than the value**, because the one thing this reading exists to
/// see — `-0.0` retained where `0.0` was meant — is invisible to `==`. The
/// swimmability travels with it so that the comparison is not satisfied by a
/// loader which read no field at all: `false` beside a zero is what a loader that
/// never looked answers, and every fixture reading this states `true`. Both
/// numbers a declaration may state are carried, because each is folded into a
/// save by its bits and each therefore has its own signed zero to normalise —
/// and because a normalisation applied at one call site and not at the other is
/// exactly what a reading of one of them cannot see.
#[derive(Debug, PartialEq, Eq)]
struct Retained {
    swimmable: bool,
    resistance_bits: u32,
    ascent_bits: u32,
}

/// The two required fields every fixture here states, plus the solidity it is
/// about.
fn declared_solid(solid: bool) -> Vec<String> {
    vec![
        text_field("name", AMBER),
        text_field("texture", QUARTZ),
        raw_field("solid", if solid { "true" } else { "false" }),
    ]
}

/// Those fields followed by `extra`.
fn declared_solid_and(solid: bool, extra: &[String]) -> Vec<String> {
    let mut fields = declared_solid(solid);
    fields.extend_from_slice(extra);
    fields
}

/// A root holding one declaration file, written from `fields`.
fn root_declaring(directory: &TempDir, fields: &[String]) -> Result<PathBuf, Box<dyn Error>> {
    content_root(directory, &[(AMBER_FILE, declaration_of(fields))])
}

/// A root whose one non-solid declaration states `move_resistance` as `stated`.
///
/// Non-solid throughout, per the spec's standing fixture rule: a resistant volume
/// is only ever measured by something moving through it, and a solid one stops
/// what would have moved.
fn root_resisting(directory: &TempDir, stated: &str) -> Result<PathBuf, Box<dyn Error>> {
    root_declaring(
        directory,
        &declared_solid_and(false, &[raw_field(MOVE_RESISTANCE_FIELD, stated)]),
    )
}

/// A root whose one non-solid, swimmable declaration states `swim_ascent` as
/// `stated`.
///
/// Swimmable because a volume that lifts a swimmer is a volume somebody can be
/// in, and non-solid for the reason [`root_resisting`] is: a solid fixture is
/// never overlapped, so a test over one measures collision and reports a clean
/// pass.
fn root_lifting(directory: &TempDir, stated: &str) -> Result<PathBuf, Box<dyn Error>> {
    root_declaring(
        directory,
        &declared_solid_and(
            false,
            &[
                raw_field(SWIMMABLE_FIELD, "true"),
                raw_field(SWIM_ASCENT_FIELD, stated),
            ],
        ),
    )
}

/// What the definition `registry` holds for [`AMBER`] says about its medium.
fn medium_of(registry: &BlockRegistry, name: &str) -> Result<Medium, Box<dyn Error>> {
    let definition = registry.resolve(&BlockName::parse(name)?)?;
    Ok(Medium {
        solid: definition.is_solid,
        swimmable: definition.swimmable,
        move_resistance: definition.move_resistance,
        swim_ascent: definition.swim_ascent,
    })
}

/// What the content root at `root` registered for [`AMBER`], or the refusal that
/// stopped it, rendered.
///
/// Total rather than fallible; see the module header.
fn medium_or_refusal(root: &Path) -> Result<Medium, String> {
    let registry = registry_from(root).map_err(|refused| refused.to_string())?;
    medium_of(&registry, AMBER).map_err(|missing| missing.to_string())
}

/// What the content root at `root` retained for [`AMBER`] by its bits, or the
/// refusal that stopped it.
fn retained_or_refusal(root: &Path) -> Result<Retained, String> {
    let registry = registry_from(root).map_err(|refused| refused.to_string())?;
    let name = BlockName::parse(AMBER).map_err(|broken| broken.to_string())?;
    let definition = registry
        .resolve(&name)
        .map_err(|missing| missing.to_string())?;
    Ok(Retained {
        swimmable: definition.swimmable,
        resistance_bits: definition.move_resistance.to_bits(),
        ascent_bits: definition.swim_ascent.to_bits(),
    })
}

#[test]
fn a_resistance_written_as_a_whole_number_registers_as_that_number() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_resisting(&directory, A_RESISTANCE_WRITTEN_AS_AN_INTEGER)?;

    assert_eq!(
        medium_or_refusal(&root),
        Ok(Medium {
            solid: false,
            swimmable: false,
            move_resistance: 4.0,
            swim_ascent: AN_UNSTATED_ASCENT,
        }),
        "a mod author writing a resistance of four writes `4`, and the host carries that as an \
         integer rather than as a number — a separate variant from the one `4.5` arrives in. A \
         reader that accepted only the fractional variant would refuse half the values the \
         modding guide shows, and refuse them for a reason the author cannot read off their \
         own line. The swimmability travels with it because the two fields are independent by \
         declaration: resisting movement is not floating in it"
    );
    Ok(())
}

#[test]
fn a_resistance_written_as_a_fraction_registers_as_that_number() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_resisting(&directory, A_RESISTANCE_WRITTEN_AS_A_NUMBER)?;

    assert_eq!(
        medium_or_refusal(&root),
        Ok(Medium {
            solid: false,
            swimmable: false,
            move_resistance: 4.5,
            swim_ascent: AN_UNSTATED_ASCENT,
        }),
        "the other variant, and the other half of the same sentence in the guide. `4.5` reaches \
         the host as a number where `4` reaches it as an integer, so the two are distinct paths \
         through the reader rather than one value restated — and a reader handling only this \
         one is the mirror of the failure next door"
    );
    Ok(())
}

#[test]
fn a_solid_block_that_states_neither_medium_field_is_neither_buoyant_nor_resistant() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(&directory, &declared_solid(true))?;

    assert_eq!(
        medium_or_refusal(&root),
        Ok(Medium {
            solid: true,
            swimmable: false,
            move_resistance: 0.0,
            swim_ascent: AN_UNSTATED_ASCENT,
        }),
        "**absence means a constant, never solidity**, and this is the one fixture in the file \
         that can see the difference. Three fields on this declaration already default to \
         whatever it says about `solid`, because one bit used to answer their questions; \
         nothing has ever answered these two, so a derived default here would invent a claim \
         no author made — and it would make every solid block in existence swimmable, which is \
         a stone wall a player can float inside. A non-solid fixture agrees with both readings \
         and cannot report this"
    );
    Ok(())
}

#[test]
fn a_swimmable_block_that_states_no_resistance_resists_nothing() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &declared_solid_and(false, &[raw_field(SWIMMABLE_FIELD, "true")]),
    )?;

    assert_eq!(
        medium_or_refusal(&root),
        Ok(Medium {
            solid: false,
            swimmable: true,
            move_resistance: 0.0,
            swim_ascent: AN_UNSTATED_ASCENT,
        }),
        "the second direction the absence has to be a constant in: a declaration that says a \
         player can swim in it has said **nothing** about how much it slows them, and a loader \
         deriving one of the two from the other would hand a mod author one decision where they \
         made two. Water declares both because water is both; a still pool of something \
         weightless is a declaration this must not refuse to express"
    );
    Ok(())
}

#[test]
fn a_resistance_of_zero_registers_rather_than_being_refused_as_the_silence_it_resembles()
-> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &declared_solid_and(
            false,
            &[
                raw_field(SWIMMABLE_FIELD, "true"),
                raw_field(MOVE_RESISTANCE_FIELD, A_RESISTANCE_OF_NOTHING),
            ],
        ),
    )?;

    assert_eq!(
        medium_or_refusal(&root),
        Ok(Medium {
            solid: false,
            swimmable: true,
            move_resistance: 0.0,
            swim_ascent: AN_UNSTATED_ASCENT,
        }),
        "`0` is exactly `unaffected`, which is what every declaration written before this field \
         existed means by its silence — so a floor written `> 0` rather than `>= 0` refuses the \
         one value the whole scale is stated against. The swimmability is stated alongside \
         because zero is also what a loader that never read the field answers: without it this \
         comparison would be satisfied by a reader that had stopped looking"
    );
    Ok(())
}

#[test]
fn a_resistance_beyond_any_scale_the_engine_moves_at_registers_unclamped() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_resisting(&directory, A_RESISTANCE_BEYOND_ANY_SCALE)?;

    assert_eq!(
        medium_or_refusal(&root),
        Ok(Medium {
            solid: false,
            swimmable: false,
            move_resistance: 1e30,
            swim_ascent: AN_UNSTATED_ASCENT,
        }),
        "the scale is unbounded above on purpose: a block this resistant is effectively \
         unwalkable and is still a finite number with a well-defined answer, so refusing it \
         would need a ceiling nobody can derive and clamping it would silently give a mod \
         author a block other than the one they wrote. This is the half of `that exact value` \
         that can fail — a zero is satisfied by a loader that never read the field, and this is \
         not"
    );
    Ok(())
}

#[test]
fn a_resistance_of_signed_zero_is_retained_as_the_unsigned_zero_a_save_folds() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &declared_solid_and(
            false,
            &[
                raw_field(SWIMMABLE_FIELD, "true"),
                raw_field(MOVE_RESISTANCE_FIELD, A_RESISTANCE_OF_SIGNED_NOTHING),
            ],
        ),
    )?;

    assert_eq!(
        retained_or_refusal(&root),
        Ok(Retained {
            swimmable: true,
            resistance_bits: 0.0_f32.to_bits(),
            ascent_bits: AN_UNSTATED_ASCENT.to_bits(),
        }),
        "a save folds this number **by its bits**, and `-0.0` and `0.0` have different ones — so \
         two declarations meaning the same thing would hash differently and every player \
         holding either block would be told on their next launch that it no longer behaves as \
         it did when their world was saved. `==` cannot see this: `-0.0 == 0.0` is true, which \
         is why the comparison is over bits and why it needs the swimmability beside it to stop \
         a loader that read nothing satisfying it"
    );
    Ok(())
}

#[test]
fn an_ascent_written_as_a_fraction_registers_as_that_number() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_lifting(&directory, AN_ASCENT_WRITTEN_AS_A_NUMBER)?;

    assert_eq!(
        medium_or_refusal(&root),
        Ok(Medium {
            solid: false,
            swimmable: true,
            move_resistance: 0.0,
            swim_ascent: 3.5,
        }),
        "the number a mod author writes to say how fast their liquid carries a swimmer upward, \
         and the whole of what they can predict their block's feel from. It is deliberately \
         **slower** than what an absent field means: a loader that added this key to the list \
         it recognises and then never read it answers the jump speed, which is a value no \
         fixture stating another number can be satisfied by. The resistance is stated beside it \
         because the two are independent declarations — how fast a volume lifts is not how much \
         it slows"
    );
    Ok(())
}

#[test]
fn an_ascent_written_as_a_whole_number_registers_as_that_number() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_lifting(&directory, AN_ASCENT_WRITTEN_AS_AN_INTEGER)?;

    assert_eq!(
        medium_or_refusal(&root),
        Ok(Medium {
            solid: false,
            swimmable: true,
            move_resistance: 0.0,
            swim_ascent: 4.0,
        }),
        "the other variant, and a second distinct path through the reader rather than the same \
         value restated: Luau carries `4` as an integer and `3.5` as a number, and the host \
         hands the two to the loader as separate `ScriptValue` cases. It is the same pair the \
         resistance already states, and they share a reader — so a reader written fresh for this \
         field that handles only one of them refuses half the values the modding guide shows, \
         and refuses them for a reason its author cannot read off their own line"
    );
    Ok(())
}

#[test]
fn an_ascent_of_zero_registers_rather_than_meaning_the_field_was_never_written() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_lifting(&directory, AN_ASCENT_OF_NOTHING)?;

    assert_eq!(
        medium_or_refusal(&root),
        Ok(Medium {
            solid: false,
            swimmable: true,
            move_resistance: 0.0,
            swim_ascent: 0.0,
        }),
        "**this is the field where a stated zero and a silence mean opposite things**, which is \
         what makes it unlike the two beside it. An absent `swimmable` is `false` and an absent \
         `move_resistance` is `0.0`, so for both of those a declaration saying nothing and a \
         declaration saying the smallest thing agree. Here silence means the jump speed — a \
         swimmer flung upward — and `0` means a volume that holds a swimmer up and lifts them \
         nowhere, which is the still pool a mod author writes on purpose. A loader folding this \
         zero into its default hands them the opposite block and no line to read"
    );
    Ok(())
}

#[test]
fn an_ascent_stated_without_a_swimmability_registers_and_holds_nobody_up() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_declaring(
        &directory,
        &declared_solid_and(
            false,
            &[raw_field(SWIM_ASCENT_FIELD, AN_ASCENT_WRITTEN_AS_A_NUMBER)],
        ),
    )?;

    assert_eq!(
        medium_or_refusal(&root),
        Ok(Medium {
            solid: false,
            swimmable: false,
            move_resistance: 0.0,
            swim_ascent: 3.5,
        }),
        "the two are independent **in both directions**, and this is the direction a loader is \
         tempted to collapse: resolving the ascent to zero wherever nothing said the volume was \
         swimmable looks strictly simpler and deletes a rule elsewhere. What it costs is the mod \
         author's own number, permanently — the registry reports a block they did not write, a \
         save folds the wrong value, and a later declaration adding `swimmable = true` finds its \
         ascent was thrown away by the load that read it. Whether a volume holding nobody up \
         lifts anybody is a question about a medium, answered where a definition becomes one and \
         never where it is read"
    );
    Ok(())
}

#[test]
fn an_ascent_of_signed_zero_is_retained_as_the_unsigned_zero_a_save_folds() -> TestResult {
    let directory = TempDir::new()?;
    let root = root_lifting(&directory, AN_ASCENT_OF_SIGNED_NOTHING)?;

    assert_eq!(
        retained_or_refusal(&root),
        Ok(Retained {
            swimmable: true,
            resistance_bits: 0.0_f32.to_bits(),
            ascent_bits: 0.0_f32.to_bits(),
        }),
        "the same normalisation the resistance already owes, at the second call site that owes \
         it — and a reading of the first cannot see the second. A save folds this number **by \
         its bits**, so a `-0.0` retained where `0.0` was meant tells every player holding the \
         block that it no longer behaves as it did when their world was saved, over a \
         declaration nobody changed. `==` is blind to it, which is why the comparison is over \
         bits; the swimmability travels with it so a loader that read nothing cannot satisfy it"
    );
    Ok(())
}
