//! The hostile-mod harness: six named cases, what each one needs the host to
//! have done, and the three answers it is allowed to give.
//!
//! # Why the verdict is three-valued
//!
//! Two of the six are contained **precisely by producing no fault**:
//! `sandbox-escape` reaches nothing and so raises nothing, and `hostile-index`
//! is contained by a metamethod that never ran. A harness whose verdict were
//! "did a fault appear?" would call both of those uncontained, and — far worse —
//! would report a case whose script never compiled as clean, because a script
//! that never ran raises nothing either. So each case declares the evidence its
//! containment requires, and the verdict names the two failures apart:
//! `Uncontained` for a case that ran and did not produce its evidence,
//! `NotExercised` for one whose script never compiled. A harness that stopped
//! running must not read like one that ran clean.
//!
//! # The harness is asserted against the host's own enforcement
//!
//! Every scenario in this file runs **through** the harness, so nothing else in
//! this suite can see the harness cheating. A harness carrying its own copy of
//! the deny list, its own budget, or its own latch agrees with the host by
//! construction: it would report all six contained on the day the host's
//! enforcement was deleted. What keeps it honest is that the deny list it probes
//! is the host's declared one, the faults it requires are the host's own, and the
//! limits that stop the runaway cases are the ones the host ships — the host is
//! built here with no configuration at all, so the numbers doing the containing
//! are the numbers a server runs. `tests/harness_boundaries.rs` enforces that as
//! a text guard.
//!
//! # The two cases that could not be written by hand
//!
//! `Uncontained` and `NotExercised` cannot be reached by any of the six against a
//! working host — that is the point of them. They are driven by handing the
//! harness a case of the caller's own making: one that returns quietly while
//! declaring it needs a fault, and one whose script does not parse. Both are
//! judged by the same code path the six go through.

use std::error::Error;

use mc_script::{FaultKind, ScriptFault, ScriptHost, ScriptValue};

#[path = "support/hostile/mod.rs"]
mod hostile;

use hostile::{CaseOutcome, CaseReport, ContainmentEvidence, HostileCase, hostile_cases, run};

type TestResult = Result<(), Box<dyn Error>>;

/// A benign chunk the host is asked for after the six have run, and the value it
/// owes.
///
/// It is arithmetic rather than a literal so that a host answering with
/// something it was handed cannot satisfy it.
const A_BENIGN_CHUNK: &str = "return 3 + 4";
const WHAT_THE_BENIGN_CHUNK_RETURNS: &str = "integer 7";

/// A case that returns quietly while declaring that containing it requires a
/// fault.
///
/// This is what a hostile case looks like the day the mechanism it was written
/// against is removed: it runs to the end and reports nothing.
const A_CASE_THAT_RUNS_TO_COMPLETION: &str = "return function()\n\treturn 1\nend\n";

/// A case whose script cannot be compiled at all.
const A_CASE_THAT_DOES_NOT_PARSE: &str = "return function( end\n";

/// A case that fails for a reason other than the one it declares.
const A_CASE_THAT_RAISES_ITS_OWN_ERROR: &str =
    "return function()\n\terror('a fault of the wrong kind')\nend\n";

fn new_host() -> Result<ScriptHost, Box<dyn Error>> {
    ScriptHost::new().map_err(|error| format!("the host could not be constructed: {error}").into())
}

/// What an evaluation produced, as one comparable line.
///
/// A fault renders too, so a test expecting a value and handed a fault fails
/// with the fault in its diff rather than needing an unwrap.
fn outcome(evaluated: Result<ScriptValue, ScriptFault>) -> String {
    match evaluated {
        Ok(ScriptValue::Nil) => "nil".to_owned(),
        Ok(ScriptValue::Boolean(flag)) => format!("boolean {flag}"),
        Ok(ScriptValue::Integer(number)) => format!("integer {number}"),
        Ok(ScriptValue::Number(number)) => format!("number {number}"),
        Ok(ScriptValue::Text(text)) => format!("text {text}"),
        Ok(ScriptValue::Table(_)) => "table".to_owned(),
        Ok(ScriptValue::Function(_)) => "function".to_owned(),
        Ok(ScriptValue::Opaque) => "opaque".to_owned(),
        Err(fault) => format!("fault: {fault}"),
    }
}

/// The report a contained case owes, built from the case itself so that a
/// renamed or reordered case is a difference rather than a coincidence.
fn contained(case: &HostileCase) -> CaseReport {
    CaseReport {
        name: case.name,
        outcome: CaseOutcome::Contained,
    }
}

#[test]
fn the_harness_reports_exactly_the_six_hostile_cases_it_is_named_for() -> TestResult {
    let named: Vec<&str> = hostile_cases().iter().map(|case| case.name).collect();

    assert_eq!(
        named,
        vec![
            "infinite-loop",
            "memory-bomb",
            "sandbox-escape",
            "faulting-callback",
            "runaway-cascade",
            "hostile-index",
        ],
        "the harness is this specification's deliverable, and what it is worth to somebody \
         reading its output is that the list is fixed and legible: six shapes a bad mod takes, \
         each named for what it does rather than for the mechanism that stops it. A case that \
         quietly disappeared from the list would take its whole shape out of the suite with \
         nothing else able to notice, because every one of these scenarios runs through this \
         one array"
    );
    Ok(())
}

const WHY_EACH_CASE_DECLARES_ITS_OWN_EVIDENCE: &str = "the four fault kinds are not interchangeable and the declaration is what stops them \
     being treated as though they were: a memory bomb stopped for exhausting its ticks has \
     measured the wrong mechanism entirely, and a harness asking only whether *something* \
     faulted would call that containment. The two that require no fault are why the evidence \
     is declared rather than inferred — a sandbox escape is contained by reaching nothing and \
     a hostile index by never running, so demanding a fault of either would report a working \
     host as broken and hide the shape of both.";

/// What each of the six owes, written out here rather than derived from the
/// harness — a list built by asking the harness what it declares would agree
/// with the harness whatever it declared.
fn the_evidence_the_six_owe() -> Vec<(&'static str, ContainmentEvidence)> {
    let fault = |kind| ContainmentEvidence::FaultReported(kind);
    vec![
        ("infinite-loop", fault(FaultKind::BudgetExhausted)),
        ("memory-bomb", fault(FaultKind::Allocation)),
        (
            "sandbox-escape",
            ContainmentEvidence::EveryDeniedGlobalUnavailable,
        ),
        ("faulting-callback", fault(FaultKind::ScriptError)),
        ("runaway-cascade", fault(FaultKind::CascadeRefused)),
        ("hostile-index", ContainmentEvidence::MetamethodNotInvoked),
    ]
}

#[test]
fn every_hostile_case_declares_the_containment_evidence_it_requires() -> TestResult {
    let declared: Vec<(&str, ContainmentEvidence)> = hostile_cases()
        .iter()
        .map(|case| (case.name, case.requires))
        .collect();

    assert_eq!(
        declared,
        the_evidence_the_six_owe(),
        "{WHY_EACH_CASE_DECLARES_ITS_OWN_EVIDENCE}"
    );
    Ok(())
}

const WHY_THE_HOST_HAS_TO_BE_USABLE_AFTERWARDS: &str = "containing six hostile mods one at a time is worth nothing if the seventh thing the \
     server asks of the scripting host is refused. Every case here runs against **one** host, \
     in sequence, and the benign chunk afterwards is the claim the whole deliverable is \
     about: a limit that latched and stayed latched, a queue left full of somebody's cascade, \
     or memory never given back would each leave a host that survived every hostile mod and \
     can no longer run an honest one. The expected reports are built from the case list \
     itself, so a case that silently stopped running cannot be hidden by a shorter answer.";

#[test]
fn all_six_hostile_cases_are_contained_in_sequence_and_the_host_still_evaluates_afterwards()
-> TestResult {
    let mut host = new_host()?;
    let cases = hostile_cases();

    let mut reported = Vec::new();
    for case in &cases {
        reported.push(run(&mut host, case));
    }
    let afterwards = outcome(host.evaluate("benign", A_BENIGN_CHUNK));

    assert_eq!(
        (reported, afterwards.as_str()),
        (
            cases.iter().map(contained).collect::<Vec<_>>(),
            WHAT_THE_BENIGN_CHUNK_RETURNS
        ),
        "{WHY_THE_HOST_HAS_TO_BE_USABLE_AFTERWARDS}"
    );
    Ok(())
}

const WHY_A_QUIET_CASE_IS_NAMED_RATHER_THAN_PASSED: &str = "this is what every one of the six looks like the day the mechanism it was written \
     against is deleted: the script runs to the end and nothing is reported. A harness that \
     said `contained` here — or that said nothing at all, which is the same thing to whoever \
     reads the run — would certify a host with no enforcement left in it. The name is half \
     the requirement: an operator or a reviewer needs to know *which* shape stopped being \
     contained, and a bare count of failures does not say.";

#[test]
fn a_case_that_runs_to_completion_without_its_declared_evidence_is_reported_uncontained_by_name()
-> TestResult {
    let mut host = new_host()?;
    let quiet = HostileCase::from_source(
        "quiet-callback",
        ContainmentEvidence::FaultReported(FaultKind::BudgetExhausted),
        A_CASE_THAT_RUNS_TO_COMPLETION,
    );

    assert_eq!(
        run(&mut host, &quiet),
        CaseReport {
            name: "quiet-callback",
            outcome: CaseOutcome::Uncontained,
        },
        "{WHY_A_QUIET_CASE_IS_NAMED_RATHER_THAN_PASSED}"
    );
    Ok(())
}

const WHY_A_CASE_THAT_NEVER_RAN_IS_NOT_CONTAINED: &str = "a script that does not compile exercises nothing, and it fails in exactly the way a \
     contained case does: no fault is raised by a callback that was never built. Reported as \
     contained, a typo in one hostile script would retire that shape from the suite silently \
     and every later run would agree the host was clean. `NotExercised` is the whole reason \
     the verdict has three values rather than two — it is the difference between a harness \
     that looked and a harness that could not look.";

#[test]
fn a_case_whose_script_does_not_compile_is_reported_not_exercised_by_name() -> TestResult {
    let mut host = new_host()?;
    let malformed = HostileCase::from_source(
        "unparseable",
        ContainmentEvidence::FaultReported(FaultKind::ScriptError),
        A_CASE_THAT_DOES_NOT_PARSE,
    );

    assert_eq!(
        run(&mut host, &malformed),
        CaseReport {
            name: "unparseable",
            outcome: CaseOutcome::NotExercised,
        },
        "{WHY_A_CASE_THAT_NEVER_RAN_IS_NOT_CONTAINED}"
    );
    Ok(())
}

const WHY_THE_WRONG_FAULT_IS_NOT_CONTAINMENT: &str = "the case above shows a harness cannot accept silence; this one shows it cannot accept \
     noise either. A judge asking `did anything fault?` passes this case, and with it passes \
     a memory bomb stopped by its tick budget — which is the masking this project has \
     measured and which leaves the cap itself untested while the run reads green. The kind is \
     compared for equality, which is what the fault enum carrying no payload on any variant \
     was decided for.";

#[test]
fn a_case_that_faults_for_a_reason_other_than_the_one_it_declares_is_uncontained() -> TestResult {
    let mut host = new_host()?;
    let mismatched = HostileCase::from_source(
        "wrong-fault",
        ContainmentEvidence::FaultReported(FaultKind::Allocation),
        A_CASE_THAT_RAISES_ITS_OWN_ERROR,
    );

    assert_eq!(
        run(&mut host, &mismatched),
        CaseReport {
            name: "wrong-fault",
            outcome: CaseOutcome::Uncontained,
        },
        "{WHY_THE_WRONG_FAULT_IS_NOT_CONTAINMENT}"
    );
    Ok(())
}

const WHY_THE_PROBE_HAS_TO_ASK_ABOUT_THE_HOSTS_OWN_LIST: &str = "the escape case is the one that could most easily agree with the host by construction. \
     A harness carrying its own list of names to try would report every one of them gone on \
     the day the host stopped removing the fourteenth — and it would report them gone even if \
     the host removed nothing the harness had thought to name. So the probe is generated from \
     `ScriptHost::DENIED_GLOBALS`, and the names come back **out of the script** that was \
     asked about them: this compares what content was really asked with what the host says it \
     denies, in the host's own order. A harness that quietly narrowed its probe reddens here \
     rather than reporting a clean escape case.";

#[test]
fn the_escape_case_asks_the_running_script_about_every_global_the_host_declares_denied()
-> TestResult {
    let mut host = new_host()?;

    let probed: hostile::EscapeProbe = hostile::probe_denied_globals(&mut host)?;

    assert_eq!(
        (probed.checked, probed.standing),
        (
            ScriptHost::DENIED_GLOBALS
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<String>>(),
            Vec::<String>::new()
        ),
        "{WHY_THE_PROBE_HAS_TO_ASK_ABOUT_THE_HOSTS_OWN_LIST}"
    );
    Ok(())
}
