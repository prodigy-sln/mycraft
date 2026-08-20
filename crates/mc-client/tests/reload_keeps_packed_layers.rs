//! What a reader packs from the content a reload published: the indices that
//! assignment states, and the same indices a section carried before the reload.
//!
//! # Every reading here goes through the packer
//!
//! Asking the published assignment which layer it holds for a key leaves the
//! consumer free to re-derive one of its own, which is the exact failure the
//! policy exists to close. So both readings below pack real quads through
//! [`build_section_geometry`] — the same function the client's own `scene_of`
//! calls — and read the layer back out of the corners it produced.
//!
//! # Why the second reading needs a reload in front of it
//!
//! That the packer honours a stated assignment rather than a sorted one is
//! already pinned, over an assignment a fixture wrote by hand
//! (`stated_layers_are_honoured.rs`). What is new here is *where the assignment
//! comes from*: the product's own reload produces one, the client asks for it, and
//! the reader packs from that. A reading that reached the packing path any other
//! way would re-prove what is already proved through the same code path.
//!
//! The assignment a reload produces is deliberately not the lexicographic one and
//! nobody had to arrange that: `base:amber` sorts ahead of all four shipped keys
//! and takes the highest layer, because appending is what the policy does. Both
//! readings assert that ordering rather than describing it, so a fixture in which
//! the sorted answer and the stated answer happened to agree fails instead of
//! passing for the wrong reason.
//!
//! # These blocks declare `texture` equal to `name`, and nothing here turns on it
//!
//! The packer resolves a face's key from its block's declaration, so a fixture
//! naming a different texture would pack perfectly well. These keep the two the
//! same because what the readings below are about is *which layer* an assignment
//! states, and a second difference between name and key would be one more thing
//! a reader has to hold while following them.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;

use mc_client::content::ContentView;
use mc_core::id::BlockName;
use mc_render::geometry::{GeometryError, SectionGeometry, SectionOrigin, build_section_geometry};
use mc_render::texture::TextureResolution;
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};

use input::InputHarness;
use reload::{
    AMBER, AMBER_FILE, DIRT, GRASS, GRASS_TOP, STONE, WATER, accepted, adoption, amber, declaring,
    shipped,
};
use reload_content::{
    NOTHING_IS_SERVING, THE_NEXT_UNUSED_LAYER, candidate_against, fresh_layers, publishing,
};
use reload_world::{floor_of, playing, standing};
use support::{TestResult, content_root};

/// How many corners one quad is packed into.
///
/// Stated so the expectations below cover the whole run of corners rather than
/// sampling one: a packer writing the right layer into the first corner of each
/// quad and the wrong one into the other three draws three quarters of the world
/// from the wrong texture.
const CORNERS_PER_QUAD: usize = 4;

/// Where the packed section sits. Nothing here depends on it.
const SECTION: [i32; 3] = [0, 0, 0];

/// Where `base:amber` sorts among the keys a session states once it has been
/// declared — first, ahead of every shipped key.
///
/// Asserted rather than described, because it is the whole of what makes these two
/// readings falsifiable: a reader that re-derived its layers from a sort would give
/// the key in this position layer zero.
const AMBER_SORTS_FIRST: usize = 0;

/// What packing a run of quads came to.
///
/// A total verdict rather than an error propagated out of the test: a packer that
/// accepted what it should refuse fails on this comparison instead of ending the
/// test before its assertion ran.
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
fn a_section_not_meshed_again_packs_the_layers_it_carried_before_a_key_was_appended() -> TestResult
{
    let mut client = a_client_over(STONE)?;
    let before = packing(&shipped_blocks(), &resolution_of(&client)?)?;

    let root = declaring(shipped()?, AMBER_FILE, &amber())?;
    let candidate = candidate_against(&root, client.content())?;
    let answered = adoption(client.adopt(candidate));
    let appended = publishing(client.content())?.layers.get(AMBER).copied();
    let after = packing(&shipped_blocks(), &resolution_of(&client)?)?;

    assert_eq!(
        (answered, appended, after, before),
        (
            accepted(AMBER),
            Some(THE_NEXT_UNUSED_LAYER),
            Packed::Corners(corners_of_the_shipped_blocks()?),
            Packed::Corners(corners_of_the_shipped_blocks()?)
        ),
        "a reload appended a layer, and the sections the world had already uploaded were not meshed \
         again. Re-packing one of them against the content now serving has to produce the bytes it \
         produced before — `base:stone` on the third layer either side — or every vertex on the \
         device is pointing at somebody else's texture, and nothing anywhere reports it. The two \
         readings are compared against the same derived expectation *and* against each other, so a \
         packer that moved both together is caught as well. The second element is the scenario's own \
         premise, asserted rather than assumed: **a client that published nothing at all would leave \
         both readings agreeing for the wrong reason**, because a layer that was never appended \
         cannot renumber anything"
    );
    Ok(())
}

#[test]
fn a_reader_handed_the_published_content_packs_the_layer_that_assignment_states() -> TestResult {
    let mut client = a_client_over(STONE)?;
    let root = declaring(shipped()?, AMBER_FILE, &amber())?;
    let candidate = candidate_against(&root, client.content())?;

    let answered = adoption(client.adopt(candidate));
    let stated = publishing(client.content())?.layers;
    let packed = packing(&[AMBER], &resolution_of(&client)?)?;

    assert_eq!(
        (answered, stated.keys().position(|key| key == AMBER), packed),
        (
            accepted(AMBER),
            Some(AMBER_SORTS_FIRST),
            Packed::Corners(vec![THE_NEXT_UNUSED_LAYER; CORNERS_PER_QUAD])
        ),
        "the assignment this reader was handed across the reload is deliberately not the \
         lexicographic one, and the middle of this comparison is what says so: the key sorts first \
         among everything the session states and holds the highest layer of the lot. A reader \
         deriving its own answer from that order packs zero into every corner and draws the new \
         block as whichever block owns layer zero — a picture wrong in an entirely plausible way, \
         which is the failure nothing downstream can report"
    );
    Ok(())
}

/// A client playing a one-column floor of `floor`, with the shipped content root
/// serving.
fn a_client_over(floor: &'static str) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(&content_root()?, standing(), |registry| {
        floor_of(registry, floor)
    })?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// The layers a reader builds out of what `client` is publishing, through the
/// client's own view.
///
/// **Asked of the client rather than of a content set handed in**, so what is
/// packed below is what the client would draw from and not a second read of the
/// same root — which would agree with the fixture by construction while the
/// publication carried nothing.
///
/// # Errors
///
/// Returns an error where the client publishes nothing, which is a client with no
/// world rather than one a reader could be handed anything from.
fn resolution_of(client: &InputHarness) -> Result<TextureResolution, Box<dyn Error>> {
    let published = client.content().ok_or(NOTHING_IS_SERVING)?;
    Ok(ContentView::of(&published.resolved).into_resolution())
}

/// Which key each shipped block draws on the facing these quads show.
///
/// **Stated, because a block's name is no longer its key.** Every quad below
/// faces upward, and the grass block declares `base:grass_top` for that facing
/// while dirt, stone and water each state one key across all six of theirs. A
/// fixture that went on treating the two as the same word would build quads
/// naming blocks no root declares — which is what this list was before the grass
/// block gained facings, and it read as a packer refusal rather than as a stale
/// fixture.
const UPWARD: [(&str, &str); 4] = [
    (DIRT, DIRT),
    (GRASS, GRASS_TOP),
    (STONE, STONE),
    (WATER, WATER),
];

/// The blocks one quad each is packed for, in the order [`UPWARD`] states them.
fn shipped_blocks() -> Vec<&'static str> {
    UPWARD.iter().map(|(block, _)| *block).collect()
}

/// The layer every corner must carry for one quad per shipped block, in the order
/// they are packed.
///
/// Derived twice over rather than written down: the block names its upward key,
/// and that key's layer is its position in the ascending list. So the grass
/// block's quad carries a five because `base:grass_top` is sixth among the eight
/// keys, and not because anybody wrote a five.
///
/// # Errors
///
/// Returns an error if the shipped key list is not the ascending one this
/// arithmetic rests on, or if it does not hold a key a block draws.
fn corners_of_the_shipped_blocks() -> Result<Vec<u16>, Box<dyn Error>> {
    let assigned = fresh_layers()?;
    let mut corners = Vec::new();
    for (block, key) in UPWARD {
        let layer = assigned.get(key).ok_or_else(|| {
            format!("a launch assigns no layer to `{key}`, which `{block}` draws upward")
        })?;
        corners.extend(std::iter::repeat_n(*layer, CORNERS_PER_QUAD));
    }
    Ok(corners)
}

/// What packing one quad per entry of `blocks` against `layers` came to.
///
/// # Errors
///
/// Returns an error if a fixture id is not a block name, or if a plane index does
/// not fit.
fn packing(blocks: &[&str], resolution: &TextureResolution) -> Result<Packed, Box<dyn Error>> {
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
