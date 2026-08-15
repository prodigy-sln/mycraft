//! Which array-texture layer each of the shipped blocks occupies when a launch
//! prepares one.
//!
//! # A layer index is inside every committed golden frame
//!
//! Layers are assigned positionally over a sorted set of texture keys, and the
//! index a key lands on travels inside every packed vertex — so it travels inside
//! every image this project has committed. That makes the key set the one input
//! here that cannot be allowed to move for an incidental reason, and "which blocks
//! happen to be visible in the world this launch plays" is the most incidental
//! reason available: a save that broke the last stone out of existence would
//! renumber the array texture, and nothing would draw attention to it because no
//! golden is shot after a resume.
//!
//! # Two scenarios, two tests, and they may not be folded into one
//!
//! The first is a **guard**: the three indices it names are green today and have to
//! stay green, and the change this suite belongs to is only safe because they do.
//! The second is a **driver**: it is red before the key set is redefined, because
//! the shipped water block is not solid, the mesher emits no face for a voxel that
//! is not solid, and a key set built out of the meshed quads therefore has no entry
//! for it at all. Folded into one test, the guard's pass and the driver's failure
//! would share an assertion, and the surviving half would be reported as though the
//! whole thing had spoken.
//!
//! # One test states literals and the other derives its number, deliberately
//!
//! Dirt, grass and stone are compared against `0`, `1` and `2` written out, because
//! that is what "no committed golden may move" looks like written down: the numbers
//! are the claim, and deriving them from the code that assigns them would make the
//! test agree with whatever that code did today. Water's layer is the opposite
//! case — nothing is committed about it, so its expected value is derived from the
//! declared key set, whose own content the same assertion pins.

#[path = "support/handed.rs"]
mod handed;

use std::error::Error;

use mc_client::launch::prepare_launch;
use mc_core::block::{BlockId, BlockRegistry};
use mc_core::id::TextureKey;
use mc_render::texture::TextureLayers;
use mc_world::persistence::Acceptance;
use tempfile::TempDir;

use handed::{TestResult, shipped_content, where_no_save_is};

/// Neither launch below has a save to read, so what a player said about loading one
/// whose blocks have changed decides nothing here.
const ACCEPTING: Acceptance = Acceptance::OnlyUnchangedBlocks;

/// The blocks the shipped content declares, spelled as content spells them.
///
/// Said out loud in a test under `tests/`, which the hardcoded-name scan does not
/// read: what layer a *named* block occupies is the whole subject, and a fixture
/// that derived the names from the registry could not tell one block's layer from
/// another's.
const DIRT: &str = "base:dirt";
const GRASS: &str = "base:grass";
const STONE: &str = "base:stone";
const WATER: &str = "base:water";

/// Every texture key the shipped content declares, in the order layers are handed
/// out in — which is lexicographic order of the key, by the renderer's own
/// declaration.
const SHIPPED_TEXTURE_KEYS: [&str; 4] = [DIRT, GRASS, STONE, WATER];

/// The three layers every committed golden frame was shot with.
const DIRTS_LAYER: u16 = 0;
const GRASSES_LAYER: u16 = 1;
const STONES_LAYER: u16 = 2;

#[test]
fn a_launch_puts_the_terrain_blocks_on_the_layers_the_committed_frames_were_shot_with() -> TestResult
{
    let content = shipped_content()?;
    let nowhere = TempDir::new()?;

    let prepared = prepare_launch(&content, &where_no_save_is(&nowhere), ACCEPTING)?;

    assert_eq!(
        [
            layer_of(&prepared.layers, DIRT)?,
            layer_of(&prepared.layers, GRASS)?,
            layer_of(&prepared.layers, STONE)?,
        ],
        [Some(DIRTS_LAYER), Some(GRASSES_LAYER), Some(STONES_LAYER)],
        "these three indices are packed into the vertices of every golden frame this repository \
         has committed, so they are written out here rather than derived: if the definition of the \
         texture key set moves any of them, four golden sets depict a world drawn with the wrong \
         textures and the only symptom is an image diff nobody can explain from the change that \
         caused it. This is the assertion that says the redefinition was safe, and it is verified \
         first"
    );
    Ok(())
}

#[test]
fn a_launch_gives_the_one_block_no_quad_draws_a_layer_of_its_own() -> TestResult {
    let content = shipped_content()?;
    let nowhere = TempDir::new()?;

    let prepared = prepare_launch(&content, &where_no_save_is(&nowhere), ACCEPTING)?;

    assert_eq!(
        (
            declared_texture_keys(&prepared.registry)?,
            layer_of(&prepared.layers, WATER)?
        ),
        (declared_keys_the_fixture_expects(), Some(waters_layer()?)),
        "the shipped water block is not solid, so the mesher emits no face for it and no quad \
         anywhere in the generated world names it. A layer set built out of the meshed quads \
         therefore leaves it with no layer at all, and anything that ever drew water would fail \
         its whole section with an unresolved texture. Which layer it should occupy is derived \
         rather than remembered: layers are handed out in lexicographic order of the key, and the \
         first half of this assertion is what pins the four keys that order is over — a root \
         declaring some other set would fail here rather than quietly agree"
    );
    Ok(())
}

/// The layer `key` occupies in `layers`, or nothing where it occupies none.
///
/// # Errors
///
/// Returns an error if `key` is not a namespaced texture key.
fn layer_of(layers: &TextureLayers, key: &str) -> Result<Option<u16>, Box<dyn Error>> {
    Ok(layers.layer_of(&TextureKey::parse(key)?))
}

/// Every texture key the content root a launch read declares, in lexicographic
/// order.
///
/// Enumerated from the definitions one at a time rather than through whatever
/// accessor the registry offers for exactly this: that accessor is the subject of
/// the change these scenarios grade, and a fixture built out of it would agree with
/// it however wrong it was. It also reads `texture` and never `name` — the two
/// agree for all four shipped blocks, so a fixture reading the wrong one would look
/// perfectly healthy until the day a mod declares them differently.
///
/// # Errors
///
/// Returns an error if the registry cannot produce a definition it counted.
fn declared_texture_keys(registry: &BlockRegistry) -> Result<Vec<String>, Box<dyn Error>> {
    let mut keys = Vec::with_capacity(registry.registered_count());
    for raw in 0..u32::try_from(registry.registered_count())? {
        keys.push(
            registry
                .definition(BlockId::from_raw(raw))?
                .texture
                .as_str()
                .to_owned(),
        );
    }
    keys.sort();
    Ok(keys)
}

/// The keys the fixture says the shipped root declares, in the same shape the
/// enumeration above reports.
fn declared_keys_the_fixture_expects() -> Vec<String> {
    SHIPPED_TEXTURE_KEYS
        .iter()
        .map(|key| (*key).to_owned())
        .collect()
}

/// Which layer water occupies, derived from the declared key set.
///
/// Layers are assigned positionally over the keys in lexicographic order, so a
/// key's layer *is* its position among them, and water sorts last of the four.
///
/// # Errors
///
/// Returns an error if the fixture's own key list does not hold the key it is
/// about, or if a position that far along does not fit a layer index.
fn waters_layer() -> Result<u16, Box<dyn Error>> {
    let at = SHIPPED_TEXTURE_KEYS
        .iter()
        .position(|key| *key == WATER)
        .ok_or("the fixture's list of declared keys does not hold the block it is about")?;
    Ok(u16::try_from(at)?)
}
