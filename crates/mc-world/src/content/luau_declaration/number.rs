//! What a declaration may state as a **number**, and everything one can get
//! wrong.
//!
//! `move_resistance` is the first number a declaration may state, so the four
//! things that can be wrong with one are settled here rather than inside the
//! field that happens to be first to want them: a value of the wrong kind, a
//! value that is not a finite number at all, a value below the floor, and a
//! value above the ceiling. Every later number on a declaration inherits this
//! vocabulary, which is why it is a module and not four lines inside a field's
//! reader. The fields that read one live here too, beside what their absence
//! means, so that everything a stated number can be is in one file.
//!
//! **The ceiling arrived with `opacity` and the two medium numbers were folded
//! into it rather than left beside it.** Their own bound is the width an `f32`
//! keeps, which nothing can exceed and stay finite, so they read through the
//! bounded reader unchanged — and the modding guide's `3.4e38` becomes a thing
//! the program states rather than a thing the page claims on its behalf.
//!
//! # A child of [`super`] rather than a module beside it
//!
//! For the reason [`super::texture`] is one: every refusal here is one of the
//! parent's [`FieldFault`]s, which a child may reach and a sibling could not
//! without widening that type's visibility to the whole of `content`. The
//! refusals a mod author reads are one vocabulary, and a second fault type here
//! would be a second place for the modding guide and the program to disagree.

use std::ops::RangeInclusive;

use mc_core::block::Opacity;
use mc_script::{ScriptHost, ScriptTable, ScriptValue};

use super::{FieldFault, MOVE_RESISTANCE_FIELD, OPACITY_FIELD, SWIM_ASCENT_FIELD};

/// What a declaration means by saying nothing about resisting movement.
///
/// A constant for the reason [`super::SWIMMABLE_BY_DEFAULT`] is one, and what
/// the scale already spells "unaffected": the tick divides by `1 + resistance`,
/// so a declaration saying nothing divides by one and moves as it always did.
const MOVE_RESISTANCE_BY_DEFAULT: f32 = 0.0;

/// What a declaration means by saying nothing about lifting a swimmer.
///
/// A constant for the reason the one above is, and the same value the player's
/// own jump leaves the ground at: a declaration written before this field
/// existed lifts exactly as it always did.
///
/// **The one loader default that is not also its fold identity.** An empty cell
/// contributes an ascent of `0.0`, because a cell holding nothing lifts nobody;
/// a declaration saying nothing contributes `9.0`. The two are right for their
/// own jobs and disagree, which is why a definition that is not swimmable has
/// its ascent masked away where a definition becomes a medium — without that,
/// an ordinary block sharing a voxel with water would fold its unstated `9.0`
/// over the water's own number.
const SWIM_ASCENT_BY_DEFAULT: f32 = 9.0;

/// What a declaration means by saying nothing about how much light it stops.
///
/// A constant for the reason the two above are, and the value that means "as it
/// always was": every declaration written before this field existed stops all
/// the light, which is the only behaviour the engine had.
const OPACITY_BY_DEFAULT: f32 = 1.0;

/// What a stated number may be, and how each end of that is said in the
/// sentence refusing a value outside it.
///
/// **The ends are carried in words rather than formatted from the numbers**,
/// because a refusal is prose read mid-edit by whoever wrote the line: "may not
/// be more than one" is a sentence and "may not be more than 1" is a
/// diagnostic. It is also what spares the width `move_resistance` is kept at
/// from ever being said out loud.
pub(super) struct Bounds {
    within: RangeInclusive<f32>,
    floor_in_words: &'static str,
    ceiling_in_words: &'static str,
}

impl Bounds {
    /// What the two numbers a medium states may be: not negative, and no wider
    /// than the width the engine keeps.
    ///
    /// **The ceiling is unreachable rather than decorative**, which is what
    /// makes folding these two fields into the bounded reader a no-op: a `f64`
    /// wider than `f32::MAX` narrows to an infinity and is refused for
    /// finiteness first, and no Luau integer reaches that width at all. It is
    /// stated so that there is one reader rather than two, and
    /// `docs/modding/blocks-items.md:80` already documents the bound.
    pub(super) fn at_least_zero() -> Self {
        Self {
            within: 0.0..=f32::MAX,
            floor_in_words: "zero",
            ceiling_in_words: "3.4e38",
        }
    }

    /// What a degree of opacity may be, read off [`Opacity`]'s own two ends so
    /// that the range a declaration is refused against and the range the type
    /// admits cannot drift apart.
    pub(super) fn a_degree() -> Self {
        Self {
            within: Opacity::CLEAR.get()..=Opacity::OPAQUE.get(),
            floor_in_words: "zero",
            ceiling_in_words: "one",
        }
    }
}

/// How much a declaration says its volume slows what moves through it.
///
/// Its absence means a **constant** and not `defaulting_to_solidity`; see
/// [`super::defaulting_to`] for why that distinction is load-bearing.
pub(super) fn declared_resistance(
    host: &ScriptHost,
    declaration: &ScriptTable,
) -> Result<f32, FieldFault> {
    optional_number_within(
        host.read_field(declaration, MOVE_RESISTANCE_FIELD),
        MOVE_RESISTANCE_FIELD,
        Bounds::at_least_zero(),
        MOVE_RESISTANCE_BY_DEFAULT,
    )
}

/// How fast a declaration says its volume lifts a swimmer who asks to rise.
///
/// Read through the same reader [`declared_resistance`] uses, and deliberately
/// adds no vocabulary of its own: the four things that can be wrong with a
/// stated number are settled in one place below, including the `-0.0 → 0.0`
/// normalisation a save's fold depends on. Independent of `swimmable` here — a
/// declaration stating one and not the other is registered as written, and what
/// a volume that holds nobody up does with a declared ascent is decided where a
/// definition becomes a medium.
pub(super) fn declared_ascent(
    host: &ScriptHost,
    declaration: &ScriptTable,
) -> Result<f32, FieldFault> {
    optional_number_within(
        host.read_field(declaration, SWIM_ASCENT_FIELD),
        SWIM_ASCENT_FIELD,
        Bounds::at_least_zero(),
        SWIM_ASCENT_BY_DEFAULT,
    )
}

/// How much of the light reaching a block's volume that block stops.
///
/// Read through the same reader the two medium numbers use, and the first field
/// to give that reader a ceiling anybody can reach. Its absence means
/// [`OPACITY_BY_DEFAULT`]; whether the degree it states contradicts what the
/// same declaration says about hiding its neighbours is asked by
/// [`super::declared_opacity`], once both fields have been read.
pub(super) fn declared_degree(
    host: &ScriptHost,
    declaration: &ScriptTable,
) -> Result<Opacity, FieldFault> {
    let stated = optional_number_within(
        host.read_field(declaration, OPACITY_FIELD),
        OPACITY_FIELD,
        Bounds::a_degree(),
        OPACITY_BY_DEFAULT,
    )?;
    Ok(match Opacity::new(stated) {
        Some(degree) => degree,
        // Unreachable: `Bounds::a_degree` is read off the two ends `new` itself
        // admits, so the branches above have already refused everything it
        // would. Written as a fallback rather than unwrapped for the reason
        // `super::FIELD_NAMES_READ` is — this crate denies panicking
        // conversions, and a definition refused here would be refused for a
        // reason no author could act on.
        None => Opacity::OPAQUE,
    })
}

/// A field a declaration may leave out, which has to be a finite number inside
/// `bounds` whenever it is stated.
///
/// **The loader's only numeric reader**, so what it refuses and the words it
/// refuses in are the vocabulary every number on a declaration is read through.
/// `move_resistance` wanted it first, `swim_ascent` reads through it unchanged
/// and `opacity` is the first to give it a ceiling anybody can reach: a second
/// reader would be a second place for the modding guide and the program to
/// disagree about what a number may be.
/// Four things can be wrong with a stated number and each is a separate branch:
/// the wrong kind of value, a value that is not a finite number at all, a value
/// below the floor, and a value above the ceiling.
///
/// **The bounds travel as one value rather than as two parameters**, because
/// `clippy.toml` caps a function at four and `code-quality.md` §2 names that
/// remedy — and because the words each end is refused in belong beside the
/// number they describe rather than beside the call.
///
/// **Finiteness is asked before the ceiling as well as before the floor.**
/// `infinity > 1.0` is true, so a ceiling reached first refuses `math.huge`
/// with a sentence about `one` and teaches its author that the fix is a smaller
/// number — when what they wrote is not a number and has no smaller spelling.
/// Nothing could report that before opacity: no earlier field had a ceiling
/// anybody could reach.
///
/// **Both numeric kinds are accepted.** Luau writes `4` as an integer and `4.5`
/// as a number and the host carries the two as separate [`ScriptValue`] variants,
/// so a reader taking one of them would refuse half the values the modding guide
/// shows — and refuse them for a reason no author can read off their own line.
///
/// **Narrowed before it is judged.** What the engine keeps is the `f32` a tick
/// divides by, so the questions are asked of that and not of what the script
/// wrote: `1e40` is a perfectly finite Luau number and an infinity at the width
/// it is kept at, and admitting it would hand the physics a resistance no author
/// declared.
///
/// **Finiteness is asked before the floor**, because `NaN >= 0.0` is false: a
/// floor test reached first refuses a NaN with a sentence about zero and sends
/// its author looking for a minus sign they never wrote.
///
/// **`-0.0` is normalised to `0.0`.** A save folds this number by its bits, and
/// the two zeroes have different ones — so two declarations meaning the same
/// thing would hash differently and tell every player holding either block that
/// it no longer behaves as it did. `+ 0.0` is what does it: IEEE-754 addition
/// answers `+0.0` for `-0.0 + 0.0` and leaves every other finite value exactly
/// where it was.
pub(super) fn optional_number_within(
    declared: Option<ScriptValue>,
    field: &str,
    bounds: Bounds,
    absent: f32,
) -> Result<f32, FieldFault> {
    let stated = match declared {
        None => return Ok(absent),
        Some(ScriptValue::Integer(whole)) => whole as f32,
        Some(ScriptValue::Number(fraction)) => fraction as f32,
        Some(found) => return Err(FieldFault::wrong_kind(field, &found, "a number")),
    };
    if !stated.is_finite() {
        return Err(FieldFault::invalid(
            field,
            &format!("`{field}` must be a finite number"),
        ));
    }
    if stated < *bounds.within.start() {
        return Err(FieldFault::invalid(
            field,
            &format!("`{field}` may not be less than {}", bounds.floor_in_words),
        ));
    }
    if stated > *bounds.within.end() {
        return Err(FieldFault::invalid(
            field,
            &format!("`{field}` may not be more than {}", bounds.ceiling_in_words),
        ));
    }
    Ok(stated + 0.0)
}
