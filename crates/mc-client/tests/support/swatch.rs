//! Reading a rendered swatch against the colours the texture behind it is made
//! of.
//!
//! Split out of the scenario file by responsibility, the way the renderer's own
//! offscreen suite splits its instruments from its scenarios: what a placeholder
//! layer is made of and how a region of a frame is measured against it are one
//! job, and the scenarios are another.
//!
//! **Every instrument here reports how many pixels it looked at beside its
//! verdict.** A region that accepts nothing makes "nothing strayed" true, so a
//! test asserting only the verdict would go green over an empty rectangle —
//! which is the same vacuous pass a comparison against a frame nobody drew would
//! give.
//!
//! **Nothing here reads a colour out of a frame to decide what that frame should
//! have held.** The colours come from `placeholder_texels`, which is a generator
//! and a pure function of a texture key; the frame is only ever the thing being
//! judged.

use std::collections::BTreeSet;
use std::error::Error;

use mc_core::id::{BlockName, TextureKey};
use mc_render::texture::placeholder::{PLACEHOLDER_SIZE, placeholder_texels};
use mc_testkit::frame::Rgba8Image;

use super::hud_frames::Rect;
use super::probe::distance;

/// How many colours a placeholder layer is made of.
///
/// Two, by construction: every texel is the declared mean plus or minus one
/// fixed step, laid out on a checkerboard, so an even-sided layer holds exactly
/// as many of one as of the other.
pub const TEXEL_COLORS: usize = 2;

/// How far a rendered pixel may sit from the texel colour it was drawn from, in
/// ΔE, and how far two colours must stand apart to count as told apart.
///
/// The array texture and the colour target are both sRGB, so a sampled texel is
/// decoded to linear light and encoded back on write and the round trip is the
/// byte it started as. The tolerance is for that encode and for nothing else.
pub const SAME_COLOR: f64 = 2.0;

/// The colours `block`'s placeholder layer is made of, in a stable order.
///
/// Derived from the key the block's own name spells, through the generator, and
/// never from a frame.
///
/// # Errors
///
/// Returns a failure when the name is not a texture key, or when the layer turns
/// out not to be made of exactly [`TEXEL_COLORS`] colours — which is the premise
/// the two-colour form rests on, and it is checked rather than assumed.
pub fn texel_colors(block: &BlockName) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
    let key = TextureKey::parse(block.as_str())?;
    let distinct: BTreeSet<[u8; 3]> = placeholder_texels(&key, PLACEHOLDER_SIZE)
        .into_iter()
        .map(|[red, green, blue, _]| [red, green, blue])
        .collect();
    require(
        distinct.len() == TEXEL_COLORS,
        format!(
            "a placeholder layer has to be made of exactly {TEXEL_COLORS} colours for the \
             assertion this feeds to mean what it says, but `{key}`'s holds {count}",
            key = key.as_str(),
            count = distinct.len()
        ),
    )?;
    Ok(distinct.into_iter().collect())
}

/// How a region of a frame stands against the colours it is expected to be made
/// of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwatchReading {
    /// How many pixels were looked at.
    pub considered: u64,
    /// How many sit further than [`SAME_COLOR`] from every one of the colours.
    pub strayed: u64,
    /// How many of those colours appear at all.
    pub shown: usize,
}

/// How the pixels of `rect` stand against `colors`.
///
/// # Errors
///
/// Returns the distance metric's own failure.
pub fn swatch_reading(
    frame: &Rgba8Image,
    rect: Rect,
    colors: &[[u8; 3]],
) -> Result<SwatchReading, Box<dyn Error>> {
    let mut seen = SwatchReading {
        considered: 0,
        strayed: 0,
        shown: 0,
    };
    let mut found = vec![false; colors.len()];
    for (x, y) in rect_pixels(rect) {
        let Some([red, green, blue, _]) = frame.pixel(x, y) else {
            continue;
        };
        seen.considered += 1;
        match nearest_color(&[red, green, blue], colors)? {
            Some(index) => found.get_mut(index).map_or((), |found| *found = true),
            None => seen.strayed += 1,
        }
    }
    seen.shown = found.into_iter().filter(|found| *found).count();
    Ok(seen)
}

/// Which of `colors` `shown` sits within [`SAME_COLOR`] of, or nothing when it
/// sits within that of none of them.
///
/// # Errors
///
/// Returns the distance metric's own failure.
fn nearest_color(shown: &[u8; 3], colors: &[[u8; 3]]) -> Result<Option<usize>, Box<dyn Error>> {
    for (index, color) in colors.iter().enumerate() {
        if distance(*shown, *color)? <= SAME_COLOR {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// Every pixel coordinate `rect` covers.
fn rect_pixels(rect: Rect) -> impl Iterator<Item = (u32, u32)> {
    (rect.y..rect.y + rect.height)
        .flat_map(move |y| (rect.x..rect.x + rect.width).map(move |x| (x, y)))
}

/// Fails with `explanation` unless `holds`.
///
/// A fixture that does not have the property an assertion rests on is a broken
/// fixture rather than a failed behaviour, and it says so before the assertion
/// runs.
///
/// # Errors
///
/// Returns `explanation` when `holds` is false.
pub fn require(holds: bool, explanation: String) -> Result<(), Box<dyn Error>> {
    if holds {
        Ok(())
    } else {
        Err(explanation.into())
    }
}
