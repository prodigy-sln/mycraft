//! The mismatch report: what an agent reads instead of looking at a screen.
//!
//! The report is machine-readable on purpose. Its whole job is to let whoever
//! finds a red golden test answer "what drifted, by how much, against which
//! thresholds, on which adapter" without re-running anything, so the fields
//! below are a contract rather than a convenience.
//!
//! The fixtures here are uniform, which the row-asymmetry rule permits by
//! exemption: nothing in this file asserts image *content*. The frames exist
//! only to produce a comparison with a known failing-pixel count, and every
//! assertion is against a JSON field.

mod common;

use common::{TestResult, grey, uniform, with_leading_pixels};
use mc_testkit::frame::{
    AdapterProvenance, Backend, CaptureId, FrameReport, Thresholds, compare, write_report,
};
use serde_json::Value;
use tempfile::TempDir;

const CAPTURE: &str = "area-budget-320x180";
const WIDE: u32 = 320;
const TALL: u32 = 180;
const BASELINE: u8 = 128;
/// Twelve levels away is a distance of about 4.67 — over the default per-pixel
/// tolerance, under the ceiling.
const TWELVE_LEVELS: u8 = 140;
/// Six of 57 600 pixels is 0.0104%, just past the 0.01% area budget, so this
/// pair really is a mismatch and really does have a report to write.
const FAILING_PIXELS: u64 = 6;
const ADAPTER: &str = "NVIDIA GeForce RTX 4090";
const DRIVER: &str = "566.36";

/// Every field the provenance block promises, as JSON pointers.
const PROVENANCE_FIELDS: [&str; 8] = [
    "/adapter/name",
    "/adapter/backend",
    "/adapter/driver_description",
    "/thresholds/per_pixel_delta_e",
    "/thresholds/max_failing_fraction",
    "/thresholds/hard_ceiling_delta_e",
    "/failing_pixels",
    "/max_delta_e",
];

fn report_from(provenance: &AdapterProvenance) -> Result<FrameReport, Box<dyn std::error::Error>> {
    let expected = uniform(WIDE, TALL, grey(BASELINE))?;
    let actual = with_leading_pixels(&expected, grey(TWELVE_LEVELS), FAILING_PIXELS as usize)?;
    let comparison = compare(&expected, &actual, &Thresholds::default());

    Ok(FrameReport::new(
        &CaptureId::new(CAPTURE)?,
        &comparison,
        provenance,
        None,
    ))
}

fn identified_adapter() -> AdapterProvenance {
    AdapterProvenance::new(ADAPTER, Backend::Vulkan, Some(DRIVER))
}

#[test]
fn a_written_mismatch_report_parses_as_json_whose_failing_pixel_count_is_a_number() -> TestResult {
    let workspace = TempDir::new()?;
    let path = workspace.path().join("report.json");

    write_report(&report_from(&identified_adapter())?, &path)?;

    let document: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    assert_eq!(
        document.get("failing_pixels").and_then(Value::as_u64),
        Some(FAILING_PIXELS),
        "the failing-pixel count must be readable as a number, not parsed out \
         of prose"
    );
    Ok(())
}

#[test]
fn a_mismatch_report_records_the_environment_and_the_thresholds_that_judged_it() -> TestResult {
    let document: Value = serde_json::from_str(&report_from(&identified_adapter())?.to_json()?)?;

    let missing: Vec<&str> = PROVENANCE_FIELDS
        .iter()
        .filter(|pointer| document.pointer(pointer).is_none())
        .copied()
        .collect();

    assert!(
        missing.is_empty(),
        "a verdict is only reproducible if the report says what produced it; \
         missing {missing:?}"
    );
    Ok(())
}

#[test]
fn an_adapter_reporting_no_driver_description_records_the_field_as_unknown() -> TestResult {
    let anonymous = AdapterProvenance::new(ADAPTER, Backend::Vulkan, None);

    let document: Value = serde_json::from_str(&report_from(&anonymous)?.to_json()?)?;

    assert_eq!(
        document
            .pointer("/adapter/driver_description")
            .and_then(Value::as_str),
        Some("unknown"),
        "an absent driver description is recorded as `unknown`; omitting the \
         field would make a reader guess whether it was missing or unread"
    );
    Ok(())
}
