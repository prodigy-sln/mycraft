//! Block textures: a flat render of one axis-aligned face, and what the seam
//! question says about it.
//!
//! A texture is not a preview with a flag. A preview exists to be *read* by an
//! agent correcting its own geometry, so it is shaded to make shape legible; a
//! texture exists to be *sampled* by a mesh, where a face factor baked into the
//! image would light every block twice. That is why flatness belongs to the
//! render rather than to a material — a material table is shared across a whole
//! art set and cannot express a per-emission intent — and why an isometric
//! "texture" is refused outright rather than emitted.
//!
//! The terrain shader derives its uv from section-local position in whole
//! blocks, samples with `AddressMode::Repeat`, and the mesher merges each run of
//! matching faces into one quad. So a quad merged across N blocks shows the
//! texture N times across one unbroken surface, and opposing edges that
//! disagree draw a grid over every large flat area. That is the whole reason
//! the seam verdict exists — and it is *reported* rather than enforced, because
//! a decorative one-off block need not tile and refusing it would be the tool
//! inventing a rule its author never asked for.

mod emit;
mod seam;
mod set;

pub use emit::{EmittedFace, FaceSelection, SeamPolicy, TextureRequest, TextureSet, emit};
pub use seam::{Line, PixelPos, SeamVerdict};
pub use set::AxisAlignedView;
