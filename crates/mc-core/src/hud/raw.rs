//! A HUD declaration exactly as a source spelled it, and the checking that turns
//! one into an element.
//!
//! Every refusal names the origin, the element and the field, because those
//! three are what whoever wrote the declaration needs in order to fix it. The
//! name is read before anything else is checked, so a declaration whose `size`
//! holds a string is still refused *by name* — a checker that failed on the
//! type first would have thrown the name away before anyone could ask for it.

use std::fmt;

use super::declared::DeclaredValue;
use super::element::{
    ACCEPTED_FIELDS, ANCHOR_FIELD, ANCHOR_NAMES, Anchor, COLOR_DIGITS, COLOR_FIELD, DRAW_FIELD,
    DRAW_KINDS, Draw, DrawKind, HudElement, HudOrigin, NAME_FIELD, OFFSET_FIELD,
    OFFSET_WHEN_ABSENT, OUTLINE_FIELD, READABLE_VALUES, ReadableValue, Rgba8, SIZE_FIELD,
    SOURCE_FIELD,
};
use super::source::HudFault;
use crate::id::HudElementName;

/// What `offset` and `size` have to be, in the words a refusal quotes back.
const PAIR_OF_NUMBERS: &str = "a pair of whole numbers";

/// A HUD declaration as read, before anything about it has been checked.
///
/// A key-and-value list rather than a struct of named options, because this
/// crate cannot name a serialization format and so cannot lean on one to refuse
/// unknown fields. The accepted key set is
/// [`ACCEPTED_FIELDS`](super::ACCEPTED_FIELDS), and a key outside it is refused
/// naming that key — a silently ignored typo is a debugging trap for whoever
/// wrote the declaration.
#[derive(Debug, Clone)]
pub struct RawHudElement {
    fields: Vec<(String, DeclaredValue)>,
}

impl RawHudElement {
    /// A declaration whose keys and values are `fields`, in the order the
    /// source spelled them.
    pub fn new(fields: Vec<(String, DeclaredValue)>) -> Self {
        Self { fields }
    }

    /// Checks the declaration and turns it into an element attributed to
    /// `origin`.
    ///
    /// # Errors
    ///
    /// Returns a [`HudFault`] naming `origin`, the element as it named itself,
    /// and the field at fault, if a key is not accepted, a required field is
    /// missing, a field holds the wrong kind of value, or a value is outside
    /// what the model accepts.
    pub fn into_element(self, origin: &HudOrigin) -> Result<HudElement, HudFault> {
        let declared = self.declared_name();
        self.check(origin).map_err(|fault| HudFault {
            origin: origin.clone(),
            element: declared,
            field: Some(fault.field),
            cause: fault.cause,
        })
    }

    /// The name this declaration gives itself, as written.
    ///
    /// Read before anything is checked, so that a refusal can still say which
    /// element it is about — the alternative is telling a content author only
    /// that something in the file is wrong.
    fn declared_name(&self) -> Option<String> {
        match self.stated(NAME_FIELD) {
            Some(DeclaredValue::Text(spelled)) => Some(spelled.clone()),
            _ => None,
        }
    }

    /// What the declaration says about `field`, or nothing where it is silent.
    fn stated(&self, field: &str) -> Option<&DeclaredValue> {
        self.fields
            .iter()
            .find(|(key, _)| key == field)
            .map(|(_, value)| value)
    }

    fn check(&self, origin: &HudOrigin) -> Result<HudElement, FieldFault> {
        self.refuse_unaccepted_field()?;
        let spelled = required_text(self.stated(NAME_FIELD), NAME_FIELD)?;
        Ok(HudElement {
            name: HudElementName::parse(&spelled)
                .map_err(|error| FieldFault::invalid(NAME_FIELD, error))?,
            anchor: required_anchor(self.stated(ANCHOR_FIELD))?,
            offset: optional_offset(self.stated(OFFSET_FIELD))?,
            size: required_size(self.stated(SIZE_FIELD))?,
            draw: self.check_draw()?,
            outline: optional_color(self.stated(OUTLINE_FIELD), OUTLINE_FIELD)?,
            origin: origin.clone(),
        })
    }

    /// Refuses the first key nobody accepted, naming it.
    fn refuse_unaccepted_field(&self) -> Result<(), FieldFault> {
        let unaccepted = self
            .fields
            .iter()
            .find(|(key, _)| !ACCEPTED_FIELDS.contains(&key.as_str()));
        match unaccepted {
            Some((key, _)) => Err(FieldFault::unaccepted(key)),
            None => Ok(()),
        }
    }

    /// The draw kind and the one field it needs, with the field it cannot use
    /// refused rather than ignored.
    ///
    /// A declaration stating `source` beside `draw = "fill"` believes something
    /// is reading it, and registering the element anyway ships that belief.
    fn check_draw(&self) -> Result<Draw, FieldFault> {
        let spelled = required_text(self.stated(DRAW_FIELD), DRAW_FIELD)?;
        let kind = DrawKind::parse(&spelled)
            .ok_or_else(|| FieldFault::unpublished(DRAW_FIELD, &spelled, &DRAW_KINDS))?;
        match kind {
            DrawKind::Fill => {
                refuse_if_stated(self.stated(SOURCE_FIELD), SOURCE_FIELD, kind)?;
                let color = required_color(self.stated(COLOR_FIELD), COLOR_FIELD)?;
                Ok(Draw::Fill { color })
            }
            DrawKind::BlockTexture => {
                refuse_if_stated(self.stated(COLOR_FIELD), COLOR_FIELD, kind)?;
                let source = required_source(self.stated(SOURCE_FIELD))?;
                Ok(Draw::BlockTexture { source })
            }
        }
    }
}

/// One field that is wrong, before it is known which element or origin it
/// belongs to. Kept separate so that every check reads as a plain question
/// about a value and the attribution is written once.
#[derive(Debug)]
struct FieldFault {
    field: String,
    cause: String,
}

impl FieldFault {
    /// A field that is present and of the right kind, but whose value is not
    /// acceptable.
    fn invalid(field: &str, cause: impl fmt::Display) -> Self {
        Self {
            field: field.to_owned(),
            cause: cause.to_string(),
        }
    }

    /// A required field that was not declared at all.
    fn missing(field: &str) -> Self {
        Self {
            field: field.to_owned(),
            cause: format!("`{field}` is required and was not declared"),
        }
    }

    /// A field holding something other than the kind of value it is declared
    /// in.
    fn wrong_kind(field: &str, found: &DeclaredValue, expected: &str) -> Self {
        Self {
            field: field.to_owned(),
            cause: format!("`{field}` must be {expected}, but is {}", found.kind()),
        }
    }

    /// A key nobody declared acceptable.
    fn unaccepted(field: &str) -> Self {
        Self {
            field: field.to_owned(),
            cause: format!("`{field}` is not a field a HUD element declares"),
        }
    }

    /// A spelling outside a published vocabulary, refused by offering that
    /// whole vocabulary.
    ///
    /// The offering is built from the published set itself, so the message
    /// cannot drift away from what the model accepts.
    fn unpublished(field: &str, spelled: &str, published: &[&str]) -> Self {
        let offered = published.join("`, `");
        Self {
            field: field.to_owned(),
            cause: format!("`{field}` must be one of `{offered}`, but is `{spelled}`"),
        }
    }

    /// A field that the declared draw kind cannot read.
    fn without_effect(field: &str, kind: DrawKind) -> Self {
        Self {
            field: field.to_owned(),
            cause: format!(
                "`{field}` has no effect on a `{}` element and must not be declared",
                kind.as_str()
            ),
        }
    }
}

fn required_text(declared: Option<&DeclaredValue>, field: &str) -> Result<String, FieldFault> {
    let value = declared.ok_or_else(|| FieldFault::missing(field))?;
    match value {
        DeclaredValue::Text(spelled) => Ok(spelled.clone()),
        _ => Err(FieldFault::wrong_kind(field, value, "a string")),
    }
}

/// The two whole numbers a declaration writes an offset or a pair of extents
/// as.
fn required_pair(declared: Option<&DeclaredValue>, field: &str) -> Result<[i64; 2], FieldFault> {
    let value = declared.ok_or_else(|| FieldFault::missing(field))?;
    let stated = match value {
        DeclaredValue::List(stated) => stated,
        _ => return Err(FieldFault::wrong_kind(field, value, PAIR_OF_NUMBERS)),
    };
    match stated.as_slice() {
        [DeclaredValue::Integer(across), DeclaredValue::Integer(down)] => Ok([*across, *down]),
        _ => Err(FieldFault::wrong_kind(field, value, PAIR_OF_NUMBERS)),
    }
}

fn required_size(declared: Option<&DeclaredValue>) -> Result<[u32; 2], FieldFault> {
    let [across, down] = required_pair(declared, SIZE_FIELD)?;
    match (positive_extent(across), positive_extent(down)) {
        (Some(across), Some(down)) => Ok([across, down]),
        _ => Err(FieldFault::invalid(
            SIZE_FIELD,
            format!(
                "`{SIZE_FIELD}` states an extent of `{across}` by `{down}`, and both have to be \
                 strictly positive"
            ),
        )),
    }
}

fn positive_extent(number: i64) -> Option<u32> {
    u32::try_from(number).ok().filter(|extent| *extent > 0)
}

/// The displacement a declaration states, or no displacement where it is
/// silent.
fn optional_offset(declared: Option<&DeclaredValue>) -> Result<[i32; 2], FieldFault> {
    if declared.is_none() {
        return Ok(OFFSET_WHEN_ABSENT);
    }
    let [across, down] = required_pair(declared, OFFSET_FIELD)?;
    match (i32::try_from(across), i32::try_from(down)) {
        (Ok(across), Ok(down)) => Ok([across, down]),
        _ => Err(FieldFault::invalid(
            OFFSET_FIELD,
            format!(
                "`{OFFSET_FIELD}` states a displacement of `{across}` by `{down}`, which is \
                 further than a screen reaches"
            ),
        )),
    }
}

fn required_anchor(declared: Option<&DeclaredValue>) -> Result<Anchor, FieldFault> {
    let spelled = required_text(declared, ANCHOR_FIELD)?;
    Anchor::parse(&spelled)
        .ok_or_else(|| FieldFault::unpublished(ANCHOR_FIELD, &spelled, &ANCHOR_NAMES))
}

fn required_source(declared: Option<&DeclaredValue>) -> Result<ReadableValue, FieldFault> {
    let spelled = required_text(declared, SOURCE_FIELD)?;
    ReadableValue::parse(&spelled)
        .ok_or_else(|| FieldFault::unpublished(SOURCE_FIELD, &spelled, &READABLE_VALUES))
}

fn required_color(declared: Option<&DeclaredValue>, field: &str) -> Result<Rgba8, FieldFault> {
    let spelled = required_text(declared, field)?;
    Rgba8::parse(&spelled).ok_or_else(|| {
        FieldFault::invalid(
            field,
            format!(
                "`{field}` must be `#RRGGBBAA` with {COLOR_DIGITS} hex digits and no shorthand, \
                 but is `{spelled}`"
            ),
        )
    })
}

/// The colour a declaration states, or none where it is silent — but still a
/// fault where it states one wrongly.
///
/// "The author did not declare this" and "the author declared it wrongly" are
/// different mistakes, and collapsing the second into the first is how an
/// outline written in shorthand becomes an element with no outline at all.
fn optional_color(
    declared: Option<&DeclaredValue>,
    field: &str,
) -> Result<Option<Rgba8>, FieldFault> {
    match declared {
        Some(_) => required_color(declared, field).map(Some),
        None => Ok(None),
    }
}

fn refuse_if_stated(
    declared: Option<&DeclaredValue>,
    field: &str,
    kind: DrawKind,
) -> Result<(), FieldFault> {
    match declared {
        Some(_) => Err(FieldFault::without_effect(field, kind)),
        None => Ok(()),
    }
}
