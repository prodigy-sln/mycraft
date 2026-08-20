//! What the base game's grass, dirt and stone draw, now that art exists for
//! them.
//!
//! # Three readings about one declaration, and none of them is about a name
//!
//! The grass block is the first shipped block to state a texture per facing, and
//! what makes it worth shipping is that the engine knows nothing about it: the
//! six keys are in a Luau file, the manifest says which model face bakes each
//! one, and the client joins the two by key. So each reading below crosses that
//! join — from the declaration, through the key, to the image — rather than
//! stopping at either end of it.
//!
//! # Why the manifest is read as text
//!
//! `key`, `model` and `face` are stated in `content/base/textures.toml`, and the
//! type that parses it lives in `tools/voxforge`, which nothing under `crates/`
//! may depend on. Reading the three words is what makes "baked from the grass
//! model's *top* face" a claim rather than a hope — a declaration pointing at
//! `base:grass_top` and a manifest baking that key from the *bottom* face is a
//! world where every other assertion here stays green.
//!
//! # The four sides being different is a measured property of this art
//!
//! Four different model faces do not have to bake to four different images:
//! `stone-block.mcvox` is uniform noise and its six faces are six distinct
//! images, which is the case showing the implication does not hold in the other
//! direction either. So the pairwise inequality is asserted rather than assumed,
//! over the decoded texels rather than over file names.

mod support;

use std::error::Error;
use std::fs;

use mc_core::content::Face;
use mc_core::id::{BlockName, TextureKey};

use support::swatch::require;
use support::{PreparedScene, TestResult, built_sets, prepare_scene_at, repository_root};

/// The shipped blocks these readings are about.
const GRASS: &str = "base:grass";
const DIRT_BLOCK: &str = "base:dirt";

/// The keys the grass block's six facings are declared against.
const GRASS_TOP: &str = "base:grass_top";
const DIRT: &str = "base:dirt";
const SIDES: [&str; 4] = [
    "base:grass_side_north",
    "base:grass_side_south",
    "base:grass_side_east",
    "base:grass_side_west",
];

/// The model the grass block's art is baked from, and the face `base:grass_top`
/// comes off it.
///
/// `top` is voxforge's own word for the +Y side of a model. The compass words
/// the world uses sit on top of it and are the manifest's business, not this
/// reading's.
const GRASS_MODEL: &str = "models/grass-block.mcvox";
const THE_TOP_FACE: &str = "top";
const THE_BOTTOM_FACE: &str = "bottom";

#[test]
fn the_grass_blocks_upward_face_draws_the_image_baked_from_the_models_top() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    let (grass, top) = (BlockName::parse(GRASS)?, TextureKey::parse(GRASS_TOP)?);
    let baked = manifest_entry_for(GRASS_TOP)?;

    let prepared = prepare_scene_at(root.path())?;

    assert_eq!(
        (
            prepared.resolution.key_of(&grass, Face::Up),
            baked.model.as_str(),
            baked.face.as_str(),
            prepared.texels.covering(&top).is_some(),
        ),
        (Some(&top), GRASS_MODEL, THE_TOP_FACE, true),
        "the block says which key its upward face draws and the manifest says which model face \
         bakes that key, and the two have to meet on the same key or a grass block grows dirt on \
         top. Every one of the four is load-bearing: without the middle pair this passes against a \
         manifest baking `base:grass_top` from the underside"
    );
    Ok(())
}

#[test]
fn the_grass_blocks_underside_and_every_dirt_face_draw_one_image() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    let (grass, dirt_block) = (BlockName::parse(GRASS)?, BlockName::parse(DIRT_BLOCK)?);
    let dirt = TextureKey::parse(DIRT)?;
    let baked = manifest_entry_for(DIRT)?;

    let prepared = prepare_scene_at(root.path())?;

    let every_dirt_face: Vec<Option<&TextureKey>> = Face::ALL
        .iter()
        .map(|face| prepared.resolution.key_of(&dirt_block, *face))
        .collect();
    assert_eq!(
        (
            prepared.resolution.key_of(&grass, Face::Down),
            every_dirt_face.as_slice(),
            baked.model.as_str(),
            baked.face.as_str(),
        ),
        (
            Some(&dirt),
            [Some(&dirt); 6].as_slice(),
            GRASS_MODEL,
            THE_BOTTOM_FACE,
        ),
        "one model paints two blocks: the grass block's underside *is* plain dirt, so it is baked \
         from that face and the dirt block draws the same image on all six of its own. That is why \
         no dirt model exists — a second one would be a copy of the same voxels, and the day the \
         two drifted a grass block would sit on ground of a different colour"
    );
    Ok(())
}

#[test]
fn the_grass_blocks_four_sides_draw_four_images_no_two_of_which_are_alike() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    let grass = BlockName::parse(GRASS)?;

    let prepared = prepare_scene_at(root.path())?;

    let drawn = the_four_side_images(&prepared, &grass)?;
    let alike: Vec<(&str, &str)> = pairs(&drawn)
        .into_iter()
        .filter(|((_, one), (_, other))| one == other)
        .map(|((mine, _), (theirs, _))| (*mine, *theirs))
        .collect();

    assert_eq!(
        (alike.as_slice(), drawn.len()),
        ([].as_slice(), SIDES.len()),
        "four faces of one model do not have to bake to four different images — stone's six do, \
         and stone is uniform noise, so neither direction of that implication holds. This is the \
         measurement: the four sides are four pictures, and a bake that took one face and wrote it \
         four times would be caught here rather than by somebody noticing a wall looks repetitive"
    );
    Ok(())
}

/// One side facing's key and the image it draws.
type SideImage = (&'static str, Vec<[u8; 4]>);

/// The image each of the grass block's four side facings draws, in the order
/// [`SIDES`] names them.
///
/// # Errors
///
/// Returns an error when a facing declares no key, declares one this reading is
/// not about, or names a key the built set covers nothing for — all three of
/// which are a root nobody described rather than a renderer decision.
fn the_four_side_images(
    prepared: &PreparedScene,
    grass: &BlockName,
) -> Result<Vec<SideImage>, Box<dyn Error>> {
    let mut drawn = Vec::new();
    for (facing, key) in [Face::North, Face::South, Face::East, Face::West]
        .into_iter()
        .zip(SIDES)
    {
        let declared = prepared
            .resolution
            .key_of(grass, facing)
            .ok_or_else(|| format!("the grass block declares no key for its {facing:?} facing"))?;
        require(
            declared.as_str() == key,
            format!(
                "this reading is about the four keys the grass block declares for its sides, and \
                 its {facing:?} facing draws `{shown}` rather than `{key}`",
                shown = declared.as_str()
            ),
        )?;
        let texels = prepared
            .texels
            .covering(declared)
            .ok_or_else(|| format!("the built set covers nothing for `{key}`"))?
            .to_vec();
        drawn.push((key, texels));
    }
    Ok(drawn)
}

/// What the shipped manifest states for one texture key.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BakedFrom {
    model: String,
    face: String,
}

/// The manifest entry naming `key`, read out of the committed TOML as text.
///
/// **Text, because the type that parses this file lives in `tools/voxforge` and
/// nothing under `crates/` may depend on it.** The entries are `[[texture]]`
/// tables, so the file is split on that word and the block naming `key` is the
/// one read.
///
/// # Errors
///
/// Returns an error when the manifest is unreadable, when no entry names `key`,
/// or when the entry naming it states no model or no face — each of which is a
/// manifest nobody described rather than a client that did anything.
fn manifest_entry_for(key: &str) -> Result<BakedFrom, Box<dyn Error>> {
    let at = repository_root()?
        .join("content")
        .join("base")
        .join(built_sets::MANIFEST_FILE_NAME);
    let written = fs::read_to_string(&at)?;
    let quoted = format!("\"{key}\"");
    let entry = written
        .split("[[texture]]")
        .find(|block| stated(block, "key").as_deref() == Some(key))
        .ok_or_else(|| {
            format!(
                "no entry of {} names {quoted}, so there is nothing this key is baked from and \
                 the reading below would be about a manifest nobody wrote",
                at.display()
            )
        })?;
    Ok(BakedFrom {
        model: stated(entry, "model")
            .ok_or_else(|| format!("the entry naming {quoted} states no model"))?,
        face: stated(entry, "face")
            .ok_or_else(|| format!("the entry naming {quoted} states no face"))?,
    })
}

/// The quoted value `field` is given in `entry`, or nothing where it states
/// none.
///
/// Comment lines are skipped: the shipped manifest explains itself at length and
/// its prose names every one of these words.
fn stated(entry: &str, field: &str) -> Option<String> {
    entry
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| {
            let (name, value) = line.split_once('=')?;
            (name.trim() == field).then(|| value.trim().trim_matches('"').to_owned())
        })
}

/// Every unordered pair of `drawn`.
fn pairs<T>(drawn: &[T]) -> Vec<(&T, &T)> {
    drawn
        .iter()
        .enumerate()
        .flat_map(|(index, one)| drawn.iter().skip(index + 1).map(move |other| (one, other)))
        .collect()
}
