//! Whether the keys the shipped content's blocks declare and the keys its built
//! set covers are the same keys.
//!
//! # Nothing anywhere asked this before, and that is why the sea was magenta
//!
//! Four instruments looked at the art and none of them was about this. The
//! colour oracle falls back to the same generator the product does, so it was
//! truthfully told water drew what an uncovered key draws. The goldens are
//! minted from the renderer they grade, so the stand-in *was* the reference
//! image. The layer test asserts water has a layer, not that it has an image.
//! And the gate builds the set and refuses a committed one — both about the set
//! matching its own manifest, never about the manifest matching the
//! declarations. `voxforge build`'s own advisory scan runs in the other
//! direction: it reports a manifest key no block uses.
//!
//! # The rule binds the base game and would be wrong as a gate stage
//!
//! A mod author's first block declares a key nothing has baked, gets a generated
//! texture and a running game — that is designed behaviour and
//! `an_unauthored_key_draws_a_generated_texture.rs` is what holds it. What is
//! stricter here is that the *base game*'s job is to prove the contract is
//! complete (`content/CLAUDE.md`), so a project-wide check over arbitrary roots
//! would be false by design. This is a reading about one root.
//!
//! # The verdict is total and both lists are read out of what was observed
//!
//! `assert!(uncovered.is_empty())` cannot tell an empty answer from a scan that
//! can no longer look — a vanished content root would go green forever. So
//! [`Coverage`] enumerates, and the two arms that mean "there was nothing to
//! compare" are answers a reader is shown rather than silence.
//!
//! Neither list is a list this file holds. The declared keys come out of the
//! registry a launch built, one definition at a time; the covered keys come out
//! of the index the build wrote. **Neither is filtered against the other before
//! the comparison**, which is the shape `standards/global/testing.md` §2 records
//! two mirrors of a nine-name list being held at six by: one filtered its
//! needles by presence in what it observed, the other skipped what it could not
//! rank, and neither reddened. A ninth key added by a later spec makes this red.

mod support;

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::Path;

use mc_core::art::{INDEX_FILE_NAME, TextureSetIndex};
use mc_core::block::{BlockId, BlockRegistry};
use mc_render::texture::supplied::SuppliedTexels;

use support::{TestResult, built_sets, content_root, prepare_scene_at};

/// Every texture key the shipped content declares, in the order this reading
/// reports them — lexicographic, which is what a `BTreeSet` of key strings walks
/// in.
///
/// **Written out rather than derived.** The point of the reading is that this
/// list and the built set's own list are the same list, and a fixture deriving
/// either from the other would agree with whatever the tree did today.
const SHIPPED_TEXTURE_KEYS: [&str; 8] = [
    "base:dirt",
    "base:grass_side_east",
    "base:grass_side_north",
    "base:grass_side_south",
    "base:grass_side_west",
    "base:grass_top",
    "base:stone",
    "base:water",
];

/// A block a fixture adds, declaring a key no manifest anywhere bakes.
///
/// A whole declaration rather than an edit of a shipped one, for the reason
/// `an_unauthored_key_draws_a_generated_texture.rs` gives: a shipped block with
/// its key changed is a root that lost a texture, not one that gained a block.
const UNDRAWN_FILE: &str = "undrawn.luau";
const UNDRAWN_KEY: &str = "example:undrawn";
const UNDRAWN_DECLARATION: &str = "return {\n\tname = \"example:undrawn\",\n\ttexture = \"example:undrawn\",\n\tsolid = true,\n}\n";

/// What a content root's declared keys and its built set's covered keys say
/// about each other.
///
/// Total. The first two arms are the ones an absence assertion cannot report:
/// they are what "there was nothing to compare" looks like when it is said out
/// loud instead of passing.
#[derive(Debug, PartialEq, Eq)]
enum Coverage {
    /// No block of the root declares a texture key at all, so "every declared
    /// key is covered" would be true of nothing.
    NothingIsDeclared,
    /// The built set covers no key at all — a set whose index names none, or a
    /// reading that has lost the ability to see one.
    NothingIsCovered,
    /// The two lists are the same list, named here in the order both were read
    /// in.
    DeclaredAndCoveredAreTheSameKeys(Vec<String>),
    /// They are not: keys the blocks declare that nothing bakes, and keys the
    /// set bakes that no block declares.
    TheyDiffer {
        uncovered: Vec<String>,
        unused: Vec<String>,
    },
}

#[test]
fn the_shipped_roots_declared_keys_and_its_built_sets_covered_keys_are_the_same_keys() -> TestResult
{
    let root = content_root()?;

    let coverage = coverage_of(&root)?;

    assert_eq!(
        coverage,
        Coverage::DeclaredAndCoveredAreTheSameKeys(
            SHIPPED_TEXTURE_KEYS
                .iter()
                .map(|key| (*key).to_owned())
                .collect()
        ),
        "the base game's job is to prove the contract is complete, so every key its blocks \
         declare has to be a key its manifest bakes. A key named under `uncovered` draws the \
         generated stand-in on screen — a magenta checkerboard — and every other instrument in \
         this suite reports that as correct, because an uncovered key drawing a stand-in is \
         exactly what the design says happens. A key named under `unused` is art nothing shows. \
         The whole list is compared rather than searched, so a ninth key a later spec declares \
         fails here rather than slipping past"
    );
    Ok(())
}

#[test]
fn a_key_the_manifest_bakes_no_entry_for_is_named_in_the_verdict_as_uncovered() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?
        .declaring_block(UNDRAWN_FILE, UNDRAWN_DECLARATION)?;

    let coverage = coverage_of(root.path())?;

    assert_eq!(
        coverage,
        Coverage::TheyDiffer {
            uncovered: vec![UNDRAWN_KEY.to_owned()],
            unused: Vec::new(),
        },
        "this root is the shipped one with a block added that declares a key no manifest bakes, \
         so the reading above is only a reading at all while this one names that key and nothing \
         else. A verdict of `DeclaredAndCoveredAreTheSameKeys` here is a scan that has stopped \
         being able to look; a second name under `uncovered` is the shipped root carrying an \
         uncovered key of its own"
    );
    Ok(())
}

/// What the root at `root` says about its own coverage.
///
/// The launch is what reads the set, so the texels are taken off a preparation
/// rather than out of a second reader — a root whose set is stale or absent is
/// refused here, before any list is built, and the refusal is that root's own.
///
/// # Errors
///
/// Returns the preparation's refusal, the index's, or the registry's.
fn coverage_of(root: &Path) -> Result<Coverage, Box<dyn Error>> {
    let prepared = prepare_scene_at(root)?;
    let declared = declared_texture_keys(&prepared.registry)?;
    let covered = covered_texture_keys(root, &prepared.texels)?;
    Ok(compared(&declared, &covered))
}

/// How two key lists stand against each other.
fn compared(declared: &BTreeSet<String>, covered: &BTreeSet<String>) -> Coverage {
    if declared.is_empty() {
        return Coverage::NothingIsDeclared;
    }
    if covered.is_empty() {
        return Coverage::NothingIsCovered;
    }
    if declared == covered {
        return Coverage::DeclaredAndCoveredAreTheSameKeys(declared.iter().cloned().collect());
    }
    Coverage::TheyDiffer {
        uncovered: declared.difference(covered).cloned().collect(),
        unused: covered.difference(declared).cloned().collect(),
    }
}

/// Every distinct texture key the blocks of the root a launch read declare.
///
/// Enumerated from the definitions one at a time, reading `texture` over all six
/// facings and never `name`: the grass block declares a key per facing, so the
/// two have not agreed since it did.
///
/// # Errors
///
/// Returns an error if the registry cannot produce a definition it counted.
fn declared_texture_keys(registry: &BlockRegistry) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let mut keys = BTreeSet::new();
    for raw in 0..u32::try_from(registry.registered_count())? {
        for key in registry.definition(BlockId::from_raw(raw))?.textures.keys() {
            keys.insert(key.as_str().to_owned());
        }
    }
    Ok(keys)
}

/// Every key the built set under `root` covers: named by the index the build
/// wrote, and answered for by the texels the launch decoded.
///
/// **Both halves, because either alone is a weaker claim.** The index is what
/// says which keys the set is *about*, including any the declarations never
/// name; `covering` is what says the art actually reached a value. A key the
/// index names and the decode did not produce is not covered, and this is where
/// that would show.
///
/// # Errors
///
/// Returns the index's own read or parse failure. A root whose set the launch
/// accepted has an index; one that has none was refused before this is reached.
fn covered_texture_keys(
    root: &Path,
    texels: &SuppliedTexels,
) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let at = root.join(built_sets::SET_DIRECTORY).join(INDEX_FILE_NAME);
    let recorded = TextureSetIndex::parse(&fs::read_to_string(&at)?)?;
    Ok(recorded
        .entries()
        .iter()
        .filter(|entry| texels.covering(&entry.key).is_some())
        .map(|entry| entry.key.as_str().to_owned())
        .collect())
}
