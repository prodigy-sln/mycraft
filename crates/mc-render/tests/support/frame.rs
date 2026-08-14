//! Reading a captured frame: regions of it, and how far it sits from what was
//! derived for it.
//!
//! Every instrument here reports **how many pixels it looked at** beside its
//! verdict, and that is the point of the shape. A predicate that accepts nothing
//! makes "no pixel disagreed" and "no pixel strayed" both true, so a test that
//! asserted only the verdict would go green over an empty region — which is the
//! same vacuous pass a comparison against a frame nobody drew would give.

use std::error::Error;

use mc_testkit::frame::Rgba8Image;

/// A rectangle of a frame, in physical pixels, the way a HUD plan states one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// Whether `(x, y)` falls inside this rectangle.
    #[must_use]
    pub const fn holds(self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// This rectangle grown by `margin` pixels on every side, held at the frame
    /// origin so a rectangle at the edge does not wrap into a huge one.
    #[must_use]
    pub const fn grown_by(self, margin: u32) -> Self {
        Self {
            x: self.x.saturating_sub(margin),
            y: self.y.saturating_sub(margin),
            width: self.width + 2 * margin,
            height: self.height + 2 * margin,
        }
    }

    /// How many pixels this rectangle covers.
    #[must_use]
    pub const fn area(self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

/// Every pixel of `frame`, as its coordinates and its four channels.
pub fn pixels(frame: &Rgba8Image) -> impl Iterator<Item = (u32, u32, [u8; 4])> {
    (0..frame.height())
        .flat_map(move |y| (0..frame.width()).map(move |x| (x, y)))
        .filter_map(move |(x, y)| frame.pixel(x, y).map(|channels| (x, y, channels)))
}

/// How two frames stand at the pixels `chosen` accepts.
///
/// `considered` is reported beside the verdict on purpose: a predicate that
/// accepts nothing makes both counts zero, and a test asserting `different == 0`
/// over an empty region asserts nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comparison {
    pub considered: u64,
    pub same: u64,
    pub different: u64,
    pub first_different: Option<(u32, u32)>,
}

/// Compares `left` and `right` at every pixel `chosen` accepts.
///
/// A pixel one frame has and the other does not counts as a disagreement, which
/// is what two frames of different sizes are.
pub fn compare_frames(
    left: &Rgba8Image,
    right: &Rgba8Image,
    chosen: impl Fn(u32, u32) -> bool,
) -> Comparison {
    let mut seen = Comparison {
        considered: 0,
        same: 0,
        different: 0,
        first_different: None,
    };
    for (x, y, channels) in pixels(left).filter(|(x, y, _)| chosen(*x, *y)) {
        seen.considered += 1;
        if right.pixel(x, y) == Some(channels) {
            seen.same += 1;
        } else {
            seen.different += 1;
            seen.first_different = seen.first_different.or(Some((x, y)));
        }
    }
    seen
}

/// How far a region of a frame sits from the colour it is expected to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strays {
    pub considered: u64,
    pub count: u64,
    pub first: Option<(u32, u32, [u8; 3])>,
}

/// The pixels `chosen` accepts whose channels sit further than `tolerance` from
/// `expected`.
///
/// Per channel and in bytes rather than perceptually: the expected values here
/// are derived by arithmetic from a declared colour and a declared alpha, and
/// the tolerance exists for the last unit of the target's own encode, not for a
/// difference anybody should be willing to call the same colour.
pub fn strays_from(
    frame: &Rgba8Image,
    chosen: impl Fn(u32, u32) -> bool,
    expected: [u8; 3],
    tolerance: u8,
) -> Strays {
    let mut seen = Strays {
        considered: 0,
        count: 0,
        first: None,
    };
    for (x, y, channels) in pixels(frame).filter(|(x, y, _)| chosen(*x, *y)) {
        seen.considered += 1;
        let [red, green, blue, _] = channels;
        let shown = [red, green, blue];
        if shown
            .iter()
            .zip(expected)
            .any(|(left, right)| left.abs_diff(right) > tolerance)
        {
            seen.count += 1;
            seen.first = seen.first.or(Some((x, y, shown)));
        }
    }
    seen
}

/// Fails with `explanation` unless `holds`.
///
/// The shape every fixture check in these suites takes: a fixture that does not
/// have the property an assertion rests on is a broken fixture, not a failed
/// behaviour, and it says so before the assertion runs.
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
