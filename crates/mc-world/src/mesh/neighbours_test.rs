//! Guard. A neighbour goes in named by a facing and comes back out for that
//! facing, and for no other.
//!
//! There is one facing-to-slot mapping in this crate and it is the facing's own
//! discriminant. That is the whole reason the six sections are not six named
//! fields: a second place where the mapping is written down is a second place it
//! can be written down wrongly, and two slots wired to each other is a mistake
//! that produces a perfectly plausible mesh with one wall of the world decided
//! against the wrong chunk.
//!
//! What is observable from outside the container is the round trip, so that is
//! what is asserted, with six sections a reader can tell apart rather than six
//! copies of one. A container that kept only the last neighbour handed to it, or
//! that stored two facings in one slot, answers with the wrong block rather than
//! with a section that merely looks similar.
//!
//! Absence is the other half. It is six independent options, never one flag: a
//! section is routinely meshed with the chunk below it loaded and the other five
//! not, and reading that as "no neighbours at all" would put a seam under
//! everything that is still streaming in.

use std::error::Error;

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};

use super::Neighbours;
use crate::mesh::Facing;
use crate::section::{Contents, LocalPos, Section};

/// The error type these guards propagate with `?`.
type GuardResult = Result<(), Box<dyn Error>>;

/// What a cell holding nothing reads as here. Every section around is filled
/// with a block, so this is an answer no assertion below expects — which is
/// exactly why it is spelled rather than folded into one of the six names.
const NOTHING: &str = "nothing";

/// A block per facing, so the six sections around are told apart by what they
/// hold rather than by being different objects.
const BEYOND_NEG_X: &str = "example:beyond_neg_x";
const BEYOND_POS_X: &str = "example:beyond_pos_x";
const BEYOND_NEG_Y: &str = "example:beyond_neg_y";
const BEYOND_POS_Y: &str = "example:beyond_pos_y";
const BEYOND_NEG_Z: &str = "example:beyond_neg_z";
const BEYOND_POS_Z: &str = "example:beyond_pos_z";

/// Those six, in the order the facings are declared in.
const AROUND: [&str; 6] = [
    BEYOND_NEG_X,
    BEYOND_POS_X,
    BEYOND_NEG_Y,
    BEYOND_POS_Y,
    BEYOND_NEG_Z,
    BEYOND_POS_Z,
];

/// What these guards attribute their definitions to.
const GUARD_ORIGIN: &str = "a neighbour guard's registry";

/// The voxel every section here is asked about. Any voxel would do; each section
/// is filled with a single block.
const ANY_VOXEL: LocalPos = LocalPos { x: 0, y: 0, z: 0 };

/// A registry holding the six blocks the sections around are made of.
fn registry_of_the_six() -> Result<BlockRegistry, Box<dyn Error>> {
    let mut declared = Vec::with_capacity(AROUND.len());
    for name in AROUND {
        // All six are solid, and nothing else they could declare is stated:
        // what these guards are about is which section was consulted for which
        // facing, and nothing along that path reads a definition beyond its
        // solidity.
        declared.push(Ok(BlockDefinition {
            name: BlockName::parse(name)?,
            textures: FaceTextures::uniform(TextureKey::parse(name)?),
            is_solid: true,
            replaceable: false,
            breakable: true,
            breaks_into: None,
            drawn: true,
            occludes: true,
            targetable: true,
            swimmable: false,
            move_resistance: 0.0,
            origin: DefinitionOrigin::new(GUARD_ORIGIN),
        }));
    }
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(
        DefinitionOrigin::new(GUARD_ORIGIN),
        declared,
    ))?;
    Ok(registry)
}

/// One section per facing, each filled with a block none of the others holds.
fn sections_around(registry: &BlockRegistry) -> Result<Vec<Section>, Box<dyn Error>> {
    let mut around = Vec::with_capacity(AROUND.len());
    for name in AROUND {
        around.push(Section::filled(&BlockName::parse(name)?, registry)?);
    }
    Ok(around)
}

/// Which block the section beyond `facing` is made of, or an explanation of why
/// asserting on it would have been vacuous.
fn block_beyond(neighbours: &Neighbours<'_>, facing: Facing) -> Result<String, Box<dyn Error>> {
    let beyond = neighbours.at(facing).ok_or_else(|| {
        format!("a section was supplied for {facing}, so one has to come back for it")
    })?;
    Ok(match beyond.block_at(ANY_VOXEL)? {
        Contents::Empty => NOTHING.to_owned(),
        Contents::Holds(name) => name.as_str().to_owned(),
    })
}

#[test]
fn a_neighbour_named_by_a_facing_comes_back_for_that_facing_and_no_other() -> GuardResult {
    let registry = registry_of_the_six()?;
    let around = sections_around(&registry)?;
    let mut neighbours = Neighbours::none();
    for (facing, section) in Facing::ALL.into_iter().zip(&around) {
        neighbours = neighbours.with(facing, section);
    }

    let mut read_back = Vec::with_capacity(AROUND.len());
    for facing in Facing::ALL {
        read_back.push(block_beyond(&neighbours, facing)?);
    }

    assert_eq!(
        read_back,
        AROUND.map(str::to_owned).to_vec(),
        "each of the six sections went in named by a different facing, and each has to come \
         back for the facing it went in under. Two facings sharing a slot, or a slot read \
         through a second mapping, hands the mesher a real section that belongs somewhere else \
         — and the faces it decides against it are as plausible as the right ones"
    );
    Ok(())
}

#[test]
fn naming_one_neighbour_leaves_the_other_five_absent() -> GuardResult {
    let registry = registry_of_the_six()?;
    let below = Section::filled(&BlockName::parse(BEYOND_NEG_Y)?, &registry)?;

    let neighbours = Neighbours::none().with(Facing::NegY, &below);

    let supplied: Vec<bool> = Facing::ALL
        .into_iter()
        .map(|facing| neighbours.at(facing).is_some())
        .collect();
    assert_eq!(
        supplied,
        vec![false, false, true, false, false, false],
        "absence is per neighbour and never all-or-nothing. A section with the one below it \
         loaded and the other five still streaming is the ordinary case at a chunk boundary, \
         and a container answering 'some neighbours were supplied' for all six would decide \
         five boundaries against sections that are not there"
    );
    Ok(())
}
