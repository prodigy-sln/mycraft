//! Classifying a refusal that came from the backend rather than from a limit.
//!
//! # Why this is decided from the error's identity and not from its text
//!
//! The host distinguishes its two ordinary outcomes structurally: a callback
//! that raised returns through the protected call, and an entry the host stopped
//! cannot return at all. The third state — the call fails while the guard is
//! still clear — is the backend refusing on its own account, and the only thing
//! left to read is the error itself.
//!
//! Reading its *message* would be the obvious move and is the wrong one twice
//! over. Measured, an allocation refusal arrives carrying **no message at all**,
//! so text matching cannot see the one case it most needs to; and any other
//! error whose text happens to mention memory would be classified as an
//! allocation, which is how a mod that raises a well-chosen string gets its
//! failure filed under the host's own condition. Both directions are asserted
//! below, because a mapping that is right for the wrong reason stays right only
//! until the next release rewords itself.
//!
//! # Two things this deliberately does not cover
//!
//! An error the backend wraps around a host callback's own failure never reaches
//! this arm: the host's protected call catches it, so it arrives as an ordinary
//! raised value and is classified by shape rather than here.
//!
//! And this calls the same function the adapter calls, which is agreement
//! between two copies of one decision — the adapter can stop calling it entirely
//! and this stays green. `tests/backend_errors.rs` carries the end-to-end half
//! for the script-error side. The allocation side has no in-process construction
//! until the allocator backstop exists, at which point a single allocation too
//! large for the interrupt to see fails here with the guard clear. Until then,
//! what would go red if the adapter stopped consulting this is: nothing.

use mlua::Error;

use super::classify_backend_error;
use crate::fault::FaultKind;

/// What the host makes of each refusal, keeping each one's description beside
/// its verdict so a failure names the case rather than an index.
fn classified(refusals: Vec<(&'static str, Error)>) -> Vec<(&'static str, FaultKind)> {
    refusals
        .into_iter()
        .map(|(described, error)| (described, classify_backend_error(&error)))
        .collect()
}

#[test]
fn an_allocation_refusal_is_classified_as_one_however_little_it_says() {
    let observed = classified(vec![
        (
            "out of memory",
            Error::MemoryError("not enough memory".to_owned()),
        ),
        (
            "out of memory, carrying the empty message an abort arrives with",
            Error::MemoryError(String::new()),
        ),
    ]);

    assert_eq!(
        observed,
        vec![
            ("out of memory", FaultKind::Allocation),
            (
                "out of memory, carrying the empty message an abort arrives with",
                FaultKind::Allocation
            ),
        ],
        "the second case is the one that decides this. An allocation refusal was measured to \
         arrive with no message at all, so a host reading the text sees nothing, calls it a \
         script error, and files a condition of the whole state against whichever mod happened \
         to be running when memory ran out."
    );
}

#[test]
fn every_other_refusal_is_a_script_error_however_much_its_text_sounds_like_memory() {
    let observed = classified(vec![
        (
            "a runtime error whose text happens to mention memory",
            Error::RuntimeError("not enough memory to smelt that".to_owned()),
        ),
        (
            "source that would not parse",
            Error::SyntaxError {
                message: "unexpected symbol".to_owned(),
                incomplete_input: false,
            },
        ),
        ("the call stack ran out", Error::StackError),
    ]);

    assert_eq!(
        observed,
        vec![
            (
                "a runtime error whose text happens to mention memory",
                FaultKind::ScriptError
            ),
            ("source that would not parse", FaultKind::ScriptError),
            ("the call stack ran out", FaultKind::ScriptError),
        ],
        "the first case is the misattribution pointing the other way: a mod author who writes \
         `not enough memory` into an error of their own has it reported as the host running out \
         of memory, which is a fault kind quarantine and the pressure rule both treat \
         differently. Neither direction is visible to a test that only feeds in errors whose \
         text agrees with their kind."
    );
}
