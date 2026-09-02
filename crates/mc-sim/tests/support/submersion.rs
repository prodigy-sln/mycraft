//! Whether a player that has sunk to the bed of a body of water has its eye
//! inside that water, or over its surface.
//!
//! **The whole of this module is a reading, and it is taken over the shipped
//! path rather than beside it.** Where the eye stands is
//! [`mc_sim::player::eye_pose`]'s answer — the same call the simulation
//! publishes as a tick's camera — so a change to how the eye is derived from the
//! feet arrives here rather than being mirrored past it. Nothing below restates
//! [`mc_sim::player::EYE_HEIGHT`]: a fixture holding its own copy of the value
//! under test could never report that it moved, and reporting exactly that is
//! what this reading exists for.
//!
//! **The deepest column is the witness, and it is the only one that needs to
//! be.** A resting eye stands `depth − EYE_HEIGHT` blocks under a body of water
//! whose surface is level, because the feet rest on the run's own floor and the
//! surface stands `depth` above it. That quantity rises with depth and with
//! nothing else, so a body of water whose deepest column leaves the eye dry has
//! no column that does not — which is what makes one reading over one column an
//! answer about the whole sea.
//!
//! **Every verdict is enumerated and every figure is reported.** The reading
//! answers which of two things happened, never whether something was absent, so
//! a volume that has stopped holding any water at all arrives as an error and a
//! dry eye arrives as its own verdict carrying the margin — neither can be
//! mistaken for the submerged answer, and neither passes.

use std::error::Error;

use glam::Vec3;
use mc_sim::player::eye_pose;
use mc_sim::replay::{BlockVolume, ResolvedVoxels};
use mc_world::section::Contents;

use super::described;
use super::sea::{adrift, require_resting_at, rested};

/// What a position no volume reaches is called.
///
/// A third answer beside a block's name and the word for a cell holding
/// nothing, never folded into either: a volume that reached nowhere would
/// otherwise read as a volume holding nothing everywhere, and the dry verdict
/// would be reported about a world that was not there.
pub const OUTSIDE: &str = "no such cell";

/// What the reading says when the eye's own cell holds the water.
pub const INSIDE_THE_MEDIUM: &str = "the eye stands inside the medium";

/// What it says when that cell holds anything else.
pub const OVER_THE_SURFACE: &str = "the eye stands over the surface";

/// How far over a body of water's own top face a settle begins, in blocks.
///
/// A height to fall from and never a resting place; where the fall stops is the
/// volume's answer and is checked against the run's floor rather than assumed.
const FALL_FROM: f32 = 4.0;

/// A body of water a reading can be taken over: what it holds, the same volume
/// resolved for a tick to advance against, the block it is made of, and how long
/// a fall into it is watched for.
///
/// **The volume and its resolution are both carried because they answer
/// different questions.** A fall is advanced against what the registry resolved;
/// what the eye's cell holds is read off the volume's own declaration. Deriving
/// the second from the first would make the reading agree with whatever the
/// medium table happened to say.
pub struct Submergible<'a> {
    pub volume: &'a dyn BlockVolume,
    pub voxels: &'a ResolvedVoxels,
    /// The block this body of water is made of, spelled as content spells it.
    pub water: &'a str,
    /// How long a fall into it is watched for, in ticks.
    pub watch: u32,
}

/// A contiguous run of water in one column, and the two faces that bound it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterColumn {
    pub x: u32,
    pub z: u32,
    /// The lowest cell of the run.
    pub bottom: u32,
    /// The highest cell of the run.
    pub top: u32,
}

impl WaterColumn {
    /// How many water voxels the run holds.
    #[must_use]
    pub const fn depth(self) -> u32 {
        self.top - self.bottom + 1
    }

    /// Where a player standing on the bed rests its feet: the run's own floor,
    /// which is the top face of whatever holds the water up.
    ///
    /// An expectation rather than a fact about the volume, and [`eye_at_rest`]
    /// refuses a settle that ends anywhere else — so a run standing over
    /// nothing is reported instead of being asserted about.
    #[must_use]
    pub const fn lakebed(self) -> f32 {
        self.bottom as f32
    }

    /// The water's own top face.
    #[must_use]
    pub const fn surface(self) -> f32 {
        (self.top + 1) as f32
    }
}

/// Where a resting eye stands against the surface over it.
///
/// **Two verdicts and no third**, so "the eye's cell is one I could not read" is
/// not one of them: it arrives as an error from the reading instead. Each
/// verdict carries the column it is about and the margin, which is what a
/// scenario naming *how far* has to compare.
#[derive(Debug, Clone, PartialEq)]
pub enum EyeAtRest {
    /// The eye's own cell holds the water, `below` blocks under its top face.
    InsideTheMedium {
        column: WaterColumn,
        holds: String,
        eye: f32,
        below: f32,
    },
    /// It holds something else, `above` blocks over that top face.
    OverTheSurface {
        column: WaterColumn,
        holds: String,
        eye: f32,
        above: f32,
    },
}

impl EyeAtRest {
    /// The verdict for an eye at `eye` whose own cell holds `holds`.
    fn of(column: WaterColumn, holds: String, eye: f32, water: &str) -> Self {
        if holds == water {
            return Self::InsideTheMedium {
                column,
                holds,
                eye,
                below: column.surface() - eye,
            };
        }
        Self::OverTheSurface {
            column,
            holds,
            eye,
            above: eye - column.surface(),
        }
    }

    /// Which column the reading was taken on.
    #[must_use]
    pub const fn column(&self) -> WaterColumn {
        match self {
            Self::InsideTheMedium { column, .. } | Self::OverTheSurface { column, .. } => *column,
        }
    }

    /// The reading as figures a scenario states absolutely.
    #[must_use]
    pub fn stated(&self) -> StatedReading {
        let (verdict, holds, eye, margin) = match self {
            Self::InsideTheMedium {
                holds, eye, below, ..
            } => (INSIDE_THE_MEDIUM, holds, *eye, *below),
            Self::OverTheSurface {
                holds, eye, above, ..
            } => (OVER_THE_SURFACE, holds, *eye, *above),
        };
        StatedReading {
            verdict,
            eye_cell_holds: holds.clone(),
            depth: self.column().depth(),
            lakebed_top_face: blocks(self.column().lakebed()),
            surface_top_face: blocks(self.column().surface()),
            eye: blocks(eye),
            margin: blocks(margin),
        }
    }
}

/// Everything the reading answers, as values a scenario states by hand and
/// compares whole.
///
/// **Seven members and seven distinct failures.** A verdict that flipped, a cell
/// holding something else, a depth that changed, a bed or a surface at another
/// height, an eye derived differently and a margin that moved are seven
/// different defects, and a comparison that folded them together would report
/// whichever one it noticed first.
///
/// The four heights are carried as text at this suite's comparison epsilon —
/// four places, so a disagreement of `1e-4` blocks shows — because a margin is
/// the difference of two heights, and asking whether such a difference is
/// bitwise some decimal fraction is a question about the last bit of a
/// subtraction rather than about the reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatedReading {
    pub verdict: &'static str,
    pub eye_cell_holds: String,
    pub depth: u32,
    pub lakebed_top_face: String,
    pub surface_top_face: String,
    pub eye: String,
    pub margin: String,
}

/// A height or a margin, in blocks, at this suite's comparison epsilon.
#[must_use]
pub fn blocks(height: f32) -> String {
    format!("{height:.4}")
}

/// Where a player that has sunk to the bed of `sea`'s deepest column ends up
/// standing, against the surface over it.
///
/// # Errors
///
/// Returns an error if the volume holds no water at all, if the fall has not
/// come to rest inside the watch, or if it came to rest anywhere but the run's
/// own floor.
pub fn eye_at_rest(sea: &Submergible<'_>) -> Result<EyeAtRest, Box<dyn Error>> {
    let column = deepest_water_column(sea)?;
    let from = Vec3::new(
        column.x as f32 + 0.5,
        column.surface() + FALL_FROM,
        column.z as f32 + 0.5,
    );
    let rest = rested(adrift(from), sea.voxels, sea.watch)?;
    require_resting_at(
        rest.state,
        column.lakebed(),
        "the bed of the deepest column",
    )?;
    let at = eye_pose(&rest.state).eye;
    let [_, eye, _] = at;
    Ok(EyeAtRest::of(column, cell_holding(sea, at), eye, sea.water))
}

/// The column of `sea` whose run of water is deepest.
///
/// **The filter and the ranking, stated apart.** A column is admitted when it
/// holds any water at all; of those, the one whose topmost run is deepest wins,
/// ties going to the first in the walk's own order — x fastest, then z. The
/// tie-break decides only which of several equally deep columns is read, never
/// what is read about it.
///
/// # Errors
///
/// Returns an error if no column of the volume holds the water.
pub fn deepest_water_column(sea: &Submergible<'_>) -> Result<WaterColumn, Box<dyn Error>> {
    let extent = sea.volume.extent();
    let columns = (0..extent.z).flat_map(|z| (0..extent.x).map(move |x| (x, z)));
    let mut deepest: Option<WaterColumn> = None;
    for column in columns.filter_map(|(x, z)| water_run(sea, x, z)) {
        if deepest.is_none_or(|held| column.depth() > held.depth()) {
            deepest = Some(column);
        }
    }
    deepest.ok_or_else(|| {
        format!(
            "no column of this volume holds `{}`, so there is no body of water for a reading \
             about standing inside one to be taken over",
            sea.water
        )
        .into()
    })
}

/// The topmost run of `sea`'s water in one column.
fn water_run(sea: &Submergible<'_>, x: u32, z: u32) -> Option<WaterColumn> {
    let top = (0..sea.volume.extent().y)
        .rev()
        .find(|&y| is_water(sea, x, y, z))?;
    let bottom = (0..=top)
        .rev()
        .take_while(|&y| is_water(sea, x, y, z))
        .last()?;
    Some(WaterColumn { x, z, bottom, top })
}

/// Whether the cell at a position holds `sea`'s water.
fn is_water(sea: &Submergible<'_>, x: u32, y: u32, z: u32) -> bool {
    matches!(
        sea.volume.block_at(x, y, z),
        Some(Contents::Holds(name)) if name.as_str() == sea.water
    )
}

/// What the cell a point stands in holds: the block's own name, the word for a
/// cell holding nothing, or [`OUTSIDE`].
///
/// **Floored on all three axes and saturated on none of them.** A negative
/// coordinate is outside every volume and says so, rather than arriving at
/// column zero as an unsigned conversion would leave it.
fn cell_holding(sea: &Submergible<'_>, at: [f32; 3]) -> String {
    let cell = at.map(|axis| u32::try_from(axis.floor() as i64).ok());
    let [Some(x), Some(y), Some(z)] = cell else {
        return OUTSIDE.to_owned();
    };
    sea.volume
        .block_at(x, y, z)
        .map_or_else(|| OUTSIDE.to_owned(), described)
}
