//! What a layout composes when the block the session holds has no texture to
//! draw.
//!
//! # "No indicator" is a ruling about the outline too, and it is made here
//!
//! An outline is composed for an element of any draw kind, so a declaration that
//! states one and resolves to nothing would leave a black ring around nothing at
//! the anchor the indicator names. That is an indicator — a reader sees a
//! bordered empty square where a swatch belongs, and "the system drew no
//! indicator" would be false of the picture while true of the fill. So **an
//! element whose draw resolves to nothing composes no ring either**, and the
//! assertion below is stated over the whole plan rather than over the textured
//! rectangles in it, because a count of textured rectangles cannot see a ring.
//!
//! The rule is keyed on the paint being unavailable and never on clipping: an
//! element pushed off the target still has a fill to draw and still draws its
//! outline as far as the target reaches, which is a different question and one
//! the clipping scenarios already own.
//!
//! # Every number here is derived from the declarations, not from a run
//!
//! Two of the three fixtures are fills stating no outline, so each contributes
//! exactly one rectangle; the third resolves to nothing and contributes none.
//! Two is therefore the whole plan. A composition that resolved the unstocked
//! block to some other block's layer reads three, and one that kept the ring
//! reads six — the four strips of `26 × 26 − 24 × 24` plus the two fills.
//!
//! # The fixtures name nothing the base game ships
//!
//! This file sits under `src/`, where the scan forbidding shipped HUD element
//! names and colour literals in Rust looks. Every name below is in a `fixture:`
//! namespace and every colour is one no shipped declaration states.

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

use mc_core::hud::source::InMemoryHudSource;
use mc_core::hud::{DeclaredValue, HudLayout, HudOrigin, RawHudElement};
use mc_core::id::{BlockName, TextureKey};

use crate::hud::{HudFrame, Painted, PaintedRect, compose};
use crate::surface::SurfaceSize;
use crate::texture::TextureLayers;

use super::held_swatch;

type TestResult = Result<(), Box<dyn Error>>;

/// The reference target, where one UI unit is one physical pixel.
const TARGET: SurfaceSize = SurfaceSize {
    width: 1280,
    height: 720,
};

/// A block the array texture was filled for, so the layers below hold something
/// and "resolves to no layer" is a fact about the held block rather than about
/// an empty lookup.
const STOCKED: &str = "fixture:stocked";

/// The block the session holds. Nothing meshed it, so no layer was ever
/// assigned to it — the spelling gap, arriving exactly as it does in the
/// product.
const UNSTOCKED: &str = "fixture:unstocked";

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

/// The layers an array texture filled for [`STOCKED`] alone resolved to.
///
/// # Errors
///
/// Fails when the fixture's own key is not a namespaced id.
fn stocked_layers() -> Result<TextureLayers, Box<dyn Error>> {
    let keys = BTreeSet::from([TextureKey::parse(STOCKED)?]);
    Ok(TextureLayers::resolve(&keys))
}

/// How many of `planned` sample the array texture.
fn textured(planned: &[PaintedRect]) -> usize {
    planned
        .iter()
        .filter(|rect| matches!(rect.paint, Painted::Texture(_)))
        .count()
}

#[test]
fn a_held_block_whose_texture_occupies_no_layer_draws_nothing_and_is_reported_by_name() -> TestResult
{
    let layers = stocked_layers()?;
    let held = BlockName::parse(UNSTOCKED)?;
    let layout = layout_of(&[LEFT_BAR, RIGHT_BAR, SWATCH])?;

    let swatch = held_swatch(Some(&held), &layers);
    let planned = compose(
        &HudFrame {
            layout: Arc::new(layout),
            held: swatch.texture(),
        },
        TARGET,
        &layers,
    );
    let named = swatch
        .unresolved_report()
        .is_some_and(|report| report.contains(UNSTOCKED));

    assert_eq!(
        (planned.len(), textured(&planned), named),
        (FILLS_ALONE, 0, true),
        "a held block with no array layer draws no swatch and no ring around where one would \
         have been, and says which block it was — resolving it to another block's layer would \
         draw a plausible picture of the wrong thing, and saying nothing would leave a content \
         author with a missing indicator and no reason for it"
    );
    Ok(())
}
