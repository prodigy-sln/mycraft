//! The captures this repository has committed a frame for, redrawn and read
//! back beside the blobs on disk.
//!
//! # Why the set is enumerated rather than named
//!
//! `crates/mc-render/goldens/` holds four directories and every one of them is a
//! picture some run has to keep producing. Three are terrain captures shot
//! through `record_terrain`; the fourth is the HUD capture shot through
//! `record_frame`, the one call the windowed client makes. **A reading that
//! named three of the four would be silent about the one the HUD lives in**, and
//! a reading that named a list of its own would go on agreeing with itself the
//! day a fifth capture was declared. So the list comes from
//! [`declared_capture_ids`], the shooting is dispatched on which kind each id
//! is, and the count is checked back against the directories that actually
//! exist — a capture nothing here redrew is a capture this reading did not judge.
//!
//! # What is compared, and why it is bytes
//!
//! The frame drawn now against the bytes committed for it, with no tolerance at
//! all. That is a stronger claim than the golden lifecycle's own — its default
//! thresholds forgive `0.0001 × 1280 × 720` = 92 wrong pixels — and it is the
//! claim this spec is entitled to make: where the eye stands in nothing that
//! declares a tint the strength written into the frame record is the literal
//! `0.0`, and `mix(a, b, 0.0)` returns `a` bit-exactly under every form a backend
//! compiles it into. There is no branch and no second code path, so a correct
//! implementation moves no bit of a dry frame and a reading that allowed 92
//! wrong pixels would be blind to a tint that reached ninety of them.
//!
//! **Nothing here mints.** The blob is opened with [`read_png`] and compared by
//! hand, so a run under `MYCRAFT_UPDATE_GOLDENS` cannot turn this comparison
//! into a photograph of itself. The opt-in is read and reported anyway, because
//! a run with it set may have had the committed bytes rewritten underneath it by
//! a golden binary in the same invocation.
//!
//! # The tint is resolved for every capture, and that is what makes this evidence
//!
//! Both shooters resolve the eye's medium through the simulation's own resolver:
//! the terrain captures through [`super::goldens::drawn_over`] and the HUD
//! capture through [`HudCapture::over`], each going to
//! [`super::frames::snapshot_in`]. **A capture shot with a hard-coded absence of
//! a tint would match its committed bytes whatever the draw path did**, because
//! the fixture rather than the world would have decided the frame was dry — and
//! this whole reading, and the golden lifecycle beside it, would be a statement
//! about a renderer that cannot tint.

use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use mc_render::capture::{
    DECLARED_CAPTURE_TICKS, HUD_CAPTURE_TICKS, SCENE_REVISION, capture_id, declared_capture_ids,
    hud_capture_id,
};
use mc_testkit::frame::{OptIns, Rgba8Image, read_png};

use super::goldens::{drawn_over, golden_root};
use super::hud_frames::{HudCapture, hud_holding_default_block};

/// The file the golden lifecycle keeps a capture's frame in.
const GOLDEN_FILE: &str = "default.png";

/// Which call a capture is recorded through, which is the whole of what tells
/// the two kinds apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Through {
    /// `record_terrain`, below the client's frame call.
    TheTerrainPassAlone,
    /// `record_frame`, the one call the windowed client makes.
    TheClientsWholeFrame,
}

/// One capture this repository has committed a frame for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    pub id: String,
    pub tick: u16,
    pub through: Through,
}

impl Committed {
    /// Where this capture's committed frame sits.
    ///
    /// # Errors
    ///
    /// Returns an error if the repository root cannot be located.
    pub fn blob(&self) -> Result<PathBuf, Box<dyn Error>> {
        Ok(golden_root()?.join(&self.id).join(GOLDEN_FILE))
    }

    /// The pixels committed for this capture.
    ///
    /// # Errors
    ///
    /// Returns the decode failure, and the read failure for a capture whose
    /// directory is not there — which is a committed frame this reading was
    /// asked about and cannot open, rather than a capture that matched.
    pub fn committed_pixels(&self) -> Result<Rgba8Image, Box<dyn Error>> {
        Ok(read_png(&self.blob()?)?)
    }
}

/// Every capture the current scene revision declares, in the order its ids are
/// declared in.
///
/// **Built from the two tick declarations and named by the two id functions**,
/// which is how [`declared_capture_ids`] itself builds the list this is checked
/// against below. A capture whose id neither function produces is one no golden
/// lifecycle would ever look for.
///
/// # Errors
///
/// Returns the name failure for a tick whose id cannot be a directory name.
pub fn declared() -> Result<Vec<Committed>, Box<dyn Error>> {
    let mut declared = Vec::new();
    for tick in DECLARED_CAPTURE_TICKS {
        declared.push(Committed {
            id: capture_id(tick, SCENE_REVISION)?,
            tick,
            through: Through::TheTerrainPassAlone,
        });
    }
    for tick in HUD_CAPTURE_TICKS {
        declared.push(Committed {
            id: hud_capture_id(tick, SCENE_REVISION)?,
            tick,
            through: Through::TheClientsWholeFrame,
        });
    }
    Ok(declared)
}

/// How many capture directories stand under the golden root.
///
/// **Counted off the filesystem rather than taken from the declaration**, so a
/// reading claiming to have judged every committed capture has something to be
/// wrong against: a directory nothing above redrew, or a declaration with no
/// directory behind it, moves this number away from the number compared.
///
/// # Errors
///
/// Returns the read failure. A golden root that is not there holds nothing,
/// which is a state to report rather than an error to raise.
pub fn directories_committed() -> Result<usize, Box<dyn Error>> {
    let read = match fs::read_dir(golden_root()?) {
        Ok(read) => read,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    let mut standing = 0;
    for entry in read {
        entry?;
        standing += 1;
    }
    Ok(standing)
}

/// Whether the ids above are exactly the ids the capture library declares.
///
/// The one place this module's own list can drift from the product's, stated as
/// a value a reading can put inside its verdict rather than as a comment.
///
/// # Errors
///
/// Returns the name failure.
pub fn the_declared_ids_agree() -> Result<bool, Box<dyn Error>> {
    let mut named: Vec<String> = declared()?.into_iter().map(|capture| capture.id).collect();
    let mut library = declared_capture_ids(SCENE_REVISION)?;
    named.sort();
    library.sort();
    Ok(named == library)
}

/// Whether the run that is reading these blobs may also be rewriting them.
#[must_use]
pub fn the_golden_update_opt_in_is_set() -> bool {
    OptIns::from_environment().update_goldens
}

/// The frame `capture` draws now, over the content root at `root`, or `None`
/// when the opt-in permitted the absence of a device.
///
/// # Errors
///
/// Returns the preparation, pipeline, spawn or capture failure.
pub fn drawn(capture: &Committed, root: &Path) -> Result<Option<Rgba8Image>, Box<dyn Error>> {
    match capture.through {
        Through::TheTerrainPassAlone => drawn_over(root, capture.tick),
        Through::TheClientsWholeFrame => hud_drawn(capture.tick, root),
    }
}

/// The HUD capture at `tick` over the content root at `root`, shot the way
/// `hud_goldens.rs` shoots the one this repository committed: the client's own
/// default held block, its own layout, and its own frame call.
fn hud_drawn(tick: u16, root: &Path) -> Result<Option<Rgba8Image>, Box<dyn Error>> {
    let Some(context) = super::frames::device()? else {
        return Ok(None);
    };
    let mut capture = HudCapture::over(context.as_ref(), u32::from(tick), root)?;
    let shipped = hud_holding_default_block(root, &capture.content)?;
    let request = super::frames::request(&context, &hud_capture_id(tick, SCENE_REVISION)?)?;
    Ok(Some(capture.capture(&shipped, &request)?))
}

/// How a redrawn frame stands against the bytes committed for it.
///
/// **Classified rather than measured.** How many bytes two different pictures
/// differ in is whatever it is, and an expectation naming that number would be a
/// quantity copied from a run of the code under test — the exact thing that makes
/// a snapshotted count worthless. What a reader of a failure needs is which
/// capture moved and in which of the two ways, and both are here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Difference {
    /// It is not even the shape the committed frame is.
    ASizeTheCommittedFrameIsNot,
    /// The same shape, holding bytes the committed frame does not.
    BytesTheCommittedFrameDoesNotHold,
}

/// What redrawing a set of captures and comparing each against a committed frame
/// came to.
///
/// A total verdict for the reason [`Standing`] is one, and `compared` is carried
/// in both arms so a run that redrew nothing fails on the count before its
/// verdict is weighed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frames {
    /// Every capture drew exactly the bytes committed for it.
    EveryCommittedFrameIsDrawnByteForByte { compared: usize },
    /// The ones that did not, each named with what differed.
    Moved {
        compared: usize,
        moved: Vec<(String, Difference)>,
    },
}

/// Redraws the first of each pair over `root` and compares it, byte for byte,
/// against the frame committed for the second.
///
/// The two are separate so the comparator can be driven over a pair that has to
/// disagree. `None` where the opt-in permitted the absence of a device.
///
/// # Errors
///
/// Returns the preparation, pipeline, spawn, capture or decode failure.
pub fn frames_against(
    root: &Path,
    pairs: &[(Committed, Committed)],
) -> Result<Option<Frames>, Box<dyn Error>> {
    let mut moved = Vec::new();
    let mut compared = 0;
    for (shot_now, committed) in pairs {
        let Some(now) = drawn(shot_now, root)? else {
            return Ok(None);
        };
        compared += 1;
        moved.extend(differing(&now, committed)?);
    }
    Ok(Some(if moved.is_empty() {
        Frames::EveryCommittedFrameIsDrawnByteForByte { compared }
    } else {
        Frames::Moved { compared, moved }
    }))
}

/// How `now` stands against the bytes committed for `committed`, or nothing at
/// all where the two agree exactly.
fn differing(
    now: &Rgba8Image,
    committed: &Committed,
) -> Result<Option<(String, Difference)>, Box<dyn Error>> {
    let blob = committed.committed_pixels()?;
    let named = committed.id.clone();
    if blob.width() != now.width() || blob.height() != now.height() {
        return Ok(Some((named, Difference::ASizeTheCommittedFrameIsNot)));
    }
    if blob.as_bytes() == now.as_bytes() {
        return Ok(None);
    }
    Ok(Some((named, Difference::BytesTheCommittedFrameDoesNotHold)))
}
