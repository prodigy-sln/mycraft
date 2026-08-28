//! What the ray-marched judge predicts once a block passes light, and where the
//! numbers it predicts from come from.
//!
//! # Every reading here is about the judge and not about a frame
//!
//! Nothing below draws anything. A prediction is a statement about the world,
//! the camera and what the content declares, so these run on a machine with no
//! GPU at all — which is the point, because the instrument they are about is the
//! one every frame reading in this suite is graded against. An instrument nobody
//! checks is a set of expectations nobody checks.
//!
//! # Why the sea and not a world of this file's own
//!
//! The world is the shipped one, generated from the shipped root with the one
//! field this spec adds written into the sea's own declaration
//! (`support::content::shipped_with_the_sea_declaring`). So the strata, the
//! seed, the art and the four declarations are all the shipped ones and only the
//! degree is the fixture's. A world assembled here instead would be a world the
//! product never generates, and the run rule below is precisely a rule about
//! what real bodies of water look like — one to two cells deep, entered at a
//! grazing angle, with an air gap where the shore breaks them.
//!
//! # The camera is the player's own at the closing tick
//!
//! Taken from the simulation rather than declared, because what these readings
//! need is a pose that crosses a lot of sea and the walk already provides one:
//! at tick 119 the march classifies 204 of 576 samples as crossing the sea,
//! against 56 at the opening tick. Nothing here asserts that number — it is why
//! the tick was chosen, and the readings state their own premises.

mod support;

use std::error::Error;
use std::fs;

use glam::Vec3;
use mc_core::block::{BlockRegistry, Opacity};
use mc_core::content::{LayerAssignment, TEXTURE_EDGE};
use mc_core::id::{BlockName, TextureKey};
use mc_render::texture::supplied::SuppliedTexels;
use mc_sim::camera::CameraPose;
use mc_world::content::LuauFileDefinitionSource;
use mc_world::mesh::Facing;

use support::composite::Palette;
use support::content::{ContentRoot, shipped_copy, shipped_with_the_sea_declaring};
use support::frames::CAPTURE_SIZE;
use support::march::{Basis, Lens, March};
use support::oracle::{self, Crossed, Sighted, Surface, Voxels};
use support::{TestResult, prepare_scene_at, repository_root};

/// The block the shipped world's only translucent body is made of.
const SEA: &str = "base:water";

/// The degree the sea declares in every fixture here.
const HALF: f32 = 0.5;

/// The tick whose camera crosses the most sea. See this module's header.
const CLOSING: u32 = 119;

#[test]
fn a_ray_crossing_many_cells_of_one_sea_is_predicted_as_one_layer_over_what_lies_beyond()
-> TestResult {
    let root = shipped_with_the_sea_declaring(HALF)?;
    let prepared = prepare_scene_at(root.path())?;
    let voxels = Voxels {
        world: &prepared.world,
        registry: &prepared.registry,
    };
    let camera = support::frames::player_pose(CLOSING, &prepared.world, &prepared.registry)?;

    let deepest = deepest_crossing(&camera, &voxels)?;

    assert_eq!(
        (
            deepest.cells_of_sea > 1,
            deepest.layers,
            deepest.classified.as_str(),
        ),
        (true, 1, "base:water over base:grass"),
        "the ray through {:?} passes through {} cells of the sea and the engine draws it one \
         face: a block draws no face against its own kind, and the face the ray leaves the run \
         through has its normal along the ray and is culled. So a maximal run of one kind is one \
         layer. A judge accumulating a layer per *voxel* would answer {} of them here, and a \
         reading graded against that prediction would report a renderer defect that is not there",
        deepest.sample,
        deepest.cells_of_sea,
        deepest.cells_of_sea
    );
    Ok(())
}

#[test]
fn the_colour_predicted_for_a_ray_crossing_the_sea_is_neither_layers_own() -> TestResult {
    let root = shipped_with_the_sea_declaring(HALF)?;
    let prepared = prepare_scene_at(root.path())?;
    let voxels = Voxels {
        world: &prepared.world,
        registry: &prepared.registry,
    };
    let palette = Palette::of(&prepared.registry, &prepared.resolution, &prepared.texels);
    let camera = support::frames::player_pose(CLOSING, &prepared.world, &prepared.registry)?;

    let deepest = deepest_crossing(&camera, &voxels)?;
    let composed = palette.predicted_mean(&deepest.crossed)?;
    let sea = palette.mean_of(deepest.crossed.layers.first().ok_or(NO_LAYER)?)?;
    let beyond = palette.mean_of(deepest.crossed.beyond.as_ref().ok_or(NOTHING_BEYOND)?)?;

    assert_eq!(
        (composed == sea, composed == beyond, composed),
        (false, false, [107, 119, 124]),
        "the judge has to predict the *blend* and not the nearer block's own colour, or every \
         frame reading graded against it would be asking for a sea drawn as though it stopped all \
         the light. Composed {composed:?} from the sea's own {sea:?} over the lakebed's own \
         {beyond:?} at the declared degree of {HALF}, in linear light. The triple is stated rather \
         than derived a second time here because a comparison against arithmetic written twice in \
         one file is a comparison with itself"
    );
    Ok(())
}

#[test]
fn a_prediction_composes_from_the_declared_degree_and_never_from_the_byte_a_vertex_carries()
-> TestResult {
    let root = two_sheets()?;
    let registry = registry_over(&root)?;
    let loaded = mc_sim::content::load(root.path(), &LayerAssignment::none())?;
    let resolution = mc_client::content::ContentView::of(&loaded.resolved).into_resolution();
    let texels = flat_layers(&[(SHEER, WHITE), (DARK, BLACK)])?;
    let palette = Palette::of(&registry, &resolution, &texels);

    let composed = palette.predicted_mean(&a_sheer_sheet_over_a_dark_one()?)?;

    assert_eq!(
        composed,
        EXACTLY,
        "the prediction is composed from the degree a declaration *states*, and `{SHEER}` states \
         {A_SLIVER}. Rounded to the byte a packed vertex carries it would be \
         {quantised}/255, and this composition would answer {THROUGH_THE_BYTE:?} instead — six \
         code values away, because near black the transfer function's slope is 12.92 and a \
         rounding of half a code value in the degree is amplified rather than lost. Every other \
         pair of colours in this project hides that difference under a third of a code value, \
         which is exactly why a reading that would notice needs a pair chosen for it",
        quantised = Opacity::new(A_SLIVER).ok_or(NOT_A_DEGREE)?.quantised()
    );
    Ok(())
}

#[test]
fn the_judge_and_its_composition_name_nothing_from_the_draw_path_they_grade() -> TestResult {
    let mut naming = Vec::new();
    for module in THE_JUDGES_OWN_SOURCES {
        naming.extend(draw_path_names_in(&read_support(module)?, module));
    }

    assert_eq!(
        Independence::of(naming),
        Independence::NothingFromTheDrawPath,
        "the prediction has to share no code with the draw path it grades, and these are the two \
         modules that compose one. The names looked for are the ones that would quietly end that: \
         `quantised` is the encoding a packed vertex carries, `opacity_of` is the degree the \
         *packer* partitions on, and the three module paths are where the geometry, the pipelines \
         and the mip chain live. Comment lines are skipped, so a paragraph naming one of them as \
         the thing not to do is not a use of it"
    );
    Ok(())
}

#[test]
fn that_same_reading_reports_a_module_that_does_name_one() -> TestResult {
    let doctored = format!(
        "{}\n    let byte = self.degree.quantised();\n",
        read_support(COMPOSITION)?
    );

    assert_eq!(
        Independence::of(draw_path_names_in(&doctored, COMPOSITION)),
        Independence::Names(vec![format!("`{COMPOSITION}` names `quantised`")]),
        "a scan asserting only an absence goes green forever the day it stops being able to look, \
         so the same scan is driven over the same module with one line added that reaches for the \
         encoding. It has to report that line rather than answering that it found nothing"
    );
    Ok(())
}

/// What a reading of the two modules' independence came to.
///
/// **An enumerated verdict and never an emptiness.** `NothingFromTheDrawPath`
/// rejects every other answer including "I could not look", so a module that had
/// gone missing or a scan that had stopped scanning reddens rather than reading
/// as a clean tree.
#[derive(Debug, PartialEq, Eq)]
enum Independence {
    NothingFromTheDrawPath,
    Names(Vec<String>),
}

impl Independence {
    /// The verdict `naming` amounts to.
    fn of(naming: Vec<String>) -> Self {
        if naming.is_empty() {
            Self::NothingFromTheDrawPath
        } else {
            Self::Names(naming)
        }
    }
}

/// The two modules that compose a prediction between them.
const THE_JUDGES_OWN_SOURCES: [&str; 2] = [JUDGE, COMPOSITION];
const JUDGE: &str = "oracle.rs";
const COMPOSITION: &str = "composite.rs";

/// The names whose presence would mean the prediction had started sharing the
/// draw path's own arithmetic.
const OF_THE_DRAW_PATH: [&str; 6] = [
    "quantised",
    "opacity_of",
    "mc_render::geometry",
    "mc_render::gpu",
    "texture::mip",
    "levels_for",
];

/// Every draw-path name `source` uses, outside its comments.
fn draw_path_names_in(source: &str, module: &str) -> Vec<String> {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .flat_map(|line| OF_THE_DRAW_PATH.iter().map(move |name| (line, *name)))
        .filter(|(line, name)| line.contains(name))
        .map(|(_line, name)| format!("`{module}` names `{name}`"))
        .collect()
}

/// One module of this suite's support tree, as it stands on disk.
fn read_support(module: &str) -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(
        repository_root()?
            .join("crates")
            .join("mc-client")
            .join("tests")
            .join("support")
            .join(module),
    )?)
}

/// The two blocks the quantisation reading is about, and what fills their
/// layers.
///
/// **White over black, and neither is decoration.** The degree's rounding error
/// is at most half a code value, which over ordinary colours moves a composite
/// by under a third of one — invisible. Against black the sRGB transfer function
/// is a straight line of slope 12.92, so the same error lands six code values
/// apart, and a reading can see it.
const SHEER: &str = "fixture:sheer";
const DARK: &str = "fixture:dark";
const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];

/// The degree `fixture:sheer` declares.
///
/// Chosen so that the byte it quantises to — 1 of 255 — stands almost twice the
/// declared degree, which is the widest relative gap the encoding has anywhere.
const A_SLIVER: f32 = 0.002;

/// What composing from the declared degree gives, and what composing from the
/// byte would give instead.
const EXACTLY: [u8; 3] = [7, 7, 7];
const THROUGH_THE_BYTE: [u8; 3] = [13, 13, 13];

/// What a run reports when the fixture's own degree is not one the engine keeps.
const NOT_A_DEGREE: &str =
    "this fixture's degree is not one an `Opacity` admits, so there is no byte to compare against";

/// What a run reports when the deepest crossing carries no layer at all.
const NO_LAYER: &str =
    "the deepest crossing of the sea carries no layer, so there is nothing composed to be about";

/// What a run reports when the deepest crossing met nothing at all.
const NOTHING_BEYOND: &str = "the deepest crossing met no opaque surface, so there is nothing for the sea to be composed \
     over and the reading below would be about the sky";

/// One sheet of `fixture:sheer` standing in front of one of `fixture:dark`, as
/// a ray that crossed both would report them.
fn a_sheer_sheet_over_a_dark_one() -> Result<Crossed, Box<dyn Error>> {
    Ok(Crossed {
        layers: vec![Surface {
            block: BlockName::parse(SHEER)?,
            facing: Some(Facing::PosZ),
            along: 1.0,
        }],
        beyond: Some(Surface {
            block: BlockName::parse(DARK)?,
            facing: Some(Facing::PosZ),
            along: 2.0,
        }),
    })
}

/// A content root declaring the two blocks the quantisation reading needs.
///
/// `occludes = false` is stated on the sheer one because `occludes` falls back
/// to `solid`: a solid block that passed light and hid what lay behind it is a
/// contradiction the loader refuses, taking the whole root with it.
fn two_sheets() -> Result<ContentRoot, Box<dyn Error>> {
    let mut root = shipped_copy()?.declaring_no_blocks()?;
    root = root.declaring_block(
        "sheer.luau",
        &format!(
            "return {{\n\tname = \"{SHEER}\",\n\ttexture = \"{SHEER}\",\n\tsolid = true,\n\
             \toccludes = false,\n\topacity = {A_SLIVER:?},\n}}\n"
        ),
    )?;
    root.declaring_block(
        "dark.luau",
        &format!("return {{\n\tname = \"{DARK}\",\n\ttexture = \"{DARK}\",\n\tsolid = true,\n}}\n"),
    )
}

/// A registry holding exactly what `root` declares.
fn registry_over(root: &ContentRoot) -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(root.path().to_path_buf()))?;
    Ok(registry)
}

/// A layer of one flat colour for each named key.
fn flat_layers(declared: &[(&str, [u8; 3])]) -> Result<SuppliedTexels, Box<dyn Error>> {
    let mut stated = Vec::with_capacity(declared.len());
    for (key, [red, green, blue]) in declared {
        stated.push((
            TextureKey::parse(key)?,
            vec![[*red, *green, *blue, 255]; (TEXTURE_EDGE * TEXTURE_EDGE) as usize],
        ));
    }
    Ok(SuppliedTexels::stating(stated))
}

/// The declared sample whose ray passes through the most cells of the sea.
struct Deepest {
    sample: (u32, u32),
    /// How many cells of the sea the ray passes through, counted one voxel at a
    /// time — the naive rule the run rule is contrasted with.
    cells_of_sea: usize,
    /// How many layers the judge answers for that same ray.
    layers: usize,
    classified: String,
    crossed: Crossed,
}

/// The crossing of the sea that passes through the most of it, from `camera`.
fn deepest_crossing(camera: &CameraPose, voxels: &Voxels<'_>) -> Result<Deepest, Box<dyn Error>> {
    let sea = BlockName::parse(SEA)?;
    let basis = Basis::of(camera);
    let lens = Lens::of(CAPTURE_SIZE);
    let mut deepest: Option<Deepest> = None;
    for (sample, crossed) in oracle::crossed_samples(camera, CAPTURE_SIZE, voxels)? {
        if !matches!(&crossed.sighted(), Sighted::Through { layers, .. } if layers.contains(&sea)) {
            continue;
        }
        let cells_of_sea = cells_of(&sea, basis.eye, basis.ray_through(sample, &lens), voxels)?;
        if deepest
            .as_ref()
            .is_none_or(|held| cells_of_sea > held.cells_of_sea)
        {
            deepest = Some(Deepest {
                sample,
                cells_of_sea,
                layers: crossed.layers.len(),
                classified: crossed.sighted().described(),
                crossed,
            });
        }
    }
    deepest.ok_or_else(|| {
        format!(
            "no declared sample of this camera crosses `{SEA}` at all, so there is no crossing for \
             these readings to be about — which is a world or a declaration that changed, not a \
             judge that answered wrongly"
        )
        .into()
    })
}

/// How many voxels holding `block` a ray passes through, counted one voxel at a
/// time.
///
/// **The naive rule, written out on purpose.** It walks the same grid the judge
/// walks and applies the other rule to it, so the contrast the reading asserts
/// is between two rules over one walk rather than between two walks.
fn cells_of(
    block: &BlockName,
    origin: Vec3,
    direction: Vec3,
    voxels: &Voxels<'_>,
) -> Result<usize, Box<dyn Error>> {
    let mut march = March::of(origin, direction);
    let mut counted = 0;
    while march.travelled <= 272.0 {
        match voxels.drawn_degree(march.voxel)? {
            Some((held, _degree)) if held == block => counted += 1,
            Some(_) => return Ok(counted),
            None => {}
        }
        march.step();
    }
    Ok(counted)
}
