//! What a block's volume is to move through follows a reload, not only an edit.
//!
//! # Why this exists beside the scenarios about a declared medium
//!
//! The world resolves what it holds into views the tick reads directly, and there
//! are **two** places any of them is written: one edit at a time, and the whole
//! registry replaced by content read while the game was running. The scenarios in
//! `player_buoyancy.rs` and `player_resistance.rs` resolve a view once and walk
//! through it; not one of them says anything whatever about the second.
//!
//! That a medium cannot be left behind by a wholesale replacement is an argument
//! about how the code is shaped, and **an argument is not a witness**.
//! `reload_solidity_views.rs` and `reload_targeting_views.rs` are the same
//! question asked about the other two answers a voxel carries, and this file is
//! modelled on them. What is narrower here is that a medium is not a bit: it is an
//! index into a table the registry decides the width of, so a replacement that
//! kept the old table, or kept the old indices against a new table, answers with
//! some *other* declaration's medium rather than with a stale one — and the tick
//! cannot tell the difference.
//!
//! # The instrument is the player, because there is no second view to disagree
//! with
//!
//! Solidity has two readings that can be played off each other — the bitset and
//! the block store through the registry. A medium has one: nothing outside the
//! physics asks what a volume does to movement. So what is read here is what the
//! tick produces, on the tick after the swap, which is the only place the answer
//! surfaces at all.
//!
//! **The resistance half reads `velocity` and never a difference of positions.**
//! The divisor acts on the velocity and carries forward, so the velocity is the
//! quantity the scenario is about; recovering a displacement by subtracting two
//! positions is the reading this project has measured as agreeing only near the
//! origin.
//!
//! **The buoyancy half compares a tick that asked to jump against an identical
//! tick that did not**, rather than reading a height. A tick held against a block
//! can end higher than it began without any jump having been honoured, and a fall
//! that happens to end at the same height as a launch is not a launch.
//!
//! # Every root here restates water whole, and the two roots differ in one field
//!
//! Water is the shipped block that declares a medium, so it is the declaration an
//! author edits. Each fixture writes it out in full on both sides and moves one
//! field, which is a constraint on the fixture that no assertion can enforce: a
//! root restating water with fewer fields than the shipped one states would be
//! several edits at once, and `drawn`, `occludes` and `targetable` would each fall
//! back to its own `solid = false`.

#[path = "support/roots.rs"]
mod roots;
mod support;

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use glam::Vec3;
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_sim::action::TickIntent;
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::simulation::{Simulation, seat};
use mc_sim::world::World;
use mc_world::content::LuauFileDefinitionSource;
use mc_world::world::{VoxelWorld, WorldPos};

use roots::{Adoption, WATER_FILE, accepted, adoption, shipped};
use support::{DIRT, STONE, TestResult, WATER, published_content};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// How far the fixture world's floor and its water reach on each horizontal axis:
/// the whole column.
const SPAN: u32 = 16;

/// The rows the stone floor fills, and the rows the water fills above it.
///
/// The floor's top face is at `y = 8.0`, so a player standing on it has its feet
/// there and its box — 1.8 blocks tall — reaches into rows 8 and 9, both of which
/// hold water. That is what makes the standing player submerged rather than merely
/// beside the sea.
const FLOOR_ROWS: (u32, u32) = (0, 8);
const WATER_ROWS: (u32, u32) = (8, 16);

/// Where the feet of a player standing on the floor sit.
const STANDING_FEET: Vec3 = Vec3::new(8.5, 8.0, 3.5);

/// Where the feet of a player adrift in the water sit: four rows above the floor,
/// so nothing it does is decided by ground contact.
const ADRIFT_FEET: Vec3 = Vec3::new(8.5, 12.0, 3.5);

/// How fast a walk carries the player, in blocks per second. Declared, never
/// measured.
const WALK_SPEED: f32 = 4.5;

/// What the water declares before the reload, and what it declares after.
///
/// **Powers of two less one**, so `1 + resistance` is a power of two and the
/// division is exact in `f32` — which is what lets the readings below be bit
/// equalities rather than comparisons against a tolerance somebody widened until
/// it passed.
const RESISTANCE_SERVED: f32 = 1.0;
const RESISTANCE_RELOADED: f32 = 3.0;

/// The ascent every root above states, which is the value the loader supplies to
/// a declaration that says nothing.
///
/// Written out rather than left absent, so that each root here states every
/// medium field it has and none of them stands as a default beside two that were
/// declared. The value is the loader's own, so stating it changes nothing about
/// what those scenarios measure.
const ASCENT_UNCHANGED: f32 = 9.0;

/// What the water declares about carrying a swimmer upward before the reload and
/// after it, and the resistance both sides of that scenario state.
///
/// The three are the specification's own fixture and are not free to be chosen
/// for arithmetic convenience: `1 + 0.5` is not a power of two, so the readings
/// below are compared against a tolerance rather than by bits.
const RESISTANCE_UNDER_THE_ASCENT: f32 = 0.5;
const ASCENT_SERVED: f32 = 3.5;
const ASCENT_RELOADED: f32 = 1.5;

/// How fast a held jump leaves the tick under each of those two declarations, in
/// blocks per second.
///
/// A closed form and never a figure read off a run: a launch replaces the
/// vertical velocity outright, gravity takes one tick's bite before the medium
/// divides, so the tick ends at `(ascent − GRAVITY · TICK_DURATION) / (1 +
/// resistance)`. With `GRAVITY = 30.0` and `TICK_DURATION = 1/60` that bite is
/// `0.5`, giving `(3.5 − 0.5) / 1.5 = 2.0` and `(1.5 − 0.5) / 1.5 = 0.666667`.
const RISE_SERVED: f32 = 2.0;
const RISE_RELOADED: f32 = 0.6667;

/// How far a measured rate may sit from the stated one, in blocks per second.
///
/// **A ceiling derived from both directions rather than a figure loosened until
/// something passed.**
///
/// From below: `0.6667` is the specification's own rounding of `2/3`, and the
/// value the arithmetic actually produces is `0.66666669` — `3.3e-5` away. The
/// `f32` path contributes far less than that: `GRAVITY · TICK_DURATION` rounds to
/// exactly `0.5`, both subtractions are exact, and the single division is
/// correctly rounded, so one ulp near `0.667` is `6.0e-8`. The transcription
/// dominates and the floor is `3.4e-5`. This sits a factor of three above it.
///
/// From above: the nearest wrong answer this has to reject is a rise that came
/// out at `1.0` — what a tick gives if it applied the reloaded ascent and either
/// skipped gravity's bite or skipped the medium's division — a gap of `0.333`,
/// three thousand times this. Every other wrong answer in reach is further off: a
/// stale view reports the served `2.0` twice, and one falling back on the
/// loader's default ascent gives `5.667`.
const TOLERANCE: f32 = 1e-4;

#[test]
fn a_reload_that_only_raises_a_resistance_slows_the_very_next_tick_of_a_walk() -> TestResult {
    let mut simulation = swimming_in(
        &water_declaring(true, RESISTANCE_SERVED, ASCENT_UNCHANGED),
        STANDING_FEET,
    )?;
    let before = walked(&mut simulation);
    let candidate = shipped()?.restating(
        WATER_FILE,
        &water_declaring(true, RESISTANCE_RELOADED, ASCENT_UNCHANGED),
    )?;

    let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(
        &mut simulation,
        candidate.candidate()?,
    ));
    let after = walked(&mut simulation);

    assert_eq!(
        (answered, before.to_bits(), after.to_bits()),
        (
            accepted(DIRT),
            (WALK_SPEED / (1.0 + RESISTANCE_SERVED)).to_bits(),
            (WALK_SPEED / (1.0 + RESISTANCE_RELOADED)).to_bits()
        ),
        "an author changed one number in one file and the very next tick of the walk is slower. \
         No cell of the world was written and the world was not rebuilt, so the only thing that \
         can carry the new number to the tick is the wholesale replacement the reload performs — \
         one that took the new registry and kept what the old one had resolved about the volume \
         goes on dividing by the old value, and reports the first of these two figures twice. The \
         acceptance is asserted beside them because a *refused* reload also leaves the walk \
         exactly as it was"
    );
    Ok(())
}

#[test]
fn a_reload_that_only_gives_a_block_buoyancy_lets_the_very_next_jump_off_the_ground_lift()
-> TestResult {
    let sunk = water_declaring(false, RESISTANCE_SERVED, ASCENT_UNCHANGED);
    let buoyant = water_declaring(true, RESISTANCE_SERVED, ASCENT_UNCHANGED);

    let lifts = (
        a_jump_lifts(&sunk, None)?,
        a_jump_lifts(&sunk, Some(&buoyant))?,
    );

    assert_eq!(
        lifts,
        (false, true),
        "the water an author was serving held nobody up, and the file they saved says it does. A \
         jump asked for with nothing under the feet was refused before the swap and is honoured \
         after it, on the tick that follows — which is what says the reload rebuilt the answer \
         rather than the one the launch resolved. Read as a jumping tick against an identical \
         tick that asked for nothing, because a tick can end higher than it began without any \
         jump having been honoured"
    );
    Ok(())
}

#[test]
fn a_reload_that_takes_a_blocks_buoyancy_away_stops_the_very_next_jump_off_the_ground_lifting()
-> TestResult {
    let buoyant = water_declaring(true, RESISTANCE_SERVED, ASCENT_UNCHANGED);
    let sunk = water_declaring(false, RESISTANCE_SERVED, ASCENT_UNCHANGED);

    let lifts = (
        a_jump_lifts(&buoyant, None)?,
        a_jump_lifts(&buoyant, Some(&sunk))?,
    );

    assert_eq!(
        lifts,
        (true, false),
        "an author took `swimmable = true` back out of the file, and a player who could hold \
         itself up in that water on one tick cannot on the next. **This is the direction a stale \
         view survives in**: a replacement that kept what it had already resolved goes on lifting \
         the player through water the content no longer says anything of the kind about, and the \
         author's edit is invisible until the game is restarted. The pair is read on the same \
         instrument as the scenario above it, so the two differ in which answer the reload \
         produced rather than in whether it produced one"
    );
    Ok(())
}

#[test]
fn a_reload_that_only_lowers_a_swim_ascent_slows_the_very_next_held_jump() -> TestResult {
    let mut simulation = swimming_in(
        &water_declaring(true, RESISTANCE_UNDER_THE_ASCENT, ASCENT_SERVED),
        ADRIFT_FEET,
    )?;
    let before = a_held_jump_ends_at(&mut simulation);
    let candidate = shipped()?.restating(
        WATER_FILE,
        &water_declaring(true, RESISTANCE_UNDER_THE_ASCENT, ASCENT_RELOADED),
    )?;

    let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(
        &mut simulation,
        candidate.candidate()?,
    ));
    require_admitted(&answered)?;
    let after = a_held_jump_ends_at(&mut simulation);

    assert!(
        (before - RISE_SERVED).abs() <= TOLERANCE && (after - RISE_RELOADED).abs() <= TOLERANCE,
        "an author lowered one number in one file and the very next tick of the held jump carries \
         the swimmer more slowly: {RISE_SERVED} blocks per second before the swap and \
         {RISE_RELOADED} after it, within {TOLERANCE}, and these ticks ended at {before} and \
         {after}. No cell of the world was written and the world was not rebuilt, so the only \
         thing that can carry the new number to the tick is the wholesale replacement the reload \
         performs — one that kept what the old registry had resolved about the volume goes on \
         launching at the old rate and reports {RISE_SERVED} twice. **Both readings, because \
         either alone is satisfied by a failure**: the first says the fixture really was lifting \
         the swimmer at the declared rate to begin with, so the second is a change rather than a \
         volume that never carried anybody"
    );
    Ok(())
}

/// What `base:water` says when an author has stated its medium as given and left
/// everything else exactly as the shipped declaration has it.
///
/// Written out in full rather than as a diff against the shipped file, so that a
/// reader can see the whole of what the author's root says — and so that the two
/// roots a scenario compares differ in the one field it names and in nothing that
/// fell back to a default.
///
/// **Three medium arguments and not two with a default**, for the reason the
/// whole-file restatement above exists: a builder that states one medium field
/// and lets another stand would put two roots' difference somewhere the caller
/// never wrote it.
fn water_declaring(swimmable: bool, move_resistance: f32, swim_ascent: f32) -> String {
    [
        "return {".to_owned(),
        format!("\tname = \"{WATER}\","),
        format!("\ttexture = \"{WATER}\","),
        "\tsolid = false,".to_owned(),
        "\tbreakable = false,".to_owned(),
        "\treplaceable = true,".to_owned(),
        "\tdrawn = true,".to_owned(),
        "\toccludes = false,".to_owned(),
        "\ttargetable = true,".to_owned(),
        format!("\tswimmable = {swimmable},"),
        // Debug rather than Display, so a whole number reaches the file as `1.0`
        // rather than `1`: both are numbers the loader takes, and a fixture that
        // means "a number" should not become one that means "an integer" for the
        // values that happen to be round.
        format!("\tmove_resistance = {move_resistance:?},"),
        format!("\tswim_ascent = {swim_ascent:?},"),
        "}".to_owned(),
        String::new(),
    ]
    .join("\n")
}

/// Whether a tick that asked to jump ends higher than an identical tick that
/// asked for nothing, for a player adrift in the water `serving` declares —
/// optionally after a reload to the water `reloaded` declares.
///
/// Two simulations rather than one rewound, because a tick is not undoable: they
/// are built from the same root and given the same reload, so the only difference
/// between them is the intent.
///
/// # Errors
///
/// Returns an error if a root cannot be written or read, or if a reload was
/// refused — a refusal would leave both halves under the serving declaration and
/// the comparison would be about a swap that never happened.
fn a_jump_lifts(serving: &str, reloaded: Option<&str>) -> Result<bool, Box<dyn Error>> {
    let jumped = one_tick(serving, reloaded, jumping())?;
    let unjumped = one_tick(serving, reloaded, MovementIntent::default())?;
    Ok(jumped > unjumped)
}

/// Where one tick under `intent` leaves the feet of a player adrift in the water
/// `serving` declares, optionally after a reload to `reloaded`.
///
/// # Errors
///
/// Returns an error if a root cannot be written or read, or if the reload was
/// refused.
fn one_tick(
    serving: &str,
    reloaded: Option<&str>,
    intent: MovementIntent,
) -> Result<f32, Box<dyn Error>> {
    let mut simulation = swimming_in(serving, ADRIFT_FEET)?;
    if let Some(reloaded) = reloaded {
        let candidate = shipped()?.restating(WATER_FILE, reloaded)?;
        let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(
            &mut simulation,
            candidate.candidate()?,
        ));
        require_admitted(&answered)?;
    }
    simulation.advance(TickIntent {
        movement: intent,
        action: None,
    });
    Ok(simulation.latest().player.position.y)
}

/// The velocity along the walk's own axis after one tick of a full-deflection
/// walk.
///
/// The velocity and not a difference of positions: the divisor acts on the
/// velocity and carries forward, so this is the quantity the resistance is about,
/// and a displacement recovered as `end − start` agrees with it only near the
/// origin.
fn walked(simulation: &mut Simulation) -> f32 {
    simulation.advance(TickIntent {
        movement: MovementIntent {
            forward: 1.0,
            ..MovementIntent::default()
        },
        action: None,
    });
    simulation.latest().player.velocity.x
}

/// The vertical velocity one tick of a held jump leaves the player at.
///
/// The velocity and not a difference of positions, for the reason the walk's own
/// reading gives: a launch *sets* the velocity, so that is the quantity the
/// declared ascent is about, and a displacement recovered as `end − start` agrees
/// with it only near the origin.
fn a_held_jump_ends_at(simulation: &mut Simulation) -> f32 {
    simulation.advance(TickIntent {
        movement: jumping(),
        action: None,
    });
    simulation.latest().player.velocity.y
}

/// An intent that asks for a jump and nothing else.
fn jumping() -> MovementIntent {
    MovementIntent {
        jump: true,
        ..MovementIntent::default()
    }
}

/// A simulation of a stone floor under a body of water, with the player's feet at
/// `feet` and its box inside that water.
///
/// The root is the shipped one with water restated as `serving` says, read
/// through the same door a candidate is read through.
///
/// # Errors
///
/// Returns an error if the root cannot be written or read, or if the world does
/// not build.
fn swimming_in(serving: &str, feet: Vec3) -> Result<Simulation, Box<dyn Error>> {
    let root = shipped()?.restating(WATER_FILE, serving)?;
    let registry = Arc::new(registry_over(root.path())?);
    let mut blocks = VoxelWorld::empty(COLUMNS);
    fill(&mut blocks, FLOOR_ROWS, STONE, &registry)?;
    fill(&mut blocks, WATER_ROWS, WATER, &registry)?;
    let spawn = PlayerState {
        position: feet,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        // Stated rather than derived, and false even for the standing fixture:
        // what the first tick makes of the ground is the physics' answer to give,
        // and a spawn claiming contact it has not been given would be this fixture
        // deciding it.
        on_ground: false,
    };
    let content = published_content(&registry)?;
    let world = World::new(blocks, Arc::clone(&registry))?;
    Ok(seat(spawn, world, content).simulation)
}

/// Writes `name` into every cell of `blocks` whose row lies in `rows`.
///
/// # Errors
///
/// Returns an error if `name` is not a namespaced id, or if the world refuses a
/// write.
fn fill(
    blocks: &mut VoxelWorld,
    rows: (u32, u32),
    name: &str,
    registry: &BlockRegistry,
) -> Result<(), Box<dyn Error>> {
    let block = BlockName::parse(name)?;
    for y in rows.0..rows.1 {
        fill_row(blocks, y, &block, registry)?;
    }
    Ok(())
}

/// Writes `block` into every cell of `blocks` in the row at `y`.
///
/// # Errors
///
/// Returns an error if the world refuses a write.
fn fill_row(
    blocks: &mut VoxelWorld,
    y: u32,
    block: &BlockName,
    registry: &BlockRegistry,
) -> Result<(), Box<dyn Error>> {
    for x in 0..SPAN {
        for z in 0..SPAN {
            blocks.set_block(WorldPos { x, y, z }, block, registry)?;
        }
    }
    Ok(())
}

/// A registry holding what the content root at `root` declares.
///
/// # Errors
///
/// Returns an error if the root is refused.
fn registry_over(root: &Path) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root.to_owned()))?;
    Ok(registry)
}

/// Refuses unless the candidate was admitted at all.
fn require_admitted(answered: &Adoption) -> Result<(), Box<dyn Error>> {
    if matches!(answered, Adoption::Accepted { .. }) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the candidate restating water to be admitted, and it answered \
         {answered:?}. Nothing about the medium could then have moved, and the comparison would be \
         about a swap that never happened"
    )
    .into())
}
