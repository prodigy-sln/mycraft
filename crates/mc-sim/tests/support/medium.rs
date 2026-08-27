//! The declared media the physics is asserted against, and the two doors a tick
//! reaches one through.
//!
//! **Nothing here is a hand-written answer.** Every fixture is a *declaration* —
//! a registry that says what a block is, and a volume that says which block is
//! where — resolved through the same [`ResolvedVoxels`] the shipped simulation
//! resolves. A fixture that answered `medium_at` directly would be a second
//! statement of the rule under test, and would go on agreeing with itself while
//! the table a real registry builds said something else entirely.
//!
//! **The two doors are separate on purpose.** [`resolved`] hands back the view;
//! [`world_holding`] hands back a [`World`], which builds its own view and
//! forwards to it, and which is the object `simulation.rs` actually passes to
//! `advance_player`. A view a test resolved itself is not that object, and until
//! something asserts through the world nothing has said the forwarding happens
//! at all.
//!
//! **What these fixtures would fail to catch if they were built differently.**
//! A medium written over the *whole* footprint is uniform on x and z, so a fold
//! that read a box's z where it meant its x would land on a column declaring the
//! same thing and report the right answer for the wrong reason. So a declared
//! medium is written over [`MEDIUM_COLUMNS`] — a box that holds [`FEET`]'s own
//! columns and **not their transpose** — and the floor, which is about solidity
//! and not about a medium, spans everything.
//!
//! **Every resistance a *walk* is measured through is a power of two less one**,
//! so `1 + r` is a power of two and the divisor is exact in `f32`. That is a
//! property of the fixtures rather than of the rule: it is what lets a ratio be
//! asserted without a tolerance chosen by loosening one until it passed.
//!
//! **The blocks that declare an ascent state `0.5` instead, and it is the
//! specification's number rather than a convenient one.** `1 + 0.5` is `1.5`,
//! which is not a power of two — so the exactness above is not available to
//! them, and it turns out not to be needed: `GRAVITY × TICK_DURATION` rounds to
//! exactly `0.5` in `f32`, which leaves `(3.5 − 0.5) / 1.5` exactly `2.0` and
//! `(9.0 − 0.5) / 1.5` the correctly rounded `17/3`. Measured rather than
//! assumed, and recorded here because a reader of the paragraph above would
//! otherwise read these fixtures as breaking its rule for no reason.

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_sim::replay::{Extent, ResolvedVoxels};
use mc_sim::world::World;
use mc_world::world::{VoxelWorld, WorldPos};

use super::volume::{AN_UNSTATED_ASCENT, Cells, Declaration, registry_of_declarations};

/// A block that stops nobody and states a resistance of `0.0` — a *written*
/// zero, which is a different fact from the silence of a cell holding nothing.
pub const CLEAR: &str = "fixture:clear";

/// A block that stops nobody and resists as much as it carries: `1 + 1 = 2`, so
/// what moves through it goes half as far.
pub const THICK: &str = "fixture:thick";

/// A block that stops nobody and resists three times over: `1 + 3 = 4`, so what
/// moves through it goes a quarter as far.
pub const THICKER: &str = "fixture:thicker";

/// A block that stops nobody and states the one resistance here whose
/// `1 + resistance` is **not** a power of two.
///
/// Every other resistance in this module divides exactly, which makes a division
/// and a multiplication by the reciprocal agree — so none of them can tell the
/// two apart, and the engine's doc comment promises a division. **Measured, not
/// chosen**: at `2.5` the two forms differ by one unit in the last place, and at
/// the obvious first candidate `0.5` they do not differ at all.
pub const AWKWARD: &str = "fixture:awkward";

/// A block that stops nobody and states a resistance beyond any scale the engine
/// moves at, which is the boundary a division has to stay finite and forward
/// across.
pub const SETTING: &str = "fixture:setting";

/// A block that stops nobody, that a player can hold itself up in, and that
/// states **no** resistance — the half of the pair that says buoyancy costs
/// nothing by itself.
pub const BUOYANT: &str = "fixture:buoyant";

/// A block that stops nobody, that a player can hold itself up in, and that
/// declares how fast it lifts one — the workhorse of every scenario about a
/// declared ascent.
pub const LIFTING: &str = "fixture:lifting";

/// The same block with a smaller declared ascent and **every other answer
/// identical**, so a pair of the two differs in the ascent and in nothing else.
///
/// That is what lets one scenario say the greater of two ascents wins a fold and
/// another say a *sink* through them is the same either way — neither claim is
/// stateable against a pair that differs anywhere else.
pub const LIFTING_LESS: &str = "fixture:lifting-less";

/// A block that stops nobody, that a player can hold itself up in, and that
/// declares an ascent of **zero** — a written zero, which is a different fact
/// from the silence of a declaration that says nothing about the ascent at all.
pub const LIFTING_NOT_AT_ALL: &str = "fixture:lifting-not-at-all";

/// A block that stops nobody, that a player can hold itself up in, and that says
/// **nothing** about how fast it lifts one, so its ascent is whatever the loader
/// means by an absent field.
///
/// The pair to [`LIFTING_NOT_AT_ALL`]: one states the zero and one states
/// nothing, and an implementation conflating the two answers them alike.
pub const LIFTING_BY_DEFAULT: &str = "fixture:lifting-by-default";

/// A block that stops nobody, that a player can hold itself up in, that resists
/// nothing, and whose declared ascent is **exactly one tick of gravity** — the
/// boundary at which a swimmer neither rises nor sinks.
pub const HOLDING_DEPTH: &str = "fixture:holding-depth";

/// A block that stops nobody, that a player can hold itself up in, that resists
/// nothing, and that declares an ascent far past any speed one tick may spend —
/// the boundary the tick's own displacement bound has to hold.
pub const LIFTING_ABSURDLY: &str = "fixture:lifting-absurdly";

/// A block that stops nobody, that **nobody can hold itself up in**, and that
/// declares [`LIFTING_ABSURDLY`]'s enormous ascent anyway.
///
/// It differs from [`LIFTING_ABSURDLY`] in its buoyancy and **in nothing else**,
/// which is the whole of what makes it a control: a declared ascent that reached
/// a swimmer from a volume holding nobody up would show up here and nowhere
/// else, and the two answers a fold could give are `2` blocks per second apart
/// from `5 999`.
pub const HOLDS_NOBODY_UP: &str = "fixture:holds-nobody-up";

/// A block that stops a player and states no medium at all: an ordinary floor.
pub const PLAIN_STONE: &str = "fixture:plain-stone";

/// A block that stops a player and states [`THICKER`]'s resistance — the same
/// resistance under a solid block, so a fold that lowered the box before asking
/// picks it up and reports a walk a quarter as long as the one it owes.
pub const CLINGING_STONE: &str = "fixture:clinging-stone";

/// How much [`THICK`] slows what moves through it.
pub const THICK_RESISTANCE: f32 = 1.0;

/// How much [`THICKER`] and [`CLINGING_STONE`] slow what moves through them.
pub const THICKER_RESISTANCE: f32 = 3.0;

/// How much [`AWKWARD`] slows what moves through it. `1 + 2.5 = 3.5`, which is
/// not a power of two.
pub const AWKWARD_RESISTANCE: f32 = 2.5;

/// How much [`SETTING`] slows what moves through it.
///
/// Well below `f32::MAX` and far above any speed one tick expresses, so the
/// division neither overflows nor is a division by an infinity.
pub const SETTING_RESISTANCE: f32 = 1e30;

/// How much [`LIFTING`], [`LIFTING_LESS`], [`LIFTING_NOT_AT_ALL`] and
/// [`LIFTING_BY_DEFAULT`] slow what moves through them.
///
/// The one resistance here whose `1 + r` is not a power of two, and it is the
/// specification's own number rather than a chosen one — see this module's doc
/// for why the arithmetic stays exact regardless.
pub const LIFTING_RESISTANCE: f32 = 0.5;

/// How fast [`LIFTING`] lifts a swimmer, in blocks per second.
pub const LIFTING_ASCENT: f32 = 3.5;

/// How fast [`LIFTING_LESS`] lifts one. Less than [`LIFTING_ASCENT`], which is
/// the whole of what the pair is for.
pub const LESSER_ASCENT: f32 = 1.5;

/// How fast [`HOLDING_DEPTH`] lifts a swimmer: exactly what one tick of gravity
/// takes back, so the two cancel and the tick ends where it began.
pub const DEPTH_HOLDING_ASCENT: f32 = 0.5;

/// How fast [`LIFTING_ABSURDLY`] and [`HOLDS_NOBODY_UP`] declare they lift a
/// swimmer.
///
/// Far past any speed one tick may spend, and far past the world is tall, so an
/// implementation that honoured it unbounded lands nowhere near an
/// implementation that bounds it — which is what makes both of them cheap
/// falsifiers rather than close calls.
pub const ABSURD_ASCENT: f32 = 9000.0;

/// How far every declared volume here reaches on each axis.
pub const EXTENT: Extent = Extent {
    x: 16,
    y: 16,
    z: 16,
};

/// The first row above a floor: a floor written up to but not including this
/// presents its top face at `y = 8.0`, which is where feet come to rest.
pub const FLOOR_TOP: u32 = 8;

/// Where the player stands in every fixture built here.
///
/// Off-lattice on both horizontal axes and **different on each**, so the box
/// covers column `x = 10` and column `z = 3`. Nothing below is measured from a
/// coordinate small enough to flatter the arithmetic, and the two axes stay
/// distinguishable.
pub const FEET: Vec3 = Vec3::new(10.5, FLOOR_TOP as f32, 3.5);

/// The columns a declared medium is written over: `x` in `[8, 12)`, `z` in
/// `[0, 6)`.
///
/// [`FEET`]'s own columns lie inside it and **their transpose does not** —
/// `x = 3` is outside — so a fold that read the box's z where it meant its x
/// reads a column no run ever wrote to, finds nothing there, and reports the
/// unresisted answer.
const MEDIUM_COLUMNS: (u32, u32, u32, u32) = (8, 12, 0, 6);

/// How many chunk columns square the worlds built by [`world_holding`] are.
const WORLD_COLUMNS: u32 = 1;

/// The thirteen media a player's box can be *inside*, which is every fixture
/// here that something is asserted to move through.
///
/// Each medium is stated in full beside the name that carries it, including the
/// zeroes and the ascent most of them leave unstated: a fixture that left any of
/// them out would be asserting about whatever the builder happened to default to
/// rather than about what it declared. That is why these entries are tall, and
/// why they are not collapsed behind a helper fixing two of the three answers —
/// such a helper is the thing this module's own builder refuses.
///
/// **The seven that state an ascent are declared here rather than in a registry
/// of their own**, so that a scenario about an ascent resolves through the same
/// [`ResolvedVoxels`] door as every other medium scenario in this suite. A
/// second registry would be a second statement of how a declaration becomes a
/// medium, and the one thing a scenario about an ascent has to reach is the
/// production door from a definition to a medium view.
///
/// **Three groups rather than one list**, because a fixture table stating every
/// answer in full outgrows what one function may say. The order is the order the
/// three name them, and the six that were here before this file learned about an
/// ascent keep the positions they had.
fn media_a_player_moves_through() -> Vec<(&'static str, Declaration)> {
    let mut declared = media_that_only_resist();
    declared.extend(media_that_hold_a_swimmer_up());
    declared.extend(media_at_the_boundaries_of_a_declared_lift());
    declared
}

/// The five media that slow what moves through them and hold nobody up, each
/// leaving its ascent unstated because no scenario over one asks about a rise.
fn media_that_only_resist() -> Vec<(&'static str, Declaration)> {
    let hollow = Declaration::like_solidity(false);
    vec![
        (
            CLEAR,
            hollow.stating_a_medium(false, 0.0, AN_UNSTATED_ASCENT),
        ),
        (
            THICK,
            hollow.stating_a_medium(false, THICK_RESISTANCE, AN_UNSTATED_ASCENT),
        ),
        (
            THICKER,
            hollow.stating_a_medium(false, THICKER_RESISTANCE, AN_UNSTATED_ASCENT),
        ),
        (
            SETTING,
            hollow.stating_a_medium(false, SETTING_RESISTANCE, AN_UNSTATED_ASCENT),
        ),
        (
            AWKWARD,
            hollow.stating_a_medium(false, AWKWARD_RESISTANCE, AN_UNSTATED_ASCENT),
        ),
    ]
}

/// The four media a player can hold itself up in that share one resistance, so
/// that a scenario comparing two of them is comparing their declared ascents and
/// nothing else.
///
/// [`BUOYANT`] is the exception and states no resistance at all, which is what
/// makes it the witness that buoyancy costs nothing by itself.
fn media_that_hold_a_swimmer_up() -> Vec<(&'static str, Declaration)> {
    let hollow = Declaration::like_solidity(false);
    vec![
        (
            BUOYANT,
            hollow.stating_a_medium(true, 0.0, AN_UNSTATED_ASCENT),
        ),
        (
            LIFTING,
            hollow.stating_a_medium(true, LIFTING_RESISTANCE, LIFTING_ASCENT),
        ),
        (
            LIFTING_LESS,
            hollow.stating_a_medium(true, LIFTING_RESISTANCE, LESSER_ASCENT),
        ),
        (
            LIFTING_NOT_AT_ALL,
            hollow.stating_a_medium(true, LIFTING_RESISTANCE, 0.0),
        ),
    ]
}

/// The four media that sit at an edge of what a declared ascent may be: the
/// field left unstated, an ascent gravity exactly cancels, an ascent past any
/// speed one tick may spend, and that last one declared by a block a player
/// cannot hold itself up in at all.
///
/// Grouped because each is a *boundary* rather than a rate, and because the last
/// two differ only in their buoyancy — which is what lets one scenario ask what
/// buoyancy is worth with everything else held still.
fn media_at_the_boundaries_of_a_declared_lift() -> Vec<(&'static str, Declaration)> {
    let hollow = Declaration::like_solidity(false);
    vec![
        (
            LIFTING_BY_DEFAULT,
            hollow.stating_a_medium(true, LIFTING_RESISTANCE, AN_UNSTATED_ASCENT),
        ),
        (
            HOLDING_DEPTH,
            hollow.stating_a_medium(true, 0.0, DEPTH_HOLDING_ASCENT),
        ),
        (
            LIFTING_ABSURDLY,
            hollow.stating_a_medium(true, 0.0, ABSURD_ASCENT),
        ),
        (
            HOLDS_NOBODY_UP,
            hollow.stating_a_medium(false, 0.0, ABSURD_ASCENT),
        ),
    ]
}

/// The two media declared on **solid** blocks, which no player's box is ever
/// inside.
///
/// Separate from the thirteen above because they are asserted about from the
/// outside:
/// [`CLINGING_STONE`] is the witness that `medium_around` structurally cannot
/// reach the block underfoot, and a resistance declared on it must therefore
/// reach nobody. A fixture that could not tell these two groups apart could not
/// state that.
fn media_a_player_cannot_enter() -> Vec<(&'static str, Declaration)> {
    let stone = Declaration::like_solidity(true);
    vec![
        (
            PLAIN_STONE,
            stone.stating_a_medium(false, 0.0, AN_UNSTATED_ASCENT),
        ),
        (
            CLINGING_STONE,
            stone.stating_a_medium(false, THICKER_RESISTANCE, AN_UNSTATED_ASCENT),
        ),
    ]
}

/// The registry every medium fixture resolves through: all fifteen, in the
/// order the two groups above name them.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id, or if the registry refuses
/// the batch.
pub fn media_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let mut declared = media_a_player_moves_through();
    declared.extend(media_a_player_cannot_enter());
    registry_of_declarations(&declared)
}

/// The five blocks a registry declares when a scenario needs a medium table
/// wider than one bit.
///
/// Five distinct `(swimmable, move_resistance)` answers, so that with the "no
/// medium here" every empty cell carries there are six — and five is the first
/// count that cannot fit two bits. They differ in one field at a time and in
/// both, so no two of them collapse under a table keyed on either field alone.
///
/// Named apart from the physics fixtures above and never mixed with them: those
/// exist to be walked through, these exist only to make a table wide.
pub const BUOYANT_ONLY: &str = "fixture:buoyant-only";
pub const SLOWED_ONCE: &str = "fixture:slowed-once";
pub const SLOWED_TWICE: &str = "fixture:slowed-twice";
pub const BUOYANT_AND_SLOWED: &str = "fixture:buoyant-and-slowed";
pub const SLOWED_THRICE: &str = "fixture:slowed-thrice";

/// What [`SLOWED_ONCE`] and [`SLOWED_THRICE`] declare, for a scenario that has
/// to say which answer it expects to read back.
pub const ONE_RESISTANCE: f32 = 1.0;
pub const THRICE_RESISTANCE: f32 = 3.0;

/// A registry declaring the five above and nothing else.
///
/// One statement of "a registry wide enough to need more than one bit", shared
/// by the scenario that measures the width and the one that writes into it: two
/// copies would be two places for the count to drift, and a copy that quietly
/// fell to four media would leave the other's claim silently weaker.
///
/// # Errors
///
/// Returns an error if a name is not a namespaced id, or if the registry refuses
/// the batch.
pub fn registry_of_many_media() -> Result<BlockRegistry, Box<dyn Error>> {
    let hollow = Declaration::like_solidity(false);
    registry_of_declarations(&[
        (
            BUOYANT_ONLY,
            hollow.stating_a_medium(true, 0.0, AN_UNSTATED_ASCENT),
        ),
        (
            SLOWED_ONCE,
            hollow.stating_a_medium(false, ONE_RESISTANCE, AN_UNSTATED_ASCENT),
        ),
        (
            SLOWED_TWICE,
            hollow.stating_a_medium(false, 2.0, AN_UNSTATED_ASCENT),
        ),
        (
            BUOYANT_AND_SLOWED,
            hollow.stating_a_medium(true, 1.0, AN_UNSTATED_ASCENT),
        ),
        (
            SLOWED_THRICE,
            hollow.stating_a_medium(false, THRICE_RESISTANCE, AN_UNSTATED_ASCENT),
        ),
    ])
}

/// A volume of [`EXTENT`] holding nothing at all.
#[must_use]
pub const fn hollow() -> Cells {
    Cells::empty(EXTENT)
}

/// How far a volume reaches when a scenario is about the world's own height: the
/// 256 rows a chunk column holds, which is what "over a world 256 blocks tall"
/// means.
pub const TALL_EXTENT: Extent = Extent {
    x: 16,
    y: 256,
    z: 16,
};

/// A volume of [`TALL_EXTENT`] holding nothing at all.
#[must_use]
pub const fn tall() -> Cells {
    Cells::empty(TALL_EXTENT)
}

/// A volume of [`EXTENT`] whose every column is `floor` up to but not including
/// [`FLOOR_TOP`], and holds nothing above it.
///
/// The floor spans the whole footprint because it is about solidity: a player
/// that walked off the declared medium's columns would otherwise fall out of
/// every fixture at once, and a scenario about a divisor would start reporting
/// a scenario about a ledge.
///
/// # Errors
///
/// Returns an error if `floor` is not a namespaced block name.
pub fn floored(floor: &str) -> Result<Cells, Box<dyn Error>> {
    hollow().holding(
        WorldPos { x: 0, y: 0, z: 0 },
        WorldPos {
            x: EXTENT.x,
            y: FLOOR_TOP,
            z: EXTENT.z,
        },
        floor,
    )
}

/// The same volume with `block` filling the rows `[low, high)` of
/// [`MEDIUM_COLUMNS`].
///
/// # Errors
///
/// Returns an error if `block` is not a namespaced block name.
pub fn flooded(volume: Cells, low: u32, high: u32, block: &str) -> Result<Cells, Box<dyn Error>> {
    let (west, east, south, north) = MEDIUM_COLUMNS;
    volume.holding(
        WorldPos {
            x: west,
            y: low,
            z: south,
        },
        WorldPos {
            x: east,
            y: high,
            z: north,
        },
        block,
    )
}

/// The view a tick reads `volume` through: resolved once, through
/// [`media_registry`].
///
/// # Errors
///
/// Returns an error if the registry refuses, or if the volume holds a name it
/// does not know.
pub fn resolved(volume: &Cells) -> Result<ResolvedVoxels, Box<dyn Error>> {
    Ok(ResolvedVoxels::resolve(volume, &media_registry()?)?)
}

/// A real [`World`] over [`media_registry`], holding `runs` and nothing else.
///
/// Each run is the half-open box from its first position up to but not including
/// its second, written in the order given.
///
/// # Errors
///
/// Returns an error if a name is not known, if a run reaches outside the world,
/// or if the registry refuses.
pub fn world_holding(runs: &[(WorldPos, WorldPos, &str)]) -> Result<World, Box<dyn Error>> {
    let registry = Arc::new(media_registry()?);
    let mut blocks = VoxelWorld::empty(WORLD_COLUMNS);
    for &(low, high, block) in runs {
        let name = BlockName::parse(block)?;
        for at in every_cell(low, high) {
            blocks.set_block(at, &name, &registry)?;
        }
    }
    Ok(World::new(blocks, registry)?)
}

/// Every cell of the half-open box from `low` up to but not including `high`.
fn every_cell(low: WorldPos, high: WorldPos) -> impl Iterator<Item = WorldPos> {
    (low.y..high.y).flat_map(move |y| {
        (low.z..high.z).flat_map(move |z| (low.x..high.x).map(move |x| WorldPos { x, y, z }))
    })
}
