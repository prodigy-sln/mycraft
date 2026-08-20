//! A key nobody has drawn art for, and what a launch does about it.
//!
//! # This is a mod author's first block and it must never be a refusal
//!
//! Somebody writes a block declaration, states a texture key, and runs the
//! client. They have not baked anything and there is no entry for their key in
//! any index. What they get is the generated texture — the same stand-in every
//! key in this project drew until this spec — and a running game. A refusal here
//! would send them looking for a build step nobody told them about, and the
//! refusal that *does* name a build step is about the set being absent, which is
//! a different thing entirely and is asserted apart from this.
//!
//! # The mixed run is the one that catches an all-or-nothing implementation
//!
//! A launch that filled every layer from the generator the moment one key was
//! uncovered would satisfy every reading about an uncovered key on its own, and
//! a launch that refused the run would satisfy none of them. The second reading
//! below holds both halves in one preparation, which is the only shape that
//! separates the three.
//!
//! # The third reading is a vacuity control and it is green on arrival
//!
//! Before any texel reaches a layer, every face already draws a generated
//! texture — so a set that is present, current and covers nothing lets the
//! launch through for a reason that has nothing to do with this phase. That is
//! what it is for: it is what stops the fallback being deleted once the set
//! works, and a phase that reddens it has broken something rather than found
//! something.

mod support;

use std::error::Error;

use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;
use mc_render::texture::mip::levels_for;
use mc_render::texture::placeholder::placeholder_texels;

use support::swatch::require;
use support::{PreparedScene, TestResult, built_sets, prepare_scene_at};

/// A key no manifest bakes and no shipped block declares.
const UNDRAWN: &str = "example:undrawn";

/// A key the shipped manifest does bake, for the run that holds both.
const STONE: &str = "base:stone";

/// The file the block declaring [`UNDRAWN`] is written into, and what it says.
///
/// A whole declaration rather than an edit of a shipped one: what this is about
/// is a block somebody added, and a shipped block with its key changed would be
/// a root that had lost a texture rather than one that had gained a block.
const UNDRAWN_FILE: &str = "undrawn.luau";
const UNDRAWN_DECLARATION: &str = "return {\n\tname = \"example:undrawn\",\n\ttexture = \"example:undrawn\",\n\tsolid = true,\n}\n";

#[test]
fn a_declared_key_the_set_does_not_cover_is_filled_from_the_texture_generated_for_it() -> TestResult
{
    let root = built_sets::a_root_with_a_built_set()?
        .declaring_block(UNDRAWN_FILE, UNDRAWN_DECLARATION)?;
    let undrawn = TextureKey::parse(UNDRAWN)?;

    let prepared = prepare_scene_at(root.path())?;

    require_declared(&prepared, &undrawn)?;
    let generated = placeholder_texels(&undrawn, TEXTURE_EDGE);
    let filled = level_zero(&undrawn, &prepared)?;

    assert_eq!(
        (
            prepared.texels.covering(&undrawn).is_some(),
            filled == generated,
            first_disagreement(&filled, &generated),
        ),
        (false, true, None),
        "nothing was baked for this key, so the set offers nothing for it and its layer is filled \
         from the texture generated out of the key itself. The launch completed, which is the \
         other half and is what `prepare_scene` returning at all says: a mod author's first block \
         costs them a stand-in texture and never a refusal. The third element names the first \
         texel that disagrees rather than printing two hundred and fifty-six of them, which is \
         the mistake `terrain_goldens.rs` records having made once"
    );
    Ok(())
}

#[test]
fn one_covered_key_and_one_uncovered_key_are_filled_differently_in_the_same_run() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?
        .declaring_block(UNDRAWN_FILE, UNDRAWN_DECLARATION)?;
    let (undrawn, stone) = (TextureKey::parse(UNDRAWN)?, TextureKey::parse(STONE)?);

    let prepared = prepare_scene_at(root.path())?;

    require_declared(&prepared, &undrawn)?;
    require_declared(&prepared, &stone)?;
    let (stones, undrawns) = (
        level_zero(&stone, &prepared)?,
        level_zero(&undrawn, &prepared)?,
    );

    assert_eq!(
        (
            Some(stones.as_slice()) == prepared.texels.covering(&stone),
            stones == placeholder_texels(&stone, TEXTURE_EDGE),
            undrawns == placeholder_texels(&undrawn, TEXTURE_EDGE),
        ),
        (true, false, true),
        "one run, two keys, two answers. The middle element is what makes this more than the two \
         single-key readings put together: an implementation that fell back for the whole set the \
         moment one key was uncovered fills stone from the generator too, and every other \
         assertion in this file stays green while it does"
    );
    Ok(())
}

#[test]
fn a_current_set_covering_nothing_leaves_every_declared_key_on_its_generated_texture() -> TestResult
{
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::with_an_index_naming_no_keys(root.path())?;

    let prepared = prepare_scene_at(root.path())?;

    let mut generated = Vec::new();
    let mut supplied = Vec::new();
    for (key, _layer) in prepared.resolution.layers().entries() {
        let level = level_zero(key, &prepared)?;
        if level == placeholder_texels(key, TEXTURE_EDGE) {
            generated.push(key.as_str().to_owned());
        } else {
            supplied.push(key.as_str().to_owned());
        }
    }
    require(
        !generated.is_empty(),
        "this root's blocks declare no key at all, so 'every key drew a generated texture' would \
         be true of nothing"
            .to_owned(),
    )?;

    assert!(
        supplied.is_empty(),
        "a set that is present and current while covering no key is the state this phase's \
         fallback has to survive, and it is green before any texel is wired — deliberately. It is \
         what stops the generated texture being deleted once the set works, so a phase that \
         reddens it has broken something rather than found something. Generated: {generated:?}, \
         supplied: {supplied:?}"
    );
    Ok(())
}

/// What fills `key`'s layer: the level zero the upload loop writes.
///
/// # Errors
///
/// Returns the refusal preparing that layer's levels raised, or the absence of a
/// level zero, which a chain cannot be built without.
fn level_zero(key: &TextureKey, prepared: &PreparedScene) -> Result<Vec<[u8; 4]>, Box<dyn Error>> {
    Ok(levels_for(key, &prepared.texels, TEXTURE_EDGE)?
        .first()
        .cloned()
        .ok_or("a filled layer has no level zero")?)
}

/// Where two layers first disagree, as an index and the two texels there.
///
/// **A summary rather than two whole layers.** A failure printing 256 texels
/// twice buries the sentence a reader needs, which is the mistake
/// `terrain_goldens.rs` records having made once and the reason it never
/// debug-prints an outcome.
fn first_disagreement(
    filled: &[[u8; 4]],
    expected: &[[u8; 4]],
) -> Option<(usize, [u8; 4], [u8; 4])> {
    filled
        .iter()
        .zip(expected)
        .enumerate()
        .find(|(_, (one, other))| one != other)
        .map(|(at, (one, other))| (at, *one, *other))
}

/// Fails unless `key` occupies a layer of `prepared`'s assignment.
///
/// A key nothing declares is a key nothing draws, and a reading about how its
/// layer is filled would be about a layer that does not exist.
fn require_declared(prepared: &PreparedScene, key: &TextureKey) -> Result<(), Box<dyn Error>> {
    require(
        prepared.resolution.layers().layer_of(key).is_some(),
        format!(
            "`{key}` has to occupy a layer for a reading about what fills that layer to be about \
             anything. The root's blocks declare {declared:?}",
            key = key.as_str(),
            declared = prepared
                .resolution
                .layers()
                .entries()
                .map(|(key, _)| key.as_str().to_owned())
                .collect::<Vec<_>>()
        ),
    )
}
