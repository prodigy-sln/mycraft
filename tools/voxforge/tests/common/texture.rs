//! Fixtures and verdicts shared by the block-texture tests.
//!
//! **Every fixture here is small on purpose, and the reason is the metric.**
//! The edge leg compares a texture's wrap against the largest step the texture
//! already contains *in the same row*, so a fixture whose interior steps
//! saturate at 255 grades that leg at nothing — it can never fail. The four-step
//! gradient below is `#000000`, `#555555`, `#aaaaaa`, `#ffffff` because
//! `255 / 3 = 85` exactly: the wrap is 255, the largest interior step is 85, and
//! both are integers a reader derives by division rather than reads off a
//! render. A `scale` of 16 would have meant sixteen materials and a step of 17,
//! and the fixture would stop being readable for nothing gained.
//!
//! Every verdict here is an **enumerated** answer. "Refused, naming the period"
//! and "emitted six faces" must never compare equal, and neither must "the words
//! named no leg" and "the words named two". A boolean or an `is_empty()` cannot
//! tell those apart, which is the whole reason a total `SeamVerdict` exists one
//! layer down.

use std::error::Error;
use std::num::NonZeroU32;

use voxforge::fault::{Fault, Origin};
use voxforge::material::{Emissive, Material, MaterialTable, Srgb8};
use voxforge::name::MaterialKey;
use voxforge::render::View;
use voxforge::texture::{
    AxisAlignedView, EmittedFace, FaceSelection, SeamPolicy, SeamVerdict, TextureRequest,
    TextureSet, emit,
};
use voxforge::volume::{StateSelection, assemble};

use super::{FIXTURE_FILE, Mention, loaded};

/// One pixel per voxel: the smallest render there is, and the one an image of a
/// single voxel needs.
pub const ONE_PER_VOXEL: NonZeroU32 = match NonZeroU32::new(1) {
    Some(scale) => scale,
    None => NonZeroU32::MIN,
};

/// One tone a texture fixture is painted in.
///
/// The spelling, the material key and the colour travel together so that a
/// document's grid, its palette and the material table cannot drift apart —
/// three declarations of one fact is three chances for a fixture to paint
/// something other than what its test says it paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tone {
    /// The character a grid spells it with.
    pub spelling: char,
    /// The namespaced material it stands for.
    pub key: &'static str,
    /// The colour that material declares.
    pub colour: Srgb8,
}

/// That tone, whose colour is written the way a document writes one.
#[must_use]
pub const fn tone(spelling: char, key: &'static str, hex: u32) -> Tone {
    Tone {
        spelling,
        key,
        colour: Srgb8 {
            red: (hex >> 16) as u8,
            green: (hex >> 8) as u8,
            blue: hex as u8,
        },
    }
}

/// `#000000` — the bottom of the four-step gradient.
pub const BLACK: Tone = tone('a', "base:tone_black", 0x000000);
/// `#555555` — one step of 85 above [`BLACK`].
pub const LOW: Tone = tone('b', "base:tone_low", 0x555555);
/// `#aaaaaa` — two steps of 85 above [`BLACK`].
pub const HIGH: Tone = tone('c', "base:tone_high", 0xaaaaaa);
/// `#ffffff` — the top of the four-step gradient.
pub const WHITE: Tone = tone('d', "base:tone_white", 0xffffff);
/// `#808080` — the mid grey whose shaded and flat bytes differ.
pub const GREY: Tone = tone('e', "base:tone_grey", 0x808080);
/// `#404040` — a quarter tone, 64 below [`GREY`].
pub const DIM: Tone = tone('f', "base:tone_dim", 0x404040);
/// `#00ff00` — the one tone here that is not grey, so that a metric summing or
/// averaging channels answers differently from one taking the largest.
pub const LIME: Tone = tone('g', "base:tone_lime", 0x00ff00);

/// `#ff0000`, the `right` face's marker.
pub const RED: Tone = tone('h', "base:mark_red", 0xff0000);
/// `#0000ff`, the `left` face's marker.
pub const BLUE: Tone = tone('i', "base:mark_blue", 0x0000ff);
/// `#ffff00`, the `top` face's marker.
pub const YELLOW: Tone = tone('j', "base:mark_yellow", 0xffff00);
/// `#00ffff`, the `bottom` face's marker.
pub const CYAN: Tone = tone('k', "base:mark_cyan", 0x00ffff);
/// `#ff00ff`, the `front` face's marker.
pub const MAGENTA: Tone = tone('l', "base:mark_magenta", 0xff00ff);
/// `#ff8000`, the `back` face's marker.
pub const ORANGE: Tone = tone('m', "base:mark_orange", 0xff8000);

/// The four-step greyscale gradient, ascending in equal steps of 85.
pub const GRADIENT: [Tone; 4] = [BLACK, LOW, HIGH, WHITE];

/// The two tones the checker alternates between.
pub const CHECKER_PALETTE: [Tone; 2] = [BLACK, WHITE];

/// The four tones of the one fixture here that is not greyscale.
pub const LIME_PALETTE: [Tone; 4] = [BLACK, DIM, GREY, LIME];

/// The three tones the staircase is built from.
pub const STAIRCASE_PALETTE: [Tone; 3] = [BLACK, GREY, WHITE];

/// The marker cube's six voxels: one distinct colour per face, in
/// `AxisAlignedView::ALL` order — front, back, left, right, top, bottom.
///
/// Each has **exactly one coordinate at an extreme** of the `[4, 4, 4]` cube and
/// the other two interior, so no voxel lies on two faces, each face's own marker
/// is the only one it can see, and every boundary row and column of every image
/// stays grey — which is what keeps the seam legs unaffected by the markers.
pub const MARKERS: [(Tone, (u32, u32, u32)); 6] = [
    (MAGENTA, (1, 2, 3)),
    (ORANGE, (2, 1, 0)),
    (BLUE, (0, 2, 1)),
    (RED, (3, 1, 2)),
    (YELLOW, (1, 3, 2)),
    (CYAN, (2, 0, 1)),
];

/// A one-part model of `size` declaring `scale`, sliced on `y`, holding whatever
/// `tone_of` answers for each voxel.
///
/// A `y` layer prints one row per `z` ascending, its columns running `x`
/// ascending — the spec's own table, which is Phase 2's graded contract and is
/// relied on here rather than restated.
#[must_use]
pub fn model(
    size: (u32, u32, u32),
    scale: u32,
    palette: &[Tone],
    tone_of: &dyn Fn(u32, u32, u32) -> Option<Tone>,
) -> String {
    let (extent_x, extent_y, extent_z) = size;
    let entries: String = palette
        .iter()
        .map(|tone| {
            format!(
                "\"{spelling}\" = \"{key}\"\n",
                spelling = tone.spelling,
                key = tone.key
            )
        })
        .collect();
    format!(
        "schema = 1\nname = \"base:fixture\"\nscale = {scale}\n\
         size = [{extent_x}, {extent_y}, {extent_z}]\norigin = [0, 0, 0]\nslice = \"y\"\n\n\
         [palette]\n\".\" = \"empty\"\n{entries}{layers}",
        layers = layers(size, tone_of)
    )
}

/// Every `y` layer of a model of `size`, as the document spells them.
fn layers(size: (u32, u32, u32), tone_of: &dyn Fn(u32, u32, u32) -> Option<Tone>) -> String {
    let (extent_x, extent_y, extent_z) = size;
    (0..extent_y)
        .map(|y| {
            let art: Vec<String> = (0..extent_z)
                .map(|z| {
                    (0..extent_x)
                        .map(|x| tone_of(x, y, z).map_or('.', |tone| tone.spelling))
                        .collect()
                })
                .collect();
            format!(
                "\n[[layers]]\ny = {y}\ngrid = \"\"\"\n{grid}\n\"\"\"\n",
                grid = art.join("\n")
            )
        })
        .collect()
}

/// A material table declaring `palette`, none of it emissive.
///
/// # Errors
///
/// Returns an error when a key is not namespaced, which would leave the table
/// short of a material the fixtures spell for a reason nothing to do with the
/// texture.
pub fn table(palette: &[Tone]) -> Result<MaterialTable, Box<dyn Error>> {
    let mut declared = std::collections::BTreeMap::new();
    for tone in palette {
        declared.insert(
            MaterialKey::parse(tone.key)?,
            Material {
                color: tone.colour,
                emissive: Emissive::NONE,
            },
        );
    }
    Ok(MaterialTable::new("materials", declared))
}

/// One material file's text, as the tool reads it off disk.
#[must_use]
pub fn material_text(tone: Tone) -> String {
    format!(
        "name = \"{key}\"\ncolor = \"#{red:02x}{green:02x}{blue:02x}\"\nemissive = 0.0\n",
        key = tone.key,
        red = tone.colour.red,
        green = tone.colour.green,
        blue = tone.colour.blue
    )
}

/// What one emission asks for.
#[derive(Debug, Clone, Copy)]
pub struct Emission {
    /// Which faces.
    faces: FaceSelection,
    /// Whether the seam verdict binds.
    seams: SeamPolicy,
    /// How many pixels one voxel spans.
    pixels_per_voxel: NonZeroU32,
}

impl Emission {
    /// One face, declared seamless, so its verdict binds.
    ///
    /// # Errors
    ///
    /// Returns the refusal when `face` is not an axis-aligned one.
    pub fn seamless(face: View) -> Result<Self, Fault> {
        Ok(Self::one(
            AxisAlignedView::parse(face)?,
            SeamPolicy::Required,
        ))
    }

    /// One face, undeclared, so its verdict is reported and binds on nothing.
    ///
    /// # Errors
    ///
    /// Returns the refusal when `face` is not an axis-aligned one.
    pub fn reported(face: View) -> Result<Self, Fault> {
        Ok(Self::one(
            AxisAlignedView::parse(face)?,
            SeamPolicy::Reported,
        ))
    }

    /// A block's whole six faces, declared seamless.
    #[must_use]
    pub fn seamless_set() -> Self {
        Self::every(SeamPolicy::Required)
    }

    /// A block's whole six faces, undeclared.
    #[must_use]
    pub fn reported_set() -> Self {
        Self::every(SeamPolicy::Reported)
    }

    /// The same emission, at that many pixels per voxel.
    #[must_use]
    pub fn at(mut self, pixels_per_voxel: NonZeroU32) -> Self {
        self.pixels_per_voxel = pixels_per_voxel;
        self
    }

    /// That one face, under `seams`.
    fn one(face: AxisAlignedView, seams: SeamPolicy) -> Self {
        Self {
            faces: FaceSelection::One(face),
            seams,
            pixels_per_voxel: super::preview::EIGHT_PER_VOXEL,
        }
    }

    /// Every face, under `seams`.
    fn every(seams: SeamPolicy) -> Self {
        Self {
            faces: FaceSelection::All,
            seams,
            pixels_per_voxel: super::preview::EIGHT_PER_VOXEL,
        }
    }
}

/// What an emission produced.
///
/// Two answers that must never compare equal: a refusal emits nothing, and an
/// emission refuses nothing.
#[derive(Debug)]
pub enum Outcome {
    /// These faces, in the order they were emitted.
    Emitted(TextureSet),
    /// This refusal, and no face at all.
    Refused(Fault),
}

impl Outcome {
    /// Every face emitted, and none at all where the emission was refused.
    #[must_use]
    pub fn faces(&self) -> &[EmittedFace] {
        match self {
            Self::Emitted(set) => &set.faces,
            Self::Refused(_) => &[],
        }
    }

    /// The one face emitted, or `None` where that is not what happened.
    #[must_use]
    pub fn only(&self) -> Option<&EmittedFace> {
        match self.faces() {
            [only] => Some(only),
            _ => None,
        }
    }

    /// The refusal's words, and nothing where nothing was refused.
    #[must_use]
    pub fn cause(&self) -> &str {
        match self {
            Self::Emitted(_) => "",
            Self::Refused(fault) => &fault.cause,
        }
    }
}

/// What emitting `text`'s model under `asked` produced, painted from `palette`.
///
/// # Errors
///
/// Returns the refusal when the document does not load or the model does not
/// assemble — both of which are failures of the fixture rather than answers
/// about a texture, and must not reach an assertion as one.
pub fn emitted(text: &str, palette: &[Tone], asked: Emission) -> Result<Outcome, Box<dyn Error>> {
    let model = loaded(text)?;
    let volume = assemble(&model, &StateSelection::default())?;
    let materials = table(palette)?;
    let request = TextureRequest {
        faces: asked.faces,
        pixels_per_voxel: asked.pixels_per_voxel,
        scale: model.scale,
        seams: asked.seams,
        origin: Origin::new(FIXTURE_FILE),
    };
    Ok(match emit(&volume, &materials, request) {
        Ok(set) => Outcome::Emitted(set),
        Err(fault) => Outcome::Refused(fault),
    })
}

/// What one emitted face measures, and what was said about it.
///
/// An enumerated answer rather than three separate reads: "no face was emitted"
/// and "a face was emitted whose image is empty" must not compare equal, and
/// neither must either of them and "the emission was refused".
#[derive(Debug, PartialEq, Eq)]
pub enum OneFace {
    /// One face came out, of that size, carrying those verdicts.
    Emitted {
        /// How many pixels across its image is.
        width: u32,
        /// How many pixels down it is.
        height: u32,
        /// What each leg answered, in the declared order.
        verdicts: Vec<SeamVerdict>,
    },
    /// Something other than exactly one face came out.
    NotOne(usize),
    /// The emission was refused, saying this.
    Refused(String),
}

/// What `outcome` emitted, as one answer.
#[must_use]
pub fn one_face(outcome: &Outcome) -> OneFace {
    if let Outcome::Refused(fault) = outcome {
        return OneFace::Refused(fault.cause.clone());
    }
    match outcome.only() {
        Some(only) => OneFace::Emitted {
            width: only.image.width(),
            height: only.image.height(),
            verdicts: only.verdicts.clone(),
        },
        None => OneFace::NotOne(outcome.faces().len()),
    }
}

/// That face, emitted at that size and said to tile across every edge.
#[must_use]
pub fn tiling(width: u32, height: u32) -> OneFace {
    OneFace::Emitted {
        width,
        height,
        verdicts: vec![SeamVerdict::TilesAcrossEveryEdge],
    }
}

/// Whether `text` names `expected`, and in that order.
///
/// An enumerated verdict rather than a comparison of two `Option`s: `None`
/// sorts below `Some`, so a missing first token would compare as "in order" and
/// pass.
#[must_use]
pub fn named_in_order(text: &str, expected: &[&str]) -> Mention {
    let mut reached = 0;
    for token in expected {
        let Some(found) = text.find(token) else {
            return Mention::Missing((*token).to_owned());
        };
        if found < reached {
            return Mention::OutOfOrder((*token).to_owned());
        }
        reached = found;
    }
    Mention::Ordered
}

/// One leg of the seam question, as a refusal's words name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leg {
    /// The passing answer.
    Tiles,
    /// The model is not one block across an in-plane axis.
    Period,
    /// Some pixel shows the void.
    Opacity,
    /// The wrap steps further than the content does.
    Edges,
}

impl Leg {
    /// Every leg, in the order they are evaluated.
    pub const ALL: [Self; 4] = [Self::Tiles, Self::Period, Self::Opacity, Self::Edges];

    /// The token a verdict about this leg carries.
    ///
    /// Contract rather than prose: an agent parses these lines, and a mapping is
    /// decidable only where its spelling is pinned. The four are mutually
    /// exclusive, so no message can name two legs by accident.
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Tiles => "tiles across every edge",
            Self::Period => "period",
            Self::Opacity => "transparent",
            Self::Edges => "edges disagree",
        }
    }
}

/// Which legs some words name.
#[derive(Debug, PartialEq, Eq)]
pub enum Legs {
    /// Exactly that one, and no other.
    Only(Leg),
    /// More than one, in the order the words name them.
    Several(Vec<Leg>),
    /// None at all.
    Silent,
}

/// Which legs `text` names.
#[must_use]
pub fn legs_named(text: &str) -> Legs {
    let mut found: Vec<(usize, Leg)> = Leg::ALL
        .into_iter()
        .filter_map(|leg| text.find(leg.token()).map(|at| (at, leg)))
        .collect();
    found.sort_by_key(|(at, _)| *at);
    let named: Vec<Leg> = found.into_iter().map(|(_, leg)| leg).collect();
    match named.as_slice() {
        [] => Legs::Silent,
        [only] => Legs::Only(*only),
        _ => Legs::Several(named),
    }
}

/// What an emission that was supposed to refuse actually did.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Refused, naming those legs, and missing none of the values it had to
    /// name.
    Named {
        /// Which legs the words name.
        legs: Legs,
        /// Which expected values they do not name.
        missing: Vec<String>,
    },
    /// Not refused at all: this many faces came out.
    Emitted(usize),
}

/// How `outcome` refused, and which of `expected` its words leave unnamed.
#[must_use]
pub fn refusal(outcome: &Outcome, expected: &[&str]) -> Refusal {
    match outcome {
        Outcome::Emitted(set) => Refusal::Emitted(set.faces.len()),
        Outcome::Refused(fault) => Refusal::Named {
            legs: legs_named(&fault.cause),
            missing: unnamed(&fault.cause, expected),
        },
    }
}

/// What a refusal said, without asking which leg it was about.
///
/// Separate from [`Refusal`] because a request precondition — a face set of a
/// model that is not a cube — is deliberately *not* a seam verdict, and pinning
/// which legs its words may name would grade a classification the scenario does
/// not make.
#[derive(Debug, PartialEq, Eq)]
pub enum Words {
    /// Refused, naming everything it had to.
    NamedEverything,
    /// Refused, and these are absent from its words.
    Missing(Vec<String>),
    /// Not refused at all: this many faces came out.
    Emitted(usize),
}

/// Whether `outcome` refused naming every one of `expected`.
#[must_use]
pub fn words(outcome: &Outcome, expected: &[&str]) -> Words {
    match outcome {
        Outcome::Emitted(set) => Words::Emitted(set.faces.len()),
        Outcome::Refused(fault) => match unnamed(&fault.cause, expected) {
            missing if missing.is_empty() => Words::NamedEverything,
            missing => Words::Missing(missing),
        },
    }
}

/// Every one of `expected` that `text` does not contain.
#[must_use]
pub fn unnamed(text: &str, expected: &[&str]) -> Vec<String> {
    expected
        .iter()
        .filter(|token| !text.contains(*token))
        .map(|token| (*token).to_owned())
        .collect()
}

/// Nothing — what [`unnamed`] answers for words that named everything.
#[must_use]
pub fn nothing_unnamed() -> Vec<String> {
    Vec::new()
}
