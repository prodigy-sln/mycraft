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

## Phase 3 — The shipped game draws its sea, and the golden set re-shoots once

Twelve scenarios: FR-1.3-S1, FR-1.3-S2, FR-2.5-S1, FR-2.6-S1..S3, FR-6.1-S1,
FR-6.1-S2, FR-6.2-S1..S3, FR-7.1-S1.

| Scenario | File | Test |
|---|---|---|
| FR-1.3-S1 | `crates/mc-client/tests/shipped_blocks_are_declared_in_luau.rs` | `the_shipped_content_declares_four_blocks_with_water_alone_soft_unbreakable_and_seen_through` |
| FR-1.3-S2 | `crates/mc-client/tests/shipped_blocks_are_declared_in_luau.rs` | the same test — **one ordered comparison of all four blocks; see below** |
| FR-2.5-S1 | `crates/mc-sim/tests/scene_contract.rs` | `the_surface_shows_one_upward_face_per_column_with_the_landmark_capping_one_and_the_sea_over_the_rest` |
| FR-2.6-S1 | `crates/mc-sim/tests/replay_world.rs` | `the_meshed_replay_shows_the_block_that_fills_its_sea` |
| FR-2.6-S2 | `crates/mc-sim/tests/scene_contract.rs` | `every_blocks_meshed_area_equals_the_independent_walks_area_for_that_block` |
| FR-2.6-S3 | `crates/mc-sim/tests/scene_contract.rs` | `the_meshed_quad_count_matches_the_committed_scene_contract_snapshot` — **the scenario was amended after pass-1 validation; the test is unchanged. See below** |
| FR-6.1-S1 | `crates/mc-render/tests/golden_inventory.rs` | `the_committed_goldens_are_exactly_the_directories_the_current_revision_declares` — **an existing test, judged sufficient; measurement below** |
| FR-6.1-S2 | `crates/mc-render/tests/golden_inventory.rs` | `a_revision_whose_goldens_were_never_captured_fails_naming_the_path_it_looked_for` — **an existing test, judged sufficient; measurement below** |
| FR-6.2-S1 | `crates/mc-client/tests/replay_oracle.rs` | `every_declared_sample_of_every_judged_frame_is_sky_or_a_block_the_world_places_and_some_is_sea` |
| FR-6.2-S2 | `crates/mc-client/tests/the_judge_marches_through_a_block_nothing_draws.rs` | `the_march_passes_through_an_obstacle_nothing_draws_and_reports_what_stands_beyond_it` |
| FR-6.2-S3 | `crates/mc-client/tests/replay_oracle.rs` | `every_sample_a_marched_ray_calls_terrain_is_drawn_as_something_other_than_sky` |
| FR-7.1-S1 | `crates/mc-client/tests/the_sea_the_camera_sees_is_the_water_layer.rs` | `every_declared_capture_draws_the_sea_wherever_a_marched_ray_says_the_sea_is` |

### Why FR-1.3-S1 and FR-1.3-S2 share one test rather than getting one each

The two scenarios cut the same reading in half — water's six facts and the other
three blocks' four — and the reading is **one ordered comparison of the whole
shipped set against a hand-written table**. Splitting it would produce either two
identical whole-list comparisons, which is the same claim proved twice through one
code path, or two *filtered* comparisons, which is the shape this project has been
bitten by twice: a hand-maintained list compared by filtering cannot see an extra
member. A fifth shipped block, a missing one, a reordering and a changed field are
four distinct failures of the single comparison, and none of them survives it.

Both sides are a `Reading` struct rather than a formatted line, so the failure
output names the field that differs instead of leaving a reader to diff two
sentences by eye — which is what the RED below shows.

### FR-2.6-S3 asked for a quantity that does not exist, and the scenario was amended rather than the test

**Pass-1 validation returned one scenario gap and no severity finding anywhere.**
The gap was FR-2.6-S3, which required *"a quad count derived from an independent
walk of the same world rather than one snapshotted from a run of the mesher"*, and
was mapped to a test that compares against the committed `SCENE_QUAD_COUNT` — a
number whose own doc comment says it is *"a snapshot, deliberately not an oracle:
it verifies nothing"*. The scenario said derived and the test snapshots, so the
mapping was unmet. The reviewer added that no test derives a quad count, because a
per-voxel walk cannot see the mesher's merge boundaries without re-implementing
them.

**The scenario was amended on 2026-08-23. The test is untouched.** The reason is
not that deriving the count was hard:

**The count is fixed by the sweep's loop nesting, not by the geometry.** The
mesher's own header states it — the output is the scanline-greedy decomposition
and *deliberately not* the fewest rectangles covering the same faces, "only the
first of them is a *single* answer — which is what makes the output comparable at
all". Grow a run along the primary axis, extend it along the secondary while a
whole row matches. A merger growing columns before rows is equally correct and
reports a different count for identical geometry. An independent walk agreeing
with the mesher would have to repeat those ordering choices, at which point it is
a copy of its subject. **The quantity the scenario asked for is not
well-defined**, which is a different thing from expensive.

#### The guard proposed for the residual was wrong, and this is what replaced it

The conductor's brief for this amendment proposed stating the guard as *areas by
independent derivation, merge shape by the goldens, quad count as the tripwire
that makes the golden failure land here first*, on the reasoning that a
merge-shape change preserving summed area still moves rendered pixels. **That is
false, and it was measured rather than argued:**

- A packed terrain vertex carries `x, y, z, facing, layer, section` and **no field
  derived from a quad's extent**.
- Texture coordinates come from the corner's own section-local position, and the
  terrain sampler is `AddressMode::Repeat` on all three axes. The shader's own doc
  comment says what that buys: *"a face merged across four blocks shows the
  texture four times rather than stretched once"*.
- So one 4×1 quad and four 1×1 quads emit the same texels at the same depths. **A
  re-partition of the same visible faces is pixel-neutral by design**, and goldens
  are compared perceptually against a disagreement budget besides, so they could
  not see the difference if there were one.

The codebase already knew this and said so in a failure message:
`no_two_quads_cover_the_same_face` explains that two quads overlapping is
"invisible in a count of covered area and invisible in a rendered frame — the
second face is drawn in the same place as the first".

**What actually closes the residual is the exact-partition property, and it is
stronger than the goldens would have been.**
`crates/mc-world/tests/mesh_properties.rs` holds three proptests over randomly
generated section contents and all six surroundings: covered ⊆ visible, visible ⊆
covered, and no face covered twice. Together they pin the quads to an exact
partition of the visible-face set **per face and per position** — which catches a
face relocated with its area preserved, something no area sum can see. Run on the
tree this entry was written against:

```
cargo nextest run -p mc-world --test mesh_properties --no-fail-fast
3 tests run: 3 passed, 0 skipped
```

with `every_face_an_independent_scan_finds_visible_is_covered_by_a_quad`,
`no_face_a_quad_covers_has_a_solid_voxel_against_it` and
`no_two_quads_cover_the_same_face` each named PASS. A complete run rather than a
cancelled one — a bare `3 tests run`, no slash.

**So a merge change is one of two things and there is no third.** It keeps the
partition, in which case it is area-neutral and pixel-neutral by construction and
observable only as a count — not a defect, but a strategy change somebody owes an
explanation for, which is what the tripwire extracts. Or it breaks the partition,
in which case those three redden and name the face. The four instruments and their
four reaches are recorded in `spec.md` at FR-2.6.

#### Two sites said something now known to be false, and both were repaired in this amendment

`SCENE_QUAD_COUNT`'s doc comment and the failure message of the test mapped to
FR-2.6-S3 both asserted that a moved quad count means every committed golden is a
golden of a different scene, and instructed the reader to bump the scene revision,
delete the golden directories and re-shoot. **True of the change they were written
for** — ambient occlusion, which is per-vertex and therefore does make merge shape
visible — and **false of a pixel-neutral re-partition**, for which that instruction
churns the whole golden set to reproduce identical images. Both were made
conditional rather than deleted, alongside this amendment: the remedy is stated
for the case that needs it, and a re-partition is named as the case that does not. No golden was touched and `SCENE_REVISION` was not bumped, because
nothing here moves a pixel or a count.

### FR-6.1-S1 and FR-6.1-S2 are closed by tests that already existed, and here are the measurements

"An existing test already covers it" is only an answer with the measurement
attached, so both were made to fail on purpose.

**FR-6.1-S1** — a stale directory. `mkdir crates/mc-render/goldens/player-walk-t000-r0`,
then `cargo nextest run -p mc-render --test golden_inventory --no-fail-fast` gives
`3 tests run: 2 passed, 1 failed`, the failure being
`the_committed_goldens_are_exactly_the_directories_the_current_revision_declares`
reporting that the golden root has to hold exactly the captures revision `r1`
declares. The directory was then removed and the four committed captures confirmed
still present. So a directory of a revision nobody declares fails the gate, which
is the half of the scenario a missing-golden check cannot see.

**FR-6.1-S2** needs no mutation to demonstrate: its test *constructs* the
uncaptured revision, and asserts three things at once — the failure is a
`MissingGolden`, its rendered text contains the path, and nothing was minted on
the way past. The third is the one that matters and no other test makes it.

Writing a second test for either would re-prove the same claim through the same
code path.

### `SECOND_REVISION` collided with the revision this phase moves to, and the fix is structural

`crates/mc-render/tests/golden_inventory.rs` declared
`const SECOND_REVISION: &str = "r2"`, with a doc comment saying `r2` was "what the
day after ambient occlusion looks like". T13 makes `SCENE_REVISION` **`"r2"`**.
`the_capture_ids_of_a_second_scene_revision_all_carry_it_and_none_repeats_the_first`
would then have compared `declared_capture_ids("r2")` against itself, found all
four ids repeated where it expects none, and failed for a reason with nothing to
do with this spec.

**Replaced by a function deriving it from the current revision** —
`a_revision_nothing_was_captured_for()` answers `format!("{SCENE_REVISION}_uncaptured")`
— which cannot collide however far the revision advances, and which no committed
directory can ever be named. The suffix is spelled in the alphabet a capture id
admits, so the scenario stays about a *missing* golden rather than a rejected
name. It landed with the tests rather than waiting for T13, because it is correct
at `r1` as well: `r1_uncaptured` is neither `r1` nor a committed directory. All
three tests in that binary pass on the tree as it stands.

### The rename of the judge's solidity question changed what one other caller was asking, and the answer is drawnness

`Voxels::is_solid` became `is_drawn` and `first_solid_face` became
`first_drawn_face`. Three call sites, and they were **not** all asking one
question:

- `the_grass_top_the_camera_sees_is_its_baked_image.rs` asks "what block face is
  this pixel of". Drawnness, plainly; the rename is right for it.
- `support/faces.rs`'s `reachable` asked whether either of the two cells between
  the eye and a face was solid, and described what it was avoiding as *"the camera
  inside terrain"*. `first_exposed` reaches it too.

**The decision, taken deliberately rather than inherited from the rename:
`reachable` asks drawnness.** Its purpose is stated in its own second sentence —
the frame is otherwise of whatever is nearer than the face the reading is about —
and what makes a frame be of something nearer is that something being **drawn**,
not its being an obstacle. The two were the same set of cells only while every
drawn block collided. They are not any more: an eye may legitimately stand in a
cell of water and still cannot read a face through one, because water's own face
toward the eye is emitted and is nearer. The reasoning is written into the
helper's own doc comment, where the next reader meets it.

**A correction to the first version of this record, which claimed drawnness was
"the stricter question — every cell solidity rejected, drawnness rejects too".
That is false, and this phase's own fixture is the counterexample.** The two
questions are **incomparable rather than nested**, and the difference runs both
ways:

- `drawn = true, solid = false` — water after T09. Drawnness **rejects** it where
  solidity admitted it. This is the tightening the reading relies on, and it is
  real.
- `drawn = false, solid = true` — **exactly the pillar in the FR-6.2-S2 fixture,
  the one 72 of 576 samples stopped at.** Solidity **rejected** it; drawnness
  **admits** it. A pose refused as blocked is now accepted with an invisible
  obstacle between the eye and the face.

And the second of those has a failure mode rather than merely being a difference.
`exposed_side_faces` uses the first of the two outward cells for a second purpose,
stated in its own doc comment: that the neighbouring cell is clear *"so the mesher
emitted that face at all"*. A block declared `drawn = false, solid = true` and
saying nothing about occlusion answers `occludes` from its solidity, so it
**occludes** — the grass face this reading was going to read is culled and never
emitted, and the reading would be taken against a face that is not in the picture.

The claim happened to hold on shipped content, because dirt, grass and stone
declare nothing about being drawn and so answer it from their solidity, leaving
water the only block where the two part. **But it held because of what today's
four declarations say, not because of any relation between the predicates** — and
the next fixture declaring an undrawn obstacle would break it silently. That is
the shape this project keeps paying for: a claim true of the content, written into
a doc comment where the next reader takes it as true of the code. The helper's doc
comment now names the limit instead, and says what closing it would take — asking
about occlusion as well, which it does not do today.

Measured rather than assumed: `a_drawn_side_keeps_its_left_to_right_order` and
`a_drawn_side_shows_turf_above_dirt`, the two readings built on that helper, pass
under both questions — the faces it finds sit at `x = 0`, far from the sea's strip
on the `+x` edge, so the change moves no pose today. It is the day the search
order or the world changes that it would have mattered silently.

### Additional coverage

Each line states what it catches. None is a scenario's own test.

- `crates/mc-client/tests/shipped_blocks_are_declared_in_luau.rs` ·
  the same ordered comparison also carries **`breakable`**, which neither FR-1.3
  scenario asks of the other three blocks. It catches a loader that came to
  default breakability from something other than its documented `true`, on the
  three blocks that state nothing about it — which no other reading of the shipped
  root makes.
- `crates/mc-client/tests/shipped_blocks_are_declared_in_luau.rs` ·
  `Launched::Unreadable` is a new arm of the launch verdict. A block the registry
  registers but will not read back used to be rendered as one *line of the list*,
  where it would have been compared against a block's expected reading and failed
  as a wrong block rather than as a registry that could not answer. It now stops
  the reading and says so.
- `crates/mc-client/tests/the_sea_the_camera_sees_is_the_water_layer.rs` ·
  `require_nothing_else_is_that_colour` — the lower half of the tolerance's
  bracket, asserted on every run instead of quoted in a header. It catches a
  palette change that brought the sky or any other layer within the tolerance of
  water's, which would turn the reading into one that cannot tell them apart while
  it went on passing. Green from the start, deliberately: it is a control.
- `crates/mc-client/tests/support/oracle.rs` · `predicted_terrain` is now the
  samples `sighted_samples` does not call sky, rather than a second march of its
  own. Not a test, but it removes the one way FR-6.2-S1 and FR-6.2-S3 could have
  come to disagree about which samples are terrain — two marches would have been
  two answers, and the day they parted the difference would have read as a
  renderer fault.

### How to tell a cancelled run from a complete one, before any count below

`cargo nextest`'s summary is `N tests run` when the run completed and `N/M tests
run` when it was cancelled at the first failure. **Every count in this section is
the bare form and every invocation carries `--no-fail-fast`**; a slashed count
appears nowhere below, and the invocation is quoted beside each one.

### The failing output, with the invocation beside every count

The whole workspace, on the tree these tests were written on:

```
cargo nextest run --workspace --no-fail-fast
     Summary [  45.600s] 1413 tests run: 1408 passed, 5 failed, 1 skipped
        FAIL mc-client::replay_oracle every_declared_sample_of_every_judged_frame_is_sky_or_a_block_the_world_places_and_some_is_sea
        FAIL mc-client::shipped_blocks_are_declared_in_luau the_shipped_content_declares_four_blocks_with_water_alone_soft_unbreakable_and_seen_through
        FAIL mc-client::the_sea_the_camera_sees_is_the_water_layer every_declared_capture_draws_the_sea_wherever_a_marched_ray_says_the_sea_is
        FAIL mc-sim::replay_world the_meshed_replay_shows_the_block_that_fills_its_sea
        FAIL mc-sim::scene_contract the_surface_shows_one_upward_face_per_column_with_the_landmark_capping_one_and_the_sea_over_the_rest
```

**Five failures and no sixth**, so nothing outside this phase's scenarios moved —
which is the reading that says the judge's rename, the walk's third question and
the changed `reachable` cost no test elsewhere. Every one of the five is an
**assertion** failure with both sides printed; none is a compile error and none is
a fixture guard firing.

The waves the brief predicted, against what was measured:

| Wave | Predicted | Measured |
|---|---|---|
| Red before T09 | FR-1.3-S1, FR-1.3-S2, FR-2.6-S1, FR-6.2-S2 | FR-1.3 and FR-2.6-S1 red as predicted. **FR-2.5-S1 joined this wave** — its expectation names water's upward area, so it is red before T09 rather than at it. FR-6.2-S2 is green here for a reason of ownership, below. |
| Red the moment T09 lands | FR-2.5-S1, FR-2.6-S2, FR-2.6-S3 | FR-2.6-S2 and FR-2.6-S3 **green now, as predicted** — see the vacuity note below, which is why that green is not evidence. |
| Not satisfiable until T12 | FR-6.2-S1, FR-6.2-S3, FR-7.1-S1 | FR-6.2-S1 and FR-7.1-S1 red as predicted. **FR-6.2-S3 is green now** — a correction to `tasks.md`, below. |

The four assertion failures, quoted:

```
cargo nextest run -p mc-sim --test scene_contract --test replay_world --no-fail-fast
     Summary 13 tests run: 11 passed, 2 failed, 0 skipped

the_surface_shows_one_upward_face_per_column_with_the_landmark_capping_one_and_the_sea_over_the_rest
  left: {base:grass: 4095, base:stone: 1}
 right: {base:grass: 4095, base:stone: 1, base:water: 131}

the_meshed_replay_shows_the_block_that_fills_its_sea
  the sea covers 131 columns of the declared world and every one of them is open to
  the air above it, so the mesh owes at least that much water area — and it meshed 0.
```

The 131 is **derived in the test** from the surface heights and the declared sea
level, by `support::submerged_columns`, and touches neither the mesher nor the
walk. That it reproduces the census the architecture measured independently is
confirmation, not a copy: no figure from that document is written down anywhere in
these tests.

```
cargo nextest run -p mc-client --test shipped_blocks_are_declared_in_luau \
  --test the_judge_marches_through_a_block_nothing_draws --no-fail-fast
     Summary 4 tests run: 3 passed, 1 failed, 0 skipped

the_shipped_content_declares_four_blocks_with_water_alone_soft_unbreakable_and_seen_through
  left:  ... Reading { name: "base:water", ..., drawn: false, occludes: false, targetable: false }
 right:  ... Reading { name: "base:water", ..., drawn: true,  occludes: false, targetable: true  }
```

Water's row is the only one of the four that differs, and it differs in exactly
the two fields T09 states — `occludes` already reads `false` because it defaults
from water's `solid = false`, which is the coincidence FR-1.3 exists to stop being
load-bearing.

```
cargo nextest run -p mc-client --test replay_oracle \
  --test the_sea_the_camera_sees_is_the_water_layer \
  --test the_grass_top_the_camera_sees_is_its_baked_image --no-fail-fast
     Summary 7 tests run: 5 passed, 2 failed, 0 skipped

every_declared_sample_of_every_judged_frame_is_sky_or_a_block_the_world_places_and_some_is_sea
  left:  [{tick 0, outside [], samples 576, "the sea at no sample at all"},
          {tick 59, ..., "the sea at no sample at all"},
          {tick 119, ..., "the sea at no sample at all"}]
 right:  [... "the sea at one sample or more" x3]

every_declared_capture_draws_the_sea_wherever_a_marched_ray_says_the_sea_is
  left:  [{tick 0, "the sea at no sample at all", []}, {tick 59, ...}, {tick 119, ...}]
 right:  [... "the sea at one sample or more" x3]
```

**Both wave-three failures report the measured fact rather than a bare
disagreement**: zero water samples at all three declared capture ticks, from the
camera the simulation actually publishes. That is the architecture's own
measurement reproduced by an instrument that had not seen it, and it is a reading
that dates itself — it cannot have come from a tree where the spawn has moved.
The classification is otherwise complete and clean at every tick: no class outside
the declared five, and 576 of 576 samples classified.

### The per-tick classification as it stands, and what `PREDICTION_FLOOR` is bracketed by

Measured on this tree, through `sighted_samples` at the three judged ticks:

| Tick | Terrain predicted | Classes |
|---|---|---|
| 0 | 441 of 576 | 441 `base:grass`, 135 sky |
| 59 | 544 of 576 | 544 `base:grass`, 32 sky |
| 119 | 542 of 576 | 539 `base:grass`, 3 `base:stone`, 34 sky |

`PREDICTION_FLOOR` is **not moved**, and this is the bracket it has to sit in.
From below: above zero, since its whole job is to catch a march that collapsed —
a floor of zero is satisfied by an oracle predicting nothing, which is the one
failure the one-sided comparison beside it cannot see. From above: below the
smallest of the three counts, which is 441 at tick 0, or a correct march fails it.
It is 100, which is 341 clear of the lower of those and far above zero.

**The re-derivation for the moved spawn is T12's and is owed there**, because the
counts above are properties of where the camera stands and every one of them moves
with it. What T12 records is the same bracket measured against the new pose: the
smallest per-tick terrain count, and whether 100 still sits well inside it. The
architecture's screen says candidate poses predict 300 to 500, so it should — but
that is a screen and not a measurement, and the floor is only honest once measured.

`SAMPLE_SPACING` and `SAMPLE_ORIGIN` are **not moved** and no sample has been
relocated. If T12's spawn puts a sample within a pixel of the sea's silhouette,
the remedy is to move that sample and record the move and its reason here — never
to widen `DISAGREEMENT_BUDGET`, which is unchanged at 2, and never to add a budget
to FR-7.1-S1, which deliberately has none.

### FR-2.6-S2's equality is green before the change it is about and green after it, and that is not evidence

`every_blocks_meshed_area_equals_the_independent_walks_area_for_that_block`
compares the mesher's per-block area against the independent per-voxel walk. The
walk was rewritten in this phase from two questions to three — is this block
**drawn**, does what lies beyond it fail to **occlude**, and is what lies beyond it
a **different block**. The equality was green before that rewrite and is green
after it.

**It is green for a reason that has nothing to do with the rewrite being right.**
Every block the base game ships still has `drawn == occludes == solid`, and no two
adjacent cells hold one non-occluding block, so all three questions reduce to the
two the walk asked before. Measured rather than argued — three mutations, each one
line, each reverted by hand:

| Mutation | What was broken | Result |
|---|---|---|
| M-A | the walk asks `is_solid` instead of `drawn` | `13 tests run: 11 passed, 2 failed` — **did not bite** |
| M-B | the walk asks `is_solid` of what lies beyond instead of `occludes` | `13 tests run: 11 passed, 2 failed` — **did not bite** |
| M-C | the same-kind question deleted entirely | `13 tests run: 11 passed, 2 failed` — **did not bite** |

Invocation for all three:
`cargo nextest run -p mc-sim --test scene_contract --test replay_world --no-fail-fast`.
Baseline on the unmutated tree is the same `13 tests run: 11 passed, 2 failed`, and
the two failures are the same two in every case. Reverted by re-editing each line;
`grep` confirms `resolve(name)?.drawn` and `occludes && name != block` each stand
once in the file.

**Three mutations that do not bite are evidence about the content, not a test
gap**, and they are the sharpest statement available of why this phase exists:
the walk's three questions are *indistinguishable* from the old two on the world
as it ships today. What makes the pair meaningful is two things that arrive with
T09 — FR-2.6-S1's non-zero water area, which is the control, and the sea being the
first place where the first two questions hold and the third decides. A reviewer
meeting this equality green should read it as the phase not having landed yet, not
as the walk being confirmed.

#### The other half of this entry: the same three mutations, re-run after T09

**Neither half means anything alone, which is why they are one entry.** The
"before" above says the three questions are indistinguishable on the old content.
That is, taken by itself, evidence *against* the walk being a real
reimplementation — it is the exact shape of a judge agreeing with its subject for
a reason that has nothing to do with either being right. Only the "after" says the
walk implements three questions rather than one.

**The prediction is written down here before the measurement exists**, so the
record cannot be fitted to whatever the re-run turns out to say. It rests on a
census of the declared world taken on this tree — 178 water voxels, and their
1 068 sides classified as 201 open to air or to the world's edge, 662 meeting
another water voxel, and 205 meeting a block that occludes. The three sum to
178 × 6 exactly, which is the arithmetic check on the census itself.

| Mutation | Prediction after T09 | Why, and by how much |
|---|---|---|
| M-A · the walk asks `is_solid` instead of `drawn` | **bites** | Water is the one shipped block that is drawn without colliding, so a walk asking solidity emits none of its faces. Water's whole entry disappears and the total face area falls by **201**. |
| M-B · the walk asks `is_solid` of what lies beyond instead of `occludes` | **does not bite, and cannot** | See below. |
| M-C · the same-kind question deleted | **bites, and is the largest of the three** | The 662 water-to-water sides stop being culled, so water's area rises from 201 to **863** — the interior of the sea, which is four times everything else it draws. It is also the only clause whose deletion can move a *non*-water block by nothing at all: clause 3 is only ever reached where the neighbour does not occlude, and water is the only shipped block that does not. |

**M-B cannot bite at any point in this spec, and that is a structural fact rather
than a gap in this phase.** After T09 the four shipped declarations read
`dirt`, `grass`, `stone` at `solid = true` — so `occludes` defaults to `true` —
and `water` at `solid = false, occludes = false`. **Every shipped block therefore
has `occludes == is_solid`**, water included, so `!occludes(beyond)` and
`!is_solid(beyond)` are the same expression over the whole replay world however
the sea is declared. The walk is only ever run over shipped content, so its
`occludes` arm is indistinguishable there by construction.

That is not a hole this phase should fill by widening FR-2.6-S2. The question
"does a face cull on `occludes` rather than on `solid`" is answered by FR-2.2-S1
and FR-2.2-S2, whose fixtures declare the two apart on purpose — `occludes = false,
solid = true` and `occludes = true, solid = false` — precisely because no shipped
block does. What the entry records instead is the reach of each instrument: the
shipped-content equality can witness clauses 1 and 3, and clause 2 is witnessed
only by synthetic fixtures. **The day a shipped block declares `occludes` and
`solid` apart, M-B becomes measurable here and should be re-run.**

**The measurement, once T09 has landed:** taken, and it is the section headed
*"The other half of the walk-mutation entry, measured after T09"* at the end of
this Phase 3 record. **All three predictions held, and the two deltas landed on
201 and 863 to the unit.**

### FR-6.2-S2's green half is a test-support rename, so its RED is recorded rather than handed over

T11 — `Voxels::is_solid` to `is_drawn`, `first_solid_face` to `first_drawn_face`,
the module header's water paragraph deleted and replaced — is listed in `tasks.md`
among the implementation tasks, but every file it touches is
`crates/mc-client/tests/support/oracle.rs`, and `tasks.md`'s own Phase 3 preamble
names that file as one the test author "owns outright". So the change that turns
FR-6.2-S2's test green is the test author's, and the test cannot be handed over
red. The RED was measured instead, by mutation:

```
cargo nextest run -p mc-client --test the_judge_marches_through_a_block_nothing_draws \
  --test shipped_blocks_are_declared_in_luau \
  --test the_grass_top_the_camera_sees_is_its_baked_image \
  --test a_drawn_side_keeps_its_left_to_right_order \
  --test a_drawn_side_shows_turf_above_dirt --no-fail-fast
```

With `drawn_block` reading `is_solid` — the judge exactly as it stood before this
phase — `7 tests run: 5 passed, 2 failed`: the standing FR-1.3 failure, plus

```
the_march_passes_through_an_obstacle_nothing_draws_and_reports_what_stands_beyond_it
  left:  (true, 72)
 right:  (true, 0)
```

72 of the 576 declared samples look at the landmark pillar from the declared pose;
all 72 still stop at it when its stone is declared `drawn = false, solid = true`.
Reverted by re-editing the line, and the same binaries then report
`3 tests run: 3 passed` over the three that were exercised. **Nothing outside the
mutated call moved in either direction**, which is what says the reading is about
the judge's question and not about the fixture.

The declared pose is `eye = [16, 50, 16]` looking at `[12, 50, 12]`, and the
distance is the whole of the choice: the pillar is one column wide, so an eye 45
blocks out — the distance the derived probes observe it from — puts it under **no
sample of the declared grid at all**, measured, before the pose was moved in. The
control assertion is what reported that, which is why it is half of the
comparison rather than a comment.

### FR-6.2-S3 is green before T12, and `tasks.md` says it cannot be — a correction

`tasks.md` T12 lists FR-6.2-S3 among the scenarios "unsatisfiable until T12", and
the phase preamble repeats it. **Measured, that is wrong, and it is recorded here
as a correction rather than absorbed**, because the spec folder is archived at
completion and somebody will later cite the unsatisfiable set as evidence the
phase was ordered correctly.

`every_sample_a_marched_ray_calls_terrain_is_drawn_as_something_other_than_sky`
passes on the tree as it stands: in the three-binary client run above it is one of
the five passes, and the only failure in `replay_oracle` is the new FR-6.2-S1 test.
The reason is plain once stated: the scenario asks that every sample the judge
calls terrain is drawn as something other than sky, and it says nothing about
water. With the spawn where it is, the judge calls 441 to 544 samples terrain at
the three ticks and every one of them is grass or stone, all of which the renderer
already draws. Nothing in the assertion depends on water being in frame.

**That is a fact about how much of FR-6.2-S3 rests on the sea, and it is not a
licence to skip T12 or to weaken FR-6.2-S1.** What it does mean is that FR-6.2-S3
will not be the reading that reports a spawn move gone wrong. FR-6.2-S1 and
FR-7.1-S1 are, and both are red now and stay red until T12 lands. The one thing
FR-6.2-S3 *does* newly carry is the water samples once they exist: after T12 its
predicted-terrain list includes them, so a sea predicted but drawn as sky fails
here as well as in FR-7.1-S1 — by a different route, since one judges against the
clear colour and the other against water's own layer.

### FR-7.1-S1's tolerance, both bounds and the nearest wrong answer

Measured through `support::art::drawn_texels` and `linear_mean` — the client's own
reader, never the draw — over the shipped content root:

- **`base:water` is not one of the seven keys the shipped manifest bakes.** It has
  no PNG; its layer is the generated stand-in. Stated because the precedent this
  reading follows, `the_grass_top_the_camera_sees_is_its_baked_image.rs`, *requires*
  its key to be covered and would be about the fallback if it were not. This one is
  about the fallback by construction, and says so in its header.
- Water's layer mean is `[150, 49, 141]` over 256 texels, and **every texel sits
  within ΔE 3.71 of it** — the generator's checkerboard is one step either side of
  the declared mean, so a magnified face shows a colour at most that far off and a
  minified one converges on the mean.
- The nearest **wrong** answer is `base:stone` at **ΔE 62.40**. Then `base:dirt`
  71.49, the declared clear colour 79.38, the four grass sides 79.11 to 79.98, and
  `base:grass_top` 114.77.

So the tolerance sits anywhere in (3.71 + the sRGB round trip, 62.40) and it is
**8** — twice the texel spread, and 54 ΔE clear of every other thing one of these
pixels could be showing. Not loosened until green; it was chosen before the test
had ever run against a frame. The lower half of the bracket is asserted on every
run by `require_nothing_else_is_that_colour` rather than left to this record.

**No disagreement budget.** FR-6.2-S3's comparison allows two samples to fall the
wrong side of a silhouette; FR-7.1-S1 allows none, because the requirement says
"every sample pixel where the judge predicts water". If T12's spawn puts a
predicted water sample on the sea's edge, the remedy is to move that sample and
record it here.

### The lint, run directly, and which tree it measured

`cargo clippy --workspace --all-targets --all-features -- -D warnings` finished
with **no diagnostics**, on the tree these tests were committed from. Run by the
test author rather than inferred from a green suite, because a suite and a lint
answer different questions.

It caught four things the 1 413-test run could not, all of them in this phase's own
new code, and all of them fixed rather than allowed:

- `crates/mc-client/tests/support/reload.rs` · `Declaration::text` reached 37 lines
  against `clippy.toml`'s cap of 30 once the three appearance fields were added.
  Split into the required half and `stated_fields`, the optional half.
- `crates/mc-client/tests/support/oracle.rs` · `Result<Vec<((u32, u32), Sighted)>, ...>`
  tripped `type_complexity`. Named as `SightedSample`.
- `crates/mc-client/tests/replay_oracle.rs` · `classifications` at 32 lines, split
  into `summarised`.
- `crates/mc-client/tests/the_sea_the_camera_sees_is_the_water_layer.rs` ·
  `the_sea_in_each_capture` at 31 lines, split into `predicted_sea`.

The brief predicted the third question would push against the four-argument and
two-nesting caps in `crates/mc-sim/tests/support/oracle.rs`. It did not: the
question was added as a `shows` predicate over a three-armed `Beyond`, so the
existing `Face` pair and split helper absorbed it and no new argument or nesting
level arrived. Recorded because a prediction that did not come true is worth as
much as one that did.

### The other half of the walk-mutation entry, measured after T09

The predictions above were recorded at `bc7541a` before the declaration landed.
Re-run on `d13f734`, after T09, both spawn moves and the `r2` re-shoot. Baseline
`cargo nextest run -p mc-sim --test scene_contract --test replay_world
--no-fail-fast` → `14 tests run: 14 passed, 0 skipped`; same invocation for each
mutation; each one line, reverted by re-editing and confirmed with
`git diff --exit-code`.

| Mutation | Predicted | Measured | Verdict |
|---|---|---|---|
| M-A · `drawn` → `is_solid` | bites, water loses all 201 | `11 passed, 3 failed` — *"`base:water` meshes 201 and walks **0**"* | **as predicted, to the unit** |
| M-B · `occludes` → `is_solid` beyond | does not bite | `14 passed, 0 failed` | **as predicted** |
| M-C · same-kind question deleted | bites largest, 201 → 863 | `10 passed, 4 failed` — *"`base:water` meshes 201 and walks **863**"* | **as predicted, to the unit** |

**Both deltas landed exactly on figures written down before the measurement
existed.** 201 was the whole of water's area under M-A and 863 was the whole of
it under M-C, which is 201 + the 662 interior sides the census counted. That is
the pair this entry was split into two halves for: the before-half said the three
questions were indistinguishable on the old content, and the after-half says the
walk implements three questions rather than one.

**M-B did not bite, exactly as predicted, and the prediction is the finding.**
The team lead expected it to move the faces behind water. It cannot: after T09
the four shipped declarations are `dirt`, `grass`, `stone` at `solid = true` — so
`occludes` defaults true — and `water` at `solid = false, occludes = false`.
Every shipped block therefore has `occludes == is_solid`, so `!occludes(beyond)`
and `!is_solid(beyond)` are one expression over the whole replay world however
the sea is declared. The walk only runs over shipped content, so **that clause is
unwitnessable there by construction, not by an oversight of this phase.**

That is not a hole to close by widening FR-2.6-S2. FR-2.2-S1 and FR-2.2-S2
already answer it, and their Phase 2 fixtures declare `occludes` and `solid`
apart *on purpose* because no shipped block does. What this entry records is the
**reach of each instrument**: the shipped-content equality witnesses clauses 1
and 3, clause 2 is witnessed only by synthetic fixtures, and M-B becomes
measurable here the day a shipped block declares the two apart.

**A property of the count, which no assertion in this phase was written to check,
and which a mutation aimed at something else happened to witness.**

The property: **`WATERY_SIDES` is not a count of horizontal adjacencies.** It
decomposes into horizontal water-to-water sides and *vertical* ones, and the
vertical part is non-zero — two sides for every column the sea fills to a depth
of two or more, being the up-side of the lower cell and the down-side of the
upper. It has to be non-zero over this world by declaration: `LOWEST_SURFACE` is
32 and `SEA_LEVEL` is 34, so a column at the bottom of the band is two deep.

The evidence: M-C reddens **four** tests where M-A reddens three, and the extra
one is `the_world_floor_shows_one_downward_face_per_column`. That test sums the
walk's downward faces, and it can only move if deleting the same-kind question
un-culls a **downward** face — which only a vertically stacked pair can produce,
because every other neighbour below a water cell is the lakebed and occludes.
A `WATERY_SIDES` that counted only horizontal adjacencies would have left that
test untouched.

**So the run is what witnessed it, and the property is what is recorded.** Stated
this way round because the shape is worth more than the incident: a mutation
checking one clause of a predicate reported a fact about the *census beside it*,
which is the kind of thing only two independent instruments in the same run can
do for each other.

**The census stayed green through all three windows**, which is the design rather
than luck: the mutations change the walk and the census reads the world. Had it
reddened, that would have been a mutation reaching somewhere it should not.

### The four repairs T09 to T13 made necessary

Each was disputed by the implementer and each verdict was `test-wrong`. None was
a test that was wrong in kind — all four were hand-written expectations that
reddened on a deliberate change, which is the discipline working rather than
failing.

| Test | What went stale | Repair |
|---|---|---|
| `the_built_set_fills_its_layers::the_block_no_quad_draws_still_holds_a_layer_and_no_face_takes_its_key` | water was the witness for "a layer is allocated for a key no quad samples", and T09 drew the sea | re-homed onto `stone.luau` restated `drawn = false, solid = true` |
| `replay_player::…` | column `(32, 32)` and yaw `225°` | `(63, 35)` and `230°`, and renamed |
| `entry_leaves_a_generated_spawn_alone::…` | same pair | same |
| `launch_player::…` | column only | same |

**The layer test is the one worth reading.** Its expectation could have been
renumbered from 0 to the 11 quads water now draws, and that was the cheap repair.
It is worse than cheap: the first element of its tuple ("water holds a layer")
only witnesses declaration-driven allocation while the third establishes that no
quad draws it, so once the sea is drawn a *mesh*-derived layer set would give
water a layer too and the reading could no longer tell the two mechanisms apart.
**A green test that has stopped asking its question is worse than a red one,
because it reads as evidence.**

The re-homed version is **stronger than what it replaces**, because the old one
rested on a coincidence this spec deletes — water happened to be unmeshed because
it happened to be non-solid — and the new one declares its premise outright.
Stone because it is the most present block: **13 315** of meshed face area against
grass 6 900 and dirt 783, so a launch emitting even a handful of its faces misses
by a wide margin rather than by one. Measured by mutation: declaring that same
stone drawn puts **163** quads into the comparison against an expected 0, so the
antecedent is load-bearing by a factor of 163 rather than by one.

### The spawn fixtures were right to redden, and one rename

All three hardcode the spawn column rather than reading `SPAWN_COLUMN`, and
`launch_player`'s own doc says why: *"a fixture reading the constant it asserts
against would agree with a spawn that moved as readily as with one that did
not."* Surface height is still read from the world, so only the column and the
yaw are restated. A sibling that *derives* rather than hardcodes followed the
move untouched, so there is one instrument of each kind and it stays that way.

The figures were verified rather than transcribed, because two of the three
compare `to_bits` and a slip there lands as an integer that decodes to something
plausible. `1115553792 → 63.5`, `1108606976 → 37.0`, `1108213760 → 35.5`,
`1082160332 → 4.014257431`; re-encoding returns those four integers;
`radians(230) = 4.014257280` and the nearest `f32` is the reported value.
**And the published camera target agrees with the declared yaw** —
`target − eye = (−0.642788, 0, −0.766045)`, unit to seven places, against
`(cos 230°, sin 230°)` — which is what says the camera is the new yaw's rather
than a stale target carried alongside a changed number, and is the one thing a
bit comparison could not have caught.

**`the_player_spawns_over_the_declared_column_facing_the_landmark` is renamed,
and not to name the sea.** The yaw is there for the sea now and the pillar is a
residue measured rather than intended — but naming it for the sea would repeat
the defect one generation on. **The test asserts a pose and never looks at what
is in frame**, which is exactly why somebody had to go and measure the pillar to
find out whether the *name* was still true. A name checkable only by running a
different test is the thing to remove, not the particular purpose it names. It
becomes `…_at_the_declared_yaw_and_has_not_yet_landed`, and the purpose of the
column and the yaw lives in the spawn constants' own doc comments.

Evidenced rather than argued, in the end: the spawn moved **twice**. A landmark
name would have needed re-checking twice and a sea name once; the pose name
survived both untouched.

### Two figures the coastal spawn falsified, both mine, both restated

Both conclusions survive and both supporting sentences did not — which is the
shape this phase kept meeting, and it is why they are restated rather than
repaired to a fresher number.

- **`PREDICTION_FLOOR`.** Its doc said a frame 78 % not sky predicts around 450 of
  576. Measured at the coastal spawn the frames are 58 % / 71 % / 55 % not sky and
  predict **335 / 408 / 317**. The floor is **not moved**: re-derived from both
  directions, above zero because a collapsed oracle is what it exists to catch,
  and below the tightest of 317, which leaves it 3.2× under.
- **Why tick 0 is the perturbed frame.** The header said its horizon sits highest
  and that by tick 59 the frame is almost entirely terrain. That was a genuine
  selection when the sky counts were 135 against 32. They are now
  **241 / 168 / 259**, and **tick 0 is not even the roomiest — tick 119 is.**
  Measured at the same time, the 3° control finds **22 / 26 / 25** disagreements
  at the three ticks: an order of magnitude over the budget of 2 at every one, and
  near enough the same at each.

So that reason has not merely gone stale, it has **stopped selecting anything**
while still reading as though it decided something. It is recorded as such. Tick 0
is kept because it is the opening frame and because keeping it leaves the control
unchanged in every respect but the spawn; a justification that sounds decisive
over an 18-sample margin is worse than one that admits it is arbitrary.

`DISAGREEMENT_BUDGET` is **not raised** and stands at 2, with the unpitched
comparison measured at **zero** disagreements at all three ticks — the whole
budget unspent. **No sample was relocated.**

### Water in frame, and one degenerate pass the enumerated verdict closes for free

Water samples per tick at the declared spawn: **56 / 200 / 111** of 576, so
FR-7.1-S1 judges **367 predicted water pixels across the three frames, every one
inside ΔE 8 of the water layer's mean, with no disagreement budget at all.** The
tolerance was chosen before the test had ever run against a frame, and 200 of
those pixels are at tick 59 where a wrong tolerance had the most chances to show.

**The degenerate pass worth naming**: the declared spawn stands *on* the sea level
with the sea one column over, so it is worth asking whether the eye ever sits
inside a water voxel. If it did, FR-7.1-S1 would pass for a reason unrelated to
the renderer — `first_drawn` tests the voxel the eye occupies before it steps, so
a submerged eye classifies **all 576** samples as water and every one is trivially
drawn in water's own layer.

**A correction to what this record first said, and it was mine.** It read: *"ruled
out by the counts — 56, 200 and 111, none of them 576 — and ruled out for free. A
classification obliged to total its grid cannot hide a saturated frame."* **That
is false.** Totalling the grid and being spread across it are different
properties, and only the first is asserted: 576 samples all classified
`base:water` total the grid perfectly. Traced through the file, a submerged eye
passes **everything**:

- the classification totals 576, and one class doing so satisfies it;
- `PREDICTION_FLOOR` is a **floor** — 576 clears 100, so the collapse detector is
  silent;
- FR-6.2-S3 asks that predicted terrain not be drawn as sky, and 576 water samples
  are all non-sky;
- FR-7.1-S1 then judges a frame that genuinely *is* water at every sample and
  passes **honestly**.

**Nothing in the workspace bounds terrain from above.** So the saturated case was
not unrepresentable, only absent — and what made it absent was where the spawn
happened to sit. Writing a contingency down as a property is the error this whole
phase has been refusing, and it went into this record under my own hand.

**This phase created the exposure**, which is why it is this phase's to close.
`predicted_terrain` now derives from the classification rather than marching
separately — right, and documented, and it stops the two disagreeing about which
samples are terrain — but it removes the second opinion that would otherwise have
reported a saturated frame.

**Closed by a guard**, `replay_oracle.rs` ·
`the_camera_of_every_judged_frame_stands_in_open_air`, which asserts the premise
rather than the symptom: the cell the eye occupies is **named** at each judged
tick and compared against nothing at all three. Not a sky floor — a floor on sky
constrains the declared poses rather than stating a property, and a frame
legitimately filled by a hillside is fine. This converts *"the counts happen not
to be 576"* into *"the counts cannot be 576, because the eye is not inside
anything drawn"*, and it survives every future spawn move without anyone
re-measuring.

Falsified by mutation: sinking the eye six blocks gives
`left: [(0, Some("base:dirt")), (59, Some("base:stone")), (119, Some("base:dirt"))]`
against `right: [(0, None), (59, None), (119, None)]` — exactly one test reddens,
and it names the block the eye would be standing in. Reverted by hand.

**It needs no positive control of its own**, and the reason is worth recording
because an absence assertion normally does. The way this one would rot is a judge
whose `is_drawn` answers false everywhere — and that judge predicts *nothing* as
terrain, which `PREDICTION_FLOOR` catches in the same file. The two fail in
opposite directions and neither can go quiet without the other speaking.

### The census, and why it is committed rather than re-run from a scratch file

`crates/mc-sim/tests/replay_world.rs` ·
`the_declared_sea_is_the_one_the_heightmap_implies_and_its_sides_add_up`.

The figures the mutation predictions rest on were originally taken by a throwaway
test that was run once and deleted — recoverable from nothing but this record,
which is the position Phase 2 was in when it reported on a tree object that had
never been committed. They are now a test, in three layers:

1. **Two enumerations compared** — the heightmap rule against a scan of all
   `64 × 64 × 256` cells. Neither is snapshotted. **This closed the one real
   worry**: 178 might have been a floor rather than a total if the rule missed
   water the generator placed. It does not.
2. **The arithmetic** — sides sum to six a voxel, an invariant of any world,
   asserted on its own so a classification defect reports itself as that rather
   than as the sea having moved.
3. **Four committed numbers**, a **tripwire and not an oracle**, in
   `SCENE_QUAD_COUNT`'s own shape and for its reason. No arithmetic derives them:
   178 is a sum over submerged columns of `SEA_LEVEL − surface` and the heightmap
   is a hash.

**Layer 3 earns its place because 1 and 2 cannot do its job.** Both are
internally consistent about *whatever world they are handed*, so both stay green
if the generator changes and the sea moves. The snapshot is the only layer that
notices the world itself changed. Proven by mutation: moving `WATERY_SIDES` 662 →
661 gives `left: (178, 201, 662, 205)` against `right: (178, 201, 661, 205)`,
reverted by hand.

**It may check the census and never set it.** The mesher reports water's area and
the independent walk agrees with it, so those two corroborate each other; a
census disagreeing with them is **this census's** fault, and the repair is to find
the cells it missed, never to adopt their figure. That constraint is in the test's
own doc, where a reader holding a red census beside a green mesher will look.

**Independence, in the terms that matter**: it resolves **no block definition at
all**, calls none of the walk's `Beyond`, `shows`, `Side::step` or
`inside_the_world`, writes out its own six offsets and bounds, and classifies a
neighbour by block-name text. What it shares is `block_at` and `surface_height` —
the *subject world's* own accessors — which is reading the subject rather than
sharing a derivation of visibility, the same relationship `submerged_columns`
already has.

**And it pins an assumption that was previously only asserted.** The reason a
census taken before T09 is trusted after it is that it measures *voxels* while
T09 changes *declarations* — a claim about `ReplayWorld::generate`, which takes
the registry, rather than a given. The test reads **178** and **201 / 662 / 205**,
the figures recorded at `bc7541a`, on a tree that has since seen the declaration,
two spawn moves and the `r2` re-shoot. **That invariance is a result**: world
generation does not depend on what a block declares.

Runtime, measured before committing it as required: **0.086 s** bare and
**1.62 s** under `cargo llvm-cov nextest`, a 19× factor consistent with the ~30×
the hot-reload record measured on another workload. Well under the minute that
would have needed a decision.

### A file four commits from a ceiling, noted against PRO-952

`crates/mc-client/tests/support/oracle.rs` stands at **525 non-blank of the 600
the gate allows** — measured with `Get-Content | Measure-Object -Line`, which is
the gate's own counter and **does not count blank lines** (the file is 557 total).
It is the file this phase grew most, carrying the judge's rewritten header,
`Sighted`, `SightedSample`, `sighted_samples`, `drawn_block` and
`first_drawn_face`.

Nothing in this phase breaches it. **PRO-952 is the named risk**: that module's
own header records it as the change which needs a second march rule, because a
first-drawn-voxel march is right only while every drawn block is opaque. Whoever
picks it up should meet this note before the gate hands them the same fact.

### Additional coverage, added after the four repairs

- `crates/mc-client/tests/replay_oracle.rs` ·
  `the_camera_of_every_judged_frame_stands_in_open_air` — **catches an eye inside
  a drawn voxel at any judged tick**, which would classify all 576 samples as that
  block and pass every other reading in this phase honestly: the grid still
  totals, the prediction floor is a floor, every sample is non-sky, and the frame
  really is that colour. Nothing else bounds terrain from above, so before this
  the saturated case was prevented only by where the declared spawn sits. The
  pair could not catch it because `predicted_terrain` now derives from the
  classification rather than marching separately — correct, and it costs the
  second opinion that would have disagreed under saturation.

## Phase 4 — What can be aimed at, and what a new player holds

Twelve scenarios: FR-3.1-S1, FR-3.1-S2, FR-3.2-S1, FR-3.2-S2, FR-3.3-S1,
FR-3.3-S2, FR-3.4-S1, FR-3.4-S2, FR-3.5-S1, FR-4.1-S1..S3.

| Scenario | File | Test |
|---|---|---|
| FR-3.1-S1 | `crates/mc-sim/tests/block_targeting_is_declared.rs` | `a_break_takes_a_block_that_stops_nobody_but_declares_a_ray_may_stop_at_it` |
| FR-3.1-S2 | `crates/mc-sim/tests/block_targeting_is_declared.rs` | `a_break_reaches_past_a_solid_block_that_declares_no_ray_may_stop_at_it` |
| FR-3.2-S1 | `crates/mc-sim/tests/block_targeting_is_declared.rs` | `a_block_placed_into_an_empty_cell_is_what_the_next_ray_across_that_cell_stops_at` |
| FR-3.2-S2 | `crates/mc-sim/tests/block_targeting_is_declared.rs` | `breaking_the_block_a_ray_stopped_at_lets_the_next_ray_reach_the_one_behind_it` |
| FR-3.3-S1 | `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs` | `a_swing_aimed_through_the_shipped_water_stops_at_it_and_not_at_the_stone_behind_it` |
| FR-3.3-S2 | `crates/mc-sim/tests/the_shipped_water_is_aimable_only_within_reach.rs` | `a_swing_at_water_first_met_beyond_five_blocks_from_the_eye_finds_no_target_at_all` |
| FR-3.4-S1 | `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs` | `a_break_swung_at_the_shipped_water_is_refused_as_indestructible_and_leaves_it_in_the_cell` |
| FR-3.4-S2 | `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs` | `a_break_swung_at_the_shipped_water_leaves_the_solid_block_behind_it_untouched` |
| FR-3.5-S1 | `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs` | `a_placement_into_the_shipped_water_replaces_it_with_the_block_being_placed` — **an existing test, unchanged to the byte; see below** |
| FR-4.1-S1 | `crates/mc-sim/tests/held_block.rs` | `the_shipped_content_puts_dirt_in_a_new_players_hand` |
| FR-4.1-S2 | `crates/mc-sim/tests/held_block.rs` | `a_registry_of_blocks_that_stop_nobody_offers_none_though_one_is_drawn` |
| FR-4.1-S3 | `crates/mc-sim/tests/held_block_after_a_reload.rs` | `a_reload_that_takes_solidity_off_the_first_two_blocks_puts_the_third_in_hand` |

### The RED, on the tree it was taken on

Committed as `c1e7c90`, working tree clean at the moment of the reading —
which the reading itself dates, because a run naming eleven tests FAILED
cannot have come from a tree on which they pass.

```
cargo nextest run -p mc-sim --no-fail-fast
```
→ `Summary [3.065s] 179 tests run: 168 passed, 11 failed, 0 skipped`

```
cargo nextest run --workspace --no-fail-fast
```
→ `Summary [44.004s] 1428 tests run: 1417 passed, 11 failed, 1 skipped`

A **bare** `N tests run` is a complete run; a slashed `N/M tests run` is a
cancelled one and says nothing about the rest. Both counts above are bare and
both invocations carry `--no-fail-fast`. The workspace figure recorded for
`cbe49c0` was 1 415, and 1 428 − 1 415 = 13 is exactly the net count of tests
this phase adds (four for targeting, two for the placement rule, one for
reach, one for the reload's aiming view, two for the held block, one for the
held block after a reload, and three replacing one in the water file).

The same eleven fail in both runs and no twelfth does. Each is an **assertion**
failure rather than a compile error, and each fails in the direction the
scenario predicts:

| Test | left (this tree) | right (the scenario) |
|---|---|---|
| `a_break_takes_a_block_that_stops_nobody…` | `[(12,11,8), base:dirt → nothing]` | `[(10,11,8), fixture:aimable → nothing]` |
| `a_break_reaches_past_a_solid_block…` | `[(10,11,8), fixture:unaimable → nothing]` | `[(12,11,8), base:dirt → nothing]` |
| `a_block_placed_into_an_empty_cell…` | `(Refused(Occupied), [(11,11,8) nothing → fixture:aimable])` | `(Changed at (10,11,8), [(10,11,8) nothing → base:dirt, (11,11,8) nothing → fixture:aimable])` |
| `breaking_the_block_a_ray_stopped_at…` | `[(12,11,8), base:dirt → nothing]` | `[(10,11,8) fixture:aimable → nothing, (12,11,8) base:dirt → nothing]` |
| `a_swing_aimed_through_the_shipped_water…` | both halves `Changed` at `(9,10,8)`, stone emptied | `(Refused(Indestructible), [])` and `Changed` at `(9,10,8)` |
| `a_swing_at_water_first_met_beyond_five_blocks…` | `((NoTarget, []), (NoTarget, []))` | `((NoTarget, []), (Indestructible, []))` |
| `a_break_swung_at_the_shipped_water_is_refused…` | `(Changed at (9,10,8), "base:water")` | `(Refused(Indestructible), "base:water")` |
| `a_break_swung_at_the_shipped_water_leaves_the_solid_block…` | `(Changed at (9,10,8), [(9,10,8) base:stone → nothing])` | `(Refused(Indestructible), [])` |
| `a_place_aimed_at_a_replaceable_block…` | `Changed at (11,11,8), Empty → base:dirt` | `Changed at (12,11,8), fixture:buildable → base:dirt` |
| `a_place_aimed_at_a_replaceable_cell_the_player_is_standing_in…` | `((InsidePlayer, []), (NoTarget, []))` | `((InsidePlayer, []), Changed at (9,11,8) base:water → base:dirt)` |
| `a_swing_passes_through_a_block_a_reload_said…` | both halves `[(10,11,8) base:stone → nothing]` | second half `[(12,11,8) base:dirt → nothing]` |

**FR-3.1-S2 is the phase's named shape of RED and it has it**: the reported cell
is the `targetable = false, solid = true` block itself, and the scenario demands
the cell beyond it.

**FR-3.2-S1 and S2 fail against a view built at load and never re-written, which
is the ruling.** Both were designed against that implementation and not only
against today's, and the two answers are recorded because they are not the same
answer:

- FR-3.2-S1's cell holds **nothing** when the world is built, so a build-once
  view answers "not aimable" there forever. It then carries the second tick's
  ray on to the block behind and refuses the placement as `Occupied`, because
  the cell it steps back into is the one the first tick filled. That is exactly
  the left-hand column above — today's tree and a build-once implementation give
  the same answer here, and both differ from the scenario's.
- FR-3.2-S2's near cell **is** declared aimable at load, so a build-once view
  admits the first swing and then stops the second ray at the cell it has just
  emptied, refusing it as nothing to break. One cell moves where the scenario
  demands two. Today's tree fails differently (the first swing goes straight
  through), which is why the assertion is the whole two-cell diff rather than a
  count.

### The lint, run directly because a green suite is no evidence about one

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
→ exit 0 on `c1e7c90`. `cargo fmt --all -- --check` → exit 0.

It found one thing worth recording: `too_many_lines` (31/30) on the first
placement reading, before it was refactored to build its tick through a helper.
No test was loosened to reach that; the assertion is unchanged.

**The lint is the only instrument for the window that opens next.** The
adaptation commit `733485e` names a type the implementation has not written, so
`cargo build --tests -p mc-sim` fails with `unresolved import
mc_sim::replay::ResolvedVoxels` until T15 lands and the gate has no compilable
tree to run on until then. That is the window
`standards/global/testing.md` §2 describes; the reading above was taken on
`c1e7c90`, the commit *before* it, which is the last tree that compiles.

### Three scenarios were green before the change, and a fourth the brief did not name

**FR-3.5-S1, FR-4.1-S1, FR-4.1-S3 and — not on the brief's list — FR-4.1-S2.**
A scenario green before the change it is about is not evidence, so what makes
each non-vacuous is recorded rather than assumed.

- **FR-4.1-S2 is green today and the brief predicted it would not be.** The
  prediction rested on `registry_of_declarations` not existing; that helper is
  test-side and this phase's test author writes it, so once written the
  assertion compiles and passes — `default_held_block` reads `is_solid` and is
  unchanged by this phase (`architecture.md` Decision 11), so there was never a
  production change for it to be red against. Recorded rather than smoothed
  over. What makes it non-vacuous: its first block is the only fixture in the
  tree declaring `drawn = true, targetable = true, solid = false`, so a rule
  that had drifted onto either field answers that block; and it is asserted as a
  **pair** against a registry differing in one word, which answers `base:stone`
  — so "answers nothing for anything" fails the second half. Mutation M3 below
  is the proof.
- **FR-4.1-S1** pins the shipped answer through the split and cannot falsify the
  rule, because content is read in file-name order and dirt is both the first
  block and the first solid one. What keeps the *rule* honest is
  `held_block.rs`'s own registry, which registers a non-solid block first. This
  is also the scenario that keeps the HUD golden still, and it answers
  `base:dirt`.
- **FR-4.1-S3** pins a rule the split does not move. Its non-vacuity is the
  restate-don't-remove construction: a candidate that *dropped* dirt and grass
  would leave stone the first block registered as well as the first that
  collides, and a rule reading plain registration order would answer stone too.
  Restating them keeps dirt first in registration order, so "first block" and
  "first colliding block" stay two different answers.
- **FR-3.5-S1** is green today through the *old* mechanism and must be green
  afterwards through the *new* one, and nothing about that can be argued. Its
  whole evidence is mutation M2 below, plus the two additional readings that
  exercise the new rule directly. The test is unchanged **to the byte** — its
  assertion, its cell, its `from` and its `to` — which was confirmed by the diff
  of the file it lives in touching no line of it.

### The fuse was blown, not annotated

`a_break_aimed_through_the_shipped_water_reaches_the_solid_block_behind_it` is
**gone**, and FR-3.4-S1 and FR-3.4-S2 are what its own doc comment asked for:
*"the repair is a new scenario, not a new expectation."* A test red for a known
reason is fixed before its phase closes, so it was replaced in the RED commit
rather than left failing across the phase. The file's header no longer records a
debt; it records that the debt is paid and what a player now meets.

**A second reading in that file also reddened and no task named it.** The
placement reading survives *verbatim*, but only because the placement rule
gained the replaceable-hit-cell case — without it the placement lands at
`(8, 11, 8)`, which is inside the player's box, and the answer is
`Refusal::InsidePlayer`. That is what M2 measures.

### FR-3.3-S1 and FR-3.4-S1/S2 share one run, and that is worth saying plainly

Three assertions over one fixture and one aim, not three independent witnesses.
They are different claims — *the ray stopped at the water rather than at the
stone*, *the refusal is indestructibility and the water is still there*, *no cell
of the world moved* — and each is written so that its own failure is
distinguishable, but a reader counting witnesses should count one run.

What keeps FR-3.3-S1 from being a restatement of FR-3.4 is its **control**: the
same player, the same aim, the same everything with one declared cell of water
removed, which empties the stone. That half is about where the ray stopped and
nothing else can produce it.

### Additional coverage

Each line states what it catches. None is a scenario's own test.

- `crates/mc-sim/tests/a_placement_aimed_at_a_replaceable_cell.rs` ·
  `a_place_aimed_at_a_replaceable_block_lands_in_that_blocks_own_cell` — the new
  placement rule, exercised directly, with no refusal anywhere in the
  expectation. The fixture registry's `fixture:buildable` is **solid and
  replaceable at once**, which nothing content ships is, so a ray stops at it on
  today's tree and the two rules land the block in two different cells with two
  different `from` values. This is the only reading that separates them without
  depending on anything becoming targetable, and it is red before the change and
  green after — which FR-3.5-S1's own test cannot be.
- `crates/mc-sim/tests/a_placement_aimed_at_a_replaceable_cell.rs` ·
  `a_place_aimed_at_a_replaceable_cell_the_player_is_standing_in_changes_nothing`
  — catches the new branch picking a cell and skipping the box check. Choosing a
  different cell is not licence to stop asking whether the player is standing in
  it, and a branch that did would build a block through the player while every
  scenario about water stayed green. The refusing half is green before and after
  (both rules choose a cell the box occupies); the accepting half, aimed level at
  the identical fixture, is red before and green after, so the reading as a whole
  is not a standing fact.
- `crates/mc-sim/tests/reload_targeting_views.rs` ·
  `a_swing_passes_through_a_block_a_reload_said_no_ray_may_stop_at` — **a second
  witness on `World::adopt`.** FR-3.2's two scenarios cover `World::write` and
  say nothing about the other place either view is written. That a wholesale
  replacement cannot write the two bits apart is an argument about how the code
  is shaped, and an argument is not a witness. A candidate restates `base:stone`
  as `solid = true, targetable = false` — stated *against* the default, so a
  reader answering from the default answers `true` — and a swing has to start
  passing through it with no cell of the world written. This is also what makes
  the phase mutation `tasks.md` names (stop writing the targetable bit in
  `adopt` only) able to bite at all; see M4.
- `crates/mc-sim/tests/resolved_voxel_updates.rs` ·
  `setting_one_voxels_answers_changes_that_voxel_and_no_other` — renamed from
  `solidity_updates.rs` · `setting_one_voxels_solidity_changes_that_voxel_and_no_other`
  and widened to ask about both views. **The only instrument in the tree that can
  see the second bitset's addressing**: every scenario about aiming writes a cell
  and reads *that same cell* back, so a pair of bitsets addressed wrongly in the
  same way answers all of them correctly. Both positions are settled to the same
  pair — an obstacle no ray stops at — from different starting answers, so the
  collision view is the only one that may move at the raised position and the
  aiming view the only one that may move at the hollowed one. A `set` that
  settled either view from the other's argument, or ignored one of them, lands a
  row on the wrong pair or drops it.

### Mutations run, and what each proved

Each was run by hand, observed, reverted **by hand** (never
`git checkout -- <file>`), and confirmed with `git diff --exit-code`. The
outcome is recorded either way, including the ones that did not bite.

**All four are run and recorded below.** M3 was run once at RED time, over
`mc-sim` alone, because FR-4.1-S2 is green before the change and green after it
and a scenario with no RED of its own needs its falsifier front-loaded rather
than queued behind a window that can be crossed without being run. The other
three needed the split to exist. M3 was then run a second time across the whole
workspace, in the same hand-over as the rest, to close the breadth its first run
could not reach.

**One thing about M3's first run is a fact about this working tree rather than
about the code, and it is recorded because it is the signature of a flaky test.**
That window collided with the implementer's first minutes on T15: two agents held
this tree, and the compile failure that ended its `mc-client` half was theirs,
not a result. What tells the two apart afterwards is the failing-test *count* and
the named outcomes, which is why both are written out. The gap it left is closed
under M3' below, and closing it found a witness the argument had missed.

All four were run in **one hand-over**, sequentially, on `c978e19` with the tree
clean and 0/0 against origin. At most one was applied at any instant; each was
reverted by hand — the exact line re-edited, never `git checkout --` — and each
revert was confirmed before the next was applied.

Window baseline and window close, the same invocation both times:

```
cargo nextest run --workspace --no-fail-fast
```
→ open: `1428 tests run: 1428 passed, 1 skipped`
→ close: `1428 tests run: 1428 passed, 1 skipped`

Bare counts, so both are complete runs rather than the head of a cancelled one.
Every mutation below was measured with that same whole-workspace invocation, so a
reddening test anywhere in the tree would have been seen rather than filtered out
by a crate selector.

**A note on the tree check, because it is the kind of thing that reads as a
warning later.** After the last revert `git status` went on reporting
`crates/mc-sim/src/world/mod.rs` as modified while `git diff` reported nothing.
That was a stale stat cache and not a content difference, and it was settled
against the object rather than against the porcelain: `git cat-file blob
HEAD:…/world/mod.rs` compared with `cmp` against the working file reports the
bytes identical, `git hash-object` answers the same SHA HEAD names, and
`git diff HEAD --stat` is empty. Three readings of the content, one of the stat.

### M1 — the write site stops applying targetability

`World::write` settles both answers and then leaves the aiming bit at whatever
the load resolved, so the decision is computed and never applied. That is the
"policy is not wiring" shape exactly.

→ `1428 tests run: 1422 passed, 6 failed, 1 skipped`

| reddened | |
|---|---|
| `mc-sim::block_targeting_is_declared` | `a_block_placed_into_an_empty_cell_is_what_the_next_ray_across_that_cell_stops_at` (FR-3.2-S1) |
| `mc-sim::block_targeting_is_declared` | `breaking_the_block_a_ray_stopped_at_lets_the_next_ray_reach_the_one_behind_it` (FR-3.2-S2) |
| `mc-sim::edit_replay` | `the_replay_leaves_every_cell_holding_the_block_the_schedule_derives_for_it` |
| `mc-sim::edit_replay` | `the_run_answers_every_edit_with_the_change_the_schedule_derives_ten_thousand_times_over` |
| `mc-client::click_dispatch` | `a_single_left_click_changes_one_block_over_the_ten_ticks_that_follow_it` |
| `mc-client::edit_geometry` | `breaking_a_block_where_the_footprint_ends_leaves_its_faces_absent_and_reports_no_error` |

**The expectation was FR-3.2-S1 and S2 "and nothing else". Six reddened, and the
expectation was too narrow rather than a test being wrong.** The rule it came
from — a third reddening test means one of them measures something other than
what it claims — is about a test whose *stated* subject is unrelated. That is not
what these are. All four extras drive real edits end to end and then act again:
`edit_replay` replays a derived schedule of ten thousand edits, `click_dispatch`
follows one click across ten ticks, `edit_geometry` breaks and re-reads. Under
the mutation the cell just edited keeps a stale aiming bit, so the *next* ray
stops at the wrong cell — which is FR-3.2-S2's defect reached through four other
fixtures. An end-to-end run legitimately reddens for any defect in the pipeline
it runs, and reporting that as a fault in those tests would be the mistake.

So the answer to "what calls the second bitset, and what would go red if the
write site stopped setting it" is **six tests, both FR-3.2 scenarios among them**.
The wiring is not merely tested; it is tested six ways.

### M4 — `adopt` keeps the aiming view the previous registry resolved

The registry and the collision view are replaced; the aiming view is left as it
was.

→ `1428 tests run: 1426 passed, 2 failed, 1 skipped`

| reddened | |
|---|---|
| `mc-sim::reload_targeting_views` | `a_swing_passes_through_a_block_a_reload_said_no_ray_may_stop_at` |
| `mc-client::reload_keeps_the_player` | `a_reload_leaves_the_player_where_the_same_ticks_with_no_reload_would_have_put_them` |

**`tasks.md`'s prediction for this mutation is now falsified by measurement
rather than by argument.** It said *"FR-3.2 must redden while FR-3.1 stays
green"*. **Neither FR-3.2 scenario is in the list**, because neither reaches
`adopt` — both drive `World::write` through an ordinary edit. Had the mutation
been run against that expectation it would have been recorded as "did not bite",
which reads as information about the code and would have been information about
the prediction. The witness it actually needs is the reload reading this phase
added for exactly this reason.

**And `adopt` turns out to have two witnesses rather than one**, the second in
`mc-client` and reached by a different route — a reload that must leave the
player where the same ticks without one would have put them.

### M2 — the old step-back rule restored

`landing` returns one step back whatever the hit cell holds, which is the rule as
it stood before this phase.

→ `1428 tests run: 1425 passed, 3 failed, 1 skipped`

| reddened | |
|---|---|
| `mc-sim::shipped_water_is_not_broken_and_is_built_through` | `a_placement_into_the_shipped_water_replaces_it_with_the_block_being_placed` (FR-3.5-S1) |
| `mc-sim::a_placement_aimed_at_a_replaceable_cell` | `a_place_aimed_at_a_replaceable_block_lands_in_that_blocks_own_cell` |
| `mc-sim::a_placement_aimed_at_a_replaceable_cell` | `a_place_aimed_at_a_replaceable_cell_the_player_is_standing_in_changes_nothing` |

**The refusal, recorded rather than predicted: `Refusal::InsidePlayer`.**

```
left:  (Some(Refused(InsidePlayer)), [])
right: (Some(Changed { cell: (9, 11, 8), from: base:water, to: base:dirt }),
        [((9, 11, 8), "base:water", "base:dirt")])
```

**This is the whole of FR-3.5-S1's evidence and it holds.** That test is green
before the change and green after it, so nothing about its passing could stand on
its own; what it needed was proof that it can tell the two rules apart, and under
the old rule it goes red with the refusal the arithmetic said it would. It is a
test that can see the mechanism, not one that survived a mechanism change by
being blind to it.

Both additional-coverage readings reddened too, so the new rule has **three**
witnesses and FR-3.5-S1 does not rest on one. The guard's accepting half is the
sharpest of them: under the old rule it answers `InsidePlayer` for the very
reason the rule was added, because one step back from a west face is the cell the
player's own head is in.

### M3' — the held block reads drawnness, measured across the workspace

`default_held_block` finds on `drawn` instead of `is_solid`. Run once before at
RED time over `mc-sim` alone; run again here across the workspace to close the
breadth gap that run left open.

→ `1428 tests run: 1426 passed, 2 failed, 1 skipped`

| reddened | |
|---|---|
| `mc-sim::held_block` | `a_registry_of_blocks_that_stop_nobody_offers_none_though_one_is_drawn` (FR-4.1-S2) |
| `mc-client::shipped_blocks_are_declared_in_luau` | `a_content_root_declaring_nothing_solid_refuses_to_start_rather_than_opening_a_window` |

**The gap is closed and the argument that stood in for it was too narrow.** What
was written in M3's own record was that `launch.rs` resolves over shipped
content, where `base:dirt` is both drawn and solid, so the two fields cannot
disagree there — explicitly labelled reasoning rather than a measurement. The
reasoning was right about `launch.rs` and wrong about `mc-client`: a second
witness exists, over a root declaring nothing solid, and under the mutation a
client that should refuse to start instead fails for an unrelated reason
(`RefusedOtherwise("a new world could not be generated")` where the scenario
demands `RefusedForHavingNothingToPlace`). **That is the difference between an
argument and an observation, and it is why the run was worth making.**

FR-4.1-S1 stayed green, **as it must** — the shipped dirt is both drawn and
solid, which is exactly why that scenario cannot falsify the rule and is not
meant to. FR-4.1-S3 stayed green for the same reason: the candidate it reloads
declares `solid = false` on dirt and grass without saying anything about
drawnness, so the two follow each other there.

### The second fuse was predicted before it was measured, and this is the cashing

Recorded because a prediction that is cashed is worth more than either half
alone, and because the provenance is what makes it one.

The phase-4 test-author brief, committed at `b20e8f8` **before any implementation
existed**, derived from the shipped-water fixture's own aim that
`a_placement_into_the_shipped_water_replaces_it_with_the_block_being_placed`
would redden alongside the break test the task list named — and that no ray
geometry could fix it. The ray leaves `(8.5, 11.62, 8.5)` along
`(0.866, -0.5, 0)`, crosses `x = 9` at `t = 0.577` where `y = 11.33`, and so
enters the water cell `(9, 11, 8)` through that cell's **West** face; a step-back
rule therefore lands the placement in `(8, 11, 8)`, which the player's own box
covers at `HALF_WIDTH = 0.3` and `HEIGHT = 1.8`. The generalisation was the part
that settled it: once water is targetable the cell one step back is by
construction never replaceable, because had it been, the walk would have stopped
there instead.

Measured after **T15 alone**, with the placement rule not yet written:
`cargo nextest run -p mc-sim --no-fail-fast` →
`179 tests run: 176 passed, 3 failed, 0 skipped`, and that test was one of the
three.

**M2 above is the same fact from the other side**, and the two together are what
FR-3.5-S1 rests on: the failure was predicted from the arithmetic before the code
existed, and the repair was proved to be what the test can see.

### The helper population, re-measured — and this is the third figure it has given

```
grep -rn "fn registry_declaring\|solid: bool\|is_solid: bool\|&\[(&str, bool)\]" --include=*.rs crates/ | grep -v "/src/" | wc -l
```
→ **26** on `dd4577b`, the tree this phase opened on. It read 21 before Phase 1
and 24 after, and each growth came from the previous phase's own adaptation,
which is the argument for re-running it rather than carrying it. Do not carry 26
forward either.

Of those 26, exactly one was this phase's: `registry_declaring` at
`crates/mc-sim/tests/support/volume.rs`, with three call sites
(`held_block.rs`, `replay_solidity.rs`, `resolved_voxel_updates.rs`). It is
widened **by addition**, exactly as Phase 2 widened `mesh_common`: the
one-boolean signature keeps its shape and delegates, a four-field `Declaration`
and a `registry_of_declarations` are added beside it, and all three existing
call sites moved by zero lines.

**`mesh_common/mod.rs` was left alone, and Phase 2's note predicting otherwise
was wrong.** Nothing in `mc-world` reads targetability — the mesher does not —
and `registry_of_declarations` there already answers `targetable: states.solid`,
which is what every meshing fixture means. It stands at 490 non-blank lines
against the gate's 600 and had no business growing here.

### The chamber's `base:water` is not what its doc comment said, and must stay as it is

`crates/mc-sim/tests/support/chamber.rs` declares `base:water` through `open()`
with `breakable: true`, where content declares `breakable = false` — so the
comment claiming these are *"the four blocks content ships, spelled and declared
as content declares them"* was already inaccurate before this spec.

**The fixture registry borrows content's *names*, not its *declarations*.** The
sentence has been repaired to say that, in those words, and the declarations
have deliberately **not** been repaired to match the old sentence. Making that
water `targetable = true` breaks `crates/mc-sim/tests/block_breaking.rs`, which
builds `filled_with(WATER).cell(EMPTIED, STONE)` — a chamber whose *background*
is water, used to tell "this cell was emptied" apart from "this cell holds the
background". A targetable background stops every ray in that fixture at the
first cell it crosses. The scenarios that are about what content really declares
build over `support::content_registry()` and read the shipped root.

### Three fixture blocks were added to the chamber overlay

Named for what they separate, in the register the module already uses:

- `fixture:aimable` — stops nobody, and a ray stops at it.
- `fixture:unaimable` — stops a player, and a ray goes through it.
- `fixture:buildable` — stops a player *and* may be built over, which nothing
  content ships is.

The first two are needed as a pair rather than one: a rule reading solidity
where it means targetability reports the wrong cell in *opposite* directions for
them. Nothing asserts the overlay's length —
`the_fixture_registry_numbers_a_block_of_base_content_first_and_the_unbreakable_block_after_it`
asks only that id 0 is dirt and that the unbreakable block is not id 0 — and the
three blocks are appended, so ids move for nothing that reads them.

### File sizes, measured on the RED tree

The gate counts **non-blank** lines: 600 for test files.

| File | non-blank | was |
|---|---|---|
| `crates/mc-sim/tests/support/chamber.rs` | 518 | 456 |
| `crates/mc-sim/tests/support/roots.rs` | 340 | 308 |
| `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs` | 297 | 191 |
| `crates/mc-sim/tests/a_placement_aimed_at_a_replaceable_cell.rs` | 213 | new |
| `crates/mc-sim/tests/block_targeting_is_declared.rs` | 208 | new |
| `crates/mc-sim/tests/support/volume.rs` | 197 | 132 |
| `crates/mc-sim/tests/reload_targeting_views.rs` | 159 | new |
| `crates/mc-sim/tests/the_shipped_water_is_aimable_only_within_reach.rs` | 141 | new |
| `crates/mc-sim/tests/held_block.rs` | 98 | 49 |
| `crates/mc-sim/tests/held_block_after_a_reload.rs` | 90 | new |

`chamber.rs` is the one to watch: it had the least headroom of the files this
phase grows and is now at 518 of 600. Nothing else this phase touched is within
100 lines of a ceiling. Two files elsewhere in the tree remain close and were not
touched: `crates/mc-world/tests/mesh_common/mod.rs` at 490 and
`crates/mc-client/tests/support/oracle.rs` at 525.

### One dispute, ruled `test-wrong`, and the fixture it repaired

`mc-client::reload_leaves_the_player_alone::a_candidate_that_would_have_trapped_the_player_and_was_refused_moves_them_nowhere`
was the one workspace failure after the split landed. Three of its four members
matched; only the break's outcome moved, from `Emptied((8, 12, 8))` to
`Refused(Indestructible)`.

**Verdict: `test-wrong`, at the fixture rather than at the assertion.** The
implementation conforms — a swing refused as indestructible on a water cell the
eye is inside is FR-3.3-S1 and FR-3.4-S1 arriving, not a defect in them. That
test grades no scenario of this phase; it grades the reload's clearing path, and
it failed because **its own fixture's stated premise died**. The fixture put
water in the player's head cell and said why: *"it leaves their eye in a cell
they can aim out of."* True while nothing non-solid could be aimed at. False
now — the walk considers the voxel the ray starts in, at distance zero, before it
steps, so a break from inside water stops on the water and `breakable = false`
refuses it. The ceiling is never opened and the way out never exists.

**The repair states the premise instead of inheriting it.** The scenario now
serves a root of its own declaring `base:water` neither solid nor findable by a
swing, and the candidate it is refused for still declares it solid. **That is the
durable half and the reason the repair is correct rather than merely working: a
fixture that inherits a property from content is hostage to every content
change, and this one will not lose the same premise twice.**

**Why the water was not simply moved or removed, which was the first repair
proposed.** The head cell carried *two* reasons and only one died — it is also
the cell the refused candidate would make solid, which is what makes the
candidate one that would have trapped the player further. The player's box spans
exactly two cells, the feet's and the head's, and the feet's must stay stone or
the break frees them. So the head cell is the only cell in which a candidate
declaring water solid traps them further, and emptying it would have left **a
green test that had stopped asking its own question** — the question its name
states. A fixture-registry block was not available either: this is an `mc-client`
test over a copied `content/base`, with no `fixture:` namespace to reach for.

**The repair was measured, not assumed.** With the serving root pointed back at
shipped water and everything else identical, `cargo test -p mc-client --test
reload_leaves_the_player_alone` reproduces exactly the reported failure —
`left: (Indestructible, …)` against `right: (Emptied((8, 12, 8)), …)` — and with
the one serving-root line restored, `2 passed; 0 failed`. One line moves it, so
the repair is load-bearing rather than incidental.

**The falsification is a real defect and is tracked as work of its own, not
closed by this repair.** A player whose eye is inside water can aim at nothing at
all: every swing is refused as indestructible on the water their head is in, and
every placement for want of a face. `base:water` is the only shipped block that
can produce it, being the only one declaring `targetable = true` together with
`breakable = false`, so today the reach is exactly "a player whose eye is in
water" — and it widens the moment any content declares a second targetable
non-solid block. It is out of scope here: no scenario of this spec covers the
origin cell, and the fix is a design decision rather than a mechanical one. The
fixture's doc comments say this too, so nobody meets a quietly-repaired fixture
and learns nothing from it.

### The interface the implementation must provide

Decided here because the tests are its first consumer.

```rust
// mc-sim, replay: the pre-resolved views, renamed because the type now answers
// two questions and should not be named for one of them.
pub struct ResolvedVoxels { /* extent, two bitsets */ }
impl ResolvedVoxels {
    pub fn resolve(volume: &dyn BlockVolume, registry: &BlockRegistry)
        -> Result<Self, RegistryError>;   // unchanged in shape
    /// Records what the voxel at `at` answers to each question, in this order.
    pub fn set(&mut self, at: WorldPos, solid: bool, targetable: bool);
}

// mc-sim, player: the second narrow trait, beside `Solidity` and named for the
// capability rather than for the type that answers it.
pub trait Targetable {
    fn is_targetable(&self, at: BlockPos) -> bool;
}
impl Targetable for ResolvedVoxels { /* … */ }
impl Targetable for World { /* … */ }   // read by `targeted`, as `Solidity` is

// mc-sim, world/action/trace: the walk stops at the first targetable voxel.
pub fn targeted(origin: Vec3, direction: Vec3, reach: f32, world: &dyn Targetable)
    -> Option<Hit>;
```

`Solidity` keeps its meaning at every site that reads it. No test written here
names `Targetable` except `resolved_voxel_updates.rs`, which has no other door to
the second view; every other fixture reaches it through `World` and
`Simulation::advance`, because a test calling the same pure function the adapter
calls is agreement between two copies of one decision.

The placement rule the ruling adds, stated as the tests read it: **when the cell
the ray stopped at holds a block content declares `replaceable`, the placement
lands in that cell rather than one step back.** Stated in terms of the
declaration and never of a block name — a name in that branch is a hardcoded
content decision in Rust, which is invariant 1.

---

## Phase 5 — What a declaration change is classified as, in a save and in a live reload

Nine scenarios: FR-5.1-S1, FR-5.1-S2, FR-5.1-S3, FR-5.2-S1, FR-5.2-S2,
FR-5.3-S1, FR-7.2-S1, FR-7.2-S2, FR-7.2-S3.

| Scenario | File | Test |
|---|---|---|
| FR-5.1-S1 | `crates/mc-world/tests/save_folds_the_split_properties.rs` | `a_block_whose_targetability_alone_moved_records_a_different_behaviour_and_the_same_appearance` |
| FR-5.1-S2 | `crates/mc-world/tests/save_folds_the_split_properties.rs` | `a_block_that_stopped_being_drawn_records_the_same_behaviour_and_a_different_appearance` |
| FR-5.1-S3 | `crates/mc-world/src/persistence/format_test.rs` | `every_shipped_blocks_recorded_behaviour_is_the_fold_an_independent_oracle_computes` **and** `every_shipped_blocks_recorded_appearance_is_the_fold_an_independent_oracle_computes` — one guard per fold, each asserting its own leading byte |
| FR-5.2-S1 | `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | `the_shipped_content_reports_every_block_the_older_save_holds_as_behaving_differently` (the verdict half) |
| FR-5.2-S1 | `crates/mc-client/tests/changed_blocks_named_on_the_error_stream.rs` | `the_committed_pre_luau_save_names_every_block_it_holds_against_the_shipped_content` (the "one line on the error stream" half) |
| FR-5.2-S2 | `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | `the_same_save_loads_by_default_and_is_refused_naming_all_four_when_strictness_is_asked_for` |
| FR-5.3-S1 | `crates/mc-world/tests/save_folds_the_split_properties.rs` | `a_block_that_stopped_occluding_is_reported_as_retextured_and_as_nothing_else` |
| FR-7.2-S1 | `crates/mc-client/tests/reload_marks_sections.rs` | `a_candidate_that_stops_stone_being_drawn_leaves_every_section_of_the_world_to_mesh` |
| FR-7.2-S2 | `crates/mc-client/tests/reload_marks_sections.rs` | `a_candidate_that_stops_stone_occluding_leaves_every_section_of_the_world_to_mesh` |
| FR-7.2-S3 | `crates/mc-client/tests/reload_marks_sections.rs` | `a_candidate_taking_stones_targetability_away_publishes_a_later_serial_and_marks_no_section` |

**FR-5.1-S3 needs both guards and neither alone.** The scenario states two
leading bytes; a single guard over one fold cannot see the other fold's byte fail
to move, and a shared constant would move both together — which is the defect
`format_test.rs`'s own header says the split exists to report. FR-5.2-S1 likewise
has two halves in two crates, because the verdict and the line a player reads are
two different observables and the second lives in the client.

### Additional coverage

| File | Test | What it catches that a scenario's own does not |
|---|---|---|
| `crates/mc-world/tests/save_folds_the_split_properties.rs` | `the_same_comparison_reports_a_block_whose_targetability_moved_as_changed_instead` | FR-5.3-S1 asserts `changed: []`, and an implementation whose `changed` list is *always* empty satisfies that half forever. This is the same comparison over the same two roots with the behaviour property moved instead, so the name moves from one list to the other and the empty list has something saying it can be non-empty. It is also the verdict-level witness that `targetable` is behaviour, where FR-5.1-S1 reads the folds directly. |
| `crates/mc-world/tests/save_per_face_appearance.rs` | `an_unchanged_declaration_records_the_appearance_the_stated_byte_sequence_folds_to` | A **third** hand-built byte oracle, and one no task text named. It states the appearance revision and its field list independently of `format_test.rs`, over a six-distinct-key fixture, so it sees the appended `drawn`/`occludes` bytes and the revision move as well. `docs/technical/world-format.md` already names this file and `format_test.rs` as *the two guards that build the expected bytes by hand*; walking past it would have left one of the two behind. **Its fixture is drawn and see-through**, which is what makes it the witness for the two flags' *order* — see the section below. |
| `crates/mc-client/tests/shipped_binary.rs` | `the_shipped_binary_told_to_refuse_a_changed_save_leaves_it_shut_and_says_why` | FR-5.2-S2 read in a **real process** rather than through a library call. It is the only reading in the workspace that can see the client fail to *print*, and the only one that grades whether the argument reached a decision at all. |

Two further readings moved with the change and are fallout rather than coverage:
`the_same_comparison_reports_a_block_whose_declaration_the_content_no_longer_holds`
(the missing-block control — its three survivors move out of `retextured` into
`changed`) and
`the_shipped_binary_over_a_save_whose_blocks_behave_differently_names_them_on_its_error_stream`
(the process-level witness of the line, now plural).

### How to tell a cancelled run from a complete one, before any count below

`cargo nextest`'s tell is the slash. A **bare** `N tests run` is a complete run; a
slashed `N/M tests run` is a **cancelled** one and says nothing whatever about the
rest. Every count below is bare and every invocation below carries
`--no-fail-fast`, including the ones that would have been sound anyway —
provenance recorded unevenly reads as provenance absent.

### The failing output, with the invocation beside every count

Taken on the working tree this commit records, and the reading dates itself: a
run naming fifteen tests FAILED cannot have come from a tree on which they pass.

```
cargo nextest run --workspace --no-fail-fast --no-tests=pass
```
→ `Summary [51.729s] 1435 tests run: 1420 passed, 15 failed, 1 skipped`

The workspace count recorded for `477137d` was **1 428**, and 1 435 − 1 428 = 7 is
exactly the net count of tests this phase adds: four in
`save_folds_the_split_properties.rs` and three in `reload_marks_sections.rs`.
Every other change is to an existing test's expectation, not to how many there
are.

All fifteen failures are **assertion** failures — no compile error stands in for
one anywhere in this phase — and each fails in the direction the scenario
predicts:

| Test | left (this tree) | right (the scenario) |
|---|---|---|
| `…recorded_behaviour_is_the_fold_an_independent_oracle_computes` | the four shipped blocks' recorded behaviour folds | the same four folded from a byte sequence led by `2` and ending in `targetable` |
| `…recorded_appearance_is_the_fold_an_independent_oracle_computes` | the four shipped blocks' recorded appearance folds | the same four folded from a byte sequence led by `3` and ending in `drawn`, `occludes` |
| `an_unchanged_declaration_records_the_appearance_the_stated_byte_sequence_folds_to` | the fold this build records | the fold of `[3, name, six keys, drawn, occludes]` |
| `a_block_whose_targetability_alone_moved…` | `Folds { behaviour_moved: false, appearance_moved: false }` | `Folds { behaviour_moved: true, appearance_moved: false }` |
| `a_block_that_stopped_being_drawn…` | `Folds { behaviour_moved: false, appearance_moved: false }` | `Folds { behaviour_moved: false, appearance_moved: true }` |
| `a_block_that_stopped_occluding_is_reported_as_retextured…` | `{ missing: [], changed: [], retextured: [] }` | `{ missing: [], changed: [], retextured: [fixture:andesite] }` |
| `the_same_comparison_reports_a_block_whose_targetability_moved_as_changed_instead` | `{ missing: [], changed: [], retextured: [] }` | `{ missing: [], changed: [fixture:andesite], retextured: [] }` |
| `the_shipped_content_reports_every_block_the_older_save_holds…` | `changed: [base:water]`, `retextured: [dirt, grass, stone]` | `changed: [dirt, grass, stone, water]`, `retextured: []` |
| `the_same_save_loads_by_default_and_is_refused_naming_all_four…` | strict arm names `base:water` | strict arm names all four |
| `the_same_comparison_reports_a_block_whose_declaration_the_content_no_longer_holds` | `missing: [water]`, `changed: []`, `retextured: [dirt, grass, stone]` | `missing: [water]`, `changed: [dirt, grass, stone]`, `retextured: []` |
| `the_committed_pre_luau_save_names_every_block_it_holds…` | ``mycraft: `base:water` no longer behaves …`` | ``mycraft: `base:dirt`, `base:grass`, `base:stone`, `base:water` no longer behave …`` |
| `the_shipped_binary_over_a_save_whose_blocks_behave_differently…` | the singular line, from a real child process | the plural four-name line |
| `the_shipped_binary_told_to_refuse_a_changed_save_leaves_it_shut…` | `SaidSomethingElseAboutTheSave` (the child named `base:water` alone) | `RefusedNamingTheBlockAndTheWayOut` over all four |
| `a_candidate_that_stops_stone_being_drawn…` | `(Accepted{dirt}, NoSectionAtAll)` | `(Accepted{dirt}, EverySectionOfTheShippedWorld{marked: 256})` |
| `a_candidate_that_stops_stone_occluding…` | `(Accepted{dirt}, NoSectionAtAll)` | `(Accepted{dirt}, EverySectionOfTheShippedWorld{marked: 256})` |

Scoped runs behind the same numbers, each with its own invocation:

```
cargo nextest run -p mc-world --no-fail-fast -E 'binary(save_folds_the_split_properties) or binary(save_per_face_appearance) or test(/recorded_(behaviour|appearance)_is_the_fold/)'
```
→ `Summary [0.425s] 8 tests run: 1 passed, 7 failed, 350 skipped`

```
cargo nextest run -p mc-world --no-fail-fast -E 'binary(shipped_declarations_and_an_older_save)'
```
→ `Summary [0.085s] 5 tests run: 2 passed, 3 failed, 0 skipped`

```
cargo nextest run -p mc-client --no-fail-fast -E 'binary(changed_blocks_named_on_the_error_stream)'
```
→ `Summary [0.208s] 6 tests run: 5 passed, 1 failed, 0 skipped`

```
cargo nextest run -p mc-client --no-fail-fast -E 'binary(shipped_binary)'
```
→ `Summary [1.239s] 4 tests run: 2 passed, 2 failed, 0 skipped`

```
cargo nextest run -p mc-client --no-fail-fast -E 'binary(reload_marks_sections)'
```
→ `Summary [0.408s] 9 tests run: 7 passed, 2 failed, 0 skipped`

### The lint and the docs, run directly because a green suite is no evidence about either

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
→ `Finished dev profile` — no diagnostic. Run at the gate's own severity: anything
short of `-D warnings` asks a different question, and without it cargo attributes
a diagnostic to the first binary and marks the rest `(1 duplicate)`, which means
*this same diagnostic, repeated* rather than *a pre-existing one lives elsewhere*.

```
RUSTDOCFLAGS='-D warnings -D rustdoc::broken_intra_doc_links' cargo doc --workspace --no-deps --quiet
```
→ exit 0. `cargo clippy` does not resolve intra-doc links, and several doc
comments rewritten here name other items; only rustdoc can report a broken one.

```
cargo fmt --all -- --check
```
→ clean.

### The two byte guards were red first, which the brief predicted they would not be

The phase brief recorded, from `format_test.rs`'s own header, that its two guards
are *green from the moment they are written* and that their falsification has to
be mutation. **That is a property of a guard authored after the change it is
about, and it does not hold here.** Written before the implementation, each guard
states a revision and a field list the fold does not produce yet, so each
disagrees today:

```
FAIL mc-world persistence::format::tests::every_shipped_blocks_recorded_behaviour_is_the_fold_an_independent_oracle_computes
FAIL mc-world persistence::format::tests::every_shipped_blocks_recorded_appearance_is_the_fold_an_independent_oracle_computes
```

So FR-5.1-S3 has a genuine first red run rather than a mutation standing in for
one, and the file's header now says which of the two cases it is in. The mutation
the phase owes for these is still worth running and is still the sharper
instrument for the *asymmetry* — leaving `BEHAVIOUR_REVISION` at 1 while the
behaviour list grows should redden the behaviour guard alone — because that is a
claim about the two constants being independent, which no single red run makes.

### FR-7.2-S3 is green before the change and green after it

Today's geometry key is `(is_solid, textures)` and already ignores `targetable`,
so `a_candidate_taking_stones_targetability_away_publishes_a_later_serial_and_marks_no_section`
passes on this tree:

```
PASS [ 0.277s] (3/9) mc-client::reload_marks_sections a_candidate_taking_stones_targetability_away_publishes_a_later_serial_and_marks_no_section
```

**Recorded as green-before rather than contrived into a red.** A test bent until
it fails once is worse than an honest green with a mutation behind it, and S3 is
the *negative* half of this phase's cross-check: an implementation folding all
five declaration properties into one geometry key passes FR-7.2-S1 and S2 and
fails only here. Its falsification is the phase's own mutation — put `targetable`
into `drawn_of`'s key, and S3 must redden alone.

Its other half carries weight now regardless. The scenario asks for an **accepted**
reload whose **published serial advances**, and both are asserted in the same
comparison as the zero:

- a **refused** reload marks nothing, so `Marking::NoSectionAtAll` alone is
  satisfied by a reload that never happened;
- a reload publishing nothing reports no serial, which `Run::OneReportedNothing`
  is a distinct arm for;
- `Run::EachLaterThanTheLast` is measured against the serial the *launch*
  published, read through `serial_serving`, so a counter that never moves is
  `Run::OutOfOrder` rather than a pass.

### The flag order had one witness and it was the base game's opinion

`drawn` and `occludes` are appended to the appearance list in that order, and the
order is part of the record. The only thing that can see a fold emitting them the
other way round is a block whose two flags **disagree** — and for one commit the
only such block in the workspace was `content/base/blocks/water.luau:28-29`
(`drawn = true, occludes = false`), the one shipped declaration where they differ.

**That is the shape that bit Phase 4**, where a property's only shipped witness was
water and this spec's own change removed it. Here the exposure existed in advance:
the day anybody declares water's two flags alike, or stops shipping water, the
ordering loses the only thing that can see it and a transposed pair produces bytes
nobody compares — without a single test failing.

**The repair is that `save_per_face_appearance.rs`'s fixture now states them
differently** — `DRAWN = true`, `OCCLUDES = false`, a pane of glass — so the
ordering is graded by a fixture nothing outside that file decides. Drawn and
see-through is a real combination rather than a tie-breaker built to be unequal.

**No second test was added, and that is deliberate.** The asymmetric fixture is a
strict improvement on the symmetric one for every defect this file can reach, so a
second test differing only in a fixture value would prove a subset of what the
first now proves, through the same code path — the re-proof `testing.md` §1 names
as worse than no test. One guard over a fixture that can see more is the whole of
what was needed.

**Measured rather than argued, without a mutation window.** The two byte sequences
were folded in a scratchpad with an independent FNV-1a-64 — no tree file touched,
nothing broken and reverted — over this fixture's actual name and six keys:

| Fixture | oracle, `drawn` first | a fold emitting `occludes` first | reddens? |
|---|---|---|---|
| `drawn = true, occludes = false` (now) | `0x0378cf0fd44c0497` | `0x03756a0fd4492321` | **yes** |
| `drawn = true, occludes = true` (before) | `0x0378ce0fd44c02e4` | `0x0378ce0fd44c02e4` | **no — bit for bit identical** |

The same fixture also reddens against a fold answering `true, true` from a
constant instead of from the declaration (`0x0378cf0fd44c0497` against
`0x0378ce0fd44c02e4`); the symmetric one agreed with that too. So the change buys
three defect classes, not one: a transposition, a flag folded twice, and a flag
not read from the declaration at all.

`format_test.rs`'s appearance guard keeps the water witness and its doc comment now
says what that witness is worth — a fact about what the base game currently
declares, which goes quiet without failing — and names the fixture as the one that
cannot.

### One dispute, ruled `test-wrong`, and the fourth byte oracle

`crates/mc-world/tests/save_declarations.rs:95` pinned a behaviour fold to a
constant stating **revision 1**, and `a_solid_breakable_block_that_breaks_into_nothing_records_the_version_1_behaviour`
failed once the behaviour list grew. Verdict **`test-wrong`**: `spec.md` FR-5.1-S3
states the behaviour fold's leading byte is `2`, and `architecture.md` Decision 2
is BINDING that `DeclaredBehaviour` gains `targetable` appended. The test states
what the format used to mean; the implementation conforms.

**The value was re-derived by hand and never taken from a run**, which is the
whole reason the constant exists — the file's own doc comment says a number
copied out of a green run records whatever the writer did that day. The scratch
computation shares no code with the writer, and the fold was checked against
FNV's published vectors before it was pointed at anything:

```
""     0xcbf29ce484222325   ok      "a"      0xaf63dc4c8601ec8c   ok
"foobar" 0x85944171f73967e8 ok
```

The fixture's resolved definition was read off `tests/common/mod.rs:174-184`
(`registry_from`), reached through `registry_of` → `registry_declaring` with
`is_solid = true`. Twenty bytes, one more than before:

```text
  02                          input version 2
  0d                          the name is 13 bytes long
  66 69 78 74 75 72 65        f i x t u r e
  3a                          :
  73 74 6f 6e 65              s t o n e
  01                          solid
  00                          not replaceable
  01                          breakable
  00                          breaks into nothing
  01                          targetable
```
→ `0xbee1_336f_0dc4_f79d`

**The trailing `01` is the fixture's own doing, not a default the loader
derived.** These registries are built in memory, so `defaulting_to_solidity`
never runs; `registry_from` states `targetable: is_solid` and `registry_of`
declares every block solid.

**The check that makes the re-derivation trustworthy is not that it agrees with
the implementation.** The same scratch computation, run over the *nineteen*-byte
version 1 input, reproduces `0x5e9d_3089_5b2e_0d5f` — the constant it replaces,
derived by somebody else at another time. A method that lands on the old number
for the old input is the method the old number was derived by. Only after that
was it compared against the run, and the two agree
(`13754331289031604125` = `0xbee1336f0dc4f79d`), so there was no disagreement to
report ahead of reconciling.

**Four hand-built oracles over the save's declaration lists, and this was the last
one still at the old revision.** Swept with
`grep -rn "cbf2_9ce4_8422_2325\|0000_0100_0000_01b3" --include=*.rs crates/ tools/`
on `fb39188`: `format_test.rs` (both folds), `save_per_face_appearance.rs`,
`save_declarations.rs`, and two in `mc-core` — `tests/hash.rs` and
`tests/art_fold.rs` — which guard the *arithmetic* and a texture-set index and
fold no `BlockDefinition` at all (`grep -c "BlockDefinition\|behaviour_of\|appearance_of"`
→ 0 in both). `mc-render/src/texture/placeholder.rs` restates the constants for an
unrelated production fold.

**How this one was missed is the inverse of §7.1's failure, and together they say
something neither says alone.** `save_per_face_appearance.rs` was **absent** from
T20's fallout sweep and was found by surveying for stated revision bytes.
`save_declarations.rs` **was on that sweep list** and was walked past anyway. So a
list is necessary and not sufficient: **an enumeration can be walked past even when
it is right**, and what actually found both was a grep for the thing itself —
`grep -rn "REVISION" --include=*.rs crates/` — rather than a reading of the list.
The grep is the instrument; the list is a reminder to reach for it.

### The three `docs/modding/` quoted lines, and the producer that keeps two of them honest

`crates/mc-client/tests/documented_refusals.rs` walks `docs/modding/` and compares
every fenced block whose first line begins `mycraft: ` against a set of texts
produced by real runs. Three of those blocks are the changed-blocks line —
`blocks-items.md` once and `hot-reload.md` twice — and all three read the
singular `base:water` sentence today.

After the folds move, the producer that emits them (`launch_notices.rs`'s
`a_launch_over_a_save_whose_block_behaves_differently`, which loads the committed
pre-spec save) emits the **plural four-name** line, and nothing emits the singular
one. On `main`'s ruling, the answer is a second producer rather than a rewritten
page:

`crates/mc-client/tests/support/launch_notices.rs` ·
`a_launch_over_a_save_this_build_wrote_and_one_edited_declaration`

It models the offline loop `hot-reload.md` walks a player through, step for step:
a world holding all four shipped blocks is saved **by this build** against the
shipped content, a copy of the shipped root has the one line `solid = false`
in `blocks/water.luau` replaced by `solid = true` — the shipped text edited rather
than a declaration rewritten, so water's other six fields are exactly as they
ship — and the save is launched against it. Under any revision, the only thing the
save and that root disagree about is water's solidity, so the load reports one
block and the sentence reads in the singular.

Two premises refuse to hand a line back rather than emit a wrong one: the fixture
requires the shipped declaration to hold exactly one line matching the one it
replaces, and the producer requires the load to have reported `base:water` and
nothing else. That second premise is what makes the reading below evidence the
producer works rather than evidence the pages happen to match:

```
cargo nextest run -p mc-client --no-fail-fast -E 'binary(documented_refusals) or binary(documented_property_refusals)'
```
→ `Summary [7.214s] 9 tests run: 9 passed, 0 skipped`

A producer that had failed its premise would have propagated an error out of
`printed_refusals()` and reddened every one of those nine.

**What is still owed, and it is the implementation's.** After the folds move,
`docs/modding/blocks-items.md:724` must become the plural four-name line — that
page is where the "every save in existence" claim lives — while `hot-reload.md`'s
two stay singular, each being true of its own narrative. Nothing in the gate reads
`docs/user/gameplay.md`, which carries the same singular line outside the scanned
root; that one is only ever caught by a grep.

### The singular clause lost a witness in one file and kept two elsewhere

`changed_blocks_named_on_the_error_stream.rs` used to assert both counts of the
sentence, the singular one over the committed save. That save now disagrees about
every block it holds, so both readings in that file are plural. The clause is not
left unwitnessed: `crates/mc-client/src/notice_test.rs` ·
`one_changed_block_is_told_in_the_singular` asserts it over a one-name list, and
the producer above reaches it through a **real load**. A third reading through a
launch was considered and not written — it would exercise the same branch of the
same composer as the unit test, which is a re-proof through one code path rather
than a second witness.

### `THE_CLAUSE` lost its `s`, and that is the fix rather than a loosening

`shipped_binary.rs` waits on its child's error stream for a fragment of the line.
The fragment was `no longer behaves`; the plural line reads `no longer behave as
they did`, which does **not** contain it — so the reading would have waited out its
twenty-second patience and reported the client as never printing. The fragment is
now `no longer behave`, which is a prefix of both spellings and therefore strictly
stronger than the one it replaces: it catches the notice under either count, which
is what the refusal reading at `answered_by` uses it for.

### File sizes on the RED tree, and one worth watching

`grep -c '[^[:space:]]'`, which is what the gate's `Measure-Object -Line` counts.
The limit is 600 for a test file.

| File | Non-blank | Headroom |
|---|---|---|
| `crates/mc-client/tests/shipped_binary.rs` | 589 | **11** |
| `crates/mc-client/tests/support/launch_notices.rs` | 392 | 208 |
| `crates/mc-client/tests/reload_marks_sections.rs` | 381 | 219 |
| `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | 363 | 237 |
| `crates/mc-world/src/persistence/format_test.rs` | 324 | 276 |
| `crates/mc-world/tests/save_per_face_appearance.rs` | 285 | 315 |
| `crates/mc-world/tests/save_folds_the_split_properties.rs` | 272 | 328 |
| `crates/mc-client/tests/changed_blocks_named_on_the_error_stream.rs` | 213 | 387 |

`shipped_binary.rs` went from 578 to 589 non-blank: the four-name line needs a
third continuation line, the changed list is now built from an array rather than a
single `parse`, and the fragment carries a paragraph saying why it stops short of
the verb's ending. **Eleven lines is what is left, and nothing else should go into
that file.** The producer this phase needed was deliberately put in
`launch_notices.rs` for exactly that reason.

### The fixture constraints no assertion can enforce

Held by the code that builds them and by a reader, and stated here because a count
cannot see shape:

- Every root in `save_folds_the_split_properties.rs` states `solid = true` on both
  sides. Each of `drawn`, `occludes` and `targetable` **defaults to whatever its own
  declaration says about `solid`**, so a root saying `solid = false` and nothing
  more states all four as false — four edits rather than one, and the scenarios say
  *differ in nothing but one*.
- For the same reason, `reload_marks_sections.rs`'s two existing solidity scenarios
  still mark the world under the new key, and **not because solidity is in it**:
  `Declaration::of(STONE).solid(false)` states nothing about `drawn` or `occludes`,
  so both default to `false` along with it. A future author who spelled `drawn`
  explicitly beside `solid` in one of those fixtures would silently change what the
  scenario is about. That is now written into the file's own header.
- `save_per_face_appearance.rs`'s fixture states `drawn` and `occludes`
  **differently**, and that is a constraint rather than an incidental value: a
  fixture stating them alike folds the same two bytes in either order and the
  ordering silently loses its only content-independent witness. The section above
  carries the measurement. Do not "tidy" that fixture into a uniformly solid,
  occluding block.
- The committed save fixture is **never regenerated**. It was written from
  `content/base/` while the four blocks were still TOML, with the shipped reader of
  the day. The day it is regenerated it stops being evidence about anything, and
  every FR-5.2 reading rests on it.

### A dead helper, recorded rather than removed

`crates/mc-world/tests/common/mod.rs:109` `block_file(name, texture, solid)` emits
TOML — `name = "…"` / `texture = "…"` / `solid = …` — from before block
declarations moved to Luau. A grep of `crates/mc-world/tests/` for `block_file`
finds no caller. It is dead, and it is left alone: removing it is not this phase's
change and it is nobody's dispute.

### The interface the implementation must provide

```rust
// mc-world, persistence/format.rs — the two folds.
const BEHAVIOUR_REVISION: u8 = 2;
const APPEARANCE_REVISION: u8 = 3;

#[derive(Serialize)]
struct DeclaredBehaviour<'a> {
    input_version: u8,
    name: &'a str,
    is_solid: bool,
    replaceable: bool,
    breakable: bool,
    breaks_into: Option<&'a str>,
    targetable: bool,            // appended, never inserted
}

#[derive(Serialize)]
struct DeclaredAppearance<'a> {
    input_version: u8,
    name: &'a str,
    textures: [&'a str; 6],      // Face::ALL order
    drawn: bool,                 // appended, in this order
    occludes: bool,
}

// mc-sim, world/reload.rs — the geometry key.
fn drawn_of(registry: &BlockRegistry) -> BTreeMap<&BlockName, (bool, bool, &FaceTextures)>
// mapping (declared.drawn, declared.occludes, &declared.textures).
// `solid` leaves this key; `targetable` never enters it.
```

Both structs stay **written out by hand and never derived** from `BlockDefinition`:
a derive would bind every save to a struct that changes for other reasons. The new
fields are **appended after the existing ones**, because the canonical encoding
writes a struct positionally — a rename changes no byte and an insertion in the
middle changes every one. No production signature changes anywhere else; what
changes is the answer.

### Every documentation site this phase touched, and what could see it

Measured on `6e7799b`, clean tree. **The scope of each grep is in the sentence
that reports it**, because the result is worth nothing without it:

- `grep -rn "no longer behave" docs/` — four sites, the changed-blocks line.
- `grep -rn "five fields\|six fields\|nine fields" docs/` — the fold-membership
  and declaration-field counts, across all of `docs/`, not just the modding root.
- `grep -rn "revision 1\|revision 2\|revision 3\|APPEARANCE_REVISION\|BEHAVIOUR_REVISION" docs/`
  — every stated revision number.
- `git diff --stat 2e3a60e..HEAD -- docs/` — what the phase actually edited.

**The instrument holds three of these and is blind to the rest.**
`crates/mc-client/tests/documented_refusals.rs` walks `docs/modding/` and
compares **fenced blocks whose first line begins `mycraft: `** against texts
produced by real runs. Everything else below — including the prose in the same
files, one line down from a block it does hold — is checked by nothing.

| # | Site | Gate | Disposition |
|---|---|---|---|
| 1 | `docs/modding/blocks-items.md:732` | **held** | changed to the plural four-name line |
| 2 | `docs/modding/hot-reload.md:335` | **held** | **defended** — singular, and true of its own narrative |
| 3 | `docs/modding/hot-reload.md:519` | **held** | **defended** — the page's own loop moves one block |
| 4 | `docs/modding/blocks-items.md:718` | blind | five fields → **six** |
| 5 | `docs/modding/blocks-items.md:725` | blind | new: a build adding a field names every block in the save |
| 6 | `docs/user/gameplay.md` §water | blind | the attribution sentence — see below |
| 7 | `docs/user/gameplay.md` §old save | blind | rewritten as two records moving at two builds |
| 8 | `docs/user/gameplay.md:131` | blind | the line reframed as a mod's ordinary job |
| 9 | `docs/user/gameplay.md:140` | blind | **defended** — singular, now correct for the mod case |
| 10 | `docs/user/gameplay.md:265` | blind | the retexture paragraph, restated for both records |
| 11 | `docs/technical/world-format.md:658` | blind | appearance list at revision 3 |
| 12 | `docs/technical/world-format.md:667` | blind | behaviour list at revision 2, and the cost |
| 13 | `docs/technical/world-format.md:680` | blind | all four reported changed |
| 14 | `docs/technical/world-format.md:702` | blind | why one fold cannot see the other's byte |
| 15 | `docs/technical/world-format.md:711` | blind | the pinned fold in `save_declarations.rs` |
| 16 | `docs/technical/world-format.md:717` | blind | the measured blast radius |
| 17 | `docs/technical/testing.md` | blind | what the documentation guard holds and does not |
| 18 | `docs/INDEX.md:76` | blind | **STALE — see below** |

**Eighteen rows at claim grain against the phase's own count of thirteen**, and
the difference is bookkeeping — `world-format.md`'s six paragraphs and
`gameplay.md`'s five were counted as two sites there — **except for row 18,
which no count carried.**

### Row 18: `docs/INDEX.md:76` is stale on the tree about to be gated

The routing row for `blocks-items.md` still reads *"all **six** fields with type,
default and bound"* and *"which **five** fields a save folds into a block's
recorded behaviour"*. Both are false:

- `docs/modding/blocks-items.md:63` is `## The nine fields` — nine since Phase 1,
  so that half has been wrong for four phases.
- `docs/modding/blocks-items.md:718` now says **six**, so the other half was
  falsified by this phase.

Found by `grep -rn "five fields\|six fields\|nine fields" docs/` on `6e7799b`,
which is a grep across **all** of `docs/` rather than the modding root — the
narrower sweep every phase ran would not have reached `INDEX.md` at all. Nothing
in the gate reads it. **Not fixed here**: this is a documentation file and the
last write of this spec is this one, so it is reported rather than taken.

### The mutations, and the one that bit differently than anyone predicted

**M1 — `targetable` into `drawn_of`'s geometry key.** FR-7.2-S3's own test
reddened **alone**. That is the falsification the scenario could not get from a
first red run, because it is green before the change and green after it: an
implementation folding all five declaration properties into one key passes
FR-7.2-S1 and S2 and fails only there.

**M2 — `BEHAVIOUR_REVISION` back to 1 with `targetable` still folded.** Both
**behaviour**-stating guards reddened and **neither appearance**-stating one did.
So the asymmetry the two revision constants exist to produce is measured rather
than argued: two bytes that always moved together would have been
indistinguishable from one byte, and nothing had ever tested that they do not.

**M2's third result is the finding, and it is worth more than either
confirmation: the whole-verdict guard over the committed pre-spec save stayed
GREEN under M2.** It looks like it covers the revision byte. It provably does
not.

**Why, derived rather than relayed** — a scratchpad fold over `fixture:stone`'s
own field list, touching no tree file and needing no window:

| Fold | Bytes | Value |
|---|---|---|
| what a revision-1 build recorded | `01` + name + 4 flags | `0x5e9d_3089_5b2e_0d5f` |
| M2: byte left at 1, list grown | `01` + name + 4 flags + `01` | `0xf326_d765_ef40_b6ba` |
| correct: byte moved, list grown | `02` + name + 4 flags + `01` | `0xbee1_336f_0dc4_f79d` |

**Both the mutant and the correct answer differ from what was recorded**, so both
classify every block as `changed` and the verdict is identical under either. A
comparison that only ever asks *"does this differ from what the save recorded?"*
cannot see the leading byte at all — the byte is one input to a fold whose
output is only ever tested for **inequality** against a value from another era.

That is not a defect in the guard, which was written to assert what a player is
told. It is a **limit that now has a measurement behind it**, and it is the
reason FR-5.1-S3 is asserted as a hand-built byte sequence in two files rather
than through any verdict: the byte oracles are the only witnesses in the
workspace that compare a fold to a *stated* value instead of to a stale one.

### Two of the three gate-visible sites were defended rather than fixed

Rows 2 and 3 were **correct as they stood** and the work was establishing that,
not editing them. `hot-reload.md:519` walks a player through writing a save with
the current build and then editing one declaration, so under the new revisions
only that block's behaviour moves and the singular line is the factually correct
thing for the page to show.

**A mechanical sweep matching every quoted line to the new answer would have
broken both**, and it would have looked like diligence. Two of three is not a
rounding error — it is the majority of the sites the instrument can see.

So a sweep has to **report what it declined as well as what it changed**. A sweep
that reports only its edits is indistinguishable from one that missed the sites
it left alone, and the sites it should leave alone are exactly the ones where the
document is right and the test is the thing under suspicion. That is the same
ordering `testing.md` §2 gives for an over-tight assertion: measure before
choosing, because the cheapest way to green is sometimes to break something
correct.
