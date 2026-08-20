# Tasks: The grass block looks like a grass block — per-face keys, baked art, real pixels from disk

**Spec**: [spec.md](spec.md) (SPEC-019, rigor `high`) ·
**Architecture**: [architecture.md](architecture.md) (binding, 16 decisions, one deferred) ·
**Requirements record**: [requirements.md](requirements.md) ·
**Branch**: `feature/PRO-947-grass-block-art` · **Issue**: PRO-947 ·
**Created**: 2026-08-18

One task = one coherent scenario group in one area. **The nine phases are the
architecture's and are not re-cut here** — every cut line is defended in
`architecture.md` `## Phases` and the constraint they all serve is stated below.
`[P]` = independent of other `[P]` tasks in the same phase. **56 tasks.**

Every `Scenarios:` line carries full scenario ids, comma-separated, with no
ranges — so a mechanical check can expand the whole breakdown without parsing
prose. The phase table's totals are a reader's summary; the task lines are the
record.

## The count is not restated, it is computed

A count kept by hand beside the thing it counts drifts, and this one has drifted
three times inside this spec. Run these two, from the spec folder:

```
grep -oE "FR-[0-9]+\.[0-9]+-S[0-9]+:" spec.md | sort -u | wc -l
grep -oE "FR-[0-9]+\.[0-9]+-S[0-9]+" tasks.md | sort -u | wc -l
```

They must agree, and their `comm -3` must be empty. That is what "every scenario
is assigned to exactly one task" means here; nothing in this file asserts it in
prose.

| Phase | Tasks | Requirements | What it delivers |
|---|---|---|---|
| **P1** — The stable fold moves to `mc-core` | T01–T03 | — | `fnv_1a_64` in `mc_core::hash`, and the one assertion about *both* save hashes that is only writable before P2 |
| **P2** — A block declares six facings, and a save records them | T04–T11 | FR-1.1, FR-1.2, FR-1.4, FR-9.1 | `Face`, `FaceTextures`, `TEXTURE_EDGE`, `Facing::face`, the Luau table form and its refusals, the layer budget by a new route, `DeclaredAppearance` and `INPUT_VERSION` 1→2 |
| **P3** — Resolution at packing, and at the indicator | T12–T19 | FR-1.3, FR-2.1, FR-2.2 | `TextureResolution`, both signatures, the whole plumbing, **PRO-902 closed at both sites** |
| ~~**P4**~~ — *retired; its five documentation edits were made directly* | — | — (FR-8.2 retired) | the repaired citation, a provenance header on each generator, and `docs/modding/voxel-models.md`. T20–T24 left unused |
| **P5** — `voxforge build` | T25–T35 | FR-3.1, FR-3.2, FR-3.3, FR-3.4 | the manifest, `mc_core::art`, the fold, the grouping, every refusal, `.gitignore` |
| **P6** — The gate builds the art and refuses a committed one | T36–T39 | FR-7.1 | two new gate stages, the PowerShell-driving harness and `-ArtOnly` that drive them, and the one stated exception to "every stage runs" |
| **P7** — The client judges the set and refuses by name | T40–T43 | FR-5.1, FR-5.2 | `mc-client/src/textures/`, the six-arm verdict, the new `PreparationError` variants |
| **P8** — The mip chain, as arithmetic | T44–T47 | FR-6.1 | `to_linear`/`to_stored`/`reduced`/`chain`/`levels_for`, pure and **unwired** |
| **P9** — Real pixels | T48–T56 | FR-4.1, FR-4.2, FR-4.3, FR-6.2, FR-7.2, FR-8.1 | texels reach `write_layer`, the mips and the sampler are wired, `grass.luau` declares six facings, the goldens are re-shot at `r1` |

---

## The one constraint every cut serves

**The golden set is re-shot exactly once, in P9.**

Everything that moves a pixel is in P9. Everything that *could* move a pixel but
does not have to — the mip arithmetic, the sampler request, the set verdict,
per-face resolution while the shipped content still declares uniform keys —
lands earlier and picture-neutral, **where a golden diff is a defect rather than
an expectation**.

So in P1–P8, a golden mismatch is never "the art changed". It is the phase's
first and loudest signal that something was wired that should not have been.
Three specific moves spring it, each a few lines from code the phase is
legitimately writing:

| The tempting move | Where it sits | Phase it belongs to |
|---|---|---|
| Setting `mip_level_count` to `MIP_LEVELS` "since `chain` exists now" | `crates/mc-render/src/gpu/buffers.rs` | **P9**, T51 |
| Passing `TERRAIN_SAMPLER` into `terrain_sampler` "to make the constant used" | `crates/mc-render/src/gpu/buffers.rs`, `gpu/hud.rs` | **P9**, T51 |
| Handing `built_set`'s texels to `FrameRenderer::new` "they are right there" | `crates/mc-client/src/startup.rs` | **P9**, T48 |

If a task you are implementing would move a pixel and it is not in P9, stop and
report it. It is a finding about the breakdown, not a task to reschedule
quietly.

---

## Read this before implementing anything

### The test author is fresh per phase, and owns that phase's tests

At rigor `high` a phase's tests are authored by a test author that has not seen
any implementation and owns them for the whole phase. **The implementation
context never edits a test file — not for a rename, not for a doc comment, not
for a formatting fix.** Disputed failures go to the test author with exactly one
verdict:

- `test-correct` — the implementation conforms; the implementer fixes it.
- `test-wrong` — the author fixes and commits it.
- `scenario-ambiguous` — **this goes to the team lead, not back to the
  implementer.** It is a spec defect and the ruling is theirs.

There is one narrow exemption and it is not an edit: an implementer may
temporarily break a line inside a test file to run a falsifier that lives there,
provided the edit is reverted **by hand** and `git diff --exit-code` is clean
before anything else happens. No such edit may ever land.

### Each phase boundary switches both people, and the closing report is the whole inheritance

Write the closing report for somebody who has read `spec.md`, `architecture.md`
and this file, and nothing else. It must carry: which tasks closed, the arrival
colours actually measured, every mutation **run** with its outcome including the
non-bites, any fixture constraint no assertion can enforce, and every deferred
observation. Anything held only in a conversation is lost at the boundary.

### Staging, and the two rules that have already cost this project work

- **Explicit paths only.** `git add -A`, `git add .` and `git commit -a` are
  banned, with no exception for "the tree is clean, I checked" — a sweep once
  pulled a test author's in-flight file into an implementation commit. **Other
  sessions share this working tree**: never stage a path you did not write, and
  re-read `git status --short` and `git stash list` before concluding anything
  from a gate run.
- **Revert a mutation check by hand** — re-edit the line you broke. Never
  `git checkout -- <file>`: it once wiped an uncommitted implementation. Confirm
  with `git diff --exit-code` before continuing.

### Reading a gate run

- Filter failures on **`FAIL \[`**, never `^FAIL`. nextest indents that line, and
  `^FAIL` silently destroys the failing test's name — which is the whole of what
  you needed.
- **The gate counts non-blank lines** (`Get-Content | Measure-Object -Line`), so
  `wc -l` disagrees with it and will tell you a file is fine when it is not. Source
  files cap at 500, test files at 600.
- **A doc-only commit is not gate-neutral.** `gitleaks dir .` scans the working
  tree, so a phase that closes with a documentation task still runs the full gate.
  `docs/technical/working-in-this-repo.md` records this.
- **A green suite is no evidence about a lint.** Where a phase opens with an
  adaptation commit and no compilable tree, the gate cannot run at all and
  anything only it can see accumulates silently across that window. Whoever
  authors tests inside it runs
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  directly. Without `-D warnings`, cargo attributes the diagnostic to the first
  binary and marks the rest `(1 duplicate)` — which means *this same diagnostic,
  repeated*, not *a pre-existing one lives elsewhere*.

### Two load-sensitive tests are live in this tree

`reload_remesh_blocks_no_tick` (tracked as PRO-954) waits for a whole-world
re-mesh and has failed once at 28.6 s past a 15 s bound under a concurrent
instrumented workspace run, then passed on re-run; its five readings are recorded
in `docs/technical/testing.md`. A second load-sensitive test is tracked as
PRO-953. **Neither is quarantined and both witness something real.**

- A failure in either is **re-run before it is diagnosed** and is never
  attributed to the spec in flight on a first sighting.
- Read the next occurrence as **the verdict arm plus that test's captured
  stderr**, not the arm alone.
- **P3 keeps `reload_remesh_blocks_no_tick` on exactly its sensitive path**, by
  widening `changes_geometry` to all six keys (D3). P3's test author must know
  this before meeting it.

---

## The traps, named out loud

### Trap 1 — `crates/mc-client/src/app/mod.rs` is at exactly 500 non-blank lines

Measured at `87bbb84` with the gate's own counter. The source limit **is** 500,
so the file has **zero** headroom: one added line fails the size stage. P3 touches
it — `app/mod.rs:344` is what composes a swatch and it moves from
`texture_layers()` to `texture_resolution()`.

A same-length rename is survivable and anything else is not.

**So this is a task and not a warning: T12, first in P3, and nothing else in the
phase may touch `app/mod.rs` before it lands.** The reason it is planned rather
than flagged is that a cap hit while doing something else does not reject the
something else — **it rejects whatever is cheapest to drop.** The previous spec met
this and what it dropped was an *explanation*: the size stage silently evicted
prose rather than rejecting code, and the reasoning survived only because somebody
reported the eviction. A cut chosen under gate pressure, by whoever happens to be
holding the file, at the moment they are trying to land something else, is the
worst version of that decision available. T12 states the shape and defends it in
advance.

`crates/mc-client/src/session/mod.rs` is at 485 and is the second file to
re-measure before touching. `crates/mc-client/tests/support/probe.rs` is at 551 of
a 600-line test limit, and **P9 re-derives its constants on 49 lines of
headroom** — P9's pair should have that number before they start, not discover it.

Re-measure with the gate's counter, not `wc -l`:

```powershell
(Get-Content -LiteralPath crates/mc-client/src/app/mod.rs | Measure-Object -Line).Lines
```

### Trap 2 — P1 must not be merged into P2 for tidiness

P1 has no scenarios. It is the `fnv_1a_64` move plus **one additional-coverage
test that is only writable before P2 changes what an appearance is**: every
`behaviour_of` and `appearance_of` value the shipped content produces, against
values an independent FNV in the test computes over the same postcard bytes.

Once `DeclaredAppearance` gains six keys and `INPUT_VERSION` goes to 2, that
assertion can no longer be made about *both* halves at once, and FR-9.1-S4 —
"every hash other than a block appearance is unchanged" — has nothing behind it
but the claim itself. **Merging P1 into P2 loses the assertion permanently.**

### Traps 3 and 4 — retired with FR-8.2, 2026-08-18

Both had P4's gate stage and P4's generator repairs as their only subjects, and
both go with them. **The numbers are left unused rather than closed up**, so that
every "read Trap 5" through "read Trap 11" elsewhere in this file still means
what it meant.

Trap 3's lesson outlives this spec and is not left to the archive: a control for
a path rule must be shaped like the platform's own paths, because a POSIX-only
control on a Windows checkout goes green against the very literal it exists to
catch. It is recorded in `docs/technical/testing.md`, beside the rest of the
falsifiability material.

### Trap 5 — FR-2.1-S4 must be authored at the retained-packing seam

It is **the only witness separating design option (a) from option (b)**; under
option (a) the entire rest of the spec passes.

**A test written against a reload passes under both options and proves nothing.**
D3 is why: `changes_geometry` widens to all six keys, so a facing-key change marks
every section, and `take_remesh_work` drains the entire dirty set into one batch —
there is no partial-drain window, so retained-but-not-re-meshed is unreachable
through a production reload.

The seam is one level down and is real: `Retained::rebuilt` re-packs the **entire**
retained list on every batch. The test retains a meshed list built under content A,
hands the packer content B's resolution, re-packs, and reads the layer back through
`SectionGeometry::layer_at` (`crates/mc-render/src/geometry/mod.rs:110-124`, which
exists for exactly this).

### Trap 6 — a refused art build must exit non-zero, and the skipped stage must record itself

FR-7.1-S6 requires the test stage **not** to run when `voxforge build` refuses,
because a refused build leaves the previous set intact (FR-3.3-S10) and running the
suite anyway grades a stale set. This is the one stated exception to the script's
own "every stage runs even if an earlier one fails" property, and the header must
say so in a sentence beside the property.

The skip must record itself — `$Failures.Add('tests (not run: art build failed)')`
— because otherwise the summary lists one fewer stage and a reader cannot tell
*the tests were not run* from *the tests are not in this list*. **A gate that omits
a stage silently is one step from a gate that skips its way to green.**

### Trap 7 — P7 makes a bare `cargo nextest run` fail without a built set

That is **FR-7.2-S2 working, not a regression.** The gate is green because P6
taught it to build the set first. Say this in P7's closing report in as many words,
or the next pair will "fix" it by making the client tolerate an absent set — which
deletes FR-5.1 and FR-5.2 together.

### Trap 8 — FR-3.4-S3 is a negative control and it is the one that constrains the fold

Every *positive* staleness scenario is satisfied by folding the whole `content/`
tree. FR-3.4-S3 — a model under `content/base/models/` that no manifest entry
names is edited, and the value does not move — is the only thing that stops it,
and folding the whole tree would turn each unrelated content edit into a spurious
launch refusal.

Its sibling is FR-3.4-S4: the recorded value must be **derived** from the stated
byte sequence (D11), never snapshotted from a run of the code under test.

### Trap 9 — FR-8.1-S4 and FR-8.1-S5 must not come to share a path

S4 is a golden minted from the renderer it verifies. S5 is the derived witness:
the grass top face judged against a mean computed from the built PNG, decoded by
the **client's** decoder and never by the draw. If a refactor lets S5 read a value
the frame produced, the pair collapses into one snapshot and this spec's only
independent judgement of the picture is gone. **No mutation detects this** — it is
reviewer-held, and it is named here because that is the only defence it has.

### Trap 10 — `voxforge build` must not reach the Luau host

FR-3.3-S11's "a key that no loadable block declares" read literally puts a Luau VM
inside a texture baker, and lets a broken block declaration refuse an art build
that has nothing to do with it. It is **a text scan over `blocks/*.luau`,
advisory, never a refusal**. A declaration that *computes* its key is not seen and
is reported unused; that limitation is documented in a comment on the scan itself
as well as in `docs/modding/voxel-models.md`, and it is acceptable precisely
because a false positive costs one line of output.

### Trap 11 — both generators pick their salt by a search

`gen_grass.py` and `gen_stone.py` each terminate on a score threshold. Deterministic
on this input — the spike's byte equality proves that — but it is **the one place a
future edit could break reproduction without touching a grid**. The provenance
headers added to both scripts say so; nothing else touches the search.

**The salt search is now unguarded, and that is correct.** FR-8.2's two
reproduction scenarios were what would have caught an edit to it; they were retired
with the rest of FR-8.2 on 2026-08-18, and nothing replaces them. Reproduction is
not a property this repository holds any more, because nothing re-runs the
generators: the tracked models are the source of
truth and the scripts are provenance. An edit that broke the search would break a
run nobody performs. **The sentence is kept rather than dropped** — a thing recorded
as unguarded is safe, and a thing that merely stops being mentioned is not.

---

## Phase 1 — The stable fold moves to `mc-core`

**No spec scenarios.** Done means: `fnv_1a_64` lives in `mc_core::hash`, `mc-world`
calls it there, no stored hash has changed value, and the assertion that says so
about *both* save hashes exists — because after P2 it cannot.

**Picture: unchanged.** Nothing here reaches a frame.

- [x] **T01** `fnv_1a_64` moves to `mc_core::hash`, unchanged, and `mc-world` imports it
      — `crates/mc-core/src/hash.rs` (new), `crates/mc-core/src/lib.rs`,
      `crates/mc-world/src/persistence/format.rs`
      Scenarios: —
      - D10 is binding and is narrower than it looks: **only the byte fold moves**
        (`format.rs:358-365`). `folded()` and `DefinitionHash` stay in `mc-world`,
        because `folded` names `postcard` and `postcard` is confined to
        `mc-world/src/persistence/` by that crate's own manifest comment.
      - The function is private today (`fn fnv_1a_64`). It becomes `pub` in
        `mc-core` and keeps its whole doc comment: the reason it is hand-written —
        the standard library's hasher is unspecified and moves with the toolchain —
        is the reason two independent programs may share it, and it is about to
        acquire a second consumer.
      - `placeholder.rs:64-70` carries a **third** copy of the same two constants,
        inlined, with its own reasoning. **It is left alone.** It hashes a key to a
        colour and is not a value two programs must agree on; folding it in is a
        refactor with a golden-frame blast radius and no correctness gain.
      - `FNV_OFFSET_BASIS` and `FNV_PRIME` move with the function. `format.rs` keeps
        no copy.

- [x] **T02** The pre-P2 witness — both save hashes, against an independent oracle
      — `crates/mc-world/src/persistence/format_test.rs` (new, sibling),
      `crates/mc-world/src/persistence/format.rs` (the `#[path]` mod line)
      Scenarios: —
      Depends on: T01
      - **This is Trap 2 and it is the whole reason P1 exists as a phase.**
        `behaviour_of` and `appearance_of` are `pub(crate)`, so the sibling
        `format_test.rs` is the only vehicle; that layout is this project's
        convention and is why the gate excludes `*_test.rs` from coverage.
      - The oracle **computes FNV-1a-64 in the test**, over the same postcard bytes,
        from the stated constants. It must share no code with `mc_core::hash` — a
        test calling the function under test is agreement between two copies of one
        decision.
      - Both halves, in one test each: every shipped block's `behaviour_of` and
        every shipped block's `appearance_of`. After P2 the appearance half changes
        by design, and FR-9.1-S4 is what carries the behaviour half forward — but
        **FR-9.1-S4 alone never observes the two together**, which is the assertion
        being bought here.
      - No value in this test is copied from a run. Every expected number is
        arithmetic over bytes the test itself built.
      - **Status, added at completion: done, and the box was the thing that was
        missed.** `format_test.rs` ships both halves —
        `every_shipped_blocks_recorded_behaviour_is_the_fold_an_independent_oracle_computes`
        and its appearance twin — with the fold computed in the test from the stated
        constants and the `#[path]` mod line in place. Nothing was outstanding; the
        checkbox was never ticked, which is the one thing a task list cannot
        measure about itself.

- [x] **T03** [P] The stable fold is recorded as a contract, not as an implementation detail
      — `docs/technical/architecture.md`
      Scenarios: —
      - Key Principle 3 binds at the moment of implementation, and the audience here
        is the **engine reader**: `fnv_1a_64` is now in `mc-core` because voxforge
        and `mc-client` must agree on one value forever and `crates/` may not depend
        on `tools/` (SPEC-013 FR-9.1). One paragraph, naming the constraint and the
        rule that a second implementation on either side is the defect this
        arrangement exists to make unspellable.
      - State plainly that the index contract and the fold's byte sequence, which
        this paragraph will point at, do not exist yet and land in P5. A surface
        that genuinely does not exist yet belongs to a later phase; its absence is
        stated rather than left silent.

### Mutations — P1

Run each by hand, observe, revert by hand, confirm `git diff --exit-code` clean.
**Record the outcome as measured, including the ones that do not bite** — a
non-bite is evidence about the code's structure, not automatically a gap.

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
| M1 | Change `FNV_OFFSET_BASIS` in `mc_core::hash` by one | T02 reddens on both halves; `save_identity`/`save_determinism` redden too | **Bit, run pre-move** against `format.rs`'s copy, `--no-fail-fast`: **5 of 1223** red. Both T02 halves, plus `save_declarations::a_solid_breakable_block_that_breaks_into_nothing_records_the_version_1_behaviour` and both `shipped_declarations_and_an_older_save` guards. **The prediction is wrong about `save_identity`/`save_determinism`** and instructively so: those compare two saves *to each other*, so a hash that moves in lockstep leaves them green — a whole family that cannot see a moved value. **Re-run post-move against `mc_core::hash`: identical — 5 of 1223, the same five names.** The instrument was seen to still work after the target changed crates rather than assumed to. **Run a third time once `crates/mc-core/tests/hash.rs` existed: 6 of 1224** — the same five plus the new published-surface guard |
| M2 | Leave a private copy of `fnv_1a_64` in `format.rs` and call that | **non-bite** — behaviour is identical. The move is compiler-held and reviewer-held, never test-held, and saying so is the point of running it | **Did not bite, as predicted**, run post-move with `--no-fail-fast`: **0 of 1223** red, whole workspace. The import removed, the constants and the fold restored privately to `format.rs`, `folded` calling that. Nothing in the suite can see which crate the fold lives in, which is what T02's header says out loud and is the reason the move is held by the compiler and by a reader instead |
| M3 | `fnv_1a_64` returns the basis without folding | T02 reddens; predicted to redden very widely | **Bit, run pre-move**, `--no-fail-fast`: **13 of 1223** red across `mc-client` and `mc-world` — both T02 halves, three launch/reload acceptance guards, both `save_acceptance` guards, both `save_declarations` guards, `save_determinism::one_block_declared_solid_and_then_not_stores_different_bytes`, `save_resolution`, and both `shipped_declarations_and_an_older_save` guards. As predicted, very wide. **Re-run post-move with `crates/mc-core/tests/hash.rs` in place: 14 of 1224** — the same thirteen plus the published-surface guard |

**Four further falsifiers were run by the test author**, all `--no-fail-fast`, all
reverted by hand. Neither of P1's guards could ever be red first — T02 is green by
construction and `hash.rs` was written after its subject landed — so each has to
answer for its fixture as well as for its subject, and each is asked what it alone
can see:

| # | Falsifier | Where | Outcome |
|---|---|---|---|
| F1 | Drop `base:water` from the test's stated shipped set | T02's own file | **Bit** — both halves red. The two maps are compared whole, so a block the repository ships and the test does not name is caught, and an empty registry cannot satisfy "every shipped block" by having none |
| F2 | The appearance oracle stops folding the texture key | T02's own file | **Bit, and exactly one half** — appearance red, behaviour green. The oracle genuinely reads the texture, and the two halves are independent rather than one assertion written twice |
| F3 | `fnv_1a_64` folds its bytes in reverse order | `mc-core/src/hash.rs` | **Bit: 6 of 1224**, the same six M1 reddens. The published-surface guard sees it through its two-bytes/same-two-reversed pair, which exists for exactly this: a fold that tallied its bytes instead of folding them answers identically for both |
| F4 | `fnv_1a_64` answers zero for an empty input | `mc-core/src/hash.rs` | **Bit: 1 of 1224 — the new guard, and nothing else in the workspace.** Every other witness on this function folds a non-empty postcard record, so the empty-sequence boundary had **no witness at all** before this commit. This is the single-witness case `testing.md` §1 names, and it is the strongest justification `crates/mc-core/tests/hash.rs` has for existing |

---

## Phase 2 — A block declares six facings, and a save records them

**19 scenarios.** Done means: a declaration states `texture` as one string or as a
table of exactly six facings, the refusals name what is wrong in the guide's own
terms, per-face keys spend layers from the same budget by a new route, and a save
records an appearance folded over all six.

**Picture: unchanged**, and that is load-bearing — all four shipped blocks still
declare one string, so every key resolves to the layer it already had.

**Red on arrival**: the skeleton is `FaceTextures::uniform` only — the table form
refuses everything. That reddens FR-1.1-S2/S3/S4 and every FR-1.2 refusal by
message, and leaves FR-1.1-S1 green as a control. A skeleton that *accepts* every
table would pass FR-1.2-S2 (the empty table) for the wrong reason.

- [x] **T04** `Face`, `FaceTextures` and `TEXTURE_EDGE` in `mc-core`; `Facing::face` in `mc-world`
      — `crates/mc-core/src/content.rs`, `crates/mc-world/src/mesh/facing.rs`
      Scenarios: —
      - D1 option (c) is binding. `mc-core` cannot see `mc_world::mesh::Facing`, and
        `Facing` cannot move — it depends on `crate::section::{Axis, LocalPos,
        SECTION_SIZE}` and `super::PlanePos`. So `mc-core` defines content's own
        published vocabulary and `mc-world` provides the one exhaustive `match`.
      - The mapping is `up = PosY`, `down = NegY`, `north = NegZ`, `south = PosZ`,
        `east = PosX`, `west = NegX`. **It is the only place in the workspace where
        a compass word meets an axis.**
      - `FaceTextures::stating` takes `[TextureKey; 6]` **positionally in `Face::ALL`
        order** — not `[(Face, TextureKey); 6]`, which would let two entries name
        `Up` and make `at` a lookup that can miss. `at` is total: no `Option`, no
        indexing.
      - `TEXTURE_EDGE: u32 = 16` sits beside `LAYERS_A_SESSION_MAY_ASSIGN` and for
        the same stated reason. `mc-render` asserts its array-texture extent against
        it at compile time exactly as it already does for the layer bound, and
        `PLACEHOLDER_SIZE` (`placeholder.rs:80`) is **deleted** —
        `placeholder_texels(key, size)` keeps its parameter.
      - **Additional coverage, and it closes D1's own strongest objection:** an
        exhaustive round trip over `Facing::ALL` and `Face::ALL`. Two enums for six
        directions reads as duplication on sight; the round trip is what closes the
        drift mechanically, and option (a) had the same drift with nowhere to put
        such a test.
      - **Do not reorder `Facing`'s declaration** to make the mapping tidier. That
        order is the emission order, the neighbour slot order and `Ord`.

- [x] **T05** The Luau `texture` field reads a string or a table of six facings
      — `crates/mc-world/src/content/luau_declaration.rs`,
      `crates/mc-core/src/block/definition.rs`
      Scenarios: FR-1.1-S1, FR-1.1-S2, FR-1.1-S3
      Depends on: T04
      - `BlockDefinition.texture: TextureKey` → `BlockDefinition.textures:
        FaceTextures`.
      - The table is read through `host.field_names(table, SIX_FACINGS_READ)` and
        `host.read_field(table, word)`, **raw** — so a declaration's metatable
        neither supplies a facing it did not state nor hides one it did. That is the
        property the module header already states, extended one level down, and it
        is not optional.
      - `RECOGNISED_FIELDS` order is untouched: `texture` keeps its name and its
        position. `crates/mc-client/tests/documented_refusals.rs` compares that list
        to `docs/modding/blocks-items.md` line for line.
      - FR-1.1-S3 is the scenario that says the same key may be named twice; nothing
        about `FaceTextures` may make that a refusal.

- [x] **T06** Six keys reach the registry and spend layers from the one budget
      — `crates/mc-core/src/block/registry.rs`, `crates/mc-core/src/content.rs`,
      `crates/mc-sim/src/content.rs`
      Scenarios: FR-1.1-S4, FR-1.4-S1, FR-1.4-S2, FR-1.4-S3
      Depends on: T05
      - `BlockRegistry::texture_keys` unions `definition.textures.keys()`. It must
        keep asking the **registry** and never a world (`registry.rs:40-45`).
      - `LayerAssignment` and `LayerBudget` are **unchanged contracts**. FR-1.4's
        refusal is the existing one reached by a new route, and keys stay appended,
        never renumbered — a layer index rides inside every packed vertex, eight
        bits wide.
      - FR-1.1-S4 and FR-1.4-S1 both name a number. **Derive both**: five is six
        declared keys minus the one named twice (T05's FR-1.1-S3 declaration); 256
        is 250 plus six. Neither may be copied from a run.
      - FR-1.4-S3 is the all-or-nothing half and is the one a passing implementation
        gets wrong quietly: on refusal every layer already assigned still holds the
        key it held.
      - `ResolvedBlock.texture` → `ResolvedBlock.textures`. The seam at
        `mc-sim/src/content.rs:164` stays cut — `replaceable`, `breakable` and
        `breaks_into` still do not cross.

- [x] **T07** A texture table that is not exactly six facings is refused, naming what is wrong
      — `crates/mc-world/src/content/luau_declaration.rs`
      Scenarios: FR-1.2-S1, FR-1.2-S2, FR-1.2-S3, FR-1.2-S4, FR-1.2-S5, FR-1.2-S6, FR-1.2-S7
      Depends on: T05
      - The refusals follow the module's existing `FieldFault` shapes. The
        architecture's `## Interfaces` table gives each wording; **final wording is
        the implementer's against `documented_refusals.rs`**, which compares the
        guide to a real run line for line.
      - Two of these are the same shape from opposite sides and both are owed:
        FR-1.2-S1/S2 name **the facings that were not stated**; FR-1.2-S3/S4 name
        **the six a table may state**. A single message doing both jobs fails one of
        them.
      - FR-1.2-S4 (`Up`) is the exactness witness: `Face::named` matches exactly and
        does not case-fold. An implementation that lower-cases the word passes S3 and
        fails S4, which is why both exist.
      - FR-1.2-S6 reuses `NamespacedIdError::MultipleSeparators` unchanged. A second
        wording is a second place to disagree.
      - FR-1.2-S7 is about `texture` itself being neither form — a different fault
        from anything inside a table.

- [x] **T08** Every refusal this spec raises reaches the modding guide, in the field order it already states
      — `docs/modding/blocks-items.md`,
      `crates/mc-client/tests/documented_refusals.rs`
      Scenarios: FR-1.2-S8
      Depends on: T07
      - The guide and the code stay **line for line**. This is the conformance test
        that already exists; what changes is that seven more refusals must appear in
        it, in `RECOGNISED_FIELDS` order.
      - The audience is the **mod author** and the standard is that they can act on
        it without reading Rust: what a refusal looks like, and how to read it.

- [x] **T09** A save records an appearance folded over all six keys, and the appearance revision goes 1 → 2
      — `crates/mc-world/src/persistence/format.rs`,
      `crates/mc-world/src/persistence/table.rs`
      Scenarios: FR-9.1-S1, FR-9.1-S2, FR-9.1-S3, FR-9.1-S4
      Depends on: T04, T06
      - **CORRECTION, ruled 2026-08-18: the version byte becomes PER FIELD LIST.**
        This task and `architecture.md` both said "`INPUT_VERSION` goes 1 → 2" as
        one number, and as written that **breaks FR-9.1-S4**. `format.rs:249` holds
        a single `INPUT_VERSION` folded as the first field of **both**
        `DeclaredBehaviour` and `DeclaredAppearance`, so a shared bump moves every
        block's *behaviour* hash — which FR-9.1-S4 forbids in as many words — and
        reddens `save_declarations::a_solid_breakable_block_that_breaks_into_nothing_records_the_version_1_behaviour`,
        a file this phase does not touch.

        **Behaviour stays at revision 1; appearance goes to revision 2.** The tests
        are written to `STATED_BEHAVIOUR_REVISION = 1` and
        `STATED_APPEARANCE_REVISION = 2`.

        This is a wording defect rather than a design change: `spec.md` is the
        source of truth and `architecture.md` is a plan for satisfying it, so the
        scenario wins. `format.rs:244`'s own doc comment already says "adding a
        field to **one of them**" — the single shared constant was conflating two
        independent revisions before this spec arrived.
      - `DeclaredAppearance` gains the six keys **in `Face::ALL` order**.
        `DeclaredBehaviour` is untouched. The version byte at `format.rs:264` is
        documented as exactly the mechanism for this case; no new migration
        machinery and no new flag.
      - **The consequence is player-visible and intended**: every save written
        before this spec reports every block's appearance as changed on next load,
        through the existing `--load-changed-blocks` path. Every block's appearance
        really did change.
      - **FR-9.1-S4 is a negative assertion about hashes that did not move**, and it
        is the one this whole phase can break quietly. Fold a definition through
        both `behaviour_of` and `appearance_of` and assert the behaviour value
        against a value derived **by hand**, not snapshotted. T02 is the wider
        version of this and it is already committed; S4 is what carries it forward.
      - FR-9.1-S2's value is derived from the stated byte sequence, never from a
        run.

- [x] **T10** `drawn_of` compares all six keys
      — `crates/mc-sim/src/world/reload.rs`
      Scenarios: —
      Depends on: T06
      - **Forced, not chosen.** `reload.rs:81` keys `drawn_of` on `(is_solid,
        &TextureKey)` and that field is becoming six, so this function must be
        touched whatever else is decided. Deferring it needs a stopgap governed by
        no scenario, and leaves a reload that changes only `north` marking nothing.
      - D3 option (a): widen it. **The whole-world marking rule is untouched.**
        `docs/technical/rendering.md:490-510` states the binary rule as a
        specification and says outright that narrowing it is a specification change
        and not an optimisation somebody may take while passing. This spec does not
        specify it.
      - No new scenario is owed: `rendering.md:497-510`'s section-count assertions
        already guard it. **This is the change that keeps
        `reload_remesh_blocks_no_tick` on its sensitive path** — see the load note
        above before reading a failure.

- [x] **T11** [P] The declaration's two forms, the six words and their axes, and what a save now notices
      — `docs/modding/blocks-items.md`, `docs/modding/hot-reload.md`,
      `docs/technical/world-format.md`
      Scenarios: —
      Depends on: T08, T09
      - **Mod author** (`blocks-items.md`): `texture`'s two forms; all six facing
        words with the axis each maps to; every field with its type and its bound;
        **a complete worked grass declaration that runs.** A reference listing names
        without a working example is not documentation.
      - **Mod author** (`hot-reload.md`): a `texture` edit is now visible, and the
        per-field table gains the per-facing row.
      - **Engine** (`world-format.md`): `INPUT_VERSION` 1→2 and the appearance
        fold's six keys, with the consequence stated — a pre-spec save reports every
        block as changed, and why that is correct rather than a migration bug.
      - The player-facing half of this waits for P9, where the world actually looks
        different. Say so rather than leaving it silent.

### Mutations — P2

Every run below is `cargo nextest run --workspace --all-features --no-fail-fast`
against a tree of **1246 tests, 1 skipped**, each mutation reverted by hand with
`git diff --exit-code` clean between them. Nothing from the window landed.

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
| M4 | `FaceTextures::at` returns `Face::Up`'s key for every face | FR-1.1-S2, FR-1.1-S3 redden | **Bit: 8 of 1246.** Wider than predicted, and instructively: FR-1.1-S2 and S3 as named, plus all four of `per_face_layers` (a block's six facings collapse to one key, so its layer count moves) and both `save_per_face_appearance` guards (the fold reads through `at`). The funnel is `at` itself — every reading of a declared key in the workspace goes through it |
| M5 | `Face::named` lower-cases `word` before matching | FR-1.2-S4 reddens; FR-1.2-S3 stays green — the pair is the point | **Bit: 3 of 1246, and the pair held exactly.** FR-1.2-S4 red, **FR-1.2-S3 green** — which is the whole reason both scenarios exist. The other two are `documented_refusals`' own pair, because the guide now quotes a refusal the client stopped printing: the page-versus-program comparison catches a wording change nobody edited the page for |
| M6 | `Facing::face` maps `north` to `PosZ` and `south` to `NegZ` | **predicted non-bite in P2** — the round trip is still a bijection. Only P3's FR-1.3-S2 can see it | **Did not bite, as predicted: 0 of 1246**, whole workspace. The round trip is a completeness guard and says so in its own header; a swap of two words is still six facings naming six distinct faces. **This names a hole P2 cannot close** — P3's FR-1.3-S2 is the only witness in the workspace for the axis assignment, and until it lands the mapping is held by a reader alone |
| M7 | `appearance_of` folds only the `up` key | FR-9.1-S1 reddens | **Bit: 2 of 1246** — FR-9.1-S1 as named, and FR-9.1-S2 with it. Both halves of `save_per_face_appearance` and nothing else: the shipped blocks all state one string, so the guards over *them* cannot see a fold that reads one key. That is exactly why FR-9.1-S1/S2 are written over six distinct keys |
| M8 | Leave the appearance revision at 1 | FR-9.1-S3 reddens | **Bit: 2 of 1246 — and the prediction is wrong about which.** FR-9.1-S3 stayed **green**: with six keys under revision 1 the fold still differs from the one-key fold a pre-spec save recorded, so the older save still reports its blocks retextured. What reddens is the two byte-sequence oracles — `format_test`'s appearance half and `save_per_face_appearance`'s stated-bytes guard. **The revision byte is visible only to a test that states the byte sequence**, never to one comparing two folds |
| M9 | `DeclaredBehaviour` gains a field | FR-9.1-S4 and T02's behaviour half redden | **Bit: 5 of 1246.** Both as named, plus `save_declarations::a_solid_breakable_block_that_breaks_into_nothing_records_the_version_1_behaviour` and the other two `shipped_declarations_and_an_older_save` guards. This is the mutation the per-list revision byte exists to make impossible to take by accident, and five witnesses see it |
| M10 | `drawn_of` compares only the `up` key | the section-count assertions redden. **Expect this one to be load-sensitive**; re-run before diagnosing | **Did NOT bite: 0 of 1246, and this one is a real gap rather than a structural non-bite.** The prediction and T10's "no new scenario is owed" are both wrong. **No fixture in the workspace reloads content that re-points one facing and leaves the other five alone**, so `at(Face::Up)` and the whole `FaceTextures` compare identically everywhere — a fixture property no assertion can see. `changes_geometry` could narrow back to one key tomorrow with the suite green, while a reload changing `north` would be accepted and mark no section at all. (The single red in that run was `a_declaration_that_allocates_past_the_memory_cap_is_refused_naming_its_file`, which passed on re-run **with M10 still applied** — load-sensitive, not M10.) Raised to P2's test author, who authored the witness (`854d894`). **M10 re-run against it: 1 of 1248 — `a_candidate_re_pointing_one_facing_of_a_block_leaves_every_section_of_the_world_to_mesh`, alone**, with its nothing-changed control green. The gap is closed and the closing is measured rather than asserted |
| M11 | `texture_keys` unions only `Face::Up`'s key | FR-1.1-S4 and FR-1.4-S1 redden | **Bit: 4 of 1246** — all of `per_face_layers` except its shipped-root control, which is correct: that control's root declares four blocks of one key each, so a union that reads one key per block still answers four |

**They did bite wider than predicted**, in the shape the note above expects: M4
and M9 both funnel through one accessor that many scenarios reach.

### The falsifier the two metatable guards were owed

`test-map.md` recorded both raw-read guards as **green on arrival for the wrong
reason** — under a skeleton refusing every table, a guard asserting that a table
is refused cannot tell a raw read from a believed metatable — and named the
falsifier owed. Both were run, whole workspace, `--no-fail-fast`, reverted by
hand. Each bit **exactly once**, and
`a_table_of_six_different_keys_registers_each_facing_with_the_key_written_against_it`
stayed green through both.

| # | Falsifier | Outcome |
|---|---|---|
| F1 | A facing read through a path that believes `__index`: a non-raw read added to `mc-script` for the run, with the statedness check consulting it | **Bit: 1 of 1246** — `a_declarations_metatable_supplies_no_facing_it_did_not_state`, alone. The table is then accepted and the block registers with an `up` key its author never wrote, which is the silent loss the guard names |
| F2 | The table enumerated through a path that believes `__iter`: the metamethod invoked rather than a raw walk | **Bit: 1 of 1246** — `a_declarations_metatable_hides_no_facing_it_did_state`, alone. The `top` the table really holds goes unseen and the declaration is accepted |

The isolation is the result worth keeping: neither guard reddens for the other's
reason, so the two halves of the raw-read property have a witness each rather
than one guard doing both jobs and neither being falsifiable.

---

## Phase 3 — Resolution at packing, and at the indicator

**12 scenarios.** Done means: a drawn face and the held-block indicator both resolve
their layer from the block's declaration and its facing, and **neither parses a
block name afterwards**. PRO-902 closes at both sites.

**Picture: unchanged.** All four shipped blocks declare `texture == name`, so every
key resolves to the layer it already had. A golden mismatch in this phase is a
wiring defect, never art.

**Red on arrival**: the skeleton resolves every facing to `Face::Up`'s key. It
reddens FR-1.3-S2/S3 and FR-2.2-S1. A skeleton that keeps parsing the name leaves
FR-1.3 entirely green — every shipped block declares its own name — and is the
wrong one, so **both** were applied and `test-map.md` carries a column each.

**Two predictions in the paragraph above were measured false and are corrected
here rather than left to be met a second time.** FR-1.3-S1 was predicted green as
a control and **arrives red under both skeletons**: the prediction was right about
what a control is for and wrong about this scenario having one clause — it also
says *and on no other face*, which a skeleton drawing `up`'s key everywhere
fails. And **FR-2.1-S4 arrives red under both skeletons** for a reason that is not
option (a) — neither content's key reaches a corner — so its arrival colour says
nothing about the design question either way. M14′ is the only instrument that
does.

- [x] **T12** Split `app/mod.rs` on a responsibility boundary, before anything else touches it
      — `crates/mc-client/src/app/report.rs` (new),
      `crates/mc-client/src/app/mod.rs`
      Scenarios: —
      - **This task exists because the file is at exactly 500 of 500 and P3 must
        touch it** (Trap 1). It is first in the phase and nothing else in P3 may
        edit `app/mod.rs` before it lands. **It owns no scenario, changes no
        behaviour, and the phase's test author authors nothing for it**: the suite
        is green either side and it commits as `refactor:`, on its own.
      - **The cut is stated here and is a responsibility boundary, not a line
        count.** Extract `report`, `report_swatch` and `report_remesh` — the three
        recurring-fault reporters, with the three `reported*` fields they own — into
        `app/report.rs`. Their shared responsibility is one sentence: *say a
        recurring fault once, however many frames repeat it, and never end the run
        for it.* None of them touches `wgpu`, a `Session`, or a snapshot; all three
        write one line to stderr and dedup against their own last message.
      - **Three reasons this is the right seam, in order of weight.** First, the
        module's own doc comments already argue these three are one considered
        policy — `report_swatch` is documented as "separate from `report_remesh`
        rather than folded into it", and that argument is about the group, which is
        the sign the group is a unit. Second, `app/` already decomposes this way and
        states the rule: `app/reload.rs`'s header says "a child module rather than a
        sibling because it writes the fields `App` owns". Third, it is the cluster
        **P3 itself perturbs** — `report_swatch`'s text changes when `held_swatch`
        takes a resolution and `HeldSwatch` gains the face it looked at (T18), so
        the extraction and the change land in the same phase rather than the change
        landing on top of an unplanned one.
      - **`report_reload` stays in `app/reload.rs` and is not gathered with the
        other three.** That module's stated contract is `App`'s *whole* share of a
        reload, and pulling its reporter out would break the contract to satisfy a
        symmetry. If a future reader wants all four dedup reporters in one place,
        that is a change to `reload.rs`'s contract and belongs to whoever takes it —
        deliberately, not while doing something else.
      - The new module's header states the shared policy and why there are three
        reporters and not one: **a third recurring fault must not silence the other
        two**, and "an edit could not be shown" is the wrong sentence for a block
        whose texture occupies no layer. `app/mod.rs`'s header keeps its "one frame
        is one call" argument, which is why the frame sequence itself — `redraw` →
        `exchange_remesh` → `present` → `draw` — is **not** what gets split.
      - **Record the headroom this leaves, measured with the gate's own counter,**
        in `## Notes` and in the closing report. The cluster is 41 non-blank lines
        at `87bbb84`, so `mod.rs` is expected to land near 460 — but the number that
        goes in the record is the one measured after the split, not this estimate.
        The next person to add a line needs to know what they have.

        ```powershell
        (Get-Content -LiteralPath crates/mc-client/src/app/mod.rs | Measure-Object -Line).Lines
        ```

- [x] **T13** `TextureResolution` — one value carrying the block→faces map and the layers
      — `crates/mc-render/src/texture/resolution.rs` (new),
      `crates/mc-render/src/texture/mod.rs`
      Scenarios: —
      - D2 is binding **and it is a type decision, not the option decision** —
        option (b) is the spec's. Two loose values travelling side by side through
        `PreparedScene`, `Unuploaded`, `Retained`, `Remesher::retire` and
        `FrameRenderer` create a new defect class: a batch packed with a reload's
        *new* keys against its *old* layers resolves to a wrong-but-valid layer, a
        plausible wrong picture with no error anywhere.
      - `TextureLayers` is **unchanged** — the spec's "unchanged contract" holds
        literally. `TextureResolution` is what travels; `TextureLayers` is what it
        contains and what fills the array texture.
      - **`TextureResolution` deliberately carries no `ContentSerial`, and that
        absence is load-bearing.** A bundled value invites being stamped with one so
        that "packed against the content serving" becomes checkable, and it must not
        be: FR-2.1-S4 depends on retained quads being packed against a *newer*
        resolution than the one they were meshed under.
      - New file rather than growing `texture/mod.rs` (117 lines): that module's
        header is a sustained argument about the layer assignment being stated
        rather than derived, and a second concern lands better beside it than inside
        it.

- [x] **T14** The six facing words map to the world's axes, and content states it
      — `crates/mc-render/src/geometry/mod.rs`,
      `crates/mc-world/src/mesh/facing.rs`
      Scenarios: FR-1.3-S1, FR-1.3-S2, FR-1.3-S3
      Depends on: T13
      - **These three are the only witnesses in the workspace for the axis
        assignment.** P2's round trip cannot see a north/south swap (M6); this is
        where it reddens. Author them against a block placed in the world and a face
        read back, not against `Facing::face` directly — a test calling the mapping
        is agreement between two copies of one decision.
      - The published contract is *stated by content and never inferred by the
        engine*, which is what FR-1.3 means and what makes T04's exhaustive `match`
        the requirement itself rather than an implementation of it.

- [x] **T15** A face draws the key its block declared, and an uncovered key refuses the section
      — `crates/mc-render/src/geometry/mod.rs`
      Scenarios: FR-2.1-S1, FR-2.1-S2, FR-2.1-S3
      Depends on: T13
      - `layer_for` (`geometry/mod.rs:180-192`) currently parses `quad.block` as a
        texture key and its comment says why that was true. **The comment goes with
        the parse.** `build_section_geometry` takes a `&TextureResolution`;
        `PLANE_AXES`, `QUAD_INDEX_PATTERN`, the winding derivation and `layer_at`
        are untouched.
      - `GeometryError::UnresolvedTexture { block }` becomes
        `{ block, face, key: Option<TextureKey> }`. A block with six keys leaves a
        reader guessing otherwise, and FR-2.1-S3 requires both the block **and** the
        facing to be named.
      - FR-2.1-S3 says *rather than drawing layer zero*. An implementation that
        falls back to zero draws a plausible picture and reports nothing, which is
        the failure this scenario is shaped against.
      - FR-2.1-S1 is one of PRO-902's two witnesses: the block is named
        `example:amber` and declares `example:gold`, so a name-parsing implementation
        resolves the wrong layer or none.
      - FR-2.1-S2's "one layer spent" is derived: two blocks, one distinct key.

- [x] **T16** The retained-packing seam — sections re-packed against a resolution they were not meshed under
      — `crates/mc-client/src/remesh.rs`, `crates/mc-render/src/geometry/mod.rs`
      Scenarios: FR-2.1-S4
      Depends on: T15
      - **Read Trap 5 before writing anything here.** This is the only witness
        separating option (a) from option (b), and a test written against a reload
        passes under both.
      - Retain a meshed list built under content A, hand the packer content B's
        resolution, re-pack, read the layer back through `SectionGeometry::layer_at`.
        `Retained::rebuilt` re-packs the entire retained list on every batch, which
        is the production behaviour being driven and not a fixture convenience.
      - `Unuploaded`'s one-way door stays: `uploaded_to` remains the only route to
        an owned value. Check this holds after the ripple, because the compiler will
        not.

- [x] **T17** A reload naming an uncovered key is refused, and the sections keep drawing what they drew
      — `crates/mc-client/src/session/reload.rs`, `crates/mc-client/src/app/reload.rs`
      Scenarios: FR-2.1-S5
      Depends on: T15
      - The all-or-nothing half. The refusal is the existing reload refusal reached
        by a new route; what this scenario adds is that the *picture* is untouched,
        not merely that the content was not adopted.

- [x] **T18** The held-block indicator resolves the same way the world does
      — `crates/mc-render/src/hud/held.rs`, `crates/mc-render/src/gpu/hud.rs`,
      `crates/mc-client/src/app/mod.rs`, `crates/mc-client/src/app/report.rs`
      Scenarios: FR-2.2-S1, FR-2.2-S2, FR-2.2-S3, FR-2.2-S4
      Depends on: T12, T13
      - **The second PRO-902 site, and closing only one is worse than closing
        neither** — `crates/mc-render/CLAUDE.md` warns that it leaves a block drawing
        in-world with a blank indicator. Both are replaced by the same lookup and
        neither parses a block name afterwards. Delete that warning in T19, with the
        gap it warned about.
      - `INDICATOR_FACE: Face = Face::North`, named once with its reason: a side face
        makes the canonical block recognisable — grass's side carries both the growth
        and the earth, and a top-only indicator shows a green square that says
        "grass" only to someone who already knows. `north` because the four sides are
        interchangeable for this purpose and an arbitrary choice made once and
        written down beats one made implicitly.
      - `HeldSwatch`'s three arms stay total (FR-2.2-S4) and gain the face they were
        looking at. `FrameRenderer::texture_layers()` becomes `texture_resolution()`
        — what a swatch is looked up in has to be what the array texture was filled
        from, which is why it is lent from there.
      - **T12 has already made room in `app/mod.rs`, and this task is why it did.**
        `swatch` (`app/mod.rs:339`) changes its lookup and `report_swatch` — now in
        `app/report.rs` — changes its text, because `HeldSwatch` gains the face it
        was looking at. Re-measure both files with the gate's counter before
        writing; if T12 has not landed, stop rather than growing the file.

- [x] **T19** [P] Resolution at packing, and the item a future change must not break
      — `docs/technical/rendering.md`, `crates/mc-render/CLAUDE.md`
      Scenarios: —
      Depends on: T16, T18
      - Delete `crates/mc-render/CLAUDE.md`'s "Known gap" section. Both sites are
        closed, so the warning goes with them.
      - **Engine** (`rendering.md`): a `Quad` carries no resolved texture; resolution
        happens where vertices are built, because a retained mesh is re-packed and
        never re-resolved at mesh time. Stamping a key into a `Quad` re-introduces a
        stale key on the one path built not to re-mesh.
      - **And, in the "a reload that changes what is drawn re-meshes the whole world"
        section, verbatim:** a texture-key change marks every section and
        `take_remesh_work` drains the whole dirty set into one batch; those two facts
        together are what make the retained-but-not-re-meshed state unreachable in
        production today. **Bounding the re-mesh batch turns it into a production
        path** — and a whole-world re-mesh measured at 9.1 ms is exactly the thing
        somebody will one day bound. Whoever does must make the retained sections
        re-pack against the serving resolution before that batch is drawn.
      - That sentence is the single most important thing this spec has to leave
        behind, and the spec folder is archived and pruned. It must not be what the
        prune deletes.

### Mutations — P3

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
Baseline either side of every row: **1261 tests run, 1261 passed, 1 skipped, 0
failed.** Every run is `cargo nextest run --workspace --all-features
--no-fail-fast`; every revert was re-edited by hand and confirmed with
`git diff --exit-code`.

**M14's row is corrected rather than merely annotated, and the mandate has moved
to M14′.** Measured twice independently — by the phase's test author in a detached
worktree at `6f51361`, and by the implementer against the landed implementation —
with the same numbers both times. `test-map.md` carries the same two rows from the
test author's side; what is below is the implementer's own run and is not a copy
of it.

**A throwaway reference implementation of the phase runs 1261 of 1261 green**, so
nothing authored here is red-that-should-be-green and the `testing.md` §2 trap
whose cheapest repair is to break working code is excluded by measurement rather
than by inspection.

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
| M12 | `layer_for` parses `quad.block` as a key again | FR-2.1-S1 reddens; every FR-2.2 stays green — the two sites are independent and this proves it | **bit, 9 of 1261.** FR-2.1-S1 red as predicted and **every FR-2.2 green** as predicted. Wider than the row expected: FR-1.3-S1/S2/S3, FR-2.1-S3, FR-2.1-S4, FR-2.1-S5 and both geometry additional-coverage tests went with it, because a name-parsing packer answers wrongly for every facing at once |
| M13 | `held_swatch` parses the block name again | FR-2.2-S2 reddens; FR-2.1 stays green | **bit, 5 of 1261.** FR-2.2-S1/S2/S3/S4 and the `ContentView` indicator inversion; **FR-2.1 entirely green**, which is the other half of the independence M12 showed. `the_unresolved_report_names_the_facing_the_indicator_looked_at` stayed green and correctly so — it pins the stated facing, not the lookup |
| M14 | `Retained::rebuilt` re-packs against the resolution the quads were meshed under — i.e. the worker never adopts a retired resolution | **Bites 2, and FR-2.1-S4 is not among them.** ~~FR-2.1-S4 reddens; this is option (a) and if it does not bite, the phase is not done~~ — **withdrawn**, and the strikethrough is kept because a prediction that would have sent somebody hunting a red that cannot appear, and then told them the phase was not done, is worth leaving visible. M14 is real coverage of a real property; it is not the option-(a) question | **bit, 2 of 1261 — and FR-2.1-S4 is not among them.** Both are in `reload_supersedes_a_batch_in_flight`. The row is aimed at the wrong site: FR-2.1-S4 packs the retained list itself, as Trap 5 prescribes, so `rebuilt` never runs. See M14′ |
| M14′ | option (a) at the packer: `build_section_geometry` freezes the resolution it is first handed and resolves every later section from it, which is what a quad remembering a mesh-time key amounts to at that seam | **Bites 4, FR-2.1-S4 among them, with FR-2.1-S1 and FR-2.1-S2 green** — the green half is what says it is aimed at the retained list rather than at resolution in general. **The mandate moved here from M14: if M14′ does not bite, the phase is not done.** | **bit, 4 of 1261, FR-2.1-S4 among them**, and FR-2.1-S1 and S2 stayed green — which is what says it is aimed at the retained list rather than at resolution in general. FR-2.1-S4's red arrives as the refusal `UnresolvedTexture { block: base:stone, face: North, key: Some(base:cobalt) }` rather than as the assertion diff: the meshing key is retired from the serving assignment, so the meshed key resolves to no layer at all under the serving content. The other three reds — `reload_draws_the_new_block` and the two supersede readings — are artefacts of the stand-in's process-wide scope and are **not** what real option (a) would redden |
| M15 | `INDICATOR_FACE` = `Face::Up` | FR-2.2-S1 reddens | **bit, 3 of 1261.** FR-2.2-S1 as predicted, plus FR-2.2-S3 (the unstocked block's `up` key *is* covered, which is the fixture constraint doing its job) and `the_unresolved_report_names_the_facing_the_indicator_looked_at`, which exists for exactly this |
| M16 | `Facing::face` swaps `north` and `south` (M6 re-run) | FR-1.3-S2 reddens — the hole P2 could not close | **bit, 3 of 1261. The hole is closed.** M6 measured 0 of 1246 in P2. FR-1.3-S2 as predicted, plus FR-2.1-S3 and FR-2.1-S5, both of which name a facing |
| M17 | `layer_for` falls back to layer zero instead of refusing | FR-2.1-S3 reddens; the goldens stay green, which is exactly why S3 is worded *rather than drawing layer zero* | **bit, 4 of 1261.** FR-2.1-S3 as predicted, and **every golden stayed green**, as the row argued. Also `a_quad_naming_a_block_the_content_states_nothing_about_is_refused_with_no_key`, the `stated_layers` refusal reading, and one supersede reading. FR-2.1-S5 stayed green: its serving key *is* covered, so no fallback is reached |

---

## Phase 4 — retired, and its work already done

FR-8.2 was retired on 2026-08-18 with all seven of its scenarios; `spec.md`'s
FR-8.2 entry carries the retirement, what each scenario asked for, and the
owner's ruling. What survived the retirement was not a phase but five small
documentation edits with no scenarios, no tests, no mutations and no picture,
so they were **made directly rather than run as a phase**: `grass-block.mcvox`'s
citation repaired to the generators that are actually there, a provenance header
on each of `gen_grass.py`, `gen_stone.py` and `assemble_grass.py` saying each ran
once and is not maintained, and a `## Where the shipped models came from` section
in `docs/modding/voxel-models.md` stating that the tracked `.mcvox` file is the
source of truth and the generators are not a build step. That section also
carries the spike's byte-for-byte measurement, since `architecture.md` is
archived and then pruned.

**The number 4 and the task numbers T20–T24 are deliberately left unused**, so
every reference elsewhere still means what it meant. P5 begins at T25. `-ArtOnly`
and the PowerShell-driving harness moved to P6's T36; `-GeneratorRoot` died with
gate stage 8.

The scripts were kept rather than deleted for one reason: the five hand-authored
courses in `assemble_grass.py` (`y = 10`…`14` — the sod shadow and three lone
blades at deliberately different depths) are design intent recorded nowhere else.
The model carries the result and not the reasoning.

---

## Phase 5 — `voxforge build`

**28 scenarios**, the largest phase. Done means: a mod author bakes a model's faces
into a named set with one command, the set is reproducible byte for byte, every
refusal names what is wrong, and the index records a fold over exactly the sources
the manifest reached.

**Picture: unchanged.** Nothing consumes the set yet; the client is untouched.

**If this phase needs splitting, the seam is FR-3.1/FR-3.2 (write and cache) before
FR-3.3/FR-3.4 (refuse and fold)** — the architecture names that seam and no other.

**Red on arrival**: `build` exists and writes nothing. That reddens FR-3.1-S1/S2/S3
and every FR-3.3 refusal by message, and leaves FR-3.1-S6 (zero entries) green as a
control — which is exactly the shape `testing.md` warns about, so **a second,
over-eager skeleton is owed** for the scenarios asserting a refusal writes nothing:
a build that writes all seven images before checking anything reddens FR-3.3-S10
and FR-3.3-S2's "and write no images".

**Corrected 2026-08-18, measured: FR-3.1-S6 is red on arrival, not green.** The
prediction above describes a weaker test than the one this file itself demanded
in T28 — the index asserted *present* and naming nothing. The test author wrote
that one, so the reading is `Naming([])` against `Absent`, which no build that
writes nothing can satisfy. The control the phase was warned about is closed and
there is no vacuity-green left in FR-3.1. The sentence is corrected rather than
deleted, because a prediction that missed is evidence about how the phase was
read.

- [x] **T25** `mc_core::art` — the index is a line format, parsed by neither side twice
      — `crates/mc-core/src/art.rs` (new), `crates/mc-core/src/lib.rs`,
      `crates/mc-core/tests/`
      Scenarios: —
      Depends on: T01
      - D7 is binding and its constraints are read from the tree: voxforge writes
        the index, `mc-client` reads it, neither may depend on the other (SPEC-013
        FR-9.1, enforced by `crates/mc-testkit/tests/workspace_layering.rs`), and
        **`toml` may not reach `mc-core`** (stated at `mc-core/src/hud/mod.rs:5`).
        A pure parse/render pair over `&str`/`String`, **no new dependency**, and
        `mc-core` opens no file.
      - `key <image> <key>` — image name first, key last — **because a `TextureKey`
        may contain whitespace and an image name may not.** `namespaced.rs:48` says
        outright that no character set is imposed: a key is `namespace:path` with
        exactly one separator and both sides non-empty, and nothing else. A key may
        therefore contain whitespace, a newline, a path separator or `..`.
      - **Refused on both sides, render and parse: any ASCII control character in a
        key or a source path.** Without it, `key = "base:a\nfold 0000000000000000"`
        is a spellable manifest entry making `rendered()` emit an index `parse` reads
        with a forged fold or a forged extra source.
      - **Additional coverage, one separate test function per `IndexError` arm** —
        `NotAnIndex`, `UnknownRecord`, `MalformedFold`, `UnsafePath`,
        `ControlCharacter`, `DuplicateKey` — plus a render→parse round trip. An
        enumerated error with one arm unconstructible in a test is an arm nobody has
        read.
      - **Decided, 2026-08-18: a `key` record whose key is not a namespaced id is
        refused as `UnknownRecord { word: "key" }`.** The six arms leave that case
        homeless and no test asserts it, so it was the implementer's call. A seventh
        arm was rejected on this task's own rule — it is the one arm no test in this
        phase could construct, and the tests are not the implementer's to add. What
        the reader is told is that a line beginning `key` is not one this format can
        read, which is true of it; the arm's message is worded to carry both cases
        rather than only the unknown-word one. Recorded on the mapping site in
        `art.rs` as well as here.

- [x] **T26** The fold's byte sequence, stated so a test can build an independent oracle
      — `crates/mc-core/src/art.rs`, `crates/mc-core/tests/`
      Scenarios: FR-3.4-S4
      Depends on: T25
      - D11 states the sequence and it is binding: for each source in order, the
        recorded path as UTF-8 bytes preceded by its length as a little-endian `u64`;
        then the file's bytes preceded by their length as a little-endian `u64`.
        FNV-1a-64 over that concatenation, offset basis `0xcbf29ce484222325`, prime
        `0x100000001b3`.
      - **Length prefixes rather than separators**, so a file containing the
        separator byte cannot forge a boundary. That is the reasoning
        `format.rs:330-333` already gives for postcard's own length prefixes.
      - Source order: the manifest first; then each model in the order of its first
        mention, de-duplicated; then **every `*.toml` in the materials directory,
        sorted by file name**. Every material `.toml` and nothing that is not one,
        because `load_materials` (`tools/voxforge/src/material.rs:167`) filters on
        the extension before sorting — so `*.toml` is exactly the build's input.
      - **The expected value is arithmetic in the test, never a run's output.**
        FR-3.4-S4's whole point is *rather than a value derived from the standard
        library's hasher*, and a snapshotted number satisfies both implementations.
      - `folded_sources(sources: &[(&str, &[u8])])` is where both sides meet: each
        reads its own bytes and hands them here.

- [x] **T27** The manifest, and the four refusals a manifest can raise before a model is opened
      — `tools/voxforge/src/texture/manifest.rs` (new),
      `tools/voxforge/src/cli/args.rs`, `content/base/textures.toml` (new), `.gitignore`
      Scenarios: FR-3.3-S1, FR-3.3-S2, FR-3.3-S3, FR-3.3-S4, FR-3.3-S5, FR-3.3-S6
      Depends on: T25
      - `build` is a fourth `Command` variant beside `preview`, `inspect`, `texture`
        (`cli/args.rs:41-48`). Its body lives in a new `cli/build.rs` —
        `cli/mod.rs` is at 365 non-blank lines against a 500 limit and this would not
        fit. `Command::document()` returns the manifest path for `Build`; `states()`
        returns `&[]`, because a manifest states no part states.
      - Every DTO field is `Option<toml::Value>` with `deny_unknown_fields` — the
        shape `format/dto.rs` already uses — so a manifest key nobody recognises is
        refused in the author's terms rather than serde's.
      - `face` parses through `AxisAlignedView`'s own vocabulary (`texture/set.rs:24-30`):
        `front`, `back`, `left`, `right`, `top`, `bottom`. **Not through `view_named`**,
        which would name the isometric views too, so FR-3.3-S4's refusal would list
        faces a manifest may not select.
      - `pixels_per_voxel = 1` is stated at the top level of the committed manifest,
        so the 16×16 decision is visible in a reviewable file rather than in a
        default.
      - `.gitignore` gains `/content/base/textures/` — **the directory**, so a stray
        file in it is ignored too, and the manifest one level up is untouched.
      - **Re-check, do not assume**, that `content/base/textures.toml` is invisible
        to all three content scanners. The spec verified this for the *models*
        directory; the manifest is a file at the root of the content directory and
        the three read `<root>/blocks` and `<root>/hud`, one level, by extension.
        Architecture Assumption 4 names this as the implementer's to confirm.

- [x] **T28** `build` writes one image per entry and one index, and reports what it wrote
      — `tools/voxforge/src/cli/build.rs` (new), `tools/voxforge/src/texture/emit.rs`
      Scenarios: FR-3.1-S1, FR-3.1-S2, FR-3.1-S3, FR-3.1-S5, FR-3.1-S6
      Depends on: T27
      - **D12: group entries by model.** For each distinct model, load and assemble
        once, call `emit` with `FaceSelection::All` and `SeamPolicy::Reported`. That
        is not an optimisation — the cubic precondition lives in `emit`'s
        `FaceSelection::All` arm only (`emit.rs:120-151`), so a per-entry
        `FaceSelection::One` would never reach it and FR-3.3-S9 would be
        unreachable. It also removes six renders per model.
      - PNG encoding is unchanged (`tools/voxforge/src/render/mod.rs:394`), already
        graded for determinism by `tests/preview_determinism.rs`.
      - FR-3.1-S6 (zero entries) is the vacuity control: an index naming zero keys,
        written, and zero images reported. An implementation that writes nothing at
        all also satisfies "zero images" — so **the index must be asserted present
        and well-formed**, not merely the count.
      - FR-3.1-S5 is the grass/dirt case that P9 depends on: the `bottom` face of
        `grass-block.mcvox` becomes `base:dirt`'s art.

- [x] **T29** [P] The same sources produce the same bytes, twice
      — `tools/voxforge/tests/`
      Scenarios: FR-3.1-S4
      Depends on: T28
      - Byte-identical, both runs, images and index. **This scenario plus the fold is
        the permanent successor to the twelve sha256 values in `requirements.md`
        §1.1** — the digests do **not** move into `docs/`. A table of numbers copied
        into a page is a second copy that must then be updated on every art change,
        and the copy that stops being updated is the one a reader trusts. The
        durable form of a measurement is the command that reproduces it.

- [x] **T30** A build that finds its output current does no work
      — `tools/voxforge/src/cli/build.rs`
      Scenarios: FR-3.2-S1, FR-3.2-S2, FR-3.2-S3
      Depends on: T28, T26
      - **D13: the fold is the cache key, and it is whole-set.** Fold the sources;
        if the value equals the index's recorded value **and** every image the index
        names is present, report that nothing needed rebuilding and touch no file.
        Otherwise rebuild the whole set. Per-entry caching was rejected: it needs a
        second, finer-grained record the client would then also have to understand,
        for seven 16×16 images.
      - FR-3.2-S1 asserts **bytes unchanged**, not mtimes. An implementation that
        rewrites identical bytes satisfies a naive reading and fails this one.

- [x] **T31** A model that will not bake to a block texture is refused before it is baked
      — `tools/voxforge/src/cli/build.rs`, `tools/voxforge/src/texture/seam.rs`
      Scenarios: FR-3.3-S7, FR-3.3-S8, FR-3.3-S9, FR-3.3-S14
      Depends on: T28
      - `SeamPolicy::Required` is FR-3.3-S7's refusal. The build refuses on the
        first failing seam verdict of a face **some entry selected**, and passes over
        verdicts on faces nobody asked for — refusing on those would refuse a build
        for a face the manifest never wanted. `EmittedFace.verdicts` is a
        never-empty enumeration (`emit.rs:70-77`), so "this tiles" is an answer
        rather than an absence.
      - **FR-3.3-S8 is FR-3.3-S7's positive control** and belongs in a separate test
        function: every selected face tiles across all four edges and the build
        completes.
      - **D5, FR-3.3-S14:** `model.scale × pixels_per_voxel ≠ TEXTURE_EDGE` refuses
        at build time, naming the model, the product and the edge. Without it a
        `scale = 32` model bakes a 32×32 set that builds cleanly, commits cleanly,
        passes the gate, and refuses the launch under FR-4.3-S1 with a message about
        an *image* — pointing a mod author at a file they never authored. voxforge
        already depends on `mc-core`, so this is one constant and no new edge.

- [x] **T32** A refused build changes nothing, and an unused key is named rather than refused
      — `tools/voxforge/src/cli/build.rs`, `tools/voxforge/src/cli/mod.rs`
      Scenarios: FR-3.3-S10, FR-3.3-S11
      Depends on: T31
      - `written_together` (`cli/mod.rs:301`) already deletes landed files when a
        later write fails. FR-3.3-S10 extends it across the whole set: the fourth of
        seven entries refused leaves the *previous build's* images and index
        unchanged. **This is what P6's FR-7.1-S6 depends on** — a refused build
        leaves a stale set on disk, which is why the gate must not then test against
        it.
      - **Trap 10 governs FR-3.3-S11**: a text scan over `blocks/*.luau`, advisory,
        never a refusal, with the computed-key limitation in a comment on the scan
        itself.

- [x] **T33** A key whose image name would not be a file name, or which would forge an index record
      — `tools/voxforge/src/cli/build.rs`, `crates/mc-core/src/art.rs`
      Scenarios: FR-3.3-S12, FR-3.3-S13
      Depends on: T27
      - D9. `image = key.replace(':', "__") + ".png"`, refused unless the result
        matches `[A-Za-z0-9._-]+\.png` and is neither `.` nor `..`. **The client
        never re-derives the name**: it reads it from the index and refuses any name
        failing the same shape.
      - **This is correctness and reproducibility, not a threat model.** A server
        owner chooses which mods are installed, so a declaration is not hostile
        input. The arguments that survive are plainer and stronger: a content string
        silently becoming a filesystem path is a foot-gun whatever the author
        intended, a key that will not round-trip through the index is a correctness
        defect in a file the client parses, and a build whose output location depends
        on punctuation in a key is not reproducible. `docs/modding/` says exactly
        that and no more.

- [x] **T34** The fold covers exactly the sources the manifest reached
      — `tools/voxforge/src/cli/build.rs`
      Scenarios: FR-3.4-S1, FR-3.4-S2, FR-3.4-S3, FR-3.4-S5
      Depends on: T26, T28
      - **Read Trap 8. FR-3.4-S3 is the negative control** and the only scenario
        that stops "fold the whole `content/` tree", which satisfies every positive
        staleness scenario while turning each unrelated content edit into a spurious
        launch refusal.
      - FR-3.4-S5: a source the manifest reaches that cannot be read refuses the
        build and names it. A fold that silently treats an unreadable source as empty
        records a value that is not a function of what the build consumed.
      - **D8: paths in the index are relative to the manifest's directory.** That is
        what makes a `copy_tree`'d test root re-fold to the same value inside a temp
        directory, keeping ~40 fixture tests green without being touched. Absolute
        paths would make a copied root permanently stale and would put developer home
        directories into a file the gate builds.
      - **All four of these arrive green, and this task is verification rather than
        implementation.** Leg one had to decide which sources are folded, because
        D13 makes that fold the cache key — so FR-3.4-S1, S2, S3 and S5 were
        satisfied by T30's work rather than by anything done here. **Their arrival
        red is on record and is not re-derivable at this point**: red under skeleton
        A (`Unavailable` against `Moved`/`Stayed`, and `Completed(0)` against
        `NamingEverything`), red again in the shared tree before leg one's
        implementation landed.
      - **So their non-vacuity here rests on M26 alone, and M26 is the blunt one.**
        It reddens FR-3.4-S3 and the `*.toml` reading as the row claims, but it also
        folds the build's own output directory and takes FR-3.2-S1 and S2 with it.
        Four scenarios green on arrival with one broad mutation under them is not
        the coverage a green column looks like, and the reading to make of a
        non-bite here is correspondingly weaker. **Run M26 and record what it does**
        — including if it does something other than what leg one measured, which was
        4 above its baseline where the test author measured 5 against the reference.

- [x] **T35** [P] `voxforge build`, end to end, for somebody who has never run it
      — `docs/modding/voxel-models.md`, `docs/technical/architecture.md`
      Scenarios: —
      Depends on: T32, T33, T34
      - **Mod author**: `voxforge build`, every manifest field with its type and its
        bound, the index's shape, **every refusal including the file-name rule and
        the edge rule and what each looks like**, the unused-key report and its
        computed-key limitation, and **a complete worked example from a model to a
        named key that runs**.
      - **Engine** (`architecture.md`): the index contract and the fold's byte
        sequence in `mc-core` — a contract between two programs that may not depend
        on each other, where a second implementation on either side is the defect the
        arrangement exists to make unspellable. `voxforge build` in the topology.
        `TEXTURE_EDGE` as a contract constant, asserted by the renderer at compile
        time and enforced by the build tool, because three numbers that can disagree
        is how a 32×32 set builds cleanly and refuses at launch.
      - State plainly that nothing consumes the set yet — the client learns to judge
        it in P7 and to draw it in P9.

### Mutations — P5

**Leg one's rows were re-run against the shipped implementation** in a detached
worktree at `87022ec` with its own target directory, `--no-fail-fast`, each
applied and reverted by hand with `git diff --exit-code` clean between. The
baseline both sides of every one: **1299 run, 1292 passed, 7 failed, 1 skipped**
— the seven being leg two's scenarios, red because their tasks have not been
implemented. "N new" below counts failures above that baseline.

**Leg two's rows were run the same way at `1d5966e`, in its own detached
worktree with its own target directory on `E:`.** Its baseline is
**1299 run, 1299 passed, 1 skipped** — the same denominator, with leg one's
seven now green — measured in that worktree before the first mutation and not
inherited from the shared tree. Reverts were by **re-editing the mutated lines**
rather than by `git checkout --`, each proved with `git diff --exit-code`.

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
| M24 | `folded_sources` uses `DefaultHasher` | FR-3.4-S4 reddens | **2 new**: FR-3.4-S4 and the length-prefix reading. **Green**: every build reading — the fold's value moves and nothing about the build does |
| M25 | Drop the length prefixes from the fold | **predicted non-bite** unless a test constructs the forging pair. If it does not bite, that is an additional-coverage test owed, not a shrug | **The prediction is wrong: it bites, 2 new** — the same two. The forging pair is constructed, so no additional-coverage test is owed. **Green**: every build reading |
| M26 | Fold every file under `content/` | FR-3.4-S3 reddens; every positive staleness scenario stays green — Trap 8 measured | **4 new**: FR-3.4-S3 and the `*.toml`-only reading, plus FR-3.2-S1 and S2 as collateral — folding everything folds the build's own output, so the set is never current. **Green**: FR-3.4-S1 and S2, which is the half the row is about. FR-3.3-S3 also stays green here and did not in the test author's run: this form *adds* the tree to the manifest's own source list rather than replacing it, so the missing model keeps the key that names it |
| ~~M27~~ | The cache reports "nothing needed rebuilding" unconditionally | FR-3.2-S3 reddens | **Mis-aimed, and replaced rather than reported. 15 new.** Read literally, no build ever writes anything, so nearly every reading in the phase reddens — and a mutation that reddens everything it touches says nothing about what it is aimed at |
| M27′ | The staleness comparison alone is skipped; a set that is not there is still not current | FR-3.2-S3 reddens | **3 new**: FR-3.2-S3, FR-3.4-S1 and FR-3.4-S2. **Green: FR-3.2-S1 and FR-3.2-S2**, which is what says the mutation moved the *staleness* answer and not the reporting |
| M28 | Bypass `written_together` and write each entry as it is emitted | FR-3.3-S10 reddens | **1 new**: FR-3.3-S10, and nothing else. **Green**: every other build reading, including both cache readings — the mutation moves *when* a file lands and not what is in it |
| M29 | Accept any derived image name | FR-3.3-S12 reddens | **1 new**: FR-3.3-S12. **Green**: FR-3.3-S13, which is the pair worth watching — the two refusals sit in one function and a mutation that took both would say nothing about either |
| M30′ | `emit` once per entry with that entry's own face | FR-3.3-S9 reddens — the cubic precondition becomes unreachable, which is why D12 groups. **The original M30 was mis-aimed** (17 of 1299) and the test author replaced it | **1 new**: FR-3.3-S9, exactly as aimed. **Green**: every image and index reading, which is what says the replacement is aimed where M30 was not — the images are still correct under it |
| M31 | Refuse on a seam verdict for a face no entry selected | FR-3.3-S8 reddens; FR-3.3-S7 stays green | **2 new**: FR-3.3-S8 as predicted, **and FR-3.3-S10**. **Green: FR-3.3-S7**, as predicted. The second is worth having: under this mutation the refusal comes out of `emit` naming a *face*, so the entry's key is missing from it and the all-or-nothing reading fails on `Missing(["base:k3"])` rather than on a file. That reading therefore witnesses the key-naming half of the seam refusal as well as the leaves-nothing half |
| M32 | The unused-key report becomes a refusal | FR-3.3-S11 reddens | **6 new**: both FR-3.3-S11 readings, **and all four shipped-root readings** — `a_manifest_of_seven_entries_*`, `every_written_path_*` and the determinism pair. Collateral, and it is a measurement rather than noise: **the committed manifest bakes five keys no block declaration spells today** (`base:grass_top` and the four `base:grass_side_*`), because the per-face declaration those keys are for is **T53's, in P9** — corrected from an earlier reading of this note that said P8, which is the mip chain and touches no declaration. A refusal there refuses the shipped build outright, which is precisely the outcome Trap 10 and FR-3.3-S11 exist to prevent |
| M36 | *(leg two's; the numbers in this file are per-phase, and P6's table below has an unrelated M36)* Drop the control-character refusal, leaving the file-name rule alone to judge a key | FR-3.3-S13 reddens — asked because both refusals live in one function and the derived name of a key carrying a line break *also* fails the file-name rule, so S13's green could have been S12's rule answering for it | **1 new**: FR-3.3-S13. Its green is its own: the file-name rule does refuse the same key, but with the wrong words, and the reading is on the words |
| M33 | *(test author's)* `rendered` drops the zero padding while `parse` stays strict | the round trip reddens — it is green under both skeletons | **1 new**: the round trip, and nothing else. **Green**: all six arm tests |
| M34 | *(test author's)* `parse` accepts any first line | `NotAnIndex` reddens — it is green under both skeletons | **1 new**: `a_first_line_that_is_not_the_magic_is_refused_naming_it`. **Green**: the round trip |
| M35 | *(test author's)* a current set is re-encoded and rewritten anyway | FR-3.2-S1 reddens; FR-3.2-S2 stays green | **1 new**: FR-3.2-S1. **Green**: FR-3.2-S2, as predicted — this is the mutation the tampered image exists for |

---

## Phase 6 — The gate builds the art and refuses a committed one

**6 scenarios.** Done means: the gate refuses a committed built image, builds the
set before it tests on **both** branches, and when the build refuses does not test
against the set built previously — recording the skip rather than omitting it.

**Picture: unchanged.**

**Read Trap 6 before starting.**

- [x] **T36** The generated set is absent from version control, and the gate enforces it — and `-ArtOnly` is born
      — `scripts/sdd-gate.ps1`, `crates/mc-client/tests/` (the PowerShell-driving harness, new)
      Scenarios: FR-7.1-S1, FR-7.1-S2
      Depends on: T27
      - Stage 7: fails when `git ls-files -- <ContentRoot>/textures` reports
        anything, **naming each path**. `-ContentRoot` (default `content/base`) is
        the fixture seam.
      - **This is where D15's fixture seams and the harness are introduced.** They
        were T23's until FR-8.2 was retired on 2026-08-18 and took gate stage 8
        with it; P6 is now the first stage that needs them. `-ArtOnly` runs the art
        stages and nothing else. `-GeneratorRoot` is **not** introduced — it
        existed only for stage 8 and is retired with it.
      - The stage sits **after** the `-Quick` early exit and **outside** the
        `if ($SkipCoverage)` block. That placement is what makes T37's FR-7.1-S3
        and S4 one placement rather than two.
      - A Rust test drives `pwsh -File scripts/sdd-gate.ps1 -ArtOnly -ContentRoot
        <temp>` against fixture trees and asserts **an enumerated verdict over exit
        code and output**, not an absence.
      - **Nothing in this repository has ever tested `scripts/sdd-gate.ps1`.** A
        structural text scan can answer ordering and guarding honestly; it cannot
        answer what a stage *does*. If the harness cannot be written honestly, say
        so and fall back to structural-only, **recording which scenarios are then
        unwitnessed** rather than letting the gap be silent.
      - **There is deliberately no `-RepoRoot`**, and it is not an oversight to be
        tidied later: pointing the whole script at a temporary tree makes
        `git ls-files` fail with "not a git repository" and `cargo run -p voxforge`
        fail with "no such package", so a clean fixture and a broken one both
        answer the wrong question. Each stage is parameterised on the path it
        inspects instead.
      - The gate script is PowerShell and the size stage only walks `*.rs`, so
        growth here is unconstrained by it. That is not a licence: keep each stage
        one responsibility.
      - **`git` runs against the real repository**, which is why the stage is
        parameterised on the path it inspects and not on a repository root. A
        temporary tree makes `git ls-files` fail with "not a git repository", so a
        clean fixture reddens for a reason unrelated to the property.
      - FR-7.1-S2 is the positive control and a separate test function: a tree
        carrying the manifest, the models and the materials and no built image
        passes.
      - **The stage is what keeps the `.gitignore` rule from drifting back.** Say so
        in the stage's own comment.

- [x] **T37** The gate builds the set before it tests, on both branches
      — `scripts/sdd-gate.ps1`
      Scenarios: FR-7.1-S3, FR-7.1-S4
      Depends on: T36
      - Stage 8 runs `cargo run -p voxforge --quiet -- build <Manifest>`, writing the
        tool's own output through. `-Manifest` defaults to
        `content/base/textures.toml`.
      - **One placement, two scenarios**: the stage sits after the `-Quick` early
        exit and **outside** the `if ($SkipCoverage)` block, so the instrumented path
        and the coverage-skipping path both reach it. An implementation that puts it
        inside one branch passes exactly one of S3 and S4 — and **which one it passes
        tells you which branch it landed in**.
      - The current tests+coverage stage becomes stage 9; renumber the banner
        comments so a reader's map matches the script.
      - **The shipped build prints five unused-key lines today and the stage must
        not read them as a failure.** `voxforge build content/base/textures.toml`
        completes with exit 0 and says `base:grass_top` and the four
        `base:grass_side_*` are baked and named by no block declaration. That is
        correct: the per-face declaration naming them is T53's, in P9, and
        FR-3.3-S11 makes the report advisory precisely so an art build is not held
        hostage to a block file nobody has written yet. A stage keying on
        non-empty output, or on stderr being silent, goes red on the base game's
        own art the day it lands — and whoever wrote it would be reading their
        stage rather than the five keys. **Key on the exit code.** Measured in P5
        by M32, which turned the report into a refusal and took four shipped-root
        readings and the build with it.

- [x] **T38** A refused build fails the gate and the test stage records that it did not run
      — `scripts/sdd-gate.ps1`
      Scenarios: FR-7.1-S5, FR-7.1-S6
      Depends on: T37
      - **Trap 6.** FR-7.1-S5: the stage reproduces the build's refusal — which is
        why the stage runs `cargo run -p voxforge` against the real workspace and
        not against a temporary tree, where the refusal would be "no such package".
      - FR-7.1-S6: stage 9 is skipped, and **the skip records itself** —
        `$Failures.Add('tests (not run: art build failed)')`. The script's structure
        already guarantees failing fast: every stage records into `$Failures` and the
        summary is `if ($Failures.Count -eq 0) { GATE PASSED; exit 0 }` followed by
        `exit 1` (`sdd-gate.ps1:398-408`), so a refused build exits non-zero whatever
        happens afterwards. What must be added is the record.
      - **The script header's "Every stage runs even if an earlier one fails" becomes
        false and must gain the exception in one sentence beside it**, not be left
        standing.
      - **Confirm by hand once**, as the architecture's Risks entry asks: break the
        manifest, run the gate, and check stage 9 is reported as *not run* rather
        than run and passing.

- [x] **T39** [P] Two stages, and the one exception to "every stage runs"
      — `docs/technical/testing.md`
      Scenarios: —
      Depends on: T38
      - **Engine**: the two new stages, what each inspects, the two fixture
        parameters and why there is no `-RepoRoot`, `-ArtOnly` as a stage selector
        beside `-Quick`, and the stated exception with its reason — a refused build
        leaves the previous set intact, so running the suite anyway grades a stale
        set.
      - Record that a gate that omits a stage silently is one step from a gate that
        skips its way to green, and that this is why the skip is recorded rather than
        merely performed.

### Mutations — P6

Run against the two gate test binaries (11 tests), in a detached worktree with
its own target directory, both removed afterwards. **The scoping is provable
rather than assumed**: `scripts/sdd-gate.ps1` is not compiled, and the only Rust
that opens it is `crates/mc-client/tests/gate/`, so no other test in the
workspace can see any of these mutations. The stated green for every row is the
arrival run in `test-map.md`; the 11-test green after each revert is what was
actually re-observed.

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
| M33 | Move stage 8 inside the `if ($SkipCoverage)` block | exactly one of FR-7.1-S3, FR-7.1-S4 reddens | **4 red**, and the aimed half is exactly as predicted: FR-7.1-S3 turns to `NoSetIsBuiltOnThisPath` while **FR-7.1-S4 stays green** — one of the two, and it is the instrumented one, which is what says which branch the build landed in. **Green**: FR-7.1-S1, S2, the `-Quick` placement, the header claim and both controls. The other three are collateral and are a measurement rather than noise: with the build inside a branch `-ArtOnly` does not take, stage 8 never runs at all, so FR-7.1-S5, S6 and the advisory-key reading lose their subject. The placement is therefore load-bearing twice over — for the coverage paths, and for the art stages being selectable at all |
| M34 | The skip does not add to `$Failures` | FR-7.1-S6 reddens | **2 red**: FR-7.1-S6 as aimed, arriving at `NothingRecordsThatTheTestsWereSkipped` — the reading the control fixture measured — **and FR-7.1-S5**, which is the overlap `test-map.md` already names: the two assert the same failure list and are separated by what else each asks. **Green**: the other nine, including S3, S4 and the advisory-key reading, which is what says the failure is about the record and not about the guard |
| M35 | Stage 9 runs anyway after a refused build — the guard broken off the chain, so every test branch is reached whatever the build did and the record merely sits beside it | FR-7.1-S6 reddens | **1 red, exactly as aimed**: FR-7.1-S6, at `SomeTestRunsWhateverTheArtBuildDid` — the reading the control fixture `RECORDING_A_SKIP_THAT_DOES_NOT_HAPPEN` measured. **Green**: all ten others, FR-7.1-S5 included — the failure list is still right, which is precisely why the structural half is owed its own reading. **The first spelling of this row was re-aimed** (below) |
| M35b | The guard keeps its shape and stops being about the build (`if ($false)`) | — (an added row) | **2 red**: FR-7.1-S6 and FR-7.1-S5, both on the failure list losing its second entry, while the structural verdict stays `TheTestsRunOnlyBesideTheRecordedSkip`. **This was the row's first spelling and it was re-aimed, not merely reported**: it turns the behavioural half of S6 and leaves the structural half — the half whose instrument is pre-verified — untouched, so it says nothing about what the scan grades. Kept as a row of its own because it is the witness that the two halves are independent: the scan cannot see a condition that stopped being about the build, and the run cannot see a chain that came apart |
| M36 | Stage 7 checks a path that does not exist | FR-7.1-S1 reddens; FR-7.1-S2 stays green — the narrow-instrument control | **1 red, exactly as aimed**: FR-7.1-S1, at `(EveryStageItRanPassed, 0, false)` — the stage passes and names nothing. **Green: FR-7.1-S2**, as predicted, and the other nine. This is the pair earning its keep: a stage aimed at a path nobody has answers a dirty tree and a clean one identically, and only the fixture differing by one tracked file can tell them apart |
| M37 | Stage 8 swallows voxforge's output | FR-7.1-S5 reddens | **2 red**: FR-7.1-S5 as aimed, at `(StagesFailed([…both stages…]), 1, false)` — the stage still fails with the right list and stops saying why — **and the advisory-key reading**, for the same reason from the other side: it too asserts the tool's own line reaches the reader. **Green**: FR-7.1-S6, whose failure list is unchanged, and the seven others. **The first application of this row landed malformed** — the edit produced `build $Manifest2>&1`, an undefined variable under `Set-StrictMode`, so the scriptblock died before running anything and `Invoke-Stage` read a stale `$LASTEXITCODE` and reported the stage **ok**. It was reverted by hand and re-run correctly. A landed edit is not a correct edit, and the harness comparing the file whole after writing cannot tell the two apart |

**A deferred observation, outside this diff.** The malformed M37 exposed that
`Invoke-Stage` decides on `$LASTEXITCODE`, so a scriptblock that fails *without*
running a native command inherits the previous command's exit code and the stage
reports ok. That is pre-existing helper behaviour, not something this phase
introduced or touched, and it is recorded rather than fixed in passing.

---

## Phase 7 — The client judges the set and refuses by name

**10 scenarios.** Done means: a contributor who has not run the build gets one
sentence telling them what to run, a content root declaring no art launches
normally, and the verdict is a **total enumeration returned in `Ok`**.

**Picture: unchanged. No texels reach the array texture yet** — the set is judged
and then not used, which is what keeps this phase picture-neutral. Handing the
texels on is P9's T48.

**Read Trap 7 before closing this phase.**

- [x] **T40** `built_set` — a six-arm verdict, returned rather than raised
      — `crates/mc-client/src/textures/mod.rs` (new),
      `crates/mc-client/src/textures/index.rs` (new)
      Scenarios: FR-5.1-S1, FR-5.1-S2, FR-5.1-S3, FR-5.1-S6, FR-5.1-S7, FR-5.1-S8
      Depends on: T25, T26
      - **D6: six arms** — `NoArtDeclared`, `Absent`, `StaleAgainstSources`,
        `SourceMissing`, `ImageMissing`, `Current` — with the presence of
        `content/base/textures.toml` separating the first two. Applied literally to
        four verdicts, a root with **no manifest at all** has an absent index and
        would be told to run the art build before content declaring no art will
        load, which blames the wrong party.
      - **`assert!(no_refusal_printed)` cannot tell a healthy set from a client that
        lost the ability to check.** A total enumeration reddens for free when the
        check stops looking, which is the whole reason FR-5.1 is written as one
        verdict per launch.
      - **The verdict is returned in `Ok`, never as an error.** Returning it only as
        an error leaves three arms unconstructible in `Ok`, and "a total enumeration,
        never an absence check" would not be what the suite was holding.
      - **The client re-folds the sources the index recorded, in the recorded order.
        It never reads the manifest.** Paths are relative to the manifest's directory
        and are resolved against the content root the client was given (D8) — which
        is what keeps every `copy_tree`'d fixture green.
      - FR-5.1-S7 (an index naming zero keys, current) is the vacuity control for
        `Current`.

- [x] **T41** A recorded source that is gone, and an index naming an image that is not there
      — `crates/mc-client/src/textures/mod.rs`
      Scenarios: FR-5.1-S4, FR-5.1-S5
      Depends on: T40
      - Both are arms that a naive implementation folds into `StaleAgainstSources`,
        and both are worded *rather than reporting it as current*. They are separate
        arms because they name a different thing to the person reading the message: a
        source that moved, versus an image that did not land.

- [x] **T42** The refusal a verdict becomes, and the launch that reads it
      — `crates/mc-client/src/startup.rs`, `crates/mc-client/src/textures/mod.rs`
      Scenarios: FR-5.2-S1, FR-5.2-S2
      Depends on: T41
      - `refusal_for(verdict) -> Option<PreparationError>` is a **separate named
        function**, and FR-5.2's two scenarios assert its text while FR-5.1's assert
        the verdict. Five new `PreparationError` variants beside `NoContentRoot` —
        five and not four, because an `Option<PathBuf>` cannot be conditionally
        rendered inside one `thiserror` format string.
      - `BUILD_THE_TEXTURE_SET` is a `const` beside `LOAD_CHANGED_BLOCKS`, **spelled
        once**, for the reason that constant carries: a message quoting a command
        nothing accepts reads as a way out and is not one. `README.md` and
        `docs/modding/voxel-models.md` quote the same string.
      - **FR-5.2 is the "these two messages must never collapse" pair**: "the build
        step was not run" refuses the launch by name; "this key was never authored"
        is a silent, documented per-key fallback a mod author hits on their first
        block. The second does not exist until P9 — S2 asserts only that the
        *refusal* is absent, which is what it can honestly assert here.
      - `prepare_scene` calls `built_set(root)` and maps the verdict through
        `refusal_for`. That is what makes P9's FR-7.2 true of the golden suites,
        since they go through `prepare_scene`.
      - **`startup.rs` is at 364 non-blank lines**; re-measure before writing.

- [x] **T43** [P] What a contributor sees, and what to run
      — `README.md`, `docs/modding/voxel-models.md`,
      `docs/technical/architecture.md`
      Scenarios: —
      Depends on: T42
      - **Contributor** (`README.md`): `cargo run -p voxforge -- build
        content/base/textures.toml`, one line above `cargo run -p mc-client`.
        ADR-026 accepts that `cargo build` alone no longer produces a complete game,
        and this refusal is the whole of the mitigation.
      - **Mod author** (`voxel-models.md`): each verdict that refuses, the sentence
        it produces, and how to clear it. Including `NoArtDeclared` — a root that
        ships no art launches, and that is deliberate.
      - **Engine** (`architecture.md`): the verdict is a total enumeration returned
        in `Ok`, never an error and never an absence check; a client that lost the
        ability to check must redden.
      - **Record Trap 7 in the closing report and in `docs/technical/testing.md`**: a
        bare `cargo nextest run` now fails without a built set, and that is FR-7.2-S2
        working. The gate is green because P6 taught it to build first.

### Mutations — P7

**Run in a detached worktree at `4c4a62b` plus the test author's then-uncommitted
files, with its own `CARGO_TARGET_DIR` on `E:`; both removed afterwards.** Every
row is a full `cargo nextest run --workspace --no-fail-fast`, reverted by hand
with `git diff --exit-code` clean between each.

**The GREEN every row is measured against: 1328 run, 1328 passed, 1 skipped, exit
0** — taken in that worktree before the first mutation and again after the last
revert, identical both times. It is 1328 rather than the main tree's 1329 because
the worktree predates one control the test author added afterwards, and it carries
the committed `content/base/blocks/water.luau` rather than the project owner's
in-flight edit, so the two `mc-world::shipped_declarations_and_an_older_save`
failures visible in the shared tree are absent from it.

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
| M38 | `built_set` returns `Current` unconditionally | FR-5.1-S1 through S5 redden; S6/S7 stay green — the total-enum property measured | **13 red.** Predicted five, and the direction is right. S1–S5 red **and S8**, both readings in `an_unreadable_set_names_what_it_cannot_read.rs`, FR-5.2-S1, and all four of `documented_refusals`. S6, S7, both `Current` controls and FR-5.2-S2 green. The prediction was written before S8 and the additional coverage existed |
| M39 | `refusal_for` returns `None` for `Absent` | FR-5.2-S1 reddens | **6 red.** FR-5.2-S1 as predicted, plus FR-5.1-S1 — which reads the *command* out of the refusal and has none to read — plus all four of `documented_refusals`, whose first built-set producer can no longer produce a line |
| M40 | Collapse `NoArtDeclared` into `Absent` | FR-5.1-S8 reddens | **2 red**, and the second is the interesting one. FR-5.1-S8 as predicted, **plus `one_content_path_to_the_registry::a_root_holding_only_a_declaration_in_the_retired_format_declares_no_block_at_all`** — a root built from scratch with no `textures.toml`, which is exactly the shape D6 added the arm for, reddening from a suite this phase never wrote a line of. Reproduced identically on a second run |
| M41 | Collapse `SourceMissing` into `StaleAgainstSources` | FR-5.1-S4 reddens | **2 red.** FR-5.1-S4 as predicted, plus `documented_refusals` — the page quotes the source-missing sentence and the client now prints the stale one. A second witness on that arm, through the documentation rather than through a verdict |
| M42 | The client re-reads the manifest instead of the index's recorded sources | predicted to redden FR-5.1-S3; **if it does not, the re-fold is not being driven** and that is a gap | **3 red, and the prediction was wrong about which.** `the_client_refolds_the_sources_the_index_recorded_and_never_reads_the_manifest` reddens — the re-fold *is* driven. **FR-5.1-S3 stays green**, and that is not a gap: the gained manifest entry names a model the manifest already reaches, so a re-derived list is the same list and the manifest's own changed bytes make the fold differ either way. The two that do bite are the additional-coverage test written for exactly this, and **FR-5.1-S4** — a directory scan cannot see a file that is gone, so the removed source drops out of the derived list and the set reads `StaleAgainstSources` instead of `SourceMissing` |
| M43 | Resolve index paths as absolute | predicted to redden broadly across the `copy_tree` fixtures — D8 measured | **65 red**, which is what "broadly" meant. Nine of `built_set_verdict` including both `Current` controls, both unreadable-set readings, both of FR-5.2, all four `documented_refusals` and the per-facing listing, and then everything that prepares a scene over a copied root: the goldens, the terrain probes, the HUD suites, the oracle, byte-determinism, the edit-geometry suites, the content refusals. D8 is load-bearing across the whole workspace and not only inside this phase |

**How M42 was written, since it decides what the row is worth.** The client has no
TOML parser and adding one to run a mutation would have measured a different tree.
What the mutation replaces is the half of the manifest read that the witness test
is about: the recorded source list is discarded for the materials, and every
`*.toml` under `<root>/materials/` is folded in sorted order instead — which is
`voxforge`'s own `material_declarations`, and therefore exactly what a client that
consulted the manifest would compute. The non-material sources are left as the
index records them, so the mutation isolates the re-derivation and changes nothing
else.

**M38 through M41 each reddened `documented_refusals` as well as their named
scenario, and that is worth reading as a result rather than as noise.** The page
guard is wired to a real run of `prepare_scene` through the fixtures the test
author added for T43, so a change to *what the client says* now reddens the
documentation as well as the verdict. Before this phase the modding pages quoted
three refusals of roughly fourteen; they now quote thirteen or fourteen of about
twenty.

**One row was run twice.** M40's revert asserted on a string that the mutation had
made ambiguous — `return Ok(SetVerdict::Absent);` appears twice once
`NoArtDeclared` is collapsed into it — so the revert refused, and the following
run re-measured M40 rather than M41. It reproduced exactly: same two tests, same
counts. The revert was then made against the surrounding `if` block, which is
unique. This is the failure mode the "read `git diff` before believing a mutation
run" rule exists for, caught by that rule.

---

## Phase 8 — The mip chain, as arithmetic

**5 scenarios.** Done means: the sRGB transfer pair, the box average in linear
light, the chain and the level-count refusal all exist as pure functions under
normal coverage — **and none of them is wired**.

**Picture: unchanged, and this is the phase where that is easiest to break.**
`mip_level_count` stays 1 and the sampler stays nearest. FR-6.2 is deliberately not
in this phase, because a `TERRAIN_SAMPLER` constant nothing consults is policy
without wiring — and policy without wiring is the failure `testing.md` §2 names.

- [x] **T44** The transfer pair and the box average, in linear light
      — `crates/mc-render/src/texture/mip.rs` (new),
      `crates/mc-render/src/texture/mip_test.rs` (new)
      Scenarios: FR-6.1-S1, FR-6.1-S2, FR-6.1-S3, FR-6.1-S4
      Depends on: T04
      - **The array texture is `Rgba8UnormSrgb` and decodes to linear on sample**;
        `crates/mc-render/src/gpu/buffers.rs:10-16` already records that this is
        load-bearing. Averaging the stored bytes of 0 and 255 gives 128, which
        decodes to linear 0.216 rather than 0.5 — every level would come out darker
        than the one above it. That is the classic sRGB mipping fault and is, in that
        module's own words about the neighbouring trap, "plausible-looking, and wrong
        in the direction nothing notices".
      - **FR-6.1-S2 pins the byte at 188 precisely because 128 is what the wrong
        implementation produces.** A scenario saying only "midway between" would
        accept both. Do not soften it.
      - `MIP_LEVELS = TEXTURE_EDGE.ilog2() + 1` — **derived, never written as `5`.**
        A size and a level count that can disagree is a copy that overruns.
      - FR-6.1-S3 (uniform colour survives every level) and FR-6.1-S4 (each 2×2 texel
        averages exactly the four it covers, *and not any other four*) are a pair: S3
        stays green under an off-by-one in the four-texel selection and S4 is what
        catches it.
      - **Measure the arithmetic path before choosing any tolerance.** These are
        `f32` round trips through a transfer function; an exact comparison that
        happens to be the *consistent* one with neighbouring tests can still fail
        against a correct implementation. Derive a tolerance from both directions —
        above the measured error, below the smallest difference the test must still
        catch — never by loosening until green.

- [x] **T45** `levels_for` — supplied or placeholder, and a layer with too few levels is refused
      — `crates/mc-render/src/texture/mip.rs`,
      `crates/mc-render/src/texture/supplied.rs` (new)
      Scenarios: FR-6.1-S5
      Depends on: T44
      - `SuppliedTexels` lands here as the pure value (`stating`, `none`,
        `covering`), because `levels_for` needs it. **It is not wired to anything** —
        `FrameRenderer` gains it in P9.
      - `TextureError::TooFewLevels { key, offered, declared }` and
        `WrongTexelCount { key, offered, declared }`. **FR-6.1-S5's refusal is
        reachable with no device**, which is why this whole module sits outside
        `src/gpu/` and under normal coverage. `crates/mc-render/CLAUDE.md`: anything
        expressible as a pure function is not exempt.
      - `placeholder_texels(key, size)` is the fallback and is **not deleted, not
        repurposed** — it becomes the documented fallback for an unauthored key, and
        `placeholder_test.rs` stays valid and keeps guarding it.

- [x] **T46** The probe and swatch constants, re-derived offline — before any pixel moves
      — a scratch measurement; results recorded in `## Notes` below
      Scenarios: —
      Depends on: T44, T28
      - **The architecture's Risks entry places this in P8 deliberately.** Decode the
        built PNGs and compute the strata means and the pairwise ΔE **offline**,
        against `crates/mc-client/tests/support/probe.rs` (`STRATA`, `COVERAGE_FLOOR`,
        `SAME_COLOR = 2.0`, `DIFFERENT_COLOR = 10.0`) and
        `crates/mc-client/tests/support/swatch.rs` (`TEXEL_COLORS = 2`).
      - The premises are known to change: a real face has three to six colours,
        measured, against three hash-derived ones with far more separation. **The
        assertions are re-derived against the shipped art, never relaxed.**
      - **If two strata land inside `SAME_COLOR` of each other, that is a spec
        conversation, not a tolerance edit.** Raise it; do not decide it here.
      - Nothing is committed by this task except the numbers, in `## Notes`. P9's
        test author inherits them, and inheriting a measurement is the whole reason
        this is a task rather than a paragraph in P9.
      - `probe.rs` is at **551 non-blank lines against a 600 test-file limit**.
        Whoever re-derives its constants in P9 has 49 lines of headroom; say so in
        the closing report.

- [x] **T47** [P] The mip chain and why it is averaged in linear light
      — `docs/technical/rendering.md`
      Scenarios: —
      Depends on: T45
      - **Engine**: mip levels are averaged in linear light; the array texture is
        `Rgba8UnormSrgb` and decodes on sample; box-filtering the stored bytes gives
        128 where the correct answer is 188 and every level comes out darker than the
        one above it. This is item 4 of what a future change must not break.
      - Why the mips are generated on the CPU: wgpu has no built-in mip generation,
        and a GPU blit or compute pass would land in `src/gpu/` — the one subtree
        excluded from coverage thresholds, where golden frames are the only defence.
        A box filter over a 16×16 chain is five levels of trivial arithmetic and a
        pure function under normal coverage.
      - State plainly that none of it is wired yet, and that the sampler and the
        upload arrive in P9.

### Mutations — P8

Every reading below is workspace-wide, in a detached worktree at `d2a342f` with
its own `CARGO_TARGET_DIR` outside the repository, after `voxforge build`, both
removed afterwards. That worktree carries `content/base/blocks/water.luau`
**unedited**, so the two `mc-world::shipped_declarations_and_an_older_save`
guards that are red in the shared tree pass here. **Baseline: 1339 run, 1339
passed, 1 skipped.** Each row states the green as well as the red.

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
| M44 | `reduced` averages the stored bytes instead of linear light — spelled as `covered_average` calling `mean_of_stored` for red, green and blue, alpha untouched | FR-6.1-S2 reddens at 128 against 188. **The sharpest mutation in this spec** — the defect it models is invisible in every other instrument | **Bit, and wider than this row said. 1337 passed, 2 failed**: FR-6.1-S2 and FR-6.1-S4. S2 reads `left: [[128, 64, 128, 255]]` against `right: [[188, 64, 128, 255]]`, the two held channels coming back untouched in both. The prediction of one red was the one this table got wrong; `test-map.md`'s skeleton B — which is this mutation run before there was an implementation — recorded both, and both reproduced exactly. Every golden green |
| M45 | `chain` stops at level 4 | FR-6.1-S1 reddens | **Bit. Spelled first as the halting off-by-one `while edge > 2`: 1333 passed, 6 failed** — too broad, because it also shortens a 4-edge chain from three levels to two and so reddens FR-6.1-S5 about a count that scenario is not measuring. **Narrowed to `levels.truncate(MIP_LEVELS as usize - 1)`**, which leaves short chains alone: **1334 passed, 5 failed** — FR-6.1-S1, FR-6.1-S3, `the_declared_level_count_is_derived_from_the_texture_edge`, and both `levels_for` readings, which use `?` and meet `TooFewLevels` where they expected `Ok`. FR-6.1-S5 green, which is the point of the narrowing: the refusal itself still works. Three more than `test-map.md` predicted, and the reason is structural — `levels_for` reads `chain`'s answer, so anything shortening the chain reaches its two readings too. Every golden green |
| M46 | `reduced` selects the four texels offset by one, **wrapped back into the buffer** — the base index shifted by one and every one of the four gathers taken modulo `size · size`. **Spell it in bounds.** The obvious spelling, which shifts the base and lets the last group read past the end, fills the tail with zeros and reddens FR-6.1-S2 and FR-6.1-S3 as well: three reds, of which two are a second defect riding on the first, and the phase's claim that FR-6.1-S3 is the control then reads as false | FR-6.1-S4 reddens **alone, workspace-wide**; FR-6.1-S3 stays green — the pair measured | **Bit, exactly as narrow as predicted. 1338 passed, 1 failed** — FR-6.1-S4 and nothing else in the workspace. Reads `[[70, 200, ..], [101, 166, ..], [200, 70, ..], [192, 143, ..]]` against `[[55, 216, ..], [85, 183, ..], [183, 85, ..], [216, 55, ..]]`; the reds are the `[70, 101, 200, 192]` `test-map.md` computed offline for an off-by-one, reproduced from the running code. FR-6.1-S2 stays green because the wrap hands its 2 × 2 fixture the same four texels in a different order, and FR-6.1-S3 because a uniform image averages to itself however the four are chosen. Every golden green |
| M47 | `MIP_LEVELS` written as the literal `5` | **predicted non-bite** — same value today. Record it: the guard is the compile-time assertion against `TEXTURE_EDGE`, not a test | **Non-bite in the suite, confirmed: 1339 passed, 0 failed.** But it does not reach the gate: `cargo clippy -p mc-render --all-targets --all-features -- -D warnings` **exits 101** with `error: unused import: mc_core::content::TEXTURE_EDGE`, because the literal is the constant's only remaining reader. **There is no compile-time assertion** — the row's wording anticipated one and none was needed. The guard is the derivation plus the import it requires, and the instrument that reports it is the lint stage, not the suite. **The caveat matters: a `5` written into a file with another use for `TEXTURE_EDGE`, or none at all, leaves no dead import and passes silently.** This is `testing.md` §2's "a green suite is no evidence about a lint", from the other side |
| M48 | `levels_for` accepts a short chain — the `TooFewLevels` guard deleted | FR-6.1-S5 reddens | **Bit, exactly. 1338 passed, 1 failed** — FR-6.1-S5 alone. `supplied_texels_of_the_wrong_count` stays green, which says the two refusals are read through separate paths and not through one shared guard. Every golden green |
| M49 | Set `mip_level_count = MIP_LEVELS` in `buffers.rs` | **the goldens must redden.** If they do not, P8 has already moved a pixel and the one-re-shoot constraint is broken. Revert immediately | **Bit. 1335 passed, 4 failed**: `mc-client::terrain_goldens`, `mc-client::hud_goldens`, and two of `mc-client::terrain_probes` (`all_three_declared_block_colours_reach_the_frame`, `mirroring_the_frame_left_to_right_turns_only_the_landmark_probe_red`). Every one of the ten pure readings in `mip_test.rs` stayed green throughout, which is the whole point: **they cannot see a device.** Reverted immediately; with `mip_level_count: 1` all four are green, and that is this phase's statement that no pixel has moved |

---

## Phase 9 — Real pixels

**21 scenarios.** Done means: a player starts the client and the world is grass,
dirt and stone. This is the increment where MVP 2 stops being invisible to them.

**Picture: changes, once.** Everything in this phase moves a pixel, which is why it
is one phase.

**The re-shoot follows `docs/technical/rendering.md` verbatim** — probes, then
oracle, then HUD prediction, then a mint naming **only** the `terrain_goldens` and
`hud_goldens` binaries. **A bare `MYCRAFT_UPDATE_GOLDENS=1 cargo nextest run`
reaches `golden_mismatch` and corrupts the set permanently.** `SCENE_REVISION` is
**not** bumped: it identifies the scene contract — pose, world, camera path, tick
list, merge predicate, vertex format — and this spec changes none of it. Bumping
would redefine the revision as "something visible changed" and oblige a bump for
every future art change. The commit message carries the why.

**P9 lands two independent pixel changes and a golden diff cannot attribute
between them.** Guidance, not a binding decision, and it costs no second committed
re-shoot: land the art with `mip_level_count = 1` and the nearest sampler, take an
**uncommitted** reference capture, then land the sampler and the mips and mint
once. A bad diff then has a bisect point.

**Two numbers to start with rather than meet.** `crates/mc-client/tests/support/probe.rs`
is at **551 non-blank lines against a 600-line test limit — 49 of headroom** — and
T54 re-derives its constants inside it. `crates/mc-render/tests/terrain_offscreen.rs`
is at 460 of 600. Re-measure both with the gate's own counter before writing, not
`wc -l`, and take T46's measurements as the input so the growth is the constants
themselves rather than exploratory code. A cap hit while re-deriving a tolerance
rejects whatever is cheapest to drop, and here the cheapest thing to drop is the
comment explaining where a number came from.

- [x] **T48** The client decodes, and the renderer is handed texels
      — `crates/mc-client/src/textures/decode.rs` (new),
      `crates/mc-client/src/startup.rs`, `crates/mc-client/src/content.rs`,
      `crates/mc-render/src/gpu/buffers.rs`, `crates/mc-render/src/gpu/hud.rs`
      Scenarios: FR-4.1-S1, FR-4.1-S2, FR-4.1-S3, FR-4.1-S4
      Depends on: T45, T40
      - **D4, forced by the tree**: `mc-render` has no `std::fs`, no `PathBuf` and no
        image decoder anywhere in `src/`, and this spec does not give it one. It
        gains the ability to be *handed* level-0 texels per key. The read and the PNG
        decode land in `mc-client`, already the composition root and already the only
        crate that builds `TextureLayers`.
      - `decode.rs` is **the only file in `mc-client` that may name `image::`**, and
        `image` is added to its manifest with the boundary comment its other confined
        dependencies carry.
      - Decoding in `mc-sim` was rejected: pixels are the client's half of the split,
        and a server that needed them would break the asymmetry that makes a texture
        pack a legal client modification and a block declaration not.
      - `SuppliedTexels` is held by `FrameRenderer` for the whole run, given at
        construction. **It is not carried by `Unuploaded`/`retire`**: the built set is
        a pre-build artefact that does not change while the client runs, so a reload
        appending a key finds either art that was already read or no art at all —
        which is FR-4.2's fallback reached by a second road, needing no new
        machinery.
      - FR-4.1-S1's fixture must hold texels **differing from the generated texture**
        for the same key, or the scenario passes under an implementation that ignores
        the image entirely. FR-4.1-S3 is water: a layer assigned, no face drawn.
        FR-4.1-S4 is the extra-image case: the launch completes and the image is
        unsampled.

- [x] **T49** A key the set does not cover falls back, and the run continues
      — `crates/mc-render/src/texture/mip.rs`, `crates/mc-client/src/startup.rs`
      Scenarios: FR-4.2-S1, FR-4.2-S2, FR-4.2-S3
      Depends on: T48
      - **This is a mod author's first block and it must never be a crash, a refusal
        or a message.** The key resolves to its generated placeholder, exactly as
        every key does today, and the run continues.
      - FR-4.2-S2 is the mixed case in one run and is the one that catches an
        all-or-nothing implementation. FR-4.2-S3 is the vacuity control: a set that
        is present and current and covers nothing still launches.

- [x] **T50** An image the array texture cannot hold is refused rather than uploaded
      — `crates/mc-client/src/textures/decode.rs`
      Scenarios: FR-4.3-S1, FR-4.3-S2
      Depends on: T48
      - Defence in depth against a hand-tampered set. The set is derived and
        gitignored, but it is a directory a person can write into, and D5's build-time
        refusal (T31) is the one that names the model instead of the image.
      - Both refusals name the key. A message about a file with no key in it leaves a
        mod author with a filename they never typed.

- [x] **T51** The sampler is a pure request, and the device is what refuses it
      — `crates/mc-render/src/texture/sampler.rs` (new),
      `crates/mc-render/src/gpu/mod.rs`, `crates/mc-render/src/gpu/buffers.rs`,
      `crates/mc-render/Cargo.toml`
      Scenarios: FR-6.2-S3, FR-6.2-S4, FR-6.2-S5
      Depends on: T45
      - **The choice is forced by wgpu, not preferred.**
        `wgpu-core-30.0.0/src/device/resource.rs:2288-2316` refuses a sampler with
        `anisotropy_clamp != 1` unless `min_filter`, `mag_filter` **and**
        `mipmap_filter` are all `Linear`, in three separate arms. Anisotropy and
        crisp voxel magnification cannot both be had; this spec takes filtered. The
        bind-group layout already declares `filterable: true` /
        `SamplerBindingType::Filtering` (`gpu/pipeline.rs:94-103`) — **no layout
        change is needed and none should be made.**
      - **FR-6.2-S4 is FR-6.2-S3's positive control and a separate test function**:
        the same inspection applied to a request that *does* ask for anisotropy must
        report it. Without it, `asks_for_anisotropy` returning `false` unconditionally
        leaves S3 green forever.
      - **D14's seam is the decision, not an implementation detail.** `buffers` is a
        private module, `sampler()` at `buffers.rs:263` is a private free function
        taking no request, and nothing under `crates/mc-render/tests/` can reach
        either today. `terrain_sampler(device, &SamplerRequest)` becomes `pub` and is
        re-exported from `gpu` under `#[cfg(feature = "gpu")]`, threaded
        `FrameRenderer::new → TerrainRenderer::new → SceneBuffers::new` as one
        borrowed `TerrainTextures<'_>` carrying both the request and the supplied
        texels — one parameter, because `clippy::too_many_arguments` has a threshold
        of 4. **Without this parameter, D14 is a decision whose two witnesses cannot
        be written.**
      - **FR-6.2-S5 uses a real device refusal, not a pre-check.** A pure pre-check
        re-implementing wgpu's rule is a second copy of a vendor constraint that
        drifts silently when the vendor changes, and the scenario's subject is the
        device. Wrap `create_sampler` in `device.push_error_scope(Validation)` /
        `pop_error_scope` and map a refusal to
        `RendererError::TerrainSampler { requested }`; `SamplerRequest: Display` is
        what lets it name the combination.
      - `pollster` moves from `[dev-dependencies]` to an optional `[dependencies]`
        entry under the `gpu` feature, **and its manifest comment at
        `Cargo.toml:73-75` is amended in the same commit** — today it says device
        acquisition belongs to the client and the harness, never to this crate's
        library. Blocking on an already-ready error scope is not device acquisition,
        but the rule as written forbids it, so it is amended deliberately rather than
        contradicted silently. `pollster` is featureless and **must not drag `wgpu`
        into the `--no-default-features` graph** — the gate's gpu-free stage is what
        would catch it.
      - **Architecture Assumption 5 is the one interface not read out of the tree.**
        If `pop_error_scope` cannot be blocked on inside `SceneBuffers::new`, the
        fallback is the pure pre-check and FR-6.2-S5 is authored against that
        instead. Report it if you hit it; do not silently change the design.

- [x] **T52** What the sampler does to the picture
      — `crates/mc-render/tests/terrain_offscreen.rs`, `crates/mc-testkit/`
      Scenarios: FR-6.2-S1, FR-6.2-S2
      Depends on: T51, T48
      - **A test that reads back the descriptor it caused to be built is agreement
        between two copies of one decision.** S3 and S4 assert the request; these two
        assert the consequence in a captured frame, and that is the whole reason both
        pairs exist.
      - FR-6.2-S2 needs a renderer built with a **second** sampler configuration —
        unfiltered minification — which is what the `TerrainTextures` parameter buys.
        Production passes `TERRAIN_SAMPLER` from the composition root; the capture
        harness passes the other value.
      - `terrain_offscreen.rs`'s centre-pixel comparison against
        `placeholder_mean_color` under a `SAME_TEXTURE` tolerance is one of the three
        premises this spec invalidates. **Re-derive it against the shipped art; do not
        loosen it until green.** T46's numbers are the input.

- [x] **T53** The shipped blocks declare their real art, and draw it
      — `content/base/blocks/grass.luau`, `content/base/textures.toml`,
      `crates/mc-client/tests/`
      Scenarios: FR-8.1-S1, FR-8.1-S2, FR-8.1-S3
      Depends on: T48, T28
      - `grass.luau` declares six facings; dirt and stone keep a single key. This is
        the first shipped declaration to use the table form, and it is the worked
        example `docs/modding/blocks-items.md` shows.
      - FR-8.1-S2 is the shared-image assertion: the grass block's negative-Y face
        and every dirt face draw the **same** image. FR-8.1-S3 asserts the four side
        images are **pairwise unequal** — stone's six faces are already six distinct
        images even though the block is uniform noise, so "different model faces"
        does not imply "different images" by construction and this must be measured.
      - `crates/mc-client/tests/support/swatch.rs`'s `TEXEL_COLORS = 2` is the
        second invalidated premise. Re-derive from T46's numbers.

- [x] **T54** The goldens are re-shot at `r1`, and one witness does not share their path
      — `crates/mc-client/tests/terrain_goldens.rs`,
      `crates/mc-client/tests/hud_goldens.rs`,
      `crates/mc-client/tests/support/probe.rs`
      Scenarios: FR-8.1-S4, FR-8.1-S5
      Depends on: T52, T53
      - **Read Trap 9.** S4 is a golden minted from the renderer it verifies. S5 is
        the derived witness: the grass top face judged against a mean computed from
        the built PNG, **decoded by the client's decoder and never by the draw**. If a
        refactor lets S5 read a value the frame produced, the pair collapses into one
        snapshot. No mutation detects this; it is reviewer-held.
      - Procedure, verbatim from `docs/technical/rendering.md`: probes, then oracle,
        then HUD prediction, then a mint naming **only** the two golden binaries.
        `SCENE_REVISION` is not bumped; the commit message carries the why.
      - `probe.rs`'s `STRATA`, `COVERAGE_FLOOR` and the ΔE constants are the third
        invalidated premise and the one most likely to need a second round. **Expect
        one round of re-derivation to be wrong and to be caught by S5 rather than by
        S4** — that is what the pair is for.

- [x] **T55** A change to the art fails distinguishably from a change to the renderer
      — `crates/mc-client/tests/`
      Scenarios: FR-7.2-S1, FR-7.2-S2
      Depends on: T54
      - **These belong to P9 and not to P7**, and the reason is worth keeping: the
        discriminating half — *report the set as stale **rather than** as a
        golden-frame mismatch* — cannot be exercised where no pixel depends on the
        set, because there is no golden mismatch to be distinguished from. In P7 it
        would pass vacuously.
      - This is true of the golden suites because `prepare_scene` calls `built_set`
        (T42), and every golden is shot through `prepare_scene`.

- [x] **T56** [P] What the player sees, what a mod author writes, and the records this spec owes
      — `docs/user/gameplay.md`, `docs/technical/rendering.md`,
      `docs/technical/testing.md`, `docs/technical/decisions.md`,
      `docs/modding/blocks-items.md`
      Scenarios: —
      Depends on: T54, T55
      - **Player** (`gameplay.md`): the world is grass, dirt and stone. What they can
        now see and how to get to it. **And**: a save from before this build reports
        its blocks as changed on next load, why that is correct — every block's
        appearance really did change — and what the `--load-changed-blocks` path does
        about it.
      - **Mod author** (`blocks-items.md`): the worked grass declaration, now that a
        shipped block uses the table form.
      - **Engine** (`rendering.md`): the sampler and why anisotropy is refused; the
        re-shoot at `r1` and why `SCENE_REVISION` was not bumped.
      - **Engine** (`testing.md`): the re-derived probe constants and what they were
        derived from.
      - **ADR-026** moves from "Implementation pending" to implemented, naming this
        spec as its first consumer and pointing at where each of its five items
        landed: FR-5.1 (item 1), FR-7.1-S3/S4 (item 2), FR-3.4 (item 3), FR-7.1-S1
        (item 4), FR-3.2 (item 5).
      - **A new ADR: "Terrain magnifies with nearest and minifies with linear;
        anisotropy is refused."** It is a one-way door with a vendor constraint behind
        it, which is the shape `decisions.md` exists for, and it needs a home that
        outlives this spec folder so nobody re-opens it as a bug.

### Mutations — P9

Every reading below is workspace-wide, in a detached worktree at `10b1564` with
its own `CARGO_TARGET_DIR` on `E:`, after `voxforge build`, both removed
afterwards. That worktree carries `content/base/blocks/water.luau` **unedited**,
so the two `mc-world::shipped_declarations_and_an_older_save` guards that are red
in the shared tree pass here. **Baseline: 1366 run, 1366 passed, 0 failed, 1
skipped** — a wholly green tree, which is itself the measurement that says those
two shared-tree reds are entirely the owner's uncommitted edit and nothing of
this phase.

`git diff` was read on the mutated tree before every run — the first attempt at
M50 wrote to a path Python on Windows could not resolve, the diff came back
empty, and the run reported 1366 passed. **That is the "a stage reported ok and
the edit never ran" trap, caught by the diff and by nothing else.** Each revert
was by hand and `git diff --exit-code` was clean between every pair.

Each row states the green as well as the red.

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
| M50 | `SuppliedTexels::covering` returns `None` always | FR-4.1-S1/S2, FR-8.1-S1/S2/S3 redden; every FR-4.2 stays green — the fallback pair measured | **Bit. 1352 passed, 14 failed — and three departures from this row, each of which is a finding.** (a) **FR-8.1-S2 stayed green.** It reads `key_of` and the shipped manifest and never touches the supply, so it is a declaration-and-manifest reading rather than an art one; the prediction over-counted. (b) **FR-4.2-S2 reddened**, against "every FR-4.2 stays green" — the mixed reading asserts the *covered* half as well, which is exactly what makes it more than the two single-key readings put together, so this row was wrong in the direction that says the test earns its keep. (c) **FR-6.2-S2 reddened at its fixture premise rather than at its assertion**: with nothing covered, both sampler configurations draw a period-2 placeholder checkerboard that averages away by level 1, so `unfiltered > 0` fires and says so by name instead of reporting that filtering helped. The remaining reds are FR-4.1-S1/S2/S4, FR-8.1-S1/S3/S5, the reload-supply guard, two `mip` unit readings, one `supplied` unit reading, and both golden suites. FR-4.1-S3, FR-4.2-S1 and FR-4.2-S3 green. **The mutation is also broader than the production path**: `support/art.rs`'s oracle calls `covering` too, so the frame and the expectation fall back together and `terrain_probes` stays green for that reason. M50b was added to isolate the half this one cannot |
| M50b | `write_layer` fills every layer from `SuppliedTexels::none()` — the **device path only**, leaving the pure value intact. **Added by the implementer**, and the reason is a method rather than an extra row. `covering` is called by `support/art.rs`'s oracle *as well as* by production, so under M50 the frame and the expectation fall back **together**: they agree because they share a computation, not because the draw does anything. That is the shape `testing.md` §2 warns about and the shape that hid `launch::start` for a whole phase. Mutating the device path alone is the only way to ask whether the *draw* consults the supply | every reading that takes its texels from a value stays green; only the frame-based readings redden. This is the "policy is not wiring" question asked directly | **Bit, exactly as narrow as that. 1358 passed, 8 failed**, and every one of the eight is a frame: both golden suites, the reload-supply guard, `hud_held_block`'s indicator, two of `terrain_probes`, FR-8.1-S5 and FR-6.2-S2. Every pure reading — `the_built_set_fills_its_layers`, all three of `an_unauthored_key_draws_a_generated_texture`, all three of `the_shipped_blocks_draw_their_baked_art`, every `mip` and `supplied` unit test — stayed **green**, which is the point: they cannot see a device. **So the wiring has six witnesses besides the goldens**, and the answer to "what would go red if the draw stopped consulting the supply" is not "only a golden". That question is worth asking of any pure core in this project, and after `launch::start` it was the one worth spending a run on |
| M51 | `TERRAIN_SAMPLER.magnify = Linear` | FR-6.2-S3 and FR-6.2-S1 both redden — request and consequence | **Bit. 1358 passed, 8 failed.** Both predicted reds landed, and the row **under-predicted by six**: `hud_held_block`'s indicator, `reload_draws_the_new_block`, `reload_hand_shows_the_new_block`, the reload-supply guard and both golden suites. Every one of the six reads a **magnified** face, which is the finding — nearest magnification is not a single-witness decision, it is witnessed by seven readings across three crates, and FR-6.2-S1 is the one that names *why* rather than merely noticing |
| M52 | `asks_for_anisotropy` returns `false` always | FR-6.2-S4 reddens; FR-6.2-S3 stays green — the positive control earning its keep | **Bit, exactly, and this is the row with the most to say for the least. 1365 passed, 1 failed** — FR-6.2-S4 and *nothing else in 1366 tests*. FR-6.2-S3 stayed green because its expected tuple states `false` for a clamp of one, which is still what a dead function answers. So the positive control is the only instrument in the workspace that can see this, measured rather than argued, and `test-map.md`'s note that M52 was unreadable under the skeleton is now closed |
| M53 | `mip_level_count` back to 1 | FR-6.2-S2 reddens | **Bit. 1363 passed, 3 failed** — FR-6.2-S2 and both golden suites. **Spelled as two lines, deliberately**: the descriptor *and* `write_layer`'s `.take`, because the descriptor alone leaves the loop writing levels 1 to 4 into a one-level texture, which is a device validation error outside any error scope and would have reddened noisily for a reason that is not this row's subject. Two lines is what "back to 1" coherently means. The ten pure readings in `mip_test.rs` stayed green throughout — they cannot see a device, which is the same statement M49 made from the other side of the same seam |
| M54 | The launch draws layer zero for an uncovered key instead of the placeholder — spelled as `levels_for`'s fallback returning opaque black | FR-4.2-S1 reddens | **Bit, and wider. 1357 passed, 9 failed**: all three FR-4.2 readings (each compares against `placeholder_texels`, so each sees it), the `mip` unit reading of the fallback, `reload_draws_the_new_block`, `reload_hand_shows_the_new_block`, `terrain_offscreen`'s centre-pixel depth reading, FR-6.2-S1 — and **`mc-world::no_hardcoded_hud_names`**. That last one is the surprise and it is worth keeping: the literal chosen for the mutation, `[0, 0, 0, 255]`, is a colour the base game's HUD declares, so a source scan for hardcoded HUD colours reddened over a mip-chain edit. Noise for this row, and evidence that the scan works. `terrain_offscreen` reddening is the invalidated premise `test-map.md` recorded as *re-measured rather than repaired* proving it does witness the generator after all |
| M55 | `grass.luau` reverts to a single key | FR-8.1-S1/S2/S3 redden, and the goldens with them | **Bit, very wide, and the row is wrong about one of the three. 1343 passed, 23 failed.** FR-8.1-S2, FR-8.1-S3 and both golden suites reddened as predicted. **FR-8.1-S1 stayed green**, and the reason is a real limitation worth writing down: with `texture = "base:grass_top"` all six faces draw that key, so `key_of(grass, Face::Up)` still answers `base:grass_top` and every element of S1's tuple still holds. **S1 constrains the upward face and nothing else** — it cannot tell a six-facing declaration from a single-key one whose single key is the top key, and S2 and S3 are what carry the six-facing claim. The pair is complete; S1 alone is not. The other nineteen reds are the eight-key premises re-derived in `7d39ae9`, going red in the mirror image of the way they went red when T53 landed — which says those fixtures genuinely bind the eight-key set rather than having been loosened to accept it |
| M56 | FR-8.1-S5's probe reads a value the frame produced | **no mutation detects this.** Reviewer-held; recorded here because that is its only defence | **Not attempted, and the reason is structural rather than a judgement about effort:** the mutation lives inside a test file, and an implementer may not edit one. There is nothing to measure — the claim of this row is that no *production* edit reproduces it, and no production edit can, because the defect is a change to how the expectation is derived. It stays reviewer-held. What can be said from the runs above is the shape of the thing being protected: under M50b, FR-8.1-S5 reddens while every pure reading stays green, so it is genuinely judging a frame; under M50 it reddens at its premise, so it genuinely reads the built PNG. Those two together are as close as measurement gets to "the two halves are on different paths" |
| M57 | `refusal_for` swallows `StaleAgainstSources` | FR-7.2-S1 reddens as a golden mismatch instead — which is the exact confusion FR-7.2 exists to prevent | **Bit, and the shape is confirmed rather than assumed. 1358 passed, 8 failed.** FR-7.2-S1 fails reporting `Some(Failed(GoldenFailure { reason: Mismatch(...) }))` at 718 006 of 921 600 pixels and max ΔE 87.94 — a golden mismatch over an edited model, reproduced exactly. **FR-7.2-S2 stayed green**, which is the stated green: the mutation touches staleness only and absence still refuses. The other reds are two `built_set_verdict` readings and all five of `documented_refusals`, the last because the stale refusal is quoted on a modding page and the page and the program are compared line for line |

**One deferred observation, from M57 and not a blocker.**
`art_and_renderer_failures_are_told_apart.rs`'s `refusal_of` debug-prints the
whole `GoldenOutcome` when a run reaches a golden verdict, and a `Comparison`
carries `failing_mask: Some(FailingMask { failing: [bool; 921_600] })`. The
failure message came to **5.5 MB** and buried the sentence a reader needs under a
boolean array. `terrain_goldens.rs` records having made that mistake once and
never debug-prints an outcome for exactly this reason. It is a test file and
therefore the test author's; it costs nothing while the test is green, which is
why it is recorded rather than raised.

---

## Scenarios that will be green on arrival inside their own phase

Each phase's test author measures its arrival colours and records them; this list
is what the tasks stage predicts, so a disagreement is information rather than a
surprise.

| Scenario | Phase | Why it is green before the phase's implementation | What it therefore grades |
|---|---|---|---|
| FR-1.1-S1 | P2 | the uniform-key skeleton is what `texture` already means | that widening the field did not break the form that already worked |
| FR-1.3-S1 | P3 | every shipped block declares `texture == name`, so `up` already resolves | that the *other* two axes are what the mapping is for |
| FR-3.1-S6 | P5 | a build that writes nothing writes zero images | it needs the index asserted present, or it is vacuous |
| FR-4.2-S3 | P9 | before texels are wired, every face already draws a generated texture | it is the control that keeps the fallback from being deleted once the set works |
| FR-5.1-S7 | P7 | an empty index is trivially current | it constrains `Current`'s arm to be reachable without art |

**FR-4.2-S3 and FR-5.1-S7 are green for the right reason and must not be "fixed".**
They are vacuity controls, and a phase that reddens them has broken something.

---

## Weak instruments, named

- **Every FR-7.1 scenario is graded by a PowerShell-driving Rust test, and nothing
  in this repository has ever tested `scripts/sdd-gate.ps1`.** `-ArtOnly` could
  pass its own tests and diverge from a real gate run. The structural test on
  stage ordering is what ties the selection to the real sequence. If it cannot be
  written honestly, **say so and record which scenarios are then unwitnessed**
  rather than letting the gap be silent.
- **FR-6.2-S3 and S4 inspect a constant.** They are agreement between two copies of
  one decision unless FR-6.2-S1 and S2 hold the consequence in a captured frame.
  All four are owed.
- **FR-1.2-S8's instrument is a text comparison** between the guide and a real run.
  It cannot tell a refusal that is documented from one that is correct.

---

## Mechanisms no scenario covers, and where each gets its test

| Mechanism | Instrument | Task |
|---|---|---|
| `Facing::face` is a bijection | exhaustive round trip over both `ALL` arrays | T04, additional coverage |
| Both save hashes agree with an independent FNV before the appearance changes | hand-computed oracle over postcard bytes | T02, additional coverage |
| Every `IndexError` arm is reachable and named | one test function per arm | T25, additional coverage |
| A rendered index round-trips through `parse` | render → parse → compare | T25, additional coverage |
| `drawn_of` still marks the whole world on a texture edit | the existing section-count assertions | T10, inherited |
| `uploaded_to` is still the only route to an owned `Unuploaded` | the compiler, and a reviewer | T16, reviewer-held |
| The manifest is invisible to all three content scanners | re-checked by hand, recorded | T27 |
| FR-8.1-S4 and S5 do not share a path | reviewer-held; no mutation detects it | T54 |

---

## Notes

[Deferred observations and follow-ups discovered during implementation. Never
delete task text; append status markers only.]

### Deferred observations standing at the end of P5

Both were inherited by leg two, neither is built, and Out of Scope is binding.

- **A manifest naming a model outside its own directory** — `model =
  "../shared/x.mcvox"` — builds cleanly and writes an index whose `source` line
  `parse` later refuses as an unsafe path, so the set is unreadable to the
  client that has to read it. Leg two did not close it: no scenario reaches it,
  and P7 is where the user gets a refusal by name. Still standing.
- **`stating` cannot refuse an image name carrying a space**, which would render
  a `key` record parsing back on the wrong split. voxforge's own file-name rule
  (FR-3.3-S12) now stops it arising from a manifest — a key with a space derives
  a name with a space, which `is_an_ordinary_image_name` refuses — so it remains
  absent defence-in-depth on the `mc-core` side rather than a live defect. Still
  standing, and narrower than it was.

### The unused-key report should empty at P9 — measured in P5 leg two

**Today `voxforge build content/base/textures.toml` names five keys as unused**,
and every one of them is a key T53 is going to declare:

```
`base:grass_top` is baked here and named by no block declaration
`base:grass_side_north` is baked here and named by no block declaration
`base:grass_side_south` is baked here and named by no block declaration
`base:grass_side_east` is baked here and named by no block declaration
`base:grass_side_west` is baked here and named by no block declaration
```

`base:dirt` and `base:stone` are already declared, which is why the count is
five and not seven.

**When T53 gives `grass.luau` its six facings, the report should empty. If it
does not, something is wrong** — a key the manifest bakes that nothing declares,
or a facing declared against a key the manifest does not bake, and the surviving
line names which. It costs one command to check, and it is the earliest place
the manifest and the declaration can be seen to agree about the six spellings:
T49's fallback and P7's set verdict both catch a disagreement eventually, but
they catch it at *launch*, as a key the set does not cover — the build says it
while the person editing the two files is still looking at them.

The five are written out above rather than left as a count, so the comparison is
exact. **A tolerated warning is how a real one gets missed**, and five lines that
have always been there is exactly the shape of a warning nobody reads.

### P9's premises, measured in P5 leg one — recorded here for P9's pair

Free measurements, taken because the art build had to open both shipped models
anyway. **A premise measured cheaply now is one nobody has to discover
expensively later**, and all three of these are things P9 would otherwise meet
as a surprise.

- **Both shipped models are cubes of their declared scale.** `grass-block.mcvox`
  and `stone-block.mcvox` each assemble to sixteen voxels on every axis at
  `scale = 16`, so `emit`'s cubic precondition passes for both.
- **All twelve faces tile across every edge** under `SeamPolicy::Reported`, so
  neither T31's seam refusal nor the gate's art build refuses the shipped set.
- **The grass block's six baked images are pairwise unequal** — six distinct
  sha256 values at one pixel per voxel. **This is the premise FR-8.1-S3 rests
  on**: four side images being different is a measured property of this model's
  art, not a consequence of taking four different model faces, and stone's
  uniform noise is the case that proves the distinction.

Measured at `87022ec` with `voxforge texture --all-faces --pixels-per-voxel 1`,
which is the same emission path `voxforge build` drives.

### P5 leg two's outcome — what P6 has to know

Closed: **T31, T32, T33, T34, T35**, and with them Phase 5. The seven scenarios
that arrived red — FR-3.3-S7, S10, S11, S12, S13, S14 and the missing-blocks
reading — are green, and the denominator did not move: **1299 run, 1299 passed,
1 skipped**, against the test author's own 1299.

- **The arrival was measured, not assumed.** All seven failed at an *assertion*
  and none at a compile error. Two were already refusing for the wrong reason
  and the failure text says which: FR-3.3-S12 refused at the file *write*
  (`base__a/b.png: the image could not be written`), and FR-3.3-S13 at
  `TextureSetIndex::stating`, quoting the derived image name rather than the
  key. Both are now refused in `load_manifest`.
- **The refusal order as built, unchanged from leg one's and now complete.**
  Manifest refusals (including the two key refusals T33 added) → source reads,
  where a missing model is refused with the key that named it → load, the
  scale-times-pixels refusal, emit, the selected-face seam refusal → the index
  rendered → nothing written until it exists → the unused-key report last, after
  every path the build promised.
- **The image file name is derived in exactly one place and it moved.**
  `image_named` left `cli/build.rs` for `texture/manifest.rs`, and
  `ManifestEntry` now carries its own `image`. That is what puts D9's validation
  where the author can still see their manifest, and it leaves the build with no
  second derivation to drift from.
- **The shape rule itself is in `mc_core::art::is_an_ordinary_image_name`**, not
  in voxforge, because D7 has the client refusing a name failing the same rule
  rather than re-deriving one. P7 calls this; it has no `crates/`-side caller
  yet, which is the same commitment `mc_core::hash` carried before this leg.
- **`TextureSetIndex::parse` was deliberately not tightened.** It already
  refuses an image name that is a path (`UnsafePath`); making it refuse
  everything the new shape rule refuses would change behaviour the test author
  owns tests for, and D9 puts that refusal on the *client*, not on the format.
- **The unused-key report is a real finding on the shipped tree, not just a
  fixture's.** `voxforge build content/base/textures.toml` prints five unused
  keys today — `base:grass_top` and the four `base:grass_side_*` — because the
  per-face block declaration that names them is **T53's, in P9**. M32 measured
  what a refusal there would cost: four shipped-root readings, and the build
  itself. **P6's gate stage must not treat that output as a failure**, and P9
  should drive the count to zero — both banked where their phase will meet
  them, under T37 and in the Notes block below.
- **M26's third measurement agrees with leg one's second**: 4 new, the same
  four. FR-3.4-S3 and the `*.toml`-only reading as the row is about, plus
  FR-3.2-S1 and S2 as collateral, with FR-3.4-S1, S2 and FR-3.3-S3 green. The
  test author's 5 stands as a measurement against a different implementation,
  not as a number this one missed.
- **T34 needed no implementation and none was written.** Its four scenarios were
  green on arrival for the reason leg one recorded, and the only new evidence
  about them is M26 above.
- **The gate is green at this boundary**, on the shared tree, all stages.

### T46's measurements — recorded here for P9's test author

**Taken at `d2a342f`, against the built set the shipped manifest bakes.** Nothing
was committed by this task except these numbers.

> **Read this before using any figure below. Every number here was computed
> offline and no test in this repository asserts one of them.** They are a dated
> observation, not a maintained record and not a contract — the code cannot go
> red if one is wrong. That is not a formality: this phase corrected a number in
> `test-map.md` that had been produced by exactly this kind of program, where a
> line labelled "closest approach" computed the farthest, and the output went
> into prose that afterwards read as measured. **An offline oracle has no test.**
> What stands behind the figures below instead is §"The second, independent
> check" — read which figures it covers, because it does not cover all of them.

**How to reproduce them.** Two steps, and the first is not optional — the built
set is not in version control:

```
cargo run -p voxforge -- build content/base/textures.toml
```

then a standalone program (built outside the workspace, with `image = { version =
"0.25.10", default-features = false, features = ["png"] }` as its only dependency)
that decodes the seven PNGs under `content/base/textures/` and, for each, counts
distinct RGB texels, computes the mean two ways, and measures ΔE. **The colour
maths is `crates/mc-testkit/src/frame/color.rs` copied verbatim** — `srgb8_to_lab`
via linear RGB and CIE XYZ (D65), then CIE76 — because what is wanted here is what
`probe.rs` will itself compute, not a second opinion about the metric. The
generated means it is read against are `placeholder_mean_color` copied the same
way (FNV-1a 64 over the key's text, each of the low three bytes scaled into
`40 + (byte · 176) >> 8`).

**The check program is reproduced differently, and deliberately so**: it is a
second binary outside the workspace with **path dependencies on `mc-testkit`,
`mc-render` (`default-features = false`) and `mc-core`**, so it calls the shipped
`compare`, `chain` and `placeholder_mean_color` rather than copies of them. It
also parses `content/base/materials/*.toml` for `name` and `color` and matches
every decoded texel against that palette. Cargo resolves the workspace-inherited
lints and dependency versions through each member's own root, so a path
dependency from outside the workspace builds without touching `Cargo.toml`.
**Neither program is committed** — what is durable is this recipe and the numbers.

**Two means are reported per texture and they are not the same colour.** The
*linear-light* mean is what the smallest mip level holds and what a minified face
converges to; the *stored-byte* mean is what averaging without the transfer gives.
They stand up to ΔE 2.38 apart on the grass sides — above `SAME_COLOR` — so P9 has
to say which one it declares.

#### Per texture, all seven 16 × 16 and all fully opaque

| Key | Distinct texel colours | Mean in linear light | Mean of stored bytes | ΔE apart | Furthest texel from the linear mean | Texels within ΔE 10 of it |
|---|---|---|---|---|---|---|
| `base:dirt` | **3** | `[138, 106, 70]` | `[138, 105, 70]` | 0.73 | 9.56 | **100.00%** |
| `base:grass_top` | **5** | `[104, 165, 78]` | `[103, 164, 78]` | 0.58 | 9.09 | **100.00%** |
| `base:grass_side_north` | **6** | `[130, 117, 71]` | `[129, 113, 70]` | 2.38 | 39.39 | **0.00%** |
| `base:grass_side_south` | **6** | `[130, 117, 71]` | `[129, 113, 70]` | 2.38 | 39.39 | **0.00%** |
| `base:grass_side_east` | **6** | `[129, 117, 71]` | `[128, 113, 70]` | 2.38 | 39.08 | **0.00%** |
| `base:grass_side_west` | **6** | `[130, 116, 71]` | `[128, 112, 70]` | 2.21 | 40.10 | **43.36%** |
| `base:stone` | **3** | `[126, 126, 126]` | `[125, 125, 125]` | 0.39 | 12.07 | **66.41%** |

The texel colours themselves, most common first:

- **`base:dirt`** — `[139, 106, 69]` 73.8%, `[119, 88, 64]` 15.2%, `[157, 124, 85]` 10.9%
- **`base:grass_top`** — `[106, 168, 79]` 45.3%, `[85, 143, 66]` 18.4%, `[95, 156, 72]` 16.8%, `[117, 178, 87]` 10.2%, `[128, 187, 96]` 9.4%
- **`base:stone`** — `[125, 125, 125]` 66.4%, `[96, 96, 96]` 16.8%, `[154, 154, 154]` 16.8%
- **the four grass sides** — the three dirt colours over roughly 80% of the face and the three lightest grass colours over the rest; north 46.9/26.6/14.5/8.2/2.7/1.2%, and the other three within a texel or two of that

#### The second, independent check — what it covers and what it does not

**Not the same program run twice.** A separate binary re-reaches the same
quantities by routes sharing no arithmetic with the first, plus a positive
control and four properties. Every check passed; what each would have caught is
stated, because a check nobody can characterise is worth as little as the oracle
it is guarding.

| Figure | The second route | What it would have caught |
|---|---|---|
| **The distinct texel colours, and the counts 3 / 5 / 6** | `content/base/materials/` — TOML a person wrote, decoded by nothing | **The strongest of the checks.** Every distinct colour in all seven textures is byte-identical to a declared material, and each face uses exactly the materials it should: dirt from `dirt`/`dirt_dark`/`dirt_light`, `grass_top` from the five grass tones, each side from three grass and three dirt. A channel swap would read `[69, 106, 139]`, a premultiplied-alpha slip or a colour-space mix-up in the decoder would land on colours no material declares, and an off-by-one in the byte scaling would land outside the 48 declared colours on every texture. The palette is matched byte for byte, so a face bakes its material colour unshaded |
| **The linear-light means** | `mc_render::texture::mip::chain`'s smallest level — four rounds of 2 x 2 `f32` averaging against one flat `f64` average | A mean computed over the wrong texels, a transfer applied in the wrong direction, or a channel dropped. See the one-byte finding below, which is what it actually turned up |
| **Every ΔE, including the 9.59** | `mc_testkit::frame::compare` — **the code `probe.rs::distance` itself runs** | A defect in the CIELAB conversion the first program copied. It reproduces all six load-bearing pairs to the reported precision, so P9's probe will compute these same numbers |
| **The generated means `[47, 200, 183]`, `[174, 139, 81]`, `[47, 191, 136]`** | `mc_render::texture::placeholder::placeholder_mean_color` — the shipped generator | A transcription error in the FNV-1a and band arithmetic the first program copied. All three identical |
| **The share-within-tolerance percentages** | recomputed through the `mc-testkit` distance above | A wrong tolerance comparison. All seven reproduce exactly, including the 0.00% figures |
| **The per-colour share percentages (73.8%, 45.3%, …)** | **none** | **Nothing. These have no second route** — they are a straight tally of decoded texels, and only the decode itself stands behind them. The counts they are taken over *are* checked, so a wrong tally would have to be a wrong division rather than a wrong reading |

**The positive control, and its bound.** ΔE(black, white) must be **exactly 100**,
because CIELAB puts `L*` on 0..100 and both sit on the neutral axis; a conversion
that had lost its scale would still be symmetric and still answer zero on
identical inputs, so no property below can see it. Measured: **100.000004**. The
deviation is `color.rs`'s D65 white point stated to six places, not a defect —
and the bound was derived from that rather than loosened until green. **The first
bound tried was 1e-9 and it failed against correct code**, which is
`testing.md` §2's over-tight assertion reproduced in the instrument rather than
in the suite; the recorded bound is 1e-4, four orders above the constant's own
rounding and four below the whole ΔE a scale defect would cost.

**The four properties, all holding.** Every mean channel lies inside the range of
the texels it averages; ΔE is symmetric across all 21 pairs; the **triangle
inequality holds over all 343 triples**, which a sign error on one Lab axis would
break while leaving symmetry and self-distance intact; and the share within a
tolerance is monotone in the tolerance.

**What the mip chain turned up, and P9 needs it.** The colour the *smallest mip
level* holds is not always the flat mean: `base:dirt` reads `[139, 106, 71]`
against `[138, 106, 70]`, `base:grass_top` `[105, 165, 79]` against
`[104, 165, 78]`, and `base:stone` `[127, 127, 127]` against `[126, 126, 126]` —
**one byte on one or two channels, ΔE 0.39 to 0.68**. The four grass sides agree
exactly. The cause is rounding at each of four hierarchical levels against a
single average, and it is not a defect in either. **It matters because a probe
declares a mean and a minified face shows the mip level**, so a P9 assertion
tight enough to care about a byte must say which of the two it means.

#### Three premises P9 inherits as **false**

1. **`swatch.rs`'s `TEXEL_COLORS = 2` is dead, and `texel_colors` will refuse.**
   It errors outright when a layer does not hold exactly two colours, and no
   shipped texture holds two: the count is 3, 5 or 6. This is not a threshold to
   widen — the two-colour form was a property of the generator (a checkerboard of
   mean ± one step), and shipped art has no such property. Whatever replaces it
   has to read the colours a texture actually holds.
2. **`probe.rs`'s `STRATA` means cannot come from `placeholder_mean_color`.** The
   generated means are a different colour entirely from the art that replaces
   them: `base:dirt` `[47, 200, 183]` against the shipped `[138, 106, 70]` is **ΔE
   62.94**; `base:grass` `[174, 139, 81]` against `base:grass_top` `[104, 165, 78]`
   is **ΔE 42.35**; `base:stone` `[47, 191, 136]` against `[126, 126, 126]` is **ΔE
   55.56**.
3. **`share_within(frame, mean, DIFFERENT_COLOR)` no longer measures what its name
   says, for two of the three strata.** It counts pixels within ΔE 10 of a
   texture's mean, and on shipped art that mean is a colour the texture need not
   contain: **0% of a grass side's own texels** sit within ΔE 10 of that side's own
   mean (43.36% for west), because the mean of a two-tone face — dirt body, green
   cap — falls in the gap between them. Stone reaches only **66.41%**, its two
   variant shades sitting at ΔE 10.80 and 12.07. Only `base:dirt` and
   `base:grass_top` reach 100%. A floor derived on the old assumption that a
   texture's pixels cluster around its mean will be measuring a fraction of the
   face it thinks it is measuring.

#### Pairwise ΔE, and the one number that constrains P9's choice of strata

| Pair | ΔE |
|---|---|
| `base:dirt` vs `base:grass_top` | 48.49 |
| `base:dirt` vs `base:stone` | 26.89 |
| `base:grass_top` vs `base:stone` | 53.70 |
| `base:dirt` vs `base:grass_side_north` / `_south` | 10.27 |
| `base:dirt` vs `base:grass_side_east` | 10.65 |
| **`base:dirt` vs `base:grass_side_west`** | **9.59** |
| `base:grass_top` vs each grass side | 38.00 – 39.03 |
| `base:stone` vs each grass side | 27.08 – 27.43 |
| the four grass sides against each other | 0.00 – 1.05 |

> **P9 must not make a grass side a stratum.** `base:dirt` against
> `base:grass_side_west` is **ΔE 9.59, already under `DIFFERENT_COLOR = 10.0`**,
> and `distinct_means` fails on any pair at or under it. Adding a side to the
> strata turns `texture_variety` red **against a correct renderer**, and the
> cheapest way to green that is to raise `DIFFERENT_COLOR` — the one edit that
> would stop the probe telling two textures apart at all. The other three sides
> sit at 10.27 to 10.65, under a single ΔE of headroom, so this is not about one
> unlucky face.

**The constraint is narrow, and the strata themselves are clear.** `base:dirt`,
`base:grass_top` and `base:stone` stand **26.89 to 53.70 ΔE apart**, comfortably
outside both 2.0 and 10.0, so **no spec conversation is owed and nothing has to be
raised or relaxed.** Keep the texture-variety probe taking its strata from those
three. Nothing about the sides is wrong either: a grass side *is* mostly dirt, and
four faces near-identical to each other (0.00 – 1.05 apart) are near-identical on
purpose.

**`COVERAGE_FLOOR = 0.08` is untouched by any of this.** It is a geometric bound
derived from the pose and the heightmap, and no colour measured here bears on it.

### P8's outcome — what P9 has to know

**What landed, and where.** `crates/mc-render/src/texture/mip.rs` (175 non-blank
lines) holds `to_linear`, `to_stored`, `reduced`, `chain` and `levels_for`.
`MIP_LEVELS` and `TextureError` went into `crates/mc-render/src/texture/mod.rs`
beside `LayerError` — the path `mc_render::texture::TextureError` is what P9's
`RendererError::Texture(#[from] TextureError)` names, and it is the path rather
than the file that the tests bind. `SuppliedTexels` already existed from an
earlier phase and was not touched.

**Nothing is wired, and the goldens are what says so.** `mip_level_count` is
still `1`, the sampler is still nearest, and `write_layer` still writes one
level. M49 above is the reading: setting `mip_level_count = MIP_LEVELS` while
nothing fills the extra levels turns four tests red, and with `1` they are green.

**188 is computed, not captured, and P9 must not treat it as device-verified.**
The ten readings in `mip_test.rs` observe no device, no `mip_level_count` and no
sampler. **None of them can tell whether a chain is ever uploaded**, and nothing
in this phase establishes that 188 is what a device sampling that texture would
produce — that is FR-6.2's ground and it has to be captured. Treating this
phase's arithmetic as evidence for FR-6.2 makes the two halves of that claim two
copies of one belief agreeing with each other.

**`levels_for`'s check order is not observable, and it checks the texel count
first.** With 255 texels at edge 16 a chain built anyway still yields five
levels, so `WrongTexelCount` and `TooFewLevels` cannot be told apart by ordering;
the tests bind the variants and their fields and say nothing about which runs
first. Do not read that silence as licence to check late.

**Alpha is averaged linearly and no test discriminates it.** `Rgba8UnormSrgb`
decodes RGB through the transfer function and alpha linearly, so `reduced`
averages alpha where it stands — the format's definition, not a preference. Every
one of the seven shipped textures is fully opaque (measured in T46 above), so
both treatments answer 255 and nothing separates them. The reason is written on
`reduced` itself, where the edit that would reverse it gets made. **The first
translucent texture must bring a test with it.**

**`crates/mc-client/tests/support/probe.rs` is still at 551 non-blank lines
against a 600 limit — 49 lines of headroom** for T54's re-derivation. T46's
measurements above are what keep that growth to the constants themselves.

**T46's figures carry an independent check, and it is recorded with them.** The
means, every ΔE, the distinct-colour counts and the generated means were each
re-reached by a route sharing no arithmetic with the program that produced them —
the shipped `chain`, the shipped `compare`, the shipped `placeholder_mean_color`,
and `content/base/materials/` respectively — alongside a positive control on the
metric's scale and four properties. **The per-colour share percentages have no
second route and the table says so.** This is not ceremony: the phase corrected a
figure in `test-map.md` that an unchecked offline oracle had produced, and T46
produces the same kind of artefact for a phase with no more reason to doubt it
than this one had.

**One correction went back to the test author rather than into their files.**
`test-map.md` and `mip_test.rs`'s module doc reported the narrowest rounding
margin among FR-6.1-S4's four expected texels as 0.3918 of a byte; re-derived
through the shipped arithmetic it is **0.1514**, at 85.349 — 0.3918 is the
*widest* of the four. The conclusion is unchanged (the round trip's worst
pre-rounding error is 1.53e-5 of a byte, so exact equality still holds by three
orders of magnitude) and no assertion moved. The test author corrected both
records.

**The gate is green at this boundary, all twelve stages, exit 0** — read in a
detached worktree at `307900d`, because the shared tree carries an uncommitted
`content/base/blocks/water.luau` that is not this phase's and that correctly
reddens two `mc-world::shipped_declarations_and_an_older_save` guards. In the
worktree, which holds the *committed* water file: **1339 run, 1339 passed, 1
skipped, coverage 94.03%** against an 80% threshold. On the shared tree the same
suite runs 1337 passed and those two failed — **no third red belongs to P8** —
and every gate stage other than `tests` reports ok there as well.

### Measured at `87bbb84`, before any phase opened

Non-blank line counts, taken with the gate's own counter, for the files closest to
a limit:

| File | Lines | Limit | Headroom |
|---|---|---|---|
| `crates/mc-client/src/app/mod.rs` | 500 | 500 | **0** |
| `crates/mc-client/src/session/mod.rs` | 485 | 500 | 15 |
| `crates/mc-client/tests/support/probe.rs` | 551 | 600 | 49 |
| `crates/mc-render/tests/terrain_offscreen.rs` | 460 | 600 | 140 |
| `crates/mc-world/src/content/luau_declaration.rs` | 366 | 500 | 134 |
| `tools/voxforge/src/cli/mod.rs` | 365 | 500 | 135 |
| `crates/mc-client/src/startup.rs` | 364 | 500 | 136 |

Two of these are load-bearing on a phase and are handled rather than noted:

- **`app/mod.rs` binds in P3**, and **T12 is the answer** — a planned split on a
  responsibility boundary, first in the phase, before anything else touches the
  file. Trap 1.
- **`probe.rs` binds in P9**, where T54 re-derives its constants on **49 lines of
  headroom**. That is not a lot for a file whose constants are about to gain a
  derivation each, and it is stated here so P9's pair start with the number rather
  than meeting it. T46's measurements are what keep the growth to the constants
  themselves rather than to exploratory code.

### P2's outcome — what a later phase has to know

**`crates/mc-world/src/content/luau_declaration.rs` reached 509 non-blank lines**
against the 500 ceiling once the `texture` field grew a second form, so the
reading of that field and its seven refusals moved to
`crates/mc-world/src/content/luau_declaration/texture.rs`. Measured after the
split with the gate's own counter: parent **375**, child **175**. The cut is a
responsibility boundary — one field with two shapes, one of which is a table with
a shape of its own — and it is a **child** rather than a sibling so that both
halves raise the parent's own `FieldFault`: the refusals a mod author reads are
one vocabulary held against one page, and a second fault type would be a second
place for the page and the program to disagree — a sibling would have needed that
type's visibility widened to the whole of `content`.

The pair landed first as `luau_declaration.rs` + `luau_declaration/`, which was
**one style deviation in a tree that otherwise agrees with itself**, and the team
lead ruled it back to `luau_declaration/mod.rs`. The reasoning is worth keeping
because it generalises: both layouts give the **identical module tree**, so the
split's load-bearing part argues for neither, which leaves consistency — and
moving this repository to `foo.rs` + `foo/` is a call taken deliberately, never
one made while getting a file under a line cap.

**The convention was measured rather than recalled**, on both sides. After the
conversion: **46 `mod.rs` files** across the tree (32 of them under a `src/`),
and **two** sibling `foo.rs` + `foo/` pairs left — `mc-render/build.rs`, where
Cargo requires a build script to sit, and `mc-client/tests/support/reload_watch.rs`,
which is test support. **Neither is a choice, and there are none at all in
production `src/`.** That is what makes the deviation genuinely alone where it
mattered, which the ruling had assumed and the measurement established.

**T04's "`mc-render` asserts its array-texture extent at compile time" has nothing
to assert, and no assertion was invented.** `mc-render` carried no extent constant
independent of `PLACEHOLDER_SIZE` — that constant *was* the extent, used for the
allocation and the fill alike — so `gpu/buffers.rs` now names `TEXTURE_EDGE`
directly at both sites and there is no second number to hold it against. The
layer bound is genuinely different: `MAX_LAYER` comes from the packed vertex's
eight-bit field, so two independently-derived numbers really do meet there.
`const _: () = assert!(TEXTURE_EDGE == TEXTURE_EDGE)` is a check that cannot
fail, which `testing.md` §2 forbids outright — and worse than useless, because it
*reads* as a guard. **Ruled by the team lead as a discharged task rather than a
gap**: an assertion needs two independently-derived values, T04's instruction
rested on the premise that `mc-render` had a second one, and that premise was
false.

Worth noticing across the whole spec: this is the **third** planning premise that
did not survive contact with the tree, after the shared version byte and the
`command_line.rs` "absence assertion" that turned out to be a doc comment. All
three are statements about what the tree already contains, made by reading it
rather than grepping it.

**P3 through P9's task text is dense with that shape** — `reload.rs:81`,
`geometry/mod.rs:180-192`, `placeholder.rs:80`, `texture/mod.rs` at 117 lines,
`app/mod.rs` at exactly 500. The cheap mitigation is for each phase's implementer
to check their own brief's line-and-file claims **before** building on them
rather than after, which is how all three of these were caught.

**`drawn_of`'s widening to six keys had no witness, and now has one.** T10's "no
new scenario is owed" was measured false by M10; P2's test author authored the
pair at `854d894` — a candidate re-pointing `north` marks every section, and one
restating the same six marks none — on the section-count instrument
`rendering.md:497-510` already names. **Re-running M10 against them reddens
exactly the first, 1 of 1248.** Both arrived green, because T10 was already
correct, so the falsifier *is* the evidence rather than a displayed red. The
control is the half that matters to whoever touches this next: without it, an
implementation marking whenever a declaration states a table would satisfy the
first reading.

**The appearance revision byte is invisible to a fold-versus-fold comparison** —
see M8. Only a test that states the byte sequence can see it. Whoever next moves
either revision should expect `format_test`'s halves and
`save_per_face_appearance`'s stated-bytes guard to be the whole of what reports
it, and should not read a green `shipped_declarations_and_an_older_save` as
evidence the byte is right.

### T12's outcome — the headroom the split left

Measured with the gate's own counter after the split, and again after T18 had
changed `swatch`'s lookup:

- **`crates/mc-client/src/app/mod.rs` — 460 non-blank lines of 500. Forty free.**
  41 came out (the three reporters with their doc comments, which is exactly the
  cluster size the task estimated) and one went back in as `mod report;`. T18's
  edit was a same-shape replacement and moved the count by nothing.
- **`crates/mc-client/src/app/report.rs` — 61 of 500.**
- `crates/mc-client/src/startup.rs` moved from 364 to **369** of 500, because
  `build_section_geometry`'s third argument no longer fits on one line.
- `crates/mc-client/src/session/mod.rs` is untouched and still at 485. It remains
  the second file to re-measure before touching.

**The task's "green either side" could not be taken literally, and what was
checked instead is stated here rather than claimed as the stronger thing.** The
tests commit had already landed, so no `cargo nextest` run could compile either
side of T12 — that is the adaptation window this file's "Reading a gate run"
section warns about. What was run either side was
`cargo build --workspace --all-features` (green) and
`cargo clippy --workspace --all-features -- -D warnings` (clean). The suite's
first green of the phase is at `ac6f0df`.

### P3's outcome — what the next phase has to know

- **The two PRO-902 sites are closed together and neither parses a block name.**
  M12 and M13 are the evidence that they are independent: each reddens its own
  site and leaves the other entirely green.
- **The mapping hole P2 could not close is closed.** M16 — M6 re-run — bit 3 of
  1261 where M6 bit 0 of 1246. `per_face_axes.rs` is what closed it.
- **`Quad` must not gain a resolved key or layer.** That is now written into
  `docs/technical/rendering.md` and `crates/mc-render/CLAUDE.md`, and **M14′ is
  the only instrument that would see it undone** — not M14, which cannot reach
  FR-2.1-S4 at all. A later phase re-running this table must run M14′, and its
  non-bite is a stop.
- **Deferred observation, outside this phase's diff.** The two new refusals a mod
  author can now meet — the packer's `UnresolvedTexture` and the indicator's
  unresolved report — are quoted in `docs/modding/blocks-items.md` as block
  quotations rather than as fenced terminal transcripts. `documented_refusals.rs`
  scans `docs/modding/` for fenced blocks beginning `mycraft: ` and compares them
  against a real run, and the run it compares against is assembled in
  `tests/support/printed_refusals.rs`, which the implementation context may not
  edit. Quoting either as a terminal line needs that harness to produce it first.
  The page's own header already records that the guide quotes three of roughly
  fourteen reachable refusals; these are two more on that list.

### P5 leg one's outcome — what leg two has to know

Closed: **T25, T26, T27, T28, T29, T30**. Fifteen scenarios — FR-3.1-S1…S6,
FR-3.2-S1…S3, FR-3.3-S1…S6, FR-3.4-S4 — plus the ten additional-coverage
readings in `art_index.rs` and `art_fold.rs`.

- **Arrival, measured rather than inherited.** `mc_core::art` did not exist, so
  the two `mc-core` test binaries did not compile. A deliberately naive `art.rs`
  — no validation, unpadded fold, lenient parse, no length prefixes — was written
  first to reach the assertions: **7 of the 9 new `mc-core` tests red at the
  assertion**, the round trip and `NotAnIndex` green under it, which is exactly
  the pair M33 and M34 exist for.
- **T34's four scenarios are already green** — FR-3.4-S1, S2, S3 and S5 — and so
  are FR-3.3-S8 and FR-3.3-S9. Not scope creep: T30's cache key **is** the fold,
  so which sources are folded had to be decided in this leg. **T34 is therefore
  verification and mutation rather than implementation**, and M26 is its
  instrument. FR-3.3-S9 came free from D12's grouping, as the architecture said
  it would.
- **The refusal order the remaining tasks land in.** `load_manifest` refuses
  before anything is read; the fold's sources are read next and a model that is
  not there is refused there, with the key that named it; only then are models
  loaded and emitted; the index is rendered last and nothing is written until it
  exists. T31's seam and edge refusals belong between the emission and the
  encoding; T33's file-name and line-break refusals belong in `load_manifest`,
  where the author can still see their own manifest.
- **FR-3.3-S13 is red for a reason worth knowing.** A line-break key is already
  refused today, but by `TextureSetIndex::stating` after every image is baked,
  and the message quotes the *derived image name* (`base__line\nbreak.png`)
  rather than the key — so the test's `base:line` token is missing. Moving the
  refusal to the manifest, which is what T33 asks for, fixes both halves.
- **The committed manifest was measured before it was committed.**
  `grass-block.mcvox` and `stone-block.mcvox` are each a cube of their declared
  scale and all twelve faces tile, so neither the cubic precondition nor T31's
  seam refusal will refuse the shipped build. The four grass side images are
  pairwise unequal, which is the premise P9's FR-8.1-S3 rests on.
- **The image name is derived in one place** — `image_named` in
  `cli/build.rs` — and D9's validation is the only thing missing from it. The
  client must not re-derive it: it reads the name from the index.
- **Deferred observation, outside this leg's diff.** A manifest naming a model
  outside the manifest's own directory — `model = "../shared/x.mcvox"` — builds
  cleanly and writes an index whose `source` line `parse` refuses as an unsafe
  path, so the set would be unreadable to the client that has to read it.
  `stating` validates control characters and nothing else, by the test author's
  stated interface decision, so nothing catches it on the way out. No scenario
  reaches this and it is not built here.
- **Deferred observation, recorded by the test author and left standing.**
  `stating` cannot refuse an image name carrying a space, which would render a
  `key` record that parses back on the wrong split. voxforge's own file-name rule
  (FR-3.3-S12, T33) is what stops it arising, so it is defence-in-depth that is
  currently absent rather than a live defect.
- **The gate cannot be green at this boundary and that is the split working.**
  Every stage but the tests passes on the shared tree — format, clippy with
  `-D warnings`, the gpu-free pair, rustdoc's intra-doc links, `cargo machete`,
  and the size limits (the largest file this leg wrote is `art.rs` at 386
  non-blank of 500). The test stage is red on leg two's seven scenarios and
  stays red until T31–T34 land.
