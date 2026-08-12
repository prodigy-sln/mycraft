//! The six sections around the one being meshed, each supplied or not.
//!
//! There is exactly one mapping from a facing to a slot in this crate, and it is
//! the facing's own discriminant. That is the whole reason the six sections are
//! not six named fields: a second place where the mapping is written down is a
//! second place it can be written down wrongly, and two slots wired to each
//! other produces a perfectly plausible mesh with one wall of the world decided
//! against the wrong chunk. Because the slot is the discriminant, a swapped slot
//! and a reordered emission are the same mistake and fail together.
//!
//! Absence is six independent options and never one flag. A section is routinely
//! meshed with the chunk below it loaded and the other five still streaming, and
//! reading that as "no neighbours at all" would put a seam under everything that
//! has not arrived yet.

use crate::section::Section;

use super::Facing;

/// How many sections surround one.
const AROUND_A_SECTION: usize = Facing::ALL.len();

/// The sections beyond each of a section's six faces.
#[derive(Debug, Clone, Copy)]
pub struct Neighbours<'a> {
    around: [Option<&'a Section>; AROUND_A_SECTION],
}

impl Default for Neighbours<'_> {
    fn default() -> Self {
        Self {
            around: [None; AROUND_A_SECTION],
        }
    }
}

impl<'a> Neighbours<'a> {
    /// Nothing loaded around the section at all.
    ///
    /// Every boundary face is then decided as an absent neighbour is — visible,
    /// rather than sealed shut against content that is not there.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// These neighbours with `section` beyond `facing`.
    ///
    /// Naming the facing rather than handing over an index is what makes a
    /// swapped neighbour visible at the call site, which is where a caller can
    /// still see it is wrong.
    #[must_use]
    pub fn with(mut self, facing: Facing, section: &'a Section) -> Self {
        if let Some(slot) = self.around.get_mut(facing as usize) {
            *slot = Some(section);
        }
        self
    }

    /// The section beyond `facing`, if one was supplied.
    ///
    /// Crate-internal, and staying that way. The boundary resolution is the only
    /// caller, and keeping it off the public surface is also what mechanically
    /// stops the independent visible-face oracle from reaching the very
    /// facing-to-slot mapping it exists to judge.
    pub(crate) fn at(&self, facing: Facing) -> Option<&'a Section> {
        self.around.get(facing as usize).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
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
    use mc_core::id::{BlockName, TextureKey};

    use super::Neighbours;
    use crate::mesh::Facing;
    use crate::section::{LocalPos, Section};

    /// The error type these guards propagate with `?`.
    type GuardResult = Result<(), Box<dyn Error>>;

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
            declared.push(Ok(BlockDefinition {
                name: BlockName::parse(name)?,
                texture: TextureKey::parse(name)?,
                is_solid: true,
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
        Ok(beyond.block_at(ANY_VOXEL)?.as_str().to_owned())
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
}
