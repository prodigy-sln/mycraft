//! The floor: what holds the player up, when it stops doing so, and the arc a
//! jump traces before it comes back.
//!
//! **Ground contact is asserted as an end-of-tick query and never as a memory.**
//! The two scenarios that pin it point in opposite directions on purpose — a
//! player walking clear of a ledge must *lose* contact on the tick nothing is
//! beneath its box, and a player whose feet lie exactly on a voxel's top face
//! must *report* it — so an implementation that carried the previous tick's
//! answer forward, or that answered a constant, fails one of the two whichever
//! way it leans. That is also why the jump's arc is asserted here: what makes
//! "the feet are back on the floor at tick 35" well-defined is that contact is
//! re-decided after the tick's last axis has been resolved.
//!
//! **The arc's numbers are the discrete integrator's, not the continuum's.** A
//! jump sets the vertical velocity, the same tick's gravity takes its first
//! bite before the position moves, and the position integrates what is left.
//! Under that ordering the apex is 1.275 blocks and not the `v²/2g` = 1.35 a
//! textbook gives — a figure no correct implementation here could ever produce.
//! Every height below is therefore the closed form of that sum
//! ([`risen_after`]), which is an independent derivation of the same arithmetic
//! rather than a second copy of the subject's loop, and never a value read off a
//! run.
//!
//! Comparisons use the declared 1 × 10⁻⁴ epsilon, except where *unchanged* is
//! the claim: that is a question about bits, which is both its exact form and
//! the form `clippy::float_cmp` has no quarrel with.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_sim::player::{MovementIntent, PlayerState, Solidity, advance_player};

use support::solidity::Ground;

type TestResult = Result<(), Box<dyn Error>>;

/// How far two figures this feature calls equal may differ, in blocks or in
/// blocks per second. The specification's declared comparison epsilon.
const EPSILON: f32 = 1e-4;

/// How long one tick simulates, in seconds. Declared, never measured.
const TICK_DURATION: f32 = 1.0 / 60.0;

/// How fast falling accelerates, in blocks per second squared. Declared.
const GRAVITY: f32 = 30.0;

/// How fast a jump leaves the ground, in blocks per second. Declared.
const JUMP_SPEED: f32 = 9.0;

/// How far the player's box reaches from the feet centre on x and z. Declared.
const HALF_WIDTH: f32 = 0.3;

/// The topmost solid voxel of the floor the jump happens over, and where its top
/// face — and so a standing player's feet — therefore is.
const FLOOR_SURFACE: i32 = 63;
const FLOOR_TOP: f32 = (FLOOR_SURFACE + 1) as f32;

/// The column a fall is measured onto, and where its feet belong afterwards:
/// one block above the surface, because the surface voxel's top face is there.
const LANDING_SURFACE: i32 = 40;
const LANDING_TOP: f32 = (LANDING_SURFACE + 1) as f32;

/// How far above a surface every fall below starts.
///
/// Four blocks is long enough that the fall is unambiguously a fall and short
/// enough that it never reaches the terminal speed, so nothing here depends on
/// the clamp another scenario owns.
const DROP_HEIGHT: f32 = 4.0;

/// How long every fall below is given to land and settle.
///
/// A four-block fall takes 31 ticks and a six-block one 38, so 60 is a margin
/// rather than a measurement — and a player that has landed stays landed, so
/// giving it longer cannot change the answer.
const SETTLE_TICKS: u32 = 60;

/// How long a settled player is then watched for: ten seconds of ticks.
const RESTING_TICKS: u32 = 600;

/// The column two surface heights meet at, and the heights either side of it.
///
/// Adjacent columns differing by two blocks is what gives a box straddling both
/// a taller answer to find. The feet centre sits just east of the boundary, so
/// the box reaches back over the lower column while standing over the higher.
const STEP_BOUNDARY: i32 = 10;
const LOWER_SURFACE: i32 = 40;
const HIGHER_SURFACE: i32 = 42;
const HIGHER_TOP: f32 = (HIGHER_SURFACE + 1) as f32;

/// Where the feet centre stands for every fall below.
///
/// Off-lattice, and just east of the step fixture's boundary: the box reaches
/// back over the lower column while the feet centre stands over the higher one,
/// which is what makes the straddle a straddle rather than a stand.
const FEET_X: f32 = 10.05;

/// The first column with nothing in it, where the floor a walk starts on runs
/// out.
const LEDGE_EDGE: i32 = 10;

/// Where the feet centre stands on solid floor.
///
/// One block back from the ledge's edge, so the same position serves the walk
/// that leaves the floor and the jumps that come back down to it.
const STAND_X: f32 = 9.0;

/// How long the walk toward the ledge is given to leave it.
///
/// The box's trailing face has 1.3 blocks to cover at the declared walk speed,
/// which is 18 ticks; 40 leaves room for a slower answer to still be caught.
const LEDGE_TICKS: u32 = 40;

/// Where every test below puts the player on the axis its fixture ignores.
///
/// Different from every x in this file, so that a query reading a box's z where
/// it meant its x lands on a column of the ledge or the step that answers
/// differently, instead of on one that happens to agree.
const FIXTURE_Z: f32 = 3.5;

/// What the vertical velocity is at the end of the tick that jumped.
///
/// The jump sets the declared 9.0 and the same tick's gravity takes its first
/// 0.5 before anything can observe the state, because gravity is applied after
/// the jump and before the position moves. That ordering is the whole reason the
/// apex below is 1.275 blocks rather than 1.35.
const AFTER_LAUNCH: f32 = JUMP_SPEED - GRAVITY * TICK_DURATION;

/// How fast the player is already falling where a jump is asked for in mid-air.
const AIRBORNE_SPEED: f32 = -2.0;

/// Where that fall is a tick later: gravity's work alone, the jump having been
/// refused.
const REFUSED_JUMP: f32 = AIRBORNE_SPEED - GRAVITY * TICK_DURATION;

/// How many ticks after leaving the floor the jump is at its highest.
const APEX_TICKS: u32 = 17;

/// How many ticks after leaving the floor the feet are back on it.
const RETURN_TICKS: u32 = 35;

/// How far above the floor the feet stand `ticks` ticks after the jump left it.
///
/// The closed form of the declared integrator's sum rather than a second copy of
/// its loop: the tick that jumps sets the velocity to [`JUMP_SPEED`], each tick
/// takes `GRAVITY × TICK_DURATION` from it before the position moves, so after
/// `n` ticks the feet have risen `dt × (v₀n − g·dt·n(n+1)/2)`. At `n = 17` that
/// is the specification's declared apex of 1.275 blocks, at `n = 34` its
/// 0.141666, and at `n = 35` it is zero — the feet back on the floor.
const fn risen_after(ticks: f32) -> f32 {
    TICK_DURATION * (JUMP_SPEED * ticks - GRAVITY * TICK_DURATION * ticks * (ticks + 1.0) / 2.0)
}

/// The flat floor the jumps happen over.
fn floor() -> Ground {
    Ground::Flat {
        surface: FLOOR_SURFACE,
    }
}

/// The flat surface every fall below lands on.
fn landing() -> Ground {
    Ground::Flat {
        surface: LANDING_SURFACE,
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

/// A player let go from rest above a surface, out of contact with anything.
fn dropped_at(x: f32, surface_top: f32) -> PlayerState {
    PlayerState {
        position: Vec3::new(x, surface_top + DROP_HEIGHT, FIXTURE_Z),
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// An intent that asks for a jump and nothing else.
fn jumping() -> MovementIntent {
    MovementIntent {
        jump: true,
        ..MovementIntent::default()
    }
}

/// An intent that asks to walk forward and nothing else.
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

/// Where a player that jumped from the floor is `ticks` ticks later, having
/// asked for nothing else since.
fn after_jumping(ticks: u32, world: &dyn Solidity) -> PlayerState {
    let launched = advance_player(
        standing_at(Vec3::new(STAND_X, FLOOR_TOP, FIXTURE_Z)),
        &jumping(),
        world,
    );
    advance(
        launched,
        &MovementIntent::default(),
        world,
        ticks.saturating_sub(1),
    )
}

/// Whether the box around `state` has left the ledge entirely.
///
/// The fixture's own geometry rather than the physics': the ledge's last solid
/// column is the one before [`LEDGE_EDGE`], so nothing is beneath the box only
/// once its *trailing* face has passed that column's far side. A box still
/// overhanging solid ground is still standing on it, which is why this is not
/// asked of the feet centre.
fn cleared_the_ledge(state: &PlayerState) -> bool {
    state.position.x - HALF_WIDTH >= LEDGE_EDGE as f32
}

#[test]
fn a_fall_comes_to_rest_one_block_above_the_surface_it_landed_on() -> TestResult {
    let floor = landing();

    let landed = advance(
        dropped_at(FEET_X, LANDING_TOP),
        &MovementIntent::default(),
        &floor,
        SETTLE_TICKS,
    );

    assert!(
        (landed.position.y - LANDING_TOP).abs() <= EPSILON,
        "a voxel fills [v, v + 1), so the top face of surface {LANDING_SURFACE} is at \
         {LANDING_TOP} and that is where feet stand — not at {}, which is where a fall stopped \
         one block early or sank one block in ends up",
        landed.position.y
    );
    Ok(())
}

#[test]
fn a_player_at_rest_holds_its_height_and_its_contact_for_ten_seconds() -> TestResult {
    let floor = landing();
    let rested = advance(
        dropped_at(FEET_X, LANDING_TOP),
        &MovementIntent::default(),
        &floor,
        SETTLE_TICKS,
    );
    let mut state = rested;
    let mut disturbed = Vec::new();

    for tick in 1..=RESTING_TICKS {
        state = advance_player(state, &MovementIntent::default(), &floor);
        if state.position.y.to_bits() != rested.position.y.to_bits() || !state.on_ground {
            disturbed.push(format!(
                "tick {tick} left it at {} with contact {}",
                state.position.y, state.on_ground
            ));
        }
    }

    assert!(
        disturbed.is_empty(),
        "gravity pulls on a resting player every one of {RESTING_TICKS} ticks and the floor \
         answers every one of them, so the height stays exactly {} and contact stays reported: \
         {} ticks disagreed, the first {:?}",
        rested.position.y,
        disturbed.len(),
        disturbed.first()
    );
    Ok(())
}

#[test]
fn walking_clear_of_a_ledge_loses_ground_contact() -> TestResult {
    let ledge = Ground::Ledge {
        edge: LEDGE_EDGE,
        surface: FLOOR_SURFACE,
    };
    let mut state = standing_at(Vec3::new(STAND_X, FLOOR_TOP, FIXTURE_Z));
    let mut departed = None;

    for _ in 0..LEDGE_TICKS {
        state = advance_player(state, &walking_forward(), &ledge);
        if cleared_the_ledge(&state) {
            departed = Some(state);
            break;
        }
    }

    assert!(
        departed.is_some_and(|state| !state.on_ground),
        "contact is a question asked of the world at the end of every tick, not a memory of \
         having been on the ground: the first tick with no solid voxel under any part of the box \
         reports none. The walk ended at {:?}",
        departed.map(|state| (state.position.x, state.on_ground))
    );
    Ok(())
}

#[test]
fn feet_lying_exactly_on_a_voxels_top_face_report_ground_contact() -> TestResult {
    let floor = floor();
    let touching = PlayerState {
        on_ground: false,
        ..standing_at(Vec3::new(STAND_X, FLOOR_TOP, FIXTURE_Z))
    };

    let settled = advance_player(touching, &MovementIntent::default(), &floor);

    assert!(
        settled.on_ground,
        "a voxel fills [v, v + 1), so a box whose bottom face is exactly on a voxel's top face \
         overlaps nothing at all — and a contact test that only asked about overlap would call \
         that player airborne. Lowering the box by the declared epsilon first is what makes \
         standing on the ground distinguishable from hovering a nothing above it"
    );
    Ok(())
}

#[test]
fn a_box_straddling_two_column_heights_comes_to_rest_on_the_taller() -> TestResult {
    let step = Ground::Step {
        boundary: STEP_BOUNDARY,
        west: LOWER_SURFACE,
        east: HIGHER_SURFACE,
    };

    let landed = advance(
        dropped_at(FEET_X, HIGHER_TOP),
        &MovementIntent::default(),
        &step,
        SETTLE_TICKS,
    );

    assert!(
        (landed.position.y - HIGHER_TOP).abs() <= EPSILON,
        "the box reaches {HALF_WIDTH} blocks either side of {FEET_X}, so it covers the \
         column of surface {LOWER_SURFACE} as well as the one of surface {HIGHER_SURFACE}, and \
         the first thing it meets on the way down is the taller: {HIGHER_TOP}, not {}",
        landed.position.y
    );
    Ok(())
}

#[test]
fn landing_stops_the_fall_rather_than_carrying_its_speed_on() -> TestResult {
    let floor = landing();
    let mut state = dropped_at(FEET_X, LANDING_TOP);
    let mut landing = None;

    for _ in 0..SETTLE_TICKS {
        state = advance_player(state, &MovementIntent::default(), &floor);
        if state.on_ground {
            landing = Some(state);
            break;
        }
    }

    assert!(
        landing.is_some_and(|state| state.velocity.y.abs() <= EPSILON),
        "the tick a fall is stopped by the ground is the tick its speed goes, or the next tick \
         starts by driving the player into the floor with everything the fall had built up. The \
         fall ended at {:?}",
        landing.map(|state| (state.position.y, state.velocity.y))
    );
    Ok(())
}

#[test]
fn a_jump_from_the_ground_leaves_at_the_declared_jump_speed() -> TestResult {
    let floor = floor();
    let standing = standing_at(Vec3::new(STAND_X, FLOOR_TOP, FIXTURE_Z));

    let launched = advance_player(standing, &jumping(), &floor);

    assert!(
        (launched.velocity.y - AFTER_LAUNCH).abs() <= EPSILON,
        "a jump sets the vertical velocity to the declared {JUMP_SPEED} blocks per second, and \
         the same tick's gravity takes its first {} of it before the state can be looked at, so \
         the tick ends at {AFTER_LAUNCH} and not at {}",
        GRAVITY * TICK_DURATION,
        launched.velocity.y
    );
    Ok(())
}

#[test]
fn a_jump_asked_for_in_mid_air_does_nothing_to_the_fall() -> TestResult {
    let airborne = PlayerState {
        velocity: Vec3::new(0.0, AIRBORNE_SPEED, 0.0),
        on_ground: false,
        ..standing_at(Vec3::new(STAND_X, FLOOR_TOP + DROP_HEIGHT, FIXTURE_Z))
    };

    let asked = advance_player(airborne, &jumping(), &Ground::Void);

    assert!(
        (asked.velocity.y - REFUSED_JUMP).abs() <= EPSILON,
        "a jump is honoured from the ground and from nowhere else, so a player already falling \
         at {AIRBORNE_SPEED} is falling at {REFUSED_JUMP} a tick later — gravity's work and \
         nothing else — rather than at the {} a second jump would have given it",
        asked.velocity.y
    );
    Ok(())
}

#[test]
fn a_jump_held_through_a_landing_launches_again_on_the_next_tick() -> TestResult {
    let floor = floor();
    let standing = standing_at(Vec3::new(STAND_X, FLOOR_TOP, FIXTURE_Z));

    let relaunched = advance(standing, &jumping(), &floor, RETURN_TICKS + 1);

    assert!(
        (relaunched.velocity.y - AFTER_LAUNCH).abs() <= EPSILON,
        "the arc lands on tick {RETURN_TICKS}, so the tick after it starts from the ground with \
         a jump still asked for, and a jump asked for from the ground is honoured whether or not \
         the request is a new one: {AFTER_LAUNCH}, not {}",
        relaunched.velocity.y
    );
    Ok(())
}

#[test]
fn the_jump_reaches_its_apex_seventeen_ticks_after_leaving_the_floor() -> TestResult {
    let floor = floor();

    let apex = after_jumping(APEX_TICKS, &floor);

    let risen = apex.position.y - FLOOR_TOP;
    assert!(
        (risen - risen_after(APEX_TICKS as f32)).abs() <= EPSILON,
        "the declared integrator takes gravity's bite before the position moves, so the highest \
         the feet get is {} blocks and not the 1.35 the continuous v²/2g gives — a figure no \
         correct implementation of this model can produce. This one reached {risen}",
        risen_after(APEX_TICKS as f32)
    );
    Ok(())
}

#[test]
fn thirty_four_ticks_after_the_jump_the_feet_are_still_above_the_floor() -> TestResult {
    let floor = floor();

    let falling = after_jumping(RETURN_TICKS - 1, &floor);

    let risen = falling.position.y - FLOOR_TOP;
    let declared = risen_after((RETURN_TICKS - 1) as f32);
    assert!(
        (risen - declared).abs() <= EPSILON && !falling.on_ground,
        "the tick before the arc closes, the feet are still {declared} blocks clear of the floor \
         with nothing under them — the descent is symmetric with the climb, one tick short of \
         landing. This one was {risen} blocks up with contact {}",
        falling.on_ground
    );
    Ok(())
}

#[test]
fn thirty_five_ticks_after_the_jump_the_feet_are_back_on_the_floor() -> TestResult {
    let floor = floor();

    let landed = after_jumping(RETURN_TICKS, &floor);

    assert!(
        (landed.position.y - FLOOR_TOP).abs() <= EPSILON && landed.on_ground,
        "the arc closes exactly on the floor at tick {RETURN_TICKS}, where the rise the \
         integrator has accumulated comes back to zero: the feet belong at {FLOOR_TOP} in \
         contact with the ground, not at {} with contact {}",
        landed.position.y,
        landed.on_ground
    );
    Ok(())
}
