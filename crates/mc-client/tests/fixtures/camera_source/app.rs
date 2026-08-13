//! A client frame path that builds its camera from what the simulation
//! published — the legitimate call site, spelled the way the real one is.
//!
//! This file is a fixture and is never compiled. It exists so that the scan in
//! `tests/camera_source.rs` can be pointed at a tree that does name the
//! renderer's camera constructor, in a file with the same name as the real
//! crate's one call site, and be seen to report it.

use mc_render::camera::camera_view;

fn frame_camera(published: &SimSnapshot) -> CameraView {
    camera_view(published.camera.eye, published.camera.target)
}
