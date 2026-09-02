//! The per-frame uniform record, asked of the build's own validator rather than
//! of a copy of it.
//!
//! # Why this record needs a check at all, and needs one now
//!
//! `min_binding_size: None` is what the pipeline declares for this binding, and
//! it catches exactly one thing: a buffer too small for what the shader reads.
//! It cannot see a CPU that writes one field where the shader reads another. At
//! two fields — a matrix and six planes, of wildly different shapes — writing
//! them in the wrong order was very nearly inconceivable. The record is about to
//! carry an eye position, a reach and a colour, and at six fields a transposition
//! is a plausible wrong picture with **no error anywhere**: the frame draws,
//! every test that reads a mean colour still reads one, and what the picture
//! means is silently different.
//!
//! Invariant 5 is that verification arrives before the thing it verifies, so
//! this lands while the record still has two fields and is checked green on
//! them.
//!
//! # Two shaders declare it and neither can check itself
//!
//! Both stages bind the same buffer. The cull stage reads the view projection
//! and the planes; the terrain stage reads those and whatever is appended after
//! them. So cull's declaration has to be a **valid prefix** of terrain's — equal
//! for as far as it goes, and never longer, never differently ordered. That is a
//! wholly new invariant, introduced with the record's growth, and no shader
//! compiler and no runtime binding check can see it violated: two structs whose
//! fields diverge after the first four both compile, both bind, and each reads
//! the other's bytes as its own.
//!
//! # Driven through the validator, never re-implemented beside it
//!
//! This file includes `build/validate.rs` through `#[path]`, which is the file
//! `build.rs` includes. Every reading below hands the validator a directory and
//! reads its verdict, so there is nothing here that could agree with a table
//! while the build disagrees with the shaders. The doctored sources are written
//! from the shipped ones, so a shipped `Frame` that grows arrives in the fixture
//! automatically and the two defects stay defects: **a pair of fields exchanged**
//! is wrong at any length, and **a field cull declares that terrain does not** is
//! wrong however long terrain becomes.
//!
//! # An enumerated verdict, and a control on it
//!
//! A reading that only asked whether a doctored directory was refused could not
//! tell a validator that refuses everything from one that looks at this record,
//! so the shipped directory is read in the same shape and has to come back
//! accepted. The refusal arm carries which shader the message attributes the
//! fault to, because a refusal that names neither file leaves whoever hits it at
//! the build with two shaders to read and no reason to start with either.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

// The build's own validator, included exactly as `build.rs` includes it. Each
// test binary that links it uses a subset of what it exports.
#[allow(dead_code)]
#[path = "../build/validate.rs"]
mod validate;

use validate::validate_shader_directory;

type TestResult = Result<(), Box<dyn Error>>;

/// The two shaders that declare the per-frame record.
const TERRAIN_SHADER: &str = "terrain.wgsl";
const CULL_SHADER: &str = "cull.wgsl";

/// How the record's declaration opens, in both shaders.
const FRAME_DECLARATION: &str = "struct Frame {";

/// How a struct declaration closes in both of them.
const DECLARATION_CLOSES: &str = "};";

/// The record's two fields with the first two exchanged.
///
/// **Wrong at any length**, which is what makes it the right doctoring for a
/// record that is about to grow: the order is what carries the offsets, so a
/// pair of fields swapped reads every byte of one out of the other and compiles
/// perfectly. Written out rather than derived from the shipped text, because a
/// transformation that reorders whatever it finds would silently become a no-op
/// on a record whose first two fields are the same shape.
const A_FRAME_WITH_ITS_FIRST_TWO_FIELDS_EXCHANGED: &str = "\
    planes: array<vec4<f32>, 6>,\n\
    view_projection: mat4x4<f32>,\n";

/// The record's two fields with a third appended that terrain does not declare.
///
/// The prefix invariant broken in the direction only this file can see: cull
/// reading a field past the end of what terrain writes. **Wrong however long
/// terrain becomes**, because the appended name is one no stage has any use for.
const A_CULL_FRAME_REACHING_PAST_TERRAINS: &str = "\
    view_projection: mat4x4<f32>,\n\
    planes: array<vec4<f32>, 6>,\n\
    tail: f32,\n";

/// What the validator said about a directory of shaders.
///
/// **Three arms, and the accepting one is not the absence of the others.** A
/// validator that had come to refuse every directory would satisfy a reading
/// that only asked whether the doctored ones were turned away; a validator that
/// looks at nothing satisfies one that only asks whether the shipped ones pass.
/// Naming the shader a refusal blames is the third fact, and it is the one
/// whoever hits this at the build actually needs.
#[derive(Debug, PartialEq, Eq)]
enum WhatTheValidatorSaid {
    /// Every source in the directory was accepted.
    EveryShaderAccepted,
    /// Refused, and the message names this shader.
    RefusedNaming(String),
    /// Refused, and the message names neither shader that declares the record.
    RefusedNamingNeitherShader(String),
}

/// The shipped shader directory, resolved from this crate's own manifest.
fn shipped_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders")
}

/// `source` with the body of its `Frame` declaration replaced by `fields`.
///
/// # Errors
///
/// Returns an error if the source declares no such record, or if the
/// declaration is not closed — either of which means the fixture is describing a
/// shader that no longer exists and must refuse rather than doctor nothing.
fn with_a_frame_of(source: &str, fields: &str) -> Result<String, Box<dyn Error>> {
    let opens = source
        .find(FRAME_DECLARATION)
        .ok_or("this shader declares no `Frame` record for the fixture to doctor")?;
    let body = opens + FRAME_DECLARATION.len();
    let closes = source[body..]
        .find(DECLARATION_CLOSES)
        .ok_or("this shader's `Frame` declaration is not closed")?
        + body;
    Ok(format!(
        "{}{FRAME_DECLARATION}\n{fields}{}",
        &source[..opens],
        &source[closes..]
    ))
}

/// A copy of the shipped shader directory in which `doctored` carries a `Frame`
/// of `fields`.
///
/// Every other source is copied unchanged, so the directory is the shipped set
/// with one defect in it — a fixture holding the doctored shader alone would be
/// refused for the shader it is missing rather than for the record it states.
///
/// # Errors
///
/// Returns an error if the shipped directory cannot be read or the copy cannot
/// be written.
fn shipped_but_for(doctored: &str, fields: &str) -> Result<TempDir, Box<dyn Error>> {
    let directory = TempDir::new()?;
    for entry in fs::read_dir(shipped_directory())? {
        let path = entry?.path();
        let Some(name) = path.file_name() else {
            continue;
        };
        let source = fs::read_to_string(&path)?;
        let written = if name == doctored {
            with_a_frame_of(&source, fields)?
        } else {
            source
        };
        fs::write(directory.path().join(name), written)?;
    }
    Ok(directory)
}

/// What the validator said about the shaders in `directory`.
fn what_the_validator_said(directory: &Path) -> WhatTheValidatorSaid {
    let Err(refusal) = validate_shader_directory(directory) else {
        return WhatTheValidatorSaid::EveryShaderAccepted;
    };
    let refused = refusal.to_string();
    match [TERRAIN_SHADER, CULL_SHADER]
        .into_iter()
        .find(|shader| refused.contains(shader))
    {
        Some(shader) => WhatTheValidatorSaid::RefusedNaming(shader.to_owned()),
        None => WhatTheValidatorSaid::RefusedNamingNeitherShader(refused),
    }
}

#[test]
fn the_shipped_shaders_declare_the_record_the_build_expects_and_are_accepted() -> TestResult {
    assert_eq!(
        what_the_validator_said(&shipped_directory()),
        WhatTheValidatorSaid::EveryShaderAccepted,
        "the control on the two readings below, and it is not optional: a check on this record \
         that over-fires fails the build of a correct tree, and a check that refuses everything \
         satisfies every doctored reading there is. It is also the half that has to be green \
         **now**, while the record still holds a matrix and six planes — verification arriving \
         before the thing it verifies means the instrument is known good on the shape that \
         exists, so that when the record grows the only new thing is the record"
    );
    Ok(())
}

#[test]
fn a_terrain_frame_with_two_fields_exchanged_fails_the_build_naming_that_shader() -> TestResult {
    let doctored = shipped_but_for(TERRAIN_SHADER, A_FRAME_WITH_ITS_FIRST_TWO_FIELDS_EXCHANGED)?;

    assert_eq!(
        what_the_validator_said(doctored.path()),
        WhatTheValidatorSaid::RefusedNaming(TERRAIN_SHADER.to_owned()),
        "this shader compiles, binds, and draws a frame. The order of a uniform's fields is \
         what carries their offsets, so a pair exchanged makes the stage read every byte of one \
         out of the other — and the CPU that filled the buffer is perfectly correct, so there \
         is no error at any layer and nothing at runtime to catch. `min_binding_size` sees only \
         a buffer that is too small, which this one is not. At two fields of wildly different \
         shapes the mistake was hard to make; at six it is a plausible wrong picture with no \
         symptom, and the build is the only place it can be reported"
    );
    Ok(())
}

#[test]
fn a_cull_frame_declaring_a_field_terrain_does_not_fails_the_build_naming_that_shader() -> TestResult
{
    let doctored = shipped_but_for(CULL_SHADER, A_CULL_FRAME_REACHING_PAST_TERRAINS)?;

    assert_eq!(
        what_the_validator_said(doctored.path()),
        WhatTheValidatorSaid::RefusedNaming(CULL_SHADER.to_owned()),
        "both stages bind one buffer, so cull's declaration has to be a valid prefix of \
         terrain's — equal for as far as it goes and never reaching past it. This one reaches \
         past, which means the cull stage reads bytes the CPU never wrote for it and decides \
         which quads are drawn on them. It is a wholly new invariant, introduced by the record \
         growing on terrain's side only, and nothing else in the toolchain can see it: two \
         structs that diverge after their common fields both compile and both bind, and each \
         reads the other's bytes as its own"
    );
    Ok(())
}
