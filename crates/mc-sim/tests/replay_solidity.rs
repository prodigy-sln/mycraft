//! Where solidity comes from, and where the world stops.
//!
//! Every phase before this one asserted the physics against solidity a fixture
//! *declared*: a floor at `y = 40` was solid because the fixture said so. This
//! phase asserts the other half — that what the fixture says comes from a block
//! **definition** and from nothing else, and that outside the volume those
//! definitions describe, nothing is solid at all.
//!
//! **The two halves of the name question point in opposite directions on
//! purpose.** One scenario fills a column with `base:stone` and declares that
//! definition non-solid; the next fills one with `mod:cloud` — a name no content
//! in this repository declares — and declares *that* definition solid. An
//! implementation comparing a name to a shipped one fails the first, and one
//! comparing against a list of names it knows fails the second, so neither
//! survives the pair. The water scenario is the third leg and is deliberately
//! *not* declared: water's non-solidity is a fact about `content/base/blocks`,
//! so it is asserted against the registry this repository actually ships.
//!
//! **What is solid is as load-bearing as what is not.** Six of the eight
//! scenarios below assert an absence — the player falls through, walks off,
//! finds no ground — and an implementation whose bitset was empty would satisfy
//! all six while asserting nothing. They are not paired one-to-one with
//! controls; the file is: the lakebed and the cloud column are positions only a
//! bitset with the right bits set can produce, and every scenario about an edge
//! carries a *guard* that the mirror position inside the world is solid, so a
//! collapsed answer fails the guard rather than passing the scenario.
//!
//! Comparisons use the declared 1 × 10⁻⁴ epsilon.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::player::{BlockPos, MovementIntent, PlayerState, Solidity, advance_player};
use mc_sim::replay::{Extent, ReplayWorld, SolidVoxels};
use mc_world::column::COLUMN_HEIGHT;

use support::volume::{NamedSlab, registry_declaring};
use support::{
    FOOTPRINT, SEA_LEVEL, STONE, TestResult, WATER, block_at, content_registry, every_column,
    replay_world, surface_height,
};

/// How far two figures this feature calls equal may differ, in blocks or in
/// blocks per second. The specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// How fast a walk carries the player, in blocks per second. Declared.
const WALK_SPEED: f32 = 4.5;

/// How far one tick of held walk covers, in blocks.
const WALK_PER_TICK: f32 = WALK_SPEED * TICK_DURATION;

/// How long a fall is given to land and settle. A four-block fall takes 31
/// ticks, and a player that has landed stays landed, so a longer watch cannot
/// change the answer.
const SETTLE_TICKS: u32 = 60;

/// How far above the sea's top face a fall into the declared world starts.
///
/// Water fills a submerged column up to and including [`SEA_LEVEL`], so its top
/// face is at `SEA_LEVEL + 1` and a fall from three blocks over that starts in
/// open air and ends on the lakebed.
const START_ABOVE_SEA: u32 = 3;

/// The declared slab fixture: sixteen blocks across, thirty-two tall, filled to
/// its tenth voxel — so its top face is at `y = 11`.
const SLAB_EXTENT: Extent = Extent {
    x: 16,
    y: 32,
    z: 16,
};
const SLAB_TOP: u32 = 10;
const SLAB_TOP_FACE: f32 = (SLAB_TOP + 1) as f32;

/// Where a fall onto the slab starts, and how long it is watched for.
///
/// Five blocks above the slab's face, which is 35 ticks of falling; forty ticks
/// carries a player the slab does not hold a further 1.8 blocks *past* that
/// face, so passing through and stopping on it are separated by more than the
/// comparison epsilon rather than by a hair.
const SLAB_START_HEIGHT: f32 = 16.0;
const SLAB_FALL_TICKS: u32 = 40;

/// Where the feet centre stands over the slab.
///
/// Different on the two horizontal axes, as every fixture in this suite is, so
/// that a query reading one axis where it meant the other is not handed a
/// coordinate that agrees by accident.
const SLAB_X: f32 = 8.5;
const SLAB_Z: f32 = 4.5;

/// The block whose definition says it is not solid, and the one whose definition
/// says it is.
///
/// The first is a name this repository ships and content declares **solid**; the
/// second is a name content does not declare at all. Each is given the opposite
/// definition here, which is what makes the pair close name-matching: an engine
/// that knew `base:stone` was solid fails the first, and one that treated an
/// unfamiliar name as air fails the second.
const HOLLOW_BLOCK: &str = STONE;
const UNFAMILIAR_BLOCK: &str = "mod:cloud";

/// What fills the space above either slab: the same in both, so the only
/// difference between the two scenarios is the definition under test.
///
/// A block this test's own registry declares and nothing else knows.
/// [`NamedSlab`] says what is *there* at every cell it reaches, so the space
/// over the slab holds a declared non-solid block rather than nothing at all —
/// and the name is the fixture's own, because the base game ships no block whose
/// job is to mean empty space.
const OPEN_BLOCK: &str = "fixture:open";

/// The column a walk out of the world starts in, and the row it walks along.
///
/// Different numbers, so that a query reading a box's z where it meant its x
/// lands on a column of the world that answers differently.
///
/// The **last** column rather than the first, and the walk goes east: this is
/// the only place in the phase that names a coordinate past the extent's *upper*
/// bound. The other three scenarios about the world's edges are on the low side
/// by their own wording — below `y = 0`, a feet centre at `x = −5.0`, a negative
/// coordinate on each axis — so a westward walk here would have been a fourth
/// statement about the low side, leaving `x = FOOTPRINT` queried by nothing at
/// all. FR-3.5-S1's wording names no edge, so either direction satisfies it and
/// only one of them exercises both halves of the bound.
const EDGE_COLUMN: u32 = FOOTPRINT - 1;
const EDGE_ROW: u32 = 32;

/// Where the feet centre starts that walk: the middle of the last column, so the
/// box reaches from `x = 63.2` to `x = 63.8` and is entirely inside the world.
const EDGE_START_X: f32 = (FOOTPRINT - 1) as f32 + 0.5;

/// How many ticks that walk needs before nothing is beneath the box.
///
/// The box's *leading* face has half a block plus the declared half-width of
/// 0.3 = 0.8 blocks to cover before it passes `x = FOOTPRINT`, which is 10.67
/// ticks of walking at the declared [`WALK_PER_TICK`] — so the eleventh tick is
/// the first with the whole box east of the world. A box still overhanging solid
/// ground is still standing on it, which is why this is not asked of the feet
/// centre.
const LEAVING_TICKS: u32 = 11;

/// How far into that walk the floor is checked to still be there.
///
/// Five ticks leaves the box overhanging the world's edge and still supported by
/// it. Without this the scenario would pass against an implementation that never
/// reported ground contact at all.
const STILL_SUPPORTED_TICKS: u32 = 5;

/// Where a fall below the world starts.
///
/// The whole box is under `y = 0`: a box straddling the world's floor is still
/// inside the world, and the scenario is about the player that is not.
const BELOW_WORLD_FEET: f32 = -2.0;

/// How long that fall is watched for. Sixty ticks reaches 30 blocks per second,
/// which is short of the declared terminal speed — so what this measures is
/// gravity still being applied, not a clamp.
const VOID_TICKS: u32 = 60;

/// Where a player west of the world stands, and how long it is watched for.
///
/// The height is deep inside the world's stone: the column at `x = 0` is solid
/// at `y = 10` whatever its surface height is, so an implementation that clamped
/// a negative coordinate to column 0 would find ground and stand the player on
/// it. That is the defect this scenario exists for, and the guard below asserts
/// the fixture really does present it.
const WEST_OF_WORLD_X: f32 = -5.0;
const WEST_OF_WORLD_Y: f32 = 10.0;
const WEST_TICKS: u32 = 10;

/// A voxel the declared world is solid at, from which each negative coordinate
/// below is one step away.
///
/// The world's floor, so the assertion that a step to `y = -1` is not solid is a
/// step off something rather than a step off nothing.
const SOLID_CORNER: BlockPos = BlockPos { x: 0, y: 0, z: 0 };

/// The first coordinate past the loaded extent on each axis.
///
/// The declaration's own figures rather than the subject's: the replay spans
/// [`FOOTPRINT`] columns on x and on z and a whole [`COLUMN_HEIGHT`] on y, so
/// the extent holds x and z in `[0, 64)` and y in `[0, 256)` and these are the
/// three coordinates immediately outside it. Three numbers rather than one
/// because the vertical axis is not the footprint — a probe that used 64 on all
/// three would still be inside the world on y and would assert nothing there.
const PAST_X: i32 = FOOTPRINT as i32;
const PAST_Y: i32 = COLUMN_HEIGHT as i32;
const PAST_Z: i32 = FOOTPRINT as i32;

/// How far a body let go from rest has fallen after `ticks` ticks.
///
/// The closed form of the declared integrator's sum rather than a second copy of
/// the subject's loop: each tick takes `GRAVITY × TICK_DURATION` from the
/// velocity *before* the position moves, so after `n` ticks the drop is
/// `g · dt² · n(n + 1) / 2`.
const fn fallen_after(ticks: f32) -> f32 {
    GRAVITY * TICK_DURATION * TICK_DURATION * ticks * (ticks + 1.0) / 2.0
}

/// A player let go from rest at `position`, out of contact with anything.
fn dropped_at(position: Vec3) -> PlayerState {
    PlayerState {
        position,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// A player standing still, in contact with the ground, at `position`.
fn standing_at(position: Vec3) -> PlayerState {
    PlayerState {
        position,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    }
}

/// An intent that asks to walk forward and nothing else — which at yaw 0 is a
/// walk along +x.
fn walking_forward() -> MovementIntent {
    MovementIntent {
        forward: 1.0,
        ..MovementIntent::default()
    }
}

/// Where `ticks` submissions of `intent` leave `state`.
fn advance(
    state: PlayerState,
    intent: &MovementIntent,
    world: &dyn Solidity,
    ticks: u32,
) -> PlayerState {
    (0..ticks).fold(state, |state, _| advance_player(state, intent, world))
}

/// The declared world, and its voxels resolved through the registry this
/// repository ships as content.
fn declared_world() -> Result<(ReplayWorld, SolidVoxels), Box<dyn Error>> {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    let voxels = SolidVoxels::resolve(&world, &registry)?;
    Ok((world, voxels))
}

/// A slab of `filling` under open space, resolved through a registry declaring
/// that block's solidity and nothing else about it.
fn slab_of(filling: &str, is_solid: bool) -> Result<SolidVoxels, Box<dyn Error>> {
    let slab = NamedSlab::new(SLAB_EXTENT, SLAB_TOP, filling, OPEN_BLOCK)?;
    let registry = registry_declaring(&[(filling, is_solid), (OPEN_BLOCK, false)])?;
    Ok(SolidVoxels::resolve(&slab, &registry)?)
}

/// Where a fall onto the slab starts.
fn over_the_slab() -> Vec3 {
    Vec3::new(SLAB_X, SLAB_START_HEIGHT, SLAB_Z)
}

/// A column of the declared world that stands under water.
#[derive(Debug, Clone, Copy)]
struct Submerged {
    x: u32,
    z: u32,
    surface: u32,
}

/// The first such column, if the declared world has one.
///
/// Three conditions, and every one of them is a fixture-shape constraint no
/// assertion in the test can enforce. The column must be **submerged**, or there
/// is no water for the player to fall through and the scenario is about an
/// ordinary landing. Its two coordinates must **differ**, and its transpose must
/// stand at a **different height**, because a resting height read from column
/// `(z, x)` instead of `(x, z)` is the one indexing defect a fall onto a
/// horizontally uniform answer cannot see.
fn a_submerged_column(world: &ReplayWorld) -> Result<Option<Submerged>, Box<dyn Error>> {
    for (x, z) in every_column() {
        let surface = surface_height(world, x, z)?;
        if x == z || surface >= SEA_LEVEL || surface_height(world, z, x)? == surface {
            continue;
        }
        if block_at(world, x, surface + 1, z)? == WATER {
            return Ok(Some(Submerged { x, z, surface }));
        }
    }
    Ok(None)
}

/// Where a fall into a submerged column starts: open air over the sea's face.
fn over_the_sea(column: Submerged) -> Vec3 {
    Vec3::new(
        column.x as f32 + 0.5,
        (SEA_LEVEL + START_ABOVE_SEA) as f32,
        column.z as f32 + 0.5,
    )
}

/// Where the walk out of the world starts, on the floor of the last column.
fn at_the_edge(surface: u32) -> Vec3 {
    Vec3::new(EDGE_START_X, (surface + 1) as f32, EDGE_ROW as f32 + 0.5)
}

/// The three positions one step negative of [`SOLID_CORNER`], one per axis.
fn one_step_negative() -> [BlockPos; 3] {
    [
        BlockPos {
            x: -1,
            ..SOLID_CORNER
        },
        BlockPos {
            y: -1,
            ..SOLID_CORNER
        },
        BlockPos {
            z: -1,
            ..SOLID_CORNER
        },
    ]
}

/// The three positions just past the extent's far side, one per axis.
///
/// Each keeps [`SOLID_CORNER`]'s other two coordinates, so the guard that the
/// world is solid *there* is a statement about the same query these three go
/// through. Unlike [`one_step_negative`] they are not adjacent to that corner —
/// the far side is the width of the world away from it — and they do not need
/// to be: what makes an absence non-vacuous here is that the bitset holds bits
/// at all, not that this particular neighbour holds one.
fn beyond_the_extent() -> [BlockPos; 3] {
    [
        BlockPos {
            x: PAST_X,
            ..SOLID_CORNER
        },
        BlockPos {
            y: PAST_Y,
            ..SOLID_CORNER
        },
        BlockPos {
            z: PAST_Z,
            ..SOLID_CORNER
        },
    ]
}

#[test]
fn a_fall_through_water_comes_to_rest_on_the_surface_beneath_it() -> TestResult {
    let (world, voxels) = declared_world()?;
    let column = a_submerged_column(&world)?.ok_or(
        "the declared world holds no submerged column whose transposed column stands at a \
         different height, so this scenario would be measuring neither water nor which \
         column was consulted",
    )?;
    let lakebed = (column.surface + 1) as f32;

    let landed = advance(
        dropped_at(over_the_sea(column)),
        &MovementIntent::default(),
        &voxels,
        SETTLE_TICKS,
    );

    assert!(
        (landed.position.y - lakebed).abs() <= EPSILON && landed.on_ground,
        "content declares water non-solid, so a fall into {column:?} passes through it and \
         lands on the lakebed at {lakebed} — not at {}, which is where a fall stopped on the \
         water's own top face, or one that read the transposed column, ends up",
        landed.position.y
    );
    Ok(())
}

#[test]
fn a_fall_onto_blocks_their_definition_calls_non_solid_passes_straight_through() -> TestResult {
    let hollow = slab_of(HOLLOW_BLOCK, false)?;
    let unimpeded = SLAB_START_HEIGHT - fallen_after(SLAB_FALL_TICKS as f32);

    let fallen = advance(
        dropped_at(over_the_slab()),
        &MovementIntent::default(),
        &hollow,
        SLAB_FALL_TICKS,
    );

    assert!(
        (fallen.position.y - unimpeded).abs() <= EPSILON && !fallen.on_ground,
        "`{HOLLOW_BLOCK}` is declared non-solid here whatever content says about it, so the \
         fall is never interrupted and reaches {unimpeded} with nothing under it — not \
         {} with ground contact {}, which is a solidity read off the name rather than off \
         the definition",
        fallen.position.y,
        fallen.on_ground
    );
    Ok(())
}

#[test]
fn a_fall_onto_blocks_their_definition_calls_solid_stops_on_their_top_face() -> TestResult {
    let cloud = slab_of(UNFAMILIAR_BLOCK, true)?;

    let landed = advance(
        dropped_at(over_the_slab()),
        &MovementIntent::default(),
        &cloud,
        SLAB_FALL_TICKS,
    );

    assert!(
        (landed.position.y - SLAB_TOP_FACE).abs() <= EPSILON && landed.on_ground,
        "`{UNFAMILIAR_BLOCK}` is a name no content declares, and its definition says solid, so \
         a voxel filling [{SLAB_TOP}, {SLAB_TOP_FACE}) holds the player up at \
         {SLAB_TOP_FACE} — not at {}, which is where a fall through a block the engine did \
         not recognise ends up",
        landed.position.y
    );
    Ok(())
}

#[test]
fn a_walk_past_the_edge_of_the_loaded_world_leaves_it_and_loses_the_ground() -> TestResult {
    let (world, voxels) = declared_world()?;
    let surface = surface_height(&world, EDGE_COLUMN, EDGE_ROW)?;
    let start = standing_at(at_the_edge(surface));
    let beyond = EDGE_START_X + WALK_PER_TICK * LEAVING_TICKS as f32;

    let held = advance(start, &walking_forward(), &voxels, STILL_SUPPORTED_TICKS);
    let gone = advance(
        held,
        &walking_forward(),
        &voxels,
        LEAVING_TICKS - STILL_SUPPORTED_TICKS,
    );

    assert!(
        held.on_ground,
        "the walk has to start on ground the world is holding up, or losing it below is a \
         claim about nothing: after {STILL_SUPPORTED_TICKS} ticks the box still overhangs \
         column {EDGE_COLUMN} at height {surface} and must still report contact"
    );
    assert!(
        (gone.position.x - beyond).abs() <= EPSILON && !gone.on_ground,
        "outside the loaded world is not solid, so nothing stops the walk and nothing holds \
         it up: {LEAVING_TICKS} ticks carry the feet centre to {beyond} with no contact — not \
         to {} with contact {}, which is a world whose edge acts as a wall or as a floor",
        gone.position.x,
        gone.on_ground
    );
    Ok(())
}

#[test]
fn a_fall_below_the_bottom_of_the_world_keeps_accelerating_and_never_lands() -> TestResult {
    let (_world, voxels) = declared_world()?;
    let start = Vec3::new(
        EDGE_ROW as f32 + 0.5,
        BELOW_WORLD_FEET,
        EDGE_ROW as f32 + 0.5,
    );
    let expected_speed = -GRAVITY * TICK_DURATION * VOID_TICKS as f32;
    let mut state = dropped_at(start);
    let mut contacts = 0_u32;

    for _ in 0..VOID_TICKS {
        state = advance_player(state, &MovementIntent::default(), &voxels);
        contacts += u32::from(state.on_ground);
    }

    assert!(
        voxels.is_solid(SOLID_CORNER),
        "the world has to be solid at its own floor {SOLID_CORNER:?}, or a fall that finds \
         nothing below it is a fall through a world that holds nothing anywhere"
    );
    assert!(
        contacts == 0 && (state.velocity.y - expected_speed).abs() <= EPSILON,
        "nothing under the world holds the player up and gravity keeps working, so \
         {VOID_TICKS} ticks below y = 0 report no contact at any of them and a speed of \
         {expected_speed} — not {contacts} contacts and {}",
        state.velocity.y
    );
    Ok(())
}

#[test]
fn feet_west_of_the_world_find_no_ground_however_solid_the_first_column_is() -> TestResult {
    let (_world, voxels) = declared_world()?;
    let inside = BlockPos {
        x: 0,
        y: WEST_OF_WORLD_Y as i32,
        z: EDGE_ROW as i32,
    };
    let start = Vec3::new(WEST_OF_WORLD_X, WEST_OF_WORLD_Y, EDGE_ROW as f32 + 0.5);
    let mut state = dropped_at(start);
    let mut contacts = 0_u32;

    for _ in 0..WEST_TICKS {
        state = advance_player(state, &MovementIntent::default(), &voxels);
        contacts += u32::from(state.on_ground);
    }

    assert!(
        voxels.is_solid(inside),
        "the column at x = 0 has to be solid at {inside:?}, or standing the player on it \
         would be indistinguishable from letting it fall and this scenario would pass \
         against the defect it exists for"
    );
    assert!(
        contacts == 0,
        "a feet centre at x = {WEST_OF_WORLD_X} is west of the world and finds nothing under \
         it, whatever column 0 holds at that height — not the {contacts} of {WEST_TICKS} \
         ticks a coordinate saturated or masked into column 0 reports"
    );
    Ok(())
}

#[test]
fn a_position_with_a_negative_coordinate_on_any_axis_is_never_solid() -> TestResult {
    let (_world, voxels) = declared_world()?;

    let solid: Vec<BlockPos> = one_step_negative()
        .into_iter()
        .filter(|at| voxels.is_solid(*at))
        .collect();

    assert!(
        voxels.is_solid(SOLID_CORNER),
        "the world has to be solid at {SOLID_CORNER:?}, or the three positions one step \
         negative of it are not solid for the reason this scenario is about"
    );
    assert!(
        solid.is_empty(),
        "a negative coordinate names a position outside the world on that axis and is not \
         solid, on any of the three: {solid:?} came back solid, which is what a coordinate \
         clamped to zero or wrapped into the far edge of the footprint answers"
    );
    Ok(())
}

#[test]
fn a_position_beyond_the_loaded_extent_on_any_axis_is_never_solid() -> TestResult {
    let (_world, voxels) = declared_world()?;

    let solid: Vec<BlockPos> = beyond_the_extent()
        .into_iter()
        .filter(|at| voxels.is_solid(*at))
        .collect();

    assert!(
        voxels.is_solid(SOLID_CORNER),
        "the world has to be solid at {SOLID_CORNER:?}, or a bitset holding nothing anywhere \
         answers this scenario without the extent ever being consulted"
    );
    assert!(
        solid.is_empty(),
        "x = {PAST_X}, y = {PAST_Y} and z = {PAST_Z} are each the first coordinate the extent \
         does not hold, so each names a position outside the world on that axis and is not \
         solid: {solid:?} came back solid, which is what an index that computed an offset \
         without testing the upper bound and aliased into a neighbouring row of the bitset \
         answers"
    );
    Ok(())
}
