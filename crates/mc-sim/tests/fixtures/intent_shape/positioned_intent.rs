//! A fixture, not production code, and never compiled.
//!
//! It is a `.rs` file under `tests/fixtures/`, which cargo builds no target
//! from — only `tests/*.rs` and `tests/*/main.rs` become test binaries — so this
//! file exists solely to be *read* by the scan in `tests/intent_shape.rs`.
//!
//! It is the positive control for that scan, and it is shaped to fail it in both
//! directions at once. `MovementIntent` here declares a field naming a position
//! among otherwise innocent ones, so a scan that reported nothing is caught. And
//! `PlayerState` beside it declares a position the scan must pass over, because
//! the simulation's own state legitimately has one — so a scan that merely
//! searched the file's text for the word, and would therefore report the real
//! module for its `PlayerState`, is caught as well.

pub struct PlayerState {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

pub struct MovementIntent {
    pub forward: f32,
    pub strafe: f32,
    pub position: [f32; 3],
    pub yaw_delta: f32,
    pub pitch_delta: f32,
    pub jump: bool,
}
