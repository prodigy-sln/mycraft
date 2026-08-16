//! Running one hostile case against a host, and reporting what the host did
//! about it.
//!
//! Every observation here is taken from the host's own report — the faults it
//! raised, the value it handed back, the lines it recorded content printing —
//! and never from a second opinion this module formed for itself. What is
//! decided here is only *how* a case is driven; whether what came back counts as
//! containment is decided against the case's declared evidence one level up.

use mc_script::{
    Attachment, ComponentName, DispatchReport, FaultKind, ScriptFunction, ScriptHost, ScriptValue,
    SubjectName,
};

use super::{HostileCase, Shape, scripts};

/// How many drain rounds a cascade is given before the harness gives up on it.
///
/// A fixture guard and not a limit on the host: the host bounds each round on
/// its own, and this only stops the harness spinning if a queue never empties.
const DRAIN_ROUNDS_ALLOWED: usize = 64;

/// What running a case produced, before anything decides what it means.
#[derive(Debug)]
pub(crate) enum Observed {
    /// The case's script could not be compiled, so nothing was exercised.
    DidNotCompile,
    /// The script compiled and did not hand back what driving it required.
    NothingToDrive,
    /// Every fault the host raised while the case ran, in order.
    Faults(Vec<FaultKind>),
    /// What a chunk that went looking for the denied globals found.
    Escape(EscapeProbe),
    /// Whether the metatable on a table the mod supplied got its turn.
    Metamethod { invoked: bool },
}

/// What the escape case asked the running script, and what answered.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EscapeProbe {
    /// Every global the script was asked about, in the order it was asked —
    /// reported by the script itself, so a probe that quietly narrowed is
    /// visible here rather than reporting a clean escape.
    pub checked: Vec<String>,
    /// The ones still reachable from a content chunk.
    pub standing: Vec<String>,
}

impl EscapeProbe {
    /// Whether the sandbox held: nothing the host denies was reachable, and the
    /// question was put about the host's whole declared list.
    ///
    /// The second half is what stops this agreeing with itself. An empty
    /// `standing` is exactly what a probe that asked about nothing reports.
    pub(crate) fn every_denied_global_is_gone(&self) -> bool {
        self.standing.is_empty() && self.asked_about_the_hosts_whole_list()
    }

    fn asked_about_the_hosts_whole_list(&self) -> bool {
        self.checked.len() == ScriptHost::DENIED_GLOBALS.len()
            && self
                .checked
                .iter()
                .zip(ScriptHost::DENIED_GLOBALS.iter())
                .all(|(asked, declared)| asked == declared)
    }
}

/// What the escape case found, for a caller that wants to inspect the probe
/// rather than only the verdict it produced.
///
/// It runs the harness's own case through the harness's own path, so what it
/// reports is what the verdict was decided on and not a second probe that could
/// drift from it.
///
/// # Errors
///
/// Fails if the harness declares no escape case, or if running it produced
/// anything other than a probe.
pub fn probe_denied_globals(host: &mut ScriptHost) -> Result<EscapeProbe, String> {
    let escape = super::hostile_cases()
        .into_iter()
        .find(|case| case.shape() == Shape::ProbesEveryDeniedGlobal)
        .ok_or("the harness declares no sandbox-escape case")?;
    match observe(host, &escape) {
        Observed::Escape(probed) => Ok(probed),
        other => Err(format!(
            "the sandbox-escape case reported {other:?} rather than a probe of the denied globals"
        )),
    }
}

/// Evaluates the case's script and drives whatever it handed back.
pub(crate) fn observe(host: &mut ScriptHost, case: &HostileCase) -> Observed {
    let source = scripts::source_of(case, host);
    match host.evaluate(case.name, &source) {
        Err(fault) if fault.kind == FaultKind::Compilation => Observed::DidNotCompile,
        Err(_) => Observed::NothingToDrive,
        Ok(ScriptValue::Text(reported)) if case.shape() == Shape::ProbesEveryDeniedGlobal => {
            Observed::Escape(reported_by(&reported))
        }
        Ok(ScriptValue::Function(callback)) => drive(host, case, callback),
        Ok(_) => Observed::NothingToDrive,
    }
}

/// Attaches the case's callback and drives it the way its shape requires.
fn drive(host: &mut ScriptHost, case: &HostileCase, callback: ScriptFunction) -> Observed {
    let attachment = attachment_for(case.name);
    host.attach(attachment.clone(), callback);
    match case.shape() {
        Shape::SuppliesATableThatCounts => watch_the_metamethod(host, &attachment),
        Shape::RequestsFollowUpForever => {
            Observed::Faults(faults_from_the_cascade(host, &attachment))
        }
        _ => Observed::Faults(kinds(&host.dispatch(std::slice::from_ref(&attachment)))),
    }
}

/// The attachment a case runs under: one subject, the case's own name as its
/// component, so one case's faults and quarantine state are its own.
fn attachment_for(name: &str) -> Attachment {
    Attachment {
        subject: SubjectName::new(scripts::HOSTILE_SUBJECT),
        component: ComponentName::new(name),
    }
}

fn kinds(report: &DispatchReport) -> Vec<FaultKind> {
    report.faults.iter().map(|fault| fault.kind).collect()
}

/// Seeds the cascade, then drains what it left so the next case starts on an
/// empty queue.
///
/// Every round's faults are kept: a refusal raised while the queue was full
/// happens rounds before the drain ends, and keeping only the last round would
/// throw away the evidence this case exists for.
fn faults_from_the_cascade(host: &mut ScriptHost, attachment: &Attachment) -> Vec<FaultKind> {
    let mut report = host.dispatch(std::slice::from_ref(attachment));
    let mut seen = kinds(&report);
    let mut drained = 0;
    while report.pending > 0 && drained < DRAIN_ROUNDS_ALLOWED {
        report = host.dispatch(&[]);
        seen.extend(kinds(&report));
        drained += 1;
    }
    seen
}

/// Hands the host a table with a hostile `__index`, has it read a field the
/// table does not hold, and reports whether the metamethod got its turn.
///
/// Three witnesses rather than one, because each is blind to a different
/// failure: the counter says the metamethod ran, the printed line says so
/// independently of a script-side number the host would have to trust, and a
/// value coming back for a field nothing stored says the host believed it.
fn watch_the_metamethod(host: &mut ScriptHost, attachment: &Attachment) -> Observed {
    let supplied = host.dispatch(std::slice::from_ref(attachment));
    let Some(ScriptValue::Table(table)) = supplied.results.get(attachment).cloned() else {
        return Observed::NothingToDrive;
    };
    let said_before = host.printed().len();
    let read = host.read_field(&table, scripts::A_FIELD_THE_TABLE_LACKS);
    let afterwards = host.dispatch(std::slice::from_ref(attachment));
    let counted = afterwards.results.get(attachment);
    Observed::Metamethod {
        invoked: !matches!(counted, Some(ScriptValue::Integer(0)))
            || host.printed().len() > said_before
            || read.is_some(),
    }
}

/// What the escape chunk reported: the names it was asked about, then the ones
/// that answered.
fn reported_by(reported: &str) -> EscapeProbe {
    let (checked, standing) = reported.split_once('|').unwrap_or((reported, ""));
    EscapeProbe {
        checked: names_in(checked),
        standing: names_in(standing),
    }
}

fn names_in(list: &str) -> Vec<String> {
    list.split(',')
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}
