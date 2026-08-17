# Architecture: Blocks are defined in Luau — swap the loader, not the registry

Spec: `spec.md` (SPEC-016, rigor `high`, **73 scenarios** — 63 counted in the
tree at the start of this stage, plus FR-4.3-S9, which this stage found and the
spec now carries (see D4), plus FR-7's nine, added by the 2026-08-16 amendment
(see D9)). Requirements record: `requirements.md`. Binding reasoning for FR-7:
`docs/planning/client-server-split.md`.

## Drivers

### Quality attributes that matter here, with their evidence

| Attribute | Why it matters for *this* feature | Evidence in the tree |
|---|---|---|
| **Containment of untrusted content** | A declaration file is the first production path that runs mod-authored code. Invariant 3 applies before any callback exists. | 17 of 63 scenarios are FR-4.x. `crates/mc-script/CLAUDE.md` threat model: "Mod code is untrusted." |
| **Determinism of refusal text** | `crates/mc-client/tests/documented_refusals.rs` compares a quoted refusal against a real run **line for line** (`normalised`, `judged`). Any run-to-run variation makes it intermittently red, which this project's standards rank below plainly red. | FR-2.4-S3; `documented_refusals.rs:420` |
| **Save compatibility** | `persistence/format.rs:307,319` fold a definition into `DeclaredBehaviour`/`DeclaredAppearance` and deliberately exclude `origin`. A save written before the swap must load after it, and it is the only fixed oracle over a whole resolved definition. | FR-1.3-S4 |
| **No panic on a content path** | `mc-script` denies `unwrap`/`expect`/`panic!` at its crate root; its invariant 4 forbids a panic anywhere content reaches. The obvious interior-mutability design panics on a re-entrant borrow. | FR-1.1-S5, FR-3.1-S3 |
| **Evolvability** | PRO-918 (hot reload), PRO-904, PRO-902/914 and PRO-919 all extend *this* loader. Every later block field is born here. | `product/roadmap.md:129–134` |
| **Documentation truth** | Key Principle 3 plus a mechanical guard. Six pages currently promise nothing can be authored in Luau. | FR-6.1, FR-6.2 |

### Constraints

- **`product/roadmap.md:133`** — "identical fields, no new surface, exit criterion
  *the world renders identically*". Binding, and it is what puts `extends` out of
  scope (the lead has ruled; not re-opened here).
- **`ScriptHost` is `!Send`** and pinned to its thread by design (`host.rs:59–71`,
  `mc-script/CLAUDE.md` §Threading). `DefinitionSource::definitions` takes `&self`;
  `ScriptHost::evaluate` takes `&mut self`.
- **`mc-script` depends on `mlua` alone** and knows nothing of the workspace. Its
  `SubjectName`/`ComponentName` are deliberately opaque — it may not learn what a
  block is.
- **Nothing in the workspace depends on `mc-script` today.** Verified against every
  member manifest. This spec creates the first such edge.
- **`mc-world` keeps `toml`** for `hud_toml_source.rs`, so
  `crates/mc-world/tests/dependency_graph.rs` holds unchanged in both directions.
- `mc-client/src/launch.rs:143` prepares a launch on a `std::thread::spawn`. The
  definition source is therefore constructed **inside** that thread; whatever it
  holds must not need to cross the join.

### Volatile, and expensive to reverse

- **Volatile:** the block field set (three roadmap issues add to it next), the
  refusal wording (documentation quotes it verbatim), the three new bounds.
- **Expensive to reverse:** the `mc-world → mc-script` edge; the shape of the new
  host capability, which is a public method other crates will build on; the
  chunk-name/origin split that makes FR-3.3-S1 falsifiable.

## Boundaries

| External dependency | Volatility (Vendor/Regulatory/Stability/Substitutability) | Port | Adapter location | Direct-use justification |
|---|---|---|---|---|
| **Luau VM via `mlua` 0.12** (vendored C++), and every value content produces | V: low (OSS) · R: none · **S: high** (pre-1.0 crate, vendored VM) · **Sub: high cost** (the VM *is* the product decision). Also a **source of nondeterminism** — it runs untrusted code — which alone mandates a port under Section 3. | `ScriptHost`, `ScriptValue`, `ScriptTable`, `ScriptFault`, `FaultKind`, `HostLimits` (existing, `mc-script`) | `crates/mc-script/src/luau/` (existing) | — |
| **Filesystem** (`std::fs`, directory listing, file reads) | std library, in-process | none | — | Section 3 exclusion: standard library. Consistent with `toml_source.rs` today. |
| **TOML parser** (`toml`), HUD declarations only | unchanged by this spec | `HudSource` | `crates/mc-world/src/content/hud_toml_source.rs` (unchanged) | — |

**The port for this spec already exists and the obligation is to keep it intact.**
`mc-world` gains `mc-script` in `[dependencies]` and **must never gain `mlua`**.
Rust's extern-crate rules make that structural rather than a rule someone
remembers: `mc-world` cannot name an `mlua` type without a manifest entry.
Litmus test — "if the Luau VM disappeared tomorrow, how many files change?"
Answer: `crates/mc-script/src/luau/` plus `HostLimits`. `mc-world` names only
`ScriptHost`, `ScriptValue`, `ScriptTable`, `ScriptFault`, `FaultKind`,
`HostLimits`. That must remain true and is what the engine-reader record below
must state as the thing a future change may not break.

## Decisions

### D1 — The new source lives in `crates/mc-world/src/content/`. **BINDING**

**Options.**

1. `crates/mc-world/src/content/luau_source.rs`; `mc-world → mc-script`.
2. Inside `mc-script`; `mc-script → mc-core`.
3. A new crate (`mc-content`) depending on `mc-core` + `mc-script`, with
   `mc-client` and `mc-world`'s tests depending on it.

**Against the drivers.** Option 2 is genuinely refused, and the spec's reasoning
survives testing: `mc-script`'s whole identity is opaque `SubjectName`/
`ComponentName` and a host that never interprets a namespace. Teaching it what a
block is inverts that and puts `mc-core` under the crate that must stay
substitutable if the VM changes.

Option 3 is the real alternative and deserves its cost stated, because the
consequence of option 1 is that **`mc-sim`, `mc-render` and `mc-client` all newly
resolve a vendored C++ VM** — nothing in the workspace resolves `mc-script`
today. `mc-render` in particular has no business reaching a scripting host.
Against that: in a workspace build Luau is compiled once regardless (`mc-script`
is a member), so the measurable cost is confined to `cargo build -p mc-render`
alone; `crates/mc-testkit/tests/workspace_layering.rs` polices reach onto
`tools/` and is unaffected; and `mc-sim` acquires a direct `mc-script` edge two
issues later (PRO-919) in any case. A new crate to protect one crate from an edge
that costs it nothing measurable today is isolation for its own sake, and it
would move `content_loading.rs`, `definition_source_seam.rs` and the module that
already holds `TomlFileHudSource` apart from each other.

**Recommendation: option 1.** Content loading already lives in
`mc-world/src/content/`, beside the HUD source that stays there.

**Strongest argument against it:** the layering is genuinely worse, and the day
someone wants `mc-world` reusable headlessly — or wants a second consumer of
script-loaded content that is not the world — the edge is in the wrong place and
the extraction touches every consumer's manifest. **Revisit when** a third
`DefinitionSource` implementation appears, or when a crate other than `mc-world`
needs to read content through script. The extraction is mechanical at that point;
nothing here forecloses it.

### D2 — The source owns a host for the duration of one `definitions()` call, and collects eagerly. **BINDING**

The problem the spec names: `definitions(&self)` against `evaluate(&mut self)`.

**Options.** (a) `RefCell<ScriptHost>` on the source with a lazy stream that
borrows per item; (b) `RefCell<ScriptHost>` with eager collection inside
`definitions()`; (c) a host constructed inside `definitions()`, used to evaluate
every file, and dropped before the call returns — the stream being
`Box::new(collected.into_iter())`.

**Against the drivers.** (a) is the shape the spec warns about: two overlapping
streams, or a stream outliving its call, is a re-entrant borrow, which panics —
forbidden on a content path. (b) removes the overlap but keeps a `!Send` host
inside a type constructed on a spawned thread and held across meshing.

**Recommendation: (c).** It makes the panic unexpressible rather than avoided:
there is no shared mutable host to borrow twice. `LuauFileDefinitionSource` holds
a `PathBuf` and a record of what was printed, so it is `Send` and imposes nothing
on `spawn_preparation`. FR-1.1-S5 and FR-3.1-S3 become true by construction —
which is the point, since a scenario that can only fail through a panic is a poor
instrument. The port's stream contract is unharmed: a `Vec<Result<…>>` yielded in
order still expresses "failed part-way through", exactly as
`InMemoryDefinitionSource` already does.

**Strongest argument against it:** a fresh script state per call costs its
385,952-byte baseline plus sandbox construction each time, and — more seriously —
**one state now holds the residue of up to 4,096 evaluated chunks against a
16 MiB absolute backstop.** See Risks; this is the thing to verify first.

Binding consequences:
- The `ScriptTable` handle for each declaration is converted to a
  `BlockDefinition` and dropped **before the next file is evaluated**. No handle
  outlives its file, and none outlives `definitions()`.
- A host that will not start is `DefinitionSourceError::Unreadable { origin: <root>/blocks }`
  quoting `HostError`. It is not the content's fault and must not be reported as
  a malformed declaration.
- The loader uses **`HostLimits::default()`** — the shipped limits, never a
  test-sized host. This is what the spec's note on FR-1.1-S1 depends on.

### D3 — The raw key enumeration is bounded at the host and sorted there. **BINDING**

Approved by the lead; this decides its shape.

```rust
/// The keys a script table holds in its own right.
pub enum FieldNames {
    /// Every key, sorted, at most as many as the caller allowed.
    Enumerated(Vec<String>),
    /// The table holds more keys than the caller allowed. None were copied out.
    MoreThanAllowed { allowed: usize },
}

impl ScriptHost {
    pub fn field_names(&self, table: &ScriptTable, most: NonZeroUsize) -> FieldNames;
}
```

Four properties are binding.

- **Raw.** No metatable is consulted — not `__index`, not `__iter`, not
  `__pairs`, not `__len`. FR-4.2-S4 and FR-4.2-S5 hold it to that. The
  implementer verifies the backend call actually is raw rather than assuming it;
  `vm.rs:293` and `vm.rs:312` are the precedents to follow.
- **Bounded at the host, and the bound is a parameter.** This is the only place
  the field-count bound can actually bind: a `Vec<String>` returned unbounded
  would allocate the hundred-thousand-key table the bound exists to refuse, and
  the refusal would arrive after the allocation it was meant to prevent. The
  bound is a *parameter* because `mc-script` may not learn a block-specific
  number. At most `most` names are copied out; a table with more is reported
  without retaining them.
- **Total over key types.** A key that is not a string (`{ true }` has key `1`)
  is rendered to text by the same rendering `print` uses, never skipped. FR-2.4
  says an unrecognised field is refused rather than ignored; a skipped key would
  make that promise partial. *See Assumptions — no scenario covers this.*
- **Sorted lexicographically, in `mc-script`, and documented as such.** The
  alternative — return the state's order and let the loader sort — was
  considered and rejected: Lua leaves hash-part order unspecified, so the state's
  order carries **no information at all**, and returning noise invites a second
  consumer to render it. The asymmetry decides it: a caller forgetting to sort
  produces an intermittently red guard, and no caller can ever want an order that
  does not exist.

**Strongest argument against:** sorting is a policy `mc-script` otherwise refuses
to hold, and the array part of a table *does* have a meaningful order that
lexicographic sorting mangles (`"1", "10", "2"`). Accepted: a block declaration
has no array part, and a caller that needs positional access needs an indexed
read, not this.

### D4 — Script output retained by the host must be bounded. **BINDING; ruled in scope, and now FR-4.3-S9**

`ScriptHost::printed` is a `Vec<String>` that grows without limit
(`host.rs:69,291`; `vm.rs:376–384` pushes every `print` unbounded). Its own doc
comment says so. Until now no production code called `evaluate` or `dispatch`, so
nothing content-authored could reach it. **This spec creates the first production
path**, and with the shipped limits one declaration chunk can reach it hard: each
`print` is a host call costing one interrupt tick, so a chunk may make on the
order of 500,000 calls inside its 1,000,000-tick budget, each handing over a
string built inside the 256 KiB per-entry cap — the script-side string becomes
garbage while the host-side copy is retained outside every limit. Multiplied by
4,096 declarations this is tens of gigabytes.

That is a hostile mod taking down the server through the loader, which Invariant 3
makes non-negotiable and which therefore outranks "the spec did not list it".

Draining instead of bounding does not fix it: a single chunk exceeds any
per-file drain. So the bound goes where the growth is, and three properties are
binding:

- **It lives in `HostLimits`**, beside the call-and-loop budget and the memory
  cap. A limit that exists only as a constant inside the host is a limit no
  operator can read or set.
- **Reaching it stops recording; it does not drop the oldest.** The first line a
  chunk printed is the one that locates a failed load; the millionth is not.
- **Truncation is observable.** `printed()` alone cannot distinguish "the mod
  printed nothing" from "the host stopped keeping it", and those are different
  facts. `dropped_print_lines()` is what tells them apart, and FR-4.3-S9 asserts
  the distinction rather than merely that the buffer stopped growing.

This was escalated and ruled in scope: the spec now carries **FR-4.3-S9** and the
reasoning sits in its Technical Considerations under "Why script output needs a
bound". Its positive control is the top-level-`print` control already owed by
FR-4.2-S3 — a truncation counter that never counted would otherwise read the
same as a chunk that printed nothing, which is the failure this bound is being
made observable to avoid.

### D5 — The chunk name and the definition origin are deliberately different. **BINDING**

`ScriptHost::evaluate(name, source)` stamps `name` into the fault's
`ScriptOrigin::chunk`. FR-3.3-S1 requires the refusal's origin to be
`content/base/blocks/amber.luau` "and not the chunk name the scripting host was
given".

If the loader passes the full path as the chunk name, the two coincide and the
scenario becomes true by construction — the exact "agreement between two copies"
shape `testing.md` §2 warns about. So: **the chunk name is the file's name alone
(`amber.luau`); the `DefinitionOrigin` is built by the loader from the full path**,
by the same `origin_of` that `toml_source.rs:75` uses. An implementation that
lifted the origin out of the `ScriptFault` then produces `amber.luau` and goes
red, which is what makes the scenario worth running.

### D6 — A chunk fault's cause is composed from the fault's typed fields, never from `ScriptFault: Display`. **BINDING**

`ScriptFault`'s `Display` (`fault.rs:113`) opens with ``chunk `amber.luau` ``.
Splicing it into a `DefinitionFault`, which already renders its own origin
(`source.rs:47`), states the location twice —
`crates/mc-client/tests/refusals_state_a_cause_once.rs` exists because that class
of duplication shipped once already, and it counts occurrences rather than
searching for them.

So the loader reads `ScriptFault::kind`, `::line` and `::cause` and composes:
`<kind>` , optionally `, line <n>`, then `: <cause>`. `FaultKind::as_str` already
spells "call and loop budget exhausted" and "allocation refused", which is what
FR-4.1-S1 and FR-4.1-S2 are asserting against. This text is quoted verbatim by
`docs/modding/`, so it is a documented contract from the moment it lands.

### D7 — Check order inside the loader. **BINDING**

Fixed, because three scenarios assert which refusal wins:

1. List `<root>/blocks/`. Unreadable → `Unreadable { origin: <root>/blocks }`
   (FR-1.2-S3).
2. Keep the entries whose name carries the extension `luau`, passing over every
   other entry in silence (FR-1.2-S2). Sort by file name (FR-1.2-S1). Then
   **refuse** — `Unreadable`, naming the entry's whole path — any kept entry
   that is not a file (FR-1.2-S4).

   **Refused, not dropped, and the two halves of this step are deliberately
   different.** An earlier wording said only "keep the entries that are files
   with extension `luau`", which reads as one filter and cannot coexist with a
   scenario demanding a refusal that names the path: a filter that quietly
   dropped `nested.luau` would leave the good declaration beside it registering
   cleanly and nothing refused at all. Naming a thing `*.luau` is how a mod
   author says *this is a declaration*, so an entry that says it and is not one
   is answered, while an entry that never claimed to be one is not. Sorting
   ahead of the file-type check is what makes a root holding two offenders
   refuse the same one on every run.
3. **Count before reading anything.** Over 4,096 → refuse naming the directory
   and the count bound (FR-4.3-S1 requires this to win over an invalid file).
4. Per file, in order: **size from directory metadata before opening**
   (FR-4.3-S2 requires this to win over a syntax error), then read, then
   `ScriptHost::evaluate`.
5. Not a table → refuse naming the file, no block, no field (FR-1.1-S3/S4).
6. Read `name` raw for attribution, as `raw.rs:99` does today, so every later
   refusal can name the block.
7. `field_names(table, 64)` → over the bound refuses naming the file and the
   field-count bound (FR-4.3-S5); otherwise every name not among the six
   recognised is refused, all of them listed (FR-2.4-S1/S3).
8. Check the six fields and their id rules, producing a `BlockDefinition`.

### D8 — Bounds live in `mc-world` as named constants, measured as stated. **BINDING**

`4_096` declarations · `256 * 1024` bytes per file · `256` **characters** of
declared text (`chars().count()`, not bytes — the spec says characters and
non-ASCII would otherwise refuse at a different point than documented) · `64`
field names. Each refusal states the observed quantity and the bound, so a reader
can tell "slightly over" from "far over", and each bound accepts its own limit
(FR-4.3-S6/S7/S8).

**Narrowed at phase 4: the field-count refusal states the bound alone, and
cannot state the observed quantity.** D3 has the enumeration stop one key past
the allowance rather than walk a table it is refusing to allocate, so
`FieldNames::MoreThanAllowed { allowed }` carries the allowance and nothing else
— how many keys the declaration really held is a number the design deliberately
never learns. Reporting it would mean copying out the hundred-thousand-key table
the bound exists to refuse, which is the requirement inverted. The general rule
holds for the other three bounds, where the observed quantity is known before
anything is allocated on the strength of it; FR-4.3-S5 asks for the file and the
bound and does not ask for a count.

**One rationale in the spec is wrong and the requirement still stands.** The
declared-text bound is justified as protecting the copy out of the script state;
it cannot, because `ScriptHost::read_field` returns `ScriptValue::Text(String)` —
the copy is already made by the time the loader sees it. What that bound really
buys: it bounds what a `BlockDefinition` **retains** (4,096 × three strings), and
the transient copy is separately bounded at ~16 MiB by the host's memory
backstop, since the string had to exist inside the state first. Worth keeping,
for the reason that is true. The field-count bound's rationale *is* correct and
is honoured by D3.

### D9 — The simulation loads content; the client is handed it resolved. **BINDING; added by the 2026-08-16 amendment**

`docs/planning/client-server-split.md` is the binding reasoning and is not
re-derived. What it decides for this spec:

**The construction moves, the loader does not.** `LuauFileDefinitionSource` and
`TomlFileHudSource` stay in `crates/mc-world/src/content/` where D1 put them.
`mc-script` keeps its no-workspace-crate property. No crate moves and `mc-client`
gains no dependency. What changes is which crate says `new`: `mc-sim` gains the
function that turns a content root into loaded content, and the three doors in
`mc-client` — `prepare_launch`, `prepare_scene` and `empty_hud` — call it instead
of constructing sources of their own. `content_root` moves with them, because a
client that resolves the content directory for itself is a client that can still
read it.

`prepare_launch` and `prepare_scene` keep their names, their signatures' shape
and their place in `mc-client`: they still mesh, pack and answer with a
`PreparedLaunch`/`PreparedScene`. The golden frames are shot through them and the
exit criterion is decided there, so moving *them* is the thing this spec must not
do.

**Two values, and the distinction is the whole decision.**

- What `mc-sim` hands back for the *simulation* is what it hands back today: a
  registry carrying all six fields, because every edit resolves the name it
  writes against it. In this arrangement the client binary is also the server, so
  that registry is still in scope in `mc-client`. **That is a residue of one
  binary hosting both halves, and it is what the composition-root spec removes.**
- What the client *draws and meshes from* is a resolved content value carrying
  each block's name, its texture key, its solidity and a layer assignment — and
  none of `replaceable`, `breakable` or `breaks_into`. FR-7.3-S1 is the
  assertion that distinguishes a seam from a rename, and it is worded so the
  fixture is written down rather than read: no registry built, no path opened,
  no scripting host constructed.

**FR-7.2 is asserted by discrimination, not by absence.** A type that simply has
no `breakable` field cannot fail a test about not having one; the test would not
compile. So the oracle is two roots differing *only* in the mutation rules
resolving to one and the same client content (S1), against two roots differing in
a `texture` and a `solid` resolving to content that differs (S2). Both directions
are needed: S1 alone is satisfied by a resolver that returns a constant.

**The layer assignment stops being derived.** `startup.rs:336` (`layers_of`)
resolves an index as a key's position in the lexicographically sorted key set,
and that index rides inside every packed vertex — so inserting one block
renumbers every index after it, silently, which is a live defect on hot reload in
one process today. The resolved value states key-to-index pairs and the client
honours them. FR-7.4-S1's fixture makes the assignment disagree with the
positional order **on purpose**: a test comparing two copies of the same sort
cannot fail. FR-7.4-S2 closes the other half — a block for which the assignment
names no layer is refused rather than drawn as layer zero.

**Traced before this decision was written: the layer index is the only
registry-derived value in a packed vertex.** `geometry/vertex.rs:75–81` — three
section-local coordinates, a `Facing`, a layer index and a scene section index.
Position and facing are geometric; the section index is written `0` by
`build_section_geometry` and filled at scene assembly. So no second
registry-derived value rides beside the layer, and FR-7.4-S1 is not undermined by
one.

**What the trace found instead, and it is the same hole one level down.** The
layer index is not derived through the registry on the *consuming* side.
`TextureLayers::resolve(&registry.texture_keys())` builds the assignment from
each block's declared **`texture`**; `layer_for` (`geometry/mod.rs:171`) selects
an entry by parsing the block's **`name`**. The two agree only because all four
shipped blocks declare them identical. `crates/mc-render/CLAUDE.md` and
`docs/technical/architecture.md:589–590` both record it as a known gap;
`product/roadmap.md:122` assigns it to **PRO-902/PRO-914**. **A second consumer
does the same thing and none of those notes mentions it**:
`mc_render::hud::held::held_swatch` (`hud/held.rs:109`) resolves the held-block
indicator by the block's name too.

**Not closed here** — it changes `build_section_geometry`'s binding signature and
the roadmap constrains this spec to identical fields and no new surface. **Pinned
here instead**, by FR-7.4-S3: a block declaring a `texture` other than its name
is refused at packing time naming the block. That is a test rather than a
comment, it is falsifiable today, and it goes red the day PRO-902 closes the gap,
which is exactly when somebody should look at it. It is a scenario with a known
expiry, and that is recorded rather than left for a later reader to discover as a
puzzle.

**Consequence for the documentation deliverable, which is the part with teeth.**
FR-6.2 checks that the walkthrough's example *loads*; nothing checks that it
*draws*. So a worked example declaring `texture` different from `name` would pass
every guard this spec has and still leave an author's block invisible. The spec's
documentation deliverable now constrains T22 accordingly.

**The interim instrument is a source scan, and it is the weaker one.** The
dependency-closure guard cannot pass while one binary hosts both halves, so
FR-7.1 scans `crates/mc-client/src/` for four **chokepoints** — `registry.apply(`,
`HudLayout::load`, `BlockRegistry::new`, `content_root` — rather than for type
names, because renaming a source does not rename the door. It follows
`crates/mc-client/tests/seam_boundaries.rs`: whole-path exemption comparison
(never a bare file name), doc comments stripped, sibling `*_test.rs` skipped, a
positive control, and — FR-7.1-S3 — an enumerated verdict rather than
`hits.is_empty()`, so a scan that read nothing reports that instead of reporting
a clean client.

**Known residual, and it belongs in the guard's own doc comment:** somebody
adding a *second* door — a new public registration call — bypasses it, and no
text scan closes that.

**Strongest argument against:** two of the four needles are total only because
the tree says so today (`BlockRegistry::apply` the only way to populate a
registry, `HudLayout::load` the only door into a layout). That is a property
those two modules assert about themselves, and a text scan cannot verify it.
Accepted, named, and it is exactly why the closure guard is a better instrument
and why the composition-root spec should take it as its exit criterion.

### Trivial decisions, one line each

- The extension is `luau`, one declaration per file, chunk returns a table — settled in `requirements.md` Decisions 3 and 4.
- `toml_source.rs` and `raw.rs` are deleted whole; the field/default/refusal logic is re-expressed against `ScriptValue`, the `serde` derive is not ported.
- `mc-world` keeps `toml` (HUD) and `serde` (persistence); `dependency_graph.rs` holds unchanged. Verify at phase 5 rather than assume.
- The directory constant stays `blocks`; `mc-client/tests/support/content.rs`'s single `DECLARATION_EXTENSION` splits in two, because HUD stays TOML.

## Interfaces

### `mc-script` (new public surface, engine-facing)

```rust
pub enum FieldNames {
    Enumerated(Vec<String>),                 // sorted, at most `most` entries
    MoreThanAllowed { allowed: usize },      // nothing copied out
}

impl ScriptHost {
    /// The keys `table` holds in its own right, without invoking script.
    pub fn field_names(&self, table: &ScriptTable, most: NonZeroUsize) -> FieldNames;

    /// How many lines of script output this host stopped retaining. (D4)
    pub fn dropped_print_lines(&self) -> u64;
}

pub struct HostLimits {
    // …existing fields…
    /// Total bytes of script output one host retains. (D4)
    pub retained_print_bytes: NonZeroUsize,
}
```

### `mc-world` (new public surface)

```rust
pub struct LuauFileDefinitionSource { /* root: PathBuf, printed: RefCell<…> */ }

impl LuauFileDefinitionSource {
    pub fn new(root: impl Into<PathBuf>) -> Self;   // infallible, touches no disk
    /// What content printed while this source was last read.
    pub fn printed(&self) -> Vec<String>;
}

impl DefinitionSource for LuauFileDefinitionSource {
    fn origin(&self) -> DefinitionOrigin;           // the root, as today
    fn definitions(&self) -> DefinitionStream<'_>;  // eagerly collected (D2)
}
```

`printed()` exists because FR-4.2-S3 needs an observable — "the host having
recorded nothing printed on that chunk's behalf" — and the host is internal to
`definitions()` under D2. Asserting instead against a separately constructed
`ScriptHost` would be agreement between two copies of the same decision.
**It needs a positive control:** a declaration that calls `print` at top level
must show up in `printed()`, or an implementation that never records anything
leaves FR-4.2-S3 green forever. That control is not in the spec; it belongs in
`test-map.md` under additional coverage.

### Error contract

Every refusal is `DefinitionSourceError`. Which variant, fixed:

| Condition | Variant | `origin` | `block` | `field` |
|---|---|---|---|---|
| `blocks/` missing or unlistable | `Unreadable` | `<root>/blocks` | — | — |
| host would not start | `Unreadable` | `<root>/blocks` | — | — |
| file unreadable | `Unreadable` | file path | — | — |
| declaration count over bound | `Malformed` | `<root>/blocks` | `None` | `None` |
| file size over bound | `Malformed` | file path | `None` | `None` |
| chunk failed (compile, raise, budget, memory, sandbox) | `Malformed` | file path | `None` | `None` |
| chunk returned a non-table | `Malformed` | file path | `None` | `None` |
| field count over bound | `Malformed` | file path | declared name if read | `None` |
| unrecognised field(s) | `Malformed` | file path | declared name if read | first unrecognised, all listed in the cause |
| field missing / wrong kind / bad id / over text bound | `Malformed` | file path | declared name if read | the field |

Empty source and duplicate name stay `RegistryError::NoDefinitions` and
`::AlreadyRegistered`, unchanged — FR-3.1-S2 and FR-3.2-S1 come from
`registry.rs` and this spec must not reimplement them.

## Data

No change to `BlockDefinition`, to the registry, or to the save format. That is
the point of the spec, and FR-1.3-S4 is what proves it: `behaviour_of` and
`appearance_of` (`persistence/format.rs:307,319`) exclude `origin`, so a save
written against the TOML declarations must load against the Luau ones reporting
nothing changed. No migration, no retention rule, no sensitive data.

`content/base/blocks/{dirt,grass,stone,water}.luau` replace the four `.toml`
files, preserving exactly the fields recorded in `requirements.md` — only
`water` declares `solid = false` and `replaceable = true`.

## Integration

| Touched | What connects | What must not break |
|---|---|---|
| `crates/mc-world/Cargo.toml` | `mc-script` added, with a note in the style of the `toml`/`postcard` entries confining it to `src/content/` | `mlua` never appears here (Boundaries) |
| `crates/mc-world/src/content/mod.rs` | `luau_source` in, `toml_source` and `raw` out | its module doc says the module is the only place that knows a definition is TOML — rewrite it |
| `crates/mc-client/src/launch.rs:188` (`prepare_launch`) | the construction leaves for `mc-sim` (D9) | `PreparationError::NothingToPlace` (line 123) already implements FR-1.3-S3 — it is a regression guard here, not new work |
| `crates/mc-client/src/startup.rs:267` (`prepare_scene`) | the construction leaves for `mc-sim` (D9) | **the golden-frame path**; the roadmap's "renders identically" criterion is decided here, and FR-5.1-S3 exists because it is a second entry point untested until something asserts through it. The function itself does **not** move |
| `crates/mc-client/src/startup.rs:370` (`empty_hud`), `:188` (`content_root`) | both move to `mc-sim` (D9) | the third and fourth chokepoints; leaving either behind needs an exemption on a door the guard exists to watch |
| `crates/mc-sim` | gains the function that turns a content root into loaded content, and the resolved value the client's view is built from | it gains `mc-world`'s content module as a consumer, not the scripting host as a direct edge; `mc-client` must still name no scripting host |
| `crates/mc-client/src/startup.rs:335` (`layers_of`) | stops deriving a layer index positionally and starts honouring a stated assignment (phase 6) | it is the one place the key set is chosen, and both preparation paths call it — that must stay true |
| 13 test call sites naming `TomlFileDefinitionSource` | across `mc-world`, `mc-sim`, `mc-client` tests and their `support/` modules | `mc-sim/tests/support/chamber.rs:227` names the type in a doc comment about file-name order |
| `crates/mc-client/tests/support/content.rs` | one `DECLARATION_EXTENSION` serves blocks and HUD; must split | the shipped-copy helpers' refusal that a fixture adds a file the shipped root already declares |
| `crates/mc-client/tests/documented_refusals.rs` | `BLOCK_FILE` → `amber.luau`, `CARRYING_AN_UNRECOGNISED_FIELD` → a Luau chunk | verdicts, three controls and both normalisations unchanged; **the HUD half stays TOML**, so the parser-bump note in its header stays true for that half and must be narrowed rather than deleted |
| `crates/mc-world/tests/dependency_graph.rs` | nothing | both assertions must still hold — verify, do not assume |
| `crates/mc-testkit/tests/workspace_layering.rs` | nothing | `INSPECTED` is a per-crate roster; re-read it with the new edge in mind |
| `crates/mc-world/tests/no_hardcoded_block_names.rs` | nothing | it scans `.rs` only; moving content between file formats does not touch it, and it is an MVP-2 exit criterion |

### Documentation (Key Principle 3, part of done)

- **Mod author** — `docs/modding/blocks-items.md` becomes the Luau contract: file
  layout, declaration shape, all six fields with type/bound/default, the id rule,
  all-or-nothing loading, every refusal and how to read it, the four bounds, and
  a worked example that runs. `docs/modding/README.md`'s first-block walkthrough
  rewritten against `amber.luau`; FR-6.2 holds its example to loading.
- **Player** — stated plainly: nothing visible changes. Same blocks, terrain,
  held block, saves.
- **Engine reader** — `docs/technical/architecture.md`: the new `DefinitionSource`
  implementation, the `mc-world → mc-script` edge with D1's reasoning **and the
  rule that `mlua` must never reach `mc-world`**, the raw key enumeration and what
  it is for, the four bounds with rationale (including D8's correction), D5's
  chunk-name/origin split, and D9's seam — the simulation loads, the client
  receives resolved content, the layer assignment is honoured rather than derived,
  and which four chokepoints the source scan watches and why chokepoints rather
  than type names. **The passage asserting that a client which cannot reach the
  scripting host cannot draw the world is corrected ahead of the implementation**,
  because it reads as an as-built record and a future spec author would follow
  it; it is corrected rather than deleted, so the question "should the client
  evaluate content?" finds an answer instead of silence.
- **Retire all of it** — the six "nothing can be authored in Luau" statements
  listed in `requirements.md`, plus `blocks-items.md`'s "What MVP 2 changes",
  `README.md`'s "a mod is a directory of data files" and its `*.toml` routing row,
  and `content/CLAUDE.md`'s "MVP 1 today vs. MVP 2" and `mycraft.state` note.

## Phasing

The load-bearing constraint: no phase may open with its scenarios already green.
Two forces work against that here and they decide the shape.

**Force one — much of FR-3 and FR-4.1/4.2 is free the moment the loader exists.**
All-or-nothing, duplicate naming and empty-source refusal come from
`registry.rs`. The budget, the memory cap and the sandbox come from
`ScriptHost::evaluate`. `__index` rawness comes from `read_field`. The spec says
as much, and FR-4.1 is explicitly "wiring". None of these can be made red by a
later phase — only by there being no loader yet. **They therefore belong in the
first phase**, where a skeleton can still redden them.

**Force two — one skeleton reddens phase 1, and it must be the *registering* one.**
Fourteen of phase 1's twenty-five scenarios assert a refusal, so a
refuse-everything skeleton passes them vacuously. A skeleton that walks
`<root>/blocks/` and yields a fixed `BlockDefinition` per file reddens all
twenty-five: the registering scenarios fail on field values, the refusing ones
fail because something registered. Checked against each.

| Phase | Scenarios | What it delivers | Why it is red on arrival |
|---|---|---|---|
| **1 — A declaration is evaluated, checked and registered, under the host's guard** (25) | FR-1.1-S1..S5, FR-2.1-S1..S7, FR-2.3-S1, FR-2.3-S2, FR-3.1-S1..S3, FR-3.2-S1, FR-4.1-S1..S4, FR-4.2-S1..S3 | `LuauFileDefinitionSource`; evaluation through `ScriptHost::evaluate` at shipped limits (D2); the three required fields read raw; `BlockName`/`TextureKey` parsing; `DefinitionFault` attribution | nothing exists; the registering skeleton above |
| **2 — Which files are declarations, and where a refusal points** (7) | FR-1.2-S1..S4, FR-3.3-S1..S3 | extension and file-type filter, file-name sort, the loader-owned origin (D5), the composed cause (D6) | **only if phase 1 does the minimum** — see the constraint below |
| **3 — The optional fields, the residue id, and the field nobody recognises** (11) | FR-2.2-S1..S4, FR-2.3-S3, FR-2.3-S4, FR-2.4-S1..S3, FR-4.2-S4, FR-4.2-S5 | `ScriptHost::field_names` (D3); `replaceable`/`breakable`/`breaks_into` with their defaults; the unknown-field refusal; `__iter`/`__len` rawness | phases 1–2 ignore extra fields and never read the three optionals |
| **4 — Every content-supplied quantity has a bound** (9) | FR-4.3-S1..S9 | the four loader bounds and their check order (D7, D8), plus the retained-output bound in `HostLimits` and its truncation counter (D4) | nothing bounds anything yet |
| **5 — The base game ships in Luau, the simulation is what loads it, TOML is retired, the pages are true** (15) | FR-1.3-S1..S4, FR-5.1-S1..S3, FR-6.1-S1..S3, FR-6.2-S1..S2, FR-7.1-S1..S3 | the four `.luau` declarations; the construction moved into `mc-sim` (D9); delete `toml_source.rs`, `raw.rs` and the `.toml` files; both call sites; 13 test call sites; the whole documentation deliverable; the guard's fixture moved; the client-source scan | `content/base/` still holds TOML, both call sites still use the TOML source, and `mc-client` still names all four chokepoints |
| **6 — The client receives content resolved** (6) | FR-7.2-S1..S2, FR-7.3-S1, FR-7.4-S1..S3 | the resolved content value; the client's view built from it alone; the layer assignment shipped and honoured, with a block it names no layer for refused, and the name-for-texture substitution pinned | no resolved value exists, and phase 5 deliberately leaves `layers_of` deriving an index positionally. **FR-7.4-S3 is green on arrival** — it pins behaviour the tree already has |

**Binding sequencing constraint on phase 1.** Phase 1 lists `<root>/blocks/` in
the order the filesystem returns, filters nothing, and derives the refusal's
origin from the `ScriptFault`. It does **not** sort, does **not** filter by
extension, and does **not** own the origin. This is deliberate under
`testing.md` §2 ("implement deliberately less first"): copying
`toml_source.rs:47–58` wholesale into phase 1 would hand phase 2 four scenarios
that pass on arrival. Whoever implements phase 1 will be tempted; the tasks
breakdown must say so out loud.

**Scenarios that are green on arrival inside their own phase, named rather than
hidden.** FR-2.4-S2 (all six fields register) after phase 2; FR-4.3-S6/S7/S8 (each
bound accepts its own limit) against an unbounded loader; FR-1.3-S3 (refuse to
start with no solid block), which `PreparationError::NothingToPlace` already
implements. All of these are controls — the accept-side of a bound and the
positive-control side of a refusal — and `testing.md` §2 wants exactly them.
Their value is that they redden if the refusing side over-fires, which is a real
failure mode. They should not be counted as evidence that their phase did work.

**FR-7.4-S3 is green on arrival for a different reason and must be read
differently.** It is not a control: it pins behaviour the tree already has —
`layer_for` selecting by the block's name — so that the name-for-texture
substitution is a test instead of a comment in two CLAUDE.md files. It is not
evidence phase 6 did work, and it is the one scenario in this spec with a known
expiry: PRO-902 closing the gap turns it red, and that is the signal it exists to
give.

## Assumptions

A reviewer may veto any of these; each stands in for something no scenario states.

1. **A non-string key is an unrecognised field.** `{ true, name = … }` has key
   `1`; D3 renders it to text and D7 step 7 refuses it. The spec has no scenario
   for a non-string key, and the alternative — skipping it — makes FR-2.4's
   promise partial. Worth a scenario.
2. **A field-count refusal may name the block** if `name` was already read
   (D7 step 6). FR-4.3-S5 asks only for the file and the bound and does not
   forbid the block.
3. **Declared text is measured in `chars()`.** The spec says characters.
4. **`printed()` on the source reports the last read**, not an accumulation
   across reads. Nothing depends on the other reading, and accumulation would be
   unbounded across the hot reloads PRO-918 brings.
5. **`math.random` is reachable from a declaration chunk** (it is in
   `PERMITTED_GLOBALS`) and no scenario forbids a non-deterministic declaration.
   D2's fresh state per call makes two reads *more* alike, not less. Left alone;
   noted because worldgen-in-script will have to face it.

## Risks

| Risk | What to verify, and when |
|---|---|
| **4,096 chunks in one script state exhaust the 16 MiB backstop.** D2 keeps one state for a whole `definitions()` call; compiled chunks and their residue accumulate, and only `collected_memory_in_use()` forces a collection — as a side effect of a getter, which is the only public lever. FR-4.3-S6 is the scenario that finds out. | **Verify first, in phase 4, before the rest of phase 4 is built.** If it trips: drop handles earlier, force collection every N files, or — last — give the loader its own documented `HostLimits`. That last one is **DEFERRED**: revisit only if the first two fail, and note that FR-1.1-S1 must still run at shipped limits. |
| **Unbounded retained script output (D4).** A single declaration chunk can hand the host tens of gigabytes of `print` text within its shipped budget, outside every script-side limit. Invariant 3. | Fixed in this spec by D4, ruled in scope, and pinned by FR-4.3-S9 in phase 4. |
| **`documented_refusals.rs` goes intermittently red** if the unrecognised-field list is ever rendered in the state's own key order. | D3 sorts in `mc-script`. The phase-3 test author should assert the order over a table built to have several unrecognised keys, run repeatedly. |
| **The golden frames move.** `prepare_scene` is the path they are shot from. A field mapped to the wrong place changes what is drawn. | FR-1.3-S4 (the save hash oracle) and FR-5.1-S3 are the two instruments. Run the golden-frame suites in phase 5 before the documentation work, not after. |
| **13 test call sites and a shared fixture constant.** The single `DECLARATION_EXTENSION` in `mc-client/tests/support/content.rs` serves blocks *and* HUD; splitting it wrongly silently retargets HUD fixtures. | Phase 5. Both `hud_content_loading.rs` and `shipped_hud_outlines.rs` must stay green. |
| **Vendor-failure blast radius.** If `mlua`/Luau had to be replaced, the files changing are `crates/mc-script/src/luau/` plus `HostLimits`. This spec must not widen that. | Structural: `mlua` absent from `mc-world/Cargo.toml`. State it in the engine-reader record as the thing a future change may not break. |
| **`mc-render` and `mc-sim` newly resolve a vendored C++ VM** (D1). | Accepted with a named revisit condition. Re-read `workspace_layering.rs`'s `INSPECTED` roster in phase 5. |
| **The construction move and the layer-assignment change both touch the golden path** (D9). `prepare_scene` is what every golden frame is shot through, and a layer index rides inside every packed vertex — an assignment resolved differently redraws the world with no error anywhere. | Run the golden-frame suites twice: after the construction move in phase 5, and again after the assignment is honoured in phase 6. A difference after the first is a wiring defect; a difference after the second is the assignment. Running them once at the end cannot tell those apart. |
| **The seam becomes a rename.** A resolved value that is a newtype over the registry, or a client view that reaches back through it, leaves every scenario green while nothing was cut. | FR-7.3-S1 is the instrument and it only works if the fixture is *written down*: no registry built, no path opened, no scripting host constructed. That is a constraint on the code that builds the fixture, which no assertion can enforce — the phase-6 test author holds it, and a reviewer reads it. |

**Why the seam splits across two phases.** FR-7.1's scan is red on arrival in
phase 5 only because `mc-client` still names all four chokepoints when the phase
opens; moving the construction is what greens it, which is the ordinary TDD
sequence and puts the scan in the same phase as the move. FR-7.2 to FR-7.4 need
a value that does not exist yet, and they must not be reachable until it does —
so **phase 5 deliberately leaves `layers_of` deriving an index positionally and
leaves the registry travelling whole.** That is the same "implement deliberately
less first" the phase-1/phase-2 split already runs on, and the same temptation
applies: whoever moves the construction in phase 5 will be tempted to define the
resolved value while they are in there, and taking it hands phase 6 four of its
five scenarios green on arrival.

## Phase totals

25 + 7 + 11 + 9 + 15 + 6 = **73**, which is the whole spec: the 63 counted at the
start of the architecture stage, plus FR-4.3-S9, plus FR-7's nine from the
2026-08-16 amendment.
