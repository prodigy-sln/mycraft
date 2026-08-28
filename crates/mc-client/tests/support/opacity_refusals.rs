//! The three refusals a declared degree of opacity raises that a modding page
//! quotes, each as a person running the client from their own game directory
//! reads it.
//!
//! **Three roots and not one**, for the reason [`crate::printed_refusals`] builds
//! ten: a root is refused whole, so a root carrying two mistakes is refused for
//! whichever the loader reaches first and the second refusal would be one no run
//! ever prints.
//!
//! Every one is produced by the client's own preparation over a real content root
//! and rendered through the shipped reporting. Nothing here writes out what a
//! refusal is expected to say — the wording is the implementer's, and the point of
//! the guard these feed is that the page and the program are compared against each
//! other rather than each against somebody's belief about the other.
//!
//! # Why this is a module of its own
//!
//! [`crate::printed_refusals`] is within fifty non-blank lines of the size the
//! gate allows a test file, and three more producers with the prose they need do
//! not fit. `per_facing_refusals` and `built_set_refusals` set the precedent and
//! the split is a responsibility boundary as well as a line count: everything here
//! is one field's refusals.
//!
//! # Why three and not one, and why the last two are a pair
//!
//! A page quoting one refusal per field would be cheaper and would teach a mod
//! author the wrong thing twice over.
//!
//! - **The ceiling** is the first bound on this declaration anybody can reach.
//!   The two numbers a medium states stop at a width nobody types on purpose;
//!   this one stops at `1.0`, which is a value somebody writes reaching for "more
//!   opaque than opaque".
//! - **The contradiction, written out.** `occludes = true` beside a degree below
//!   one asks for two things the engine cannot both honour, and the remedy is a
//!   line to *delete*.
//! - **The same contradiction, never written.** `occludes` falls back to the
//!   block's own solidity, so `solid = true` alone already says the block hides
//!   what is behind it — and the remedy is the opposite one, a line to **add**.
//!
//! The last two are the pair, and they are why the loader has two sentences
//! rather than one. A single sentence quoting `occludes = true` at an author whose
//! file contains no such line sends them grepping for something that is not there,
//! which is the unfindable refusal this project's whole refusal doctrine exists to
//! prevent.
//!
//! # Each fixture can only be refused by its own arm
//!
//! That is a property of the two declarations rather than a hope about them. The
//! written case states `solid = false`, so the fallback would give `occludes =
//! false` and only the line the author wrote can produce the refusal; the derived
//! case states no `occludes` at all, so only the fallback can. A pair that both
//! stated `solid = true` would leave a reader unable to say which arm either one
//! exercised, and a loader answering both from one arm would satisfy the guard.
//!
//! # The ceiling fixture is deliberately minimal, and that buys a second witness
//!
//! It states `solid = true` and no `occludes`, so it is *also* a block that
//! occludes — and it is refused for the ceiling only because the value is judged
//! before the two fields are compared. A loader that reversed that order would
//! produce the contradiction's sentence here instead, and the guard these feed
//! would report a page quoting a refusal no run prints. The branch order has its
//! own witness in `mc-world`; this one is free.

// Each test binary linking this module drives a subset of it.
#![allow(dead_code)]

use std::error::Error;

use crate::printed_refusals::{BLOCK_FILE, as_read_from_a_game_directory};
use crate::support::content;

/// A degree half again past the ceiling.
///
/// The value somebody writes reaching for "more opaque than opaque", which is
/// what a percentage-shaped intuition produces on a scale that runs to one.
const A_DEGREE_PAST_THE_CEILING: &str = "return {\n\tname = 'example:amber',\n\ttexture = 'example:amber',\n\tsolid = true,\n\topacity = \
     1.5,\n}\n";

/// A block that states in writing both that light passes through it and that it
/// hides what is behind it.
///
/// **`solid = false` on purpose.** The fallback would then answer `occludes =
/// false`, so the only thing that can produce this refusal is the line the author
/// wrote — which is what makes this fixture a witness for the written arm rather
/// than for whichever arm the loader happened to take.
const A_DEGREE_BESIDE_A_WRITTEN_OCCLUSION: &str = "return {\n\tname = 'example:amber',\n\ttexture = 'example:amber',\n\tsolid = false,\n\toccludes \
     = true,\n\topacity = 0.5,\n}\n";

/// A block that states that light passes through it and never mentions occlusion
/// at all.
///
/// The first thing a mod author writes when they reach for glass: solid, because
/// you cannot walk through a pane, and see-through, because that is the point. It
/// is refused, and the sentence it earns has to name `solid` — there is no
/// `occludes` in this file for a reader to find.
const A_DEGREE_ON_A_BLOCK_THAT_SAYS_NOTHING_ABOUT_OCCLUSION: &str = "return {\n\tname = 'example:amber',\n\ttexture = 'example:amber',\n\tsolid = true,\n\topacity = \
     0.5,\n}\n";

/// The three, in the order the page introduces them.
///
/// # Errors
///
/// Returns an error if a fixture root cannot be built, if a root that must refuse
/// is accepted, or if a refusal does not name the fixture root.
pub fn opacity_refusals() -> Result<Vec<String>, Box<dyn Error>> {
    Ok(vec![
        a_degree_past_the_ceiling()?,
        a_degree_beside_a_written_occlusion()?,
        a_degree_on_a_block_that_occludes_by_being_solid()?,
    ])
}

/// What the client writes for a declaration stating a degree above the ceiling.
///
/// # Errors
///
/// Returns an error if the root is accepted, or if the refusal does not name the
/// fixture root.
pub fn a_degree_past_the_ceiling() -> Result<String, Box<dyn Error>> {
    refused_over(A_DEGREE_PAST_THE_CEILING)
}

/// What the client writes for a declaration stating a degree below one beside a
/// written `occludes = true`.
///
/// # Errors
///
/// Returns an error if the root is accepted, or if the refusal does not name the
/// fixture root.
pub fn a_degree_beside_a_written_occlusion() -> Result<String, Box<dyn Error>> {
    refused_over(A_DEGREE_BESIDE_A_WRITTEN_OCCLUSION)
}

/// What the client writes for a solid declaration stating a degree below one and
/// no occlusion at all.
///
/// # Errors
///
/// Returns an error if the root is accepted, or if the refusal does not name the
/// fixture root.
pub fn a_degree_on_a_block_that_occludes_by_being_solid() -> Result<String, Box<dyn Error>> {
    refused_over(A_DEGREE_ON_A_BLOCK_THAT_SAYS_NOTHING_ABOUT_OCCLUSION)
}

/// What the client writes over a copy of the shipped root carrying `declaration`
/// as one extra block.
///
/// A copy rather than the shipped root itself, for the reason
/// `crate::support::content` records: a fixture editing `content/base/` would
/// leave the repository in whatever state a failed run ended in.
fn refused_over(declaration: &str) -> Result<String, Box<dyn Error>> {
    as_read_from_a_game_directory(
        &content::shipped_copy()?.declaring_block(BLOCK_FILE, declaration)?,
    )
}
