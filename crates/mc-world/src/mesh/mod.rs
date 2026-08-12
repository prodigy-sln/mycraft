//! Turning a section into the faces it actually shows, merged into as few
//! rectangles as one fixed sweep produces.
//!
//! The output is quads and not vertices. There is no triangulation here, no
//! index buffer, no winding order, no texture coordinates and no bit packing:
//! every one of those is derivable from a quad and every one of them is a
//! decision about a GPU, which belongs to the renderer rather than to storage.
//! Keeping the output unpacked is also what keeps this crate free of anything
//! rendering-shaped.
//!
//! A quad names its block by name and never by runtime id, because a runtime id
//! means something only to the registry that assigned it — a mesh still in
//! flight when the block set is swapped underneath it would otherwise resolve to
//! a different block. The cost is one reference-counted clone per quad, never
//! per voxel.
//!
//! Meshing is a pure read. Every parameter is a shared reference, a section has
//! no interior mutability, and the mesh handed back is owned, so mutating an
//! input is not expressible rather than merely discouraged. That is what lets
//! this move onto worker threads later as an integration and not as a rewrite.

mod facing;
mod neighbours;
mod plane;
mod resolve;
mod sweep;

use mc_core::id::BlockName;
use thiserror::Error;

use crate::section::{LocalPos, SectionError};

pub use facing::Facing;
pub use neighbours::Neighbours;
pub use sweep::mesh_section;

/// Where a quad starts inside its plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanePos {
    pub primary: u32,
    pub secondary: u32,
}

/// How far a quad runs inside its plane, at least one voxel along each axis.
///
/// A distinct type from [`PlanePos`] despite the identical shape, because
/// confusing where a rectangle starts with how far it runs is exactly the
/// mistake that separating a palette position from a runtime id was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaneExtent {
    pub primary: u32,
    pub secondary: u32,
}

/// One merged rectangle of visible faces, all of them pointing the same way and
/// all of them holding the same block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quad {
    pub facing: Facing,
    /// The coordinate, along the facing's axis, of the **solid voxel that
    /// emitted the face** — never of the face itself.
    ///
    /// Face coordinates would put a +X face at x = 15 on plane 16, forcing an
    /// axis that runs to 16 inclusive; voxel coordinates keep every plane inside
    /// `0..16`, which is the bound every other coordinate in this crate has.
    pub plane: u32,
    pub origin: PlanePos,
    pub extent: PlaneExtent,
    pub block: BlockName,
}

/// Every quad one section shows, in the one order it shows them in.
///
/// The order is facing, then plane ascending, then secondary ascending, then
/// primary ascending — and it is the sweep's loop nesting rather than a sort, so
/// there is no comparator anywhere that could disagree with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionMesh {
    quads: Vec<Quad>,
}

impl SectionMesh {
    /// The quads this mesh holds.
    #[must_use]
    pub fn quads(&self) -> &[Quad] {
        &self.quads
    }

    /// The quads this mesh holds, taken out of it.
    ///
    /// Exists so that whatever builds a vertex buffer from a mesh need not clone
    /// what it is about to consume.
    #[must_use]
    pub fn into_quads(self) -> Vec<Quad> {
        self.quads
    }

    /// A mesh holding `quads`, in the order the sweep produced them.
    fn of(quads: Vec<Quad>) -> Self {
        Self { quads }
    }
}

/// Why a section could not be meshed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MeshError {
    /// A voxel of the meshed section holds a block the registry does not
    /// register.
    ///
    /// There is no honest mesh for it. Reading it as non-solid punches a hole
    /// through the world and reading it as solid seals a cavity, and both are
    /// silent and indistinguishable from a correct mesh at the call site.
    #[error(
        "no block is registered under the name `{name}`, which the voxel at ({x}, {y}, {z}) holds",
        name = name.as_str(),
        x = position.x,
        y = position.y,
        z = position.z
    )]
    UnresolvedBlock { name: BlockName, position: LocalPos },
    /// A voxel of a supplied neighbour that faces the meshed section holds a
    /// block the registry does not register.
    ///
    /// The position is in the neighbour's own frame, which is the one somebody
    /// looking for the block would use.
    #[error(
        "no block is registered under the name `{name}`, which the voxel at ({x}, {y}, {z}) of \
         the {facing} neighbour holds",
        name = name.as_str(),
        x = position.x,
        y = position.y,
        z = position.z
    )]
    UnresolvedNeighbourBlock {
        name: BlockName,
        facing: Facing,
        position: LocalPos,
    },
    #[error(transparent)]
    Section(#[from] SectionError),
    /// An internal invariant, not anything a caller did: an index into the
    /// mesher's own fixed-size arrays that those arrays do not have.
    ///
    /// The sweep performs some tens of thousands of reads of its own arrays
    /// while raw indexing and every form of panic are lint-denied, so every one
    /// of them is `get`-shaped and every residual `None` needs somewhere to go.
    /// Folding them into a section's own corruption would report a mesher bug as
    /// a storage bug.
    #[error("index {index} is not one of the {length} the mesher's own array holds")]
    CorruptMeshIndex { index: usize, length: usize },
}
