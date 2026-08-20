//! A section that was meshed once and never meshed again still draws the keys
//! the content now serving declares.
//!
//! # This is the one reading that separates the two designs, and it is not a
//! reload
//!
//! The feature had two possible shapes. Under one, a quad remembers the texture
//! key its block declared at the moment it was meshed; under the other, a quad
//! stays purely geometric and the key is resolved where vertices are built. Every
//! other reading in this spec passes under both.
//!
//! **A test written against a reload passes under both as well, and that is why
//! this one is not one.** A texture-key change marks every section of the world,
//! and the dirty set is drained whole into a single batch, so there is no state a
//! running client can be in where a section is retained *and* redrawn against
//! content it was not meshed under. Writing the scenario as a reload would have
//! left a reader believing a path was covered that no client ever takes.
//!
//! The seam that is real sits one level down and is production behaviour rather
//! than a fixture convenience: the re-mesh worker keeps the whole meshed list for
//! the run and **re-packs all of it on every batch**, against whatever resolution
//! it currently holds. So a section nobody re-meshed is re-packed under the new
//! content — which is exactly the state this reads.
//!
//! # Both halves of the comparison are asserted, and the first half is the point
//!
//! The same retained list is packed twice, against the content it was meshed
//! under and against the content serving now. A packer that resolved at mesh time
//! would give the *old* layer both times, so the reading that fails first is
//! "under the new content, the old key is gone" — and the reading beside it, that
//! the old content still gives the old key, is what says the fixture can tell the
//! two apart at all.
//!
//! # What this cannot show on its own
//!
//! Nothing a deficient implementation of the stated interfaces does makes this
//! red: both designs satisfy every signature. Its falsifier is the mutation that
//! re-packs the retained list against the resolution its quads were meshed under
//! — the rejected design, applied by hand. A green here is evidence only once
//! that mutation has been seen to bite.

#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

use mc_client::content::ContentView;
use mc_client::remesh::Retained;
use mc_core::block::BlockRegistry;
use mc_core::content::{LayerAssignment, ResolvedContent};
use mc_core::id::TextureKey;
use mc_render::geometry::{SectionOrigin, build_section_geometry};
use mc_render::texture::TextureResolution;
use mc_sim::replay::SectionQuads;
use mc_sim::world::World;

use reload::{Declaration, STONE, STONE_FILE, restating, shipped};
use reload_world::floor_of;
use support::TestResult;

/// The key the block declares against `north` while the retained sections are
/// meshed, and the key the content serving afterwards declares there instead.
///
/// Neither is the block's own name, and the block's other five facings keep it —
/// which is what makes "the north faces moved" a statement about one facing.
const WHILE_MESHING: &str = "base:cobalt";
const SERVING_AFTERWARDS: &str = "base:diorite";

/// What re-packing the retained list against one content set produced.
#[derive(Debug, PartialEq, Eq)]
struct Repacked {
    /// Whether any corner was packed with the layer the *meshing* content gave
    /// `north`.
    draws_the_meshed_key: bool,
    /// Whether any corner was packed with the layer the *serving* content gives
    /// it.
    draws_the_serving_key: bool,
}

/// What a re-pack against the content serving now has to come to.
const THE_SERVING_KEY_ALONE: Repacked = Repacked {
    draws_the_meshed_key: false,
    draws_the_serving_key: true,
};

/// And what one against the content the sections were meshed under has to come
/// to, which is the fixture's own guard that the two are distinguishable.
const THE_MESHED_KEY_ALONE: Repacked = Repacked {
    draws_the_meshed_key: true,
    draws_the_serving_key: false,
};

/// A content set as the loader read it: what a client draws from, and what a
/// world is resolved against.
struct Content {
    resolved: ResolvedContent,
    registry: Arc<BlockRegistry>,
}

#[test]
fn sections_repacked_without_being_remeshed_draw_the_key_the_serving_content_declares() -> TestResult
{
    let meshing = content_pointing_north_at(WHILE_MESHING, &LayerAssignment::none())?;
    let serving = content_pointing_north_at(SERVING_AFTERWARDS, meshing.resolved.layers())?;
    let both = Layers {
        meshed: layer_for(&meshing.resolved, WHILE_MESHING)?,
        serving: layer_for(&serving.resolved, SERVING_AFTERWARDS)?,
    };
    require_distinct(&both)?;

    let retained = meshed_under(&meshing)?;
    let under_the_content_it_was_meshed_under =
        repacked(&retained.meshed, &resolution_of(&meshing.resolved), &both)?;
    let under_the_content_serving_now =
        repacked(&retained.meshed, &resolution_of(&serving.resolved), &both)?;

    assert_eq!(
        (
            under_the_content_serving_now,
            under_the_content_it_was_meshed_under
        ),
        (THE_SERVING_KEY_ALONE, THE_MESHED_KEY_ALONE),
        "these sections were meshed while `north` drew from {WHILE_MESHING} and are packed again, \
         without being meshed again, against content declaring {SERVING_AFTERWARDS} there. A quad \
         that remembered the key its block declared when it was meshed would carry \
         {WHILE_MESHING} into a picture of content nobody is playing, silently, on exactly the \
         path that exists so a world need not be meshed twice. The second half is the fixture's \
         own guard: the same list against the content it *was* meshed under still gives the old \
         key, so a reading that could not tell the two apart fails here"
    );
    Ok(())
}

/// The content a root declaring `STONE` with `north` drawing from `key` resolves
/// to, read against the layers `spent` already holds.
///
/// Read through the loader the client's own reload reads through, so the
/// assignment is the one a session would actually be holding: the second call
/// appends its new key beside the first's rather than renumbering anything.
///
/// # Errors
///
/// Returns an error if the root cannot be written or if the loader refuses it.
fn content_pointing_north_at(
    key: &str,
    spent: &LayerAssignment,
) -> Result<Content, Box<dyn Error>> {
    let root = restating(
        shipped()?,
        STONE_FILE,
        &Declaration::of(STONE).repointing_north(key),
    )?;
    let loaded = mc_sim::content::load(root.path(), spent)?;
    Ok(Content {
        resolved: loaded.resolved,
        registry: Arc::new(loaded.registry),
    })
}

/// The client's view of `content`, which is what a packer is handed.
fn resolution_of(content: &ResolvedContent) -> TextureResolution {
    ContentView::of(content).into_resolution()
}

/// The layer `content` states for `key`.
///
/// # Errors
///
/// Returns an error if the key does not parse or if the content assigns it no
/// layer — a fixture whose key never reached the assignment would make every
/// reading below vacuously false.
fn layer_for(content: &ResolvedContent, key: &str) -> Result<u16, Box<dyn Error>> {
    let wanted = TextureKey::parse(key)?;
    content
        .layer_assignment()
        .find(|(named, _)| **named == wanted)
        .map(|(_, layer)| layer)
        .ok_or_else(|| format!("this fixture's content has to assign `{key}` a layer").into())
}

/// Refuses two layers that are the same number.
///
/// # Errors
///
/// Returns an error when they agree, because every reading below distinguishes
/// the two contents by the layer index a corner carries, and two keys sharing an
/// index would make both readings true of either.
fn require_distinct(both: &Layers) -> Result<(), Box<dyn Error>> {
    if both.meshed == both.serving {
        return Err(format!(
            "the two keys this fixture points `north` at have to hold different layers, and both \
             hold {layer} — the readings below tell the two contents apart by nothing else",
            layer = both.meshed
        )
        .into());
    }
    Ok(())
}

/// The two layers every reading here tells the two contents apart by.
///
/// Carried as one value so the pair cannot be handed over in the wrong order,
/// and so no function on this path takes more arguments than the lint allows.
struct Layers {
    meshed: u16,
    serving: u16,
}

/// The sections a launch over the content `content` describes would have
/// retained, meshed by the world's own whole-world mesh.
///
/// # Errors
///
/// Returns an error if the root cannot be read, if the world refuses a write, or
/// if it cannot be meshed.
fn meshed_under(content: &Content) -> Result<Retained, Box<dyn Error>> {
    let blocks = floor_of(&content.registry, STONE)?;
    Ok(Retained {
        meshed: World::new(blocks, Arc::clone(&content.registry))?.mesh()?,
        resolution: resolution_of(&content.resolved),
    })
}

/// Whether re-packing `meshed` against `resolution` puts either layer into any
/// corner.
///
/// # Errors
///
/// Returns an error if a section will not pack, or if the whole list produced no
/// corners at all — a retained list that meshed nothing satisfies "the old key
/// is gone" for free.
fn repacked(
    meshed: &[SectionQuads],
    resolution: &TextureResolution,
    both: &Layers,
) -> Result<Repacked, Box<dyn Error>> {
    let mut packed = BTreeSet::new();
    let mut corners = 0usize;
    for section in meshed {
        let geometry = build_section_geometry(
            &section.quads,
            SectionOrigin::new(section.origin),
            resolution,
        )?;
        for layer in (0usize..).map_while(|corner| geometry.layer_at(corner)) {
            packed.insert(layer);
            corners += 1;
        }
    }
    if corners == 0 {
        return Err(
            "a floor of solid blocks packs corners; this retained list packed none, and \
                    every reading over it would be vacuous"
                .into(),
        );
    }
    Ok(Repacked {
        draws_the_meshed_key: packed.contains(&both.meshed),
        draws_the_serving_key: packed.contains(&both.serving),
    })
}
