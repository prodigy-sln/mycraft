//! What a declared degree of opacity draws, against the arithmetic it is owed.
//!
//! # Every expectation here is absolute, and none is a comparison
//!
//! Each expected colour is composited by hand from the two layers' own colours
//! and the degree the declaration states. Nothing below compares one rendering
//! to another, because a comparison cannot see a change that moved both — and a
//! blended pass that ignored its input entirely would move both.
//!
//! # And every one is computed in linear light
//!
//! The colour attachment is `Rgba8UnormSrgb`. The hardware decodes both
//! operands, mixes, and re-encodes, so an expectation computed on the stored
//! bytes is a different colour: over this fixture's own palette it stands
//! **ΔE 15.60** from the right answer at a half blend and **ΔE 9.72** at a
//! quarter, against a tolerance of 6. Both would be red against a correct frame,
//! and the cheapest green for either is a wider tolerance — which is why the
//! arithmetic lives in `support::art::composited` and not in any of these tests.
//!
//! # The fixture, and the distances it was built to hold
//!
//! Three flat-coloured blocks — a wall at `(32, 200, 90)`, a pane at
//! `(235, 120, 40)` and a blocker at `(120, 40, 160)` — with the sky at
//! `(135, 206, 235)`. **The closest two colours any reading in this file has to
//! tell apart stand ΔE 15.40 apart**: the sky against a quarter of the pane laid
//! over it. The pane's own colour against a half of it over the wall stand
//! ΔE 50.40 apart, and the wall against that same blend ΔE 54.52. Nothing
//! asserts that a fixture's colours are far enough apart, so
//! `pixel_census::require_told_apart` asserts it on every run instead, against
//! twice the tolerance — the separation two colours need for a pixel to be able
//! to belong to at most one of them.
//!
//! # Geometry, and why the panes are smaller than the wall
//!
//! Everything stands in one section, drawn from an eye at `(8, 8, 40)` looking
//! down `-Z`. The wall fills the section's whole far face; every pane stands
//! nearer the eye and is inset, so that **its projection lands strictly inside
//! the wall's** — a pane whose silhouette overhung the wall would blend against
//! the sky along its edge and put a colour in the frame that no reading here
//! names. The inset leaves thirteen pixels of margin at 256 square, derived from
//! the two depths and the declared sixty-degree field of view rather than read
//! off any frame.

mod support;

use mc_render::color::CLEAR_COLOR_SRGB;

use support::TestResult;
use support::pixel_census::{Expected, MANY_PIXELS, Presence, census, owed, require_told_apart};
use support::translucency::{Declared, PIXELS_IN_THE_FRAME, Pane, TELLS_THEM_APART, drawn};

/// The three blocks these readings declare, and the flat colour each layer is
/// filled with.
const WALL: &str = "example:wall";
const PANE: &str = "example:pane";
const BLOCKER: &str = "example:blocker";
const WALL_COLOUR: [u8; 3] = [32, 200, 90];
const PANE_COLOUR: [u8; 3] = [235, 120, 40];
const BLOCKER_COLOUR: [u8; 3] = [120, 40, 160];

/// Where each surface stands, as the emitting voxel's own coordinate on the
/// depth axis. A larger plane is nearer the eye, which stands at `z = 40`.
const WALL_PLANE: u32 = 0;
const PANE_PLANE: u32 = 4;
const BLOCKER_PLANE: u32 = 12;

/// The degrees these readings declare.
const HALF: f32 = 0.5;
const A_QUARTER: f32 = 0.25;
const NONE_AT_ALL: f32 = 0.0;

/// What the census calls each of the colours a frame here may hold.
const THE_SKY: &str = "the sky";
const THE_WALL: &str = "the wall, wherever nothing covers it";
const THROUGH_THE_PANE: &str = "the wall seen through the pane";
const SKY_THROUGH_THE_PANE: &str = "the sky seen through the pane";
const THE_PANE_ITSELF: &str = "the pane's own colour, unblended";
const THE_BLOCKER: &str = "the blocker standing in front of the pane";

/// The two presences these readings name.
const MANY: Presence = Presence::AtLeastMany;
const NONE: Presence = Presence::NotOnce;

#[test]
fn a_pane_at_half_a_degree_shows_the_wall_through_it_and_leaves_the_rest_of_the_wall_alone()
-> TestResult {
    let expected = wall_pane_and_the_blend_between_them();
    require_told_apart(&expected, TELLS_THEM_APART)?;

    let Some(shot) = drawn(&[opaque_wall(), pane_at(HALF)], &wall_behind_a_pane())? else {
        return Ok(());
    };

    let counted = census(&shot.frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (counted.considered, counted.shown.clone(), counted.strayed),
        (
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[MANY, MANY, MANY, NONE]),
            NONE,
        ),
        "one frame owes two answers at once: where a ray crosses the pane and then meets the \
         wall, the pixel is the even blend of the two blocks' own colours; where it meets the \
         wall directly, the pixel is the wall's own colour with nothing mixed into it. The pane's \
         own colour appearing anywhere at all is a pane drawn opaque. First stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

#[test]
fn a_pane_at_a_whole_degree_hides_the_wall_behind_it() -> TestResult {
    let expected = wall_pane_and_the_blend_between_them();
    require_told_apart(&expected, TELLS_THEM_APART)?;

    let Some(shot) = drawn(&[opaque_wall(), pane_at(1.0)], &wall_behind_a_pane())? else {
        return Ok(());
    };

    let counted = census(&shot.frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (counted.considered, counted.shown.clone(), counted.strayed),
        (
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[MANY, MANY, NONE, MANY]),
            NONE,
        ),
        "a whole degree is what a declaration that says nothing about opacity means, and it is \
         what every block in this repository draws at today. A ray crossing this pane and then \
         meeting the wall shows the pane and nothing of the wall — a pass that blended whatever it \
         was handed, or that read a degree belonging to another block, puts pixels on the third \
         line. First stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

#[test]
fn a_pane_at_a_quarter_of_a_degree_shows_three_quarters_of_the_sky_behind_it() -> TestResult {
    let expected = [
        Expected::new(THE_SKY, CLEAR_COLOR_SRGB),
        Expected::blend(
            SKY_THROUGH_THE_PANE,
            PANE_COLOUR,
            CLEAR_COLOR_SRGB,
            f64::from(A_QUARTER),
        ),
        Expected::new(THE_PANE_ITSELF, PANE_COLOUR),
    ];
    require_told_apart(&expected, TELLS_THEM_APART)?;

    let Some(shot) = drawn(&[pane_at(A_QUARTER)], &[inset_pane(PANE, PANE_PLANE)])? else {
        return Ok(());
    };

    let counted = census(&shot.frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (counted.considered, counted.shown.clone(), counted.strayed),
        (
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[MANY, MANY, NONE]),
            NONE
        ),
        "what stands behind this pane is the colour the pass cleared to, and a quarter of a \
         degree lets three quarters of it through. This is the reading the sRGB trap is sharpest \
         in: the same mix computed on the stored bytes lands ΔE 9.72 away, past the tolerance and \
         inside no other named colour — so it would read as a frame nothing accounts for rather \
         than as arithmetic done in the wrong space. First stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

#[test]
fn an_opaque_block_in_front_of_a_pane_is_drawn_with_nothing_mixed_into_it() -> TestResult {
    let expected = wall_pane_blocker_and_the_blend_between_them();
    require_told_apart(&expected, TELLS_THEM_APART)?;

    let Some(shot) = drawn(&three_blocks(), &a_pane_behind_a_blocker())? else {
        return Ok(());
    };

    let counted = census(&shot.frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (counted.considered, counted.shown.clone(), counted.strayed),
        (
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[MANY, MANY, MANY, NONE, NONE]),
            NONE,
        ),
        "the blocker stands nearer the eye than the pane and covers every pixel of it — the same \
         world extent at two thirds the distance projects half again as wide — so the pane \
         contributes to nothing. A blended pass ignoring the depth the opaque pass already wrote \
         mixes the pane into the blocker and puts pixels on the fourth or the fifth line. First \
         stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

#[test]
fn a_pane_at_no_degree_at_all_leaves_the_wall_behind_it_exactly_as_it_was() -> TestResult {
    let expected = wall_pane_and_the_blend_between_them();
    require_told_apart(&expected, TELLS_THEM_APART)?;

    let Some(shot) = drawn(
        &[opaque_wall(), pane_at(NONE_AT_ALL)],
        &wall_behind_a_pane(),
    )?
    else {
        return Ok(());
    };

    let counted = census(&shot.frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (counted.considered, counted.shown.clone(), counted.strayed),
        (
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[MANY, MANY, NONE, NONE]),
            NONE,
        ),
        "a block stopping none of the light reaching it contributes to no pixel at all, so the \
         wall behind it is drawn exactly as the wall beside it — one colour across the whole of \
         it, and nothing where the pane's edge falls. The wall standing at {MANY_PIXELS} pixels or \
         more is the second half: a frame in which it is a handful makes that true for a reason \
         that has nothing to do with the pane. First stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

/// The five a frame also holding an opaque blocker may hold.
fn wall_pane_blocker_and_the_blend_between_them() -> [Expected; 5] {
    [
        Expected::new(THE_SKY, CLEAR_COLOR_SRGB),
        Expected::new(THE_WALL, WALL_COLOUR),
        Expected::new(THE_BLOCKER, BLOCKER_COLOUR),
        blend_over_the_wall(HALF),
        Expected::new(THE_PANE_ITSELF, PANE_COLOUR),
    ]
}

/// The three blocks that reading declares, and the geometry it draws.
fn three_blocks() -> [Declared; 3] {
    [
        opaque_wall(),
        pane_at(HALF),
        Declared::opaque(BLOCKER, BLOCKER_COLOUR),
    ]
}

fn a_pane_behind_a_blocker() -> [Pane; 3] {
    [
        whole_wall(),
        inset_pane(PANE, PANE_PLANE),
        inset_pane(BLOCKER, BLOCKER_PLANE),
    ]
}

/// The four colours a wall-behind-a-pane frame may hold: the sky, the wall, the
/// blend, and the pane's own colour.
///
/// One list for four readings, because what separates them is the degree
/// declared and the presences owed — not the colours a frame may contain.
fn wall_pane_and_the_blend_between_them() -> [Expected; 4] {
    [
        Expected::new(THE_SKY, CLEAR_COLOR_SRGB),
        Expected::new(THE_WALL, WALL_COLOUR),
        blend_over_the_wall(HALF),
        Expected::new(THE_PANE_ITSELF, PANE_COLOUR),
    ]
}

/// The pane's colour laid over the wall's at `opacity`, in linear light.
fn blend_over_the_wall(opacity: f32) -> Expected {
    Expected::blend(
        THROUGH_THE_PANE,
        PANE_COLOUR,
        WALL_COLOUR,
        f64::from(opacity),
    )
}

/// The opaque backdrop, and a pane declaring `opacity` in front of it.
fn opaque_wall() -> Declared {
    Declared::opaque(WALL, WALL_COLOUR)
}

fn pane_at(opacity: f32) -> Declared {
    Declared::opaque(PANE, PANE_COLOUR).at(opacity)
}

/// The geometry four of these readings share.
fn wall_behind_a_pane() -> [Pane; 2] {
    [whole_wall(), inset_pane(PANE, PANE_PLANE)]
}

/// The backdrop: the whole of the section's far face.
fn whole_wall() -> Pane {
    Pane {
        block: WALL,
        plane: WALL_PLANE,
        x: 0..16,
        y: 0..16,
    }
}

/// A surface at `plane`, inset far enough that its projection lands strictly
/// inside the wall's.
///
/// **The inset is derived rather than eyeballed.** At the declared sixty-degree
/// field of view the wall's half-face subtends `8 / (39 tan 30°) = 0.355` of the
/// frame's half-width, and this surface at plane 4 subtends
/// `5 / (35 tan 30°) = 0.247` — thirteen pixels clear of the wall's edge at 256
/// square.
///
/// **The same world extent at a nearer plane necessarily covers a farther one**,
/// which is what makes one function enough for both the pane and the blocker
/// standing in front of it: at plane 12 the same rectangle subtends
/// `5 / (27 tan 30°) = 0.321`, nine pixels wider on every side than the pane it
/// hides.
fn inset_pane(block: &'static str, plane: u32) -> Pane {
    Pane {
        block,
        plane,
        x: 3..13,
        y: 3..13,
    }
}
