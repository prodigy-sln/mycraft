//! What each shader must say, compared against `validate_tables`' copy of what
//! the CPU writes.
//!
//! Split out of [`super`], which answers a different question: how the sources
//! are found, parsed and validated at the downlevel profile. That half is about
//! naga and a directory; this half is text comparisons over one source against
//! one table, and it grows every time a record does.
//!
//! Every refusal here is one of the parent's [`ShaderError`]s, so the message
//! whoever hits this at the build reads is one vocabulary — the same reason
//! `mc-world`'s declaration loader keeps its field faults in one type.
//!
//! **None of these is evidence that the values are right.** They close a drift
//! between two copies, and this project shipped a table on which all three copies
//! agreed and all three were wrong. What can say a value is right is a reading of
//! a drawn frame.

use super::{
    FRAME_RECORD, PLANE_AXES, QUAD_INDEX_PATTERN, SECTION_RECORD, ShaderError, TERRAIN_SHADER,
    VERTEX_LAYOUT,
};

/// How the winding literal's declaration begins.
///
/// Matched as text rather than evaluated as a constant expression: the value is
/// a literal by construction, and walking naga's constant arena to reach it
/// would be a second, larger thing to get wrong.
const INDEX_PATTERN_DECLARATION: &str = "const QUAD_INDEX_PATTERN";

/// How the plane-axis table's declaration begins.
const PLANE_AXES_DECLARATION: &str = "const PLANE_AXES";

/// How the image-swap table's declaration begins.
pub(super) const IMAGE_SWAPS_DECLARATION: &str = "const IMAGE_SWAPS";

/// How the image-sign table's declaration begins.
pub(super) const IMAGE_SIGNS_DECLARATION: &str = "const IMAGE_SIGNS";

/// How the section record's declaration begins, in both shaders.
const SECTION_RECORD_DECLARATION: &str = "struct Section {";

/// How the per-frame record's declaration begins, in both shaders.
const FRAME_RECORD_DECLARATION: &str = "struct Frame {";

/// The cull shader's winding literal against the geometry builder's.
pub(super) fn check_index_pattern(file: &str, source: &str) -> Result<(), ShaderError> {
    let found = declared_values(source, INDEX_PATTERN_DECLARATION);
    if found == QUAD_INDEX_PATTERN {
        return Ok(());
    }
    Err(ShaderError::IndexPatternMismatch {
        file: file.to_owned(),
        found,
        expected: QUAD_INDEX_PATTERN.to_vec(),
    })
}

/// The terrain shader's plane-axis table against the geometry builder's.
///
/// The shader's copy is one flat list, because a `vec2` constructor per row
/// would put a bracket inside the literal that the reader below would have to
/// understand. The rows are compared flattened for the same reason: what the
/// build has to answer is whether the twelve numbers agree, and reporting them
/// as the shader wrote them is what lets a developer diff the two by eye.
pub(super) fn check_plane_axes(file: &str, source: &str) -> Result<(), ShaderError> {
    let found = declared_values(source, PLANE_AXES_DECLARATION);
    let expected: Vec<u32> = PLANE_AXES.into_iter().flatten().collect();
    if found == expected {
        return Ok(());
    }
    Err(ShaderError::PlaneAxesMismatch {
        file: file.to_owned(),
        found,
        expected,
    })
}

/// One of the terrain shader's image-basis tables against the geometry
/// builder's, named by its `declaration`.
///
/// Flat and flattened for the reasons [`check_plane_axes`] gives. **And the
/// reason these checks exist rather than trust is worth a sentence: none of them
/// is evidence that the values are right.** They close a drift between two
/// copies, and this project shipped a table on which all three copies agreed and
/// all three were wrong. What can say the values are right is a reading of a
/// drawn face — FR-8.1-S7 for where its bands sit, FR-8.1-S8 for which way it
/// runs.
pub(super) fn check_image_basis(
    file: &str,
    source: &str,
    declaration: &str,
    expected: Vec<u32>,
) -> Result<(), ShaderError> {
    let found = declared_values(source, declaration);
    if found == expected {
        return Ok(());
    }
    Err(ShaderError::ImageBasisMismatch {
        file: file.to_owned(),
        found,
        expected,
    })
}

/// Fails unless every field of the packed vertex sits where the geometry builder
/// puts it.
///
/// Each shift and width is a named scalar constant the decode itself reads, so
/// this compares the numbers the shader actually uses rather than a comment
/// beside them — which is what the three-hand-written-copies defect turned on.
pub(super) fn check_vertex_layout(file: &str, source: &str) -> Result<(), ShaderError> {
    for (declaration, expected) in VERTEX_LAYOUT {
        let found = declared_scalar(source, declaration);
        if found != Some(expected) {
            return Err(ShaderError::VertexLayoutMismatch {
                file: file.to_owned(),
                field: declaration.to_owned(),
                found,
                expected,
            });
        }
    }
    Ok(())
}

/// Fails unless the shader's section record is the one the scene writes, field
/// for field and type for type.
///
/// Names as well as types, because the two failures differ: a field renamed is a
/// shader that no longer compiles, but a field **inserted, removed or exchanged
/// with its neighbour** compiles perfectly and reads every later field out of the
/// wrong four bytes. The order is what carries the offsets, so the comparison is
/// over the whole list in order rather than over its membership.
pub(super) fn check_section_record(file: &str, source: &str) -> Result<(), ShaderError> {
    let found = declared_fields(source, SECTION_RECORD_DECLARATION);
    let expected: Vec<String> = SECTION_RECORD
        .iter()
        .map(|(name, scalar)| format!("{name}: {scalar}"))
        .collect();
    if found == expected {
        return Ok(());
    }
    Err(ShaderError::SectionRecordMismatch {
        file: file.to_owned(),
        found,
        expected,
    })
}

/// Fails unless the shader's per-frame record is the one the frame uniform is
/// written as — the whole of it for the terrain stage, a valid prefix of it for
/// the cull stage.
///
/// **The two shaders are held to different halves of one rule**, which is what
/// makes this check different from every other one here. The terrain stage reads
/// the record to its end, so its declaration is the record. The cull stage reads
/// only what the record opens with, so it may stop early — but it may not stop
/// *differently*, and it may not reach past what terrain declares, because the
/// bytes past that end are bytes the CPU never wrote for it.
///
/// A shader declaring no such record at all reads as an empty list and is
/// refused rather than passed: an empty list is a prefix of everything, which is
/// exactly the pass a check like this must not give.
pub(super) fn check_frame_record(file: &str, source: &str) -> Result<(), ShaderError> {
    let found = declared_fields(source, FRAME_RECORD_DECLARATION);
    let expected: Vec<String> = FRAME_RECORD
        .iter()
        .map(|(name, kind)| format!("{name}: {kind}"))
        .collect();
    if file == TERRAIN_SHADER {
        if found == expected {
            return Ok(());
        }
        return Err(ShaderError::FrameRecordMismatch {
            file: file.to_owned(),
            found,
            expected,
        });
    }
    if !found.is_empty() && expected.starts_with(&found) {
        return Ok(());
    }
    Err(ShaderError::FramePrefixMismatch {
        file: file.to_owned(),
        found,
        expected,
    })
}

/// The value of a `const NAME: u32 = <literal>u;` declaration, or `None` where
/// the shader has no such declaration or it has outgrown that shape.
///
/// Blunt in the same way [`declared_values`] is, and for the same reason: a
/// declaration this cannot read reports as absent, which is a refusal rather
/// than a pass.
fn declared_scalar(source: &str, declaration: &str) -> Option<u32> {
    let (_, after_name) = source.split_once(declaration)?;
    let (_, after_equals) = after_name.split_once('=')?;
    let (value, _) = after_equals.split_once(';')?;
    value.trim().trim_end_matches('u').parse().ok()
}

/// The `name: type` of every field of a struct declaration, in order.
///
/// Comment lines inside the struct are skipped, so a field may be explained
/// where it is declared without the explanation reading as a field.
fn declared_fields(source: &str, declaration: &str) -> Vec<String> {
    let Some((_, body)) = source.split_once(declaration) else {
        return Vec::new();
    };
    let Some((body, _)) = body.split_once('}') else {
        return Vec::new();
    };
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .map(|line| {
            line.trim_end_matches(',')
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// The values `declaration` names in the source, or nothing when it names none.
///
/// An absent or unreadable declaration returns an empty list rather than a
/// variant of its own: "the shader does not say how it winds a quad" and "the
/// shader winds it differently" are the same defect from the build's point of
/// view, and both are reported by showing what was found.
///
/// The parse is deliberately blunt — the first `(` after the name to the first
/// `)` after that — which is exactly enough for a constructor call over integer
/// literals and nothing else. A declaration that outgrew that shape would read
/// as empty here, which is a refusal rather than a pass.
fn declared_values(source: &str, declaration: &str) -> Vec<u32> {
    let Some((_, after_name)) = source.split_once(declaration) else {
        return Vec::new();
    };
    let Some((_, after_open)) = after_name.split_once('(') else {
        return Vec::new();
    };
    let Some((values, _)) = after_open.split_once(')') else {
        return Vec::new();
    };
    values
        .split(',')
        .filter_map(|value| value.trim().trim_end_matches('u').parse().ok())
        .collect()
}
