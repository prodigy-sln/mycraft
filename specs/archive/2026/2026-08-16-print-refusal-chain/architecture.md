# Architecture: A content refusal names the file, the block and the field it is about

Spec: `spec.md` (SPEC-015, rigor `high`, 26 scenarios). Requirements:
`requirements.md`. Branch `feature/PRO-939-print-refusal-chain` at `adb24d9`.

## Drivers

**Quality attributes that matter here, with the evidence.**

| Attribute | Why it matters for this feature | Evidence |
|-----------|--------------------------------|----------|
| **Operability for a mod author** | The whole capability is text on a terminal (Key Principle 7). A refusal that names the file, the declaration and the field turns a bisect into one edit. | `docs/modding/README.md:130-136` documents "change one file at a time" as the workaround. |
| **Falsifiability of the reporting path** | The defect being repaired is a *green test over a behaviour that never happens*: `crates/mc-client/tests/hud_launch.rs:67-75` walks `source()` by hand, asserts against its own walk, and never reaches any printing. Any design that leaves reporting testable only by re-implementing it reproduces exactly this. | `testing.md` §2, "Policy is not wiring". |
| **Coverage visibility** | ADR-013, as narrowed on 2026-08-12, excludes **`mc-client` wholesale** and only `crates/mc-render/src/gpu/` of `mc-render`. Reporting that lands in `mc-client` lands in the blind spot; reporting that lands in `mc-render/src/window.rs` lands beside `exit_code`, inside the denominator. | `docs/technical/decisions.md`, ADR-008 record note (2026-08-12); `crates/mc-render/src/window.rs:184-195`. |
| **Evolvability without an exemption list** | A guard whose scope is a hand-maintained list of permitted sites cannot go red when a new site stops reporting; it goes green with one more entry. This is how the original defect survived. | Team lead's Scope B ruling; `crates/mc-client/tests/seam_boundaries.rs:1-62`. |

**Constraints.**

- **Out of Scope is binding**, and two bullets bite hard: *no new error variants
  or changed refusal conditions*, and *argument parsing and exit-status selection
  stay in `main.rs`*. Together they rule out the obvious repair for the
  hand-formatted sites in `crates/mc-client/src/events.rs` (give each a typed
  error carrying its cause) — see Decision 3.
- No regulatory or data-sensitivity dimension. No personal data crosses any
  boundary here; the report is a refusal a person reads on their own terminal.
- Solo, agent-driven project. There is no reviewer; the gate and these scenarios
  carry the weight.

**Volatile, and expensive to reverse.**

- *Volatile*: the exact text `toml` 0.9.12 renders for an unknown field. Its
  five-line caret diagnostic is quoted verbatim in two documentation pages and
  compared by FR-7's guard, so a `toml` bump moves documented text.
- *Expensive to reverse*: where the renderer lives and what is allowed to
  construct a reported ending. Both touch five files across two crates and a
  public enum; both are Decision 1 and Decision 2 and get analysis.
- *Cheap to reverse*: the wording of any sentence, the needle list of a guard,
  the choice of fixture. These get one line.

## Boundaries

The design touches no network, no vendor SDK and no clock. Two entries are
nonetheless real and neither is exempt.

| External dependency | Volatility (V/R/S/Sub) | Port | Adapter location | Direct-use justification |
|---------------------|------------------------|------|------------------|--------------------------|
| **The process's error stream** — where a refusal is written | V low · R none · S stable · Sub trivial | `&mut dyn std::io::Write`, the sink parameter of `report` | `crates/mc-client/src/main.rs` supplies `std::io::stderr()`; nothing else in the library names a stream | — |
| **`toml` 0.9.12's `Display` for a deserialization refusal** | V low · R none · **S: a minor bump can restyle the caret diagnostic** · Sub none (it is the parser) | none — it is an in-process pure library, reached only through `mc-world`'s own `DefinitionFault.cause` | `crates/mc-world/src/content/toml_source.rs` | Architecture-principles §3 exclusion: in-process pure library, no process or network boundary, no vendor-risk axis. **It is listed because its output is quoted in `docs/modding/` and compared by FR-7's guard**, which is what makes a version bump visible instead of silent. |

The error stream is the boundary this feature actually exists to cross, and
making it a parameter rather than an `eprintln!` is the whole of what makes
FR-2, FR-3 and FR-6 assertable (D1 in `requirements.md`). It is a `&mut dyn
Write` and not a bespoke `ReportSink` trait: `Write` **is** the port for a byte
stream, and wrapping the standard library's own interface in a second one is the
"never wrap the framework" rule in `code-quality.md` §1.

## Decisions

### Decision 1 — The renderer and the reporting live in `mc-render::window` · BINDING

Satisfies: FR-1.1-S1..S4, FR-6.1-S1..S4.

**Options.**

| | Where | What it buys | What it costs |
|---|---|---|---|
| A | `crates/mc-client/` (a new `report.rs`, or `lib.rs`) | The reporting sits in the composition root that owns the failures. Shortest diff. | Lands in the crate ADR-013 excludes **wholesale** from the coverage denominator — the exact blindness D1 quotes `tools/voxforge/src/main.rs` against. `mc-client`'s own `lib.rs` states it "holds no policy"; how a failure reads is policy. |
| B | `crates/mc-render/src/window.rs`, beside `Ending` and `exit_code` | One place turns an ending into a status, one into text, and they are the same place. Inside the coverage denominator (ADR-013's narrowing leaves only `src/gpu/` out). `window_test.rs` already exists as the sibling unit file. | `mc-render` gains a generic `&dyn Error` walk that has nothing to do with drawing. |
| C | `crates/mc-core/` for the chain walk, constructors in `mc-render` | Layering purity: a chain renderer is a primitive and `mc-core` depends on nothing. | Splits one concept across two crates. The walk would sit where no ending lives and no caller is, and `mc-core` gains an export with one consumer. |

**Decision: B.** The driver is coverage visibility, and it is decisive rather
than aesthetic: option A puts the one thing this spec exists to make observable
into the one crate the gate does not measure. `exit_code`'s own doc comment
already argues exactly this for the sibling decision — *"It lives here, beside
the endings, rather than inside a `main` that no test can call — which is also
what keeps it inside the coverage denominator ADR-013 draws around this crate."*
Reporting is the other half of the same sentence.

**Strongest argument against B:** a crate named `mc-render` now owns a generic
`std::error::Error` walk, which is not rendering in any graphical sense, and a
reader looking for "how a failure is printed" will not guess that crate. The
mitigation is that the walk is not generic *machinery* offered to the workspace —
it is private to the module and reachable only through `Ending`'s constructors,
so what `mc-render` exports is still "an ending, its status and its text".
Option C is the honest alternative if `mc-core` ever grows a diagnostics module
for another reason; it is not worth a two-crate split today.

**Shape.**

```rust
// crates/mc-render/src/window.rs

/// `failure` and every failure beneath it, outermost first, joined with `": "`.
///
/// Depth-general: a content refusal is two layers and a save refusal is three,
/// and "print one more level" is correct for the first and wrong for the second.
/// A layer whose own message spans several lines is rendered whole.
fn rendered(failure: &dyn Error) -> String;

/// Says how the run ended, for every ending that is not the player closing the
/// window, to `sink`.
pub fn report(ending: &Ending, sink: &mut dyn Write) -> io::Result<()>;
```

`report` writes `mycraft: ` and then the rendered text **unmodified** — no
re-indentation of continuation lines. A report may span several lines
(FR-1.1-S4); rewriting them to align under the prefix would make the printed
text differ from the text FR-7's guard compares and from the block a person
copies into a search. One line, BINDING.

`Ending::Frame(reason)` keeps its own sentence and does not go through
`rendered`: `FatalReason` is a plain `Copy` enum with no `Error` impl
(`crates/mc-render/src/surface.rs:129-134`), so there is no failure and no chain
there. `Ending::Startup(failure)` **does** go through `rendered` —
`StartupError` is an `Error` whose two variants carry no `#[source]` today
(`crates/mc-render/src/surface.rs:220-236`), so this changes no character now and
means a source added later is printed rather than dropped. FR-6.1-S3 and S4 are
the two arms.

### Decision 2 — `Ending::Failed` becomes unconstructible outside `mc-render`, and there are exactly three doors · BINDING

Satisfies: FR-4.1-S1 (makes the guarded property true rather than merely
scanned-for), FR-2.1-S1..S6, FR-3.1-S1..S2.

This is the answer to the lead's stated requirement — *"the guard must be able to
state that **every** reported failure goes through the renderer, with no
exemptions"*.

**Options.**

- **A — a text scan alone.** FR-4 as literally written: a guard walks
  `crates/mc-client/src` and reports any site composing a report itself. Buys the
  enumerated verdict the spec asks for. Costs: a scan approximates. It cannot see
  that a value carries a cause, so its criterion is always a spelling, and a new
  spelling escapes it.
- **B — a compile-time door, plus the scan.** Mark the `Failed` variant
  `#[non_exhaustive]` so no other crate can write the struct literal, and give
  `Ending` three constructors that each render through `rendered`. The scan stays
  and guards what the compiler cannot: that nobody re-opens the door, and that no
  site hands a constructor text it built out of a failure itself.
- **C — a newtype `Report` payload.** `Failed { report: Report }` with private
  construction. Equivalent strength to B, larger diff (every read site changes
  shape, not just its pattern), and it invents a type where an attribute suffices.

**Decision: B.** The property was measured rather than assumed — a two-crate
probe confirms `#[non_exhaustive]` on a struct *variant* rejects cross-crate
construction with **E0639** ("cannot create non-exhaustive variant using struct
expression") and requires `..` in cross-crate patterns with **E0638**. So after
this change, `crates/mc-client/src/**` *cannot* build a reported failure by hand;
it is not a rule anybody has to keep. That is what "design so the wiring is what
breaks" asks for, and it is the difference between a guard that reports an
offence and a design in which the offence does not compile.

**Strongest argument against B:** `#[non_exhaustive]` is a coarse instrument
aimed at semver, used here for an intra-workspace invariant, and it imposes a
cost on every future reader of the enum — a pattern that must carry `..` for a
reason that is not "this variant may grow fields". It also makes one *existing
test file* stop compiling (`crates/mc-client/tests/quit_saving.rs:178` matches
`Ending::Failed { report }` and needs `, ..`), which at rigor `high` is an edit
only the test author may make. I judge the trade worth it because the alternative
is that the central property of this spec rests on a scan whose criterion is a
spelling; but a reviewer who disagrees should overturn this one rather than
Decision 1.

**The three doors, and why exactly three.**

```rust
impl Ending {
    /// `failure` and everything beneath it, then `guidance` where the site has
    /// any. A way out is not a cause: it says what to do, not what happened,
    /// so it is said after the whole chain and never inside it.
    pub fn failed(failure: &dyn Error, guidance: &str) -> Self;

    /// A sentence the site knows and the failure does not, then `failure` and
    /// everything beneath it — the same `": "` joiner, one layer up.
    pub fn failed_under(context: &str, failure: &dyn Error) -> Self;

    /// A refusal with nothing beneath it.
    ///
    /// `&'static str` and not `&str`, and that is the load-bearing detail:
    /// a literal cannot be a `format!`, so this door cannot be the one a
    /// hand-composed report walks through.
    pub fn stated(sentence: &'static str) -> Self;
}
```

Call sites after the change — seven `failed_under`, two `stated`, two `failed`:

| Site | Today | After |
|------|-------|-------|
| `mc-client/src/main.rs:43` | `Ending::Failed { report: failure.to_string() }` | `Ending::failed(&failure, &failure.way_out())` |
| `mc-client/src/app.rs:169` | same | `Ending::failed(&failure, &failure.way_out())` |
| `mc-client/src/session.rs:498` | `format!("the world could not be saved to {path}: {refused}")` | `Ending::failed_under(&format!("the world could not be saved to {path}"), &refused)` |
| `mc-client/src/gpu_startup.rs:127` | `format!("the adapter `{named}` … : {cause}")` | `Ending::failed_under(&format!("the adapter `{named}` …"), &cause)` |
| `mc-client/src/events.rs:69, 85, 162, 166, 179` | `failed(&format!("…: {failure}"))` | `Ending::failed_under("…", &failure)` |
| `mc-client/src/events.rs:188, 192` | `failed("the device was already handed to a window")` | `Ending::stated("…")` |

Every one of these is a joiner leaving a format string, exactly as for
`LaunchError::Load`. The local `failed` helper in `events.rs` disappears.

**Note for the reader of the spec:** the spec's Technical Considerations name
four sites. There are **eleven constructions across five files** — `events.rs`
contributes seven through a local helper the spec does not mention, five of which
interpolate a failure. They are in scope under Scope B and are listed above so
nobody reads them as a widening. See Assumptions for the disposition.

### Decision 3 — The `events.rs` sites take a context sentence rather than a new error type · BINDING

Satisfies: FR-4.1-S1 without an exemption.

Forced, not chosen, and worth stating because the obvious design is unavailable.
The five sites at `events.rs:69, 85, 162, 166, 179` each have a sentence the
failure does not know ("no window could be opened") *and* a `winit` failure that
carries it. The idiomatic Rust repair is a typed error per site with `#[source]`.
**Out of Scope forbids it**: "No variant is added or removed."

So the sentence travels as data rather than as a type: `failed_under(context,
failure)` prepends one synthetic layer to the chain. The rendered text is
character-for-character what those sites produce today, plus whatever `winit`
carries beneath — which is the feature. If a later spec lifts the no-new-variants
constraint, these become typed errors and `failed_under` loses five of its seven
callers; that is a two-way door and is not worth pre-empting.

### Decision 4 — The way out is appended after the whole chain, on the same line, with its sentence unchanged · BINDING

Satisfies: FR-5.1-S3.

This is the one question the spec left open, and the lead's recommendation is on
record: *the way-out sentence is guidance rather than a link in the causal chain,
so it prints after the whole chain rather than inside it — on its own line.*

**I adopt the substance and decline the line break.** The reason is a
measurement, not a preference.

**Options.**

- **A — keep the suffix in `Display`.** `PreparationError::Launch` keeps
  `#[error("{0}{way_out}")]` and the renderer stops descending at it. Rejected:
  it needs an exemption for exactly one variant, which is the shape Scope A was
  refused for, and under a full walk the way-out lands at position 1 of an
  N-layer report — *before* the refusal it answers — which FR-5.1-S3 forbids in
  as many words ("SHALL contain it after the refusal it is a way out of").
- **B — move the sentence down onto `LoadError::Unresolvable`.** It would then
  sit at the innermost layer, which is the end of the chain, with no new
  mechanism at all. Rejected for two reasons: `LoadError` lives in `mc-world`,
  and the flag is spelled once in `crates/mc-client/src/startup.rs:41` precisely
  so the parse that accepts it and the message that advertises it cannot
  disagree — a domain error type quoting a client's command-line flag reopens
  that. And it rewrites a refusal sentence in another crate, which Out of Scope
  forbids.
- **C — appended after the chain, same line, sentence byte-for-byte unchanged.**
- **D — appended after the chain, on its own line** (the lead's wording), which
  additionally requires stripping the leading `". "` from `way_out_of`'s string.

**Decision: C.** `PreparationError::Launch` becomes `#[error(transparent)]`;
`way_out_of` stays in `startup.rs`, unchanged, exposed as a
`PreparationError::way_out(&self) -> String` returning the empty string where
there is no way out; `Ending::failed` appends it after the rendered chain.

The measurement that decides C over D: **`LoadError::Unresolvable` carries no
`#[source]`** (`crates/mc-world/src/persistence/error.rs:141-152`). So on the one
path that produces a way out, the chain is exactly two layers — `"{save} could
not be read"` then the `Unresolvable` sentence — which is precisely what today's
`{0}` already renders. Under C the player-visible text is **byte identical to
today**, including the leading `". "`. Under D it is the only text in this whole
spec that changes without a scenario asking it to, and it costs a sentence edit
that Out of Scope's "no rewritten sentences beyond removing the interpolation"
does not licence.

**Strongest argument against C — and it is a real one.** Today the way-out is
welded to the type: any site that reports a `PreparationError::Launch` gets the
sentence for free. After C it is welded to the *call*, so a third construction
site that forgot `&failure.way_out()` would silently drop a player's only way
back into their world, and no scenario would notice — FR-5.1-S3 asserts the
sentence is present once, not that every site supplies it. Three things hold it
instead: there are exactly two `failed` call sites and Decision 2 makes a third
one impossible to add without going through the same door; `way_out()` is a
method on the failure, so the coupling is one call rather than a copy; and the
guidance parameter is not optional, so a site cannot omit it by forgetting —
only by passing something else. If a reviewer wants this closed properly, the
answer is a `Reported` trait on the failure rather than a line break, and that is
a different spec.

**On the line break specifically:** D's appeal is that causes and advice read as
different kinds of thing. That is true, and it is what Decision 4 buys by taking
the sentence out of `Display` entirely. The newline adds nothing to that
separation that the sentence's own wording does not already carry, and it
forfeits byte-identity. If the lead still wants D, it is a one-line change to
`report`'s join and a one-word change to `way_out_of` — flag it and I will bind D
instead.

### Decision 5 — The three `Display` changes, exhaustively · BINDING

Satisfies: FR-5.1-S1, FR-5.1-S2, FR-5.1-S3.

| Site | From | To |
|------|------|-----|
| `crates/mc-sim/src/persistence.rs:52` — `LaunchError::Load` | `#[error("{save} could not be read: {source}")]` | `#[error("{save} could not be read")]`, `#[source]` unchanged |
| `crates/mc-sim/src/persistence.rs:69` — `LaunchError::WorldGen` | `#[error("a new world could not be generated: {0}")]` | `#[error("a new world could not be generated")]`, `#[from]` unchanged |
| `crates/mc-client/src/startup.rs:146` — `PreparationError::Launch` | `#[error("{0}{way_out}", way_out = way_out_of(.0))]` | `#[error(transparent)]`; the way out moves to `way_out()` per Decision 4 |

`LaunchError::Load`'s doc comment states the assumption this spec invalidates —
*"The refusal a turned-away player reads is rendered from `Display` alone —
nothing walks the source chain"* — and must be corrected in the same edit, not
left as a comment describing a world that no longer exists.

**A correction to the spec's prose, resolved rather than escalated.** The spec
and `requirements.md` both claim the text after these two changes is "byte
identical". Measured, that holds only where the cause has no further source. It
does for the save path (`LoadError::Unresolvable`, no source). It does **not** for
`LaunchError::WorldGen` reaching `WorldGenError::UnnamedBlock`
(`crates/mc-sim/src/replay/world.rs:291-295`), which carries a `#[source]
NamespacedIdError` that nothing prints today — so that report gains a layer. That
is the feature working, not a regression, and FR-1.1-S3 demands exactly this
depth. The prose overstates; the scenarios are right.

FR-5.1-S2 is satisfied on the path it names: a block the registry does not
declare surfaces as `SectionError::UnknownBlock` — *"no block is registered under
the name `…`"* — reached through `WorldGenError::Section`, which is
`#[error(transparent)]` and therefore *is* that layer rather than sitting above
it. The block is named once. (`crates/mc-world/src/section/mod.rs:88-91`.)

### Decision 6 — FR-4's guard: two roots' worth of scope, five needles, no exemptions · BINDING

Satisfies: FR-4.1-S1, FR-4.1-S2, FR-4.1-S3.

Shape follows `crates/mc-client/tests/seam_boundaries.rs` — production text with
doc comments stripped, `*_test.rs` skipped, a `tempfile` fixture as the positive
control — with one deliberate departure: it reports an **enumerated verdict**
rather than asserting an absence, per `testing.md` §2 and the spec's own note at
FR-4.

```rust
enum Verdict {
    EveryReportedFailureIsRenderedByTheRenderer,
    ComposedItsOwnReport(Vec<Site>),   // FR-4.1-S2
    NoSourceWasRead,                    // FR-4.1-S3
}
```

- **Root: `crates/mc-client/src`, every production `.rs`, `exempt: |_| false`.**
  Not one entry. This is only possible because Decision 2 leaves nothing in that
  tree with a legitimate reason to turn a failure into text — verified against
  the tree: `.to_string()` occurs at exactly four sites there today
  (`app.rs:170`, `app.rs:190`, `app.rs:217`, `main.rs:44`) and every one of them
  is a failure being flattened.
- **Needles:** `Ending::Failed` (the raw variant spelling), `.to_string()`, and
  the three error-binding interpolations this tree actually uses — `{failure}`,
  `{cause}`, `{refused}`.
- **The positive-control fixture commits every needle**, following the idiom
  `the_same_scan_reports_a_non_core_file_that_advances_the_simulation_wherever_it_sits`
  established for `OUTSIDE_THE_CORE_GUARD`: a needle no fixture ever commits is a
  needle nobody has watched match anything, and a mistyped one reports a clean
  scan forever. The expected hit count is derived from the needle list, not
  written as a literal.

**What this guard does and does not prove, stated rather than implied.** The
first two needles plus `#[non_exhaustive]` carry the invariant: a reported
failure cannot be composed in `mc-client` at all. The last three are a
naming-convention guard over the remaining hole — a site handing
`failed_under` a context it built by interpolating an error under a *differently
named* binding escapes them. That hole is narrow and it is real; it is written
down here rather than papered over, because a guard claiming totality it does not
have is the failure this spec is about, one level up.

**Consequence: `app.rs:190` and `app.rs:217` convert too.** These flatten a
re-mesh failure into the overlay rather than into an ending, so FR-4.1-S1's
*scan criterion* ("into an ending's reported text") does not reach them — but
FR-4.1's *requirement sentence* does ("Every failure the client reports is
rendered by the one renderer"), and so does the `.to_string()` needle. They
become `rendered(&failure)` — which means `rendered` is `pub` in
`mc-render::window` after all, not private as Decision 1 first sketches it.
Exempting them instead would put a hand-maintained exemption list back into the
one guard whose whole point is not having one. Two words each, and a player
reading the overlay gets the cause they do not get today.

### Decision 7 — FR-7's guard recognises a quoted refusal by its own prefix · BINDING

Satisfies: FR-7.1-S1, FR-7.1-S2, FR-7.1-S3.

The guard must find "the refusals quoted in `docs/modding/`" without a
hand-maintained table of page-to-fixture, which would age exactly as an exemption
list does.

**Decision: a quoted refusal is a fenced code block under `docs/modding/` whose
first line begins `mycraft: `.** That is derived from the artefact itself — it is
what `report` writes — so there is no marker convention for an author to keep in
step, and a page that stops quoting a refusal changes the verdict to *no quoted
refusal was found* rather than passing silently.

The guard lives in `crates/mc-client/tests/` (it must run the client's own
preparation over a fixture root), reaches `docs/` via
`CARGO_MANIFEST_DIR/../../docs/modding`, and reports:

```rust
enum Verdict {
    EveryQuotedRefusalIsTheRefusalPrinted,
    Mismatch { quoted: String, produced: String },  // FR-7.1-S2
    NoQuotedRefusalWasFound,                        // FR-7.1-S3
}
```

Each block is matched against the text produced for the fixture declarations the
scenarios already name (`blocks/amber.toml` with `slid`, and the HUD's
`malformed-readout.toml`), so the documentation is compared against a real run
rather than against a second copy of the expected string. `Mismatch` carries both
sides, which is FR-7.1-S2's requirement in the verdict's own shape.

### Trivial decisions — one line each

- The joiner is `": "`, outermost first, on the precedent of
  `crates/mc-testkit/src/frame/golden.rs:445-456` and D2 in `requirements.md`.
- `report` returns `io::Result<()>`; `main` discards it, because a client that
  cannot write to its own stderr has nowhere to say so.
- The `mycraft: ` prefix moves into `report` with the rest of the reporting.
- `crates/mc-client/tests/hud_launch.rs`'s hand-walked `chain` helper is deleted,
  not adapted; its needles become FR-3.1-S1's (`requirements.md`, DO-2).

### Deferred

- **`mc-testkit`'s `describe` stays a second spelling of the walk.** It is
  private to `golden.rs`, serves a test harness rather than a player, and
  replacing it would put `mc-testkit` behind `mc-render::window`'s API for one
  function. *Revisit when:* `mc-testkit` next needs a chain rendered anywhere
  else, or when `rendered`'s grammar changes — at which point two spellings would
  become two answers.

## Interfaces

```rust
// crates/mc-render/src/window.rs

pub enum Ending {
    Closed,
    Startup(StartupError),
    Frame(FatalReason),
    #[non_exhaustive]
    Failed { report: String },
}

impl Ending {
    pub fn failed(failure: &dyn Error, guidance: &str) -> Self;
    pub fn failed_under(context: &str, failure: &dyn Error) -> Self;
    pub fn stated(sentence: &'static str) -> Self;
}

/// `failure` and every failure beneath it, outermost first, joined with `": "`.
pub fn rendered(failure: &dyn Error) -> String;

/// Says how the run ended, for every ending that is not the player closing the
/// window.
pub fn report(ending: &Ending, sink: &mut dyn Write) -> io::Result<()>;

pub const fn exit_code(ending: &Ending) -> u8;   // unchanged, stays in main's hands
```

```rust
// crates/mc-client/src/startup.rs

impl PreparationError {
    /// What to tell a player beyond what this says — the way out where there is
    /// one, and the empty string where there is not.
    pub fn way_out(&self) -> String;
}
```

**Error contracts.** `rendered` and the three constructors are total: they take
`&dyn Error` and cannot fail. `report` fails only for a sink that will not accept
bytes. No new error type is introduced anywhere, and no existing variant is added
or removed.

**Rendering contract, stated precisely because four scenarios assert exact
strings:**

- `rendered(f)` = `Display(f)`, then for each `e` in the `source()` walk from
  `f`, `": "` followed by `Display(e)`.
- A layer's own message is emitted whole, line breaks included (FR-1.1-S4).
- A failure with no source renders as its own message with no trailing separator
  and no empty layer (FR-1.1-S2, FR-2.1-S6).
- `failed_under(c, f)` = `c` + `": "` + `rendered(f)`.
- `failed(f, g)` = `rendered(f)` + `g`, where `g` is empty or begins with its own
  separator (Decision 4).
- `report` writes `"mycraft: "` + the above + `"\n"`, unmodified.

## Data

No entity, no field, no migration, no retention rule. Nothing is persisted and no
data model is touched. The only stored artefact this feature reads is
`docs/modding/*.md`, and only to compare text.

## Integration

| File | What connects | What must not break |
|------|---------------|---------------------|
| `crates/mc-render/src/window.rs` | `rendered`, three constructors, `report`; `#[non_exhaustive]` on `Failed` | `exit_code`'s behaviour and its existing coverage at `window_test.rs:36-47` — it stays in `main`'s hands per Out of Scope |
| `crates/mc-render/src/window_test.rs` | FR-1.1 and FR-6.1 unit tests | — |
| `crates/mc-client/src/main.rs` | `report()` deleted; calls `mc_render::window::report(&ending, &mut io::stderr())`; the `Ending` at :43 goes through `failed` | `run()` and `ExitCode::from(exit_code(..))` stay here (Out of Scope) |
| `crates/mc-client/src/app.rs` | :169 through `failed`; :190 and :217 through `rendered` | the frame path's control flow — only the text changes |
| `crates/mc-client/src/events.rs` | seven sites; the local `failed` helper removed | the ending-wins-first rule in `stop()` |
| `crates/mc-client/src/gpu_startup.rs`, `session.rs` | one site each through `failed_under` | `ending_after_saving`'s rule that a save failure never masks an earlier failure |
| `crates/mc-client/src/startup.rs` | `Launch` transparent; `way_out()` exposed | `LOAD_CHANGED_BLOCKS` stays one constant serving both the parse and the message |
| `crates/mc-sim/src/persistence.rs` | two format strings; `Load`'s doc comment corrected | the `Load`/`Missing` distinction that decides whether a world is generated |
| `crates/mc-client/tests/quit_saving.rs:178` | pattern needs `, ..` | **test file — the test author owns this edit, not the implementer** |
| `crates/mc-client/tests/hud_launch.rs` | superseded; `chain` deleted | nothing — its scenario is re-covered by FR-3.1-S1 with the element name it never checked |
| `docs/modding/README.md`, `blocks-items.md`, `docs/technical/architecture.md`, `testing.md`, `docs/user/gameplay.md` | Key Principle 3's five pages, per the spec's Documentation section | FR-7 guards the two under `docs/modding/` |

## Assumptions

Each is a place a driver was missing and I chose rather than halted. A reviewer
can veto any of them.

1. **`Ending` is not an "error type" for the purposes of Out of Scope.** That
   bullet forbids new *error variants* and changed refusal conditions.
   `#[non_exhaustive]` adds no variant, changes no refusal, and rewrites no
   sentence — but it does change a public enum's construction rules, which is the
   largest single thing in this design. If the lead reads Out of Scope as
   covering it, Decision 2 falls back to option A (the scan alone) and the
   spec's central property becomes weaker than the lead asked for. **This is the
   assumption to check first.**
2. **The seven `events.rs` construction sites are in scope.** The spec names four
   sites and these are not among them, but Scope B says "every reported failure"
   and FR-4's guard cannot be exemption-free while they stand. Their text changes
   only by gaining whatever `winit` carries beneath.
3. **`app.rs:190` and `app.rs:217` are in scope** (Decision 6), on FR-4.1's
   requirement sentence rather than on S1's scan criterion.
4. **`way_out_of`'s sentence keeps its leading `". "`** (Decision 4, option C).
   If the lead binds the line break instead, this reverses.
5. **A `&mut dyn Write` sink is what "a caller-supplied sink" means in D1** —
   not a bespoke trait, not a `Vec<String>` of lines.
6. **Nothing outside `mc-client` and `mc-render` constructs an `Ending`.**
   Verified by grep across `crates/` and `tools/` at `adb24d9`; the only other
   mentions are `mc-render`'s own `exit_code` match and two `mc-client` test
   files.

## Risks

- **`#[non_exhaustive]` breaks a test file the implementer may not edit.**
  `quit_saving.rs:178` stops compiling until `, ..` is added. At rigor `high` the
  test author owns test files, so `/sdd-tasks` must put this edit in the test
  author's phase *before* the implementation phase, or the phase opens with a
  tree that does not compile — and `testing.md` §2 is explicit that a
  non-compiling tree means the gate cannot run, so anything only the gate can see
  accumulates silently across that window. **Verify early**, before any other
  task.
- **The five `events.rs` sites depend on `winit` error types having useful
  `source()` chains.** If they do not, the text is unchanged and nothing is lost;
  if they do, five reports get longer. Either outcome is fine, but a scenario
  asserting an exact string there would be asserting `winit`'s internals. None
  does, and none should.
- **`toml` 0.9.12's caret diagnostic is quoted verbatim in two documentation
  pages.** A minor bump restyles it and FR-7's guard goes red. That is the guard
  working — it is listed here so the redness is diagnosed as a dependency bump
  rather than as a flake. Blast radius: two documentation pages and one test.
- **FR-6.1-S1 asserts the shipped root registers one block per `.toml`.** It
  reads the *product's own* `content/base/`, so adding a block file to the game
  changes the derived count — correctly, since the count is derived by counting
  the files. It does mean a content change can redden a client test; that is the
  intended coupling and the spec says so.
- **The renderer is depth-general and the chain is now walked to the bottom.**
  Any error type in the workspace that interpolates its own source will start
  saying it twice. Three are known and fixed (Decision 5); the spec's exhaustive
  claim was checked against `LaunchError`, `PreparationError`, `LoadError`,
  `SaveError`, `WorldGenError`, `SectionError`, `RegistryError` and
  `StartupError` and holds for those. It is **not** verified for every error type
  in the workspace, because only these reach a reported ending — if a future
  variant does, FR-5's rule is what it must obey.
- **Two layers may name the same subject without either interpolating the
  other.** `WorldGenError::UnnamedBlock` quotes `{text}` and its source
  `NamespacedIdError::MissingNamespace` quotes the same `{text}`
  (`crates/mc-core/src/id/namespaced.rs:14-25`). That is not the FR-5 defect — no
  layer states its own *cause* — but a reader sees the name twice. Recorded as a
  deferred observation rather than fixed: fixing it means rewriting a sentence in
  `mc-core`, which Out of Scope forbids.
