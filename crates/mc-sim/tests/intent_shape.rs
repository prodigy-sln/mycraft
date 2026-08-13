//! The intent a client submits has no field a position could travel in.
//!
//! Invariant 4 in structural form. The simulation is authoritative because it
//! derives the player's next state from its own previous one, and the cheapest
//! way to lose that is not a bug in the physics — it is a field on the intent
//! that a client fills in and the server believes. A type with no such field
//! cannot be trusted wrongly, so this is asserted about the *declaration* rather
//! than about any code path that reads it.
//!
//! **An absence assertion goes green forever the day the thing it guards is
//! quietly removed**, and this one has two ways to do that: a scan that finds no
//! struct reports no fields, and a scan that read the whole file rather than the
//! one type would report the simulation's own `PlayerState`, whose position is
//! entirely legitimate. So the same function is pointed at a fixture declaring
//! both shapes at once — an intent that does carry a position, beside a state
//! that may — and exactly one of them has to come back
//! (`standards/global/testing.md` §2).
//!
//! The fixture is a real file under `tests/fixtures/` rather than a temporary
//! directory written at run time, unlike the sibling scan in
//! `crates/mc-client/tests/winit_boundary.rs`. Two reasons: this crate has no
//! dev-dependencies at all today, and a control worth having is a control
//! someone can read — the forbidden shape sits in the diff as source, next to
//! the scan that has to catch it, rather than inside a string literal.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

/// The type a client fills in and the simulation is handed.
const INTENT_TYPE: &str = "MovementIntent";

/// Every field the intent may declare, and no other, in sorted order.
///
/// A magnitude to walk forward and sideways by, the two changes of view, and
/// whether a jump is wanted. Each is a request; none is an answer.
const DECLARED_FIELDS: [&str; 5] = ["forward", "jump", "pitch_delta", "strafe", "yaw_delta"];

/// Words that make a field name a place or a rate of travel, matched anywhere in
/// the name.
const POSITION_OR_VELOCITY: [&str; 8] = [
    "position",
    "location",
    "coordinate",
    "origin",
    "translation",
    "velocity",
    "speed",
    "momentum",
];

/// Names of an absolute orientation, matched whole.
///
/// Whole rather than anywhere, because a *change* of view is spelled with a
/// `_delta` suffix and is the one thing the intent is allowed to say about where
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
    /// Those of them that name a position, a velocity or an absolute
    /// orientation.
    positional: Vec<String>,
}

/// The text between the braces of `struct {type_name} { ... }`.
///
/// The opening brace is part of what is searched for, so a longer name starting
/// with the same letters is not a match.
fn struct_body<'a>(source: &'a str, type_name: &str) -> Option<&'a str> {
    let opening = format!("struct {type_name} {{");
    let after = source.split_once(&opening)?.1;
    Some(after.split_once('}')?.0)
}

/// The field a line of a struct body declares, if it declares one.
fn field_name(line: &str) -> Option<String> {
    let declaration = line.trim();
    if declaration.starts_with("//") || declaration.starts_with("#[") {
        return None;
    }
    let (named, _) = declaration.split_once(':')?;
    Some(named.trim().trim_start_matches("pub ").trim().to_owned())
}

/// Whether a field's name says where the player is, how fast, or which way it
/// faces.
fn names_a_position(field: &str) -> bool {
    POSITION_OR_VELOCITY
        .iter()
        .any(|forbidden| field.contains(forbidden))
        || ABSOLUTE_ORIENTATION.contains(&field)
}

/// Reads the type `type_name` out of a Rust source and reports the fields it
/// declares, and which of them name something a client may not state.
fn scan_intent(source: &Path, type_name: &str) -> Result<IntentShape, Box<dyn Error>> {
    let text = fs::read_to_string(source)?;
    let body = struct_body(&text, type_name)
        .ok_or_else(|| format!("{} declares no `{type_name}`", source.display()))?;
    let mut fields: Vec<String> = body.lines().filter_map(field_name).collect();
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
    let shape = scan_intent(&crate_source("src/player/mod.rs"), INTENT_TYPE)?;

    assert_eq!(
        (named(&shape.fields), named(&shape.positional)),
        (DECLARED_FIELDS.to_vec(), Vec::new()),
        "the intent is a request and never an answer, so it declares exactly {DECLARED_FIELDS:?} \
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

    let shape = scan_intent(&fixture, INTENT_TYPE)?;

    assert_eq!(
        (shape.fields.len(), named(&shape.positional)),
        (6, vec!["position"]),
        "the fixture's intent declares six fields, one of them a position, and the state beside \
         it declares four more the scan must not reach: {shape:?}"
    );
    Ok(())
}
