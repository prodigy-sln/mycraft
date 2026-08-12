//! The one place an sRGB colour becomes a linear one.
//!
//! The colour target is sRGB-encoded and the hardware performs the encode on
//! write, while `wgpu`'s clear values are **linear**. Clearing to the declared
//! sky colour therefore means handing the pass the linear form of it, and the
//! frame reads back as roughly (200, 231, 245) rather than the (135, 206, 235)
//! that was declared. Every probe compares against the declared value, so the
//! decode is what ties the two together.
//!
//! This is the module whose bug nothing downstream can see. A wrong transfer
//! function — a plain 2.2 gamma, the encode applied where the decode belongs, no
//! conversion at all — moves the cleared frame and the golden that records it in
//! the same direction at the same moment, and a probe comparing "the declared
//! colour" against a frame cleared with the same wrong function agrees with
//! itself. That is why the conversion lives here as a pure function checked
//! against arithmetic done by hand, and not inside whatever configures a pass.

/// Where the sRGB transfer function stops being a straight line.
///
/// Below this the standard uses a linear segment, because the power curve's
/// slope goes to infinity at zero and 8-bit values near black would quantise
/// into each other.
const LINEAR_SEGMENT_LIMIT: f64 = 0.040_45;

/// The slope of that linear segment.
const LINEAR_SEGMENT_SLOPE: f64 = 12.92;

/// The offset and scale of the power segment, and its exponent.
const POWER_SEGMENT_OFFSET: f64 = 0.055;
const POWER_SEGMENT_SCALE: f64 = 1.055;
const POWER_SEGMENT_EXPONENT: f64 = 2.4;

/// The largest value an 8-bit channel takes.
const CHANNEL_MAX: f64 = 255.0;

/// The sky the replay clears to, as declared: sRGB (135, 206, 235).
///
/// The declared form rather than the linear one, because this is the value a
/// human reads in the spec and the value the frame probes look for. Everything
/// that needs it linear goes through [`srgb8_to_linear`], so there is exactly
/// one conversion and no second constant that could be converted once already.
pub const CLEAR_COLOR_SRGB: [u8; 3] = [135, 206, 235];

/// Decodes an sRGB-encoded 8-bit colour into linear light.
///
/// `f64` rather than `f32` because this feeds `wgpu::Color`, which is `f64`, and
/// narrowing on the way in would round twice.
#[must_use]
pub fn srgb8_to_linear(channels: [u8; 3]) -> [f64; 3] {
    channels.map(|channel| decode(f64::from(channel) / CHANNEL_MAX))
}

/// One channel, already scaled to 0..=1.
fn decode(encoded: f64) -> f64 {
    if encoded <= LINEAR_SEGMENT_LIMIT {
        return encoded / LINEAR_SEGMENT_SLOPE;
    }
    ((encoded + POWER_SEGMENT_OFFSET) / POWER_SEGMENT_SCALE).powf(POWER_SEGMENT_EXPONENT)
}

#[cfg(test)]
#[path = "color_test.rs"]
mod tests;
