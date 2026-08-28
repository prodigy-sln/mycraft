//! What a capture is called, and which captures the replay declares.
//!
//! A capture id names a directory of committed goldens, so it carries the
//! revision of the scene those goldens were shot from. That turns the day the
//! mesh contract changes from a silent re-shoot into a **rename**: the commit
//! shows added and removed files rather than a modified binary blob nobody can
//! read, and a bumped revision with no new goldens fails as a *missing* golden
//! naming the path it looked for.
//!
//! The revision is a parameter everywhere below and never read from
//! [`SCENE_REVISION`] inside. An id function that quietly substituted the
//! current revision would answer every question about the next one with the
//! current one's names — which is precisely the collision the revision exists to
//! prevent, arriving through the function meant to prevent it.
//!
//! # Why the alphabet is checked here
//!
//! An id becomes a path segment under the golden root. The capture harness
//! validates the same alphabet when it takes one, but it is a dev-dependency of
//! this crate and cannot be named from the library, so a revision that would not
//! survive that validation has to be refused where the id is built. The rule is
//! the harness's: lowercase `a-z`, `0-9`, `-` and `_`, which excludes both path
//! separators and `.`, so no revision can walk out of the directory it names.

use thiserror::Error;

/// The revision of the scene the committed goldens were captured from.
///
/// It names a **declared observation**, and it is bumped whenever a change makes
/// a same-named capture incomparable with the frames already committed under
/// that name — whether the change is to the **mesh contract** or to the
/// **declared camera path**, which since PRO-957 includes the physics the
/// scripted walk runs under.
///
/// **The tripwire sees only the first of those.** `crates/mc-sim`'s scene
/// contract compares quad count and per-block area, both properties of the mesh
/// alone; a camera path that moved over an unchanged world leaves every one of
/// them green, and the failure arrives as an image comparison instead. **There
/// is no guard for the second case.**
///
/// The narrower sentence this replaces — mesh contract only, tripwire fails
/// first — was true when it was written, and it is worth saying why rather than
/// leaving it to read as carelessness: the camera derived from the spawn and the
/// spawn from the world, so the trajectory *was* a function of the mesh
/// contract. Content-declared physics is a third input, and it severs that
/// chain.
///
/// **`r4` is the first bump taken *under* that sentence rather than the one that
/// caused it** — `r3` is what falsified the old wording. SPEC-030 moved what the sea
/// declares — a resistance of `0.5` where it was `1.6`, and an ascent of `3.5`
/// where there was none — and the scripted walk wades that sea, so every pose
/// after it enters the water moved while the mesh contract stood still. The
/// four `r3` directories were shot under the old declaration and no frame under
/// this one may be compared against them.
///
/// **`r5` is a third cause again, and it is the *vertex format*.** SPEC-031 gave
/// the packed vertex an eight-bit opacity field at shift 36, and a packed vertex
/// is a contract item a capture is a photograph of. The blended pass it feeds is
/// not what bumps this: a second draw over the same faces at the same depths
/// moves no pixel while nothing declares a degree below one, which phase 2
/// measured by verifying the `r4` set green after the whole renderer rework. Nor
/// is the content: `content/base/blocks/water.luau` declaring `opacity = 0.5`
/// changes what the frames *show*, and an art edit is expressly not a bump
/// (2026-08-26). It is the format alone, and the two arrive together because a
/// field nothing writes is a field nothing can photograph.
///
/// **The tripwire is blind here for the third time, and three is a pattern.**
/// `r3`, `r4` and `r5` all left `scene_contract.rs` green: their causes were the
/// declared physics, the declared physics again, and now the vertex format, none
/// of which is the mesh or the spawn. Every bump so far whose cause was neither
/// of those two has been invisible to the only automated instrument that runs
/// before an image is compared. `docs/technical/rendering.md` states it as a
/// property of the guard rather than as a third footnote.
pub const SCENE_REVISION: &str = "r5";

/// The ticks of the replay that carry a committed golden.
///
/// Three ticks of the declared intent script, chosen because they are three
/// different things the script puts in front of the camera: the spawn still
/// falling, the end of the straight walk, and the tick after the turn and the
/// jump. 59 rather than the 60 the orbit's half turn used to name — there is no
/// half turn any more, and 59 is the last tick before the script starts turning.
pub const DECLARED_CAPTURE_TICKS: [u16; 3] = [0, 59, 119];

/// The ticks of the replay that carry a committed golden *of a composed HUD*.
///
/// One, and deliberately not three. The HUD does not animate and the held block
/// is set once, so ticks 59 and 119 would assert the same rectangles against
/// different terrain.
///
/// **The reason tick 0 was chosen is no longer true, and the criterion that
/// replaced it does not decide either.** It was the frame with the least terrain
/// coverage, so the crosshair stood against the most sky, while the spawn was
/// inland.
///
/// **Measured per pixel against the declared clear colour, the ranking flips with
/// the declared physics.** Each row is the share of the frame standing more than
/// ΔE 10 from `CLEAR_COLOR_SRGB`, taken on that revision's own committed frames:
///
/// | revision | tick 0 | tick 59 | tick 119 | least covered |
/// |---|---|---|---|---|
/// | `r1` | 77.91% | 95.01% | 94.22% | tick 0, by 16.31 |
/// | `r2` | 57.11% | 71.05% | 55.00% | tick 119, by 2.11 |
/// | `r3` | 57.11% | 66.63% | 50.95% | tick 119, by 6.16 |
/// | `r4` | 57.11% | 67.62% | 59.20% | tick 0, by 2.09 |
///
/// Tick 0 is byte-identical across all four because it is dry — the spawn is
/// still falling and inland of the coast — while the two wet frames move whenever
/// the sea's declaration does. **A margin of two points that reverses on a content
/// edit is not a criterion**, and the selection has now reversed twice.
///
/// **The `r2` row stood in this comment through the `r3` bump without being
/// re-measured.** PRO-957 moved both wet frames and left the figures, so the
/// stated 2.1-point margin described a tree two revisions back while reading as
/// current. Nothing reads a doc comment, so a stale figure here is reported by
/// nothing at all — which is why every row above names the revision it was taken
/// on.
///
/// Tick 0 is kept, and the honest reason is that there is no measured one: the
/// set has always been shot here, moving it would re-shoot a capture for no
/// benefit anybody has demonstrated, and the HUD's own assertions pass against
/// this frame. That is a reason to leave it alone rather than a reason to have
/// picked it. **A later spec wanting maximum crosshair contrast must measure at
/// its own revision** rather than inheriting any row above.
///
/// A set of its own rather than a second use of [`DECLARED_CAPTURE_TICKS`]: the
/// two sets are shot through different calls — the terrain captures through
/// `record_terrain` and this one through the frame call the windowed client makes
/// — and a shared list would make one of those two choices look like a
/// consequence of the other.
pub const HUD_CAPTURE_TICKS: [u16; 1] = [0];

/// What every capture id of this replay begins with.
///
/// It names what the frames are *of*, so it changed with them: these are a
/// player walking, not a camera orbiting. Renaming rather than overwriting is
/// what makes the re-shoot a diff a reader can judge — three directories
/// removed and three added, instead of three binary blobs quietly modified.
const CAPTURE_PREFIX: &str = "player-walk";

/// What a HUD capture's id carries that a terrain capture's does not.
///
/// The two sets stand side by side under one golden root, so the id has to say
/// which of the two draw paths a frame came through — otherwise a HUD capture at
/// tick 0 and the terrain capture at tick 0 would name the same directory and one
/// would overwrite the other's reference.
const HUD_SEGMENT: &str = "-hud";

/// Why a revision cannot appear in a capture id.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CaptureIdShapeError {
    #[error(
        "a scene revision cannot be empty: the id would end in a bare `-` and two revisions \
         would collide"
    )]
    EmptyRevision,
    #[error(
        "the scene revision `{revision}` contains `{character}`, which is not one of a-z, 0-9, \
         `-` or `_`; a capture id becomes a directory name under the golden root"
    )]
    IllegalCharacter { revision: String, character: char },
}

/// The id of the capture declared at `tick` for scene revision `revision`.
///
/// # Errors
///
/// Returns [`CaptureIdShapeError`] when `revision` is empty or holds a character
/// that cannot appear in a directory the golden lifecycle writes.
pub fn capture_id(tick: u16, revision: &str) -> Result<String, CaptureIdShapeError> {
    id_of("", tick, revision)
}

/// The id of the HUD capture declared at `tick` for scene revision `revision`.
///
/// # Errors
///
/// Returns [`CaptureIdShapeError`] when `revision` is empty or holds a character
/// that cannot appear in a directory the golden lifecycle writes.
pub fn hud_capture_id(tick: u16, revision: &str) -> Result<String, CaptureIdShapeError> {
    id_of(HUD_SEGMENT, tick, revision)
}

/// The id of the capture declared at `tick`, with `segment` saying which draw
/// path it came through.
///
/// One statement of the shape for both sets. Written twice, the tick width or the
/// separator could drift on one side only, and a directory listing would stop
/// sorting the two sets together for a reason nothing states.
fn id_of(segment: &str, tick: u16, revision: &str) -> Result<String, CaptureIdShapeError> {
    check_revision(revision)?;
    // Three digits, zero-padded, so a directory listing sorts the captures in
    // tick order — which is the order a reader compares three frames of one
    // walk in. The declared intent script is 120 ticks long, and it is the
    // script's length rather than any period of the simulation that bounds
    // this, so the width never overflows.
    Ok(format!("{CAPTURE_PREFIX}{segment}-t{tick:03}-{revision}"))
}

/// The ids of every capture `revision` declares: the terrain set in tick order,
/// then the HUD set in tick order.
///
/// Exactly the directory names the golden root may hold at that revision — which
/// is what lets an inventory check report a set left behind by a previous
/// revision as well as one that is missing.
///
/// **Both sets, because only the pair covers the path the product draws
/// through.** The terrain captures go through `record_terrain` and the HUD
/// capture through the client's own frame call; a list holding one of the two
/// would leave the other's directory reported as one no test declares.
///
/// # Errors
///
/// Returns [`CaptureIdShapeError`] when `revision` cannot appear in an id.
pub fn declared_capture_ids(revision: &str) -> Result<Vec<String>, CaptureIdShapeError> {
    let terrain = DECLARED_CAPTURE_TICKS
        .iter()
        .map(|tick| capture_id(*tick, revision));
    let hud = HUD_CAPTURE_TICKS
        .iter()
        .map(|tick| hud_capture_id(*tick, revision));
    terrain.chain(hud).collect()
}

/// Whether `revision` can stand in a path segment the golden lifecycle writes.
fn check_revision(revision: &str) -> Result<(), CaptureIdShapeError> {
    if revision.is_empty() {
        return Err(CaptureIdShapeError::EmptyRevision);
    }
    match revision.chars().find(|character| !is_legal(*character)) {
        Some(character) => Err(CaptureIdShapeError::IllegalCharacter {
            revision: revision.to_owned(),
            character,
        }),
        None => Ok(()),
    }
}

/// Whether `character` may appear in a capture id.
fn is_legal(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit() || matches!(character, '-' | '_')
}
