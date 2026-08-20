//! What reaches an array-texture layer once a content root's art has been
//! built: the image's own texels for a key the set covers, and nothing at all
//! for a key it does not.
//!
//! # Read through the launch, not past it
//!
//! Every reading here goes through `prepare_scene` — the one statement of the
//! sequence a window opens on and the one every golden is shot through — and
//! takes the texels off the `PreparedScene` it hands back. A test calling
//! `built_set` directly would be asking whether the *reader* works, which
//! `built_set_verdict.rs` already asks; what is new in this phase is that the
//! answer reaches the thing that fills a layer. A preparation that read the set
//! and dropped it, which is exactly what it did before this phase, passes the
//! first question and fails this one.
//!
//! # What "fills a layer" is asserted at
//!
//! `write_layer` is a device call and belongs to the subtree golden frames are
//! the only defence for. The arithmetic in front of it is not:
//! `mip::levels_for` decides, per key, whether the layer is filled from supplied
//! texels or from the generator, and hands back the levels the upload loop
//! writes. So the seam these readings stand at is `levels_for`'s level zero —
//! the last value that is still a value before it becomes a queue write.
//!
//! # The expected colours are read from the palette, not snapshotted
//!
//! A face bakes its material colour **unshaded** — measured across all seven
//! shipped images, every distinct texel colour is byte-identical to a material
//! declared in `content/base/materials/`. So what `base:stone`'s image is made
//! of is stated below as the three *material names* the model is built from, and
//! the colours are read out of the TOML a person wrote. Nothing here is a record
//! of what a decoder did on the day somebody looked.
//!
//! That is a stronger expectation than three committed triples and a weaker
//! commitment: a decoder that swapped two channels, applied a transfer function
//! it should not have, or shaded a face lands on colours no material declares,
//! while a deliberate palette edit flows through both sides at once — which is
//! correct, because the image is rebuilt from those same files.

mod support;

use std::error::Error;

use mc_core::content::{Face, TEXTURE_EDGE};
use mc_core::id::{BlockName, TextureKey};
use mc_render::texture::mip::levels_for;
use mc_render::texture::placeholder::placeholder_texels;

use support::art::{declared_material_colors, drawn_colors};
use support::swatch::require;
use support::{PreparedScene, TestResult, built_sets, prepare_scene_at};

/// The shipped keys these readings are about.
const STONE: &str = "base:stone";
const GRASS_TOP: &str = "base:grass_top";
const DIRT: &str = "base:dirt";
const WATER: &str = "base:water";

/// The block whose faces would draw water if any of them were emitted.
const WATER_BLOCK: &str = "base:water";

/// The materials `stone-block.mcvox` is made of.
///
/// **Material names, not colours.** Which three materials a model's voxels are
/// made of is a statement about the art and belongs here; what colour each of
/// them is belongs to `content/base/materials/`, and this reading goes and
/// reads it. Nothing about the expectation is a snapshot of a run.
const STONES_MATERIALS: [&str; 3] = ["stone", "stone_dark", "stone_light"];

/// A key the shipped content declares against no facing, and the file its art is
/// copied into.
///
/// `example:` rather than `base:`, because a key nobody declares is a key from
/// outside the shipped content by definition — a `base:` spelling would read as
/// a shipped key that had gone missing.
const AN_UNDECLARED_KEY: &str = "example:baked_but_undeclared";
const ITS_IMAGE: &str = "example__baked_but_undeclared.png";

#[test]
fn a_covered_keys_layer_is_filled_from_its_image_and_not_from_the_generator() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    let stone = TextureKey::parse(STONE)?;
    let generated = placeholder_texels(&stone, TEXTURE_EDGE);

    let prepared = prepare_scene_at(root.path())?;

    let supplied = offered(&prepared, &stone)?;
    require_unlike_the_generator(&supplied, &generated)?;
    let filled = levels_for(&stone, &prepared.texels, TEXTURE_EDGE)?;

    assert_eq!(
        (
            filled.first().map(Vec::as_slice) == Some(supplied.as_slice()),
            drawn_colors(&stone, &prepared.texels),
            supplied.len(),
        ),
        (
            true,
            declared_material_colors(&STONES_MATERIALS)?,
            (TEXTURE_EDGE * TEXTURE_EDGE) as usize,
        ),
        "the layer's own level zero is the image's decoded texels — not the generated texture, \
         and not a resized buffer. What those texels are made of is stated as the three materials \
         the model is built from and read out of `content/base/materials/`, so a decoder that \
         swapped two channels, applied a transfer function it should not have, or shaded a face \
         lands on colours no material declares — and none of the three would have moved a \
         committed triple"
    );
    Ok(())
}

/// The texels the launch offered for `key`.
///
/// # Errors
///
/// Returns an error where the set covers nothing for it, which for the shipped
/// keys here is a fixture that was never built rather than a client decision.
fn offered(prepared: &PreparedScene, key: &TextureKey) -> Result<Vec<[u8; 4]>, Box<dyn Error>> {
    Ok(prepared
        .texels
        .covering(key)
        .ok_or_else(|| {
            format!(
                "the built set covers `{key}` and the launch offered no texels for it",
                key = key.as_str()
            )
        })?
        .to_vec())
}

/// Fails unless the image's texels differ from the texture generated for the
/// same key.
///
/// **The scenario's own condition, checked rather than assumed.** Were the two
/// equal, the reading it guards would pass under a launch that ignored the
/// image entirely — which is exactly the launch this phase replaces.
fn require_unlike_the_generator(
    supplied: &[[u8; 4]],
    generated: &[[u8; 4]],
) -> Result<(), Box<dyn Error>> {
    require(
        supplied != generated,
        format!(
            "this reading is about an image whose texels differ from the texture generated for \
             the same key. The generated texture's first texel is {:?} and the image's is {:?}",
            generated.first(),
            supplied.first()
        ),
    )
}

#[test]
fn two_covered_keys_are_each_filled_from_their_own_image() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    let (top, dirt) = (TextureKey::parse(GRASS_TOP)?, TextureKey::parse(DIRT)?);

    let prepared = prepare_scene_at(root.path())?;

    let filled = |key: &TextureKey| -> Result<Vec<[u8; 4]>, Box<dyn Error>> {
        Ok(levels_for(key, &prepared.texels, TEXTURE_EDGE)?
            .first()
            .ok_or("a filled layer has no level zero")?
            .clone())
    };
    let (tops, dirts) = (filled(&top)?, filled(&dirt)?);
    let takes_its_own =
        |key: &TextureKey, layer: &[[u8; 4]]| prepared.texels.covering(key) == Some(layer);

    assert_eq!(
        (
            takes_its_own(&top, &tops),
            takes_its_own(&dirt, &dirts),
            tops == dirts,
        ),
        (true, true, false),
        "two keys the set covers are two images, and each layer takes its own. A launch that read \
         one image and used it everywhere satisfies every reading about a single key; the last \
         element is what separates that from filling each layer from its own"
    );
    Ok(())
}

#[test]
fn the_block_no_quad_draws_still_holds_a_layer_and_no_face_takes_its_key() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    let water = TextureKey::parse(WATER)?;
    let block = BlockName::parse(WATER_BLOCK)?;

    let prepared = prepare_scene_at(root.path())?;

    let drawn: Vec<&BlockName> = prepared
        .meshed
        .iter()
        .flat_map(|section| section.quads.iter())
        .map(|quad| &quad.block)
        .filter(|name| **name == block)
        .collect();
    assert_eq!(
        (
            prepared.resolution.layers().layer_of(&water).is_some(),
            prepared.resolution.key_of(&block, Face::Up).cloned(),
            drawn.len(),
        ),
        (true, Some(water), 0),
        "water is declared and is not solid, so it spends a layer of the array texture and the \
         mesher emits no face that could sample it. Both halves are the claim: a layer set built \
         out of the meshed quads would leave it with none, and a mesher that emitted its faces \
         would draw a block the content says is not there"
    );
    Ok(())
}

#[test]
fn an_image_named_for_a_key_no_block_declares_leaves_the_launch_alone() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::also_naming(root.path(), AN_UNDECLARED_KEY, ITS_IMAGE)?;
    let undeclared = TextureKey::parse(AN_UNDECLARED_KEY)?;

    let prepared = prepare_scene_at(root.path())?;

    assert_eq!(
        (
            prepared.resolution.layers().layer_of(&undeclared),
            prepared.texels.covering(&undeclared).is_some(),
        ),
        (None, true),
        "an index may name art for a key no block in this root declares — a set built for content \
         that has since dropped a block is the ordinary way there. The launch completes and that \
         image simply occupies no layer, so nothing samples it. A client that refused it would \
         make every set stale the moment a declaration was deleted"
    );
    Ok(())
}
