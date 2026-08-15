//! Which of a part's states is assembled, and what a document whose states and
//! layers disagree is told.
//!
//! A flicker is different voxels rather than a transform, which is the whole
//! reason states exist — so the fixture's two states are graded on the voxels
//! they produce and not on a name that reached the assembler. `low` fills one
//! cell and `high` fills two, at positions that share no axis, so a default
//! that silently took the *last* declared state, or took none at all, answers
//! with a different list rather than the same list under another name.
//!
//! The handle is solid and stateless on purpose: it anchors the assembled
//! volume, so the flame's art changes the answer without the normalisation
//! moving underneath it.

mod common;

use common::{
    TestResult, all_named, assembled, assembly_refusal, at, positions_of, refusal, solid_y_layers,
    unnamed,
};
use voxforge::format::{PartName, StateName};
use voxforge::volume::StateSelection;

/// The material the flame is painted with.
const FLAME: &str = "base:flame";

/// The states the flame declares, `low` first and so its default.
const TWO_STATES: &str = "states = [\"low\", \"high\"]\n";

/// A part with no layer of its own, hanging off one that has some.
const PART_WITHOUT_LAYERS: &str = r#"schema = 1
name = "base:probe"
scale = 16
slice = "y"

[palette]
"w" = "base:oak_plank"

[[parts]]
name = "handle"
size = [1, 1, 1]
origin = [0, 0, 0]

[[parts]]
name = "stem"
size = [1, 1, 1]
origin = [0, 0, 0]
attach = { to = "handle", at = [0, 1, 0] }

[[layers]]
part = "handle"
y = 0
grid = """
w
"""
"#;

/// A torch whose flame declares `states` and carries `flame_layers`.
fn torch(states: &str, flame_layers: &str) -> String {
    format!(
        r#"schema = 1
name = "base:torch"
scale = 16
slice = "y"

[palette]
"." = "empty"
"f" = "base:flame"
"w" = "base:oak_plank"

[[parts]]
name = "handle"
size = [2, 3, 2]
origin = [0, 0, 0]

[[parts]]
name = "flame"
size = [2, 2, 2]
origin = [0, 0, 0]
attach = {{ to = "handle", at = [0, 3, 0] }}
{states}{handle}{flame_layers}"#,
        handle = solid_y_layers("handle", (2, 3, 2), 'w'),
    )
}

/// One layer of the flame, belonging to `state` unless that is empty.
fn flame_layer(state: &str, plane: u32, rows: &[&str]) -> String {
    let grid = rows.join("\n");
    let belongs = if state.is_empty() {
        String::new()
    } else {
        format!("state = \"{state}\"\n")
    };
    format!("\n[[layers]]\npart = \"flame\"\n{belongs}y = {plane}\ngrid = \"\"\"\n{grid}\n\"\"\"\n")
}

/// The flame's `low` art: one cell, at its own `(0, 0, 0)`.
fn low_layers() -> String {
    flame_layer("low", 0, &["f.", ".."])
}

/// The flame's `high` art: that same cell and a second on the layer above,
/// diagonally across it so no reflection of the pair maps onto the `low` one.
fn high_layers() -> String {
    format!(
        "{}{}",
        flame_layer("high", 0, &["f.", ".."]),
        flame_layer("high", 1, &["..", ".f"])
    )
}

/// Both declared states, fully arted, plus whatever `extra` adds.
fn both_states(extra: &str) -> String {
    format!("{}{}{extra}", low_layers(), high_layers())
}

#[test]
fn a_part_with_states_assembled_without_a_selection_uses_its_first_declared_state() -> TestResult {
    // The handle occupies y = 0..2 and the flame hangs at y = 3, so nothing
    // reaches below the pre-normalisation origin and normalisation moves
    // nothing. `low` fills the flame's own (0, 0, 0), which is (0, 3, 0).
    let volume = assembled(
        &torch(TWO_STATES, &both_states("")),
        &StateSelection::default(),
    )?;

    assert_eq!(
        positions_of(&volume, FLAME)?,
        vec![at(0, 3, 0)],
        "the first declared state is the default, and `high` would have filled two cells rather than one"
    );
    Ok(())
}

#[test]
fn a_part_assembled_with_a_selected_state_uses_that_states_layers() -> TestResult {
    // `high` adds the flame's local (1, 1, 1), which the attachment puts at
    // (1, 4, 1).
    let selection = StateSelection::default().with(PartName::new("flame"), StateName::new("high"));
    let volume = assembled(&torch(TWO_STATES, &both_states("")), &selection)?;

    assert_eq!(
        positions_of(&volume, FLAME)?,
        vec![at(0, 3, 0), at(1, 4, 1)],
        "naming a state replaces that part's art rather than adding to it or being ignored"
    );
    Ok(())
}

#[test]
fn a_layer_naming_a_state_its_part_did_not_declare_is_refused_naming_both() -> TestResult {
    let stray = flame_layer("middle", 1, &["f.", ".."]);
    let fault = refusal(&torch(TWO_STATES, &both_states(&stray)))?;

    assert_eq!(
        (
            fault.part.as_deref(),
            unnamed(&fault, &["`middle`", "`low`", "`high`"]),
        ),
        (Some("flame"), all_named()),
        "art belonging to a state nobody declared is art that would never be drawn, and the declared states are what the repair needs; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_declared_state_no_layer_belongs_to_is_refused_naming_the_state_and_the_part() -> TestResult {
    let fault = refusal(&torch(TWO_STATES, &low_layers()))?;

    assert_eq!(
        (
            fault.part.as_deref(),
            unnamed(&fault, &["`high`", "no layer"]),
        ),
        (Some("flame"), all_named()),
        "a state with no art assembles to nothing at all, which is a missing layer rather than a legitimately empty one; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_layer_of_a_stateful_part_omitting_its_state_is_refused_naming_the_declared_ones() -> TestResult
{
    let stateless = flame_layer("", 1, &["f.", ".."]);
    let fault = refusal(&torch(TWO_STATES, &both_states(&stateless)))?;

    assert_eq!(
        (
            fault.part.as_deref(),
            unnamed(&fault, &["`low`", "`high`", "state"]),
        ),
        (Some("flame"), all_named()),
        "a layer that names no state under a part that has some belongs to all of them or none, and neither is a thing the format can mean; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn requesting_a_state_the_part_never_declared_is_refused_naming_the_declared_ones() -> TestResult {
    let selection =
        StateSelection::default().with(PartName::new("flame"), StateName::new("blazing"));
    let fault = assembly_refusal(&torch(TWO_STATES, &both_states("")), &selection)?;

    assert_eq!(
        unnamed(&fault, &["`blazing`", "`low`", "`high`"]),
        all_named(),
        "a request naming a state that does not exist is refused rather than quietly served the default; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_stateless_part_declaring_no_layer_at_all_is_refused_naming_that_part() -> TestResult {
    let fault = refusal(PART_WITHOUT_LAYERS)?;

    assert_eq!(
        (
            fault.part.as_deref(),
            unnamed(&fault, &["`stem`", "no layer"]),
        ),
        (Some("stem"), all_named()),
        "a part contributing nothing is a part whose layers were never written, and silently assembling around it hides the omission; cause was: {}",
        fault.cause
    );
    Ok(())
}
