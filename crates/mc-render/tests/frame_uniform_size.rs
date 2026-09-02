//! How large the per-frame uniform buffer is allocated, against how large the
//! shader that reads it says the record is.
//!
//! # Not a table, and the difference is the whole value
//!
//! Every other cross-check in `build/validate_tables.rs` is a second copy of a
//! `mc-render` value, compared against the shader and closed by an agreement
//! test asserting the copy still equals the original. That arrangement cannot
//! work here, and adding a `FRAME_UNIFORM_BYTES` row to that table would have
//! been worse than nothing: the table is compared against the **shader**, never
//! against `buffers.rs`, so somebody who grows the record, updates both shaders
//! and the adjacent table, and forgets the constant three modules away gets an
//! undersized buffer and a green build. This project has already shipped a guard
//! asserting a constant equalled a copy of itself.
//!
//! What is compared below is the **production constant** against a size **naga
//! derives from the shader's own declaration**. Neither side is a copy of the
//! other and no third statement of the number exists to drift.
//!
//! # Green on the record as it stands, which is the point of writing it now
//!
//! The record holds a matrix and six planes today — `64 + 96` — and this test
//! passes on it. It is here *before* the record grows so that it guards the
//! growth rather than arriving with it: whoever appends an eye position, a reach
//! and a colour to the shader has to move the constant in the same commit or
//! this reddens. That is Invariant 5 read literally, and it is the reason the
//! check is worth more before the feature than after it.
//!
//! **Nothing here states `160`.** A test asserting today's number would have to
//! be edited by the person growing the record, which is exactly the edit that
//! would hide the defect — the number is not the subject, the agreement is.
//!
//! # Why the size is the right question, and what it still cannot see
//!
//! `min_binding_size: None` is what the binding declares, so an undersized
//! buffer is caught nowhere at all: the stage reads past the end and gets
//! whatever the driver leaves there. The size is what this file watches.
//!
//! It sees the size and **not the order**. A record whose fields are transposed
//! is exactly as large as a correct one, and that is
//! `shader_frame_record.rs`'s subject rather than this file's. The two are
//! deliberately separate: one asks whether the buffer is big enough, the other
//! whether the bytes in it mean what the shader thinks.
//!
//! # A target of its own, and what that does and does not buy
//!
//! `FRAME_UNIFORM_BYTES` lives behind the `gpu` feature, so this is declared
//! with `required-features` rather than guarded by a `cfg` inside a file that
//! compiles either way. What that buys is granularity: the two readings here
//! disappear as a **binary**, where a `cfg` inside `shader_frame_record.rs`
//! would have taken one of that file's four tests out of a gpu-free run while
//! the file went on reporting the other three as a complete pass.
//!
//! **It does not announce itself, and the neighbouring comment in `Cargo.toml`
//! claims it does.** Measured: `cargo nextest run -p mc-render
//! --no-default-features` reports `118 tests run: 118 passed, 0 skipped` and
//! names neither this target nor `pass_format`, and `cargo nextest list` does
//! not name them either. So a gpu-free run is silent about both. That is worth
//! knowing rather than assuming — the whole reason to prefer a missing binary to
//! a missing test is that the absence is coarse enough to notice, not that
//! anything reports it.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use mc_render::gpu::FRAME_UNIFORM_BYTES;
use naga::TypeInner;

// The build's own validator, included exactly as `build.rs` includes it, for the
// one table this file reads. Each test binary that links it uses a subset.
#[allow(dead_code)]
#[path = "../build/validate.rs"]
mod validate;

use validate::SECTION_RECORD;

type TestResult = Result<(), Box<dyn Error>>;

/// The shader whose declaration of the record is the one the buffer is sized
/// for.
///
/// Terrain rather than cull, because terrain reads the whole record and cull
/// reads a prefix of it — so terrain's is the longer of the two and the one an
/// allocation has to cover.
const TERRAIN_SHADER: &str = "terrain.wgsl";

/// The struct the per-frame uniform buffer carries.
const FRAME_RECORD: &str = "Frame";

/// The struct the section table carries, which the control below reads.
const SECTION_RECORD_NAME: &str = "Section";

/// How wide every scalar of the section record is, so its declared size is a
/// figure this file can state without copying one.
const BYTES_PER_SCALAR: u64 = 4;

/// The shipped shader directory, resolved from this crate's own manifest.
fn shipped_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders")
}

/// The size, in bytes, that `shader` declares for the struct named `record` —
/// naga's own answer, derived from the declaration by the same front end wgpu
/// hands the source to.
///
/// **Errors rather than answering zero for a record it cannot find.** A lookup
/// that returned a number regardless is the one defect this file could not
/// otherwise report: it would agree with the constant today and go on agreeing
/// after the record grew, which is precisely the moment the check exists for.
///
/// # Errors
///
/// Returns an error if the shader cannot be read, does not parse, declares no
/// such name, or declares it as something other than a struct.
fn declared_size_of(shader: &str, record: &str) -> Result<u64, Box<dyn Error>> {
    let source = fs::read_to_string(shipped_directory().join(shader))?;
    let module = naga::front::wgsl::parse_str(&source)
        .map_err(|error| format!("{shader} does not parse: {error}"))?;
    let found = module
        .types
        .iter()
        .find(|(_, held)| held.name.as_deref() == Some(record));
    let Some((_, held)) = found else {
        return Err(format!("{shader} declares no type named `{record}`").into());
    };
    match held.inner {
        TypeInner::Struct { span, .. } => Ok(u64::from(span)),
        ref other => Err(format!("`{record}` in {shader} is a {other:?} and not a struct").into()),
    }
}

#[test]
fn the_frame_uniform_is_allocated_at_the_size_the_shader_declares_for_the_record() -> TestResult {
    assert_eq!(
        FRAME_UNIFORM_BYTES,
        declared_size_of(TERRAIN_SHADER, FRAME_RECORD)?,
        "the buffer is allocated at the constant and the stage reads at the declaration, and \
         nothing between them checks that the two agree: the binding declares \
         `min_binding_size: None`, so a buffer too small for what the shader reads is caught at \
         no layer — the stage reads past the end and gets whatever the driver left there. The \
         two sides share no code and no third statement of the number exists, so this cannot \
         be satisfied by a copy agreeing with itself. **Neither side is written down here**, \
         which is what makes it guard a record that grows rather than record one that has: \
         appending a field to the shader without moving the constant reddens this, and so does \
         moving the constant without the shader"
    );
    Ok(())
}

#[test]
fn the_same_reading_gives_the_section_record_its_own_declared_size() -> TestResult {
    let stride = SECTION_RECORD.len() as u64 * BYTES_PER_SCALAR;

    assert_eq!(
        declared_size_of(TERRAIN_SHADER, SECTION_RECORD_NAME)?,
        stride,
        "the control on the reading above, and it answers the one question that reading cannot \
         ask of itself: whether the size comes out of the declaration at all. A lookup that had \
         come to return a fixed number would agree with the frame constant today and go on \
         agreeing after the record grew — silently, at exactly the moment it was written for. \
         This asks the same function for a **different** struct in the **same** shader and \
         requires a different answer, one derived from a field list rather than stated: twelve \
         four-byte scalars. A reading that discriminates by name and reports a size that \
         follows the declaration is the only thing that satisfies both"
    );
    Ok(())
}
