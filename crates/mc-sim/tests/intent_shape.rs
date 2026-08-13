//! The intents a client submits have no field a position could travel in.
//!
//! Invariant 4 in structural form. The simulation is authoritative because it
//! derives the player's next state, and the cell an edit lands in, from its own
//! state — and the cheapest way to lose that is not a bug in the physics or in
//! the raycast. It is a field on an intent that a client fills in and the server
//! believes. A type with no such field cannot be trusted wrongly, so this is
//! asserted about the *declarations* rather than about any code path that reads
//! them.
//!
//! Two declarations are scanned: the movement a client asks for each tick, and
//! the action it asks for when it clicks. Naming *what* you wish to place is an
//! intent; naming *where* it goes is not, and neither type has anywhere to put
//! the second.
//!
//! **An absence assertion goes green forever the day the thing it guards is
//! quietly removed**, and this one has three ways to do that: a scan that finds
//! no declaration reports no fields, a scan that read the whole file rather than
//! the one type would report the simulation's own state, and — for an enum whose
//! variants carry braces of their own — a scan that stopped at the first `}` it
//! met would read only the first variant and report nothing about the rest. So
//! the same functions are pointed at fixtures declaring the forbidden shapes,
//! and exactly the expected fields have to come back
//! (`standards/global/testing.md` §2).
//!
//! The fixtures are real files under `tests/fixtures/` rather than temporary
//! directories written at run time, unlike the sibling scan in
//! `crates/mc-client/tests/winit_boundary.rs`. Two reasons: this crate has no
//! dev-dependencies at all today, and a control worth having is a control
//! someone can read — the forbidden shape sits in the diff as source, next to
//! the scan that has to catch it, rather than inside a string literal.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The two Rust keywords a scanned declaration can begin with.
const STRUCT: &str = "struct";
const ENUM: &str = "enum";

/// The type a client fills in to move, and the one it fills in to edit.
const MOVEMENT_TYPE: &str = "MovementIntent";
const ACTION_TYPE: &str = "ActionIntent";

/// Every field the movement intent may declare, and no other, in sorted order.
///
/// A magnitude to walk forward and sideways by, the two changes of view, and
/// whether a jump is wanted. Each is a request; none is an answer.
const MOVEMENT_FIELDS: [&str; 5] = ["forward", "jump", "pitch_delta", "strafe", "yaw_delta"];

/// Every field the action intent may declare, across all of its variants.
///
/// One, and it is a **name**: which block the client wishes to place. Breaking
/// carries nothing at all, which is why the type is an enum — "a break request
/// carrying a block name" is unrepresentable rather than merely unused.
const ACTION_FIELDS: [&str; 1] = ["block"];

/// Words that make a field name a place, a cell or a rate of travel, matched
/// anywhere in the name.
const POSITION_OR_VELOCITY: [&str; 9] = [
    "position",
    "location",
    "coordinate",
    "cell",
    "origin",
    "translation",
    "velocity",
    "speed",
    "momentum",
];

/// Names of an absolute orientation, matched whole.
///
/// Whole rather than anywhere, because a *change* of view is spelled with a
/// `_delta` suffix and is the one thing an intent is allowed to say about where
/// the player looks. `yaw` is an orientation a client stated; `yaw_delta` is a
/// mouse movement it is reporting.
const ABSOLUTE_ORIENTATION: [&str; 8] = [
    "yaw",
    "pitch",
    "roll",
    "orientation",
    "rotation",
    "facing",
    "heading",
    "direction",
];

/// What a scan of one declared type found.
#[derive(Debug)]
struct IntentShape {
    /// Every field the type declares, sorted.
    fields: Vec<String>,
    /// Those of them that name a position, a cell, a velocity or an absolute
    /// orientation.
    positional: Vec<String>,
}

/// The text between the braces of `{keyword} {type_name} { … }`.
///
/// The opening brace is part of what is searched for, so a longer name starting
/// with the same letters is not a match.
///
/// **Brace depth is counted rather than stopping at the first `}`**, and that is
/// not tidiness. An enum variant carrying fields opens a brace of its own, so a
/// scan that stopped at the first closing one would read the first variant and
/// nothing after it — reporting a clean absence for every field declared later,
/// which is precisely the shape a client-supplied target would be added in.
fn declared_body<'a>(source: &'a str, keyword: &str, type_name: &str) -> Option<&'a str> {
    let opening = format!("{keyword} {type_name} {{");
    let after = source.split_once(&opening)?.1;
    let mut depth = 1_usize;
    for (offset, character) in after.char_indices() {
        depth = match character {
            '{' => depth.saturating_add(1),
            '}' => depth.saturating_sub(1),
            _ => depth,
        };
        if depth == 0 {
            return after.get(..offset);
        }
    }
    None
}

/// Every field a declaration's body declares, including those inside variants.
///
/// A variant's fields sit on the same line as its name and its braces, so each
/// line is cut at every brace and comma before a field is looked for in it.
/// Comment and attribute lines are dropped whole first, so that cutting one at a
/// comma cannot leave a fragment that no longer looks like a comment.
fn declared_fields(body: &str) -> Vec<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//") && !line.starts_with("#["))
        .flat_map(|line| line.split(['{', '}', ',']))
        .filter_map(field_name)
        .collect()
}

/// The field a fragment of a declaration declares, if it declares one.
fn field_name(fragment: &str) -> Option<String> {
    let declaration = fragment.trim();
    let (named, _) = declaration.split_once(':')?;
    Some(named.trim().trim_start_matches("pub ").trim().to_owned())
}

/// Whether a field's name says where the player is, which cell it means, how
/// fast it is going, or which way it faces.
fn names_a_position(field: &str) -> bool {
    POSITION_OR_VELOCITY
        .iter()
        .any(|forbidden| field.contains(forbidden))
        || ABSOLUTE_ORIENTATION.contains(&field)
}

/// Reads the type `type_name` out of a Rust source and reports the fields it
/// declares, and which of them name something a client may not state.
fn scan_intent(
    source: &Path,
    keyword: &str,
    type_name: &str,
) -> Result<IntentShape, Box<dyn Error>> {
    let text = fs::read_to_string(source)?;
    let body = declared_body(&text, keyword, type_name)
        .ok_or_else(|| format!("{} declares no `{type_name}`", source.display()))?;
    let mut fields = declared_fields(body);
    fields.sort();
    let positional = fields
        .iter()
        .filter(|field| names_a_position(field))
        .cloned()
        .collect();
    Ok(IntentShape { fields, positional })
}

/// The module the player's vocabulary is declared in.
fn crate_source(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Field names as string slices, which is the form the expectations are written
/// in.
fn named(fields: &[String]) -> Vec<&str> {
    fields.iter().map(String::as_str).collect()
}

#[test]
fn the_movement_intent_declares_five_fields_and_names_no_position() -> TestResult {
    let shape = scan_intent(&crate_source("src/player/mod.rs"), STRUCT, MOVEMENT_TYPE)?;

    assert_eq!(
        (named(&shape.fields), named(&shape.positional)),
        (MOVEMENT_FIELDS.to_vec(), Vec::new()),
        "the intent is a request and never an answer, so it declares exactly {MOVEMENT_FIELDS:?} \
         and nothing through which a client could state where it is, how fast it is going or \
         which way it is facing"
    );
    Ok(())
}

/// The control, in both directions at once.
///
/// The fixture's intent declares a position among five innocent fields, so a
/// scan that had stopped reporting anything is caught; the fixture's
/// `PlayerState` declares one too, legitimately, so a scan that read the file
/// instead of the type — and would therefore report the real module for its own
/// player state — is caught with it. Exactly one field comes back, and the
/// count says which type was read.
#[test]
fn the_same_scan_reports_a_fixture_intent_that_declares_a_position() -> TestResult {
    let fixture = crate_source("tests/fixtures/intent_shape/positioned_intent.rs");

    let shape = scan_intent(&fixture, STRUCT, MOVEMENT_TYPE)?;

    assert_eq!(
        (shape.fields.len(), named(&shape.positional)),
        (6, vec!["position"]),
        "the fixture's intent declares six fields, one of them a position, and the state beside \
         it declares four more the scan must not reach: {shape:?}"
    );
    Ok(())
}

#[test]
fn the_action_request_declares_one_field_and_it_names_a_block_rather_than_a_place() -> TestResult {
    let shape = scan_intent(&crate_source("src/world/action/mod.rs"), ENUM, ACTION_TYPE)?;

    assert_eq!(
        (named(&shape.fields), named(&shape.positional)),
        (ACTION_FIELDS.to_vec(), Vec::new()),
        "an action request says what a client wants done and never where, so across every variant \
         it declares exactly {ACTION_FIELDS:?} — a block to place, which is a name and not a \
         place — and no coordinate, no cell and no absolute orientation the server could be made \
         to believe"
    );
    Ok(())
}

/// The control for the enum scan, and it fails in three directions at once.
///
/// The fixture's offending field sits in the **last** variant, behind one that
/// carries braces of its own, so a scan that stopped at the first `}` reads only
/// `Place` and reports a clean absence — the exact failure the widening has to
/// avoid, and one that would otherwise look identical to a healthy pass. Its
/// `block` field is correctly not flagged, so the control also says the scan
/// discriminates rather than objecting to any variant with data. And the
/// `EditReport` beside it names a cell the scan must not reach, because the
/// server's own answer legitimately does. Two fields come back, and the count
/// says which declaration was read.
#[test]
fn the_same_scan_reports_a_fixture_action_whose_later_variant_declares_a_target_cell() -> TestResult
{
    let fixture = crate_source("tests/fixtures/intent_shape/positioned_action.rs");

    let shape = scan_intent(&fixture, ENUM, ACTION_TYPE)?;

    assert_eq!(
        (shape.fields.len(), named(&shape.positional)),
        (2, vec!["target_cell"]),
        "the fixture's action declares two fields across three variants — an innocent block name \
         in the second and a cell in the third — and the report beside it declares three more the \
         scan must not reach: {shape:?}"
    );
    Ok(())
}
