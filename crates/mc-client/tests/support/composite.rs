//! The colours a marched ray's crossing is predicted to draw, composed in linear
//! light from what each layer declares.
//!
//! # It shares no code with the draw path it grades
//!
//! That is FR-5.1-S2's whole subject and it is a constraint no assertion can
//! enforce, so it is written here where a reader meets it. Three inputs, three
//! places:
//!
//! - the **degree** comes from
//!   [`BlockDefinition::opacity`](mc_core::block::BlockDefinition), read off the
//!   registry through [`super::oracle`]'s own door — **never** from
//!   `TextureResolution::opacity_of`, which is what the packer partitions on,
//!   and **never** through [`Opacity::quantised`](mc_core::block::Opacity),
//!   which is the encoding a packed vertex carries;
//! - the **colours** come from the images on disk through [`super::art`], which
//!   reads a file and never a frame;
//! - the **composition** is [`super::art::composited`], `src-over` written out
//!   from a transfer pair taken from IEC 61966-2-1.
//!
//! Nothing here calls into `mc_render`'s geometry, gpu or mip modules, and
//! `the_judge_composes_what_a_ray_passes_through.rs` is what says so with a
//! positive control rather than leaving it to this paragraph.
//!
//! # Why the prediction is a set of colours and not one colour
//!
//! A layer is an image, not a swatch. A magnified face shows one of its texels,
//! a fully minified one shows the layer's mean, and one at middle distance shows
//! a reduced texel that is neither — so
//! [`super::art::landmarks_at_every_scale`] answers "what may a pixel of *this*
//! layer be" with a set, and a composition of two layers inherits that from both
//! sides at once. Predicting the mean-over-mean triple alone and widening a
//! tolerance until the spread fitted inside it would be the tolerance doing the
//! layer's work, which is the mistake this project has paid for by name.
//!
//! **Measured, and it is the whole reason the wider set is used**: over the
//! three declared captures the worst a blended sample stands from the nearest
//! colour predicted for it falls from **ΔE 7.13** against the narrow set to
//! **ΔE 1.29** against this one, and the worst an unblended sample stands falls
//! from **ΔE 16.14** to **ΔE 7.42**. Neither number is a tolerance; both are the
//! measured-error side of one.
//!
//! So [`Palette::predicted`] answers the set, and [`Palette::predicted_mean`]
//! answers the single triple a reading quotes in a failure message.
//!
//! # The degree is the declared `f32` and the quantisation is on the other side
//!
//! A declared `0.5` reaches a fragment as `128 / 255 = 0.50196`. Composing from
//! the byte instead would be this prediction quietly adopting the encoding it
//! exists to check, and the difference — under a third of one code value over
//! this project's own colours — belongs on the **measured error** side of
//! whatever tolerance a reading derives.

use std::collections::BTreeSet;
use std::error::Error;

use mc_core::block::{BlockRegistry, MediumTint};
use mc_core::content::TEXTURE_EDGE;
use mc_core::id::BlockName;
use mc_render::color::CLEAR_COLOR_SRGB;
use mc_render::texture::TextureResolution;
use mc_render::texture::supplied::SuppliedTexels;

use super::art::{composited, drawn_texels, landmarks_at_every_scale, linear_mean};
use super::oracle::{Crossed, Surface};
use super::probe::distance;

/// Everything a prediction is composed from, and the three places each part
/// comes from.
#[derive(Debug, Clone, Copy)]
pub struct Palette<'a> {
    registry: &'a BlockRegistry,
    resolution: &'a TextureResolution,
    texels: &'a SuppliedTexels,
}

impl<'a> Palette<'a> {
    /// The palette a world's registry, resolution and texels describe.
    #[must_use]
    pub const fn of(
        registry: &'a BlockRegistry,
        resolution: &'a TextureResolution,
        texels: &'a SuppliedTexels,
    ) -> Self {
        Self {
            registry,
            resolution,
            texels,
        }
    }

    /// How much light `block` declares it stops, as the number the declaration
    /// states.
    ///
    /// # Errors
    ///
    /// Returns the registry's own refusal for a block it does not register.
    pub fn degree_of(&self, block: &BlockName) -> Result<f64, Box<dyn Error>> {
        Ok(f64::from(self.registry.resolve(block)?.opacity.get()))
    }

    /// Every colour a pixel of `surface` may legitimately draw, at any distance.
    ///
    /// [`super::art::landmarks_at_every_scale`] rather than
    /// [`super::art::landmarks`], and the difference is measured rather than
    /// stylistic: a face at middle distance shows a *reduced* texel, which for a
    /// grass side stands as far as ΔE 16.14 from every colour the narrower set
    /// offers. Judging such a pixel against the narrower set and then widening a
    /// tolerance to fit would be the tolerance doing the layer's work.
    ///
    /// # Errors
    ///
    /// Returns an error for a surface the eye stood inside — there is no facing
    /// and therefore no key, and choosing one would be an invention — or for a
    /// block the resolution does not describe.
    pub fn landmarks_of(&self, surface: &Surface) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
        Ok(landmarks_at_every_scale(
            &drawn_texels(&self.key_of(surface)?, self.texels),
            TEXTURE_EDGE,
        ))
    }

    /// The one colour a pixel of `surface` shows where its whole face converges
    /// on the layer's mean.
    ///
    /// # Errors
    ///
    /// As [`landmarks_of`](Self::landmarks_of).
    pub fn mean_of(&self, surface: &Surface) -> Result<[u8; 3], Box<dyn Error>> {
        Ok(linear_mean(&drawn_texels(
            &self.key_of(surface)?,
            self.texels,
        )))
    }

    /// Every colour a pixel whose ray met `crossed` may legitimately draw,
    /// ascending and without repeats.
    ///
    /// Composed from the far end inwards, which is the order `src-over` puts
    /// them in: the surface that stopped the ray is the background, and each
    /// layer in turn is laid over what the layers behind it already came to.
    ///
    /// # Errors
    ///
    /// As [`landmarks_of`](Self::landmarks_of), or the registry's refusal for a
    /// layer it does not register.
    pub fn predicted(&self, crossed: &Crossed) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
        let mut standing: Vec<[u8; 3]> = match &crossed.beyond {
            None => vec![CLEAR_COLOR_SRGB],
            Some(surface) => self.landmarks_of(surface)?,
        };
        for layer in crossed.layers.iter().rev() {
            let degree = self.degree_of(&layer.block)?;
            let over = self.landmarks_of(layer)?;
            standing = over
                .iter()
                .flat_map(|src| standing.iter().map(|dst| composited(*src, *dst, degree)))
                .collect::<BTreeSet<[u8; 3]>>()
                .into_iter()
                .collect();
        }
        Ok(standing)
    }

    /// The one colour `crossed` comes to when every layer and the surface behind
    /// them all show their own mean.
    ///
    /// What a failure message quotes, so a reader sees a triple rather than a
    /// set of eighty.
    ///
    /// # Errors
    ///
    /// As [`predicted`](Self::predicted).
    pub fn predicted_mean(&self, crossed: &Crossed) -> Result<[u8; 3], Box<dyn Error>> {
        let mut standing = match &crossed.beyond {
            None => CLEAR_COLOR_SRGB,
            Some(surface) => self.mean_of(surface)?,
        };
        for layer in crossed.layers.iter().rev() {
            standing = composited(
                self.mean_of(layer)?,
                standing,
                self.degree_of(&layer.block)?,
            );
        }
        Ok(standing)
    }

    /// How far the nearest colour `crossed` predicts stands from `drawn`.
    ///
    /// # Errors
    ///
    /// As [`predicted`](Self::predicted), or the distance metric's own failure.
    pub fn stands_from(&self, crossed: &Crossed, drawn: [u8; 3]) -> Result<f64, Box<dyn Error>> {
        let mut nearest = f64::MAX;
        for colour in self.predicted(crossed)? {
            nearest = nearest.min(distance(colour, drawn)?);
        }
        Ok(nearest)
    }

    /// How far the nearest colour `crossed` predicts stands from the nearest
    /// colour any one of its operands would draw **unblended**.
    ///
    /// **The half of a tolerance that no reading can measure for itself.** A
    /// composite standing near one of the colours it was composed from is a
    /// composite a reading cannot tell from that colour — so an implementation
    /// that lost the blend entirely, or lost what stands behind it, would be
    /// accepted. Asserted per crossing on every run rather than argued once in a
    /// comment, which is the shape `require_told_apart` already takes for a
    /// fixture's palette.
    ///
    /// A crossing with no layer at all has no operand to be confused with and
    /// answers [`f64::MAX`].
    ///
    /// # Errors
    ///
    /// As [`predicted`](Self::predicted), or the distance metric's own failure.
    pub fn unblended_stands_from(&self, crossed: &Crossed) -> Result<f64, Box<dyn Error>> {
        if crossed.layers.is_empty() {
            return Ok(f64::MAX);
        }
        let predicted = self.predicted(crossed)?;
        let mut unblended: Vec<[u8; 3]> = match &crossed.beyond {
            None => vec![CLEAR_COLOR_SRGB],
            Some(surface) => self.landmarks_of(surface)?,
        };
        for layer in &crossed.layers {
            unblended.extend(self.landmarks_of(layer)?);
        }
        nearest_between(&predicted, &unblended)
    }

    /// Every colour a pixel whose ray met `crossed` may draw, seen from inside a
    /// medium declaring `tint`.
    ///
    /// **Each layer is carried by its own distance and the carried layers are
    /// then composed**, which is the order the law and `src-over` put them in.
    /// A pixel whose ray met nothing is the medium's own colour rather than the
    /// sky's — the clear carries the tint, so the far field and the empty field
    /// arrive at one colour by two routes.
    ///
    /// # Errors
    ///
    /// As [`predicted`](Self::predicted).
    pub fn predicted_through(
        &self,
        crossed: &Crossed,
        tint: Option<MediumTint>,
    ) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
        let mut standing: Vec<[u8; 3]> = match &crossed.beyond {
            None => vec![carried(CLEAR_COLOR_SRGB, tint, f32::INFINITY)],
            Some(surface) => self
                .landmarks_of(surface)?
                .into_iter()
                .map(|colour| carried(colour, tint, surface.along))
                .collect(),
        };
        for layer in crossed.layers.iter().rev() {
            let degree = self.degree_of(&layer.block)?;
            let over: Vec<[u8; 3]> = self
                .landmarks_of(layer)?
                .into_iter()
                .map(|colour| carried(colour, tint, layer.along))
                .collect();
            standing = over
                .iter()
                .flat_map(|src| standing.iter().map(|dst| composited(*src, *dst, degree)))
                .collect::<BTreeSet<[u8; 3]>>()
                .into_iter()
                .collect();
        }
        Ok(standing)
    }

    /// How far the nearest colour `crossed` predicts, seen through `tint`,
    /// stands from `drawn`.
    ///
    /// # Errors
    ///
    /// As [`predicted_through`](Self::predicted_through), or the distance
    /// metric's own failure.
    pub fn stands_from_through(
        &self,
        crossed: &Crossed,
        drawn: [u8; 3],
        tint: Option<MediumTint>,
    ) -> Result<f64, Box<dyn Error>> {
        let mut nearest = f64::MAX;
        for colour in self.predicted_through(crossed, tint)? {
            nearest = nearest.min(distance(colour, drawn)?);
        }
        Ok(nearest)
    }

    /// The texture key `surface`'s face draws from.
    fn key_of(&self, surface: &Surface) -> Result<mc_core::id::TextureKey, Box<dyn Error>> {
        let facing = surface.facing.ok_or_else(|| {
            format!(
                "a ray met `{}` without entering it by any face, which happens only where the eye \
                 already stood inside that voxel. There is no face to read a texture key off, and \
                 picking one would put an invented colour on the expectation side of an assertion",
                surface.block.as_str()
            )
        })?;
        Ok(self
            .resolution
            .key_of(&surface.block, facing.face())
            .ok_or_else(|| {
                format!(
                    "the resolution describes no block called `{}`, so the face a ray entered by \
                     draws from no key this prediction can read a colour out of",
                    surface.block.as_str()
                )
            })?
            .clone())
    }
}

/// `own`, carried toward the medium's colour by how far away it stands.
///
/// **The one statement of the law on the prediction side.** `min(1, d / D)` in
/// linear light, through the transfer pair `super::art` declares from
/// IEC 61966-2-1 — which shares no code with the draw path. A second spelling of
/// it would be two predictions that could part.
///
/// A medium declaring nothing carries nothing, and that is the arithmetic
/// identity rather than a branch a caller has to remember.
#[must_use]
pub fn carried(own: [u8; 3], tint: Option<MediumTint>, along: f32) -> [u8; 3] {
    tint.map_or(own, |medium| {
        composited(
            medium.color(),
            own,
            f64::from((along / medium.distance()).min(1.0)),
        )
    })
}

/// How near the nearest pair of `one` and `other` stand, in ΔE.
///
/// **One loop over the pairs rather than two nested ones**, so a reading that
/// wants the separation between two sets of colours says so in one place. Every
/// distance is the harness's own metric driven through
/// [`super::probe::distance`], never a second implementation of it.
///
/// # Errors
///
/// Returns the distance metric's own failure.
pub fn nearest_between(one: &[[u8; 3]], other: &[[u8; 3]]) -> Result<f64, Box<dyn Error>> {
    let mut nearest = f64::MAX;
    for (colour, against) in one
        .iter()
        .flat_map(|colour| other.iter().map(move |against| (colour, against)))
    {
        nearest = nearest.min(distance(*colour, *against)?);
    }
    Ok(nearest)
}
