//! What a block definition is, where it was declared, and the dense id a
//! registry assigns it.

use crate::block::{MediumTint, Opacity};
use crate::content::FaceTextures;
use crate::id::BlockName;

/// Where a definition was declared, as an opaque human-readable label.
///
/// Opaque on purpose. This crate performs no I/O and must not learn what a file
/// or a content root is, so a script chunk name is exactly as expressible here as
/// a file path. It exists to be quoted back to whoever wrote the definition when
/// something about it is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionOrigin(String);

impl DefinitionOrigin {
    /// Labels an origin.
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }

    /// The label as it was given.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Everything the engine knows about a block, and all of it comes from content.
///
/// The origin travels with the definition rather than with the batch it arrived
/// in, because a duplicate name must be reported against both the place that
/// declared it first and the place that declared it again — which needs the
/// first origin still to be known when the second arrives.
///
/// **Not [`Eq`]**, because [`move_resistance`](Self::move_resistance) is an
/// `f32`. That is a deliberate cost of letting a declaration state a number:
/// `PartialEq` is what every comparison in the engine uses, and a definition was
/// never a map key or a set member.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockDefinition {
    pub name: BlockName,
    /// The key each of its six faces draws from.
    ///
    /// A declaration states one key for all six or a key per facing, and the two
    /// forms arrive here as one value — so nothing downstream branches on which
    /// of them was written.
    pub textures: FaceTextures,
    /// Whether this block stops a player who walks into it.
    ///
    /// Collision and nothing else. It says nothing about whether the block is
    /// seen ([`drawn`](Self::drawn)), whether it hides what is behind it
    /// ([`occludes`](Self::occludes)), whether a swing can find it
    /// ([`targetable`](Self::targetable)), or whether a new block may be built
    /// over it ([`replaceable`](Self::replaceable)). Each of those is a separate
    /// declaration on purpose: they coincide across the blocks the base game
    /// happens to ship, and deriving any of them from this one would put that
    /// accident in the engine where content could not override it.
    pub is_solid: bool,
    /// Whether a placement may overwrite this block.
    ///
    /// Content's word, read by the placement rule and by nothing else. Absent in
    /// a declaration means `false` — the conservative half, so that a block
    /// which says nothing cannot be built through.
    pub replaceable: bool,
    /// Whether this block can be broken at all.
    ///
    /// Absent in a declaration means `true`: breakable is the ordinary case and
    /// a block that says nothing is an ordinary block. `false` is what makes a
    /// block indestructible, and any block may declare it — including one that
    /// also names a residue, which is then simply never reached.
    pub breakable: bool,
    /// What the cell holds once this block is broken, or nothing where breaking
    /// it leaves the cell empty.
    ///
    /// `None` is the common case and says the cell becomes empty, because the
    /// absence of a block is not a residue worth naming. Indestructibility is
    /// [`breakable`](Self::breakable) and never this field's silence. It is a
    /// [`BlockName`] rather than a [`BlockId`] because ids belong to a registry
    /// and definitions arrive in batches: a block may legitimately name a
    /// residue that a later batch registers, so the name is resolved where a
    /// break reads it and not where it is declared.
    pub breaks_into: Option<BlockName>,
    /// Whether any face of this block is emitted.
    ///
    /// Appearance and nothing else, read by the mesher. Absent in a declaration
    /// means whatever that declaration says about [`is_solid`](Self::is_solid) —
    /// which is what keeps every declaration written before this field existed
    /// meaning what it meant, since one bit used to answer this question too.
    pub drawn: bool,
    /// Whether this block hides the face of a neighbour that would meet it.
    ///
    /// Separate from [`drawn`](Self::drawn) because a block may be seen without
    /// hiding what is behind it, which is the whole of what makes water look like
    /// water. Absent in a declaration means whatever that declaration says about
    /// [`is_solid`](Self::is_solid).
    pub occludes: bool,
    /// Whether a swing can find this block.
    ///
    /// What the crosshair may settle on, read where a trace resolves what a
    /// player is aiming at. Absent in a declaration means whatever that
    /// declaration says about [`is_solid`](Self::is_solid). Whether the block
    /// then yields to that swing is [`breakable`](Self::breakable): this field
    /// decides only whether the swing arrives.
    pub targetable: bool,
    /// Whether a player can hold itself up in this block's volume.
    ///
    /// What makes a volume something to swim in rather than something to fall
    /// through. Absent in a declaration means `false`, a **constant** and not
    /// whatever that declaration says about [`is_solid`](Self::is_solid): no
    /// single bit ever answered this question, so deriving it would invent a
    /// claim no author made — and would make every solid block in existence
    /// swimmable.
    pub swimmable: bool,
    /// How much this block's volume slows what moves through it.
    ///
    /// Finite and not less than zero; `0.0` is exactly "unaffected", and the
    /// speed through the volume is divided by `1 + move_resistance`. Independent
    /// of [`swimmable`](Self::swimmable) in both directions — a volume may resist
    /// without being one a player can swim in, and the other way about. Absent in
    /// a declaration means `0.0`, a constant for the same reason.
    pub move_resistance: f32,
    /// How fast this block's volume lifts a swimmer who asks to rise, in blocks
    /// per second before gravity and before the volume's own resistance.
    ///
    /// Finite and not less than zero. Absent in a declaration means the speed
    /// the player's own jump leaves the ground at, so a declaration that says
    /// nothing lifts exactly as it always did — a constant, not whatever that
    /// declaration says about [`is_solid`](Self::is_solid), for the reason
    /// [`move_resistance`](Self::move_resistance) is one.
    ///
    /// Independent of [`swimmable`](Self::swimmable) here, in both directions:
    /// this reports what was declared. Whether a volume that holds nobody up
    /// lifts anybody is decided where a definition becomes a medium, not here.
    pub swim_ascent: f32,
    /// How much of the light reaching this block it stops.
    ///
    /// The one question it answers is how much of what lies beyond this block is
    /// still seen through it, and it answers nothing else: whether any face is
    /// emitted at all is [`drawn`](Self::drawn), and whether a neighbour's
    /// meeting face is suppressed is [`occludes`](Self::occludes). A block may
    /// be seen through without any of those three answers moving.
    ///
    /// Absent in a declaration means [`Opacity::OPAQUE`], a **constant** and not
    /// whatever that declaration says about [`is_solid`](Self::is_solid): no
    /// single bit ever answered this question, so deriving it would invent a
    /// claim no author made — and would make every non-solid block in the game
    /// invisible.
    pub opacity: Opacity,
    pub origin: DefinitionOrigin,
    /// What this block does to the light of everything seen from **inside** it.
    ///
    /// The one question it answers is what the world looks like to an eye
    /// standing in this block's volume, and it answers nothing else: how much
    /// of what lies beyond this block is seen *through* it is
    /// [`opacity`](Self::opacity), and the two are separate claims one
    /// declaration may make together.
    ///
    /// Absent in a declaration means `None`, which is no tint at all — not a
    /// colourless one, and not one at some default distance. There is no
    /// default tint and no default distance anywhere in the engine, so a block
    /// either says what its medium does to a view or says nothing about the
    /// matter.
    pub tint: Option<MediumTint>,
}

/// A block's dense runtime id, valid only for the registry that assigned it.
///
/// Never an on-disk or on-wire identity: ids are reassigned freely whenever the
/// definition set changes. `u32` rather than `u16` because an id is never stored
/// per voxel, so the width is free and a 65 535-block ceiling in a public
/// contract would only be a future migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(u32);

impl BlockId {
    /// The id numbered `raw`.
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// The id's number.
    pub const fn get(self) -> u32 {
        self.0
    }
}
