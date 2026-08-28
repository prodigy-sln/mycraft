//! Whether anything in a frame is a mixture of two surfaces, asked of every
//! pixel at once.
//!
//! # Why this needs a whole-frame reading and not a region
//!
//! The other readings in this suite name the colours they expect and count them.
//! That answers "is the blend right where I looked". It cannot answer the
//! question this file is about — **is a blend the only thing a declared opacity
//! produced, anywhere in the picture** — because a pass that blended a surface
//! nobody declared translucent would put its mixture somewhere nobody named, and
//! a region-scoped reading would walk straight past it.
//!
//! `swatch.rs` is region-scoped by construction and nothing whole-frame existed,
//! so `pixel_census::classified` is new. Its shape is not free: it returns a
//! **total enumerated verdict**, never an absence, and it reports how many pixels
//! it looked at beside that verdict — because `assert!(strays.is_empty())` cannot
//! tell an empty answer from a scan that has stopped being able to look, and a
//! classifier whose colour list came back empty would answer "every pixel
//! accounted for" exactly as loudly as a correct one.
//!
//! # The three classes, and the qualifier that makes the third one narrow
//!
//! A pixel is accounted for when it stands at the clear colour, at a declared
//! layer's own colour, or **between two of those that adjoin it in screen
//! space** — and that last qualifier is the whole reading. Both source colours
//! have to appear within one pixel of the sample, so a boundary between two
//! surfaces is admitted and the *interior* of a region that is a mixture of two
//! layers is not: an interior pixel's neighbours are all the same mixture, and
//! none of them is either layer. That is what leaves a blended surface reported
//! rather than explained away by the class that exists for silhouettes.
//!
//! # One world, declared twice
//!
//! The two readings differ in exactly one character of one declaration file. The
//! world, the camera, the geometry and the art are identical, which is what makes
//! the difference between the two verdicts attributable to the declared degree
//! and to nothing else.

mod support;

use support::TestResult;
use support::pixel_census::{Accounting, Presence, classified};
use support::translucency::{Declared, PIXELS_IN_THE_FRAME, Pane, TELLS_THEM_APART, drawn};

/// The two blocks this world declares, and the flat colour each layer holds.
const WALL: &str = "example:wall";
const PANE: &str = "example:pane";
const WALL_COLOUR: [u8; 3] = [32, 200, 90];
const PANE_COLOUR: [u8; 3] = [235, 120, 40];

/// Where the two surfaces stand on the depth axis.
const WALL_PLANE: u32 = 0;
const PANE_PLANE: u32 = 4;

/// The one degree that is redeclared between the two readings.
const HALF: f32 = 0.5;

#[test]
fn a_world_whose_every_block_stops_all_the_light_leaves_no_pixel_unaccounted_for() -> TestResult {
    let declared = [
        Declared::opaque(WALL, WALL_COLOUR),
        Declared::opaque(PANE, PANE_COLOUR),
    ];

    let Some(shot) = drawn(&declared, &world())? else {
        return Ok(());
    };

    let classification = classified(&shot.frame, &shot.keys, &shot.texels, TELLS_THEM_APART)?;
    assert_eq!(
        (
            classification.considered,
            classification.verdict,
            classification.at_no_declared_colour,
        ),
        (
            PIXELS_IN_THE_FRAME,
            Accounting::EveryPixelAccounted,
            Presence::NotOnce,
        ),
        "every block this world declares stops all the light reaching it, so every pixel of the \
         frame is the colour the pass cleared to or the colour of one of the two layers — nothing \
         in it is a mixture of two surfaces, because nothing declared that it should be. The \
         count is asserted beside the verdict deliberately: a classifier that visited no pixel at \
         all reports the same verdict as a clean frame. First pixel nothing accounted for: {:?}",
        classification.first_unaccounted
    );
    Ok(())
}

#[test]
fn redeclaring_one_block_at_half_a_degree_puts_pixels_at_a_colour_no_layer_holds() -> TestResult {
    let declared = [
        Declared::opaque(WALL, WALL_COLOUR),
        Declared::opaque(PANE, PANE_COLOUR).at(HALF),
    ];

    let Some(shot) = drawn(&declared, &world())? else {
        return Ok(());
    };

    let classification = classified(&shot.frame, &shot.keys, &shot.texels, TELLS_THEM_APART)?;
    assert_eq!(
        (
            classification.considered,
            classification.verdict,
            classification.at_no_declared_colour,
        ),
        (
            PIXELS_IN_THE_FRAME,
            Accounting::PixelsAccountedByNothing,
            Presence::AtLeastMany,
        ),
        "this is the same world, the same camera and the same art as the reading above, with one \
         declaration stating half a degree instead of a whole one. That single character has to \
         put hundreds of pixels at a value that is no layer's own colour and no mixture of two \
         layers meeting on screen — which is what a blend is, and what nothing else in this \
         renderer produces. A verdict of every-pixel-accounted here is a declared degree that \
         reached no fragment"
    );
    Ok(())
}

/// The world both readings draw: a wall filling the section's far face, and a
/// pane inset in front of it.
///
/// The inset keeps the pane's projection strictly inside the wall's, so the
/// pixels the pane covers are pixels that would otherwise show the wall and
/// nothing else. A pane overhanging the wall would mix with the sky along its
/// edge and put a second kind of pixel into the second reading's answer.
fn world() -> [Pane; 2] {
    [
        Pane {
            block: WALL,
            plane: WALL_PLANE,
            x: 0..16,
            y: 0..16,
        },
        Pane {
            block: PANE,
            plane: PANE_PLANE,
            x: 3..13,
            y: 3..13,
        },
    ]
}
