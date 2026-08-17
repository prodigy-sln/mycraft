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
//! # These fixtures declare `texture` equal to `name`, against the convention
//!
//! Everywhere else in this feature a block's texture key is deliberately not its
//! own name. Here it must be, and that is a constraint on the fixture rather
//! than a fact about the requirement: an entry of the assignment is selected by
//! the block's **name** today, so a fixture whose blocks named a different
//! texture would be refused for that reason instead — red for the wrong reason,
//! reading as a defect in the assignment when it is not one. The one reading
//! below whose subject *is* that substitution states the two differently, on
//! purpose.

use std::error::Error;

use mc_client::content::ContentView;
use mc_core::content::{ResolvedBlock, ResolvedContent};
use mc_core::id::{BlockName, TextureKey};
use mc_render::geometry::{GeometryError, SectionGeometry, SectionOrigin, build_section_geometry};
use mc_render::hud::{HeldSwatch, held_swatch};
use mc_render::texture::TextureLayers;
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};

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

    let packed = packing(&blocks_of(&DISAGREEING), view.layers())?;

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

    let unassigned = packing(&[UNASSIGNED_BLOCK.to_owned()], view.layers())?;
    let assigned = packing(&[NAMING_TWO_OF_THREE[0].0.to_owned()], view.layers())?;

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
fn a_block_whose_declared_texture_is_not_its_own_name_is_refused_at_packing_time_naming_the_block()
-> TestResult {
    let view = ContentView::of(&substituting()?);

    let packed = packing(&[SUBSTITUTED_BLOCK.to_owned()], view.layers())?;

    assert_eq!(
        packed,
        Packed::RefusedNaming(SUBSTITUTED_BLOCK.to_owned()),
        "the assignment names a layer for the key this block declares, and the packer still \
         selects an entry by parsing the block's own *name* as a texture key. The two agree only \
         because every shipped block declares them identically. This pins that substitution as a \
         reading rather than as a comment in two CLAUDE.md files: a declaration whose `texture` \
         differs from its `name` loads and then does not draw"
    );
    Ok(())
}

/// The second site of the same substitution, which no existing note records.
///
/// The geometry builder is not the only consumer resolving a block by parsing
/// its name as a texture key: the held-block indicator does it too. A spec
/// closing the gap that found one site and left the other would show a block
/// drawing correctly in the world while its indicator drew nothing, which reads
/// as a HUD bug. One reading through the indicator is what stops that.
#[test]
fn the_held_block_indicator_resolves_by_the_blocks_name_too_and_shows_nothing_for_such_a_block()
-> TestResult {
    let view = ContentView::of(&substituting()?);
    let substituted = BlockName::parse(SUBSTITUTED_BLOCK)?;
    let plainly_keyed = BlockName::parse(PLAINLY_KEYED_BLOCK)?;

    assert_eq!(
        (
            held_swatch(Some(&substituted), view.layers()),
            held_swatch(Some(&plainly_keyed), view.layers())
        ),
        (
            HeldSwatch::Unresolved {
                block: substituted.clone(),
                key: Some(TextureKey::parse(SUBSTITUTED_BLOCK)?),
            },
            HeldSwatch::Shows {
                key: TextureKey::parse(PLAINLY_KEYED_BLOCK)?,
            }
        ),
        "the indicator parses the held block's own name as a texture key, so a block declaring a \
         different one draws no swatch even though the assignment names a layer for the key it \
         declared. The block beside it, whose key is its name, still shows — without that half \
         an indicator that resolved nothing at all would read the same"
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
            texture: TextureKey::parse(name)?,
            is_solid: true,
        });
    }
    let mut assignment = Vec::new();
    for (key, layer) in layers {
        assignment.push((TextureKey::parse(key)?, *layer));
    }
    Ok(ResolvedContent::stating(stated, assignment))
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
                texture: TextureKey::parse(ITS_DECLARED_KEY)?,
                is_solid: true,
            },
            ResolvedBlock {
                name: BlockName::parse(PLAINLY_KEYED_BLOCK)?,
                texture: TextureKey::parse(PLAINLY_KEYED_BLOCK)?,
                is_solid: true,
            },
        ],
        vec![
            (TextureKey::parse(ITS_DECLARED_KEY)?, 0),
            (TextureKey::parse(PLAINLY_KEYED_BLOCK)?, 1),
        ],
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
fn packing(blocks: &[String], layers: &TextureLayers) -> Result<Packed, Box<dyn Error>> {
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
    match build_section_geometry(&quads, SectionOrigin::new(SECTION), layers) {
        Ok(geometry) => Ok(Packed::Corners(corner_layers(&geometry))),
        Err(GeometryError::UnresolvedTexture { block }) => {
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
