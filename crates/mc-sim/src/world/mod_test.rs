//! Guard. What [`World::adopt`] refuses, and what it leaves behind when it
//! refuses.
//!
//! # Why this is a unit test and not a reload scenario
//!
//! `adopt` settles solidity before it writes either view, so a candidate naming
//! nothing the world holds refuses **without having changed anything** — that is
//! the property, and it is what makes "a failed reload changes nothing" true by
//! construction rather than by care.
//!
//! Nothing reaching `adopt` through a reload can see it. The admission stage
//! checks the names the world holds first and turns such a candidate away before
//! `adopt` is called at all, so `adopt`'s own refusal is **pre-empted in
//! production** — a mutation assigning `self.registry` before resolving solidity
//! passed 1 110 tests, and this file is the answer to that.
//!
//! **That is not the same as a decision nothing consults.** `adopt` is called on
//! every accepted reload; one branch of it is unreachable, and that branch is the
//! second line of defence over a player's world. An accessor no production code
//! calls is a test agreeing with itself; a guarded write path whose guard is
//! doubled is a write path with two guards, and the inner one is worth a witness
//! precisely because the outer one hides it.
//!
//! # The refusal is named exactly, and the second guard is why the first is not
//! vacuous
//!
//! "The registry is still the one it had" is satisfied just as well by an `adopt`
//! that never replaces the registry at all — delete the assignment and the first
//! guard below goes green for good. So the second one asks the same call to
//! succeed and requires the replacement to have happened, which is the only thing
//! that tells a refusal that changed nothing from a function that changes nothing.
//!
//! The two are separate test functions rather than one, for the reason the
//! structural-invariant guards elsewhere are: as one test, "the control failed
//! while the real assertion still passed" is not something a run can show you.

use std::error::Error;
use std::sync::Arc;

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin, RegistryError};
use mc_core::id::{BlockName, TextureKey};
use mc_world::world::{VoxelWorld, WorldPos};

use super::World;

/// The error type these guards propagate with `?`.
type GuardResult = Result<(), Box<dyn Error>>;

/// The block the world below holds, and the one a refused candidate stops
/// declaring.
const GROUND: &str = "fixture:ground";

/// A block both registries declare, so that neither of them is a source
/// declaring nothing — which registration refuses outright, for a reason that has
/// nothing to do with what these guards are about.
const SPARE: &str = "fixture:spare";

/// A block only the accepted candidate declares, so that "the registry was
/// replaced" is a question with an answer rather than a hope.
const EXTRA: &str = "fixture:extra";

/// What these definitions are attributed to. Nothing asserts it; a definition has
/// to say where it came from.
const GUARD_ORIGIN: &str = "a world-adoption guard's declared registry";

/// How many chunk columns the guard's world spans on each axis.
const COLUMNS: u32 = 1;

/// The one cell the guard's world holds a block in.
const A_BLOCK: WorldPos = WorldPos { x: 1, y: 1, z: 1 };

#[test]
fn a_candidate_missing_a_block_the_world_holds_is_refused_and_leaves_the_registry_it_had()
-> GuardResult {
    let mut world = a_world_holding_the_ground()?;
    let ground = BlockName::parse(GROUND)?;

    let refused = world.adopt(Arc::new(declaring(&[SPARE])?));

    assert_eq!(
        (refused, world.registry().resolve(&ground).is_ok()),
        (
            Err(RegistryError::UnknownName {
                name: ground.clone()
            }),
            true
        ),
        "solidity is settled before either view is written, so a candidate that cannot answer for \
         a block the world holds is refused having touched nothing. The refusal alone is not the \
         claim — a world left named against a registry its bitset was never resolved with is the \
         disagreement this type exists to make unspellable, and it is what a reload's own \
         admission check hides by turning such a candidate away one call earlier"
    );
    Ok(())
}

#[test]
fn a_candidate_answering_for_everything_the_world_holds_replaces_the_registry() -> GuardResult {
    let mut world = a_world_holding_the_ground()?;
    let extra = BlockName::parse(EXTRA)?;

    let took = world.adopt(Arc::new(declaring(&[GROUND, SPARE, EXTRA])?));

    assert_eq!(
        (took, world.registry().resolve(&extra).is_ok()),
        (Ok(()), true),
        "the control the guard above cannot do without: it asserts that a refusal left the registry \
         alone, and an `adopt` that never assigned one at all would satisfy that for good. This is \
         the same call being asked to do the thing — the world answers for a block only the \
         candidate declared, which it can only do if the replacement happened"
    );
    Ok(())
}

/// A one-column world holding [`GROUND`] in a single cell, named against a
/// registry that declares it.
fn a_world_holding_the_ground() -> Result<World, Box<dyn Error>> {
    let registry = Arc::new(declaring(&[GROUND, SPARE])?);
    let mut blocks = VoxelWorld::empty(COLUMNS);
    blocks.set_block(A_BLOCK, &BlockName::parse(GROUND)?, &registry)?;
    Ok(World::new(blocks, registry)?)
}

/// A registry declaring exactly `names`, each of them solid.
///
/// Solid throughout, because what these guards are about is whether a name can be
/// answered for at all — and a resolve refuses an unknown name whatever it would
/// have said about it.
fn declaring(names: &[&str]) -> Result<BlockRegistry, Box<dyn Error>> {
    let origin = DefinitionOrigin::new(GUARD_ORIGIN);
    let mut declared = Vec::new();
    for name in names {
        declared.push(Ok(BlockDefinition {
            name: BlockName::parse(name)?,
            texture: TextureKey::parse(name)?,
            is_solid: true,
            replaceable: false,
            breakable: true,
            breaks_into: None,
            origin: origin.clone(),
        }));
    }
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(origin, declared))?;
    Ok(registry)
}
