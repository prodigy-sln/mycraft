//! The fault a mod author reads, and the kind an operator's tooling compares.
//!
//! The identity newtypes and the limit record are vocabulary: they are stored
//! and compared and never parsed, so round-tripping a string through one proves
//! nothing about anything. What is worth pinning here is the *rendering* — the
//! one artefact of this crate a human has to act on — and the *comparability* of
//! the fault kind, which is what lets a harness assert an expected sequence of
//! outcomes by equality rather than by picking discriminants apart.
//!
//! # The rendering this file binds
//!
//! ```text
//! <origin>[, subject `S`][, component `C`][, line N][, refused `S`/`C`]: <kind>: <cause>
//! ```
//!
//! `<origin>` is one of four shapes, because both of its fields are optional and
//! all four are producible:
//!
//! | chunk | round | rendered |
//! |---|---|---|
//! | `Some` | `Some` | ``chunk `NAME`, round N`` |
//! | `Some` | `None` | ``chunk `NAME`` |
//! | `None` | `Some` | `round N` |
//! | `None` | `None` | `unattributed` |
//!
//! `<kind>` is spelled by the fault's own formatter rather than by a `Display`
//! on the kind itself, so the public surface stays exactly what it declares:
//!
//! | kind | rendered |
//! |---|---|
//! | `BudgetExhausted` | `call and loop budget exhausted` |
//! | `Allocation` | `allocation refused` |
//! | `ScriptError` | `script error` |
//! | `Compilation` | `compilation failed` |
//! | `CascadeRefused` | `cascade refused` |
//! | `CascadeDeferred` | `cascade deferred` |
//! | `HostMemoryPressure` | `host memory pressure` |
//!
//! Whole strings are asserted rather than substrings. A fault that lost its
//! chunk, gained a subject it has no business naming, or dropped the line it
//! parsed is a different string, and a `contains` check sees none of those.
//!
//! `cause` is unbounded script-controlled text and is spliced verbatim: raw is
//! correct here, and it is also why every consumer downstream inherits whatever
//! a mod put there at whatever length a mod chose.

use mc_script::{
    Attachment, ChunkName, ComponentName, FaultKind, HostLimits, RoundIndex, ScriptFault,
    ScriptOrigin, SubjectName,
};

/// A fault with nothing optional set, which each test below then shapes.
///
/// The four `None`s are the interesting part: a test that has to spell them
/// makes it obvious which fields it is deliberately leaving empty.
fn fault(origin: ScriptOrigin, kind: FaultKind, cause: &str) -> ScriptFault {
    ScriptFault {
        origin,
        subject: None,
        component: None,
        kind,
        line: None,
        refused_target: None,
        cause: cause.to_owned(),
    }
}

fn chunk(name: &str) -> ScriptOrigin {
    ScriptOrigin {
        chunk: Some(ChunkName::new(name)),
        round: None,
    }
}

#[test]
fn a_chunk_level_fault_names_its_chunk_and_claims_no_attachment() {
    let aborted = fault(
        chunk("runaway.luau"),
        FaultKind::BudgetExhausted,
        "the chunk did not return",
    );

    assert_eq!(
        aborted.to_string(),
        "chunk `runaway.luau`: call and loop budget exhausted: the chunk did not return",
        "a chunk runs before any attachment exists, so a fault from one has no subject and no \
         component to name — and a rendering that invented either would send a mod author \
         looking for a callback that was never invoked"
    );
}

#[test]
fn a_compilation_fault_names_the_line_it_parsed_out_of_the_backend_message() {
    let mut refused = fault(
        chunk("furnace.luau"),
        FaultKind::Compilation,
        "Expected identifier when parsing expression, got 'end'",
    );
    refused.line = Some(3);

    assert_eq!(
        refused.to_string(),
        "chunk `furnace.luau`, line 3: compilation failed: Expected identifier when parsing \
         expression, got 'end'",
        "the line is a typed field precisely so it can be rendered on its own terms; a mod \
         author who cannot see which line failed has an error and no way to locate it"
    );
}

#[test]
fn an_invocation_fault_names_the_defining_chunk_the_round_and_both_halves_of_its_attachment() {
    let mut raised = fault(
        ScriptOrigin {
            chunk: Some(ChunkName::new("furnace.luau")),
            round: Some(RoundIndex::new(2)),
        },
        FaultKind::ScriptError,
        "attempt to index a nil value",
    );
    raised.subject = Some(SubjectName::new("base:furnace"));
    raised.component = Some(ComponentName::new("base:on_tick"));

    assert_eq!(
        raised.to_string(),
        "chunk `furnace.luau`, round 2, subject `base:furnace`, component `base:on_tick`: \
         script error: attempt to index a nil value",
        "this is the most common fault in the system and the one an author reads first: it has \
         to name the file the callback was defined in, the round it ran in, and the attachment \
         it belongs to"
    );
}

#[test]
fn a_refused_cascade_names_the_target_it_would_not_admit_as_well_as_the_requester() {
    let mut declined = fault(
        ScriptOrigin {
            chunk: Some(ChunkName::new("chain.luau")),
            round: Some(RoundIndex::new(4)),
        },
        FaultKind::CascadeRefused,
        "the pending queue is full",
    );
    declined.subject = Some(SubjectName::new("base:source"));
    declined.component = Some(ComponentName::new("base:on_tick"));
    declined.refused_target = Some(Attachment {
        subject: SubjectName::new("base:target"),
        component: ComponentName::new("base:on_tick"),
    });

    assert_eq!(
        declined.to_string(),
        "chunk `chain.luau`, round 4, subject `base:source`, component `base:on_tick`, refused \
         `base:target`/`base:on_tick`: cascade refused: the pending queue is full",
        "a refusal names two attachments — the one that asked and the one that was turned away — \
         and collapsing them into one leaves an operator unable to tell which mod to look at"
    );
}

#[test]
fn a_fault_the_host_raised_on_its_own_behalf_names_a_round_and_no_script() {
    let pressure = fault(
        ScriptOrigin {
            chunk: None,
            round: Some(RoundIndex::new(7)),
        },
        FaultKind::HostMemoryPressure,
        "script memory would reach the host backstop before this invocation's own cap",
    );

    assert_eq!(
        pressure.to_string(),
        "round 7: host memory pressure: script memory would reach the host backstop before this \
         invocation's own cap",
        "this invocation could have failed for a reason that is not its own, so naming a subject \
         or a component would file the blame against an author who did nothing wrong"
    );
}

#[test]
fn a_fault_that_can_attribute_itself_to_nothing_says_so_rather_than_rendering_a_gap() {
    let unplaced = fault(
        ScriptOrigin {
            chunk: None,
            round: None,
        },
        FaultKind::ScriptError,
        "the backend reported no location",
    );

    assert_eq!(
        unplaced.to_string(),
        "unattributed: script error: the backend reported no location",
        "both origin fields are public and optional, so this shape is constructible and its \
         rendering has to be decided rather than left to whatever a format string does with two \
         empty options"
    );
}

#[test]
fn every_fault_kind_is_distinguishable_from_every_other_by_equality() {
    let kinds = [
        FaultKind::BudgetExhausted,
        FaultKind::Allocation,
        FaultKind::ScriptError,
        FaultKind::Compilation,
        FaultKind::CascadeRefused,
        FaultKind::CascadeDeferred,
        FaultKind::HostMemoryPressure,
    ];

    let mut disagreements: Vec<String> = Vec::new();
    for (left_index, left) in kinds.iter().enumerate() {
        disagreements.extend(
            kinds
                .iter()
                .enumerate()
                .filter(|(right_index, right)| (left == *right) != (left_index == *right_index))
                .map(|(_, right)| format!("`{left:?}` against `{right:?}`")),
        );
    }

    assert!(
        disagreements.is_empty(),
        "a harness compares an expected sequence of outcomes against an observed one by \
         equality, which only says anything if every kind is equal to itself and to nothing \
         else. These pairs answered the wrong way: {disagreements:?}"
    );
}

#[test]
fn the_default_memory_backstop_leaves_room_above_the_enforced_cap() {
    let limits = HostLimits::default();

    assert!(
        limits.memory_backstop.get() > limits.memory_cap.get(),
        "the backstop is the absolute ceiling the whole VM may reach and the cap is what one \
         invocation may add above its entry baseline. A backstop at or below the cap puts the \
         host into permanent memory pressure from its first invocation, with every fault \
         attributed to the host rather than to whatever caused it: backstop {}, cap {}",
        limits.memory_backstop,
        limits.memory_cap
    );
}
