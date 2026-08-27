//! What a declaration may state as a **number**, and everything one can get
//! wrong.
//!
//! `move_resistance` is the first number a declaration may state, so the four
//! things that can be wrong with one are settled here rather than inside the
//! field that happens to be first to want them: a value of the wrong kind, a
//! value below the floor, a value that is not a finite number at all, and a
//! value too large for the width the engine keeps. Every later number on a
//! declaration inherits this vocabulary, which is why it is a module and not
//! four lines inside a field's reader. The fields that read one live here too,
//! beside what their absence means, so that everything a stated number can be
//! is in one file.
//!
//! # A child of [`super`] rather than a module beside it
//!
//! For the reason [`super::texture`] is one: every refusal here is one of the
//! parent's [`FieldFault`]s, which a child may reach and a sibling could not
//! without widening that type's visibility to the whole of `content`. The
//! refusals a mod author reads are one vocabulary, and a second fault type here
//! would be a second place for the modding guide and the program to disagree.

use mc_script::{ScriptHost, ScriptTable, ScriptValue};

use super::{FieldFault, MOVE_RESISTANCE_FIELD, SWIM_ASCENT_FIELD};

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

/// How much a declaration says its volume slows what moves through it.
///
/// Its absence means a **constant** and not `defaulting_to_solidity`; see
/// [`super::defaulting_to`] for why that distinction is load-bearing.
pub(super) fn declared_resistance(
    host: &ScriptHost,
    declaration: &ScriptTable,
) -> Result<f32, FieldFault> {
    optional_number_at_least_zero(
        host.read_field(declaration, MOVE_RESISTANCE_FIELD),
        MOVE_RESISTANCE_FIELD,
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
    optional_number_at_least_zero(
        host.read_field(declaration, SWIM_ASCENT_FIELD),
        SWIM_ASCENT_FIELD,
        SWIM_ASCENT_BY_DEFAULT,
    )
}

/// A field a declaration may leave out, which has to be a finite number no less
/// than zero whenever it is stated.
///
/// **The loader's only numeric reader**, so what it refuses and the words it
/// refuses in are the vocabulary every number on a declaration is read through.
/// `move_resistance` wanted it first and `swim_ascent` reads through it
/// unchanged: a second reader would be a second place for the modding guide and
/// the program to disagree about what a number may be.
/// Four things can be wrong with a stated number and each is a separate branch:
/// the wrong kind of value, a value below the floor, a value that is not a finite
/// number at all, and a value too large for the width the engine keeps.
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
pub(super) fn optional_number_at_least_zero(
    declared: Option<ScriptValue>,
    field: &str,
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
    if stated < 0.0 {
        return Err(FieldFault::invalid(
            field,
            &format!("`{field}` may not be less than zero"),
        ));
    }
    Ok(stated + 0.0)
}
