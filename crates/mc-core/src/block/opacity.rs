//! How much light a block stops, and the one place its encoding into a byte is
//! written down.

/// How much light a block stops, as a degree between stopping none of it and
/// stopping all of it.
///
/// `1.0` stops all of it — an ordinary block, and what a declaration that says
/// nothing about the matter means. `0.0` stops none, which is a block that can
/// be seen through completely. Both bounds are inclusive and every value between
/// them is a legal declaration.
///
/// **A type rather than a bare `f32`, because the range is an invariant and not
/// a convention.** [`new`](Self::new) is the only door that takes an arbitrary
/// number, so nothing downstream has to re-ask whether the value it holds is one
/// the engine can keep — which is what lets the renderer encode it into eight
/// bits without a clamp, and what makes [`Eq`] sound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opacity(f32);

/// Sound because [`Opacity::new`] admits no NaN and [`Opacity::OPAQUE`] is a
/// literal: every value of this type is finite, so equality is reflexive.
///
/// Written out rather than derived because the compiler cannot see that
/// invariant, and it is there so that
/// [`ResolvedBlock`](crate::content::ResolvedBlock) keeps the `Eq` its own
/// documentation leans on.
impl Eq for Opacity {}

impl Opacity {
    /// A block that stops all the light reaching it, which is what a declaration
    /// saying nothing about opacity means.
    pub const OPAQUE: Self = Self(1.0);

    /// A block that stops none of the light reaching it.
    ///
    /// The other end of what [`new`](Self::new) admits, named so that whoever
    /// refuses a declaration against that range reads the bound from here
    /// instead of writing the number a second time.
    pub const CLEAR: Self = Self(0.0);

    /// `stated` as a degree of opacity, or nothing where it is not one.
    ///
    /// A NaN and both infinities fall outside `0.0..=1.0` and are refused by the
    /// same comparison the bounds are, which is what makes every value of this
    /// type finite.
    ///
    /// **It answers one question and does not say which way a value was
    /// wrong.** Telling a mod author that `math.huge` is not finite rather than
    /// that it is above the ceiling needs those cases distinguished, and that
    /// distinction belongs to the loader that has a field name and a file to
    /// quote — not here, where there is nothing to attribute a refusal to.
    #[must_use]
    pub fn new(stated: f32) -> Option<Self> {
        (0.0..=1.0).contains(&stated).then_some(Self(stated))
    }

    /// The degree as it was declared.
    ///
    /// The number an independent prediction composes from, and never
    /// [`quantised`](Self::quantised): a judge deriving its expectation from the
    /// encoding the draw path uses would be sharing that path's arithmetic
    /// instead of checking it.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }

    /// Whether any light reaches what stands behind a block drawn at this
    /// degree.
    ///
    /// **The one definition of what makes a face translucent**, so the loader
    /// refusing a contradiction and the packer choosing a draw ask the same
    /// question rather than each writing `< 1.0` for themselves.
    #[must_use]
    pub fn passes_light(self) -> bool {
        self.0 < Self::OPAQUE.0
    }

    /// This degree as the byte a packed vertex carries.
    ///
    /// **The one definition of that encoding**, so the renderer, the shader's
    /// expectations and any test that reasons about a packed vertex all read it
    /// from here rather than each writing `* 255` themselves.
    ///
    /// Rounds half away from zero, so a declared `0.5` encodes as `128` and
    /// decodes as `128/255 = 0.50196`. The error is bounded by half a code value
    /// by construction and is a term on the measured-error side of any tolerance
    /// derived against a rendered frame.
    #[must_use]
    pub fn quantised(self) -> u8 {
        (self.0 * f32::from(u8::MAX)).round() as u8
    }

    /// The degree a [`quantised`](Self::quantised) byte carries.
    ///
    /// The other half of the same encoding, here rather than at the packer so
    /// that the two directions cannot drift. Total by construction: every byte
    /// divided by [`u8::MAX`] lands inside the range this type admits, which is
    /// what lets a packed vertex be decoded without a fallback value.
    ///
    /// **It is not the inverse of `quantised`, and the asymmetry is the
    /// encoding's rather than this function's.** A declared `0.5` encodes as
    /// `128` and comes back as `0.50196`; two hundred and fifty-six bytes cannot
    /// name every degree a declaration may state. Whoever needs the declared
    /// number reads it from the declaration, and whoever grades a rendered frame
    /// carries the half-code-value error on the measured side of its tolerance.
    #[must_use]
    pub fn from_quantised(stored: u8) -> Self {
        Self(f32::from(stored) / f32::from(u8::MAX))
    }
}
