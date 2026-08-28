//! Reading a whole frame against the colours it is allowed to hold.
//!
//! # Three instruments, one rule
//!
//! [`census`] asks *which named colours a frame shows and whether anything
//! strayed from all of them*; [`classified`] asks the stronger, total question
//! FR-2.3 is about — is **every** pixel the clear colour, a declared layer's own
//! colour, or a sample between two of those that adjoin it on screen;
//! [`runs_across`] asks what one horizontal line runs through, in order, which
//! is how a seam is seen without committing a pixel coordinate.
//!
//! All three obey the rule `swatch.rs` states for regions and none of them
//! relaxes it: **every one reports how many pixels it looked at beside its
//! verdict**, and none of them ever reads a colour out of the frame in order to
//! decide what that frame should have held. The expected colours come from
//! `support::art` — the layer's own texels, and arithmetic over them — and the
//! frame is only ever the thing being judged.
//!
//! # Why the verdicts are enumerated and not absences
//!
//! `assert!(strays.is_empty())` cannot tell an empty answer from a scan that can
//! no longer look: a classifier whose colour list came back empty, or whose loop
//! stopped visiting pixels, answers "nothing strayed" exactly as loudly as a
//! clean frame does. So [`Accounting`] is a total enumeration that also carries
//! the pixel count, and a frame nothing looked at fails on the count before its
//! verdict is ever read.
//!
//! # A count is a band, never a number
//!
//! How many pixels a region covers is a property of where the eye happens to
//! stand, and committing one would make a nudged camera a failure of the reading
//! rather than of the thing that moved. [`Presence`] is the band instead:
//! nothing at all, fewer than [`MANY_PIXELS`], or at least that many. It is one
//! hundred because that is the floor two of this spec's scenarios state in their
//! own words.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use mc_core::id::TextureKey;
use mc_render::color::CLEAR_COLOR_SRGB;
use mc_render::texture::supplied::SuppliedTexels;
use mc_testkit::frame::Rgba8Image;

use super::art::{composited, drawn_colors};
use super::probe::distance;
use super::swatch::require;

/// How many pixels a colour has to cover before a reading about it is about a
/// region rather than about an edge.
pub const MANY_PIXELS: u64 = 100;

/// How often a named colour appeared, as a band rather than a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// Not one pixel of the frame stood at it.
    NotOnce,
    /// Some did, but fewer than [`MANY_PIXELS`] — carried with the count, because a
    /// handful of pixels where hundreds were expected is a different failure
    /// from none at all.
    Fewer(u64),
    /// At least [`MANY_PIXELS`] did.
    AtLeastMany,
}

impl Presence {
    /// `counted` as a band.
    #[must_use]
    pub const fn of(counted: u64) -> Self {
        if counted == 0 {
            Self::NotOnce
        } else if counted >= MANY_PIXELS {
            Self::AtLeastMany
        } else {
            Self::Fewer(counted)
        }
    }
}

/// Each of `expected` beside the presence it is owed, in the order it was
/// named — the whole of what a [`Census`]'s `shown` is compared against.
///
/// The two lists are zipped rather than written out a second time so that a
/// reading names each colour once. A presence list of the wrong length answers
/// short, which the comparison reports as the missing lines.
#[must_use]
pub fn owed(expected: &[Expected], presence: &[Presence]) -> Vec<(&'static str, Presence)> {
    expected
        .iter()
        .zip(presence)
        .map(|(one, count)| (one.name, *count))
        .collect()
}

/// One colour a frame is expected to hold, and what to call it in a verdict.
#[derive(Debug, Clone, Copy)]
pub struct Expected {
    pub name: &'static str,
    pub colour: [u8; 3],
}

impl Expected {
    /// `colour`, called `name` in every verdict it appears in.
    #[must_use]
    pub const fn new(name: &'static str, colour: [u8; 3]) -> Self {
        Self { name, colour }
    }

    /// The sky, which is a declared colour rather than a layer.
    #[must_use]
    pub const fn sky() -> Self {
        Self::new("the sky", CLEAR_COLOR_SRGB)
    }

    /// `over` laid on `under` at the degree `over`'s block declares.
    #[must_use]
    pub fn blend(name: &'static str, over: [u8; 3], under: [u8; 3], opacity: f64) -> Self {
        Self::new(name, composited(over, under, opacity))
    }
}

/// What a frame showed, named colour by named colour.
#[derive(Debug)]
pub struct Census {
    /// How many pixels were looked at.
    pub considered: u64,
    /// How often each named colour appeared, in the order it was named.
    pub shown: Vec<(&'static str, Presence)>,
    /// How many pixels stood within the tolerance of none of them.
    pub strayed: Presence,
    /// The first such pixel and what it showed, for the failure message. Never
    /// asserted against: which pixel comes first is a property of where the eye
    /// stands.
    pub first_stray: Option<Seen>,
}

/// How `frame` stands against `expected`, at `tolerance`.
///
/// A pixel counts for the **first** named colour it sits within `tolerance` of,
/// which is only unambiguous while the named colours stand further than twice
/// that apart — [`require_told_apart`] is what says so, and every reading that
/// uses this calls it.
///
/// # Errors
///
/// Returns the distance metric's own failure.
pub fn census(
    frame: &Rgba8Image,
    expected: &[Expected],
    tolerance: f64,
) -> Result<Census, Box<dyn Error>> {
    let named = named_by_colour(frame, expected, tolerance)?;
    let mut tally = Tally::over(expected.len());
    for (at, shown) in shown_pixels(frame) {
        tally.saw(at, shown, named.get(&shown).copied().flatten());
    }
    Ok(tally.into_census(expected))
}

/// What a pass over a frame accumulates on its way to a [`Census`].
struct Tally {
    considered: u64,
    counted: Vec<u64>,
    strayed: u64,
    first_stray: Option<Seen>,
}

impl Tally {
    /// A tally over `names` named colours, having seen nothing.
    fn over(names: usize) -> Self {
        Self {
            considered: 0,
            counted: vec![0; names],
            strayed: 0,
            first_stray: None,
        }
    }

    /// Records one pixel, `named` being the colour it was classified as.
    fn saw(&mut self, at: (u32, u32), shown: [u8; 3], named: Option<usize>) {
        self.considered += 1;
        let Some(count) = named.and_then(|index| self.counted.get_mut(index)) else {
            self.strayed += 1;
            self.first_stray = self.first_stray.or(Some((at, shown)));
            return;
        };
        *count += 1;
    }

    /// The tally as the verdict it stands for.
    fn into_census(self, expected: &[Expected]) -> Census {
        Census {
            considered: self.considered,
            shown: expected
                .iter()
                .zip(self.counted)
                .map(|(one, count)| (one.name, Presence::of(count)))
                .collect(),
            strayed: Presence::of(self.strayed),
            first_stray: self.first_stray,
        }
    }
}

/// Every pixel of `frame` that could be read, as its place and its colour.
fn shown_pixels(frame: &Rgba8Image) -> impl Iterator<Item = ((u32, u32), [u8; 3])> + '_ {
    coordinates(frame).filter_map(move |(x, y)| {
        frame
            .pixel(x, y)
            .map(|[red, green, blue, _]| ((x, y), [red, green, blue]))
    })
}

/// Which of `expected` each colour the frame actually holds stands within
/// `tolerance` of.
///
/// Classified once per distinct colour rather than once per pixel: the metric is
/// a pure function of two colours, and a frame of a quarter of a million pixels
/// holds a handful of them.
fn named_by_colour(
    frame: &Rgba8Image,
    expected: &[Expected],
    tolerance: f64,
) -> Result<BTreeMap<[u8; 3], Option<usize>>, Box<dyn Error>> {
    let mut named = BTreeMap::new();
    for shown in distinct_colours(frame) {
        named.insert(shown, nearest(shown, expected, tolerance)?);
    }
    Ok(named)
}

/// Every colour `frame` holds, without repetition.
fn distinct_colours(frame: &Rgba8Image) -> BTreeSet<[u8; 3]> {
    coordinates(frame)
        .filter_map(|(x, y)| frame.pixel(x, y))
        .map(|[red, green, blue, _]| [red, green, blue])
        .collect()
}

/// What every pixel of a frame came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accounting {
    /// Every pixel stands at the clear colour, at a declared layer's own
    /// colour, or between two of those that adjoin it in screen space.
    EveryPixelAccounted,
    /// At least one stands at a value none of those three admits.
    PixelsAccountedByNothing,
}

/// What a whole frame classified to.
#[derive(Debug)]
pub struct Classification {
    /// How many pixels were looked at.
    pub considered: u64,
    pub verdict: Accounting,
    /// How many pixels stand at neither the clear colour nor any declared
    /// layer's own colour, however they were finally accounted for.
    pub at_no_declared_colour: Presence,
    /// The first pixel nothing accounted for and what it showed, for the
    /// message.
    pub first_unaccounted: Option<Seen>,
}

/// Classifies every pixel of `frame` against the clear colour and the layers
/// `keys` name, filled from `texels`.
///
/// **The layer colours come from what fills the layer and never from the frame
/// under test**, which is what makes this a reading rather than a frame
/// certifying itself.
///
/// The third admitted class is *a sample between two of the others that adjoin
/// it in screen space*, and the qualifier is load-bearing: both source colours
/// must appear within one pixel of the sample, so a boundary between two
/// surfaces is admitted and the **interior** of a region that is a mix of two
/// layers is not. That is what leaves a blended surface unaccounted for while an
/// ordinary silhouette is accounted for.
///
/// The segment between two colours is walked in linear light at
/// [`STEPS_ALONG_A_SEGMENT`] steps, which puts consecutive candidates far closer
/// together than `tolerance` for any pair these fixtures hold.
///
/// # Errors
///
/// Returns the distance metric's own failure, or a key that is not a texture
/// key.
pub fn classified(
    frame: &Rgba8Image,
    keys: &[TextureKey],
    texels: &SuppliedTexels,
    tolerance: f64,
) -> Result<Classification, Box<dyn Error>> {
    let own = declared_colours(keys, texels);
    let accounted = standing_at(frame, &own, tolerance)?;
    let at_no_declared_colour = accounted
        .iter()
        .filter(|(_, _, named)| named.is_none())
        .count() as u64;
    let (unaccounted, first_unaccounted) = unaccounted_in(frame, &accounted, &own, tolerance)?;
    Ok(Classification {
        considered: accounted.len() as u64,
        verdict: if unaccounted == 0 {
            Accounting::EveryPixelAccounted
        } else {
            Accounting::PixelsAccountedByNothing
        },
        at_no_declared_colour: Presence::of(at_no_declared_colour),
        first_unaccounted,
    })
}

/// One pixel: where it is, what it showed, and which declared colour it stands
/// at.
type Accounted = ((u32, u32), [u8; 3], Option<usize>);

/// Every pixel of `frame` beside the declared colour it stands at, if any.
fn standing_at(
    frame: &Rgba8Image,
    own: &[[u8; 3]],
    tolerance: f64,
) -> Result<Vec<Accounted>, Box<dyn Error>> {
    let mut named = BTreeMap::new();
    for shown in distinct_colours(frame) {
        named.insert(shown, nearest_colour(shown, own, tolerance)?);
    }
    Ok(shown_pixels(frame)
        .map(|(at, shown)| (at, shown, named.get(&shown).copied().flatten()))
        .collect())
}

/// A pixel and what it showed, for a message that names one rather than
/// counting.
type Seen = ((u32, u32), [u8; 3]);

/// How many pixels neither stand at a declared colour nor between two that
/// adjoin them, and the first of those.
fn unaccounted_in(
    frame: &Rgba8Image,
    accounted: &[Accounted],
    own: &[[u8; 3]],
    tolerance: f64,
) -> Result<(u64, Option<Seen>), Box<dyn Error>> {
    let mut admitted: BTreeMap<([u8; 3], Vec<usize>), bool> = BTreeMap::new();
    let mut counted = 0;
    let mut first = None;
    for (at, shown, _) in accounted.iter().filter(|(_, _, named)| named.is_none()) {
        let around = adjoining(frame, *at, accounted);
        let key = (*shown, around.clone());
        let known = admitted.get(&key).copied();
        let answered = match known {
            Some(answered) => answered,
            None => between_two_adjoining(*shown, &around, own, tolerance)?,
        };
        admitted.insert(key, answered);
        if answered {
            continue;
        }
        counted += 1;
        first = first.or(Some((*at, *shown)));
    }
    Ok((counted, first))
}

/// How many points are tried along the segment between two colours.
const STEPS_ALONG_A_SEGMENT: u32 = 64;

/// The named colours the row `y` of `frame` runs through, left to right, with no
/// run repeated adjacently.
///
/// **A seam is an extra run**, which is how one is seen without committing a
/// pixel coordinate to a fixture: two abutting cells of one kind draw one run,
/// and anything drawn between them — a doubled blend, a gap showing the
/// background, a line of any colour at all — splits it into three.
///
/// A pixel at none of `expected` is named [`SOMETHING_NAMED_NOTHING`] rather
/// than skipped, so a seam of an unforeseen colour is reported instead of
/// silently joining the run beside it.
///
/// # Errors
///
/// Returns the distance metric's own failure.
pub fn runs_across(
    frame: &Rgba8Image,
    y: u32,
    expected: &[Expected],
    tolerance: f64,
) -> Result<Vec<&'static str>, Box<dyn Error>> {
    let mut runs: Vec<&'static str> = Vec::new();
    for x in 0..frame.width() {
        let Some([red, green, blue, _]) = frame.pixel(x, y) else {
            continue;
        };
        let named = match nearest([red, green, blue], expected, tolerance)? {
            Some(index) => expected
                .get(index)
                .map_or(SOMETHING_NAMED_NOTHING, |one| one.name),
            None => SOMETHING_NAMED_NOTHING,
        };
        if runs.last() != Some(&named) {
            runs.push(named);
        }
    }
    Ok(runs)
}

/// What [`runs_across`] calls a pixel standing at none of the colours it was
/// given.
pub const SOMETHING_NAMED_NOTHING: &str = "a colour nothing declared accounts for";

/// `frame` with the column at `x` painted `colour`, whole.
///
/// The synthetic control for [`runs_across`]: the engine draws no seam between
/// two cells of one kind — `sweep.rs` emits no face between them and the field
/// that would override that is deferred — so a frame holding one has to be made
/// rather than rendered. A test author reaching for a world fixture here finds it
/// unbuildable.
///
/// # Errors
///
/// Returns the image-shape failure, which repainting cannot produce.
pub fn with_a_seam_painted_down(
    frame: &Rgba8Image,
    x: u32,
    colour: [u8; 3],
) -> Result<Rgba8Image, Box<dyn Error>> {
    let [red, green, blue] = colour;
    let mut pixels = frame.as_bytes().to_vec();
    for y in 0..frame.height() {
        let at = ((y * frame.width() + x) * 4) as usize;
        for (byte, value) in pixels.iter_mut().skip(at).take(3).zip([red, green, blue]) {
            *byte = value;
        }
    }
    Ok(Rgba8Image::from_rgba(
        frame.width(),
        frame.height(),
        pixels,
    )?)
}

/// Fails unless every two of `expected` stand further than twice `tolerance`
/// apart.
///
/// **The half of a tolerance no assertion can otherwise enforce.** A pixel is
/// classified as the first colour it sits within `tolerance` of, so two colours
/// nearer than `2 x tolerance` have overlapping claims and a census over them
/// answers whichever was named first — silently, and identically to a correct
/// one. Stating the separation in a comment is not enough, because a palette
/// edit moves the colours and not the comment.
///
/// # Errors
///
/// Returns the distance metric's own failure, or the pairs that stand too close,
/// which is a broken fixture rather than a failed behaviour.
pub fn require_told_apart(expected: &[Expected], tolerance: f64) -> Result<(), Box<dyn Error>> {
    let mut too_close = Vec::new();
    for (index, one) in expected.iter().enumerate() {
        for other in expected.iter().skip(index + 1) {
            too_close.extend(overlapping(one, other, tolerance)?);
        }
    }
    require(
        too_close.is_empty(),
        format!(
            "a census calls a pixel the first colour it stands within ΔE {tolerance} of, so two \
             named colours nearer than ΔE {} have overlapping claims and the answer is whichever \
             was named first. These do: {too_close:?}",
            tolerance * 2.0
        ),
    )
}

/// How `one` and `other` overlap, or nothing where they stand clear of each
/// other.
fn overlapping(
    one: &Expected,
    other: &Expected,
    tolerance: f64,
) -> Result<Option<String>, Box<dyn Error>> {
    let apart = distance(one.colour, other.colour)?;
    if apart > tolerance * 2.0 {
        return Ok(None);
    }
    Ok(Some(format!(
        "`{}` {:?} and `{}` {:?} at ΔE {apart:.2}",
        one.name, one.colour, other.name, other.colour
    )))
}

/// Every colour the layers `keys` name are filled with, and the clear colour.
fn declared_colours(keys: &[TextureKey], texels: &SuppliedTexels) -> Vec<[u8; 3]> {
    let mut found: BTreeSet<[u8; 3]> = keys
        .iter()
        .flat_map(|key| drawn_colors(key, texels))
        .collect();
    found.insert(CLEAR_COLOR_SRGB);
    found.into_iter().collect()
}

/// Whether `shown` sits on the segment between two of the colours `around`
/// names.
fn between_two_adjoining(
    shown: [u8; 3],
    around: &[usize],
    own: &[[u8; 3]],
    tolerance: f64,
) -> Result<bool, Box<dyn Error>> {
    for (one, other) in pairs_of(around) {
        if sits_between(shown, own.get(one), own.get(other), tolerance)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Every unordered pair of `around`.
fn pairs_of(around: &[usize]) -> Vec<(usize, usize)> {
    around
        .iter()
        .enumerate()
        .flat_map(|(index, one)| {
            around
                .iter()
                .skip(index + 1)
                .map(move |other| (*one, *other))
        })
        .collect()
}

/// Whether `shown` sits on the segment between `one` and `other` in linear
/// light, walked at [`STEPS_ALONG_A_SEGMENT`] steps.
fn sits_between(
    shown: [u8; 3],
    one: Option<&[u8; 3]>,
    other: Option<&[u8; 3]>,
    tolerance: f64,
) -> Result<bool, Box<dyn Error>> {
    let (Some(one), Some(other)) = (one, other) else {
        return Ok(false);
    };
    for step in 0..=STEPS_ALONG_A_SEGMENT {
        let along = f64::from(step) / f64::from(STEPS_ALONG_A_SEGMENT);
        if distance(shown, composited(*one, *other, along))? <= tolerance {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The declared colours standing within one pixel of `at`, without repetition.
fn adjoining(frame: &Rgba8Image, at: (u32, u32), accounted: &[Accounted]) -> Vec<usize> {
    let (x, y) = at;
    let mut found = BTreeSet::new();
    for (near_x, near_y) in neighbourhood(frame, x, y) {
        let index = (near_y * frame.width() + near_x) as usize;
        if let Some((_, _, Some(stands_at))) = accounted.get(index) {
            found.insert(*stands_at);
        }
    }
    found.into_iter().collect()
}

/// The coordinates within one pixel of `x`, `y`, inside `frame`.
fn neighbourhood(frame: &Rgba8Image, x: u32, y: u32) -> Vec<(u32, u32)> {
    let rows = y.saturating_sub(1)..=(y + 1).min(frame.height().saturating_sub(1));
    let columns = x.saturating_sub(1)..=(x + 1).min(frame.width().saturating_sub(1));
    rows.flat_map(move |near_y| columns.clone().map(move |near_x| (near_x, near_y)))
        .filter(|near| *near != (x, y))
        .collect()
}

/// Which of `expected` `shown` sits within `tolerance` of, first named wins.
fn nearest(
    shown: [u8; 3],
    expected: &[Expected],
    tolerance: f64,
) -> Result<Option<usize>, Box<dyn Error>> {
    for (index, one) in expected.iter().enumerate() {
        if distance(shown, one.colour)? <= tolerance {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// The same, over bare colours.
fn nearest_colour(
    shown: [u8; 3],
    colours: &[[u8; 3]],
    tolerance: f64,
) -> Result<Option<usize>, Box<dyn Error>> {
    for (index, colour) in colours.iter().enumerate() {
        if distance(shown, *colour)? <= tolerance {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// Every coordinate of `frame`, row by row.
fn coordinates(frame: &Rgba8Image) -> impl Iterator<Item = (u32, u32)> {
    let (width, height) = (frame.width(), frame.height());
    (0..height).flat_map(move |y| (0..width).map(move |x| (x, y)))
}
