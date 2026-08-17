# Test map — Blocks are defined in Luau

Scenario → test file → test name. Test names are behavioural and carry no
scenario reference; this file is the only place the mapping lives.

Test command — the four binaries this phase adds:

```
cargo test -p mc-world --no-fail-fast --test luau_declarations \
  --test luau_declaration_fields --test luau_declaration_batches \
  --test luau_declaration_guard
```

They also run under `cargo nextest run -p mc-world`, which is what the gate
invokes.

Shared fixtures: `crates/mc-world/tests/luau_common/mod.rs`.

---

## Phase 1 — A declaration is evaluated, checked and registered, under the host's guard

**25 scenarios · 27 tests.**

### FR-1.1 — A chunk is evaluated once and the table it returns is the declaration

| Scenario | File | Test |
|---|---|---|
| FR-1.1-S1 | `crates/mc-world/tests/luau_declarations.rs` | `a_declaration_chunk_that_returns_a_table_registers_the_block_it_states` |
| FR-1.1-S2 | `crates/mc-world/tests/luau_declarations.rs` | `a_texture_key_the_chunk_assembled_registers_as_the_chunk_computed_it` |
| FR-1.1-S3 | `crates/mc-world/tests/luau_declarations.rs` | `a_chunk_returning_a_function_is_refused_naming_its_file_alone` |
| FR-1.1-S4 | `crates/mc-world/tests/luau_declarations.rs` | `a_chunk_returning_nothing_is_refused_naming_its_file_alone` |
| FR-1.1-S5 | `crates/mc-world/tests/luau_declarations.rs` | `a_source_asked_for_its_definitions_twice_yields_the_same_three_both_times` |

**FR-1.1-S1 carries a strengthener the scenario does not ask for, and it is
load-bearing.** Its declaration performs 200,000 loop edges and allocates 64 KiB
before returning. FR-4.1's four scenarios are all-unwanted, so a loader that gave
itself an absurdly small budget or memory cap would satisfy every one of them;
this is the test that reddens instead. Measured against the shipped limits it
evaluates in about 4 ms; measured against a budget of 100,000 ticks it is refused
for budget exhaustion, and against a 16 KiB per-entry cap it is refused for
allocation.

**FR-1.1-S5 is a weak instrument and must not read as evidence.** Under the
architecture's D2 there is no shared mutable host to borrow twice, so the
re-entrant borrow the scenario guards against is unexpressible rather than
avoided. It is kept because it costs nothing and would catch a later drift back
to interior mutability. It is not evidence this phase did work.

### FR-2.1 / FR-2.3 — The required fields, their kinds, and the namespacing rule

| Scenario | File | Test |
|---|---|---|
| FR-2.1-S1 | `crates/mc-world/tests/luau_declaration_fields.rs` | `a_declaration_stating_it_is_not_solid_registers_a_block_that_reports_non_solid` |
| FR-2.1-S2 | `crates/mc-world/tests/luau_declaration_fields.rs` | `a_declaration_naming_a_texture_other_than_itself_registers_both_as_stated` |
| FR-2.1-S3 | `crates/mc-world/tests/luau_declaration_fields.rs` | `a_declaration_that_states_no_solidity_is_refused_naming_the_file_the_block_and_the_field` |
| FR-2.1-S4 | `crates/mc-world/tests/luau_declaration_fields.rs` | `a_solidity_written_as_text_is_refused_naming_the_field_and_the_kind_of_value_it_holds` |
| FR-2.1-S5 | `crates/mc-world/tests/luau_declaration_fields.rs` | `a_declaration_that_states_no_texture_is_refused_naming_the_file_the_block_and_the_field` |
| FR-2.1-S6 | `crates/mc-world/tests/luau_declaration_fields.rs` | `a_declaration_that_states_no_name_is_refused_naming_the_field_and_no_block` |
| FR-2.1-S7 | `crates/mc-world/tests/luau_declaration_fields.rs` | `a_name_written_as_a_number_is_refused_naming_the_field_and_no_block` |
| FR-2.3-S1 | `crates/mc-world/tests/luau_declaration_fields.rs` | `a_name_carrying_no_namespace_is_refused_naming_the_field_and_the_rule_it_broke` |
| FR-2.3-S2 | `crates/mc-world/tests/luau_declaration_fields.rs` | `a_texture_key_carrying_two_separators_is_refused_naming_that_field` |

**FR-2.1-S2 reads the texture key back by two routes** — resolved from the block
and enumerated out of the whole registry — because the name/texture confusion it
exists for is the one that survived the spec's whole first draft.

### FR-3.1 / FR-3.2 — All-or-nothing, and a name declared twice

| Scenario | File | Test |
|---|---|---|
| FR-3.1-S1 | `crates/mc-world/tests/luau_declaration_batches.rs` | `a_root_whose_third_declaration_is_refused_leaves_the_registry_holding_what_it_held` |
| FR-3.1-S2 | `crates/mc-world/tests/luau_declaration_batches.rs` | `a_declarations_directory_holding_nothing_at_all_is_refused_naming_the_root` |
| FR-3.1-S3 | `crates/mc-world/tests/luau_declaration_batches.rs` | `a_root_whose_refused_declaration_was_repaired_registers_all_of_its_blocks` |
| FR-3.2-S1 | `crates/mc-world/tests/luau_declaration_batches.rs` | `two_declarations_of_one_name_are_refused_naming_both_files` |

**FR-3.1-S1 asserts which file was refused as well as what the registry kept.**
The registry it applies to is not empty — it already holds `fixture:granite` —
and the refusal has to name the broken declaration. Without both halves the
scenario is satisfied by any loader that failed for a reason of its own before
reaching the third file.

**FR-3.1-S3 is a weak instrument and must not read as evidence**, for the same
reason as FR-1.1-S5: D2's fresh host per call makes the second application
indistinguishable from the first, so what the scenario describes is true by
construction rather than defended against.

**FR-3.1-S2 cannot be reddened by any loader that reads a directory.** The
empty-source refusal comes from `registry.rs`, which this feature does not touch;
a loader yielding nothing for an empty directory satisfies it however it was
written. It went green the moment a skeleton existed. Kept as a control — it
reddens if a loader ever invents a definition for a directory holding none — and
recorded here rather than counted as work.

**FR-3.2-S1's fixture does not depend on sorting.** Registration order is the
filesystem's in this phase. The two files are named and written so that
creation order and file-name order agree, which is the only way the scenario's
"first" and "second" are well defined before the loader owns an order of its
own. Its positive control — two files declaring two distinct names, both
registering — is FR-1.2-S1 and belongs to phase 2.

### FR-4.1 / FR-4.2 — The host's guard, and metamethods that never run

| Scenario | File | Test |
|---|---|---|
| FR-4.1-S1 | `crates/mc-world/tests/luau_declaration_guard.rs` | `a_declaration_that_loops_without_returning_is_refused_naming_the_budget` |
| FR-4.1-S2 | `crates/mc-world/tests/luau_declaration_guard.rs` | `a_declaration_that_allocates_past_the_memory_cap_is_refused_naming_the_cap_and_not_the_budget` |
| FR-4.1-S3 | `crates/mc-world/tests/luau_declaration_guard.rs` | `a_declaration_that_reaches_for_the_clock_is_refused_naming_its_file` |
| FR-4.1-S4 | `crates/mc-world/tests/luau_declaration_guard.rs` | `a_declaration_that_assigns_a_global_is_refused_naming_its_file` |
| FR-4.2-S1 | `crates/mc-world/tests/luau_declaration_guard.rs` | `a_declaration_carrying_a_metatable_the_loader_never_reads_registers_its_own_fields` |
| FR-4.2-S2 | `crates/mc-world/tests/luau_declaration_guard.rs` | `a_solidity_supplied_only_through_a_metatable_is_refused_naming_that_field` |
| FR-4.2-S3 | `crates/mc-world/tests/luau_declaration_guard.rs` | `a_metatable_that_prints_on_every_access_prints_nothing_while_the_root_is_read` |

**FR-4.1-S1 and FR-4.1-S2 each assert both limits, not one.** One limit masks the
other: a memory bomb under a small budget dies of ticks and reports the wrong
limit while passing. S1 requires the budget named and the memory cap not; S2
requires the memory cap named and the budget not. S2 additionally requires the
refusal to state the shipped per-entry cap **by its own byte count**, read from
`HostLimits::default()` rather than written down — a loader that quietly gave
itself a smaller cap names a different figure and reddens.

**FR-4.2-S3 asserts the outcome of the load alongside the empty print record.** A
loader that never evaluated the chunk prints nothing too, so emptiness on its own
is satisfied for the wrong reason; the fixture states no `solid` of its own, so a
correct loader refuses it and registers nothing.

---

## Additional coverage

Tests that carry no scenario, each with what it catches.

| File | Test | What it catches |
|---|---|---|
| `crates/mc-world/tests/luau_declaration_guard.rs` | `a_declaration_that_prints_at_its_top_level_is_recorded_as_having_printed_it` | An implementation that records nothing in `printed()` at all. Without it FR-4.2-S3 is green forever over a source that never recorded anything — and phase 4's FR-4.3-S9 depends on the same control, since a truncation counter that never counted reads exactly like a chunk that printed nothing. |
| `crates/mc-world/tests/luau_declaration_guard.rs` | `a_source_read_twice_reports_what_the_second_read_printed_and_not_both` | A print record that accumulates across reads rather than reporting the last one (architecture Assumption 4). Hot reload re-reads a root as often as an author saves a file, so an accumulating record grows without bound and overstates what every later read printed. No scenario covers it. |

---

## Green on arrival, named rather than hidden

Run against a skeleton that walks `<root>/blocks/` and yields one fixed
`BlockDefinition` per file — the skeleton the tasks breakdown prescribes for this
phase — **26 of the 27 tests fail on an assertion**. The one that passes is
`a_declarations_directory_holding_nothing_at_all_is_refused_naming_the_root`
(FR-3.1-S2), for the reason recorded above.

---

## Phase 2 — Which files are declarations, and where a refusal points

**7 scenarios · 8 tests.**

Test command — the two binaries this phase adds:

```
cargo test -p mc-world --no-fail-fast --test luau_declaration_files \
  --test luau_declaration_locations
```

### FR-1.2 — Which entries are declarations, and in what order

| Scenario | File | Test |
|---|---|---|
| FR-1.2-S1 | `crates/mc-world/tests/luau_declaration_files.rs` | `declarations_register_in_file_name_order_rather_than_in_the_order_the_directory_lists_them` |
| FR-1.2-S2 | `crates/mc-world/tests/luau_declaration_files.rs` | `only_the_luau_files_directly_under_the_declarations_directory_are_declarations` |
| FR-1.2-S3 | `crates/mc-world/tests/luau_declaration_files.rs` | `a_content_root_with_no_declarations_directory_is_refused_naming_that_directory` |
| FR-1.2-S4 | `crates/mc-world/tests/luau_declaration_files.rs` | `a_directory_named_like_a_declaration_is_refused_naming_its_path` |

**FR-1.2-S1's fixture carries a third file the scenario does not mention, and
without it the scenario cannot fail here.** The spec's pair — `amber.luau`
declaring `example:zinc`, `zinc.luau` declaring `example:amber` — separates a
file-name sort from a *block-name* sort, and phase 1 passed that observation
forward as the gap it is: it does not separate a file-name sort from **no sort at
all**. Measured on this project's platform, NTFS hands `read_dir` its entries in
its own case-insensitive name order, so a fixture of lowercase names arrives
already sorted and a loader that never sorts registers them in exactly the
required order. `_cobalt.luau` is what breaks that: `_` sorts *after* every
letter under NTFS's collation and *before* every lowercase letter under the byte
ordering every plausible Rust sort uses. Measured: the listing hands back
`amber.luau`, `zinc.luau`, `_cobalt.luau`; the required order is `_cobalt.luau`,
`amber.luau`, `zinc.luau`. Every sort an implementation might reach for — over
`OsStr`, over the whole path, over a lowercased rendering — puts `_cobalt.luau`
first, so the fixture does not pin a collation; only the absence of a sort fails.

**FR-1.2-S2's `notes.txt` holds a declaration that would register.** Filling it
with prose would let a loader that reads every entry pass by failing to *parse*
it, which is a different rule than the one under test. As written, a loader that
does not filter by extension registers `example:notes` and reddens on the
enumerated list rather than on an accident of syntax.

**FR-1.2-S3 and FR-1.2-S4 were green the moment they were written**, and the
reason is not the one the tasks breakdown anticipated — see "Green on arrival"
below. Both are kept as regression guards over the filter phase 2 adds: a
refusal that came to name the *root* rather than `<root>/blocks` reddens S3, and
a filter that silently skipped a directory named `nested.luau` instead of
refusing it reddens S4.

### FR-3.3 — A chunk that will not compile or that raises is still located

| Scenario | File | Test |
|---|---|---|
| FR-3.3-S1 | `crates/mc-world/tests/luau_declaration_locations.rs` | `a_chunk_that_will_not_compile_is_located_by_its_path_and_not_by_the_name_the_host_was_given` |
| FR-3.3-S2 | `crates/mc-world/tests/luau_declaration_locations.rs` | `a_declaration_file_that_is_not_valid_luau_is_refused_naming_the_line_the_compiler_named` |
| FR-3.3-S3 | `crates/mc-world/tests/luau_declaration_locations.rs` | `a_chunk_that_raises_before_returning_is_refused_naming_the_error_it_raised` |

**FR-3.3-S1 compares a whole path, which every other suite here refuses to do,
and the exception is the requirement itself.** `amber.luau` and
`<root>/blocks/amber.luau` both contain `amber.luau`, so the `contains` check
that is right everywhere else is precisely the one that cannot separate the
origin from the chunk name the host was given. The expectation is built with
`Path::join` and rendered the way the loader renders a path, so no separator is
written down and the assertion stays portable.

**FR-3.3-S2's line number is derived from the fixture, never from a run.** The
test locates `local broken = =` inside the chunk it wrote and expects the
refusal to name that line, so editing the fixture moves the expectation with it.
Measured against the host directly: the fault carries `line = Some(3)` and a
cause of `Expected identifier when parsing expression, got '='`, with the line
stripped out of the text into a typed field — which is why a cause composed from
kind and text alone drops it.

---

## Additional coverage — phase 2

| File | Test | What it catches |
|---|---|---|
| `crates/mc-world/tests/luau_declaration_locations.rs` | `the_host_is_told_to_call_a_chunk_by_its_file_name_and_not_by_its_path` | An implementation that greens FR-3.3-S1 by handing the scripting host the **whole path** as the chunk name. That makes the origin and the host's label coincide, and FR-3.3-S1 can then never fail — the "agreement between two copies of one decision" shape D5 exists to prevent. Nothing in the scenarios can observe the label the host was given, so this does: the chunk catches its own error with `pcall` and declares the positioned message (`[string "amber.luau"]:1: probe`) as its `name`, and the refusal quotes it straight back. It is **green on arrival** — phase 1 already passes the file name alone — and is a control rather than evidence phase 2 did work. |

---

## Green on arrival — phase 2, and the diagnosis matters

`tasks.md` states that if any of T07's four scenarios is green when this phase
opens, phase 1 took `toml_source.rs:47–58` and the phase must be reopened.
**Three of the seven are green, and phase 1 did not take those lines.** Verified
by reading `crates/mc-world/src/content/luau_source.rs`: `declaration_files`
lists and collects with no `retain` on extension and no `sort_by`, and
`faulted_origin` lifts the origin out of the `ScriptFault`. The four scenarios
that the trap would have greened — FR-1.2-S1, FR-1.2-S2 and FR-3.3-S1 — are red.

| Scenario | Green because |
|---|---|
| FR-1.2-S3 | `declaration_files` already reports a `blocks/` it cannot list as `Unreadable { origin: <root>/blocks }`, which is the refusal the scenario asks for. Phase 1's `unreadable(&declarations, …)` satisfies it outright, and no faithful strengthening changes that. |
| FR-1.2-S4 | The tasks breakdown expected this one to be red, and its reasoning does not hold: it assumed the trap's `retain` on extension is what keeps a directory named `nested.luau` in the listing. **No filter at all keeps it too** — phase 1 hands the path to `fs::read_to_string`, which fails on a directory, and the refusal is `Unreadable` naming that path. The scenario asks for exactly that, so it is green under both the trap and the minimum, and its greenness is evidence about neither. |
| FR-3.3-S3 | The scenario asks the refusal to name "that file" and the error the chunk raised. Phase 1's origin is the bare chunk name `amber.luau`, which *contains* the file name and so satisfies the suite's established reading of "names the file", and the composed cause already carries the raised message. Only FR-3.3-S1 words the contrast that separates a chunk name from a path, which is why it is the one of the three that is red. |

None of the three is evidence phase 2 did work. Each reddens if the phase's own
work over-fires: a filter that skipped rather than refused (S4), a refusal
relocated to the root (S3), or a cause composed from `ScriptFault: Display`
instead of its typed fields (S3 of FR-3.3, through the doubled location
`refusals_state_a_cause_once.rs` counts).

---

## Phase 3 — The optional fields, the residue id, and the field nobody recognises

**11 scenarios · 11 tests**, plus four additional-coverage tests on the host
capability the phase adds.

Test command — the two `mc-world` binaries and the one `mc-script` binary this
phase adds:

```
cargo test -p mc-world --no-fail-fast --test luau_declaration_keys \
  --test luau_declaration_options
cargo test -p mc-script --no-fail-fast --test raw_field_names
```

Shared fixtures: `crates/mc-world/tests/luau_common/mod.rs`, extended with
`ASH`/`ASH_FILE`, the `Behaviour` reading and its documented defaults, the
`Blamed` verdict with `judged`/`blamed_by`, and `named_in_order`.

### Interface decided here, and it is binding on the implementation

`mc-script` gains, exactly as architecture D3 states it:

```rust
pub enum FieldNames {           // Debug + Clone + PartialEq + Eq
    Enumerated(Vec<String>),                // sorted, at most `most` entries
    MoreThanAllowed { allowed: usize },     // nothing copied out
}

impl ScriptHost {
    pub fn field_names(&self, table: &ScriptTable, most: NonZeroUsize) -> FieldNames;
}
```

`FieldNames` is compared by value in every test here, so it must derive `Debug`
and `Eq`. `field_names` takes `&self`, which is what lets the loader enumerate a
declaration it is already holding a handle to without a second mutable borrow of
the host.

### FR-2.4 / FR-4.2 — Which fields a declaration carries

| Scenario | File | Test |
|---|---|---|
| FR-2.4-S1 | `crates/mc-world/tests/luau_declaration_keys.rs` | `a_field_the_loader_does_not_recognise_is_refused_beside_the_ones_it_does` |
| FR-2.4-S2 | `crates/mc-world/tests/luau_declaration_keys.rs` | `a_declaration_stating_all_six_recognised_fields_and_nothing_else_registers` |
| FR-2.4-S3 | `crates/mc-world/tests/luau_declaration_keys.rs` | `two_unrecognised_fields_are_named_in_the_same_order_every_time` |
| FR-4.2-S4 | `crates/mc-world/tests/luau_declaration_keys.rs` | `an_iterator_that_hides_a_field_does_not_stop_it_being_refused` |
| FR-4.2-S5 | `crates/mc-world/tests/luau_declaration_keys.rs` | `an_iterator_that_invents_a_field_does_not_stop_a_declaration_registering` |

**FR-4.2-S4 and FR-4.2-S5 assert through the loader, not against a host built in
the test.** `field_names` being raw proves nothing about a loader that does not
call it — that is the "policy is not wiring" failure `testing.md` §2 names, and
the same reason FR-4.1's four scenarios assert through the loader. The
host-level tests below are about properties no loader scenario can reach.

**FR-2.4-S1 asserts the recognised six as a set, not in an order.** The spec asks
the refusal to name "the fields it does recognise" and says nothing about their
order; FR-2.4-S3 is what holds *any* list settled, and it does so over the whole
cause, which covers the recognised half for free.

**FR-2.4-S3 carries a stronger observable than the scenario asks for, and the
scenario's own half cannot fail today.** Measured: a fixed set of keys in a fresh
script state comes back in the same order on every read, so "named in the same
order on every run" is satisfied by an implementation that passes the backend's
order straight through — eight repeated reads cannot tell those apart. What can
is *which* order. The fixture states `slid` before `replacable`; the state hands
them back in that same order (measured: the whole five-key declaration comes back
`solid, name, slid, replacable, texture`); so the required lexicographic
`replacable, slid` is the one answer neither the writing nor the backend
produces. Both halves are asserted in one comparison.

**FR-2.4-S1, FR-2.2-S3, FR-2.2-S4, FR-2.3-S3 and FR-4.2-S4 compare a total
verdict rather than propagating "this root was not refused".** `Blamed` has an
arm for a root that registered, so a loader that accepts what it should refuse
fails on the comparison instead of ending the test before its assertion ran. The
first draft of these tests used the suite's existing `fault_from`, which returns
an error in that case — three of them then never reached the expectation they
exist to state.

### FR-2.2 / FR-2.3 — The optional fields and the residue id

| Scenario | File | Test |
|---|---|---|
| FR-2.2-S1 | `crates/mc-world/tests/luau_declaration_options.rs` | `a_declaration_stating_none_of_the_optional_fields_gets_the_documented_defaults` |
| FR-2.2-S2 | `crates/mc-world/tests/luau_declaration_options.rs` | `an_unbreakable_block_may_still_name_what_it_would_have_left_behind` |
| FR-2.2-S3 | `crates/mc-world/tests/luau_declaration_options.rs` | `a_replaceability_written_as_a_number_is_refused_rather_than_defaulted` |
| FR-2.2-S4 | `crates/mc-world/tests/luau_declaration_options.rs` | `a_residue_written_as_a_number_is_refused_rather_than_read_as_no_residue` |
| FR-2.3-S3 | `crates/mc-world/tests/luau_declaration_options.rs` | `a_residue_carrying_no_namespace_is_refused_naming_the_rule_it_broke` |
| FR-2.3-S4 | `crates/mc-world/tests/luau_declaration_options.rs` | `a_residue_nothing_in_the_root_declares_still_registers` |

**FR-2.3-S4 asserts the residue it retained and not only that the block
registered.** The scenario's own wording — "SHALL register the block" — is
satisfied by a loader that drops `breaks_into` entirely, which is the *other* way
to make a residue unresolvable and the state the tree is in when the phase opens.
Keeping the scenario's assertion and adding the retained name is what makes it
falsifiable.

**FR-2.2-S2's residue is declared in the same root; FR-2.3-S4's is not, and that
is deliberate.** S2's subject is that `breakable = false` and a residue are
independent; S4's is that the loader does not resolve a residue at all. With both
fixtures naming an undeclared `example:ash`, one eager-resolution defect would
redden both and read as two. Declared there, undeclared here, each fails for its
own reason.

**FR-2.2-S1 and FR-2.2-S2 each compare all three optional fields at once**, so a
loader that resolved two of them the way the pages promise is not mistaken for
one that resolved all three that way.

---

## Additional coverage — phase 3

Tests that carry no scenario, each with what it catches. All four are on
`ScriptHost::field_names`, whose four binding properties (raw, bounded, total
over key types, sorted) are only half reachable from the loader.

| File | Test | What it catches |
|---|---|---|
| `crates/mc-script/tests/raw_field_names.rs` | `every_key_a_table_holds_comes_back_lexicographically_ordered` | An enumeration that returns the script state's own key order. No loader scenario can see this: FR-2.4-S3 asserts stability, and the state's order *is* stable within a process. The fixture is built so that state order, written order and lexicographic order are three different orders — measured, a declaration written `slid, replacable, name, texture, solid` comes back `solid, name, slid, replacable, texture`. An unstable list is what makes `documented_refusals.rs` intermittently red, which this project's standards rank below plainly red. |
| `crates/mc-script/tests/raw_field_names.rs` | `a_key_that_is_not_a_string_is_rendered_rather_than_passed_over` | An enumeration that skips a key it cannot read as text, which would make FR-2.4's "an unrecognised field is refused" a promise holding only for the key types somebody thought of — and the field it loses is exactly the one nobody meant to write. Architecture Assumption 1; no scenario reaches it. The expected rendering is **derived**: the chunk prints the same three values and the expectation is assembled from `printed()`, so the test states the contract ("the same rendering `print` uses") rather than a transcript of a run. |
| `crates/mc-script/tests/raw_field_names.rs` | `a_table_holding_more_keys_than_the_caller_allowed_copies_none_of_them_out` | Both sides of the parameterised bound in one comparison. A bound stated only from the refusing side leaves `>` and `>=` indistinguishable; one stated only from the accepting side is not a bound. It also pins that the over-bound answer *reports* rather than truncating — a list silently cut to the allowance has already made the allocation the bound exists to refuse. The loader's own 64 is phase 4's (FR-4.3-S5); this is the host parameter phase 3 delivers. |
| `crates/mc-script/tests/raw_field_names.rs` | `a_length_metamethod_reporting_nothing_does_not_hide_what_a_table_holds` | An enumeration sized with a call that consults `__len`. **This is the reachable half of rawness and it was measured:** `mlua`'s `Table::pairs` is already raw against `__iter` and `__pairs` on this toolchain, while `Table::len` honours `__len` and answered `0` for a table whose raw length is 3. A host that sized itself that way reports every declaration as carrying no fields — refusing nothing, losing every typo, and looking exactly like a table that really was empty. No scenario names `__len`. |

---

## Green on arrival — phase 3, with the diagnosis for each

**Three of the eleven are green when the phase opens.** Two were named in advance
and the third is the same shape as one of them. None is evidence phase 3 did
work, and each reddens if this phase's own work over-fires.

| Scenario | Green because | What it would catch |
|---|---|---|
| FR-2.4-S2 | phase 2's loader reads the six fields it knows and never asks what else the table holds, so a declaration carrying all six and nothing else registers | an unrecognised-field check that over-fires and takes the whole-contract declaration down with the misspelled one |
| FR-4.2-S5 | the same reason, from the other side: with no unrecognised-field check there is nothing for a metatable's `__iter` to mislead. Not in the roster the tasks breakdown names; its diagnosis is exactly FR-2.4-S2's | an enumeration that believes `__iter` — it would see one invented field, recognise none of it, and refuse a declaration whose own three fields are perfectly stated |
| FR-2.2-S1 | phase 1 hardcodes the three optional fields at their documented defaults without reading them (recorded in `tasks.md`, "Phase 1 — recorded at completion") | a default resolved differently once the fields are actually read |

The other eight fail on an assertion, not on a fixture guard or a compile error:

```
luau_declaration_keys:    2 passed; 3 failed
luau_declaration_options: 1 passed; 5 failed
```

**`raw_field_names.rs` does not compile, and that is the legitimate RED for it.**
`FieldNames` and `ScriptHost::field_names` genuinely do not exist yet, which is
the one case `testing.md` §2 allows a compile error to stand in for an assertion
— the type existing *is* what T09 delivers. Every behavioural scenario of this
phase reaches an assertion.

---

## Phase 4 — Every content-supplied quantity has a bound

**9 scenarios · 9 tests**, plus four additional-coverage tests.

Test command — the two `mc-world` binaries this phase adds, and the one
`mc-script` binary:

```
cargo test -p mc-world --no-fail-fast --test luau_root_bounds \
  --test luau_declaration_bounds
cargo test -p mc-script --no-fail-fast --test retained_print_output \
  --test shipped_defaults
```

Shared fixtures: `crates/mc-world/tests/luau_common/mod.rs`, extended with
`refusal_and_cause`, `mentioning` and `blaming_the_declaration`.

### Interface decided here, and it is binding on the implementation

`mc-script` gains, as architecture D4 and its Interfaces section state it:

```rust
pub struct HostLimits {
    // …existing six…
    /// Bytes of script output one host retains. Shipped default: 256 * 1024.
    pub retained_print_bytes: NonZeroUsize,
}

impl ScriptHost {
    pub fn dropped_print_lines(&self) -> u64;
}
```

Four decisions the architecture leaves open, settled here because a test cannot
be written without them:

- **The shipped default is `256 * 1024` bytes.** Nothing in the spec or the
  architecture states it, and `shipped_defaults.rs` refuses to build until a
  seventh limit is given one — by its own design, so that a limit added later
  cannot be quietly outside what anything checks. The figure is the per-entry
  memory cap, on purpose: the host-side copy of what content printed cannot then
  outgrow the allowance the chunk had to build it in. Written as its own literal
  rather than computed from the cap, which would make it follow the cap wherever
  it went and bound nothing.
- **The bound binds where the recording happens, not where the host collects
  it.** A chunk that never returns has already grown the buffer, so a bound
  applied when `evaluate` drains the state's print sink arrives after the
  allocation it exists to refuse.
- **It bounds one host's whole life, not one entry.** The loader evaluates every
  declaration in a root through a single host; an allowance that started again
  on each collection would bound one declaration and nothing across four
  thousand, which is the arithmetic that made the bound necessary.
- **A line is retained whole or not at all**, and reaching the bound stops
  recording rather than dropping the oldest.

The loader's four bounds are `4_096` declarations, `256 * 1024` bytes per file,
`256` characters of declared text and `64` field names, and **each refusal
states its quantities as plain decimal figures** — the tests look for `4097`
and `4096`, `307200` and `262144`, `257` and `256`. That is the house style
`HostError::unusable_memory` already writes in, and a refusal is only useful to
a reader who can tell slightly over from far over.

### FR-4.3 — The declaration count and the file size

| Scenario | File | Test |
|---|---|---|
| FR-4.3-S1 | `crates/mc-world/tests/luau_root_bounds.rs` | `a_root_holding_more_declarations_than_allowed_is_refused_on_the_count_and_not_on_a_broken_file` |
| FR-4.3-S2 | `crates/mc-world/tests/luau_root_bounds.rs` | `a_declaration_file_past_the_size_bound_is_refused_on_its_size_and_not_on_what_it_holds` |
| FR-4.3-S6 | `crates/mc-world/tests/luau_root_bounds.rs` | `a_root_holding_exactly_as_many_declarations_as_are_allowed_registers_every_one` |
| FR-4.3-S7 | `crates/mc-world/tests/luau_root_bounds.rs` | `a_declaration_file_of_exactly_the_size_allowed_registers_the_block_it_declares` |

**FR-4.3-S2's forbidden half is derived, not written down.** The same broken
declaration is loaded twice — once in a file small enough to be read, once
padded past the bound — and the *first* run's whole cause is the needle the
second run's refusal must not contain. Nothing in the test spells a word of the
compiler's diagnostic, so a backend that rewords its own messages moves both
halves together. The small root's own refusal is asserted beside it, because a
broken declaration that turned out not to be refusable would leave an empty
needle and a test that passes over any implementation whatever.

**FR-4.3-S6 and FR-4.3-S7 assert the fixture's measured size in the same
comparison as the outcome.** A padding helper producing 262,143 bytes, or a
builder writing 4,095 files, would make an accepting test pass for a reason
nothing else here reports — and the accepting side is precisely what separates
`>` from `>=`.

**FR-4.3-S6 is also this phase's answer to the architecture's first-listed
risk** — 4,096 chunks accumulating in one script state against the 16 MiB
backstop. It registers all 4,096 and the whole run costs about 6.8 s, so none of
the named fallbacks (dropping handles earlier, forcing periodic collection, a
loader-specific `HostLimits`) is needed. Measured separately before the phase
opened: peak collected memory 2,105,960 bytes against the 16 MiB backstop, about
12.5%.

### FR-4.3 — What one declaration may state

| Scenario | File | Test |
|---|---|---|
| FR-4.3-S3 | `crates/mc-world/tests/luau_declaration_bounds.rs` | `a_name_of_one_character_too_many_is_refused_naming_that_field_and_the_length_it_allows` |
| FR-4.3-S4 | `crates/mc-world/tests/luau_declaration_bounds.rs` | `a_texture_key_of_one_character_too_many_is_refused_naming_that_field_and_the_length_it_allows` |
| FR-4.3-S5 | `crates/mc-world/tests/luau_declaration_bounds.rs` | `a_declaration_carrying_more_fields_than_are_read_is_refused_naming_the_bound_and_no_one_field` |
| FR-4.3-S8 | `crates/mc-world/tests/luau_declaration_bounds.rs` | `a_name_of_exactly_the_characters_allowed_registers_the_block_under_that_name` |

**FR-4.3-S8's name is accented, and that is the only place in this feature where
the unit of the bound is visible.** D8 measures declared text in
`chars().count()`. A 256-character ASCII fixture is 256 bytes, so it cannot tell
a loader counting characters from one counting bytes — and both refusals next
door are ASCII on purpose, so that they are about the bound rather than about
the unit. The accepting name is exactly 256 characters and 380 bytes, so a
loader measuring bytes refuses a declaration the documentation promises to
accept, and reddens here.

**FR-4.3-S5's filler fields are unrecognised on purpose.** D7 runs the
enumeration before any field is checked, so a loader that asked *which* fields
it knows before *how many* there are refuses one filler key by name — a
different refusal than the one the bound owes. The expected refusal names the
file and the block and **no field**, which settles architecture Assumption 2 in
favour of naming the block: the loader reads `name` for attribution before it
enumerates, so it has one to quote back even where it has no field to blame.

**FR-4.3-S5 asserts the bound alone and not the observed quantity, and that is
a conflict with D8.** D8 says every refusal states the observed quantity and the
bound. The field-count refusal structurally cannot: D3's `MoreThanAllowed`
carries only the allowance, because the enumeration stops one key past the bound
rather than counting a table it is refusing to allocate. The bound is what the
refusal owes; the count is what the design deliberately never learns.

### FR-4.3-S9 — The bound on retained script output

| Scenario | File | Test |
|---|---|---|
| FR-4.3-S9 | `crates/mc-script/tests/retained_print_output.rs` | `output_past_what_a_host_retains_keeps_the_earliest_lines_and_reports_that_it_kept_no_more` |

**It runs two hosts and compares them, because the scenario is about a
distinction.** One host's chunk printed far too much, the other's printed
nothing. Asserting the truncated record alone is satisfied by a host that never
recorded anything at all — which is exactly the absence-that-reads-as-agreement
the bound is being made observable to avoid. Its positive control on the other
side is phase 1's `a_declaration_that_prints_at_its_top_level_is_recorded_as_
having_printed_it`, re-run and still green.

**Every line the fixture prints is distinct, and the bound is deliberately not a
whole number of lines.** Distinct lines are what separate keeping the earliest
from keeping the latest; the four spare bytes are what separate keeping whole
lines from keeping the front of one nobody printed. Only
`retained_print_bytes` is configured — the budget and the memory cap stay
shipped, because a fixture that shrank the budget to make printing cheap would
find the chunk stopped for exhausting its budget and would report a bound that
was never reached.

---

## Additional coverage — phase 4

| File | Test | What it catches |
|---|---|---|
| `crates/mc-world/tests/luau_root_bounds.rs` | `a_root_over_the_count_is_refused_before_any_entry_is_checked_to_be_a_file` | The count check landing after the file-type loop. FR-4.3-S1 proves the count beats *evaluation*; nothing in the spec reaches the step before it, and phase 2 passed the gap forward explicitly — `declaration_files` now runs collect, sort, then `confirmed_file` per entry, so a count checked after that loop costs 4,097 `fs::metadata` calls before the refusal meant to pre-empt them. The fixture's intruder is a directory named `_nested.luau`, which sorts first under every ordering, so a loader checking in the wrong order meets it immediately and refuses it by name. |
| `crates/mc-world/tests/luau_declaration_bounds.rs` | `a_residue_of_one_character_too_many_is_refused_on_the_same_bound_as_a_name` | A declared-text bound applied to two of the three strings a definition retains. The spec words FR-4.3-S3 and S4 over `name` and `texture` while the bound's own rationale counts *three* strings apiece, and `breaks_into` is the easiest to overlook — it is optional, and it is the one id the loader deliberately never resolves. |
| `crates/mc-script/tests/retained_print_output.rs` | `output_that_exactly_fills_what_a_host_retains_is_kept_whole_and_nothing_is_reported_dropped` | The accepting side of the retained-output bound, which no scenario states — the other three bounds have S6/S7/S8 and this one had nothing. It also rejects a counter that reports a line dropped whenever output merely *reaches* the allowance, which would make every full record read as a truncated one. Owed by `tasks.md`, T16. |
| `crates/mc-script/tests/retained_print_output.rs` | `a_second_chunk_printing_after_the_bound_was_reached_adds_nothing_and_is_reported_dropped` | An allowance that starts again whenever the host collects what a chunk printed. That bounds one declaration and nothing at all across a content root of four thousand, which is the arithmetic D4 was written from. No scenario reaches it, and it is the property the loader actually depends on. |

`crates/mc-script/tests/shipped_defaults.rs` gains the seventh limit in its
`HostLimits` record — the file spells every field with no `..` precisely so that
a limit added later cannot arrive unasserted, and honouring that meant deciding
the shipped value here. Not a new test: an existing one widened, and it is the
only thing constraining what a server nobody configured will retain.

---

## Green on arrival — phase 4, with the diagnosis for each

**Three of the nine, and all three are the ones `tasks.md` predicted.** None is
evidence phase 4 did work; each reddens if this phase's own work over-fires.

| Scenario | Green because | What it would catch |
|---|---|---|
| FR-4.3-S6 | 4,096 files already register against a loader that bounds nothing — **and the memory risk did not trip**, which is what T13 existed to find out. Recorded as measured rather than inferred | a count bound spelled `>=`, and a loader whose script state cannot survive a full content root |
| FR-4.3-S7 | a 256 KiB file already registers against a loader that never looks at a file's size | a size bound spelled `>=`, and a padding path that mis-measures the boundary |
| FR-4.3-S8 | a 256-character name already registers against a loader that never measures declared text | a text bound spelled `>=`, **and a bound measured in bytes** — the accented fixture is what makes that second failure reachable at all |

The additional-coverage test
`output_that_exactly_fills_what_a_host_retains_is_kept_whole_and_nothing_is_
reported_dropped` is green on the same terms, the moment the `HostLimits` field
exists.

The other six fail on an assertion, not on a fixture guard:

```
luau_root_bounds:          2 passed; 3 failed
luau_declaration_bounds:   1 passed; 4 failed
```

**`retained_print_output.rs` and `shipped_defaults.rs` do not compile, and that
is the legitimate RED for them** — the same case phase 3 recorded for
`raw_field_names.rs`. `HostLimits::retained_print_bytes` and
`ScriptHost::dropped_print_lines` genuinely do not exist, which is the one case
`testing.md` §2 allows a compile error to stand in for an assertion, the field
existing *being* what T16 delivers. **The assertion-level RED was measured
anyway**, against a hand-applied stub adding the field at its documented default
and a `dropped_print_lines` returning zero: the truncating scenario reported all
64 lines kept and 0 dropped against 20 and 44, and the host-life test reported
40 kept and 0 dropped against 20 and 20. The stub was reverted by hand and
`git diff --exit-code` over `crates/*/src/` was clean before the commit.

**These fixtures are slow, and the cost is stated rather than absorbed.** Under
`cargo nextest` the three 4,096-entry roots cost about 6.1 s, 6.8 s and 6.8 s
each — writing four thousand files on NTFS, then evaluating four thousand chunks
through one script state. That is well past `testing.md` §3's one-second
guidance for an integration test and there is no faster fixture that still
answers the question: the bound is about how many files a directory holds, and a
directory holding fewer does not test it. `mc-world`'s whole suite runs in
15.7 s.

---

## Phase 5 — The base game ships in Luau, the simulation loads it, TOML is retired, the pages are true

**15 scenarios · 15 tests, plus 8 of additional coverage.**

Test command — the five binaries this phase adds or moves:

```
cargo nextest run --no-fail-fast \
  -E 'binary(shipped_declarations_and_an_older_save) + binary(shipped_blocks_are_declared_in_luau) \
      + binary(one_content_path_to_the_registry) + binary(client_names_no_content_door) \
      + binary(walkthrough_declaration_loads) + binary(documented_refusals)'
```

### Every reading of the shipped four also names the file that declared it

The four blocks, their order, their solidity, the one that may be built through
and the one a player holds are all facts the game already had. A test asserting
only those would have been green before this feature and green after it, and
would have said nothing about either. So **each scenario's own assertion is kept
exactly as worded and the declaring file is asserted beside it, in the same
comparison** — `testing.md` §1's "a stronger observable alongside a scenario's
own". A definition carries the origin it was read from, the registry hands it
back, and a block declared by `dirt.toml` is not the block this feature is about.

The file name alone is compared and never the whole path: an origin renders with
the platform's own separators, and an expectation spelling one out would pass on
one operating system and fail on the other for a reason that has nothing to do
with content.

### FR-1.3 — The four base declarations, and the save that must still load

| Scenario | File | Test |
|---|---|---|
| FR-1.3-S1 | `crates/mc-client/tests/shipped_blocks_are_declared_in_luau.rs` | `the_shipped_content_declares_four_blocks_in_luau_with_water_alone_soft_and_replaceable` |
| FR-1.3-S2 | `crates/mc-client/tests/shipped_blocks_are_declared_in_luau.rs` | `a_launch_puts_the_first_solid_block_the_declarations_name_into_the_players_hand` |
| FR-1.3-S3 | `crates/mc-client/tests/shipped_blocks_are_declared_in_luau.rs` | `a_content_root_declaring_nothing_solid_refuses_to_start_rather_than_opening_a_window` |
| FR-1.3-S4 | `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | `a_world_saved_before_the_declarations_changed_language_reports_no_block_as_changed` |

**S1, S2 and S3 assert through `prepare_launch`**, which the spec names as the
first of the two call sites. All three answer with one of four enumerated
outcomes — what registered, what is held, refused for having nothing to place, or
refused otherwise — so a launch that refused and a launch that held the wrong
block are the same failed comparison rather than one failure and one propagated
error.

**FR-1.3-S4 is the only fixed oracle in the spec over a whole resolved
definition**, and what makes it one is that the save it compares against was
written before the declarations changed language.
`crates/mc-world/tests/fixtures/world_saved_against_the_toml_declarations.mcw` is
committed, was written from `content/base/` while its four blocks were still
TOML, and holds all four of them. **It is not regenerated**: a save produced from
the declarations under test agrees with them by construction and could not fail.
The comparison is the whole `RegistryVerdict` — missing, changed *and*
retextured all empty — rather than "nothing changed", because a texture key
mapped to the wrong place shows up as *retextured*, which a load would accept
silently.

**FR-1.3-S3 is green on arrival**, as the breakdown predicts:
`PreparationError::NothingToPlace` already implements it. Its fixture strips a
copy of the shipped root down to the one non-solid declaration **by file stem
rather than by extension**, so it is the same fixture before and after the swap —
a fixture naming an extension would have had to be rewritten at exactly the
moment nobody would look at it.

### FR-5.1 — One path from a content root to the registry

| Scenario | File | Test |
|---|---|---|
| FR-5.1-S1 | `crates/mc-client/tests/one_content_path_to_the_registry.rs` | `a_root_holding_only_a_declaration_in_the_retired_format_declares_no_block_at_all` |
| FR-5.1-S2 | `crates/mc-client/tests/one_content_path_to_the_registry.rs` | `a_root_declaring_in_both_formats_registers_the_luau_declaration_and_never_the_other` |
| FR-5.1-S3 | `crates/mc-client/tests/one_content_path_to_the_registry.rs` | `the_scene_the_golden_frames_are_shot_through_registers_the_blocks_the_shipped_declarations_state` |

**All three assert through the client's own preparation and not through the
reader**, and that is the whole of why they can fail. Asked of
`LuauFileDefinitionSource`, a directory of retired declarations is refused today
and was refused the day that reader was written — it never knew the format, so
S1 and S2 would be green on arrival and inert. What is worth asserting is that
*the client* no longer has a second way to read one, which is a fact about the
path rather than about any reader on it.

**S3 asserts through `prepare_scene` and nothing else does.** It is a second
entry point onto a path `prepare_launch`'s tests already cover, and a second
entry point is untested until something asserts through it rather than through
the caller that was already covered.

**The verdict has three answers rather than "it refused".** A root the client
reads through the retired reader does not fail quietly: it registers a block and
then fails further down over a world it could not generate. Measured before
implementation, FR-5.1-S1 answers today with `refused: the replay world could not
be generated` — which a comparison asking only whether something was refused
would have accepted as the refusal the scenario is about.

### FR-6.1 — The documentation-agreement guard, with its fixture moved

| Scenario | File | Test |
|---|---|---|
| FR-6.1-S1 | `crates/mc-client/tests/documented_refusals.rs` | `every_refusal_the_modding_pages_quote_is_a_refusal_the_client_prints` |
| FR-6.1-S2 | `crates/mc-client/tests/documented_refusals.rs` | `a_quoted_refusal_altered_to_text_the_client_never_prints_is_reported_with_both_sides` |
| FR-6.1-S3 | `crates/mc-client/tests/documented_refusals.rs` | `a_page_that_quotes_no_refusal_at_all_is_reported_as_quoting_none` |

The guard already existed. **Its mechanism, its three enumerated verdicts, its
three controls and both normalisations are unchanged**; what moved is
`BLOCK_FILE` (`amber.toml` → `amber.luau`) and
`CARRYING_AN_UNRECOGNISED_FIELD`, which is now a chunk that returns a table.
The header's note that a parser version bump reddens this guard is **narrowed to
the HUD half rather than deleted** — the HUD declaration is still TOML and the
note is still true of it, while the block half now compares against a refusal
MyCraft writes itself.

**All four tests in this binary are red through the suite's own fixture guard
rather than through the verdict comparison**, and that is the honest shape of
this phase's RED: a Luau declaration placed in a root the client still reads with
the retired reader is not read at all, so the run is not refused and there is no
printed refusal to compare a page against. `as_read_from_a_game_directory`
refuses to compare in that state by design. The comparison becomes an assertion
the moment T18 lands.

### FR-6.2 — The walkthrough's declaration is a declaration that loads

| Scenario | File | Test |
|---|---|---|
| FR-6.2-S1 | `crates/mc-client/tests/walkthrough_declaration_loads.rs` | `the_declaration_the_first_block_walkthrough_shows_registers_the_block_it_states` |
| FR-6.2-S2 | `crates/mc-client/tests/walkthrough_declaration_loads.rs` | `a_walkthrough_quoting_no_declaration_at_all_is_reported_as_quoting_none` |

**Two constraints on `docs/modding/README.md` that the guard imposes and nothing
else states.** First, **every** fenced block tagged `luau` on that page is a
whole declaration that loads — a fragment of one is shown without that tag,
because a page that grows a second example is a page somebody will type the
second example out of. Second, the quoted declaration **states its `name` as a
literal**: what registers is compared against the name read out of the page's own
text, which is an oracle this side of the loader, and a name a chunk assembled
would have nothing to be held to. A first-block walkthrough that computed its
block's name would be teaching the wrong lesson anyway.

**FR-6.2-S2 is green from the moment it is written**, because the verdict it
asserts comes from guard code the test author owns whole. It is not evidence the
phase did work; it is the vacuity control, and its value is that it reddens if
the recogniser is ever widened to treat prose about a declaration as a quotation
of one.

### FR-7.1 — The client's own sources name no content door

| Scenario | File | Test |
|---|---|---|
| FR-7.1-S1 | `crates/mc-client/tests/client_names_no_content_door.rs` | `the_clients_own_sources_name_none_of_the_doors_content_is_read_through` |
| FR-7.1-S2 | `crates/mc-client/tests/client_names_no_content_door.rs` | `the_same_scan_reports_a_client_source_that_names_a_door_and_says_which_door_it_named` |
| FR-7.1-S3 | `crates/mc-client/tests/client_names_no_content_door.rs` | `a_scan_that_read_no_client_source_says_so_rather_than_reporting_a_clean_client` |

Follows `crates/mc-client/tests/seam_boundaries.rs`: whole-path exemption
comparison (kept in that form even though **nothing is exempt**, because an
exemption written as a bare name is one rename away from excusing the file it was
watching), doc comments stripped, sibling `*_test.rs` skipped, a `tempfile`
fixture as the positive control.

**The verdict is enumerated and not `hits.is_empty()`**, which is what FR-7.1-S3
is: a scan that read nothing and a clean client must never compare equal.
**S2's expected report is derived from the needle list** rather than written out,
so a needle added without a fixture committing it fails here rather than standing
unwatched forever.

**S2 and S3 are green from the moment they are written**, for the same reason
FR-6.2-S2 is: both are controls over guard code the test author owns whole, and
no implementation can redden them. Only S1 is red on arrival, and it is what the
construction move greens.

**Known residual, recorded in the guard's own doc comment:** a *second* door — a
new public registration call — bypasses every needle here, and no text scan
closes it. The instrument that would is the dependency-closure guard, which
cannot pass while one binary hosts both halves and is therefore the
composition-root spec's exit criterion.

## Additional coverage — phase 5

| File | Test | What it catches |
|---|---|---|
| `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | `the_committed_save_really_does_need_all_four_of_the_blocks_the_base_game_ships` | FR-1.3-S4 compares against an **empty** verdict, which is also what a save needing nothing produces. A fixture whose name table was lost or truncated would read as agreement forever. |
| `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | `the_same_comparison_reports_a_block_whose_declaration_the_content_no_longer_holds` | The other direction: a comparison that came back empty whatever it was handed. This is the reading that says `resolve` over this save can report something. |
| `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | `the_shipped_root_declares_its_blocks_in_luau_and_its_hud_in_the_format_that_did_not_change` | A `.toml` block declaration left behind in `content/base/blocks/` after the retirement — a file no reader opens again and every reader of the directory still sees — and, in the same comparison, a swap that took the HUD declarations with it. The single `DECLARATION_EXTENSION` in `tests/support/content.rs` served both, so this is the reading that would catch it being split wrongly. |
| `crates/mc-client/tests/client_names_no_content_door.rs` | `a_door_named_in_a_sibling_unit_test_file_is_passed_over_and_the_module_beside_it_is_not` | The file filter, in both directions: a filter that skipped too much would leave the positive control green while scanning almost nothing. |
| `crates/mc-client/tests/client_names_no_content_door.rs` | `prose_about_a_door_in_a_doc_comment_is_not_a_use_of_it` | The doc-comment strip. `startup.rs` and `launch.rs` both discuss `content_root` and `HudLayout::load` in prose today, so without it the guard is unsatisfiable for a reason that has nothing to do with the seam — which is the state in which somebody deletes the guard. |
| `crates/mc-client/tests/walkthrough_declaration_loads.rs` | `a_walkthrough_quoting_a_declaration_that_loads_agrees_and_its_other_blocks_are_passed_over` | That the agreement verdict can come out of a real load at all. Without it nothing has watched it happen, and a recogniser reporting a disagreement for every block would leave both scenario tests green and the real page red — read as a documentation problem rather than as a broken guard. It carries a second fenced block that is not a declaration, so a recogniser treating every fenced block as one is caught here. |
| `crates/mc-client/tests/walkthrough_declaration_loads.rs` | `a_walkthrough_quoting_a_declaration_the_loader_refuses_is_reported_with_both_sides` | That a page whose example does *not* load is reported with both the quotation and the reason. The example is broken by leaving a field out rather than by mistyping the language, because a declaration that reads perfectly well to somebody skimming is the only kind that survives review. |
| `crates/mc-client/tests/documented_refusals.rs` | `pages_quoting_the_printed_refusals_verbatim_agree_and_their_other_blocks_are_passed_over` | Pre-existing, and recorded here because it belongs to FR-6.1's set and no scenario words it: that a page quoting the printed text verbatim is *accepted*, over a set of blocks that is not empty. |
| `crates/mc-client/tests/documented_refusals.rs` | *(no test of its own — a third fixture inside `printed_refusals`)* | The **rendering** of the declared-text refusal, which no assertion in the tree could see. `luau_declaration_bounds.rs` asks whether both quantities are *mentioned* in the cause, and a substring check is equally true of a well-formed message and of one carrying eighteen stray spaces from a line continuation that never landed — which is what shipped. Adding the refusal to this guard's run and quoting it on `blocks-items.md` puts it under a comparison that is line for line, with no new assertion and no message literal duplicated into a test. The page is the oracle. |
| `crates/mc-world/tests/luau_declaration_guard.rs` | `a_declaration_that_printed_past_what_is_kept_is_recorded_differently_from_one_that_printed_little` | A record the host truncated reading identically, at the loader's own boundary, to one of a chunk that printed exactly that much — the absence-that-reads-as-agreement D4 closes at the host and a bare list of lines re-opens one level up. No scenario words it; it is a gap the lead ruled in. Two roots side by side, because a record asserted on its own is satisfied by a loader that reports truncation always *and* by one that reports it never. Measured against a skeleton that always answers whole: the noisy root's record accounts for **1,024 of the 2,048 lines it printed and says nothing about the rest**. |

## Green on arrival — phase 5, with the diagnosis for each

**Four, where the breakdown predicts one.** The three the breakdown does not
predict are all controls of guards the test author writes whole, which is
structural rather than a phase that did less than it was given: a guard's own
vacuity control and its own positive control are green the moment the guard
compiles, because nothing in the implementation can reach them. `testing.md` §2
wants exactly those tests; they are simply not evidence about phase 5.

| Scenario | Green because |
|---|---|
| FR-1.3-S3 | `PreparationError::NothingToPlace` at `crates/mc-client/src/launch.rs:123` already implements it, and `simulation_to_play` asks for the held block before it asks for a world — so a root declaring nothing solid is refused there and not in generation. Predicted by the breakdown. It is a regression guard over the construction move: it reddens if the refusal is lost when the registry stops being built in `mc-client`. |
| FR-7.1-S2 | The positive control is a `tempfile` fixture, so it exercises the scan the test author wrote and nothing else. It cannot be asserted over the real crate instead: the real crate names all four doors today and would stop the day the move lands, which is a control green exactly when the rule is broken. |
| FR-7.1-S3 | The vacuity control, same shape: a directory holding no `src/` is a fact about the fixture. |
| FR-6.2-S2 | The vacuity control of a new guard, same shape. |

## Recorded at authoring — phase 5

- **A retirement surface larger than the one `tasks.md` counts, and it reddens
  at T18 rather than T19.** The counted 22 lines are the places that name
  `TomlFileDefinitionSource`; they are found by a grep for the type. Five more
  `mc-client` test files reach the retired reader **through
  `prepare_scene`/`prepare_launch`** and name no type at all, so no grep for the
  type finds them, and every one of them goes red the moment the construction
  moves. Four were retargeted at authoring, deliberately, so that the transition
  at T18 is a set of tests going green rather than a set going red beside the
  golden frames: `launch_builds_only_the_world_it_needs.rs`,
  `refusals_state_a_cause_once.rs`, `saved_world_texture_layers.rs` (each a
  `zz-beacon` declaration and, in two of them, the four generator declarations
  named by file) and `hud_held_block.rs` (`dirt.toml`/`zz-dirt.toml`). The fifth,
  `crates/mc-client/tests/content_refusals.rs`, is **not** retargeted at
  authoring: all six of its tests are about refusal *text*, and the text a Luau
  refusal carries is better read off a real run than predicted.
- **The scan forbids one spelling in `mc-client` that nothing else does.**
  `content_root` is a bare spelling, so the function `mc-sim` grows to resolve
  the shipped content directory must not be called that — `main.rs` calls it and
  would be reported. That is the guard doing its job rather than an accident, and
  it is written here because it is decided before the implementation rather than
  during it.
- **The retirement, done at T19, and what it cost beyond renaming a type.** The
  22 lines across 11 files were mechanical. Three things were not:
  - **`crates/mc-client/tests/support/content.rs`'s single
    `DECLARATION_EXTENSION` became two**, `BLOCK_DECLARATION_EXTENSION` (`luau`)
    and `HUD_DECLARATION_EXTENSION` (`toml`), and the private helper it fed takes
    the extension as a **parameter** so that every caller states which directory
    it is talking about at the point it asks. The trap it defuses: one constant
    served `declaring_no_blocks` and `declaring_no_hud`, and editing it in place
    would have retargeted every HUD fixture silently — a HUD fixture that stops
    finding declarations reports a root that never declared one, not a fixture
    that looked for the wrong thing.
  - **`crates/mc-world/tests/content_loading.rs` was deleted whole**, its subject
    being the reader that no longer exists. Every property it asserted is
    asserted by the Luau suites through the reader that survives, bar one whose
    *statement* was lost: that a texture key needs no file on disk. That is
    already asserted by FR-2.1-S2's test through the same code path, so a
    replacement test would be a second witness that witnesses nothing; it is
    recorded instead in `crates/mc-world/tests/luau_common/mod.rs`'s module doc,
    where the fixtures make it structural.
  - **`crates/mc-world/tests/dependency_graph.rs` was green for a reason that had
    stopped being true.** Both its assertions held, exactly as `tasks.md`
    predicted — and `toml` is now the *HUD* format's parser, so a guard naming
    only it said nothing whatever about how block definitions arrive, and its own
    second test could no longer tell a loader that reads block declarations from
    one that has stopped. Both tests now name **both** ways a declaration is
    read: `mc-core` reaches neither `toml` nor `mlua`, and `mc-world` reaches
    both. The two tests control each other — a mistyped needle fails the loader
    half rather than passing the contract half in silence.
- **Five `mc-client` test files reached the retired reader without naming it**,
  which is the count the type-grep could not give: the four retargeted at
  authoring plus `content_refusals.rs`, whose six tests are about refusal *text*
  and were retargeted at T19 against a real run rather than a predicted one. A
  sixth, `overlay_over_content.rs`, failed to compile only because the `support/`
  module it links did; it names no door of its own.
- **Measured, not assumed.** With the tests as written and no implementation,
  10 of the 15 scenarios are red and 19 test functions fail in total (the
  additional coverage and the retargeted fixtures make up the rest). `cargo fmt
  --check` is clean and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes —
  the latter after `clippy::excessive_nesting` bit a fixture helper's
  `for` + `if`/`else`, which is the fourth phase running that a green suite has
  been no evidence about a lint.
- **The declared-text fixture overlongs `texture` and not `name`, and the choice
  was measured rather than argued.** Both fields are checked by one bound through
  one rendering, so either trips the same refusal down the same path; what
  differs is what the refusal quotes back onto a page. Run both ways:
  `texture` gives `… block `example:amber`, field `texture`: `texture` holds 257
  characters, and a declared value may hold at most 256`, while `name` gives a
  line carrying a **257-character block id** — unreadable, and teaching a mod
  author nothing the count in the cause does not already say. The bound is the
  subject either way; the field is chosen for the reader.
- **Three of roughly fourteen reachable refusals are quoted, and the gap is filed
  rather than closed.** A mod author can trip every required-field and wrong-kind
  case, malformed ids, duplicates, an empty root, compile errors, raises, all
  three host limits, a non-table return, an entry named `.luau` that is not a
  file, and all four bounds. Expanding this guard's run twelve-fold inside a
  validation fix, at the end of the spec that found the defect, is the change
  that introduces defects when there is least appetite to go looking — and the
  quoting and the prose want writing together rather than one retrofitted onto
  the other.
- **The loader's print record became one value, and the shape is an enum rather
  than a struct.** The ruled-in gap needed a boundary that can say "the host
  stopped keeping this"; what it is answered with is
  `Printed::Whole(Vec<String>)` / `Printed::Truncated { kept, dropped:
  NonZeroU64 }`, returned by `LuauFileDefinitionSource::printed()`. **A struct of
  two fields makes the distinction available; a variant makes it unmissable** —
  a caller cannot reach the lines without saying which of the two answers it is
  holding, which is exactly `FieldNames::{Enumerated, MoreThanAllowed}`'s shape
  from D3 in this same spec. `NonZeroU64` is what makes `Truncated { dropped: 0 }`
  — a record that says it stopped keeping and then names nothing — unrepresentable
  rather than merely wrong. The change carries no new mechanism and computes
  nothing the host was not computing already.
  - **It strengthens three tests that were not about it.** Every reading in
    `luau_declaration_guard.rs` that asserts *nothing* was printed now compares
    against `Printed::Whole(…)`, so each of them also rejects a record the host
    had stopped keeping — free, and previously unreachable.
  - **Its RED is a compile failure and that is the honest shape**, because the
    defect *is* that the boundary type cannot express the distinction. Measured
    rather than left as a claim: a skeleton answering `Whole` unconditionally was
    put in by hand, the new test failed on its **assertion** — the noisy root
    accounting for 1,024 of 2,048 printed lines and reporting no truncation,
    while the quiet root passed — and the skeleton was reverted by hand with
    `git diff --exit-code` clean over every crate's `src/` before the commit.
- **After T17, T18 and the T19 retirement, 13 of the 15 are green.** The two that
  are not are FR-6.1-S1 and FR-6.2-S1, and neither is the implementation's: both
  are waiting on T22 to rewrite the pages. Every control that could only be
  watched from a real run has now been watched — the documentation guard's
  verbatim and drift controls both pass against a Luau refusal, which is the
  evidence that its comparison works and that FR-6.1-S1's redness is the pages
  and not the guard.

---

## Phase 6 — The client receives content resolved

**6 scenarios · 6 tests, plus 4 of additional coverage.**

Phase 5's 15 and phase 6's 6, plus phase 1's 25, phase 2's 7, phase 3's 11 and
phase 4's 9, is the spec's 73.

Test command — the four binaries this phase adds:

```
cargo test -p mc-sim --no-fail-fast --test resolved_client_content
cargo test -p mc-client --no-fail-fast --test client_view_of_resolved_content \
  --test stated_layers_are_honoured --test client_derives_no_layer_assignment
```

They also run under `cargo nextest run --workspace`, which is what the gate
invokes.

### Interfaces decided here, and they are binding on the implementation

The seam is a value, and nothing else in this spec pins its shape. Five
decisions, each of which a test here could not have been written without:

```rust
// mc-core — the resolved content value. It lives here and not in `mc-sim`
// because `mc-render` may not name `mc-sim`, and because a content primitive
// with no I/O is what this crate is for.
pub struct ResolvedBlock {          // Debug + Clone + PartialEq + Eq
    pub name: BlockName,
    pub texture: TextureKey,
    pub is_solid: bool,
}
pub struct ResolvedContent {        // Debug + Clone + PartialEq + Eq
    // Carries no `replaceable`, no `breakable` and no `breaks_into`.
}
impl ResolvedContent {
    pub fn stating(
        blocks: impl IntoIterator<Item = ResolvedBlock>,
        layers: impl IntoIterator<Item = (TextureKey, u16)>,
    ) -> Self;
    pub fn blocks(&self) -> impl Iterator<Item = &ResolvedBlock>;      // registration order
    pub fn layer_assignment(&self) -> impl Iterator<Item = (&TextureKey, u16)>;
}

// mc-sim — the simulation resolves one when it reads a content root.
pub struct LoadedContent {
    pub registry: BlockRegistry,    // the residue, unchanged: this binary is also the server
    pub hud: HudLayout,
    pub resolved: ResolvedContent,
}

// mc-render — a stated assignment is honoured, and a packed corner's layer is
// readable so that "honoured" can be asserted through the packer.
impl TextureLayers {
    pub fn stated(assignment: impl IntoIterator<Item = (TextureKey, u16)>) -> Self;
}
impl SectionGeometry {
    pub fn layer_at(&self, vertex_index: usize) -> Option<u16>;
}

// mc-client — the client's view, built from the resolved value and nothing else.
pub struct ContentView { /* mc_client::content */ }
impl ContentView {
    pub fn of(content: &ResolvedContent) -> Self;
    pub fn layers(&self) -> &TextureLayers;
    pub fn is_solid(&self, block: &BlockName) -> Option<bool>;
}
```

- **`ResolvedContent` is compared by value in three tests**, so `Debug` and `Eq`
  are load-bearing rather than incidental: FR-7.2-S1's whole oracle is that two
  roots resolve to values that compare equal.
- **`startup::layers_of` stops taking a registry.** Both preparation paths build
  a `ContentView` from `LoadedContent::resolved` and pack against
  `view.layers()`. The property `layers_of`'s own doc comment records — that the
  geometry a player is handed and the geometry a golden is shot from are packed
  against layers that cannot differ — is what must stay true, not the function.
- **The order the simulation states its assignment in is deliberately not
  pinned.** No reading here depends on which key lands on which layer; what
  FR-7.2-S1 pins is that the assignment names one distinct layer per declared
  texture key, counted from zero. That leaves the shipped four exactly where
  every committed golden frame has them.
- **`ContentView::is_solid` has no in-process consumer yet**, and that is stated
  rather than hidden. `mc-world`'s mesher still takes a `&BlockRegistry`, so
  solidity reaches it through the registry that is still travelling — the same
  residue of one binary hosting both halves that `mc-sim`'s content module
  already records. FR-7.3-S1 requires the view to report it, and the
  composition-root spec is what makes something read it.

### FR-7.2 — What the resolved value carries, and what it does not

| Scenario | File | Test |
|---|---|---|
| FR-7.2-S1 | `crates/mc-sim/tests/resolved_client_content.rs` | `two_roots_differing_only_in_how_a_world_is_mutated_resolve_the_same_client_content` |
| FR-7.2-S2 | `crates/mc-sim/tests/resolved_client_content.rs` | `two_roots_differing_in_a_texture_and_a_solidity_resolve_client_content_that_differs_in_both` |

**Neither is evidence alone and both are here for that reason.** A type with no
`breakable` field cannot fail a test about not having one — the test would not
compile, which is a compile error standing in for an assertion. So S1 asserts
that two roots differing only in the three mutation rules resolve to *equal*
values, and S2 that two roots differing in one block's `texture` and another
block's `solid` resolve to values differing in *both*. S1 alone is satisfied by
a resolver returning a constant; S2 is what rules that out.

**S1 reads the content back in the same comparison as the equality.** Measured
against a resolver that states nothing, the equality half passes — both roots
resolve to the same empty value — and only the readback fails. Without it the
scenario would have been green over a resolver that resolved nothing at all.

**The two changes in S2 sit on different blocks on purpose.** A resolution
carrying only one of the two fields would otherwise still report a difference on
the one block that changed in both, and one difference would read as two.

**Every fixture states a texture key that is not the block's own name**, on the
same terms as the `mc-world` suites: a resolution that read a name into both
fields would be green against any fixture that declared them alike, and that
confusion survived this feature's whole first draft.

**The assignment is read back by shape and not by mapping**, because which key
lands on which layer is the simulation's to decide and no scenario here depends
on it. What is pinned is that it names each declared texture key exactly once
and hands out that many distinct layers from zero — a resolution answering with
an empty assignment, or one key short, would otherwise satisfy both scenarios.

### FR-7.3 — The client's view, built from that value alone

| Scenario | File | Test |
|---|---|---|
| FR-7.3-S1 | `crates/mc-client/tests/client_view_of_resolved_content.rs` | `the_clients_view_reports_each_stated_blocks_layer_and_solidity_as_the_value_states_them` |

**This is the single assertion separating a seam from a rename, and it works
only because the fixture is written down.** The resolved content in that file is
literal data: no content root is copied, no path is opened, no registry is built
and no scripting host is constructed anywhere in the file. **No assertion in it
can enforce that** — it is held by the code that builds the fixture and by a
reviewer reading it, and the file's own doc comment says so. What it rules out
is a resolved value that is a newtype over the registry and a client view that
reaches back through one; either would make the file fail to compile or fail to
run without a content root.

**The stated assignment disagrees with the sorted position of its keys**, which
is what makes the reading falsifiable at all: the three keys sort `jade`, `onyx`,
`quartz` and the value names them `1`, `2`, `0`. Measured against a view that
derives positionally, it reports `2`, `1`, `0` where the value says `0`, `2`, `1`.

**The blocks name texture keys that are not their own names**, so a view that
reported a block's layer by looking its *name* up in the assignment answers
nothing for any of the three rather than accidentally answering correctly. A
fourth block the value states nothing about is asked after in the same
comparison, so a view answering for anything handed to it fails here.

### FR-7.4 — The layer assignment is honoured, not derived

| Scenario | File | Test |
|---|---|---|
| FR-7.4-S1 | `crates/mc-client/tests/stated_layers_are_honoured.rs` | `corners_are_packed_with_the_layer_the_assignment_names_and_not_with_a_sorted_position` |
| FR-7.4-S2 | `crates/mc-client/tests/stated_layers_are_honoured.rs` | `a_block_the_assignment_names_no_layer_for_is_refused_by_name_rather_than_packed_from_layer_zero` |
| FR-7.4-S3 | `crates/mc-client/tests/stated_layers_are_honoured.rs` | `a_block_whose_declared_texture_is_not_its_own_name_is_refused_at_packing_time_naming_the_block` |

**All three assert through the packer, never through the view.** Asking the view
which layer it holds for a key leaves the consumer free to re-derive one of its
own, which is the exact failure the requirement is about — so every reading packs
real quads through `build_section_geometry`, the same function the client's own
`scene_of` calls, and reads the layer back out of the corners it produced. That
discharges the additional coverage the amendment recorded as owed beside
FR-7.4-S1; it is the scenario's own observable rather than an extra test.

**FR-7.4-S1 carries its own fixture guard inside the assertion.** The scenario is
worded around a *disagreeing* assignment because a test comparing two copies of
one sort cannot fail, and whether this fixture disagrees is a property of the
fixture that no outcome would report. So the comparison's second half is the
three stated layers themselves, against `[2, 0, 1]` — the positions those keys
occupy in sorted order being `0`, `1`, `2`. A future edit that made the fixture
agree with the sort fails here rather than passing silently forever.

**FR-7.4-S1 also asserts every corner and not the first of each quad.** A packer
writing the right layer into a quad's first corner and the wrong one into the
other three would draw three quarters of the world from the wrong texture, and a
sampled assertion could not see it.

**FR-7.4-S2 asserts two verdicts in one comparison.** The block the assignment
names no layer for must be refused *by name*, and the block beside it that the
assignment does name must still pack — without the second half a packer that
refused everything would read the same as one that refused the right thing.

**These fixtures declare `texture` equal to `name`, against this feature's
standing convention, and that is a constraint on the fixture rather than a fact
about the requirement.** An entry of the assignment is selected by the block's
*name* today, so a fixture whose blocks named a different texture would redden on
`UnresolvedTexture` — red for the wrong reason, reading as a defect in the
assignment when it is not one. FR-7.4-S3 is the scenario that owns that
behaviour, and its fixture states the two differently on purpose.

**The verdict is a total enum.** `Packed::{Corners, RefusedNaming,
RefusedOtherwise}` means a packer that accepts what it should refuse fails on the
comparison rather than ending the test before its assertion ran, and a refusal
for some other reason is not silently read as the refusal under test.

#### FR-7.4-S3 is green the moment it is written, it is not a control, and it has a known expiry

It pins a **live defect** the tree already has, so that the name-for-texture
substitution is a reading instead of a comment in two `CLAUDE.md` files. The
assignment is built from each block's declared `texture`; an entry is selected by
the block's **`name`** (`crates/mc-render/src/geometry/mod.rs:171`). The two agree
only because all four shipped blocks declare them identically. Closing the gap
changes `build_section_geometry`'s binding signature and belongs to the per-face
texture work, which the roadmap owns; it is deliberately **not** closed here.

**When that work lands, this test goes red — and that red is the success signal,
not a regression.** Whoever meets it deletes the scenario together with the spec
that closed the gap. It is written down here because a red test with no
explanation is exactly the kind that gets annotated and then ignored.

Its greenness is **not** evidence that phase 6 did work.

---

## Additional coverage — phase 6

| File | Test | What it catches |
|---|---|---|
| `crates/mc-client/tests/stated_layers_are_honoured.rs` | `the_held_block_indicator_resolves_by_the_blocks_name_too_and_shows_nothing_for_such_a_block` | The **second** site of the substitution FR-7.4-S3 pins, which no existing note records: `mc_render::hud::held::held_swatch` (`crates/mc-render/src/hud/held.rs:109`) resolves the held-block indicator by parsing the block's own name as a texture key. `crates/mc-render/CLAUDE.md` and `docs/technical/architecture.md:589–590` record the geometry builder's copy and **neither mentions this one**, so a spec closing the gap would fix one site and leave the other — showing a block drawing correctly in the world while its indicator drew nothing, which reads as a HUD bug. A block whose key *is* its name is asked after in the same comparison, so an indicator resolving nothing at all does not read the same. **Green on arrival and expiring with FR-7.4-S3**, for the same reason and at the same moment. |
| `crates/mc-client/tests/client_derives_no_layer_assignment.rs` | `the_clients_own_sources_derive_no_layer_assignment_of_their_own` | The one thing no behavioural reading in this phase can reach: that the **preparation paths** — the ones a player launches through and every golden frame is shot through — build the view rather than going on deriving an assignment from the registry they are still handed. See "what the tests cannot express" below for why it has to be a scan. Two needles, both chokepoints rather than type names: `texture_keys(`, where a derivation starts, and `TextureLayers::resolve`, where one ends. Measured against a skeleton that adds the view and leaves `layers_of` alone: `DerivedIn(["src/content.rs names \`TextureLayers::resolve\`", "src/startup.rs names \`texture_keys(\`", "src/startup.rs names \`TextureLayers::resolve\`"])` — the first entry being the skeleton's own view, which is the second way a client derives one and is caught on the same needle. |
| `crates/mc-client/tests/client_derives_no_layer_assignment.rs` | `the_same_scan_reports_a_client_source_that_derives_one_and_says_how_it_derived_it` | The positive control, and the only direction that guard has: a walk that broke, a filter that skipped everything or a needle that stopped matching would report a clean client forever. The fixture commits **both** spellings and the expected report is derived from the needle list, so a needle added without a fixture committing it fails here rather than standing unwatched. |
| `crates/mc-client/tests/client_derives_no_layer_assignment.rs` | `a_scan_that_read_no_client_source_says_so_rather_than_reporting_a_clean_client` | The vacuity control, and the reason that verdict is enumerated rather than an emptiness check: a scan that read nothing and a clean client must never compare equal. |

The scan follows `crates/mc-client/tests/client_names_no_content_door.rs` down to
the whole-path report and the doc-comment strip. **Its file-filter and
doc-comment controls are deliberately not repeated**: they are the same two
carve-outs for the same two reasons, and a fixture proving a filter twice proves
it once.

---

## Green on arrival — phase 6, with the diagnosis for each

**Four test functions of the ten, and two of them are scenarios.** None is
evidence phase 6 did work.

| Test | Green because |
|---|---|
| FR-7.4-S3 | It **pins** behaviour the tree already has and is not a control. See above: the one reading in this spec with a known expiry, and the later work turning it red is the signal it exists to give. |
| the held-block indicator | The same live defect at its second site, so it is green for FR-7.4-S3's reason exactly, and expires with it. |
| the scan's positive control | A `tempfile` fixture exercising guard code the test author owns whole. It cannot be asserted over the real crate instead — the real crate derives an assignment today and would stop the day the view lands, which is a control green exactly when the rule is broken. |
| the scan's vacuity control | Same shape: a directory holding no `src/` is a fact about the fixture. |

The other six fail on an **assertion**, not on a fixture guard and not on a
compile error. Measured against a hand-applied skeleton — the resolved value and
the view existing, the simulation stating nothing, and `ContentView::of`
deriving an index positionally instead of honouring the assignment:

```
resolved_client_content:          0 passed; 2 failed
client_view_of_resolved_content:  0 passed; 1 failed
stated_layers_are_honoured:       2 passed; 2 failed
client_derives_no_layer_assignment: 2 passed; 1 failed
```

The skeleton was reverted by hand and `git diff --exit-code` was clean before the
commit.

## Recorded at authoring — phase 6

- **The wiring from the preparation paths to the stated assignment is not
  behaviourally falsifiable in this phase, and the reason outlives it.** A
  client that honours the assignment and a client that derives one answer
  *identically* for every content root that can be built today, because the
  assignment a simulation states over a real root agrees with the order a
  positional derivation produces. Nothing about a real launch can tell them
  apart — not the goldens either, since permuting the assignment permutes the
  array texture's fill in the same breath and the picture is unchanged. So
  FR-7.4-S1 asserts the honouring over a value written down, and a source scan
  is what stands between that and a production path still deriving. **The two
  stop agreeing the moment an assignment is appended rather than renumbered**,
  which is hot reload's, and at that point the scan can be replaced by the
  reading it stands in for.
- **A green suite is no evidence about a lint, for the fifth phase running.**
  `clippy::type_complexity` bit
  `fn reported_by(..) -> Result<Vec<(Option<u16>, Option<bool>)>, Box<dyn Error>>`
  while every one of these tests behaved exactly as designed. It was found by
  running `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  directly, **under the skeleton**, because a phase whose tests name types that
  do not exist has no compilable tree for the gate to run on until the
  implementation lands. `cargo fmt --check` found four formatting slips in the
  same pass and each was fixed by hand.
- **`SectionGeometry::layer_at` is new surface and it is the test's doing.**
  Nothing outside `mc-render` could read the layer a corner was packed with:
  `SceneGeometry` hands out bytes, and `PackedVertex` keeps its only constructor
  private on purpose so that `unpack` stays total. Decoding the bit layout in a
  test would have been a second copy of the packing decision. The accessor
  mirrors `world_corner`, which is the sibling reading it was modelled on.
- **`ContentView::is_solid` is asked for by FR-7.3-S1 and consumed by nothing.**
  Recorded rather than quietly added: the mesher reads solidity through the
  registry that is still travelling, which is the residue of one binary hosting
  both halves, and the composition-root spec is what gives the view a consumer.
  A reviewer meeting an accessor with no caller should meet this note first.

### What the amendment's tests have to carry, recorded before they were written

Left standing, and every one of them is now discharged — phases 5 and 6 above
say where. It is kept because an obligation and its discharge are worth reading
together, and because it is the record of what was owed before anybody knew how
it would be paid.

FR-7's eight arrived with the 2026-08-16 amendment. Four obligations do not
follow from the scenario wording and would be lost if they were not written down
now.

- **FR-7.1-S1's verdict is enumerated, not an absence.** `hits.is_empty()`
  cannot tell an empty answer from a scan that can no longer look — a moved
  directory, a broken walk, an exemption grown to cover the tree. The verdict
  distinguishes *every door unnamed*, *these doors named here*, and *no source
  read*, which is what makes FR-7.1-S3 a test rather than a caveat.
- **FR-7.1-S2 is the positive control and every needle needs one.** A needle
  matching nothing even when the offence is committed passes its scan forever, so
  the fixture commits **all four** and the expected hit count is derived from the
  needle list rather than written down. That is
  `crates/mc-client/tests/seam_boundaries.rs`'s own fourth control, for the same
  reason.
- **FR-7.2 is asserted by discrimination and needs both directions.** Two roots
  differing only in `replaceable`/`breakable`/`breaks_into` resolving alike (S1)
  is satisfied by a resolver that returns a constant; two roots differing in a
  `texture` and a `solid` resolving differently (S2) is what rules that out.
  Neither alone is evidence.
- **FR-7.3-S1's fixture is written down, and no assertion can enforce that.**
  The resolved value is literal data in the test: no registry built, no path
  opened, no scripting host constructed. It is a constraint held by the code that
  builds the fixture and by a reviewer reading it, and it is the only thing
  separating this scenario from a rename — so it is stated here rather than
  assumed.
- **FR-7.4-S1's assignment must disagree with the positional order.** A fixture
  whose assignment happens to equal the sort over its sorted keys is two copies
  of one decision agreeing with each other, and it cannot fail whatever the
  client does.
- **FR-7.4-S1's blocks must declare `texture` equal to `name`, and that is a
  constraint on the fixture rather than a fact about the requirement.** An entry
  of the assignment is selected by the block's *name* today
  (`crates/mc-render/src/geometry/mod.rs:171`), so a fixture whose blocks declare
  a different `texture` reddens on `UnresolvedTexture` — red for the wrong
  reason, which reads as a defect in the assignment and is not one. FR-7.4-S3 is
  the scenario that owns that behaviour; S1 must not accidentally re-test it.
- **FR-7.4-S3 will be green the moment it is written, and it is not a control.**
  It pins the name-for-texture substitution the tree already has, so that a live
  defect is a test instead of a comment in two CLAUDE.md files. **It has a known
  expiry: PRO-902/PRO-914 closing the gap turns it red, and that red is a success
  signal, not a regression.** Whoever meets it deletes the scenario with the spec
  that closed the gap — it is written down here because a red test with no
  explanation is exactly what gets annotated and then ignored.

### Additional coverage the amendment already owes

Recorded now so the phase that writes it inherits the reason. **The two phase-5
rows were written at phase 5** and appear in its own additional-coverage table
above; they are left here so the debt and its discharge sit together.

| Where | What it catches |
|---|---|
| Phase 5, beside FR-7.1 | The scan's **file filter**: a needle named in a sibling `*_test.rs` under `crates/mc-client/src/` is skipped while the same needle in the module beside it is still found. A filter that skipped too much would leave FR-7.1-S2 green while scanning almost nothing. |
| Phase 5, beside FR-7.1 | The **doc-comment strip**: prose naming a door — `startup.rs` and `launch.rs` both discuss `content_root` and `HudLayout::load` in doc comments today — is not a use of it. Without this the guard is unsatisfiable for a reason that has nothing to do with the seam. |
| Phase 6, beside FR-7.4-S1 | The assignment is honoured **through the packer**, not only through the resolver: the layer index that ends up inside a packed vertex is the one the assignment names. Asserting only what the resolver returns leaves the consumer free to re-derive, which is the exact failure the requirement is about. |
| Phase 6, beside FR-7.4-S3 | The **second** site of the same substitution: `mc_render::hud::held::held_swatch` (`crates/mc-render/src/hud/held.rs:109`) resolves the held-block indicator by the block's name too. `crates/mc-render/CLAUDE.md` and `docs/technical/architecture.md:589–590` both record the geometry builder's copy and **neither mentions this one**, so a spec closing the gap would find one site and leave the other. One test through the indicator is what stops that. |
