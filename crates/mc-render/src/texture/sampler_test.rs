//! What the terrain sampler asks the device for, and the inspection that says
//! whether a request asks for anisotropy.
//!
//! **These two readings inspect a constant, and on their own they are agreement
//! between two copies of one decision.** What a sampler request *says* and what
//! a sampled frame *looks like* are different claims, and only the second is
//! about the picture — `crates/mc-render/tests/terrain_sampling.rs` holds it.
//! Both pairs are owed; neither substitutes for the other.
//!
//! **The second reading here is the first one's positive control and it is a
//! separate test function.** `asks_for_anisotropy` returning `false`
//! unconditionally satisfies the first reading forever, and nothing else in this
//! crate would notice: no other caller asks the question, and a frame drawn
//! through a sampler with `anisotropy_clamp = 1` looks exactly like a frame
//! drawn through one the inspection failed to notice was clamped. So the
//! inspection is fed a request that *does* ask, and has to say so.
//!
//! # Why the combination is what it is, written from the constraint and not from
//! the constant
//!
//! `wgpu-core-30.0.0/src/device/resource.rs:2288-2316` refuses a sampler with
//! `anisotropy_clamp != 1` unless magnification, minification **and** mip
//! interpolation are all linear, in three separate arms. Anisotropy and crisp
//! voxel magnification therefore cannot both be had, and this project takes the
//! crisp magnification: a block texture is sixteen texels of deliberate pattern
//! and a linear magnification filter blurs it towards its own mean. The two
//! halves that remain — linear minification and linear interpolation between mip
//! levels — are what stops a distant face shimmering as the camera moves.
//!
//! Each of the four values below is written out rather than read off
//! `TERRAIN_SAMPLER`. A reading that took its expectation from the constant
//! would agree with whatever the constant became, which is the one thing this
//! reading exists not to do.

use std::error::Error;

use super::{Filter, SamplerRequest, TERRAIN_SAMPLER, asks_for_anisotropy};

type TestResult = Result<(), Box<dyn Error>>;

/// What a sampler asks for when it asks for no anisotropy at all.
///
/// wgpu's own resting value for `anisotropy_clamp`, and the only one its
/// validation accepts beside a fully linear filter triple.
const NO_ANISOTROPY: u16 = 1;

/// A clamp that does ask for anisotropy, and the one the vendor's own
/// documentation names as the usual maximum.
const SIXTEEN_SAMPLES: u16 = 16;

#[test]
fn the_terrain_sampler_asks_for_nearest_magnification_and_linear_minification_without_anisotropy()
-> TestResult {
    let requested = TERRAIN_SAMPLER;

    assert_eq!(
        (
            requested.magnify,
            requested.minify,
            requested.between_levels,
            requested.anisotropy,
            asks_for_anisotropy(&requested),
        ),
        (
            Filter::Nearest,
            Filter::Linear,
            Filter::Linear,
            NO_ANISOTROPY,
            false,
        ),
        "all four are stated together because the device refuses three of the sixteen \
         combinations of them and the refusal names no single field: wgpu accepts an anisotropy \
         clamp above one only when magnification, minification and mip interpolation are all \
         linear. This project keeps nearest magnification — a block texture is sixteen texels of \
         deliberate pattern, and linear magnification blurs it towards its own mean — so the \
         clamp has to stay at one and anisotropy is refused rather than deferred. It requested \
         {requested}"
    );
    Ok(())
}

#[test]
fn a_request_whose_clamp_stands_above_one_is_reported_as_asking_for_anisotropy() -> TestResult {
    let anisotropic = SamplerRequest {
        anisotropy: SIXTEEN_SAMPLES,
        ..TERRAIN_SAMPLER
    };

    assert!(
        asks_for_anisotropy(&anisotropic),
        "this is the reading above's positive control and it is what stops \
         `asks_for_anisotropy` answering `false` for a constant reason. Nothing else in this \
         crate would notice if it did: no other caller asks the question, and a frame drawn \
         through a clamp nobody inspected looks exactly like a frame drawn through one that was \
         inspected. It answered `false` for {anisotropic}"
    );
    Ok(())
}
