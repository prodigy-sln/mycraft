//! A layer assignment the client is handed is honoured, not re-derived — and a
//! block the assignment names no layer for is refused by name rather than drawn
//! from layer zero.
//!
//! # Why this is not about networking
//!
//! A layer index rides inside every packed vertex. Deriving it as a key's
//! position in the sorted key set means inserting one block renumbers every
//! index after it and the whole world is textured wrong, silently, with no error
//! anywhere. That is a live defect on hot reload today, in one process, and it
//! is what these readings close.
//!
//! # Every assertion here goes through the packer
//!
//! Asking the view what layer it holds for a key would leave the consumer free
//! to re-derive one of its own, which is the exact failure the requirement is
//! about. So every reading below packs real quads through
//! [`build_section_geometry`] — the same function the client's own `scene_of`
//! calls — and reads the layer back out of the corners it produced.
//!
//! # Two of these read the resolution, and they used to read its absence
//!
//! A block's texture key is no longer its own name. The two readings at the end
//! of this file were written when it was — they pinned the substitution as a
//! *failure*, so that closing it would turn them red and the red would be the
//! announcement. It has been closed, and they are inverted here rather than
//! deleted: what they reach is the property through
//! [`ContentView`], the client's own construction of a resolution out of resolved
//! content, which is the shipped route and is not the route either of the
//! mapped readings for this takes. The two blocks in them are named for one
//! mineral and declare another, exactly as before.

#[path = "support/staged_layers.rs"]
mod staged_layers;

use std::error::Error;

use mc_client::content::ContentView;
use mc_core::block::Opacity;
use mc_core::content::{FaceTextures, ResolvedBlock, ResolvedContent};
use mc_core::id::{BlockName, TextureKey};
use mc_render::geometry::{GeometryError, SectionGeometry, SectionOrigin, build_section_geometry};
use mc_render::hud::{HeldSwatch, INDICATOR_FACE, held_swatch};
use mc_render::texture::TextureResolution;
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};

use staged_layers::assigned;

type TestResult = Result<(), Box<dyn Error>>;

/// How many corners one quad is packed into.
///
/// Written here so the expectation below states the whole run of corners rather
/// than sampling one of them: a packer that wrote the right layer into the first
/// corner of each quad and the wrong one into the other three would draw three
/// quarters of the world from the wrong texture.
const CORNERS_PER_QUAD: usize = 4;

/// Where the packed section sits. Nothing here depends on it.
const SECTION: [i32; 3] = [0, 0, 0];

/// Three blocks and the layer the assignment names for each.
///
/// The keys sort `example:amber`, `example:cobalt`, `example:zinc`, so the
/// positional assignment over them is `0`, `1`, `2`. These three deliberately
/// state something else: a reading that compared two copies of the same sort
/// could not fail whatever the client did.
const DISAGREEING: [(&str, u16); 3] = [
    ("example:amber", 2),
    ("example:cobalt", 0),
    ("example:zinc", 1),
];

/// An assignment naming a layer for two of the three blocks the content states.
const NAMING_TWO_OF_THREE: [(&str, u16); 2] = [("example:amber", 1), ("example:zinc", 0)];

/// The block [`NAMING_TWO_OF_THREE`] leaves without a layer.
const UNASSIGNED_BLOCK: &str = "example:cobalt";

/// A block whose declared texture key is not its own name, and the key it
/// declares.
const SUBSTITUTED_BLOCK: &str = "example:amber";
const ITS_DECLARED_KEY: &str = "example:quartz";

/// A block beside it whose key *is* its own name, so that a reading about the
/// first one is not satisfied by an answer given to everything.
const PLAINLY_KEYED_BLOCK: &str = "example:jade";

/// The layers those two keys hold, and they are deliberately not their sorted
/// positions: `example:jade` sorts ahead of `example:quartz` and holds the higher
/// layer, which is a session that met the substituted block's key first.
const SUBSTITUTED_ASSIGNMENT: [(&str, u16); 2] = [(ITS_DECLARED_KEY, 0), (PLAINLY_KEYED_BLOCK, 1)];

/// What packing a run of quads came to.
///
/// A total verdict rather than an error propagated out of the test: a packer
/// that accepted what it should refuse then fails on this comparison instead of
/// ending the test before its assertion ran.
#[derive(Debug, PartialEq, Eq)]
enum Packed {
    /// The layer every corner was packed with, quad by quad.
    Corners(Vec<u16>),
    /// The section was refused, naming this block.
    RefusedNaming(String),
    /// The section was refused for a reason that is not about a texture.
    RefusedOtherwise(String),
}

#[test]
fn corners_are_packed_with_the_layer_the_assignment_names_and_not_with_a_sorted_position()
-> TestResult {
    let content = stating(&DISAGREEING, &DISAGREEING)?;
    let view = ContentView::of(&content);

    let packed = packing(&blocks_of(&DISAGREEING), view.resolution())?;

    assert_eq!(
        (packed, DISAGREEING.map(|(_, layer)| layer).to_vec()),
        (Packed::Corners(corners_for(&DISAGREEING)), vec![2, 0, 1]),
        "a layer index rides inside every packed vertex, so a client that derives one instead of \
         honouring the one it was handed renumbers the array texture the moment a block is \
         inserted — and textures the whole world wrong with no error anywhere. The second half of \
         this comparison is the fixture's own guard: these three layers are not the positions \
         their keys occupy in sorted order, and a fixture where they were could not fail whatever \
         the client did"
    );
    Ok(())
}

#[test]
fn a_block_the_assignment_names_no_layer_for_is_refused_by_name_rather_than_packed_from_layer_zero()
-> TestResult {
    let content = stating(&DISAGREEING, &NAMING_TWO_OF_THREE)?;
    let view = ContentView::of(&content);

    let unassigned = packing(&[UNASSIGNED_BLOCK.to_owned()], view.resolution())?;
    let assigned = packing(&[NAMING_TWO_OF_THREE[0].0.to_owned()], view.resolution())?;

    assert_eq!(
        (unassigned, assigned),
        (
            Packed::RefusedNaming(UNASSIGNED_BLOCK.to_owned()),
            Packed::Corners(corners_for(&NAMING_TWO_OF_THREE[..1]))
        ),
        "packing a block the assignment says nothing about from layer zero draws it as whichever \
         block owns layer zero — a picture that is wrong in an entirely plausible way, which is \
         the failure nothing downstream can report. It must be refused by name instead. The \
         second half is what says the refusal is about this block and not about every block: the \
         one the assignment does name still packs"
    );
    Ok(())
}

#[test]
fn a_block_whose_declared_texture_is_not_its_own_name_packs_from_the_key_it_declared() -> TestResult
{
    let view = ContentView::of(&substituting()?);

    let packed = packing(&[SUBSTITUTED_BLOCK.to_owned()], view.resolution())?;

    assert_eq!(
        packed,
        Packed::Corners(corners_for(&SUBSTITUTED_ASSIGNMENT[..1])),
        "the assignment names a layer for the key this block declares, and that is the layer its \
         corners carry. Reached through the client's own view of resolved content rather than \
         through a resolution a fixture assembled, because that construction is where a lookup \
         keyed on a block's name would survive: every other reading hands the packer a resolution \
         somebody wrote out by hand"
    );
    Ok(())
}

/// The second site of the same lookup, reached the same way.
///
/// The geometry builder is not the only consumer that has to resolve a block to
/// a key: the held-block indicator does too, and closing one site while leaving
/// the other would show a block drawing correctly in the world with a blank
/// indicator beside it — which reads as a HUD fault and sends whoever chases it
/// to the wrong module. One reading through the indicator, over the same view,
/// is what holds the two together.
#[test]
fn the_held_block_indicator_shows_the_layer_of_the_key_the_block_declared() -> TestResult {
    let view = ContentView::of(&substituting()?);
    let substituted = BlockName::parse(SUBSTITUTED_BLOCK)?;
    let plainly_keyed = BlockName::parse(PLAINLY_KEYED_BLOCK)?;

    assert_eq!(
        (
            held_swatch(Some(&substituted), view.resolution()),
            held_swatch(Some(&plainly_keyed), view.resolution())
        ),
        (
            HeldSwatch::Shows {
                key: TextureKey::parse(ITS_DECLARED_KEY)?,
                face: INDICATOR_FACE,
            },
            HeldSwatch::Shows {
                key: TextureKey::parse(PLAINLY_KEYED_BLOCK)?,
                face: INDICATOR_FACE,
            }
        ),
        "the indicator draws the key the held block declares, whether or not that key is the \
         block's own name. The block beside it, whose key *is* its name, still shows — without \
         that half a lookup that answered with the declared key for everything and the name for \
         nothing would read the same as one that had stopped resolving at all"
    );
    Ok(())
}

/// Resolved content stating one solid block per entry of `blocks`, each naming
/// its own name as its texture key, and the assignment `layers` names.
///
/// # Errors
///
/// Returns an error if a fixture id is not a namespaced id.
fn stating(
    blocks: &[(&str, u16)],
    layers: &[(&str, u16)],
) -> Result<ResolvedContent, Box<dyn Error>> {
    let mut stated = Vec::new();
    for (name, _) in blocks {
        stated.push(ResolvedBlock {
            name: BlockName::parse(name)?,
            textures: FaceTextures::uniform(TextureKey::parse(name)?),
            is_solid: true,
            opacity: Opacity::OPAQUE,
        });
    }
    Ok(ResolvedContent::stating(stated, assigned(layers)?))
}

/// Resolved content whose first block declares a texture key that is not its own
/// name, beside one whose key is its name, with a layer for both keys.
///
/// # Errors
///
/// Returns an error if a fixture id is not a namespaced id.
fn substituting() -> Result<ResolvedContent, Box<dyn Error>> {
    Ok(ResolvedContent::stating(
        vec![
            ResolvedBlock {
                name: BlockName::parse(SUBSTITUTED_BLOCK)?,
                textures: FaceTextures::uniform(TextureKey::parse(ITS_DECLARED_KEY)?),
                is_solid: true,
                opacity: Opacity::OPAQUE,
            },
            ResolvedBlock {
                name: BlockName::parse(PLAINLY_KEYED_BLOCK)?,
                textures: FaceTextures::uniform(TextureKey::parse(PLAINLY_KEYED_BLOCK)?),
                is_solid: true,
                opacity: Opacity::OPAQUE,
            },
        ],
        assigned(&SUBSTITUTED_ASSIGNMENT)?,
    ))
}

/// The block names of a fixture table.
fn blocks_of(table: &[(&str, u16)]) -> Vec<String> {
    table.iter().map(|(name, _)| (*name).to_owned()).collect()
}

/// The layer every corner must carry, for one quad per entry of `table` in the
/// order the entries are written.
///
/// Derived from the fixture table rather than copied out of a run, so editing an
/// entry moves the expectation with it.
fn corners_for(table: &[(&str, u16)]) -> Vec<u16> {
    table
        .iter()
        .flat_map(|(_, layer)| std::iter::repeat_n(*layer, CORNERS_PER_QUAD))
        .collect()
}

/// What packing one quad per entry of `blocks` against `layers` came to.
///
/// # Errors
///
/// Returns an error if a fixture id is not a block name.
fn packing(blocks: &[String], resolution: &TextureResolution) -> Result<Packed, Box<dyn Error>> {
    let mut quads = Vec::new();
    for (plane, block) in blocks.iter().enumerate() {
        quads.push(Quad {
            facing: Facing::PosY,
            plane: u32::try_from(plane)?,
            origin: PlanePos {
                primary: 0,
                secondary: 0,
            },
            extent: PlaneExtent {
                primary: 1,
                secondary: 1,
            },
            block: BlockName::parse(block)?,
        });
    }
    match build_section_geometry(&quads, SectionOrigin::new(SECTION), resolution) {
        Ok(geometry) => Ok(Packed::Corners(corner_layers(&geometry))),
        Err(GeometryError::UnresolvedTexture { block, .. }) => {
            Ok(Packed::RefusedNaming(block.as_str().to_owned()))
        }
        Err(other) => Ok(Packed::RefusedOtherwise(other.to_string())),
    }
}

/// The layer every corner of `geometry` was packed with, in the order they were
/// emitted.
fn corner_layers(geometry: &SectionGeometry) -> Vec<u16> {
    (0..)
        .map_while(|corner| geometry.layer_at(corner))
        .collect()
}
