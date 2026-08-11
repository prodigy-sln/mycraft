//! Writing a captured frame to disk and reading it back.
//!
//! The on-disk assertion below is deliberately about named coordinates in the
//! decoded file rather than a buffer round trip: a symmetric pair of mistakes
//! on write and read cancels out, and the file is what the agent reading this
//! harness's output actually opens. It is split across rows because PNG is
//! row-ordered — a column split would be invariant under exactly the inversion
//! the assertion exists to catch.
//!
//! **Nothing from `frame::png` sits on the read side of that assertion.**
//! Decoding with `read_png` would make it a round trip again, and a round trip
//! is precisely what the compensating pair defeats: flip on write, flip on
//! read, and the buffer that comes back is the one that went in while the file
//! on disk is upside-down. The independent decode below is the only thing
//! standing between that bug and every golden this project commits.

mod common;

use std::error::Error;
use std::fs;
use std::path::Path;

use common::{TestResult, grey, split_by_row, uniform, with_leading_pixels};
use image::{ImageFormat, RgbaImage};
use mc_testkit::frame::{ImageIoError, read_png, write_png};
use tempfile::TempDir;

const EDGE: u32 = 64;
const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];
const MAGENTA: [u8; 3] = [255, 0, 255];
const OPAQUE_WHITE: [u8; 4] = [255, 255, 255, 255];
const OPAQUE_BLACK: [u8; 4] = [0, 0, 0, 255];
/// A column midway across the frame, away from either side edge.
const MIDDLE_COLUMN: u32 = 32;

#[test]
fn an_image_written_to_a_png_decodes_back_to_the_same_pixels() -> TestResult {
    let workspace = TempDir::new()?;
    let target = workspace.path().join("frame.png");
    // Asymmetric in both axes, so a transposed or flipped file is a different
    // buffer rather than the same one.
    let original = with_leading_pixels(&uniform(EDGE, EDGE, grey(128))?, MAGENTA, 12)?;

    write_png(&original, &target)?;
    let decoded = read_png(&target)?;

    assert_eq!(
        decoded.width(),
        original.width(),
        "the width survives the file"
    );
    assert_eq!(
        decoded.height(),
        original.height(),
        "the height survives the file"
    );
    assert!(
        decoded.as_bytes() == original.as_bytes(),
        "the decoded pixels must be identical to the written ones"
    );
    Ok(())
}

#[test]
fn a_written_png_keeps_the_white_half_at_the_top_where_it_was_drawn() -> TestResult {
    let workspace = TempDir::new()?;
    let target = workspace.path().join("split.png");
    // Split across rows, not columns: PNG is row-ordered, so an inverted file is
    // the failure this asserts against, and only a row split can see it.
    let original = split_by_row(EDGE, EDGE, WHITE, BLACK)?;

    write_png(&original, &target)?;
    let on_disk = decode_without_the_harness(&target)?;

    assert_eq!(
        on_disk
            .get_pixel_checked(MIDDLE_COLUMN, 0)
            .ok_or("the written file is missing its first row")?
            .0,
        OPAQUE_WHITE,
        "the first row of the file must be white"
    );
    assert_eq!(
        on_disk
            .get_pixel_checked(MIDDLE_COLUMN, EDGE - 1)
            .ok_or("the written file is missing its last row")?
            .0,
        OPAQUE_BLACK,
        "the last row of the file must be black"
    );
    Ok(())
}

/// Decodes `path` with no code from `frame::png` on the read side.
///
/// This is what makes the assertion above about the **file** rather than about
/// a round trip. `read_png` would cancel any flip `encode_png` introduced, so
/// putting it here would leave the one failure mode this test exists for
/// unobserved — which is what it did before.
///
/// # Errors
///
/// Returns the read or decode failure.
fn decode_without_the_harness(path: &Path) -> Result<RgbaImage, Box<dyn Error>> {
    let encoded = fs::read(path)?;
    Ok(image::load_from_memory_with_format(&encoded, ImageFormat::Png)?.to_rgba8())
}

#[test]
fn a_target_whose_directory_cannot_be_created_names_the_path_and_the_cause() -> TestResult {
    let workspace = TempDir::new()?;
    // A plain file standing where the target's directory would have to go: no
    // directory can be created there, on any platform.
    let blocker = workspace.path().join("occupied");
    fs::write(&blocker, b"a file, not a directory")?;
    let target = blocker.join("frame.png");

    let image = uniform(EDGE, EDGE, grey(128))?;
    let error = write_png(&image, &target)
        .err()
        .ok_or("writing beneath a file must fail")?;

    let ImageIoError::Directory { path, .. } = &error else {
        return Err(format!("expected a directory failure, got {error:?}").into());
    };
    assert!(
        path.starts_with(&blocker),
        "the failure must name the path it could not create, got {path:?}"
    );
    assert!(
        error.source().is_some(),
        "the failure must carry the underlying cause, got {error:?}"
    );
    Ok(())
}
