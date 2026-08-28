//! Two see-through surfaces over one opaque one: what their composition is
//! worth, what a texture's own alpha adds to it, and what stands between two
//! cells of one kind.
//!
//! # The composition is a third colour and that is what makes it assertable
//!
//! One pane at half a degree over the wall gives `(174, 166, 70)`; two of them
//! give `(207, 145, 57)`; the pane's own unblended colour is `(235, 120, 40)`.
//! The three stand ΔE 25.38, 25.06 and 50.40 apart pairwise, so "the composition
//! of both is a distinct colour from the one either cell alone produces" is a
//! statement a census can make rather than a claim a comment has to carry.
//!
//! The expectation for two panes is the closed form —
//! `C(1 - (1-a)²) + D(1-a)²` — rather than the composite applied twice. Both
//! answer the same triple here, measured; the closed form is the one that does
//! not depend on how many times the hardware rounds through an eight-bit
//! attachment on the way.
//!
//! # A texture's alpha multiplies the declared degree, it does not replace it
//!
//! The model takes the **product**: the declared degree decides which draw a
//! face lands in, and the texel's alpha modulates inside the blended one. So a
//! pane declaring half a degree whose texture carries alpha 128 blends at
//! `0.5 x 128/255 = 0.251`, and a pane at that same degree whose texture is
//! opaque blends at the degree alone. That is what the scenario states, and the
//! reading below asserts both halves in one frame. Its wording was amended to
//! the product model during this phase — `decisions.md`, D-I5.
//!
//! **Two panes differing in exactly one byte is what makes the texel's alpha
//! falsifiable**, and it is the whole shape of the reading rather than a
//! convenience. A single pane says nothing: whichever colour it draws is
//! consistent with an alpha taken from the declaration alone, from the texture
//! alone, or from the product of the two, because one number can be arrived at
//! three ways. Two panes of one colour at one degree, separated by nothing but
//! the alpha their textures carry, cannot be — a draw path reading the
//! declaration alone puts both on the same colour, one reading the texture alone
//! puts the sheer pane at an even blend instead of at a quarter, and only the
//! product puts each where the census expects. The two expected colours stand
//! ΔE 26.43 apart, better than four times the tolerance.
//!
//! # A seam is an extra run, which is how one is seen without a committed pixel
//!
//! `runs_across` reads one row of the frame and names the colours it passes
//! through, in order, collapsing repeats. Two abutting cells of one kind draw one
//! run of the composite; anything drawn between them — a doubled blend where the
//! two overlap by a fraction of a pixel, a gap showing the wall, a line of any
//! colour at all — splits that run into three. The reading names a colour it
//! cannot place rather than skipping it, so a seam of an unforeseen colour is
//! reported instead of quietly joining the run beside it.
//!
//! **The positive control cannot be a rendered world.** The engine draws no seam
//! between two cells of one kind: `sweep.rs` emits no face between them and the
//! field that would let content ask for one is deferred. So the control is the
//! rendered frame with a seam painted into it, at the column where the two cells
//! meet, fed to the same reading.

mod support;

use std::ops::Range;

use mc_render::color::CLEAR_COLOR_SRGB;

use support::TestResult;
use support::pixel_census::{
    Expected, Presence, census, owed, require_told_apart, runs_across, with_a_seam_painted_down,
};
use support::translucency::{Declared, FRAME, PIXELS_IN_THE_FRAME, Pane, TELLS_THEM_APART, drawn};

/// The blocks these readings declare.
///
/// `PANE` and `SHEER_PANE` carry the same colour and differ only in the alpha
/// their texture holds, which is what makes one reading about that byte.
const WALL: &str = "example:wall";
const PANE: &str = "example:pane";
const SHEER_PANE: &str = "example:sheer_pane";
const WALL_COLOUR: [u8; 3] = [32, 200, 90];
const PANE_COLOUR: [u8; 3] = [235, 120, 40];

/// The alpha a texture carries where a reading is about that byte, and the alpha
/// every other texture in this suite carries.
const HALF_A_TEXEL: u8 = 128;
const AN_OPAQUE_TEXEL: u8 = 255;

/// Where each surface stands on the depth axis. A larger plane is nearer the
/// eye, which stands at `z = 40`.
const WALL_PLANE: u32 = 0;
const NEAR_PLANE: u32 = 8;
const FAR_PLANE: u32 = 4;

/// The degree every translucent block here declares.
const HALF: f32 = 0.5;

/// What the census calls each colour.
const THE_SKY: &str = "the sky";
const THE_WALL: &str = "the wall, wherever nothing covers it";
const ONE_PANE: &str = "one pane over the wall";
const TWO_PANES: &str = "two panes over the wall";
const A_SHEER_PANE: &str = "the pane whose texture is half clear, over the wall";
const THE_PANE_ITSELF: &str = "the pane's own colour, unblended";

/// The two presences these readings name, spelled short so an expectation reads
/// as a list rather than as a paragraph.
const MANY: Presence = Presence::AtLeastMany;
const NONE: Presence = Presence::NotOnce;

/// The row a seam is read across, and the column the two abutting cells meet in.
///
/// Both are derived from the fixture's own symmetry rather than found in a
/// frame: the eye looks at `(8, 8)`, the two cells meet at world `x = 8`, and a
/// point on the view axis projects to the middle of the frame.
const THE_MIDDLE_ROW: u32 = FRAME.height >> 1;
const WHERE_THE_TWO_CELLS_MEET: u32 = FRAME.width >> 1;

#[test]
fn a_textures_own_alpha_multiplies_the_degree_its_block_declares() -> TestResult {
    let sheer = Declared::opaque(SHEER_PANE, PANE_COLOUR)
        .at(HALF)
        .textured_at_alpha(HALF_A_TEXEL);
    let expected = classes_telling_the_two_textures_apart();
    require_told_apart(&expected, TELLS_THEM_APART)?;
    let panes = [
        whole_wall(),
        inset(PANE, FAR_PLANE, 3..8),
        inset(SHEER_PANE, FAR_PLANE, 8..13),
    ];

    let Some(shot) = drawn(&[wall(), pane(), sheer], &panes)? else {
        return Ok(());
    };

    let counted = census(&shot.frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (counted.considered, counted.shown.clone(), counted.strayed),
        (
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[MANY, MANY, MANY, MANY, NONE]),
            NONE,
        ),
        "these two panes declare the same degree and are drawn in the same colour; the only thing \
         separating them is the alpha their textures carry, 255 against 128. A draw path taking \
         its alpha from the declaration alone puts both halves of the frame on the third line and \
         nothing on the fourth; one taking it from the texture alone puts the sheer pane at an \
         even blend rather than at a quarter. First stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

#[test]
fn two_separated_panes_over_one_wall_compose_to_a_colour_neither_reaches_alone() -> TestResult {
    let expected = classes_over_the_wall();
    require_told_apart(&expected, TELLS_THEM_APART)?;
    let panes = [
        whole_wall(),
        inset(PANE, FAR_PLANE, 3..13),
        inset(PANE, NEAR_PLANE, 3..8),
    ];

    let Some(shot) = drawn(&[wall(), pane()], &panes)? else {
        return Ok(());
    };

    let counted = census(&shot.frame, &expected, TELLS_THEM_APART)?;
    assert_eq!(
        (counted.considered, counted.shown.clone(), counted.strayed),
        (
            PIXELS_IN_THE_FRAME,
            owed(&expected, &[MANY, MANY, MANY, MANY, NONE]),
            NONE,
        ),
        "the two cells are of one kind and stand four blocks apart, so a ray crossing both draws \
         two faces and a ray crossing one draws one. Both answers have to be in the frame at once \
         and they have to be different colours — a pass that drew only the nearer translucent \
         face, or that let the second overwrite the first instead of composing with it, puts the \
         whole covered region on the third line. First stray: {:?}",
        counted.first_stray
    );
    Ok(())
}

#[test]
fn two_adjacent_cells_of_one_kind_draw_one_unbroken_run_and_a_painted_seam_is_reported()
-> TestResult {
    let expected = classes_over_the_wall();
    require_told_apart(&expected, TELLS_THEM_APART)?;
    let panes = [
        whole_wall(),
        inset(PANE, FAR_PLANE, 3..8),
        inset(PANE, FAR_PLANE, 8..13),
    ];

    let Some(shot) = drawn(&[wall(), pane()], &panes)? else {
        return Ok(());
    };

    let seamed = with_a_seam_painted_down(&shot.frame, WHERE_THE_TWO_CELLS_MEET, doubled().colour)?;
    assert_eq!(
        (
            runs_across(&shot.frame, THE_MIDDLE_ROW, &expected, TELLS_THEM_APART)?,
            runs_across(&seamed, THE_MIDDLE_ROW, &expected, TELLS_THEM_APART)?,
        ),
        (
            vec![THE_SKY, THE_WALL, ONE_PANE, THE_WALL, THE_SKY],
            vec![
                THE_SKY, THE_WALL, ONE_PANE, TWO_PANES, ONE_PANE, THE_WALL, THE_SKY,
            ],
        ),
        "the two cells are of one kind and stand side by side, so what crosses the frame is one \
         unbroken run of the composite: five runs, and the join between the cells is not one of \
         the boundaries. The second half is what says the reading could have seen a seam — the \
         same row of the same frame with one column of a doubled blend painted in where the two \
         cells meet, which is what an engine drawing the face between them would produce, and \
         which splits that run into three"
    );
    Ok(())
}

/// The five colours a frame holding one kind of pane may show.
fn classes_over_the_wall() -> [Expected; 5] {
    [
        Expected::new(THE_SKY, CLEAR_COLOR_SRGB),
        Expected::new(THE_WALL, WALL_COLOUR),
        single(),
        doubled(),
        Expected::new(THE_PANE_ITSELF, PANE_COLOUR),
    ]
}

/// The five a frame holding two panes of one colour and two textures may show.
fn classes_telling_the_two_textures_apart() -> [Expected; 5] {
    [
        Expected::new(THE_SKY, CLEAR_COLOR_SRGB),
        Expected::new(THE_WALL, WALL_COLOUR),
        single(),
        Expected::blend(
            A_SHEER_PANE,
            PANE_COLOUR,
            WALL_COLOUR,
            product(HALF_A_TEXEL),
        ),
        Expected::new(THE_PANE_ITSELF, PANE_COLOUR),
    ]
}

/// The degree a pane at [`HALF`] blends at when its texture carries `stored`.
///
/// The alpha channel of an `Rgba8UnormSrgb` texture is **not** put through the
/// transfer function — the format's own definition — so the texel's share is a
/// division and not a decode, and the two multiply.
fn product(stored: u8) -> f64 {
    f64::from(HALF) * f64::from(stored) / f64::from(u8::MAX)
}

/// One pane at half a degree over the wall, its texture opaque.
fn single() -> Expected {
    Expected::blend(ONE_PANE, PANE_COLOUR, WALL_COLOUR, product(AN_OPAQUE_TEXEL))
}

/// Two of them, by the closed form of `src-over` applied twice at one degree.
fn doubled() -> Expected {
    let through_both = 1.0 - (1.0 - product(AN_OPAQUE_TEXEL)).powi(2);
    Expected::blend(TWO_PANES, PANE_COLOUR, WALL_COLOUR, through_both)
}

/// The opaque backdrop, and the pane that stands in front of it.
fn wall() -> Declared {
    Declared::opaque(WALL, WALL_COLOUR)
}

fn pane() -> Declared {
    Declared::opaque(PANE, PANE_COLOUR).at(HALF)
}

/// The whole of the section's far face.
fn whole_wall() -> Pane {
    Pane {
        block: WALL,
        plane: WALL_PLANE,
        x: 0..16,
        y: 0..16,
    }
}

/// A surface at `plane` spanning `x`, inset far enough vertically and in depth
/// that its projection lands strictly inside the wall's — `translucency`'s own
/// note carries the derivation.
fn inset(block: &'static str, plane: u32, x: Range<u32>) -> Pane {
    Pane {
        block,
        plane,
        x,
        y: 3..13,
    }
}
