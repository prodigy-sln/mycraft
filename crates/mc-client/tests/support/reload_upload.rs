//! What the report of an accepted reload hands the frame path, and what a packer
//! does with it.
//!
//! # The layers come out of the client's own report, never out of a second read
//!
//! `App`'s whole share of a reload is an upload of layers somebody else built and
//! a scene somebody else packed, and `crates/mc-client/src/app/` needs a real
//! window that nothing in this workspace constructs. So the value is asserted
//! where it crosses out of the client's core: [`until_taken_up`] reads
//! [`ReloadReport::Accepted`] itself rather than through phase 3's `Attempt`,
//! which discards everything but the fact that something was taken up. A fixture
//! that rebuilt the layers from a second read of the same content root would agree
//! with itself while the report carried nothing at all.
//!
//! # The author's edit happens after the client has launched
//!
//! [`declaring_after_launch`] and [`restating_after_launch`] write into the root a
//! running client is already playing, and each refuses the state that would make
//! its scenario vacuous — a file that is already there, or one that is not. Phase 3
//! shipped three fixtures that edited the root *before* the client played it, and a
//! client already serving the edited content makes every claim about what the reload
//! changed either trivially true or red against a correct implementation.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names a report that carries the layers the content now serving states, which
//! the implementation has not written yet. A binary including this must declare
//! `mod support;`, the input harness, [`crate::reload`], [`crate::reload_remesh`]
//! and [`crate::reload_watch`] as well.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use mc_client::session::reload::ReloadReport;
use mc_client::upload::Unuploaded;
use mc_render::geometry::{GeometryError, SectionGeometry, SectionOrigin, build_section_geometry};
use mc_render::texture::TextureLayers;
use mc_world::mesh::Quad;

use crate::input::InputHarness;
use crate::reload::Declaration;
use crate::reload_remesh::{Meshed, Sections, require};
use crate::reload_watch::{may_cross_another, pause_between_boundaries};
use crate::support::content::{BLOCK_DIRECTORY, ContentRoot};

/// How many corners one quad is packed into.
///
/// Stated so a reading covers the whole run of corners rather than sampling one: a
/// packer writing the right layer into the first corner of each quad and the wrong
/// one into the other three draws three quarters of the world from the wrong
/// texture.
pub const CORNERS_PER_QUAD: usize = 4;

/// How many faces a block set on top of a solid floor shows: five of its six, the
/// downward one buried against what it stands on.
pub const A_BLOCK_ON_A_FLOOR_SHOWS: usize = 5;

/// What a run of tick boundaries made of a content change.
///
/// **A total verdict**, so a scenario expecting a taking up cannot be satisfied by
/// a refusal or by a run in which no boundary reported anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TakenUp {
    /// A candidate was taken up, and its report handed over these layers — still
    /// wrapped, because the device has not been given them yet.
    ///
    /// **Carried wrapped rather than unwrapped here, and that is not tidiness.**
    /// `Unuploaded::uploaded_to` is the one route to an owned `TextureLayers`, and
    /// it is what the frame path goes through; a fixture that cloned the borrow out
    /// would leave that route reachable from nothing in this workspace, which is a
    /// smaller version of the hole the wrapper was added to close. The scenario
    /// about the held-block indicator drives it for exactly that reason.
    Layers(Unuploaded),
    /// A candidate was refused, in the words a person reads.
    Refused { said: String },
    /// No boundary reported anything before the run gave up.
    NothingReported,
}

/// Crosses tick boundaries until one reports what it made of the content root.
///
/// The patience is [`crate::reload_watch`]'s, so there is one statement of it
/// rather than a second number beside it.
pub fn until_taken_up(client: &mut InputHarness) -> TakenUp {
    let started = Instant::now();
    while may_cross_another(started) {
        client.tick();
        match client.take_reload_report() {
            None => pause_between_boundaries(),
            Some(ReloadReport::Refused(said)) => return TakenUp::Refused { said },
            Some(ReloadReport::Accepted { layers, .. }) => return TakenUp::Layers(layers),
        }
    }
    TakenUp::NothingReported
}

/// The layers a taking up handed over, still unuploaded, or an error naming what
/// happened instead.
///
/// # Errors
///
/// Returns an error unless a candidate was taken up.
pub fn layers_handed_over(taken: TakenUp) -> Result<Unuploaded, Box<dyn Error>> {
    match taken {
        TakenUp::Layers(layers) => Ok(layers),
        other => Err(format!(
            "this fixture needs a candidate to have been taken up before it can draw with the \
             layers its report handed over, and the run came to {other:?}"
        )
        .into()),
    }
}

/// Writes a declaration the root does not hold into the root a client is already
/// playing, and says where it landed.
///
/// # Errors
///
/// Returns an error if the root already declares that file, or if the write fails.
pub fn declaring_after_launch(
    root: &ContentRoot,
    file_name: &str,
    declaration: &Declaration,
) -> Result<PathBuf, Box<dyn Error>> {
    let declared = root.path().join(BLOCK_DIRECTORY).join(file_name);
    require(
        !declared.exists(),
        format!(
            "this fixture has to declare `{BLOCK_DIRECTORY}/{file_name}` for the first time in a \
             root a client is already playing, and that root declares it already. What it would \
             build is a client that launched with the block rather than one an author added it to"
        ),
    )?;
    fs::write(&declared, declaration.text())?;
    Ok(declared)
}

/// Replaces a declaration the root does hold in the root a client is already
/// playing, and says where it landed.
///
/// # Errors
///
/// Returns an error if the root does not declare that file, or if the write fails.
pub fn restating_after_launch(
    root: &ContentRoot,
    file_name: &str,
    declaration: &Declaration,
) -> Result<PathBuf, Box<dyn Error>> {
    let declared = root.path().join(BLOCK_DIRECTORY).join(file_name);
    require(
        declared.is_file(),
        format!(
            "this fixture has to restate `{BLOCK_DIRECTORY}/{file_name}` in a root a client is \
             already playing, and that root does not declare it. What it would build is a root \
             that gained a declaration rather than one whose declaration an author edited"
        ),
    )?;
    fs::write(&declared, declaration.text())?;
    Ok(declared)
}

/// What the packer made of one block's faces.
///
/// **A total verdict**: a block whose corners carry two different layers, one
/// whose section the packer refused, and one with no faces at all each have an arm
/// of their own, so an assertion against [`Faces`](Self::Faces) rejects every one
/// of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packed {
    /// Every corner of every one of the block's faces carries this one layer, and
    /// these are the other blocks whose faces were packed onto it.
    Faces {
        corners: usize,
        layer: u16,
        sharing: Vec<String>,
    },
    /// Its corners carry these layers, which is not exactly one.
    Layers(BTreeSet<u16>),
    /// A section was refused, naming this block.
    RefusedNaming(String),
    /// A section was refused for a reason that is not about a texture.
    RefusedOtherwise(String),
    /// There were no sections to pack, because the mesh came to this instead.
    NotMeshed(String),
}

/// What packing everything `meshed` produced against `layers` wrote for `block`.
///
/// **The layer is read back out of the packed corners** and never asked of the
/// assignment: a consumer free to re-derive an assignment of its own is the exact
/// failure the appended-never-renumbered policy exists to close, and an assertion
/// that asked the assignment would agree with it whatever the packer wrote.
#[must_use]
pub fn packed(meshed: &Meshed, block: &str, layers: &TextureLayers) -> Packed {
    match meshed {
        Meshed::Sections(sections) => packing(sections, block, layers),
        other => Packed::NotMeshed(format!("{other:?}")),
    }
}

/// The same, over sections a scenario is already holding.
#[must_use]
fn packing(sections: &Sections, block: &str, layers: &TextureLayers) -> Packed {
    let mut written: BTreeMap<String, Vec<u16>> = BTreeMap::new();
    for section in sections.values() {
        let origin = SectionOrigin::new(section.origin);
        match build_section_geometry(&section.quads, origin, layers) {
            Ok(geometry) => record_corners(&mut written, &section.quads, &geometry),
            Err(GeometryError::UnresolvedTexture { block: named }) => {
                return Packed::RefusedNaming(named.as_str().to_owned());
            }
            Err(other) => return Packed::RefusedOtherwise(other.to_string()),
        }
    }
    packed_for(&written, block)
}

/// What the corners recorded for `block` amount to.
fn packed_for(written: &BTreeMap<String, Vec<u16>>, block: &str) -> Packed {
    let corners = written.get(block).cloned().unwrap_or_default();
    let distinct: BTreeSet<u16> = corners.iter().copied().collect();
    let [layer] = distinct.iter().copied().collect::<Vec<u16>>()[..] else {
        return Packed::Layers(distinct);
    };
    Packed::Faces {
        corners: corners.len(),
        layer,
        sharing: sharing(written, block, layer),
    }
}

/// Every block other than `block` whose corners carry `layer`.
fn sharing(written: &BTreeMap<String, Vec<u16>>, block: &str, layer: u16) -> Vec<String> {
    written
        .iter()
        .filter(|(named, corners)| named.as_str() != block && corners.contains(&layer))
        .map(|(named, _)| named.clone())
        .collect()
}

/// Records the layer each of `geometry`'s corners carries against the block whose
/// face emitted it.
///
/// Four corners per quad, in the order the quads were handed to the packer, which
/// is the one relation between a quad and the corners it became.
fn record_corners(
    written: &mut BTreeMap<String, Vec<u16>>,
    quads: &[Quad],
    geometry: &SectionGeometry,
) {
    for (index, quad) in quads.iter().enumerate() {
        let corners = (0..CORNERS_PER_QUAD)
            .filter_map(|corner| geometry.layer_at(index * CORNERS_PER_QUAD + corner));
        written
            .entry(quad.block.as_str().to_owned())
            .or_default()
            .extend(corners);
    }
}
