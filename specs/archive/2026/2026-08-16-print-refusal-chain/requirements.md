# Requirements gathering — PRO-939

Source: Linear PRO-939, "A content refusal tells the author nothing: the source
chain is never printed". Raised by the conductor at the close of PRO-916.

## Verification of the issue's diagnosis against the tree

Every claim in the issue was checked against `main` at `87b05ed`. Four survived,
three did not, and one important fact was missing.

### Survived

1. **A mod author who mistypes a field sees only `mycraft: the shipped content
   could not be read`.** `PreparationError::Content` declares exactly that
   sentence (`crates/mc-client/src/startup.rs:101`), and nothing under it is
   printed.
2. **The file, the block and the field are all constructed into the error
   value.** `DefinitionFault` (`crates/mc-core/src/block/source.rs:40-58`)
   carries `origin`, `block`, `field` and `cause`, and its `Display` renders all
   four. `TomlFileDefinitionSource` fills it in both refusal shapes
   (`crates/mc-world/src/content/toml_source.rs:92-101`,
   `crates/mc-world/src/content/raw.rs:86-91`).
3. **`crates/mc-client/tests/hud_launch.rs` walks `source()` by hand** (lines
   67-75) and asserts against that hand-built chain, with a doc comment saying
   the scenario is that *a person running the client* is told it. It calls
   `prepare_scene` — an error value — and never reaches any printing. The test
   passes; the behaviour it describes does not happen.
4. **`docs/modding/README.md` documents a bisect as the workaround** (lines
   130-136), and `docs/INDEX.md` already registers that page as covering "how
   much of a refusal actually reaches the terminal".

### Did not survive

5. **"`crates/mc-client/src/main.rs:74` prints `eprintln!("mycraft: {failure}")`
   — Display only, with no walk of `source()`."** Line 74 prints
   `{report}`, and `report` is a `String`. Line 70 prints `{failure}`, but that
   is `Ending::Startup(StartupError)`, a different path — `StartupError`
   (`crates/mc-render/src/surface.rs:221-236`) has no source and loses nothing.
   **The chain is destroyed one layer earlier than the issue says**, at
   `crates/mc-client/src/app.rs:170` (`report: failure.to_string()`), where the
   typed `PreparationError` becomes an `Ending::Failed { report: String }`.
   `crates/mc-client/src/main.rs:44` does the same for the missing-content-root
   case. This matters: a `source()` walk added at `main.rs:74` would walk a
   `String` and change nothing. The fix has to reach the two conversion sites.
6. **"The content variant's doc comment reasons that the refusal underneath
   already names all three."** That doc comment is on `PreparationError::Hud`
   (`startup.rs:103-117`), not on `Content`, which carries no doc comment at all.
   The reasoning quoted is real and is real about the HUD variant.
7. **"both [docs] currently describe what a person sees when they get it wrong,
   and the honest bisect workaround" (from the task brief).** Only
   `docs/modding/README.md` does. `docs/modding/blocks-items.md:122-125` states
   the refusal contract — "Every failure names its **origin**: the file it came
   from, the block name it was declared under ... and the specific field at
   fault" — and says nothing about what is printed. It is the page that makes
   the promise; it is not the page that admits the promise is unkept. Both still
   need correcting, for different reasons.

### Missing from the diagnosis, and material

8. **A refused block or HUD declaration does not stop the launch before the
   window opens.** `main::run` spawns the preparation on a worker, opens a
   device, and the event loop creates a window; the preparation is collected at
   the **first redraw** (`app.rs:168`, reached from `events.rs:202-210`). So a
   mod author with a bad file sees a window open and close again. That makes
   `docs/modding/README.md:126` — "The client exits without opening a window" —
   false for exactly the refusals this spec is about. It is true only of
   `NoContentRoot`, which is decided before the device is opened.
   Changing that ordering is **out of scope**; correcting the sentence in the
   paragraph this spec is rewriting anyway is not.
9. **The chain for a content refusal is exactly two layers deep, not three.**
   `RegistryError::Source` is `#[error(transparent)]`, so it forwards both its
   `Display` and its `source()`; `DefinitionSourceError::Malformed` renders
   `DefinitionFault` and has no source of its own, because `DefinitionFault` is
   a plain struct with a `Display` and not an `Error`. A renderer must therefore
   be depth-general rather than "print one more level".
10. **`hud_launch.rs`'s hand-built chain asserts two needles, not three** — the
    file and the field. Its own doc comment says "told both". The module header
    says "naming the file, the element and the field". The element name is never
    checked.

## Decisions taken from the repository rather than asked

**D1 — Where the reporting lives.** `tools/voxforge/src/main.rs` is three lines,
and its doc comment states the rule verbatim: "every decision — argument
parsing, dispatch, rendered text, exit-code selection — lives in the library
where a test can reach it. A binary carrying any of that would earn the coverage
exclusion the binary crates have, and with it the blindness that exclusion
brings." `crates/mc-client/src/main.rs` carries `run()` and `report()` and is
therefore the blindness that comment predicts. The reporting moves into the
library, written to a caller-supplied sink, on the precedent already set in this
workspace. Binding the exact shape is `/sdd-architect`'s job.

**D2 — The rendered grammar.** Two existing renderers agree:
`crates/mc-testkit/src/frame/golden.rs:445-456` (`describe`) and
`crates/mc-client/tests/hud_launch.rs:67-75` (`chain`) both join a failure and
its causes with `": "`, outermost first. A third spelling would be a third
answer; the shipped renderer adopts that grammar.

**D3 — Asserting through the printing path.** A subprocess test on the real
binary was considered and rejected for the content path: the refusal is only
collected after a device is opened and a window created (finding 8), so such a
test would need a GPU and a display server to observe the one thing it is for.
What is available without either is (a) driving the library's reporting with a
failure produced by the client's *own* preparation of a real malformed content
root, and (b) a text guard in the established `tests/seam_boundaries.rs` /
`tests/winit_boundary.rs` idiom — with its positive control — that no site may
render a failure into a reported ending by itself. (a) alone is agreement
between two copies of one decision, which `testing.md` §2 names explicitly;
(b) is what makes a program that stops reporting go red.

**D4 — Scope.** Reporting only. The error types are not redesigned, what is
refused and when is unchanged, and the launch ordering in finding 8 is left
alone. Adjacent findings are recorded as deferred observations below.

## Deferred observations

- **DO-1.** A refused block or HUD declaration opens a window before it refuses
  (finding 8). A player-visible flash of a window that closes itself. Fixing it
  means joining the preparation before opening the device, which is exactly the
  frozen-desktop trade `startup.rs`'s module header rejects on purpose — so it
  needs a design, not a patch.
- **DO-2.** `hud_launch.rs` never asserts the element name (finding 10) even
  though its own module header claims that scenario. This spec supersedes that
  test's role, so the gap closes here rather than being carried.
- **DO-3 — closed, not deferred.** `crates/mc-client/src/session.rs:498-503` and
  `crates/mc-client/src/gpu_startup.rs:127-133` compose failure text by hand
  with `format!` into an `Ending::Failed`. The first draft of this spec proposed
  a guard written to tolerate them. The scenario audit was right that "about
  rendering a failure rather than about the string literal" does not separate
  them from a violation, because both *do* render a failure. They are therefore
  converted to the renderer rather than exempted (FR-4), on the grounds that a
  guard with a hand-maintained exemption list is how the original defect
  survived.

## Findings made after the first draft

11. **A layer's message can be several lines.** Probed `toml` 0.9.12 directly: a
    declaration carrying an unrecognised field renders as a five-line caret
    diagnostic — `TOML parse error at line 4, column 1`, a rule line, the
    offending source line, a `^^^^` marker, then `unknown field \`slid\`,
    expected one of ...`. So the report is a block rather than a line, the docs
    quote a block, and `DefinitionFault.field` is `None` on that path —
    `crates/mc-world/tests/content_loading.rs:281-289` already hedges between
    `.field` and `.cause` for exactly this reason. Raised by the scenario audit,
    confirmed independently.

12. **Three variants state their own cause and would say it twice under a chain
    walk.** `LaunchError::Load` (`crates/mc-sim/src/persistence.rs:52`),
    `LaunchError::WorldGen` (`:69`) and `PreparationError::Launch`
    (`crates/mc-client/src/startup.rs:146`) each interpolate their source into
    their own `Display` *and* expose it as `source()`. `LaunchError::Load`'s doc
    comment states the assumption out loud — "The refusal a turned-away player
    reads is rendered from `Display` alone — nothing walks the source chain" —
    and this spec invalidates it. All three reach the terminal through the same
    `app.rs:170` site as a content refusal, so a renderer installed there cannot
    avoid them. For `Load` and `WorldGen` the repair is byte-identical (both
    already join with `": "`, the renderer's own joiner); `PreparationError::Launch`
    wraps its cause in a *suffix* and needs design. This is the whole of Q1.

13. **`DefinitionFault`'s `block` and `field` are `Option<String>`**
    (`crates/mc-core/src/block/source.rs:40-45`), so the file/block/field triple
    degenerates for a duplicate name, an empty root and an unreadable file —
    three of the four ways `docs/modding/README.md` says an author gets it
    wrong. FR-2.1-S3 and FR-2.1-S4 exist because of this. Raised by the scenario
    audit, confirmed independently.

14. **`exit_code` is already asserted** at
    `crates/mc-render/src/window_test.rs:36-47`, through the same code path a
    scenario about exit status would use. The first draft's FR-5.1-S2 re-proved
    it and was deleted rather than rewritten.

## Decisions taken by the team lead

**Q1 — Scope A or Scope B? → Scope B.** Every reported failure renders through
one renderer, and the three variants in finding 12 stop interpolating their own
source. The instruction that this was "a reporting defect, do not widen it" is
amended, on the grounds that Scope A cannot state FR-4's guard: a renderer with
a hand-maintained exemption list does not go red when a path stops reporting, it
goes green with one more entry in the list. The boundary the original
instruction was aimed at — no error-type redesign — holds exactly: no new
variants, no changed refusal conditions, no rewritten sentences, and two of the
three changes are byte-identical for a reader.

**The suffix in `PreparationError::Launch` goes to `/sdd-architect`**, with the
lead's recommendation on record: the way-out sentence is guidance rather than a
link in the causal chain, so it prints after the whole chain rather than inside
it. The architect may overturn it with a reason.

**Q2 — is the report one line? → No; an earlier ruling of one line was
withdrawn.** The lead first ruled one line, on the reasoning that a greppable
single line survives being pasted into a search or a message. Finding 11's probe
falsified the premise: the five-line caret diagnostic comes from `toml` rather
than from a choice available to us, and it is better for the author than
anything a joined line could carry. What survives the withdrawal is the
outermost-to-innermost order and the `": "` joiner; what dies is "no embedded
newline". The folding scenario therefore asserts **three** chain levels rendered
in full — not "several", because a two-level chain is rendered correctly by an
implementation that takes one `source()` hop and stops.

**Q3 — `docs/modding/README.md:126`.** Corrected as part of this spec. The
launch ordering behind it stays out of scope and stays recorded as DO-1.

## Scenario audit — disposition

`sdd-scenario-auditor` reviewed the 15 scenarios of the first draft and returned
**GAPS FOUND**: 14 gaps and 5 contradictions. Every one was verified against the
tree before being applied. Dispositions:

- **Applied as new scenarios (10).** The duplicate-name, unparseable-file and
  empty-root refusal shapes on the block side (FR-2.1-S3/S5/S4) and the
  unparseable-file shape on the HUD side (FR-3.1-S2); the two uncovered `report`
  arms, `Ending::Startup` and `Ending::Frame` (FR-6.1-S3/S4); the two vacuity
  verdicts (FR-4.1-S3, FR-7.1-S3); the multi-line folding scenario (FR-1.1-S4);
  and the widening of FR-7.1-S1 from one page to every page under
  `docs/modding/`.
- **Applied as amendments (5).** FR-1.1-S1/S2/S3 gained concrete renderings and
  exact-equality assertions, replacing placeholders that would have had the test
  author snapshot expected strings from a run of the code under test.
  FR-2.1-S2's "the name that declaration gives itself" became `example:amber`.
  FR-2.1's requirement sentence was softened to mirror the two qualifications
  `docs/modding/blocks-items.md:122-125` already makes, since `DefinitionFault`'s
  `block` and `field` are `Option<String>` and three refusal shapes degenerate.
  FR-6.1-S1's oracle is now derived by counting `.toml` files rather than
  compared to an unstated baseline.
- **Applied as a rewrite (1).** FR-4.1-S1's criterion is now "renders a failure
  **carrying an underlying cause**", which is a property of the value rather than
  a list of exempt paths. The auditor offered a path-exemption alternative and
  recommended against it; so did DO-3's revision.
- **Declined, with reason (2).** The auditor's Gap 7 and Gap 8 propose
  exit-status scenarios reached through "the new reporting entry point". There is
  no such entry point: Out of Scope keeps exit-status selection in `main.rs`, so
  the only path to it is the one `crates/mc-render/src/window_test.rs:36-47`
  already covers, and a scenario there would re-prove it through the same code
  path. The lead endorsed the deletion independently. Recorded in the spec so a
  later reviewer does not re-raise it.
- **Noted, no change (1).** FR-3.1-S3 ("a root declaring no HUD is valid") is
  true on `main` today and would pass with this feature unimplemented. Kept as a
  regression guard, but it does not discharge scenario guideline 4 for FR-3 —
  FR-3.1-S2 does.
- **Contradiction 5** — DO-3 named "the guard in FR-3" where the guard is FR-4 —
  was already fixed in this file's revision before the transcription arrived.

The auditor judged FR-4 and FR-7 well-founded rather than over-reach: FR-4 is the
only instrument that can see the defect class, since a purely behavioural
replacement for `hud_launch.rs` reproduces the exact failure mode being repaired,
and FR-7 guards drift that has already happened once. It identified the real
over-reach elsewhere — an exit-status scenario duplicating an existing test, and
a scenario half of which ranged over an empty set. Both were removed.

## Open questions

None.
