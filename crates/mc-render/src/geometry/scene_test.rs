//! The one place a scene's capacity is decided.
//!
//! Assembly is the *only* capacity gate in the renderer: a scene holding more
//! sections than the visible-set buffer can address cannot be constructed, so
//! there is nothing downstream left to re-check and nothing that could disagree
//! with this answer. That makes the refusal here load-bearing — the alternative
//! is a frame that draws the first 1024 sections and silently omits the rest,
//! which looks like a world that was generated smaller rather than like a bug.
//!
//! The refusal is asserted against the count that went in and the capacity that
//! was exceeded, because an error saying only "too many" leaves whoever reads it
//! guessing at both numbers.
//!
//! The assembly at exactly capacity is a guard, not a second scenario: without
//! it, an `assemble` that refused every scene it was ever handed would satisfy
//! the refusal below and pass forever.

use std::collections::BTreeSet;
use std::error::Error;

use crate::geometry::{SectionGeometry, SectionOrigin, build_section_geometry};
use crate::texture::TextureLayers;

use super::{SceneError, SceneGeometry};

type TestResult = Result<(), Box<dyn Error>>;

/// How many sections the visible-set buffer addresses. Declared, not measured:
/// the replay uses 256 of them, so nothing in this project discovers this number
/// by running into it.
const SECTION_CAPACITY: usize = 1024;

/// Where every section in this suite sits. Assembly counts sections; where they
/// are is the frustum's question, not this one.
const SOMEWHERE: [i32; 3] = [0, 0, 0];

/// `count` sections holding no quads, so the section count is the only capacity
/// this scene can exceed.
fn empty_sections(
    count: usize,
    layers: &TextureLayers,
) -> Result<Vec<SectionGeometry>, Box<dyn Error>> {
    let mut sections = Vec::with_capacity(count);
    for _ in 0..count {
        sections.push(build_section_geometry(
            &[],
            SectionOrigin::new(SOMEWHERE),
            layers,
        )?);
    }
    Ok(sections)
}

#[test]
fn assembling_more_sections_than_the_scene_holds_names_the_count_and_the_capacity() -> TestResult {
    let layers = TextureLayers::resolve(&BTreeSet::new());

    SceneGeometry::assemble(empty_sections(SECTION_CAPACITY, &layers)?).map_err(|refusal| {
        format!(
            "a scene of exactly {SECTION_CAPACITY} sections must assemble, or the refusal \
             below proves nothing: {refusal}"
        )
    })?;

    let over_capacity = empty_sections(SECTION_CAPACITY + 1, &layers)?;
    let refusal = SceneGeometry::assemble(over_capacity).err().ok_or(
        "a scene holding more sections than the visible-set buffer can address must be \
         refused, not truncated to the sections that fit",
    )?;

    match refusal {
        SceneError::TooManySections { found, capacity } => assert_eq!(
            (found, capacity),
            (SECTION_CAPACITY + 1, SECTION_CAPACITY),
            "the refusal must name both the section count it was handed and the capacity \
             that count exceeded"
        ),
        other => {
            return Err(format!("expected a section-capacity refusal, got {other:?}").into());
        }
    }
    Ok(())
}
