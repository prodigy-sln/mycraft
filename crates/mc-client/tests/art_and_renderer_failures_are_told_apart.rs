//! A change to the art's sources and a change to the renderer, failing
//! differently.
//!
//! # The confusion this exists to prevent
//!
//! Every golden this repository commits is now a picture of art built from
//! models on disk. So two entirely different mistakes have the same first
//! symptom — a frame that does not match its golden — and they want opposite
//! responses. *Somebody edited a model and did not rebuild* is fixed by one
//! command and nothing is wrong with the renderer. *Somebody changed the draw
//! path* is a defect, and re-minting the golden over it would record the defect
//! as ground truth. A suite that reported both as an image diff would teach
//! whoever met the first one to re-mint, which is precisely how the second one
//! ships.
//!
//! # Why this belongs to the phase where pixels come from the set
//!
//! The discriminating half is *rather than as a golden-frame mismatch*, and it
//! cannot be exercised where no pixel depends on the set: with nothing drawn
//! from the art there is no golden mismatch to be told apart from, and both
//! readings would pass for a reason that goes away the moment the art is wired.
//! They are here, and not in the phase that built the verdict, for that reason
//! alone.
//!
//! # The instrument is the golden entry point itself
//!
//! Both readings go through `goldens::verified_over`, which is
//! `terrain_goldens.rs`'s own path with the content root as a parameter. Asking
//! `prepare_scene` directly would be asking whether the *verdict* exists —
//! `built_set_verdict.rs` asks that — and would say nothing about whether the
//! suites that shoot goldens consult it. They do, because every golden is shot
//! through `prepare_scene`, and that is the property these two hold.
//!
//! # The verdict is enumerated, not absent
//!
//! Neither reading asserts that no golden mismatch was reported. It asserts
//! *which* refusal came back, which rejects every other answer including the two
//! that mean the check stopped looking — and a `PreparationError` arm and a
//! `GoldenOutcome` are different types, so "stale rather than a mismatch" is
//! carried by the answer's shape as well as by its value.

mod support;

use std::error::Error;

use mc_client::startup::{BUILD_THE_TEXTURE_SET, PreparationError};
use mc_testkit::frame::GoldenOutcome;

use support::goldens::{OPENING, verified_over};
use support::{TestResult, built_sets, refusal_printed_over};

#[test]
fn a_model_edited_since_the_build_stops_a_golden_run_as_a_stale_set() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::with_a_model_edited(root.path())?;

    let run = verified_over(root.path(), OPENING);

    let refused = refusal_of(run)?;
    assert!(
        matches!(refused, PreparationError::TextureSetStale),
        "a model edited since the set was built is one command away from being right, and the run \
         has to say so before it draws anything. Reported instead: {refused:?} — and if that is a \
         golden mismatch, whoever meets it will re-mint the golden over art built from the old \
         model and the repository will carry a reference nobody can explain"
    );
    let printed = refusal_printed_over(root.path())?;
    assert!(
        printed.contains(BUILD_THE_TEXTURE_SET),
        "the refusal has to carry the command that clears it, or the distinction it draws costs \
         somebody the same afternoon a golden diff would. It said: {printed}"
    );
    Ok(())
}

#[test]
fn a_run_with_no_set_built_at_all_stops_as_an_absent_set() -> TestResult {
    let root = built_sets::a_root_with_a_built_set()?;
    built_sets::without_the_index(root.path())?;

    let run = verified_over(root.path(), OPENING);

    let refused = refusal_of(run)?;
    assert!(
        matches!(refused, PreparationError::TextureSetAbsent),
        "a checkout that has never run the art build has no set at all, and the whole of the \
         mitigation for that is this sentence naming the build command. A golden mismatch here \
         would tell a new contributor their renderer is broken on their first run. Reported \
         instead: {refused:?}"
    );
    let printed = refusal_printed_over(root.path())?;
    assert!(
        printed.contains(BUILD_THE_TEXTURE_SET),
        "this is the one refusal a contributor meets before they have ever seen the game, so it \
         names the command. It said: {printed}"
    );
    Ok(())
}

/// What a golden verdict reads as to a person.
///
/// **Never `{outcome:?}`.** A `Failed` carries the per-pixel failing mask, and
/// one of those is 921 600 booleans — debug-printing it turned this failure into
/// **5.5 MB** and buried the sentence a reader needs under a boolean array.
/// `terrain_goldens.rs`'s header records making that mistake once and never
/// debug-printing an outcome since; this was the same mistake in a second file,
/// found by a mutation rather than by anybody reading, which is why the arm is
/// named here rather than formatted.
///
/// `GoldenFailure` has a hand-written `Display` that says which golden, how many
/// pixels stood past the tolerance, the worst distance and where the evidence
/// went. That is the useful half and it carries no mask, so the `Failed` arm
/// delegates to it rather than being flattened to a word.
fn said_about(outcome: Option<&GoldenOutcome>) -> String {
    match outcome {
        None => "no verdict at all — the opt-in permitted the absence of a device".to_owned(),
        Some(GoldenOutcome::Pass) => "Pass: the capture matched its committed golden".to_owned(),
        Some(GoldenOutcome::GoldenUnchanged) => {
            "GoldenUnchanged: a mint left the golden as it was".to_owned()
        }
        Some(GoldenOutcome::GoldenWritten { paths }) => {
            format!("GoldenWritten: {} golden(s) minted", paths.len())
        }
        Some(GoldenOutcome::GoldenWrittenWithoutProvenance { paths, failure }) => format!(
            "GoldenWrittenWithoutProvenance: {} golden(s) minted, sidecar refused with {failure}",
            paths.len()
        ),
        Some(GoldenOutcome::Failed(failure)) => format!("Failed: {failure}"),
    }
}

/// The preparation refusal a golden run ended in.
///
/// # Errors
///
/// Returns a failure when the run produced a golden verdict of any kind, or a
/// failure of some other kind entirely: both mean the reading below would be
/// about something other than the set, and both are worth naming rather than
/// unwrapping past.
fn refusal_of(
    run: Result<Option<GoldenOutcome>, Box<dyn Error>>,
) -> Result<PreparationError, Box<dyn Error>> {
    let failure = match run {
        Ok(outcome) => {
            return Err(format!(
                "this reading is about a golden run stopped by the state of the texture set, and \
                 the run reached a golden verdict instead: {verdict}. A set that is not current \
                 must not be drawn from at all",
                verdict = said_about(outcome.as_ref())
            )
            .into());
        }
        Err(failure) => failure,
    };
    let said = failure.to_string();
    match failure.downcast::<PreparationError>() {
        Ok(refused) => Ok(*refused),
        Err(_) => Err(format!(
            "the run failed for a reason that is not a preparation refusal at all, so nothing \
             here can say what it decided about the set: {said}"
        )
        .into()),
    }
}
