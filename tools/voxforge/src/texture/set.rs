//! The six faces of a block, as one type that cannot hold anything else.
//!
//! A face set carrying an isometric view is representable nonsense — a block
//! has six faces and none of them is a corner — so the newtype refuses the
//! impossible value **once**, at its one fallible constructor, rather than at
//! every site that would otherwise have to assume it.
//!
//! Each face also knows the two **model** axes its image runs along, because
//! nothing downstream can infer them and a consumer mapping a texture onto a
//! mesh has to be told: the printed pair is the whole of the declared
//! mitigation for that.

use crate::fault::{Fault, Origin};
use crate::format::Axis;
use crate::render::View;

/// One of the six axis-aligned faces of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AxisAlignedView(View);

impl AxisAlignedView {
    /// Every face of a block, in the order a set is emitted and reported in.
    pub const ALL: [Self; 6] = [
        Self(View::Front),
        Self(View::Back),
        Self(View::Left),
        Self(View::Right),
        Self(View::Top),
        Self(View::Bottom),
    ];

    /// `view` as a block face, refused when it is not one.
    ///
    /// # Errors
    ///
    /// Returns a [`Fault`] naming the value given and every face there is, since
    /// an agent repairing its own command line has only the message to repair
    /// from.
    pub fn parse(view: View) -> Result<Self, Fault> {
        Self::ALL
            .into_iter()
            .find(|face| face.0 == view)
            .ok_or_else(|| {
                let names: Vec<&str> = Self::ALL.iter().map(|face| face.as_str()).collect();
                Fault::about(
                    Origin::new("voxforge"),
                    format!(
                        "`{spelled}` is not a block face — a block has six, {offered}",
                        spelled = view.as_str(),
                        offered = names.join(", ")
                    ),
                )
                .in_field("face")
            })
    }

    /// The view this face is seen from.
    #[must_use]
    pub fn view(self) -> View {
        self.0
    }

    /// The face as a caller spells it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.0.as_str()
    }

    /// The model axis this face's image columns run along.
    ///
    /// Derived from D4's camera basis rather than tabulated independently: the
    /// image's horizontal axis is `right`, and for an axis-aligned face that
    /// vector lies along exactly one model axis.
    #[must_use]
    pub fn columns(self) -> Axis {
        match self.0 {
            View::Top | View::Bottom | View::Front | View::Back => Axis::X,
            _ => Axis::Z,
        }
    }

    /// The model axis this face's image rows run along.
    ///
    /// `y` for the four side faces, whose images run up the model; `z` for the
    /// two plan views, where there is no `y` in the picture at all. That is the
    /// distinction the printed pair exists to carry, and it is why a bare index
    /// could not say which kind of line an edge failure was measured along.
    #[must_use]
    pub fn rows(self) -> Axis {
        match self.0 {
            View::Top | View::Bottom => Axis::Z,
            _ => Axis::Y,
        }
    }
}
