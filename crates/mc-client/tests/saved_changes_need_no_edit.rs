//! A change a save holds is drawn wherever it is, and it is drawn because the
//! client resumed the save rather than because the player edited something.
//!
//! # The reported symptom, stated as a test
//!
//! What was reported is that a resumed world looks untouched until the first edit,
//! and then "everything appears at once". Measured, the second half is not what
//! happens: one edit marks its own section and the six around it, so what the
//! reporter saw appear was the whole of their own local digging. A saved change
//! further away than one section's neighbourhood is never repaired by editing at
//! all — which is why the column these scenarios watch is deliberately distant from
//! the column they edit in.
//!
//! # Why (60, 64, 60), and why exactly six quads
//!
//! The cell sits in chunk column (3, 3), so its section's near corner is
//! (48, 64, 48). It stands sixteen blocks above the highest surface the generator
//! produces anywhere, so a solid block there is alone in its section with nothing
//! adjacent in any direction and shows all six of its faces, none of them merged
//! with anything. In the world the seed makes, that section holds nothing at all and
//! carries no quads. Six or none, and no arithmetic in between.
//!
//! (63, 64, 63) is the same claim at the footprint's edge: it lies on the outermost
//! layer of its section along both +x and +z, and neither neighbouring column
//! exists. The absent neighbours have to be passed over — a preparation that sealed
//! the world off there would bury two of the six faces, and one that reported the
//! missing columns would refuse the launch outright.
//!
//! # The edit is real, and it is somewhere else
//!
//! The scenario about the first edit drives the simulation the launch handed back:
//! one tick asking to break the landmark pillar's top in chunk column (0, 0), then
//! the drain, re-mesh and splice the client itself performs, then the same packing.
//! Meshing the edited world directly would walk straight past the marking that
//! decides which sections are re-meshed, which is the very thing that must **not**
//! be what repairs the far column.
//!
//! That harness is copied from `edit_geometry.rs` rather than lifted out of it: that
//! file is a graded test of its own, and harmless duplication is cheaper than moving
//! a helper another suite's assertions rest on.
//!
//! # Nothing may be left outstanding
//!
//! Marking all 256 sections dirty at load and letting the frame path drain them
//! would satisfy every count here while shipping the reported defect: the first
//! frames would still be drawn from the generated world, one batch at a time. So one
//! scenario asserts a resumed session has nothing outstanding to re-mesh before the
//! player touches anything — with an edit afterwards as its positive control, since
//! "nothing outstanding" is also what a simulation that had stopped noticing edits
//! would report.

#[path = "support/handed.rs"]
mod handed;

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use mc_client::launch::{PreparedLaunch, prepare_launch};
use mc_client::startup::{PreparationError, scene_of};
use mc_core::block::{BlockId, BlockRegistry};
use mc_core::id::BlockName;
use mc_render::geometry::scene::SceneGeometry;
use mc_render::texture::TextureLayers;
use mc_sim::action::{ActionIntent, EditReport, TickIntent};
use mc_sim::player::{BlockPos, MovementIntent};
use mc_sim::replay::{SectionQuads, remesh, splice};
use mc_sim::simulation::Simulation;
use mc_world::persistence::{Acceptance, SavedPlayer};
use mc_world::world::WorldPos;

use handed::{AResumedWorld, TestResult, generated_blocks, resumed, shipped_content};

/// Every save here is written against the registry the same content root produces,
/// so nothing about its blocks can have changed and the acceptance decides nothing.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// How far below level the resumed player looks, in degrees.
///
/// The declared pitch limit and not the vertical: straight down a look direction has
/// no horizontal component and no unique view matrix, and the simulation clamps a
/// view to this value in any case.
const LOOKING_DOWN_DEGREES: f32 = -89.0;

/// The landmark pillar's topmost block, which is what the one edit here breaks, and
/// where the save stands the player to break it from.
///
/// Three blocks of clear air over the pillar: high enough that nothing lands in the
/// player's own box, low enough that the eye is 3.6 blocks from the face it meets,
/// which is well inside the declared reach of 5.0.
const THE_LANDMARKS_TOP: (u32, u32, u32) = (12, 64, 12);
const OVER_THE_LANDMARK: [f32; 3] = [12.5, 67.0, 12.5];

/// The cell a save holds a block in, far from the column the edit lands in: chunk
/// column (3, 3), sixteen blocks above the highest surface the generator makes.
const FAR_FROM_THE_EDIT: (u32, u32, u32) = (60, 64, 60);

/// The same claim at the footprint's edge — the outermost layer of the same section
/// along both +x and +z, with neither neighbouring column in existence.
const WHERE_THE_FOOTPRINT_ENDS: (u32, u32, u32) = (63, 64, 63);

/// Where the section both of them stand in has its near corner, which is how a scene
/// records a section.
const THE_FAR_SECTION: [i32; 3] = [48, 64, 48];

/// How many faces a solid block with nothing adjacent in any direction shows: all
/// six, none of them merged with anything.
const ALONE_IN_ITS_SECTION_SHOWS: u32 = 6;

/// How many sections a session that has re-meshed everything it owes still owes.
const NOTHING_OUTSTANDING: usize = 0;

#[test]
fn a_launch_resuming_a_save_draws_a_change_the_generated_world_never_held() -> TestResult {
    let content = shipped_content()?;
    let saved = a_save_holding_a_block_at(&content, FAR_FROM_THE_EDIT)?;

    let launched = prepare_launch(&content, &saved.save(), ACCEPTING);

    assert_eq!(
        (
            quads_in(&launched, THE_FAR_SECTION),
            saved.stored_at(FAR_FROM_THE_EDIT)
        ),
        (
            Ok(Some(ALONE_IN_ITS_SECTION_SHOWS)),
            Ok(saved.written_at(FAR_FROM_THE_EDIT))
        ),
        "the save holds a block at {FAR_FROM_THE_EDIT:?}, alone in its section and above everything \
         the generator draws, so the geometry a launch resuming that save hands over shows all six \
         of its faces. In the world the seed makes that section holds nothing and carries none, \
         which is what a launch that meshed the generated world hands over instead — and what the \
         player then walks through. The second half is the round trip: what the save holds on disk \
         is what this fixture wrote into it"
    );
    Ok(())
}

#[test]
fn an_edit_in_another_column_leaves_a_distant_saved_change_drawn() -> TestResult {
    let content = shipped_content()?;
    let saved = a_save_holding_a_block_at(&content, FAR_FROM_THE_EDIT)?;
    let mut handed = Handed::resuming(prepare_launch(&content, &saved.save(), ACCEPTING)?);

    let broke = handed.advance(ActionIntent::Break);
    handed.hand_over()?;

    assert_eq!(
        (
            quads_in_scene(handed.scene(), THE_FAR_SECTION),
            changed_cell(broke)
        ),
        (
            Some(ALONE_IN_ITS_SECTION_SHOWS),
            Some(cell_at(THE_LANDMARKS_TOP))
        ),
        "the player's first edit after resuming lands in chunk column (0, 0), and the saved block \
         it is asked about stands in chunk column (3, 3) — further away than the seven sections an \
         edit marks, so nothing about the edit could repair it even in principle. The expected \
         value is six and not \"whatever the launch handed over\", which is what makes this the \
         scenario saying the repair is not an edit's job: on a client that meshes the generated \
         world the section carries none before the edit and none after it, and the player who digs \
         where they stand still cannot see what they built across the world. The second half is the \
         control that the edit happened at all"
    );
    Ok(())
}

#[test]
fn a_launch_resuming_a_save_draws_a_change_on_its_sections_outermost_layer() -> TestResult {
    let content = shipped_content()?;
    let saved = a_save_holding_a_block_at(&content, WHERE_THE_FOOTPRINT_ENDS)?;

    let launched = prepare_launch(&content, &saved.save(), ACCEPTING);

    assert_eq!(
        (
            quads_in(&launched, THE_FAR_SECTION),
            saved.stored_at(WHERE_THE_FOOTPRINT_ENDS)
        ),
        (
            Ok(Some(ALONE_IN_ITS_SECTION_SHOWS)),
            Ok(saved.written_at(WHERE_THE_FOOTPRINT_ENDS))
        ),
        "the block sits on the outermost layer of its section along both +x and +z, and neither of \
         those neighbouring columns exists. Its faces there are shown, which means the absent \
         neighbours were passed over rather than sealed off — four quads would say the world had \
         been closed at its own edge — and rather than reported, since a refusal to mesh past the \
         footprint fails this test outright. It is the launch-time half of the boundary an edit is \
         already graded on"
    );
    Ok(())
}

#[test]
fn a_resumed_session_has_nothing_outstanding_to_re_mesh_before_the_first_edit() -> TestResult {
    let content = shipped_content()?;
    let saved = a_save_holding_a_block_at(&content, FAR_FROM_THE_EDIT)?;
    let mut handed = Handed::resuming(prepare_launch(&content, &saved.save(), ACCEPTING)?);

    let before_any_edit = handed.outstanding();
    let broke = handed.advance(ActionIntent::Break);
    let after_the_edit = handed.outstanding();

    assert_eq!(
        (before_any_edit, after_the_edit > 0, changed_cell(broke)),
        (NOTHING_OUTSTANDING, true, Some(cell_at(THE_LANDMARKS_TOP))),
        "the geometry a launch hands over has to be complete when it hands it over. Marking every \
         section of the resumed world dirty and letting the frame path drain them satisfies every \
         count in this suite and still ships the reported defect — a whole-world mesh moved onto \
         the render thread, one batch at a time, with the first frames drawn from the world the \
         player did not save. The two halves after the first are the positive control: this session \
         does still notice an edit, so the nothing it owes beforehand is a world already meshed \
         rather than a world nothing is watching"
    );
    Ok(())
}

/// What a launch came to: the preparation it produced, or the refusal it gave
/// instead.
type Launched = Result<PreparedLaunch, PreparationError>;

/// A save holding the generated world with one solid block standing at `at`.
fn a_save_holding_a_block_at(
    content: &Path,
    at: (u32, u32, u32),
) -> Result<AResumedWorld, Box<dyn Error>> {
    resumed(content, resting_over_the_landmark(), |registry| {
        let mut blocks = generated_blocks(registry)?;
        blocks.set_block(cell(at), &a_solid_block(registry)?, registry)?;
        Ok(blocks)
    })
}

/// The player these saves record: over the landmark, looking down at it.
///
/// The one edit in this suite is driven from wherever the save stood the player, so
/// this is the aim as well as the position. One degree off vertical drifts a ray
/// 0.0175 blocks sideways per block it falls, which over the 3.6 blocks this one
/// drops is 0.07 — it starts over the centre of the landmark's block column and
/// stays inside it.
fn resting_over_the_landmark() -> SavedPlayer {
    SavedPlayer {
        position: OVER_THE_LANDMARK,
        yaw: 0.0,
        pitch: LOOKING_DOWN_DEGREES.to_radians(),
    }
}

/// The first solid block the registry declares.
///
/// Derived rather than named: what the scenarios need is a block that shows faces,
/// and a named one would silently stop being solid the day the content says so.
fn a_solid_block(registry: &BlockRegistry) -> Result<BlockName, Box<dyn Error>> {
    (0..u32::try_from(registry.registered_count())?)
        .filter_map(|raw| registry.definition(BlockId::from_raw(raw)).ok())
        .find(|definition| definition.is_solid)
        .map(|definition| definition.name.clone())
        .ok_or_else(|| "the content root declares no solid block to save a change made of".into())
}

/// The simulation a launch handed back, the sections it retained, and the geometry
/// the renderer has been handed so far.
#[derive(Debug)]
struct Handed {
    simulation: Simulation,
    meshed: Vec<SectionQuads>,
    layers: TextureLayers,
    registry: Arc<BlockRegistry>,
    scene: SceneGeometry,
}

impl Handed {
    /// The client as one launch left it: playing the world it resumed, holding the
    /// sections a later edit splices into, and showing the scene it packed from
    /// them.
    fn resuming(prepared: PreparedLaunch) -> Self {
        Self {
            simulation: prepared.simulation,
            meshed: prepared.meshed,
            layers: prepared.layers,
            registry: prepared.registry,
            scene: prepared.scene,
        }
    }

    /// The geometry the renderer has been handed.
    const fn scene(&self) -> &SceneGeometry {
        &self.scene
    }

    /// Advances one tick asking the world for `action`, without re-meshing anything.
    fn advance(&mut self, action: ActionIntent) -> Option<EditReport> {
        self.simulation.advance(TickIntent {
            movement: MovementIntent::default(),
            action: Some(action),
        })
    }

    /// How many sections the session still owes a re-mesh, taking them as the frame
    /// path would.
    fn outstanding(&mut self) -> usize {
        self.simulation
            .take_remesh_work()
            .map_or(0, |work| work.keys().len())
    }

    /// Re-meshes whatever is outstanding, splices it back where it came from and
    /// hands the renderer the scene that results.
    ///
    /// Nothing outstanding re-hands the geometry that was already there, which is
    /// what a tick that marked nothing produces.
    fn hand_over(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(work) = self.simulation.take_remesh_work() {
            splice(&mut self.meshed, remesh(&work, &self.registry)?)?;
        }
        self.scene = scene_of(&self.meshed, &self.layers)?;
        Ok(())
    }
}

/// How many quads the scene a launch handed over holds for the section whose near
/// corner is `origin` — or the refusal the launch gave instead.
///
/// A refusal comes back as the failed comparison rather than as a propagated error,
/// so that "it refused to prepare anything" and "it prepared the wrong geometry" are
/// one failed assertion instead of two kinds of failure.
fn quads_in(launched: &Launched, origin: [i32; 3]) -> Result<Option<u32>, String> {
    let prepared = launched.as_ref().map_err(PreparationError::to_string)?;
    Ok(quads_in_scene(&prepared.scene, origin))
}

/// How many quads `scene` holds for the section whose near corner is `origin`.
///
/// Nothing where the scene carries no record for that section at all, which is a
/// different answer from a record carrying no quads and is kept apart from it
/// deliberately.
fn quads_in_scene(scene: &SceneGeometry, origin: [i32; 3]) -> Option<u32> {
    scene
        .sections()
        .iter()
        .find(|record| record.origin == origin)
        .map(|record| record.quad_count)
}

/// The cell one report says a block changed in.
///
/// A refusal contributes nothing: it is an answer to a question that was asked, and
/// what these scenarios need is the block that changed.
fn changed_cell(report: Option<EditReport>) -> Option<BlockPos> {
    match report? {
        EditReport::Changed { cell, .. } => Some(cell),
        EditReport::Refused(_) => None,
    }
}

/// A cell as the world spells a position.
const fn cell(at: (u32, u32, u32)) -> WorldPos {
    let (x, y, z) = at;
    WorldPos { x, y, z }
}

/// A cell as an edit report spells a position.
const fn cell_at(at: (u32, u32, u32)) -> BlockPos {
    let (x, y, z) = at;
    BlockPos {
        x: x as i32,
        y: y as i32,
        z: z as i32,
    }
}
