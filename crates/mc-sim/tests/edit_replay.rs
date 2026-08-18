//! Twenty-five thousand edits in one continuous run, and the world they leave.
//!
//! The only assertion in the suite that covers a whole run rather than a
//! declared fixture, and so the only one that could catch a resolution wrong
//! somewhere nobody wrote a scenario about. Every operation goes through
//! `Simulation::advance` — the same request, targeting and edit path a click
//! reaches — one action per tick, and the world is judged by reading it back out
//! and diffing it against the fixture *as declared*.
//!
//! **The schedule, the declared world and the derived expectation live in
//! `tests/fixtures/edit_replay/schedule.rs`**, pulled in below. That directory
//! is test-author-owned exactly as this file is, so the oracle stays out of
//! reach of the context it judges; what it buys over keeping everything here is
//! two files a reader can hold, each inside the limit `code-quality.md` sets.
//! This file is the rig and the assertions, and nothing else.
//!
//! # The four things that keep a run this size from grading itself
//!
//! **The expected world is folded out of the schedule by arithmetic, never read
//! back from a run.** `Schedule` carries the block each working cell holds as it
//! is *built*, before any simulation exists, and every operation records the
//! answer that fold derives for it. A number snapshotted from the first green
//! run would commit whatever the code happened to do that day.
//!
//! **Aiming reuses the server's published *state* and never its *targeting*.**
//! [`turned_onto`] reads the eye out of the published snapshot and turns the view
//! onto the centre of a chosen cell with plain trigonometry. Nothing here asks
//! the simulation what it is looking at, so the raycast stays the subject.
//!
//! **The schedule contains operations that must be refused, each asserted by
//! name.** Without them "succeed at everything" and "correct" leave the same
//! world. They are a solid block just past the reach, a block content declares
//! unbreakable, and a placement into a cell content does not declare
//! replaceable. Per ruling 66 the first expects `NoTarget`: the reach is bounded
//! at one site, so "nothing is there" and "something is there but too far"
//! arrive at the same place and there is no `OutOfReach` to expect.
//!
//! **Every assertion is over block identities at coordinates.** A count cannot
//! see shape, so one is asserted for the criterion itself and for the number of
//! operations a selection picked out, and never in place of cells and names.
//!
//! The fixture constraints no assertion can enforce — that every targeted cell
//! is the first solid cell along its ray, that the eye never moves, that the
//! reach is cleared from both sides, and why the run does not end where it
//! started — are stated where the fixture is built, beside the arithmetic they
//! rest on.
//!
//! # Three properties only a run this long can show
//!
//! **The dirty set is bounded by the footprint's own section count** and not by
//! the edit count, because its key is a section rather than an edit.
//!
//! **`Section::compact` is off the edit path, and the reason is a bound, not
//! "nothing grows".** `Palette::replace` finds an existing entry before appending
//! one — including one whose refcount has fallen to zero — so after *N* edits
//! naming *K* blocks a palette holds at most its initial entries plus *K*. This
//! run names four, so no section here outgrows six however often it is dug out.
//!
//! **It runs headless: it never meshes and never uploads.** That, and not
//! margin, is why the renderer's fixed scene capacity is not on this path — ten
//! thousand breaks and ten thousand places in a pessimal world mesh to some
//! 243 000–263 000 quads against a capacity of 262 144, at or past it.

mod support;

#[path = "fixtures/edit_replay/schedule.rs"]
mod schedule;

use std::error::Error;
use std::f32::consts::TAU;

use glam::Vec3;
use mc_sim::action::{EditReport, TickIntent};
use mc_sim::player::MovementIntent;
use mc_sim::simulation::{SimSnapshot, Simulation, seat};
use mc_world::world::WorldPos;

use schedule::{
    BEHIND_THE_UNBUILDABLE, BEYOND_THE_REACH, BREAKS, INDESTRUCTIBLE, PLACES, REFUSALS,
    SECTIONS_IN_THE_FOOTPRINT, SHORT_OF_THE_REACH, Schedule, Step, UNBUILDABLE_CELL, chamber,
    section, spawn, voxel,
};
use support::chamber::{BlockChamber, UNBREAKABLE, UNBUILDABLE, differences, fixture_content};
use support::{NOTHING, STONE, TestResult, described};

#[test]
fn the_replay_leaves_every_cell_holding_the_block_the_schedule_derives_for_it() -> TestResult {
    let chamber = chamber();
    let schedule = Schedule::of_the_whole_run();
    let run = run(&schedule, &chamber)?;

    assert_eq!(
        differences(&chamber.build()?, run.simulation.world()),
        schedule.expected_differences(),
        "every cell of the world is compared against the fixture as declared, and the expected \
         side is folded out of the schedule before any simulation exists. The nine cells that \
         differ are the finish — one lane built, one crumbled, one holed — so a run that edited \
         nothing fails here as surely as one that edited the wrong cell"
    );
    Ok(())
}

#[test]
fn every_operation_the_schedule_requires_refused_is_refused_by_name_and_changes_nothing()
-> TestResult {
    let chamber = chamber();
    let schedule = Schedule::of_the_whole_run();
    let run = run(&schedule, &chamber)?;

    assert_eq!(
        (
            disagreements(&schedule, &run, Step::is_refused)?,
            schedule.refused_count(),
            held_at_the_refusals(&run)?
        ),
        (
            (0, None),
            REFUSALS,
            vec![
                (BEHIND_THE_UNBUILDABLE, STONE.to_owned()),
                (INDESTRUCTIBLE, UNBREAKABLE.to_owned()),
                (UNBUILDABLE_CELL, UNBUILDABLE.to_owned()),
                (SHORT_OF_THE_REACH, NOTHING.to_owned()),
                (BEYOND_THE_REACH, STONE.to_owned()),
            ]
        ),
        "every round asks for a placement against a block past the reach, a break against a block \
         content declares unbreakable, and a placement into a cell content does not declare \
         replaceable. Each has to come back under the name the schedule derives — a wrongly \
         shaped fixture would otherwise pass as a correct refusal — and each cell has to still \
         hold what it was declared with. The count is there because a selection picking none of \
         them out would report every one of them correct"
    );
    Ok(())
}

#[test]
fn the_run_answers_every_edit_with_the_change_the_schedule_derives_ten_thousand_times_over()
-> TestResult {
    let chamber = chamber();
    let schedule = Schedule::of_the_whole_run();
    let run = run(&schedule, &chamber)?;

    assert_eq!(
        (
            disagreements(&schedule, &run, Step::is_changed)?,
            reported(&schedule, &run)
        ),
        ((0, None), (PLACES, BREAKS)),
        "every answer carries the cell it changed and the two names it changed between, so an \
         edit taking the cell one step short of the hit, or one ignoring what content declares a \
         block breaks into, is caught where it happened rather than at the end of the run. The \
         two counts are read off the answers rather than assumed, and both stand above the ten \
         thousand the criterion fixes"
    );
    Ok(())
}

#[test]
fn the_whole_run_leaves_one_section_and_its_neighbour_waiting_to_be_meshed() -> TestResult {
    let chamber = chamber();
    let schedule = Schedule::of_the_whole_run();
    let mut run = run(&schedule, &chamber)?;
    let work = run
        .simulation
        .take_remesh_work()
        .ok_or("a run that edited the world reported nothing to re-mesh")?;

    assert_eq!(
        (
            work.keys().collect::<Vec<_>>(),
            work.keys().len() <= SECTIONS_IN_THE_FOOTPRINT
        ),
        (vec![section(0), section(1)], true),
        "nothing drained the dirty set for the whole run, and it holds two entries rather than \
         one per edit: every cell the schedule touches lies in the lowest section of the only \
         column this footprint has, and the section above it is the one face-adjacent neighbour \
         the footprint holds. The bound is the footprint's own section count, which is what keeps \
         it independent of how many edits pile up behind a drain"
    );
    Ok(())
}

/// One continuous run of the schedule, and what every operation answered.
#[derive(Debug)]
struct Run {
    simulation: Simulation,
    answers: Vec<Option<EditReport>>,
}

/// Drives the whole schedule through one simulation, one action per tick.
fn run(schedule: &Schedule, chamber: &BlockChamber) -> Result<Run, Box<dyn Error>> {
    let mut simulation = seat(spawn(), chamber.build()?, fixture_content()?).simulation;
    let mut answers = Vec::with_capacity(schedule.steps().len());
    for step in schedule.steps() {
        let movement = turned_onto(&simulation.latest(), step.aim());
        let action = Some(step.intent()?);
        answers.push(simulation.advance(TickIntent { movement, action }));
    }
    Ok(Run {
        simulation,
        answers,
    })
}

/// The look deltas that turn the published view onto the centre of `cell`.
///
/// Read off the **published eye** and the published orientation — the server's
/// own state, never its targeting — and applied to the orientation the tick
/// starts from, which is the one this snapshot carries.
fn turned_onto(published: &SimSnapshot, cell: WorldPos) -> MovementIntent {
    let eye = Vec3::from_array(published.camera.eye);
    let centre = Vec3::new(cell.x as f32, cell.y as f32, cell.z as f32) + Vec3::splat(0.5);
    let toward = centre - eye;
    MovementIntent {
        yaw_delta: toward.z.atan2(toward.x).rem_euclid(TAU) - published.player.yaw,
        pitch_delta: toward.y.atan2(toward.x.hypot(toward.z)) - published.player.pitch,
        ..MovementIntent::default()
    }
}

/// How many operations answered something other than what the schedule derives,
/// and the first of them: which operation, what was derived, what came back.
type Disagreements = (usize, Option<(usize, String, String)>);

/// Every operation `wanted` selects, judged against the schedule's own answer.
fn disagreements(
    schedule: &Schedule,
    run: &Run,
    wanted: fn(&Step) -> bool,
) -> Result<Disagreements, Box<dyn Error>> {
    let mut seen = 0;
    let mut first = None;
    for (index, (step, answered)) in schedule.steps().iter().zip(&run.answers).enumerate() {
        if !wanted(step) {
            continue;
        }
        let derived = step.derived_report()?;
        if &derived == answered {
            continue;
        }
        seen += 1;
        if first.is_none() {
            first = Some((index, format!("{derived:?}"), format!("{answered:?}")));
        }
    }
    Ok((seen, first))
}

/// How many placements, and how many breaks, came back as a change to the
/// world.
fn reported(schedule: &Schedule, run: &Run) -> (usize, usize) {
    let mut counts = (0, 0);
    for (step, answered) in schedule.steps().iter().zip(&run.answers) {
        if !matches!(answered, Some(EditReport::Changed { .. })) {
            continue;
        }
        if step.is_place() {
            counts.0 += 1;
        } else {
            counts.1 += 1;
        }
    }
    counts
}

/// What the world holds at each cell a refused operation was about, in the
/// order a world is walked in.
fn held_at_the_refusals(run: &Run) -> Result<Vec<(WorldPos, String)>, Box<dyn Error>> {
    [
        BEHIND_THE_UNBUILDABLE,
        INDESTRUCTIBLE,
        UNBUILDABLE_CELL,
        SHORT_OF_THE_REACH,
        BEYOND_THE_REACH,
    ]
    .into_iter()
    .map(|cell| {
        let held = run
            .simulation
            .world()
            .block_at(voxel(cell))
            .ok_or("the world reaches no cell the schedule aimed at")?;
        Ok((cell, described(held)))
    })
    .collect()
}
