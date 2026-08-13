//! A second camera, invented beside the published one — the defect the scan
//! exists to catch.
//!
//! This file is a fixture and is never compiled. Nothing authoritative ever
//! agreed to the viewpoint below: it is a pose written down in the client, which
//! is exactly the policy this crate is excluded from coverage for not holding.
//! A frame drawn through it looks like a frame, so no picture can tell.

use mc_render::camera::camera_view;

const OVERVIEW_EYE: [f32; 3] = [32.0, 96.0, 32.0];
const OVERVIEW_TARGET: [f32; 3] = [32.0, 40.0, 32.0];

fn overview_camera() -> CameraView {
    camera_view(OVERVIEW_EYE, OVERVIEW_TARGET)
}
