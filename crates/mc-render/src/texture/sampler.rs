//! What the terrain pass asks a sampler for, as a value rather than as a
//! descriptor.
//!
//! **The request is stated here so that two different things can be said about
//! it.** What a sampler *asks for* is a pure value with no device behind it, and
//! it is what `sampler_test.rs` reads; what a sampler *does to a picture* needs a
//! device and a captured frame, and lives in
//! `crates/mc-render/tests/terrain_sampling.rs`. A test that read back the
//! descriptor it caused to be built would be agreement between two copies of one
//! decision, so both are owed.
//!
//! It is also what lets a capture ask for a *second* configuration. The
//! difference linear minification makes is only observable against a run that
//! minifies without it, and a free function reaching for a constant cannot be
//! asked for the other one.
//!
//! # Why the combination is what it is
//!
//! `wgpu-core-30.0.0/src/device/resource.rs:2288-2316` refuses a sampler whose
//! `anisotropy_clamp` stands above one unless magnification, minification **and**
//! mip interpolation are all linear, in three separate arms. Anisotropy and crisp
//! voxel magnification therefore cannot both be had, and this project takes the
//! crisp magnification: a block texture is sixteen texels of deliberate pattern
//! and a linear magnification filter blurs it towards its own mean. The two
//! halves that remain — linear minification and linear interpolation between mip
//! levels — are what stops a distant face shimmering as the camera moves.
//!
//! The refusal is left to the device rather than pre-checked here. A pre-check
//! would be a second copy of a vendor constraint, and it would go on agreeing
//! with itself the day the vendor changed the rule.

use std::fmt;

/// How a sampler picks its answer when a texel does not land on a pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// The nearest texel, whole.
    Nearest,
    /// A blend of the texels either side.
    Linear,
}

impl fmt::Display for Filter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let said = match self {
            Self::Nearest => "nearest",
            Self::Linear => "linear",
        };
        formatter.write_str(said)
    }
}

/// The resting clamp: a sampler that asks for no anisotropic filtering at all.
///
/// wgpu's own default, and the only value its validation accepts beside a fully
/// linear filter triple.
pub const NO_ANISOTROPY: u16 = 1;

/// What a terrain sampler is asked to be.
///
/// Every field is part of one request rather than four independent ones, because
/// the device's validation is over the combination: a refusal names no single
/// field, and neither does [`Display`](fmt::Display) here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplerRequest {
    /// How a texel larger than a pixel is resolved.
    pub magnify: Filter,
    /// How texels smaller than a pixel are resolved within one mip level.
    pub minify: Filter,
    /// How the two mip levels either side of the wanted detail are combined.
    pub between_levels: Filter,
    /// How many samples an anisotropic filter may take, where
    /// [`NO_ANISOTROPY`] asks for none.
    pub anisotropy: u16,
}

impl fmt::Display for SamplerRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "magnify {magnify}, minify {minify}, between levels {between}, anisotropy clamp \
             {anisotropy}",
            magnify = self.magnify,
            minify = self.minify,
            between = self.between_levels,
            anisotropy = self.anisotropy
        )
    }
}

/// What every terrain fragment in this project is sampled through.
///
/// Nearest magnification keeps a block texture's sixteen texels of deliberate
/// pattern crisp; linear minification and linear interpolation between mip
/// levels are what stop a distant face shimmering. Anisotropy is refused rather
/// than deferred: the device will not accept it beside nearest magnification,
/// and the magnification is the half this project keeps.
pub const TERRAIN_SAMPLER: SamplerRequest = SamplerRequest {
    magnify: Filter::Nearest,
    minify: Filter::Linear,
    between_levels: Filter::Linear,
    anisotropy: NO_ANISOTROPY,
};

/// Whether `requested` asks the device for anisotropic filtering.
///
/// Its own function rather than a comparison at each call, because the resting
/// value is a vendor convention — one sample, not zero — and a caller writing
/// `!= 0` would be asking a question the device does not answer.
#[must_use]
pub const fn asks_for_anisotropy(requested: &SamplerRequest) -> bool {
    requested.anisotropy > NO_ANISOTROPY
}

#[cfg(test)]
#[path = "sampler_test.rs"]
mod tests;
