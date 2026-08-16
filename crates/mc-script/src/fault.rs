//! What the host reports when something inside script goes wrong.
//!
//! One flat fault type, deliberately. The two definition ports already in the
//! engine sit under an `Unreadable`/`Malformed` split, which separates "the
//! source could not be read at all" from "one item in it was wrong". A script
//! host has no such division: every fault it raises is about one entry into
//! script, and the discriminating axis is *why* that entry failed rather than
//! *which field* was bad.
//!
//! Three facts are typed fields rather than text inside [`ScriptFault::cause`],
//! and the reason is the same for all three. The backend's message formatting
//! is not a contract — it is a pre-1.0 dependency's rendering — so a fact buried
//! in it leaves substring matching as the only available assertion. The line, the
//! refused target and the kind are all facts something downstream has to act on,
//! so they are carried where they can be compared.

use std::fmt;

use crate::{Attachment, ChunkName, ComponentName, RoundIndex, SubjectName};

/// Why an entry into script ended the way it did.
///
/// A plain enum carrying no data on any variant, and that is load-bearing
/// rather than incidental: a harness declaring the containment evidence it
/// expects compares an expected kind against an observed one **by equality**.
/// Hang a payload on one variant and every such comparison degrades to picking
/// the discriminant apart, which is a weaker check written in more code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaultKind {
    /// The invocation used its whole call-and-loop budget without returning.
    BudgetExhausted,
    /// The invocation allocated past what it is allowed to hold.
    Allocation,
    /// Script raised an error of its own.
    ScriptError,
    /// A chunk could not be compiled.
    Compilation,
    /// Follow-up work could not be admitted, so it will never run.
    CascadeRefused,
    /// Follow-up work did not fit this round and runs in a later one.
    CascadeDeferred,
    /// The host itself was short of memory, so this failure may not be the
    /// running attachment's own.
    HostMemoryPressure,
}

impl FaultKind {
    /// How this kind reads in a rendered fault.
    fn as_str(self) -> &'static str {
        match self {
            Self::BudgetExhausted => "call and loop budget exhausted",
            Self::Allocation => "allocation refused",
            Self::ScriptError => "script error",
            Self::Compilation => "compilation failed",
            Self::CascadeRefused => "cascade refused",
            Self::CascadeDeferred => "cascade deferred",
            Self::HostMemoryPressure => "host memory pressure",
        }
    }
}

/// Where a fault came from: the chunk that defined the failing script, and the
/// dispatch round it ran in.
///
/// Both are optional because both shapes occur. A chunk evaluated outside any
/// round has a chunk and no round; a fault the host raises on its own behalf has
/// a round and no chunk.
///
/// **The chunk is the defining chunk, not the round's.** Naming only the round
/// would leave the most common fault in the system pointing at no file at all,
/// which hands a mod author an error and no way to locate it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptOrigin {
    /// The chunk that defined the script that failed.
    pub chunk: Option<ChunkName>,
    /// The dispatch round the failure happened in.
    pub round: Option<RoundIndex>,
}

/// One entry into script that did not end the way it was asked to.
///
/// Returned as a value. Nothing a script does reaches a caller as a panic or as
/// a backend error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptFault {
    /// The defining chunk and the dispatch round, either of which may be absent.
    pub origin: ScriptOrigin,
    /// What the failing callback was attached to. Absent for a chunk-level
    /// fault, which runs before any attachment exists, and for a fault the host
    /// raises about its own condition, which would otherwise blame whoever
    /// happened to be running.
    pub subject: Option<SubjectName>,
    /// The behaviour that failed, absent on the same terms as `subject`.
    pub component: Option<ComponentName>,
    /// Why it failed.
    pub kind: FaultKind,
    /// The line the backend named, where it named one.
    pub line: Option<u32>,
    /// The attachment whose follow-up work was turned away. Carried only by
    /// [`FaultKind::CascadeRefused`], which is the one kind naming two
    /// attachments: the one that asked and the one that was refused.
    pub refused_target: Option<Attachment>,
    /// What went wrong, in the terms it was reported in.
    ///
    /// **Unbounded, script-controlled text.** Rendering it raw is correct —
    /// rendering it any other way would run script on the host's schedule — but
    /// raw and safe-to-splice are different properties. Every consumer that
    /// logs it, formats it or shows it to a human inherits whatever a mod put
    /// here, at whatever length a mod chose.
    pub cause: String,
}

impl fmt::Display for ScriptFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_origin(&self.origin, formatter)?;
        if let Some(subject) = &self.subject {
            write!(formatter, ", subject `{}`", subject.as_str())?;
        }
        if let Some(component) = &self.component {
            write!(formatter, ", component `{}`", component.as_str())?;
        }
        if let Some(line) = self.line {
            write!(formatter, ", line {line}")?;
        }
        if let Some(refused) = &self.refused_target {
            write!(
                formatter,
                ", refused `{}`/`{}`",
                refused.subject.as_str(),
                refused.component.as_str()
            )?;
        }
        write!(formatter, ": {}: {}", self.kind.as_str(), self.cause)
    }
}

/// Renders the four shapes an origin has, rather than leaving two empty options
/// to whatever a format string would do with them.
///
/// A fault that can place itself nowhere says so. The alternative renders a gap,
/// and a gap is what a reader mistakes for a locator that got lost.
fn write_origin(origin: &ScriptOrigin, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match (&origin.chunk, origin.round) {
        (Some(chunk), Some(round)) => {
            write!(
                formatter,
                "chunk `{}`, round {}",
                chunk.as_str(),
                round.get()
            )
        }
        (Some(chunk), None) => write!(formatter, "chunk `{}`", chunk.as_str()),
        (None, Some(round)) => write!(formatter, "round {}", round.get()),
        (None, None) => write!(formatter, "unattributed"),
    }
}
