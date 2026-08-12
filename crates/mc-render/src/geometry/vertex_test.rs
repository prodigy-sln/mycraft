//! Bit-packing a vertex: the round trip, the width, and the refusal.
//!
//! Three properties, and each one exists because a plausible implementation
//! fails exactly it while satisfying the other two.
//!
//! The round trip is probed at the awkward end of every field at once: a corner
//! coordinate of 16, which is the one value that does not fit in the four bits a
//! `0..16` voxel coordinate needs and is precisely where a `+X` face at plane 15
//! lands; a coordinate of 0 beside it, so a field shifted into its neighbour
//! shows up rather than landing on the same number; and a facing on the axis
//! whose bits sit next to the coordinates'. A pack that truncated 16 to 0, or
//! that read the facing out of the wrong three bits, returns a vertex that looks
//! entirely reasonable — and this test names the field that moved.
//!
//! The width is asserted against the packed value's own size rather than against
//! `to_le_bytes`'s length. That length is fixed by the signature, so asserting it
//! could never be red; the size of the type is the only form of "at most eight
//! bytes" that a struct-of-fields implementation actually fails.
//!
//! The refusal is the third: a coordinate of 17 has to come back as an error
//! naming the axis and the value, because the alternative — masking it down to a
//! representable 1 — produces a vertex on the far side of the section that no
//! later stage can tell from a deliberate one.

use std::error::Error;

use mc_world::mesh::Facing;
use mc_world::section::Axis;

use super::{PackError, PackedVertex, Vertex};

type TestResult = Result<(), Box<dyn Error>>;

/// The most bytes a packed vertex may occupy. A `16³` section is meant to stay
/// cache-resident, which is what makes the width a contract rather than a
/// preference.
const PACKED_WIDTH: usize = 8;

/// The last section-local corner coordinate. Corners run `0..=16`, one further
/// than the voxel coordinates they are derived from, because the face a voxel at
/// plane 15 emits along `+X` sits at x = 16.
const LAST_LOCAL_COORDINATE: u32 = 16;

/// One past the last corner coordinate, which is what packing must refuse.
const BEYOND_LAST_LOCAL_COORDINATE: u8 = 17;

#[test]
fn packing_and_unpacking_a_vertex_returns_its_position_facing_and_layer() -> TestResult {
    let vertex = Vertex {
        local: [16, 0, 15],
        facing: Facing::PosX,
        layer: 3,
        section: 7,
    };

    let restored = PackedVertex::pack(&vertex)?.unpack();

    assert_eq!(
        (restored.local, restored.facing, restored.layer),
        ([16, 0, 15], Facing::PosX, 3),
        "a packed vertex must come back carrying the position, facing and texture layer \
         it went in with"
    );
    Ok(())
}

#[test]
fn a_packed_vertex_occupies_no_more_than_eight_bytes() -> TestResult {
    let packed = PackedVertex::pack(&Vertex {
        local: [16, 16, 16],
        facing: Facing::PosZ,
        layer: 2,
        section: 5,
    })?;

    assert!(
        size_of_val(&packed) <= PACKED_WIDTH,
        "a packed vertex must occupy at most {PACKED_WIDTH} bytes, but this one occupies {}; \
         a vertex holding its fields side by side is not bit-packed, whatever it is called",
        size_of_val(&packed)
    );
    Ok(())
}

#[test]
fn packing_a_corner_beyond_the_section_names_the_axis_and_the_value() -> TestResult {
    let beyond = Vertex {
        local: [BEYOND_LAST_LOCAL_COORDINATE, 0, 0],
        facing: Facing::PosX,
        layer: 3,
        section: 7,
    };

    let refusal = PackedVertex::pack(&beyond).err().ok_or(
        "a corner coordinate of 17 has no representation, so packing must refuse it rather \
         than truncate it to one that exists",
    )?;

    match refusal {
        PackError::CoordinateOutOfRange { axis, value, max } => assert_eq!(
            (axis, value, max),
            (
                Axis::X,
                u32::from(BEYOND_LAST_LOCAL_COORDINATE),
                LAST_LOCAL_COORDINATE
            ),
            "the refusal must name the axis that overflowed, the value it carried, and the \
             largest one it could have carried"
        ),
        other => {
            return Err(format!("expected a coordinate refusal, got {other:?}").into());
        }
    }
    Ok(())
}
