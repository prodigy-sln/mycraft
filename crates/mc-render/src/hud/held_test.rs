//! Which layer the held-block indicator draws from, and what a layout composes
//! when the block the session holds has nothing to draw.
//!
//! # The indicator resolves the way the world does, and this is the second site
//!
//! A drawn face and this indicator are two consumers of one question: which key
//! does *this block* draw on *this facing*. They were two separate parses of a
//! block's own name, and closing only one of them is worse than closing neither
//! — a block would draw correctly in the world and show a blank indicator, which
//! reads as a HUD bug and sends whoever chases it to the wrong module. So the
//! readings below are deliberately the same shape as the packer's, against a
//! block whose name is not the key it declares.
//!
//! # A side face, and it is written down rather than implied
//!
//! The indicator looks at one facing and it has to be a stated one. A side face
//! is what makes the canonical block recognisable — a grass block's side carries
//! both the growth and the earth, where its top is a green square that says
//! "grass" only to somebody who already knows. The four sides are
//! interchangeable for that purpose, so the choice is arbitrary, made once and
//! recorded. What these readings hold is that it is *a stated one*: the six keys
//! below are pairwise distinct and hold six different layers, so an indicator
//! looking at any other facing draws a different layer and fails here.
//!
//! # "No indicator" is a ruling about the outline too, and it is made here
//!
//! An outline is composed for an element of any draw kind, so a declaration that
//! states one and resolves to nothing would leave a black ring around nothing at
//! the anchor the indicator names. That is an indicator — a reader sees a
//! bordered empty square where a swatch belongs, and "the system drew no
//! indicator" would be false of the picture while true of the fill. So **an
//! element whose draw resolves to nothing composes no ring either**, and the
//! assertion for it is stated over the whole plan rather than over the textured
//! rectangles in it, because a count of textured rectangles cannot see a ring.
//!
//! The rule is keyed on the paint being unavailable and never on clipping: an
//! element pushed off the target still has a fill to draw and still draws its
//! outline as far as the target reaches, which is a different question and one
//! the clipping scenarios already own.
//!
//! # Every number here is derived from the declarations, not from a run
//!
//! Two of the three fixtures in the unresolved reading are fills stating no
//! outline, so each contributes exactly one rectangle; the third resolves to
//! nothing and contributes none. Two is therefore the whole plan. A composition
//! that resolved the unstocked block to some other block's layer reads three,
//! and one that kept the ring reads six — the four strips of `26 × 26 − 24 × 24`
//! plus the two fills.
//!
//! Every layer below is stated by the fixture's own table and **none of them is
//! the layer a lexicographic assignment would give its key**, so a reading taken
//! against a sort could not fail whatever the lookup did.
//!
//! # The fixtures name nothing the base game ships
//!
//! This file sits under `src/`, where the scan forbidding shipped HUD element
//! names and colour literals in Rust looks. Every name below is in a `fixture:`
//! namespace and every colour is one no shipped declaration states.

use std::error::Error;
use std::sync::Arc;

use mc_core::block::Opacity;
use mc_core::content::{Face, FaceTextures};
use mc_core::hud::source::InMemoryHudSource;
use mc_core::hud::{DeclaredValue, HudLayout, HudOrigin, RawHudElement};
use mc_core::id::{BlockName, TextureKey};

use crate::hud::{HudFrame, Painted, PaintedRect, compose};
use crate::surface::SurfaceSize;
use crate::texture::{TextureLayers, TextureResolution};

use super::{HeldSwatch, held_swatch};

type TestResult = Result<(), Box<dyn Error>>;

/// The reference target, where one UI unit is one physical pixel.
const TARGET: SurfaceSize = SurfaceSize {
    width: 1280,
    height: 720,
};

/// A block declaring a different key on each of its six facings.
///
/// The subject of the first reading: six distinct keys on six distinct layers is
/// what makes "the indicator drew the *north* key" falsifiable at all.
const BANDED: &str = "fixture:banded";

/// The six keys it declares, positionally in the order a declaration writes its
/// facings: up, down, north, south, east and west.
///
/// Minerals rather than compass words, so the facing a key belongs to cannot be
/// recovered from its spelling.
const BANDED_KEYS: [&str; 6] = [
    "fixture:crown",
    "fixture:sole",
    "fixture:cobalt",
    "fixture:diorite",
    "fixture:emerald",
    "fixture:feldspar",
];

/// The key [`BANDED`] declares against `north`.
const BANDED_NORTH: &str = "fixture:cobalt";

/// A block whose name is not the key it declares.
///
/// The second site of the substitution the packer's own readings cover: a lookup
/// parsing the held block's name resolves this one to nothing and draws no
/// indicator at all, while the assignment names a layer for the key it declared.
const RENAMED: &str = "fixture:amber";
const ITS_DECLARED_KEY: &str = "fixture:gold";

/// A block whose `north` key occupies no layer while its `up` key does.
///
/// Both halves are load-bearing. The uncovered `north` is what the reading is
/// about; the covered `up` is what stops a lookup consulting the wrong facing
/// from reporting the same unresolved answer for the wrong reason.
const UNSTOCKED: &str = "fixture:unstocked";
const ITS_UNCOVERED_KEY: &str = "fixture:unlit";
const ITS_COVERED_KEY: &str = "fixture:lit";

/// The six keys [`UNSTOCKED`] declares, in the same positional order.
const UNSTOCKED_KEYS: [&str; 6] = [
    ITS_COVERED_KEY,
    ITS_COVERED_KEY,
    ITS_UNCOVERED_KEY,
    ITS_COVERED_KEY,
    ITS_COVERED_KEY,
    ITS_COVERED_KEY,
];

/// Which layer each covered key holds.
///
/// Sorted, these eight keys run amber… — and not one entry below is on the layer
/// that order would give it. [`ITS_UNCOVERED_KEY`] is deliberately absent.
const ASSIGNED: [(&str, u16); 8] = [
    ("fixture:crown", 4),
    ("fixture:sole", 2),
    ("fixture:cobalt", 5),
    ("fixture:diorite", 0),
    ("fixture:emerald", 3),
    ("fixture:feldspar", 1),
    (ITS_DECLARED_KEY, 6),
    (ITS_COVERED_KEY, 7),
];

/// How many rectangles the two outline-less fills contribute, and therefore how
/// many the whole plan holds once the indicator contributes none.
const FILLS_ALONE: usize = 2;

/// One declaration as this suite writes it.
#[derive(Debug, Clone, Copy)]
struct Declared {
    name: &'static str,
    anchor: &'static str,
    size: [i64; 2],
    /// The fill colour, or nothing for the textured swatch.
    color: Option<&'static str>,
    outline: Option<&'static str>,
}

/// A thin bar, stating no outline, so its whole contribution to a plan is one
/// rectangle.
const LEFT_BAR: Declared = Declared {
    name: "fixture:left-bar",
    anchor: "center",
    size: [9, 1],
    color: Some("#3366CCFF"),
    outline: None,
};

/// The same, crossing it.
const RIGHT_BAR: Declared = Declared {
    name: "fixture:right-bar",
    anchor: "center",
    size: [1, 9],
    color: Some("#3366CCFF"),
    outline: None,
};

/// The indicator: a textured square that states an outline, which is what puts
/// the ring in question at all.
const SWATCH: Declared = Declared {
    name: "fixture:swatch",
    anchor: "bottom",
    size: [24, 24],
    color: None,
    outline: Some("#0A0B0CFF"),
};

impl Declared {
    /// This declaration in the form a source hands over.
    fn raw(self) -> RawHudElement {
        let [across, down] = self.size;
        let mut fields = vec![
            ("name".to_owned(), text(self.name)),
            ("anchor".to_owned(), text(self.anchor)),
            (
                "size".to_owned(),
                DeclaredValue::List(vec![
                    DeclaredValue::Integer(across),
                    DeclaredValue::Integer(down),
                ]),
            ),
        ];
        match self.color {
            Some(color) => {
                fields.push(("draw".to_owned(), text("fill")));
                fields.push(("color".to_owned(), text(color)));
            }
            None => {
                fields.push(("draw".to_owned(), text("block-texture")));
                fields.push(("source".to_owned(), text("held-block")));
            }
        }
        if let Some(outline) = self.outline {
            fields.push(("outline".to_owned(), text(outline)));
        }
        RawHudElement::new(fields)
    }
}

fn text(spelled: &str) -> DeclaredValue {
    DeclaredValue::Text(spelled.to_owned())
}

/// A layout holding exactly `declarations`, in the order given.
///
/// # Errors
///
/// Fails when the layout refused a declaration or registered a different number
/// of them: a plan derived from a layout that registered nothing satisfies a
/// claim about what is *not* in it for free.
fn layout_of(declarations: &[Declared]) -> Result<HudLayout, Box<dyn Error>> {
    let stated = declarations
        .iter()
        .map(|declared| (HudOrigin::new(declared.name), declared.raw()))
        .collect();
    let layout = HudLayout::load(&InMemoryHudSource::new(
        HudOrigin::new("this suite"),
        stated,
    ))?;
    if layout.elements().len() != declarations.len() {
        return Err(format!(
            "this fixture has to register all {} of its declarations, or what is composed from it \
             is not what it states, but it registered {}",
            declarations.len(),
            layout.elements().len()
        )
        .into());
    }
    Ok(layout)
}

/// The resolution the array texture was filled from: three blocks and the layers
/// [`ASSIGNED`] states.
///
/// # Errors
///
/// Fails when a fixture id is not a namespaced id, or when the six keys of a
/// declaration are not six.
fn resolution() -> Result<TextureResolution, Box<dyn Error>> {
    let mut layers = Vec::with_capacity(ASSIGNED.len());
    for (key, layer) in ASSIGNED {
        layers.push((TextureKey::parse(key)?, layer));
    }
    Ok(TextureResolution::stating(
        [
            (
                BlockName::parse(BANDED)?,
                six_of(BANDED_KEYS)?,
                Opacity::OPAQUE,
            ),
            (
                BlockName::parse(RENAMED)?,
                FaceTextures::uniform(TextureKey::parse(ITS_DECLARED_KEY)?),
                Opacity::OPAQUE,
            ),
            (
                BlockName::parse(UNSTOCKED)?,
                six_of(UNSTOCKED_KEYS)?,
                Opacity::OPAQUE,
            ),
        ],
        TextureLayers::stated(layers),
    ))
}

/// `keys`, positionally, as a declaration states them.
///
/// # Errors
///
/// Fails when a key is not a namespaced id.
fn six_of(keys: [&str; 6]) -> Result<FaceTextures, Box<dyn Error>> {
    let mut parsed = Vec::with_capacity(keys.len());
    for key in keys {
        parsed.push(TextureKey::parse(key)?);
    }
    let stated: [TextureKey; 6] = parsed
        .try_into()
        .map_err(|_unexpected| "a declaration states exactly six facings")?;
    Ok(FaceTextures::stating(stated))
}

/// The layer [`ASSIGNED`] states for `key`.
///
/// # Errors
///
/// Fails when the table names no layer for it.
fn layer_of(key: &str) -> Result<u16, Box<dyn Error>> {
    ASSIGNED
        .into_iter()
        .find(|(named, _)| *named == key)
        .map(|(_, layer)| layer)
        .ok_or_else(|| format!("this fixture states no layer for `{key}`").into())
}

/// The layers of `planned` that sample the array texture, in plan order.
fn sampled(planned: &[PaintedRect]) -> Vec<u16> {
    planned
        .iter()
        .filter_map(|rect| match rect.paint {
            Painted::Texture(layer) => Some(layer),
            Painted::Fill(_) => None,
        })
        .collect()
}

/// How many of `planned` sample the array texture.
fn textured(planned: &[PaintedRect]) -> usize {
    sampled(planned).len()
}

/// What a frame holding `held` composes, over `declarations`.
///
/// # Errors
///
/// Fails when the layout refuses a declaration.
fn composed(
    held: &HeldSwatch,
    declarations: &[Declared],
    resolution: &TextureResolution,
) -> Result<Vec<PaintedRect>, Box<dyn Error>> {
    Ok(compose(
        &HudFrame {
            layout: Arc::new(layout_of(declarations)?),
            held: held.texture(),
        },
        TARGET,
        resolution.layers(),
    ))
}

#[test]
fn the_indicator_draws_the_layer_assigned_to_the_held_blocks_north_key() -> TestResult {
    let resolution = resolution()?;
    let held = BlockName::parse(BANDED)?;

    let swatch = held_swatch(Some(&held), &resolution);
    let planned = composed(&swatch, &[SWATCH], &resolution)?;

    assert_eq!(
        sampled(&planned),
        vec![layer_of(BANDED_NORTH)?],
        "the indicator draws the key the held block declares against `north`. This block declares \
         six pairwise distinct keys on six distinct layers, so an indicator consulting any other \
         facing — or the block's own name — samples a different layer and lands here"
    );
    Ok(())
}

#[test]
fn the_indicator_draws_the_declared_key_rather_than_nothing_when_the_name_is_not_the_key()
-> TestResult {
    let resolution = resolution()?;
    let held = BlockName::parse(RENAMED)?;

    let swatch = held_swatch(Some(&held), &resolution);
    let planned = composed(&swatch, &[SWATCH], &resolution)?;

    assert_eq!(
        sampled(&planned),
        vec![layer_of(ITS_DECLARED_KEY)?],
        "this block is called {RENAMED} and declares {ITS_DECLARED_KEY}. A lookup parsing the \
         held block's own name finds no layer for it and draws no indicator at all — a blank \
         square beside a block that draws perfectly well in the world, which reads as a HUD fault \
         and sends whoever chases it to the wrong module"
    );
    Ok(())
}

#[test]
fn a_north_key_with_no_layer_reports_the_indicator_unresolved_naming_the_block_and_the_key()
-> TestResult {
    let resolution = resolution()?;
    let held = BlockName::parse(UNSTOCKED)?;

    let swatch = held_swatch(Some(&held), &resolution);
    let planned = composed(&swatch, &[LEFT_BAR, RIGHT_BAR, SWATCH], &resolution)?;
    let named = swatch
        .unresolved_report()
        .is_some_and(|report| report.contains(UNSTOCKED) && report.contains(ITS_UNCOVERED_KEY));

    assert_eq!(
        (planned.len(), textured(&planned), named),
        (FILLS_ALONE, 0, true),
        "the key this block declares against `north` occupies no layer, so the indicator draws no \
         swatch and no ring around where one would have been, and says which block and which key \
         it was. Resolving it to layer zero would draw another block's texture — a picture that is \
         wrong in an entirely plausible way — and naming only the block would leave an author \
         holding six keys to guess which of them is the one without art. Its `up` key *is* \
         covered, so an indicator consulting the wrong facing would report nothing wrong at all"
    );
    Ok(())
}

#[test]
fn an_empty_hand_reports_nothing_held_rather_than_the_block_last_held() -> TestResult {
    let resolution = resolution()?;
    let held = BlockName::parse(BANDED)?;

    let while_holding = held_swatch(Some(&held), &resolution);
    let after = held_swatch(None, &resolution);
    let planned = composed(&after, &[SWATCH], &resolution)?;

    assert_eq!(
        (
            while_holding.texture().is_some(),
            after.texture(),
            after.unresolved_report(),
            planned.len()
        ),
        (true, None, None, 0),
        "an empty hand is not a fault and is not the block that was in it a moment ago. The \
         first half is what stops this passing on a lookup that resolves nothing for anybody: \
         the same block did resolve while it was held"
    );
    Ok(())
}

#[test]
fn the_unresolved_report_names_the_facing_the_indicator_looked_at() -> TestResult {
    let resolution = resolution()?;
    let held = BlockName::parse(UNSTOCKED)?;

    let report = held_swatch(Some(&held), &resolution)
        .unresolved_report()
        .ok_or("a held block whose key occupies no layer has something to report")?;

    assert!(
        report.contains(Face::North.as_str()),
        "the indicator consults one stated facing, and the report has to say which. A block \
         declares up to six keys and only one of them is the one without art; a sentence naming \
         the block and the key but not the facing leaves the author unable to tell a mistake in \
         their declaration from a missing image. What the report said was: {report}"
    );
    Ok(())
}
