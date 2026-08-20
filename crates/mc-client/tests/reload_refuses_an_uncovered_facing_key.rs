//! A reload naming a facing key the session cannot give a layer is refused, and
//! the sections already drawn keep drawing exactly what they drew.
//!
//! # The refusal is the one that already existed, reached by a new route
//!
//! A session hands out 256 array-texture layers and a relaunch is the only thing
//! that gives any back, so a content set needing one more than remains is
//! refused whole. What is new is that a **facing** key can now be the one that
//! does not fit: an author re-pointing a single side of a single block is asking
//! for a layer exactly as an author declaring a new block is.
//!
//! # What this adds to the budget refusal that is already covered
//!
//! That such a candidate is turned away, and that no layer is quietly appended
//! on the way out, is already read elsewhere. This asks the question those cannot:
//! **what is on the screen afterwards.** A refusal that had nonetheless moved the
//! resolution the packer holds would leave the sections already uploaded drawing
//! a key nobody accepted — and every assertion about the published assignment
//! would still be green, because the assignment is not what a corner carries.
//!
//! So the reading below goes through the packer, both before the candidate is
//! read and after, and against a **stated** expectation rather than against
//! itself: the north faces draw the key the serving content declares there, which
//! is not the key that block's name spells and not the key the refused candidate
//! wanted.
//!
//! # The session is at its budget by construction, and the launch is what fills it
//!
//! The launch content re-points `north` at one key beyond the four the shipped
//! root declares, and the session it launches into has one layer left. That is
//! what makes the launch fit exactly and the next key not fit at all — both
//! arithmetic, neither a digit.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_content.rs"]
mod reload_content;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::sync::Arc;

use mc_client::content::ContentView;
use mc_core::id::{BlockName, TextureKey};
use mc_render::geometry::{SectionOrigin, build_section_geometry};
use mc_render::texture::TextureResolution;
use mc_sim::simulation::PublishedContent;
use mc_world::mesh::{Facing, PlaneExtent, PlanePos, Quad};

use input::InputHarness;
use reload::{Declaration, STONE, STONE_FILE, restating, shipped};
use reload_content::{Reading, reading, spent_all_but_one};
use reload_world::{floor_of, playing_serving, standing};
use support::TestResult;

/// How many array-texture layers one session may assign: the declared budget.
///
/// **Written out rather than read from `LAYERS_A_SESSION_MAY_ASSIGN`.** That
/// constant is a declaration this suite grades, and a message assembled from it
/// reads back whatever it became. The value is the packed vertex's eight-bit
/// layer field — two to the eighth.
const A_SESSIONS_BUDGET: usize = 256;

/// The key the launch content points `north` at, and the key the refused
/// candidate wants there instead.
///
/// Neither is the block's own name. The first is what the sections on screen are
/// drawing; the second is what nothing must end up drawing.
const SERVING: &str = "base:cobalt";
const REFUSED: &str = "base:diorite";

/// How many corners one quad is packed into.
///
/// Stated so the expectation covers the whole run of corners rather than
/// sampling one: a packer writing the right layer into the first corner and the
/// wrong one into the other three draws three quarters of every face wrong.
const CORNERS_PER_QUAD: usize = 4;

/// Where the packed section sits. Nothing here depends on it.
const SECTION: [i32; 3] = [0, 0, 0];

/// What packing a quad came to.
///
/// A total verdict rather than an error propagated out of the test: a packer that
/// refused what it should accept fails on the comparison instead of ending the
/// test before its assertion ran.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Packed {
    /// The layer every corner was packed with.
    Corners(Vec<u16>),
    /// The section was refused, saying this.
    Refused(String),
}

#[test]
fn a_reload_naming_a_facing_key_with_no_layer_is_refused_and_the_sections_keep_drawing()
-> TestResult {
    let client = a_client_whose_north_faces_draw_the_serving_key()?;
    let drawing = Packed::Corners(vec![layer_of(&client, SERVING)?; CORNERS_PER_QUAD]);
    let before = north_face_of_stone(&client)?;

    let candidate = restating(
        shipped()?,
        STONE_FILE,
        &Declaration::of(STONE).repointing_north(REFUSED),
    )?;
    let read = reading(&candidate, serving(&client)?.resolved.layers())?;
    let after = north_face_of_stone(&client)?;

    assert_eq!(
        (read, after, before),
        (
            Reading::Refused {
                said: over_budget(A_SESSIONS_BUDGET + 1, A_SESSIONS_BUDGET)
            },
            drawing.clone(),
            drawing
        ),
        "the session has handed out every layer it has and the candidate re-points one facing of \
         one block at a key that would need another, so it is refused whole. What the sections on \
         screen go on drawing is the reading that matters: they were packed against {SERVING} and \
         they still are. A refusal that had nonetheless moved what the packer resolves against \
         would draw a key nobody accepted, and every assertion about the published assignment \
         would stay green while it did — an assignment is not what a corner carries"
    );
    Ok(())
}

/// What a refusal over the session's budget says, in the words a page quotes.
///
/// The counts are the caller's arithmetic; the budget is [`A_SESSIONS_BUDGET`].
fn over_budget(needed: usize, spent: usize) -> String {
    format!(
        "this content needs {needed} texture layers and a session has {A_SESSIONS_BUDGET}; \
         {spent} are already assigned, and relaunching reclaims every layer retired since the \
         client started"
    )
}

/// A client playing a floor of stone, whose stone declares [`SERVING`] against
/// `north`, and whose session has spent every layer of its budget.
///
/// The launch is what spends the last one: the session starts with a single
/// layer free and the launch content declares a fifth key, so the layer that
/// fits is the last one there is.
fn a_client_whose_north_faces_draw_the_serving_key() -> Result<InputHarness, Box<dyn Error>> {
    let root = restating(
        shipped()?,
        STONE_FILE,
        &Declaration::of(STONE).repointing_north(SERVING),
    )?;
    let (simulation, holding) = playing_serving(
        root.path(),
        standing(),
        |registry| floor_of(registry, STONE),
        &spent_all_but_one()?,
    )?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// The content `client` is publishing.
///
/// # Errors
///
/// Returns an error where the client publishes none, which is a client with no
/// world rather than one serving anything a candidate could be read against.
fn serving(client: &InputHarness) -> Result<Arc<PublishedContent>, Box<dyn Error>> {
    client.content().ok_or_else(|| {
        "this fixture's client publishes no content, so there are no spent layers to read a \
         candidate against"
            .into()
    })
}

/// The layer the content `client` serves states for `key`.
///
/// # Errors
///
/// Returns an error if the key does not parse, if nothing is serving, or if the
/// serving content assigns it no layer — a fixture whose key never reached the
/// assignment would make the reading vacuous.
fn layer_of(client: &InputHarness, key: &str) -> Result<u16, Box<dyn Error>> {
    let wanted = TextureKey::parse(key)?;
    serving(client)?
        .resolved
        .layer_assignment()
        .find(|(named, _)| **named == wanted)
        .map(|(_, layer)| layer)
        .ok_or_else(|| {
            format!("the content this client serves has to assign `{key}` a layer").into()
        })
}

/// What packing one negative-Z face of stone against the resolution `client` is
/// serving came to.
///
/// A hand-built quad rather than one taken out of a mesh: the subject is which
/// layer the packer resolves for a stated facing of a stated block, and a mesh
/// would supply the same quad with more machinery in front of it.
///
/// # Errors
///
/// Returns an error if the block name does not parse or if nothing is serving.
fn north_face_of_stone(client: &InputHarness) -> Result<Packed, Box<dyn Error>> {
    let resolution: TextureResolution =
        ContentView::of(&serving(client)?.resolved).into_resolution();
    let quads = [Quad {
        facing: Facing::NegZ,
        plane: 3,
        origin: PlanePos {
            primary: 0,
            secondary: 0,
        },
        extent: PlaneExtent {
            primary: 1,
            secondary: 1,
        },
        block: BlockName::parse(STONE)?,
    }];
    Ok(
        match build_section_geometry(&quads, SectionOrigin::new(SECTION), &resolution) {
            Ok(geometry) => Packed::Corners(
                (0usize..)
                    .map_while(|corner| geometry.layer_at(corner))
                    .collect(),
            ),
            Err(refused) => Packed::Refused(refused.to_string()),
        },
    )
}
