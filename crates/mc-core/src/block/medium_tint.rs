//! What a block does to the light of everything seen from inside it, and the
//! one place the bound on its distance is written down.

/// The colour a medium carries what is seen from inside it toward, and how far
/// a surface stands from the eye before it is drawn wholly at that colour.
///
/// The colour is the three **sRGB-encoded** channel bytes a declaration stated,
/// carried unchanged. This crate performs no I/O and knows no transfer
/// function, so whoever draws a frame decodes it there — which is also what
/// keeps the value a mod author wrote the value every later reader sees.
///
/// The distance is in blocks and is measured radially from the eye. A surface
/// standing at it is drawn wholly at the colour, one at half of it halfway
/// toward the colour, and nothing beyond it is drawn any further along: the ramp
/// stops where the distance says it does.
///
/// **A type rather than a colour beside a number, because the bound is an
/// invariant and not a convention.** [`new`](Self::new) is the only door that
/// takes an arbitrary distance, so nothing downstream has to re-ask whether the
/// value it holds is one the engine can keep — which is what lets the draw path
/// carry the reciprocal of the distance with no guard against dividing by zero
/// and no branch for the case that would need one.
///
/// **Not [`Eq`]**, for the reason
/// [`BlockDefinition`](crate::block::BlockDefinition) is not one: it holds an
/// `f32`. `PartialEq` is what every comparison in the engine uses, and a tint
/// was never a map key or a set member.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MediumTint {
    color: [u8; 3],
    distance: f32,
}

impl MediumTint {
    /// The tint of `color` reaching its full strength at `distance` blocks, or
    /// nothing where that distance is not finite and greater than zero.
    ///
    /// **Zero is outside the range rather than at its edge.** A medium reaching
    /// full strength at no distance at all hides everything including itself,
    /// which is a different claim from any this type admits and not a weaker
    /// one. The exclusive floor is also what makes the reciprocal the draw path
    /// carries **defined** — which is all it guarantees, and deliberately: a
    /// subnormal distance is admitted and its reciprocal is an infinity, which
    /// draws every pixel wholly at the colour. That is the right answer for a
    /// medium you can see `1e-45` blocks through, so the floor is not raised to
    /// [`f32::MIN_POSITIVE`] to buy a finiteness nothing needs.
    ///
    /// **It answers one question and does not say which way a value was
    /// wrong.** Telling a mod author that `math.huge` is not finite rather than
    /// that it breaks the floor needs those two cases distinguished, and that
    /// distinction belongs to the loader that has a field name and a file to
    /// quote — not here, where there is nothing to attribute a refusal to.
    #[must_use]
    pub fn new(color: [u8; 3], distance: f32) -> Option<Self> {
        (distance.is_finite() && distance > 0.0).then_some(Self { color, distance })
    }

    /// The three sRGB channel bytes as they were declared.
    #[must_use]
    pub const fn color(self) -> [u8; 3] {
        self.color
    }

    /// The distance, in blocks, at which the tint reaches its full strength.
    #[must_use]
    pub const fn distance(self) -> f32 {
        self.distance
    }
}

#[cfg(test)]
#[path = "medium_tint_test.rs"]
mod tests;
