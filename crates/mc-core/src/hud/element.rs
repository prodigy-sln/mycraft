//! What a HUD element is, where it was declared, and the vocabulary a
//! declaration may draw on.
//!
//! Every vocabulary here is published twice over — once as the type the model
//! holds and once as the spellings a declaration writes — and the second is
//! derived from the first, never typed beside it. A refusal has to offer
//! whoever wrote an unknown spelling the ones that are accepted, and a listing
//! maintained separately from the values it lists is a listing that goes stale
//! the first time one is added.

use crate::id::HudElementName;

/// The key a declaration names itself by.
pub(super) const NAME_FIELD: &str = "name";
/// The key a declaration states which screen anchor it is measured from in.
pub(super) const ANCHOR_FIELD: &str = "anchor";
/// The key a declaration states its displacement from that anchor in.
pub(super) const OFFSET_FIELD: &str = "offset";
/// The key a declaration states its extents in.
pub(super) const SIZE_FIELD: &str = "size";
/// The key a declaration states how it is drawn in.
pub(super) const DRAW_FIELD: &str = "draw";
/// The key a filled declaration states its colour in.
pub(super) const COLOR_FIELD: &str = "color";
/// The key a textured declaration names the live state it reads in.
pub(super) const SOURCE_FIELD: &str = "source";
/// The key a declaration states its contrast outline in.
pub(super) const OUTLINE_FIELD: &str = "outline";

/// Every field a declaration may spell.
///
/// The accepted set is the check, rather than a list of rejected spellings: a
/// field nobody accepted is refused naming itself, and no field by which a
/// declaration could reach something the engine owns — the debug overlay above
/// all — can be added without this constant moving.
///
/// It is assembled from the same constants the checking reads, so a field the
/// model learns to understand is accepted by construction, and one it does not
/// cannot be quietly accepted by adding a spelling here.
pub const ACCEPTED_FIELDS: [&str; 8] = [
    NAME_FIELD,
    ANCHOR_FIELD,
    OFFSET_FIELD,
    SIZE_FIELD,
    DRAW_FIELD,
    COLOR_FIELD,
    SOURCE_FIELD,
    OUTLINE_FIELD,
];

/// The ways an element may be drawn, in the spelling a declaration uses.
pub const DRAW_KINDS: [&str; 2] = [DrawKind::Fill.as_str(), DrawKind::BlockTexture.as_str()];

/// The live engine state a declaration may bind a draw to, in the spelling a
/// declaration uses.
///
/// Content names a readable value; content cannot compute with one.
pub const READABLE_VALUES: [&str; 1] = [ReadableValue::HeldBlock.as_str()];

/// The nine screen anchors, in the spelling a declaration uses.
pub const ANCHOR_NAMES: [&str; 9] = [
    Anchor::TopLeft.as_str(),
    Anchor::Top.as_str(),
    Anchor::TopRight.as_str(),
    Anchor::Left.as_str(),
    Anchor::Center.as_str(),
    Anchor::Right.as_str(),
    Anchor::BottomLeft.as_str(),
    Anchor::Bottom.as_str(),
    Anchor::BottomRight.as_str(),
];

/// What a declaration means by saying nothing about its offset.
pub(super) const OFFSET_WHEN_ABSENT: [i32; 2] = [0, 0];

/// How many hex digits a colour is written in: two per channel, four channels,
/// and no shorthand.
pub(super) const COLOR_DIGITS: usize = 8;

/// Where on the screen an element is measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    /// Every anchor, in the order [`ANCHOR_NAMES`] lists them.
    const ALL: [Self; 9] = [
        Self::TopLeft,
        Self::Top,
        Self::TopRight,
        Self::Left,
        Self::Center,
        Self::Right,
        Self::BottomLeft,
        Self::Bottom,
        Self::BottomRight,
    ];

    /// The spelling a declaration writes this anchor as.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top-left",
            Self::Top => "top",
            Self::TopRight => "top-right",
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
            Self::BottomLeft => "bottom-left",
            Self::Bottom => "bottom",
            Self::BottomRight => "bottom-right",
        }
    }

    pub(super) fn parse(spelled: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|anchor| anchor.as_str() == spelled)
    }
}

/// A colour as a declaration states it: `#RRGGBBAA`, eight hex digits, no
/// shorthand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    /// The colour `spelled` states, or nothing where it is not eight hex digits
    /// behind a `#`.
    ///
    /// Shorthand is refused rather than expanded. A rule can be relaxed later
    /// without invalidating content already written, and cannot be tightened —
    /// the same direction the namespaced-id rule takes.
    pub(super) fn parse(spelled: &str) -> Option<Self> {
        let digits = spelled.strip_prefix('#')?;
        if digits.len() != COLOR_DIGITS || !digits.bytes().all(|digit| digit.is_ascii_hexdigit()) {
            return None;
        }
        Some(Self {
            r: channel(digits, 0)?,
            g: channel(digits, 2)?,
            b: channel(digits, 4)?,
            a: channel(digits, 6)?,
        })
    }
}

/// The channel written at `at`, two hex digits wide.
fn channel(digits: &str, at: usize) -> Option<u8> {
    u8::from_str_radix(digits.get(at..at + 2)?, 16).ok()
}

/// A named piece of live engine state a declaration may bind a draw to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadableValue {
    HeldBlock,
}

impl ReadableValue {
    /// Every published readable value, in the order [`READABLE_VALUES`] lists
    /// them.
    const ALL: [Self; 1] = [Self::HeldBlock];

    /// The spelling a declaration names this value by.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HeldBlock => "held-block",
        }
    }

    pub(super) fn parse(spelled: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|published| published.as_str() == spelled)
    }
}

/// Which of the two drawing capabilities a declaration asks for, before the
/// field that capability needs has been read.
///
/// Separate from [`Draw`] because the coupling rules are stated against the kind
/// alone: a declaration naming `fill` forbids `source` whether or not it states
/// an acceptable `color`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DrawKind {
    Fill,
    BlockTexture,
}

impl DrawKind {
    /// Every draw kind, in the order [`DRAW_KINDS`] lists them.
    const ALL: [Self; 2] = [Self::Fill, Self::BlockTexture];

    /// The spelling a declaration writes this kind as.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::BlockTexture => "block-texture",
        }
    }

    pub(super) fn parse(spelled: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == spelled)
    }
}

/// How an element's rectangle is filled.
///
/// The engine knows a filled rectangle and a textured rectangle. It does not
/// know what a crosshair is — that is composed in content from two fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Draw {
    Fill { color: Rgba8 },
    BlockTexture { source: ReadableValue },
}

/// Where a declaration was made, as an opaque human-readable label.
///
/// Opaque on purpose, exactly as a block definition's origin is: this crate
/// performs no I/O and must not learn what a file or a content root is, so a
/// script chunk name is as expressible here as a file path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudOrigin(String);

impl HudOrigin {
    /// Labels an origin.
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }

    /// The label as it was given.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One HUD element, and all of it comes from content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudElement {
    pub name: HudElementName,
    pub anchor: Anchor,
    /// UI units, `+x` right and `+y` down. Absent in a declaration means
    /// `[0, 0]`.
    pub offset: [i32; 2],
    /// UI units, both extents strictly positive.
    pub size: [u32; 2],
    pub draw: Draw,
    /// Absent in a declaration means the element carries no outline.
    pub outline: Option<Rgba8>,
    pub origin: HudOrigin,
}
