//! The replay's three declared captures, shot from the player's own published
//! camera, against their committed goldens.
//!
//! **Ordering is binding and it is not a preference.** These goldens are minted
//! from this renderer, so a golden shot before the derived probes pass is a
//! photograph of whatever the renderer happened to do that day, and it then
//! passes forever. `terrain_probes.rs` is what makes shooting them safe, and
//! `replay_oracle.rs` is what makes shooting them from *this* camera safe —
//! nothing here can substitute for either.
//!
//! **The camera is reached by advancing, not by asking.** The orbit these
//! captures used to be shot through was a pure function of the tick index, so a
//! frame could be taken at tick 59 without the fifty-eight before it ever having
//! happened. An integrated player has no such property, and the reproducibility
//! these goldens now rest on is the one the spec names in its place: the same
//! declared spawn under the same declared script produces the same state at
//! every tick of every run.
//!
//! Each scenario runs with `MYCRAFT_UPDATE_GOLDENS` **unset**. With it set the
//! golden is minted and matched in the same run and the scenario asserts
//! nothing — the spec's own wording — so the opt-in is *read* through
//! `OptIns::from_environment` rather than assumed either way. No test in this
//! project sets an environment variable; `set_var` is `unsafe` in edition 2024.
//!
//! **This binary is the mint target, and holds only judgements that are safe to
//! mint.** Every capture here is judged against *its own* tick's golden, so
//! running it under the update opt-in writes each frame to the reference it
//! belongs to. The one test that deliberately judges one tick against another's
//! golden lives in `golden_mismatch.rs`, alone, because minting through it would
//! write a tick-59 frame as tick 0's ground truth. That separation is what lets
//! `docs/technical/rendering.md` name a binary rather than a test function in
//! the re-shoot procedure — a file name a refactor cannot move silently.
//!
//! # Where the goldens live, and where this file lives
//!
//! The golden root stays `crates/mc-render/goldens/`, as `spec.md`'s binding
//! table requires. This file cannot: it renders the replay, which needs the
//! world `mc-sim` generates and the draw path `mc-render` owns, and neither of
//! those crates may resolve the other in any dependency kind. The composition
//! root is the only crate that resolves both.

mod support;

use mc_testkit::frame::{GoldenOutcome, OptIns};

use support::TestResult;
use support::goldens::{DECLARED_TICKS, verified};

#[test]
fn every_declared_capture_matches_the_golden_committed_for_it() -> TestResult {
    let mut unmatched = Vec::new();
    for tick in DECLARED_TICKS {
        let Some(outcome) = verified(tick)? else {
            return Ok(());
        };
        unmatched.extend(reported(tick, &outcome));
    }

    assert!(
        unmatched.is_empty(),
        "the world is a pure function of its seed and the player's camera is a pure function \
         of the declared spawn and the declared intent script, so each of these ticks draws \
         the same picture on every run and the committed golden is what says which picture \
         that is. Three ticks and not one because they are three different things the script \
         puts in front of the camera — the spawn still in the air, the end of the straight \
         walk, and the tick after the turn and the jump. Unmatched: {}",
        unmatched.join("; ")
    );
    Ok(())
}

/// How `tick`'s verdict reads to a person, or nothing at all when it matched.
///
/// Never `{outcome:?}`. A mismatch carries the per-pixel failing mask, and
/// debug-printing one buries the sentence a reader needs under eleven megabytes
/// of booleans — measured, on the first run of this test. `GoldenFailure`'s own
/// `Display` says which golden, how many pixels stood past the tolerance, the
/// worst distance, and where it wrote the evidence.
fn reported(tick: u16, outcome: &GoldenOutcome) -> Option<String> {
    if matched(outcome) {
        return None;
    }
    Some(match outcome {
        GoldenOutcome::Failed(failure) => format!("tick {tick}: {failure}"),
        _ => format!(
            "tick {tick}: the golden was minted rather than matched, with the update opt-in \
             unset: {outcome:?}"
        ),
    })
}

/// Whether `outcome` is the verdict a run with `MYCRAFT_UPDATE_GOLDENS` unset
/// has to reach.
///
/// With the opt-in set, the golden is minted and matched in the same run and
/// the scenario asserts nothing — which is `spec.md`'s own wording for the
/// golden scenario, not a loophole invented here.
fn matched(outcome: &GoldenOutcome) -> bool {
    match outcome {
        GoldenOutcome::Pass => true,
        GoldenOutcome::GoldenWritten { .. } | GoldenOutcome::GoldenUnchanged => {
            OptIns::from_environment().update_goldens
        }
        _ => false,
    }
}
