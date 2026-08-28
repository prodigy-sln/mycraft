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

use mc_core::block::Opacity;
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

/// The last section index a scene holds, and the value the round trip below
/// carries so that the field packed *above* it cannot move it unnoticed.
///
/// A field appended at the top of the used range is the one edit that can shift
/// its neighbour without shifting anything else, and a section index of 7 —
/// which is what this reading used to carry, and never asserted — comes back
/// unchanged under a great many wrong shifts.
const LAST_SECTION: u16 = 1023;

/// The byte a declared half encodes to.
///
/// Derived rather than observed: `0.5 x 255` is `127.5`, which
/// `Opacity::quantised` rounds half away from zero to 128.
const A_HALF_ENCODES_TO: u8 = 128;

/// The degree that byte carries back, written out as the arithmetic rather than
/// taken from `Opacity::from_quantised`, which is one of the two halves under
/// test here.
///
/// **The round trip is deliberately not the identity on this field.** A declared
/// `0.5` comes back as `0.501960...`, because two hundred and fifty-six bytes
/// cannot name every degree a declaration may state. A reading demanding the
/// declared number back would be red against a correct packer, and its cheapest
/// green would be to widen the field.
const A_HALF_COMES_BACK_AS: f32 = A_HALF_ENCODES_TO as f32 / u8::MAX as f32;

#[test]
fn packing_and_unpacking_a_vertex_returns_every_field_it_went_in_with() -> TestResult {
    let vertex = Vertex {
        local: [16, 0, 15],
        facing: Facing::PosX,
        layer: 3,
        section: LAST_SECTION,
        opacity: Opacity::new(0.5).ok_or("a half is a degree of opacity")?,
    };

    let restored = PackedVertex::pack(&vertex)?.unpack();

    assert_eq!(
        (
            restored.local,
            restored.facing,
            restored.layer,
            restored.section,
            restored.opacity.get(),
        ),
        (
            [16, 0, 15],
            Facing::PosX,
            3,
            LAST_SECTION,
            A_HALF_COMES_BACK_AS,
        ),
        "a packed vertex must come back carrying the position, facing, texture layer, section \
         and degree of opacity it went in with. The last two are asserted together because they \
         are neighbours in the word: a degree written into the section's bits leaves the section \
         wrong, and a section overrunning its ten bits leaves the degree wrong, and neither is \
         visible from the field that moved"
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
        opacity: Opacity::OPAQUE,
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
        opacity: Opacity::OPAQUE,
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
