//! The content a participant receives once somebody else has read a content
//! root.
//!
//! # What this carries, and what it deliberately does not
//!
//! A block's **name**, the **key each of its six faces draws from** and its
//! **solidity**, plus the **layer assignment** the renderer draws against. Those
//! are what a client draws and predicts with.
//!
//! It carries none of `replaceable`, `breakable` or `breaks_into`. Those are the
//! rules by which a world is **mutated**, the server recomputes every one of
//! them, and a client holding them would be holding rules it may not apply. That
//! absence is asserted by discrimination rather than by inspection — two content
//! roots differing in nothing but those three resolve to values that compare
//! equal — because a type that simply has no such field cannot fail a test about
//! not having one.
//!
//! # Why the layer assignment travels rather than being derived
//!
//! **A layer index rides inside every packed vertex.** Derived as a key's
//! position in a sorted key set, inserting one block renumbers every index after
//! it and the whole world is textured wrong — silently, with no error anywhere,
//! and not localised to the block that caused it. That is not a networking
//! concern: it is a live defect on hot reload, in one process.
//!
//! So the assignment is **stated** by whoever read the content and **honoured**
//! by whoever draws. The falsifier is the one thing separating that from a
//! rename: hand a client an assignment that is deliberately *not* the positional
//! order and the rendered indices follow it.
//!
//! # Why this lives in `mc-core`
//!
//! It is a content primitive with no I/O, which is what this crate is for — and
//! `mc-render` has to be able to accept a stated assignment while never naming
//! the simulation. Putting the value in `mc-sim` would have made the renderer
//! reach for a crate the dependency rules forbid it.
//!
//! # What is deliberately absent
//!
//! No identity, digest or hash of the content set. With one process nothing can
//! disagree, so nothing could falsify such a field, and a test that cannot fail
//! reads as evidence and is not. The moment it becomes falsifiable is the moment
//! a second participant exists. **This is the opposite of the layer assignment**,
//! which is here precisely because its consumer exists today: the mesher and the
//! packer read it on every frame this project already draws.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::id::{BlockName, TextureKey};

/// How many array-texture layers one session may assign.
///
/// Eight bits of the packed vertex carry a layer index
/// (`mc_render::geometry::vertex`), so this is a property of the
/// content-to-renderer contract and not of either side. `mc-render` asserts
/// agreement with its own bound at compile time; **it is never restated
/// elsewhere.**
pub const LAYERS_A_SESSION_MAY_ASSIGN: usize = 256;

/// The edge of one block texture, in texels.
///
/// A property of the content-to-renderer contract rather than of either side, on
/// the same terms as [`LAYERS_A_SESSION_MAY_ASSIGN`]: the renderer allocates its
/// array texture to it and fills every layer to it, and an art build has to bake
/// to it. **It is never restated elsewhere** — an allocation and a fill that
/// disagreed about the size would be a copy that either overruns or leaves a
/// band unwritten.
pub const TEXTURE_EDGE: u32 = 16;

/// One of a block's six faces, in the vocabulary a declaration writes.
///
/// The words content spells, not the axes a mesher works in: `mc-world` holds
/// the single total mapping between the two, because this crate cannot see a
/// section's coordinate system and must not learn one. A declaration states
/// which face draws what, and nothing infers it from a block's name or shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Face {
    Up,
    Down,
    North,
    South,
    East,
    West,
}

impl Face {
    /// Every face, in the order a refusal lists them and a declaration is read
    /// in.
    ///
    /// The order is fixed because a refusal quotes it back and a save folds by
    /// it: an order that could vary would make one refusal unquotable on a page
    /// and the other a hash that moves for no reason.
    pub const ALL: [Self; 6] = [
        Self::Up,
        Self::Down,
        Self::North,
        Self::South,
        Self::East,
        Self::West,
    ];

    /// The word a declaration spells this face with.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::North => "north",
            Self::South => "south",
            Self::East => "east",
            Self::West => "west",
        }
    }

    /// The face `word` names, or nothing where it names none.
    ///
    /// **Exact, and never case-folded.** `Up` is not `up`: accepting it would
    /// register a block whose `up` facing nobody declared, drawing whatever the
    /// capitalised entry happened to be read as. Derived from
    /// [`as_str`](Self::as_str) rather than written out again, so the two
    /// spellings of one fact cannot drift.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|face| face.as_str() == word)
    }
}

/// Which texture key each of a block's six faces draws from.
///
/// Six fields rather than an array, because [`at`](Self::at) is **total** — a
/// face always has a key, there is no `None` to handle and no index to get
/// wrong. [`stating`](Self::stating) takes the keys positionally in
/// [`Face::ALL`] order rather than paired with their faces: a list of pairs
/// could name one face twice and leave another unnamed, which would put the
/// missing case back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceTextures {
    up: TextureKey,
    down: TextureKey,
    north: TextureKey,
    south: TextureKey,
    east: TextureKey,
    west: TextureKey,
}

impl FaceTextures {
    /// One key on all six faces.
    ///
    /// What a declaration stating its texture as a single string means, and what
    /// every block this repository ships states.
    #[must_use]
    pub fn uniform(key: TextureKey) -> Self {
        Self {
            up: key.clone(),
            down: key.clone(),
            north: key.clone(),
            south: key.clone(),
            east: key.clone(),
            west: key,
        }
    }

    /// A key per face, positionally in [`Face::ALL`] order.
    #[must_use]
    pub fn stating(keys: [TextureKey; 6]) -> Self {
        let [up, down, north, south, east, west] = keys;
        Self {
            up,
            down,
            north,
            south,
            east,
            west,
        }
    }

    /// The key `face` draws from.
    #[must_use]
    pub fn at(&self, face: Face) -> &TextureKey {
        match face {
            Face::Up => &self.up,
            Face::Down => &self.down,
            Face::North => &self.north,
            Face::South => &self.south,
            Face::East => &self.east,
            Face::West => &self.west,
        }
    }

    /// Every distinct key these faces draw from.
    ///
    /// A set, because two faces of one block may legitimately share a key and
    /// they then share its array-texture layer — a grass block naming `base:dirt`
    /// above and below costs five layers and not six.
    #[must_use]
    pub fn keys(&self) -> BTreeSet<TextureKey> {
        Face::ALL
            .into_iter()
            .map(|face| self.at(face).clone())
            .collect()
    }
}

/// A content set needs more array-texture layers than a session has left.
///
/// **The sentence lives here and nowhere else.** It is the one a mod author
/// reads, it is quoted on `docs/modding/hot-reload.md`, and whatever wraps it
/// carries no wording of its own. "Relaunching reclaims every layer retired
/// since the client started" is literally true and its arithmetic is
/// `spent - live.len()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "this content needs {needed} texture layers and a session has \
     {LAYERS_A_SESSION_MAY_ASSIGN}; {spent} are already assigned, and relaunching reclaims every \
     layer retired since the client started"
)]
pub struct LayerBudget {
    pub needed: usize,
    pub spent: usize,
}

/// Which array-texture layer each key the serving content names holds, and how
/// many layers this session has spent.
///
/// **Appended, never renumbered, within a session.** A layer index rides inside
/// every packed vertex, so renumbering would re-texture the whole world silently
/// on every reload. Keeping the assignment means every vertex already on the GPU
/// stays valid and only the sections whose blocks changed need meshing again.
///
/// Constructed only by [`none`](Self::none) and [`appending`](Self::appending),
/// so density is a property of the type rather than of a comment — there is no
/// door through which an assignment with a gap in it could arrive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayerAssignment {
    /// The layer each key the serving content names holds, **in key order and
    /// not in layer order**: a key that already held a layer keeps it while a
    /// newly introduced one takes the next unspent index, so after any reload
    /// the two orders differ. Nothing reading this may depend on either.
    live: Vec<(TextureKey, u16)>,
    /// How many layers have ever been handed out this session — the high-water
    /// mark, and **a primary field rather than a derived one**: `live.len()`
    /// would be wrong, because a retired layer is spent and is not live.
    spent: u16,
}

impl LayerAssignment {
    /// A session that has spent nothing.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// This assignment with each key of `keys` on the layer it already holds, or
    /// on the next unspent one.
    ///
    /// **All or nothing**: a candidate introducing two keys with one layer free
    /// appends neither.
    ///
    /// # Errors
    ///
    /// Returns [`LayerBudget`] naming the count needed and the count already
    /// spent when the result would not fit [`LAYERS_A_SESSION_MAY_ASSIGN`].
    pub fn appending(&self, keys: &BTreeSet<TextureKey>) -> Result<Self, LayerBudget> {
        // Counted before anything is handed out, which is what makes this all or
        // nothing: a candidate introducing two keys with one layer free appends
        // neither rather than the one that fits.
        let introduced = keys
            .iter()
            .filter(|key| self.layer_of(key).is_none())
            .count();
        let needed = usize::from(self.spent) + introduced;
        if needed > LAYERS_A_SESSION_MAY_ASSIGN {
            return Err(LayerBudget {
                needed,
                spent: usize::from(self.spent),
            });
        }
        let mut next = self.spent;
        let mut live = Vec::with_capacity(keys.len());
        for key in keys {
            live.push((key.clone(), self.layer_for(key, &mut next)));
        }
        Ok(Self { live, spent: next })
    }

    /// The layer `key` already holds, or the next unspent one — which `next`
    /// then moves past.
    fn layer_for(&self, key: &TextureKey, next: &mut u16) -> u16 {
        if let Some(held) = self.layer_of(key) {
            return held;
        }
        let taken = *next;
        *next += 1;
        taken
    }

    /// The layer `key` holds, or nothing where the serving content does not name
    /// it.
    #[must_use]
    pub fn layer_of(&self, key: &TextureKey) -> Option<u16> {
        self.live
            .iter()
            .find(|(held, _)| held == key)
            .map(|(_, layer)| *layer)
    }

    /// How many layers this session has handed out, retired ones included.
    #[must_use]
    pub const fn spent(&self) -> u16 {
        self.spent
    }

    /// Each live key and the layer it holds.
    pub fn entries(&self) -> impl Iterator<Item = (&TextureKey, u16)> {
        self.live.iter().map(|(key, layer)| (key, *layer))
    }
}

/// Which accepted content set a reader is looking at.
///
/// Counts accepted reloads within one process and answers a question a reader in
/// that process actually asks — deliberately not a content identity or hash, and
/// deliberately not called a revision, which already names the golden-capture
/// revision. Saturating `u32`, mirroring the tick counter, so the two counters a
/// reader sees share one convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentSerial(u32);

impl ContentSerial {
    /// The serial the content a launch read is published under.
    pub const FIRST: Self = Self(0);

    /// The next serial after this one.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// This serial as the number a reader compares.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// One block, as a participant that only draws needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBlock {
    /// What the block is called.
    pub name: BlockName,
    /// The key each of its faces is drawn from. **Never a file path**: what
    /// pixels a key resolves to is the renderer's concern.
    pub textures: FaceTextures,
    /// Whether it stops a player, which is also what the mesher culls faces by.
    pub is_solid: bool,
}

/// Everything a content root declares that a client needs, and nothing it does
/// not.
///
/// See the module note for what is absent and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedContent {
    /// In registration order, which is the order the reader established.
    blocks: Vec<ResolvedBlock>,
    /// Which layer each key holds, and how many the session has spent.
    layers: LayerAssignment,
}

impl ResolvedContent {
    /// Content stating `blocks` and assigning `layers`.
    ///
    /// **Both are stated rather than derived from each other**, which is the
    /// whole point: a constructor that assigned layers by sorting the blocks
    /// would make every consumer agree with a sort by construction, and the
    /// disagreement a test needs to be able to express would be unexpressible.
    /// **A [`LayerAssignment`] rather than arbitrary pairs**, so a sparse or
    /// unordered assignment cannot enter through the one public, infallible door
    /// and make [`LayerAssignment::spent`] silently lie.
    pub fn stating(
        blocks: impl IntoIterator<Item = ResolvedBlock>,
        layers: LayerAssignment,
    ) -> Self {
        Self {
            blocks: blocks.into_iter().collect(),
            layers,
        }
    }

    /// Every block, in registration order.
    pub fn blocks(&self) -> impl Iterator<Item = &ResolvedBlock> {
        self.blocks.iter()
    }

    /// Which layer each texture key occupies.
    pub fn layer_assignment(&self) -> impl Iterator<Item = (&TextureKey, u16)> {
        self.layers.entries()
    }

    /// The assignment itself, for whoever has to append to it.
    ///
    /// A reload's build stage reads the layers the serving content has already
    /// spent, and this is where they live.
    #[must_use]
    pub const fn layers(&self) -> &LayerAssignment {
        &self.layers
    }
}
