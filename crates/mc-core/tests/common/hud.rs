//! Fixture builders for the HUD declaration suites.
//!
//! Every fixture is built from a *declaration* rather than from a constructed
//! [`HudElement`], because the only way content reaches the model is through a
//! declaration and a fixture that skipped it would grade a path no content
//! takes. A HUD element the engine ships in Rust is not expressible here, which
//! is the point.
//!
//! The builders take the spec's declaration table as their starting point and
//! let each scenario perturb exactly one field, so that what a test is about is
//! the difference between its fixture and [`minimal_fill`] or
//! [`minimal_block_texture`].

use std::error::Error;

use mc_core::hud::source::HudFault;
use mc_core::hud::{DeclaredValue, HudElement, HudOrigin, RawHudElement};

/// A declaration's keys and values, before it has been handed to the model.
pub type Declaration = Vec<(&'static str, DeclaredValue)>;

/// Where every fixture in these suites was declared.
pub const FIXTURE_ORIGIN: &str = "crosshair-horizontal.toml";

/// The name every fixture declares, unless the scenario is about the name.
pub const FIXTURE_NAME: &str = "base:crosshair-horizontal";

/// `#RRGGBBAA` white, the spelling the spec's table states.
pub const OPAQUE_WHITE: &str = "#FFFFFFFF";

/// `#RRGGBBAA` black, the spelling the spec's table states for outlines.
pub const OPAQUE_BLACK: &str = "#000000FF";

/// A declared string.
pub fn text(spelled: &str) -> DeclaredValue {
    DeclaredValue::Text(spelled.to_owned())
}

/// A declared whole number.
pub fn integer(number: i64) -> DeclaredValue {
    DeclaredValue::Integer(number)
}

/// A declared pair of whole numbers, as `offset` and `size` are written.
pub fn extents(across: i64, down: i64) -> DeclaredValue {
    DeclaredValue::List(vec![integer(across), integer(down)])
}

/// Where the fixtures were declared.
pub fn origin() -> HudOrigin {
    HudOrigin::new(FIXTURE_ORIGIN)
}

/// The smallest declaration the spec's table accepts: a centred white fill,
/// stating no offset and no outline.
pub fn minimal_fill() -> Declaration {
    vec![
        ("name", text(FIXTURE_NAME)),
        ("anchor", text("center")),
        ("size", extents(9, 1)),
        ("draw", text("fill")),
        ("color", text(OPAQUE_WHITE)),
    ]
}

/// The smallest declaration drawn from a block texture rather than a colour.
pub fn minimal_block_texture() -> Declaration {
    vec![
        ("name", text(FIXTURE_NAME)),
        ("anchor", text("bottom")),
        ("size", extents(24, 24)),
        ("draw", text("block-texture")),
        ("source", text("held-block")),
    ]
}

/// `declaration` with `key` removed.
pub fn without(declaration: Declaration, key: &str) -> Declaration {
    declaration
        .into_iter()
        .filter(|(spelled, _)| *spelled != key)
        .collect()
}

/// `declaration` stating `key` as `value`, replacing any it already stated.
pub fn with(declaration: Declaration, key: &'static str, value: DeclaredValue) -> Declaration {
    let mut stated = without(declaration, key);
    stated.push((key, value));
    stated
}

/// `declaration` as a source hands it over, before anything about it has been
/// checked.
///
/// This is the only shape a HUD declaration reaches the model in, which is why
/// the source fixtures hold these rather than checked elements: a scenario about
/// a declaration stating `size = [0, 4]` cannot be expressed by a fixture that
/// holds already-accepted elements, and hand-building the fault such a
/// declaration is supposed to produce would grade nothing.
pub fn declared(declaration: Declaration) -> RawHudElement {
    RawHudElement::new(
        declaration
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

/// The element `declaration` registers as.
///
/// # Errors
///
/// Returns the refusal's own message if the model would not register it.
pub fn registered(declaration: Declaration) -> Result<HudElement, Box<dyn Error>> {
    match check(declaration) {
        Ok(element) => Ok(element),
        Err(fault) => {
            Err(format!("the declaration must register, but was refused: {fault}").into())
        }
    }
}

/// The fault `declaration` is refused with.
///
/// # Errors
///
/// Fails if the model registers the declaration instead of refusing it — a
/// scenario about a refusal that accepted an element has learned nothing.
pub fn refused(declaration: Declaration) -> Result<HudFault, Box<dyn Error>> {
    match check(declaration) {
        Ok(element) => {
            Err(format!("the declaration must be refused, but registered {element:?}").into())
        }
        Err(fault) => Ok(fault),
    }
}

/// The lowercase hyphenated words `cause` spells, split at every character a
/// message might delimit a listing with.
///
/// Tokenising rather than searching for substrings is what makes a
/// "lists the accepted values" check unabsorbable: `top` is a substring of
/// `top-left`, so a substring search for the nine anchors is satisfied by a
/// message that lists only the four compound ones and `center`. Splitting on
/// anything that is not a lowercase letter or a hyphen also leaves the message
/// free to quote, comma-separate or bullet its listing however it likes.
pub fn listed_words(cause: &str) -> Vec<&str> {
    cause
        .split(|character: char| !character.is_ascii_lowercase() && character != '-')
        .filter(|word| !word.is_empty())
        .collect()
}

fn check(declaration: Declaration) -> Result<HudElement, HudFault> {
    declared(declaration).into_element(&origin())
}
