//! Where the player stood and which way they faced, written into a save and read
//! back out of it.
//!
//! "Quit and resume" means resuming where you left off. A player set down on
//! re-derived ground is wherever the terrain happens to put them, which is the
//! one thing this feature promises not to do — so the place and the facing are
//! both stored, and a resumed player is placed from the save rather than from
//! the world.
//!
//! **Exact equality is the right assertion here and a tolerance would be
//! wrong**, and that is a measurement rather than a preference. The stored form
//! is the number's own four bytes, little-endian, and reading them back is
//! `f32::from_le_bytes` over what `f32::to_le_bytes` produced — a bit-for-bit
//! identity with no arithmetic anywhere in it. The facing below is written in
//! degrees and converted once, in a helper both the value handed to the writer
//! and the value expected of the reader are taken from, so the conversion cannot
//! contribute an error either: whatever bits it produces are the bits that go in
//! and the bits that must come out. A tolerance here would be a hole a writer
//! that quietly rounded, clamped or normalised an angle would fit through.
//!
//! **A finite position outside the world's footprint is legitimate and must
//! load.** `mc-sim`'s own reasoning is that the player is not confined to the
//! world, so a save recording a player four blocks past the edge is a save a
//! player produces by walking off it. The out-of-footprint test is the control
//! for the two refusals: only a coordinate that is not a finite number is
//! refused, and a reader that refused by *range* would pass both refusals and
//! fail only there.
//!
//! The two refusals are read against saves written out byte by byte, because a
//! non-finite coordinate is a thing no writer will produce — and because the
//! offset each one sits at is then stated by the fixture rather than inherited
//! from the reader being tested.

mod common;

use std::error::Error;

use common::handbuilt::{self, HandBuilt};
use common::persistence::{save_in, world_at, world_holding};
use common::{TestResult, registry_of};
use mc_world::persistence::{self, LoadError, SavedPlayer};
use mc_world::world::WorldPos;
use tempfile::TempDir;

/// The one block the worlds here hold, and where it sits.
///
/// The world is incidental to every scenario in this file: a save records the
/// player whatever it holds, and holding one block rather than none keeps these
/// saves the same shape as the ones every other suite writes.
const HELD: &str = "fixture:andesite";
const A_CELL: WorldPos = world_at(1, 1, 1);

/// Where the player stood, for the scenario about standing somewhere.
///
/// Three coordinates that differ from each other and from every other number in
/// this file, so a reader that read one axis where another was meant lands on a
/// value this test reports rather than on the same number by coincidence. All
/// three are inside the fixture world's sixteen-block footprint, which is what
/// separates this from the out-of-footprint scenario below.
const STOOD_AT: [f32; 3] = [12.5, 40.0, 8.5];

/// Which way the player faced, in degrees, for the scenario about facing.
///
/// Converted to radians exactly once, by [`facing`], and both the value the
/// writer is handed and the value the reader is expected to report come from
/// there — so the conversion is not something either side could disagree about.
const TURNED_TO: f32 = 225.0;
const LOOKED_DOWN: f32 = -30.0;

/// Where the player stood when they walked off the edge.
///
/// The fixture world is one column a side, which is sixteen blocks, so twenty is
/// four blocks past its edge. Finite, and therefore legitimate.
const FOUR_BLOCKS_PAST_THE_EDGE: [f32; 3] = [20.0, 40.0, 8.5];

/// The coordinate a stored position is refused for, and the axis it sits on.
///
/// Placed on the **last** of the three coordinates deliberately. A reader that
/// checked the first coordinate and stopped would answer this scenario correctly
/// if the fixture put the bad value on `x`; putting it on `z` costs nothing and
/// catches that reader.
///
/// An infinity rather than a NaN, because the refusal has to carry the value and
/// a NaN is not equal to itself — an assertion naming one could never hold. Both
/// are "not a finite number" and the reader must refuse either.
const NOT_A_COORDINATE: f32 = f32::INFINITY;

/// The angle a stored facing is refused for.
const NOT_AN_ANGLE: f32 = f32::NEG_INFINITY;

/// Which way the player faced, in radians.
fn facing() -> (f32, f32) {
    (TURNED_TO.to_radians(), LOOKED_DOWN.to_radians())
}

/// A player standing at `position`, facing due north and level.
fn standing_at(position: [f32; 3]) -> SavedPlayer {
    SavedPlayer {
        position,
        yaw: 0.0,
        pitch: 0.0,
    }
}

/// What a save written with `player` in it reports the player to be.
fn saved_and_read_back(
    player: SavedPlayer,
) -> Result<Result<SavedPlayer, LoadError>, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let registry = registry_of(&[HELD])?;
    let world = world_holding(&[(A_CELL, HELD)], &registry)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, player, &registry)?;
    Ok(persistence::saved_player(&path))
}

/// What a save recording `player` and nothing else reports the player to be.
///
/// Built byte by byte, so the value under test sits at the offset this suite's
/// fixture says it does rather than at whichever offset the writer chose.
fn read_back_from_bytes(
    player: SavedPlayer,
) -> Result<Result<SavedPlayer, LoadError>, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let path = handbuilt::written(
        &directory,
        "recorded_by_hand.mcw",
        HandBuilt {
            player,
            ..HandBuilt::default()
        },
    )?;
    Ok(persistence::saved_player(&path))
}

#[test]
fn a_save_written_with_the_player_somewhere_reports_that_place() -> TestResult {
    let read_back = saved_and_read_back(standing_at(STOOD_AT))?;

    assert_eq!(
        read_back.map(|player| player.position),
        Ok(STOOD_AT),
        "the place is stored and not re-derived, because a resumed player put down on whatever \
         height the terrain reports is not standing where they left off — which is the whole of \
         what quitting and coming back is supposed to mean. Four bytes per coordinate, \
         little-endian, read back as the same four bytes: nothing here rounds, so nothing here is \
         allowed to be approximately right"
    );
    Ok(())
}

#[test]
fn a_save_written_with_the_player_turned_and_looking_down_reports_that_facing() -> TestResult {
    let (yaw, pitch) = facing();

    let read_back = saved_and_read_back(SavedPlayer {
        position: STOOD_AT,
        yaw,
        pitch,
    })?;

    assert_eq!(
        read_back.map(|player| (player.yaw, player.pitch)),
        Ok((yaw, pitch)),
        "facing is half of where you left off and it is the half a resume is most likely to drop, \
         because a world that comes back with the blocks in the right places looks correct until \
         the player notices they are pointing somewhere they never pointed. The two angles differ \
         in magnitude and in sign, so a save that stored one of them twice, or stored them the \
         other way round, reports a facing this assertion names"
    );
    Ok(())
}

#[test]
fn a_save_written_with_the_player_past_the_edge_of_the_world_reports_that_place() -> TestResult {
    let read_back = saved_and_read_back(standing_at(FOUR_BLOCKS_PAST_THE_EDGE))?;

    assert_eq!(
        read_back.map(|player| player.position),
        Ok(FOUR_BLOCKS_PAST_THE_EDGE),
        "the player is not confined to the world — walking off the edge is something the \
         simulation already allows and asks about — so a position four blocks past it is a save a \
         player can legitimately produce. This is the control for the two refusals below: a \
         reader that refused a position for being outside the footprint would satisfy both of \
         them and would refuse a world nobody did anything wrong to"
    );
    Ok(())
}

#[test]
fn a_stored_position_holding_a_coordinate_that_is_not_a_number_is_refused_by_axis() -> TestResult {
    let read_back = read_back_from_bytes(SavedPlayer {
        position: [12.5, 40.0, NOT_A_COORDINATE],
        yaw: 0.0,
        pitch: 0.0,
    })?;

    assert_eq!(
        read_back,
        Err(LoadError::NotFinite {
            axis: "z",
            value: NOT_A_COORDINATE
        }),
        "a coordinate that is not a finite number cannot be stood at, and every arithmetic done \
         to it afterwards spreads it: a position carried into the simulation infects a velocity, a \
         collision box and a camera before anything looks wrong on screen. Refusing it at the \
         boundary is the only place it is still one value with a name — and naming the axis is \
         what tells the difference between a save worth mending and one worth abandoning"
    );
    Ok(())
}

#[test]
fn a_stored_facing_holding_an_angle_that_is_not_a_number_is_refused_by_angle() -> TestResult {
    let read_back = read_back_from_bytes(SavedPlayer {
        position: STOOD_AT,
        yaw: NOT_AN_ANGLE,
        pitch: 0.0,
    })?;

    assert_eq!(
        read_back,
        Err(LoadError::NotFinite {
            axis: "yaw",
            value: NOT_AN_ANGLE
        }),
        "an angle that is not a finite number is the same failure one field along, and it is \
         checked separately because it is stored separately: a reader that validated the three \
         coordinates and took the two angles on trust would pass every position scenario in this \
         file and hand the simulation a look direction with no direction in it"
    );
    Ok(())
}
