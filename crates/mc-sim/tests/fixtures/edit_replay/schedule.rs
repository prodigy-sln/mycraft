//! The declared world of the scripted replay, its schedule, and the expectation
//! folded out of that schedule by arithmetic.
//!
//! **This fixture is compiled, unlike its neighbours.** The two under
//! `tests/fixtures/intent_shape/` exist to be *read* by a scan and say so in
//! their own headers; this one is pulled into `tests/edit_replay.rs` with
//! `#[path]`. What it shares with them is the thing that matters: everything
//! under `crates/mc-sim/tests/fixtures/**` is test-author-owned, so the oracle
//! this file *is* — the expectation every assertion is judged against — cannot
//! be edited by the context it judges. It lives here rather than in
//! `tests/support/`, which is implementation-owned, and rather than in
//! `tests/edit_replay.rs`, which would then stand over the 600-line limit for a
//! test file.
//!
//! # The world, and the aim
//!
//! Everything is arithmetic over the declaration, and no number here was read
//! off a run.
//!
//! **The eye does not move.** The feet stand at (8.5, 10.0, 9.5) on a floor
//! whose top face is 10.0, so the eye is 1.62 above them at (8.5, 11.62, 9.5); a
//! grounded player asked for no movement has its tick of fall resolved back onto
//! the floor's own face, so the state is a fixed point and the eye is the same
//! value on tick 1 and on tick 26 700.
//!
//! **Every targeted cell is the first solid cell along its ray.** Each working
//! column is aimed at the centre of its own wall cell at x = 12, 3.5 blocks away
//! horizontally. The ray crosses x = 11 at 5/6 of the way to that centre and
//! x = 12 at 7/8, and the aim is off the eye by at most 1.12 blocks in y and 1.0
//! in z, so at either crossing it sits within 0.14 of the centre — well inside
//! the half block that would put it in the next cell along. The ray therefore
//! meets the cell in front of the wall while that cell holds a block, and the
//! wall cell itself while it does not, and which of the two a break reaches is
//! [`Working::breaks`]'s own answer rather than the run's.
//!
//! **Reach.** The farthest hit is a wall cell in the corner of the working set,
//! entered at sqrt(3.5² + 0.98² + 0.875²) = 3.74 blocks, well inside the 5.0 the
//! specification fixes. The one operation outside it is [`BEYOND_THE_REACH`],
//! whose own note is the longest in this file and worth reading.
//!
//! **The refused placement lands in a cell that is occupied and not solid.** A
//! placement lands in the cell the ray came *from*, which the traversal tested
//! and found not solid before it advanced, so the only reachable occupied cell
//! holds a block that stops nobody and that content does not declare
//! replaceable. The overlay's unbuildable block sits at (6, 11, 9) with a solid
//! block behind it at (5, 11, 9): the ray passes through the first and stops at
//! the second, and the placement lands back in the first.
//!
//! # Why the run does not simply end where it started
//!
//! A cycle returns its cell to the block it was declared with, so the final
//! world grades only the *last* operation in each column. [`Finish`] is what
//! makes it discriminate: one lane ends built, one holding a residue no fixture
//! declared, one with its wall cell broken out, so the world ends nine cells
//! away from the declared one and a run that did nothing at all fails. Every
//! earlier operation is graded by its own answer, whose `from` names what its
//! cell held when it ran.

use std::error::Error;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, EditReport, Refusal};
use mc_sim::player::{BlockPos, PlayerState};
use mc_sim::world::SectionKey;
use mc_world::column::{ColumnCoordinate, SECTIONS_PER_COLUMN};
use mc_world::world::WorldPos;

use mc_world::section::Contents;

use crate::support::chamber::{BlockChamber, CRUMBLING, UNBREAKABLE, UNBUILDABLE, at};
use crate::support::{DIRT, NOTHING, STONE};

/// How many successful operations of each kind the criterion asks for.
const CRITERION: usize = 10_000;

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// Where the feet stand: on the floor's top face, between the two walls.
const FEET: Vec3 = Vec3::new(8.5, 10.0, 9.5);

/// The column the working set's wall cells stand in, and the column in front of
/// them — where a placement against a wall cell's near face lands, one step back
/// through the face the ray entered by.
const WALL_X: u32 = 12;
const FACING_X: u32 = WALL_X - 1;

/// The rows and lanes the nine working columns stand in. Three of each, so a
/// finish falls to a whole lane and the arithmetic below needs no division.
const ROWS: [u32; 3] = [10, 11, 12];
const LANES: [u32; 3] = [8, 9, 10];

/// The solid block half a block past the reach, on a ray meeting nothing before
/// it, and the cell a placement against it would land in.
///
/// **The operation aimed there is a *place*, and that is what makes the reach
/// falsifiable here rather than untestable.** Spelled as a break, a reach that
/// was never enforced would break that block on the first round — and every
/// round after it would then walk a ray that meets nothing at all, which against
/// a total `Solidity` does not terminate. The falsifier would be a hang instead
/// of a red assertion, which is the one thing a limit must not have; it was
/// measured, at 116 seconds and killed. A placement leaves the block standing
/// and fills the cell in front of it instead, so the mutant run finishes and
/// fails on its answers in 0.04 seconds.
pub const BEYOND_THE_REACH: WorldPos = at(8, 11, 15);
pub const SHORT_OF_THE_REACH: WorldPos = at(8, 11, 14);

/// The block content declares cannot be broken.
pub const INDESTRUCTIBLE: WorldPos = at(5, 12, 9);

/// The cell holding a block that is neither solid nor replaceable, and the solid
/// block behind it that the refused placement is aimed at.
pub const UNBUILDABLE_CELL: WorldPos = at(6, 11, 9);
pub const BEHIND_THE_UNBUILDABLE: WorldPos = at(5, 11, 9);

/// One round is the five-step cycle in every working column, then the three
/// operations that must be refused.
const PLACES_PER_ROUND: usize = ROWS.len() * LANES.len() * 2;
const BREAKS_PER_ROUND: usize = ROWS.len() * LANES.len() * 3;
const REFUSALS_PER_ROUND: usize = 3;

/// How many rounds it takes to reach the criterion's placements.
const ROUNDS: usize = CRITERION.div_ceil(PLACES_PER_ROUND);

/// What the whole schedule asks for: the rounds, plus a finish that places in
/// two of the three lanes and breaks in two, once per row.
pub const PLACES: usize = ROUNDS * PLACES_PER_ROUND + ROWS.len() * 2;
pub const BREAKS: usize = ROUNDS * BREAKS_PER_ROUND + ROWS.len() * 2;
pub const REFUSALS: usize = ROUNDS * REFUSALS_PER_ROUND;

/// The criterion, stated where it cannot drift away from the schedule.
const _: () = assert!(PLACES >= CRITERION);
const _: () = assert!(BREAKS >= CRITERION);

/// How many sections the footprint holds: its columns, each sixteen deep.
pub const SECTIONS_IN_THE_FOOTPRINT: usize = (COLUMNS * COLUMNS * SECTIONS_PER_COLUMN) as usize;

/// What a working cell holds as the schedule folds it.
///
/// An enum rather than a name, so "what breaking this leaves behind" is a total
/// function *this file* owns. It is the expectation; the registry is what the
/// run consults, and an expectation read out of the registry would be the two
/// agreeing with themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    /// The cell holds no block at all. Not the chamber's background, not a
    /// block that stands in for absence — nothing.
    Empty,
    Stone,
    Crumbling,
    Dirt,
}

impl Held {
    /// What this cell holds: the name content spells the block with, or nothing.
    const fn contents(self) -> Contents<&'static str> {
        match self {
            Self::Empty => Contents::Empty,
            Self::Stone => Contents::Holds(STONE),
            Self::Crumbling => Contents::Holds(CRUMBLING),
            Self::Dirt => Contents::Holds(DIRT),
        }
    }

    /// What this cell holds, as text — a block by name, or [`NOTHING`].
    const fn described(self) -> &'static str {
        match self.contents() {
            Contents::Empty => NOTHING,
            Contents::Holds(name) => name,
        }
    }

    /// Whether a ray stops at a cell holding this.
    const fn is_solid(self) -> bool {
        !matches!(self, Self::Empty)
    }

    /// What breaking this leaves in the cell: the block the fixture registry's
    /// declaration names, and the world's fill where it names none.
    const fn residue(self) -> Self {
        match self {
            Self::Crumbling => Self::Dirt,
            _ => Self::Empty,
        }
    }
}

/// How a working column is left when the run ends — one per lane, so the three
/// lanes end differently and the final world is satisfied neither by a run that
/// dug nothing out nor by one that filled nothing in.
///
/// `Built` leaves a placed block standing. `Crumbled` places a crumbling block
/// and breaks it, so the cell holds the residue its definition names and no
/// fixture declared. `Holed` breaks the wall cell itself, so the world ends
/// short a block it was declared with.
#[derive(Debug, Clone, Copy)]
enum Finish {
    Built,
    Crumbled,
    Holed,
}

const FINISHES: [Finish; 3] = [Finish::Built, Finish::Crumbled, Finish::Holed];

/// What one operation asks for.
#[derive(Debug, Clone, Copy)]
enum Asked {
    Break,
    Place(Contents<&'static str>),
}

/// The answer the schedule derives for one operation.
#[derive(Debug, Clone)]
enum Answer {
    Changed {
        cell: WorldPos,
        from: Contents<&'static str>,
        to: Contents<&'static str>,
    },
    Refused(Refusal),
}

/// One operation: the cell whose centre the view is turned onto, what is asked
/// for there, and what the schedule derives must come back.
#[derive(Debug, Clone)]
pub struct Step {
    aim: WorldPos,
    asked: Asked,
    answer: Answer,
}

impl Step {
    /// The cell whose centre the view is turned onto for this operation.
    pub const fn aim(&self) -> WorldPos {
        self.aim
    }

    /// What this operation asks the simulation for.
    ///
    /// # Errors
    ///
    /// Returns the refusal if the block it names is not a namespaced id.
    pub fn intent(&self) -> Result<ActionIntent, Box<dyn Error>> {
        Ok(match self.asked {
            Asked::Break => ActionIntent::Break,
            // A placement names a block, and nothing is not a block. Nothing in
            // this file schedules one, and a schedule that ever did would be
            // asking for an operation the request type cannot express — so it is
            // reported here rather than quietly dropped, which would shorten the
            // run below the criterion with every count still agreeing.
            Asked::Place(Contents::Empty) => {
                return Err("a placement names a block; this schedule places nothing".into());
            }
            Asked::Place(Contents::Holds(block)) => ActionIntent::Place {
                block: BlockName::parse(block)?,
            },
        })
    }

    /// The answer the schedule derives for this operation, as a report.
    ///
    /// # Errors
    ///
    /// Returns the refusal if a block it names is not a namespaced id.
    pub fn derived_report(&self) -> Result<Option<EditReport>, Box<dyn Error>> {
        Ok(Some(match &self.answer {
            Answer::Changed { cell, from, to } => EditReport::Changed {
                cell: voxel(*cell),
                from: parsed(*from)?,
                to: parsed(*to)?,
            },
            Answer::Refused(refusal) => EditReport::Refused(refusal.clone()),
        }))
    }

    /// Whether the schedule derives a refusal for this operation.
    pub const fn is_refused(&self) -> bool {
        matches!(self.answer, Answer::Refused(_))
    }

    /// Whether the schedule derives a change for this operation.
    pub const fn is_changed(&self) -> bool {
        matches!(self.answer, Answer::Changed { .. })
    }

    /// Whether this operation asks for a placement rather than a break.
    pub const fn is_place(&self) -> bool {
        matches!(self.asked, Asked::Place(_))
    }
}

/// One column of the working set, and what the fold has left in it.
#[derive(Debug, Clone, Copy)]
struct Working {
    /// The cell every operation in this column is aimed at.
    wall: WorldPos,
    /// The cell in front of it, where a placement against the wall lands.
    facing: WorldPos,
    wall_holds: Held,
    facing_holds: Held,
    finish: Finish,
}

impl Working {
    /// Asks for `block` in the cell in front of the wall.
    fn places(&mut self, steps: &mut Vec<Step>, block: Held) {
        steps.push(Step {
            aim: self.wall,
            asked: Asked::Place(block.contents()),
            answer: Answer::Changed {
                cell: self.facing,
                from: self.facing_holds.contents(),
                to: block.contents(),
            },
        });
        self.facing_holds = block;
    }

    /// Asks for a break along this column's one ray.
    ///
    /// Which cell that reaches is the fold's own answer and not the run's: the
    /// cell in front of the wall while it holds a block, and the wall cell
    /// itself while it does not.
    fn breaks(&mut self, steps: &mut Vec<Step>) {
        let (cell, held) = if self.facing_holds.is_solid() {
            (self.facing, &mut self.facing_holds)
        } else {
            (self.wall, &mut self.wall_holds)
        };
        steps.push(Step {
            aim: self.wall,
            asked: Asked::Break,
            answer: Answer::Changed {
                cell,
                from: held.contents(),
                to: held.residue().contents(),
            },
        });
        *held = held.residue();
    }

    /// The operations that leave this column holding what its lane's finish
    /// says it holds when the run is over.
    fn finished(&mut self, steps: &mut Vec<Step>) {
        match self.finish {
            Finish::Built => self.places(steps, Held::Stone),
            Finish::Crumbled => {
                self.places(steps, Held::Crumbling);
                self.breaks(steps);
            }
            Finish::Holed => self.breaks(steps),
        }
    }
}

/// Every operation of the run in order, and the world the fold walks through as
/// it is built.
#[derive(Debug)]
pub struct Schedule {
    steps: Vec<Step>,
    columns: Vec<Working>,
}

impl Schedule {
    /// The whole run: [`ROUNDS`] rounds, then the finish.
    #[must_use]
    pub fn of_the_whole_run() -> Self {
        let mut schedule = Self {
            steps: Vec::new(),
            columns: working_columns(),
        };
        for _ in 0..ROUNDS {
            schedule.round();
        }
        let Self { steps, columns } = &mut schedule;
        for column in columns.iter_mut() {
            column.finished(steps);
        }
        schedule
    }

    /// Every operation, in the order it is asked for.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// One round: dig out and refill every working column, then ask for the
    /// three operations that must be refused.
    fn round(&mut self) {
        let Self { steps, columns } = self;
        for column in columns.iter_mut() {
            column.places(steps, Held::Stone);
            column.breaks(steps);
            column.places(steps, Held::Crumbling);
            column.breaks(steps);
            column.breaks(steps);
        }
        steps.extend(refusals());
    }

    /// Every cell the schedule leaves holding a block other than the one the
    /// chamber declares it with.
    #[must_use]
    pub fn expected_differences(&self) -> Vec<(WorldPos, String, String)> {
        let mut moved: Vec<(WorldPos, String, String)> = self
            .columns
            .iter()
            .flat_map(|column| {
                [
                    (column.facing, Held::Empty, column.facing_holds),
                    (column.wall, Held::Stone, column.wall_holds),
                ]
            })
            .filter(|(_, declared, now)| declared != now)
            .map(|(cell, declared, now)| {
                (
                    cell,
                    declared.described().to_owned(),
                    now.described().to_owned(),
                )
            })
            .collect();
        // `support::chamber::differences` walks `Extent::positions()`, which is
        // y, then z, then x, and yields its rows in that order — so this sorts
        // the same way. **The agreement is load-bearing and lives nowhere else:**
        // if that walk ever ran another way, the assertion comparing the two
        // would go red for a reason that is not a defect, and this is where a
        // reader is told to look.
        moved.sort_by_key(|(cell, _, _)| (cell.y, cell.z, cell.x));
        moved
    }

    /// How many operations the schedule requires to be refused.
    #[must_use]
    pub fn refused_count(&self) -> usize {
        self.steps.iter().filter(|step| step.is_refused()).count()
    }
}

/// The declared world: nothing anywhere, one layer of floor to stand on, the
/// nine wall cells the working columns are aimed at, and the three cells the
/// refused operations need.
#[must_use]
pub fn chamber() -> BlockChamber {
    let floored =
        BlockChamber::empty(COLUMNS).run(at(0, FLOOR_LAYER, 0), at(16, FLOOR_LAYER + 1, 16), STONE);
    working_columns()
        .iter()
        .fold(floored, |chamber, column| chamber.cell(column.wall, STONE))
        .cell(BEHIND_THE_UNBUILDABLE, STONE)
        .cell(INDESTRUCTIBLE, UNBREAKABLE)
        .cell(UNBUILDABLE_CELL, UNBUILDABLE)
        .cell(BEYOND_THE_REACH, STONE)
}

/// A player standing still on the floor between the two walls, facing +x.
#[must_use]
pub fn spawn() -> PlayerState {
    PlayerState {
        position: FEET,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    }
}

/// A section of the one column this footprint has.
#[must_use]
pub const fn section(index: usize) -> SectionKey {
    SectionKey {
        column: ColumnCoordinate { x: 0, z: 0 },
        index,
    }
}

/// A declared cell's contents as a report carries them.
///
/// # Errors
///
/// Returns the refusal if the name it carries is not a namespaced id.
fn parsed(contents: Contents<&'static str>) -> Result<Contents<BlockName>, Box<dyn Error>> {
    Ok(match contents {
        Contents::Empty => Contents::Empty,
        Contents::Holds(name) => Contents::Holds(BlockName::parse(name)?),
    })
}

/// A world position as the signed voxel a report names it by.
#[must_use]
pub const fn voxel(cell: WorldPos) -> BlockPos {
    BlockPos {
        x: cell.x as i32,
        y: cell.y as i32,
        z: cell.z as i32,
    }
}

/// The nine columns the schedule digs out and refills, one finish per lane.
fn working_columns() -> Vec<Working> {
    ROWS.iter()
        .flat_map(|&row| {
            LANES
                .iter()
                .zip(FINISHES)
                .map(move |(&lane, finish)| Working {
                    wall: at(WALL_X, row, lane),
                    facing: at(FACING_X, row, lane),
                    wall_holds: Held::Stone,
                    facing_holds: Held::Empty,
                    finish,
                })
        })
        .collect()
}

/// The operations that must be refused, in the order they close a round.
fn refusals() -> [Step; REFUSALS_PER_ROUND] {
    [
        refused(
            BEYOND_THE_REACH,
            Asked::Place(Contents::Holds(STONE)),
            Refusal::NoTarget,
        ),
        refused(INDESTRUCTIBLE, Asked::Break, Refusal::Indestructible),
        refused(
            BEHIND_THE_UNBUILDABLE,
            Asked::Place(Contents::Holds(STONE)),
            Refusal::Occupied,
        ),
    ]
}

/// One operation the schedule derives a refusal for.
fn refused(aim: WorldPos, asked: Asked, refusal: Refusal) -> Step {
    Step {
        aim,
        asked,
        answer: Answer::Refused(refusal),
    }
}
