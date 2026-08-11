//! Row-padding arithmetic for texture-to-buffer copies.
//!
//! A copy's rows are aligned to 256 bytes, so a frame whose width defeats that
//! alignment comes back with filler on every row. 257 × 129 is the shape that
//! defeats it in both directions: 1028 content bytes padded to 1280, 129 times
//! over. Proving this here means the first device-backed capture is testing the
//! device, not the arithmetic.

use super::{ReadbackError, padded_row_bytes, unpad_rows};

const WIDTH: u32 = 257;
const HEIGHT: u32 = 129;
const ROW_BYTES: usize = 1028;
const PADDED_ROW_BYTES: usize = 1280;
/// A byte that must never survive into the unpadded output.
const FILLER: u8 = 0xAB;

/// One marker byte per row, so a dropped, duplicated or reordered row is
/// visible in the output rather than hidden by uniform content.
fn row_marker(row: u32) -> u8 {
    u8::try_from(row % 251).unwrap_or(0)
}

fn padded_rows() -> Vec<u8> {
    let mut buffer = Vec::with_capacity(PADDED_ROW_BYTES * HEIGHT as usize);
    for row in 0..HEIGHT {
        buffer.extend(std::iter::repeat_n(row_marker(row), ROW_BYTES));
        buffer.extend(std::iter::repeat_n(FILLER, PADDED_ROW_BYTES - ROW_BYTES));
    }
    buffer
}

fn expected_rows() -> Vec<u8> {
    let mut buffer = Vec::with_capacity(ROW_BYTES * HEIGHT as usize);
    for row in 0..HEIGHT {
        buffer.extend(std::iter::repeat_n(row_marker(row), ROW_BYTES));
    }
    buffer
}

#[test]
fn a_row_that_defeats_the_copy_alignment_is_padded_up_to_it() -> Result<(), ReadbackError> {
    assert_eq!(
        padded_row_bytes(WIDTH)?,
        1280,
        "1028 content bytes round up to the next 256-byte multiple"
    );
    Ok(())
}

#[test]
fn a_row_that_already_fills_the_copy_alignment_is_left_alone() -> Result<(), ReadbackError> {
    assert_eq!(
        padded_row_bytes(64)?,
        256,
        "256 content bytes are already aligned and gain no padding"
    );
    Ok(())
}

#[test]
fn a_width_whose_row_cannot_be_addressed_is_rejected() {
    assert!(
        padded_row_bytes(u32::MAX).is_err(),
        "a row of 4 × u32::MAX bytes has no representable padded length"
    );
}

#[test]
fn unpadding_strips_the_filler_from_every_row() -> Result<(), ReadbackError> {
    let unpadded = unpad_rows(&padded_rows(), ROW_BYTES, PADDED_ROW_BYTES, HEIGHT)?;

    assert_eq!(
        unpadded.len(),
        ROW_BYTES * HEIGHT as usize,
        "the output holds exactly the content bytes of every row"
    );
    assert!(
        unpadded == expected_rows(),
        "every row must come back in order with its filler removed"
    );
    Ok(())
}

#[test]
fn a_buffer_shorter_than_its_padded_rows_is_rejected() {
    let truncated = vec![0_u8; PADDED_ROW_BYTES * (HEIGHT as usize - 1)];

    assert!(
        unpad_rows(&truncated, ROW_BYTES, PADDED_ROW_BYTES, HEIGHT).is_err(),
        "a buffer that cannot hold every padded row is not a frame"
    );
}
