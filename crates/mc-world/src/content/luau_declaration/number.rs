//! What a declaration may state as a **number**, and everything one can get
//! wrong.
//!
//! `move_resistance` is the first number a declaration may state, so the four
//! things that can be wrong with one are settled here rather than inside the
//! field that happens to be first to want them: a value of the wrong kind, a
//! value below the floor, a value that is not a finite number at all, and a
//! value too large for the width the engine keeps. Every later number on a
//! declaration inherits this vocabulary, which is why it is a module and not
//! four lines inside a field's reader.
//!
//! # A child of [`super`] rather than a module beside it
//!
//! For the reason [`super::texture`] is one: every refusal here is one of the
//! parent's [`FieldFault`]s, which a child may reach and a sibling could not
//! without widening that type's visibility to the whole of `content`. The
//! refusals a mod author reads are one vocabulary, and a second fault type here
//! would be a second place for the modding guide and the program to disagree.

use mc_script::ScriptValue;

use super::FieldFault;

/// A field a declaration may leave out, which has to be a number the engine can
/// divide by whenever it is stated.
///
/// **The loader's first numeric reader**, so what it refuses and the words it
/// refuses in are the vocabulary every later number on a declaration inherits.
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
