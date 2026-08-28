//! What `opacity` means once the rest of the declaration has been read.
//!
//! The only field on a declaration whose acceptance depends on **another
//! field**, so the rule that decides it and the sentences it is refused in live
//! here rather than beside the reading of the number itself. [`super::number`]
//! holds what a stated number may be; this holds what this particular number may
//! be stated *beside*.
//!
//! A child of [`super`] for the reason [`super::texture`] is one: the refusal is
//! one of the parent's [`FieldFault`]s, and the refusals a mod author reads are
//! one vocabulary.

use mc_core::block::Opacity;
use mc_script::{ScriptHost, ScriptTable};

use super::{FieldFault, OCCLUDES_FIELD, OPACITY_FIELD, SOLID_FIELD, number};

/// How much light this declaration says the block stops, refused where that
/// contradicts what the same declaration says about hiding its neighbours.
///
/// **The loader's first cross-field refusal**, and it lives here rather than in
/// the caller so that the two lines which can contradict each other are read in
/// one place. `occludes` suppresses the neighbour's meeting face, so a block
/// light was meant to pass through would have nothing left behind it to show —
/// and whichever way the engine broke that tie it would be silently overruling
/// a line somebody wrote.
///
/// It is raised **on `opacity`**, because that is the field whoever hits this
/// has just added and the one they are looking at, and it names `occludes` too:
/// a refusal blaming one half of a contradiction leaves the other half
/// unfindable.
///
/// **Neither half is an offence alone.** An opaque block that hides what is
/// behind it is every block anybody has ever written, and a block that passes
/// light while hiding nothing is the whole point of the field — so the
/// condition is the conjunction and never either side of it.
///
/// **The condition is the occlusion the block ends up with, not the line it was
/// written on.** `occludes` falls back to solidity, so `solid = true` with no
/// `occludes` states occlusion just as surely as the word does — and it is the
/// resolved value the mesher reads. Asking only about the written line would
/// register a solid block that passes light, draw its face with the geometry
/// behind it culled, and refuse nothing.
pub(super) fn declared(
    host: &ScriptHost,
    declaration: &ScriptTable,
    occludes: bool,
) -> Result<Opacity, FieldFault> {
    let opacity = number::declared_degree(host, declaration)?;
    if occludes && opacity.passes_light() {
        let in_writing = host.read_field(declaration, OCCLUDES_FIELD).is_some();
        return Err(FieldFault::invalid(
            OPACITY_FIELD,
            &cannot_pass_light_and_occlude(in_writing),
        ));
    }
    Ok(opacity)
}

/// The sentence that contradiction is refused in, naming whichever line made
/// the block occlude.
///
/// **Two sentences, because the two remedies are different.** An author who
/// wrote `occludes = true` has a line to delete. An author who wrote
/// `solid = true` and no `occludes` has a line to *add*, and quoting them an
/// `occludes` their file does not contain sends them hunting for something that
/// is not there — the same mistake as blaming a floor for a value that is not a
/// number.
fn cannot_pass_light_and_occlude(in_writing: bool) -> String {
    let where_from = if in_writing {
        String::new()
    } else {
        format!(
            ", and this block occludes by stating `{SOLID_FIELD} = true` and no `{OCCLUDES_FIELD}`"
        )
    };
    format!(
        "`{OPACITY_FIELD}` below one cannot be stated with `{OCCLUDES_FIELD} = true`{where_from}: \
         a block light passes through cannot also hide what lies beyond it"
    )
}
