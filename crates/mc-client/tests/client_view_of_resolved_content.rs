//! The client's view of content is built from the resolved value it was handed
//! and from nothing else.
//!
//! # This is the one assertion that separates a seam from a rename, and it only
//! works because the fixture is written down
//!
//! **The resolved content below is literal data.** No content root is copied, no
//! path is opened, no block registry is built and no scripting host is
//! constructed anywhere in this file — the only things it names are the value
//! and the view built from it. That is a constraint on the code that builds the
//! fixture, and **no assertion in this file can enforce it**: it is held by
//! whoever writes the fixture and by whoever reads it in review, which is why it
//! is stated here rather than assumed.
//!
//! What it rules out is the pair of failures that would leave every other
//! scenario in this phase green while nothing had actually been cut: a resolved
//! value that is a newtype over the registry, and a client view that reaches
//! back through one. Either would make this file fail to compile or fail to run
//! without a content root, which is exactly the alarm wanted.
//!
//! # The stated assignment disagrees with the sorted position on purpose
//!
//! Layers used to be handed out as a key's position in the lexicographically
//! sorted key set. If this fixture's assignment agreed with that order, the view
//! could go on deriving one and nothing here could fail — two copies of one
//! decision agreeing with each other. The three keys sort `jade`, `onyx`,
//! `quartz`, and the assignment names them `1`, `2`, `0`.
//!
//! # A block's texture key is never its own name
//!
//! Each block below names a key that is not its own name, so a view that
//! reported a block's layer by looking its *name* up in the assignment answers
//! nothing for any of the three rather than accidentally answering correctly.

use std::error::Error;

use mc_client::content::ContentView;
use mc_core::content::{ResolvedBlock, ResolvedContent};
use mc_core::id::{BlockName, TextureKey};

type TestResult = Result<(), Box<dyn Error>>;

/// The three blocks the value below states: the name, the texture key, whether
/// the block is solid, and the layer the assignment names for that key.
///
/// One table, from which both the fixture and the expectation are built, so
/// nothing here is a number copied out of a run.
const STATED: [(&str, &str, bool, u16); 3] = [
    ("example:amber", "example:quartz", true, 0),
    ("example:cobalt", "example:onyx", false, 2),
    ("example:zinc", "example:jade", true, 1),
];

/// What the view reports for one block: the layer its texture key occupies, and
/// whether it is solid.
type Reported = (Option<u16>, Option<bool>);

/// A block the value states nothing about.
///
/// A view that answered for a block it was never handed would be inventing
/// content, which is the whole failure this seam exists to make impossible.
const UNSTATED_BLOCK: &str = "example:tin";

#[test]
fn the_clients_view_reports_each_stated_blocks_layer_and_solidity_as_the_value_states_them()
-> TestResult {
    let content = written_down()?;

    let view = ContentView::of(&content);

    assert_eq!(
        (
            reported_by(&view)?,
            view.is_solid(&BlockName::parse(UNSTATED_BLOCK)?)
        ),
        (expected(), None),
        "the simulation reads a content root and the client is handed what came back. What the \
         client draws and meshes from has to be built from that value alone — a view that \
         reached back through a registry, or a resolved value that were one, would leave every \
         other reading in this phase green while nothing had been cut. The layers stated here \
         are deliberately not the sorted position of their keys, so a view that goes on \
         deriving an index answers `2`, `1`, `0` where the value says `0`, `2`, `1`"
    );
    Ok(())
}

/// The resolved content value, stated rather than read from anywhere.
///
/// # Errors
///
/// Returns an error if a fixture id is not a namespaced id.
fn written_down() -> Result<ResolvedContent, Box<dyn Error>> {
    let mut blocks = Vec::new();
    let mut assignment = Vec::new();
    for (name, texture, is_solid, layer) in STATED {
        blocks.push(ResolvedBlock {
            name: BlockName::parse(name)?,
            texture: TextureKey::parse(texture)?,
            is_solid,
        });
        assignment.push((TextureKey::parse(texture)?, layer));
    }
    Ok(ResolvedContent::stating(blocks, assignment))
}

/// What `view` reports for each block the value states: the layer its texture
/// key occupies, and whether it is solid.
///
/// # Errors
///
/// Returns an error if a fixture id is not a namespaced id.
fn reported_by(view: &ContentView) -> Result<Vec<Reported>, Box<dyn Error>> {
    let mut reported = Vec::new();
    for (name, texture, _, _) in STATED {
        reported.push((
            view.layers().layer_of(&TextureKey::parse(texture)?),
            view.is_solid(&BlockName::parse(name)?),
        ));
    }
    Ok(reported)
}

/// What the value states, in the shape the view reports it.
fn expected() -> Vec<Reported> {
    STATED
        .iter()
        .map(|(_, _, is_solid, layer)| (Some(*layer), Some(*is_solid)))
        .collect()
}
