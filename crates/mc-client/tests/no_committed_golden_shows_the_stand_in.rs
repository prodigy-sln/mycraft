//! Whether the reference images this repository commits are photographs of the
//! defect.
//!
//! # A golden cannot judge this and never could
//!
//! `terrain_goldens` compares a frame against the image committed for it, so a
//! frame of the stand-in matched a golden of the stand-in and passed — for four
//! captures, across two spec cycles. The reference set is minted from the
//! renderer it grades, which is a closed loop with no outside reference anywhere
//! in it. This reading is the outside reference: it names three colours nothing
//! in the shipped art may produce and looks for them in the committed bytes,
//! without rendering anything.
//!
//! # Why the ids are read rather than listed
//!
//! `declared_capture_ids` is the authority on which golden directories may exist
//! (`golden_inventory` holds the set on disk to exactly that list), so the ids
//! come from it. A capture added at this revision is therefore judged here on
//! the day it is added, and a directory that has gone missing fails at the
//! decode rather than dropping quietly out of a filtered list.
//!
//! # Three colours, and the third is the other two after minification
//!
//! The stand-in checkerboards two texels one step either side of its mean. A face
//! far enough from the camera samples a smaller mip level, where the two have
//! averaged into the mean itself — so a frame can show the stand-in without
//! holding either texel colour, and the four committed captures hold 13 259 to
//! 19 596 pixels of exactly that.
//!
//! **Three exact colours, and the blends between them are not reached.** Sampling
//! interpolates between two mip levels, so a frame also carries a tail: the RGB
//! box the two texels span, `(140,38,131)` through `(160,58,151)`, minus the three
//! themselves — 10 293 of those at tick 0 and in the HUD capture, 9 512 at tick
//! 59, 7 036 at tick 119, reaching ten bytes from the mean. A tolerance would reach
//! them and would also start deciding how near a colour is allowed to be, so this
//! stays exact: the tail cannot exist without the three, and a regression that put
//! water back on the generator shows tens of thousands of each.

mod support;

use std::error::Error;
use std::path::PathBuf;

use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;
use mc_render::capture::{SCENE_REVISION, declared_capture_ids};
use mc_render::texture::placeholder::placeholder_texels;
use mc_testkit::frame::read_png;

use support::swatch::require;
use support::{TestResult, repository_root};

/// The key whose stand-in these three colours belong to.
const WATER: &str = "base:water";

/// The colours a frame drawing the generated stand-in for `base:water` holds:
/// the checkerboard's two texels, and the colour they minify to.
///
/// The first two are a pure function of the key and are checked against the
/// generator below, so they cannot go stale. The third is the mip chain's answer
/// for the pair and is stated rather than recomputed — recomputing it would call
/// the chain that produced the pixels being judged.
const THE_STAND_IN_ON_SCREEN: [[u8; 3]; 3] = [[140, 38, 131], [150, 49, 141], [160, 58, 151]];

/// The file each committed capture's reference image is written as.
const REFERENCE_IMAGE: &str = "default.png";

/// How many pixels of the stand-in one committed capture holds.
#[derive(Debug, PartialEq, Eq)]
struct Showing {
    capture: String,
    pixels: usize,
}

#[test]
fn no_committed_reference_image_holds_a_pixel_of_the_generated_stand_in() -> TestResult {
    require_the_literals_are_the_generators(&TextureKey::parse(WATER)?)?;
    let declared = declared_capture_ids(SCENE_REVISION)?;
    require(
        !declared.is_empty(),
        format!(
            "scene revision `{SCENE_REVISION}` declares no capture at all, so this reading would \
             be about no committed image"
        ),
    )?;

    let showing = stand_in_pixels_in_each(&declared)?;

    assert_eq!(
        showing,
        declared
            .iter()
            .map(|capture| Showing {
                capture: capture.clone(),
                pixels: 0,
            })
            .collect::<Vec<_>>(),
        "every committed reference image was a photograph of this defect: 77 987 pixels of these \
         three colours in each of the two tick-0 captures, 165 232 at tick 59 and 191 792 at tick \
         119, out of 1280 by 720 — 8.46% to 20.81% of the frame. Counting the trilinear blends \
         between them as well takes it to 88 280, 174 744 and 198 828, which is the 9.58% to \
         21.57% the spec's defect table reports; this scan names three exact colours and does not \
         reach the blends. A golden cannot report any of it, because the golden *is* the \
         photograph. A count above zero here is a capture that was not re-shot, or one re-shot \
         before the art was baked"
    );
    Ok(())
}

/// How many pixels of the stand-in each of `declared` holds, in the order they
/// are declared.
///
/// # Errors
///
/// Returns the decode failure for a capture whose reference image is missing or
/// unreadable — a vanished directory is a failure here rather than an absence
/// that reads as a clean result.
fn stand_in_pixels_in_each(declared: &[String]) -> Result<Vec<Showing>, Box<dyn Error>> {
    let root = golden_root()?;
    let mut showing = Vec::with_capacity(declared.len());
    for capture in declared {
        let image = read_png(&root.join(capture).join(REFERENCE_IMAGE))?;
        showing.push(Showing {
            capture: capture.clone(),
            pixels: image
                .as_bytes()
                .chunks_exact(4)
                .filter(|pixel| {
                    THE_STAND_IN_ON_SCREEN
                        .iter()
                        .any(|[red, green, blue]| pixel.starts_with(&[*red, *green, *blue]))
                })
                .count(),
        });
    }
    Ok(showing)
}

/// Where the committed reference images live.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located.
fn golden_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(repository_root()?
        .join("crates")
        .join("mc-render")
        .join("goldens"))
}

/// Fails unless the two texel colours stated above are the ones the generator
/// produces for `key`.
///
/// **The control on three literals.** They are a pure function of the key, so a
/// change to the generator would leave this reading looking for colours nothing
/// draws and passing over frames full of the ones it does.
fn require_the_literals_are_the_generators(key: &TextureKey) -> Result<(), Box<dyn Error>> {
    let mut generated: Vec<[u8; 3]> = placeholder_texels(key, TEXTURE_EDGE)
        .into_iter()
        .map(|[red, green, blue, _]| [red, green, blue])
        .collect();
    generated.sort_unstable();
    generated.dedup();
    let stated: Vec<[u8; 3]> = THE_STAND_IN_ON_SCREEN
        .iter()
        .copied()
        .filter(|color| generated.contains(color))
        .collect();
    require(
        stated == generated,
        format!(
            "this reading looks for the colours the generator produces for `{key}`, and it \
             produces {generated:?} where the three stated here are {THE_STAND_IN_ON_SCREEN:?}. \
             Two of the three have to be exactly the generator's pair or the scan is looking for \
             something nothing draws",
            key = key.as_str()
        ),
    )
}
