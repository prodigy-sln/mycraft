//! A world in which every drawn block is opaque, and what the ray-marched
//! oracle predicted over it before an opacity could be declared at all.
//!
//! # What this is for
//!
//! SPEC-031 gives the oracle a second rule: a ray that crosses a translucent
//! block no longer stops there. The rule has to arrive without moving a single
//! prediction over a world that holds nothing translucent, and the only way to
//! state that is to have the earlier answers written down **before** the change
//! was made. That is what `predictions.txt` beside this module is.
//!
//! # It is a reading of a different program, and that is the one thing it must
//! not be mistaken for
//!
//! `testing.md` §2 forbids an expected quantity copied out of a run of the code
//! under test, because a number snapshotted from the first green run commits
//! whatever the code happened to do that day. This fixture is the deliberate
//! opposite of that and not an exception to it: the values were taken from a
//! tree on which `Opacity` did not exist, by an oracle that had no second rule
//! to get wrong, and they will be compared against a **later** oracle that does.
//! The recording's own header names the commit it was taken on, so a reader can
//! tell the two apart without taking anybody's word for it.
//!
//! Nothing in this module regenerates that file. Re-recording it against a tree
//! that already carries the change would destroy exactly the property it exists
//! to carry, so the only writer it ever had was a throwaway, and the header
//! records what produced the values rather than offering to produce them again.
//!
//! # Why this root and not the shipped one
//!
//! The shipped content root will not serve, and the reason is dated: phase 3 of
//! this spec declares `opacity = 0.5` on `content/base/blocks/water.luau`. A
//! recording taken over the shipped root would stop being a recording of an
//! all-opaque world on the very commit it was needed for. So the world is
//! generated from a root of this suite's own, under
//! `tests/fixtures/all_opaque/`, which declares the four blocks the replay's
//! strata name and states no opacity at any of them — which the loader that
//! cannot read the field ignores and the loader that can reads as `1.0`.
//!
//! The voxels are nonetheless the shipped world's own: `ReplayWorld::generate`
//! takes its shape from the seed and reads only names out of the registry it is
//! handed, so a root registering the same four names generates the same world.
//! That is asserted rather than assumed.
//!
//! # Why the camera is declared and not simulated
//!
//! `replay_oracle.rs` takes its poses from the simulation, which is right there
//! and wrong here. A pose the physics produces moves when the physics moves —
//! the shipped water's declared rates have already moved it twice — and this
//! recording is one that may never be re-taken. A moved pose would redden the
//! comparison for a reason with nothing to do with the oracle, and the cheapest
//! way to green it would be to re-record, which is the one thing that must not
//! happen. Declared poses make the recording a function of the world, the lens
//! and the march, and of nothing else.
//!
//! # How the three viewpoints were chosen
//!
//! The filter and the ranking are stated separately, because a constraint the
//! filter never applied is invisible to every ordering of it (`testing.md` §2).
//!
//! **The filter.** A candidate is admitted when the eye stands inside the
//! replay's footprint and in open air — no drawn voxel at the eye, which is what
//! keeps a saturated 576-of-one-class recording out — when its forward direction
//! is not parallel to the world's up axis, where the oracle's basis is
//! degenerate, and when its grid classifies at least one sample as sky and at
//! least one as terrain.
//!
//! **The ranking.** Three of the five classes the world can offer are sparse:
//! the subsurface is buried, the depths are buried deeper, and the sea covers
//! 131 of 4096 columns. One viewpoint is taken for each, the one that classifies
//! the most samples as it. Grass and sky need no viewpoint of their own — they
//! arrive in every candidate — so between them the three name every class the
//! world holds, which is what makes a march that stopped answering about one of
//! them visible here.
//!
//! **The space swept.** Eyes at `(x + 0.5, y, z + 0.5)` for `x` and `z` from 4
//! to 58 on a stride of 6 and `y` in {36, 40, 44, 48, 52}; targets at
//! `eye + 32 · (dx, dy, dz)` for `dx`, `dz` in −2..=2 and `dy` in −2..=1,
//! excluding the pairs with `dx = dz = 0`. 35 328 candidates examined, 26 795
//! admitted. **Integer direction components on purpose**: a pose stated as a
//! yaw and a pitch puts a target through `sin` and lands a hair off the axis it
//! was meant to be on — `f32::consts::PI`'s sine is −8.7e−8 — and these
//! coordinates have to be exactly what they read as.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use mc_core::block::BlockRegistry;
use mc_sim::camera::CameraPose;
use mc_sim::replay::ReplayWorld;
use mc_world::content::LuauFileDefinitionSource;

use super::frames::CAPTURE_SIZE;
use super::oracle::{self, Voxels};

/// The class every sample of this world classifies as, other than the sky.
///
/// **Written out rather than read off the fixture root**, for the reason
/// `replay_oracle.rs` states about its own list: a set discovered from the thing
/// under test agrees with it whatever it comes to hold.
pub const THE_BLOCKS: [&str; 4] = ["base:dirt", "base:grass", "base:stone", "base:water"];

/// The file names the fixture root declares those blocks under.
///
/// The same four the shipped root uses, and that is legibility rather than a
/// requirement — **measured, because the obvious reason for it is false.**
/// Declarations are registered in file-name order, so swapping which file
/// declares which name changes every block's id; doing exactly that left the
/// generated world identical and all six readings green. The world is compared
/// by what stands in each cell and not by the ids it was written through, so
/// what actually has to match the shipped root is the four *names*. Keeping the
/// file names too costs nothing and means a reader can put the two roots side by
/// side.
pub const THE_DECLARATION_FILES: [&str; 4] =
    ["dirt.luau", "grass.luau", "stone.luau", "water.luau"];

/// The field no declaration under this root may ever state.
pub const THE_FORBIDDEN_FIELD: &str = "opacity";

/// One camera the recording was taken from.
#[derive(Debug, Clone, PartialEq)]
pub struct Viewpoint {
    /// What the recording calls it.
    pub name: &'static str,
    pub eye: [f32; 3],
    pub target: [f32; 3],
}

impl Viewpoint {
    /// This viewpoint as a pose the oracle can be marched from.
    #[must_use]
    pub const fn pose(&self) -> CameraPose {
        CameraPose {
            eye: self.eye,
            target: self.target,
        }
    }
}

/// The three cameras the recording was taken from, each the best of the swept
/// space at one of the world's three sparse classes.
///
/// The measured composition of each, at the commit the recording names:
///
/// | viewpoint | dirt | grass | stone | water | sky |
/// |---|---|---|---|---|---|
/// | `the-exposed-subsurface` | 234 | 314 | — | — | 28 |
/// | `the-depths-of-the-landmark` | 11 | 337 | 162 | — | 66 |
/// | `the-sea-from-the-eastern-shore` | — | 192 | — | 288 | 96 |
///
/// Each row sums to the 576 samples of the declared grid, and the three between
/// them name all five classes. The numbers are stated here so a reader can see
/// what the recording is a recording *of*; nothing asserts against them, because
/// the file asserts against every sample individually and a count cannot see
/// shape.
pub const VIEWPOINTS: [Viewpoint; 3] = [
    Viewpoint {
        name: "the-exposed-subsurface",
        eye: [22.5, 40.0, 16.5],
        target: [54.5, 40.0, -15.5],
    },
    Viewpoint {
        name: "the-depths-of-the-landmark",
        eye: [10.5, 40.0, 10.5],
        target: [42.5, 40.0, 74.5],
    },
    Viewpoint {
        name: "the-sea-from-the-eastern-shore",
        eye: [58.5, 40.0, 16.5],
        target: [90.5, -24.0, 16.5],
    },
];

/// What one viewpoint's whole sample grid is looking at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Predicted {
    /// The viewpoint's name, as the recording spells it.
    pub viewpoint: String,
    /// The eye and the target the samples were marched from, carried through so
    /// a recording cannot be read against a camera it was not taken from.
    pub eye: [String; 3],
    pub target: [String; 3],
    /// Every declared sample pixel and the class a ray through it met, in
    /// [`oracle::sample_pixels`] order.
    pub samples: Vec<((u32, u32), String)>,
}

/// Where the fixture content root sits.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located.
pub fn content_root() -> Result<PathBuf, Box<dyn Error>> {
    Ok(fixtures()?.join("a-content-root-stating-no-opacity"))
}

/// Where the committed recording sits.
///
/// # Errors
///
/// Returns an error if the repository root cannot be located.
pub fn recording_path() -> Result<PathBuf, Box<dyn Error>> {
    Ok(fixtures()?.join("predictions.txt"))
}

/// The directory holding both.
fn fixtures() -> Result<PathBuf, Box<dyn Error>> {
    Ok(super::repository_root()?
        .join("crates")
        .join("mc-client")
        .join("tests")
        .join("fixtures")
        .join("all_opaque"))
}

/// A registry holding exactly what the fixture root declares.
///
/// # Errors
///
/// Returns an error if the root cannot be read or its declarations do not apply.
pub fn registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let mut registry = BlockRegistry::new();
    registry.apply(&LuauFileDefinitionSource::new(content_root()?))?;
    Ok(registry)
}

/// The replay world that registry generates.
///
/// # Errors
///
/// Returns the generation failure, which for this root would mean it stopped
/// declaring one of the names the replay's strata place.
pub fn world(registry: &BlockRegistry) -> Result<ReplayWorld, Box<dyn Error>> {
    Ok(ReplayWorld::generate(mc_sim::REPLAY_SEED, registry)?)
}

/// What this tree's oracle predicts at every declared sample of every declared
/// viewpoint.
///
/// **The harness that produced the committed recording.** It touches no device:
/// a march is a statement about the world and the camera and about nothing
/// drawn.
///
/// # Errors
///
/// Returns the root's refusal, the generation failure, or a block the fixture
/// root does not register.
pub fn predicted() -> Result<Vec<Predicted>, Box<dyn Error>> {
    let registry = registry()?;
    let world = world(&registry)?;
    let voxels = Voxels {
        world: &world,
        registry: &registry,
    };
    let mut predicted = Vec::new();
    for viewpoint in &VIEWPOINTS {
        let sighted = oracle::sighted_samples(&viewpoint.pose(), CAPTURE_SIZE, &voxels)?;
        predicted.push(Predicted {
            viewpoint: viewpoint.name.to_owned(),
            eye: written(viewpoint.eye),
            target: written(viewpoint.target),
            samples: sighted
                .into_iter()
                .map(|(pixel, sighted)| (pixel, sighted.described()))
                .collect(),
        });
    }
    Ok(predicted)
}

/// What the committed recording holds.
///
/// # Errors
///
/// Returns an error if the file cannot be read, or if a line of it is not one
/// of the four the format has.
pub fn recorded() -> Result<Vec<Predicted>, Box<dyn Error>> {
    parse(&recorded_text()?)
}

/// The committed recording as it stands on disk, header and all.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn recorded_text() -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(recording_path()?)?)
}

/// `predicted` written out in the recording's own format, without the prose
/// header the file carries above it.
///
/// The header is the file's and never this module's: it names the commit the
/// values were taken on, and a header a function could reproduce is one that
/// would survive a re-recording it is supposed to expose.
#[must_use]
pub fn rendered(predicted: &[Predicted]) -> String {
    let mut written = String::new();
    for viewpoint in predicted {
        written.push_str(&format!("viewpoint {}\n", viewpoint.viewpoint));
        written.push_str(&format!("eye {}\n", viewpoint.eye.join(" ")));
        written.push_str(&format!("target {}\n", viewpoint.target.join(" ")));
        for ((across, down), class) in &viewpoint.samples {
            written.push_str(&format!("{across} {down} {class}\n"));
        }
    }
    written
}

/// `text` with everything the format ignores taken out: blank lines, and the
/// prose header's own comment lines.
#[must_use]
pub fn body_of(text: &str) -> String {
    let mut body = String::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    body
}

/// Whether each of the fixture root's declarations states the forbidden field,
/// one answer per file in [`THE_DECLARATION_FILES`] order.
///
/// # Errors
///
/// Returns an error if a declaration cannot be read — a file that has gone
/// missing is reported rather than passing as a file that states nothing.
pub fn declarations_stating_an_opacity() -> Result<Vec<(String, bool)>, Box<dyn Error>> {
    let blocks = content_root()?.join("blocks");
    let mut stated = Vec::new();
    for file_name in THE_DECLARATION_FILES {
        let declaration = fs::read_to_string(blocks.join(file_name))?;
        stated.push((file_name.to_owned(), states_an_opacity(&declaration)));
    }
    Ok(stated)
}

/// Whether one declaration's text states the forbidden field.
///
/// Separate from the sweep above so the sweep's own answer can be driven over a
/// declaration that does state one: a scan asserting only an absence goes green
/// forever the day it stops being able to look.
#[must_use]
pub fn states_an_opacity(declaration: &str) -> bool {
    declaration
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .any(|line| line.contains(THE_FORBIDDEN_FIELD))
}

/// Three coordinates as the recording spells them.
///
/// Held as text rather than as `f32` from the moment they are produced, so the
/// comparison a recording feeds is over one spelling on both sides and a pose
/// that round-tripped to a different bit pattern is a difference this reports
/// rather than one it hides.
pub fn written(coordinates: [f32; 3]) -> [String; 3] {
    coordinates.map(|coordinate| coordinate.to_string())
}

/// The recording `text` holds.
fn parse(text: &str) -> Result<Vec<Predicted>, Box<dyn Error>> {
    let mut parsed: Vec<Predicted> = Vec::new();
    for line in body_of(text).lines() {
        let words: Vec<&str> = line.split(' ').collect();
        match words.as_slice() {
            ["viewpoint", name] => parsed.push(Predicted {
                viewpoint: (*name).to_owned(),
                eye: [String::new(), String::new(), String::new()],
                target: [String::new(), String::new(), String::new()],
                samples: Vec::new(),
            }),
            ["eye", x, y, z] => {
                latest(&mut parsed, line)?.eye =
                    [(*x).to_owned(), (*y).to_owned(), (*z).to_owned()];
            }
            ["target", x, y, z] => {
                latest(&mut parsed, line)?.target =
                    [(*x).to_owned(), (*y).to_owned(), (*z).to_owned()];
            }
            [across, down, class] => {
                let pixel = (across.parse()?, down.parse()?);
                latest(&mut parsed, line)?
                    .samples
                    .push((pixel, (*class).to_owned()));
            }
            _ => return Err(format!("`{line}` is not a line the recording's format has").into()),
        }
    }
    Ok(parsed)
}

/// The viewpoint being read, or the refusal that `line` arrived before any
/// viewpoint was opened.
fn latest<'a>(
    parsed: &'a mut [Predicted],
    line: &str,
) -> Result<&'a mut Predicted, Box<dyn Error>> {
    parsed
        .last_mut()
        .ok_or_else(|| format!("`{line}` stands before any viewpoint the recording names").into())
}
