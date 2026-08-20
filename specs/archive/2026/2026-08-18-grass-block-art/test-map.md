# Test map: The grass block looks like a grass block

**Spec**: [spec.md](spec.md) (SPEC-019, rigor `high`) ·
**Tasks**: [tasks.md](tasks.md) · **Architecture**: [architecture.md](architecture.md)

Every scenario maps to at least one test. Extra tests live under each phase's
**Additional coverage** heading, one line each stating what that test catches — a
test whose purpose is not written down is one nobody can later judge, which is how
a bogus test survives.

## Ownership

**The mapping below is the tasks stage's proposal, not a binding contract.** Each
phase's test author is the feature's first consumer and owns that phase's test
files, names and interface decisions for the whole phase. They **revise this table
in place** — correcting a file, a name, or a split into two tests — and record the
arrival colours they actually measured. The implementation context never edits a
test file.

What *is* binding is the left-hand column: every scenario listed here is owed a
test, and `architecture.md`'s interfaces are binding on how it is reached.

## The count is computed, never restated

From this folder:

```
grep -oE "FR-[0-9]+\.[0-9]+-S[0-9]+:" spec.md | sort -u | wc -l
grep -oE "^\| FR-[0-9]+\.[0-9]+-S[0-9]+ " test-map.md | grep -oE "FR-[0-9]+\.[0-9]+-S[0-9]+" | sort -u | wc -l
```

The two must agree and their `comm -3` must be empty. A count kept by hand beside
the thing it counts drifts, and this one has drifted three times inside this spec —
so no number appears in this file's prose.

---

## Phase 1 — The stable fold moves to `mc-core`

**No scenarios.** This phase exists for one assertion that cannot be written after
P2 changes what an appearance is.

### Additional coverage

| Test | File | What it catches |
|---|---|---|
| `the_fold_of_a_stated_byte_sequence_is_the_value_the_constants_compute` | `crates/mc-core/tests/hash.rs` | the published fold changing value, judged at the surface `voxforge` will reach rather than through the one caller `mc-world` happens to be — five stated byte sequences against expected values unrolled from the two constants at compile time, never a snapshot. **Written after T01 and green.** It is the workspace's **only** witness on an empty input (F4), and its two-bytes/same-two-reversed pair is what catches a fold that tallies instead of folding (F3) |
| `every_shipped_blocks_recorded_behaviour_is_the_fold_an_independent_oracle_computes` | `crates/mc-world/src/persistence/format_test.rs` | a behaviour hash moving while the appearance hash legitimately does; the oracle re-implements FNV in the test and shares no code with `mc_core::hash`. **Written before T01 and green** |
| `every_shipped_blocks_recorded_appearance_is_the_fold_an_independent_oracle_computes` | `crates/mc-world/src/persistence/format_test.rs` | **the assertion this phase exists for.** After P2 the appearance half changes by design, and FR-9.1-S4 never observes the two halves together. Only writable now. **Written before T01 and green** |

### Arrival colours, as measured

**Both `format_test.rs` guards were authored before T01 and are green on arrival —
deliberately, and it is the point of the ordering.** They assert the outputs of
`behaviour_of` and `appearance_of` against an oracle they compute themselves, and
neither of those is changed by moving a private fold between crates. A guard
written *after* the move can only agree with whatever the move produced; written
before it, it converts "no stored hash changed value" from something asserted
afterwards into something **witnessed across the change**.

Nothing here could therefore be red first, so falsifiability comes from the
phase's mutations instead — M1 and M3 are the instruments, and `format_test.rs`
says so in its own header.

**`crates/mc-core/tests/hash.rs` was written after T01 and is green too, and its
RED could not be recovered either.** The RED it should have had was the compile
error of naming `mc_core::hash` before that module existed; T01 landed first —
correctly, because T02's ordering was this phase's whole point — and that window
closed with it. No substitute was manufactured. Its falsifiability is F3 and F4
in `tasks.md`, and the file says so in its own header rather than claiming a
colour it never showed.

**Both guards therefore rest on measured mutation rather than on a displayed
RED, and that is stated here so a reader is not left inferring it from a green
suite.**

### Notes for the test author

- `behaviour_of` and `appearance_of` are `pub(crate)`, so the sibling
  `format_test.rs` is the only vehicle. That layout is why the gate excludes
  `*_test.rs` from the coverage denominator.
- No expected value in this phase may be copied from a run of the code under test.
- The oracle builds the canonical bytes **by hand** — the length-prefixed text,
  the flag bytes, the present/absent byte — rather than calling the encoder the
  save uses. It agreed with the encoder on the first run, which is two
  independent derivations meeting.
- **P2 revises both `format_test.rs` guards**, and by that phase's test author
  rather than by an implementer: an appearance gains six keys and
  `STATED_INPUT_VERSION` in the test moves 1 → 2 with `INPUT_VERSION`.
  `crates/mc-core/tests/hash.rs` is untouched by that — it folds stated bytes and
  knows nothing about what a declaration is.
- `crates/mc-core/tests/hash.rs` adds **no** dev-dependency. That crate's manifest
  comment states an empty `[dev-dependencies]` as a property, because
  `cargo metadata`'s resolve nodes include them; the guard uses `std` and
  `mc_core` alone and keeps that property intact.

---

## Phase 2 — A block declares six facings, and a save records them

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-1.1-S1 | `crates/mc-world/tests/luau_declaration_textures.rs` | `a_texture_stated_as_one_string_registers_that_key_on_all_six_facings` |
| FR-1.1-S2 | `crates/mc-world/tests/luau_declaration_textures.rs` | `a_table_of_six_different_keys_registers_each_facing_with_the_key_written_against_it` |
| FR-1.1-S3 | `crates/mc-world/tests/luau_declaration_textures.rs` | `a_key_named_against_both_up_and_down_is_registered_against_both` |
| FR-1.1-S4 | `crates/mc-sim/tests/per_face_layers.rs` | `six_facing_keys_naming_five_distinct_values_spend_five_layers` |
| FR-1.2-S1 | `crates/mc-world/tests/luau_declaration_texture_refusals.rs` | `a_table_stating_three_facings_is_refused_naming_the_three_it_did_not_state` |
| FR-1.2-S2 | `crates/mc-world/tests/luau_declaration_texture_refusals.rs` | `an_empty_table_is_refused_naming_all_six_facings_as_unstated` |
| FR-1.2-S3 | `crates/mc-world/tests/luau_declaration_texture_refusals.rs` | `a_table_carrying_top_is_refused_naming_the_six_facings_a_table_may_state` |
| FR-1.2-S4 | `crates/mc-world/tests/luau_declaration_texture_refusals.rs` | `a_table_carrying_a_capitalised_facing_is_refused_naming_the_six_a_table_may_state` |
| FR-1.2-S5 | `crates/mc-world/tests/luau_declaration_texture_refusals.rs` | `a_facing_holding_a_number_is_refused_reporting_that_it_must_be_a_string` |
| FR-1.2-S6 | `crates/mc-world/tests/luau_declaration_texture_refusals.rs` | `a_facing_holding_two_namespace_separators_is_refused_reporting_that` |
| FR-1.2-S7 | `crates/mc-world/tests/luau_declaration_texture_refusals.rs` | `a_texture_stated_as_a_boolean_is_refused_reporting_the_two_forms_it_may_take` |
| FR-1.2-S8 | `crates/mc-client/tests/documented_refusals.rs` | `the_modding_guide_states_every_per_facing_refusal_in_the_recognised_field_order` |
| FR-1.4-S1 | `crates/mc-sim/tests/per_face_layers.rs` | `six_unassigned_facing_keys_added_to_content_spending_250_layers_spend_256` |
| FR-1.4-S2 | `crates/mc-sim/tests/per_face_layers.rs` | `six_unassigned_facing_keys_added_to_content_spending_251_layers_refuse_the_load` |
| FR-1.4-S3 | `crates/mc-sim/tests/per_face_layers.rs` | `a_load_refused_for_want_of_layers_leaves_every_assigned_layer_holding_its_key` |
| FR-9.1-S1 | `crates/mc-world/tests/save_per_face_appearance.rs` | `two_blocks_differing_only_in_their_north_key_record_different_appearances` |
| FR-9.1-S2 | `crates/mc-world/tests/save_per_face_appearance.rs` | `an_unchanged_declaration_records_the_appearance_the_stated_byte_sequence_folds_to` |
| FR-9.1-S3 | `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | `a_save_written_before_appearances_folded_facing_keys_reports_its_blocks_as_retextured` |
| FR-9.1-S4 | `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | `a_save_written_before_this_format_is_still_loaded_without_the_player_being_asked` |

**FR-9.1-S3 and S4 moved**, from a proposed new `save_per_face_appearance.rs` to
the file that already owns the committed pre-spec save. `tests/fixtures/world_saved_against_the_toml_declarations.mcw`
is the only save in this repository genuinely written before this spec — a save
this suite wrote itself would agree with the writer by construction — so a new
file would have had to borrow that fixture and its whole header. What stayed in
`save_per_face_appearance.rs` is FR-9.1-S1 and S2, which are about what the
writer records rather than about what a loaded save reports.

### Additional coverage

| Test | File | What it catches |
|---|---|---|
| `every_facing_maps_to_one_face_and_every_face_back_to_one_facing` | `crates/mc-world/tests/mesh_facing_faces.rs` | drift between two enums for six directions — D1's own strongest objection, closed mechanically over both `ALL` arrays. It does **not** catch a north/south swap; FR-1.3-S2 in P3 is the only witness for that, and the file says so in its own header |
| `a_declarations_metatable_supplies_no_facing_it_did_not_state` | `crates/mc-world/tests/luau_declaration_textures.rs` | the texture table being read through anything other than `read_field`/`field_names` — the module header's raw-read property, extended one level down. **Green on arrival for the wrong reason: see below** |
| `a_declarations_metatable_hides_no_facing_it_did_state` | `crates/mc-world/tests/luau_declaration_textures.rs` | the same property from the other side; an `__iter` that hides a name the table holds. **Green on arrival for the wrong reason: see below** |
| `the_shipped_root_spends_one_layer_per_key_it_declares` | `crates/mc-sim/tests/per_face_layers.rs` | the shipped root growing a fifth texture key. Every arithmetic expectation in that file adds six to a count that assumes the shipped four; without this the growth would move three unrelated counts by one and be diagnosed as a budget defect |
| `a_candidate_re_pointing_one_facing_of_a_block_leaves_every_section_of_the_world_to_mesh` | `crates/mc-client/tests/reload_marks_sections.rs` | `changes_geometry` narrowing back to a single key — a reload re-pointing one facing would otherwise be accepted and mark no section at all. **Added because M10 measured that it had no witness at all, and falsified by M10 re-run against it: 1 of 1248, this test alone** |
| `a_candidate_restating_the_same_six_facing_keys_leaves_no_section_to_mesh` | `crates/mc-client/tests/reload_marks_sections.rs` | its control: a comparison that marked on *any* table-formed declaration would satisfy the reading above and be caught only here. **Green under that same M10 re-run**, which is what says the pair discriminates rather than both reddening together |
| `the_same_comparison_reports_a_block_whose_declaration_the_content_no_longer_holds` | `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | (existing, revised) a missing block and a retextured one arriving in one another's list. It is the positive control FR-9.1-S3's verdict cannot supply for itself |

### Where every number comes from

- **Five** (FR-1.1-S4) = the count of *distinct* values in the six-key fixture,
  computed by the test from that fixture. Not a digit and not a snapshot.
- **256** (FR-1.4-S1) = `A_SESSIONS_BUDGET - 6`, already spent, plus the six the
  block introduces. **257 against 256** (FR-1.4-S2) = `A_SESSIONS_BUDGET - 6 + 1`
  plus six, against the budget written out as two to the eighth. Both arithmetic,
  in the test, and the budget is written out rather than read from
  `LAYERS_A_SESSION_MAY_ASSIGN` — which is the declaration under test.
- FR-9.1-S2's appearance is built byte by byte in the test — revision byte, the
  length-prefixed name, then six length-prefixed keys in `Face::ALL` order — and
  folded by a second FNV-1a-64 written out from the published constants. Nothing
  in that file calls the fold it judges.

### Arrival colours, as measured

Measured against a throwaway T04 skeleton — `Face`, `FaceTextures`, the `textures`
field, `Facing::face`, the uniform form only, the table form refusing everything —
applied, run, and **reverted by hand**, with `git diff --exit-code` clean over
`crates/*/src` afterwards. Nothing from it landed. Without it every RED in this
phase would have been a compile error, which `testing.md` §1 accepts only for a
scenario about a type existing, and none of these are.

`cargo nextest run --workspace --all-features --no-fail-fast`: **1246 tests, 19
failed, 1 skipped.** Every failure is an assertion, none is a panic, and no test
outside this phase's own files reddened — which is the evidence that the
`texture` → `textures` adaptation across twenty-five call sites changed no
behaviour.

| Test | Arrival | Why |
|---|---|---|
| FR-1.1-S1 | **green — the control** | the uniform form already works, and all four shipped blocks use it. A skeleton that broke it would be caught here and nowhere else |
| FR-1.1-S2, S3 | red | `Refused("… `texture` must be a string, but is a table")` against the six facings expected |
| FR-1.1-S4, FR-1.4-S1, S2, S3 | red | `RefusedOtherwise("the content root's blocks could not be read")` — the table form refuses, so the root does not load at all |
| FR-1.2-S1 … S7 | red | on the words the refusal names: `[]` where three or six facings are owed, and `false` where the offending word is owed |
| FR-1.2-S8 | red | the guide quotes none of the seven |
| FR-9.1-S1 | red | `(true, true, true)` against `(true, false, true)` — the appearance does not move when `north` alone changes |
| FR-9.1-S2 | red | `Some(17874791196503406030)` against `Some(9094986339778992341)` |
| FR-9.1-S3 | red | `retextured: []` against all four blocks |
| FR-9.1-S4 | **green — the control** | it asserts a hash that must *not* move, so it is green before and must stay green after. Its falsifier is the mutation below, not a displayed red |
| `format_test.rs` appearance guard | red | six keys under revision 2 against the skeleton's one key under revision 1 |
| `format_test.rs` behaviour guard | **green** | the behaviour fold is unmoved, which is the whole claim P1 bought and FR-9.1-S4 carries forward |
| `mesh_facing_faces.rs` | green | the skeleton implements the mapping correctly; it is a completeness guard, not a driver |

### The two guards that are green for the wrong reason, stated rather than left

`a_declarations_metatable_supplies_no_facing_it_did_not_state` and
`a_declarations_metatable_hides_no_facing_it_did_state` both assert that a
declaration **is refused**, blaming the file, the block and `texture`. Under a
skeleton whose table form refuses *everything* they pass without the raw reads
existing at all.

**They start meaning something only once the table form is accepted**, and until
then they are two more witnesses that a table is refused rather than two
witnesses about metatables. Whoever closes this phase owes them the falsifier
that says otherwise: read one facing through a believing path — `pairs` rather
than `field_names`, or an indexed read rather than `read_field` — and check that
exactly these two redden while `a_table_of_six_different_keys_…` stays green.
A pass that has never been falsified is what this file exists to stop being
mistaken for evidence.

### Weak instruments and overlaps, named

- **FR-1.2-S2 cannot separate the two shapes of refusal on its own.** For an
  empty table the facings that were not stated *are* the six a table may state, so
  both wordings satisfy it. FR-1.2-S1 is what separates them, and that is why both
  scenarios exist rather than one.
- **FR-1.4-S3 is structurally hard to falsify through the door it uses.**
  `mc_sim::content::load` takes `&LayerAssignment`, so a refusal cannot mutate what
  it was handed. The test therefore asks a second question the assignment can only
  answer if nothing happened to it — the layer the *next* key it meets takes — and
  that half is falsifiable. The map comparison beside it is the scenario's own
  words and is honest about being green by construction today; it is a guard
  against a `&mut` door arriving later.
- **The blamed field is asserted for five of the seven refusals and not for
  FR-1.2-S5 and S6.** Those two say what a refusal must *report*, not what it must
  blame, and pinning `up` or `texture` there would be an over-tight assertion whose
  cheapest repair is to change working code.

### The fixture constraints no assertion can enforce

- FR-1.1-S2 needs six keys that are **pairwise distinct and none of them the
  block's own name**. This one *is* enforced — `six_distinct_keys` refuses a list
  that repeats a key or names the block, as a fixture failure — because a count of
  registered keys cannot see either mistake.
- FR-9.1-S2's six keys are **deliberately not in alphabetical order**. A fold that
  sorted the keys before folding them agrees with any fixture whose declared order
  is already sorted, and nothing in the assertion can see that.
- The seven refusal fixtures each carry **exactly one** thing wrong. A root is
  refused whole, so a declaration with two mistakes is refused for whichever the
  loader reaches first and the second refusal is one no run ever prints.

### Mutations run by the test author

Run in a **detached worktree at `3a8787d`** — the state in which the rejected
option would actually have been taken — with its own `CARGO_TARGET_DIR`, removed
afterwards. The shared tree was never opened for it. A blast radius is only
meaningful against a stated tree, so the commit is part of the measurement.

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
| M-shared | `const INPUT_VERSION: u8 = 1` → `2` — one number for both field lists, exactly as `architecture.md` and `tasks.md` T09 originally said to write it | three files redden: both `format_test.rs` halves, `save_declarations`'s pinned version-1 behaviour, and the pre-spec-save guard | **Bit: 5 of 1224**, `--no-fail-fast`. The same three places, one test wider than predicted. `format::tests::…_recorded_appearance_…` and `…_recorded_behaviour_…`; `save_declarations::a_solid_breakable_block_that_breaks_into_nothing_records_the_version_1_behaviour` (`Some(14380961474726025582)` against `Some(6817658777407196511)`) — **a file P2 does not own**; and both guards in `shipped_declarations_and_an_older_save` |

**The decisive reading, and it is worse than "a hash moved".**

```
left:  RegistryVerdict { missing: [], changed: [base:dirt, base:grass, base:stone, base:water], retextured: [] }
right: RegistryVerdict { missing: [], changed: [],                                              retextured: [] }
```

The four blocks land in **`changed`**, not in `retextured`. Those are not two
spellings of one outcome: `RegistryVerdict::refuses` turns a non-empty `changed`
into a refusal under `Acceptance::OnlyUnchangedBlocks`, so under the shared bump
**every save written before this spec is refused at load**, and what the player
is told is that the blocks they built with behave differently. Nothing behaves
differently. They were retextured. With the per-list split the same four land in
`retextured`, which refuses nothing and prompts nobody.

**So the split was necessary rather than tidier, and the cost of the alternative
is a measured number rather than an argument.**

### The zero-witness gap M10 found, and what closed it

**M10 — `drawn_of` comparing only the `up` key — did not bite: 0 of 1246**, measured
by the implementer and recorded in `tasks.md`. T10's "no new scenario is owed" and
`rendering.md:497-510`'s section-count assertions were both wrong about this, for a
reason no assertion could see: **every fixture in the workspace that reloads a
changed texture changes it as a single string**, so a one-key comparison and a
six-key one agree on all of them. Nothing anywhere re-pointed one facing and left
the other five alone — which is the only edit the widening exists for.

What that left in the tree: `changes_geometry` could narrow back to one key with the
suite green, while a reload re-pointing `north` would be accepted and mark **no
section at all** — the edit lands, the world is never built again, and there is no
error anywhere.

The two readings above close it, **in P2 rather than deferred**, because
`testing.md` §1 is explicit that a gap is closed where it is found and not left to
the phase that happens to revisit the code — and D3 means P3 revisits nothing here.

**Both are green on arrival and that is not a defect in them.** T10 is already
correct, so there is no red to display; their falsifiability is M10 biting where it
did not before, re-run by the hand that measured the 0 so the before and after sit
on one instrument.

**Measured: M10 re-applied against them reddens 1 of 1248 — the re-pointing reading
alone, with the nothing-changed control green.** Both numbers matter. The first says
the gap is closed by something that can fail; the second says the pair
*discriminates*, since a comparison marking on any table-formed declaration would
have reddened both. The same row in `tasks.md` carries it beside the original 0 of
1246, so the record holds the gap and its closing rather than just the gap.

Three fixture constraints hold those two readings up and none is enforceable by an
assertion: the block is declared in a file sorting **after** all four the game ships,
so registration order does not move and the held block is still `base:dirt`; **five
of its six facings hold the block's own name**, because a quad's layer is still
resolved by parsing that name while PRO-902 is open and a block no facing names holds
no layer at all; and the block is **never placed**, so nothing meshes it and what is
read is the marking alone. The first and third are the fixture's; the second is
structural — `Declaration::repointing_north` is the only door and writes the other
five itself.

### What P1 bought, collected one phase later

The fifth failure is `format_test.rs`'s **behaviour** half. That guard was
written before `fnv_1a_64` moved crates, deliberately, so that "no stored hash
changed value" could be witnessed across a change rather than asserted after it —
and P1 exists as a phase for the sake of its ordering.

**Here it is reporting the exact defect it was written for, one phase later, in a
mutation nobody had it in mind for.** That is the clearest vindication available
of ruling T02 ahead of T01: a guard written after the shared bump would have
agreed with whatever the bump produced, and the two revision constants would have
looked like one number doing one job.

### This finding must outlive the spec folder

`spec-disposal: archive` prunes this file; `docs/` is the only as-built record.
A future reader will open `format.rs`, see **two revision constants where there
was one**, and want to unify them. They will be right by every rule they can see —
it is duplication, and it is two numbers doing one job — and wrong by the one they
cannot, because the reasoning will have been pruned with this folder.

**The rule and its cost are owed to `docs/technical/world-format.md`** (T11's
engine-reader half), stated together:

> The revision byte is per field list. Behaviour and appearance are independently
> versioned, and unifying them turns a retexture into a refused load — measured:
> every save written before this spec reported all four of its blocks as
> `changed` rather than `retextured`, which `Acceptance::OnlyUnchangedBlocks`
> refuses.

A rule with a cost attached survives an argument. A rule without one loses to a
tidy-up.

---

## Phase 3 — Resolution at packing, and at the indicator

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-1.3-S1 | `crates/mc-client/tests/per_face_axes.rs` | `the_key_declared_for_up_is_drawn_on_the_positive_y_face_and_on_no_other` |
| FR-1.3-S2 | `crates/mc-client/tests/per_face_axes.rs` | `north_draws_along_negative_z_and_south_along_positive_z` |
| FR-1.3-S3 | `crates/mc-client/tests/per_face_axes.rs` | `east_draws_along_positive_x_and_west_along_negative_x` |
| FR-2.1-S1 | `crates/mc-render/src/geometry/mod_test.rs` | `a_face_draws_the_key_its_block_declared_when_that_key_is_not_its_name` |
| FR-2.1-S2 | `crates/mc-sim/tests/shared_texture_keys.rs` | `two_differently_named_blocks_declaring_one_texture_key_spend_one_layer` |
| FR-2.1-S3 | `crates/mc-render/src/geometry/mod_test.rs` | `a_facing_key_outside_the_assignment_refuses_the_section_naming_the_block_and_the_facing` |
| FR-2.1-S4 | `crates/mc-client/tests/retained_sections_repack_against_the_serving_content.rs` | `sections_repacked_without_being_remeshed_draw_the_key_the_serving_content_declares` |
| FR-2.1-S5 | `crates/mc-client/tests/reload_refuses_an_uncovered_facing_key.rs` | `a_reload_naming_a_facing_key_with_no_layer_is_refused_and_the_sections_keep_drawing` |
| FR-2.2-S1 | `crates/mc-render/src/hud/held_test.rs` | `the_indicator_draws_the_layer_assigned_to_the_held_blocks_north_key` |
| FR-2.2-S2 | `crates/mc-render/src/hud/held_test.rs` | `the_indicator_draws_the_declared_key_rather_than_nothing_when_the_name_is_not_the_key` |
| FR-2.2-S3 | `crates/mc-render/src/hud/held_test.rs` | `a_north_key_with_no_layer_reports_the_indicator_unresolved_naming_the_block_and_the_key` |
| FR-2.2-S4 | `crates/mc-render/src/hud/held_test.rs` | `an_empty_hand_reports_nothing_held_rather_than_the_block_last_held` |

**FR-1.3's three are authored against a block placed in a section, meshed by the
real mesher and packed by the real packer**, with each face identified by **where
its corners landed** rather than by the `Facing` the quad carries. The mapping
function is never called: a test that asked it which face `NegZ` is would be two
copies of one decision agreeing with each other, which is exactly the shape M6
measured at 0 of 1246.

### FR-2.1-S4 — the one test that must be authored at the packer

**It is the only witness separating design option (a) from option (b), and a test
written against a reload passes under both.** `changes_geometry` marks every
section on a texture-key change and `take_remesh_work` drains the whole dirty set
into one batch, so retained-but-not-re-meshed is unreachable through a production
reload.

The seam that is real: `Retained::rebuilt` re-packs the **entire** retained list on
every batch. The test retains a list meshed under content A through the world's own
whole-world mesh, hands the packer content B's resolution, re-packs, and reads the
layer back through `SectionGeometry::layer_at`.

**It arrives red, and its red is not about option (a).** Under both skeletons the
north key is never consulted, so neither content's key reaches a corner — an honest
assertion failure, but not the one this test exists for.

**M14 as `tasks.md` states it is not this test's falsifier, and that is measured
rather than argued.** M14 targets `Retained::rebuilt`; this test packs the retained
list itself, which is what `SectionGeometry::layer_at` requires and what the trap
prescribes, so `rebuilt` never runs. Applied as "the worker never adopts a retired
resolution", M14 reddens **2 of 1261 — both in `reload_supersedes_a_batch_in_flight`,
and FR-2.1-S4 is not among them.** `tasks.md`'s row predicts FR-2.1-S4 and is wrong
about where the mutation lands; the row it needs is below.

**What this test does discriminate, measured.** A packer that resolves once and
reuses that answer — which is what a quad remembering a mesh-time key amounts to at
this seam — reddens **4 of 1261**, and FR-2.1-S4 is one of them. The other three are
`reload_draws_the_new_block` and the same two supersede readings, none of which is
about a retained list being re-packed. So the assertion is non-vacuous in exactly
the direction the design question runs, and it is the only reading that asks it of
the retained list.

The two halves of the assertion are both load-bearing: it reads both re-packs, so an
implementation that resolved once fails on the first (the new key is absent) while
the second proves the fixture can tell the two contents apart.

### Additional coverage

| Test | File | What it catches |
|---|---|---|
| `a_quad_naming_a_block_the_content_states_nothing_about_is_refused_with_no_key` | `crates/mc-render/src/geometry/mod_test.rs` | the `key: None` arm of the widened refusal, which no scenario reaches — a section still holding quads for a block a reload dropped. It replaces the pre-phase refusal test, whose subject (a block whose *name* has no layer) no longer exists |
| `the_unresolved_report_names_the_facing_the_indicator_looked_at` | `crates/mc-render/src/hud/held_test.rs` | the report losing the facing in a later tidy. A block declares up to six keys and one of them is the one without art; a sentence naming the block and the key but not the facing leaves the author unable to tell a wrong declaration from a missing image |
| `two_blocks_declaring_two_texture_keys_spend_two_layers` | `crates/mc-sim/tests/shared_texture_keys.rs` | FR-2.1-S2's control. "Two blocks spend one layer" is satisfied by a loader reporting one for anything at all, and by one that folded two distinct keys together and drew both blocks from one texture |
| `a_block_whose_declared_texture_is_not_its_own_name_packs_from_the_key_it_declared` | `crates/mc-client/tests/stated_layers_are_honoured.rs` | FR-2.1-S1's property through **`ContentView`**, the client's own construction of a resolution out of resolved content. That is the shipped route and the mapped test does not take it: every other reading hands the packer a resolution a fixture wrote out by hand, so a lookup keyed on a block's name would survive there |
| `the_held_block_indicator_shows_the_layer_of_the_key_the_block_declared` | `crates/mc-client/tests/stated_layers_are_honoured.rs` | the same for FR-2.2-S2, over the same view, which is what holds the two PRO-902 sites together at the seam that produces the resolution |

**Both of the last two are inversions, not new tests.** They were written as pins on
the gap — they asserted that a block declaring a key other than its name drew
*nothing* — and `reload_keeps_packed_layers.rs`'s header and `support/reload.rs`'s
`Declaration::of` both said in writing that the red they turn when PRO-902 closes
is the announcement. **`spec.md`'s "Test premises this spec invalidates" does not
list them**, which is a fourth planning premise this spec has met and found
incomplete. The prose in both of those files is retired in the same commit.

### The proposed test that is not here, and why

`a_quad_carries_no_resolved_texture_key` was proposed as the structural half of
FR-2.1-S4. It is dropped. A `Quad` built by exhaustive struct literal already fails
to compile if a seventh field arrives, which every fixture in the workspace does,
so a test asserting it adds nothing; and a runtime version — the same quads packed
against two resolutions — re-proves FR-2.1-S4 through the same function, which
`testing.md` §1 forbids. The design question is M14's and is not otherwise
reachable.

### Arrival colours, as measured

**Two throwaway skeletons, not one.** `testing.md` §1 is explicit that one is often
not enough, and it is not here: the phase's own skeleton resolves through the
*declaration* — correctly for a renamed block — so it leaves both PRO-902 witnesses
green. Each was applied in a **detached worktree at `7406b22`** with its own
`CARGO_TARGET_DIR`, run, and removed; the shared tree never held either, and
`git status` over it after the worktree was removed showed no change outside test
files.

- **Skeleton A** — the plan's: `TextureResolution::key_of` resolves every facing to
  `Face::Up`'s key. `cargo nextest run --workspace --all-features --no-fail-fast`:
  **1261 tests run, 1252 passed, 9 failed, 1 skipped.**
- **Skeleton B** — the behaviour that shipped before this phase, wearing the phase's
  signatures: a block resolves to the key its own *name* spells and the facing is
  ignored. Same command: **1261 tests run, 1248 passed, 13 failed, 1 skipped.**

Every failure under both was an assertion, and **no test outside this phase's own
files reddened under either** — which is the evidence that adapting twenty-four
existing test files to the new signatures changed no behaviour.

Every diff below is quoted with **the skeleton that produced it**. A red attributed
to the wrong column reads as coverage of a property that column never touched.

| Scenario | Skeleton A | Skeleton B | The diff, and which skeleton produced it |
|---|---|---|---|
| FR-1.3-S1 | **red** | red | **A:** `(Some(3), [NegX, PosX, NegY, PosY, NegZ, PosZ])` against `(Some(3), [PosY])` — the layer is right and the *set of faces drawing it* is all six |
| FR-1.3-S2 | red | red | **A:** `(Some(3), Some(3))` against `(Some(0), Some(6))` |
| FR-1.3-S3 | red | red | **A:** `(Some(3), Some(3))` against `(Some(1), Some(2))` |
| FR-2.1-S1 | green | **red** | **B:** `Refused { block: "example:amber", face: West, key: Some("example:amber") }` against twenty-four corners of layer 1. A resolves through the declaration, so a renamed block already draws its declared key and this column is silent |
| FR-2.1-S2 | green | green | neither — the only scenario with no red; see below |
| FR-2.1-S3 | red | red | **A:** `(Corners([0,0,0,0]), …)` against `(Refused { block, face: North, key: Some("example:unlit") }, …)` |
| FR-2.1-S4 | red | red | **A and B alike:** `Repacked { meshed: false, serving: false }` twice, against `serving`-only and `meshed`-only. Neither red is about option (a); see the section above |
| FR-2.1-S5 | red | red | **A:** `Corners([2, 2, 2, 2])` against `Corners([255, 255, 255, 255])`, either side of the refusal, with the refusal's own sentence equal on both sides |
| FR-2.2-S1 | red | red | **A:** `[4]` against `[5]` — the `up` key's layer where the `north` key's is owed |
| FR-2.2-S2 | green | **red** | **B:** `[]` against `[6]` — no indicator at all where the declared key's layer is owed. A is silent for the same reason as FR-2.1-S1 |
| FR-2.2-S3 | red | red | **A:** `(7, 1, false)` against `(2, 0, true)` — a swatch and its ring drawn where none is owed |
| FR-2.2-S4 | green | red | **B:** `(false, None, None, 0)` against `(true, …)`. It asserts a totality, and B trips its own fixture guard: the first half is that the same block *did* resolve while it was held |

Two additional-coverage readings sit only in one column each and are recorded here so
neither is mistaken for a scenario's own: `the_unresolved_report_names_the_facing_the_indicator_looked_at`
is **red under A alone** (B reports unresolved for its own reason, and the report does
name the facing), and the two `stated_layers_are_honoured.rs` inversions are **red under
B alone** — `RefusedNaming("example:amber")` against four corners of layer 0, and
`Unresolved { … face: North, key: Some("example:amber") }` against
`Shows { key: "example:quartz", face: North }`.

**FR-1.3-S1 was predicted green as a control and is red.** The prediction was about
the first half of the scenario; the scenario also says *and on no other face*, and
under a skeleton that draws `up`'s key everywhere that half fails. The prediction
was right about what a control is for and wrong about this scenario having only one
clause.

### The one scenario with no red, stated rather than left to be inferred

**FR-2.1-S2 is green under both skeletons and under both mutations, and it is green
because P2 already satisfies it**: `BlockRegistry::texture_keys` unions what the six faces declare, so
two blocks sharing a key already spend one layer. Nothing a deficient
implementation of *this phase's* interfaces does can move it.

Its falsifiability is its own control rather than a mutation. The two readings share
one fixture shape and differ in exactly one string — the second block's declared key
— and they report **1** and **2**. A loader answering a constant, or one folding two
distinct keys together, fails one of them. Both numbers are counted from the fixture
by the test; neither is a digit.

### Weak instruments and overlaps, named

- **FR-2.1-S1 and FR-2.2-S2 are one property at two sites** and neither covers the
  other. M12 and M13 are what say so, and they are the phase's own instruments for
  the fact that closing one site while leaving the other draws a block correctly in
  the world with a blank indicator beside it.
- **FR-2.1-S3's `face` assertion is a second witness for `north` ↔ negative Z**,
  reached through `Facing::face` directly rather than through a placed block. It is
  not a substitute for FR-1.3-S2 and does not weaken it: it compares a hand-written
  `Face::North` against a quad the fixture built facing `NegZ`.
- **`the_unresolved_report_names_the_facing_the_indicator_looked_at` pins a
  decision**, not a derivation: it asserts the report carries the word `north`,
  written out, rather than `INDICATOR_FACE.as_str()`, which would be two copies of
  one constant. Changing the indicator's facing is meant to fail here.
- `reload_remesh_blocks_no_tick` is load-sensitive and is kept on exactly its
  sensitive path by this phase's widening of `changes_geometry`. **Re-run before
  diagnosing**; read the occurrence as the verdict arm plus that test's captured
  stderr. It passed on both arrival runs.

### The fixture constraints no assertion can enforce

- **`per_face_axes.rs`'s six keys are minerals in alphabetical order and the facings
  they are declared against are not.** A key spelled `example:northward` would let
  the mapping be recovered from the fixture, which is the one thing that file must
  not allow. `six_keys` *does* enforce pairwise distinctness, because a count of
  registered keys cannot see two facings sharing one.
- **Every layer table in this phase disagrees with the sorted order of its own
  keys.** A reading taken against a lexicographic assignment cannot fail whatever
  the packer did, because both sides would be the same sort. `staged_layers::assigned`
  is what reaches such an assignment the only way a session can.
- **`held_test.rs`'s unstocked block has an uncovered `north` and a covered `up`.**
  Only the first is what FR-2.2-S3 is about; the second is what stops a lookup
  consulting the wrong facing from reporting the same unresolved answer for the
  wrong reason.
- **FR-2.1-S5's launch content re-points `north` before the client ever runs**, and
  the session it launches into has exactly one layer free. That is what makes the
  launch fit and the reload not fit, and it is what makes the "keeps drawing" half
  a statement about a per-facing key rather than about the block's own name.
- **FR-2.1-S4's two contents are read through `mc_sim::content::load`, the second
  against the first's assignment.** Appending is what a reload does, so the second
  key lands beside the first rather than renumbering it; a fixture that built two
  assignments independently could have given the two keys the same index and made
  both readings true of either. `require_distinct` refuses that as a fixture failure.

### Mutations run by the test author

Run in the same detached-worktree arrangement, on top of a **throwaway reference
implementation** of the phase — skeleton A with `key_of` corrected to consult the
face it is given, which is the whole of what the two skeletons withheld. Each was
applied, run with `--no-fail-fast`, and **reverted by hand**, never with
`git checkout --`. The shared tree was never opened for any of them.

| # | Mutation | Predicted | Outcome |
|---|---|---|---|
| — | *(none — the reference implementation itself)* | every test green, or a test of this phase's is over-tight | **1261 run, 1261 passed, 1 skipped.** Nothing here is red that should be green — the trap `testing.md` §2 names as *red that should have been green*, whose cheapest repair is to break working code |
| M14, as `tasks.md` states it | `Message::Retire` never adopts the resolution it is sent, so the worker keeps packing against the one its quads were meshed under | `tasks.md`: FR-2.1-S4 reddens, and it is the sharpest mutation in the spec | **Bit 2 of 1261, and FR-2.1-S4 is not among them.** Both are in `reload_supersedes_a_batch_in_flight`: `a_batch_meshed_against_content_that_stopped_serving_is_discarded_and_the_next_one_is_drawn` and `a_batch_that_cannot_be_packed_is_reported_and_the_worker_still_draws_the_next_one`. Real coverage of a real property, and **not the option-(a) question** |
| M14', the one FR-2.1-S4 is aimed at | `build_section_geometry` freezes the resolution it is first handed and resolves every later section from it — what a quad remembering a mesh-time key amounts to at this seam | FR-2.1-S4 reddens; the two PRO-902 witnesses stay green | **Bit 4 of 1261**, with FR-2.1-S4 among them, plus `reload_draws_the_new_block` and the same two supersede readings. **FR-2.1-S1 and FR-2.1-S2 stayed green**, which is what says the mutation is aimed at the retained list rather than at resolution in general |

**M14's row in `tasks.md` needs both halves.** As written it names a red and no green,
and a falsifier that reddens everything it touches proves nothing about what it is
aimed at. The green half, measured: FR-2.1-S1, FR-2.1-S2 and every FR-2.2 reading stay
green under M14' — so the two PRO-902 sites and the layer arithmetic are untouched by
it, and the only thing it moves is what a *re-packed* section draws.

### The lint, run directly

A green suite is no evidence about a lint, and this phase's window has no compilable
tree in the shared checkout until the implementation lands. So
`cargo clippy --workspace --all-targets --all-features -- -D warnings` was run inside
the worktree, against both skeletons, and **is clean**. It was not clean first: it
named four findings in `per_face_axes.rs` (two over the 30-line function limit, one
`indexing_slicing`, one `map_err_ignore`), two `map_err_ignore` in the sibling unit
files, and an `excessive_nesting` plus a fifth over-long function in the retained
test. All are fixed in the committed files rather than allowed.

**`crates/mc-client/src/app/mod.rs` is untouched by this commit.** T12 is the
implementer's and nothing here edits that file.

### Notes for the implementer

- `ContentView::layers`/`into_layers` become `resolution`/`into_resolution`;
  `PreparedScene.layers` and `PreparedLaunch.layers` become `resolution`;
  `Retained.layers` becomes `resolution`; `Unuploaded` wraps a `TextureResolution`.
  Every test file in the workspace is already written against those names.
- `TerrainRenderer::upload_textures` still takes `&TextureLayers`. Only
  `FrameRenderer::upload_textures` takes the whole resolution, because it is the one
  that also answers a swatch. Three harnesses pass `resolution.layers()` for that
  reason and it is not an oversight.
- `HeldSwatch::Shows` gains `face`, and `Unresolved` gains `face` beside its
  `key: Option<TextureKey>`. `None` now means *the content states no such block*
  rather than *the name is not a texture key*, and the two sentences differ.

---

## Phase 4 — The models say truthfully where they came from

**No scenarios and no tests.** FR-8.2 was retired on 2026-08-18 with all seven of
its scenarios (FR-8.2-S1 through S7); `spec.md`'s FR-8.2 entry carries the
retirement and the reason. The phase that remains repairs one citation, gives each
generator a header saying it is provenance rather than a build step, and writes
`docs/modding/voxel-models.md` — none of which a test can grade, because none of it
is behaviour.

**Nothing is owed here and nothing is unwitnessed.** The distinction matters: an
empty mapping in this file usually means a phase whose tests live elsewhere. This
one has no tests to live anywhere.

---

## Phase 5 — `voxforge build`

**The implementation is split at the FR-3.1/FR-3.2 seam** (write and cache, then
refuse and fold), by the team lead's ruling. **The tests are not split with it.**
All twenty-eight were authored before any implementation existed, by one author
who is not replaced between the two legs, so the seam is an implementation
boundary and not a file boundary — a second author arbitrating tests it did not
write would be strictly worse than one author carrying both legs.

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-3.1-S1 | `tools/voxforge/tests/build_writes_a_set.rs` | `a_manifest_of_seven_entries_writes_seven_images_each_named_for_its_key` |
| FR-3.1-S2 | `tools/voxforge/tests/build_writes_a_set.rs` | `a_manifest_of_seven_entries_writes_one_index_naming_all_seven_keys` |
| FR-3.1-S3 | `tools/voxforge/tests/build_writes_a_set.rs` | `every_written_path_is_reported_on_the_builds_output` |
| FR-3.1-S4 | `tools/voxforge/tests/build_determinism.rs` | `two_builds_of_unchanged_sources_produce_byte_identical_images` |
| FR-3.1-S5 | `tools/voxforge/tests/build_writes_a_set.rs` | `the_bottom_face_of_the_grass_model_is_written_as_the_dirt_keys_art` |
| FR-3.1-S6 | `tools/voxforge/tests/build_writes_a_set.rs` | `a_manifest_with_no_entries_writes_an_index_naming_no_keys_and_reports_no_images` |
| FR-3.2-S1 | `tools/voxforge/tests/build_cache.rs` | `a_build_whose_output_is_current_leaves_every_images_bytes_unchanged` |
| FR-3.2-S2 | `tools/voxforge/tests/build_cache.rs` | `a_build_whose_output_is_current_reports_that_nothing_needed_rebuilding` |
| FR-3.2-S3 | `tools/voxforge/tests/build_cache.rs` | `editing_a_model_the_manifest_names_rewrites_the_images_derived_from_it` |
| FR-3.3-S1 | `tools/voxforge/tests/build_refusals.rs` | `a_manifest_path_naming_no_file_refuses_the_build_naming_the_path_given` |
| FR-3.3-S2 | `tools/voxforge/tests/build_refusals.rs` | `a_manifest_that_is_not_toml_refuses_the_build_reporting_where_parsing_stopped` |
| FR-3.3-S3 | `tools/voxforge/tests/build_refusals.rs` | `a_model_file_that_does_not_exist_refuses_the_build_naming_the_path_and_the_key` |
| FR-3.3-S4 | `tools/voxforge/tests/build_refusals.rs` | `a_face_that_is_not_one_of_the_six_refuses_the_build_naming_the_six_selectable` |
| FR-3.3-S5 | `tools/voxforge/tests/build_refusals.rs` | `two_entries_naming_one_texture_key_refuse_the_build_naming_the_key_stated_twice` |
| FR-3.3-S6 | `tools/voxforge/tests/build_refusals.rs` | `a_key_with_two_namespace_separators_refuses_the_build_reporting_that` |
| FR-3.3-S7 | `tools/voxforge/tests/build_seams.rs` | `a_selected_face_whose_opposite_edges_disagree_refuses_the_build_naming_the_edge` |
| FR-3.3-S8 | `tools/voxforge/tests/build_seams.rs` | `a_manifest_whose_selected_faces_all_tile_completes_the_build` |
| FR-3.3-S9 | `tools/voxforge/tests/build_seams.rs` | `a_model_that_is_not_cubic_refuses_the_build_naming_the_axis_that_disagrees` |
| FR-3.3-S10 | `tools/voxforge/tests/build_all_or_nothing.rs` | `a_refusal_on_the_fourth_entry_leaves_the_previous_builds_output_unchanged` |
| FR-3.3-S11 | `tools/voxforge/tests/build_reports_unused_keys.rs` | `a_key_no_block_file_spells_is_named_as_unused_and_the_build_completes` |
| FR-3.3-S12 | `tools/voxforge/tests/build_refusals.rs` | `a_key_whose_image_name_would_not_be_an_ordinary_file_name_refuses_the_build` |
| FR-3.3-S13 | `tools/voxforge/tests/build_refusals.rs` | `a_key_carrying_a_line_break_refuses_the_build_as_unwritable_to_an_index` |
| FR-3.3-S14 | `tools/voxforge/tests/build_seams.rs` | `a_model_whose_scale_times_pixels_per_voxel_is_not_the_edge_refuses_the_build` |
| FR-3.4-S1 | `tools/voxforge/tests/build_fold.rs` | `editing_a_material_the_manifest_reached_records_a_different_value` |
| FR-3.4-S2 | `tools/voxforge/tests/build_fold.rs` | `editing_a_model_the_manifest_names_records_a_different_value` |
| FR-3.4-S3 | `tools/voxforge/tests/build_fold.rs` | `editing_a_model_no_entry_names_records_the_same_value` |
| FR-3.4-S4 | `crates/mc-core/tests/art_fold.rs` | `the_recorded_value_is_the_fnv_fold_of_the_stated_byte_sequence` |
| FR-3.4-S5 | `tools/voxforge/tests/build_fold.rs` | `a_source_that_cannot_be_read_while_folding_refuses_the_build_naming_it` |

### Additional coverage

| Test | File | What it catches |
|---|---|---|
| `an_index_that_is_rendered_and_parsed_again_states_the_same_fold_sources_and_entries` | `crates/mc-core/tests/art_index.rs` | the two halves of D7's contract drifting apart. The fold it round-trips has a **high zero byte**, so a renderer that stopped padding to sixteen digits emits a line a strict parser refuses |
| `a_first_line_that_is_not_the_magic_is_refused_naming_it` | `crates/mc-core/tests/art_index.rs` | `IndexError::NotAnIndex` unconstructible in a test — an arm nobody has read. Includes the empty text, which is what an empty file offers |
| `an_unknown_leading_word_is_refused_naming_the_line_and_the_word` | `crates/mc-core/tests/art_index.rs` | `UnknownRecord` — a future record type silently passed over rather than refused, and a blank line, which is the same thing spelled with nothing |
| `a_fold_that_is_not_sixteen_hex_digits_is_refused` | `crates/mc-core/tests/art_index.rs` | `MalformedFold` — a truncated fold parsing as a smaller number that compares unequal to every real one forever, with no message anywhere. Six spellings, including a line 2 that is not a fold at all |
| `a_path_that_is_absolute_or_carries_a_parent_component_is_refused` | `crates/mc-core/tests/art_index.rs` | `UnsafePath` — a reader resolving a recorded path outside the content root it was given. **Both an absolute POSIX path and an absolute Windows one**, per Trap 3's lesson; and `a..b.toml` sits in the round trip's sources as the boundary a substring check would wrongly refuse |
| `a_key_or_source_carrying_a_control_character_is_refused_on_render_and_on_parse` | `crates/mc-core/tests/art_index.rs` | `ControlCharacter`, **and it is the index-forging case**: without it, a key of `base:a` followed by a newline and `fold 0000000000000000` is a spellable manifest entry that makes `rendered()` emit an index `parse` reads with a forged fold. Graded from both sides in one test, because refusing on one leaves either a writer that can emit the forgery or a reader that believes one |
| `an_index_naming_one_key_twice_is_refused_naming_the_key` | `crates/mc-core/tests/art_index.rs` | `DuplicateKey` on the *parse* side; FR-3.3-S5 covers only the manifest side |
| `two_sources_whose_bytes_and_paths_could_be_re_split_fold_to_different_values` | `crates/mc-core/tests/art_fold.rs` | the length prefixes being dropped — `("ab", "")` against `("a", "b")`, which concatenate to the same two bytes without them. **This is M25's target and it bites** |
| `a_material_that_is_not_a_toml_file_does_not_move_the_fold` | `tools/voxforge/tests/build_fold.rs` | folding the whole materials directory rather than the `*.toml` the build actually reads — a stray note making the set stale for an input nothing consumed |
| `a_blocks_directory_that_is_not_there_reports_every_key_as_unused_and_completes` | `tools/voxforge/tests/build_reports_unused_keys.rs` | the advisory scan hardening into a refusal **by the I/O door** — a root that ships no blocks yet refusing a perfectly good art build. FR-3.3-S11's own test cannot see it: that root has a blocks directory |

### Where every number comes from

- FR-3.4-S4's expected folds are **arithmetic in the test**: three byte sequences
  written out as explicit `[u8; N]` literals — length prefixes spelled byte by
  byte — folded by a loop over the two published constants **restated** in the
  test file. Nothing on the expected side reaches the crate under test and
  nothing was taken from a run. The constants are restated rather than imported
  because they are private to `hash.rs`, and reading a constant out of the module
  being judged makes a changed constant invisible to the guard that exists to see
  it.
- The three cases are one source, two sources, and **the same two reversed**,
  whose stated values differ — which is what says the fold is over an ordered
  sequence rather than a tally.
- "Seven entries" and every expected image name in `build_writes_a_set.rs` are
  read off the committed manifest and derived by D9's rule. **The count seven is
  the one literal**, and it is the scenario's own: without it a manifest that came
  to state nothing satisfies every derived assertion in that file.
- The ramp fixture's `85` and `255` are derived, not observed: `255 / 3 = 85`
  exactly, so four tones over sixteen columns step by 85 within a row and by 255
  across the wrap.

### Interface decisions this phase's tests make binding

- **`mc_core::art::INDEX_FILE_NAME`** — additive to `architecture.md`'s listing,
  approved by the team lead. voxforge writes the index and `mc-client` reads it;
  two string literals that must match is the "agreement between two copies of one
  decision" hazard, and a shared constant makes the mismatch unspellable.
- **`IndexError::ControlCharacter { line, field, spelled }`** — the arm gains
  `spelled` over `architecture.md`'s two fields, by the team lead's ruling. The
  line is the one the record occupies in a **rendered** index: literal on the
  parse side, and on the render side the line a reader would later see it at. One
  convention in both directions, because a different one per direction would be
  worse than an arbitrary one. But at render time that file does not exist yet, so
  a number alone is not actionable — the offending key or path travels with it,
  escaped.
- **`stating` validates control characters and nothing else.** It does not refuse
  a duplicate key or an unsafe path; `parse` refuses both. The build refuses a
  duplicate at the manifest, which is where an author can see it.
- **The unused-key scan tolerates a missing blocks directory**, reporting every
  key as unused and completing. Advisory means advisory whether the files are
  missing or the keys are.

### Arrival colours, as measured

Two throwaway skeletons and a throwaway reference implementation, each applied in
a **detached worktree at `48397a8`** with its own `CARGO_TARGET_DIR` outside the
repository, run, and removed. The shared tree was never opened for any of them and
holds nothing but the test files.

- **The reference implementation** — a complete, correct `mc_core::art` and
  `voxforge build`, written to answer one question: *is anything here red that
  should be green?* `cargo nextest run --workspace --all-features --no-fail-fast`:
  **1299 tests run, 1299 passed, 1 skipped.** Nothing is over-tight. 1299 is the
  1261 the branch carried plus this phase's 38.
- **Skeleton A** — the tasks' own: `build` exists and writes nothing, over a naive
  `art` that validates nothing, renders its fold unpadded, parses leniently and
  folds without length prefixes. Same command: **1299 run, 1263 passed, 36 failed,
  1 skipped.**
- **Skeleton B** — the reference with each image written as it is emitted, before
  any verdict is judged. Same command: **1299 run, 1297 passed, 2 failed, 1
  skipped.** This is M28 exactly.

**No test outside this phase's own files reddened under either skeleton.**

| Scenario | Skeleton A | Skeleton B | The diff, and which skeleton produced it |
|---|---|---|---|
| FR-3.1-S1 | **red** | green | **A:** `(Success, 7, [])` against the seven derived image names |
| FR-3.1-S2 | red | green | **A:** `(Success, 7, Absent)` against `Naming([…seven keys…])` |
| FR-3.1-S3 | red | green | **A:** the seven paths plus the index unnamed on stdout, against `nothing_missing()` |
| FR-3.1-S4 | red | green | **A:** `(Success, 0, {})` against `(Success, 7, {…})` |
| FR-3.1-S5 | red | green | **A:** `(premise, 6, false, false, false)` against `(premise, 6, true, true, false)` — the premise holds on both sides, which is the point of putting it there |
| FR-3.1-S6 | red | green | **A:** `(Success, [], Absent)` against `(Success, [], Naming([]))`. **Predicted green and it is red** — see below |
| FR-3.2-S1 | red | green | **A:** `(Success, 3, {})` against the tampered snapshot |
| FR-3.2-S2 | red | green | **A:** `(Success, false, false)` against `(Success, false, true)` |
| FR-3.2-S3 | red | green | **A:** `(Success, 0, [])` against `(Success, 2, [])` — see the vacuity note below |
| FR-3.3-S1…S6, S12, S13 | red | green | **A:** `(Completed(0), [])` against `(NamingEverything, [])`, all eight |
| FR-3.3-S7 | red | **red** | **B:** `(NamingEverything, Only(Edges), ["base__one.png"])` against `(…, [])` — refused correctly, having already written the image |
| FR-3.3-S8 | red | green | **A:** `(Success, [], Absent)` against two images and `Naming(["base:one", "base:two"])` |
| FR-3.3-S9 | red | green | **A:** `(Completed(0), [])` against `(NamingEverything, [])` |
| FR-3.3-S10 | red | **red** | **B:** four of seven images carrying different bytes from the previous build's, against every one of them untouched |
| FR-3.3-S11 | red | green | **A:** `(Success, 0, false, false)` against `(Success, 2, true, false)` |
| FR-3.3-S14 | red | green | **A:** `(Completed(0), [])` against `(NamingEverything, [])` |
| FR-3.4-S1, S2 | red | green | **A:** `(Success, Unavailable)` against `(Success, Moved)` |
| FR-3.4-S3 | red | green | **A:** `(Success, Unavailable)` against `(Success, Stayed)` |
| FR-3.4-S4 | red | green | **A:** three folded values against three stated ones, all three different |
| FR-3.4-S5 | red | green | **A:** `(Completed(0), [])` against `(NamingEverything, [])` |

**`Unavailable` is what makes the FR-3.4 rows mean something.** A build that
writes no index at all would otherwise read as *the value did not move*, which is
the passing answer for FR-3.4-S3.

Additional-coverage readings: the six `art_index` arm tests and both `art_fold`
tests are red under A; the round trip and `NotAnIndex` are **green under both**
and are covered by mutation instead (below).
`a_material_that_is_not_a_toml_file_does_not_move_the_fold` and
`a_blocks_directory_that_is_not_there_reports_every_key_as_unused_and_completes`
are red under A.

### FR-3.1-S6 was predicted green as a control and is red

`tasks.md` predicts it green because "a build that writes nothing writes zero
images", and adds that the index must be asserted present or the reading is
vacuous. The test does assert that — `Naming([])` against `Absent` — so the
prediction describes a weaker test than the one that was owed and written. The
disagreement is information: **the control the phase was warned about is closed,
and there is no vacuity-green left in FR-3.1.**

### The vacuity that was found and fixed, and the fixture that was rebuilt

Both were found by measurement rather than by reading, and both were repaired
before this commit.

- **FR-3.2-S3 was green under skeleton A for the wrong reason.** It asserted that
  every image the first build wrote carries different bytes afterwards — and a
  build that writes nothing wrote none, so the list of changed images was empty on
  both sides. The count of images the first build wrote is now the middle member
  of the assertion.
- **FR-3.3-S10's first fixture could not be reddened by skeleton B.** It refused
  the fourth of seven entries by naming a missing *model*; the build groups by
  model, so that refusal landed before any image of the changed model was reached,
  and an eager build had written nothing different by then. The fixture now keeps
  one model, replaces it with the ramp between the two builds, and has the fourth
  **entry** select the ramp's `front` face — which does not tile, while the six
  around it do. Skeleton B then lands four changed images and the reading reddens.

### Mutations run by the test author, and what they are worth

Run in the same detached-worktree arrangement on top of the throwaway reference
implementation, each applied, run with `--no-fail-fast`, and reverted by restoring
a byte-exact copy — never with `git checkout --`. **A mutation against a throwaway
tells you about the throwaway**, so what these buy is evidence about each
mutation's *aim*, not about shipped code: **every row below must be re-run by the
implementer against the implementation that ships.**

| # | Mutation | Predicted | Outcome, measured here |
|---|---|---|---|
| — | *(none — the reference implementation itself)* | every test green, or a test of this phase's is over-tight | **1299 run, 1299 passed, 1 skipped** |
| M24 | `folded_sources` uses `DefaultHasher` | FR-3.4-S4 reddens | **2 of 1299**: FR-3.4-S4 and the length-prefix reading. Every build reading stays green — the fold's *value* moves and nothing about the build does |
| M25 | Drop the length prefixes from the fold | `tasks.md`: **predicted non-bite** unless a test constructs the forging pair | **It bites: 2 of 1299**, the same two. The pair is constructed, so the additional-coverage test the row asks for already exists and no gap is owed |
| M26 | Fold every file under `content/` | FR-3.4-S3 reddens; every positive staleness scenario stays green | **5 of 1299.** FR-3.4-S3 reddens and so does the `*.toml`-only reading; **FR-3.4-S1 and S2 stay green**, which is the half the row is about. The other three are collateral and stated rather than hidden: folding everything folds the build's own output directory, so the set is never current (FR-3.2-S1 and S2), and the missing model is no longer folded, so FR-3.3-S3's message loses the key |
| M27 | The cache reports current unconditionally | FR-3.2-S3 reddens | **4 of 1299**: FR-3.2-S3, FR-3.4-S1, FR-3.4-S2 and FR-3.3-S10. FR-3.2-S1 and S2 stay green, which is what says the mutation moved the *staleness* answer and not the reporting |
| M28 | Bypass `written_together`, write each entry as it is emitted | FR-3.3-S10 reddens | **This is skeleton B: 2 of 1299** — FR-3.3-S10 and FR-3.3-S7. Everything else green |
| M29 | Accept any derived image name | FR-3.3-S12 reddens | **1 of 1299**: FR-3.3-S12, and nothing else. The sharpest possible aim |
| M30 | `emit` called per entry with `FaceSelection::One` | FR-3.3-S9 reddens | **Mis-aimed as first written, and replaced rather than reported.** Taking the first entry's face for a whole model group reddens **17 of 1299** — it breaks every image, and a mutation that reddens everything it touches says nothing about what it is aimed at. **M30′**, the honest form — `emit` once per entry with that entry's own face, images correct throughout — reddens **exactly 1 of 1299: FR-3.3-S9.** That is the row's claim, isolated |
| M31 | Refuse on a seam verdict for a face no entry selected | FR-3.3-S8 reddens; FR-3.3-S7 stays green | **2 of 1299**: FR-3.3-S8 as predicted, plus FR-3.3-S10, whose fixture now turns on the same distinction. **FR-3.3-S7 stays green**, which is the stated green |
| M32 | The unused-key report becomes a refusal | FR-3.3-S11 reddens | **7 of 1299**: FR-3.3-S11, its blocks-directory sibling, and the five shipped-manifest readings — five of the committed manifest's seven keys are declared by no block file **today**, so a refusal takes the whole shipped build down with it. Every refusal and fold reading stays green |
| M33 | *(added)* `rendered` drops the zero padding while `parse` stays strict | the round trip reddens — it is green under both skeletons | **1 of 1299**: the round trip, and nothing else |
| M34 | *(added)* `parse` accepts any first line | `NotAnIndex` reddens — it is green under both skeletons | **1 of 1299**: `a_first_line_that_is_not_the_magic_is_refused_naming_it`, and nothing else |
| M35 | *(added)* a current set is re-encoded and rewritten anyway | FR-3.2-S1 reddens; FR-3.2-S2 stays green | **1 of 1299**: FR-3.2-S1. This is the mutation the tampering exists for — identical bytes rewritten are invisible to any comparison that does not leave a marker behind |

**Nothing in M24–M32 was left unrun**, and M33–M35 were added because the two
`art_index` readings green under both skeletons, and the FR-3.2-S1 tamper, had no
falsifier otherwise. Every row above has a stated red **and** a stated green.

### FR-3.2-S1 pins a deliberate limitation, not only a behaviour

D13 has the cache check the fold and the **presence** of every image the index
names, not their contents. So a tampered image surviving a build is the *correct*
consequence of a stated decision. **If a later spec makes the cache validate image
content, this test goes red and that red is correct** — the repair is to change
the test, never to weaken the cache back. Recorded here because a red met without
this note reads as a regression and gets repaired on the wrong side.

### The fixture constraints no assertion can enforce

- **`Root::shipped()` deletes the output directory after copying.** The built set
  is gitignored but *present* on any machine that has run a build — and from P6
  the gate runs one. A copy carrying it finds its output current, does no work, and
  every reading in `build_writes_a_set.rs` grades somebody else's build. No
  assertion in those tests can see this; the constructor holds it.
- **The ramp cube's four tones must stay four.** The edge leg compares the wrap
  against the largest interior step *in the same line*, so a fixture whose interior
  steps saturated could never fail that leg. Four tones over sixteen columns is 85
  within and 255 across; three or five changes both numbers.
- **The marker in FR-3.2-S1 must not be a valid PNG.** It is there to be
  recognisable as something no build wrote.
- **`build_reports_unused_keys.rs` asserts an absence** — that `base:used` does not
  appear on stdout. That is safe only because a written path spells a key's colon
  as two underscores, so the literal `base:used` cannot reach stdout by being a
  path. A change to D9's derivation would silently make that clause vacuous.

### Weak instruments, named

- **FR-3.4-S5's test cannot say which stage refused.** It makes a `materials/`
  entry named `locked.toml` a *directory*, which refuses a read on every platform
  this builds for where a permission bit would not. Both a fold-time read and the
  material loader's own read satisfy the assertion. What it grades honestly is the
  observable a mod author meets: the build is refused and the source is named.
- **FR-3.3-S12 and FR-3.3-S13 are told apart by their words**, because both are
  refusals about one field raised in one place. S12's test requires the message to
  name the **derived image name** (`base__a/b.png`), which a refusal that never got
  that far cannot produce; S13's requires the word `index`, pinned in the test file
  as contract on the terms `Leg::token` states. If the implementation raises one
  message for both, both tests can pass.
- **FR-3.3-S2 requires `line 3`**, which is `toml`'s own diagnostic and not this
  tool's wording. A `toml` upgrade that changed that phrasing would redden it.

### The lint, run directly

`cargo clippy --workspace --all-targets --all-features -- -D warnings` was run
inside the worktree against the reference implementation and **is clean**. It was
not clean first: it named a byte-string literal in `art_fold.rs`, a redundant
closure in `common/build.rs`, and an over-long test function in each of
`art_index.rs` and `build_writes_a_set.rs`. All four are fixed in the committed
files rather than allowed. A green suite is no evidence about a lint, and this
phase's window has no compilable tree in the shared checkout until the
implementation lands.

`rustfmt --edition 2024 --check` is clean over the eleven files, run on those
paths alone and never as a repo-wide `cargo fmt`.

### Notes for the implementer

- **`tools/voxforge/tests/common/build.rs` is at 551 non-blank lines of a 600-line
  test limit** — 49 lines of headroom, measured with the gate's counter. It is not
  the implementer's to edit, but a phase that needs more harness has that much
  room and no more.
- **`common/build.rs` is declared by `#[path]` from the build tests alone**, not
  from `common/mod.rs`, so that a test binary with no interest in an art build does
  not link `mc-core`'s index parser to get at a document fixture.
- **`IndexError` has no arm for a `key` record whose key is not a namespaced id.**
  The six arms `architecture.md` fixes leave that case homeless; the reference
  implementation mapped it to `UnknownRecord { word: "key" }`. No test asserts it,
  so the choice is the implementer's — but it has to be made deliberately rather
  than discovered.
- **`stating` cannot refuse an image name carrying a space**, which would render a
  `key` record that parses back with the wrong split. voxforge's own file-name rule
  (FR-3.3-S12) is what stops it arising, so this is defence-in-depth that is
  currently absent rather than a live defect. Deferred observation, outside this
  phase's diff.

---

## Phase 6 — The gate builds the art and refuses a committed one

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-7.1-S1 | `crates/mc-client/tests/gate_art_stages.rs` | `a_committed_built_image_fails_the_stage_naming_the_committed_path` |
| FR-7.1-S2 | `crates/mc-client/tests/gate_art_stages.rs` | `a_tree_carrying_only_the_manifest_models_and_materials_passes_the_stage` |
| FR-7.1-S3 | `crates/mc-client/tests/gate_stage_order.rs` | `the_instrumented_path_builds_the_set_before_the_stage_that_runs_the_tests` |
| FR-7.1-S4 | `crates/mc-client/tests/gate_stage_order.rs` | `the_coverage_skipping_path_builds_the_set_before_the_stage_that_runs_the_tests` |
| FR-7.1-S5 | `crates/mc-client/tests/gate_art_stages.rs` | `a_failing_set_build_fails_the_stage_and_reproduces_the_builds_refusal` |
| FR-7.1-S6 | `crates/mc-client/tests/gate_art_stages.rs` | `a_failing_set_build_leaves_the_test_stage_unrun_and_records_that_it_did_not_run` |

The tasks stage's proposed names and files stood unchanged. The reading each one
uses did not: three of the six are graded by a scan of the script's text and are
named as such below.

### Additional coverage

| Test | File | What it catches |
|---|---|---|
| `an_art_build_naming_a_key_no_block_declares_still_passes_the_stage` | `gate_art_stages.rs` | a stage keying on non-empty output or on a silent stderr rather than on the exit code. The shipped build prints five unused-key lines today (T37) and would take the gate down with it. **The fixture is not the shipped root**: it declares no blocks at all, so its one key is unused permanently and P9's T53 cannot green this test by accident |
| `the_same_reading_tells_a_misplaced_set_build_from_a_well_placed_one` | `gate_stage_order.rs` | the positive control for FR-7.1-S3 and S4. Five fixture scripts — well placed, inside the skipping branch, after the tests, nowhere, and one running no suite on a path — must read as five different answers. Without it, a scan that stopped finding `cargo run -p voxforge` would certify the gate forever |
| `the_same_reading_tells_a_guarded_test_stage_from_an_unguarded_one` | `gate_stage_order.rs` | the positive control for FR-7.1-S6's structural half, over all five arms: guarded and recorded, no record at all, a record beside a suite that runs anyway, a guard settled before the build, and no suite anywhere |
| `the_set_is_built_after_the_quick_early_exit` | `gate_stage_order.rs` | `-Quick` becoming a bake on every edit loop. D15 and T36 both place the stage after the early exit and no scenario says so. **Its control is folded into the same assertion** — a second reading of a fixture that does build before the exit — rather than given its own function, because the reading is one line-order comparison |
| `the_header_states_an_exception_to_every_stage_running` | `gate_stage_order.rs` | the script's header claiming "Every stage runs even if an earlier one fails" beside code this phase makes it false for. The recogniser is the word `except` appearing in the header block; the script contains it nowhere today, so the arrival is unambiguous |

### Interface decisions these tests make binding

1. **`-ArtOnly` runs stages 7 and 8 and then reaches a summary of the same shape
   the gate already prints** — one line carrying the word `PASSED` or the word
   `FAILED`, and under a failure one line per failed stage. Nothing dictates the
   banner's wording; `-Quick`'s `QUICK CHECKS PASSED` shape satisfies it as-is.
2. **A refused set build records two failures, not one**:
   `art (voxforge build)` and `tests (not run: art build failed)`. It is one rule
   with no dependence on which stages were selected, and both are true under
   `-ArtOnly`: the build did refuse and the tests did not run. The alternative —
   recording the skip only where stage 9 was selected — leaves FR-7.1-S6 with no
   observable a bounded test can reach at all.
3. **The stage names are exactly D15's**: `art (generated set not committed)` and
   `art (voxforge build)`. They are compared as whole lists, so a run that fails a
   third stage fails the test.
4. **`-ContentRoot` and `-Manifest` are used as given.** A relative value is
   relative to the repository, which the script already pushes into; an absolute
   one is left alone. Every behavioural test hands `-Manifest` an absolute path in
   a temporary directory, and FR-7.1-S5 compares the gate's output against a real
   run of `voxforge` over that same absolute path — so a stage that re-derived the
   path before running the tool would fail that comparison.
5. **The set is built into a copy, never in place.** `voxforge build` writes
   beside its manifest, so a run aimed at `content/base/textures.toml` would have
   four tests writing the shipped set at once, and one aimed at a tracked fixture
   would leave built images in the repository.

### `-ContentRoot` cannot be a temporary tree, and that is the same finding as `-RepoRoot`

`architecture.md`'s D15 says the harness drives `-ContentRoot <temp>`. It cannot,
and the reason is one level down from the reason there is no `-RepoRoot`. Measured
on this checkout:

```
git ls-files -- C:/…/scratchpad/fixture/textures
fatal: … is outside repository at 'E:/_PROJEKTE/MyCraft'   (exit 128)
```

A pathspec outside the worktree is refused whether the fixture is clean or dirty,
so both answer the wrong question. A pathspec *inside* the repository naming a
directory that does not exist is not refused — it reports nothing and exits 0 —
which is what makes FR-7.1-S2's fixture legitimate.

So the two content roots are **committed fixtures**, under
`crates/mc-client/tests/fixtures/gate/`:

- `a-content-root/` — `textures.toml`, `models/block.mcvox`, `materials/plain.toml`
- `a-content-root-with-a-built-image/` — the same three files, plus one tracked
  `textures/fixture__block.png`

They differ by exactly one file, which is what makes the pair a controlled
comparison rather than two unrelated trees: M36's mutation — a stage inspecting a
path that does not exist — turns the first answer and leaves the second alone.

Only the image is tracked and not the `index.txt` the build writes with it, so
FR-7.1-S1's "name the committed path" has exactly one path to name.

The fixture root is deliberately **not** cut from `content/base`. It is one voxel
at sixteen pixels to it — the smallest tree `voxforge build` accepts, since a
block texture is sixteen texels on an edge — and it declares no blocks, so nothing
P9 does to the shipped content can move any of these tests.

### Arrival colours, as measured

Whole workspace, `cargo nextest run --workspace --no-fail-fast`:
**1310 run, 1299 passed, 11 failed, 1 skipped**, against a stated baseline of
1299 run, 1297 passed, 2 failed, 1 skipped. The two inherited failures are
`mc-world::shipped_declarations_and_an_older_save`, which are red on an uncommitted
edit to `content/base/blocks/water.luau` that belongs to the project owner and are
red correctly. **No third red appeared**, so all eleven new tests are accounted
for.

Every one of the six scenario tests failed at an **assertion**, none at a compile
error, and each on the arm that says what is missing rather than on a generic one:

| Test | Arrived as |
|---|---|
| FR-7.1-S1 | `(NoSummaryWasPrinted, 1, false)` — `A parameter cannot be found that matches parameter name 'ArtOnly'` |
| FR-7.1-S2 | `(NoSummaryWasPrinted, 1)` |
| FR-7.1-S3 | `NoSetIsBuiltOnThisPath` |
| FR-7.1-S4 | `NoSetIsBuiltOnThisPath` |
| FR-7.1-S5 | `(NoSummaryWasPrinted, 1, false)`, with the tool's real refusal quoted in the failure message |
| FR-7.1-S6 | `(NoSummaryWasPrinted, NothingRecordsThatTheTestsWereSkipped)` |

Of the five additional tests, three are red — the advisory-key run, the `-Quick`
placement and the header claim (`TheClaimStandsUnqualified`) — and **two are green
on arrival and are meant to be**: the two positive controls grade the reading
itself, which lives in the test tree, so there is nothing about them for an
implementation to turn. A phase that reddens either has broken the instrument, not
the gate.

`cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean,
run directly rather than inferred from a green suite.

### Weak instruments and overlaps, named

- **FR-7.1-S3, S4 and half of S6 are read off the script's text, not run.** A run
  of the whole gate to observe any of them costs a workspace clippy pass, a
  documentation build, two supply-chain scans and the entire suite — a second full
  run of the suite, inside the suite — and a run of `-ArtOnly` cannot observe
  stages it does not select. The scan can say a command sits before another and
  inside or outside a branch; it cannot say either command does what its stage
  claims. D15's tie is what makes that enough: the selector restates no stage, so
  the behavioural tests grade the same stage bodies the scan places.
- **The one composition left to a human is named, and T38 already asks for it**:
  break the manifest, run the real gate, and check stage 9 is reported as *not
  run* rather than run and passing. That is the only witness that the guard the
  scan reads actually keeps the suite from running.
- **The scan blanks whole-line comments and the `<# … #>` header** before it
  counts braces, so prose quoting a command is not read as the command. A comment
  *trailing* a command on the same line is not recognised — the one shape it would
  misread, stated where whoever meets it will look.
- **FR-7.1-S5 and S6 assert the same failure list.** They are separated by what
  else each asks: S5 that the tool's own refusal reaches the reader, S6 that the
  suite is unreachable. Each would survive the other being deleted.
- **The header claim is a word match.** `except` appearing in the header cannot
  tell a well-stated exception from a badly stated one. It can tell one nobody
  wrote, which is the failure worth guarding.
- **A missing `pwsh` fails these tests rather than skipping them.** An absent
  instrument and a clean one must not look alike.

### The fixture constraints no assertion can enforce

- **The fixture pair must differ by exactly one file.** Nothing asserts it. If a
  future edit gives `a-content-root` a `textures/` of its own, FR-7.1-S2 goes red
  for a reason that has nothing to do with the gate; if it gives
  `a-content-root-with-a-built-image` a second tracked image, FR-7.1-S1's list of
  named paths grows and it goes red the same way.
- **`fixture__block.png` is a real built image**, produced by running
  `voxforge build` against that fixture's own manifest once and committing the
  result. It is not a placeholder with a `.png` name.
- **The broken manifest refuses at the manifest**, before any model is opened, so
  nothing is written anywhere and the refusal is one line naming the file it was
  handed. A fixture that refused later would leave output in a temporary tree and
  would give the comparison more than one line to match.
- **The oracle is checked before it is used.** `the_refusal_of` fails the test
  when the tool did not refuse or refused about something else, because an oracle
  nobody checked is a second thing that can be wrong.

### Mutations — what can be run now, and what cannot

**None of P6's five rows can be run yet, and that is structural rather than an
omission**: M33 through M37 all mutate stages that do not exist. They belong to
the implementer, after the stages land, and each row's *stated green* is the run
recorded above.

Three of the five are nonetheless **already measured against the reading that
grades them**, because the control fixtures are those mutations' exact shapes fed
to the same instrument — measured, not predicted:

| Row | The control fixture that is its shape | What the reading answers |
|---|---|---|
| M33 | `BUILDING_INSIDE_THE_SKIPPING_BRANCH` | `(NoSetIsBuiltOnThisPath, TheSetIsBuiltBeforeTheTests)` — **exactly one** of FR-7.1-S3 and S4 turns, and it is the instrumented one, which is what says which branch it landed in |
| M34 | `BUILDING_BEFORE_EITHER_PATH` (no record anywhere) | `NothingRecordsThatTheTestsWereSkipped` — FR-7.1-S6 turns |
| M35 | `RECORDING_A_SKIP_THAT_DOES_NOT_HAPPEN` | `SomeTestRunsWhateverTheArtBuildDid` — FR-7.1-S6 turns |

M36 (a stage inspecting a path that does not exist) and M37 (a stage swallowing
the tool's output) are behavioural and have no such stand-in. What the fixtures
say about them is only that FR-7.1-S1 and S2 differ by one file and that
FR-7.1-S5 compares against a real run rather than a spelled-out sentence.

**P6's table re-uses M33–M36 from P5's, and P5's leg two added a second M36.**
The numbering in `tasks.md` is per phase; every row above is P6's.

### Confirm once by hand

Break the manifest, run the gate, and check stage 9 is reported as *not run*
rather than run and passing. `architecture.md`'s Risks entry asks for it because
it is the composition of two stages and the tests grade each separately.

**Done, 2026-08-19, and this is what it said.** A full gate run — no `-ArtOnly`,
no `-Quick` — over a manifest declaring `pixels_per_voxel = 0`:

```
── art (generated set not committed) ────
ok: art (generated set not committed)

── art (voxforge build) ────
…/textures.toml, field `pixels-per-voxel`: 0 pixels per voxel renders an image of
no pixels, which is not a smaller preview but no preview — the minimum is 1
FAIL: art (voxforge build)
FAIL: tests - not run: the art build refused and the set on disk is the previous one

════════════════════════════════════════
GATE FAILED - 2 stage(s):
  · art (voxforge build)
  · tests (not run: art build failed)
```

Exit 1. Stages 1 through 7 ran and passed first, so this is the composition and
not a short-circuit. **The witness that the suite was unreachable is that no test
command ran after stage 8**: the only two `Starting N tests across M binaries`
lines in the whole 1 536-line transcript are stage 2b's two GPU-free runs, at
lines 17 and 93, and there is no `tests + coverage (llvm-cov nextest)` header
anywhere. A run that had reached stage 9 could not have been quiet about it.

### What the implementation found that the tests could not

- **`Invoke-Stage` decides on `$LASTEXITCODE`.** A scriptblock that fails
  *without* running a native command inherits whatever the previous command
  exited with, and the stage reports `ok`. Met while running M37: a malformed
  edit produced `build $Manifest2>&1`, an undefined variable under
  `Set-StrictMode`, and stage 8 passed. Pre-existing helper behaviour, untouched
  by this phase and recorded rather than fixed in passing.
- **A landed edit is not a correct edit.** The mutation harness asserted its
  target was present exactly once and compared the file whole after writing, and
  the malformed M37 satisfied both. What caught it was reading `git diff` on the
  mutated tree before believing the run.
- **PowerShell's `Write-Host` does not wrap a long line when the stream is
  redirected** — measured at 155 characters, arriving intact. That is what lets
  stage 7 name a 103-character repository path on one line and lets the reading
  in `gate/running.rs` compare it whole.

---

## Phase 7 — The client judges the set and refuses by name

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-5.1-S1 | `crates/mc-client/tests/built_set_verdict.rs` | `an_absent_index_reports_the_set_absent_and_the_refusal_names_the_build_command` |
| FR-5.1-S2 | `crates/mc-client/tests/built_set_verdict.rs` | `a_model_edited_since_the_build_reports_the_set_stale_and_names_the_rebuild_command` |
| FR-5.1-S3 | `crates/mc-client/tests/built_set_verdict.rs` | `a_manifest_that_gained_an_entry_reports_the_set_stale_and_refuses_the_launch` |
| FR-5.1-S4 | `crates/mc-client/tests/built_set_verdict.rs` | `a_recorded_source_that_is_no_longer_present_reports_the_set_stale_and_names_it` |
| FR-5.1-S5 | `crates/mc-client/tests/built_set_verdict.rs` | `an_index_naming_an_absent_image_refuses_the_launch_naming_the_image_and_its_key` |
| FR-5.1-S6 | `crates/mc-client/tests/built_set_verdict.rs` | `a_present_and_current_set_reports_current_and_completes_the_launch` |
| FR-5.1-S7 | `crates/mc-client/tests/built_set_verdict.rs` | `an_index_naming_no_keys_and_current_against_its_sources_reports_current` |
| FR-5.1-S8 | `crates/mc-client/tests/built_set_verdict.rs` | `a_content_root_stating_no_texture_manifest_reports_no_art_declared_and_launches` |
| FR-5.2-S1 | `crates/mc-client/tests/set_refusal_and_key_fallback_differ.rs` | `an_absent_set_emits_the_build_command_refusal_and_not_the_unauthored_key_wording` |
| FR-5.2-S2 | `crates/mc-client/tests/set_refusal_and_key_fallback_differ.rs` | `a_current_set_covering_no_declared_key_launches_without_the_build_command_refusal` |

### Why these assert a verdict and not an absence

`assert!(no_refusal_printed)` cannot tell a healthy set from a client that lost the
ability to check. `SetVerdict` is a **total enumeration returned in `Ok`**, so
`assert_eq!(verdict, Current)` rejects every other arm *including* the ones meaning
"I could not look". Returning the verdict only as an error would leave three arms
unconstructible in `Ok`, and the totality would not be what the suite was holding.

FR-5.1's eight scenarios reach all six arms; `refusal_for` is a separate function
and FR-5.2's two are what assert its text.

### Additional coverage

| Test | File | What it catches |
|---|---|---|
| `the_client_refolds_the_sources_the_index_recorded_and_never_reads_the_manifest` | `crates/mc-client/tests/built_set_verdict.rs` | the client growing a second manifest parser — the drift D7 exists to make unspellable, and a defect no verdict scenario can see because both routes agree on the shipped tree. The fixture adds a `materials/*.toml` the index never recorded: a client re-deriving its source list from the manifest folds it and calls the set stale, one re-folding the recorded list cannot see it |
| `a_content_root_copied_to_a_temporary_directory_is_still_current` | `crates/mc-client/tests/built_set_verdict.rs` | index paths becoming absolute, which would make every `copy_tree`'d fixture permanently stale and put developer home directories into a file the gate builds. **Its pair, FR-5.1-S6, was moved onto the repository's own `content/base/`** so the two reach `Current` by different routes rather than being one fixture asserted twice |
| `the_command_the_refusal_quotes_is_the_one_the_pages_tell_a_contributor_to_run` | `crates/mc-client/tests/set_refusal_and_key_fallback_differ.rs` | **replaces the proposed `the_build_command_is_spelled_in_exactly_one_place`.** A scan of `mc-client/src/` for one occurrence stays green while `README.md` quotes something else, which is the drift that actually costs somebody an afternoon. This asserts the four-way agreement instead: the command spelled from `tasks.md` (never from a run), the `BUILD_THE_TEXTURE_SET` constant the refusal interpolates, `README.md`, and `docs/modding/voxel-models.md` |
| `an_index_recording_a_source_outside_the_content_root_is_refused_naming_the_path` | `crates/mc-client/tests/an_unreadable_set_names_what_it_cannot_read.rs` | **the promise P5 deferred an observation on.** A manifest naming `model = "../shared/x.mcvox"` builds cleanly and writes a `source` record `TextureSetIndex::parse` refuses; it was left unbuilt on the stated grounds that P7 gives a clear refusal naming the path. This holds that to it — the arm, and the path in the printed chain |
| `an_index_naming_an_image_that_is_not_an_ordinary_name_is_refused_naming_it` | `crates/mc-client/tests/an_unreadable_set_names_what_it_cannot_read.rs` | the client joining an index-supplied name onto a path without the shape check D9 binds it to. `mc_core::art::is_an_ordinary_image_name` had **no `crates/`-side caller** before this phase; `parse` accepts `elsewhere/base__stone.png` and the reader must not. Reachable only because FR-5.1-S5 makes the client locate an image at all |
| `texels_stated_for_a_key_come_back_for_that_key_unchanged` | `crates/mc-render/src/texture/supplied_test.rs` | `stating`/`covering` shipping undriven. The type exists in P7 only because `built_set`'s signature returns it, so `none()` is all this phase constructs — and `mc-render` is inside the coverage denominator while `mc-client` is not. Four pairwise-distinct texels, so a wrong entry, a truncated one or a reordered one cannot read as the right answer |
| `a_key_nothing_supplied_texels_for_is_covered_by_nothing` | `crates/mc-render/src/texture/supplied_test.rs` | a `covering` that answers for the wrong key. Stated **alongside** a key that is supplied, so what it reads is one entry's absence rather than every entry's |
| `a_root_declaring_no_art_supplies_nothing_for_a_key_another_root_would_cover` | `crates/mc-render/src/texture/supplied_test.rs` | the vacuity control for the row above: without it, a `covering` that had come to answer `None` unconditionally satisfies both |

### The producers the modding page's fences are held to

`crates/mc-client/tests/support/built_set_refusals.rs` — six refusals, each from a
real run of the client's own preparation, `extend`ed onto `printed_refusals()` the
way `per_facing_refusals` is and for the same two reasons: `printed_refusals.rs`
has under forty non-blank lines of headroom, and these are one requirement's
refusals.

**Five of the six name no path, which is a real difference from the eight already
there.** Every existing producer goes through `as_read_from_a_game_directory`,
whose rewrite *refuses* text that does not name the fixture root. A built set's
refusals name the command to run, or a source as the index records it — already
relative. So five go through `as_read_anywhere`, carrying the **inverse** premise
check: it fails if the text names the fixture root, which is the day one of them
grows a path and starts leaking a directory that lives for a hundred milliseconds
into a page comparison. Only the unreadable-index refusal names a path, and it is
the one that keeps the rewrite.

**The reverse guard now exists**:
`the_voxel_model_guide_states_every_refusal_a_built_texture_set_raises` in
`documented_refusals.rs`. Without it that file's scan runs pages → run only, so a
page quoting three of the six passes — an instrument that checks every quotation
is real while nothing checks every real thing is quoted reports its own blind spot
as zero, and the per-facing seven already had the guard the six lacked.

`GuideListing::EveryPerFacingRefusalIsQuotedInFieldOrder` is renamed
`EveryRefusalIsQuotedInFieldOrder` so the verdict is honest for both callers. Its
ordering half is vacuous for the six, which name no declaration field; the arm's
own doc comment says so rather than a second enum being introduced whose arms
differed by one.

**It arrived green, and its falsifiability was measured rather than assumed.**
Against a temporary copy of `voxel-models.md` with the first refusal replaced by a
paraphrase, `listing_of` returns
`NotQuoted { refusal: "mycraft: the generated texture set is not there; run …" }` —
it names the refusal that went. The probe was reverted by hand and `git diff` on
that file carries only the guard and the rename. **A temporary copy, never the
shipped page**, because the implementer was editing `docs/` in the same tree at the
time and reverting a shared file is how somebody else's work gets discarded.

**The committed positive control is owed and does not fit.** The hole an enumerated
verdict leaves open is inside its good arm: a `listing_of` that came to report every
raised refusal as quoted would answer `EveryRefusalIsQuotedInFieldOrder` forever,
and only a control catches that. **The per-facing listing has never had one either**,
so one control covers both and whoever writes it closes two holes rather than paying
off this phase's. Fitting it needs `documented_refusals.rs` split by scan direction —
pages → run in one place, run → pages in another — and both directions share `pages()`
and `quoted_refusals_in`, so the split needs a third module for the scanner rather
than a move. **Unrelated scope: a test-file restructure belongs on `main` between
specs, not inside this spec's squash.** Filed at completion.

**The gap is recorded on `listing_of` itself, not only here**, because this folder is
archived and then pruned and the person who will one day change `listing_of` is
reading `listing_of`. That comment states what would go undetected and that both
guards share it.

**The reused verdict carries an ordering clause the models page does not have, and
it is vacuous rather than wrong.** `listing_of` ranks a page's quotations by
`FIELDS_IN_THE_ORDER_THE_GUIDE_STATES` — `name`, `texture`, `solid`, `replaceable`,
`breakable`, `breaks_into` — which are *block declaration* fields. **Measured: no
`mycraft: ` fence on `voxel-models.md` names one of the six**, so `ranked_field`
answers nothing for all six built-set refusals and no ordering rule is applied to
that page. The page does name `face`, `key`, `all-faces` and `scale` eight times,
but inside voxforge fault blocks whose first line is a file name rather than
`mycraft: `, which the recogniser does not collect. It would stop being vacuous only
if a future built-set refusal quoted a *block declaration* field — a different
domain from a manifest and a model. Presence-only in behaviour today; the
separate presence-only verdict belongs with the filed split.

### What M42 actually caught, measured by the implementer

The prediction in `tasks.md` named the wrong witness, and the row above is the one
that bites.

- **FR-5.1-S3 stays green under M42, and that is not a gap.** The gained manifest
  entry names a model the manifest already reaches, so a re-derived source list is
  the *same* list — and the manifest's own changed bytes move the fold either way.
  Nothing in S3 can separate the two routes.
- `the_client_refolds_the_sources_the_index_recorded_and_never_reads_the_manifest`
  **reddens.** The material it adds is one no index recorded, so only a client that
  went back to the manifest folds it. It is the sole witness the phase predicted it
  would need.
- **FR-5.1-S4 reddens too, and nobody predicted it.** A directory scan cannot see a
  file that is *gone*: the removed `materials/dirt.toml` drops out of a re-derived
  list entirely, so the set reads `StaleAgainstSources` where the recorded list
  gives `SourceMissing`. **`without_a_recorded_source` is therefore doing more than
  its scenario claims** — it is also the evidence that recording the list is what
  makes "a source went missing" observable at all, rather than merely reported as
  staleness.
- **M38–M41 each reddened `documented_refusals` beside their named scenario**, which
  the producers are what make true: a change to what the client *says* now reddens
  the documentation and not only the verdict. M41 reaches the `SourceMissing` arm
  through the page, a second and independent route to it.

**Measuring the size limit, since two numbers in this file have been the wrong one.**
The gate counts with `(Get-Content | Measure-Object -Line).Lines`, and PowerShell's
`-Line` **skips empty lines** — so the limit is 600 *non-blank* lines, which is what
`grep -cve '^\s*$'` reports and not what `wc -l` reports. `documented_refusals.rs` is
at **594 of 600** with the guard and the comment, verified by running the size stage's
own logic over `crates/` and `tools/`: clean.

Getting there cost one thing worth recording. The comment saying the control does not
fit did not fit either — it took the file to 620. `blocks_page()`, a one-line helper
with a six-line doc comment used once, is inlined to make the room. **That inlining
was reverted once as a drive-by tidy and is here now because the room is needed**, and
it has a second justification it did not have then: both guards now spell their page
the same way, which is the symmetry this whole addition is about.

### The interface these tests bind, as their first consumer

| Decision | What it is, and why |
|---|---|
| `mc_client::textures::{SetVerdict, TextureSetError, built_set, refusal_for}` | **`refusal_for` lives beside the verdict, not in `startup.rs`.** D6's prose says `startup.rs`; the Interfaces section lists it under `// textures/mod.rs`. The listing wins — it keeps the mapping beside the enum it is total over, and `startup.rs` is the file `tasks.md` already flags for headroom |
| `built_set(root) -> Result<(SetVerdict, SuppliedTexels), TextureSetError>` | The architecture's signature, kept whole. P7 has no texels to offer and returns `SuppliedTexels::none()`; narrowing it to a bare verdict now would make P9 widen a signature whose tests this phase owns, and a P9 implementer may not edit them |
| `mc_render::texture::supplied::SuppliedTexels` exists in P7 | Forced by the line above. Pure, `stating`/`none`/`covering`, no consumer until P8's `levels_for` |
| `SetVerdict::SourceMissing { source }` and `ImageMissing { image }` carry the path **as the index records it** | `materials/dirt.toml`, not a resolved absolute path. It is what an author can search the index for, and it is the same on both platforms — a resolved path is `\`-separated on Windows and no assertion could name it once |
| `PreparationError::TextureSetSourceMissing` names its field `missing`, not `source` | **Measured, not chosen.** `thiserror` reads any field named `source` as the error's cause and the architecture's variant does not compile: `the method as_dyn_error exists for reference &PathBuf, but its trait bounds were not satisfied`. The `Display` text is unchanged |
| `TextureSetError::UnusableImageName { key, image }` — an arm the architecture does not list | The client takes an image name from the index and joins it onto a path. D9 binds it to refuse a name failing `is_an_ordinary_image_name`, and no listed arm says that. `Size` and `NotAPng` are **not** in this phase's enum: they are FR-4.3 and arrive with the decode in P9 |
| `pub const BUILD_THE_TEXTURE_SET` in `startup.rs`, beside `LOAD_CHANGED_BLOCKS` | `pub` so the page-agreement reading can compare the constant to what `README.md` and `docs/modding/voxel-models.md` quote. Private, the only oracle left is a second copy of the string |
| `TextureSetError::Index` carries its `IndexError` as a `#[source]` | The refusal a person reads is the whole chain, and the offending path is in the cause. `refusal_printed_over` renders it |

### Arrival colours, as measured

**Two throwaway skeletons, in a detached worktree with its own target directory,
removed after.** Neither is committed. The phase's committed tree does not
compile — `mc_client::textures` does not exist — which is why the lint reading
below was taken in the worktree and not from the gate.

- **Skeleton A** — `built_set` returns `(Current, SuppliedTexels::none())`
  unconditionally; `refusal_for` maps correctly; `prepare_scene` calls both.
- **Skeleton B** — the same, returning `Absent`.

| Test | A | B | Why |
|---|---|---|---|
| FR-5.1-S1 `…set_absent_and_the_refusal_names_the_build_command` | red | **green** | B *is* the answer. Its green is what makes A's `left: Current, right: Absent` the only witness |
| FR-5.1-S2 `…reports_the_set_stale_and_names_the_rebuild_command` | red | red | |
| FR-5.1-S3 `…gained_an_entry_reports_the_set_stale…` | red | red | |
| FR-5.1-S4 `…no_longer_present_reports_the_set_stale_and_names_it` | red | red | |
| FR-5.1-S5 `…absent_image_refuses_the_launch_naming_the_image_and_its_key` | red | red | |
| FR-5.1-S6 `a_present_and_current_set_reports_current…` | **green** | red | A *is* the answer for the shipped root |
| FR-5.1-S7 `…naming_no_keys_and_current…reports_current` | **green** | red | as above — and the two are the vacuity control for `Current` in both directions |
| FR-5.1-S8 `…no_texture_manifest_reports_no_art_declared_and_launches` | red | red | the arm no skeleton produces; it is the one D6 added |
| FR-5.2-S1 `an_absent_set_emits_the_build_command_refusal…` | red | **green** | under A the launch succeeds, so there is no refusal to read and the fixture says so rather than searching an empty string |
| FR-5.2-S2 `a_current_set_covering_no_declared_key_launches…` | **green** | red | |
| `…refolds_the_sources_the_index_recorded…` | **green** | red | |
| `a_content_root_copied…is_still_current` | **green** | red | |
| `the_command_the_refusal_quotes…` | red | red | red on `README.md`, which T43 writes |
| `…source_outside_the_content_root_is_refused_naming_the_path` | red | red | |
| `…image_that_is_not_an_ordinary_name_is_refused_naming_it` | red | red | |

**Measured, not predicted:** 15 tests run · A: 5 passed, 10 failed · B: 2 passed,
13 failed. Every test is red under at least one skeleton, and the five that are
green under A are green under A *only* — which is the whole of what a skeleton
returning one arm forever can be used to say.

**Collateral: none.** Full workspace under skeleton A — **1325 run, 1315 passed,
10 failed, 1 skipped**, and all ten failures are this phase's. 1325 is the 1310
baseline plus these 15. The five new `PreparationError` variants, the
`prepare_scene` call and `SuppliedTexels` reddened nothing.

**`cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0**
in the worktree with skeleton A applied, and `cargo fmt --all -- --check` is
clean. Taken directly because a phase opening on a tree that does not compile has
no gate reading at all until the implementation lands.

**The limit of this instrument.** Skeleton A and skeleton B are each one constant
arm. They can show that a reading depends on *which* arm comes back; they cannot
show that a reading depends on the fold, on the source list, or on the order the
checks are made in. Everything of that kind is what `tasks.md`'s M38–M43 are for,
and M42 in particular — the re-fold being driven at all — has no witness until
there is an implementation to break.

### The premise this phase carries, and it must be in the closing report

**A bare `cargo nextest run` now fails without a built set.** That is FR-7.2-S2
working, not a regression; the gate is green because P6 taught it to build first.
Unrecorded, the next pair "fixes" it by making the client tolerate an absent set,
which deletes FR-5.1 and FR-5.2 together.

### What FR-5.2-S2 can honestly assert here

The per-key fallback does not exist until P9. S2 asserts only that **the refusal is
absent** on a current set covering nothing — which is what is observable in this
phase. FR-4.2 is what asserts the fallback itself.

---

## Phase 8 — The mip chain, as arithmetic

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-6.1-S1 | `crates/mc-render/src/texture/mip_test.rs` | `a_sixteen_texel_image_prepares_five_levels_sized_sixteen_eight_four_two_and_one` |
| FR-6.1-S2 | `crates/mc-render/src/texture/mip_test.rs` | `two_stored_zeroes_and_two_stored_maxima_reduce_to_stored_one_eight_eight` |
| FR-6.1-S3 | `crates/mc-render/src/texture/mip_test.rs` | `an_image_of_one_colour_reduces_to_that_colour_at_every_level` |
| FR-6.1-S4 | `crates/mc-render/src/texture/mip_test.rs` | `each_texel_of_the_reduced_level_averages_exactly_the_four_texels_it_covers` |
| FR-6.1-S5 | `crates/mc-render/src/texture/mip_test.rs` | `a_layer_offered_fewer_levels_than_declared_is_refused_naming_the_key_and_the_count` |

### FR-6.1-S2 is the scenario that distinguishes the implementations — three of them, measured

The array texture is `Rgba8UnormSrgb` and decodes to linear on sample. Averaging
the **stored** bytes of 0 and 255 gives 128, which decodes to linear 0.216 rather
than 0.5 — every level darker than the one above it, the classic sRGB mipping
fault, plausible-looking and wrong in the direction nothing notices.

**188 is pinned precisely because 128 is what the wrong implementation produces.**
A scenario saying only "midway between" would accept both. Do not soften it, and do
not add a tolerance that spans the two.

### The tolerance rule for this phase

These are `f32` round trips through a transfer function. **Measure the arithmetic
path before choosing the assertion; inspecting the literals is not enough.** An
exact comparison that happens to be the *consistent* one with neighbouring tests
can still fail against a correct implementation, and the cheapest way to green such
a failure is to break the production code. Derive any tolerance from both
directions — above the measured error, below the smallest difference the test must
still catch — never by loosening until green.

### Additional coverage

| Test | File | What it catches |
|---|---|---|
| `a_stored_byte_survives_a_round_trip_through_linear_and_back` | `crates/mc-render/src/texture/mip_test.rs` | the transfer pair being inconsistent with itself, which would bias every level in one direction while each individual level still looked plausible |
| `the_declared_level_count_is_derived_from_the_texture_edge` | `crates/mc-render/src/texture/mip_test.rs` | `MIP_LEVELS` being written as `5`. A size and a level count that can disagree is a copy that overruns; the compile-time assertion is the real guard and this states the intent |
| `supplied_texels_of_the_wrong_count_are_refused_naming_the_key` | `crates/mc-render/src/texture/mip_test.rs` | `TextureError::WrongTexelCount` unconstructible in a test — an arm nobody has read, on the path a hand-tampered set reaches |
| `a_key_the_supply_covers_is_levelled_from_the_supplied_texels` | `crates/mc-render/src/texture/mip_test.rs` | **added to the four planned here, and it is the half without which the other is not a pair.** `SuppliedTexels` has no consumer before `levels_for`; without this reading a `levels_for` that ignored the supply entirely and generated every layer passes all nine other tests. Its fixture asserts the supplied art differs from what the key generates, so the green cannot be an accident of the two agreeing |
| `a_key_the_supply_does_not_cover_falls_back_to_its_generated_texels` | `crates/mc-render/src/texture/mip_test.rs` | `levels_for`'s fallback branch existing only as P9's problem. FR-4.2 asserts it end to end; this asserts it where it is pure and where a device is not needed |

### The pair that matters most here, and the shape M46 has to take

**FR-6.1-S3 stays green under an off-by-one in the four-texel selection** — a
uniform image averages to itself however the four are chosen. FR-6.1-S4 is what
catches it, and it is worded *and not of any other four* for that reason. Neither
covers the other.

**Measured, and the measurement carries a caveat M46 needs.** An off-by-one
written the obvious way — shift the base index by one — also reads *past the end*
of the last group, and a `.get()` returning `None` fills the tail with zeros. That
is a second defect riding on the first, and it reddens FR-6.1-S2 and FR-6.1-S3 as
well: **three tests, not one.** Wrapping the same off-by-one back into the buffer
isolates the selection, and then the whole workspace shows **exactly one red,
FR-6.1-S4**.

So M46 has both outcomes available depending on how it is spelled, and only one of
them measures what it claims to. **Spell it so it stays in bounds.** Three reds is
the too-broad mutation of `docs/technical/testing.md` §"A mutation that misses has
two causes" seen from the other side — it bites, and it says less than the narrow
one does. Nothing about the fixtures is at fault in either case.

### Where every number comes from

- **188** is arithmetic, not a reading: stored 0 and 255 decode to linear 0.0 and
  1.0, whose mean is exactly 0.5, and `1.055 · 0.5^(1/2.4) − 0.055 = 0.735357`,
  times 255 is **187.516**, which rounds to 188. The wrong implementations answer
  differently and are separated by the same byte — averaging the stored bytes gives
  127.5 and so 128, and a gamma-2.2 approximation gives 186. **The byte separates
  three implementations, not two.**
- **The margin, because 187.516 is only 0.016 of a byte above the boundary that
  would round it to 187.** Against that: the transfer pair's round trip over all
  256 stored bytes has a worst pre-rounding error of **1.53e-5 of a byte**, at
  stored 132 — three orders of magnitude smaller. **This is the narrowest margin
  anywhere in the phase**, narrower than any of FR-6.1-S4's, so it is the one
  number that decides whether exact equality is safe here at all. Exact equality
  is what the arithmetic supports;
  a tolerance here would have to span 187 to 188 to matter and would then admit
  the fault the scenario exists to catch.
- **FR-6.1-S4's four expected texels** — `[55, 216]`, `[85, 183]`, `[183, 85]`,
  `[216, 55]` over red and green, blue held at 128 and alpha at 255 — were derived
  offline by a standalone program implementing IEC 61966-2-1 in `f32`, sharing no
  code with this workspace. Their pre-rounding values are 55.26934, 85.34859,
  182.89182 and 216.24971, and **the narrowest margin any of the sixteen channel
  readings holds against a rounding boundary is 0.1514 of a byte**, at 85.349.
  The widest is 0.3918, at 182.892.
- **That last figure was recorded the wrong way round in the first version of this
  file and was corrected by the implementer**, who re-derived the four values
  through the shipped arithmetic path and got the same numbers with a different
  minimum. The offline program computed `0.5 − min|v − round(v)|`, and minimising
  the distance to the nearest *integer* maximises the distance to the nearest
  *boundary* — so the number reported as the closest approach was the farthest.
  The conclusion was unharmed, because the true minimum is still three orders of
  magnitude above the transfer's own error, but **an offline oracle has no test**:
  it is written once, read once, and its output is copied into prose that then
  reads as measured. That is why the derivation is written here rather than only
  the answer, and it is the one instrument in this phase nothing else checked.
- **The fixture is what makes "and not of any other four" hold**, and the wrong
  selections were computed rather than assumed. Over the same sixteen texels, an
  off-by-one gives `[70, 101, 200, 192]`, four consecutive in row-major order give
  `[31, 96, 163, 231]`, and point-sampling the top-left of each group gives
  `[0, 34, 136, 170]`. All three differ from the correct answer in every position.
- **Exact byte equality in FR-6.1-S3 is measured, not hoped for.** All 256 stored
  bytes survive a uniform 2 x 2 average and re-encode to themselves, so the
  scenario's "that same colour at every level" is achievable rather than an
  over-tight assertion inviting a rounding fudge in production code.
- **Three levels for a 4-edge** (4, 2, 1) and **five for 16** (16, 8, 4, 2, 1) are
  the halving written out; 256 texels a layer and 255 one short are `16 · 16` and
  that minus one, computed in the test.

### The interface these tests bind, as their first consumer

| Decision | What it is, and why |
|---|---|
| `mc_render::texture::MIP_LEVELS: u32` | `architecture.md`'s listing, in `texture/mod.rs`, `TEXTURE_EDGE.ilog2() + 1`. The test asserts it against **what the halving produced**, never against `ilog2` written a second time |
| `mc_render::texture::TextureError` — the **path** is bound, the file is not | `TextureError` beside `LayerError` reads naturally, and `mip.rs` re-exported reads as naturally. The tests name `crate::texture::TextureError`, and either arrangement satisfies them. P9's `RendererError::Texture(#[from] TextureError)` is why the path and not the module matters |
| Both arms carry `{ key: TextureKey, offered: usize, declared: usize }` | `architecture.md`'s contract, kept whole. `offered` is the count the layer has and `declared` the count that was wanted, so one reading of the pair works for both arms |
| `TextureError` derives `Debug, Clone, PartialEq, Eq` and carries `Display` | The refusals are asserted **as values** — an enumerated verdict rather than a substring hunt — and the scenario's "name the key and the number of levels offered" is then read off the rendered message. Without `PartialEq` the only oracle left is the text |
| `TooFewLevels.declared` is `MIP_LEVELS`, `WrongTexelCount.declared` is `size · size` | Two different questions that happen to share a field name: how many levels the array texture wants, and how many texels a layer holds |
| `chain` returns level zero **verbatim** as its first element | FR-6.1-S3 compares the whole structure, and `a_key_the_supply_covers…` reads `levels.first()` as the supplied art unchanged. A chain that started at level one would have no witness otherwise |
| `reduced(level, size)` takes the **source** edge and answers `(size/2)²` texels, row-major | So the output texel at `(r, c)` covers sources `(2r, 2c)`, `(2r, 2c+1)`, `(2r+1, 2c)`, `(2r+1, 2c+1)`. FR-6.1-S4's fixture is written against exactly that indexing |
| `to_stored` rounds to nearest and clamps to `0..=255` | Truncation loses the round trip the uniform-colour scenario depends on, and it is the cheaper of the two to write by accident |
| `levels_for` falls back to `placeholder_texels(key, size)` and never refuses for an uncovered key | T45's "not deleted, not repurposed". A mod author's first block is a fallback, not a refusal — and the refusal a launch does make is the client's, about the *set* |

### Arrival colours, as measured

**Four throwaway skeletons, applied in a detached worktree at `6dec196` with its
own `CARGO_TARGET_DIR` outside the repository, all removed.** None is committed.
The committed tree does not compile the file at all — `texture/mip.rs` does not
exist, so the sibling module is not in the tree and nothing reaches these tests —
which is why the lint and format readings below were taken in the worktree.

- **A** — every function answers a constant: `to_linear` 0.0, `to_stored` 0,
  `reduced` nothing, `chain` its own input as a single level, `levels_for` `Ok`
  with no levels.
- **B** — structurally complete and correct in every respect **except the
  transfer**: a stored byte read as if it were linear. This is the sRGB fault
  itself, and it is M44 run before there is an implementation to mutate.
- **C** — correct transfer, selection offset by one, reading past the end.
- **D** — C's off-by-one wrapped back into the buffer.

| Test | A | B | C | D |
|---|---|---|---|---|
| FR-6.1-S1 `a_sixteen_texel_image_prepares_five_levels…` | red | green | green | green |
| FR-6.1-S2 `two_stored_zeroes_and_two_stored_maxima…one_eight_eight` | red | **red** | red | green |
| FR-6.1-S3 `an_image_of_one_colour_reduces_to_that_colour_at_every_level` | red | green | red | green |
| FR-6.1-S4 `each_texel_of_the_reduced_level_averages_exactly_the_four…` | red | **red** | red | **red** |
| FR-6.1-S5 `a_layer_offered_fewer_levels_than_declared_is_refused…` | red | green | green | green |
| `a_stored_byte_survives_a_round_trip_through_linear_and_back` | red | green | green | green |
| `the_declared_level_count_is_derived_from_the_texture_edge` | red | green | green | green |
| `supplied_texels_of_the_wrong_count_are_refused_naming_the_key` | red | green | green | green |
| `a_key_the_supply_covers_is_levelled_from_the_supplied_texels` | red | green | green | green |
| `a_key_the_supply_does_not_cover_falls_back_to_its_generated_texels` | red | green | green | green |

**Measured, not predicted.** `mc-render`'s library target holds **86 tests** under
each skeleton — 76 that were there before and these 10. A: 76 passed, **10
failed**. B: 84 passed, 2 failed. C: 83 passed, 3 failed. D: 85 passed, 1 failed.

**All ten failures under A are assertion failures, not compile errors**, which is
what `testing.md` §2 asks of a behaviour scenario: the skeleton implements
deliberately less rather than nothing, so every test was seen to run and to be
checking what it says it checks. FR-6.1-S2's reading under B is the sharpest in the
phase and is quoted here because it is the phase's whole argument:

```
left:  [[128, 64, 128, 255]]
right: [[188, 64, 128, 255]]
```

The two channels held at 64 and 128 come back untouched in both, so what separates
them is the one channel the scenario is about.

**Collateral: none, measured across the whole workspace and not only the crate.**
Under skeleton D, after `voxforge build`: **1339 run, 1338 passed, 1 failed, 1
skipped**, and the one failure is FR-6.1-S4. 1339 is the tree's 1329 plus exactly
these ten. **The goldens are among the 1338**, which is this phase's own statement
that no pixel has moved — the reading `tasks.md` asks for and the one M49 is meant
to break. The worktree also carries `content/base/blocks/water.luau` **unedited**,
so the two `shipped_declarations_and_an_older_save` guards red in the main tree
pass here, which is what says that red belongs to the hand edit and not to this
phase.

**`cargo clippy -p mc-render --all-targets --all-features -- -D warnings` exits 0**
in the worktree with the tests wired, and `cargo fmt -p mc-render -- --check`
reports nothing against `mip_test.rs`. Taken directly, because until the
implementation lands the gate cannot see this file at all: an orphan sibling is not
in any module tree, so `cargo fmt --check` walks past it and clippy never compiles
it. **A test file that the gate cannot reach is a file whose lint debt accumulates
silently** — the same window `testing.md` names for a phase opening on a
non-compiling tree, reached by a different road.

### Weak instruments and overlaps, named

- **FR-6.1-S1 and FR-6.1-S3 overlap on the level count.** S3 compares the whole
  chain, so a chain that stopped early reddens both. M45 will therefore show two
  reds and not one. S3's own subject is the content, and comparing the structure
  whole is what stops it passing vacuously over a one-level chain.
- **FR-6.1-S2 and FR-6.1-S4 are the only two readings in the workspace that can
  see the transfer**, measured: under skeleton B every other test in `mc-render`
  is green. That is a two-witness path and both witnesses are owed.
- **`a_key_the_supply_covers…` and `a_key_the_supply_does_not_cover…` are one
  decision read from both sides.** Either alone leaves half the branch dead: a
  `levels_for` that always generates satisfies the second, one that always reads
  the supply satisfies the first and panics or errs on an uncovered key.
- **Nothing here observes a device, `mip_level_count`, or a sampler.** These are
  ten pure-function readings and they cannot tell whether the chain is uploaded.
  M49 — `mip_level_count = MIP_LEVELS` in `buffers.rs` — is the golden suite's
  reading, not this file's, and it is the one that says a pixel has not moved.
- **Alpha's colour space is correct here and untested, deliberately.**
  `Rgba8UnormSrgb` decodes RGB through the transfer function and alpha
  **linearly**, so averaging alpha linearly is what the format means rather than
  what anybody preferred — and that reason belongs on `reduced` itself, where the
  edit that would reverse it gets made. **No reading in this file discriminates
  it**, because every texture this increment ships is opaque and both treatments
  answer 255 for a constant 255. The fixtures hold alpha there on purpose rather
  than by accident. **The first translucent texture must bring a test with it**,
  and until one exists there is no scenario to write: a stakeholder cannot reach
  the property, and Out of Scope is binding.
- **The expected bytes are as good as the offline program that produced them.**
  It was written from the published transfer function rather than from anything in
  this workspace, and the two implementations it was checked against — stored-byte
  and gamma-2.2 — answer 128 and 186 where it answers 188. What no reading here
  establishes is that 188 is what a *device* would produce sampling the same
  texture, which is FR-6.2's ground and is captured, not computed.

### The fixture constraint no assertion could enforce, and the one that could

`a_key_the_supply_covers_is_levelled_from_the_supplied_texels` needs its supplied
art to **differ** from what the key generates, or a `levels_for` ignoring the
supply reads as correct. Unlike FR-4.1-S1's version of the same constraint, this
one is inside a pure function and the test asserts it: the climbing ramp is
compared against `placeholder_texels` for the same key before the reading is taken,
and that guard was seen to pass under skeleton A while the reading itself failed.

### Nothing in this phase is wired

`mip_level_count` stays 1 and the sampler stays nearest. **If a golden reddens in
this phase, a pixel has moved and the one-re-shoot constraint is broken** — revert
rather than re-mint.

---

## Phase 9 — Real pixels

### The mapping, as authored

**Six file names moved from the tasks stage's guesses**, and one scenario's home
with them. The reasons are worth a line each rather than a diff: FR-6.2-S1 and
S2 went into a new `terrain_sampling.rs` rather than into `terrain_offscreen.rs`
because that file is at 460 of a 600-line cap and the three sampling readings
plus their fixtures are about 150 lines — putting them there would have spent the
headroom that file's own scenarios may need. FR-6.2-S5 joins them rather than
standing alone, because all three need one device, one `mod support;` and one
statement of what the terrain sampler is measured against. FR-8.1-S5 went into a
file of its own rather than into `terrain_probes.rs`: it is the one reading in
this spec that must not come to share a path with a golden, and a file whose
whole header says so is a better place to keep that than a paragraph inside a
suite about something else.

| Scenario | File | Test |
|---|---|---|
| FR-4.1-S1 | `crates/mc-client/tests/the_built_set_fills_its_layers.rs` | `a_covered_keys_layer_is_filled_from_its_image_and_not_from_the_generator` |
| FR-4.1-S2 | `crates/mc-client/tests/the_built_set_fills_its_layers.rs` | `two_covered_keys_are_each_filled_from_their_own_image` |
| FR-4.1-S3 | `crates/mc-client/tests/the_built_set_fills_its_layers.rs` | `the_block_no_quad_draws_still_holds_a_layer_and_no_face_takes_its_key` |
| FR-4.1-S4 | `crates/mc-client/tests/the_built_set_fills_its_layers.rs` | `an_image_named_for_a_key_no_block_declares_leaves_the_launch_alone` |
| FR-4.2-S1 | `crates/mc-client/tests/an_unauthored_key_draws_a_generated_texture.rs` | `a_declared_key_the_set_does_not_cover_is_filled_from_the_texture_generated_for_it` |
| FR-4.2-S2 | `crates/mc-client/tests/an_unauthored_key_draws_a_generated_texture.rs` | `one_covered_key_and_one_uncovered_key_are_filled_differently_in_the_same_run` |
| FR-4.2-S3 | `crates/mc-client/tests/an_unauthored_key_draws_a_generated_texture.rs` | `a_current_set_covering_nothing_leaves_every_declared_key_on_its_generated_texture` |
| FR-4.3-S1 | `crates/mc-client/tests/an_image_the_array_texture_cannot_hold.rs` | `an_image_larger_than_a_layer_refuses_the_launch_naming_the_key_and_both_edges` |
| FR-4.3-S2 | `crates/mc-client/tests/an_image_the_array_texture_cannot_hold.rs` | `an_image_that_is_not_a_png_refuses_the_launch_naming_the_key_and_the_file` |
| FR-6.2-S1 | `crates/mc-render/tests/terrain_sampling.rs` | `a_magnified_face_shows_its_two_texel_colours_and_nothing_between_them` |
| FR-6.2-S2 | `crates/mc-render/tests/terrain_sampling.rs` | `moving_the_eye_half_a_texel_moves_fewer_distant_pixels_through_the_terrain_sampler` |
| FR-6.2-S3 | `crates/mc-render/src/texture/sampler_test.rs` | `the_terrain_sampler_asks_for_nearest_magnification_and_linear_minification_without_anisotropy` |
| FR-6.2-S4 | `crates/mc-render/src/texture/sampler_test.rs` | `a_request_whose_clamp_stands_above_one_is_reported_as_asking_for_anisotropy` |
| FR-6.2-S5 | `crates/mc-render/tests/terrain_sampling.rs` | `a_sampler_the_device_will_not_build_refuses_the_launch_naming_what_was_requested` |
| FR-7.2-S1 | `crates/mc-client/tests/art_and_renderer_failures_are_told_apart.rs` | `a_model_edited_since_the_build_stops_a_golden_run_as_a_stale_set` |
| FR-7.2-S2 | `crates/mc-client/tests/art_and_renderer_failures_are_told_apart.rs` | `a_run_with_no_set_built_at_all_stops_as_an_absent_set` |
| FR-8.1-S1 | `crates/mc-client/tests/the_shipped_blocks_draw_their_baked_art.rs` | `the_grass_blocks_upward_face_draws_the_image_baked_from_the_models_top` |
| FR-8.1-S2 | `crates/mc-client/tests/the_shipped_blocks_draw_their_baked_art.rs` | `the_grass_blocks_underside_and_every_dirt_face_draw_one_image` |
| FR-8.1-S3 | `crates/mc-client/tests/the_shipped_blocks_draw_their_baked_art.rs` | `the_grass_blocks_four_sides_draw_four_images_no_two_of_which_are_alike` |
| FR-8.1-S4 | `crates/mc-client/tests/terrain_goldens.rs` | `every_declared_capture_matches_the_golden_committed_for_it` *(the suite's existing test; no second one was written, because a duplicate judging the same captures through the same settings would be one more thing to re-mint and no more evidence)* |
| FR-8.1-S5 | `crates/mc-client/tests/the_grass_top_the_camera_sees_is_its_baked_image.rs` | `the_pixel_the_declared_camera_puts_a_grass_top_on_shows_that_images_own_colour` |
| FR-8.1-S6 | `crates/mc-client/tests/the_baked_faces_are_the_model_seen_from_outside.rs` | `every_baked_face_agrees_with_the_model_plane_it_is_a_view_of` |
| FR-8.1-S7 | `crates/mc-client/tests/a_drawn_side_shows_turf_above_dirt.rs` | `every_drawn_grass_side_shows_turf_high_on_it_and_dirt_low` |
| FR-8.1-S8 | `crates/mc-client/tests/a_drawn_side_keeps_its_left_to_right_order.rs` | `every_drawn_grass_side_runs_the_way_its_image_does` |

**Twenty-three new test functions for twenty-four scenarios**, the twenty-fourth
being FR-8.1-S4's, which already existed. FR-8.1-S6 to S8 were added to the spec
on 2026-08-19, after the phase had otherwise closed and the project owner found
the defect by looking at a golden; the section below is the record of why nothing
already written could see it.

### The orientation gap, which is the reason S6 to S8 exist

**A colour-based reading cannot see geometry, and until 2026-08-19 this feature
had nothing but colour-based readings.** Every instrument the spec built compares
**means, histograms or set membership** — the delta-E figures, the distinct-colour
counts, `share_within_any`, `landmarks`, `swatch_reading`, the probes' clusters —
and **every one of those is invariant under rotation, reflection and
permutation**. So is FR-8.1-S3's pairwise inequality: turn all four sides and
they stay four unequal images. FR-8.1-S5 judges the *top* face, which was the one
face the renderer had right. The goldens were minted from the broken output and
enshrined it. **1366 tests, none of which could see turf along the bottom edge of
a face**, and the report that arrived instead was a person looking at a picture.

That is the general form and it belongs in `docs/technical/testing.md`, which
outlives this folder: **when a claim is about where something sits, no statistic
over what is present can carry it.** The narrower corollary is the one that bit
here — a set-membership test over a face's colours passes for all eight dihedral
transforms of that face, so a suite built entirely out of them has exactly zero
instruments on orientation while looking fully covered.

#### What each of the three can and cannot see

| Reading | Sees | Cannot see |
|---------|------|------------|
| S6, the bake | Whether each of the **six** baked images is the model's own outermost plane, texel for texel, seen from outside. Catches any of the eight transforms applied by the baker, on any face, including the block's underside that nothing in this world ever shows. | Anything the **renderer** does with the image afterwards. It was green on all six faces while five of six were drawn wrong. |
| S7, turf above dirt | Whether a drawn side face has turf along its **top** edge — the vertical half of a face's orientation. Catches a vertical flip and a quarter turn. | A **pure lateral reversal**: mirror a side left to right and the turf is still along the top. Also says nothing about the bake. |
| S8, left-to-right order | Whether a drawn side face runs in the image's **own** left-to-right order — the horizontal half. Its per-column turf counts are unchanged by a vertical flip, so it is orthogonal to S7 rather than a stronger version of it. | Which edge the turf is on. The two together pin both axes; neither pins the other's. |

**None of the three uses a golden as an oracle**, because the goldens are a
picture of the defect. The chain of oracles is **model to image to frame**: S6
judges the image against `grass-block.mcvox`, a file a person wrote, read by a
hand-written reader that shares no line with `voxforge`; S7 judges pixels against
`content/base/materials/`; S8 judges the frame against the image, which S6 has
independently tied to the model. No step is judged against itself.

**No committed row index appears either.** S6 compares whole planes, so there is
no boundary row to write down; S7's bands are the middle third of the topmost and
bottommost texel rows of a *measured* silhouette; S8's expectation for how many
columns must disagree with the reversed image is computed from the image's own
symmetries at run time. A committed row index would have been the same mistake as
a committed RGB triple — it would agree with whatever the code did on the day it
was written, and would need re-deriving every time the model's turf courses moved.

#### The discrimination proof, and why both halves needed one

S6 arrives **green**: the bake was already correct. An assertion nobody has seen
fail is not evidence, and worse, an agreement test cannot distinguish "the image
is the plane" from "this comparison cannot tell the transforms apart" — **a
laterally symmetric image agrees with its own mirror**. So each reading carries a
second number:

- **S6** counts how many of the seven non-identity dihedral transforms its
  comparison separates from the identity, and the expectation for that count is
  **how many transforms move the image off itself**, measured from the image
  alone. Two different comparisons, so the second is not a restatement of the
  first. Art that became symmetric lowers the expectation by itself instead of
  reddening a correct bake — the over-tight-assertion trap, avoided deliberately.
- **S6 also has a separate positive control**,
  `the_same_comparison_reports_every_face_whose_image_is_turned_a_quarter_turn`,
  which turns each shipped image a quarter turn and asserts the comparison
  reports disagreement on every one. That is the only evidence the reading can
  see orientation at all, and a means-or-histogram reading would report **none**
  on all six.
- **S8** requires, as a fixture premise, that the image's per-column turf counts
  are **not palindromic** — otherwise no reading whatever could tell the face
  from its mirror and this one would pass over a reversed face in silence.

#### Measured, by two independent routes

`crates/mc-client/tests/support/model.rs` (Rust) and
`content/base/models/generators/measure_face_orientation.py` (a hand-written PNG
reader, sharing no code with it) agree on every figure:

- **Identity scores 0 disagreeing texels on all six faces.** The bake is correct.
- **Every other transform scores 82 to 210 of 256.** Per face, the smallest
  non-identity disagreement is 82 (east), 90 (south), 96 (bottom), 100 (north),
  106 (west), 160 (top) — so all seven transforms are told apart on every face,
  and the derived expectation comes out at seven each.
- **The horizontal-mirror difference is 82 to 170 of 256**: east 82, south 90,
  bottom 96, north 100, west 106, top 170. **The figure `spec.md` states as "224
  of 256" is not any shipped image's mirror difference** — it is stated here as
  measured, twice, and the spec's number wants correcting by whoever owns that
  file.
- **The bottom face's mapping was wrong on first derivation and the measurement
  is what said so.** A plan view has no world up, so its image up is fixed by the
  right-handed triple rather than by the eye: `up = normal x right` gives -z for
  the top and **+z for the bottom**, which is the top's mapping *flipped*, not
  copied. Hand-derivation copied it; the transform scan reported `rot180+mirror`
  scoring 0 and identity scoring 92. **This is exactly the shape of the defect
  the phase is closing** — a per-face mapping written out case by case, one row of
  which was wrong — and it happened again, in the oracle, while writing the test
  for it. It is why `support::model` derives all six from two lines of vector
  arithmetic instead of tabulating them.

### Arrival colours for S6, S7 and S8, as measured

- **FR-8.1-S6 — GREEN, with its positive control green.** Both readings in
  `the_baked_faces_are_the_model_seen_from_outside.rs` pass: identity 0 on all six
  faces, seven transforms told apart on each, and a quarter turn of every shipped
  image reported as disagreement. It arrives green because the bake is correct;
  the control is what makes green mean something.
- **FR-8.1-S7 — RED at the assertion, on all four facings.** Measured:

      north (-Z)  top_not_turf: 1104  bottom_not_body: 1104  both_bands_read: true
      south (+Z)  top_not_turf: 1104  bottom_not_body: 1104  both_bands_read: true
      east  (+X)  top_not_turf: 1104  bottom_not_body:    0  both_bands_read: true
      west  (-X)  top_not_turf: 1104  bottom_not_body:    0  both_bands_read: true

  1104 is the band's own area, so every band was read whole and every pixel of it
  strayed. The Z pair is **inverted** — turf where dirt belongs and dirt where
  turf belongs. The X pair has **no turf in either band and dirt in the bottom
  one**, which is a **quarter turn**: the turf band runs vertically, so it crosses
  the middle of both bands and reaches the edge of neither.
- **FR-8.1-S8 — RED at the assertion, on three of four facings.** Measured, of 16
  columns:

      north (-Z)  unlike_the_image: 14   unlike_it_reversed:  0   (must be 0 / 14)
      south (+Z)  unlike_the_image:  0   unlike_it_reversed: 10   (must be 0 / 10)
      east  (+X)  unlike_the_image: 15   unlike_it_reversed: 16   (must be 0 /  4)
      west  (-X)  unlike_the_image: 16   unlike_it_reversed: 15   (must be 0 / 14)

  **North is drawn as its image exactly reversed** — 0 columns unlike the reverse
  — so it is a 180-degree turn rather than a plain vertical flip. **South is drawn
  the right way round**, and reading 0 there is worth as much as the reds: it is
  positive evidence that the texel-centre sampling and the derived image basis are
  right, because a wrong basis would not match any face exactly. **East and west
  agree with neither**, which is what a quarter turn does to a column count.

### Arrival colours, measured rather than predicted

Measured against a **throwaway skeleton** built in a detached worktree and
destroyed afterwards: the sampler module existing with today's values,
`asks_for_anisotropy` answering `false` for a constant reason, `TerrainTextures`
threaded through but ignored, `built_set` still supplying nothing,
`mip_level_count` still 1, and `grass.luau` unchanged. Nothing of it was
committed. Re-run at `a1a56fc` with every test of this phase in place:
**1365 run, 1341 passed, 24 failed, 1 skipped**, against the phase's inherited
baseline of 1339/1339/1 in a worktree.

The twenty-four are **fourteen** of the twenty scenario tests, **eight** existing
tests this phase's key-set change correctly reddens — two in
`launch_texture_layers.rs`, two in `terrain_probes.rs`, four in
`mc-sim/tests/per_face_layers.rs` — and **two** additional-coverage drivers.

**Fourteen of the twenty scenario tests arrive red, every one at an assertion or
at a `let-else` on the wrong variant — none at a compile error.** The remaining
six arrive green and each has a measured falsifier below.

| Green on arrival | Why | Falsified by |
|---|---|---|
| FR-4.1-S3 | water already spends a layer and the mesher already emits no face for it — this is a guard, not a driver | **F4**: `solid = true` on `base:water` → 11 quads appear, red |
| FR-4.2-S1 | the generated texture is already the only thing that fills a layer | **F1**: `levels_for`'s fallback returns black instead of the generator → red |
| FR-4.2-S3 | the tasks stage predicted this one; it is the vacuity control | **F1** → red |
| FR-6.2-S1 | **not predicted by the tasks stage.** Today's sampler is already `mag_filter: Nearest`, so a magnified face already shows only its two texel colours | **F2**: `mag_filter: Linear` at the device → `(between, shown) = (116, 0)`, every scanned pixel a blend and neither colour present |
| FR-7.2-S1 | `prepare_scene` already consults the set's verdict, and the goldens already go through it | **F3**: `refusal_for` answers `None` for `StaleAgainstSources` and `Absent` → the golden run reports `Some(Pass)` **over an edited model**, which is the confusion the scenario exists to prevent, reproduced |
| FR-7.2-S2 | same | **F3** → red |

**FR-6.2-S1's green is a disagreement with `tasks.md`'s green-on-arrival list**,
which does not name it. It is information rather than a surprise: the scenario is
about magnification, this phase changes *minification*, and the magnification half
of `TERRAIN_SAMPLER` is what today's sampler already does. What it grades is that
the change does not give that up — which is exactly what M51 measures.

Each falsifier was applied by hand, run, and reverted by hand with
`git diff --exit-code` clean between them. All four bit.

### What the skeleton could not reach, and what was done instead

**FR-8.1-S5 stops at its fixture premise under an empty supply** — the set offers
nothing for `base:grass_top`, so the colour it would judge against is the
generated stand-in. To get the assertion itself to run, the skeleton was given a
stand-in supply of one flat colour for that key while the upload path still drew
the generator. Measured: the oracle chose pixel **(380, 460)**, the frame drew
**`[164, 129, 71]`** — a `base:grass` placeholder texel, mean minus one step —
and the reading reported **ΔE 43.03** against grass_top's mean `[104, 165, 78]`.

That is worth three separate things. The ray march, the projection and the
four-neighbour agreement all work and pick a pixel that genuinely shows a grass
block's top face. The assertion is reached and fails at the assertion. And 43.03
sits within 0.7 of the **42.35** predicted offline for generated-against-baked,
which is a measurement agreeing with a prediction rather than either standing
alone.

### The tolerances, derived from both directions

- **FR-8.1-S5, `SHOWS_THE_IMAGE = 12.0`.** Above: every texel of the built
  `base:grass_top` is within **ΔE 9.09** of that image's linear-light mean (100%
  within 10), and a minified face converges towards the mean rather than away
  from it. Below: the nearest wrong answer is a grass side at **38.00**, the
  generated `base:grass` at **42.35** and `base:dirt` at **48.49**. Anything in
  (9.09 + the sRGB round trip, 38.00) works; 12 is taken.

  **Say what it cannot do, because a tolerance with a stated blind spot is worth
  more than a tighter one without: this reading cannot tell one grass-top texel
  from another; it can only tell grass-top art from every other thing that pixel
  could be showing.** It would not notice the image reflected, rotated or
  shuffled. What it does tell apart is the dirt underneath, the sides beside it,
  the stone pillar, the sky, and the generated stand-in this spec replaces —
  measured at ΔE 43.03 for the last of those, against a frame the skeleton drew
  from the generator.
- **FR-6.2-S1, `SAME_TEXEL = 2.0`.** Two placeholder texels of opposite parity
  stand about ΔE 7 apart, so a blend halfway between them sits about 3.5 from
  each; 2 is under that and over a one-unit sRGB encode difference. Measured
  under the skeleton: 116 pixels scanned, **0 between, 2 shown** — so the encode
  contributes nothing at this tolerance, and under linear magnification all 116
  become blends.
- **FR-6.2-S2, `MOVED = 10.0`.** The project's own "told apart" ceiling. It sits
  over what linear minification moves a pixel by when the camera shifts half a
  texel and far under the whole contrast between two texels, which is what point
  sampling moves a pixel by when its sample crosses a texel boundary.

### The offline figures, what checks them, and what does not

Every colour figure this phase states was taken from the built set by programs
outside the workspace. **They are offline-computed and no test asserts one of
them**; what stands behind them is that there are now **three** independent
routes to the same numbers, not one:

1. T46's program, `mc-testkit`'s `color.rs` copied verbatim.
2. The check binary recorded under T46, calling the shipped `compare`, `chain`
   and `placeholder_mean_color` plus `content/base/materials/`.
3. **This phase's own program** — a hand-written PNG reader (zlib plus all five
   filter types), the sRGB transfer function, the sRGB→XYZ(D65) matrix and CIE76
   all written from the published formulae, sharing no line with either of the
   above. **Committed, at
   `content/base/models/generators/measure_built_textures.py`**, beside the model
   generators and under the same terms: provenance, not a build step, run by
   nothing and asserted by no test. It needs no third-party package and derives
   the set's path from its own location, so
   `python content/base/models/generators/measure_built_textures.py` reproduces
   every figure on any checkout that has run the art build.

**Routes 1 and 2 share their CIELAB conversion** — route 2 calls the shipped
`compare`, which is what route 1 copied — so between them they establish that the
figures predict what `probe.rs` computes. Route 3 is the one that establishes the
colour maths is *right* rather than merely consistent, because it agrees with
them having been written from the specification instead.

Route 3 reproduces **every** figure of route 1 to the last printed decimal: the
distinct-colour counts 3/5/6, both means per texture, the seven means-apart
figures 0.39–2.38, the furthest-texel figures 9.09/9.56/12.07/39.08–40.10, the
within-ΔE-10 shares 0.00/43.36/66.41/100.00%, all 21 pairwise ΔE including the
**9.59** for `base:dirt` against `base:grass_side_west`, and the three
generated-against-art figures 62.94/42.35/55.56.

**Figures this phase adds, with what checks each:**

| Figure | Where it is used | Check |
|---|---|---|
| The three cross-stratum minimum landmark distances — dirt↔stone **21.13**, dirt↔grass_top **42.62**, grass_top↔stone **48.80** | `probe.rs`'s three strata being separable at ΔE 10; `hud_held_block.rs` pairing dirt against stone | Each is over **2 × 10**, so a pixel within 10 of one stratum's landmarks cannot be within 10 of another's — a property, not a coincidence, and it is what makes the shares non-overlapping |
| The widest intra-stratum midpoint stray — **5.70** (stone), 4.77 (dirt), 2.54 (grass_top) | `art::landmarks`' claim that a blend between two of a layer's colours stays inside the set | Under ΔE 10 on all three; **no second route** |
| **81.25% – 82.81%** of a grass side's texels within ΔE 10 of a `base:dirt` landmark, over all four sides | the honesty note on `STRATA`'s dirt floor | **No second route.** It is a tally over decoded texels. Stated as a range: one side's figure presented as the general case is the error corrected below |
| ~~`base:stone`'s three colours `[96,96,96]`, `[125,125,125]`, `[154,154,154]`~~ | **no longer a figure this phase states** | Route 2's finding that **a face bakes its material colour unshaded** removed the need for it: FR-4.1-S1 now names the three *materials* `stone-block.mcvox` is built from and reads their colours out of `content/base/materials/`. See below |
| The dirt image's rarest colour at **28 texels of 256** | `hud_held_block.rs`'s claim that a 24 × 24 fill over a 16 × 16 texture shows all three | 10.9% of 256 = 27.9; and 24 pixels across 16 texels puts every column and row on at least one pixel |

### FR-4.1-S1's expectation is read from the palette, not snapshotted

**Route 2's strongest finding retired a committed number.** Every distinct texel
colour in all seven shipped images is byte-identical to a material declared in
`content/base/materials/`, and each face uses exactly the materials it should —
so **a face bakes its material colour unshaded**. That makes "what is
`base:stone`'s image made of" answerable from a file a person wrote instead of
from a run of the decoder.

So `STONES_COLOURS`, three committed RGB triples, became `STONES_MATERIALS` —
`["stone", "stone_dark", "stone_light"]` — and `art::declared_material_colors`
reads `#7d7d7d`, `#606060` and `#9a9a9a` out of the three TOML files. Which
materials a model is built from is a statement about the art and belongs in the
test; what colour each of them is belongs to the palette.

It is a stronger expectation and a weaker commitment at the same time. A decoder
that swapped two channels, applied a transfer function it should not have, or
shaded a face lands on colours no material declares — **and not one of those
three would have moved a committed triple.** A deliberate palette edit now flows
through both sides at once, which is correct: the image is rebuilt from the same
files.

**Verified in both directions, against a skeleton.** Supplied the three colours an
independent PNG decode reported, the reading passes — so the TOML parse and the
"unshaded" claim are both confirmed, from two places that share nothing. Moved
one channel of one colour by **one byte** and it reddens, naming the disagreement
compactly. The parse is a text read rather than a TOML parse for the reason this
suite gives elsewhere: these files are content, and a reading about them should
not go through the same crate the loader does.

### A figure this file got wrong, and the shape of the error

**Corrected 2026-08-19, found by the P9 implementer re-running the committed
measurement program while writing `docs/technical/testing.md`.**

Two figures here condensed a *set* into its extremum and read as the general
case:

- "0% of a grass side's texels sit within ΔE 10 of that side's mean" is true of
  **three** of the four sides. West is **43.36%**.
- "81.6% of a grass side's texels sit within ΔE 10 of a `base:dirt` landmark" is
  north's figure. The four run **81.25% to 82.81%**.

**The source was right and the condensation was wrong.** `tasks.md`'s T46 block
carries "(43.36% for west)" in the sentence this file was summarising; the
parenthetical was dropped on the way in. Neither figure changes a conclusion —
the premise `share_within_any` replaced is false either way, and the dirt floor
is dominated by grass sides at 81% as surely as at 82% — and neither is asserted
by any test. What was wrong was the *record*.

**This is the second instance of one shape in this spec**, and that is why it is
written down rather than quietly edited. The first was T46's own margin figure,
which reported the farthest approach as the closest. Both are a program's output
condensed into prose that afterwards reads as measured, and both were caught the
same way: **somebody re-ran the derivation instead of reading the number.**
`docs/technical/testing.md`'s rule — *state the figure that binds, not a set the
reader must minimise over* — was written for the first and would have caught the
second, so the failure here is that this file's author knew the rule and applied
it to tolerances rather than to descriptive figures.

The correction is a range or a three-and-one split at every site, in
`support/art.rs`, `support/probe.rs` and here. **The committed program is what
made it a five-minute check**, which is the argument for having committed it.

### `art.rs`, and the two means

`crates/mc-client/tests/support/art.rs` is new and is the one place this suite
decides what a layer is filled with. It **reads a file and never a frame**: the
built set's own image through the client's reader for a key it covers, the
generator for a key it does not.

**The mean it declares is the linear-light one**, computed by a transfer function
written there from IEC 61966-2-1 and sharing no code with
`mc_render::texture::mip` — deliberately, since the mip chain is what *produces*
the pixels a minified face shows and an oracle calling into it would be checking
an arithmetic against itself. `means_agree` is the check on it: the stored-byte
mean is integer arithmetic with no transfer function in it, and the two must stand
within **ΔE 3.0** — above the widest separation any shipped texture shows
(**2.38**, on the north and south grass sides) and far under what a transfer
applied backwards would give. **What that check does not catch** is a transfer
that is simply absent, which puts the two at zero; `drawn_colors_of` refusing a
one-colour layer is what covers that.

**The flat mean and the smallest mip level are not always the same byte.** T46's
check measured `base:dirt` at `[139, 106, 71]` through the chain against
`[138, 106, 70]` flat, and similarly for grass_top and stone — one byte on one or
two channels, ΔE 0.39 to 0.68, from rounding at four halvings rather than once.
Every tolerance here is an order of magnitude above that, so the choice decides
nothing in this phase; it is recorded on `linear_mean` itself so a later reading
tight enough to care knows which it has.

### The three invalidated premises, as rebuilt

- **`swatch.rs`'s `TEXEL_COLORS = 2` is kept and re-scoped, not widened.** It is
  exact for the *generator* and it is not a bound on anything else, so
  `texel_colors` stays — correct wherever the set covers nothing, which is every
  `example:` block in the reload suites. What is new is `drawn_colors_of(key,
  supplied)`, which answers with whatever that layer holds and refuses a layer of
  fewer than two colours. Callers whose key the set now covers moved to it:
  `hud_held_block.rs` and `hud_prediction.rs`, both of which hold `base:dirt`.
- **`probe.rs`'s `STRATA` is re-derived, and the reading under it is rebuilt.**
  The keys become `base:dirt`, `base:grass_top`, `base:stone` — texture keys, not
  block names, and `base:grass` is no longer a key at all. The means come from
  the built set through `art.rs`. **The three floors did not move**: they are
  geometric bounds and no colour bears on them. What was replaced is
  `share_within(frame, mean, 10)` — measured false, since **three of the four
  grass sides have 0.00% of their texels within ΔE 10 of their own mean and the
  fourth, west, has 43.36%**, while stone reaches only 66.41% — by
  `share_within_any(frame, landmarks, 10)`, where the landmarks are the layer's
  own colours *and* its mean.
- **`terrain_offscreen.rs`'s centre-pixel comparison was measured and stands.**
  Its fixtures are `example:` blocks that no built set covers, so they draw
  generated textures whose checkerboard averages to the declared mean at every
  mip level — a minified frame converges *towards* the value it is compared
  against, not away from it. `SAME_TEXTURE = 10.0` is untouched, and the file's
  two depth readings were green in every run above. **Recorded as re-measured
  rather than as repaired**, because `spec.md` lists it as invalidated and a
  reader meeting no change needs to know it was looked at.

### Premises this phase invalidates that the spec's list does not name

**Twelve files, not three, and the count is the finding.** When T53 was still
ahead of us this table named the three the test author could see by reading. T53
landed and reddened **twenty-three** tests across twelve files, every one of them
the same mechanical consequence: `grass.luau` stops declaring
`texture = "base:grass"` and states six facings instead, so the shipped key set
goes from four to **eight** —

```
base:dirt 0, base:grass_side_east 1, base:grass_side_north 2, base:grass_side_south 3,
base:grass_side_west 4, base:grass_top 5, base:stone 6, base:water 7
```

Dirt keeps layer 0; `base:grass` is gone as a key; stone moves 2 → 6 and water
3 → 7. **That renumbering is why the goldens are re-shot**, and it lands with T53
rather than with the sampler.

**Reading the tree found three of twelve. Running it found twelve.** That is worth
saying plainly rather than quietly correcting the number: a survey by grep over
one spelling — `"base:grass"` — cannot see a fixture that computes a layer from
`SHIPPED_KEYS.len()`, or one that treats a key list as a block list because the
two used to coincide. Nine of these were invisible to the method that found the
first three.

**Promoted to `docs/technical/testing.md`** — §"Reading the tree found three of
twelve; running it found twelve" — with the remedy attached (*land it and read
the failures, rather than surveying harder*) and the corollary that nineteen came
back on one edit because the fixture module derives from one list. This folder is
archived and pruned; that file is where a reader meets it.

| File | Failing | What it pinned |
|---|---|---|
| `crates/mc-client/tests/launch_texture_layers.rs` | 2 | `DIRT=0, GRASS=1, STONE=2` and four keys |
| `crates/mc-sim/tests/per_face_layers.rs` | 4 | `SHIPPED_KEYS: [&str; 4]`, and every arithmetic expectation over it |
| `crates/mc-client/tests/hud_held_block.rs` | — *(turned before T53 landed)* | dirt against grass as two blocks sharing no colour |
| `crates/mc-client/tests/reload_appends_layers.rs` | 4 | `base:amber` at 4; the four-key map; the spent count 4; a retired-and-re-declared key at 5 |
| `crates/mc-client/tests/reload_layer_budget.rs` | 3 | fixture arithmetic over a root that spends four layers |
| `crates/mc-client/tests/reload_builds_off_the_tick.rs` | 2 | the four-key map |
| `crates/mc-client/tests/reload_keeps_packed_layers.rs` | 2 | one quad per *key*, packed as though a key were a block name |
| `crates/mc-client/tests/reload_refusal_ends_one_attempt.rs` | 2 | the layer a corrected candidate's key takes, and a budget refusal's arithmetic |
| `crates/mc-client/tests/documented_refusals.rs` | 4 | the budget arithmetic, **and two refusals the modding page now quotes that no producer made** |
| `crates/mc-client/tests/reload_appends_a_drawable_layer.rs` | 1 | the appended layer as 4 |
| `crates/mc-client/tests/reload_hand_shows_the_new_block.rs` | 1 | the same, as `Some(4)` |
| `crates/mc-client/tests/reload_publishes_content.rs` | 1 | the four-key map, and prose saying "the four keys stay exactly where they were" |
| `crates/mc-client/tests/reload_refuses_an_uncovered_facing_key.rs` | 1 | a budget refusal's arithmetic |
| `crates/mc-client/tests/saved_world_texture_layers.rs` | 1 | stone at layer 2 |
| `crates/mc-client/tests/hud_prediction.rs` | 1 | that nothing behind the indicator already reads as a colour it draws |

**Nineteen of the twenty-three came back on one edit**, because
`support/reload_content.rs` derives every layer expectation from one list and
spells no index as a digit. That design is the reason this was an afternoon and
not a week, and it is worth naming as the thing that paid: a fixture that had
written `4` in twenty places would have had to be found in twenty places.

**Four needed judgement rather than arithmetic**, and each is recorded where it
was made:

- **`reload_appends_layers.rs` — a declaration now retires *five* keys.** Taking
  `grass.luau` away used to retire one key; it now retires `base:grass_top` and
  the four sides while `base:dirt` stays, because the dirt block declares it too.
  `layers_without` widened from one key to a slice for exactly that, and the
  scenario got **stronger**: stone and water keeping 6 and 7 rather than sliding
  to 1 and 2 is a wider claim than one key not sliding was.
- **`reload_keeps_packed_layers.rs` — a key is not a block name any more.** It
  packed one quad per *key*, using each key as a `BlockName`; that was safe while
  the two coincided and produced `RefusedNaming("base:grass_side_east")` the day
  it stopped. It now packs one quad per *block* and expects each block's upward
  facing's key, which is what the packer actually does — so the fixture crosses
  the same join the code does.
- **`saved_world_texture_layers.rs` — the number moved and so did its reason.**
  Stone at 2 was true because the goldens had never moved; stone at 6 is true
  because they were deliberately re-shot. The constant says so, because the
  assertion — *a save cannot move a layer index* — is unchanged and a reader
  meeting a 6 where a 2 is recorded elsewhere should meet a decision rather than
  a defect.
- **`hud_prediction.rs` — the fixture this file predicted would fire, firing.**
  See below.

**The spec's own premise list is what separates an intended inversion from a test
adjusted to match an implementation.** The team lead's amendment names three; it
is short by nine, and this table is what it should carry.

### `hud_prediction.rs`, and the premise that did its job

The prediction of this one is in this file's own earlier record, and it landed
exactly as written: `require_nothing_already_reads_as_the_indicator` refused with
**`considered: 676, strayed: 0`** — every pixel of the 26 × 26 footprint already
read as one of the colours the indicator draws.

The cause is the one predicted. A client holds the first solid block in
registration order, which is dirt; the indicator draws the baked dirt palette;
and those three browns are over four fifths of every grass side, which is what the terrain
behind the indicator is made of at this pose. So *every pixel of the footprint
moves* was red against a correct renderer.

**The assertion was not weakened and no tolerance moved.** The fixture moved:
`dirt.luau` and `grass.luau` are renamed aside so a client holds **stone**, whose
greys share no colour with anything the ground is made of — the nearest pair
stands ΔE 21.13 apart against the ΔE 2.0 that calls two colours the same. That
needed `HudCapture::over`, a root-taking capture, because which block a client
holds is decided by a content root and this file's subject is the indicator
rather than the block.

**The premise passing is the measurement.** It is what says stone's palette is
genuinely absent from behind the indicator, rather than something this record
asserts.

### What this phase could not measure, and who owns it

- ~~**`hud_prediction.rs`'s `require_nothing_already_reads_as_the_indicator` may
  fire once the texels are wired, and no skeleton here could tell.**~~ **It
  fired, exactly as written, and it is closed** — see the section above. Kept
  rather than deleted, because a prediction that came true is worth more as a
  record than as a line somebody removed: the reason it was written down is that
  no skeleton could reach it, and the reason it was repairable in an hour is that
  somebody had said in advance what it would look like.
- **FR-8.1-S5's tolerance has not been read against a frame drawn from the real
  art**, only against one drawn from the generator (ΔE 43.03). The margin is
  26 ΔE wide, which is why this is recorded rather than blocking. **Closed at the
  mint**: the test passes over the built art, so the reading has now been taken
  against a frame drawn from it.
- **A defect in this phase's own tests, found by a mutation rather than by
  reading.** `art_and_renderer_failures_are_told_apart.rs`'s `refusal_of`
  debug-printed a whole `GoldenOutcome`, whose `Failed` arm carries a
  921 600-element failing mask — under M57 the message came to **5.5 MB** and
  buried the sentence a reader needs under a boolean array. That is the mistake
  `terrain_goldens.rs`'s header records making once, met a second time in a
  second file. The verdict is named now and the `Failed` arm delegates to
  `GoldenFailure`'s hand-written `Display`, which carries the useful half and no
  mask. **The general form worth keeping: a rule recorded in one file's header
  does not reach the next file**, and the thing that found it was a mutation
  making the message actually print.
- ~~**M52's prediction is not readable yet.**~~ **Closed, and it is the row with
  the most to say for the least.** Measured against the real implementation:
  `asks_for_anisotropy` returning `false` always reddens **FR-6.2-S4 and nothing
  else in 1366 tests**, while FR-6.2-S3 stays green because its expected tuple
  states `false` for a clamp of one, which is still what a dead function answers.
  **The positive control is the only instrument in the workspace that can see
  this** — argued when it was written, measured now.

- **A limitation of `support/art.rs` as an oracle, found by the implementer and
  worth keeping.** `drawn_texels` answers through `SuppliedTexels::covering` and
  falls back to the generator, which is deliberate — it is the same decision the
  production path makes, so the oracle states what a layer is *actually* filled
  from. The consequence is that killing `covering` moves the oracle and the draw
  **together**: under M50 the frame and the expectation both fall back, and
  `terrain_probes` stays green because they agree by sharing a computation rather
  than because anything was drawn. That is the shape `testing.md` §2 names.

  **It is covered, and by something other than the probe.** FR-4.1-S1, S2 and S4
  assert `covering` answers `Some` for the shipped keys, and FR-8.1-S5's own
  fixture premise does the same — all four redden under M50, so a dead `covering`
  has several witnesses. What the probes cannot do is be one of them. The
  implementer's **M50b** — mutating only the device path, leaving the pure value
  intact — is what separates the two questions, and it is the right instrument
  rather than a second row: it found that the wiring has **six witnesses besides
  the goldens**, which is the answer to "what would go red if the draw stopped
  consulting the supply" and is not "only a golden".

### One interface consequence found by building the skeleton

**`RendererError` cannot keep `Copy`.** `Texture(#[from] TextureError)` carries a
`TextureKey`, which owns a `String`. Dropping the derive was enough and nothing
else in the workspace needed changing, but it is a one-line edit the
architecture's error listing does not mention and it fails the build immediately
if missed.

### Two things the gate would have caught and the suite would not

Run directly, because a green suite is no evidence about a lint:
`cargo clippy --workspace --all-targets --all-features -- -D warnings` reported
**fourteen** findings across these files on their first compile — excessive
nesting, `indexing_slicing`, `integer_division`, `manual_is_multiple_of`,
`too_many_lines` on five test functions and `too_many_arguments` on three
helpers — and `rustfmt --check` reported diffs in six. Both are clean now. The
`too_many_arguments` findings are the same pressure `TerrainTextures` exists to
relieve, met one level down in the test harness.

### FR-8.1-S1 constrains the upward face and nothing else

**Measured by M55, and it is a statement about which scenario owns which
property rather than a defect in a test.** Reverting `grass.luau` to a single key
reddens FR-8.1-S2, FR-8.1-S3 and both golden suites, and leaves **FR-8.1-S1
green** — 23 failures in the workspace and S1's test is not among them.

The reason is that the mutation was spelled `texture = "base:grass_top"`, so all
six faces draw that key. `key_of(grass, Face::Up)` still answers
`base:grass_top`, the manifest still bakes it from `top`, and the set still
covers it. **Every element of S1's tuple still holds, truthfully.**

So S1 cannot tell a six-facing declaration from a single-key one whose single key
happens to be the top key. **The pair is complete; S1 alone is not.** FR-8.1-S2
(the underside sharing dirt's image) and FR-8.1-S3 (four sides, pairwise unequal)
are what carry the six-facing claim, and between them they caught the mutation
immediately.

Written here because **the next person reading S1 in isolation will assume it
covers more than it does.** It is the same shape as this file's other
single-witness notes, with the difference that here the missing coverage is
genuinely somebody else's — two scenarios that exist and do the job — rather than
a hole.

### Two mutation rows that were wrong, in both directions

Both are M50 — `SuppliedTexels::covering` returning `None` always — and both are
information rather than failures, which is what `tasks.md`'s own note about
under-predicted rows asks for.

- **Over-predicted: FR-8.1-S2 stays green.** The table said it would redden. It
  reads a declaration and the shipped manifest and never touches the supply, so
  it is a declaration-and-manifest reading rather than an art one. That is worth
  knowing about the scenario, not just about the row: S2's subject is *which
  image two faces share*, and sharing is a property of the declaration.
- **Under-predicted: FR-4.2-S2 reddens.** The row said "every FR-4.2 stays
  green". The mixed reading asserts the *covered* half as well as the uncovered
  one, so it correctly reddens when nothing is covered — **the row being wrong in
  the direction that says the test earns its keep**, and exactly what that test's
  own header claims for its middle element.

### FR-8.1-S4 and FR-8.1-S5 must never come to share a path

S4 is a golden minted from the renderer it verifies. **S5 is the only independent
judgement of the picture this spec has**: the grass top face judged against a mean
computed from the built PNG, decoded by the **client's** decoder and never by the
draw. If a refactor lets S5 read a value the frame produced, the pair collapses
into one snapshot.

**No mutation detects this.** It is reviewer-held, and it is written here because
that is its only defence.

### The pairs, and why each half is owed

- **FR-6.2-S3/S4 assert the request; FR-6.2-S1/S2 assert the consequence.** A test
  that reads back the descriptor it caused to be built is agreement between two
  copies of one decision. S4 is S3's positive control as a separate test function —
  without it, `asks_for_anisotropy` returning `false` unconditionally leaves S3
  green forever.
- **FR-4.1 and FR-4.2 are the two halves of one branch.** A supply that covers
  nothing leaves every FR-4.2 green and every FR-4.1 red; a supply consulted for
  every key leaves FR-4.2's fallback dead. Neither half alone constrains the
  branch.
- **FR-5.2 and FR-4.2 together are what keep the two messages apart**: "the build
  step was not run" refuses the launch by name; "this key was never authored" is a
  silent, documented per-key fallback with no message on the terminal.

### The fixture constraint no assertion can enforce

**FR-4.1-S1's built image must hold texels that differ from the texture generated
for the same key.** An image that happens to match the generated texture leaves the
scenario green under an implementation that ignores the image entirely. The
scenario says so in its own wording, and it is restated here because it is a
property of the fixture rather than of any assertion.

Likewise **FR-8.1-S3**: stone's six faces are already six distinct images even
though the block is uniform noise, so "four different model faces" does not imply
"four different images" by construction. It must be measured.

### Test premises this phase invalidates, listed so none is loosened until green

| Premise | Where | What replaces it |
|---|---|---|
| `TEXEL_COLORS = 2` | `crates/mc-client/tests/support/swatch.rs:35` | re-derived against the shipped art — a real face has three to six colours, measured |
| `STRATA` clustered against `placeholder_mean_color`, `COVERAGE_FLOOR`, `SAME_COLOR = 2.0`, `DIFFERENT_COLOR = 10.0` | `crates/mc-client/tests/support/probe.rs:124` | means re-derived from real texels, ΔE constants re-checked against art far less separated than three hash-derived colours. **P8's T46 measures these offline before any pixel moves** |
| centre-pixel comparison to `placeholder_mean_color` under `SAME_TEXTURE` | `crates/mc-render/tests/terrain_offscreen.rs` | re-derived against the shipped art |

`crates/mc-render/src/texture/placeholder_test.rs` **stays valid**: the generator is
not deleted and its pairwise-separation and variation tests keep guarding the
fallback path, which FR-4.2 now depends on.

**The file these constants live in has 49 lines of headroom.** `probe.rs` is at 551
non-blank lines against a 600-line test limit, measured with the gate's own counter
at `87bbb84`; `terrain_offscreen.rs` is at 460 of 600. Re-measure before writing.
A cap hit while re-deriving a tolerance does not reject the re-derivation — it
rejects whatever is cheapest to drop, which here is the comment recording where a
number came from, and that comment is the only thing standing between the next
reader and a constant nobody can re-derive. T46's offline measurements are the
input that keeps the growth to the constants themselves.

**Expect one round of re-derivation to be wrong and to be caught by FR-8.1-S5
rather than by FR-8.1-S4.** That is what the pair is for. If two strata land inside
`SAME_COLOR` of each other, that is a spec conversation, not a tolerance edit.

### Additional coverage

**All four the tasks stage asked for are written.** The fourth arrived last, on
the team lead's ruling that a known hole is not deferred to a later phase — its
RED is spent the moment the implementation lands, so it had to exist first.

| Test | File | Arrival | What it catches |
|---|---|---|---|
| `exactly_one_file_of_the_client_names_the_image_decoder` | `crates/mc-client/tests/the_decode_stays_at_the_composition_root.rs` | **red** — no file names it yet | `image::` spreading out of `textures/decode.rs`. The verdict is the **path**, not a count, so a decoder that moved is a different answer rather than the same one |
| `the_same_scan_reports_a_file_that_does_name_the_image_decoder` | same | green | the scan above going quiet — a walk that stopped descending reports "nothing found", which reads exactly like a boundary being kept |
| `no_file_of_the_renderer_names_a_filesystem_type` | same | **green on arrival** | `std::fs`, `PathBuf` or `std::path::` arriving in `mc-render/src/` — the constraint D4 is forced by, which nothing else in this spec would notice and which compiles and draws the same picture either way |
| `the_same_scan_reports_a_file_that_does_name_a_filesystem_type` | same | green | the same absence going quiet |
| `the_renderer_resolves_no_blocking_executor_without_its_default_features` | `crates/mc-render/tests/dependency_graph.rs` | **red** | `pollster` reaching the pure half. Measured: it reaches it today through `mc-render`'s own `[dev-dependencies]` and through nothing else (`cargo tree -i pollster` names one edge), so T51's move to an optional `[dependencies]` entry under `gpu` **must remove the dev entry** for this to go green — that is what the test is asking for |
| `the_renderer_resolves_the_blocking_executor_with_its_default_features` | same | green | the absence above becoming a comparison of two identical configurations |
| `a_reload_that_appends_a_key_finds_the_supply_the_renderer_was_constructed_with` | `crates/mc-client/tests/a_reload_keeps_the_supply_the_renderer_was_built_with.rs` | **red** | a `SuppliedTexels` threaded through `Unuploaded` or the re-mesh worker's retirement, which D4 deliberately does not do — a reload would then re-fill every layer from the generator, and a world drawing its baked art would go back to hash-derived colours the moment somebody saved a block file |

**Written, in a file of its own** —
`crates/mc-client/tests/a_reload_keeps_the_supply_the_renderer_was_built_with.rs`,
holding `a_reload_that_appends_a_key_finds_the_supply_the_renderer_was_constructed_with`.
It is the only guard on D4's "`SuppliedTexels` is held for the whole run and is
**not** carried by `Unuploaded`/`retire`", and it arrives **red**.

**The fixture is one renderer across two uploads, and that is the whole of it.**
`reload_draws_the_new_block.rs` builds a fresh renderer per frame — right for what
it asks, and exactly why it cannot see this: a renderer that never survives an
upload cannot lose anything between two of them. So this run constructs one,
gives it the content root's supply, uploads the pre-reload layers and draws a
stone face; then uploads the layers a reload handed over **on that same
renderer** and draws the same pose. `upload_textures` re-fills every layer it is
given, so the second upload is where a replaced supply would show.

Nothing is placed. The reload appends a key and the frame keeps showing the
floor, so what changed between the two frames is an upload and nothing else.

**The reading, and why it is three ways rather than one.** Every pixel of the
32 × 32 square after the reload is one of the colours the *built image* holds
(`strayed_from_the_image: 0`); **none** of them is a colour the *generator* makes
(`strayed_from_the_generator: 1024`); the same holds before the reload; and the
square is pixel-for-pixel identical in both frames. A supply threaded through the
reload path and arriving empty inverts the first two and breaks the fourth.

**Measured at the assertion, not merely at a premise.** Under an empty skeleton
the run stops at its fixture premise, correctly — the set offers nothing, so both
sides would be the fallback compared with itself. Given a stand-in supply so the
assertion runs, it reported

```
left:  strayed_from_the_image: 1024, strayed_from_the_generator: 0,
       strayed_before_the_reload: 1024, the_two_frames_agree: true, considered: 1024
right: strayed_from_the_image: 0,    strayed_from_the_generator: 1024,
       strayed_before_the_reload: 0,    the_two_frames_agree: true, considered: 1024
```

— exactly inverted, which is what an unwired supply looks like. `considered:
1024` is 32 × 32, so the square is wholly on the frame, and `strayed_from_the_
generator: 0` says it is wholly on the stone face rather than overlapping sky or
an edge. That is the pose derivation confirmed by measurement rather than
asserted.

`the_two_frames_agree` is **already true** and must stay true: it is the guard
half, and it holds because a reload appends a key rather than renumbering one —
stone keeps its layer, so the scene needs no re-packing.

**A lint in this file was invisible until the implementation landed, and that is
the adaptation window doing exactly what `docs/technical/testing.md` says it
does.** The file names `TerrainTextures`, `TERRAIN_SAMPLER` and
`TerrainRenderer::new`'s fourth argument, so it did not compile before T51 — and
clippy never reached it. It arrived at `too_many_lines (32/30)` on
`a_run_that_reloads_between_two_frames` the moment the tree compiled, as the only
finding in the workspace. Arbitrated `test-wrong` in the narrow sense that the
file needed reshaping, and repaired by **extracting the fixture premise** into
`require_the_art_covers_it` — a pure extraction, verified by `git diff`: not one
assertion, tolerance, expectation or fixture value moved.

The implementer's suggested cut was launch-and-first-frame against
reload-and-second-frame. It was not taken, and the reason is the file's own
subject: the two uploads have to be **adjacent in one function** for a reader to
see that they land on one renderer, which is the whole fixture. Splitting there
would have separated the two things the test exists to hold together and needed
the renderer, the scene and the root passed across the seam.

**What it does not cover, stated in the file's own header.** It uploads through
`TerrainRenderer::upload_textures` rather than through `Unuploaded::uploaded_to`,
which takes a `FrameRenderer` the frame path constructs and nothing in this
workspace does. `upload.rs`'s header already records that gap and records that
the compiler covers the omission there. What is left uncovered is a
`FrameRenderer` that lost its supply while a `TerrainRenderer` kept it, and the
two share one field.

**The scan file links nothing but the standard library.** It deliberately does
not `mod support;`: it asks a question about the *tree*, so it stays compilable
while the crate it scans is halfway through a change — which is the window in
which a boundary is most likely to be crossed.

### Arrival colours this phase predicted, and what was measured

The prediction was: FR-4.2-S3 green, everything else red. **Measured: six green,
fourteen red**, with each of the six falsified by hand — see "Arrival colours,
measured rather than predicted" above. The prediction was right about FR-4.2-S3
and about the direction of everything else; what it missed is that five other
scenarios are already satisfied by the tree this phase starts from, of which
FR-6.2-S1 is the one worth arguing about.

The goldens stay green until art reaches `write_layer`, and red from that moment
until T54 mints — which is the expected state and is not a failure of this
phase's tests.

### The re-shoot

Verbatim from `docs/technical/rendering.md`: probes, then oracle, then HUD
prediction, then a mint naming **only** the `terrain_goldens` and `hud_goldens`
binaries. **A bare `MYCRAFT_UPDATE_GOLDENS=1 cargo nextest run` reaches
`golden_mismatch` and corrupts the set permanently.** `SCENE_REVISION` is not
bumped; the commit message carries the why.

---

## Traps that bear on test authoring, in one place

- **Green is not evidence unless the test could have been red, for the right
  reason.** Each phase's named mutations are in `tasks.md`; run every one, revert by
  hand, and record the outcome **including the non-bites**.
- **A count cannot see shape.** Fixture construction is a constraint no assertion
  can enforce; the ones that matter in this spec are named per phase above.
- **A structural-invariant test needs a positive control**, as a separate test
  function. In this spec: FR-3.3-S8, FR-6.2-S4, FR-7.1-S2, and every `IndexError`
  arm.
- **Prefer an enumerated verdict to an absence assertion.** `SetVerdict`, the header
  scan, the gate stages and `EmittedFace.verdicts` are all total; assert the arm.
- **Red for a known reason hides red for an unknown one.** A test red for an
  expected reason is fixed before the phase closes, never annotated.
- **A green suite is no evidence about a lint.** Where a phase opens with an
  adaptation commit and nothing compiles, run
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` directly.
- **Two load-sensitive tests are live** — `reload_remesh_blocks_no_tick` (PRO-954)
  and one tracked as PRO-953, neither quarantined. Re-run before diagnosing, and
  never attribute a first sighting to the spec in flight.
