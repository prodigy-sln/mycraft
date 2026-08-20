//! What a model has to be before it can be baked into a block texture.
//!
//! **One fixture carries the first two of these and it is chosen for that.**
//! The ramp cube is painted by `x` alone, so its `front`, `back`, `top` and
//! `bottom` faces each show four equal steps of 85 and wrap 255 back to black,
//! while its `left` and `right` faces see one slab apiece and are flat. A build
//! that refused on any face's verdict rather than on the verdicts of the faces
//! some entry selected would refuse the second of these — which is the whole
//! difference between a tool that bakes what a manifest asked for and one that
//! grades six images nobody wanted.
//!
//! The ramp's numbers are derived and not observed: `255 / 3 = 85` exactly, so
//! four tones over sixteen columns step by 85 within a row and by 255 across the
//! wrap. A fixture whose interior steps saturated would grade this leg at
//! nothing, since the wrap could never exceed them.

#[path = "common/build.rs"]
mod build;
mod common;

use std::error::Error;

use common::TestResult;
use common::texture::{GRADIENT, GREY, Leg, Legs, legs_named};
use mc_core::content::TEXTURE_EDGE;
use voxforge::inspect::ExitCode;

use build::{
    CUBE_MODEL, FIRST_KEY, Index, MANIFEST_FILE, Refused, Root, SECOND_KEY, block_of, built, entry,
    image_named, manifest, ramped_cube,
};

/// The step between two neighbouring tones of the ramp, within a row.
const STEP_WITHIN_A_ROW: &str = "85";

/// The step the ramp takes across the wrap, from its last column to its first.
const STEP_ACROSS_THE_WRAP: &str = "255";

/// A model one voxel short on `z`, and the axis a refusal has to name for it.
const SHORT_AXIS: &str = "z axis";
/// How far it reaches along that axis.
const VOXELS_ON_THE_SHORT_AXIS: &str = "15";

/// A model declaring half the scale a block texture is baked at.
const SMALL_MODEL: &str = "models/small.mcvox";
/// How many voxels it declares to a block.
const SMALL_MODEL_SCALE: u32 = 8;
/// How many pixels the manifest gives one voxel.
const PIXELS_PER_VOXEL: u32 = 3;
/// What those multiply to, which is not the edge a block texture has.
const THE_PRODUCT: &str = "24";
/// The edge it has to be.
const THE_EDGE: &str = "16";

/// A root holding the ramp cube, its four tones and two block files.
fn ramped_root() -> Result<Root, Box<dyn Error>> {
    let root = Root::bare()?;
    root.holding(CUBE_MODEL, &ramped_cube())?
        .painted(&GRADIENT)?
        .declaring(&[FIRST_KEY, SECOND_KEY])?;
    Ok(root)
}

#[test]
fn a_selected_face_whose_opposite_edges_disagree_refuses_the_build_naming_the_edge() -> TestResult {
    let root = ramped_root()?;
    root.holding(
        MANIFEST_FILE,
        &manifest(1, &[entry(FIRST_KEY, CUBE_MODEL, "front")]),
    )?;

    let made = built(&root)?;

    assert_eq!(
        (
            made.refusal(&[STEP_ACROSS_THE_WRAP, STEP_WITHIN_A_ROW]),
            legs_named(&made.err),
            made.images()
        ),
        (
            Refused::NamingEverything,
            Legs::Only(Leg::Edges),
            Vec::new()
        ),
        "a terrain quad merged across a run of blocks shows this texture over and over, so a wrap \
         that steps further than the texture's own interior draws a grid over every large flat \
         surface. The two numbers are what says *which* disagreement: the step across the wrap and \
         the largest step within the line it fails on. It said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_manifest_whose_selected_faces_all_tile_completes_the_build() -> TestResult {
    let root = ramped_root()?;
    root.holding(
        MANIFEST_FILE,
        &manifest(
            1,
            &[
                entry(FIRST_KEY, CUBE_MODEL, "left"),
                entry(SECOND_KEY, CUBE_MODEL, "right"),
            ],
        ),
    )?;

    let made = built(&root)?;
    let mut owed = vec![image_named(FIRST_KEY), image_named(SECOND_KEY)];
    owed.sort();

    assert_eq!(
        (made.code, made.images(), made.index().sorted()),
        (
            ExitCode::Success,
            owed,
            Index::Naming(vec![FIRST_KEY.to_owned(), SECOND_KEY.to_owned()])
        ),
        "the two faces this manifest asked for see one slab of the ramp apiece and are flat, so \
         they tile and the build completes — while the four faces nobody asked for do not tile at \
         all. This is the control for the refusal above: a build that judged every face it \
         rendered rather than every face an entry selected would refuse a set for a face the \
         manifest never wanted, and no positive scenario would notice. It said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_model_that_is_not_cubic_refuses_the_build_naming_the_axis_that_disagrees() -> TestResult {
    let root = Root::bare()?;
    let edge = TEXTURE_EDGE;
    root.holding(CUBE_MODEL, &block_of((edge, edge, edge - 1), edge, GREY))?
        .painted(&[GREY])?
        .holding(
            MANIFEST_FILE,
            &manifest(1, &[entry(FIRST_KEY, CUBE_MODEL, "front")]),
        )?;

    let made = built(&root)?;

    assert_eq!(
        (
            made.refusal(&[SHORT_AXIS, VOXELS_ON_THE_SHORT_AXIS]),
            made.images()
        ),
        (Refused::NamingEverything, Vec::new()),
        "a face set is a block's six faces, so a model that is not a cube of its declared scale \
         cannot be one. The axis is named rather than the three numbers alone, because an author \
         reading `16 by 16 by 15` still has to work out which of the three to change. This \
         refusal only exists on the whole-set path, which is why the build asks for all six faces \
         and picks from them rather than emitting one face per entry. It said: {err}",
        err = made.err
    );
    Ok(())
}

#[test]
fn a_model_whose_scale_times_pixels_per_voxel_is_not_the_edge_refuses_the_build() -> TestResult {
    let root = Root::bare()?;
    let small = SMALL_MODEL_SCALE;
    root.holding(SMALL_MODEL, &block_of((small, small, small), small, GREY))?
        .painted(&[GREY])?
        .holding(
            MANIFEST_FILE,
            &manifest(PIXELS_PER_VOXEL, &[entry(FIRST_KEY, SMALL_MODEL, "front")]),
        )?;

    let made = built(&root)?;

    assert_eq!(
        (
            made.refusal(&["small.mcvox", THE_PRODUCT, THE_EDGE]),
            made.images()
        ),
        (Refused::NamingEverything, Vec::new()),
        "a model's scale, a manifest's pixels per voxel and the edge a block texture has are three \
         numbers that can disagree, and the disagreement is caught here or nowhere useful: a set \
         baked at twenty-four builds cleanly, passes the gate, and refuses the launch with a \
         message about an *image*, pointing an author at a file they never authored. It said: \
         {err}",
        err = made.err
    );
    Ok(())
}
