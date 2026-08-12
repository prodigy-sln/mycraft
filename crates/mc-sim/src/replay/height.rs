//! The replay's surface, as interpolated value noise over a fixed lattice.
//!
//! **Spatial coherence is construction here, not luck.** Per-column white noise
//! satisfies every assertion anybody writes while describing the opposite
//! terrain — this project has the scar — so the bound the scenarios assert is
//! derived from the shape of the field rather than measured off a run:
//!
//! random values on a lattice of period 16, interpolated with classic
//! smoothstep (`3t² − 2t³`, maximum derivative 1.5) over an amplitude of 16
//! blocks, gives a maximum field slope of `1.5 × 16 / 16 = 1.5` blocks per
//! block. Two adjacent integer heights therefore differ by at most 2, which is
//! the bound FR-6.1's coherence scenario states.
//!
//! The amplitude may be lowered without redoing that arithmetic. **The lattice
//! period may not** — shortening it raises the slope proportionally, and the
//! bound goes with it.

use crate::replay::world::FOOTPRINT;

/// The lowest and highest surface the replay declares.
pub const LOWEST_SURFACE: u32 = 32;
pub const HIGHEST_SURFACE: u32 = 48;

/// How many blocks separate the lowest surface from the highest.
const AMPLITUDE: f32 = (HIGHEST_SURFACE - LOWEST_SURFACE) as f32;

/// How far apart the lattice's values sit, in blocks.
///
/// A power of two so that splitting a coordinate into a lattice cell and a
/// position inside it is a shift and a mask rather than a division.
const LATTICE_PERIOD: u32 = 16;
const LATTICE_SHIFT: u32 = LATTICE_PERIOD.trailing_zeros();
const LATTICE_MASK: u32 = LATTICE_PERIOD - 1;

const _: () = assert!(1 << LATTICE_SHIFT == LATTICE_PERIOD);

/// The surface height of every block column of the footprint, x fastest.
#[must_use]
pub fn heightmap(seed: u64) -> Vec<u32> {
    let mut heights = Vec::with_capacity((FOOTPRINT * FOOTPRINT) as usize);
    for z in 0..FOOTPRINT {
        for x in 0..FOOTPRINT {
            heights.push(surface_at(seed, x, z));
        }
    }
    heights
}

/// The surface height at one block column.
fn surface_at(seed: u64, x: u32, z: u32) -> u32 {
    let (cell_x, across) = lattice_cell(x);
    let (cell_z, along) = lattice_cell(z);

    let nearer = eased(
        lattice_value(seed, cell_x, cell_z),
        lattice_value(seed, cell_x + 1, cell_z),
        across,
    );
    let further = eased(
        lattice_value(seed, cell_x, cell_z + 1),
        lattice_value(seed, cell_x + 1, cell_z + 1),
        across,
    );

    LOWEST_SURFACE + (eased(nearer, further, along) * AMPLITUDE).round() as u32
}

/// Which lattice cell a coordinate falls in, and how far across it sits.
const fn lattice_cell(coordinate: u32) -> (u32, u32) {
    (coordinate >> LATTICE_SHIFT, coordinate & LATTICE_MASK)
}

/// `from` and `to` blended by `steps` of the way across a lattice cell, eased.
///
/// Smoothstep rather than a straight line: a linear blend leaves a visible
/// crease along every lattice boundary, where the slope changes discontinuously.
/// Its maximum derivative of 1.5 is the number the coherence bound is derived
/// from.
fn eased(from: f32, to: f32, steps: u32) -> f32 {
    let across = steps as f32 / LATTICE_PERIOD as f32;
    let weight = across * across * (3.0 - 2.0 * across);
    from + (to - from) * weight
}

/// The value the lattice holds at one of its points, in `0.0..1.0`.
///
/// A hash rather than a generator: the value at a point has to be the same
/// whichever neighbour asks for it, and asking twice must not advance anything.
fn lattice_value(seed: u64, lattice_x: u32, lattice_z: u32) -> f32 {
    let scattered = scatter(
        seed ^ u64::from(lattice_x)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(u64::from(lattice_z).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)),
    );
    // The top 24 bits: an f32 carries 24 bits of mantissa, so this is every bit
    // of the result the value can actually hold, and the division is exact.
    (scattered >> 40) as f32 / (1u64 << 24) as f32
}

/// One round of avalanche mixing, so that neighbouring inputs — and seeds one
/// apart — produce unrelated outputs.
const fn scatter(mut state: u64) -> u64 {
    state = (state ^ (state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    state = (state ^ (state >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    state ^ (state >> 31)
}
