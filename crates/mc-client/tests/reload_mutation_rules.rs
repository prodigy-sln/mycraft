//! The three rules by which a world is mutated cross a reload, and a residue
//! naming nothing is still resolved at the break.
//!
//! # These are what is observable in this increment
//!
//! Editing a `texture` is not yet a visible change — the mesher resolves a quad's
//! layer by parsing a block's *name* — so what an author can actually see change
//! under them is solidity and these three: whether a block can be broken at all,
//! what a break leaves behind, and whether a placement may build over it. Each is
//! driven through the client's own click, because what a rule is worth is what
//! the next edit a player makes does.
//!
//! # A placement overwrites only a block that is not solid, and that is by design
//!
//! The ray stops at the first cell the physics calls solid, and a placement lands
//! one step back along it — so the cell being built over is, necessarily, one the
//! ray passed through. `base:water` is the shipped block that shows it. The
//! replaceable scenario therefore hands over a stone that is neither solid nor
//! protected, which is what "a placement may overwrite a stone" has to mean for
//! a placement to be able to reach one at all.
//!
//! # A residue naming nothing is accepted, and that is the existing contract
//!
//! A `breaks_into` is resolved when a break happens and not when the declaration
//! is read, which is what lets two mods name each other's blocks without either
//! having to load first. A reload does not tighten it. What it costs is that the
//! failure arrives at the break rather than at the edit, and the last scenario is
//! where that cost is written down.

#[path = "support/input/mod.rs"]
mod input;
#[path = "support/reload.rs"]
mod reload;
#[path = "support/reload_world.rs"]
mod reload_world;
mod support;

use std::error::Error;
use std::path::Path;

use mc_core::id::BlockName;
use mc_sim::simulation::Simulation;
use winit::event::MouseButton;

use input::InputHarness;
use reload::{
    Adoption, DIRT, Declaration, GRASS, MITHRIL, STONE, STONE_FILE, accepted, adoption, candidate,
    restating, shipped,
};
use reload_world::{
    AIM_AT_THE_FAR_CELL, Edit, OVER_THE_FAR_CELL, THE_FAR_CELL, edit, floor_holding, floor_of,
    named_nothing_declared, playing, standing, wrote,
};
use support::content::ContentRoot;
use support::{TestResult, content_root};

#[test]
fn stone_declared_unbreakable_refuses_the_next_break_as_indestructible() -> TestResult {
    let mut client = a_client_on_a_stone_floor(&content_root()?)?;
    let root = restated(&Declaration::of(STONE).breakable(false))?;

    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    let broke = broken_by(&mut client);

    assert_eq!(
        broke,
        Edit::Indestructible,
        "an author who has just declared a block indestructible has said something about the very \
         next click, not about the next launch. A refusal by name is what tells the player the \
         rule refused them rather than that they missed"
    );
    Ok(())
}

#[test]
fn stone_given_a_residue_leaves_that_block_behind_when_it_is_broken() -> TestResult {
    let mut client = a_client_on_a_stone_floor(&content_root()?)?;
    let root = restated(&Declaration::of(STONE).breaking_into(DIRT))?;

    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    let broke = broken_by(&mut client);

    assert_eq!(
        broke,
        wrote(THE_FAR_CELL, STONE, DIRT),
        "breaking a block and what it leaves behind are two independent claims, and the second is \
         what the author has just written. A cell left empty is what a block declaring no residue \
         does, so an implementation still reading the content it was launched with produces \
         exactly that and looks like an ordinary break"
    );
    Ok(())
}

#[test]
fn stone_declared_replaceable_lets_the_next_placement_build_over_it() -> TestResult {
    let mut client = a_client_over_a_stone_in_the_way(&content_root()?)?;
    let root = restated(&Declaration::of(STONE).solid(false).replaceable(true))?;

    let answered = adoption(client.adopt(candidate(root.path())?));
    require_admitted(&answered)?;
    let built = placed_by(&mut client);

    assert_eq!(
        built,
        wrote(OVER_THE_FAR_CELL, STONE, DIRT),
        "replaceability is content's word about the block being built over and is read from \
         nothing else — not from whether it stops a player, which is a separate declaration this \
         candidate also changes because a placement can only ever reach a cell the ray passed \
         through. Under the content that was serving the same click builds in the empty cell in \
         front of the stone instead, which is a placement that landed somewhere else and not a \
         placement that overwrote anything"
    );
    Ok(())
}

#[test]
fn a_candidate_whose_residue_names_a_block_nothing_declares_is_accepted() -> TestResult {
    let mut client = a_client_on_a_stone_floor(&content_root()?)?;
    let root = restated(&Declaration::of(STONE).breaking_into(MITHRIL))?;

    let answered = adoption(client.adopt(candidate(root.path())?));
    let broke = broken_by(&mut client);

    assert_eq!(
        (answered, broke),
        (accepted(DIRT), named_nothing_declared(MITHRIL)),
        "a residue is resolved when a break happens and not when a declaration is read, which is \
         what lets two mods name each other's blocks without either having to load first. So this \
         candidate is taken up, and the price of that contract arrives where the contract says it \
         does: at the break, naming the block nobody declared. A reload that cross-referenced \
         residues would refuse this and break a documented promise it is not here to change"
    );
    Ok(())
}

/// A client standing on a floor of stone, with the root at `root` serving.
fn a_client_on_a_stone_floor(root: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, standing(), |registry| floor_of(registry, STONE))?;
    Ok(playing_client(simulation, holding))
}

/// A client standing on a floor of grass with one stone standing on it, in the
/// path the far aim takes on its way down.
fn a_client_over_a_stone_in_the_way(root: &Path) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, standing(), |registry| {
        floor_holding(registry, GRASS, &[(OVER_THE_FAR_CELL, STONE)])
    })?;
    Ok(playing_client(simulation, holding))
}

/// A started client already playing what it was handed.
fn playing_client(simulation: Simulation, holding: BlockName) -> InputHarness {
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    client
}

/// The shipped root with `stone` written over its stone declaration.
fn restated(stone: &Declaration) -> Result<ContentRoot, Box<dyn Error>> {
    restating(shipped()?, STONE_FILE, stone)
}

/// What a break aimed down the declared line does.
fn broken_by(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_AT_THE_FAR_CELL);
    client.click(MouseButton::Left);
    edit(client.edit())
}

/// What a placement aimed down that same line does.
fn placed_by(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_AT_THE_FAR_CELL);
    client.click(MouseButton::Right);
    edit(client.edit())
}

/// Refuses unless the candidate was admitted at all.
fn require_admitted(answered: &Adoption) -> Result<(), Box<dyn Error>> {
    if matches!(answered, Adoption::Accepted { .. }) {
        return Ok(());
    }
    Err(format!(
        "this scenario needs the candidate to be admitted, and the client answered {answered:?}. \
         The rule the next click is judged by would then be the rule it was already judged by"
    )
    .into())
}
