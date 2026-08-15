//! Everything decided before a single grid is read.
//!
//! This is D9's first two passes. The ordering is a contract rather than an
//! implementation detail: a part declaring 65 voxels on an axis is refused
//! *without its layer art needing to exist*, so a document whose extent is wrong
//! never reaches the grid reader and never earns a second, misleading complaint
//! about the art it does not have.

use std::collections::BTreeMap;

use glam::IVec3;
use toml::Value;

use crate::fault::{Fault, Origin};
use crate::format::dto::{DocumentDto, PartDto, from_value};
use crate::format::{Attach, Axis, Extent, Part, PartName, StateName};
use crate::name::ModelName;

/// The only document revision this tool understands.
const SUPPORTED_SCHEMA: i64 = 1;

/// The most voxels a part may reach on any one axis.
const MAX_EXTENT: i64 = 64;

/// The fewest voxels a part may reach on any one axis.
const MIN_EXTENT: i64 = 1;

/// Checks that the document claims a revision this tool understands.
///
/// # Errors
///
/// Returns a [`Fault`] naming `schema` when it is absent, unreadable, or newer
/// than [`SUPPORTED_SCHEMA`].
pub fn check_schema(document: &DocumentDto, origin: &Origin) -> Result<(), Fault> {
    let declared = document
        .schema
        .as_ref()
        .and_then(Value::as_integer)
        .ok_or_else(|| {
            field_refusal(
                origin,
                "schema",
                format!(
                    "a document must declare `schema`, and this one declares none — it is required, and the only supported value is {SUPPORTED_SCHEMA}"
                ),
            )
        })?;
    if declared == SUPPORTED_SCHEMA {
        return Ok(());
    }
    Err(field_refusal(
        origin,
        "schema",
        format!(
            "this document declares schema {declared}, but the highest supported schema is {SUPPORTED_SCHEMA}"
        ),
    ))
}

/// The namespaced name the document declares.
///
/// # Errors
///
/// Returns a [`Fault`] naming `name` when it is absent or does not satisfy the
/// namespaced id rule.
pub fn read_name(document: &DocumentDto, origin: &Origin) -> Result<ModelName, Fault> {
    let declared = document
        .name
        .as_ref()
        .and_then(Value::as_str)
        .ok_or_else(|| {
            field_refusal(
                origin,
                "name",
                "a document must declare a namespaced `name`, written `namespace:path`",
            )
        })?;
    ModelName::parse(declared).map_err(|cause| field_refusal(origin, "name", cause.to_string()))
}

/// How many voxels span one block edge.
///
/// # Errors
///
/// Returns a [`Fault`] naming `scale` when it is absent or below 1, quoting the
/// value exactly as it was written.
pub fn read_scale(document: &DocumentDto, origin: &Origin) -> Result<u32, Fault> {
    let declared = document
        .scale
        .as_ref()
        .and_then(Value::as_integer)
        .ok_or_else(|| {
            field_refusal(
                origin,
                "scale",
                "a document must declare a `scale`, the number of voxels spanning one block edge",
            )
        })?;
    u32::try_from(declared)
        .ok()
        .filter(|scale| *scale >= 1)
        .ok_or_else(|| {
            field_refusal(
                origin,
                "scale",
                format!("`scale` must be at least 1, but is {declared}"),
            )
        })
}

/// The axis a document, or one of its parts, slices along.
///
/// # Errors
///
/// Returns a [`Fault`] naming `slice` and the three accepted axes when the
/// declared value is not one of them.
pub fn read_axis(declared: Option<&Value>, origin: &Origin) -> Result<Axis, Fault> {
    let text = declared.and_then(Value::as_str).ok_or_else(|| {
        field_refusal(
            origin,
            "slice",
            "a document must declare a `slice` axis, one of `x`, `y` or `z`",
        )
    })?;
    match text {
        "x" => Ok(Axis::X),
        "y" => Ok(Axis::Y),
        "z" => Ok(Axis::Z),
        other => Err(field_refusal(
            origin,
            "slice",
            format!("`{other}` is not an axis — a slice axis is one of `x`, `y` or `z`"),
        )),
    }
}

/// The parts the document declares, in declaration order.
///
/// The implicit single-part form and the explicit `[[parts]]` form are the only
/// two, and a document uses exactly one of them.
///
/// # Errors
///
/// Returns a [`Fault`] naming both forms when the document declares both or
/// neither, or naming the offending part when one is not acceptable.
pub fn read_parts(
    document: &DocumentDto,
    model_slice: Axis,
    origin: &Origin,
) -> Result<Vec<Part>, Fault> {
    match (&document.size, &document.parts) {
        (Some(_), Some(_)) => Err(Fault::about(
            origin.clone(),
            "a document declares its geometry either as a top-level `size` or as `[[parts]]` tables, never both — this one declares both",
        )),
        (None, None) => Err(Fault::about(
            origin.clone(),
            "a document declares its geometry either as a top-level `size` or as `[[parts]]` tables, and this one declares neither",
        )),
        (Some(size), None) => Ok(vec![implicit_part(document, size, model_slice, origin)?]),
        (None, Some(parts)) => explicit_parts(parts, model_slice, origin),
    }
}

/// The one part an implicit-form document describes.
fn implicit_part(
    document: &DocumentDto,
    size: &Value,
    model_slice: Axis,
    origin: &Origin,
) -> Result<Part, Fault> {
    let name = PartName::new(implicit_part_name(document));
    Ok(Part {
        name,
        size: read_extent(size, origin)?,
        origin: read_pivot(document.origin.as_ref(), origin)?,
        slice: model_slice,
        attach: None,
        states: Vec::new(),
        layers: BTreeMap::new(),
    })
}

/// What to call the part an implicit-form document never named.
///
/// The document's own path, so a refusal about the part reads as being about the
/// thing the author wrote rather than about a placeholder they never saw.
fn implicit_part_name(document: &DocumentDto) -> String {
    document
        .name
        .as_ref()
        .and_then(Value::as_str)
        .and_then(|name| name.split_once(':'))
        .map_or_else(|| "model".to_owned(), |(_, path)| path.to_owned())
}

/// Every part an explicit-form document declares.
fn explicit_parts(parts: &Value, model_slice: Axis, origin: &Origin) -> Result<Vec<Part>, Fault> {
    let declared = parts.as_array().ok_or_else(|| {
        Fault::about(
            origin.clone(),
            "`parts` is declared as `[[parts]]` tables, one per part",
        )
    })?;
    declared
        .iter()
        .map(|part| explicit_part(part, model_slice, origin))
        .collect()
}

/// One part of an explicit-form document.
///
/// The part's name is read straight out of the raw table *before* the table is
/// typed, so that a refusal about an unrecognised field can say which part
/// carried it — which is the one thing typing it first would throw away.
fn explicit_part(declared: &Value, model_slice: Axis, origin: &Origin) -> Result<Part, Fault> {
    let spelled = declared
        .as_table()
        .and_then(|table| table.get("name"))
        .and_then(Value::as_str);
    let attribute = |fault: Fault| match spelled {
        Some(name) => fault.in_part(name),
        None => fault,
    };
    let part: PartDto = from_value(declared.clone(), origin).map_err(attribute)?;
    declared_part(&part, model_slice, origin).map_err(attribute)
}

/// The part a typed `[[parts]]` table describes.
fn declared_part(part: &PartDto, model_slice: Axis, origin: &Origin) -> Result<Part, Fault> {
    let name = part.name.as_ref().and_then(Value::as_str).ok_or_else(|| {
        Fault::about(origin.clone(), "every `[[parts]]` table declares a `name`").in_field("name")
    })?;
    let size = part
        .size
        .as_ref()
        .ok_or_else(|| field_refusal(origin, "size", "every part declares a `size`"))?;
    let slice = part
        .slice
        .as_ref()
        .map_or(Ok(model_slice), |slice| read_axis(Some(slice), origin))?;
    Ok(Part {
        name: PartName::new(name),
        size: read_extent(size, origin)?,
        origin: read_pivot(part.origin.as_ref(), origin)?,
        slice,
        attach: read_attach(part.attach.as_ref(), origin)?,
        states: read_states(part.states.as_ref(), origin)?,
        layers: BTreeMap::new(),
    })
}

/// The extent `declared` describes, bounded on every axis.
///
/// Read as three signed values rather than as three unsigned ones, because a
/// refusal has to quote `-1` back as `-1`: an unsigned parse throws the value
/// away and leaves the author reading about a type they did not write.
fn read_extent(declared: &Value, origin: &Origin) -> Result<Extent, Fault> {
    let axes = declared.as_array().ok_or_else(|| {
        field_refusal(
            origin,
            "size",
            "a `size` is three values, one per axis, written `[x, y, z]`",
        )
    })?;
    let [x, y, z] = triple(axes, "size", origin)?;
    Ok(Extent {
        x: bounded(x, "x", origin)?,
        y: bounded(y, "y", origin)?,
        z: bounded(z, "z", origin)?,
    })
}

/// One axis of an extent, checked against both bounds.
fn bounded(declared: i64, axis: &str, origin: &Origin) -> Result<u32, Fault> {
    if declared < MIN_EXTENT {
        return Err(field_refusal(
            origin,
            "size",
            format!(
                "on axis {axis} the declared size is {declared}, but every extent must be at least {MIN_EXTENT}"
            ),
        ));
    }
    if declared > MAX_EXTENT {
        return Err(field_refusal(
            origin,
            "size",
            format!(
                "on axis {axis} the declared size is {declared}, but the limit is {MAX_EXTENT} voxels"
            ),
        ));
    }
    u32::try_from(declared).map_err(|cause| field_refusal(origin, "size", cause.to_string()))
}

/// The pivot `declared` describes, or the origin when none is declared.
fn read_pivot(declared: Option<&Value>, origin: &Origin) -> Result<IVec3, Fault> {
    let Some(value) = declared else {
        return Ok(IVec3::ZERO);
    };
    let axes = value.as_array().ok_or_else(|| {
        field_refusal(
            origin,
            "origin",
            "an `origin` is three values, one per axis, written `[x, y, z]`",
        )
    })?;
    let [x, y, z] = triple(axes, "origin", origin)?;
    Ok(IVec3::new(
        narrowed(x, "origin", origin)?,
        narrowed(y, "origin", origin)?,
        narrowed(z, "origin", origin)?,
    ))
}

/// Where a part hangs off its parent, when it declares an attachment.
fn read_attach(declared: Option<&Value>, origin: &Origin) -> Result<Option<Attach>, Fault> {
    let Some(value) = declared else {
        return Ok(None);
    };
    let table = value.as_table().ok_or_else(|| {
        field_refusal(
            origin,
            "attach",
            "an `attach` is written `{ to = \"part\", at = [x, y, z] }`",
        )
    })?;
    let to = table.get("to").and_then(Value::as_str).ok_or_else(|| {
        field_refusal(
            origin,
            "attach",
            "an `attach` names the part it hangs off, as `to`",
        )
    })?;
    Ok(Some(Attach {
        to: PartName::new(to),
        at: attach_position(table.get("at"), origin)?,
    }))
}

/// The parent-local position an attachment lands on.
fn attach_position(declared: Option<&Value>, origin: &Origin) -> Result<IVec3, Fault> {
    let axes = declared.and_then(Value::as_array).ok_or_else(|| {
        field_refusal(
            origin,
            "attach",
            "an `attach` declares where it lands, as `at = [x, y, z]`",
        )
    })?;
    let [x, y, z] = triple(axes, "attach", origin)?;
    Ok(IVec3::new(
        narrowed(x, "attach", origin)?,
        narrowed(y, "attach", origin)?,
        narrowed(z, "attach", origin)?,
    ))
}

/// The states a part declares, in declaration order.
///
/// # Errors
///
/// Returns a [`Fault`] naming `states` when an entry is not a name. Refused
/// rather than skipped: a dropped entry leaves the part looking as though it
/// declared one fewer state, and the layer that legitimately names the missing
/// one then fails with "the part does not declare it" — a true sentence
/// pointing at the wrong line. Every other malformed declaration in this module
/// is refused, and this was the one that was not.
fn read_states(declared: Option<&Value>, origin: &Origin) -> Result<Vec<StateName>, Fault> {
    let Some(states) = declared else {
        return Ok(Vec::new());
    };
    let listed = states.as_array().ok_or_else(|| {
        field_refusal(
            origin,
            "states",
            "`states` is a list of names, written `[\"low\", \"high\"]`",
        )
    })?;
    listed
        .iter()
        .map(|state| {
            let spelled = state.as_str().ok_or_else(|| {
                field_refusal(
                    origin,
                    "states",
                    format!("`{state}` is not a state name — a state is named by a string"),
                )
            })?;
            Ok(StateName::new(spelled))
        })
        .collect()
}

/// The three integers `axes` holds.
fn triple(axes: &[Value], field: &str, origin: &Origin) -> Result<[i64; 3], Fault> {
    let read: Vec<i64> = axes.iter().filter_map(Value::as_integer).collect();
    match read.as_slice() {
        [x, y, z] => Ok([*x, *y, *z]),
        _ => Err(field_refusal(
            origin,
            field,
            format!("`{field}` is three whole numbers, one per axis, written `[x, y, z]`"),
        )),
    }
}

/// One coordinate, narrowed to the width a position is held in.
fn narrowed(declared: i64, field: &str, origin: &Origin) -> Result<i32, Fault> {
    i32::try_from(declared).map_err(|cause| field_refusal(origin, field, cause.to_string()))
}

/// A refusal attributed to one field.
fn field_refusal(origin: &Origin, field: &str, cause: impl Into<String>) -> Fault {
    Fault::about(origin.clone(), cause).in_field(field)
}
