//! A player standing inside a medium, in a client whose content root a mod
//! author edits while it runs.
//!
//! # What is the fixture's and what is the shipped path
//!
//! Two things are the fixture's: the **declarations** — a copy of the shipped
//! root holding nothing but the two blocks these readings name — and the
//! **voxels**, because no seed generates a world with a wall six blocks from a
//! player standing inside a medium. Everything between is the product's own:
//! `mc_sim::content::load` reads the root, the client watches it, a tick
//! boundary builds the candidate and reports what it made of it, `World::adopt`
//! swaps the registry the simulation resolves against, `ContentView::of` turns
//! what is now serving into a resolution, `scene_of` packs the sections against
//! it and `TerrainRenderer` draws them.
//!
//! # The tint is read where the product publishes it, and that is the whole
//! point of this module
//!
//! [`InputHarness::published`] hands back the `SimSnapshot` the simulation
//! wrote, and `App::snapshot` copies that snapshot's `tint` into the frame
//! without computing anything of its own. So the field these readings compare
//! across a reload is **the field the shipped client draws from**, resolved by
//! `eye_medium` against the world and the registry the simulation holds *at that
//! publish*.
//!
//! **This is the only instrument in the suite that can see a cache.** Every
//! FR-2 reading declares its own pose and renders one frame, so an
//! implementation that resolved the eye's medium once and remembered it would
//! satisfy all of them. What it cannot satisfy is a *second* publish, after a
//! reload, disagreeing with the first — which is what each reading below
//! asserts. A failure here is a caching defect before it is a reload defect.
//!
//! # The world is meshed once and never again
//!
//! The same `Vec<SectionQuads>` is packed before and after the reload, which is
//! what the architecture claims a changed tint costs — nothing at all, since
//! `changes_geometry` is keyed positively on the three drawing fields and a tint
//! is outside it by construction. A fixture that re-meshed between the two would
//! be asserting about a path a running client never takes for this edit.
//!
//! # Where the six blocks come from
//!
//! The medium fills `x ∈ [0, 20)` above a floor of the wall block, and the wall
//! itself is the single layer `x = 20`, so its `−X` face stands at exactly
//! `x = 20.0`. The player spawns at `x = 14.0` with yaw zero, which looks along
//! `+X`, and does not move because nothing presses a key — so the published
//! camera's own eye is `6.0` blocks from that face. **The distance is measured
//! off the published camera rather than assumed**, by subtracting two numbers,
//! so a player the physics moved is reported rather than silently mispredicted.
//!
//! The eye stands `1.62` blocks over the feet, at `y = 120.62`, inside the
//! medium's own cell. `x = 14.0` is a cell boundary and that is safe here for
//! the reason `support/medium.rs` states: cells 13 and 14 both hold the medium,
//! so the answer does not turn on the rounding — and each verdict names the tint
//! the resolver actually found, so a reading says so rather than assuming it.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names the reload fixtures, which are themselves reached that way. A binary
//! including this must declare `mod support;`, the input harness,
//! [`crate::reload`], [`crate::reload_upload`], [`crate::reload_watch`] and
//! [`crate::reload_world`] as well.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_client::startup::scene_of;
use mc_core::block::{BlockRegistry, MediumTint};
use mc_core::id::{BlockName, TextureKey};
use mc_render::camera::camera_view;
use mc_render::gpu::{RecordTarget, TerrainRenderer, TerrainTextures};
use mc_render::pass::TerrainPassConfig;
use mc_render::snapshot::{ScenePhase, TerrainSnapshot};
use mc_render::texture::TextureResolution;
use mc_render::texture::sampler::TERRAIN_SAMPLER;
use mc_render::texture::supplied::SuppliedTexels;
use mc_sim::replay::SectionQuads;
use mc_sim::simulation::SimSnapshot;
use mc_sim::world::World;
use mc_testkit::frame::Rgba8Image;
use mc_testkit::frame::gpu::{CaptureContext, draw_fn};
use mc_world::world::{VoxelWorld, WorldPos};

use crate::input::InputHarness;
use crate::reload_watch::{Reports, a_client_over};
use crate::reload_world::{registry_of, standing_at};
use crate::support::content::{ContentRoot, shipped_copy};
use crate::support::frames::{CAPTURE_SIZE, request};
use crate::support::medium::{
    MEDIUM_COLOUR, REACHES_AT, Strays, TELLS_THEM_APART, THE_CENTRE, TINT, WALL_COLOUR,
};
use crate::support::probe::{distance, pixel_color};

/// The two blocks these readings declare, and the files they are declared in.
pub const MEDIUM: &str = "example:medium";
pub const WALL: &str = "example:wall";
pub const MEDIUM_FILE: &str = "example__medium.luau";
pub const WALL_FILE: &str = "example__wall.luau";

/// The distance a reload narrows the medium's reach to, at which the wall the
/// player faces is drawn wholly at the declared colour.
pub const NARROWED_TO: f32 = 6.0;

/// A distance no medium may reach full strength at, which is what FR-4.1-S3's
/// reload states.
pub const AT_NO_DISTANCE: f32 = 0.0;

/// The world's shape. The floor is the wall block so that a player has something
/// to rest on, the medium fills every cell above it up to the wall, and the wall
/// is one layer whose `−X` face the player faces.
const FLOOR: u32 = 118;
const CEILING: u32 = 140;
const WALL_PLANE: u32 = 20;
const ACROSS: u32 = 64;
const COLUMNS: u32 = 4;

/// Where the player's feet stand, and how far the wall's face is from the eye
/// over them.
pub const SPAWN: Vec3 = Vec3::new(14.0, 119.0, 32.5);
pub const THE_WALL_STANDS_AT: f32 = 6.0;

/// How many pixels each reading examines, and how far the cluster reaches from
/// the frame's centre.
///
/// **A tight cluster rather than the whole frame**, because the wall is faced
/// squarely and a pixel away from the centre stands further from the eye than
/// the centre does — which is FR-2.1-S4's subject and would be noise here. At
/// thirty-two pixels out the ray leaves at a tangent of `0.051`, so its radial
/// distance is `6.008` against the centre's `6.0`: a mix of `0.5007` against
/// `0.5`, far inside a code value. Below the cluster the frame shows the floor
/// and above it the wall goes on, so the cluster is also the region whose
/// distance is one number.
const CLUSTER_REACH: u32 = 32;
const CLUSTER_STEP: u32 = 8;
pub const EXAMINED: usize = 81;

/// The tint a declaration states, and the pair a reload may put it into.
#[must_use]
pub fn reaching(distance_in_blocks: f32) -> Option<MediumTint> {
    MediumTint::new(TINT, distance_in_blocks)
}

/// The medium's declaration, stating a tint pair where one is given.
///
/// **`solid = false` and `occludes = false` are stated whatever the tint is**,
/// so two roots a reading compares differ in the tint pair and in nothing else.
/// The player has to be able to stand *inside* this block for any of these
/// readings to be about anything, and `occludes` falls back to `solid`, so a
/// block that both passes light and hides what is behind it would be refused,
/// taking the root with it.
#[must_use]
pub fn medium_declaring(tint: Option<(f32, [u8; 3])>) -> String {
    let stated = tint.map_or_else(String::new, |(reach, [red, green, blue])| {
        format!("\ttint = \"#{red:02X}{green:02X}{blue:02X}\",\n\ttint_distance = {reach:?},\n")
    });
    format!(
        "return {{\n\tname = \"{MEDIUM}\",\n\ttexture = \"{MEDIUM}\",\n\tsolid = \
         false,\n\toccludes = false,\n\topacity = 0.5,\n{stated}}}\n"
    )
}

/// The wall's declaration: an ordinary opaque block, saying nothing about either
/// occlusion or opacity.
#[must_use]
pub fn wall_declaration() -> String {
    format!("return {{\n\tname = \"{WALL}\",\n\ttexture = \"{WALL}\",\n\tsolid = true,\n}}\n")
}

/// A copy of the shipped root declaring these two blocks and no others, the
/// medium carrying `tint`.
///
/// # Errors
///
/// Returns the copy or write failure.
pub fn a_root_whose_medium_declares(
    tint: Option<(f32, [u8; 3])>,
) -> Result<ContentRoot, Box<dyn Error>> {
    shipped_copy()?
        .declaring_no_blocks()?
        .declaring_block(WALL_FILE, &wall_declaration())?
        .declaring_block(MEDIUM_FILE, &medium_declaring(tint))
}

/// A client playing that world over `root`, the handle its content changes are
/// reported through, and the sections it meshed at launch.
pub struct Playing {
    pub client: InputHarness,
    pub reports: Reports,
    pub meshed: Vec<SectionQuads>,
}

/// A client standing inside the medium over `root`.
///
/// # Errors
///
/// Returns the root's own refusal, the world's, the mesher's or the spawn's.
pub fn a_client_standing_in_it(root: &ContentRoot) -> Result<Playing, Box<dyn Error>> {
    let at_launch = registry_of(root.path())?;
    let meshed = World::new(a_medium_over_a_floor(&at_launch)?, at_launch)?.mesh()?;
    let (client, reports) = a_client_over(root, standing_at(SPAWN), |registry| {
        a_medium_over_a_floor(registry)
    })?;
    Ok(Playing {
        client,
        reports,
        meshed,
    })
}

/// One footprint whose floor is the wall block, whose next twenty-one layers are
/// the medium up to the wall, and whose layer at [`WALL_PLANE`] is the wall.
///
/// # Errors
///
/// Returns an error if a name does not parse or the world refuses a write.
pub fn a_medium_over_a_floor(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = VoxelWorld::empty(COLUMNS);
    let medium = BlockName::parse(MEDIUM)?;
    let wall = BlockName::parse(WALL)?;
    for (x, z) in every_cell() {
        blocks.set_block(WorldPos { x, y: FLOOR, z }, &wall, registry)?;
    }
    for (y, z) in (FLOOR + 1..CEILING).flat_map(|y| (0..ACROSS).map(move |z| (y, z))) {
        write_row(&mut blocks, registry, (y, z), (&medium, &wall))?;
    }
    Ok(blocks)
}

/// Every cell of one layer, in a declared order.
fn every_cell() -> impl Iterator<Item = (u32, u32)> {
    (0..ACROSS).flat_map(|x| (0..ACROSS).map(move |z| (x, z)))
}

/// One row of the band: the medium up to the wall, and then the wall's own
/// layer.
fn write_row(
    blocks: &mut VoxelWorld,
    registry: &BlockRegistry,
    at: (u32, u32),
    held: (&BlockName, &BlockName),
) -> Result<(), Box<dyn Error>> {
    let ((y, z), (medium, wall)) = (at, held);
    for x in 0..WALL_PLANE {
        blocks.set_block(WorldPos { x, y, z }, medium, registry)?;
    }
    blocks.set_block(
        WorldPos {
            x: WALL_PLANE,
            y,
            z,
        },
        wall,
        registry,
    )?;
    Ok(())
}

/// How far the wall's own face stands from the eye `published` camera reports.
///
/// **Subtracted from the published camera rather than assumed**, so a player the
/// physics moved between the launch and the reading is reported as a distance
/// that is not six rather than silently mispredicted.
#[must_use]
pub fn wall_stands_from(published: &SimSnapshot) -> f32 {
    WALL_PLANE as f32 - published.camera.eye[0]
}

/// A layer of one colour for each of the two blocks.
///
/// # Errors
///
/// Returns the key parse failure.
pub fn flat_layers() -> Result<SuppliedTexels, Box<dyn Error>> {
    Ok(SuppliedTexels::stating(vec![
        (TextureKey::parse(WALL)?, filled(WALL_COLOUR)),
        (TextureKey::parse(MEDIUM)?, filled(MEDIUM_COLOUR)),
    ]))
}

/// One layer's worth of texels, every one of them `colour` at full alpha.
fn filled(colour: [u8; 3]) -> Vec<[u8; 4]> {
    let [red, green, blue] = colour;
    vec![
        [red, green, blue, u8::MAX];
        (mc_core::content::TEXTURE_EDGE * mc_core::content::TEXTURE_EDGE) as usize
    ]
}

/// The frame `meshed` draws when packed against `resolution` and seen through
/// `published`, or `None` when the opt-in permitted the absence of a device.
///
/// **The camera and the tint both come off the published snapshot**, which is
/// the pair `App::snapshot` hands the renderer. Nothing here states either.
///
/// # Errors
///
/// Returns the packing, pipeline, upload or capture failure.
pub fn drawn_through(
    meshed: &[SectionQuads],
    resolution: &TextureResolution,
    published: &SimSnapshot,
) -> Result<Option<Rgba8Image>, Box<dyn Error>> {
    let scene = Arc::new(scene_of(meshed, resolution)?);
    let texels = flat_layers()?;
    let Some(context) = crate::support::frames::device()? else {
        return Ok(None);
    };
    let mut renderer = TerrainRenderer::new(
        context.device(),
        context.queue(),
        &TerrainPassConfig::offscreen(),
        &TerrainTextures {
            supplied: &texels,
            sampler: TERRAIN_SAMPLER,
        },
    )?;
    renderer.upload_textures(context.queue(), resolution.layers())?;
    renderer.upload_scene(context.queue(), &scene)?;
    let snapshot = TerrainSnapshot {
        tick: published.tick,
        camera: camera_view(published.camera.eye, published.camera.target),
        scene: Arc::clone(&scene),
        tint: published.tint,
    };
    Ok(Some(captured(&context, &mut renderer, &snapshot)?))
}

/// Every pixel of the declared cluster that `frame` drew away from the colour
/// `published`'s own tint and camera predict for the wall.
///
/// # Errors
///
/// Returns an error for a pixel outside `frame`, or the distance metric's own.
pub fn straying_from_the_wall(
    frame: &Rgba8Image,
    published: &SimSnapshot,
) -> Result<Vec<String>, Box<dyn Error>> {
    let expected = carried_wall(published);
    let mut strayed = Strays::default();
    for pixel in cluster() {
        let drawn = pixel_color(frame, pixel)?;
        let stands = distance(drawn, expected)?;
        if stands <= TELLS_THEM_APART {
            continue;
        }
        strayed.note(format!(
            "{pixel:?} drew {drawn:?} where {expected:?} was predicted, ΔE {stands:.2} away"
        ));
    }
    Ok(strayed.named())
}

/// The colour the wall is drawn at, from the tint and the camera `published`
/// carries.
///
/// The law in linear light, through the transfer pair `support::art` declares
/// from IEC 61966-2-1 — which shares no code with the draw path.
#[must_use]
pub fn carried_wall(published: &SimSnapshot) -> [u8; 3] {
    crate::support::composite::carried(WALL_COLOUR, published.tint, wall_stands_from(published))
}

/// The pixels the declared cluster examines, left to right and then down.
fn cluster() -> Vec<(u32, u32)> {
    let centre = THE_CENTRE;
    (0..9)
        .flat_map(|row| (0..9).map(move |column| (row, column)))
        .map(|(row, column)| {
            (
                centre.0 - CLUSTER_REACH + column * CLUSTER_STEP,
                centre.1 - CLUSTER_REACH + row * CLUSTER_STEP,
            )
        })
        .collect()
}

/// What a capture reports when the draw work never ran at all.
const DRAW_WORK_NEVER_RAN: &str = "the capture returned a frame without ever running the draw work, so every pixel read back \
     would be about a target nothing drew into";

/// The frame `snapshot` draws, at the declared capture size.
fn captured(
    context: &CaptureContext,
    renderer: &mut TerrainRenderer,
    snapshot: &TerrainSnapshot,
) -> Result<Rgba8Image, Box<dyn Error>> {
    let phase = ScenePhase::Ready(Arc::clone(&snapshot.scene));
    let mut ran = false;
    let image;
    {
        let mut work = draw_fn(|encoder, color| {
            let target = RecordTarget {
                device: context.device(),
                queue: context.queue(),
                encoder,
                color,
                size: CAPTURE_SIZE,
            };
            renderer.record_terrain(target, &phase, snapshot)?;
            ran = true;
            Ok(())
        });
        image = context
            .capture(&request(context, "reloaded-tint")?, &mut work)?
            .image;
    }
    if !ran {
        return Err(DRAW_WORK_NEVER_RAN.into());
    }
    Ok(image)
}

/// The reach a launch declares before any of these readings edits it.
pub const AT_LAUNCH: f32 = REACHES_AT;
