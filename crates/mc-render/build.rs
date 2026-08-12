//! Validates every shipped WGSL source before the crate is built.
//!
//! The validation itself lives in `build/validate.rs`, which
//! `tests/shader_validation.rs` includes the same way — one validator, exercised
//! by the tests exactly as the build runs it.

use std::path::Path;
use std::process::ExitCode;

#[path = "build/validate.rs"]
mod validate;

/// Where the shipped shaders live, relative to this package's root.
const SHADER_DIRECTORY: &str = "shaders";

fn main() -> ExitCode {
    println!("cargo::rerun-if-changed={SHADER_DIRECTORY}");
    println!("cargo::rerun-if-changed=build/validate.rs");

    match validate::validate_shader_directory(Path::new(SHADER_DIRECTORY)) {
        // Silence on success is deliberate: `cargo::warning` is the only channel
        // a build script has for saying something, and a line printed on every
        // build of every crate that depends on this one is how real warnings
        // stop being read.
        Ok(_) => ExitCode::SUCCESS,
        // `cargo::error` rather than a panic: `clippy::panic` is denied across
        // this workspace and a build script is not exempt from it, and the
        // message a developer has to act on reads better without a backtrace
        // wrapped round it.
        Err(error) => {
            println!("cargo::error={error}");
            ExitCode::FAILURE
        }
    }
}
