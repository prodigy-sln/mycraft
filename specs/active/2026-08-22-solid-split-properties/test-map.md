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
