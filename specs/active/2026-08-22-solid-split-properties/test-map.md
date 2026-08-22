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
exactly the message the modding guide prints. `docs/modding/blocks-items.md:396`
**already quotes that refusal**, and
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

## Two skeletons, because no single one reddens all four FR-1.1 scenarios

Measured, not argued (`standards/global/testing.md` §2, "One skeleton is often
not enough"). Command:
`cargo nextest run -p mc-world -E 'binary(luau_declaration_properties)'`.

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

## Mutations run, and what each proved

Each was applied by hand, observed, and reverted by hand; the tree was confirmed
clean against `HEAD` afterwards.

| Mutation | Result |
|---|---|
| **M1** — `RECOGNISED_FIELDS` in `luau_declaration_keys.rs` left at six, loader at nine | Exactly one test red: `a_field_one_letter_past_a_real_one_is_refused_quoting_every_field_in_declaration_order`. `a_field_the_loader_does_not_recognise_is_refused_beside_the_ones_it_does` stayed **green** — the measured proof that the filtered reading is blind in this direction. |
| **M2** — `FIELDS_IN_THE_ORDER_THE_GUIDE_STATES` left at six, loader at nine | `the_guide_introduces_the_declaration_fields_in_the_order_a_refusal_quotes_them` red; `the_modding_guide_states_every_per_facing_refusal_in_the_recognised_field_order` stayed **green** — the same blindness, one mirror over. |
| **M3** — the loader's nine reordered (`occludes` before `drawn`), page untouched | `a_field_one_letter_past_a_real_one_…` red, reporting both orders. Nothing else in `luau_declaration_keys` moved, and the reddening does not route through the modding page. |

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
