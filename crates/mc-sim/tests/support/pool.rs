//! A pool of the shipped water, deep enough and wide enough that a *rate* has
//! somewhere to happen.
//!
//! **It resolves the real content root and not a fixture registry.** The
//! scenarios this serves are about what `content/base/blocks/water.luau`
//! declares, so the declaration has to arrive by the door content arrives
//! through: [`super::content_registry`] reads the shipped root, and a volume
//! naming `base:water` and `base:stone` is resolved through it. A registry
//! assembled in Rust would be this fixture agreeing with a copy of the
//! declaration rather than with the declaration.
//!
//! **Neither fixture already in this suite can carry them, and it is worth
//! saying why rather than leaving a reader to wonder at a third one.**
//! `super::medium` declares its own `fixture:` blocks, so nothing it resolves is
//! the shipped water at all. `super::sea` stands on the generated world, whose
//! deepest column is two water voxels — less water than one second of rise
//! crosses, so a rate stated over a second has nowhere to happen in it.
//! `super::chamber` answers `VoxelMedium::NOTHING` for every voxel it holds, on
//! purpose, so a player inside it is never in any medium.
//!
//! **Every position this module hands out is guarded rather than assumed.** A
//! swimmer is refused unless the volume its box stands in is one a player can
//! hold itself up in, and unless [`CLEARANCE`] blocks of water stand between the
//! box and anything solid on every side the scenario is not about. A fixture that
//! quietly began against a wall, or half out of the water, would measure
//! collision or air and report a clean pass.
//!
//! **The box's shape is mirrored here and the mirror is not asserted.**
//! [`HALF_WIDTH`] and [`HEIGHT`] are private to the physics; they are restated so
//! the clearance guard can say what "one block clear of every wall" means. They
//! are used only to make the guard *refuse*, never to compute an expected
//! answer, so a mirror that drifted costs a scenario its guard and never its
//! oracle — and the pool is declared far larger than the guard demands, which is
//! what keeps that cost theoretical.

use std::error::Error;

use glam::Vec3;
use mc_sim::player::{BlockPos, Medium, PlayerState, Solidity, VoxelMedium};
use mc_sim::replay::{Extent, ResolvedVoxels};
use mc_world::world::WorldPos;

use super::volume::Cells;
use super::{STONE, WATER, content_registry};

/// How far the declared volume reaches on each axis.
///
/// Cubic and comfortably larger than anything asserted inside it: a walk of
/// three blocks and a rise of two both have to finish well inside the water,
/// and a fixture sized to the answer would start reporting the wall the day an
/// answer moved.
const EXTENT: Extent = Extent {
    x: 24,
    y: 24,
    z: 24,
};

/// How far the player's box reaches from the feet centre on x and z, in blocks.
///
/// The physics' own [`HALF_WIDTH`], restated — see this module's doc for why the
/// mirror is safe here and what it would and would not cost if it drifted.
const HALF_WIDTH: f32 = 0.3;

/// How tall the player's box is, in blocks. The physics' own, restated.
const HEIGHT: f32 = 1.8;

/// The top face of the pool's floor: where a player standing on it rests its
/// feet.
pub const FLOOR_TOP: u32 = 8;

/// The bottom face of the pool's ceiling, and so the top face of its water.
pub const CEILING_BOTTOM: u32 = 20;

/// The first column the water fills and the first one past it, on both
/// horizontal axes.
///
/// Twenty columns of water with two columns of rock outside them on each side,
/// so a wall is a wall rather than the edge of the volume — a box that walked
/// out of the extent would meet nothing at all, and "nothing" is not what a
/// chamber's wall answers.
const WATER_SPAN: (u32, u32) = (2, 22);

/// How much open water this module requires between a box it calls clear and
/// anything solid, in blocks.
pub const CLEARANCE: f32 = 1.0;

/// Where a swimmer clear of the floor, the ceiling and every wall begins.
///
/// Off-lattice on both horizontal axes so no reading is taken from a coordinate
/// that flatters the arithmetic, and four blocks over the floor with six above
/// the box — far more than [`CLEARANCE`] demands, because a sink of one block
/// and a rise of two both have to complete without meeting anything.
pub const CLEAR_WATER: Vec3 = Vec3::new(5.5, 12.0, 5.5);

/// Where a submerged player standing on the pool's floor begins.
pub const ON_THE_FLOOR: Vec3 = Vec3::new(5.5, FLOOR_TOP as f32, 5.5);

/// How fast a swimmer [`Pool::afloat_and_sinking`] hands over is already
/// falling, in blocks per second.
///
/// **Non-zero on purpose, and the reason is measured rather than stylistic.** A
/// scenario about what a held jump *sets* the vertical velocity to cannot tell
/// "the launch replaced what was there" from "the launch was skipped and a zero
/// was carried forward" when the fixture began at zero — phase 2 measured both
/// mutations of the launch signature leaving two such scenarios green, and the
/// one scenario that began at `−2.0` reddening under one of them. A correct
/// launch discards this entirely, so the expected answer is the same number
/// either way.
pub const A_SINK_ALREADY_UNDER_WAY: f32 = -2.0;

/// How fast a walker [`Pool::standing_on_the_floor_drifting_backwards`] hands
/// over is already moving *against* the walk it is about to be asked for, in
/// blocks per second.
///
/// The same argument one axis over: a horizontal velocity is *set* by the walk
/// each tick rather than accumulated, and a fixture starting at zero cannot see
/// a tick that failed to set it. Pointing it the wrong way makes a walk that
/// merely kept what it was handed land three blocks behind where it started
/// rather than near it.
pub const A_DRIFT_THE_WRONG_WAY: f32 = -3.0;

/// The shipped water, declared deep enough and wide enough to measure a rate in.
#[derive(Debug)]
pub struct Pool {
    /// The view a tick reads, resolved through the shipped registry.
    pub voxels: ResolvedVoxels,
}

impl Pool {
    /// A swimmer at rest, its box clear of the floor, the ceiling and every
    /// wall.
    ///
    /// At rest because the closed form a sink is stated against is the one for a
    /// fall that begins at zero; a fixture handed any other velocity would be
    /// asserted against a different sum.
    ///
    /// # Errors
    ///
    /// Returns an error if that box does not stand in water, or if anything
    /// solid lies within [`CLEARANCE`] of it.
    pub fn afloat_at_rest(&self) -> Result<PlayerState, Box<dyn Error>> {
        self.require_open_water(CLEAR_WATER)?;
        Ok(adrift(CLEAR_WATER, 0.0))
    }

    /// The same swimmer in the same place, already sinking at
    /// [`A_SINK_ALREADY_UNDER_WAY`].
    ///
    /// # Errors
    ///
    /// Returns an error if that box does not stand in water, or if anything
    /// solid lies within [`CLEARANCE`] of it.
    pub fn afloat_and_sinking(&self) -> Result<PlayerState, Box<dyn Error>> {
        self.require_open_water(CLEAR_WATER)?;
        Ok(adrift(CLEAR_WATER, A_SINK_ALREADY_UNDER_WAY))
    }

    /// A submerged player standing on the pool's floor, already drifting at
    /// [`A_DRIFT_THE_WRONG_WAY`] along the axis it is about to be asked to walk.
    ///
    /// # Errors
    ///
    /// Returns an error if that box does not stand in water, if anything solid
    /// lies within [`CLEARANCE`] of it to the sides or above, or if nothing
    /// solid is holding it up.
    pub fn standing_on_the_floor_drifting_backwards(&self) -> Result<PlayerState, Box<dyn Error>> {
        self.require_water_over_a_floor(ON_THE_FLOOR)?;
        Ok(PlayerState {
            position: ON_THE_FLOOR,
            velocity: Vec3::new(A_DRIFT_THE_WRONG_WAY, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            on_ground: true,
        })
    }

    /// Refuses unless a state a run of ticks produced still stands in water.
    ///
    /// The fixture's own bound, checked at the *end* rather than assumed from
    /// the start: a rise that left the pool, or a walk carried out of it, would
    /// otherwise be asserted against a distance part of which happened in air.
    ///
    /// # Errors
    ///
    /// Returns an error if the box no longer stands in a volume a player can
    /// hold itself up in.
    pub fn require_still_swimming(&self, state: PlayerState) -> Result<(), Box<dyn Error>> {
        self.require_swimmable(state.position, "ended")
    }

    /// Refuses unless the box at `feet` stands in water with [`CLEARANCE`]
    /// blocks of it on every side.
    fn require_open_water(&self, feet: Vec3) -> Result<(), Box<dyn Error>> {
        self.require_swimmable(feet, "begins")?;
        self.require_nothing_solid(feet, Vec3::splat(CLEARANCE), Vec3::splat(CLEARANCE))
    }

    /// Refuses unless the box at `feet` stands in water on a floor, with
    /// [`CLEARANCE`] blocks of water beside it and above it.
    fn require_water_over_a_floor(&self, feet: Vec3) -> Result<(), Box<dyn Error>> {
        self.require_swimmable(feet, "begins")?;
        self.require_nothing_solid(
            feet,
            Vec3::new(CLEARANCE, 0.0, CLEARANCE),
            Vec3::splat(CLEARANCE),
        )?;
        self.require_a_floor_under(feet)
    }

    /// Refuses unless the volume the box at `feet` stands in holds a player up.
    fn require_swimmable(&self, feet: Vec3, when: &str) -> Result<(), Box<dyn Error>> {
        let medium = self.folded(feet);
        if medium.swimmable {
            return Ok(());
        }
        Err(format!(
            "this fixture is about a player inside the shipped water, and the box whose feet are \
             at {feet} {when} in a volume nobody can hold itself up in ({medium:?}). What a \
             scenario over it would measure is air"
        )
        .into())
    }

    /// Refuses unless every voxel the box at `feet` covers, grown by `low` and
    /// `high`, holds nothing solid.
    fn require_nothing_solid(
        &self,
        feet: Vec3,
        low: Vec3,
        high: Vec3,
    ) -> Result<(), Box<dyn Error>> {
        let blocking: Vec<BlockPos> = voxels_around(feet, low, high)
            .filter(|at| self.voxels.is_solid(*at))
            .collect();
        if blocking.is_empty() {
            return Ok(());
        }
        Err(format!(
            "this fixture is about a player with open water around it, and the box whose feet are \
             at {feet} stands within {low}/{high} of {} solid voxels, the first of them {:?}. A \
             scenario over it would measure the wall it stopped against",
            blocking.len(),
            blocking.first()
        )
        .into())
    }

    /// Refuses unless something solid is directly under the box at `feet`.
    fn require_a_floor_under(&self, feet: Vec3) -> Result<(), Box<dyn Error>> {
        let under = BlockPos {
            x: floor_voxel(feet.x),
            y: floor_voxel(feet.y) - 1,
            z: floor_voxel(feet.z),
        };
        if self.voxels.is_solid(under) {
            return Ok(());
        }
        Err(format!(
            "this fixture is about a player standing on the pool's floor, and nothing solid lies \
             at {under:?} under the box whose feet are at {feet}. What it would measure is a \
             player falling"
        )
        .into())
    }

    /// The medium a tick would read from the box at `feet`.
    ///
    /// The same greatest-value fold the physics performs, over the same voxels —
    /// stated here rather than reached for, because the physics' own fold is
    /// private and a guard that could not perform it could only guess.
    fn folded(&self, feet: Vec3) -> VoxelMedium {
        voxels_around(feet, Vec3::ZERO, Vec3::ZERO)
            .map(|at| self.voxels.medium_at(at))
            .fold(VoxelMedium::NOTHING, VoxelMedium::with)
    }
}

/// The pool this module declares: rock everywhere, with the shipped water carved
/// out of it between the floor and the ceiling.
///
/// **Rock first and water over it**, rather than water with walls added, because
/// a later run wins: declaring the solid whole and then hollowing it is what
/// makes "everything outside the water is a wall" a property of one sentence
/// instead of six.
///
/// # Errors
///
/// Returns an error if the shipped content root cannot be read, if a declared
/// name is not a namespaced id, or if the registry does not know one of them.
pub fn a_pool_of_the_shipped_water() -> Result<Pool, Box<dyn Error>> {
    let registry = content_registry()?;
    let declared = Cells::empty(EXTENT)
        .holding(corner(0, 0, 0), corner(EXTENT.x, EXTENT.y, EXTENT.z), STONE)?
        .holding(
            corner(WATER_SPAN.0, FLOOR_TOP, WATER_SPAN.0),
            corner(WATER_SPAN.1, CEILING_BOTTOM, WATER_SPAN.1),
            WATER,
        )?;
    Ok(Pool {
        voxels: ResolvedVoxels::resolve(&declared, &registry)?,
    })
}

/// A world position, spelled short enough to sit inside a builder chain.
const fn corner(x: u32, y: u32, z: u32) -> WorldPos {
    WorldPos { x, y, z }
}

/// A player with nothing holding it up, at `at`, moving vertically at `rate`.
fn adrift(at: Vec3, rate: f32) -> PlayerState {
    PlayerState {
        position: at,
        velocity: Vec3::new(0.0, rate, 0.0),
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// Every voxel the box a player at `feet` carries covers, grown by `low` toward
/// its lower corner and by `high` toward its upper one.
///
/// A voxel fills `[v, v + 1)`, so the ones an interval `[min, max]` touches run
/// `floor(min)` up to and including `ceil(max) − 1` — the physics' own rule,
/// restated for the same reason the box's shape is.
fn voxels_around(feet: Vec3, low: Vec3, high: Vec3) -> impl Iterator<Item = BlockPos> {
    let min = feet - Vec3::new(HALF_WIDTH, 0.0, HALF_WIDTH) - low;
    let max = feet + Vec3::new(HALF_WIDTH, HEIGHT, HALF_WIDTH) + high;
    let (west, south, bottom) = (floor_voxel(min.x), floor_voxel(min.z), floor_voxel(min.y));
    let (east, north, top) = (ceil_voxel(max.x), ceil_voxel(max.z), ceil_voxel(max.y));
    (bottom..=top).flat_map(move |y| {
        (south..=north).flat_map(move |z| (west..=east).map(move |x| BlockPos { x, y, z }))
    })
}

/// The voxel a coordinate lies in.
fn floor_voxel(coordinate: f32) -> i32 {
    coordinate.floor() as i32
}

/// The last voxel an interval ending at `coordinate` touches.
fn ceil_voxel(coordinate: f32) -> i32 {
    coordinate.ceil() as i32 - 1
}
