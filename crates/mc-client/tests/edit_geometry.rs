//! An edit becomes geometry: what the renderer is handed after a block is
//! broken or placed, against what it was handed before it was.
//!
//! # The observation point is the scene, and it has to be
//!
//! Every scenario here prepares the replay exactly as the client does, drives
//! one request through the simulation's own tick, takes whatever that tick left
//! to re-mesh, splices the result back into the list the preparation retained,
//! and packs that list into a scene. The comparison is then between two scenes
//! the renderer could be handed. Meshing the post-edit world directly would
//! answer the same question while walking straight past the marking that decides
//! *which* sections get re-meshed — which is the one thing this phase adds, and
//! an assertion that never asks for the re-mesh work is not grading it.
//!
//! A tick that marked nothing hands back the geometry that was already there.
//! That is deliberate: it is what an edit nothing noticed produces, and the
//! assertions below have to be able to see it rather than fail on a missing
//! value.
//!
//! # What a scene lets a test see
//!
//! A `SceneGeometry` publishes one record per section — where its near corner
//! sits, and how many quads it holds. That is per-section resolution, which is
//! exactly what these scenarios need: "the edited section changed" and "the
//! section beside it changed" are different claims, and a single whole-scene
//! difference could not tell them apart.
//!
//! # The landmark pillar, and why every count below is derivable
//!
//! Greedy meshing merges adjacent coplanar faces of the same block, so a quad
//! count is only a statement about faces where merging is ruled out. The
//! declared world has one place where it is ruled out completely. The landmark
//! pillar stands in block column (12, 12) — inside column (0, 0), so its
//! sections' near corners are (0, y, 0) — and its stone reaches y = 64, while no
//! surface in the world stands higher than 48.
//!
//! - **The section at (0, 64, 0)** therefore holds exactly one solid voxel, the
//!   pillar's topmost. It shows five of its six faces; the sixth abuts the
//!   pillar's next block down, which lives in the section beneath. Nothing else
//!   in the section emits a face, so none of the five merges with anything and
//!   the section carries five quads. Break it and the cell holds the world's own
//!   empty block, which is not solid: the section carries none. There is nothing
//!   else in it for the count to be about.
//! - **The section at (0, 48, 0)** is filled by the same pillar from y = 48 to
//!   y = 63 and by nothing else, so it shows one merged run per horizontal
//!   facing — four quads — with its top and bottom faces buried. The break above
//!   uncovers the upward face at y = 63, which is the only solid voxel of that
//!   plane in the section and so cannot merge with anything: four becomes five.
//! - **A placement against the pillar's upward face** lands at (12, 65, 12), in
//!   the same section again. The block placed is derived rather than named, and
//!   it must differ from the one it stands on — two runs of the same block merge
//!   along the shared plane, and a merged pair would leave the count unable to
//!   tell a placement from nothing at all. The pillar's top then keeps its four
//!   sides and loses its upward face to what stands on it, and the placed block
//!   shows five of its own six. Four and five.
//!
//! # Where the footprint ends
//!
//! Block column (0, 0) is the −x/−z corner of the loaded footprint; its surface
//! stands at y = 34 with open sky over it. A block at (0, 35, 0) lies on the
//! outermost layer of its section along both −x and −z, and neither of those two
//! neighbouring sections exists.
//!
//! Breaking it puts the world back precisely as it was — the cell held the
//! world's empty block before the placement and holds it again after the break —
//! so the scene handed afterwards has to be the scene handed at startup, quad
//! for quad. That equality is what "the block's faces are absent" means there,
//! and it is exact rather than derived. The placement before it is the control
//! and is not optional: without it, a client that re-meshed nothing at all would
//! satisfy the equality by never having done anything.
//!
//! # The aim, and why it is a block column and not a coordinate
//!
//! Every player below floats over the block column it edits and looks down at
//! the declared pitch limit rather than at the vertical, where a view has no
//! unique matrix. One degree off vertical drifts the ray `tan 1° = 0.0175`
//! blocks sideways for every block it falls, which over the 3.6 blocks the
//! furthest of these aims drops is 0.07 — the ray starts over the centre of its
//! block column and stays inside it. Both aims are well inside the reach of 5.0
//! blocks measured from the eye, and the one tick of falling either player does
//! before its request resolves moves the eye by 0.01.
//!
//! # No device, no window, no thread
//!
//! The scene is packed by the same function the client's own preparation packs
//! it with, called directly.

mod support;

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_client::startup::scene_of;
use mc_core::block::{BlockId, BlockRegistry};
use mc_core::id::BlockName;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::texture::TextureLayers;
use mc_sim::action::{ActionIntent, EditReport, TickIntent};
use mc_sim::player::{BlockPos, MovementIntent, PlayerState};
use mc_sim::replay::{SectionQuads, remesh, splice};
use mc_sim::simulation::Simulation;
use mc_sim::world::World;
use mc_world::section::Contents;

use support::TestResult;

/// How far below level every player here looks, in degrees.
///
/// The declared pitch limit and not the vertical: straight down a look
/// direction has no horizontal component and no unique view matrix, and the
/// simulation clamps a view to this value in any case.
const LOOKING_DOWN_DEGREES: f32 = -89.0;

/// The landmark pillar's topmost block, and where a player stands to look down
/// at it.
///
/// Three blocks of clear air over the pillar: high enough that the block a
/// placement lands in is nowhere near the player's own box, low enough that the
/// eye is 3.6 blocks from the face it meets.
const THE_LANDMARKS_TOP: BlockPos = BlockPos {
    x: 12,
    y: 64,
    z: 12,
};
const ABOVE_THE_LANDMARK: Vec3 = Vec3::new(12.5, 67.0, 12.5);

/// The cell a placement against the landmark's upward face lands in.
const ON_TOP_OF_THE_LANDMARK: BlockPos = BlockPos {
    x: THE_LANDMARKS_TOP.x,
    y: THE_LANDMARKS_TOP.y + 1,
    z: THE_LANDMARKS_TOP.z,
};

/// Where the two sections the landmark's top block belongs to have their near
/// corners, which is how a scene records a section.
const THE_LANDMARKS_SECTION: [i32; 3] = [0, 64, 0];
const THE_SECTION_BENEATH_IT: [i32; 3] = [0, 48, 0];

/// How many faces the landmark's topmost block shows: five of its six, the
/// downward one buried against the pillar it stands on.
const THE_LANDMARKS_TOP_SHOWS: u32 = 5;

/// How many quads the section beneath shows: one merged run per horizontal
/// facing over the pillar's sixteen blocks, its top and bottom both buried.
const THE_PILLAR_SHOWS_BELOW: u32 = 4;

/// How many faces a block standing on the landmark's top shows: five of six
/// again, the downward one buried against what it stands on.
const A_BLOCK_STANDING_ON_IT_SHOWS: u32 = 5;

/// The corner of the loaded footprint: the surface block column (0, 0), the
/// open cell over it, and where a player stands to look down at both.
const BENEATH_THE_FOOTPRINTS_CORNER: BlockPos = BlockPos { x: 0, y: 34, z: 0 };
const AT_THE_FOOTPRINTS_CORNER: BlockPos = BlockPos {
    x: BENEATH_THE_FOOTPRINTS_CORNER.x,
    y: BENEATH_THE_FOOTPRINTS_CORNER.y + 1,
    z: BENEATH_THE_FOOTPRINTS_CORNER.z,
};
const ABOVE_THE_FOOTPRINTS_CORNER: Vec3 = Vec3::new(0.5, 37.0, 0.5);

#[test]
fn a_broken_blocks_faces_are_absent_from_the_geometry_handed_after_the_edit() -> TestResult {
    let mut handed = Handed::over(ABOVE_THE_LANDMARK)?;
    let before = quads_in(handed.scene(), THE_LANDMARKS_SECTION);
    let report = handed.act(ActionIntent::Break)?;

    assert_eq!(
        (
            changed_cell(report),
            before,
            quads_in(handed.scene(), THE_LANDMARKS_SECTION)
        ),
        (
            Some(THE_LANDMARKS_TOP),
            Some(THE_LANDMARKS_TOP_SHOWS),
            Some(0)
        ),
        "the section the landmark's top block stands in holds that block's faces and nothing \
         else, so a break that reached the renderer empties it. A world that is edited and never \
         re-meshed leaves the player digging into a picture that never changes, which is the whole \
         of this phase: the count stays where it was and the hole is invisible"
    );
    Ok(())
}

#[test]
fn a_placed_blocks_faces_are_present_in_the_geometry_handed_after_the_edit() -> TestResult {
    let mut handed = Handed::over(ABOVE_THE_LANDMARK)?;
    let held = handed.builds_with(THE_LANDMARKS_TOP)?;
    let before = quads_in(handed.scene(), THE_LANDMARKS_SECTION);
    let report = handed.act(ActionIntent::Place { block: held })?;

    assert_eq!(
        (
            changed_cell(report),
            before,
            quads_in(handed.scene(), THE_LANDMARKS_SECTION)
        ),
        (
            Some(ON_TOP_OF_THE_LANDMARK),
            Some(THE_LANDMARKS_TOP_SHOWS),
            Some(THE_LANDMARKS_TOP_SHOWS - 1 + A_BLOCK_STANDING_ON_IT_SHOWS)
        ),
        "a block built on the landmark buries the upward face it stands on and shows five of its \
         own, and the two blocks differ so neither run merges into the other. A client that \
         re-meshed nothing hands the renderer a scene the new block is simply not in, and the \
         player builds into thin air"
    );
    Ok(())
}

#[test]
fn breaking_a_block_on_a_sections_outermost_layer_uncovers_the_face_in_the_section_beside_it()
-> TestResult {
    let mut handed = Handed::over(ABOVE_THE_LANDMARK)?;
    let before = quads_in(handed.scene(), THE_SECTION_BENEATH_IT);
    let report = handed.act(ActionIntent::Break)?;

    assert_eq!(
        (
            changed_cell(report),
            before,
            quads_in(handed.scene(), THE_SECTION_BENEATH_IT)
        ),
        (
            Some(THE_LANDMARKS_TOP),
            Some(THE_PILLAR_SHOWS_BELOW),
            Some(THE_PILLAR_SHOWS_BELOW + 1)
        ),
        "the block broken sits on the lowest layer of its section, and the face it was covering \
         belongs to the section under it — the only solid voxel of its own plane there, so the \
         face it uncovers is one quad and merges with nothing. A client that re-meshed only the \
         section it edited leaves that face missing and the world showing a window into itself, \
         with every assertion about the edited section still green"
    );
    Ok(())
}

#[test]
fn breaking_a_block_where_the_footprint_ends_leaves_its_faces_absent_and_reports_no_error()
-> TestResult {
    let mut handed = Handed::over(ABOVE_THE_FOOTPRINTS_CORNER)?;
    let untouched = handed.scene().clone();
    let held = handed.builds_with(BENEATH_THE_FOOTPRINTS_CORNER)?;
    let built = handed.act(ActionIntent::Place { block: held })?;
    let with_the_block = handed.scene().clone();
    let broken = handed.act(ActionIntent::Break)?;

    assert!(
        with_the_block != untouched,
        "the control this scenario cannot do without: the block has to have reached the renderer \
         before its absence means anything, or a client that re-meshes nothing satisfies the \
         equality below by never having changed the scene at all"
    );
    assert_eq!(
        (
            changed_cell(built),
            changed_cell(broken),
            handed.scene() == &untouched
        ),
        (
            Some(AT_THE_FOOTPRINTS_CORNER),
            Some(AT_THE_FOOTPRINTS_CORNER),
            true
        ),
        "the cell lies on the outermost layer of its section along both −x and −z, and neither of \
         those neighbouring sections exists. Breaking the block puts the world back exactly as it \
         was, so the scene has to come back quad for quad — and the absent neighbours have to be \
         passed over rather than reported, since every step from the re-mesh to the packing is \
         propagated here and any refusal fails this test outright"
    );
    Ok(())
}

/// The client's own preparation, a simulation standing in the same world, and
/// the geometry the renderer has been handed so far.
#[derive(Debug)]
struct Handed {
    simulation: Simulation,
    meshed: Vec<SectionQuads>,
    layers: TextureLayers,
    registry: Arc<BlockRegistry>,
    scene: SceneGeometry,
}

impl Handed {
    /// A prepared replay with a player floating over `feet`, looking down.
    ///
    /// The simulation edits a copy of the same blocks the scene was meshed from,
    /// which is what makes the two comparable at all.
    fn over(feet: Vec3) -> Result<Self, Box<dyn Error>> {
        let prepared = support::prepare_scene()?;
        let world = World::new(
            prepared.world.blocks().clone(),
            Arc::clone(&prepared.registry),
        )?;
        let content = support::published_content(&prepared.registry)?;
        Ok(Self {
            simulation: Simulation::new(looking_down_from(feet), world, content),
            meshed: prepared.meshed,
            layers: prepared.layers,
            registry: prepared.registry,
            scene: prepared.scene,
        })
    }

    /// The geometry the renderer has been handed.
    fn scene(&self) -> &SceneGeometry {
        &self.scene
    }

    /// Advances one tick asking the world for `action`, re-meshes whatever that
    /// tick left dirty, splices it back where it came from and hands the
    /// renderer the scene that results.
    ///
    /// A tick that left nothing dirty re-hands the geometry that was already
    /// there.
    fn act(&mut self, action: ActionIntent) -> Result<Option<EditReport>, Box<dyn Error>> {
        let report = self.simulation.advance(TickIntent {
            movement: MovementIntent::default(),
            action: Some(action),
        });
        if let Some(work) = self.simulation.take_remesh_work() {
            splice(&mut self.meshed, remesh(&work)?)?;
        }
        self.scene = scene_of(&self.meshed, &self.layers)?;
        Ok(report)
    }

    /// The block a placement in these fixtures asks for: a solid one the
    /// prepared world already shows, other than the one held at `standing_on`.
    ///
    /// Derived and never named, because two separate constraints meet in it. The
    /// texture layers a scene is packed against are assigned from the *initially
    /// meshed* quads' keys, so a block whose faces appear nowhere in the prepared
    /// world has no layer to draw from and would fail the packing rather than the
    /// assertion. And a block sharing the name of the one it stands on merges its
    /// faces into that one's along the plane they share, which is precisely the
    /// arithmetic these counts are derived under.
    fn builds_with(&self, standing_on: BlockPos) -> Result<BlockName, Box<dyn Error>> {
        // Three answers and three arms. The fixture is standing on something, so
        // both of the other two are the fixture being wrong about itself — and
        // they are wrong in different ways, which a single refusal would hide.
        let standing = match self.block_at(standing_on) {
            None => {
                return Err(
                    "the fixture's world reaches no cell where the placement stands".into(),
                );
            }
            Some(Contents::Empty) => {
                return Err("the fixture's world holds nothing where the placement stands".into());
            }
            Some(Contents::Holds(name)) => name.clone(),
        };
        self.a_visible_solid_block_other_than(&standing).ok_or_else(|| {
            "the prepared world shows no solid block besides the one built on, so nothing can be \
             placed that a scene has a texture layer for"
                .into()
        })
    }

    /// What `cell` holds, as the simulation's own world reads it.
    fn block_at(&self, cell: BlockPos) -> Option<Contents<&BlockName>> {
        self.simulation.world().block_at(cell)
    }

    /// The first solid block in registration order whose faces the prepared
    /// world already shows somewhere, skipping `standing`.
    fn a_visible_solid_block_other_than(&self, standing: &BlockName) -> Option<BlockName> {
        let shown: BTreeSet<&str> = self
            .meshed
            .iter()
            .flat_map(|section| section.quads.iter())
            .map(|quad| quad.block.as_str())
            .collect();
        (0..self.registry.registered_count())
            .filter_map(|raw| u32::try_from(raw).ok())
            .filter_map(|raw| self.registry.definition(BlockId::from_raw(raw)).ok())
            .find(|definition| {
                definition.is_solid
                    && &definition.name != standing
                    && shown.contains(definition.name.as_str())
            })
            .map(|definition| definition.name.clone())
    }
}

/// A player standing still at `feet`, looking down the declared pitch limit.
fn looking_down_from(feet: Vec3) -> PlayerState {
    PlayerState {
        position: feet,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: LOOKING_DOWN_DEGREES.to_radians(),
        on_ground: false,
    }
}

/// How many quads a scene holds for the section whose near corner is `origin`.
fn quads_in(scene: &SceneGeometry, origin: [i32; 3]) -> Option<u32> {
    scene
        .sections()
        .iter()
        .find(|record| record.origin == origin)
        .map(|record| record.quad_count)
}

/// The cell one report says a block changed in.
///
/// A refusal contributes nothing: it is an answer to a question that was asked,
/// and what these scenarios are about is the block that changed.
fn changed_cell(report: Option<EditReport>) -> Option<BlockPos> {
    match report? {
        EditReport::Changed { cell, .. } => Some(cell),
        EditReport::Refused(_) => None,
    }
}
