//! Two translucent kinds of different colours over one opaque wall, which is the
//! fixture the chosen ordering model's stated artefact is shown with.
//!
//! # What the artefact is, and why it needs two colours
//!
//! `src-over` is not commutative and nothing in the blended pass sorts. For two
//! translucent surfaces along one ray at one degree over an opaque colour, the
//! two orderings agree only where the two surfaces are **the same colour** —
//! which is why the shipped sea, being one kind, shows nothing. Give the ray two
//! kinds of different colours and the weights on the near and far colours swap:
//! whichever face composites **last** takes the heavier share, and which one that
//! is has nothing to do with distance from the eye.
//!
//! # The camera does not move; the emission order does
//!
//! A camera position alone cannot show this, because the artefact is not a
//! property of where the eye stands — it is a property of the order the two
//! faces reach the index buffer. So the fixture draws one scene from one eye
//! twice, handing the packer the two panes in the two possible orders, and the
//! frames differ. **That is the artefact stated as something a reader can
//! reproduce**, rather than as a hazard a comment warns about.
//!
//! **Deterministic, and that is the reason for one section.** Between sections
//! the compaction order is whatever the atomic hands out and is not reproducible
//! between runs; inside one section quads land at fixed offsets, so emission
//! order decides and decides the same way every time. A fixture spanning two
//! sections would be demonstrating that nondeterminism instead, which is a
//! different claim and not one a test may assert.
//!
//! # The two panes are concentric, so the frame carries its own control
//!
//! Both span the same 10 x 10 of the section at different depths, so the nearer
//! one projects strictly larger and the frame holds a **ring** where only it
//! covers the wall. That ring is the same colour under both orders, which is
//! what says the difference between the two frames is confined to where two
//! translucent surfaces of different colours actually overlap — the limit
//! `docs/technical/rendering.md` states, rather than a frame-wide difference
//! that any change at all would produce.

use std::error::Error;
use std::path::PathBuf;

use super::art::composited;
use super::translucency::{Declared, FRAME, Pane, Shot, drawn};

/// The three blocks the fixture declares.
pub const WALL: &str = "example:wall";
pub const NEARER: &str = "example:nearer_pane";
pub const FARTHER: &str = "example:farther_pane";

/// What each of them draws.
///
/// The wall's and the nearer pane's are `support::translucency`'s own palette;
/// the farther pane takes the third colour that module declares, so every
/// separation this fixture needs was measured alongside the rest of them.
pub const WALL_COLOUR: [u8; 3] = [32, 200, 90];
pub const NEARER_COLOUR: [u8; 3] = [235, 120, 40];
pub const FARTHER_COLOUR: [u8; 3] = [120, 40, 160];

/// The degree both translucent panes declare.
pub const HALF: f64 = 0.5;

/// Where each surface stands on the depth axis. A larger plane is nearer the
/// eye, which stands at `z = 40`.
const WALL_PLANE: u32 = 0;
const NEARER_PLANE: u32 = 8;
const FARTHER_PLANE: u32 = 4;

/// The pixel both panes cover, which is the frame's own centre.
///
/// **Declared from the fixture's symmetry rather than found in a frame.** The
/// eye looks at `(8, 8)` and both panes are centred on it, so a point on the
/// view axis projects to the middle of the frame whatever depth it stands at.
pub const THE_OVERLAP: (u32, u32) = (FRAME.width.div_euclid(2), FRAME.height.div_euclid(2));

/// Which order the two translucent panes are handed to the packer in.
///
/// The mesher's emission order is what the packer preserves and what the index
/// buffer carries, so this is the only thing that differs between the two
/// frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emitted {
    /// The farther pane first, so the nearer one composites last — back to
    /// front, which is the order a sorted model would have chosen.
    FartherFirst,
    /// The nearer pane first, so the **farther** one composites last. Front to
    /// back, which is the artefact.
    NearerFirst,
}

impl Emitted {
    /// Both orders, so a reading may state what it examined rather than name
    /// them one at a time.
    pub const BOTH: [Self; 2] = [Self::FartherFirst, Self::NearerFirst];

    /// The colour that composites last under this order, and the one it is laid
    /// over.
    const fn last_over_first(self) -> ([u8; 3], [u8; 3]) {
        match self {
            Self::FartherFirst => (NEARER_COLOUR, FARTHER_COLOUR),
            Self::NearerFirst => (FARTHER_COLOUR, NEARER_COLOUR),
        }
    }

    /// What this order is called in a verdict.
    #[must_use]
    pub const fn described(self) -> &'static str {
        match self {
            Self::FartherFirst => "the farther pane emitted first, the nearer one composited last",
            Self::NearerFirst => "the nearer pane emitted first, the farther one composited last",
        }
    }
}

/// The colour the overlap draws under `order`, composed in linear light from the
/// declared degree and the three layers' own colours.
///
/// **Emission order and not depth.** The pane that composites last is laid over
/// what composited before it, whichever of the two stands nearer the eye. Under
/// [`Emitted::FartherFirst`] that happens to agree with a sorted model; under
/// [`Emitted::NearerFirst`] it does not, and the difference between the two is
/// the artefact.
#[must_use]
pub fn composed_when(order: Emitted) -> [u8; 3] {
    let (over, under) = order.last_over_first();
    composited(over, composited(under, WALL_COLOUR, HALF), HALF)
}

/// The colour the ring draws: the nearer pane alone over the wall, which is the
/// same under both orders.
#[must_use]
pub fn nearer_over_the_wall() -> [u8; 3] {
    composited(NEARER_COLOUR, WALL_COLOUR, HALF)
}

/// The frame `order` draws, or `None` when the opt-in permitted the absence of a
/// device.
///
/// # Errors
///
/// Returns the root's own refusal, the packing failure, or the capture failure.
pub fn shot(order: Emitted) -> Result<Option<Shot>, Box<dyn Error>> {
    drawn(&declared(), &panes(order))
}

/// The three declarations, the two translucent ones at [`HALF`].
fn declared() -> [Declared; 3] {
    [
        Declared::opaque(WALL, WALL_COLOUR),
        Declared::opaque(NEARER, NEARER_COLOUR).at(HALF as f32),
        Declared::opaque(FARTHER, FARTHER_COLOUR).at(HALF as f32),
    ]
}

/// The wall and the two panes, in the order `order` names.
///
/// The wall is always first and is opaque, so the packer's partition puts it in
/// the opaque half whatever it is handed; the two that follow are what the
/// blended half draws, in the order they are listed here.
fn panes(order: Emitted) -> [Pane; 3] {
    let wall = Pane {
        block: WALL,
        plane: WALL_PLANE,
        x: 0..16,
        y: 0..16,
    };
    let nearer = inset(NEARER, NEARER_PLANE);
    let farther = inset(FARTHER, FARTHER_PLANE);
    match order {
        Emitted::FartherFirst => [wall, farther, nearer],
        Emitted::NearerFirst => [wall, nearer, farther],
    }
}

/// One pane at `plane`, centred on the section's middle and inset far enough
/// that its projection lands strictly inside the wall's.
fn inset(block: &'static str, plane: u32) -> Pane {
    Pane {
        block,
        plane,
        x: 3..13,
        y: 3..13,
    }
}

/// Where the frame each order draws is committed for a reader of
/// `docs/technical/rendering.md` to look at.
///
/// **Two images and not one, because the artefact is the difference between
/// them.** A single frame of an unexpected colour is not evidence of anything to
/// a reader who cannot see what the other order gives; the pair is the claim.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located.
pub fn frame_on_the_page(order: Emitted) -> Result<PathBuf, Box<dyn Error>> {
    Ok(super::repository_root()?
        .join("docs")
        .join("technical")
        .join("images")
        .join(match order {
            Emitted::FartherFirst => "two-translucent-kinds-farther-emitted-first.png",
            Emitted::NearerFirst => "two-translucent-kinds-nearer-emitted-first.png",
        }))
}

/// That same path as the document has to spell it: relative to the page, which
/// sits in the same directory tree.
#[must_use]
pub const fn frame_named_on_the_page(order: Emitted) -> &'static str {
    match order {
        Emitted::FartherFirst => "images/two-translucent-kinds-farther-emitted-first.png",
        Emitted::NearerFirst => "images/two-translucent-kinds-nearer-emitted-first.png",
    }
}
