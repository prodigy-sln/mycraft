# Tasks: A content refusal names the file, the block and the field it is about

**Spec**: [`spec.md`](spec.md) (SPEC-015, rigor `high`, 26 scenarios) ·
**Architecture**: [`architecture.md`](architecture.md) ·
**Branch**: `feature/PRO-939-print-refusal-chain` · **Created**: 2026-08-16

One task = one coherent scenario group in one area. Split phases only at real
dependency boundaries. `[P]` = independent of other `[P]` tasks in the same
phase.

Four phases, eleven tasks, 26 scenarios — every scenario in exactly one task.
T08b carries no scenario of its own and is recorded as additional coverage.

| Phase | What it delivers | Scenarios |
|-------|------------------|-----------|
| 1 | One renderer, three doors, and no other way to report | 17 |
| 2 | A cause is said once | 3 |
| 3 | Nothing reports around the renderer, and the shipped binary proves one is | 3 |
| 4 | What a person reads, and the guard that keeps it true | 3 |

## Phase 1: One renderer, three doors, and no other way to report

**Why this is one phase and cannot be smaller.** `#[non_exhaustive]` on
`Ending::Failed` (Decision 2) makes cross-crate construction a compile error
(E0639, measured with a two-crate probe). The attribute and **all eleven**
construction sites in `mc-client` therefore land in one commit or the workspace
does not build. Splitting the client-facing scenarios into a later phase would
also destroy their RED: once Phase 1's implementation exists they are green on
arrival, and a test that never ran red is not evidence.

- [x] T01 **Adaptation commit — the test author's edit, and it lands first.**
      `crates/mc-client/tests/quit_saving.rs:178`: `Ending::Failed { report }`
      → `Ending::Failed { report, .. }`.
      Scenarios: none — this task enables T04 rather than satisfying anything.

      **Ownership.** This is a test file. At rigor `high` the implementation
      context may never edit one, so this edit belongs to the test author and
      must precede the implementation that adds `#[non_exhaustive]`. An
      implementer who finds this pattern unmigrated raises a dispute; they do
      not fix it.

      **The window in which the gate cannot run.** Phase 1's test authoring
      names `rendered`, `report` and the three constructors before they exist,
      so the tree does not compile until T04's implementation lands, and
      `scripts/sdd-gate.ps1` cannot run across that whole window. A green suite
      is no evidence about a lint and here there is no runnable suite at all —
      so **the test author runs
      `cargo clippy --workspace --all-targets --all-features -- -D warnings`
      directly** at the end of the authoring step and again once T04 is green.
      Checking at a lower severity asks a different question, and without
      `-D warnings` cargo attributes the diagnostic to the first binary and
      marks the rest `(1 duplicate)` — which means *this same diagnostic,
      repeated*, not *a pre-existing one lives elsewhere*.

      **Measured, so nobody plans around a hazard that is not there:** `, ..` in
      a cross-crate struct pattern compiles cleanly against an *exhaustive*
      variant, with no warning. So T01 by itself never breaks the tree and can
      land as early as the phase opens. What makes it order-critical is
      ownership, not compilation.

- [x] T02 [P] **The chain renderer** — `rendered(&dyn Error) -> String`:
      `Display` of the failure, then `": "` and `Display` of each failure in the
      `source()` walk, outermost first. Depth-general. A layer whose own message
      spans several lines is emitted whole. A failure with no source renders
      with no trailing separator and no empty layer.
      — `crates/mc-render/src/window.rs`, `crates/mc-render/src/window_test.rs`
      Scenarios: FR-1.1-S1, FR-1.1-S2, FR-1.1-S3, FR-1.1-S4

      FR-1.1-S3's three levels are the point: a two-level chain is rendered
      correctly by an implementation that takes one `source()` hop and stops.
      Assert the whole rendered string, not its parts — that is what rejects
      both the current defect (outermost only) and its mirror (innermost only).

- [x] T03 [P] **What each ending says, and the one that says nothing** —
      `report(&Ending, &mut dyn Write) -> io::Result<()>`. Writes `"mycraft: "`,
      then the text, then `"\n"`, **unmodified** — no re-indentation of
      continuation lines. `Ending::Closed` writes nothing. `Ending::Startup`
      goes through `rendered`; `Ending::Frame` keeps its own sentence, because
      `FatalReason` is a `Copy` enum with no `Error` impl and so has no chain.
      — `crates/mc-render/src/window.rs`, `crates/mc-render/src/window_test.rs`
      Scenarios: FR-6.1-S2, FR-6.1-S3, FR-6.1-S4

- [x] T04 **The three doors, `#[non_exhaustive]`, and every construction site —
      and a refused block declaration as the mod author reads it.**
      `Ending::failed`, `Ending::failed_under`, `Ending::stated`; `rendered` and
      `report` become `pub`; `main.rs`'s reporting is deleted and delegated to
      `mc_render::window::report(&ending, &mut io::stderr())`.
      — `crates/mc-render/src/window.rs`, `crates/mc-client/src/main.rs`,
      `app.rs`, `events.rs`, `session.rs`, `gpu_startup.rs`; new
      `crates/mc-client/tests/` file for the block refusals
      Scenarios: FR-2.1-S1, FR-2.1-S2, FR-2.1-S3, FR-2.1-S4, FR-2.1-S5,
      FR-2.1-S6, FR-6.1-S1
      Depends on: T01, T02, T03

      **Eleven sites across five files, not the four the spec names.**
      `events.rs` contributes seven through a local `failed()` helper the spec
      does not mention — `:69, :85, :162, :166, :179` interpolate a failure and
      become `failed_under`; `:188, :192` become `stated`; the helper at `:224`
      disappears. The rest: `main.rs:43` and `app.rs:169` through `failed`,
      `session.rs:498` and `gpu_startup.rs:127` through `failed_under`. Every
      one is a joiner leaving a format string. These are in scope under Scope B
      and are listed so nobody reads them as a widening.

      **`app.rs:190` and `:217` convert too**, to `rendered(&failure)`. They
      flatten a re-mesh failure into the overlay rather than into an ending, so
      FR-4.1-S1's *scan criterion* does not reach them — but FR-4.1's
      *requirement sentence* does, and exempting them would put a
      hand-maintained exemption list back into the one guard whose entire
      purpose is not having one. This is why `rendered` is `pub`.

      **`failed` takes its `guidance` argument as `""` in this phase.**
      `PreparationError::way_out()` does not exist until Phase 2, and
      `PreparationError::Launch` still appends its way-out sentence through
      `Display` here, so no text is lost. The two call sites gain
      `&failure.way_out()` in T06.

      **FR-2.1-S2's field name may arrive in either slot.** `toml` 0.9.12
      refuses an unrecognised field with a five-line caret diagnostic and
      leaves `DefinitionFault.field` as `None` on that path — the name reaches
      the author inside `cause`. Assert what the author reads, never which slot
      carries it.

      **FR-6.1-S1's count is derived**, by counting the `.toml` files under
      `content/base/blocks/` at test time, never written as a literal. Adding a
      block file to the shipped game changes it correctly.

- [x] T05 **A refused HUD declaration, in the same terms as a refused block
      declaration.** `crates/mc-client/tests/hud_launch.rs`'s hand-walked
      `chain` helper (`:67-75`) is **deleted, not adapted** — it walks
      `source()` itself, asserts against its own walk and never reaches any
      printing, which is the exact defect this spec repairs. Its needles become
      FR-3.1-S1's, plus the element name it never checked.
      — `crates/mc-client/tests/hud_launch.rs` (superseded),
      `crates/mc-client/tests/support/content.rs` (`shipped_with`)
      Scenarios: FR-3.1-S1, FR-3.1-S2, FR-3.1-S3
      Depends on: T04

**Done when:** `rendered`, `report` and the three doors exist in
`mc-render::window`; `Ending::Failed` is `#[non_exhaustive]` and no crate
outside `mc-render` can construct it; all eleven `mc-client` sites and
`app.rs:190`/`:217` go through the new API; `main.rs` holds argument parsing,
`exit_code` and the choice of `stderr` and nothing else; all 17 scenarios have
a passing test that was shown red first; the full gate exits 0.

## Phase 2: A cause is said once

**Why it is a separate phase.** Phase 1's walk is what *creates* the
double-statement — three variants interpolate their own source today and would
say it twice under a full walk. So these scenarios can only go red after Phase
1 lands, and their red is the honest driver for Decision 5's three edits.

- [x] T06 **The three `Display` changes, exhaustively, and `way_out()`.**
      Scenarios: FR-5.1-S1, FR-5.1-S2, FR-5.1-S3

      | Site | From | To |
      |------|------|-----|
      | `crates/mc-sim/src/persistence.rs:52` — `LaunchError::Load` | `#[error("{save} could not be read: {source}")]` | `#[error("{save} could not be read")]`, `#[source]` unchanged |
      | `crates/mc-sim/src/persistence.rs:69` — `LaunchError::WorldGen` | `#[error("a new world could not be generated: {0}")]` | `#[error("a new world could not be generated")]`, `#[from]` unchanged |
      | `crates/mc-client/src/startup.rs:146` — `PreparationError::Launch` | `#[error("{0}{way_out}", way_out = way_out_of(.0))]` | `#[error(transparent)]` |

      These three are the **only** `Display` changes this spec licenses. No
      variant is added or removed and no refusal condition changes.

      **`LaunchError::Load`'s doc comment is corrected in the same edit** — it
      states the assumption this spec invalidates ("*The refusal a turned-away
      player reads is rendered from `Display` alone — nothing walks the source
      chain*") and must not be left describing a world that no longer exists.

      **The way out is appended after the whole chain, on the same line, its
      sentence byte-for-byte unchanged** (Decision 4, option C — bound; the
      lead has accepted the architect's decline of the own-line form and it is
      not to be re-opened). `way_out_of` stays in `startup.rs` unchanged,
      exposed as `PreparationError::way_out(&self) -> String` returning the
      empty string where there is no way out; `Ending::failed` appends it after
      the rendered chain. On the one path that produces a way out,
      `LoadError::Unresolvable` carries no `#[source]`, so the chain is exactly
      two layers and the player-visible text is byte identical to today,
      leading `". "` included. T04's two `failed` call sites gain
      `&failure.way_out()` here.

      **"Byte identical" is true for one variant and false for another.** The
      `WorldGen` path reaching `WorldGenError::UnnamedBlock` *gains* a layer: it
      carries a `#[source] NamespacedIdError` that nothing prints today. That is
      the feature working, and FR-1.1-S3 demands exactly this depth. The spec's
      prose overstates; the scenarios are right.

- [x] T07 **The two `refusal()` helpers stop reading a `Display` and start
      reading what a player is shown** — the second adaptation, and it is the
      test author's edit.
      — `crates/mc-client/tests/support/persistence.rs:363-368`,
      `crates/mc-client/tests/launch_builds_only_the_world_it_needs.rs:257-262`
      Scenarios: none — it is what keeps an existing test asserting what it
      claims to.
      Depends on: T06

      **Found against the tree; the architecture does not name it.** Both
      helpers do `turned_away.to_string()` on a `PreparationError`. Once
      `Launch` is `#[error(transparent)]` and the way-out moves to `way_out()`,
      that string no longer contains `--load-changed-blocks`, so
      `launch_acceptance.rs:104`'s `told.contains(LOAD_CHANGED_BLOCKS)` goes
      **false** and an existing, correct test reddens. The fix is not to loosen
      the assertion: it is to render through the shipped path —
      `mc_render::window::rendered(&turned_away)` followed by
      `turned_away.way_out()` — which makes that test assert the text a player
      actually reads instead of a `Display` nobody prints. That is a
      strengthening this spec pays for, and it is a **second witness** on the
      way-out sentence reaching a real reader.

      `launch_acceptance.rs:68` keeps spelling `--load-changed-blocks` out
      literally rather than reading the client's constant, for the reason its
      own comment gives.

**Done when:** the three `Display` strings and the `Load` doc comment are
changed and nothing else is; `way_out()` exists and both `failed` sites pass
it; the save path names its reason once and the way-out sentence appears once,
after the refusal it answers; the two helpers render through the shipped
renderer; the full gate exits 0.

## Phase 3: Nothing reports around the renderer

**Why after Phases 1 and 2.** FR-4.1-S1's clean verdict is only true once every
site has converted. Run earlier it is red for a known reason, and red for a
known reason is how a test stops reporting anything new.

- [x] T08 **The scan, its positive control, and its "I could not look" verdict.**
      An **enumerated verdict**, not an absence assertion:
      `EveryReportedFailureIsRenderedByTheRenderer` /
      `ComposedItsOwnReport(Vec<Site>)` / `NoSourceWasRead`. Shape follows
      `crates/mc-client/tests/seam_boundaries.rs` — production text with doc
      comments stripped, `*_test.rs` skipped, a `tempfile` fixture as the
      positive control.
      — new `crates/mc-client/tests/` guard beside `seam_boundaries.rs`
      Scenarios: FR-4.1-S1, FR-4.1-S2, FR-4.1-S3

      **Root: `crates/mc-client/src`, every production `.rs`,
      `exempt: |_| false`. Not one entry.** This is only possible because Phase
      1 leaves nothing in that tree with a legitimate reason to turn a failure
      into text.

      **Needles:** `Ending::Failed` (the raw variant spelling), `.to_string()`,
      and the three error-binding interpolations this tree actually uses —
      `{failure}`, `{cause}`, `{refused}`. The positive-control fixture commits
      **every** needle: a needle no fixture ever commits is a needle nobody has
      watched match anything, and a mistyped one reports a clean scan forever.
      The expected hit count is derived from the needle list, never a literal.

      **What this guard does and does not prove — keep this honesty in the
      code, do not let it be dropped.** The first two needles plus
      `#[non_exhaustive]` carry the invariant: a reported failure cannot be
      composed in `mc-client` at all. **The last three are a naming-convention
      guard over a narrow residual hole** — a site handing `failed_under` a
      context it built by interpolating an error under a *differently named*
      binding escapes them. That hole is narrow and it is real. A guard
      claiming totality it does not have is the failure this spec is about, one
      level up.

- [x] T08b **The shipped binary, run as a real subprocess.**
      — new `crates/mc-client/tests/` file, following
      `tools/voxforge/tests/binary.rs`
      Scenarios: none of its own — additional coverage, recorded in
      `test-map.md` with one line on what it catches.

      **This is T08's other half, and the pairing is the insight rather than
      the test.** T08's scan proves nothing *composes* a report; this proves a
      report is *reached and printed*. Same requirement from opposite sides,
      and neither alone is enough: a `main` that dropped its `report` call
      entirely, or wrote to the wrong stream, passes the scan and leaves the
      whole suite green. That is the shape `tools/voxforge/tests/binary.rs`
      exists for — its own header records a `main` gutted to ignore its library
      with 123 of 125 tests still green.

      **It needs no device and no display server**, which is what makes it
      cheap. `run()` calls `startup::content_root()` first and returns on `Err`
      *before* `launch::spawn_preparation` and before `gpu_startup::open()`, so
      a binary started in a directory with no content root refuses without
      touching the GPU. Measured by running it, not reasoned: the process exits
      non-zero and writes one line to standard error.

      `CARGO_BIN_EXE_mc-client` locates the executable, so finding it is
      Cargo's problem rather than a harness's.

      **What it asserts**, all derived rather than snapshotted:
      - standard error is `"mycraft: "`, the rendered `NoContentRoot` refusal,
        and a line break — the expectation built by constructing the same typed
        refusal and rendering it, never by pasting an observed string. The
        looked-for path is `["content", "base"]` collected, so it spells itself
        per platform.
      - the exit status is non-zero.
      - the rendered refusal does **not** appear on standard output. **Not**
        that standard output equals the palette notice: that would couple a
        test about refusal reporting to the wording of an unrelated message, so
        rewording the placeholder-texture notice would redden it and teach
        whoever fixed it that this test is noisy rather than meaningful.

      **What it does not close, and this must not be over-read.** It does not
      witness the guidance supply. This path's refusal is `NoContentRoot`,
      whose way-out is empty by construction — the one production line that can
      emit the way-out sentence is `app.rs`'s, and it needs a device and a
      window. That line stays uncovered and stays labelled. A test that closes
      a real hole is exactly when somebody is most tempted to read it as
      closing the adjacent one.

**Done when:** the scan reports the clean verdict over the real tree, reports
the fixture's site over the positive control, and reports `NoSourceWasRead` for
a root that does not exist; the limits above are written down where the guard
lives; the shipped binary is run as a subprocess and its refusal, stream and
status are asserted; the full gate exits 0.

## Phase 4: What a person reads, and the guard that keeps it true

**Documentation is part of the definition of done** (Key Principle 3), and this
phase is TDD-shaped rather than a postscript: the guard in T09 is **red against
the tree as it stands today**, because `docs/modding/README.md` documents a
refusal the program does not produce. T10 is what turns it green.

- [x] T09 **The documentation-drift guard.** A quoted refusal is *a fenced code
      block under `docs/modding/` whose first line begins `mycraft: `* — derived
      from the artefact itself, so there is no marker convention for an author
      to keep in step. Reaches `docs/` via `CARGO_MANIFEST_DIR/../../docs/modding`.
      Verdicts: `EveryQuotedRefusalIsTheRefusalPrinted` /
      `Mismatch { quoted, produced }` / `NoQuotedRefusalWasFound`. Each block is
      compared against text produced by a **real run** over the fixture
      declarations the scenarios already name (`blocks/amber.toml` with `slid`,
      and `hud/malformed-readout.toml`), never against a second copy of the
      expected string.
      — new `crates/mc-client/tests/` guard
      Scenarios: FR-7.1-S1, FR-7.1-S2, FR-7.1-S3

- [x] T10 **The five pages, written from captured text.** Every quoted refusal
      is **captured from a real run and pasted**, never composed by hand.
      — `docs/modding/README.md`, `docs/modding/blocks-items.md`,
      `docs/technical/architecture.md`, `docs/technical/testing.md`,
      `docs/user/gameplay.md`
      Scenarios: none of its own — FR-7.1 guards the two `docs/modding/` pages.
      Depends on: T09

      - **Mod author — `docs/modding/README.md`.** Rewrite "When you get it
        wrong". Remove the paragraph beginning "What that line says today is
        less than the engine knows" (`:130-136`) and the "change one file at a
        time" advice; replace with the refusal a person now reads, quoted whole
        as a block, for a named declaration. **Correct `:125-126` in the same
        pass:** "The client exits without opening a window" is true of a missing
        content root and **false** of a refused declaration, which is collected
        at the first redraw, after the window opens. Verified against the file.
      - **Mod author — `docs/modding/blocks-items.md`.** "All-or-nothing
        loading" (`:122-125`) states the refusal contract and never says what
        reaches the terminal. It gains that, with the same quoted refusal, so
        the page that makes the promise is the page that shows it kept.
      - **Engine reader — `docs/technical/architecture.md`.** The reporting
        seam: where a failure is rendered, why there rather than in the binary
        (coverage visibility — ADR-013 excludes `mc-client` wholesale), what
        Phase 3's scan forbids, and the rule Phase 2 establishes that a message
        never states its own cause.
      - **Engine reader — `docs/technical/testing.md`.** Why a printing path
        needs a scan as well as an assertion; why three chain levels are the
        minimum that separates a full walk from a single hop; the shape of the
        documentation-drift guard.
      - **Player — `docs/user/gameplay.md`.** One paragraph: a player handed a
        broken content root now reads what is wrong with it instead of one
        generic sentence.

      Three of these five pages carry no scenario. That is normal for
      documentation and is stated here so nobody reads it as an omission.

**Done when:** all five pages are corrected from captured text, the guard
reports the clean verdict, and the full gate exits 0.

## Mechanisms with no scenario behind them

Recorded rather than left silent, per this project's practice of shipping known
holes labelled.

| Mechanism | Why no scenario | Where it gets its test |
|-----------|-----------------|------------------------|
| `app.rs:190`, `:217` → `rendered` into the overlay | No scenario asserts the overlay's text; FR-4.1-S1's *scan criterion* stops at an ending's reported text | **Structural only** — FR-4.1-S1's `.to_string()` needle. Nothing asserts the new overlay text, and that is a real gap, accepted because asserting it needs a GPU and a display server |
| The five `events.rs` sites' new text | An exact-string assertion there would be asserting `winit`'s internals; if `winit` carries no `source()` the text is unchanged and nothing is lost | **Structural only** — FR-4.1-S1's needles |
| `Ending::stated`'s two sites (`events.rs:188, :192`) | Nothing beneath them to render; the `&'static str` signature is the guard (a literal cannot be a `format!`) | The type system, plus FR-4.1-S1 |
| `exit_code` | Deliberately unmoved (Out of Scope) — a scenario would re-prove it through the same code path, which `testing.md` §1 calls worse than no test | Existing `crates/mc-render/src/window_test.rs:36-47` |
| Every `failed` site supplying `way_out()` | FR-5.1-S3 asserts the sentence is present once, **not** that every site supplies it. A third site forgetting it would silently drop a player's only way back into their world | **Nothing asserts it.** Held instead by there being exactly two `failed` sites, by Decision 2 making a third impossible to add outside the door, and by `guidance` being non-optional. The proper close is a `Reported` trait on the failure; that is a different spec |
| The FR-4 guard's last three needles | A naming-convention guard, not an invariant | Written down at the guard, per T08 |
| `docs/technical/architecture.md`, `testing.md`, `docs/user/gameplay.md` | Prose with no behaviour to assert | Reviewed, not guarded — normal for documentation |

## Notes

- **Falsification.** Phase 1's `rendered` and Phase 2's three `Display` changes
  are consequential passes: break each by hand, observe the suite, revert **by
  hand** (never `git checkout -- <file>`), and confirm `git diff --exit-code` is
  clean. Record the outcome either way, including mutations that did not bite —
  a mutation that does not bite is evidence about the code's structure, not
  automatically a test gap.
- **Staging.** Explicit paths only. Never `git add -A`, never `git add .`, never
  `git commit -a`, never `cargo fmt`.
- **Commits.** `test:` before `feat:` per task, never mixed. Scenario IDs go in
  commit messages on this branch and never in code or test names.
- **Two proptest-seed rules if any seed file appears:** delete seeds written by a
  deliberate mutation, commit seeds written by a genuine failure.
- **A `toml` 0.9.12 bump reddens Phase 4's guard**, because its caret diagnostic
  is quoted verbatim in two pages. That is the guard working — diagnose it as a
  dependency bump, not a flake. Blast radius: two documentation pages and one
  test.
- **Deferred observation, not fixed here.** `WorldGenError::UnnamedBlock` quotes
  `{text}` and its source `NamespacedIdError::MissingNamespace` quotes the same
  `{text}`, so a reader sees the name twice. That is **not** the FR-5 defect — no
  layer states its own *cause* — and fixing it means rewriting a sentence in
  `mc-core`, which Out of Scope forbids.
- **Deferred, from the architecture.** `mc-testkit`'s private `describe` stays a
  second spelling of the walk. Revisit when `mc-testkit` next needs a chain
  rendered elsewhere, or when `rendered`'s grammar changes — at which point two
  spellings become two answers.
