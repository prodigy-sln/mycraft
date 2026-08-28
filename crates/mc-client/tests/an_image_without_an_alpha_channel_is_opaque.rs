//! A baked image whose PNG carries three channels, and what the layer it fills
//! holds in the fourth.
//!
//! # Why this is owed the moment anything is drawn translucent
//!
//! Until a block declares an opacity, the alpha channel of an array layer is a
//! number nothing samples: the fragment stage returns the texel's colour and the
//! terrain pipeline does not blend, so a layer filled with alpha 0 and a layer
//! filled with alpha 255 draw the identical picture. The day a declared opacity
//! multiplies the texel's own alpha, the difference between those two is the
//! difference between a wall and a hole — and the source images this project
//! bakes are ordinary truecolour PNGs with no alpha channel at all.
//!
//! So the property is: **a source image that says nothing about transparency
//! means an opaque texture**, not a transparent one. It holds today, through
//! `image`'s `to_rgba8`, and this is the reading that says so out loud rather
//! than leaving it as a line in a decoder nobody has a reason to look at.
//!
//! # The fixture is committed, and its own header is what says it has no alpha
//!
//! Producing a PNG inside the test would mean encoding one with the same crate
//! that decodes it, which is agreement between two halves of one library rather
//! than a statement about a file somebody handed the client. So the image is
//! committed beside this suite, as `thirty-two-square.png` already is, and the
//! reading checks the **IHDR colour-type byte** for itself — byte 25 of any PNG,
//! `2` for truecolour and `6` for truecolour with alpha. That check shares no
//! code with the decoder, and without it a fixture silently re-saved with an
//! alpha channel would leave this reading green while asking nothing.
//!
//! # What the expectation is, and where it comes from
//!
//! The image is a stated rule rather than a snapshot: the texel at column `x`,
//! row `y` is `[x * 17, y * 17, 255 - x * 17]`, row-major, which is why the
//! expectation below is three lines of arithmetic and not two hundred and
//! fifty-six committed triples. Asserting the whole level rather than the alpha
//! channel alone costs nothing and buys the other half: a decoder that filled
//! alpha correctly while dropping the image and falling back to the generated
//! texture would satisfy every reading about alpha on its own.

mod support;

use std::error::Error;
use std::path::PathBuf;

use mc_core::content::TEXTURE_EDGE;
use mc_core::id::TextureKey;
use mc_render::texture::mip::levels_for;

use support::swatch::require;
use support::{TestResult, built_sets, prepare_scene_at};

/// The committed image this reading replaces the set's own with: 16 x 16,
/// truecolour, no alpha channel.
const AN_IMAGE_WITHOUT_AN_ALPHA_CHANNEL: &str = "sixteen-square-without-an-alpha-channel.png";

/// Where in a PNG the colour type stands, and the two values that matter.
///
/// The signature is eight bytes, the IHDR length and type are eight more, then
/// width and height are four each and the bit depth is one — so the colour type
/// is byte 25. `2` is truecolour and `6` is truecolour with an alpha channel.
const COLOUR_TYPE_BYTE: usize = 25;
const TRUECOLOUR_WITHOUT_ALPHA: u8 = 2;

/// The alpha a texel of that layer must carry.
const OPAQUE: u8 = 255;

#[test]
fn a_source_image_carrying_no_alpha_channel_fills_its_layer_opaque() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    let bytes = std::fs::read(committed_fixture(AN_IMAGE_WITHOUT_AN_ALPHA_CHANNEL)?)?;
    require_no_alpha_channel(&bytes)?;
    built_sets::with_one_image_replaced(root.path(), built_sets::A_RECORDED_IMAGE, &bytes)?;
    let key = TextureKey::parse(built_sets::THE_KEY_THAT_IMAGE_BELONGS_TO)?;

    let prepared = prepare_scene_at(root.path())?;

    let filled = levels_for(&key, &prepared.texels, TEXTURE_EDGE)?
        .first()
        .cloned()
        .ok_or("a filled layer has no level zero")?;
    let declared = the_image_this_fixture_holds();
    assert_eq!(
        (
            alphas_in(&filled),
            filled.len(),
            first_disagreement(&filled, &declared),
        ),
        (vec![OPAQUE], declared.len(), None),
        "a PNG with three channels says nothing about transparency, and what a layer filled from \
         one has to hold in its fourth channel is {OPAQUE} at every texel — the day a declared \
         opacity multiplies it, the alternative draws a hole where a wall belongs. The second and \
         third elements are the other half: a decoder that filled the channel correctly and then \
         dropped the picture, falling back to the texture generated for the key, would satisfy the \
         first element alone"
    );
    Ok(())
}

/// The alphas `filled` holds, distinct and ascending.
fn alphas_in(filled: &[[u8; 4]]) -> Vec<u8> {
    let mut found: Vec<u8> = filled.iter().map(|[_, _, _, alpha]| *alpha).collect();
    found.sort_unstable();
    found.dedup();
    found
}

/// The texels the committed image is drawn from, by the rule this module's
/// header states.
fn the_image_this_fixture_holds() -> Vec<[u8; 4]> {
    (0..TEXTURE_EDGE)
        .flat_map(|row| (0..TEXTURE_EDGE).map(move |column| (row, column)))
        .map(|(row, column)| {
            let (red, green) = ((column * 17) as u8, (row * 17) as u8);
            [red, green, OPAQUE - red, OPAQUE]
        })
        .collect()
}

/// Where two layers first disagree, as an index and the two texels there.
///
/// A summary rather than two whole layers: a failure printing 256 texels twice
/// buries the sentence a reader needs.
fn first_disagreement(
    filled: &[[u8; 4]],
    declared: &[[u8; 4]],
) -> Option<(usize, [u8; 4], [u8; 4])> {
    filled
        .iter()
        .zip(declared)
        .enumerate()
        .find(|(_, (one, other))| one != other)
        .map(|(at, (one, other))| (at, *one, *other))
}

/// Fails unless `bytes` is a PNG whose own header declares truecolour without an
/// alpha channel.
///
/// **Read out of the file rather than trusted**, and it shares no code with the
/// decoder under test: a fixture re-saved with an alpha channel would leave the
/// reading above green while asking nothing at all.
fn require_no_alpha_channel(bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let stated = bytes.get(COLOUR_TYPE_BYTE).copied();
    require(
        stated == Some(TRUECOLOUR_WITHOUT_ALPHA),
        format!(
            "this reading is about a source image that carries no alpha channel, and the \
             committed `{AN_IMAGE_WITHOUT_AN_ALPHA_CHANNEL}` declares colour type {stated:?} where \
             {TRUECOLOUR_WITHOUT_ALPHA} is truecolour without one. An image that does carry the \
             channel would make every assertion below a statement about what the file said rather \
             than about what the client does with a file that said nothing"
        ),
    )
}

/// A file committed beside this suite, located from the crate rather than from
/// wherever the test binary was started.
///
/// # Errors
///
/// Returns an error when the fixture is not there, which is a fixture that was
/// moved rather than a client that did anything.
fn committed_fixture(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let at = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("set")
        .join(name);
    if !at.is_file() {
        return Err(format!(
            "this reading is about a committed image with no alpha channel, and it is not at {}. \
             What it would build is a root nobody described",
            at.display()
        )
        .into());
    }
    Ok(at)
}
