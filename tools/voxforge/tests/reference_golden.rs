//! The one picture a human has actually looked at.
//!
//! Architecture D12, which carries no scenario: the spec states none for it.
//!
//! **What this golden is evidence of, and what it is not.** It is evidence that
//! today's render still produces the sheet a reviewer signed off on — nothing
//! more. It is **not** evidence that the orientation convention is correct. If
//! the convention table were wrong, the layer mapping and the raycaster would be
//! wrong together, the reviewer would have signed a wrong picture, and this test
//! would then defend that wrong picture forever, in perfect health.
//!
//! That is not a flaw in the design, it is the design: a golden the same code
//! produced is circular unless something outside the code makes it
//! authoritative, and here that something is a human signature recorded in the
//! spec folder. The signature is the authority; this test is only the thing that
//! notices when the code stops agreeing with it. Which is why re-shooting the
//! golden is never a repair for a red run — it is a request for a new signature.
//!
//! **Rendered at 24 pixels per voxel, and that figure is not free choice.** It
//! is the scale the reviewer actually looked at. A golden shot at the tool's
//! default 8 would be a differently-scaled cousin of the thing that was signed,
//! and the point of the artifact is that it is *the* thing.
//!
//! **Compared through ΔE, never through bytes.** `image`'s PNG encoder may
//! change its output bytes across a version bump — compression level, filter
//! choice, chunk layout — none of which changes a single pixel. A byte
//! comparison would go red for a reason that has nothing to do with this crate.
//! The tolerance is nevertheless **zero**: PNG is lossless and this renderer is
//! integer-valued and deterministic, so an unchanged renderer reproduces the
//! golden exactly. Zero-with-a-decode is a different and much stronger thing
//! than byte equality — it survives a re-encode and fails a re-render.
//!
//! **Alpha is compared separately, because ΔE cannot see it.** `compare`'s own
//! documentation says so: "alpha is not part of the metric, and is asserted on
//! the capture side instead". That matters more here than in most places — most
//! of a contact sheet is transparent background, so a build that filled the gaps
//! with opaque black would move no colour at all and pass a ΔE comparison of any
//! strictness. This test does the capture-side half itself.

mod common;

use std::error::Error;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use common::TestResult;
use mc_testkit::frame::{Rgba8Image, Thresholds, Verdict, compare, read_png};
use voxforge::format::load_document;
use voxforge::material::load_materials;
use voxforge::render::{Preview, contact_sheet};
use voxforge::volume::{StateSelection, assemble};

/// The scale the reviewer looked at, and so the only scale this golden has any
/// authority at.
const SIGNED_OFF_SCALE: NonZeroU32 = match NonZeroU32::new(24) {
    Some(scale) => scale,
    None => NonZeroU32::MIN,
};

/// Exact colour, with no area budget and no ceiling above it.
///
/// Every one of `compare`'s thresholds is strictly-greater-than, so a frame that
/// matches exactly passes all three at zero. See the note above for why exact is
/// the right strictness here and why it is still not a byte comparison.
fn exactly() -> Result<Thresholds, Box<dyn Error>> {
    Ok(Thresholds::new(0.0, 0.0, 0.0)?)
}

/// A path inside the repository, from this crate's own manifest.
fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// How today's sheet compares with the one that was signed off.
#[derive(Debug, PartialEq)]
enum Golden {
    /// The same picture, colour for colour and pixel for pixel.
    AsSigned,
    /// The colours moved.
    ColourDrift {
        /// What the comparison concluded.
        verdict: Verdict,
        /// How many pixels drifted.
        failing_pixels: u64,
        /// The furthest any one of them went.
        max_delta_e: f64,
    },
    /// A pixel's opacity moved, which the colour metric is blind to.
    OpacityMoved {
        /// Where it sits.
        column: u32,
        /// Which row.
        row: u32,
        /// What the signed sheet holds there.
        signed: u8,
        /// What today's render holds.
        rendered: u8,
    },
    /// The two are different sizes, so nothing was compared pixel for pixel.
    Resized {
        /// The signed sheet's size.
        signed: (u32, u32),
        /// Today's.
        rendered: (u32, u32),
    },
}

/// The reference model's contact sheet, as this build renders it.
///
/// # Errors
///
/// Returns the refusal when the document, its materials or its assembly is not
/// accepted — each of which would otherwise surface as an unexplained absence of
/// a picture.
fn rendered_sheet() -> Result<Preview, Box<dyn Error>> {
    let model = load_document(&repository_path(
        "content/base/models/reference-asymmetric.mcvox",
    ))?;
    let materials = load_materials(&repository_path("content/base/materials"))?;
    model.bind_materials(&materials)?;
    let volume = assemble(&model, &StateSelection::default())?;
    Ok(contact_sheet(&volume, &materials, SIGNED_OFF_SCALE)
        .image()
        .clone())
}

/// A preview as the comparison harness's own image type.
///
/// A free function rather than the `impl From<&Preview> for Rgba8Image` the
/// architecture asks for, and the reason is the orphan rule rather than
/// preference: an integration test is its own crate, so `Preview` and
/// `Rgba8Image` are **both** foreign here and that impl cannot be written in
/// this directory at all. What M4 is actually protecting is untouched — the
/// conversion is dev-only, it lives in `tests/`, and `Preview` remains
/// VoxForge's own type with the harness nowhere in the runtime graph.
///
/// # Errors
///
/// Returns an error when the pixels do not fill the frame the preview declares.
fn as_frame(preview: &Preview) -> Result<Rgba8Image, Box<dyn Error>> {
    let mut pixels = Vec::new();
    for row in 0..preview.height() {
        for column in 0..preview.width() {
            let pixel = preview
                .pixel(column, row)
                .ok_or_else(|| format!("the preview has no pixel at column {column}, row {row}"))?;
            pixels.extend_from_slice(&[pixel.red, pixel.green, pixel.blue, pixel.alpha]);
        }
    }
    Ok(Rgba8Image::from_rgba(
        preview.width(),
        preview.height(),
        pixels,
    )?)
}

/// The first pixel whose opacity differs between the two frames.
fn opacity_moved(signed: &Rgba8Image, rendered: &Rgba8Image) -> Option<Golden> {
    (0..signed.height())
        .flat_map(|row| (0..signed.width()).map(move |column| (column, row)))
        .find_map(|(column, row)| {
            let here = signed.pixel(column, row)?;
            let there = rendered.pixel(column, row)?;
            let (here, there) = (*here.get(3)?, *there.get(3)?);
            (here != there).then_some(Golden::OpacityMoved {
                column,
                row,
                signed: here,
                rendered: there,
            })
        })
}

/// How today's sheet compares with the committed one.
///
/// # Errors
///
/// Returns an error when the golden cannot be read or the sheet cannot be
/// rendered — neither of which may be reported as a match.
fn against_the_signed_sheet() -> Result<Golden, Box<dyn Error>> {
    let signed = read_png(&repository_path(
        "tools/voxforge/tests/goldens/reference-asymmetric-sheet.png",
    ))?;
    let rendered = as_frame(&rendered_sheet()?)?;

    let comparison = compare(&signed, &rendered, &exactly()?);
    if let Verdict::Mismatch(_) = comparison.verdict {
        if signed.width() != rendered.width() || signed.height() != rendered.height() {
            return Ok(Golden::Resized {
                signed: (signed.width(), signed.height()),
                rendered: (rendered.width(), rendered.height()),
            });
        }
        return Ok(Golden::ColourDrift {
            verdict: comparison.verdict,
            failing_pixels: comparison.failing_pixels,
            max_delta_e: comparison.max_delta_e,
        });
    }
    Ok(opacity_moved(&signed, &rendered).unwrap_or(Golden::AsSigned))
}

#[test]
fn the_reference_models_contact_sheet_still_matches_the_one_that_was_signed_off() -> TestResult {
    assert_eq!(
        against_the_signed_sheet()?,
        Golden::AsSigned,
        "the committed sheet is the picture a human passed, so a difference here is this build disagreeing with the only check in this spec that does not descend from its own convention table — re-shooting the golden is a request for a new signature, never a repair"
    );
    Ok(())
}
