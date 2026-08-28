//! A section the replay cannot mesh fails the whole preparation, by name.
//!
//! This is the deliberate reversal `docs/technical/rendering.md`'s non-cascade
//! rule does not reach: that rule is about re-meshing a live world, where a
//! previous mesh exists to keep. The replay is the *initial* preparation of a
//! fixed fixture, so a section that will not mesh is a defect in this feature's
//! own code, and continuing would leave every golden and every probe measuring a
//! world with a hole in it.
//!
//! **Which section is reported is part of the contract, not an incidental.**
//! Preparation runs on rayon workers, and `collect::<Result<Vec<_>, _>>()`
//! short-circuits, so it surfaces whichever section happened to fail first under
//! that run's scheduling. The fixture below makes *every* section unmeshable
//! precisely so that a reproducible answer is the only way to pass: the lowest
//! section of the lowest column, every run.

mod support;

use std::error::Error;

use mc_core::block::source::InMemoryDefinitionSource;
use mc_core::block::{BlockDefinition, BlockRegistry, DefinitionOrigin, Opacity};
use mc_core::content::FaceTextures;
use mc_core::id::{BlockName, TextureKey};
use mc_sim::replay::{PrepareError, mesh_all};

use support::{TestResult, content_registry, replay_world};

/// How many sections the replay prepares: sixteen columns of sixteen.
const SECTIONS: usize = 256;

/// A block none of the replay world holds, and the only one the foreign
/// registry below knows.
const FOREIGN: &str = "example:absent_from_the_world";

/// What the foreign registry attributes its one definition to.
const FIXTURE_ORIGIN: &str = "a preparation test's registry";

#[test]
fn a_section_that_cannot_be_meshed_fails_the_preparation_naming_its_column_and_index() -> TestResult
{
    let world = replay_world(&content_registry()?)?;
    let prepared = mesh_all(&world, &content_registry()?)?;

    let refused = mesh_all(&world, &foreign_registry()?)
        .err()
        .ok_or("a world of blocks the registry does not know cannot be meshed")?;

    assert_eq!(
        prepared.len(),
        SECTIONS,
        "the world's own registry has to prepare every section, or the refusal below \
         could be about a world that was never preparable"
    );
    let reported = match refused {
        PrepareError::Mesh {
            column,
            section_index,
            ..
        } => (column.x, column.z, section_index),
    };
    assert_eq!(
        reported,
        (0, 0, 0),
        "the refusal has to name the first section in preparation order, the same one \
         on every run — a short-circuiting collect names whichever worker lost the race"
    );
    Ok(())
}

/// A registry that knows one block, and it is not one the world holds.
///
/// It names no block it breaks into, because naming one would be naming a second
/// block this registry does not know either — and the refusal under test is about
/// the name the *world* holds, which has to be the only one missing. The other
/// two fields a definition carries are left at what saying nothing about them
/// means, for the same reason: nothing here breaks or places anything.
fn foreign_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let origin = DefinitionOrigin::new(FIXTURE_ORIGIN);
    let definition = BlockDefinition {
        name: BlockName::parse(FOREIGN)?,
        textures: FaceTextures::uniform(TextureKey::parse(FOREIGN)?),
        is_solid: true,
        replaceable: false,
        breakable: true,
        breaks_into: None,
        drawn: true,
        occludes: true,
        targetable: true,
        swimmable: false,
        move_resistance: 0.0,
        swim_ascent: 9.0,
        opacity: Opacity::OPAQUE,
        origin: origin.clone(),
    };
    let mut registry = BlockRegistry::new();
    registry.apply(&InMemoryDefinitionSource::new(origin, vec![Ok(definition)]))?;
    Ok(registry)
}
