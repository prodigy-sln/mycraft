//! Reading a `.mcvox` document, in the order the checks have to happen.
//!
//! Three passes, and the order is load-bearing. Syntax first, because nothing
//! else can be read from a file that does not parse. Then everything the header
//! declares — schema, name, scale, slice, the geometry form and every part's
//! extent — because a part declaring 65 voxels on an axis is refused *before any
//! grid is parsed*, so an over-large declaration never needs its layer art to
//! exist. Only then the art itself.
//!
//! Assembly, which places the parts relative to one another, is a later question
//! than any of these and lives elsewhere.

pub mod assemble;
pub mod grid;
pub mod header;
pub mod state;
pub mod tree;

use std::num::NonZeroU32;

use crate::fault::{Fault, Origin};
use crate::format::dto::{DocumentDto, from_text};
use crate::format::{Model, palette};

/// The model `text` describes, attributed to `origin`.
///
/// # Errors
///
/// Returns a [`Fault`] naming the origin, the element and the field at fault if
/// the document is not a legal `.mcvox` document.
pub fn document(text: &str, origin: Origin) -> Result<Model, Fault> {
    let declared: DocumentDto = from_text(text, &origin)?;

    header::check_schema(&declared, &origin)?;
    let name = header::read_name(&declared, &origin)?;
    let scale = header::read_scale(&declared, &origin)?;
    let slice = header::read_axis(declared.slice.as_ref(), &origin)?;
    let mut parts = header::read_parts(&declared, slice, &origin)?;
    // Before any layer is read: a layer naming a part is meaningless until the
    // parts themselves are known to be a tree with unambiguous names.
    tree::check(&parts, &origin)?;

    let palette = palette::resolve(declared.palette.as_ref(), &origin)?;
    let spelled = grid::read_layers(&declared, &mut parts, &palette, &origin)?;
    state::check(&parts, &origin)?;

    Ok(Model {
        name,
        // `read_scale` has already refused everything below 1, so the fallback
        // is unreachable rather than a silent correction.
        scale: NonZeroU32::new(scale).unwrap_or(NonZeroU32::MIN),
        parts,
        palette,
        origin,
        spelled,
    })
}
