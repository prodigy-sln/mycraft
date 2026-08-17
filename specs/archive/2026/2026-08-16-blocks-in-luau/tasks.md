# Tasks: Blocks are defined in Luau — swap the loader, not the registry

**Spec**: [spec.md](spec.md) (SPEC-016, rigor `high`, **73 scenarios**) ·
**Architecture**: [architecture.md](architecture.md) (binding) ·
**Branch**: `feature/PRO-917-blocks-in-luau` · **Issue**: PRO-917 ·
**Created**: 2026-08-16 · **Amended**: 2026-08-16

One task = one coherent scenario group in one area. Phases are the
architecture's: 25 · 7 · 11 · 9 · 15 · 6 = 73. `[P]` = independent of other `[P]`
tasks in the same phase.

> **Amended 2026-08-16.** `docs/planning/client-server-split.md` is binding and
> adds FR-7: the simulation loads content and the client receives it resolved.
> Phases 1–4 are untouched by it. Phase 5 gains the construction move and the
> client-source scan (12 → 15 scenarios); phase 6 is new (6). Phase 1 is
> implemented and closed; nothing in the amendment reopens it.
>
> **FR-7.4-S3 came from tracing the packing path** rather than from the planning
> document, and it pins a live defect the tree already has: the layer assignment
> is built from each block's declared `texture` and looked up by the block's
> `name`, the two agreeing only because every shipped block declares them
> identical. Closing it is PRO-902/PRO-914's. **It has a consequence for T22
> that nothing mechanical would catch** — see the constraint recorded there.

---

## Read this before implementing phase 1

### The sequencing trap, named out loud

**Phase 1 lists `<root>/blocks/` in the order the filesystem returns it,
filters nothing, and takes a refusal's origin from the `ScriptFault`. It does
not sort. It does not filter by extension. It does not own the origin.**

The specific move that springs the trap is **copying
`crates/mc-world/src/content/toml_source.rs:47–58` wholesale** — the
`declaration_files` helper, whose eleven lines do the listing, the `retain` on
extension and the `sort_by` on file name in one breath. It is the obvious
starting point, it is what a competent implementer reaches for, and taking it
hands phase 2 four of its seven scenarios (FR-1.2-S1, S2, S4 and, through the
origin, FR-3.3-S1) green on arrival. A scenario that never had a failing step is
not evidence, and this project has shipped that hole twice already.

Take from `toml_source.rs` in phase 1: `origin_of` (line 75), `unreadable`
(line 80), the `DefinitionSource` impl shape (line 61), the `BLOCKS_DIRECTORY`
constant (line 20). Leave behind: lines 55 and 56, and the
`DECLARATION_EXTENSION` constant on line 23. Phase 2 writes them, deliberately,
against tests that were red first.

`testing.md` §1 calls this "implement deliberately less first". It is not
sloppiness deferred; it is the only way phase 2's scenarios can ever be red.

### The skeleton that reddens phase 1 is the *registering* one

Phase 1 holds 25 scenarios and cannot be made smaller: all-or-nothing,
duplicate naming and empty-source refusal come free from `registry.rs`, the
budget, memory cap and sandbox come free from `ScriptHost::evaluate`, and
`__index` rawness comes free from `read_field`. **None can be reddened by a
later phase — only by there being no loader yet.**

**Seventeen of phase 1's twenty-five scenarios assert a refusal**, so a
refuse-everything skeleton passes them for the wrong reason. The skeleton that
walks `<root>/blocks/` and yields one fixed `BlockDefinition` per file reddens
all 25: the eight registering scenarios fail on field values, the seventeen
refusing ones fail because something registered.

> *Discrepancy, resolved in favour of the tree:* `architecture.md:441` says
> "Fourteen of phase 1's twenty-five scenarios assert a refusal". Counted
> scenario by scenario it is seventeen (FR-1.1-S3/S4; FR-2.1-S3/S4/S5/S6/S7;
> FR-2.3-S1/S2; FR-3.1-S1/S2; FR-3.2-S1; FR-4.1-S1/S2/S3/S4; FR-4.2-S2) against
> eight that register (FR-1.1-S1/S2/S5; FR-2.1-S1/S2; FR-3.1-S3; FR-4.2-S1/S3).
> The architecture's conclusion is unaffected and holds more strongly, not less.

---

## Phase 1 — A declaration is evaluated, checked and registered, under the host's guard

**25 scenarios.** Done means: a content root of `.luau` declarations registers
through `BlockRegistry::apply`, every refusal names what it can name, and the
host's guard is the loader's guard because the loader goes through
`ScriptHost::evaluate` at `HostLimits::default()` — never a test-sized host.

- [x] **T01** The `mc-world → mc-script` edge, the source, and a chunk that
      returns a table — `crates/mc-world/Cargo.toml`,
      `crates/mc-world/src/content/luau_source.rs` (new),
      `crates/mc-world/src/content/mod.rs`
      Scenarios: FR-1.1-S1, FR-1.1-S2, FR-1.1-S5
      - `mc-script` added to `[dependencies]` with a note in the style of the
        `toml`/`postcard` entries (`Cargo.toml:11–13`, `15–19`) confining it to
        `src/content/`. **`mlua` must never appear in this manifest** — that is
        the vendor blast-radius rule (Boundaries), and Rust's extern-crate rules
        are what make it structural.
      - D2 is binding: a host is constructed **inside** `definitions()`, used
        for every file, and dropped before the call returns; the stream is
        `Box::new(collected.into_iter())`. No `RefCell<ScriptHost>`, no handle
        outliving its file. The source holds a `PathBuf` and a record of what was
        printed, and is therefore `Send` — `launch.rs:143` spawns the thread it
        is built on.
      - `HostLimits::default()`. FR-1.1-S1 is what stops FR-4.1's four
        all-unwanted scenarios passing against an absurdly small budget, and it
        can only do that if the shipped limits are what runs.
      - A host that will not start is `Unreadable { origin: <root>/blocks }`
        quoting `HostError` — not a malformed declaration; it is not the
        content's fault.
      - **FR-1.1-S5 is a weak instrument.** Under D2 it is structurally true:
        there is no shared mutable host to borrow twice, so the panic it guards
        against is unexpressible rather than avoided. It stays because it costs
        nothing and would catch a later drift back to interior mutability — it
        is not evidence this task did work.
      - See the sequencing trap above before writing the directory listing.

- [x] **T02** [P] The chunk returned something that is not a declaration —
      `luau_source.rs`
      Scenarios: FR-1.1-S3, FR-1.1-S4
      Depends on: T01
      - Both are `Malformed`, origin = file path, `block: None`, `field: None`
        (D7 step 5, error contract). A function and a bare `return` are different
        `ScriptValue`s and must not be distinguished in the refusal — there is no
        block and no field to name in either.

- [x] **T03** [P] The three required fields, read raw, with their kinds and the
      namespaced-id rule — `luau_source.rs`
      Scenarios: FR-2.1-S1, FR-2.1-S2, FR-2.1-S3, FR-2.1-S4, FR-2.1-S5, FR-2.1-S6, FR-2.1-S7, FR-2.3-S1, FR-2.3-S2
      Depends on: T01
      - The field/default/refusal logic in `crates/mc-world/src/content/raw.rs`
        ports across against `ScriptValue`; the `serde` derive does not. `raw.rs`
        is **not** deleted until phase 5.
      - `name` is read raw first, for attribution (D7 step 6, as `raw.rs:99`
        does today), so every later refusal in the file can name the block. A
        `name` that is absent or not text leaves `block: None` — FR-2.1-S6 and
        FR-2.1-S7 are what hold that, there being no name to quote back.
      - `solid = "yes"` refuses naming the field **and the kind of value it
        holds**; `texture` is checked against `name` as a distinct field —
        FR-2.1-S2 exists because every scenario in the spec's first draft gave a
        block a texture equal to its name, and a loader reading `name` into both
        was green throughout.

- [x] **T04** [P] All-or-nothing, and a name declared twice — assert through
      `BlockRegistry::apply`; no new production code expected
      Scenarios: FR-3.1-S1, FR-3.1-S2, FR-3.1-S3, FR-3.2-S1
      Depends on: T01
      - `RegistryError::NoDefinitions` and `::AlreadyRegistered` come from
        `crates/mc-core/src/block/registry.rs` and **must not be
        reimplemented** — the spec's whole claim is that the registry is
        untouched. If a scenario here needs production code in `mc-world`, that
        is a signal the stream contract is being broken, not a licence to write
        it.
      - The duplicate refusal names `amber.luau` first and `zinc.luau` second,
        which is only well defined because the root is read in a fixed order —
        and in phase 1 that order is the filesystem's. **The fixture must
        therefore not depend on sorting**; phase 2 is where the order becomes a
        contract. FR-3.2's positive control is FR-1.2-S1, in phase 2.
      - **FR-3.1-S3 is a weak instrument**, for the same reason as FR-1.1-S5:
        D2's fresh host per call makes "apply the repaired root again" true by
        construction. Say so in `test-map.md` rather than letting it read as a
        strong one.

- [x] **T05** [P] The guard is the host's — budget, memory cap, sandbox, frozen
      environment — `luau_source.rs`
      Scenarios: FR-4.1-S1, FR-4.1-S2, FR-4.1-S3, FR-4.1-S4
      Depends on: T01
      - This task is **wiring, not mechanism**: all four come free from
        `ScriptHost::evaluate`, and what they assert is that the loader goes
        through it rather than round the side of it. They assert through the
        loader for exactly that reason.
      - D6 is binding: the cause is composed from `ScriptFault::kind`, `::line`
        and `::cause` — never from `ScriptFault: Display`, which opens with
        ``chunk `amber.luau` `` and would state the location twice.
        `crates/mc-client/tests/refusals_state_a_cause_once.rs` counts
        occurrences and exists because that duplication shipped once already.
      - `FaultKind::as_str` already spells "call and loop budget exhausted" and
        "allocation refused"; FR-4.1-S1 and FR-4.1-S2 assert against those words,
        which become a documented contract the moment they land.
      - `mc-script/CLAUDE.md` records that **one limit masks another**: filling
        a megabyte costs more interrupt ticks than a small budget allows, so a
        memory test under a small budget dies of ticks and reports the wrong
        limit while passing. FR-4.1-S2 names the memory cap and not the budget
        for that reason.

- [x] **T06** [P] `__index` never runs, and the source reports what content
      printed — `luau_source.rs`
      Scenarios: FR-4.2-S1, FR-4.2-S2, FR-4.2-S3
      Depends on: T01
      - `ScriptHost::read_field` is already raw against `__index` and PRO-916
        tests it. What is new here is `LuauFileDefinitionSource::printed()`,
        which exists because FR-4.2-S3's observable — "the host having recorded
        nothing printed on that chunk's behalf" — is otherwise unreachable: the
        host is internal to `definitions()` under D2, and asserting against a
        separately constructed `ScriptHost` would be agreement between two copies
        of one decision.
      - **`printed()` needs a positive control that is not in the spec**: a
        declaration calling `print` at top level must show up in `printed()`, or
        an implementation that never records anything leaves FR-4.2-S3 green
        forever. It goes in `test-map.md` under additional coverage, and it is
        also the control FR-4.3-S9 depends on in phase 4 (see T16).
      - Assumption 4 (architecture): `printed()` reports the **last read**, not
        an accumulation across reads.

---

## Phase 2 — Which files are declarations, and where a refusal points

**7 scenarios.** Done means: the loader decides for itself which entries are
declarations and in what order it reads them, and a refusal points at a path a
person can open rather than at a chunk name. **Red on arrival only if phase 1
did the minimum** — if FR-1.2-S1, FR-1.2-S2 or FR-3.3-S1 is green when the phase
opens, phase 1 took `toml_source.rs:47–58` and the phase must be reopened, not
accepted.

> *Corrected at phase 2, having been read against the tree:* this condition
> originally named **four** scenarios, including FR-1.2-S4. **FR-1.2-S4 does not
> discriminate and has been removed from the list rather than left to mislead.**
> A directory named `nested.luau` is refused as `Unreadable` naming its path
> under the extension filter *and* equally under no filter at all — with no
> filter the path reaches `fs::read_to_string`, which fails on a directory and
> produces the same refusal. Its state is evidence about neither implementation,
> so reading its greenness as proof that phase 1 took the trap would send a
> reader to reopen a sound phase, with no way to know the signal could not
> discriminate. **Phase 1 was verified directly instead** — `declaration_files`
> carried no `retain` on extension and no `sort_by` — which is the order of
> evidence this check should have asked for in the first place. FR-1.2-S4 is
> still worth having: it is the only thing that reddens against a loader that
> *skips* a non-file rather than refusing it, which a phase 2 mutation confirmed.

- [x] **T07** The extension filter, the file-type check, and the file-name sort
      — `luau_source.rs`
      Scenarios: FR-1.2-S1, FR-1.2-S2, FR-1.2-S3, FR-1.2-S4
      - D7 steps 1–2, in order: list; keep entries that are **files** with
        extension `luau`; sort by file name.
      - FR-1.2-S4 is why the entry's **file type** is checked and not only its
        name: a *directory* named `nested.luau` is refused. `toml_source.rs:55`
        checks the extension alone and would treat it as a declaration.
      - FR-1.2-S1's fixture makes file-name order and block-name order
        **disagree** (`amber.luau` declares `example:zinc`). The spec's first
        draft used a degenerate fixture where both orders agreed, which stayed
        green against a loader that dropped the sort entirely.
      - FR-1.2-S1 is also FR-3.2-S1's positive control — two files declaring two
        distinct names both register — which is why FR-3.2 has no accept-side
        scenario of its own.

- [x] **T08** The loader-owned origin, and the composed cause — `luau_source.rs`
      Scenarios: FR-3.3-S1, FR-3.3-S2, FR-3.3-S3
      - **D5 is binding and is what makes FR-3.3-S1 falsifiable**: the chunk name
        handed to `ScriptHost::evaluate` is the file's name alone
        (`amber.luau`); the `DefinitionOrigin` is built by the loader from the
        full path via `origin_of`. If the loader passed the full path as the
        chunk name the two would coincide and the scenario would be true by
        construction. An implementation that lifts the origin out of the
        `ScriptFault` then produces `amber.luau` and goes red — which is the
        whole point of running it.
      - FR-3.3-S2 names the line the compiler named; FR-3.3-S3 names the error
        the chunk raised. Both `block: None` — a chunk that did not return has no
        declaration to attribute to.

---

## Phase 3 — The optional fields, the residue id, and the field nobody recognises

**11 scenarios.** Done means: all six fields are read with their documented
defaults, a field the loader does not recognise is refused rather than silently
lost, and the enumeration that makes that possible runs no script.

- [x] **T09** `ScriptHost::field_names` — the raw key enumeration —
      `crates/mc-script/src/host.rs`, `crates/mc-script/src/luau/vm.rs`,
      `crates/mc-script/src/lib.rs`
      Scenarios: FR-4.2-S4, FR-4.2-S5
      - D3's shape is binding: `FieldNames::Enumerated(Vec<String>)` /
        `MoreThanAllowed { allowed: usize }`, and
        `field_names(&self, table: &ScriptTable, most: NonZeroUsize)`.
      - Four binding properties. **Raw** — no `__index`, `__iter`, `__pairs` or
        `__len` consulted; verify the backend call actually is raw rather than
        assuming it, following `vm.rs:293` and `vm.rs:312`. **Bounded at the
        host, bound as a parameter** — this is the only place the field-count
        bound can bind, since an unbounded `Vec<String>` allocates the
        hundred-thousand-key table the bound exists to refuse; `mc-script` may
        not learn a block-specific number. **Total over key types** — a
        non-string key is rendered to text by the same rendering `print` uses,
        never skipped. **Sorted lexicographically, in `mc-script`, documented as
        such** — Lua leaves hash-part order unspecified, so the state's order
        carries no information, and an unstable order makes
        `documented_refusals.rs` intermittently red (it compares line for line at
        `documented_refusals.rs:420`).
      - `__index` is *not* the metamethod this capability is exposed to;
        `__iter` and `__len` are, because `pairs` is a permitted global. FR-4.2-S4
        (an `__iter` that hides a field) and FR-4.2-S5 (an `__iter` that invents
        one) are what hold it.
      - **Totality over non-string keys has no scenario** (architecture
        Assumption 1). It gets a unit test in `mc-script` and an entry in
        `test-map.md` under additional coverage; see "Mechanisms no scenario
        covers" below.

- [x] **T10** The field nobody recognises — `luau_source.rs`
      Scenarios: FR-2.4-S1, FR-2.4-S2, FR-2.4-S3
      Depends on: T09
      - D7 step 7: enumerate at `field_names(table, 64)` **before** the six
        fields are checked, so a table of 65 fields refuses on the field-count
        bound (phase 4) rather than on a missing `solid`.
      - FR-2.4-S1's refusal names the file, the block, the offending field
        **and the fields it does recognise** — that recognised list needs a
        settled order too, not only the unrecognised one.
      - FR-2.4-S3 is what holds the order settled: two unrecognised fields, named
        **in the same order on every run**. Run it repeatedly against a table
        built with several unrecognised keys; an intermittently red
        `documented_refusals.rs` is the failure this prevents, and this project's
        standards rank intermittent red below plainly red.
      - **FR-2.4-S2 is green on arrival** — see the controls section below.

- [x] **T11** [P] The three optional fields and their defaults —
      `luau_source.rs`
      Scenarios: FR-2.2-S1, FR-2.2-S2, FR-2.2-S3, FR-2.2-S4
      Depends on: T09
      - Defaults unchanged from today: not replaceable, breakable, empty cell on
        break. Independence matters — FR-2.2-S2 registers an unbreakable block
        *with* a residue, the residue simply never being reached.
      - A wrong kind refuses rather than falling back to the default
        (FR-2.2-S3/S4). That is the distinction between "absent" and "stated
        wrongly", and a fallback would make the refusal contract partial.

- [x] **T12** [P] The residue id follows the same rule as every other id —
      `luau_source.rs`
      Scenarios: FR-2.3-S3, FR-2.3-S4
      Depends on: T09
      - FR-2.3-S4 is the one that constrains the design most: an unresolvable
        residue **registers**. A residue is resolved where a break reads it, not
        where it is declared — the loader must not go looking for
        `example:ash`.

---

## Phase 4 — Every content-supplied quantity has a bound

**9 scenarios.** Done means: each of the four loader bounds refuses the value
above it and accepts its own limit, in the fixed check order, and the retained
script output the loader first made reachable is bounded and its truncation is
visible. **T13 runs before anything else in this phase is built.**

- [x] **T13** **Verify first** — 4 096 chunks in one script state against the
      16 MiB backstop — spike over `luau_source.rs`
      Scenarios: FR-4.3-S6
      - D2 keeps one script state for a whole `definitions()` call, so compiled
        chunks and their residue accumulate across up to 4 096 files, and the
        **only public lever that forces a collection is
        `ScriptHost::collected_memory_in_use()` — a getter used for its side
        effect** (`host.rs:240`). This is the architecture's first-listed risk
        and FR-4.3-S6 is the scenario that finds out.
      - If it trips, the named fallbacks in order: drop handles earlier; force a
        collection every N files; and **last**, give the loader its own
        documented `HostLimits` — which is marked **DEFERRED** and revisited only
        if the first two fail, because FR-1.1-S1 must still run at the shipped
        limits whatever happens here.
      - FR-4.3-S6 is nominally a control that is green on arrival (see below),
        **but it may be red on arrival for a reason that has nothing to do with
        the bound** — that is exactly what this task exists to find out, and
        which of the two it is must be recorded rather than inferred.

- [x] **T14** The declaration-count bound and the file-size bound, in check
      order — `luau_source.rs`
      Scenarios: FR-4.3-S1, FR-4.3-S2, FR-4.3-S7
      Depends on: T13
      - **Check order is what these assert, not just the numbers** (D7 steps
        3–4). Count the listing **before reading anything** — FR-4.3-S1 puts an
        invalid Luau file among 4 097 and requires the count bound to win, naming
        the directory and *not* the invalid file. Take size **from directory
        metadata before opening** — FR-4.3-S2 puts a syntax error in a 300 KiB
        file and requires the size bound to win, naming the file and *not* the
        syntax error. An implementation that reads first and checks after passes
        neither.
      - D8: `4_096` declarations, `256 * 1024` bytes. Each refusal states **the
        observed quantity and the bound**, so a reader can tell "slightly over"
        from "far over".
      - FR-4.3-S7 is the accept side (exactly 256 KiB registers) and is green on
        arrival — see below.

- [x] **T15** [P] The declared-text bound and the field-count bound —
      `luau_source.rs`
      Scenarios: FR-4.3-S3, FR-4.3-S4, FR-4.3-S5, FR-4.3-S8
      Depends on: T13
      - D8: `256` **characters** measured by `chars().count()`, not bytes — the
        spec says characters, and bytes would refuse non-ASCII at a different
        point than the documentation states. `64` field names, enforced **inside**
        the enumeration (T09), not after it.
      - D8 also corrects the spec: the declared-text bound does **not** protect
        the copy out of the script state — `read_field` returns
        `ScriptValue::Text(String)`, so that copy is already made. What it really
        bounds is what a `BlockDefinition` **retains** (4 096 × three strings);
        the transient copy is separately bounded at ~16 MiB by the host's memory
        backstop. Keep the bound, document the reason that is true.
      - A field-count refusal **may** name the block if `name` was already read
        (architecture Assumption 2); FR-4.3-S5 asks for the file and the bound
        and does not forbid the block.
      - FR-4.3-S8 is the accept side (exactly 256 characters registers) and is
        green on arrival — see below.

- [x] **T16** The bound on script output the host retains, and its truncation
      counter — `crates/mc-script/src/limits.rs`,
      `crates/mc-script/src/host.rs`, `crates/mc-script/src/luau/vm.rs`
      Scenarios: FR-4.3-S9
      Depends on: T13
      - Why this is in scope at all (D4): `ScriptHost::printed` is a
        `Vec<String>` that grows without limit (`host.rs:69`, `host.rs:291`;
        `vm.rs:376–384` pushes every `print`), and its own doc comment says so.
        Nothing in production called `evaluate` or `dispatch` before, so no
        content could reach it — **this spec builds the first path**. Each
        `print` costs one interrupt tick, so a chunk can make on the order of
        500 000 calls inside its budget, each string built inside the per-entry
        cap and then becoming script-side garbage while the host-side copy is
        retained outside every limit. Across 4 096 declarations that is tens of
        gigabytes: Invariant 3 breached by a file anybody can write. Draining
        between declarations does not answer it — one chunk alone exceeds any
        per-file drain.
      - Three binding properties. **It lives in `HostLimits`** as
        `retained_print_bytes: NonZeroUsize`, beside the budget and the memory
        cap, so an operator can read and set it. **Reaching it stops recording
        rather than dropping the oldest** — the first line a chunk printed
        locates a failed load; the millionth does not. **Truncation is
        observable** via `dropped_print_lines(&self) -> u64`.
      - FR-4.3-S9 asserts the *distinction*, not merely that the buffer stopped
        growing: a record the host truncated must read **differently** from one
        nothing printed to.
      - **Its positive control is already owed** and is the one from T06: a
        truncation counter that never counted reads exactly like a chunk that
        printed nothing, so without a control proving a top-level `print` shows
        up in `printed()`, FR-4.3-S9 passes over a recorder that never records.
        Both entries go in `test-map.md`; if T06's control was not written, it is
        written here before FR-4.3-S9 is accepted.

---

## Phase 5 — The base game ships in Luau, the simulation is what loads it, TOML is retired, the pages are true

**15 scenarios.** Done means: `content/base/blocks/` holds four `.luau` files
and no `.toml`, `TomlFileDefinitionSource` and `raw.rs` are gone from the tree,
**the simulation and not the client constructs the definition source, the HUD
source and the registry they are applied to**, existing saves load unchanged, the
client's own sources name none of the four content doors, and the documentation
is true for all three audiences with every retired statement actually retired.
**Run the golden-frame suites before the documentation work, not after.**

**What this phase deliberately does not do**, because phase 6's scenarios have to
be red when it opens: it does not define the resolved content value, and it
leaves `startup.rs:335`'s `layers_of` deriving a layer index positionally over
the sorted key set. Moving the construction and defining what travels across it
are two changes, and taking both here hands phase 6 four of its five scenarios
green on arrival. Same discipline as the phase-1/phase-2 split, same temptation.

- [x] **T17** The four base declarations, and the save that must still load —
      `content/base/blocks/{dirt,grass,stone,water}.luau`
      Scenarios: FR-1.3-S1, FR-1.3-S2, FR-1.3-S4
      - Fields preserved exactly as recorded in `requirements.md`: only `water`
        declares `solid = false` and `replaceable = true`.
      - **FR-1.3-S4 is the single most valuable scenario in the spec** and the
        only one comparing a whole resolved definition against a fixed oracle.
        `crates/mc-world/src/persistence/format.rs:307,319` fold a definition into
        `DeclaredBehaviour` and `DeclaredAppearance` and **deliberately exclude
        `origin`** — which is what makes a save written against the TOML
        declarations loadable against the Luau ones. It is what would catch a
        field mapped to the wrong place or a default resolved differently by the
        new reader; nothing else in the spec can.
      - FR-1.3-S2 (`base:dirt` in hand) depends on registration order, which is
        the file-name sort from phase 2 — it is the shipped-content witness for
        `docs/modding/README.md`'s first-solid-block rule.

- [x] **T18** Both call sites, not one — and both move to `mc-sim` —
      `crates/mc-sim/` (new content module),
      `crates/mc-client/src/launch.rs:40,188`,
      `crates/mc-client/src/startup.rs:27,188,267,370`,
      `crates/mc-client/src/main.rs:50`
      Scenarios: FR-1.3-S3, FR-5.1-S3
      Depends on: T17
      - **D9 is binding: the construction moves, the loader does not.**
        `LuauFileDefinitionSource` and `TomlFileHudSource` stay in
        `crates/mc-world/src/content/`; `mc-script` keeps its
        no-workspace-crate property; no crate moves and `mc-client` gains no
        dependency. `mc-sim` gains the function that turns a content root into
        loaded content, and `mc-client` calls it.
      - **Four things leave `mc-client`, not two.** `registry.apply(` and
        `BlockRegistry::new` in `launch.rs:187–188` and `startup.rs:266–267`;
        `HudLayout::load` in `launch.rs:192`, `startup.rs:272` **and
        `startup.rs:370` (`empty_hud`)**; and `content_root` at
        `startup.rs:188`, whose one caller is `main.rs:50`. Leaving `empty_hud`
        or `content_root` behind means exempting them from T18b's scan, and an
        exemption on a door the guard exists to watch is how a guard stops being
        one.
      - **`prepare_launch` and `prepare_scene` themselves do not move**, and
        neither does `layers_of` or `scene_of`. They keep their names and their
        place in `mc-client`; the golden frames are shot through them and the
        exit criterion is decided there. What leaves is the construction.
      - `prepare_scene` (`startup.rs:267`) is **the golden-frame path**.
        FR-5.1-S3 exists precisely because it is a second entry point onto a path
        the first one's tests already cover — a second entry point is untested
        until something asserts **through it**, not through the caller that was
        already covered.
      - **FR-1.3-S3 is green on arrival**: `PreparationError::NothingToPlace` at
        `launch.rs:123` already implements it. It is a regression guard here, not
        new work — see below.
      - Run the golden-frame suites at this point, before T22. A difference here
        is a wiring defect in the move; phase 6 runs them again, and a difference
        *there* is the layer assignment. Running them once at the end cannot tell
        the two apart.

- [x] **T18b** The client's own sources name no content door —
      `crates/mc-client/tests/` (a new guard)
      Scenarios: FR-7.1-S1, FR-7.1-S2, FR-7.1-S3
      Depends on: T18 for green; **written before T18** so it is red first
      - Four needles, and they are **chokepoints rather than type names**:
        `registry.apply(`, `HudLayout::load`, `BlockRegistry::new`,
        `content_root`. Renaming a source does not rename the door. The tree
        documents that `BlockRegistry::apply` is the only way to populate a
        registry (`crates/mc-core/src/block/registry.rs`) and that
        `HudLayout::load` is the only door into a layout
        (`crates/mc-core/src/hud/layout.rs`), which is what makes those two
        total rather than representative.
      - `crates/mc-client/tests/seam_boundaries.rs` is the shape: the
        `Guard`/`Scan` pair, **exemptions compared against the whole path
        relative to the crate root and never against a bare file name**, doc
        comments stripped so prose about a door is not a use of it, sibling
        `*_test.rs` files skipped, and a `tempfile` fixture as the positive
        control. That file's own doc comment records why the whole-path form
        exists; read it before writing a needle list.
      - The scan reads `crates/mc-client/src/` and **wants no exemption at all**.
        If one turns out to be needed, that is a signal a door was left behind
        rather than a licence to write the exemption.
      - **FR-7.1-S3 is why the verdict is enumerated rather than
        `hits.is_empty()`.** An empty answer and a scan that can no longer look
        are different facts, and an absence assertion cannot tell them apart.
      - **Known residual, and it belongs in the guard's own doc comment:**
        somebody adding a *second* door — a new public registration call —
        bypasses this, and no text scan closes it. The instrument that would is
        the dependency-closure guard, which cannot pass while one binary hosts
        both halves and is therefore the composition-root spec's exit criterion.

- [x] **T19** Delete the TOML block loader and retire the type across the tree —
      `crates/mc-world/src/content/{toml_source.rs,raw.rs,mod.rs}`,
      `content/base/blocks/*.toml`, 11 test files in three crates
      Scenarios: FR-5.1-S1, FR-5.1-S2
      Depends on: T18
      - `toml_source.rs` and `raw.rs` are deleted whole. `content/mod.rs`'s
        module doc says this module "is the only place in the workspace that
        knows a definition is spelled in TOML" — rewrite it; after this change
        it knows a definition is spelled in Luau, and the HUD source beside it is
        what still knows TOML.
      - **The retirement surface, counted in the tree** (the architecture says
        "13 test call sites"; that number is the *total* `::new` count):
        13 `TomlFileDefinitionSource::new` call sites in all, of which **2 are
        production** (`launch.rs:188`, `startup.rs:267`, both T18's) and **11 are
        tests**. Test-side the type is named on **22 lines across 11 files in
        three crates** — 11 constructor calls, 10 `use` lines, and one doc
        comment at `crates/mc-sim/tests/support/chamber.rs:227` explaining
        file-name order, which a grep for `::new` does not find.
        Files: `mc-client/tests/{capture_geometry_ignores_an_unreadable_save,
        hud_held_block, launch_builds_only_the_world_it_needs,
        refusals_state_a_cause_once}.rs`,
        `mc-client/tests/support/{content,handed,mod}.rs`,
        `mc-sim/tests/support/{chamber,mod}.rs`,
        `mc-world/tests/{content_loading,definition_source_seam}.rs`.
      - `crates/mc-client/tests/support/content.rs:37`'s single
        `DECLARATION_EXTENSION` serves **blocks and HUD** and must split in two.
        Splitting it wrongly silently retargets the HUD fixtures:
        `hud_content_loading.rs` and `shipped_hud_outlines.rs` must both stay
        green, and `mc-world/tests/shipped_hud_outlines.rs:242` has a constant of
        the same name that stays `toml`.
      - Also verify rather than assume: `mc-world/tests/dependency_graph.rs`
        (both assertions must still hold — `mc-world` keeps `toml` for the HUD,
        `mc-core` still resolves without it),
        `mc-testkit/tests/workspace_layering.rs` (re-read the `INSPECTED`
        per-crate roster with the new edge in mind), and
        `mc-world/tests/no_hardcoded_block_names.rs` (scans `.rs` only, so moving
        content between file formats does not touch it — and it is an MVP-2 exit
        criterion).
      - FR-5.1-S1 and FR-5.1-S2 are the behavioural instruments for "the TOML
        reader is gone". The scenario audit **rejected** a structural scan for
        "no production Rust constructs a TOML definition source", because once
        the type is deleted nothing can name it and the scan can never fail —
        a test that reads as evidence and is not. Do not add one back.

- [x] **T20** The documentation-agreement guard's fixture moves —
      `crates/mc-client/tests/documented_refusals.rs`,
      `crates/mc-client/tests/support/content.rs`
      Scenarios: FR-6.1-S1, FR-6.1-S2, FR-6.1-S3
      Depends on: T19
      - `BLOCK_FILE` (line 93) `amber.toml` → `amber.luau`;
        `CARRYING_AN_UNRECOGNISED_FIELD` (line 105) becomes a Luau chunk. The
        verdicts, the three controls and both normalisations are **unchanged**.
      - The refusal being compared changes from the TOML parser's five-line caret
        diagnostic to one MyCraft writes itself — origin, block, field, cause, in
        the shape `DefinitionFault` already renders. **The HUD half of the guard
        stays TOML**, so the header note about a parser version bump reddening it
        must be **narrowed to the HUD half rather than deleted**.
      - FR-6.1-S3 is the guard's own vacuity control: quoting no refusal at all
        is reported as none found, never as agreement.

- [x] **T21** The walkthrough's declaration is a declaration that loads —
      `crates/mc-client/tests/` (the FR-6.2 guard), `docs/modding/README.md`
      Scenarios: FR-6.2-S1, FR-6.2-S2
      Depends on: T19
      - FR-6.2-S2 is the vacuity control and is the reason this is worth
        building: a guard that finds no Luau declaration in the walkthrough must
        **fail**, not report agreement over nothing.

- [x] **T22** The documentation deliverable, all three audiences, and every
      retired statement — `docs/modding/blocks-items.md`,
      `docs/modding/README.md`, `docs/modding/script-{writing,surface,limits,faults}.md`,
      `docs/INDEX.md`, `docs/technical/architecture.md`, `content/CLAUDE.md`
      Scenarios: **none of its own** — see "Mechanisms no scenario covers"
      Depends on: T20, T21
      - **Mod author** — `docs/modding/blocks-items.md` becomes the Luau
        contract: file layout, declaration shape, all six fields with type, bound
        and default, the namespaced-id rule, all-or-nothing loading, every
        refusal and how to read it, what the four bounds refuse, and **a complete
        worked example that runs**. A reference listing names without a working
        example is not documentation. `docs/modding/README.md`'s first-block
        walkthrough is rewritten end to end against `amber.luau`.
      - **Binding constraint, and nothing mechanical would catch it: the worked
        example must not declare a `texture` that differs from its `name`.** A
        declaration that does **loads and then does not draw** — the packer
        answers `UnresolvedTexture`, and a failed remesh batch is logged and
        dropped rather than failing the run, so an author sees their block simply
        not appear and its held indicator draw nothing. FR-6.2 checks that the
        example *loads*, never that it draws, so every guard this spec has would
        stay green over a walkthrough that does not work.
      - **What the page must say, in substance and without provenance.** Four
        facts, in the author's own terms: a block's texture is selected by its
        name today; a declaration whose `texture` differs from its `name` will
        load and then not draw; declare the two equal for now; independent
        texture keys are coming. Key Principle 3 refuses silence, and the field
        is still documented as independent because it *is* independent at the
        loader — which is what the author is being taught here.
      - **The page names no issue.** Nothing under `docs/` outside
        `docs/planning/` may name the issue tracker: a reader of the modding
        guide cannot open it, the reference means nothing to them, and it dangles
        permanently once the issue closes. Same rule as code and test names never
        carrying scenario IDs. **State the decision, drop the pointer.** The issue
        numbers stay here in `tasks.md`, which is working material archived with
        the spec. See T25 and the spec's "What a packed vertex actually carries".
      - **Player** — stated plainly in the modding entry point and the
        consolidated docs: nothing visible changes. Same blocks, same terrain,
        same block in hand, same saves. "Not applicable to that audience" is
        refused; this *is* the answer for the player, and it is the intended
        outcome rather than an omission.
      - **Engine reader** — `docs/technical/architecture.md`: the second
        `DefinitionSource` implementation, the `mc-world → mc-script` edge with
        D1's reasoning **and the rule that `mlua` must never reach `mc-world`**
        (the vendor blast radius is `crates/mc-script/src/luau/` plus
        `HostLimits`, and that is what a future change may not break), the raw
        key enumeration and what it is for, the four bounds with rationale
        including D8's correction, and D5's chunk-name/origin split.
      - **Retire all of it — leaving any one standing while shipping the loader
        is a defect.** The six "nothing can be authored in Luau" statements:
        `docs/INDEX.md:58`, `docs/modding/README.md:198–203`,
        `docs/modding/script-writing.md:11–33` (a whole section so headed),
        `docs/modding/script-surface.md:7`, `docs/modding/script-limits.md:7`,
        `docs/modding/script-faults.md:8`. Plus:
        `blocks-items.md`'s file-layout section, TOML field block, quoted
        refusal, base-game table naming `.toml`, and its "What MVP 2 changes"
        section, which becomes the present tense; `README.md`'s "A mod is a
        directory of data files", its `*.toml` routing row for blocks, and its
        held-block explanation naming `amber.toml`/`dirt.toml`; and
        `content/CLAUDE.md`'s "The Luau host is built; authoring in Luau is not",
        its whole "MVP 1 today vs. MVP 2" section, and its `mycraft.state(...)`
        note.
      - `docs/modding/hud.md` and `voxel-models.md` change only where they route
        to a page that moved — the HUD, voxel-model and material formats stay
        TOML and `.mcvox` (Out of Scope).
      - **The architecture record's client/scripting passage is corrected**, not
        deleted. It currently asserts that the client "learns what blocks exist
        by evaluating the same block declarations the server evaluates" and that
        "a client that cannot reach the host cannot draw the world". That is a
        non-sequitur — the client needs resolved definitions, never the
        evaluator — and it reads as an as-built record, which is what makes it
        worse than the guard that was deleted. A reader arriving with "should the
        client evaluate content?" must find the answer and not silence. *(Done
        ahead of the implementation by the 2026-08-16 amendment; verify it is
        still true of the tree when this task runs.)*

---

## Phase 6 — The client receives content resolved

**5 scenarios.** Done means: there is a resolved content value that carries what
the client draws and predicts with and none of the rules by which the world is
mutated; the client's view of content is built from that value and from nothing
else; and a layer assignment is stated by the resolver and honoured by the client
rather than re-derived from a sort.

**Red on arrival** because no resolved value exists and because phase 5
deliberately left `layers_of` deriving an index positionally. If either is
already done when this phase opens, phase 5 took more than it was given and this
phase must be reopened rather than accepted.

- [x] **T23** The resolved content value, and what it does not carry —
      `crates/mc-sim/`, `crates/mc-core/`
      Scenarios: FR-7.2-S1, FR-7.2-S2
      - Carries each block's name, its texture key, its solidity and the layer
        assignment. Carries **none** of `replaceable`, `breakable`,
        `breaks_into` — those are the rules by which the world is mutated and the
        server recomputes every one of them (Invariant 4).
      - **Assert by discrimination, never by absence.** A type that has no
        `breakable` field cannot fail a test about not having one; the test would
        not compile, which is a compile error standing in for an assertion. So:
        two roots differing **only** in the three mutation rules resolve to one
        and the same client content (S1), and two roots differing in a `texture`
        and a `solid` resolve to content that differs (S2). **Both directions are
        required** — S1 alone is satisfied by a resolver that returns a constant.
      - The registry still travels to the client in this arrangement, because the
        client binary is also the server and the simulation inside it holds one
        for the whole run. That is a residue of one binary hosting both halves
        and it is the composition-root spec's to remove. Say so where the value
        is defined; do not quietly assert otherwise.

- [x] **T24** The client's view, built from the value alone —
      `crates/mc-client/`, `crates/mc-render/`
      Scenarios: FR-7.3-S1
      Depends on: T23
      - **This is the single assertion that distinguishes a seam from a rename,
        and it only works if the fixture is written down.** The test constructs
        the resolved value as literal data: no registry built, no path opened, no
        scripting host constructed. That is a constraint on the code that builds
        the fixture and no assertion can enforce it — the test author holds it
        and a reviewer reads it, which is why it is stated here rather than
        assumed.
      - The failure this rules out is a resolved value that is a newtype over the
        registry, or a client view that reaches back through one. Both leave
        every other scenario green while nothing was cut.

- [x] **T25** The layer assignment is honoured, not derived —
      `crates/mc-client/src/startup.rs` (`layers_of`),
      `crates/mc-render/src/texture/`
      Scenarios: FR-7.4-S1, FR-7.4-S2, FR-7.4-S3
      Depends on: T23
      - **Why it matters, and it is not about networking.** A layer index rides
        inside every packed vertex. Deriving it as a key's position in the sorted
        key set means inserting one block renumbers every index after it and the
        whole world is textured wrong, silently, with no error anywhere — a live
        defect on hot reload today, in one process.
      - **FR-7.4-S1's fixture must make the assignment disagree with the
        positional order on purpose.** A test that compares two copies of the
        same sort cannot fail; that is the entire reason this scenario is worded
        around a disagreeing assignment.
      - FR-7.4-S2 is the other half: a block the assignment names no layer for is
        refused naming the block, rather than drawn from layer zero. Silently
        drawing layer zero is the failure shape the whole requirement exists to
        close.
      - **FR-7.4-S3 pins a live defect and is green on arrival.** Traced
        2026-08-16: the assignment is built from each block's declared `texture`
        (`startup.rs:336` → `TextureLayers::resolve(&registry.texture_keys())`)
        and an entry is selected by the block's **`name`**
        (`crates/mc-render/src/geometry/mod.rs:171` →
        `TextureKey::parse(quad.block.as_str())`). They agree only because all
        four shipped blocks declare the two identically. **A second consumer does
        the same and no existing note mentions it**:
        `mc_render::hud::held::held_swatch` (`hud/held.rs:109`) resolves the
        held-block indicator by the block's name too.
      - **Do not close the gap here.** It changes `build_section_geometry`'s
        binding signature and the roadmap constrains this spec to identical
        fields and no new surface; `product/roadmap.md:122` assigns it to
        PRO-902/PRO-914. S3 makes it a test rather than a comment in two
        CLAUDE.md files, and **it is the one scenario in this spec with a known
        expiry** — PRO-902 turns it red, which is the signal it exists to give.
        Record that in `test-map.md` so whoever meets the red knows it is a
        success and not a regression.
      - The trace also answered the question it was asked: **the layer index is
        the only registry-derived value in a packed vertex**
        (`geometry/vertex.rs:75–81` — three coordinates, a facing, the layer, and
        a scene section index written at assembly). So FR-7.4-S1 is not
        undermined by a second value riding beside it.
      - `layers_of` is the one place the key set is chosen and **both**
        preparation paths call it. That must stay true, or the geometry a player
        is handed and the geometry a golden is shot from are packed against
        layers that can differ.
      - Run the golden-frame suites again after this lands. A difference here is
        the assignment; phase 5 already ruled out the wiring.

---

## Scenarios that are green on arrival inside their own phase

Named here rather than buried, because **a phase whose scenarios were already
green did not do work, and a breakdown that lets these read as progress
overstates phases 3, 4 and 5.** All five are controls — the accept side of a
bound, or the positive-control side of a refusal — and `testing.md` §2 wants
exactly them. Their value is that they redden if the refusing side over-fires,
which is a real failure mode.

| Scenario | Phase / task | Green because |
|---|---|---|
| FR-2.4-S2 | 3 / T10 | a declaration carrying all six fields already registers after phase 2, which ignores fields it does not read |
| FR-4.3-S6 | 4 / T13 | 4 096 files already register against an unbounded loader — **unless the memory risk trips, which is what T13 exists to find out** |
| FR-4.3-S7 | 4 / T14 | a 256 KiB file already registers against an unbounded loader |
| FR-4.3-S8 | 4 / T15 | a 256-character name already registers against an unbounded loader |
| FR-1.3-S3 | 5 / T18 | `PreparationError::NothingToPlace` at `crates/mc-client/src/launch.rs:123` already implements it |
| FR-7.4-S3 | 6 / T25 | it **pins** behaviour the tree already has, and is not a control. See T25: it is the one scenario with a known expiry, and PRO-902 turning it red is the signal it exists to give |

> `architecture.md:465` calls these "all four"; there are five. The list it
> gives names all five.

## Weak instruments, named

Two scenarios became **structurally true** under D2's host-inside-the-loader
design, which removes the failure they describe rather than defending against
it. They are kept — they cost nothing and would catch a later drift back to
interior mutability — but they are not evidence phase 1 did work, and
`test-map.md` must say so:

- **FR-1.1-S5** (definitions asked for twice) — there is no shared mutable host
  to borrow twice, so the re-entrant borrow it guards against is unexpressible.
- **FR-3.1-S3** (a repaired root applies) — a fresh host per call makes the
  second apply indistinguishable from the first.

## Mechanisms no scenario covers, and where each gets its test

This project ships these labelled rather than silently.

| Mechanism | Why no scenario reaches it | Where it gets its test |
|---|---|---|
| `field_names` is **total over non-string key types** (D3, Assumption 1) | the spec has no scenario for `{ true, name = … }`; skipping such a key would make FR-2.4's promise partial | a `mc-script` unit test in T09, recorded in `test-map.md` under additional coverage |
| A top-level `print` is **actually recorded** in `printed()` | FR-4.2-S3 asserts an absence, and an implementation recording nothing satisfies it forever | the positive control in T06, and it is also what stops FR-4.3-S9 passing over a recorder that never records |
| The **retained-output bound accepts its own limit** | FR-4.3-S9 pins only the truncating side; the other three bounds have S6/S7/S8 and this one has nothing | `test-map.md` additional coverage in T16 — output just under the bound is retained whole and `dropped_print_lines()` is zero |
| `mlua` never reaches `mc-world` | structural, not behavioural; a scan for an absence goes green forever the day the thing it guarded is removed | Rust's extern-crate rules enforce it mechanically (the crate cannot name an `mlua` type without a manifest entry); stated in the engine-reader record in T22 as the thing a future change may not break |
| `mc-client`'s resolved dependency closure excludes the scripting host | **it cannot pass while one binary hosts both halves** — a binary's closure is the union of everything inside it, so the only arrangement in which it is green today is the one where the client sources content itself, which is the rule broken. A guard green exactly when the rule is broken is inverted, not weak | **nowhere in this spec.** It is the composition-root spec's exit criterion (spec, Out of Scope). What carries the property meanwhile is T18b's source scan, which is the weaker instrument and is recorded as such |
| A **second** content door — a new public registration call — bypassing T18b's needles | a text scan sees spellings, not reachability | nothing here closes it; it goes in the guard's own doc comment as a known residual, and the closure guard above is what would close it |
| The golden frames after the layer assignment changes (T25) | no scenario compares a rendered frame; FR-7.4-S1 asserts the index, not the picture | the golden-frame suites, run twice — once after T18's construction move and once after T25 — so a wiring defect and an assignment defect are distinguishable rather than merged |
| The **documentation deliverable** (T22) | the two mechanical guards cover the block refusal (FR-6.1) and the walkthrough's example (FR-6.2); nothing mechanically checks that a retired statement was retired or that the player paragraph exists | `/sdd-validate`'s reviewer, against the retirement list in T22 — which is why that list is enumerated file by file and line by line rather than described |

## Notes

*Deferred observations and follow-ups. Never delete task text; append status
markers only.*

- **Contradictions found in binding documents, resolved in favour of the tree.**
  (a) `architecture.md:441` says fourteen of phase 1's scenarios assert a
  refusal; counted individually it is seventeen against eight that register —
  the conclusion about the skeleton is unaffected and strengthened.
  (b) `architecture.md:465` says "all four are controls" over a list of five.
  (c) `architecture.md:401` says "13 test call sites"; 13 is the total
  `TomlFileDefinitionSource::new` count across the whole tree, of which 11 are in
  tests — test-side the type is named on 22 lines in 11 files. None of the three
  changes what any phase must do.
- **Scenario totals reconcile.** 5+4+4+7+4+4+3+3+1+3+4+5+9+3+3+2 = 64 in the
  spec before the amendment, plus FR-7's 3+2+1+3 = 9, is **73**;
  25+7+11+9+15+6 = 73 across the phases; every scenario appears in exactly one
  task and no scenario appears twice.
- **Deferred observation, recorded not fixed** (traced 2026-08-16): texture
  resolution does not consult the registry. The layer assignment is built from
  each block's declared `texture` and an entry is selected by the block's `name`,
  in **two** places — `crates/mc-render/src/geometry/mod.rs:171` and
  `crates/mc-render/src/hud/held.rs:109`. `crates/mc-render/CLAUDE.md` and
  `docs/technical/architecture.md:589–590` record the first; **neither mentions
  the second**, and a spec closing the gap must find both.
  `product/roadmap.md:122` owns it as PRO-902/PRO-914. Out of scope here: it
  changes a binding signature and the roadmap constrains this spec to identical
  fields and no new surface.
- **The amendment's Out of Scope entries are binding like every other one**
  (spec): moving the composition root and restoring the dependency-closure
  guard; transport and the wire format; the content-addressed cache; client-side
  script evaluation of any kind; and a content-set identity or hash. Each carries
  its reasoning in the spec so the later spec inherits the reason rather than
  rediscovering it. **No locality field is reserved on component declarations** —
  components are a later spec's and there is nothing here to attach one to.
- **DEFERRED (architecture, Risks):** giving the loader its own documented
  `HostLimits`. Revisited only if dropping handles earlier and forcing collection
  every N files both fail in T13, and FR-1.1-S1 must run at the shipped limits
  whatever the outcome.
- **Out of Scope is binding** (spec): `extends`, any new block field, hot reload,
  per-cell state and callbacks, worldgen in script, any `mycraft.*` binding, the
  HUD loader, a mod-id character set, a second content root. Recorded, not built.

### Phase 1 — recorded at completion

- **The registering skeleton reddens 25 of phase 1's 27 tests, not all of them.**
  Measured twice, by the test author and again by the implementation, against
  skeletons that differed only in how they invented a name. The two that cannot
  be reddened by any skeleton that walks a directory:
  - **FR-3.1-S2** (an empty `blocks/` is refused naming the root) — the refusal
    is `RegistryError::NoDefinitions`, which comes from `registry.rs` and which
    this spec must not touch. A loader yielding nothing satisfies it however it
    was written. Kept as a control: it reddens if a loader ever invents a
    definition for a directory holding none.
  - **FR-1.1-S5** (definitions asked for twice) — already recorded above as a
    weak instrument; a skeleton naming each block after its file satisfies it
    outright. The test author's skeleton named every block the same and so
    reddened it, which is a fact about the skeleton and not about the loader.
  Neither is evidence phase 1 did work, and the count is recorded here rather
  than left to be inferred from a green run.
- **`FaultKind::as_str` became public** in `mc-script`. D6 requires the cause to
  be composed from the fault's typed fields, and the words it composes are
  `FaultKind`'s own; the alternative was a second spelling of them in `mc-world`
  to drift from the first. Not listed in the architecture's Interfaces section,
  which named only `field_names` and `dropped_print_lines`.
- **Phase 1 hardcodes the three optional fields at their documented defaults**
  without reading them, which is the minimum a `BlockDefinition` can be built
  from. **Consequence for phase 3:** FR-2.2-S1 (none of the three declared →
  not replaceable, breakable, empty cell) is therefore *green on arrival* in its
  own phase, on the same terms as FR-2.4-S2. It is a control — it reddens if a
  default is later resolved differently — and it is not evidence phase 3 did
  work. It is not in the five the breakdown lists above; that makes six.
- **Mutation check, phase 1.** The loader's host was replaced by one with a
  100,000-tick budget. **All nine tests in `luau_declaration_guard.rs` stayed
  green**; FR-1.1-S1 alone reddened, reporting `call and loop budget exhausted`
  for a declaration that must register. That is the measured confirmation that
  the four all-unwanted FR-4.1 scenarios cannot detect a shrunken host and that
  FR-1.1-S1 is the only instrument that can. Reverted by hand;
  `git diff --exit-code` clean before the commit.
- **Passed forward to phase 2's test author:** FR-1.2-S1's fixture as the spec
  words it (`amber.luau` declares `example:zinc`, `zinc.luau` declares
  `example:amber`, created in that order) has creation order and file-name order
  *agreeing*, so a directory listing already hands `amber.luau` back first and
  the scenario is likely green against a loader that never sorts. Making the
  sort falsifiable needs a third file, or a pair whose creation order and
  file-name order disagree.

### Phase 2 — recorded at completion

- **Three of the seven were green on arrival, and phase 1 is not at fault.**
  FR-1.2-S3 (`unreadable(&declarations, …)` already names `<root>/blocks`),
  FR-1.2-S4 (see the corrected acceptance condition above — it discriminates
  between neither implementation), and FR-3.3-S3 (the suite's "names the file"
  reading is satisfied by the bare chunk name; only FR-3.3-S1 words the contrast
  that separates a chunk name from a path). **FR-3.3-S3 was deliberately not
  strengthened** to demand the whole path: that would re-prove FR-3.3-S1's
  property through the same `chunk_refusal` code path, which is a second witness
  that witnesses nothing.
- **The NTFS finding, which is owed to `docs/technical/testing.md` at
  completion.** The phase-1 hand-off above was right that FR-1.2-S1's fixture
  could not falsify, and understated why. Measured on two volumes: `read_dir` on
  NTFS returns entries in the filesystem's own **case-insensitive name order**,
  so creation order is not observable at all and the spec's lowercase pair
  arrives *already sorted*. A third file, `_cobalt.luau`, is the discriminator —
  `_` sorts after every letter under NTFS collation and before every lowercase
  letter under the byte ordering every plausible Rust sort uses. **The general
  rule, which outlives this spec:** an ordering fixture on this platform must be
  built to falsify *collation* order, not creation order, and a `_`-prefixed name
  is the cheapest thing that does it. This survived the scenario audit and died
  only on contact with the filesystem, which is the point — an audit cannot see
  shape any more than a count can.
- **Mutation check, phase 2. Six attempted, five bit, each taking down exactly
  one test.** Deleting the file-name sort (FR-1.2-S1), making
  `names_a_declaration` always true (FR-1.2-S2), skipping a non-file entry
  instead of refusing it (FR-1.2-S4), taking the origin from the `ScriptFault`
  again (FR-3.3-S1), and dropping the line from the composed cause (FR-3.3-S2).
  That no mutation took down a cluster is the evidence that the tests
  discriminate rather than overlap. **The sixth did not bite:** replacing
  `sort_by(file_name)` with `sort()` over the whole path. That is structural and
  not a gap — every declaration shares one parent directory, so the two orders
  are the same order, and no fixture can separate them while FR-1.2-S2 forbids
  recursion. The explicit `file_name()` comparison was kept anyway because it
  states the contract the spec words. Reverted by hand; `git diff --exit-code`
  clean after each.
- **Mutation 3 is why FR-1.2-S4 stays.** Green on arrival looked like it might be
  inert; it is not. It is the only test that reddens against a loader that
  quietly *skips* a non-file rather than refusing it, which is what makes D7 step
  2's refusal binding rather than decorative.
- **A green suite is no evidence about a lint, measured here.** The GREEN commit
  passed all 1,039 workspace tests and **failed the gate** on
  `clippy::excessive_nesting` — two `for` + `if` blocks — taking two further
  stages down with it, because `mc-world` would not compile and the `gpu-free`
  stage builds on it. One defect looked like three. The refactor is not
  cosmetic: the wrapped form `rustfmt` wanted for the chained predicate would
  have reintroduced the nesting clippy had just rejected, so the two constraints
  pull against each other and only extraction into `confirmed_file` and
  `worth_reading` satisfies both.
- **Deferred observation, outside this diff.** The first full gate run aborted
  `mc-client::terrain_probes flipping_the_frame_upside_down_turns_the_orientation_probe_red`
  with `0xc0000005` (access violation, os error 998) and cancelled 914 unrun
  tests; it passed on the immediate re-run and on the final run. A GPU-path
  crash rather than an assertion failure. `standards/global/testing.md` §8 wants
  a flake quarantined immediately rather than tolerated, and this is recorded
  rather than absorbed.
- **Passed forward to phase 4, whose bounds land in the same function.**
  `declaration_files` now runs in three steps — collect entries named as
  declarations, sort, then `confirmed_file` each one. **FR-4.3-S1's count check
  must land before the `confirmed_file` loop**, or 4,097 `fs::metadata` calls
  happen before the refusal that was supposed to pre-empt them. Separately,
  `faulted_cause` now composes kind + line + cause, so **FR-4.3-S2 has to refuse
  on file size before evaluation reaches the host at all** — otherwise that
  function will faithfully report the syntax error the scenario says must not be
  named.
- **Not tested, and named rather than left silent:** the sort runs before the
  file-type check so that a root holding *two* offending entries refuses the same
  one on every run. No scenario has two offenders, so nothing pins it. It was
  left untested rather than given a test I could not argue catches something
  real today.

### Phase 3 — recorded at completion

- **Three green on arrival, and the mutations prove all three are load-bearing.**
  That order matters: "three were green on arrival" reads as weakness and is not
  what is true. **Green on arrival and inert are different things, and a mutation
  is the only instrument that tells them apart** — flipping
  `BREAKABLE_BY_DEFAULT` reddened FR-2.2-S1, which is the measurement that it is
  a control rather than a scenario riding along. Phase 2 found the same for
  FR-1.2-S4 against a loader that skips rather than refuses. Two phases running.

  The three: FR-2.4-S2 and
  FR-2.2-S1 as recorded, plus **FR-4.2-S5** (an `__iter` that *invents* a field
  does not stop registration), whose diagnosis is FR-2.4-S2's exactly: with no
  unrecognised-field check there is nothing for a metatable to mislead. No
  honest strengthening reddens it — the scenario asks only that the block
  register from its own fields, which the phase-2 loader already does. It is a
  genuine control: an enumeration that believed `__iter` would recognise none of
  the invented field and refuse a perfectly stated declaration. **That makes
  seven green-on-arrival across the spec, not six.**
- **The rawness trap is `__len`, not `pairs`, and it was measured.** On this
  toolchain `mlua`'s `Table::pairs` is **already raw** against `__iter` and
  `__pairs` — a script-side `for k in t do` sees the metamethod's list while
  `Table::pairs` sees the table's own keys. `Table::len`, however, **honours
  `__len`**: for `setmetatable({'a','b','c'}, {__len = function() return 0 end})`
  the raw length is 3 and `len()` is 0. So FR-4.2-S4/S5 guard a route an
  implementation has to go out of its way to take, while sizing an enumeration
  with `len()` is the reachable defect — a host that did would report every
  declaration as carrying no fields at all, which refuses nothing and loses every
  typo. It has no scenario and got its own test.
- **FR-2.4-S3's own wording cannot fail today.** Measured: a fixed key set in a
  fresh script state comes back in the same order on every read, so "named in the
  same order on every run" is satisfied by passing the backend's order straight
  through. The test keeps that assertion and adds the one that can fail —
  *which* order — against a fixture where written order, state order and
  lexicographic order are three different orders.
- **Mutation check, phase 3. Seven attempted, seven bit.** Dropping the sort in
  `field_names` (reddened both the `mc-script` ordering test and FR-2.4-S3, and
  nothing else); the bound as `>=` rather than `>` (all four `mc-script` tests);
  skipping non-string keys (the totality test and the `__len` test); flipping
  `BREAKABLE_BY_DEFAULT` (FR-2.2-S1 and FR-2.3-S4); falling back to the default
  instead of refusing a wrong kind (FR-2.2-S3); removing the
  `only_recognised_fields` call (FR-2.4-S1, FR-2.4-S3, FR-4.2-S4); and dropping
  the residue instead of parsing it (FR-2.2-S2, FR-2.3-S3, FR-2.3-S4).
  **The default flip is the one worth recording**: it reddened FR-2.2-S1, the
  green-on-arrival control, which is the measurement that it is a real control
  rather than an inert scenario. Reverted by hand; `git diff --exit-code` clean.
- **The field-count bound is deliberately not written yet.** D7 step 7 spells
  `field_names(table, 64)`, but the `64` and its named refusal are **T15's**
  (FR-4.3-S5). Phase 3 passes `NonZeroUsize::MAX` under the constant
  `FIELD_NAMES_READ`, so what this phase takes from the enumeration is the list
  and not the limit, and FR-4.3-S5 can still be red in its own phase. What phase
  3 *does* establish is D7's **check order** — the enumeration runs before any
  field is read, so a misspelling is refused for being one rather than for the
  required field it was meant to be. **T15 replaces the constant and the
  `more_fields_than_read` cause**; the arm is already wired and unreachable at
  `MAX`.
- **`FieldFault.field` became `Option<String>`**, from `&'static str`. Two
  reasons, both forced: an unrecognised field name comes from script and cannot
  be `'static`, and a declaration holding more fields than the loader will read
  is wrong as a whole with no single key to send its author to.
  `DefinitionFault.field` was already optional, so this removes a mismatch rather
  than adding one.
- **A green suite is no evidence about a lint, again.** The same
  `clippy::excessive_nesting` that bit phase 2 bit `field_names`' walk. The fix
  was to collect the walk into a `Result<Vec<String>, _>` bounded by
  `take(allowed + 1)` rather than to push inside a loop — which is also what
  makes the bound bind before the allocation instead of after it, so the lint and
  the requirement wanted the same shape.
- **`luau_source.rs` hit the 500-line limit and was split by responsibility**
  into `luau_source.rs` (290) and `luau_declaration.rs` (341). Which files under
  a root are declarations, versus what a declaration must say — they change for
  different reasons, and the second is the Luau counterpart of `raw.rs`. Caught
  by the gate's size stage, not by a review.
- **Owed to `docs/technical/architecture.md` at phase 5**, and named here so it
  is not lost: `ScriptHost::field_names` and `FieldNames` are new **engine-facing**
  surface in `mc-script` — not content-facing, so no `api-reference.md` entry is
  owed — and the Documentation deliverable already asks for the raw
  key-enumeration capability and what it is for. The measured `__len` fact above
  belongs with it, because it is the reason the capability is written the way it
  is.

### Phase 4 — recorded at completion

- **T13's risk does not trip, and the measurement is here rather than left as
  "it passed".** 4 096 declarations through the real `LuauFileDefinitionSource`
  path: all 4 096 yielded and registered, no fault, ~330 ms to stream and ~325 ms
  to apply. Worst-case memory — the same 4 096 chunks through one host while
  **holding every table handle** and forcing **no** interim collection — peaked
  at **2,105,960 bytes** against the shipped **16 MiB** backstop, about 12.5%.
  Baseline 320,512; dropping the handles returns it to 441,240, so the handles
  are what holds the memory and the residue is small. **None of the three named
  fallbacks is needed** and FR-1.1-S1 stays at the shipped limits.
  - *Methodology, because it nearly went wrong:* the first attempt called
    `collected_memory_in_use()` every 512 chunks, which **forces a collection the
    real loader never performs** and flatters the figure. The number above is from
    a single read at the end. The decisive evidence is the production run
    completing at all: the backstop is enforced by the allocator on raw usage, so
    finishing is itself proof that raw peak never reached it.
  - The spike was a throwaway and was deleted; FR-4.3-S6's test is what carries
    the property forward.
- **Green on arrival: FR-4.3-S6, S7, S8** — exactly the three the table predicts,
  each an accept-side control.
- **Mutation check, phase 4. Six attempted, six bit.** Moving the count check
  after the file-type loop · checking file size after reading and evaluating ·
  the count bound as `>=` rather than `>` · measuring declared text in bytes
  rather than characters · resetting the print allowance on each drain · dropping
  the oldest retained line instead of stopping. Reverted by hand;
  `git diff --exit-code` clean.
  - **The first is the one worth recording.** It was caught by
    `a_root_over_the_count_is_refused_before_any_entry_is_checked_to_be_a_file`
    **alone** — a test the author added because no scenario words it. FR-4.3-S1
    pins count-beats-*evaluation*; nothing in the spec reaches the step before it,
    the `confirmed_file` loop phase 2 passed forward. Without that extra test the
    check order is pinned at one of its two boundaries and the other is free.
  - **The bytes-versus-characters mutation bit on the *accept* side**, which is
    the right place for it: `an_accented_id_of` builds a legal 256-character id
    with comfortably more bytes, so a byte-counting loader **refuses a
    declaration the documentation promises to accept**. A refusing-side fixture
    could not have caught it — 257 ASCII characters are 257 bytes and both
    measures agree.
- **D8 narrowed rather than the enumeration changed** (see the architecture, D8).
  D8 required every refusal to state the observed quantity and the bound; the
  field-count refusal structurally cannot, because D3 stops the enumeration one
  key past the allowance rather than walking a table it is refusing to allocate.
  How many keys the declaration held is a number the design never learns, and
  learning it would mean making the allocation the bound exists to prevent.
- **`retained_print_bytes` ships at `256 * 1024`, and nothing else constrained
  it.** Neither the spec nor D4 states a number — D4 fixes the three properties
  and leaves the value open. It is the per-entry memory cap on purpose: the
  host-side copy of what a chunk printed then cannot outgrow the allowance the
  chunk had to build it in. Written as its own literal rather than computed from
  that cap, because the two answer to different pressures and one following the
  other would bound nothing.
- **The sink is its own module.** `vm.rs` stood at 489 lines against a 500 limit,
  so `luau/print_sink.rs` holds what the host keeps of script output. `vm.rs` is
  now 499 — **one line under the limit**, which the next change to that file will
  have to deal with.
- **A gap the architecture leaves open, outside this phase's scenarios and worth
  a decision before phase 5 wires the loader into `mc-sim`.**
  `LuauFileDefinitionSource::printed()` returns a bare `Vec<String>`, so at the
  *loader's* boundary a truncated record still reads identically to a chunk that
  printed less — the exact absence-that-reads-as-agreement D4 closes at the host.
  The architecture routes `dropped_print_lines` no further than `ScriptHost`, and
  FR-4.3-S9 is worded about the host, so no loader surface was invented for it.
- **These fixtures are slow and could not be made fast.** Three 4 096-entry roots
  cost roughly 6 s each — writing four thousand files on NTFS and evaluating four
  thousand chunks. Past `testing.md` §3's one-second guidance, and no smaller
  fixture answers a question about how many files a directory may hold.
  `mc-world`'s whole suite runs in about 16 s.

### Owed to the durable record at completion — recorded during phase 5

Written down while the evidence is fresh rather than at the end, because each of
these is a method finding that outlives this spec and none of them is about
blocks.

- **A retirement's surface is measured by building `--all-targets`, not by
  grepping for the type.** `tasks.md` counted 22 lines across 11 files naming
  `TomlFileDefinitionSource`, and that count was exactly right. It was also
  incomplete about what mattered: **six further test files reached the deleted
  reader through `prepare_scene`/`prepare_launch` and named no type at all**, so
  no search for the type could find them — five the test author found by reading,
  one (`crates/mc-client/tests/overlay_over_content.rs`) that only the build
  found. **The compiler found them and nothing else could have.** They go red at
  the moment the construction moves, which is exactly the moment somebody is
  trying to tell a wiring defect from a content defect. A task file's count can
  be accurate about what it counted and silent about what governs.
- **"The implementation context never edits test files" has no
  mechanical-change exemption**, and the rule looks most like ceremony exactly
  when the edit is boring — which is when it is easiest to breach and hardest to
  notice. The concrete reason it is not ceremony here: splitting
  `crates/mc-client/tests/support/content.rs`'s single `DECLARATION_EXTENSION`
  into a block half and a HUD half can **silently retarget the HUD fixtures with
  nothing failing**. That is why the split is verified after the fact rather than
  assumed — the HUD fixtures must still be reading what they were reading.
- **Run a phase's verification instrument at the point where its answer is still
  attributable.** The golden-frame suites run after the construction moves and
  **before** the documentation work, not once at the end: a difference at that
  point is a wiring defect in the move, and a difference in phase 6 is the layer
  assignment. Run once at the end and the two are indistinguishable. This is
  verification-precedes-the-thing-it-verifies applied *inside* a phase, which is
  finer-grained than the task breakdown asked for. **A clean result there is
  evidence about the move, not an absence of news**, and is reported as such.
- **Two lint findings that pull against each other**, from phases 2 and 3: the
  form `rustfmt` wants for a long method chain reintroduces the nesting
  `clippy::excessive_nesting` has just rejected, so extraction into a named
  function is the only move that satisfies both. And in phase 3 the lint and the
  requirement wanted the *same* shape — bounding a walk with
  `take(allowed + 1)` before collecting is what makes a bound bind before the
  allocation rather than after it.
- **A bound that follows another bound bounds nothing.** `retained_print_bytes`
  is written as its own literal rather than derived from the per-entry memory cap
  it happens to equal, because the two answer to different pressures and a
  derived value moves silently the day the other moves for its own reasons.

### Deferred observation — the host's own truncation surface has the shape it closes

**Filed, not fixed. Out of scope here and deliberately so:** `ScriptHost` is
shipped, tested and merged surface from PRO-916, and this spec's business with it
is to bound a buffer rather than to reshape its accessors.

The observation. `ScriptHost` exposes `printed()` and `dropped_print_lines()` as
**two accessors**. That makes the truncation distinction *available*. It does not
make it *unmissable*: a caller may read the record and never ask whether it is
whole, and a truncated record then reads exactly like a chunk that printed less —
which is the very failure `dropped_print_lines` was added to close.

**A truncation flag exposed alongside the thing it qualifies, rather than inside
it, can be read without it.** That is the whole argument, and it is the same
argument that decided the loader's boundary the other way: at
`LuauFileDefinitionSource` the two travel in one record, so nothing can consult
the lines without meeting the count.

This is worth meeting rather than re-deriving, because it is an instance of this
project's most-recorded failure family — an absence indistinguishable from a
presence — and every previous instance was likewise *a value somebody could have
consulted and did not*: an empty cause reading as a cause never populated, an
absent reviewer reading as a clean one, a scan that can no longer look reading as
a scan that found nothing.

Whoever next revisits that host surface should weigh the change against the cost
of moving shipped API, rather than start from scratch on whether there is
anything wrong with two accessors.

### The explicit-staging rule was load-bearing here, and the circumstance is the finding

`git-workflow.md` bans `git add -A` and `git add .` with a story attached. During
phase 5 that ban stopped being theoretical.

**The circumstance is the instructive part.** The implementation ran
`cargo fmt --check` and found a formatting slip in its *own* previously committed
diff — misordered imports — **while the test author had eleven files uncommitted
in the same working tree**, mid-way through the T19 retirement. Fixing it meant
committing. Explicit paths are the only reason those commits carried the two
source files they were about and none of the author's in-flight test work.

Two things worth keeping from it:

- **The accident arrived during ordinary work, not during anything anybody would
  have called risky.** A formatting fix is the least dangerous commit there is,
  which is exactly why a sweep would have gone unnoticed — and the separation it
  would have destroyed is the TDD sequence's, the same one phase 1's notes record
  a sweep destroying before.
- **Checking format before every commit rather than only at the gate is what
  surfaced it at all.** The gate would have caught the slip later, in a run where
  it would have looked like somebody else's problem — and by then the sweep, had
  there been one, would already have happened.

A rule with a live instance behind it survives a re-read that a rule with a
hypothetical behind it does not.

### The tell is that you are about to write a literal

Four times in this spec somebody was about to write down a value whose
correctness depended on what a *different* component produces, and going to look
first would have cost one command. Three times nobody looked.

- A **count** copied into the task file — 22 lines naming the retired type. The
  number was right and measured the wrong surface; the build found six more files
  a grep could not.
- A **safety claim** written into a phase note — that a fixture could not be
  mis-retargeted. True, and true for a reason nobody had checked, which is why it
  can be removed silently.
- A **refusal string** about to be pasted onto a modding page. Checking what the
  guard's run actually produces found that the dependency runs the other way:
  quoting a refusal the run does not print **fails** the guard rather than
  covering it.
- An **issue identifier** somebody was told to cite without being handed it.
  Refused rather than guessed, because a wrong tracker reference in an archived
  record is worse than a missing one — a missing one is visibly missing.

**"Verify your premises" is not actionable**, because every premise looks like a
fact from the inside and there is no moment at which the advice fires. The
trigger that can actually be noticed is narrower:

> **You are about to write a literal.** A literal is a value you are asserting
> some *other* component produces — a count, a rendered string, an identifier, a
> claim about behaviour elsewhere. That is the one situation where going to look
> costs a single command and not looking costs a red guard, a wrong pointer, or a
> page documenting a message nothing writes.

The corollary is what makes it cheap: **the check is against the producer, not
against your reasoning.** Run the thing, read what it emitted, paste that. Every
one of the four above was recoverable in one command from the right source.

### An accidental safety property has no guard of its own

Found at phase 5 and sharpened at validation. Recorded with the other durable
findings because it is a rule about how properties are kept, not about blocks.

`crates/mc-client/tests/support/content.rs`'s fixture turns out to be
**self-falsifying**: pointing the HUD helper at the block extension fails loudly
rather than emptying nothing and passing, so the silent mis-retarget the task
breakdown warned about cannot happen there. That is a stronger answer than the
by-hand verification it was given.

**But nobody designed it.** The refusal it rests on exists for an unrelated
reason — the module's rule that removing a declaration which was never there is a
failure rather than a no-op — and the safety falls out of that rule as a side
effect.

**So the rule that changes it is free to change.** A later helper that took an
extension and tolerated finding nothing would remove the property **with nothing
going red**, because no test asserts the property and the rule it depends on was
never written down as load-bearing for anything else.

The general form: **an accidental safety property has no guard of its own.** It
is not a weaker version of a designed one — it is a property with no owner, which
will be traded away the first time somebody has a good reason to change the rule
underneath it. **The only defence available is to write the dependency down where
the rule lives**, so whoever changes the rule meets the consequence rather than
discovering it. Asserting the property directly is usually the better move where
it can be done; where it cannot, the note is what there is.

### A guard that names a specific dependency silently narrows as the set it guards grows

Found at phase 5 while *verifying* `crates/mc-world/tests/dependency_graph.rs`
rather than while changing it, which is the part worth keeping: the task file
asked only that both its assertions still hold, and they did.

The guard asserted that the registry-contract crate reaches no `toml`, on the
reasoning that a declaration format's parser belongs to the loader and nowhere
else. **The day block declarations became Luau chunks, `toml` stopped being how
block declarations arrive** — so the guard went on passing while the property it
exists to protect had gone unguarded for the new way in. Its companion assertion
was worse off: naming only `toml`, it could no longer tell a loader that reads
block declarations from one that has stopped reading them at all.

This is the structural-invariant failure this project has already written down —
*a test asserting only an absence goes green forever the day the thing it guarded
against is quietly removed* — in its sharper form: **the thing was not removed,
it was replaced by something the guard cannot see.** An absence assertion cannot
notice that the world grew a second way to violate it.

Two repairs, and the second is the one that lasts:

- Name **both** ways in **both** assertions, so the pair control each other: a
  mistyped needle now fails the loader half rather than passing the contract half
  in silence.
- **Name the constant for what it actually guards.** It is the *HUD format's*
  parser now, not "the declaration format's". A constant that keeps an outdated
  name is how a guard drifts from its purpose **without anyone editing a line of
  it** — and it is what would let the next reader conclude the guard still means
  what its name says.

**The general rule:** a guard that enumerates specific dependencies silently
narrows whenever the set of things it guards against grows, and nothing about it
goes red to say so. Whenever a format, a backend or a vendor is *added* beside an
existing one rather than replacing it, every absence guard naming the old one is
already out of date. Re-read them at that moment, because no later moment will
prompt you to.


### Phase 5 — recorded at completion

- **FR-1.3-S4 is the result this whole spec turns on, and it should not sit in a
  list of fifteen.** A world saved against the TOML declarations loads against
  the Luau ones reporting **no block as missing, changed or retextured**. That is
  direct evidence for the exit criterion — the world renders identically and a
  player's save survives the swap — and it is the only scenario comparing a whole
  resolved definition against an oracle computed before this spec existed.
  `persistence/format.rs` excluding `origin` from both hashes is what makes it
  possible. **Measured under mutation:** flipping `water.luau`'s `solid` to
  `true` reddens it alone.
- **The golden-frame suites are clean after the seam move.** 10/10 across
  `terrain_goldens`, `hud_goldens`, `golden_mismatch`, `launch_and_capture_agree`
  and `replay_oracle`, run **after the construction moved and before the
  documentation work**. A difference at that point would have been a wiring
  defect in the move; a difference in phase 6 is the layer assignment; run once
  at the end the two are indistinguishable. `golden_mismatch` passing is what
  says the comparison can still fail. **A clean result here is evidence about the
  move, not an absence of news.**
- **The two red scenarios were red for a reason that was established rather than
  assumed, and the controls are what established it.** FR-6.1-S1 and FR-6.2-S1
  stayed red until T22 rewrote the pages — and `documented_refusals.rs`'s drift
  control and verbatim control both passed against a real Luau refusal
  throughout. That is what distinguishes *the pages are not written yet* from
  *the guard is broken*; without it, two red scenarios at that point would have
  been ambiguous.
- **The retirement surface was larger than a grep could measure**, and the
  general lesson is recorded above under "Owed to the durable record". Counted:
  22 lines across 11 files naming the type — exact. Uncounted: **six further test
  files reaching the deleted reader through `prepare_scene`/`prepare_launch` and
  naming no type at all**, five found by reading and one
  (`crates/mc-client/tests/overlay_over_content.rs`) only by the build.
- **The `DECLARATION_EXTENSION` split turned out to be self-falsifying**, which
  is better than the verification it was given. `empty()` errors when a directory
  declared nothing to begin with, so a `declaring_no_hud` pointed at `luau` fails
  loudly rather than emptying nothing and passing. The silent mis-retarget the
  breakdown warned about is structurally impossible in that fixture.
- **`dependency_graph.rs` was green for a reason that had stopped being true.**
  Recorded separately above, because it generalises: a guard naming one
  dependency narrows silently as the set it guards against grows.
- **Mutation check, phase 5. Three attempted, three bit, each reddening exactly
  one test.** `Printed::of` always answering `Whole` (the loader-boundary
  truncation test); `water.luau` declaring `solid = true` (FR-1.3-S4); and a
  local *variable* named `content_root` in `mc-client` (the seam guard). The last
  is worth recording: the needle is a **bare spelling**, so it catches a variable
  that is not a door at all. That is the guard working as specified rather than a
  defect — a needle with a carve-out is where the next breach lives — and it is
  the cost of the property that the scan wants no exemption.
- **`startup::content_root` was deleted rather than renamed in place**, and the
  simulation's resolver is `shipped_directory`. A `mc-sim` function named
  `content_root` would have put the needle straight back into
  `crates/mc-client/src/main.rs` at the call site. The test author raised it as a
  binding interface decision before any implementation was written, which is
  where it was cheapest to settle.
- **`PreparationError` kept its shape.** A third variant wrapping the seam's
  `ContentError` was written and then undone: it would have been a second name
  for a failure that already had one, and two names for one failure is how a
  caller ends up matching only the arm it happens to know about. An
  `impl From<ContentError>` flattens into the variants callers already match.
- **The loader-boundary truncation gap was ruled into scope and closed here.**
  `dropped_print_lines` is a number the host already computes, so carrying it
  forward is plumbing rather than surface, and the no-new-surface constraint
  governs what a *block author* writes. `Printed` is **one value rather than two
  accessors**: two accessors make the distinction available, a record makes it
  unmissable, and reproducing the available-but-not-taken shape in the act of
  closing it would have been the sharpest version of the mistake. The test
  author's `NonZeroU64` goes further than the brief asked and makes a truncated
  record that dropped nothing unrepresentable. **The host's own two-accessor
  shape is filed as a deferred observation** rather than chased.

  **An enum rather than a struct, and the distinction is the whole point.** A
  struct of `kept` and `dropped` still lets a caller write `.kept` and never ask
  — the available-but-not-taken shape one layer further in. A **variant cannot be
  read without naming which of the two answers you are holding.** That is the
  difference between hard to ignore and impossible to ignore. It is the same
  shape as `FieldNames::{Enumerated, MoreThanAllowed}` from D3 — the same
  decision, in the same spec, for the same reason — and reaching for a shape the
  spec already uses when the problem recurs is worth more than either instance:
  a reader who has understood one has understood both.

  **The free result, and it is the concrete argument for a type-level fix over a
  flag.** Three existing readings in `luau_declaration_guard.rs` assert that
  *nothing* was printed. Changing the return type made each of them compare
  against `Printed::Whole(…)`, so **every one now also rejects a record the host
  had stopped keeping** — a defect no test in that suite could previously reach.
  One change closed a hole and strengthened three unrelated assertions.
- **`content_loading.rs` was deleted rather than retargeted**, its nineteen
  properties checked one by one against the surviving suites first. The one
  orphaned *statement* — that a texture key needs no file on disk — went into
  `luau_common`'s module doc, where the fixtures make it structural, rather than
  into a replacement test that would re-prove it through the same code path.


### A bare-spelling needle cannot tell a door from a variable, and that is the design

**Recorded so the next person meets the argument instead of making the change.**

The client-source guard's needles are bare spellings — `registry.apply(`,
`HudLayout::load`, `BlockRegistry::new`, `content_root`. Measured at phase 5: a
local **variable** named `content_root` in `crates/mc-client/src/startup.rs`
fails the guard, and it is not a door at all.

**That false positive is the cost of a scan that admits no exemption, and the
trade is deliberate.** The alternative is a needle with a carve-out — matching
only some spellings, or exempting a file, or a path. A carve-out is where the
next breach lives: it is the thing somebody widens by one line when their case
looks equally harmless, and nothing goes red when they do. `tasks.md` already
says the scan **wants no exemption at all**, and that if one seems necessary it
is a signal a door was left behind rather than a licence to write it.

So whoever meets this: **it is working, and the fix is to rename your variable,
not to narrow the needle.** The scan already strips doc comments, so prose about
a door is not a use of it, and sibling `*_test.rs` files are skipped — those two
carve-outs exist and are the only two, because each removes a category that
cannot be a door rather than a case that happens to be inconvenient.

The residual this does **not** close is recorded in the guard's own doc comment:
somebody adding a *second* door — a new public registration call — bypasses any
text scan, and the instrument that would catch it is the dependency-closure
guard, which cannot pass while one binary hosts both halves.


### Phase 6 — recorded at completion

- **The count, stated so it cannot read as a discrepancy later.** Phases 1-5
  closed **67**. Phase 6 owns **six**: five opened red and one — FR-7.4-S3 —
  opened green by design. 67 + 6 = **73**, every scenario in the spec mapped and
  closed.
- **The golden-frame suites are clean after the layer assignment changed.**
  10/10 again, run at the second attributable moment: phase 5's clean run ruled
  out the wiring, so a difference here would have been the assignment and nothing
  else. Both runs together are what make either one mean anything.
- **The sharpest measurement of the phase, and it justifies a test no scenario
  asked for.** Mutation C put `TextureLayers::resolve(&registry.texture_keys())`
  back into `prepare_scene` — the production path deriving its own assignment
  again, which is *precisely* the failure FR-7.4 exists to close. **Every
  behavioural test passed**: both FR-7.2 readings, FR-7.3's, all four of
  FR-7.4's, and **both golden suites**. Only the source scan reddened.

  The reason is worth writing down because it is not obvious: a client that
  honours and a client that derives answer **identically** for every content root
  that can be built today, because the assignment the simulation states *is* the
  order a positional derivation produces. The goldens cannot see it either —
  permuting the assignment permutes the array texture's fill in the same breath,
  so the picture is unchanged.

  **So the wiring is not behaviourally falsifiable in this phase, and the scan is
  the only instrument that can see it.** Without it an implementer could add
  `ContentView`, green all six scenarios, and leave both preparation paths
  deriving — the seam becoming a rename, which is the exact risk the architecture
  names. **It becomes falsifiable the moment an assignment is appended rather
  than renumbered**, which is hot reload's, and the scan can be retired then.
  Fourth phase running in which a test no scenario asked for caught something
  real.
- **Mutation check, phase 6. Three attempted, three bit.** A `replaceable` field
  carried across the seam (FR-7.2-S1 alone — FR-7.2-S2 stayed green, which is the
  measured proof that neither direction is sufficient by itself); `ContentView`
  deriving its layers from a sorted key set (FR-7.3-S1 and two of FR-7.4's); and
  the production derivation restored (the scan alone, as above). Reverted by
  hand; `git diff --exit-code` clean.
- **FR-7.4-S3 is green and undisturbed**, along with its additional-coverage twin
  through `held_swatch`. Both pin the name-for-texture substitution as a test
  rather than a comment in two `CLAUDE.md` files. **They expire together when the
  per-face texture work lands, and that red is the success signal rather than a
  regression** — recorded in `test-map.md` so whoever meets it knows.
- **`ResolvedContent` lives in `mc-core`, not `mc-sim`.** T23 named both without
  choosing. `mc-render` has to accept a stated assignment and may not name the
  simulation, so `mc-sim` would have made the renderer reach for a crate the
  dependency rules forbid it. It is a content primitive with no I/O, which is
  what `mc-core` is for.
- **`SectionGeometry::layer_at` is new surface and the tests' doing.** Nothing
  outside `mc-render` could read a packed corner's layer: `SceneGeometry` hands
  out bytes and `PackedVertex` keeps its only constructor private so `unpack`
  stays total. The property had to be asserted where the index lands rather than
  one step before it — asking the layer table what it holds would leave the
  packer free to derive one of its own — and decoding the bit layout at the
  caller would have been a second copy of the packing decision, free to drift.
- **`ContentView::is_solid` has no production caller**, and that is stated in its
  own doc comment rather than left for a reviewer to notice. The mesher still
  culls against the `BlockRegistry` that is still travelling to the client, which
  is the residue of one binary hosting both halves the spec already states. It is
  what the mesher reads once the registry stops travelling.
- **`layers_of` is gone.** The property its doc comment recorded — both
  preparation paths choose the same layers, so the geometry a player is handed
  and the geometry a golden is shot from cannot differ — is now true by
  construction rather than by both calling one function: both build a
  `ContentView` from the same stated value.


### Two addenda, recorded after phase 6 closed

**The self-falsifying fixture is a consequence of a rule written for something
else, and that is why it needs saying out loud.** The phase 5 record notes that
`empty()` refuses a directory that declared nothing to begin with, so a HUD
fixture pointed at the block extension fails loudly rather than emptying nothing
and passing. **Nobody designed that property.** The refusal exists for an
unrelated reason — the fixture module's rule that removing a declaration which
was never there is a failure rather than a no-op — and the self-falsifying
behaviour falls out of it.

Which means **a later helper that took an extension and tolerated finding nothing
would quietly remove the property with nothing going red.** The comment now sits
on the two-constant doc block in
`crates/mc-client/tests/support/content.rs`, where somebody about to edit a
spelling is already looking, rather than on the function, and it says so.

Measured rather than reasoned: the pairing was mis-set by hand —
`declaring_no_hud` pointed at the block extension — and
`a_content_root_declaring_no_hud_is_read_without_a_word_about_the_hud` reddened
with the "root that never declared anything" refusal, by name. Reverted by hand.

*(The address in the request that produced this was wrong: `empty()` is in
`mc-client/tests/support/content.rs`, not `luau_common/mod.rs`. The ownership
reasoning held either way — both are test files — and the author put the comment
where the function is.)*

**Handed forward to the composition-root spec.**
`crates/mc-client/tests/one_content_path_to_the_registry.rs` and
`shipped_blocks_are_declared_in_luau.rs` read `PreparedScene::registry` and
`PreparedLaunch::registry` directly. That is deliberate — the residue is stated
rather than hidden — but it means those two files name the registry that stops
travelling to the client when the composition root moves. **They are not phase
6's to change and were not changed.** Their assertions are about *which blocks
registered* rather than about who holds them, so they should survive the
narrowing intact; whoever moves the root should expect to retarget the reading
and not the property.


### Deferred observation — most refusals a mod author can trip are quoted nowhere (PRO-946)

Found at validation, filed as **PRO-946** with the enumeration below intact.
Recorded here as well because this spec is where the gap was measured and where
the reasoning for deferring it lives.

**What exists.** `crates/mc-client/tests/documented_refusals.rs` compares every
refusal a `docs/modding/` page quotes against what a real run prints, line for
line. Its run produces **two** refusals — a block declaration carrying an
unrecognised field, and a HUD declaration stating an extent of zero — and
`blocks-items.md` quotes **one**.

**What a mod author can actually trip**, none of it verbatim-guarded: a missing
required field · a field of the wrong kind · a malformed id · a duplicate name ·
a root declaring no blocks · a chunk that will not compile · a chunk that raises ·
a chunk exhausting the call-and-loop budget · one exceeding the memory cap · one
touching a denied global · one assigning a global · a chunk returning a non-table ·
an entry named `.luau` that is not a file · and **all four content-root bounds**.

**Why prose is not enough**, which is the reason this is filed rather than
closed: `blocks-items.md` describes each refusal's shape and what it names, and
prose drifts. Nothing reddens when it does. That is the whole argument for the
verbatim guard existing at all, and it currently covers two of roughly fifteen.

**The dependency runs page-follows-run, not run-follows-page**, and this is the
part worth knowing before anyone starts: the guard asserts *every refusal a page
quotes is one the run prints*. So **quoting a refusal the run does not produce
fails the guard rather than covering it.** Each new quoted refusal needs a
matching fixture in `printed_refusals()` first. That is why this cannot be closed
by editing a page.

**A design note for whoever picks it up, from the test author, and it is the
reason this is not just "add twelve more fixtures".** The guard's run is not
free: each fixture is a whole content root copied and a whole preparation
refused, and `printed_refusals()` runs **once per test** in that binary. Two to
three is nothing. Fourteen would make four tests perform fourteen preparations
each. **Build the set once and share it** rather than adding twelve more
`shipped_copy()` calls to the same function — cheaper to know now than to
discover at the fourteenth fixture.

**Why it was deferred rather than done here.** Validation found one instance —
the declared-text refusal, whose rendering was broken and which no test could see
because `luau_declaration_bounds.rs` asserts the quantities are *mentioned* and a
substring check cannot see rendering. That one is closed: a fixture was added and
the refusal is quoted. Expanding the run twelve-fold, inside a validation fix, at
the end of the spec that found the defect, is the change that introduces new
defects when there is least appetite to go looking for them. **The quoting and
the prose want writing together, not one retrofitted onto the other.**
