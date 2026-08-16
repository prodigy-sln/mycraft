//! What a mod can keep between invocations, what that costs, and who gets
//! blamed for it.
//!
//! # The mechanism, and why no limit here reaches it
//!
//! `local kept = {} return function() kept[#kept + 1] = ... end` is the whole
//! construction: a table held by a closure upvalue, appended to on every
//! invocation. No state API is involved, none is needed, and nothing in this
//! design forbids it — a callback that remembers something between invocations
//! is the ordinary way to write content, not an attack. The per-invocation cap
//! bounds what one entry **adds**; it says nothing about what the state is
//! already holding on somebody's behalf, so a mod retaining a fraction of its
//! cap each time never trips it and grows without limit.
//!
//! # Stated precisely, because both halves are easy to get wrong
//!
//! **Aggregate retention is bounded.** The allocator's absolute backstop is a
//! ceiling the whole state cannot pass, and this test asserts it is not passed.
//! What is unbounded is retention **per attachment**, and the damage is
//! **misattribution**: as the ceiling is approached, every other mod's ordinary
//! allocations begin failing, and the host — which cannot tell whose retention
//! filled the state — reports those failures against nobody. So the mod that
//! did nothing wrong gets a failure it did not cause, and the mod that caused it
//! is named in no fault anywhere. That is the price the design accepts rather
//! than the defect it hides: a ledger attributing retained bytes was refused,
//! and refused with its cost written down, which is what this test writes out in
//! observable form.
//!
//! The framing is the **accidental** one. Careless retention misfiling blame is
//! what a server running mods its operator chose actually meets; a mod
//! weaponising this is not the population this project has.
//!
//! # The second vector, and exactly what is known about it
//!
//! A suspended coroutine retains its own stack, its locals and everything they
//! reference, for as long as a reference to it survives — and a reference to it
//! is itself just an upvalue. What was measured about `coroutine` is that the
//! interrupt fires inside `resume` and `wrap` and that the latch is not void
//! there. **That settles execution and nothing else.** *"The latch contains it"*
//! and *"it cannot retain across invocations"* are two claims, and only the
//! first has evidence. The second test below is about the second claim only: it
//! shows the vector is real, and it settles nothing about containment in either
//! direction.
//!
//! # Fixture construction is the constraint no assertion can enforce
//!
//! Every assertion here is equally satisfied by a fixture that filled nothing,
//! so the fill fails loudly rather than returning a state it did not establish,
//! and the growth it produced is compared against the host's **own** cap rather
//! than against a number written here. **The retained values are distinct**:
//! the backend shares identical strings, so a loop retaining the same one
//! allocates it once and every count in this file would still read plausibly
//! while nothing whatever was retained.

use std::error::Error;
use std::num::{NonZeroU32, NonZeroU64, NonZeroUsize};

use mc_script::{
    Attachment, ComponentName, DispatchReport, FaultKind, HostLimits, ScriptFault, ScriptFunction,
    ScriptHost, ScriptValue, SubjectName,
};

type TestResult = Result<(), Box<dyn Error>>;

/// What one invocation may add above the baseline it started from.
const MEMORY_CAP: usize = 256 * 1024;

/// The ceiling the whole state may reach.
///
/// Above the empty state's own baseline plus the cap, so the host agrees to
/// start; low enough that a fixture can fill the room below it in a bounded
/// number of rounds.
const MEMORY_BACKSTOP: usize = 2048 * 1024;

/// A budget the work here cannot approach, so that no test in this file
/// measures the wrong limit.
const BUDGET: u64 = 500_000;

/// How many consecutive faults would quarantine an attachment.
const FAULT_THRESHOLD: u32 = 3;

/// How much the retaining callback keeps per invocation.
///
/// A fraction of the cap, which is the whole point: it never comes close to
/// tripping the limit on what one invocation may add, and it accumulates
/// anyway.
const RETAINED_PER_INVOCATION: usize = 16 * 1024;

/// What the innocent callback asks for, and retains none of.
const AN_ORDINARY_ALLOCATION: usize = 64 * 1024;

/// The most fill rounds before the fixture declares itself unable to establish
/// the state it needs.
const FILL_ROUNDS_ALLOWED: usize = 512;

/// How many more invocations the keeper is given, after the state is full, to
/// meet the shortage it created.
const ROUNDS_AFTER_THE_FILL: usize = 32;

/// What the coroutine allocates before it suspends, and the suffix that makes
/// the value distinct from any other string in the state.
const HELD_BY_THE_COROUTINE: usize = 64 * 1024;
const A_DISTINGUISHING_SUFFIX: &str = "held";

/// A callback that keeps what it allocates in a table its closure holds.
///
/// The index on the end is load-bearing: identical strings are shared by the
/// backend, so without it every invocation appends another reference to one
/// string and the state grows by nothing at all.
fn chunk_that_keeps(bytes: usize) -> String {
    format!(
        "local kept = {{}}\n\
         return function()\n\
         \tkept[#kept + 1] = string.rep('x', {bytes}) .. #kept\n\
         \treturn #kept\n\
         end\n"
    )
}

/// A callback that allocates and keeps nothing.
fn chunk_that_keeps_nothing(bytes: usize) -> String {
    format!(
        "return function()\n\
         \tlocal held = string.rep('x', {bytes})\n\
         \treturn #held\n\
         end\n"
    )
}

/// A callback that suspends a coroutine holding what it allocated, and resumes
/// it on every later invocation.
///
/// The size comes back out of the coroutine's **own stack** each time: the first
/// invocation reports it from the yield, and the second from the return that
/// follows it — a value allocated during one invocation and still held during
/// the next, with no state API and no table in sight.
fn chunk_that_suspends_a_coroutine(bytes: usize) -> String {
    format!(
        "local suspended = nil\n\
         return function()\n\
         \tif suspended == nil then\n\
         \t\tsuspended = coroutine.create(function()\n\
         \t\t\tlocal held = string.rep('x', {bytes}) .. '{A_DISTINGUISHING_SUFFIX}'\n\
         \t\t\tcoroutine.yield(#held)\n\
         \t\t\treturn #held\n\
         \t\tend)\n\
         \tend\n\
         \tlocal resumed, size = coroutine.resume(suspended)\n\
         \tif not resumed then return -1 end\n\
         \treturn size\n\
         end\n"
    )
}

/// What a fault said about whose failure it was.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedFault {
    kind: FaultKind,
    subject: Option<String>,
    component: Option<String>,
}

/// A failure the host reported about its own condition, blaming nobody.
fn blamed_on_nobody() -> ObservedFault {
    ObservedFault {
        kind: FaultKind::HostMemoryPressure,
        subject: None,
        component: None,
    }
}

fn described(fault: &ScriptFault) -> ObservedFault {
    ObservedFault {
        kind: fault.kind,
        subject: fault
            .subject
            .as_ref()
            .map(SubjectName::as_str)
            .map(str::to_owned),
        component: fault
            .component
            .as_ref()
            .map(ComponentName::as_str)
            .map(str::to_owned),
    }
}

fn attachment(subject: &str, component: &str) -> Attachment {
    Attachment {
        subject: SubjectName::new(subject),
        component: ComponentName::new(component),
    }
}

fn host_at_the_named_limits() -> Result<ScriptHost, Box<dyn Error>> {
    let limits = HostLimits {
        call_and_loop_budget: NonZeroU64::new(BUDGET).ok_or("the budget must not be zero")?,
        memory_cap: NonZeroUsize::new(MEMORY_CAP).ok_or("the cap must not be zero")?,
        memory_backstop: NonZeroUsize::new(MEMORY_BACKSTOP)
            .ok_or("the backstop must not be zero")?,
        fault_threshold: NonZeroU32::new(FAULT_THRESHOLD)
            .ok_or("the fault threshold must not be zero")?,
        ..HostLimits::default()
    };
    ScriptHost::with_limits(limits)
        .map_err(|error| format!("the host refused these limits: {error}").into())
}

fn callback_from(
    host: &mut ScriptHost,
    chunk: &str,
    source: &str,
) -> Result<ScriptFunction, Box<dyn Error>> {
    match host.evaluate(chunk, source) {
        Ok(ScriptValue::Function(callback)) => Ok(callback),
        Ok(other) => {
            Err(format!("`{chunk}` was written to return a function, not {other:?}").into())
        }
        Err(fault) => Err(format!("`{chunk}` did not evaluate: {fault}").into()),
    }
}

/// What one invocation handed back, as one comparable line.
fn result_for(report: &DispatchReport, attachment: &Attachment) -> String {
    match report.results.get(attachment) {
        Some(ScriptValue::Integer(number)) => format!("integer {number}"),
        Some(other) => format!("{other:?}"),
        None => "no result".to_owned(),
    }
}

/// Whether `bytes` more would still fit below the ceiling, measured after a
/// collection.
fn fits(host: &ScriptHost, bytes: usize) -> bool {
    host.collected_memory_in_use().saturating_add(bytes) <= host.limits().memory_backstop.get()
}

/// What the retention did to the state, and to everybody in it.
#[derive(Debug, PartialEq, Eq)]
struct Retention {
    kept_more_than_one_invocation_may_add: bool,
    aggregate_stayed_below_the_ceiling: bool,
    what_the_innocent_mod_met: Vec<ObservedFault>,
    the_keeper_failed_in_the_end_too: bool,
    faults_naming_the_keeper: usize,
    keeper_quarantined: bool,
}

/// What the design says this looks like: unbounded per attachment, bounded in
/// aggregate, and nothing anywhere naming who caused it.
///
/// The keeper's own failure is what makes the last three lines mean anything. A
/// mod that never failed would be unnamed in every fault trivially; this one
/// fails, in the state it filled itself, and is still not named — which is the
/// refused ledger's price written out.
fn the_price_the_design_accepts() -> Retention {
    Retention {
        kept_more_than_one_invocation_may_add: true,
        aggregate_stayed_below_the_ceiling: true,
        what_the_innocent_mod_met: vec![blamed_on_nobody()],
        the_keeper_failed_in_the_end_too: true,
        faults_naming_the_keeper: 0,
        keeper_quarantined: false,
    }
}

/// Runs the retaining mod until an ordinary allocation no longer fits, keeping
/// every fault raised on the way.
///
/// It fails loudly rather than returning a state it did not establish: a fill
/// that quietly stopped early would leave every assertion below green and
/// measuring nothing.
fn keep_until_the_state_is_full(
    host: &mut ScriptHost,
    keeper: &Attachment,
) -> Result<Vec<ObservedFault>, Box<dyn Error>> {
    let mut raised = Vec::new();
    for _ in 0..FILL_ROUNDS_ALLOWED {
        if !fits(host, AN_ORDINARY_ALLOCATION) {
            return Ok(raised);
        }
        let round = host.dispatch(std::slice::from_ref(keeper));
        raised.extend(round.faults.iter().map(described));
    }
    Err(format!(
        "the fixture could not fill the state to within {AN_ORDINARY_ALLOCATION} bytes of its \
         {MEMORY_BACKSTOP}-byte ceiling in {FILL_ROUNDS_ALLOWED} rounds: it reached {} bytes, and \
         every assertion that follows would have measured a state nobody had filled",
        host.collected_memory_in_use()
    )
    .into())
}

/// Invokes the retaining mod again once the state it filled has no room left.
///
/// Each invocation keeps a little more, so a state this close to its ceiling
/// cannot serve many of them — but the loop is bounded rather than trusting
/// that, and a host that served every one of them reports as a fixture that
/// established nothing rather than as a run that never ended.
fn until_the_keeper_itself_fails(
    host: &mut ScriptHost,
    keeper: &Attachment,
) -> (Vec<ObservedFault>, bool) {
    let mut raised = Vec::new();
    for _ in 0..ROUNDS_AFTER_THE_FILL {
        let round = host.dispatch(std::slice::from_ref(keeper));
        raised.extend(round.faults.iter().map(described));
        if !raised.is_empty() {
            return (raised, true);
        }
    }
    (raised, false)
}

/// How much of the state the retaining mod is holding, and what everyone met —
/// the mod that kept nothing, and the keeper itself.
fn what_retention_cost(
    host: &mut ScriptHost,
    keeper: &Attachment,
    bystander: &Attachment,
) -> Result<Retention, Box<dyn Error>> {
    let before = host.collected_memory_in_use();
    let mut raised = keep_until_the_state_is_full(host, keeper)?;
    let kept = host.collected_memory_in_use().saturating_sub(before);
    let met: Vec<ObservedFault> = host
        .dispatch(std::slice::from_ref(bystander))
        .faults
        .iter()
        .map(described)
        .collect();
    let (by_the_keeper, keeper_failed) = until_the_keeper_itself_fails(host, keeper);
    raised.extend(met.iter().cloned());
    raised.extend(by_the_keeper);
    Ok(Retention {
        kept_more_than_one_invocation_may_add: kept > host.limits().memory_cap.get(),
        aggregate_stayed_below_the_ceiling: host.collected_memory_in_use()
            <= host.limits().memory_backstop.get(),
        what_the_innocent_mod_met: met,
        the_keeper_failed_in_the_end_too: keeper_failed,
        faults_naming_the_keeper: named(&raised, keeper),
        keeper_quarantined: host.is_quarantined(keeper),
    })
}

/// How many of these faults name `attachment` as the mod that failed.
fn named(raised: &[ObservedFault], attachment: &Attachment) -> usize {
    raised
        .iter()
        .filter(|fault| fault.component.as_deref() == Some(attachment.component.as_str()))
        .count()
}

const WHY_RETENTION_IS_UNBOUNDED_PER_MOD_AND_NAMES_NOBODY: &str = "the retaining mod keeps a fraction of its allowance on every invocation, so it trips no \
     limit — a fault naming it would mean the cap had caught something it cannot catch — and \
     ends up holding several times what one invocation may add. That much is by design. What \
     the design accepts, and what this states in observable form, is the rest: the mod that \
     asked for an ordinary allocation and kept nothing is the one that fails, its failure is \
     reported against nobody because the host cannot tell whose retention filled the state, \
     and no fault anywhere names the mod that did. An operator sees that scripting is degraded \
     and has no way to learn who degraded it. The aggregate is still bounded — the ceiling \
     holds, which is the half that must not be read as also bounding the other one.";

#[test]
fn a_mod_that_keeps_what_it_allocates_fills_the_state_and_the_failures_land_on_somebody_else()
-> TestResult {
    let mut host = host_at_the_named_limits()?;
    let keeping = callback_from(
        &mut host,
        "hoarder.luau",
        &chunk_that_keeps(RETAINED_PER_INVOCATION),
    )?;
    let ordinary = callback_from(
        &mut host,
        "furnace.luau",
        &chunk_that_keeps_nothing(AN_ORDINARY_ALLOCATION),
    )?;
    let keeper = attachment("stone-hoarder", "remember");
    let bystander = attachment("stone-furnace", "smelt");
    host.attach(keeper.clone(), keeping);
    host.attach(bystander.clone(), ordinary);

    assert_eq!(
        what_retention_cost(&mut host, &keeper, &bystander)?,
        the_price_the_design_accepts(),
        "{WHY_RETENTION_IS_UNBOUNDED_PER_MOD_AND_NAMES_NOBODY}"
    );
    Ok(())
}

/// What a suspended coroutine reported across three invocations, and whether
/// the state grew while it was suspended.
#[derive(Debug, PartialEq, Eq)]
struct Suspended {
    from_the_yield: String,
    from_the_return: String,
    once_it_is_finished: String,
    held_while_suspended: bool,
}

const WHY_THE_COROUTINE_IS_RECORDED_AS_A_SECOND_VECTOR: &str = "a suspended coroutine holds its own stack and everything on it, and a reference to one \
     is an upvalue like any other — so permitting `coroutine` adds a second route to the \
     retention above, by a mechanism no table and no state API is involved in. What is \
     asserted here is only that the route exists: the same allocation is reported from inside \
     the coroutine on a later invocation than the one that made it, and the state carried it \
     in between. **This settles nothing about containment.** What was measured about \
     `coroutine` is that the interrupt fires inside `resume` and `wrap` and that the latch is \
     not void there, which is about execution; whether retention across invocations is \
     bounded was never measured and is not measured here. If this test ever reddens because \
     retention has become bounded, the risk it characterises has been closed and the test \
     should be retired rather than repaired.";

/// Resumes the sleeping mod three times — to its yield, past it, and once more
/// after the coroutine has finished — with the state read on both sides of the
/// invocation that suspends it.
fn what_the_sleeping_mod_reported(host: &mut ScriptHost, dream: &Attachment) -> Suspended {
    let before = host.collected_memory_in_use();
    let yielded = host.dispatch(std::slice::from_ref(dream));
    let while_suspended = host.collected_memory_in_use();
    let returned = host.dispatch(std::slice::from_ref(dream));
    let afterwards = host.dispatch(std::slice::from_ref(dream));
    Suspended {
        from_the_yield: result_for(&yielded, dream),
        from_the_return: result_for(&returned, dream),
        once_it_is_finished: result_for(&afterwards, dream),
        held_while_suspended: while_suspended.saturating_sub(before) >= HELD_BY_THE_COROUTINE,
    }
}

/// What a coroutine that suspended holding an allocation owes, derived from the
/// size it was told to make rather than from a run.
fn what_it_should_still_be_holding() -> Suspended {
    let held = format!(
        "integer {}",
        HELD_BY_THE_COROUTINE + A_DISTINGUISHING_SUFFIX.len()
    );
    Suspended {
        from_the_yield: held.clone(),
        from_the_return: held,
        once_it_is_finished: "integer -1".to_owned(),
        held_while_suspended: true,
    }
}

#[test]
fn a_suspended_coroutine_holds_what_it_allocated_across_invocations() -> TestResult {
    let mut host = host_at_the_named_limits()?;
    let suspending = callback_from(
        &mut host,
        "sleeper.luau",
        &chunk_that_suspends_a_coroutine(HELD_BY_THE_COROUTINE),
    )?;
    let dream = attachment("stone-sleeper", "dream");
    host.attach(dream.clone(), suspending);

    assert_eq!(
        what_the_sleeping_mod_reported(&mut host, &dream),
        what_it_should_still_be_holding(),
        "{WHY_THE_COROUTINE_IS_RECORDED_AS_A_SECOND_VECTOR}"
    );
    Ok(())
}
