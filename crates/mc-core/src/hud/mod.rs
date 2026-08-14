//! What a HUD element declaration is, and the checking that turns one into an
//! element.
//!
//! Deliberately format-agnostic. A block declaration is checked in `mc-world`
//! because its raw form is spelled in TOML and `toml` may not reach this crate;
//! a HUD declaration is checked *here* because both the loader and the composer
//! need the model, so the raw form is expressed over [`DeclaredValue`] — an
//! untyped value this crate owns — and the reader that turns a file into one
//! lives beside the file format.
//!
//! No element is defined here. Every name, anchor, colour and draw kind arrives
//! through a declaration, which is why the base game's HUD is content on exactly
//! the terms a third-party mod's is.

mod declared;
mod element;
mod layout;
mod raw;
pub mod source;

pub use declared::DeclaredValue;
pub use element::{
    ACCEPTED_FIELDS, ANCHOR_NAMES, Anchor, DRAW_KINDS, Draw, HudElement, HudOrigin,
    READABLE_VALUES, ReadableValue, Rgba8,
};
pub use layout::{HudLayout, HudLoadError};
pub use raw::RawHudElement;
