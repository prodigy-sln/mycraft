//! What a client receives when the simulation has read a content root: the
//! fields it draws and predicts with, and none of the rules by which a world is
//! mutated.
//!
//! # Asserted by discrimination, never by absence
//!
//! A type that simply has no `breakable` field cannot fail a test about not
//! having one — the test would not compile, and a compile error standing in for
//! an assertion says nothing about which fields the resolution actually reads.
//! So the oracle is a pair of readings that pull in opposite directions:
//!
//! - Two roots differing **only** in `replaceable`, `breakable` and
//!   `breaks_into` resolve to one and the same client content. On its own that
//!   is satisfied by a resolver returning a constant.
//! - Two roots differing in one block's `texture` and another block's `solid`
//!   resolve to content that differs in **both**. That is what rules the
//!   constant out.
//!
//! Neither direction is evidence alone, which is why both are here and why the
//! first also reads the content back rather than only comparing the two roots to
//! each other.
//!
//! # A block's texture key is never its own name
//!
//! Every fixture below states them separately, for the reason the block
//! declaration suites in `mc-world` state in full: a resolution that read a
//! block's name into both fields would be green against any fixture that
//! declared the two identically, and that confusion survived this feature's
//! whole first draft.
//!
//! # The layer assignment is read back by shape, not by mapping
//!
//! Which key lands on which layer is the simulation's to decide and nothing here
//! depends on it. What these readings do pin is that the assignment names one
//! distinct layer for every texture key the root declares and no layer for
//! anything else — a resolution handing back an empty assignment, or one key
//! short, would otherwise satisfy both scenarios above for the wrong reason.

use std::error::Error;
use std::fs;

use mc_core::content::{FaceTextures, LayerAssignment, ResolvedContent};
use mc_core::id::TextureKey;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// The subdirectory of a content root that block declarations live in.
const BLOCKS_DIRECTORY: &str = "blocks";

/// One block as a fixture declares it: the file it is written to, the name it
/// gives itself, the texture key it names, whether it is solid, and whatever
/// else it states.
type Declared = (&'static str, &'static str, &'static str, bool, &'static str);

/// Three blocks stating nothing beyond the three required fields.
const PLAINLY: [Declared; 3] = [
    ("amber.luau", "example:amber", "example:quartz", true, ""),
    ("cobalt.luau", "example:cobalt", "example:onyx", false, ""),
    ("zinc.luau", "example:zinc", "example:jade", true, ""),
];

/// The same three blocks, each stating a different rule by which a world is
/// mutated — and every one of the three such rules is stated somewhere here, so
/// a resolution that carried one of them across is caught whichever it is.
const MUTATED_BY_OTHER_RULES: [Declared; 3] = [
    (
        "amber.luau",
        "example:amber",
        "example:quartz",
        true,
        "\treplaceable = true,\n",
    ),
    (
        "cobalt.luau",
        "example:cobalt",
        "example:onyx",
        false,
        "\tbreakable = false,\n",
    ),
    (
        "zinc.luau",
        "example:zinc",
        "example:jade",
        true,
        "\tbreaks_into = 'example:ash',\n",
    ),
];

/// The same three blocks with one block's texture key and another block's
/// solidity changed, and nothing else.
///
/// The two changes sit on **different** blocks on purpose: a resolution that
/// carried only one of the two fields would otherwise still report a difference
/// on the block that changed in both.
const DRAWN_DIFFERENTLY: [Declared; 3] = [
    ("amber.luau", "example:amber", "example:quartz", true, ""),
    ("cobalt.luau", "example:cobalt", "example:beryl", false, ""),
    ("zinc.luau", "example:zinc", "example:jade", false, ""),
];

/// Three blocks stating three different things about how much light they stop.
///
/// **`occludes = false` is written out on every block that passes light, and it
/// is not decoration.** `occludes` falls back to `solid`, so a solid block
/// declaring a degree below one resolves to a block that both passes light and
/// hides what lies behind it — which the loader refuses, taking the whole root
/// with it. The refusal would be correct and the fixture would be at fault, and
/// the reading below would fail for a reason that has nothing to do with the
/// seam it is about.
///
/// `example:zinc` states no degree at all, which is the case a default has to
/// answer for; the other two state one either side of it.
const STOPPING_DIFFERENT_AMOUNTS_OF_LIGHT: [Declared; 3] = [
    (
        "amber.luau",
        "example:amber",
        "example:quartz",
        true,
        "\toccludes = false,\n\topacity = 0.25,\n",
    ),
    (
        "cobalt.luau",
        "example:cobalt",
        "example:onyx",
        false,
        "\topacity = 1.0,\n",
    ),
    ("zinc.luau", "example:zinc", "example:jade", true, ""),
];

/// What each of those blocks has to resolve to, block by block.
///
/// **Written from the declarations rather than read back from a run**, and
/// stated as the value rather than as "not opaque": a `resolved_from` that
/// swapped two blocks' degrees satisfies every reading that only asks whether a
/// block differs from its neighbour.
const THE_DEGREES_THEY_DECLARE: [(&str, f32); 3] = [
    ("example:amber", 0.25),
    ("example:cobalt", 1.0),
    ("example:zinc", 1.0),
];

/// What a resolved content value states: each block's name, texture key and
/// solidity, then the texture keys its layer assignment names and the layers it
/// hands out.
type Stated = (Vec<(String, String, bool)>, Vec<String>, Vec<u16>);

#[test]
fn two_roots_differing_only_in_how_a_world_is_mutated_resolve_the_same_client_content() -> TestResult
{
    let plain = root_declaring(&PLAINLY)?;
    let mutated = root_declaring(&MUTATED_BY_OTHER_RULES)?;

    let from_plain = mc_sim::content::load(plain.path(), &LayerAssignment::none())?.resolved;
    let from_mutated = mc_sim::content::load(mutated.path(), &LayerAssignment::none())?.resolved;

    assert_eq!(
        (stated(&from_plain), from_plain),
        (expected(&PLAINLY), from_mutated),
        "`replaceable`, `breakable` and `breaks_into` are the rules by which the world is \
         mutated, and the server recomputes every one of them — a client that received them \
         would be holding rules it may not apply. Two roots that differ in nothing else must \
         therefore be indistinguishable to a client. The content is read back beside the \
         comparison because two roots resolving alike is also what a resolver answering with a \
         constant does"
    );
    Ok(())
}

#[test]
fn two_roots_differing_in_a_texture_and_a_solidity_resolve_client_content_that_differs_in_both()
-> TestResult {
    let plain = root_declaring(&PLAINLY)?;
    let drawn_differently = root_declaring(&DRAWN_DIFFERENTLY)?;

    let from_plain = mc_sim::content::load(plain.path(), &LayerAssignment::none())?.resolved;
    let from_drawn_differently =
        mc_sim::content::load(drawn_differently.path(), &LayerAssignment::none())?.resolved;

    assert_eq!(
        (stated(&from_plain), stated(&from_drawn_differently)),
        (expected(&PLAINLY), expected(&DRAWN_DIFFERENTLY)),
        "a texture key is what the renderer draws with and solidity is what the mesher culls \
         faces by, so both are the client's to receive. Two roots differing in one of each must \
         resolve to content differing in both — a resolution that dropped either would report \
         one difference where there are two, and one that answered with a constant would report \
         none at all"
    );
    Ok(())
}

#[test]
fn every_blocks_declared_degree_of_opacity_reaches_the_content_a_client_receives() -> TestResult {
    let root = root_declaring(&STOPPING_DIFFERENT_AMOUNTS_OF_LIGHT)?;

    let resolved = mc_sim::content::load(root.path(), &LayerAssignment::none())?.resolved;

    assert_eq!(
        degrees(&resolved),
        THE_DEGREES_THEY_DECLARE
            .iter()
            .map(|(name, degree)| ((*name).to_owned(), *degree))
            .collect::<Vec<_>>(),
        "how much light a block stops is what decides which of the two terrain draws its faces          land in, and this copy is the only place a declared degree crosses into what a client          receives. Asserted here at the boundary rather than inferred from a frame, because every          other witness to this line is a rendered picture that needs a device — and read as the          value each block declares, beside the name that declared it, so that a copy handing back          a constant, a default, or one block's degree under another block's name is three          different failures rather than one silence"
    );
    Ok(())
}

/// The degree each block of `content` stops light at, beside its name, sorted by
/// that name.
///
/// Sorted rather than taken in registration order: which order a directory is
/// read in is not what this reading is about, and the names travel beside the
/// degrees so nothing is lost by ordering them.
fn degrees(content: &ResolvedContent) -> Vec<(String, f32)> {
    let mut found: Vec<(String, f32)> = content
        .blocks()
        .map(|block| (block.name.as_str().to_owned(), block.opacity.get()))
        .collect();
    found.sort_by(|one, other| one.0.cmp(&other.0));
    found
}

/// A content root declaring exactly `blocks`, in a temporary directory.
///
/// The root is real and so are the files: what these scenarios ask about is the
/// reading of a content root by the simulation, and a mocked filesystem would
/// assert nothing about it. No `hud/` is written, a root declaring no HUD being
/// a valid, empty answer.
///
/// # Errors
///
/// Returns an error if the directory or a declaration cannot be written.
fn root_declaring(blocks: &[Declared]) -> Result<TempDir, Box<dyn Error>> {
    let root = TempDir::new()?;
    let declarations = root.path().join(BLOCKS_DIRECTORY);
    fs::create_dir_all(&declarations)?;
    for (file, name, texture, solid, more) in blocks {
        fs::write(
            declarations.join(file),
            declaration(name, texture, *solid, more),
        )?;
    }
    Ok(root)
}

/// A chunk returning one block declaration.
fn declaration(name: &str, texture: &str, solid: bool, more: &str) -> String {
    format!(
        "return {{\n\tname = '{name}',\n\ttexture = '{texture}',\n\tsolid = {solid},\n{more}}}\n"
    )
}

/// What `content` states, read back in the shape the expectations below are
/// built in.
fn stated(content: &ResolvedContent) -> Stated {
    let blocks = content
        .blocks()
        .map(|block| {
            (
                block.name.as_str().to_owned(),
                textured(&block.textures),
                block.is_solid,
            )
        })
        .collect();
    let mut keys: Vec<String> = content
        .layer_assignment()
        .map(|(key, _)| key.as_str().to_owned())
        .collect();
    let mut layers: Vec<u16> = content.layer_assignment().map(|(_, layer)| layer).collect();
    keys.sort();
    layers.sort_unstable();
    (blocks, keys, layers)
}

/// What a root declaring `blocks` must resolve to.
///
/// Derived from the fixture table rather than from a run of the resolution, so
/// editing a declaration moves the expectation with it and no number here was
/// ever copied out of the code under test. Blocks come back in registration
/// order, which is the file-name order these tables are written in.
fn expected(blocks: &[Declared]) -> Stated {
    let declared = blocks
        .iter()
        .map(|(_, name, texture, solid, _)| ((*name).to_owned(), (*texture).to_owned(), *solid))
        .collect();
    let mut keys: Vec<String> = blocks
        .iter()
        .map(|(_, _, texture, _, _)| (*texture).to_owned())
        .collect();
    keys.sort();
    // One distinct layer per declared texture key, counted from zero. Which key
    // occupies which is not pinned here; that a key is missed, doubled or left
    // unassigned is.
    let layers = (0..u16::try_from(keys.len()).unwrap_or(u16::MAX)).collect();
    (declared, keys, layers)
}

/// Every key a block's six facings draw from, joined — one key where all six
/// agree, and a list where they do not.
///
/// **Total over the six rather than a reading of one of them.** Every fixture in
/// this file states its texture as a single string, so the answer is one key; a
/// resolver that lost five facings, or that answered one facing's key for all six
/// while the declaration said otherwise, changes this string rather than hiding
/// behind whichever facing happened to be read.
fn textured(textures: &FaceTextures) -> String {
    textures
        .keys()
        .iter()
        .map(TextureKey::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}
