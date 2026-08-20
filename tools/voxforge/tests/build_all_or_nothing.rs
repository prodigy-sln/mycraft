//! A refused build changes nothing that was already there.
//!
//! **The refusal has to land on an entry, not on a model, and the fixture is
//! built for that.** The build loads and emits each model once and then judges
//! the faces its entries selected, so a manifest whose fourth *entry* names a
//! missing *model* is refused before the model that entry does not share is ever
//! opened — and an eager build would have written nothing changed by then, which
//! makes the assertion pass for the wrong reason. That was measured, not
//! guessed: it left this reading green under a build that writes each image as
//! it emits it.
//!
//! So the second build replaces the one model with a ramp, and the fourth entry
//! selects the ramp's `front` face, which does not tile. The three entries
//! before it select faces that do, and their images are now a different colour.
//! An eager build lands those three and reddens this; a build that judges before
//! it opens anything lands nothing and leaves the previous set exactly as it
//! was.
//!
//! What depends on this is one phase away: a refused art build leaves a stale
//! set on disk, which is why the gate must not go on to test against it.

#[path = "common/build.rs"]
mod build;
mod common;

use std::error::Error;

use common::TestResult;
use common::texture::{GRADIENT, GREY, Leg, Legs, legs_named};

use build::{
    CUBE_MODEL, Entry, MANIFEST_FILE, Refused, Root, built, entry, manifest, ramped_cube,
    uniform_cube,
};

/// The seven keys the manifest bakes.
const KEYS: [&str; 7] = [
    "base:k0", "base:k1", "base:k2", "base:k3", "base:k4", "base:k5", "base:k6",
];

/// The face each of them selects, in the same order.
///
/// The fourth is the ramp's `front`, which does not tile; the other six select
/// faces that see one slab of the ramp apiece and do.
const FACES: [&str; 7] = ["left", "right", "left", "front", "right", "left", "right"];

/// Which entry the second build is refused on, counted from zero.
const THE_FOURTH: usize = 3;

/// How many entries there are, which the refusal is the fourth of.
const OF_SEVEN: usize = 7;

/// A root holding one cube, the ramp's four tones, and seven block files.
///
/// Every tone of the ramp is declared from the start, so the only source that
/// moves between the two builds is the model itself.
fn root_of_seven() -> Result<Root, Box<dyn Error>> {
    let root = Root::bare()?;
    root.holding(CUBE_MODEL, &uniform_cube(GREY))?
        .painted(&GRADIENT)?
        .painted(&[GREY])?
        .declaring(&KEYS)?
        .holding(MANIFEST_FILE, &seven())?;
    Ok(root)
}

/// The manifest, which does not change between the two builds.
fn seven() -> String {
    let entries: Vec<Entry<'static>> = (0..OF_SEVEN)
        .map(|at| {
            entry(
                KEYS.get(at).copied().unwrap_or_default(),
                CUBE_MODEL,
                FACES.get(at).copied().unwrap_or_default(),
            )
        })
        .collect();
    manifest(1, &entries)
}

#[test]
fn a_refusal_on_the_fourth_entry_leaves_the_previous_builds_output_unchanged() -> TestResult {
    let root = root_of_seven()?;
    let first = built(&root)?;

    root.holding(CUBE_MODEL, &ramped_cube())?;
    let second = built(&root)?;

    let named = KEYS.get(THE_FOURTH).copied().unwrap_or_default();

    assert_eq!(
        (
            second.refusal(&[named]),
            legs_named(&second.err),
            first.images().len(),
            second.fingerprints()
        ),
        (
            Refused::NamingEverything,
            Legs::Only(Leg::Edges),
            OF_SEVEN,
            first.fingerprints()
        ),
        "the set on disk is either the whole of one build or the whole of another, never three \
         entries of a run that stopped. Every image and the index are compared, because an index \
         rewritten beside seven untouched images records a fold over sources the images were \
         never baked from — a set that is stale and says it is current. It said: {err}",
        err = second.err
    );
    Ok(())
}
