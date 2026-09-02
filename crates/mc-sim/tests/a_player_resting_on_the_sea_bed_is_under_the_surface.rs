//! Whether a player of the shipped world can actually get its eye under the
//! sea, and by how much.
//!
//! # The premise this file turns into an observation
//!
//! The eye stands `1.62` blocks over the feet and a swimmer that stops asking to
//! rise sinks to the bed, so a player resting on the bed of a column two water
//! voxels deep has its eye at `33.0 + 1.62 = 34.62` under a surface whose own
//! top face is `35.0` — **submerged by `0.38` blocks**. Until this file existed
//! that was arithmetic over two recorded numbers rather than something anybody
//! had watched happen, and everything built on top of it inherited the
//! difference.
//!
//! # Why there is no red here, and what stands in for it
//!
//! A measurement has no failing state to display: the world either puts the eye
//! under the surface or it does not, and the reading is written before anything
//! is built on it precisely so that the answer is not arranged. What replaces
//! red is a **second reading over a sea one voxel shallower**, built by the same
//! fixture with one number changed, which has to come back with the other
//! verdict and the freeboard rather than passing. A reading that answered
//! "submerged" over both would be a reading that answers "submerged" over
//! anything.
//!
//! # The margin is debt, and this file is the only thing that would say so
//!
//! `0.38` blocks is what separates a player who can see the water from a player
//! who cannot. Any later change to the eye's height over the feet, or to how
//! deep the generator digs its sea, spends that margin silently — nothing else
//! in this repository compares the two. So the numbers below are stated
//! absolutely and never derived from the constants under test: an eye at `1.5`
//! blocks reddens the resting height, the margin and nothing else, which is the
//! shape of failure a reader can act on.
//!
//! # What is refused rather than adjusted
//!
//! If the sea turned out too shallow, the answer is not to deepen it, widen it
//! or lower the eye. Each of those is a content or physics change made to
//! satisfy a rendering feature, and all three are refused in advance by the
//! specification this file belongs to.

mod support;

use std::error::Error;

use mc_sim::replay::{Extent, ResolvedVoxels};
use mc_world::world::WorldPos;

use support::sea::{SEA_TOP_FACE, declared_resistance, watch_for};
use support::submersion::{
    INSIDE_THE_MEDIUM, OVER_THE_SURFACE, StatedReading, Submergible, blocks, eye_at_rest,
};
use support::volume::Cells;
use support::{NOTHING, SEA_LEVEL, STONE, TestResult, WATER, content_registry, replay_world};

/// How many water voxels the shipped world's deepest column stands under.
///
/// Stated rather than read, and the third cell of that column is asserted to
/// hold nothing by `the_generated_sea_is_deep_enough_to_swim_in.rs` — so a sea
/// that grew deeper reddens there and a sea that grew shallower reddens here.
const SEA_DEPTH: u32 = 2;

/// The top face of that column's lakebed, where a player standing on it rests
/// its feet.
const LAKEBED_TOP_FACE: f32 = 33.0;

/// Where the eye of a player resting there stands.
///
/// **Written out rather than derived from [`mc_sim::player::EYE_HEIGHT`].** A
/// height computed from the constant under test agrees with whatever that
/// constant becomes, and this figure exists to disagree.
const RESTING_EYE: f32 = 34.62;

/// How far under the surface that leaves it, in blocks. The spec's load-bearing
/// premise, and the whole of the margin a player has.
const SUBMERGED_BY: f32 = 0.38;

/// The bed of the same sea with one voxel of water taken off the bottom.
const SHALLOW_LAKEBED_TOP_FACE: f32 = 34.0;

/// Where a resting eye stands over that bed.
const SHALLOW_RESTING_EYE: f32 = 35.62;

/// How far over the surface that leaves it, in blocks.
const DRY_BY: f32 = 0.62;

/// How far the declared pool reaches on each axis.
///
/// Wide enough that the player's box stands clear of every edge on the column
/// the reading is taken on, and tall enough that the fall begins inside it.
const POOL: Extent = Extent { x: 8, y: 48, z: 8 };

/// A pool of the shipped water `depth` voxels deep over a stone bed, with its
/// surface where the shipped sea puts its own.
///
/// **The shipped water and the shipped stone, resolved through the shipped
/// content root**, so a player falling into it swims and sinks by what
/// `content/base/blocks/water.luau` declares rather than by anything this file
/// says. The one thing the fixture states is the depth, which is what makes the
/// difference between the two readings below attributable to depth and to
/// nothing else.
fn a_pool_of_the_shipped_water(depth: u32) -> Result<Cells, Box<dyn Error>> {
    let bottom = SEA_LEVEL + 1 - depth;
    let bed = WorldPos {
        x: POOL.x,
        y: bottom,
        z: POOL.z,
    };
    let surface = WorldPos {
        x: POOL.x,
        y: SEA_LEVEL + 1,
        z: POOL.z,
    };
    Cells::empty(POOL)
        .holding(WorldPos { x: 0, y: 0, z: 0 }, bed, STONE)?
        .holding(
            WorldPos {
                x: 0,
                y: bottom,
                z: 0,
            },
            surface,
            WATER,
        )
}

/// What the reading has to answer, stated by hand.
fn stated(
    verdict: &'static str,
    holds: &str,
    depth: u32,
    heights: (f32, f32, f32),
) -> StatedReading {
    let (lakebed, eye, margin) = heights;
    StatedReading {
        verdict,
        eye_cell_holds: holds.to_owned(),
        depth,
        lakebed_top_face: blocks(lakebed),
        surface_top_face: blocks(SEA_TOP_FACE),
        eye: blocks(eye),
        margin: blocks(margin),
    }
}

#[test]
fn the_eye_of_a_player_resting_on_the_deepest_bed_of_the_shipped_sea_stands_inside_the_water()
-> TestResult {
    let registry = content_registry()?;
    let world = replay_world(&registry)?;
    let voxels = ResolvedVoxels::resolve(&world, &registry)?;
    let reading = eye_at_rest(&Submergible {
        volume: &world,
        voxels: &voxels,
        water: WATER,
        watch: watch_for(declared_resistance(&registry)?),
    })?;

    assert_eq!(
        reading.stated(),
        stated(
            INSIDE_THE_MEDIUM,
            WATER,
            SEA_DEPTH,
            (LAKEBED_TOP_FACE, RESTING_EYE, SUBMERGED_BY)
        ),
        "a player that has sunk to the bed of the shipped sea's deepest column has to have its \
         eye inside a cell of that sea: {SEA_DEPTH} voxels of water over a bed whose top face is \
         {LAKEBED_TOP_FACE}, an eye at {RESTING_EYE} and a surface at {SEA_TOP_FACE}, which is \
         {SUBMERGED_BY} blocks of margin and the whole of what a player has. Every figure is \
         stated rather than derived, so an eye at another height over the feet and a sea of \
         another depth are two different failures. The reading was taken on column ({}, {})",
        reading.column().x,
        reading.column().z
    );
    Ok(())
}

#[test]
fn a_sea_one_voxel_shallower_leaves_that_eye_over_the_surface_and_says_by_how_much() -> TestResult {
    let registry = content_registry()?;
    let pool = a_pool_of_the_shipped_water(SEA_DEPTH - 1)?;
    let voxels = ResolvedVoxels::resolve(&pool, &registry)?;
    let reading = eye_at_rest(&Submergible {
        volume: &pool,
        voxels: &voxels,
        water: WATER,
        watch: watch_for(declared_resistance(&registry)?),
    })?;

    assert_eq!(
        reading.stated(),
        stated(
            OVER_THE_SURFACE,
            NOTHING,
            SEA_DEPTH - 1,
            (SHALLOW_LAKEBED_TOP_FACE, SHALLOW_RESTING_EYE, DRY_BY)
        ),
        "the same reading over a sea one voxel shallower has to come back with the other verdict \
         and say how far the eye stands over the surface — {DRY_BY} blocks, its cell holding \
         `{NOTHING}` rather than `{WATER}`. This is what stands in for red: a reading that \
         answered `{INSIDE_THE_MEDIUM}` here would answer it over anything, and the submersion \
         it reports over the shipped sea would be evidence of nothing"
    );
    Ok(())
}

#[test]
fn a_declared_pool_as_deep_as_the_shipped_sea_puts_that_eye_back_under_the_surface() -> TestResult {
    let registry = content_registry()?;
    let pool = a_pool_of_the_shipped_water(SEA_DEPTH)?;
    let voxels = ResolvedVoxels::resolve(&pool, &registry)?;
    let reading = eye_at_rest(&Submergible {
        volume: &pool,
        voxels: &voxels,
        water: WATER,
        watch: watch_for(declared_resistance(&registry)?),
    })?;

    assert_eq!(
        reading.stated(),
        stated(
            INSIDE_THE_MEDIUM,
            WATER,
            SEA_DEPTH,
            (LAKEBED_TOP_FACE, RESTING_EYE, SUBMERGED_BY)
        ),
        "the pool the dry reading is taken over differs from this one in its depth and in \
         nothing else — same blocks, same registry, same surface height, one number changed. \
         Without this control, a dry verdict over the shallow pool could be about the fixture \
         being declared rather than generated, about the stone under it, or about anything else \
         the two seas do not share, and the depth would only look like the cause"
    );
    Ok(())
}
