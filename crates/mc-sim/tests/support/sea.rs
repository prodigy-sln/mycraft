//! The sea the shipped world generates: which column a scenario about it stands
//! on, what the shipped water declares, and the same world resolved through a
//! registry in which nothing is a medium at all.
//!
//! **No number here is water's declared resistance.** The value is derived at
//! implementation from the built simulation and written in exactly one place —
//! `content/base/blocks/water.luau` — so every threshold a scenario carries is
//! arithmetic over the value read back out of the shipped registry. A literal
//! here would be a second statement of it, and the two would agree right up
//! until the day the declaration moved, which is the whole of what a derived
//! threshold is for.
//!
//! **A player is never placed at a computed height.** Every fixture is dropped
//! into open air over the column it is about and advanced under an intent that
//! asks for nothing until the world stops it; the height it stops at is then
//! *checked* against what the world says that column's lakebed is. So the world
//! decides where its lakebed is and the fixture only says whether it agrees — a
//! fixture that came to rest somewhere else refuses instead of quietly
//! asserting about the wrong place.
//!
//! **The deepest column is not unique and does not need to be.** The shipped sea
//! holds many columns at its greatest depth, and a player settles at the same
//! height on every one of them, so the tie-break below decides only *which* of
//! them a fixture stands on and never *what* it stands on.

use std::error::Error;

use glam::Vec3;
use mc_core::block::source::{DefinitionSourceError, InMemoryDefinitionSource};
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_sim::player::{MovementIntent, PlayerState, Traversal, advance_player};
use mc_sim::replay::{ReplayWorld, ResolvedVoxels};

use super::{
    SEA_LEVEL, WATER, block_at, block_name, content_registry, every_column, replay_world,
    surface_height,
};

/// How far two figures this feature calls equal may differ, in blocks. The
/// specification's declared comparison epsilon.
pub const EPSILON: f32 = 1e-4;

/// The column FR-6.1-S6 takes its shore reading on, as the specification names
/// it.
pub const SHORE_COLUMN: (u32, u32) = (63, 35);

/// The floor of the topmost water voxel, and so the height a player's feet reach
/// once it has risen through every voxel of the sea but the last.
pub const TOP_WATER_VOXEL: f32 = SEA_LEVEL as f32;

/// The sea's own top face: the height at which a player's box stops overlapping
/// water at all.
///
/// Water fills a submerged column up to and including [`SEA_LEVEL`], so its
/// topmost voxel occupies `[SEA_LEVEL, SEA_LEVEL + 1)` and a player whose feet
/// are a hair under this still overlaps it and still swims one more tick.
pub const SEA_TOP_FACE: f32 = (SEA_LEVEL + 1) as f32;

/// How deep the column every fixture here stands on has to be, in water voxels.
///
/// Guarded rather than assumed, because FR-6.1-S4's budget is stated for a
/// depth-two fixture — see [`sink_budget`] — and a shallower column would hold
/// that budget to a fall it was never derived for.
const REQUIRED_DEPTH: u32 = 2;

/// Where a settle begins: four blocks over the sea's own top face, in open air.
///
/// A height to fall from and never a resting place. Which cell the fall stops in
/// is the world's answer.
const SETTLE_FROM: f32 = SEA_TOP_FACE + 4.0;

/// How long an unresisted fall of eleven blocks is given, in ticks.
///
/// Fifty-two is what it actually spends, so this is the open-air share of
/// [`watch_for`] with room to spare rather than a figure anything is asserted
/// against.
const THROUGH_OPEN_AIR: f32 = 120.0;

/// What every registry built here is attributed to.
const NO_MEDIUM_ORIGIN: &str = "the shipped declarations with every medium taken out";

/// How long FR-6.1-S4 gives a player floating at the surface to return to the
/// lakebed of a column [`REQUIRED_DEPTH`] water voxels deep.
///
/// The specification's own form. Sinking approaches a terminal
/// `GRAVITY · TICK_DURATION / resistance`, so a fall of `depth` blocks takes
/// about `120 · depth · resistance` ticks, and the scenario states one and a
/// half times it: `1.5 × 120 × 2` is the `360` below.
#[must_use]
pub fn sink_budget(resistance: f32) -> f32 {
    360.0 * resistance
}

/// How long a fall into the sea is watched for, given what the sea resists.
///
/// Twice [`sink_budget`] plus the open air above it, so that a sink which
/// *breaches* the budget is reported by the assertion that cares about it rather
/// than running out of watch and being reported as a fall that never landed.
#[must_use]
pub fn watch_for(resistance: f32) -> u32 {
    (2.0 * sink_budget(resistance) + THROUGH_OPEN_AIR) as u32
}

/// A column of the shipped world that stands under the sea.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeaColumn {
    pub x: u32,
    pub z: u32,
    /// The topmost block of this column's lakebed.
    pub surface: u32,
}

impl SeaColumn {
    /// Where a player standing on this column's lakebed rests its feet: the top
    /// face of the topmost block under the water.
    #[must_use]
    pub const fn lakebed(self) -> f32 {
        (self.surface + 1) as f32
    }

    /// How many water voxels stand over that lakebed.
    ///
    /// Only ever called on a column [`sea_columns`] admitted, which is what makes
    /// the subtraction total.
    #[must_use]
    pub const fn depth(self) -> u32 {
        SEA_LEVEL - self.surface
    }

    /// The centre of this column, at `height`.
    #[must_use]
    pub fn at(self, height: f32) -> Vec3 {
        Vec3::new(self.x as f32 + 0.5, height, self.z as f32 + 0.5)
    }
}

/// Where a fall came to rest, and on which tick of the watch it did.
#[derive(Debug, Clone, Copy)]
pub struct Rest {
    pub tick: u32,
    pub state: PlayerState,
}

/// The shipped sea: the world that generates it, the view a tick reads it
/// through, the column every fixture here stands on, and what its water
/// declares.
#[derive(Debug)]
pub struct Sea {
    registry: BlockRegistry,
    world: ReplayWorld,
    /// The view a tick reads, resolved through the shipped registry.
    pub voxels: ResolvedVoxels,
    /// The deepest column of the sea, which is the worst case for both the rise
    /// and the sink.
    pub deepest: SeaColumn,
    /// Read back out of the shipped registry and never stated in this suite.
    pub resistance: f32,
}

impl Sea {
    /// A player settled onto the lakebed of the deepest column.
    ///
    /// # Errors
    ///
    /// Returns an error if it has not come to rest inside the watch, or if it
    /// came to rest anywhere but that column's lakebed.
    pub fn settled_on_the_lakebed(&self) -> Result<PlayerState, Box<dyn Error>> {
        let dropped = adrift(self.deepest.at(SETTLE_FROM));
        let rest = rested(dropped, &self.voxels, watch_for(self.resistance))?;
        require_resting_at(
            rest.state,
            self.deepest.lakebed(),
            "the lakebed of the deepest column",
        )?;
        Ok(rest.state)
    }

    /// A player settled onto the dry shore FR-6.1-S6 compares against.
    ///
    /// # Errors
    ///
    /// Returns an error if the world reports no height for that column, if the
    /// column is not dry, or if the settle does not end on its surface.
    pub fn shore_player(&self) -> Result<PlayerState, Box<dyn Error>> {
        let (x, z) = SHORE_COLUMN;
        let surface = self.dry_shore_surface()?;
        let dropped = adrift(Vec3::new(x as f32 + 0.5, SETTLE_FROM, z as f32 + 0.5));
        let rest = rested(dropped, &self.voxels, watch_for(self.resistance))?;
        require_resting_at(rest.state, (surface + 1) as f32, "the shore")?;
        Ok(rest.state)
    }

    /// The same world, resolved through a registry in which no block is a medium
    /// at all.
    ///
    /// **Built from the shipped declarations themselves**, with both medium
    /// answers set to what a declaration stating neither of them means and every
    /// other field left exactly as content declared it. So the only difference
    /// between this view and [`Sea::voxels`] is the medium — where a second
    /// hand-written registry would also have to restate water's solidity, its
    /// drawing and its targetability, and could differ from the shipped one in
    /// any of them without a scenario being able to see it.
    ///
    /// # Errors
    ///
    /// Returns an error if the rebuilt registry refuses a definition, or if the
    /// world holds a name it does not know.
    pub fn resisting_nothing(&self) -> Result<ResolvedVoxels, Box<dyn Error>> {
        let declared: Vec<Result<BlockDefinition, DefinitionSourceError>> = self
            .registry
            .definitions()
            .map(|held| {
                Ok(BlockDefinition {
                    swimmable: false,
                    move_resistance: 0.0,
                    ..held.clone()
                })
            })
            .collect();
        let mut without = BlockRegistry::new();
        without.apply(&InMemoryDefinitionSource::new(
            DefinitionOrigin::new(NO_MEDIUM_ORIGIN),
            declared,
        ))?;
        Ok(ResolvedVoxels::resolve(&self.world, &without)?)
    }

    /// The surface height of [`SHORE_COLUMN`], refused unless it is dry.
    fn dry_shore_surface(&self) -> Result<u32, Box<dyn Error>> {
        let (x, z) = SHORE_COLUMN;
        let surface = surface_height(&self.world, x, z)?;
        let over = block_at(&self.world, x, surface + 1, z)?;
        if surface >= SEA_LEVEL && over != WATER {
            return Ok(surface);
        }
        Err(format!(
            "the shore reading is taken on column ({x}, {z}) because it stands clear of the sea, \
             and this world puts its surface at {surface} with `{over}` over it. A walk compared \
             against a second swim is not a walk compared against a walk"
        )
        .into())
    }
}

/// What the shipped content declares water's `move_resistance` to be.
///
/// **The one place this suite asks that question.** Anything scaled by it — a
/// threshold, a tick budget, a watch — reads it from here rather than restating
/// it, so a value derived at implementation stays derived and a second copy
/// cannot drift out of step with the declaration.
///
/// # Errors
///
/// Returns an error if the name does not parse, or if the registry does not hold
/// the shipped water.
pub fn declared_resistance(registry: &BlockRegistry) -> Result<f32, Box<dyn Error>> {
    Ok(registry.resolve(&block_name(WATER)?)?.move_resistance)
}

/// The shipped sea, read through the registry this repository ships as content
/// and the world the declared seed generates.
///
/// # Errors
///
/// Returns an error if the content root cannot be read, if generation refuses,
/// if the world holds a name the registry does not know, if the generated world
/// has no sea in it, or if its deepest column is not [`REQUIRED_DEPTH`] voxels
/// deep.
pub fn the_shipped_sea() -> Result<Sea, Box<dyn Error>> {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    let voxels = ResolvedVoxels::resolve(&world, &registry)?;
    let deepest = deepest_sea_column(&world)?;
    require_depth(deepest)?;
    let resistance = declared_resistance(&registry)?;
    Ok(Sea {
        registry,
        world,
        voxels,
        deepest,
        resistance,
    })
}

/// Every column of the world that stands under the sea.
///
/// **The filter, stated apart from the ranking below.** A column belongs to the
/// sea when its surface stands under [`SEA_LEVEL`] *and* the cell directly over
/// that surface holds [`WATER`] — the second half because a column can stand low
/// without the generator having filled it, and no ordering of a ranking can
/// surface a constraint its filter never applied.
///
/// # Errors
///
/// Returns an error if the world reports no height for some column, or reaches
/// no cell over one.
fn sea_columns(world: &ReplayWorld) -> Result<Vec<SeaColumn>, Box<dyn Error>> {
    let mut under_water = Vec::new();
    for (x, z) in every_column() {
        let surface = surface_height(world, x, z)?;
        if surface < SEA_LEVEL && block_at(world, x, surface + 1, z)? == WATER {
            under_water.push(SeaColumn { x, z, surface });
        }
    }
    Ok(under_water)
}

/// The deepest column of the sea.
///
/// **The ranking, stated apart from the filter above**: least surface height
/// first, ties broken by [`every_column`]'s own order. Deepest is simultaneously
/// the worst case for the rise — furthest to climb — and for the sink — furthest
/// to fall — which is why the specification names it rather than any column.
///
/// # Errors
///
/// Returns an error if the world has no sea column at all.
pub fn deepest_sea_column(world: &ReplayWorld) -> Result<SeaColumn, Box<dyn Error>> {
    sea_columns(world)?
        .into_iter()
        .min_by_key(|column| column.surface)
        .ok_or_else(|| {
            "the shipped world generates no column standing under the sea, so there is no lakebed \
             for a scenario about swimming to stand on"
                .into()
        })
}

/// Refuses unless `column` is as deep as the budget FR-6.1-S4 states was derived
/// for.
fn require_depth(column: SeaColumn) -> Result<(), Box<dyn Error>> {
    if column.depth() == REQUIRED_DEPTH {
        return Ok(());
    }
    Err(format!(
        "FR-6.1-S4's budget is one and a half times `120 × depth × resistance` with a depth of \
         {REQUIRED_DEPTH}, and the deepest column of this world ({}, {}) stands {} water voxels \
         deep. Holding a fall of one depth to a budget derived for another is not a slack \
         assertion, it is a different one",
        column.x,
        column.z,
        column.depth()
    )
    .into())
}

/// Where a fall under an intent that asks for nothing comes to rest, and on
/// which tick of `watch` it does.
///
/// # Errors
///
/// Returns an error if it has not come to rest inside `watch`.
pub fn rested(
    from: PlayerState,
    world: &dyn Traversal,
    watch: u32,
) -> Result<Rest, Box<dyn Error>> {
    let mut state = from;
    for tick in 1..=watch {
        state = advance_player(state, &MovementIntent::default(), world);
        if state.on_ground {
            return Ok(Rest { tick, state });
        }
    }
    Err(format!(
        "a fall from {} was watched for {watch} ticks and never reached the ground; it ended at \
         {} still moving at {} blocks per second",
        from.position.y, state.position.y, state.velocity.y
    )
    .into())
}

/// Refuses unless a state has come to rest with its feet at `height`.
///
/// # Errors
///
/// Returns an error if it is not on the ground, or is on the ground somewhere
/// else.
pub fn require_resting_at(
    state: PlayerState,
    height: f32,
    what: &str,
) -> Result<(), Box<dyn Error>> {
    if state.on_ground && (state.position.y - height).abs() <= EPSILON {
        return Ok(());
    }
    Err(format!(
        "this fixture is about a player standing on {what}, which this world puts at {height}, \
         and the settle left it at {} {}. What it would assert about is a player somewhere else",
        state.position.y,
        if state.on_ground {
            "on the ground"
        } else {
            "still falling"
        }
    )
    .into())
}

/// A player at rest with nothing holding it up, at `at`.
#[must_use]
pub fn adrift(at: Vec3) -> PlayerState {
    PlayerState {
        position: at,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// An intent that asks to jump and for nothing else.
#[must_use]
pub fn holding_jump() -> MovementIntent {
    MovementIntent {
        jump: true,
        ..MovementIntent::default()
    }
}

/// An intent asking for a walk at full deflection and nothing else, which at yaw
/// 0 is a walk along +x.
#[must_use]
pub fn walking_forward() -> MovementIntent {
    MovementIntent {
        forward: 1.0,
        ..MovementIntent::default()
    }
}
