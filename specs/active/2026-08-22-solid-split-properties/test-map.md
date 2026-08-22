# Test map: One bit answers four questions

Scenario → test file → test name. Test names carry no spec or scenario id
(`standards/global/scenario-guidelines.md`); this file is the only mapping.

## Phase 1 — A block declares three new properties

Seven scenarios: FR-1.1-S1..S4, FR-1.2-S1, FR-1.2-S2, FR-8.1-S1.

| Scenario | File | Test |
|---|---|---|
| FR-1.1-S1 | `crates/mc-world/tests/luau_declaration_properties.rs` | `a_solid_block_that_states_nothing_more_is_drawn_occludes_and_can_be_aimed_at` |
| FR-1.1-S2 | `crates/mc-world/tests/luau_declaration_properties.rs` | `a_non_solid_block_that_states_nothing_more_is_undrawn_transparent_and_cannot_be_aimed_at` |
| FR-1.1-S3 | `crates/mc-world/tests/luau_declaration_properties.rs` | `a_non_solid_block_stating_it_is_drawn_still_neither_occludes_nor_can_be_aimed_at` |
| FR-1.1-S4 | `crates/mc-world/tests/luau_declaration_properties.rs` | `a_drawnness_written_as_a_number_is_refused_naming_the_field_and_the_two_values_it_accepts` |
| FR-1.2-S1 | `crates/mc-world/tests/luau_declaration_keys.rs` | `a_field_one_letter_past_a_real_one_is_refused_quoting_every_field_in_declaration_order` |
| FR-1.2-S2 | `crates/mc-client/tests/documented_refusals.rs` | `every_refusal_the_modding_pages_quote_is_a_refusal_the_client_prints` — **an existing test, judged already sufficient; see below** |
| FR-8.1-S1 | `crates/mc-client/tests/documented_property_refusals.rs` | `the_blocks_guide_quotes_both_refusals_a_misdeclared_drawnness_raises` |

### FR-1.2-S2 is closed by a test that already existed, and here is the measurement

FR-1.2-S2 asks that the engine produce, for the unrecognised-field refusal,
exactly the message the modding guide prints. **Three pages already quoted that
refusal, not one** — `grep -rn "a declaration may state" docs/` finds it on
`docs/modding/blocks-items.md`, `docs/modding/hot-reload.md:378` and
`docs/modding/README.md:183`, and the guard below sweeps every page under
`docs/modding/` rather than a named one. All three carried **byte-identical**
six-name quotations, and the guard's failing output shows the quotation and the
produced text **but not the page it came from** — so updating one page leaves the
guard red on a quotation that looks like the one just fixed, with nothing hinting
that two more exist. That is the hour it costs, and it is a fact about the
instrument rather than about this spec. The guard's own direction is what saves
it in the end: it will not go green until the last of the three is corrected.

With that said, `docs/modding/blocks-items.md:396` quotes it and
`every_refusal_the_modding_pages_quote_is_a_refusal_the_client_prints` compares
every fenced quotation under `docs/modding/` against a real run, line for line,
in the quoted → produced direction. Measured rather than argued: with the loader
at nine recognised fields and `:396` still quoting six, that test fails and
reports both halves —

```
left: Mismatch { quoted: "… a declaration may state `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`",
               produced: "… a declaration may state `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`, `drawn`, `occludes`, `targetable`" }
right: EveryQuotedRefusalIsTheRefusalPrinted
```

Writing a second test for FR-1.2-S2 would re-prove that through the same code
path, which `standards/global/testing.md` §1 forbids. What that test **cannot**
see is a refusal the page quotes *nowhere*, because its direction is quoted ⊆
produced — which is why FR-8.1-S1 needs a test of its own and gets one.

## Additional coverage

Each line states what it catches (`standards/global/testing.md` §1). None of
these is a scenario's own test.

- `crates/mc-client/tests/documented_property_refusals.rs` ·
  `a_page_quoting_one_of_them_is_reported_as_missing_the_other` — the positive
  control on FR-8.1-S1's verdict. A reading that came to answer "both are on the
  page" unconditionally leaves that guard green over any page at all, and the
  three-arm verdict alone rules out only a reading that *cannot* look. This is
  the only thing that watches the disagreeing arm come out of a real comparison.
  Green from the start, deliberately: it is a control, not a scenario.
- `crates/mc-client/tests/documented_property_refusals.rs` ·
  `the_guide_introduces_the_declaration_fields_in_the_order_a_refusal_quotes_them`
  — catches `FIELDS_IN_THE_ORDER_THE_GUIDE_STATES` (now in
  `crates/mc-client/tests/support/quoted_refusals.rs`) drifting from the loader.
  That mirror was **silently blind**: `ranked_field` returns `Option` and its
  caller skips what it cannot rank, so a list left at six ranked the three new
  fields nowhere, ordered nothing by them, and compiled. Mutation M2 below is the
  proof.
- `crates/mc-world/tests/luau_declaration_keys.rs` ·
  `a_declaration_stating_every_recognised_field_and_nothing_else_registers` —
  renamed from `…all_six_recognised_fields…` and extended to state all nine. A
  control exercising a subset of the contract stops being a control over the part
  it left out; this one now catches an unrecognised-field check that over-fires on
  any of the three new names.
- `crates/mc-world/tests/luau_declaration_keys.rs` ·
  `a_field_the_loader_does_not_recognise_is_refused_beside_the_ones_it_does` —
  unchanged in shape, but its mirror moved to nine, so it now catches the
  *opposite* drift from FR-1.2-S1's test: a loader still quoting six while this
  file expects nine. It is blind to the dangerous direction (a loader quoting a
  name the mirror does not know) because `named_in_order` filters the needles —
  which is exactly the hole FR-1.2-S1's test was written to close.

## Where a new refusal may be quoted on the blocks page, corrected

FR-8.1-S1's test says only *that* the two refusals are quoted. **Where they sit
is constrained by a different guard**, and this is a correction to advice the test
author gave that turned out to be wrong.

`the_modding_guide_states_every_per_facing_refusal_in_the_recognised_field_order`
ranks every quotation on the page by the field it blames and demands the guide's
own order. The advice given was that the region around "Reading a refusal" was
safe for the `drawn` quotation, on the grounds that the `texture`-blaming
quotations are all in the texture-table section above it. **That was derived from
one section rather than from the page**, and `ranked_field` matches
``field `texture` `` anywhere. The implementation measured the consequence:
`OutOfFieldOrder { field: "texture", after: "drawn" }`.

The real constraint, measured on the current tree: the last ranked `texture`
quotation is the declared-value bound refusal at
`docs/modding/blocks-items.md:496` (`` `texture` holds 257 characters ``), which
sits **below** "Reading a refusal", and the `drawn = 1` quotation now sits at
`:528` beneath it. Page order therefore ranks 1, 1, 1, 1, 1, 6. The `slid` and
`drawnn` quotations rank nowhere, because neither is a recognised field, so
`ranked_field` returns `None` for both — which is why they may sit beside each
other above all of it.

**The general rule, so the next field does not repeat this:** grep the whole page
for ``field `<name>` `` across every recognised field before choosing where a new
quotation goes. A section's boundaries are not the ranking's boundaries.

## Two skeletons, because no single one reddens all four FR-1.1 scenarios

Measured, not argued (`standards/global/testing.md` §2, "One skeleton is often
not enough"). Command:
`cargo nextest run -p mc-world -E 'binary(luau_declaration_properties) + binary(luau_declaration_keys)'`
— run without `--no-fail-fast`, and safe for a different reason than the
mutations below: both summaries came back `10 tests run` with no slash against a
selection of exactly ten, so nothing was cancelled unobserved. The `mc-client`
half of the same skeletons is the same shape — `3 run, 1 passed, 2 failed` and
`9 run, 7 passed, 2 failed`, both no-slash against selections of exactly that
size.

**The one run that had to have the flag, and did.** Whether the 21-site
adaptation broke anything is a claim about the *whole* workspace, and a
fail-fast run answers it for however many tests happened to start:
`cargo nextest run --workspace --no-fail-fast` over a tree with a correct loader
and a not-yet-updated page gave `1394 run, 1392 passed, 2 failed, 1 skipped`,
the two failures being the documentation guards and nothing else. The same
selection *without* the flag had reported `85/1394` — evidence about 85 tests,
in a line that reads like a verdict on 1 394.

| Skeleton | FR-1.1-S1 | FR-1.1-S2 | FR-1.1-S3 | FR-1.1-S4 |
|---|---|---|---|---|
| three fields resolved `false` unconditionally, `RECOGNISED_FIELDS` at six | **red** (assertion) | green — **vacuous** | red, but as a *refused root* rather than an assertion | **red** (assertion) |
| `RECOGNISED_FIELDS` at nine, three fields resolved `true` unconditionally | green — **vacuous** | **red** (assertion) | **red** (assertion) | **red** (`NothingRefused`) |

FR-1.1-S3 is the only one of the three defaults that reddens against both, and
it cannot reach its assertion at all until `RECOGNISED_FIELDS` accepts `drawn` —
so the field list has to grow before that failing output is worth displaying.

**One ordering constraint the intermediate states impose.** A loader that
*recognises* the three fields without *reading* them accepts `drawn = 1`, which
makes `printed_refusals()` unproducible — every consumer of it errors on the
fixture ("this scenario needs the content root … to refuse the run, and it
prepared a scene instead") rather than on an assertion. Six `mc-client` tests go
red that way. The read and the list growth land together.

## How to tell a cancelled run from a complete one, before any count below

`nextest` cancels the rest of a run on the first failure unless
`--no-fail-fast` is given, and it renders that as **`N/M tests run` — with a
slash**. A complete run has no slash: `388 run`. Measured on three runs in this
phase (`186/388`, `85/1394`, and the no-slash summaries below), not read out of
documentation.

**This matters most for the half of a mutation that says something stayed
green.** "One test red" survives a cancelled run; **"nothing else moved" does
not** — it is a statement about tests that may never have started, and it reads
identically either way. `assert!(nothing_else_failed)` from a cancelled run is
the absence-assertion defect one level up, in the report rather than in the
suite. Every count in this file therefore records its invocation, including the
ones that were safe for a different reason.

## Mutations run, and what each proved

Each was applied by hand, observed, and reverted **by re-editing the line** —
never `git checkout` — with `git diff --exit-code` clean afterwards. M1–M3 were
first run without `--no-fail-fast` and **re-run with it** before this table was
written; the re-run agreed with the original in every arm, and the numbers below
are the re-run's. M3 mutates the loader, so it was run with the implementation
author's explicit consent inside an announced window.

| Mutation | Invocation | Result |
|---|---|---|
| **M1** — `RECOGNISED_FIELDS` in `luau_declaration_keys.rs` cut to six, loader at nine | `cargo nextest run -p mc-world -E 'binary(luau_declaration_keys)' --no-fail-fast` → `6 run, 5 passed, 1 failed` | One test red: `a_field_one_letter_past_a_real_one_is_refused_quoting_every_field_in_declaration_order`. `a_field_the_loader_does_not_recognise_is_refused_beside_the_ones_it_does` printed an explicit **PASS** — the measured proof that the filtered reading is blind in this direction. |
| **M2** — `FIELDS_IN_THE_ORDER_THE_GUIDE_STATES` cut to six, loader at nine | `cargo nextest run -p mc-client -E 'binary(documented_property_refusals) + binary(documented_refusals)' --no-fail-fast` → `9 run, 8 passed, 1 failed` | One test red: `the_guide_introduces_the_declaration_fields_in_the_order_a_refusal_quotes_them`. `the_modding_guide_states_every_per_facing_refusal_in_the_recognised_field_order` **PASS** — the same blindness, one mirror over. Both remaining page guards also **PASS**, so the mirror is the only thing this mutation reaches. |
| **M3** — the loader's nine reordered (`OCCLUDES_FIELD` before `DRAWN_FIELD`) | `cargo nextest run -p mc-world --no-fail-fast` → `339 run, 338 passed, 1 failed` | `a_field_one_letter_past_a_real_one_…` alone, reporting both orders. **One test in 339 sees a reordered field list**, and the package it ran in opens nothing under `docs/`. |
| **M4** — `optional_boolean`'s `absent` made a literal `false` for all three fields (the implementation's own mutation, on the GREEN tree) | `cargo nextest run -p mc-world -p mc-core --no-fail-fast` → `388 run, 387 passed, 1 failed` | `a_solid_block_that_states_nothing_more_is_drawn_occludes_and_can_be_aimed_at` alone. **The first run of this was `186/388` under fail-fast** — a line that reads like a complete result and is evidence about 186 tests. |

**M4's three green arms are recorded because they are the measurement, not a
null result** (`testing.md` §2 asks for the outcome either way):

- **FR-1.1-S2 green** — a `false` default is what that fixture expects anyway.
  This is the vacuity the two-skeleton table above predicts, now measured against
  the real implementation rather than against a skeleton built to make the point.
- **FR-1.1-S3 green** — `drawn` is *stated* in its fixture, and `occludes` and
  `targetable` default to `false` either way. So S3 reddens against a wrong
  **value** but not against a wrong **default source**. That is a real limit of
  the fixture and it is why S1 exists.
- **FR-1.1-S4 green** — a wrong-kind refusal is raised before any default is
  reached.

### Why M3 is scoped to a package rather than mutating the page

The question it answers is: *the loader's nine in the wrong order, with the page
agreeing, so page-versus-run stays green.* **That stipulation is unobservable on
either tree this phase had.** Before the documentation task the page was stale,
so the page guard was already red for an unrelated reason — `testing.md` §2's
"red for a known reason hides red for an unknown one", and the reason the
original M3 could not have observed the stipulation at all. After it, the page
quotes the correct nine, so reordering the loader makes the page *disagree* and
reddens that guard for a second reason.

Scoping to `-p mc-world`, where **no test opens anything under `docs/`**, answers
it better than a page mutation would: with the page wholly out of scope, one test
in 339 still reddens on the reorder alone. "Does not route through the page" is
therefore a **structural** property of `luau_declaration_keys.rs` rather than an
experiment that came out the right way, and it is recorded as one.

## Which tree the lint measured, stated because the two are not the same tree

`cargo clippy --workspace --all-targets --all-features -- -D warnings` came back
**clean, exit 0** — but it measured the *mutated* tree, the one carrying the three
fields on `BlockDefinition` and reads for them in the loader. **That is not the
tree this commit contains.** The committed tree does not compile at all until T01
and T02 land, so no lint can be run against it, and a clippy line that did not
say which tree it read would be an absent instrument dressed as a clean one
(`standards/global/testing.md` §2).

What the distinction buys, concretely: the run **did** cover all 21 adaptation
sites, both mirrors and all three new test files, because those compile under the
mutation — so anything the lint could say about the test-side work has been said.
The two diagnostics it raised were:

- `error: this function has too many lines (35/30)` at
  `crates/mc-world/src/content/luau_declaration/mod.rs:149` — **the
  implementation's**, reported rather than fixed, since adding three
  `optional_boolean` reads to `check` is what trips it. It needs splitting, not an
  `#[allow]`.
- `error: all variants have the same postfix: Quoted` in
  `crates/mc-client/tests/documented_property_refusals.rs` — **mine**, fixed
  before the commit by renaming the verdict arms.

`cargo fmt --all -- --check` was clean against the same tree.

## The adaptation, and the rule it followed

`BlockDefinition` gains three fields and has no constructor, so every struct
literal moves. Measured with
`grep -rn "BlockDefinition {" --include=*.rs crates/ | grep -v "pub struct"`:
**22 constructions, 1 production** (`crates/mc-world/src/content/luau_declaration/mod.rs`,
the implementer's) and **21 in fixtures**, across 18 files.

**Every one of the 21 passes the site's own solidity through to all three new
fields.** Phase 1 moves no shipped quad, pixel, verdict or revision byte and
nothing reads the three fields yet, so a fixture must mean exactly what it meant
before. A literal `is_solid: true` became `drawn: true, occludes: true,
targetable: true`; a parameterised `is_solid` became `drawn: is_solid, …`. No
comment was added at any of the 21 — `drawn: is_solid` states itself, and
twenty-one copies of one sentence is noise.

**The shared helpers exposing a solidity boolean in a signature were left
alone** — `registry_declaring(&[(&str, bool)])` at
`crates/mc-sim/tests/support/volume.rs`, `crates/mc-world/tests/common/mod.rs`,
`crates/mc-world/tests/mesh_common/mod.rs`, and
`registry_declaring_all(names, solid)` at
`crates/mc-world/tests/save_resolution.rs`. Widening them in Phase 1 would be
speculative: nothing in this phase can state a fixture where the three differ
from solidity and observe anything. Phase 2 is where the mesher needs one, and
that is where the widening belongs.

## One test file was split, and why

`crates/mc-client/tests/documented_refusals.rs` stood at **596 non-blank lines
against the gate's 600** (`scripts/sdd-gate.ps1` counts non-blank lines via
`Measure-Object -Line`, so a blank line is free and a code line is not). Growing
`FIELDS_IN_THE_ORDER_THE_GUIDE_STATES` to nine alone costs three, leaving one.
The recogniser, the page locations, the guide's field order and the readings over
them therefore moved to `crates/mc-client/tests/support/quoted_refusals.rs`,
shared by both binaries that hold a page against a run — which also removes the
second copy of the recogniser that a new binary would otherwise have needed.
`documented_refusals.rs` is now 536 non-blank lines and all six of its tests
still pass unchanged.

## Phase 2 — The mesher decides by what is declared drawn

Fourteen scenarios: FR-2.1-S1, FR-2.1-S2, FR-2.2-S1..S4, FR-2.3-S1..S4,
FR-2.4-S1..S3, FR-2.7-S1.

**FR-2.7 and FR-2.4 were written first**, per `architecture.md` §Risks: they are
the two that cannot pass vacuously, and green anywhere else in this phase means
nothing until they are red for the right reason first.

| Scenario | File | Test |
|---|---|---|
| FR-2.7-S1 | `crates/mc-world/tests/section_drawnness.rs` | `a_cell_holding_a_solid_undrawn_block_is_reported_solid_and_not_drawn` |
| FR-2.4-S1 | `crates/mc-world/tests/mesh_declared_drawnness.rs` | `solid_undrawn_rock_shows_nothing_however_its_neighbours_are_declared` |
| FR-2.4-S2 | `crates/mc-world/tests/mesh_declared_drawnness.rs` | `a_drawn_voxel_beside_solid_undrawn_rock_shows_every_side_facing_empty_space` |
| FR-2.4-S3 | `crates/mc-world/tests/mesh_declared_drawnness.rs` | `removing_the_solid_undrawn_neighbour_is_what_makes_the_face_toward_it_appear` |
| FR-2.1-S1 | `crates/mc-world/tests/mesh_declared_drawnness.rs` | `a_voxel_declared_drawn_and_not_solid_shows_all_six_of_its_sides` |
| FR-2.1-S2 | `crates/mc-world/tests/mesh_declared_drawnness.rs` | `a_voxel_declared_solid_and_not_drawn_shows_none_of_its_sides` |
| FR-2.2-S1 | `crates/mc-world/tests/mesh_declared_occlusion.rs` | `a_solid_neighbour_that_does_not_occlude_leaves_the_face_toward_it_showing` |
| FR-2.2-S2 | `crates/mc-world/tests/mesh_declared_occlusion.rs` | `a_neighbour_that_occludes_without_being_solid_hides_the_face_toward_it` |
| FR-2.2-S3 | `crates/mc-world/tests/mesh_declared_occlusion.rs` | `a_neighbouring_section_holding_a_non_occluding_block_leaves_the_shared_face_showing` |
| FR-2.2-S4 | `crates/mc-world/tests/mesh_declared_occlusion.rs` | `a_boundary_with_no_neighbouring_section_supplied_shows_its_face` |
| FR-2.3-S1 | `crates/mc-world/tests/mesh_same_kind.rs` | `two_cells_side_by_side_holding_one_drawn_block_show_no_face_between_them` |
| FR-2.3-S2 | `crates/mc-world/tests/mesh_same_kind.rs` | `two_cells_stacked_holding_one_drawn_block_show_no_face_between_them` |
| FR-2.3-S3 | `crates/mc-world/tests/mesh_same_kind.rs` | `two_cells_in_neighbouring_sections_holding_one_drawn_block_show_no_face_across_it` |
| FR-2.3-S4 | `crates/mc-world/tests/mesh_same_kind.rs` | `two_cells_holding_two_different_drawn_blocks_each_show_their_face_between_them` |

### One skeleton reddens all fourteen, and that is a property of the tests rather than luck

`tasks.md`'s Phase 2 preamble and the test author's brief both expect **two**
skeletons — an emit-nothing one for the scenarios asserting no face, an
over-eager one for the scenarios asserting a face — on the reasoning
`testing.md` §1 gives. **Neither was built, and the reason is worth recording
because it is a correction rather than a shortcut.**

Every fixture in this phase holds a block of *both* awkward kinds at once: one
declared drawn and not solid, and one declared solid and not drawn. So the
unmodified tree behaves as an emit-nothing implementation for the first kind and
an over-eager one for the second **in the same run**, and one skeleton — `HEAD`
at `ef455f8`, with `Section::is_drawn_at` the only thing missing — reddens all of
them.

What makes that sufficient rather than convenient is structural: **no test in
this phase is a bare absence assertion.** Each scenario that asserts nothing is
emitted is written either as a complete enumerated face list that also has to
account for a drawn control voxel, or as a with-and-without pair:

| Scenario asserting no face | The form that keeps it falsifiable |
|---|---|
| FR-2.1-S2 | complete list = the six sides of a *second*, drawn voxel in the same section |
| FR-2.4-S1 | complete list = exactly one quad, from the one drawn voxel in 4 096 of undrawn rock |
| FR-2.4-S3 | the pair `(with the neighbour, without it)` = `([], [one face])` |
| FR-2.2-S2 | complete list = five sides present, the culled sixth absent from among them |
| FR-2.3-S1, S2 | complete list = six quads whose extents also pin the merge axes |
| FR-2.3-S3 | complete list = five quads, the sixth absent |

An emit-nothing implementation fails every row of that table on the faces it
owes. That is the second skeleton's job done in the shape of the assertion
instead of in a second tree, which is where `testing.md` §2 would rather have
it.

### The failing output, with the invocation beside every count

**Why `cargo test` per binary rather than `cargo nextest --no-fail-fast`.**
`nextest` builds every test target in the package before it runs anything, and
`section_drawnness.rs` does not compile until `Section::is_drawn_at` exists — so
`cargo nextest run -p mc-world -E 'binary(mesh_declared_drawnness)'` dies at
`error[E0599]: no method named is_drawn_at` and starts nothing at all. That is a
measurement, not a preference. `cargo test --test <name>` builds one target, and
runs and reports every test in it, so none of these counts can be a cancelled
run — there is no `N/M` form here to check for.

| Invocation | Result |
|---|---|
| `cargo test -p mc-world --test mesh_declared_drawnness` | `running 5 tests` → `0 passed; 5 failed` |
| `cargo test -p mc-world --test mesh_declared_occlusion` | `running 4 tests` → `0 passed; 4 failed` |
| `cargo test -p mc-world --test mesh_same_kind` | `running 4 tests` → `0 passed; 4 failed` |
| each of the three, piped to `grep -c` for the assertion-failure line | `5`, `4`, `4` |

**All thirteen are assertion failures** — not fixture-guard refusals and not
compile errors — so every one of them ran its comparison and printed both sides.

### An empty-mesh guard ahead of a non-empty expectation is a loud instrument reporting the wrong thing

One test reached its RED the wrong way first, and it is the
absent-instrument-versus-clean-instrument family with the sign flipped: not a
silence that reads as a pass, but a **failure that reads as the wrong failure**.

`a_voxel_declared_drawn_and_not_solid_shows_all_six_of_its_sides` initially read
`faces(some_quads(&mesh)?)`. `some_quads` refuses a mesh holding no quads, so
against a tree where the drawn block emits nothing the guard fired first and the
comparison never ran at all:

```
Error: "this section holds solid voxels with nothing solid beside them, so its
mesh must hold quads; every assertion that reads them is vacuous on an empty mesh"
```

That is red, and it is red for the right *reason*, and it still cost the one thing
the RED is displayed for: neither side of the comparison was printed, so nothing
confirmed the six expected faces were the six the scenario wants.

**The rule.** The guard can only help where an empty actual would *satisfy* the
comparison. Where the expectation is a non-empty literal it cannot: an empty
actual already fails against six expected elements, so the guard's only effect is
to replace an assertion failure that shows both sides with an error that shows
neither.

**Where it was dropped:** all four uses in
`crates/mc-world/tests/mesh_declared_drawnness.rs`, every one of which compares
against a non-empty literal list. The other three files of this phase never used
it — `mesh_same_kind.rs` and `mesh_declared_occlusion.rs` compare complete
enumerated lists, and FR-2.4-S3's pair has a non-empty half.

**Where it is load-bearing, and why** — the pre-existing sites, read rather than
assumed, and left exactly as they are:

| Site | Why an empty mesh would satisfy the comparison |
|---|---|
| `mesh_determinism.rs:166`, `:190`, `:214` | each compares one mesh against **another mesh**; two empty meshes are equal |
| `mesh_errors.rs:266` | same shape — the mesh against all blocks versus the mesh against fewer |
| `mesh_fixture_scale.rs:176` | `quads.len() * 2 <= visible`; no quads at all is comfortably under any ceiling |

**A deferred observation rather than a change** (`tasks.md` §Notes, scope guard):
the five remaining sites — `mesh_neighbours.rs:169`, `:229`, `:249`, `:274`,
`:298` — all compare against non-empty literals, so the guard is redundant there
in exactly the way it was redundant in mine. It is harmless until one of those
tests goes red on an empty mesh, at which point it hides both sides of the
comparison the way it hid mine. Not this phase's scenarios and not touched.

### FR-2.7-S1's RED, measured against the stub

`cargo test -p mc-world --test section_drawnness` → `running 1 test` →
`0 passed; 1 failed`, an assertion failure and not the compile error the missing
method gave before:

```
  left: (true, true, false, false)
 right: (true, false, false, true)
```

The left side is the default-equals-`solid` trap itself, failing on the value.
Predicted in this file before the stub existed and measured to the digit
afterwards, which is the point of the ruling that this RED not be a compile
error: a section answering drawnness from solidity gets both cells' second answer
wrong and neither of the first ones, and only a comparison that ran can say so.

### The whole-workspace reading, and which tree it was taken on

**Stated first, because `a7be86e` is not the tree these were read on, and neither
is any commit.** The tests commit does not compile — `Section::is_drawn_at` does
not exist in it — so no lint and no workspace run can read `a7be86e` at all, and a
reading that did not say which tree it came from would be an absent instrument
dressed as a clean one.

Every reading below was taken on `a7be86e` plus the implementation's
deliberately-less `is_drawn_at`, whose whole body was
`self.is_solid_at(pos, registry)`. **That tree object was never committed and is
recoverable from nothing but this record.** `git log --all -S "self.is_solid_at(pos,
registry)" -- crates/mc-world/src/section/mod.rs` lists no commit. The commit that
followed, `99edf1b`, carries the *finished* read — `Ok(registry.resolve(name)?.drawn)`
— so **these readings are not quotable against it**, and FR-2.7-S1 is expected
green there rather than red.

**A correction to an earlier revision of this paragraph, which claimed they were.**
The check offered as proof was
`git diff 99edf1b --stat -- crates content docs scripts tools Cargo.toml` printing
nothing. It did print nothing, and it was worthless for the purpose: it ran *after*
the implementation had already replaced the stub with the real read and committed
it, so it compared `99edf1b` against a working tree that by then matched
`99edf1b` — and said nothing whatever about the tree the readings had come from
half an hour earlier. **Phase 1's rule is that a reading is a statement about a
tree object rather than about a commit; the failure mode this adds is checking
tree equality at a later moment than the reading and treating the answer as
retroactive.** An observation of a shared tree ages exactly as fast as anybody
else's, and that includes the observation being used to validate another one.

What survives the correction, and why:

- **The `1409 / 1395 / 14` count is sound as a stub-tree reading**, and
  internally dated by its own content: the run reports `section_drawnness` FAILED,
  which the finished read cannot produce. So the stub was still in place when it
  ran.
- **`clippy` exit 0 and `fmt --check` exit 0 describe the stub tree too**, for the
  same reason — both ran before that workspace run. Neither is a statement about
  `99edf1b`, and neither is claimed as one.
- **The `scene_contract`, `replay_world` and golden readings hold either way.**
  None of those tests reads drawnness: the mesher was still deciding on solidity
  at that point, and the shipped water declares nothing until Phase 3. They are
  the same answer against the stub and against the finished read.

| Invocation | Result |
|---|---|
| `cargo nextest run --workspace --no-fail-fast` | `1409 tests run: 1395 passed, 14 failed, 1 skipped` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **exit 0**, no diagnostics |
| `cargo fmt --all -- --check` | **exit 0** |

`1409 tests run` carries **no slash**, so nothing was cancelled and the half of
the claim that says *nothing else moved* is an observation rather than a statement
about tests that never started.

**The count is derived rather than snapshotted.** Phase 1 closed at 1394 run /
1394 passed / 1 skipped. This phase adds fifteen tests — the fourteen scenarios
plus the one additional-coverage test — so 1394 + 15 = **1409**, and 1394 + 1
(the additional-coverage test, green from the start) = **1395 passed**, leaving
exactly the fourteen scenario tests red. Every arm of that arithmetic is fixed
before the run, so a wrong number in any of them is visible rather than absorbed.

**The three things this phase must not move, checked individually and green,
untouched:**

| Invocation | Result |
|---|---|
| `cargo nextest run -p mc-sim -E 'binary(scene_contract) + binary(replay_world)' --no-fail-fast` | `13 tests run: 13 passed` |
| `cargo nextest run -p mc-client -E 'binary(terrain_goldens) + binary(hud_goldens)' --no-fail-fast` | `3 tests run: 3 passed` |

Among them `the_meshed_quad_count_matches_the_committed_scene_contract_snapshot`
(the committed `SCENE_QUAD_COUNT`),
`no_quad_of_the_meshed_replay_names_the_block_that_fills_its_sea`, and
`every_declared_capture_matches_the_golden_committed_for_it` over the four
committed `…-r1` directories. `git status --short` names none of those files, so
they are green on their committed numbers with nothing about them edited — which
is this phase's own evidence that the widening is behaviour-preserving, and it is
worth more than any assertion a test author could add.

### The widening: by addition, so that none of the 26 call sites moved

`registry_declaring(blocks: &[(&str, bool)])` **keeps its signature** and now
delegates. What was added to `crates/mc-world/tests/mesh_common/mod.rs`:

- `Declaration { solid, drawn, occludes }` and `Declaration::like_solidity`,
  which is what a fixture written before the three were separable means.
- `registry_of_declarations(&[(&str, Declaration)])` — the one place a meshing
  fixture's `BlockDefinition` is built, so the one-answer and three-answer routes
  cannot drift.
- `DRAWN_ONLY`, `SOLID_AND_OCCLUDING`, `SOLID_ONLY`, `OCCLUDING_ONLY` and the
  five block names `HAZE`, `MURK`, `GHOST`, `MIST`, `SHROUD`. Named declarations
  rather than struct literals at every call site: a scenario's wording is one hop
  away, and in exchange FR-2.3-S3 gets a block that means the same thing in two
  sections by construction rather than by two literals agreeing.
- `every_side_of`, `every_side_of_but`, `named_faces`, `section_of_nothing_but`.

`section_of_nothing_but` exists because **`section_holding` cannot express an
empty cell** — it parses every palette entry as a name — and four scenarios say
"empty space". Standing a block declared to show nothing in for genuine
`Contents::Empty` would leave those scenarios satisfied by a mesher that never
looked at emptiness at all, and it would not reach the key-0 seeding.

**Measured, behaviour-preserving.** Eight test binaries link `mesh_common`:
`cargo test -p mc-world --test meshing --test mesh_budget --test mesh_determinism
--test mesh_errors --test mesh_fixture_scale --test mesh_merging --test
mesh_neighbours --test mesh_properties` → `10, 7, 3, 7, 5, 5, 8, 3` = **48 tests,
48 passed, 0 failed**, of which 47 are pre-existing and one is the new
additional-coverage test below. Every one of the 26 `registry_declaring` call
sites still means what it meant.

**What was left alone, and why.** The conductor's brief said "in all three
copies" and then corrected itself; this is what was actually done.

- `crates/mc-world/tests/common/mod.rs` — **no Phase 2 reader.** FR-2.7's test
  links `mesh_common` instead, because that is the only module in this crate that
  can build a registry where solidity and drawnness disagree, and putting the
  scenario there costs one file rather than a second `Declaration` duplicated
  into a sibling module.
- `crates/mc-sim/tests/support/volume.rs` — Phase 4, where `targeted` gives it a
  reader. Widening it here would be the speculative move Phase 1 correctly
  declined.
- `registry_declaring_all` at `crates/mc-world/tests/save_resolution.rs:117` — no
  Phase 2 reader either.

**One thing for whoever widens it next.** `mesh_common/mod.rs` is now **490
non-blank lines against the gate's 600**, up from 326. Phase 4 needs a
`targetable` answer in the same module, and there are 110 lines of headroom
rather than 274.

### Additional coverage

- `crates/mc-world/tests/mesh_errors.rs` ·
  `a_section_and_a_neighbour_both_holding_something_unresolvable_refuse_by_the_sections_own`
  — catches a shared key table resolved boundaries-first. `tasks.md` T07 calls
  the existing `UnresolvedBlock` / `UnresolvedNeighbourBlock` scenarios "the
  cheapest instrument that can see" a wrong resolution order. **They cannot see
  it, and that was verified by reading all six**: three supply
  `Neighbours::none()` and three give a wholly resolvable meshed section, so not
  one of them holds an unresolvable block on *both* sides. `sweep.rs:71-74`
  states the precedence as a deliberate decision and adds, in its own words, that
  "the scenarios leave that open" — the code documents its own gap. This test
  closes it, and its fixture excludes a second wrong order too: the section's own
  orphan sits at linear index 4 073 and the neighbour's at 3 938, so a resolver
  reporting whichever came first in linear order also names the wrong one. Green
  from the start, because the order is already right — which is why the mutation
  below is owed rather than optional.

### Mutations run, and what each proved

Both were the implementation's, run on the GREEN tree at `9a09a08` inside an
announced window, applied by hand and reverted by **re-editing the line** — never
`git checkout` — with `git diff --exit-code` clean between them and after.

**Both outcomes were predicted in writing before either was run**, and both
matched exactly, including which test would stay green and why. Prediction first
then measurement is worth more than either alone: an expectation written after the
fact cannot be wrong, and a mutation table assembled that way records only that
somebody was unsurprised.

| Mutation | Invocation | Result |
|---|---|---|
| **M5** — the third clause dropped from `visible_face`: `occludes(beyond) \|\| beyond == key` cut back to `occludes(beyond)` | `cargo nextest run -p mc-world --no-fail-fast` → `354 tests run: 351 passed, 3 failed` · `cargo nextest run --workspace --no-fail-fast` → `1409 tests run: 1406 passed, 3 failed, 1 skipped` | Exactly FR-2.3-S1, S2 and S3, workspace-wide and nothing else. **FR-2.3-S4 stayed green**, which is the control doing its job: its two cells hold two *different* drawn non-occluding blocks, so the third clause was never what culled them. |
| **M6** — the resolution order swapped, boundaries keyed before the meshed section | `cargo nextest run --workspace --no-fail-fast` → `1409 tests run: 1408 passed, 1 failed, 1 skipped` · `cargo nextest run -p mc-world -E 'binary(mesh_errors)' --no-fail-fast` → `7 tests run: 6 passed, 1 failed` | `a_section_and_a_neighbour_both_holding_something_unresolvable_refuse_by_the_sections_own` **alone**, workspace-wide, and for the right reason (below). All six pre-existing refusal scenarios printed **PASS** by name. |

Both counts carry **no slash**, so the half of each claim that says *nothing else
moved* is an observation rather than a statement about tests that never started.

**M6's failure text rules out both wrong orders at once**, which is what the
fixture was built for:

```
Error: "expected a refusal naming a block and its voxel, got
  UnresolvedNeighbourBlock { name: BlockName(NamespacedId("example:orphan_next_door")),
  facing: NegZ, position: LocalPos { x: 2, y: 6, z: 15 } }"
```

It named the neighbour's block at (2, 6, 15) — linear 3 938 — where the test
demands the section's own at (9, 14, 15) — linear 4 073. So the same failure
falsifies *resolve the boundaries first* and *resolve both and report the lower
linear index*, because both name that identical wrong block. And the six
pre-existing scenarios passing under it is the measured form of the claim that
made this test necessary: `tasks.md` T07 called them "the cheapest instrument that
can see" a wrong order, and they saw nothing. **The guard is no longer untested.**

### M5 run workspace-wide turned a derivation into an observation

The reason for the wider selection was the ordinary "nothing else moved" check,
and it bought something better. `scene_contract`, `replay_world` and all four
committed `…-r1` goldens **stayed green under a mesher with the engine rule
deleted**.

That is the reduction argument the entire phase rests on — *while the shipped
water still declares nothing, every shipped block has `drawn == occludes ==
solid`, so the third clause only ever fires where `!occludes(beyond)` already
held, and two such cells cannot be the same block* — and until this run it had
only ever been **derived, by reading the predicate**. It is now measured: the
clause genuinely never fires on shipped content, because `occludes(beyond)` culls
first every time.

Worth keeping in view for Phase 3, which is where that stops being true: the
moment `content/base/blocks/water.luau` states `drawn = true`, the shipped sea
becomes exactly the case where the first two clauses hold and the third decides —
so this measurement is also the last point at which deleting the rule costs
nothing. If a later phase repeats M5 and the goldens move, that is the rule
starting to matter rather than a regression.

### Verified green independently

`cargo nextest run -p mc-world -E 'binary(mesh_declared_drawnness) +
binary(mesh_declared_occlusion) + binary(mesh_same_kind) + binary(section_drawnness)
+ binary(mesh_errors)' --no-fail-fast` → **`21 tests run: 21 passed, 0 skipped`**,
each named in the output: the fourteen scenario tests, the additional-coverage
test, and the six pre-existing refusal scenarios it sits beside. Run by the test
author on the shut tree rather than taken from the implementation's report, since
the tests are the test author's to declare green.
