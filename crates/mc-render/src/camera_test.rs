//! The camera, the projection it implies, and which sections survive its
//! frustum.
//!
//! Every box below is placed against **one** camera, declared once: the eye at
//! the world origin looking down `-Z`, at a 60° vertical field of view, 1280 :
//! 720, near 0.5, far 512. That camera is chosen so the frustum's six
//! half-spaces are arithmetic anyone can redo by hand: for a point at depth `d`
//! in front of the eye the frustum admits `|x| <= d * 1.0264` and
//! `|y| <= d * 0.57735`, with `0.5 <= d <= 512`. The numbers in the fixtures are
//! derived from those three inequalities and from nothing the code under test
//! produced.
//!
//! **That near plane is this file's own, and no longer production's.** The
//! renderer's `NEAR_PLANE` is 0.1, so a player standing flush against a block
//! keeps the block; this fixture stays at 0.5 deliberately, and the divergence is
//! the point rather than an oversight. [`Frustum::from_view_projection`] is a
//! pure function of a *matrix*, and the near distance is an input to that matrix
//! — extracting the near half-space correctly is not a stronger claim for having
//! used the number production happens to hold. Reading it from
//! [`projection_for`] instead would make the `0.5 <= d <= 512` above a
//! restatement of a production constant, so a changed near plane would move the
//! fixture and its expectation together and the near case would keep passing
//! while quietly asserting something else each time. It would also shrink the
//! near exclusion box to depth 0.04..0.08 inside a frustum 0.08 wide, which is
//! where the "beyond exactly one plane, comfortably inside the other five"
//! property below stops being legible.
//!
//! What watches production's near plane is
//! `crates/mc-client/tests/camera_lens.rs`, which asserts the near rectangle's
//! corner radius against the half-width a player is actually stopped at. Named
//! here so a reader does not conclude from the paragraph above that nothing
//! does.
//!
//! **A box outside all six planes proves nothing.** The exclusion scenario is
//! six cases and each one puts its box beyond exactly one plane while leaving it
//! comfortably inside the other five — so a test inverted on a single plane
//! fails on that plane's case alone rather than being carried by the other five.
//! The near case is deliberately a box *in front of* the eye but nearer than the
//! near plane, which is what keeps it a different fact from the behind-the-camera
//! case below: a frustum test that only looked at distance would confuse them.
//!
//! **Every visible set asserted here is two-sided.** Each fixture carries a
//! section that must survive alongside one that must not, and the assertion is
//! on the whole returned set. An implementation that admits nothing and one that
//! admits everything both fail every test in this file, rather than each being
//! caught only by some other test's expense.
//!
//! `clippy::float_cmp` is denied and applies here: nothing below compares a
//! camera, frustum or projection value. The observable is a set of section
//! indices, which is integers.

use crate::aabb::Aabb;
use crate::geometry::scene::SectionRecord;

use crate::surface::SurfaceSize;

use super::{
    Frustum, Projection, camera_view, projection_for, view_projection, visible_sections,
    waiting_view,
};

/// The eye position every fixture is placed against.
const EYE: [f32; 3] = [0.0, 0.0, 0.0];

/// What the camera looks at: straight down `-Z`, which is the direction a
/// right-handed view matrix looks along.
const TARGET: [f32; 3] = [0.0, 0.0, -64.0];

/// The lens every fixture below is placed under. The replay declares these
/// three, and the fixtures would still be hand-derivable if it stopped.
const FOV_Y_DEGREES: f32 = 60.0;
const ASPECT: f32 = 1280.0 / 720.0;
const FAR: f32 = 512.0;

/// How near this fixture's own near plane stands.
///
/// **Not the renderer's**, which is 0.1 so that a block the player is pressed
/// against is not clipped away. Kept at 0.5 on purpose: it is an input to the
/// matrix under test rather than a fact about it, it leaves the near exclusion
/// case a box at depth 0.2..0.4 where the arithmetic is still legible, and
/// taking it from `projection_for` would make the inequality this file's header
/// derives by hand agree with production by construction. See the header.
const NEAR: f32 = 0.5;

/// A box centred on the view axis at depth 96..104, well inside all six planes:
/// at the nearest of those depths the frustum reaches x = +/-98.5 and
/// y = +/-55.4, and this box reaches 4.
const INSIDE_ALL_SIX: Aabb = Aabb {
    min: [-4.0, -4.0, -104.0],
    max: [4.0, 4.0, -96.0],
};

/// A box at depth 480..540, so the far plane at 512 passes through it.
const STRADDLING_FAR: Aabb = Aabb {
    min: [-4.0, -4.0, -540.0],
    max: [4.0, 4.0, -480.0],
};

/// A box at depth 520..560 — entirely past the far plane, and inside the other
/// five: at depth 520 the frustum still reaches x = +/-533.
const BEYOND_FAR: Aabb = Aabb {
    min: [-4.0, -4.0, -560.0],
    max: [4.0, 4.0, -520.0],
};

/// A box entirely behind the eye. `-Z` is forward, so every corner here sits at
/// a negative depth, and the frustum's own inequalities are meaningless for it.
const BEHIND_THE_CAMERA: Aabb = Aabb {
    min: [-4.0, -4.0, 50.0],
    max: [4.0, 4.0, 100.0],
};

/// The six boxes of the exclusion scenario, each beyond exactly one plane.
///
/// The four side cases share the depth band 96..104, where the frustum reaches
/// x = +/-98.5 and y = +/-55.4 at its narrowest, and sit at 300..400 on the axis
/// they are beyond — three times outside, and inside every other half-space.
/// The near case sits at depth 0.2..0.4, in front of the eye but short of the
/// 0.5 near plane, where the frustum is 0.2 wide and the box is 0.04.
const BEYOND_EACH_PLANE: [(&str, Aabb); 6] = [
    (
        "nearer than the near plane",
        Aabb {
            min: [-0.02, -0.02, -0.40],
            max: [0.02, 0.02, -0.20],
        },
    ),
    ("past the far plane", BEYOND_FAR),
    (
        "left of the left plane",
        Aabb {
            min: [-400.0, -4.0, -104.0],
            max: [-300.0, 4.0, -96.0],
        },
    ),
    (
        "right of the right plane",
        Aabb {
            min: [300.0, -4.0, -104.0],
            max: [400.0, 4.0, -96.0],
        },
    ),
    (
        "below the bottom plane",
        Aabb {
            min: [-4.0, -400.0, -104.0],
            max: [4.0, -300.0, -96.0],
        },
    ),
    (
        "above the top plane",
        Aabb {
            min: [-4.0, 300.0, -104.0],
            max: [4.0, 400.0, -96.0],
        },
    ),
];

/// Where the section that must survive sits in every fixture below.
const SURVIVOR: u32 = 0;

/// The frustum of the one declared camera.
fn declared_frustum() -> Frustum {
    let view = camera_view(EYE, TARGET);
    let projection = Projection {
        fov_y_radians: FOV_Y_DEGREES.to_radians(),
        aspect: ASPECT,
        near: NEAR,
        far: FAR,
    };
    Frustum::from_view_projection(&view_projection(&view, &projection))
}

/// A section whose only interesting property is the box the culling pass tests.
fn section_holding(aabb: Aabb) -> SectionRecord {
    SectionRecord {
        origin: [0, 0, 0],
        first_quad: 0,
        quad_count: 1,
        // Every quad of this section stops all the light reaching it, which is
        // what a section holding no declared translucency reports. Nothing the
        // frustum answers depends on it.
        opaque_quad_count: 1,
        aabb,
    }
}

#[test]
fn a_section_beyond_any_single_frustum_plane_is_left_out_of_the_visible_set() {
    let frustum = declared_frustum();

    let observed = BEYOND_EACH_PLANE.map(|(plane, outside)| {
        let sections = [section_holding(INSIDE_ALL_SIX), section_holding(outside)];
        (plane, visible_sections(&frustum, &sections))
    });
    let expected = BEYOND_EACH_PLANE.map(|(plane, _)| (plane, vec![SURVIVOR]));

    assert_eq!(
        observed, expected,
        "each of these boxes lies beyond one plane and inside the other five, so each must \
         leave the visible set holding only the section that is inside all six; a frustum \
         test inverted on one plane fails exactly one of these cases"
    );
}

#[test]
fn a_section_inside_all_six_planes_is_kept_in_the_visible_set() {
    let frustum = declared_frustum();
    let sections = [section_holding(INSIDE_ALL_SIX), section_holding(BEYOND_FAR)];

    assert_eq!(
        visible_sections(&frustum, &sections),
        vec![SURVIVOR],
        "a box at depth 96..104 reaching 4 blocks off the view axis sits inside every plane \
         of a 60-degree frustum and must be drawn; the second section is past the far plane \
         and must not be, so neither an admit-nothing nor an admit-everything test passes"
    );
}

#[test]
fn a_section_the_far_plane_cuts_through_is_kept_in_the_visible_set() {
    let frustum = declared_frustum();
    let sections = [section_holding(STRADDLING_FAR), section_holding(BEYOND_FAR)];

    assert_eq!(
        visible_sections(&frustum, &sections),
        vec![SURVIVOR],
        "the far plane at 512 passes through the first box (depth 480..540), so part of it \
         is drawable and it must survive; the second box starts at depth 520 and is entirely \
         past the plane, which is what pins where the plane actually is"
    );
}

#[test]
fn a_section_entirely_behind_the_camera_is_left_out_of_the_visible_set() {
    let frustum = declared_frustum();
    let sections = [
        section_holding(INSIDE_ALL_SIX),
        section_holding(BEHIND_THE_CAMERA),
    ];

    assert_eq!(
        visible_sections(&frustum, &sections),
        vec![SURVIVOR],
        "every corner of the second box sits at a negative depth; a frustum test that \
         compares an unsigned distance against the side planes admits it and draws the \
         world behind the viewer"
    );
}

/// The camera used before there is anything to draw is still a camera.
///
/// Nothing is drawn through it, which is exactly why it is worth asserting: a
/// pose whose eye and target coincide has no look direction, and the view matrix
/// of one is entirely `NaN`. The frame path builds that matrix for every
/// snapshot whatever the phase — the statistics do it before the phase is
/// consulted — so a degenerate waiting pose would put `NaN` through the frustum
/// on every frame of the load, silently, with a clear colour on screen that
/// looks exactly like a correct one.
#[test]
fn the_camera_used_before_anything_is_drawn_yields_a_matrix_of_real_numbers() {
    let size = SurfaceSize {
        width: 1280,
        height: 720,
    };

    let matrix = view_projection(&waiting_view(), &projection_for(size));

    assert!(
        matrix.to_cols_array().iter().all(|value| value.is_finite()),
        "the waiting pose has to have a look direction: {matrix:?} carries a value that is \
         not a number, which is what an eye standing exactly on its own target produces"
    );
}
