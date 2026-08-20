//! An image the set offers that no layer can be filled from, and the refusal it
//! becomes.
//!
//! # Defence in depth against a directory people can write into
//!
//! The built set is derived and is never committed, and the build refuses a
//! model whose scale and pixels-per-voxel do not come to a block texture's edge
//! — naming the *model*, which is what its author can fix. None of that is a
//! reason to trust the directory: it is an ordinary directory on somebody's
//! disk, and a set built by an older tool, a patched one, or a hand-edited one
//! is a set the client is handed all the same. Uploading a 32 × 32 image into a
//! 16 × 16 layer is a buffer overrun, and a file that is not a PNG is not one
//! that decodes to a smaller picture.
//!
//! # Both refusals name the key, and that is the whole point of them
//!
//! Whoever meets one of these has a *texture key* in a manifest and a *file* in
//! a directory, and only the key connects the file back to the declaration that
//! wanted it. A message about `base__stone.png` with no key in it hands a mod
//! author a filename they never typed.
//!
//! # The set stays current in both fixtures, deliberately
//!
//! An index is folded over the manifest, the models and the materials — never
//! over the images it names. So an image can be replaced without the set
//! becoming stale, which is the only way to reach these two readings at all: a
//! fixture that also moved the fold would be refused one step earlier, for
//! staleness, and would say nothing about the image.

mod support;

use mc_client::textures::{TextureSetError, built_set};

use support::{TestResult, built_sets, refusal_printed_over};

/// A 32 × 32 image committed beside this suite, and the file it is copied over.
///
/// Committed rather than written by the fixture: producing a PNG by hand needs a
/// deflate stream and four checksums, and a fixture that encoded one would be a
/// second encoder to keep correct. It sits under `tests/fixtures/`, which the
/// gate's committed-art stage does not look at — that stage is parameterised on
/// a content root, and this is not one.
const AN_IMAGE_TWICE_THE_EDGE: [&str; 4] = ["tests", "fixtures", "set", "thirty-two-square.png"];

/// The edge the image holds and the edge a layer holds, as the refusal has to
/// name them.
const OFFERED_EDGE: u32 = 32;
const DECLARED_EDGE: u32 = 16;

/// Bytes that are not a PNG and are not close to being one.
///
/// Text rather than a truncated PNG: a truncated one is a decoder question — how
/// much of a file is enough — and this reading is about a file that was never an
/// image at all, which is what a set assembled by hand actually contains.
const NOT_A_PNG: &[u8] = b"this file is not a PNG, and the client has to say so by name\n";

#[test]
fn an_image_larger_than_a_layer_refuses_the_launch_naming_the_key_and_both_edges() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    let oversized = std::fs::read(a_committed_fixture(&AN_IMAGE_TWICE_THE_EDGE)?)?;
    built_sets::with_one_image_replaced(root.path(), built_sets::A_RECORDED_IMAGE, &oversized)?;

    let read = built_set(root.path());

    let Err(TextureSetError::Size { key, found }) = read else {
        return Err(format!(
            "an image the array texture cannot hold is refused rather than uploaded, and the \
             client answered {read:?}"
        )
        .into());
    };
    let printed = refusal_printed_over(root.path())?;
    let named = [
        built_sets::THE_KEY_THAT_IMAGE_BELONGS_TO,
        &OFFERED_EDGE.to_string(),
        &DECLARED_EDGE.to_string(),
    ]
    .map(|token| printed.contains(token));
    assert_eq!(
        (key.as_str(), found, named),
        (
            built_sets::THE_KEY_THAT_IMAGE_BELONGS_TO,
            (OFFERED_EDGE, OFFERED_EDGE),
            [true; 3],
        ),
        "whoever meets this has one image to redraw and needs three things to do it: which key it \
         is the art for, what size it is, and what size a layer holds. It said: {printed}"
    );
    Ok(())
}

#[test]
fn an_image_that_is_not_a_png_refuses_the_launch_naming_the_key_and_the_file() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::with_one_image_replaced(root.path(), built_sets::A_RECORDED_IMAGE, NOT_A_PNG)?;

    let read = built_set(root.path());

    let Err(TextureSetError::NotAPng { key, image }) = read else {
        return Err(format!(
            "a file the decoder cannot read is refused by name rather than skipped, left \
             unsampled or turned into an empty layer. The client answered {read:?}"
        )
        .into());
    };
    let printed = refusal_printed_over(root.path())?;
    assert_eq!(
        (
            key.as_str(),
            image.file_name().and_then(|name| name.to_str()),
            printed.contains(built_sets::THE_KEY_THAT_IMAGE_BELONGS_TO),
            printed.contains(built_sets::A_RECORDED_IMAGE),
        ),
        (
            built_sets::THE_KEY_THAT_IMAGE_BELONGS_TO,
            Some(built_sets::A_RECORDED_IMAGE),
            true,
            true,
        ),
        "a file that will not decode is a file somebody has to go and look at, so the refusal \
         names it — and it names the key as well, because the file name is one nobody typed. It \
         said: {printed}"
    );
    Ok(())
}

/// A file committed beside this suite, located from the crate rather than from
/// wherever the test binary was started.
///
/// # Errors
///
/// Returns an error when the fixture is not there, which is a fixture that was
/// moved rather than a client that did anything.
fn a_committed_fixture(parts: &[&str]) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let at = parts.iter().fold(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        |at, part| at.join(part),
    );
    if !at.is_file() {
        return Err(format!(
            "this reading is about an image of a size no layer holds, and the committed one is \
             not at {}. What it would build is a root nobody described",
            at.display()
        )
        .into());
    }
    Ok(at)
}
