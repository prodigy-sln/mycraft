//! What the ray-marched judge does when a block is an obstacle nobody can see.
//!
//! # Why this is the sharpest single reading in the golden set's support
//!
//! The judge behind the golden comparison predicts what the player's camera is
//! looking at, and everything downstream of it trusts that prediction. For as
//! long as it decided what a ray stops at by asking whether a block was *solid*,
//! it happened to agree with the renderer over every block the base game ships —
//! because those blocks were drawn exactly when they collided. That agreement was
//! never evidence, and this is the reading that says so: **no judge deciding by
//! solidity can pass this test**, because the block it marches through is solid.
//!
//! # The fixture is a content root and never a hand-placed world
//!
//! There is one way to make a replay world — generate it against a registry — so
//! the block with the awkward declaration has to arrive through the declaration
//! itself. A copy of the shipped root restates `stone.luau` as an obstacle that
//! is not drawn, and the world generated over it is the same world, block for
//! block, as the one generated over the shipped root: only the registry differs,
//! which is exactly the difference this reading is about.
//!
//! # The control is the other half and it is not optional
//!
//! "No sample sees stone" is satisfied perfectly by a pose that never saw any
//! stone, by a march that collapsed, and by a world that lost its landmark. So
//! the same grid is classified twice, over two registries, and the shipped one
//! has to see stone before the restated one is asked not to. The two readings
//! are one comparison for that reason.

#[path = "support/reload.rs"]
mod reload;
mod support;

use std::error::Error;

use mc_client::startup::PreparedScene;
use mc_sim::camera::CameraPose;

use reload::{Declaration, STONE, STONE_FILE, restating, shipped};
use support::frames::CAPTURE_SIZE;
use support::oracle::{Sighted, Voxels, sighted_samples};
use support::{TestResult, prepare_scene, prepare_scene_at};

/// The pose both classifications are taken from.
///
/// **Declared here and deliberately not a pose the player reaches**, for the
/// reason the derived probes declare theirs: what this reading needs is a frame
/// the landmark pillar covers several samples of, and a pose reached by advancing
/// the script would tie the reading to a spawn that moves for reasons of its own.
///
/// **Close, and the distance is the whole of the choice.** The pillar is one
/// column wide, so how much of the declared grid it covers is decided by how far
/// away the eye stands: the lens's horizontal half-angle puts a one-block width
/// at roughly `1150 / d` pixels on a 1280-pixel frame, against samples 40 pixels
/// apart, so an eye 5.7 blocks out covers about five sample columns where one 45
/// blocks out covers none at all — measured, after a first pose at that distance
/// saw the pillar through no sample whatever. The eye stands on the diagonal at
/// y = 50, above the 48 the surface band tops out at, so it is in open air and
/// every column between it and the pillar is below its line of sight.
const EYE: [f32; 3] = [16.0, 50.0, 16.0];
const LOOK_AT: [f32; 3] = [12.0, 50.0, 12.0];

#[test]
fn the_march_passes_through_an_obstacle_nothing_draws_and_reports_what_stands_beyond_it()
-> TestResult {
    let undrawn = restating(
        shipped()?,
        STONE_FILE,
        &stone_that_is_an_invisible_obstacle(),
    )?;

    let as_shipped = stone_samples(&prepare_scene()?)?;
    let when_undrawn = stone_samples(&prepare_scene_at(undrawn.path())?)?;

    assert_eq!(
        (as_shipped > 0, when_undrawn),
        (true, 0),
        "the landmark pillar is stone, and from this pose {as_shipped} of the declared samples \
         look at it. Declaring that same stone `drawn = false` while leaving it `solid = true` \
         takes the pillar out of the picture without taking it out of the world, so every one of \
         those rays now has to carry on to whatever stands beyond it — and {when_undrawn} of them \
         still stopped at it. A judge that decides what a ray stops at by asking whether a block \
         is an obstacle cannot tell the two roots apart at all, which is what this measures"
    );
    Ok(())
}

/// `base:stone`, restated as an obstacle nothing draws.
///
/// The two are stated together and neither alone: `solid = true` is what makes
/// the block still be there, and `drawn = false` is what makes it invisible.
/// A declaration stating only the second would default its solidity from nothing
/// and the world would have changed underneath the reading.
fn stone_that_is_an_invisible_obstacle() -> Declaration {
    Declaration::of(STONE).solid(true).drawn(false)
}

/// How many of the declared samples this scene's stone is what the march meets.
fn stone_samples(prepared: &PreparedScene) -> Result<usize, Box<dyn Error>> {
    let voxels = Voxels {
        world: &prepared.world,
        registry: prepared.registry.as_ref(),
    };
    let stone = Sighted::Terrain(mc_core::id::BlockName::parse(STONE)?);
    Ok(sighted_samples(&declared_pose(), CAPTURE_SIZE, &voxels)?
        .into_iter()
        .filter(|(_, sighted)| *sighted == stone)
        .count())
}

/// The declared pose, as the marching oracle takes one.
fn declared_pose() -> CameraPose {
    CameraPose {
        eye: EYE,
        target: LOOK_AT,
    }
}
