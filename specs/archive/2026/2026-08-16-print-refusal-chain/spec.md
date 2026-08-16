---
id: SPEC-015
title: A content refusal names the file, the block and the field it is about
status: implemented
rigor: high
branch: feature/PRO-939-print-refusal-chain
issue: PRO-939
created: 2026-08-16
updated: 2026-08-16
completed: 2026-08-16
author: Sebastian Grunow
---

# Specification: A content refusal names the file, the block and the field it is about

## Goal

A mod author who mistypes one field in a block declaration is told which file,
which block and which field — in the text the client actually prints, not in an
error value nothing prints. Today they are told `mycraft: the shipped content
could not be read` and nothing else, and the documented way forward is to change
one file at a time until the launch works, which is a bisect rather than a
diagnostic.

## Stakeholder capability (Key Principle 7)

**Stakeholder: the mod author.**

**What they can now do:** read what is wrong with their content file — its path,
the block or element it declares, and the field at fault — from what the client
prints when it refuses, and fix it in one edit. No bisect, no reading Rust, no
rebuilding with a patch to see more.

They reach it by running `cargo run -p mc-client` exactly as
`docs/modding/README.md` already tells them to. The capability is the text on
their terminal.

## User Stories

- As a mod author, I want the client's refusal to name the file it could not
  read, so that I know which of my files to open.
- As a mod author, I want it to name the block and the field, so that I know what
  to change in that file rather than re-reading all of it.
- As a mod author, I want the same to be true of a HUD declaration, so that the
  two kinds of content I can write behave the same way when I get them wrong.
- As a player, I want a save the game will not load to tell me why once rather
  than twice, so that the way out it offers me is easy to find in it.
- As a mod author, I want `docs/modding/` to describe the message I actually get,
  so that the page I am reading and the terminal I am looking at agree.

## Functional Requirements

Four scenarios below — FR-4.1-S1, FR-4.1-S2, FR-7.1-S1 and FR-7.1-S2 — name a
scan, a source tree, a renderer and a documentation page, which is a departure
from scenario guideline 8. **The departure is deliberate and it is the point of
those two requirements.** The defect being repaired is that a behavioural test
passes while the behaviour it describes never happens; a purely behavioural
replacement reproduces that failure mode exactly. `testing.md` §2 ("Policy is
not wiring") and this workspace's existing guards in
`crates/mc-client/tests/seam_boundaries.rs` and
`crates/mc-client/tests/winit_boundary.rs` are the precedent. Each of the four
reports an enumerated verdict rather than asserting an absence, so a scan that
can no longer look is a distinct answer rather than a clean one.

### FR-1 — Rendering a failure and everything beneath it

- **FR-1.1**: A failure is rendered together with every failure beneath it,
  outermost first, layers separated by `: `.
  - FR-1.1-S1: WHEN a failure whose own message is `the shipped content could
    not be read` and whose single cause's message is `a namespaced id is written
    namespace:path` is rendered, THE SYSTEM SHALL produce exactly `the shipped
    content could not be read: a namespaced id is written namespace:path`.
  - FR-1.1-S2: IF a failure whose own message is `no content was found at
    content/base` carries no underlying cause, THEN THE SYSTEM SHALL produce
    exactly `no content was found at content/base`, with no separator appended
    and no empty layer.
  - FR-1.1-S3: WHEN a failure whose own message is `the save could not be read`,
    whose cause's message is `chunk 4 is truncated` and whose cause's own cause's
    message is `expected 512 bytes, found 128` is rendered, THE SYSTEM SHALL
    produce exactly `the save could not be read: chunk 4 is truncated: expected
    512 bytes, found 128`.
  - FR-1.1-S4: IF a layer's own message spans several lines, THEN THE SYSTEM
    SHALL render it whole, keeping its line breaks, and SHALL render the layers
    before and after it unchanged.

  FR-1.1-S3 states **three** levels rather than "several" deliberately: a
  two-level chain is rendered correctly by an implementation that takes one
  `source()` hop and stops, so only three levels can tell a full walk from a
  single hop. Asserting the whole rendered string rather than its parts is what
  rejects both the current defect (the outermost message alone) and its mirror
  (the innermost alone, losing which stage failed).

### FR-2 — A refused block declaration reaches the mod author

- **FR-2.1**: What the client writes when a block declaration is refused names
  the declaration's file; the block it was declared under, when the file could be
  read far enough to find one; and the field at fault, when the refusal is about
  one field rather than the file as a whole. Where a part is absent the text says
  nothing in its place.
  - FR-2.1-S1: WHEN the client reports a run refused by a content root whose
    `blocks/amber.toml` declares `name = "example:amber:top"`, THE SYSTEM SHALL
    write to its error sink text containing `blocks/amber.toml`,
    `example:amber:top` and `name`.
  - FR-2.1-S2: IF that declaration instead carries the unrecognised field `slid`
    alongside three well-formed ones, THEN the written text SHALL contain
    `blocks/amber.toml`, `example:amber` and `slid`.
  - FR-2.1-S3: IF `blocks/amber.toml` and `blocks/amber-copy.toml` both declare
    `name = "example:amber"`, THEN the written text SHALL contain both file names
    and `example:amber`, and SHALL name no field.
  - FR-2.1-S4: IF a content root's `blocks/` directory declares no block at all,
    THEN the written text SHALL name that root and SHALL name no block and no
    field.
  - FR-2.1-S5: IF `blocks/amber.toml` holds only the line `this is not toml at
    all`, THEN the written text SHALL contain `blocks/amber.toml` and the
    parser's own reason, and SHALL name no block and no field.
  - FR-2.1-S6: IF the run was refused because no content root was found at all,
    THEN the written text SHALL name the directory that was looked for and SHALL
    end with it rather than with a separator and an empty layer.

### FR-3 — A refused HUD declaration reaches the content author

- **FR-3.1**: A refused HUD declaration is written out in the same terms as a
  refused block declaration.
  - FR-3.1-S1: WHEN the client reports a run refused by a content root whose
    `hud/malformed-readout.toml` declares `size = [0, 4]`, THE SYSTEM SHALL write
    text containing `malformed-readout.toml`, `example:malformed-readout` and
    `size`.
  - FR-3.1-S2: IF `hud/malformed-readout.toml` holds only the line `this is not
    toml`, THEN the written text SHALL contain `malformed-readout.toml` and the
    parser's reason, and SHALL name no element and no field.
  - FR-3.1-S3: IF a content root declares no HUD elements at all, THEN THE SYSTEM
    SHALL write no refusal about the HUD, because a root declaring no HUD is a
    valid root.

### FR-4 — No failure is reported except through the renderer

- **FR-4.1**: Every failure the client reports is rendered by the one renderer,
  so that a client which stopped rendering causes could not stay green.
  - FR-4.1-S1: WHEN the client's own sources are scanned for a site that renders
    a failure **carrying an underlying cause** into an ending's reported text
    without the shared renderer, THE SYSTEM SHALL report the verdict "every
    reported failure is rendered by the renderer".
  - FR-4.1-S2: IF the same scan is pointed at a fixture that renders a failure
    carrying an underlying cause with formatting of its own, THEN THE SYSTEM
    SHALL report that fixture's site and SHALL NOT report the clean verdict.
  - FR-4.1-S3: IF the scan is pointed at a source root that does not exist, THEN
    THE SYSTEM SHALL report the verdict "no source was read" and SHALL NOT report
    the clean verdict.

  The criterion in S1 is "renders a failure carrying an underlying cause", not
  "renders a failure", and that wording is load-bearing: it is a property of the
  value rather than a list of exempt paths. A path list ages, and an exemption
  list that grew to swallow the tree is the failure
  `crates/mc-client/tests/seam_boundaries.rs`'s own header warns about — and, one
  level up, is how this spec's defect survived.

### FR-5 — A cause is said once

- **FR-5.1**: No failure states its own cause and then has that cause rendered
  beneath it again.
  - FR-5.1-S1: IF a save is present and cannot be read, THEN the written text
    SHALL name the save's path once and state the reason it could not be read
    exactly once.
  - FR-5.1-S2: IF there is no save and a world cannot be generated in its place
    because a block is missing, THEN the written text SHALL name that block
    exactly once.
  - FR-5.1-S3: IF a save is refused only because blocks it references have been
    redeclared, THEN the written text SHALL contain the sentence offering
    `--load-changed-blocks` exactly once, and SHALL contain it after the refusal
    it is a way out of.

### FR-6 — Every ending says what it should, and the ones that are not failures say nothing

- **FR-6.1**: The reporting move changes where the four endings are rendered, not
  what any of them reports.
  - FR-6.1-S1: WHEN the shipped content root `content/base/` is read, THE SYSTEM
    SHALL register one block for each `.toml` file under `content/base/blocks/`
    and SHALL write nothing about content.
  - FR-6.1-S2: IF a run ended because the player closed the window, THEN THE
    SYSTEM SHALL write nothing at all.
  - FR-6.1-S3: IF a run ends because no adapter that can draw this client could
    be acquired, THEN THE SYSTEM SHALL write text naming that refusal to its
    error sink.
  - FR-6.1-S4: IF a run ends because the graphics device was lost, THEN THE
    SYSTEM SHALL write text naming the device-loss reason to its error sink.

  S1's expected count is derived by counting the `.toml` files in the fixture
  rather than stated as a literal, so it cannot become a number snapshotted from
  a green run.

### FR-7 — The documented refusal is the refusal that is printed

- **FR-7.1**: Every refusal quoted in `docs/modding/` is text the client
  produces.
  - FR-7.1-S1: WHEN every page under `docs/modding/` that quotes a refusal is
    compared with what the client writes for the declaration that page names, THE
    SYSTEM SHALL report the verdict "every quoted refusal is the refusal
    printed".
  - FR-7.1-S2: IF a quoted refusal is altered to text the client does not
    produce, THEN THE SYSTEM SHALL report the mismatch naming both the quoted
    text and the produced text.
  - FR-7.1-S3: IF a page under `docs/modding/` quotes no refusal at all, THEN THE
    SYSTEM SHALL report the verdict "no quoted refusal was found" and SHALL NOT
    report agreement.

## Technical Considerations

### Where the chain is actually lost

The chain is destroyed one layer earlier than PRO-939 reports. A `source()` walk
added where `Ending::Failed`'s text is printed would walk a `String`.
`crates/mc-client/src/app.rs:170` and `crates/mc-client/src/main.rs:44` each call
`.to_string()` on a typed failure to build that `String`; those are the two sites
the rendering has to reach. `crates/mc-client/src/main.rs:70` prints an
`Ending::Startup(StartupError)`, which has no source and loses nothing — but it
is a second spelling of the same job and should not survive as one, which is why
FR-6.1-S3 and S4 exist: both arms move, and nothing asserts them today.

### The renderer must be depth-general

A content refusal is exactly two layers deep, because `RegistryError::Source` is
`#[error(transparent)]` and `DefinitionFault` is a `Display` struct rather than
an `Error`. "Print one more level" would be correct for every content refusal
and wrong for the save path, which is three (FR-5.1-S1). FR-1.1-S3 is what
separates the two implementations.

### The report is not one line, and the field is not always a field

An unrecognised field is refused by `toml` 0.9.12's deserializer, whose `Display`
is a five-line caret diagnostic — `TOML parse error at line 4, column 1`, a rule
line, the offending source line, a `^^^^` marker, then `unknown field ...`. This
was measured by probing the crate directly, not inferred. Two consequences the
spec is written around:

- **The report is a block, not a line.** FR-1.1-S4 exists because of it, the
  documentation quotes a block, and no assumption or exclusion here claims
  otherwise.
- **`DefinitionFault.field` is `None` on that path.** The field name reaches the
  author inside `cause`, where the caret points at it more precisely than a field
  name would. `crates/mc-world/tests/content_loading.rs:281-289` already hedges
  between the two slots for exactly this reason. FR-2.1-S2 is therefore stated as
  what the author reads and never as which slot carries it — a reader must not
  assume the typed field is populated.

### Three variants state their own cause, and would say it twice (FR-5)

This is the one place the repair is not additive, and it is why the scope covers
the launch path as well as the content path.

| Site | Today | Under a chain walk |
|------|-------|--------------------|
| `crates/mc-sim/src/persistence.rs:52` — `LaunchError::Load` | `#[error("{save} could not be read: {source}")]` with `#[source]` | reason printed twice |
| `crates/mc-sim/src/persistence.rs:69` — `LaunchError::WorldGen` | `#[error("a new world could not be generated: {0}")]` with `#[from]` | cause printed twice |
| `crates/mc-client/src/startup.rs:146` — `PreparationError::Launch` | `#[error("{0}{way_out}")]` with `#[from]` | cause printed twice, way-out sentence stranded mid-report |

`LaunchError::Load`'s own doc comment states the assumption this spec
invalidates: *"The refusal a turned-away player reads is rendered from `Display`
alone — nothing walks the source chain."*

**This is a consolidation, not a widening, and the measurement is the argument.**
For `Load` and `WorldGen` the player-visible text afterwards is byte identical:
both already join with exactly `": "`, which is the renderer's own joiner, so the
joiner moves out of the format string and not one character changes. No variant
is added or removed, no refusal condition changes, and no sentence is rewritten.
A reader of the diff should not read it as scope creep.

`PreparationError::Launch` is the exception and the one genuinely new design.
Its message wraps its cause in a *suffix*, which an appending renderer cannot
express. **Left to `/sdd-architect`, with the team lead's recommendation on
record:** the way-out sentence is guidance rather than a link in the causal
chain — a cause says what happened, `--load-changed-blocks` says what to do about
it — so it should print after the whole chain rather than inside it. The
architect may overturn that with a reason.

### The reporting belongs in the library

`tools/voxforge/src/main.rs` is three lines and states the rule: rendered text
and exit-status selection live where a test can reach them, because a binary
carrying them earns the binary crates' coverage exclusion "and with it the
blindness that exclusion brings". `crates/mc-client/src/main.rs` carries both,
which is why the defect was invisible. Moving the reporting into the library,
written to a caller-supplied sink, is what makes FR-2, FR-3 and FR-6 assertable
at all.

**Exit-status selection deliberately does not move** (see Out of Scope), so there
is no new entry point onto it and no scenario here asserts it.
`crates/mc-render/src/window_test.rs:36-47` already covers `exit_code` through
the path that still calls it; a scenario re-proving that would re-prove it
through the same code path, which `testing.md` §1 calls worse than no test.

### Why FR-4 is a scan and not an assertion

A subprocess test on the real binary cannot serve FR-2 or FR-3: a block or HUD
refusal is collected at the first redraw, after a device is opened and a window
created, so observing it that way would need a GPU and a display server. Driving
the library's reporting alone is agreement between two copies of one decision,
which `testing.md` §2 names as a way to be green while shipping the defect. The
scan is what makes a program that stopped reporting go red. That is also why
`crates/mc-client/src/session.rs:498` and `crates/mc-client/src/gpu_startup.rs:127`,
which compose failure text by hand today, are converted rather than exempted.

### Why FR-7 exists

`docs/modding/README.md` today documents a refusal the program does not produce,
in the very paragraph this spec repairs. A guard costs one test and removes the
class. It binds every page under `docs/modding/` rather than one, because the
Documentation section below commits two of them to quoting the refusal.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| Chain rendering, `": "`-joined, outermost first | `crates/mc-testkit/src/frame/golden.rs:445-456` | the grammar, and the reason a `Display` alone is half an answer |
| Hand-walked chain the launch test uses today | `crates/mc-client/tests/hud_launch.rs:67-75` | replaced by the shipped renderer; its needles become FR-3.1-S1's, plus the element name it never checked |
| Malformed-content fixtures over a real copied root | `crates/mc-client/tests/support/content.rs` | `shipped_copy().declaring_block(...)` and `shipped_with(...)` build FR-2 and FR-3's roots |
| Text guard with a verdict, a positive control and a read-something check | `crates/mc-client/tests/seam_boundaries.rs`, `crates/mc-client/tests/winit_boundary.rs` | the shape FR-4's three scenarios follow |
| A binary that decides nothing | `tools/voxforge/src/main.rs` | the target shape for `crates/mc-client/src/main.rs` |
| The refusal contract being made real | `docs/modding/blocks-items.md:122-125` | the promise the printed text has to keep, including its two qualifications that FR-2.1 now mirrors |

## Documentation (Key Principle 3 — part of the definition of done)

Written from text captured from a real run, never composed by hand. FR-7 then
holds it to the truth.

- **Mod author — `docs/modding/README.md`.** The "When you get it wrong" section
  is rewritten. The paragraph beginning "What that line says today is less than
  the engine knows" and the "change one file at a time" advice are removed, and
  replaced with the refusal a person now reads, quoted whole as a block, for a
  named declaration. The sentence "The client exits without opening a window" is
  corrected in the same pass: it is true of a missing content root and false of a
  refused declaration, which is collected after the window opens (see
  `requirements.md`, finding 8).
- **Mod author — `docs/modding/blocks-items.md`.** "All-or-nothing loading"
  states the refusal contract and does not say what reaches the terminal. It
  gains that, with the same quoted refusal, so the page that makes the promise is
  the page that shows it kept.
- **Engine reader — `docs/technical/architecture.md`.** `mc-client`'s
  wiring-only role is already recorded there; it gains the reporting seam — where
  a failure is rendered, why it is rendered there rather than in the binary, what
  the scan in FR-4 forbids, and the rule FR-5 establishes that a message never
  states its own cause.
- **Engine reader — `docs/technical/testing.md`.** Why a printing path needs a
  scan as well as an assertion, why three chain levels are the minimum that
  separates a full walk from a single hop, and the shape of the
  documentation-drift guard.
- **Player — `docs/user/gameplay.md`.** A player handed a broken content root now
  reads what is wrong with it instead of one generic sentence. One paragraph.

FR-7 guards two of these five pages — the two under `docs/modding/` that quote a
refusal. The other three gain prose with no scenario behind it, which is normal
for documentation and is stated here so nobody reads it as an omission.

## Out of Scope

Binding.

- **New error variants, or changed refusal conditions.** No variant is added or
  removed, no `#[from]` relationship is repurposed to refuse something new, and
  the same content roots load that loaded before. The three `Display` changes
  FR-5 requires are named exhaustively above and are the only ones.
- **Refusing before the window opens.** A refused declaration currently opens a
  window and closes it again (`requirements.md`, DO-1). Reordering the launch is
  a design question about the frozen-desktop trade `startup.rs` makes on purpose,
  not a reporting fix.
- **Partial loading, an error screen, or a safe mode.** Loading stays
  all-or-nothing and a refusal stays fatal.
- **Reporting more than one refusal at a time.** The loader stops at the first
  fault; collecting every fault in a root is a loader change. The six single
  refusal *shapes* in FR-2.1 and FR-3.1 are each one refusal and are in scope —
  this bullet excludes only the collection of several.
- **Colour, or any structured or machine-readable form of the report.** Plain
  text on standard error, in the grammar FR-1 states. The report may span several
  lines when a layer's own message does; that is not a layout decision this spec
  makes, it is `toml`'s diagnostic arriving intact.
- **Moving argument parsing or exit-status selection out of `main.rs`.** Only the
  reporting moves. The binary gets shorter for one reason.
- **The other binaries.** `crates/mc-server/src/main.rs` is a stub and
  `tools/voxforge` already follows the pattern.

## Dependencies

None. Everything this touches is on `main` at `87b05ed`.

## Assumptions

- A mod author runs the client from a terminal and can read its standard error.
  `docs/modding/README.md` already instructs exactly that.
- The `": "` grammar is legible to a person as well as parseable. Both existing
  renderers in this workspace already bet that, and
  `docs/modding/script-faults.md` documents a rendering grammar of the same kind
  for script faults.
- A refusal is short enough to read in a terminal without paging. The longest
  this spec produces is the two-layer content refusal wrapping `toml`'s five-line
  diagnostic — seven lines in the worst case measured.

## Open Questions

None. Q1 (Scope A or Scope B) was answered by the team lead: **Scope B**, on the
grounds that a renderer with a hand-maintained exemption list cannot go red when
a path stops reporting. The suffix question in `PreparationError::Launch` is not
an open question about scope; it is a design decision recorded above and carried
to `/sdd-architect`.

## Clarifications

### Session 2026-08-16

- Q: Is the fix a `source()` walk at the print site, as PRO-939 states? → A: No.
  The chain is flattened at the two sites that build an `Ending::Failed` from a
  typed failure (`app.rs:170`, `main.rs:44`); the print site holds a `String`.
  Recorded as `requirements.md` finding 5.
- Q: Do both modding pages currently describe the truncated message? → A: Only
  `README.md`. `blocks-items.md` states the contract without admitting it is
  unkept, so it needs a different correction. Recorded as finding 7.
- Q: Does a refused declaration stop the launch before a window opens, as
  `README.md` says? → A: No — the preparation is collected at the first redraw.
  Correcting the sentence is in scope; changing the ordering is not. Recorded as
  finding 8 and DO-1.
- Q: Is the report one line? → A: No. Ruled one line initially, then withdrawn
  once `toml` 0.9.12 was probed directly and shown to render a five-line caret
  diagnostic. The chain is still joined outermost-to-innermost with `: `; what
  does not hold is "no embedded newline".
- Q: Does walking the chain change any message a player reads today? → A: For
  `LaunchError::Load` and `LaunchError::WorldGen`, not by one character — both
  already join with `": "`. For `PreparationError::Launch`, yes, and that is the
  design question left to `/sdd-architect`.
- Q: Scope A (content only) or Scope B (every reported failure)? → A: Scope B,
  approved by the team lead. Scope A cannot state FR-4's guard without an
  exemption list, and an exemption list is how this defect survived.
