//! Emitting one face, or a block's whole six, from a single request.
//!
//! **The set is all-or-nothing.** Every verdict is computed and every byte
//! vector built before any file is opened, so a set with one bad face writes
//! nothing at all. That extends the preview's "build the whole encoding first"
//! rule across six files, and it is the one thing a shell loop calling this
//! tool six times cannot provide: by the time the fourth invocation refuses,
//! three files are already on disk.
//!
//! **The verdict is computed on both policies and binds on one.** Not every
//! texture has to tile — a decorative one-off block is legitimate content — so
//! refusing one would be the tool inventing a rule nobody asked for. Reported
//! is an observation: image emitted, verdict carried out, exit 0. Required is a
//! defect: nothing emitted, the verdict's own words as the refusal. Computing
//! it either way is deliberate, because a check that runs only under a flag
//! rots on the path most invocations take.

use std::num::NonZeroU32;

use crate::fault::{Fault, Origin};
use crate::format::{Axis, Extent};
use crate::material::MaterialTable;
use crate::render::{Preview, render_texture};
use crate::texture::seam::{self, SeamVerdict};
use crate::texture::set::AxisAlignedView;
use crate::volume::Volume;

/// Whether a seam verdict is advice or a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeamPolicy {
    /// Computed, carried out with the image, and binding on nothing.
    Reported,
    /// Computed, and a failing leg refuses the emission.
    Required,
}

/// Which faces one request asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceSelection {
    /// That one face.
    One(AxisAlignedView),
    /// A block's whole six, for which being cubic is a precondition of the
    /// request rather than a seam verdict.
    All,
}

/// Everything an emission needs to know.
#[derive(Debug, Clone)]
pub struct TextureRequest {
    /// Which faces to emit.
    pub faces: FaceSelection,
    /// How many pixels one voxel spans.
    pub pixels_per_voxel: NonZeroU32,
    /// How many voxels the document declares to one block edge.
    pub scale: NonZeroU32,
    /// Whether the seam verdict binds.
    pub seams: SeamPolicy,
    /// The document the emission is of, for anything that has to be refused.
    pub origin: Origin,
}

/// One emitted face: its image and what the seam question said about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmittedFace {
    /// Which face of the block it is.
    pub face: AxisAlignedView,
    /// Its texture.
    pub image: Preview,
    /// What each leg answered, in the declared order.
    ///
    /// **Never empty.** A texture fit to be tiled carries exactly one
    /// [`SeamVerdict::TilesAcrossEveryEdge`], and one that is not carries one
    /// entry per failing leg — so "this tiles" is an enumerated answer rather
    /// than an empty list, which cannot be told apart from a check that no
    /// longer looks.
    pub verdicts: Vec<SeamVerdict>,
}

/// Every face one request emitted, in [`AxisAlignedView::ALL`] order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureSet {
    /// The faces, in the order they are reported.
    pub faces: Vec<EmittedFace>,
}

/// The textures `request` asks for, of `volume` painted from `materials`.
///
/// # Errors
///
/// Returns a [`Fault`] when the request itself cannot be served — a face set of
/// a model that is not a cube — and, under [`SeamPolicy::Required`], when any
/// requested face's first failing leg says the texture will not tile.
pub fn emit(
    volume: &Volume,
    materials: &MaterialTable,
    request: TextureRequest,
) -> Result<TextureSet, Fault> {
    let wanted = requested_faces(volume, &request)?;
    // Every face rendered and every verdict reached before anything is refused,
    // so that a set refusing on its sixth face has still not written its first.
    let faces: Vec<EmittedFace> = wanted
        .into_iter()
        .map(|face| emitted(volume, materials, face, &request))
        .collect();
    if request.seams == SeamPolicy::Required {
        refuse_the_first_failure(&faces, &request.origin)?;
    }
    Ok(TextureSet { faces })
}

/// Which faces the request asks for, refusing a request that cannot be served.
///
/// Cubic-ness is checked **here**, on the set path only, because it is a
/// precondition of the *request* rather than a seam verdict: a face set is by
/// definition a block's six faces, so a non-cubic one is nonsense whatever the
/// seam policy says. Conditionalising it instead would leave `--all-faces` on a
/// `[4,4,3]` model emitting a "block texture set" that is not one.
fn requested_faces(
    volume: &Volume,
    request: &TextureRequest,
) -> Result<Vec<AxisAlignedView>, Fault> {
    let FaceSelection::All = request.faces else {
        let FaceSelection::One(face) = request.faces else {
            return Ok(Vec::new());
        };
        return Ok(vec![face]);
    };
    let extent = volume.extent();
    let scale = request.scale.get();
    // The axis is named, not merely the three numbers: an author reading this
    // has to know which dimension to change, and "4 by 4 by 3" leaves them to
    // work out which of the three is wrong.
    let offending = [Axis::X, Axis::Y, Axis::Z]
        .into_iter()
        .find(|axis| along(extent, *axis) != scale);
    let Some(axis) = offending else {
        return Ok(AxisAlignedView::ALL.to_vec());
    };
    Err(Fault::about(
        request.origin.clone(),
        format!(
            "a face set is a block's six faces, so the model must be a cube of the declared scale {scale} — this one is {voxels} voxels on the {axis} axis, and assembles to {x} by {y} by {z}",
            voxels = along(extent, axis),
            axis = axis.as_str(),
            x = extent.x,
            y = extent.y,
            z = extent.z
        ),
    )
    .in_field("all-faces"))
}

/// One face, rendered and judged.
fn emitted(
    volume: &Volume,
    materials: &MaterialTable,
    face: AxisAlignedView,
    request: &TextureRequest,
) -> EmittedFace {
    let image = render_texture(volume, materials, face, request.pixels_per_voxel);
    let extent = volume.extent();
    let verdicts = seam::judge(
        &image,
        &seam::InPlane {
            axis: face.columns(),
            voxels: along(extent, face.columns()),
        },
        &seam::InPlane {
            axis: face.rows(),
            voxels: along(extent, face.rows()),
        },
        request.scale.get(),
    );
    EmittedFace {
        face,
        image,
        verdicts,
    }
}

/// Refuses the first failing leg of the first failing face.
///
/// The first, so the diagnostic is reproducible: a set whose faces fail for
/// several reasons would otherwise refuse with whichever one an iteration order
/// happened to reach.
fn refuse_the_first_failure(faces: &[EmittedFace], origin: &Origin) -> Result<(), Fault> {
    for emitted in faces {
        let Some(failure) = emitted
            .verdicts
            .iter()
            .find(|verdict| **verdict != SeamVerdict::TilesAcrossEveryEdge)
        else {
            continue;
        };
        return Err(Fault::about(
            origin.clone(),
            format!(
                "the {face} face will not tile: {failure}",
                face = emitted.face.as_str()
            ),
        )
        .in_field("seamless"));
    }
    Ok(())
}

/// How far an extent reaches along one axis.
fn along(extent: Extent, axis: Axis) -> u32 {
    match axis {
        Axis::X => extent.x,
        Axis::Y => extent.y,
        Axis::Z => extent.z,
    }
}
