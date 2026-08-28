//! What the chosen ordering model gets wrong, drawn rather than warned about.
//!
//! # The model's limit is a deliverable, not a footnote
//!
//! `architecture.md` chose **B1** — one unsorted blended pass, depth-test on,
//! depth-write off — knowing that `src-over` is not commutative and that nothing
//! sorts. It is correct for the content that ships, because one translucent kind
//! means both surfaces along a ray carry the same colour and the two orderings
//! agree. The day a second kind arrives it stops being correct, and the spec
//! makes *the model's stated behaviour* the thing that has to exist rather than
//! a warning somebody writes.
//!
//! So this is that behaviour, drawn: two translucent kinds of different colours
//! over one wall, one eye, two frames, differing in nothing but the order the
//! two panes are handed to the packer. **The heavier share goes to whichever
//! composites last, and which one that is has nothing to do with which stands
//! nearer the eye.**
//!
//! # Why it is stated as two frames and not as one wrong colour
//!
//! A single frame showing an unexpected colour is a claim about arithmetic
//! somebody could get wrong in the test instead. Two frames of one scene from
//! one eye, whose only difference is a list order, are a claim about the
//! *engine*: whatever else the two colours are, they should not depend on that,
//! and they do. Both are asserted absolutely, against composites derived from
//! the declared degree and the three layers' own colours in linear light — so
//! neither frame is judged against the other and both would redden if the
//! arithmetic moved.
//!
//! # The one that agrees with a sorted model is asserted too
//!
//! Under [`Emitted::FartherFirst`] emission order and depth order happen to
//! coincide, so that frame is what a *correct* model would draw. Asserting it
//! beside the other is what makes the pair a statement about ordering rather
//! than about one arrangement being broken: a draw path that had come to
//! composite in some third order fails both.
//!
//! **It must redden if a later spec sorts the blended half.** That is correct
//! behaviour and not a fragility to design around: the model this reading is
//! about would have changed, and `docs/technical/rendering.md` would be
//! describing an engine that no longer exists.

mod support;

use mc_render::color::CLEAR_COLOR_SRGB;
use mc_testkit::frame::Rgba8Image;

use support::TestResult;
use support::artefact::{
    Emitted, THE_OVERLAP, WALL_COLOUR, composed_when, nearer_over_the_wall, shot,
};
use support::pixel_census::{Expected, MANY_PIXELS, Presence, census, owed, require_told_apart};
use support::probe::distance;
use support::translucency::{PIXELS_IN_THE_FRAME, TELLS_THEM_APART};

/// What the census calls each colour.
const THE_SKY: &str = "the sky";
const THE_WALL: &str = "the wall, wherever nothing covers it";
const THE_RING: &str = "the nearer pane alone over the wall";
const NEARER_LAST: &str = "both panes, the nearer one composited last";
const FARTHER_LAST: &str = "both panes, the farther one composited last";

/// The two presences these readings name.
const MANY: Presence = Presence::AtLeastMany;
const NONE: Presence = Presence::NotOnce;

#[test]
fn two_translucent_kinds_take_their_weights_from_emission_order_and_not_from_depth() -> TestResult {
    let expected = the_five_colours();
    require_told_apart(&expected, TELLS_THEM_APART)?;

    let mut counted = Vec::new();
    for order in Emitted::BOTH {
        let Some(shown) = shot(order)? else {
            return Ok(());
        };
        counted.push(counted_over(order, &shown, &expected)?);
    }

    assert_eq!(
        counted,
        what_each_order_owes(&expected),
        "one scene, one eye, two frames, differing only in which pane reaches the packer first. \
         Nothing sorts, so the pane whose indices land later composites later and takes the \
         heavier share — which is why the fourth and fifth lines swap between the frames. That is \
         the model's stated limit drawn rather than warned about. The third line is the control \
         that keeps it about the overlap: the ring where only the nearer pane covers the wall is \
         one colour in both. A draw path sorting back to front puts the fourth line in both \
         frames and the fifth in neither"
    );
    Ok(())
}

#[test]
fn the_two_frames_differ_at_exactly_the_pixels_where_the_two_kinds_overlap() -> TestResult {
    let (Some(farther_first), Some(nearer_first)) =
        (shot(Emitted::FartherFirst)?, shot(Emitted::NearerFirst)?)
    else {
        return Ok(());
    };

    let differing = differing_pixels(&farther_first.frame, &nearer_first.frame);
    let exchanged = exchanged_pixels(&farther_first.frame, &nearer_first.frame)?;

    assert_eq!(
        (
            Presence::of(differing),
            Presence::of(exchanged),
            differing == exchanged,
            at_the_overlap(&farther_first.frame, &nearer_first.frame)?,
        ),
        (MANY, MANY, true, BOTH_COMPOSITES),
        "the artefact is confined to where two translucent surfaces of *different colours* \
         overlap, which is the whole of what makes it invisible for the one kind that ships. So \
         every pixel that differs between the two frames has to be a pixel showing one \
         composition in the first and the other in the second — {differing} differ and \
         {exchanged} exchange — and the two counts have to be the same number, or something moved \
         that this reading has not named. The fourth element reads the one pixel the fixture's \
         own symmetry declares: the frame's centre, which both panes cover at any depth"
    );
    Ok(())
}

/// What the two orders owe between them: the same three colours in both, with
/// the two compositions exchanged.
fn what_each_order_owes(expected: &[Expected]) -> Vec<Counted> {
    vec![
        owed_by(
            Emitted::FartherFirst,
            expected,
            [MANY, MANY, MANY, MANY, NONE],
        ),
        owed_by(
            Emitted::NearerFirst,
            expected,
            [MANY, MANY, MANY, NONE, MANY],
        ),
    ]
}

/// What one frame's census came to, in the form the reading above compares.
type Counted = (&'static str, u64, Vec<(&'static str, Presence)>, Presence);

/// `shown`'s census against `expected`, named by the order that drew it.
fn counted_over(
    order: Emitted,
    shown: &support::translucency::Shot,
    expected: &[Expected],
) -> Result<Counted, Box<dyn std::error::Error>> {
    let seen = census(&shown.frame, expected, TELLS_THEM_APART)?;
    Ok((
        order.described(),
        seen.considered,
        seen.shown.clone(),
        seen.strayed,
    ))
}

/// What `order` owes: the whole frame looked at, each colour at `presence`, and
/// nothing strayed.
fn owed_by(order: Emitted, expected: &[Expected], presence: [Presence; 5]) -> Counted {
    (
        order.described(),
        PIXELS_IN_THE_FRAME,
        owed(expected, &presence),
        NONE,
    )
}

/// The five colours either frame may hold.
///
/// **The two compositions are named apart and both are always in the list**, so
/// a frame is asked whether it holds each of them rather than which of them it
/// holds. A list carrying only the one that frame is expected to show could not
/// tell "the right composition" from "a colour nothing predicted".
fn the_five_colours() -> [Expected; 5] {
    [
        Expected::new(THE_SKY, CLEAR_COLOR_SRGB),
        Expected::new(THE_WALL, WALL_COLOUR),
        Expected::new(THE_RING, nearer_over_the_wall()),
        Expected::new(NEARER_LAST, composed_when(Emitted::FartherFirst)),
        Expected::new(FARTHER_LAST, composed_when(Emitted::NearerFirst)),
    ]
}

/// What the overlap shows in the two frames when the model composites in
/// emission order.
const BOTH_COMPOSITES: (bool, bool) = (true, true);

/// Whether the frames' centre pixel stands at the composition each one's
/// emission order predicts.
fn at_the_overlap(
    farther_first: &Rgba8Image,
    nearer_first: &Rgba8Image,
) -> Result<(bool, bool), Box<dyn std::error::Error>> {
    Ok((
        stands_at(farther_first, composed_when(Emitted::FartherFirst))?,
        stands_at(nearer_first, composed_when(Emitted::NearerFirst))?,
    ))
}

/// Whether `frame`'s declared overlap pixel stands within the tolerance of
/// `colour`.
fn stands_at(frame: &Rgba8Image, colour: [u8; 3]) -> Result<bool, Box<dyn std::error::Error>> {
    let shown = support::probe::pixel_color(frame, THE_OVERLAP)?;
    Ok(distance(shown, colour)? <= TELLS_THEM_APART)
}

/// How many pixels the two frames disagree about.
fn differing_pixels(one: &Rgba8Image, other: &Rgba8Image) -> u64 {
    every_pixel(one)
        .filter(|(x, y)| one.pixel(*x, *y) != other.pixel(*x, *y))
        .count() as u64
}

/// How many pixels show one composition in `one` and the other in `other`.
fn exchanged_pixels(
    one: &Rgba8Image,
    other: &Rgba8Image,
) -> Result<u64, Box<dyn std::error::Error>> {
    let (first, second) = (
        composed_when(Emitted::FartherFirst),
        composed_when(Emitted::NearerFirst),
    );
    let mut exchanged = 0;
    for (x, y) in every_pixel(one) {
        let (Some(here), Some(there)) = (one.pixel(x, y), other.pixel(x, y)) else {
            continue;
        };
        let (here, there) = ([here[0], here[1], here[2]], [there[0], there[1], there[2]]);
        if distance(here, first)? <= TELLS_THEM_APART
            && distance(there, second)? <= TELLS_THEM_APART
        {
            exchanged += 1;
        }
    }
    Ok(exchanged)
}

/// Every coordinate of a frame, row by row.
fn every_pixel(frame: &Rgba8Image) -> impl Iterator<Item = (u32, u32)> + '_ {
    let width = frame.width();
    (0..frame.height()).flat_map(move |y| (0..width).map(move |x| (x, y)))
}

/// Named so the count in the failure message reads against something.
const _: () = assert!(MANY_PIXELS == 100);
