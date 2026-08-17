# Test map: SPEC-017 — Hot reload

Scenario → test file → test name. **Test names carry no scenario ID and never
will**; this file is the whole of the mapping, and it is where a reader goes to
find out which test grades which scenario.

Phase 2's author appends below phase 1's section rather than editing it.

---

## Phase 1 — A candidate is applied at a tick boundary, or nothing is

**26 scenarios, 26 tests.** Test command — the eleven binaries these live in, by
name, so a run of them is a run of this phase and nothing else:

```
cargo nextest run -p mc-sim -p mc-client -E 'binary(/^reload_/)'
```

And the whole of both crates, which is what a phase boundary runs:

```
cargo nextest run -p mc-sim -p mc-client
```

**Nothing here compiles until the skeleton lands, and the fixtures are arranged
to keep that window narrow.** The three names phase 1 adds — `mc_sim::reload`,
`mc_sim::simulation::Accepted` and `Session::adopt_content` — do not exist yet,
so every binary naming them is a compile error rather than a red assertion. The
fixture modules that name them are therefore reached by `#[path]` rather than
declared in `support/mod.rs`, which would have put them in **every** binary that
says `mod support;`. As it stands the window covers the eleven reload binaries
plus the six that include the input harness, and every other test binary in both
crates still builds and runs.

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-2.2-S1 | `crates/mc-sim/tests/reload_admission.rs` | `a_candidate_that_stops_declaring_a_block_the_world_holds_is_refused_naming_it` |
| FR-2.2-S2 | `crates/mc-sim/tests/reload_admission.rs` | `a_candidate_dropping_a_block_no_cell_holds_any_more_is_accepted` |
| FR-2.2-S3 | `crates/mc-sim/tests/reload_admission.rs` | `a_refusal_names_every_block_the_world_holds_that_the_candidate_dropped_ascending` |
| FR-4.1-S1 | `crates/mc-client/tests/reload_solidity.rs` | `stone_declared_not_solid_stops_holding_the_player_up` |
| FR-4.1-S2 | `crates/mc-client/tests/reload_solidity.rs` | `water_declared_solid_starts_holding_the_player_up` |
| FR-4.1-S4 | `crates/mc-sim/tests/reload_solidity_views.rs` | `the_physics_and_a_placements_occupancy_check_agree_a_stone_cell_stopped_being_solid` |
| FR-4.1-S5 | `crates/mc-client/tests/reload_solidity.rs` | `a_candidate_refused_for_another_reason_leaves_stone_holding_the_player_up` |
| FR-3.3-S1 | `crates/mc-client/tests/reload_held_block.rs` | `a_reload_that_moves_no_solid_block_to_the_front_leaves_the_same_block_in_the_hand` |
| FR-3.3-S2 | `crates/mc-client/tests/reload_held_block.rs` | `a_newly_declared_solid_block_registered_first_arrives_in_the_players_hand` |
| FR-3.3-S3 | `crates/mc-client/tests/reload_held_block.rs` | `a_candidate_registering_no_solid_block_is_refused_saying_there_would_be_nothing_to_place` |
| FR-3.3-S4 | `crates/mc-client/tests/reload_held_block.rs` | `a_placement_after_a_reload_writes_the_newly_declared_block_the_client_now_holds` |
| FR-1.3-S1 | `crates/mc-client/tests/reload_tick_boundary.rs` | `a_break_asked_for_before_the_swap_succeeds_and_one_asked_for_after_it_is_refused` |
| FR-3.2-S2 | `crates/mc-client/tests/reload_tick_boundary.rs` | `the_tick_after_an_accepted_candidate_is_the_tick_before_it_plus_one` |
| FR-3.1-S1 | `crates/mc-client/tests/reload_keeps_the_world.rs` | `a_broken_cell_is_still_empty_and_a_placed_one_still_holds_what_was_placed_after_a_reload` |
| FR-3.1-S2 | `crates/mc-client/tests/reload_keeps_the_world.rs` | `every_cell_of_the_shipped_world_holds_what_it_held_after_a_reload` |
| FR-3.1-S3 | `crates/mc-client/tests/reload_keeps_the_world.rs` | `the_cell_a_player_stood_on_still_holds_stone_once_stone_has_stopped_holding_them_up` |
| FR-3.2-S1 | `crates/mc-client/tests/reload_keeps_the_player.rs` | `a_reload_leaves_the_player_where_the_same_ticks_with_no_reload_would_have_put_them` |
| FR-3.2-S3 | `crates/mc-client/tests/reload_keeps_the_player.rs` | `a_reload_while_the_player_is_falling_leaves_their_velocity_where_the_tick_left_it` |
| FR-4.2-S1 | `crates/mc-client/tests/reload_mutation_rules.rs` | `stone_declared_unbreakable_refuses_the_next_break_as_indestructible` |
| FR-4.2-S2 | `crates/mc-client/tests/reload_mutation_rules.rs` | `stone_given_a_residue_leaves_that_block_behind_when_it_is_broken` |
| FR-4.2-S3 | `crates/mc-client/tests/reload_mutation_rules.rs` | `stone_declared_replaceable_lets_the_next_placement_build_over_it` |
| FR-4.2-S4 | `crates/mc-client/tests/reload_mutation_rules.rs` | `a_candidate_whose_residue_names_a_block_nothing_declares_is_accepted` |
| FR-4.4-S1 | `crates/mc-client/tests/reload_leaves_other_blocks_alone.rs` | `a_candidate_editing_one_declaration_leaves_every_other_blocks_declared_fields_alone` |
| FR-3.4-S1 | `crates/mc-client/tests/reload_and_the_save.rs` | `a_save_written_after_a_reload_resumes_against_that_content_without_asking_the_player` |
| FR-3.4-S2 | `crates/mc-client/tests/reload_and_the_save.rs` | `a_save_written_before_the_reload_is_refused_by_that_same_changed_content_naming_the_block` |
| FR-4.3-S1 | `crates/mc-sim/tests/reload_registers_a_new_block.rs` | `a_block_declared_for_the_first_time_is_registered_and_answers_for_its_declared_fields` |

**26 scenarios, 26 tests, each scenario exactly once.**

### Fixtures these tests own

| File | What it is |
|---|---|
| `crates/mc-client/tests/support/reload.rs` | The declarations an author edits (a Luau chunk builder), roots built from copies of the shipped one, and `Adoption` — the enumerated verdict every client scenario compares |
| `crates/mc-client/tests/support/reload_world.rs` | The worlds a reload is driven over, where the player stands in them, the three derived aims, and `Edit` — the enumerated verdict every click scenario compares |
| `crates/mc-client/tests/support/reload_save.rs` | Reading a world back out of a save, and what a relaunch against changed content makes of it. **Split out of `reload_world.rs` when that file reached the 600-line test-file limit** — by responsibility and not by count: one module is what a reload is *driven over*, the other is what is *read back afterwards*, and the second is the only half that touches a file. Three suites need it; five do not. Two cells and one conversion cross the line and are `pub` for that reason alone |
| `crates/mc-sim/tests/support/roots.rs` | The smaller half of the same thing for `mc-sim`'s own suites: copy, remove, restate, add, and `Adoption` |
| `crates/mc-client/tests/support/input/mod.rs` | Gains two forwards and nothing else: `adopt` (hands the client a candidate) and `published` (reads the published tick **without** advancing one, which `tick` cannot do) |

**The duplication between `support/reload.rs` and `support/roots.rs` is real and
is a consequence of the crate boundary**, not an oversight: `mc-sim` and
`mc-client` are separate test crates and share no code. The `mc-sim` half is
deliberately the smaller one.

### Where a test drives through `Session`, and the three that do not

Every FR-3, FR-4 and FR-5 scenario is required to drive through `Session` rather
than through `Simulation::adopt`, because a test that calls the simulation's own
door is agreement between two callers of one function: the client's drive can
stop calling it and every such test stays green. **Twenty-three of the
twenty-six do.**

Three reach `mc_sim::reload`'s public door instead, each because the thing the
scenario asks about has no client surface at all — and each is named here rather
than left to be discovered:

- **FR-2.2-S1, S2, S3** (`reload_admission.rs`) — these are about the
  simulation's own answer: which candidates it admits and what it says when it
  turns one away. In phase 1 a refusal reaches nobody; the printing is phase 3's.
  They are not FR-3/4/5 scenarios and the rule does not reach them.
- **FR-4.1-S4** (`reload_solidity_views.rs`) — the second of the two views is
  the registry the world was resolved against, and `Session` hands out no borrow
  of the world and none of the registry, deliberately. The client-honouring half
  of FR-4.1 is carried by S1, S2 and S5, which do drive through `Session`.
- **FR-4.3-S1** (`reload_registers_a_new_block.rs`) — "answers for its declared
  fields by name" is a question to a registry, and there is no client surface for
  it. The client-honouring half of FR-4.3 is FR-3.3-S2 and FR-3.3-S4, both of
  which drive through `Session` and both of which are about the same new block.

### The eight `tasks.md` predicts green on arrival, and what each grades

`tasks.md` predicts eight of these twenty-six are green under the accepting-no-op
skeleton. **Measured against that skeleton: three, not eight — 23 failed and 3
passed.** Green on arrival were **FR-3.3-S1, FR-3.4-S2 and FR-4.4-S1** only.

The other five — FR-3.1-S1, FR-3.1-S2, FR-3.2-S1, FR-3.2-S3 and FR-4.1-S5 —
reddened because of where their observation is taken rather than because the
breakdown was wrong about the skeleton: each reads the world out of a **save
written after the swap**, or compares against an **independent second run**, or
carries in the same test the control that says the swap happened. A do-nothing
swap fails those readings. The prediction was made per design and the count is
measured against a tree, which is the same distinction `tasks.md` records about
where its own list came from.

**Every row below still owes its mutation, and for two of them the red above was
misleading.** Red on arrival is not evidence the mutation bites: it says the
accepting no-op is discriminated, not that the shape `tasks.md` names is. FR-3.2-S1
and FR-3.2-S3 reddened under the skeleton **on their control halves only** —
their headline comparisons were vacuous, and the mutation round is what found it.
The correction is recorded under "the reading a swap can reach" below, and both
rows here describe the repaired reading.

Each row is listed with the mutation, with what the test is shaped to make that
mutation bite, and with the measured outcome.

| Scenario | What it grades | What it does **not** grade | Its named mutation, and how this test is shaped to feel it |
|---|---|---|---|
| FR-3.1-S1 | that the broken cell is empty and the placed cell holds what was placed, read out of a save written after the swap | that the swap did anything — carried in the same test by stone stopping holding the player up | *`adopt` clears the section holding the player's edits* — the two cells are read back out of a save, so a cleared section reports `nothing` at the placed cell |
| FR-3.1-S2 | every cell of the shipped world, against a **regeneration from the declared seed** rather than another run | which cells the player edited (none here) | *write `base:dirt` into one world position inside `adopt`* — the walk is over `declared.extent().positions()`, every cell of the footprint, and the cell count is asserted at `1 048 576` so a shortened walk fails rather than agreeing over less |
| FR-3.2-S1 | position, yaw and pitch **one tick after the swap**, against a second client advanced the same ticks with the same script and no candidate | velocity, which is FR-3.2-S3's | (a) *`adopt` sets the position to the origin* — **bit**, `(0.0724, -0.0083, 0.0196)` against `(10.0204, 10.0, 8.9110)`. (b) *run the oracle one tick short* — **bit**, 0.0724 blocks apart in x; the player is **walking**, and a guard refuses a run that never left the spawn. Measured across (b): yaw, pitch and height are bit-identical, so **only the two horizontal axes carry that signal** |
| FR-3.2-S3 | velocity **one tick after the swap**, against that same kind of oracle | position and orientation, which are S1's | *`adopt` zeroes velocity* — **bit**, `-0.5` against `-3.5`. The difference is exactly the velocity standing before the swap, which is why a guard refuses a reading taken at rest. The player is airborne over a floor whose solidity is being **taken away**, so no cell they overlap becomes solid and phase 5's clearing path is never on this route |
| FR-3.3-S1 | that the block in the hand is still the first solid one, read through `Session::held_block` | that anything about the content changed | *`adopt` sets `holding` to `base:water`* — the reading goes through the client's own accessor, so a re-derivation that never reached the session's field reddens |
| FR-3.4-S2 | that a save written before the edit is refused by the changed root, naming `base:stone` and not the other three | anything about the reload path — it uses none | *make `RegistryVerdict::resolve` report an empty `changed`* — the assertion is the refusal naming the block; an empty verdict resumes silently and the test reddens. The world holds all four blocks so "named only the one that changed" is not true by construction |
| FR-4.1-S5 | that a candidate refused for another reason leaves stone holding the player up, **and** which refusal it was | the accepted path | *write the registry before the names-held check inside `adopt_candidate`* — the reading is the physics after 60 ticks, so a registry written before the refusal drops the player through the floor |
| FR-4.4-S1 | that the three blocks the author never touched are recorded identically either side of the swap | that the author's own edit landed — that is FR-4.2-S1, S2 and S3, through the same door | *apply stone's fields to every block in the candidate build* — the comparison is over what a save records for `base:dirt`, `base:grass` and `base:water`, and a fixture guard refuses a save naming fewer than four blocks |

### The reading a swap can reach, and the two tests that were not taking it

**Found by the mutation round, and it is the finding phase 1 should be remembered
for.** FR-3.2-S1 and FR-3.2-S3 originally read their headline value from the
snapshot standing at the moment the candidate was taken up. `Simulation::adopt`
publishes no tick — which is correct, and FR-3.2-S2 pins it — so that snapshot is
the one the *previous* `advance` stored, and **nothing the swap does to the player
can change what it holds.** Both assertions read a value written before the thing
they were judging ran. Measured: setting the player's position to the origin inside
the swap left S1 green, and zeroing their velocity left S3 green — exactly the two
mutations `tasks.md` predicts would redden them.

Neither test's control half rescued it. A player teleported to the origin is still
*below* the untouched one, and a player whose velocity was zeroed still falls, so
both controls went on grading "the candidate was taken up" — correctly — while
nothing at all graded "the player crossed it untouched".

**The repair is one tick, advanced in both clients after the candidate lands.** The
swap writes the player's own fields and the next tick publishes them, so that is
the first reading a swap could have written into.

**What makes the two clients comparable at that tick is that neither is standing on
the block the candidate changed, and this is the constraint the repair turns on.**
A player standing on stone would begin to fall on that very tick — *legitimately* —
and the comparison would go red against a correct client, which is the over-tight
assertion `testing.md` §2 names. So:

- **S1's floor is grass and the stone is a ceiling.** The candidate takes stone's
  solidity away, which cannot move either player. The effect FR-3's preamble
  requires in the same test is observed by a look instead: a break aimed upward
  finds the ceiling for the untouched client and `NoTarget` for the reloading one.
  A whole layer rather than one block, so it is reached wherever the walk ended up.
- **S3 is in free fall two blocks above the floor.** One tick of falling is one
  tick of falling whatever the floor is declared to be, and the two runs part
  company later, when one lands and the other does not.

**S1's two ceiling readings are each other's control, in one run.** The `true` and
the `false` come from the same code over the same world differing only in whether
the swap happened, so neither can be vacuous while the other holds.

Measured after the repair, each mutation reddening only its own scenario:

| Mutation, inside `Simulation::adopt` | Outcome |
|---|---|
| `self.player.position = Vec3::ZERO` | **Bit.** `(0.0724, -0.0083, 0.0196)` against the oracle's `(10.0204, 10.0, 8.9110)` |
| `self.player.velocity = Vec3::ZERO` | **Bit.** velocity.y `-0.5` against `-3.5` — the difference is exactly the speed standing before the swap, one tick of gravity against seven |

Both applied by hand to a production file, uncommitted, reverted by re-editing,
`git diff --exit-code` clean on every `src/` tree afterwards.

**Phase 5 is why this mattered and not only phase 1.** FR-7's clearing search
deliberately writes the player at the swap, and FR-3.2-S3's stated job is to hold
for a player who was *not* cleared. As originally written, phase 5 could have moved
an uncleared player or reset their velocity with both of these green.

### Weak instruments and overlaps, named

- **FR-4.2-S1 and FR-1.3-S1 share their second half.** Both end in a break of
  stone refused as indestructible after the candidate landed. FR-1.3-S1 is the
  pair — a break on the *earlier* tick that succeeded — and that half is its own;
  FR-4.2-S1 is the single witness for "the rule is in force". Neither is deleted:
  the scenarios themselves overlap and one test each is the contract.
- **FR-3.1-S3 overlaps FR-3.1-S2's control half.** Both observe stone ceasing to
  hold the player up. What S3 adds is the conjunction stated sharply on one cell —
  the cell still holds `base:stone` *while* it has stopped supporting anybody —
  which a swap that emptied the cell would satisfy in S2's totality only by
  failing the totality. Read the two together.
- **FR-4.2-S3 hands over a candidate declaring stone both non-solid and
  replaceable, and it has to.** A placement lands one step back along a ray that
  stopped at the first *solid* cell, so the cell being built over is necessarily
  one the ray passed through — no solid block can ever be the thing a placement
  overwrites. `base:water` is the shipped block that shows the shape. The test
  still grades `replaceable` alone: without it the same click is refused
  `Occupied`.
- **FR-4.2-S4 asserts acceptance *and* the residue rule being in force.**
  Acceptance alone is green under an accepting no-op skeleton, which would make
  the scenario unfalsifiable. The break that follows is where the late-resolution
  contract's price arrives — the residue naming `base:mithril` is resolved at the
  break — and it is the same sentence the scenario itself gives as its reason.
- **FR-2.2-S2's fixture constraint is asserted rather than described.** `tasks.md`
  records it as a constraint no assertion can enforce; it turns out one can. The
  fixture writes water into a section, empties those cells through the world's own
  write path, and then **asserts** that the section's palette still names
  `base:water` while no cell of it holds any. A world that never held water fails
  that guard instead of passing the scenario for the wrong reason.
- **FR-2.2-S3's ordering is discriminating only because of where the fixture puts
  the blocks.** Stone is written into a lower section than grass, so a refusal
  reporting whichever it came across first names stone first. The fixture asserts
  both placements.

### An inherited guard this phase moved, and the two mutations that graded the move

`crates/mc-client/tests/seam_boundaries.rs`'s `OUTSIDE_THE_CORE_GUARD` exempts the
one file whose job it is to drain the player's input and advance the tick. That
file moved: `src/session.rs` became `src/session/mod.rs` when the reload surface
needed room, so the guard reported the core as an offender and the exemption had
to follow it. It maps to no scenario in this spec and is recorded here because the
phase moved it.

**The exemption is the core's own file — `path == "src/session/mod.rs"` — and not
its directory.** Two spellings were available and the difference is the guard's
strength, so the choice is enforced by the control rather than argued in a
comment. The control's fixture now writes four files against one exemption: the
frame path naming every needle, the core (passed over), **a sibling of the core**
(reported), and a **harness file wearing the core's own file name** (reported).

Both rejected spellings were applied by hand and measured:

| Mutation | Outcome |
|---|---|
| `path.starts_with("src/session/")` — follows the split and excuses everything ever put beside the core | **Bit.** The control reddens and **only** the control; the real scan stays green, because a widened exemption looks exactly like a clean tree to it |
| `path.ends_with("mod.rs")` — the bare-name shape, which now excuses nearly every directory in the workspace rather than one file | **Bit**, the same way and alone |

Reverted by hand both times, `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
afterwards. That each mutation reddens the control and nothing else is the
evidence the two directions are independent rather than one check written twice —
and that the real scan stayed green under both is the reason the control exists.

### `World::adopt`'s own refusal — a backstop that had no witness

`crates/mc-sim/src/world/mod_test.rs`, two guards, mapped to **no scenario** and
here on a team-lead ruling after the implementer found the hole.

`adopt` settles solidity before it writes either view, so a candidate that cannot
answer for a block the world holds refuses *having changed nothing* — that is
what makes "a failed reload changes nothing" true by construction. But the
admission stage checks the names the world holds **first**, so in the reload flow
`adopt`'s own refusal is pre-empted: assigning `self.registry` before resolving
solidity passed **1 110 tests**.

**I first argued against a test here and was wrong on a fact.** I claimed reaching
the branch would need a `World` whose `names_held` lies. It does not: the refusal
is `SolidVoxels::resolve`'s, propagated by `adopt` itself, and an ordinary world
holding a grass cell plus an ordinary candidate that does not register grass
reaches it directly. `names_held` belongs to `adopt_candidate`, which is a
wrapper — calling a function below its wrapper is not falsifying anything.

The ruling's own distinction is worth keeping: this is not a speculative accessor
with no production caller. `adopt` runs on every accepted reload, one branch of it
is pre-empted, and that branch guards a player's world against a half-applied
swap. A guarded write path whose guard is doubled deserves a witness for the inner
one precisely because the outer one hides it.

| Guard | What it grades | Mutation and outcome |
|---|---|---|
| `a_candidate_missing_a_block_the_world_holds_is_refused_and_leaves_the_registry_it_had` | the refusal named exactly, **and** that the world still answers for the block it holds | `self.registry` assigned before `SolidVoxels::resolve` — **bit.** The refusal is unchanged and the second element flips `true`→`false`: the world is left named against a registry that cannot answer for a block it holds |
| `a_candidate_answering_for_everything_the_world_holds_replaces_the_registry` | that the replacement happens at all | `self.registry = registry` deleted — **bit, and only this one.** The guard above goes green under it, which is the vacuity it exists to close: "still the registry it had" is satisfied for good by an `adopt` that never assigns one |

Each mutation reddens exactly one guard, so the two are independent rather than
one check written twice — which is why they are separate test functions: as one
test, "the control failed while the real assertion still passed" is not something
a run can show you.

The four-line `#[cfg(test)] #[path = "mod_test.rs"] mod tests;` in
`crates/mc-sim/src/world/mod.rs` is test wiring inseparable from the file it
declares, the shape `palette.rs` already carries. Committed with the test, with the
implementer's agreement recorded rather than left to be found in a diff.

### Additional coverage

Every extra assertion below sits inside a scenario's own test as a **fixture
guard** rather than as a separate test, because each is about the fixture being
the thing the scenario describes rather than about the implementation. They are
recorded here so nobody later deletes one as noise.

| Guard | Where | What it catches |
|---|---|---|
| the section names `base:water` while no cell holds it | FR-2.2-S2 | a fixture that built a world which never held water, which makes the scenario vacuous |
| stone sits in a lower section than grass | FR-2.2-S3 | a fixture in which "found first" and "ascending" happen to agree |
| the break and the placement landed in two different cells, at the two derived aims | FR-3.1-S1 | a fixture whose two edits landed in one cell, which would assert one cell twice |
| the walked player left the spawn | FR-3.2-S1 | a comparison against a player who never moved, in which an oracle a tick short agrees |
| the falling player is airborne and not at rest | FR-3.2-S3 | a comparison taken at rest, in which "adopt zeroes velocity" is invisible |
| the save names all four shipped blocks | FR-4.4-S1, FR-3.4-S2 | a world that stopped holding a block, which shortens the comparison silently |
| the run ended `Ending::Closed` | every scenario that quits | a save that was never written, which every later reading would report as a refusal rather than as an answer |
| the candidate was admitted | every scenario about an accepted swap | a scenario whose subject never happened, reported as a refusal rather than as a wrong value |

### Two things phase 1 is barred from and no test here asks for

No test in this phase asserts a content serial, a layer index, a published
content set or a re-mesh key count. Those belong to phases 2 and 4, and a phase-1
test asserting one would either fail forever or force the trap `tasks.md` names.

No test here needs the mesher to resolve a texture through the registry.
`blocks/amber.luau` declares `texture = "base:amber"`, equal to its name, and the
fixture that builds it says why: SPEC-016's pin on the name-for-texture
substitution turns red when PRO-902 closes that gap, and that red is its success
signal.

---

## Phase 2 — Layers are appended, never renumbered, and the content is published

**15 scenarios, 16 tests.** One scenario has two tests because its two halves need
two instruments and neither covers the other; that is named below.

Test command — the five binaries these live in, by name, so a run of them is a run
of this phase and nothing else:

```
cargo nextest run -p mc-client -E 'binary(/^reload_(appends_layers|layer_budget|keeps_packed_layers|publishes_content|hud_reaches_the_frame)$/)'
```

And the whole of both crates, which is what a phase boundary runs:

```
cargo nextest run -p mc-sim -p mc-client
```

**Everything in this phase lives in `mc-client`**, because every observation is
what a *reader* was handed: which layer it packs, which serial it sees, which
layout it draws. `mc-sim`'s own suites are touched only by the adaptation.

### The adaptation window, and what it measured

`mc_sim::content::load` gains a second parameter and `Simulation::new` a third, so
the phase opens with a commit that adapts every test construction site and a tree
that does not build until the implementation lands. Measured against the tree
rather than taken from the breakdown's estimate:

| What | `tasks.md` said | Measured |
|---|---|---|
| `Simulation::new` construction sites | 24, 20 of them in test files | **24 — 22 in test files** (18 files across two crates), 2 in production (`mc-sim/src/persistence.rs`, `mc-sim/src/replay/spawn.rs`) |
| `mc_sim::content::load` call sites | not estimated | **9 — 7 in test files**, 2 in production (`mc-client/src/launch.rs`, `mc-client/src/startup.rs`) |

`cargo fmt --all --check` is clean and
`cargo clippy --workspace --all-targets --all-features -- -D warnings` reports
**nothing but** unresolved imports of the new names, three arity mismatches and one
missing field and method. No lint fired.

**The window closed with the skeleton, and everything below is measured rather than
predicted.** Two further signature groups turned up once it landed and are part of
the same adaptation: `simulation_at_launch`, `simulation_to_play` and
`simulation_for` grew the launch's published content (`Launching`, a parameter group
forced by the four-argument limit) at **14 further test sites**, and
`ResolvedContent::stating` stopped taking arbitrary pairs at **three**.

### RED, measured — 16 of 16

Against the skeleton whose `appending` renumbers from zero and whose
`Simulation::adopt` publishes nothing: **15 of 16 red on the first run, and the
sixteenth after one repair.**

FR-5.1-S4 came up **green**, and it was the one case this file predicted could:
its two packings were both of the launch's assignment, so they agreed for the wrong
reason. FR-8.1-S1 and FR-8.1-S4 being red is what identified it as that skeleton
rather than as Trap 1, and the repair is recorded under "additional coverage" — the
test now asserts the appended key **has** a layer, which is the scenario's own
premise and which a client that published nothing cannot satisfy.

Every other failure is an assertion on its own subject, not a fixture error. Three
worth recording because they say the fixtures reach what they claim to:

- **FR-5.2-S1's** expected sentence came back exactly as written — `257`, `256`,
  `256` and the relaunch clause — against a `Reading::Read` where a refusal is due.
- **FR-5.2-S2** was accepted and published the launch's four layers with **no layer
  for the new key**, which is the publication half failing while the acceptance half
  holds.
- **FR-4.5-S1's drawn half** answered `(Strayed, Strayed)`: the published frame
  strays because the publication carries the shipped layout, and **the control
  strays too**, which is the evidence the widened prediction is not vacuous. The
  device was present and both frames were really drawn.

### Seven reds outside this phase, all from one cause

`cargo nextest run -p mc-sim -p mc-client` reports **343 tests, 23 failed** — the
16 above and seven more, every one of them the skeleton's `appending` and not a
regression:

- **`stated_layers_are_honoured.rs` (2) and `client_view_of_resolved_content.rs`
  (1)** — SPEC-016's falsifiers. They asserted a non-lexicographic assignment
  through a hand-written pair list, which `stating` no longer takes, and now reach
  one through **staged appends** (`support/staged_layers.rs`): stage `n` is every key
  whose layer is at most `n`, so a key already live keeps its layer and a new one
  takes the next unspent index. **This makes them stronger than they were.** A pair
  list could state an assignment no session could hold; a staged append can only
  produce one a session can actually be in, so these three now also grade that
  `appending` reuses a live key's layer — which is exactly why they are red.
- **`documented_refusals.rs` (4)** — all four fail on the shared fixture, not on
  their own subject: `printed_refusals` cannot produce a layer-budget refusal while
  `appending` never refuses, so the error propagates before any page is compared.
  **They report nothing about the pages until `appending` is correct**, which is a
  known-red hiding an unknown one and must not be allowed to persist: if any of the
  four is still red once the implementation is in, that one is a real disagreement
  between a page and a printed refusal.

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-5.1-S1 | `crates/mc-client/tests/reload_appends_layers.rs` | `a_texture_key_declared_for_the_first_time_takes_the_first_layer_nothing_holds` |
| FR-5.1-S2 | `crates/mc-client/tests/reload_appends_layers.rs` | `a_key_no_declaration_names_any_more_leaves_every_remaining_key_on_the_layer_it_held` |
| FR-5.1-S3 | `crates/mc-client/tests/reload_appends_layers.rs` | `a_key_taken_away_and_then_declared_again_gets_a_layer_it_has_never_held` |
| FR-5.1-S5 | `crates/mc-client/tests/reload_appends_layers.rs` | `a_refused_candidate_that_would_have_needed_a_layer_leaves_the_next_one_unspent` |
| FR-5.2-S1 | `crates/mc-client/tests/reload_layer_budget.rs` | `a_candidate_needing_a_layer_past_the_budget_is_refused_naming_the_counts_and_the_way_out` |
| FR-5.2-S2 | `crates/mc-client/tests/reload_layer_budget.rs` | `a_candidate_needing_exactly_the_last_layer_the_budget_holds_is_taken_up` |
| FR-5.2-S3 | `crates/mc-client/tests/reload_layer_budget.rs` | `a_candidate_introducing_two_keys_with_one_layer_left_appends_neither_of_them` |
| FR-5.1-S4 | `crates/mc-client/tests/reload_keeps_packed_layers.rs` | `a_section_not_meshed_again_packs_the_layers_it_carried_before_a_key_was_appended` |
| FR-8.1-S3 | `crates/mc-client/tests/reload_keeps_packed_layers.rs` | `a_reader_handed_the_published_content_packs_the_layer_that_assignment_states` |
| FR-8.1-S1 | `crates/mc-client/tests/reload_publishes_content.rs` | `an_accepted_candidate_publishes_the_content_a_reader_draws_with_under_a_serial_of_its_own` |
| FR-8.1-S2 | `crates/mc-client/tests/reload_publishes_content.rs` | `a_reader_that_has_not_looked_goes_on_seeing_the_content_serial_it_last_observed` |
| FR-8.1-S4 | `crates/mc-client/tests/reload_publishes_content.rs` | `two_accepted_candidates_are_published_under_two_serials_a_reader_can_tell_apart` |
| FR-8.1-S5 | `crates/mc-client/tests/reload_publishes_content.rs` | `a_refused_candidate_leaves_the_published_content_and_its_serial_exactly_as_they_were` |
| FR-4.4-S2 | `crates/mc-client/tests/reload_publishes_content.rs` | `a_candidate_identical_to_the_content_serving_leaves_the_layers_and_publishes_a_later_serial` |
| **FR-4.5-S1** (boundary half) | `crates/mc-client/tests/reload_publishes_content.rs` | `a_widened_crosshair_declaration_reaches_the_published_layout_at_its_declared_extent` |
| **FR-4.5-S1** (drawn half) | `crates/mc-client/tests/reload_hud_reaches_the_frame.rs` | `the_layout_a_reload_published_paints_the_widened_crossbar_on_a_drawn_frame` |

**15 scenarios, 16 tests. FR-4.5-S1 has two and every other scenario has one.**

**Why FR-4.5-S1 has two, and it is the scenario's own wording rather than a
choice.** It says *publish that element at its declared `[21, 1]`* **and** *compose
the frame from it*, and the two clauses are graded by two instruments: the value
where it crosses the boundary, with no device, and a frame composed on a device
with no window. Neither covers the other — a published layout nothing composes
leaves the first green, and a frame drawn from a second read of the same root
leaves the second green while the publication carries nothing. The residue between
them, `App` assigning the published layout to its own `hud` field, is held by
review as `App`'s share of an edit already is, and no test here asks for it.

### Which caller each scenario drove through, and the two that could not use `Session`

Every FR-4, FR-5 and FR-8.1 scenario is required to drive through `Session` rather
than through `Simulation::adopt`, because a test calling the simulation's own door
is agreement between two callers of one function. **Fourteen of the sixteen tests
do**, and they read the answer back through `Session::content()` — so a client that
stopped asking for the published content reddens all fourteen.

Two reach `mc_sim::content::load` instead, and each is named here rather than left
to be discovered:

- **FR-5.2-S1** (`reload_layer_budget.rs`) — a candidate that will not fit is
  refused **where the content root is read**, before any client is offered
  anything, so there is no `Session` call to make. Its subject is the refusal's
  wording, which in this increment reaches nobody: the printing is phase 3's. The
  client-honouring half of FR-5.2 is S2, which does drive through `Session`.
- **FR-5.2-S3** (`reload_layer_budget.rs`) — the same door for the same reason. Its
  *second* half is read through the client: the session is still publishing the
  four layers a launch gave it and still has one left.

**What that leaves ungraded, stated rather than left to be found.** In this phase
the *test* is what hands the serving assignment to `load`, because the worker that
will do it in production is phase 3's. So nothing here grades that the **product**
reads the serving assignment rather than `LayerAssignment::none()`. What is graded
is everything downstream of that argument — appending, retiring, reintroducing, the
bound and the refusal. **Phase 3's T15/T16 owns the missing half** and should be
told so; the fixture door is `reload_content::candidate_against`, whose whole job is
to read the layers out of what the client publishes.

### Where every number came from

**No layer index and no serial is written as a digit in a scenario's
expectation.** The digits appear in exactly two places, both labelled:

| Number | Where it is stated | Derived how |
|---|---|---|
| the four shipped keys' layers 0–3 | `support/reload_content.rs`, `fresh_layers` | each key's position in `SHIPPED_KEYS`, which is `base:dirt`, `base:grass`, `base:stone`, `base:water` — **listed, and asserted ascending by the same function**, so a reordering fails loudly instead of silently moving every expectation at once. The four are the shipped root's texture keys because each of its four declarations states `texture` equal to `name`, verified in `content/base/blocks/` |
| the layer `base:amber` takes (4) | `reload_content::THE_NEXT_UNUSED_LAYER` | `SHIPPED_KEYS.len()` — the count a launch spends, which is also the first index nothing holds. **Never written as a `4`** |
| the layer after that (5) | `reload_appends_layers.rs` | `THE_NEXT_UNUSED_LAYER + 1` |
| 255 and 256 | `reload_layer_budget.rs`, `A_SESSIONS_BUDGET` | **`256` is written out once**, as `spec.md`'s Declared Quantities table states it, and everything else is arithmetic over it: `ALL_BUT_ONE_SPENT`, `ALL_SPENT`, `THE_LAST_LAYER_THE_BUDGET_HOLDS`, and the two counts inside the refusal's sentence. It is deliberately **not** read from `LAYERS_A_SESSION_MAY_ASSIGN`: that constant is the declaration under test, and a message assembled from it would read back whatever it became |
| the fixture sizes 255 and 256 | `reload_content::spent_all_but_one`, `spent_all` | derived **from** `LAYERS_A_SESSION_MAY_ASSIGN`, which is right for *sizing a fixture*: "one short of the budget" has to follow the budget. The assertions still hold the literal 256, so a change to the constant reddens rather than moving both sides together |
| every serial | nowhere | **no absolute serial is stated anywhere in this phase.** Every claim is a relation — moved, distinct, unchanged — and `reload_content::Run` plus `reload_publishes_content::Together` are the verdicts those relations come to. Where the counter starts is the implementation's to choose |

### A session near the end of its budget, and why the fixture is honest

`spent_all_but_one` and `spent_all` reach 255 and 256 assigned layers by appending
that many keys through `LayerAssignment::appending` — the only door into the type —
rather than by constructing one. Reaching that state organically takes hundreds of
reloads. Two properties make it the state the scenarios name rather than a
convenient value:

- the synthetic keys are namespaced `zz:`, so **every one of them sorts after all
  four shipped keys**, which leaves the shipped four on 0–3 exactly as a launch
  would and keeps every expectation arithmetic rather than read out of the subject;
- the guard counts the **keys handed in**, never the result's own `spent()`. Asking
  the value under test whether it spent what it was told to would make a broken
  bound report itself as a broken fixture, and the two scenarios that grade that
  bound would error out instead of failing.

### Weak instruments, named

- **FR-5.1-S5** (a refused candidate appends no layer) — `tasks.md` records it as a
  weak instrument and it is one. Under D6 the assignment is a value carried by the
  publication, so a refused candidate structurally cannot spend the budget: there
  is no ledger to write to, and in this phase the fixture reads the same prior for
  the second attempt because the publication never moved. **It is not evidence this
  task did work.** It is kept because it would catch a drift back to a mutable
  ledger object, and its test is shaped so that the corrected save landing on the
  layer the refused one would have taken is the observable.
- **FR-5.2-S3's second half** (the session still states 255) has the same shape for
  the same reason. Its *first* half is not weak: an `appending` that appended what
  fits and refused the rest returns a read rather than a refusal, and the
  enumerated `Reading` verdict rejects that by shape.

### The vacuity these scenarios are shaped against, and how each closes it

Phase 1's finding was that an observation taken through a channel the operation
does not update cannot see the operation. `PublishedContent` is a **second**
`ArcSwap`, and three of this phase's scenarios are satisfied by a client that
publishes nothing at all. Each carries its discriminating half **in the same run**:

| Scenario | Vacuous under | The half in the same test that closes it |
|---|---|---|
| FR-8.1-S2 | a client that never publishes: "the serial it last observed" is trivially still that | one accepted candidate afterwards has to move what a **fresh** ask returns, while the `Arc` the earlier reader holds keeps its own value |
| FR-8.1-S5 | the same: a refusal "leaves the content exactly as it was" forever | the corrected save afterwards has to move it |
| FR-4.4-S2 | an implementation that noticed the content was identical and skipped the attempt | the serial has to be **later**, so a skipped attempt fails |
| FR-8.1-S1 | a publisher that publishes the content but never moves the serial | `Together` rejects `NothingMoved` by name, and FR-8.1-S4 is the paired instrument for a serial that moves once and sticks |
| FR-4.5-S1 (boundary) | a client publishing no layout at all | the extent standing **before** the reload is in the same comparison |
| FR-4.5-S1 (drawn) | a prediction covering no pixel, or two frames that were never going to differ | the shipped layout's frame must **stray** on the widened prediction, and a fixture guard ahead of both requires the widened footprint to be strictly larger |
| FR-5.1-S4 | a packer that moved the layer both before and after together | the two readings are compared against the same derived expectation **and** against each other |
| FR-8.1-S3 | a reader that happened to agree with a sort | the same test asserts the key sorts **first** among everything the session states while holding the **highest** layer |

### Additional coverage

| Test or guard | Where | What it catches |
|---|---|---|
| the fourth printed refusal | `crates/mc-client/tests/documented_refusals.rs`, `over_the_layer_budget` | a second witness on the layer-budget refusal, through the **shipped reporting chain** rather than through the error value — and the coupling phase 6's `docs/modding/hot-reload.md` needs before it may quote the sentence. Produced differently from the three beside it, and it has to be: those are refusals a *launch* meets and each names the root it was given, while a budget can only be exhausted by a session that has already spent it — and the refusal names counts rather than a path, so the fixture-root rewrite has nothing to do |
| `SHIPPED_KEYS` asserted ascending | `reload_content::fresh_layers` | a reordering of the list every expectation in the phase is derived from |
| the key count handed to `appending` | `reload_content::assigning` | a fixture that assembled fewer keys than the session it is named for, which would make the budget scenarios about a different session |
| the crossbar's footprint grew | `reload_hud_reaches_the_frame::require_a_wider_footprint` | a widened declaration landing on the pixels the shipped one already covered, which leaves the drawn half's control unable to stray |
| `spent` beside the live map | `reload_content::Publishing`, read by FR-5.1-S2, FR-5.1-S3, FR-5.2-S2 and FR-5.2-S3 | an implementation deriving the count of layers spent from the **live** entries. It answers one short the moment a key retires, which is precisely what hands a reintroduced key back the layer it used to hold — asserting the shared derivation directly localises that to one wrong number instead of several wrong layers downstream |
| the appended key **has** a layer | FR-5.1-S4, added after it arrived green | a client that published nothing on a reload. Both of that scenario's packings are then of the launch's own assignment and agree for the wrong reason — a layer that was never appended cannot renumber anything. It is the scenario's own premise ("after a candidate appended layer 4") asserted rather than assumed |
| a layer set with a gap in it is refused by name | `staged_layers::require_dense` | a falsifier quietly graded against an assignment staged appends cannot reproduce. Stated as a fixture guard **and** as the one thing that would be a finding about D9's constructor rule rather than something to work around here |

### Fixtures this phase owns

| File | What it is |
|---|---|
| `crates/mc-client/tests/support/reload_content.rs` | **New.** The layers a launch assigns and the arithmetic over them, `Publishing` and `Run`, the two near-budget assignments, `candidate_against` (which reads the serving assignment, as a reload's build stage does), the `Reading` verdict over the build door, and the widened crossbar declaration. Reached by `#[path]` for phase 1's reason: it names types the implementation has not written |
| `crates/mc-client/tests/support/reload_world.rs` | Gains `playing_serving`, so a scenario can start a client whose session has already spent layers. `playing` now states `LayerAssignment::none()` at the call, which is what makes "a launch has spent nothing" visible there |
| `crates/mc-client/tests/support/reload.rs` | `candidate` states `LayerAssignment::none()` and says why: the scenarios it serves read no layer index. A scenario that *is* about a layer goes through `candidate_against` |
| `crates/mc-client/tests/support/input/mod.rs` | Gains `content()` and nothing else — forwarded, deciding nothing, so a reader observes by asking |
| `crates/mc-client/tests/support/hud_frames.rs` | Gains `hud_published`, which composes a frame from a layout **somebody else read**. A frame drawn from a second read of the same root would agree with the fixture by construction while the publication carried nothing |
| `crates/mc-client/tests/support/staged_layers.rs` | **New.** Reaching a deliberately non-lexicographic assignment through the only constructors there are, derived from the same table the expectation comes out of. It replaces the pair lists SPEC-016's two falsifiers used and is what makes them assert a state a session can be in |
| `crates/mc-sim/tests/support/launch.rs` | Gains `launching(registry, accepting)` — the `Launching` group a launch takes, with the acceptance stated at the call because several scenarios are about it and the content not, because a launch having spent no layers is a fact |
| `crates/mc-client/tests/support/reload_save.rs` | `relaunch` publishes the **root's own** content, read through the one door rather than assembled from the registry beside it: a relaunch really does read a content root |
| `crates/mc-client/tests/support/mod.rs`, `crates/mc-sim/tests/support/mod.rs`, `crates/mc-sim/tests/support/chamber.rs` | Gain a `published_content` that resolves what a reader receives out of a registry. Three copies of ten lines, and the duplication is the `#[path]` fixture layout's price: `support/input/world.rs` and `support/persistence.rs` are reached by path so a binary need not pull in every other fixture, and eight of the binaries that use them do not declare `mod support;` at all |

### Two things phase 2 is barred from, and no test here asks for either

**No test in this phase asserts that a layer reached a device, or that a re-mesh
key was marked.** Phase 2 publishes a value; phase 4 is what makes a device see it.
The one test here that touches a device reads the **HUD**, which is a layout and not
an array-texture layer, and it composes it through the client's own frame call
rather than uploading anything.

**Trap 1 was verified directly rather than inferred.** At the head this phase
opened on, `Simulation` holds exactly one `ArcSwap` and no `content` field;
`mc_sim::content::load` still had its one-argument signature; `resolved_from`'s
derivation was untouched; and `crates/mc-sim/src/world/reload.rs` names `mark_dirty`
nowhere. Phase 1 did not spring it.

---

## Phase 3 — A saved edit becomes one attempt, built off the tick thread, and a refusal is stated once

**26 scenarios, 27 tests, plus 2 under additional coverage.** One scenario has two
tests because its two halves need two instruments and neither covers the other
(FR-1.1-S4, below) — the same shape phase 2's FR-4.5-S1 has.

Test command — the nine binaries these live in, by name, so a run of them is a run
of this phase and nothing else:

```
cargo nextest run -p mc-client -p mc-world -E 'binary(/^(reload_begins_one_attempt|reload_reads_only_declarations|reload_watches_a_real_directory|reload_builds_off_the_tick|reload_attempts_follow_one_another|reload_refuses_a_broken_declaration|reload_refuses_the_whole_root|reload_refusal_ends_one_attempt|content_watch)$/)'
```

And the whole of the three crates, which is what a phase boundary runs:

```
cargo nextest run -p mc-sim -p mc-client -p mc-world
```

### The window nothing compiles in, and how wide it is

The five names phase 3 adds — `mc_world::content::watch`, `mc_sim::reload::ContentReload`,
`ReloadStep`, `watching_shipped_content` and `mc_client::session::reload::ReloadReport` —
do not exist yet, so every binary naming them is a compile error rather than a red
assertion. Two of them are named by `support/input/mod.rs`, which **every binary that
drives the client includes**, so this window is wider than phase 1's: the nine
binaries above plus every other one that says `mod support;` and includes the input
harness. That is unavoidable — `attach_reload` and `take_reload_report` are the
client's own doors and the harness is the only thing that may call them.

`cargo clippy --workspace --all-targets --all-features -- -D warnings` reports
**nothing but** those five unresolved names (six `E0432`, two `E0603` for the private
child module, two `E0599` for the two methods). No lint fired.
`cargo fmt --all --check` is clean.

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-1.1-S1 | `crates/mc-client/tests/reload_watches_a_real_directory.rs` | `a_declaration_saved_on_disk_while_the_client_is_running_begins_one_attempt` |
| FR-1.1-S2 | `crates/mc-client/tests/reload_begins_one_attempt.rs` | `a_declaration_file_that_appears_where_the_loader_reads_begins_one_attempt` |
| FR-1.1-S3 | `crates/mc-client/tests/reload_begins_one_attempt.rs` | `a_declaration_file_deleted_from_where_the_loader_reads_begins_one_attempt` |
| **FR-1.1-S4** (the domain's coalescing) | `crates/mc-client/tests/reload_begins_one_attempt.rs` | `saves_that_reach_one_tick_boundary_begin_one_attempt_and_not_five` |
| **FR-1.1-S4** (the window at the boundary) | `crates/mc-world/tests/content_watch.rs` | `the_window_a_watch_hands_its_debouncer_is_the_declared_settling_window` |
| FR-1.1-S5 | `crates/mc-client/tests/reload_reads_only_declarations.rs` | `a_hud_declaration_written_under_the_same_root_begins_an_attempt` |
| FR-1.1-S6 | `crates/mc-client/tests/reload_begins_one_attempt.rs` | `a_root_nothing_has_changed_under_begins_no_attempt_however_many_ticks_pass` |
| FR-1.1-S7 | `crates/mc-client/tests/reload_reads_only_declarations.rs` | `an_editors_scratch_file_beside_a_declaration_begins_no_attempt` |
| FR-1.1-S8 | `crates/mc-client/tests/reload_watches_a_real_directory.rs` | `a_content_root_that_cannot_be_watched_is_reported_once_and_the_run_carries_on` |
| FR-1.1-S9 | `crates/mc-client/tests/reload_reads_only_declarations.rs` | `a_material_file_begins_no_attempt_while_the_same_watcher_begins_one_for_a_declaration` |
| FR-1.2-S1 | `crates/mc-client/tests/reload_builds_off_the_tick.rs` | `a_candidate_built_after_one_declaration_changed_carries_the_whole_root` |
| FR-1.2-S2 | `crates/mc-client/tests/reload_builds_off_the_tick.rs` | `the_ticks_a_candidate_is_built_over_put_the_player_where_a_run_with_no_reload_would` |
| FR-1.2-S3 | `crates/mc-client/tests/reload_refuses_the_whole_root.rs` | `a_declaration_that_loops_is_refused_naming_its_file_while_the_simulation_advances` |
| FR-1.2-S4 | `crates/mc-client/tests/reload_attempts_follow_one_another.rs` | `a_change_reported_while_a_candidate_is_being_built_begins_exactly_one_further_attempt` |
| FR-1.2-S5 | `crates/mc-client/tests/reload_attempts_follow_one_another.rs` | `a_builder_that_ends_without_a_candidate_is_reported_once_and_the_next_change_still_lands` |
| FR-1.3-S2 | `crates/mc-client/tests/reload_builds_off_the_tick.rs` | `every_tick_a_candidate_is_built_over_is_answered_by_the_content_in_force` |
| FR-1.3-S3 | `crates/mc-client/tests/reload_attempts_follow_one_another.rs` | `a_change_reported_before_any_tick_is_held_until_a_tick_boundary_exists` |
| FR-2.1-S1 | `crates/mc-client/tests/reload_refuses_a_broken_declaration.rs` | `a_declaration_that_will_not_compile_leaves_the_content_serving_and_names_the_file` |
| FR-2.1-S2 | `crates/mc-client/tests/reload_refuses_a_broken_declaration.rs` | `a_misspelled_field_leaves_the_content_serving_and_names_the_file_block_and_field` |
| FR-2.1-S3 | `crates/mc-client/tests/reload_refuses_a_broken_declaration.rs` | `two_files_claiming_one_block_leave_the_content_serving_and_name_both_in_file_name_order` |
| FR-2.1-S4 | `crates/mc-client/tests/reload_refuses_a_broken_declaration.rs` | `a_blocks_directory_emptied_of_declarations_leaves_the_content_serving_and_names_the_root` |
| FR-2.1-S5 | `crates/mc-client/tests/reload_refusal_ends_one_attempt.rs` | `a_refused_candidate_leaves_the_blocks_the_layers_and_the_block_in_the_hand_alone` |
| FR-2.1-S6 | `crates/mc-client/tests/reload_refusal_ends_one_attempt.rs` | `a_corrected_declaration_is_taken_up_by_the_next_attempt_after_a_refusal` |
| FR-2.1-S7 | `crates/mc-client/tests/reload_refuses_the_whole_root.rs` | `a_refused_hud_declaration_refuses_the_block_declarations_beside_it` |
| FR-2.1-S8 | `crates/mc-client/tests/reload_refusal_ends_one_attempt.rs` | `five_saves_meeting_one_refusal_are_reported_once_and_the_next_one_that_differs_is_reported` |
| FR-2.3-S1 | `crates/mc-client/tests/reload_refuses_the_whole_root.rs` | `a_declaration_that_allocates_past_the_memory_cap_is_refused_naming_its_file` |
| FR-2.3-S2 | `crates/mc-client/tests/reload_refuses_the_whole_root.rs` | `a_declaration_that_raises_is_refused_naming_its_file_and_what_it_raised` |

**26 scenarios, 27 tests, each scenario at least once and only FR-1.1-S4 twice.**

### FR-1.1-S4's two instruments, and why neither covers the other

`tasks.md` Trap 5 and architecture D10 require both, and they are in different
crates because they are different questions.

- **The domain's coalescing** — `saves_that_reach_one_tick_boundary_begin_one_attempt_and_not_five`,
  driven through the in-memory double: five saves that reach one tick boundary
  arrive as one report carrying five paths and begin **one** attempt. This is an
  assertion about `ContentReload` rather than agreement with the double, because the
  double holds no policy of its own — it hands over exactly what it was given.
- **The window at the boundary it crosses** —
  `the_window_a_watch_hands_its_debouncer_is_the_declared_settling_window`, which
  asks the adapter which `Duration` it handed the builder. **No filesystem and no
  timer.** Its falsifiability is carried inside the same comparison: a second
  adapter built with `Duration::ZERO` must report zero (an adapter answering from a
  constant rather than from what it was handed says 150 ms to both), and
  `SETTLING_WINDOW` is compared against the literal 150 ms so the pair cannot agree
  at zero.

**Neither covers the other**, and the arithmetic is why: boundaries are about 16 ms
apart and the window is 150, so five saves genuinely spread across one window reach
five *different* boundaries. The coalescing test cannot see that, and the window test
cannot see coalescing.

**The residue the window test does not close, stated rather than left to be found:**
an adapter that records the window it was given and passes the builder a different
literal. That takes two spellings at one call site, and the parameterised door
(`settling_for`) is what makes the shipped one (`watching`) the only place the
declared window is supplied. Held by review.

### Which caller each scenario drove through

**All 26 drive through `Session`**, via the input harness, except the one scenario
half whose subject is the adapter's own construction (FR-1.1-S4's window half, which
never builds a client at all). That is deliberate and it is this phase's answer to
the architecture's standing risk — *the client stops reacting to a published reload
and nothing reddens*: nothing here calls `ContentReload::at_tick_boundary` itself, so
**a `Session::tick` that stopped driving the reload reddens 25 of the 26.** A test
that drove the reload directly would be agreement between two callers of one
function.

Two consequences worth stating:

- The count of attempts is read through `Session::take_reload_report()`, never
  through an accessor for "a build is in flight". There is no such accessor, and
  asking for one would move the count these scenarios are about inside the value
  under test.
- The refusal text is read where it crosses out of the client's core. **The printing
  is `App`'s and is held by review**, exactly as `report_remesh` and `report_swatch`
  already are: `crates/mc-client/src/app.rs` needs a real window and nothing in this
  workspace constructs one. The halves either side of it are covered — the value at
  the boundary here, and `refusals_state_a_cause_once.rs`'s existing assertions on how
  a chain is rendered.

### The weak instrument, named

- **FR-1.1-S6** (nothing changed → no attempt, however many ticks) — `tasks.md`
  names it and it is one: a watcher that never fires satisfies it, and **it was one of
  only two scenarios green under an inert skeleton.** It is not evidence this task did
  work. What it would catch is a drive that read the root on every boundary, and it
  carries the tick count beside the attempt count so a client that stopped advancing
  cannot satisfy it either. Its discriminating partners are FR-1.1-S1 (a real save
  through the real adapter) and FR-1.1-S9 (an irrelevant path and a relevant one
  through one instrument).

### The premise every "while a candidate is being built" scenario carries

FR-1.2-S1, FR-1.2-S2 and FR-1.3-S2 each call `require_a_build_in_flight`, which
**refuses** unless at least one boundary was crossed between the change and the
outcome. A build that ran on the tick thread reports at the boundary that started
it, so there is no such boundary and every assertion about those ticks would hold
over none of them. Measured against a deliberately synchronous skeleton: all three
fail on that refusal rather than passing vacuously.

The drive's *order* is what makes such a boundary certain rather than likely — a
boundary collects a finished build before it starts a new one — so this is a premise
the design guarantees rather than a race the test hopes to win.

### Where every number came from

| Number | Where it is stated | Derived how |
|---|---|---|
| the five saves in one window | `reload_begins_one_attempt::SAVES_IN_ONE_WINDOW` | the scenario's own number; what matters is that it is more than one |
| the five saves meeting one refusal | `reload_refusal_ends_one_attempt::SAVES_MEETING_ONE_REFUSAL` | the same |
| 150 ms | `content_watch::A_SAVE_SETTLES_FOR` | written out once, as `spec.md`'s Declared Quantities table states it, and deliberately **not** read from `SETTLING_WINDOW` — that constant is what is under test on the other side of the comparison |
| the layer `base:amber` takes | `reload_content::THE_NEXT_UNUSED_LAYER` (phase 2's) | `SHIPPED_KEYS.len()`, never written as a digit |
| the four shipped blocks and their solidity | `reload_watch::the_four_shipped_blocks` | listed rather than read back, for the reason `support/reload.rs` lists the names: a fixture that discovered them would go on passing over a root that had stopped declaring one. Water is the one shipped block its own declaration calls not solid |
| 300 boundaries for "no attempt" | `reload_watch::BOUNDARIES_WITHOUT_AN_ATTEMPT` | not a measurement: far more boundaries than a build needs to begin and end, so an implementation that started one would have reported it many times over |
| 2 000 quiet boundaries end a counting run | `reload_watch::QUIET_BOUNDARIES_THAT_END_A_RUN` | the same reasoning in the other direction: any further attempt would have to begin *and* end inside that stretch |
| the 15 s wait and the ten settling windows | `reload_watches_a_real_directory` | the wait is generous rather than derived; the post-attempt window is `SETTLING_WINDOW * 10`, derived from the declared window because a second attempt could only follow a second debounced report |
| the walk's turn per tick | `reload_builds_off_the_tick::TURN_PER_TICK` | 20 raw counts x 0.0022 rad/count = 0.044 rad per tick, against a walk of 4.5 blocks/s over a 60 Hz tick = 0.075 blocks per tick, so the circle's radius is 0.075 / 0.044 ~ **1.7 blocks** — the walk stays inside the floor for as many boundaries as the build takes |
| every refusal's wording | nowhere | asked of a second read of the same root through `the_loaders_own_words`, and the reported text has to **end** in it. The scenario-specific needles (the file, the block, `slid`, both files, the root path, the raised sentence) are the fixture's own inputs, never the subject's output |
| every tick count | nowhere | asserted as a difference across the run, never as an absolute |

### Arrival colours, measured

**27 of 29 red, 2 green.** Measured against a scratch skeleton the test author stood
up in order to typecheck the suite and **reverted by hand** — `git diff --exit-code`
clean on every `src/` tree afterwards, `cargo fmt --all --check` clean. So these
colours are against *that* skeleton and not against the implementation's; what they
establish is that every test reaches its assertion and fails on its own subject.

Green, both named in advance:

- **FR-1.1-S4's window half** — green the moment the constructor exists, which is the
  same class as phase 6's structural scans: its falsifiability is the zero reading
  inside the same comparison rather than a red step.
- **FR-1.1-S6** — the weak instrument above, exactly as `tasks.md` predicts.

Three fixture defects that round found, all the same shape and all repaired: **the
author's edit has to happen after the client has launched.** In the first draft of
FR-1.1-S5, FR-1.1-S7 and FR-2.1-S7 the root was edited *before* the client played it,
so the client was already serving the edited content — FR-2.1-S7 would then have been
**red against a correct implementation** (asserting stone still solid while the client
had launched with it non-solid), and S5's published extent would have been the widened
one for the wrong reason. That is the over-tight-assertion trap arriving through a
fixture rather than through an expectation.

### The closing condition on this phase, measured

`tasks.md`'s ruling: phase 3 does not close until something goes red when the product
reads `LayerAssignment::none()` in place of the serving assignment.

**It is expressible, and it is measured.** The instrument is
`reload_builds_off_the_tick::a_build_the_client_drove_reads_the_layers_the_session_has_already_spent`,
which drives a reload through `Session` after the author declares a block introducing
a texture key the session has never assigned, and reads the published assignment back
through `Session::content()`.

| Mutation | Outcome |
|---|---|
| the build stage passes `LayerAssignment::none()` to `load` | **red**, naming the defect: `{amber 0, dirt 1, grass 2, stone 3, water 4}` against `{dirt 0, grass 1, stone 2, water 3, amber 4}` — every layer renumbered, which is every packed vertex already on the GPU sampling the wrong texture |
| the same mutation, blast radius | **a second test reddens that nobody planned**: FR-2.1-S6's corrected candidate expects `base:amber` on the next unused layer, so it disagrees too. Two independent witnesses |

**Why four unchanged keys cannot express it:** appending the shipped four to an empty
assignment produces the identical pair list, so the two arguments agree. Only a key
the session has not assigned separates them, which is why this test introduces one.

### Additional coverage

| Test | Where | What it catches |
|---|---|---|
| `a_build_the_client_drove_reads_the_layers_the_session_has_already_spent` | `reload_builds_off_the_tick.rs` | the closing condition above: a build stage that reads a fresh assignment rather than the serving one. Mapped to no scenario — phase 2's own record states that nothing there grades the argument, and no phase-3 scenario asks about a layer |
| `every_declaration_the_shipped_root_holds_is_content_and_nothing_else_under_it_is` | `crates/mc-world/tests/content_watch.rs` | a relevance rule that narrowed or widened, graded against the **shipped root's own listing** rather than a list written in a test: a rule keyed on the extension alone claims the material files, and one that lost a directory disclaims declarations the loader reads. It carries the premise of its own negative half — a root holding nothing but declarations is refused rather than passed |

Every other extra assertion sits inside a scenario's own test as a fixture guard.
They are recorded here so nobody later deletes one as noise.

| Guard | Where | What it catches |
|---|---|---|
| at least one boundary passed with a build in flight | FR-1.2-S1, FR-1.2-S2, FR-1.3-S2 | a synchronous build, under which every assertion about "the ticks a candidate is built over" holds over no ticks at all |
| the oracle left the spawn | FR-1.2-S2 | a comparison against a player who never moved, in which an oracle a tick short agrees |
| the shipped root holds a file no loader reads | the relevance totality test | an absence asserted over an empty set |
| the root declares the file about to be broken, restated, corrected or widened | every refusal fixture | a root that *gained* a declaration rather than one whose declaration an author edited |
| the tick count moved by the boundaries crossed | FR-1.1-S6, FR-1.1-S8, FR-1.2-S3 | a client that stopped advancing, which satisfies "no further attempt" perfectly |
| the four blocks still serving | every refusal scenario | a refusal that lost the content, which is worse than the broken file |

### Overlaps, named

- **FR-1.2-S1 and FR-1.1-S4 share their effect half.** Both restate `stone.luau` and
  read the content the client then serves. S4's subject is the *count* of attempts;
  S1's is what the candidate carried, and it reads the whole published content —
  blocks and the layer assignment — because four live keys is the whole root's answer
  and one is the changed file's. The scenarios themselves overlap and one test each is
  the contract.
- **FR-2.1-S5 and FR-2.1-S6 share the layer `base:amber` would take.** S5 asserts a
  refusal did not spend it; S6 asserts the corrected candidate takes *that same*
  layer. Read together they are what says a refused candidate cannot leak the
  session's budget — and they are also the second witness on the closing condition.

### Fixtures this phase owns

| File | What it is |
|---|---|
| `crates/mc-client/tests/support/reload_watch.rs` | **New.** The `ContentWatch` double and the handle a scenario reports on, the clients that watch the root they play, `Attempt` and the two drivers over tick boundaries, the `Refusal` verdict and the loader's own words to judge it against, the three broken-declaration texts, and what a client is serving. **507 non-blank lines against the 600 limit** — the next addition splits it by responsibility, and the seam is the obvious one: what drives a run, and what judges a refusal |
| `crates/mc-client/tests/support/input/mod.rs` | Gains two forwards and nothing else: `attach_reload` and `take_reload_report`. Both decide nothing, for the reason every method there is one call — a harness that drove the reload itself would be the only thing left calling it |

The double is a `std::sync::mpsc` channel drained one report per ask. **One report per
ask is what lets a scenario say which boundary sees which report**, and a burst that
arrived between two boundaries is therefore spelled as one report carrying several
paths — which is what the port's own vocabulary says a change report is.

### Two things phase 3 is barred from, and no test here asks for either

**No test in this phase asserts a re-mesh key, a drawn frame or a cleared player.**
Which sections a reload marks is phase 4's, and moving a player out of a cell that
became solid is phase 5's. A phase-3 test asserting either would fail forever or hand
a later phase a scenario green on arrival.

**`documented_refusals.rs` is not extended here, and that is a deviation from T16's
file list.** That suite asserts page → run: every refusal a *page quotes* is one the
client prints. `docs/modding/hot-reload.md` does not exist yet — it is T26's — so an
extension now would add a printed refusal nothing quotes, which is a test that cannot
fail. What T26 owes is the page **and** the entry in `printed_refusals` that couples
it to a real run; the reload's wrapper sentence, `BuilderLost`'s and
`RootUnwatchable`'s are the three new sentences on the page's side of that coupling.

### Correction after the implementation landed — a run's patience is time, not boundaries

**Annotated rather than rewritten.** The two rows above naming
`BOUNDARIES_WITHOUT_AN_ATTEMPT` (300) and `QUIET_BOUNDARIES_THAT_END_A_RUN` (2 000)
record what was believed when the tests were authored, and they were wrong in both
directions. The constants are gone; what replaced them is below. The colours recorded
above stand as what was measured against the scratch skeleton at `0c282d9`.

**Verdict on the dispute: `test-wrong`, on the fixture and not on any expectation.**
Twenty-one of the twenty-seven failed for one cause, deterministically, and the
implementer's measurements are the record:

- a candidate build is **0.7–1.7 ms** (`mc_sim::content::load` over the shipped root,
  release, five rounds), observable through `is_finished` after about a millisecond;
- **2 000 tight-loop boundaries cost ~0.2 ms** — a `tick()` in a test is ~100 ns,
  against a boundary in the shipped client which is a rendered frame at ~16 ms;
- so the fixture gave up an order of magnitude before the thing it was waiting for
  could happen, and a blocking collect made 64 of 64 pass — which is what proved the
  implementation right and the fixture wrong.

**A boundary count is a proxy for elapsed time whose conversion factor differs by
five orders of magnitude between a test loop and the client.** That is the general
defect, and it had a second instance nobody had asked about: **the runs that expect
*no* attempt were vacuous for the same reason.** Three hundred tight boundaries span
30 µs, so an implementation that wrongly began an attempt on an editor's scratch file
or a material file would not have finished the build before the run ended, and
FR-1.1-S6, FR-1.1-S7, FR-1.1-S8, FR-1.1-S9's first half and FR-1.3-S3's first half
would all have reported "no attempt began" over a defect that had begun one. They
passed against a correct implementation for the right answer and the wrong reason.

**What replaced both, and where the numbers come from now:**

| Quantity | Where | Derived how |
|---|---|---|
| how long a run waits for an attempt to end, and how long a quiet stretch must be before nothing more is coming | `reload_watch::long_enough_for_an_attempt_to_end` | `SETTLING_WINDOW * 2`. **Denominated in the declared window so it has no number of its own**: the window is the dominant term in the spec's one-second target, so a build that outlasted one would have blown that budget long before a fixture noticed. A 300 ms patience against a 1.7 ms build is a margin of ~175x, and it moves with the declared quantity rather than beside it |
| how long before a wedged run gives up | `reload_watch::before_giving_up` | `SETTLING_WINDOW * 40`, the same unit |
| how long a boundary waits for the next | `reload_watch::BETWEEN_QUIET_BOUNDARIES` | 1 ms, which is what makes the two bounds above durations rather than boundary counts in disguise. The boundary count now falls out of the duration instead of standing in for it |
| how many boundaries a quiet run crosses | nowhere | it is whatever the run crossed, and the two scenarios that compare a tick count compare against `crossed.len()` |

The three loops that carry a script of their own — the walking lockstep, the
support reading, and the real-filesystem wait — take their patience from the same
module through `may_cross_another` and `pause_between_boundaries`, so there is one
statement of it rather than four.

**Measured after the correction, against the implementation: 29 of 29 pass, three
consecutive runs, 2.5 s wall each.** `cargo nextest run -p mc-sim -p mc-client
-p mc-world` is **682 of 682**. `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets --all-features -- -D warnings` are clean.

### What FR-1.2-S2 does not grade, and it is not what its wording suggests

**Recorded because the next reader will assume the opposite, and because it was
measured rather than argued.** The implementer replaced the `is_finished` poll with an
unconditional `join()` — a collect that *blocks the tick* on the worker — and **every
scenario in this phase passed, FR-1.2-S2 included.**

The reason, once stated, is the channel-blindness family again: **FR-1.2-S2's
observable is where the ticks put the player and its property is when the ticks
happened.** A blocking collect changes the second and not the first — the same
inputs over the same number of ticks put the player in the same place whether or not
the tick thread waited on the way. So:

- **FR-1.2-S2 grades that a build does not change the player's progress. It does not
  grade that the build ran off the tick thread.**
- `require_a_build_in_flight` grades the weaker property that a build does not
  *complete* inside the boundary that started it. A blocking collect satisfies it,
  because the drive's own order still reports at the following boundary.
- **There is no clock-free observable of a stalled tick.** The difference is purely
  temporal, and `spec.md` deliberately keeps a wall-clock assertion out of the gate
  (FR-9's own reasoning: a flaky latency test teaches everybody to re-run it).

**The one candidate discriminator, with its cost, for whoever owns FR-9.1-S1.** When
boundaries are cheaper than a build, a *polled* collect spans many boundaries with the
build in flight and a *blocking* one spans exactly one — so "more than one boundary in
flight" separates them. It is a timing assertion wearing a count's clothing: it holds
by four orders of magnitude on this machine and would go red on one where a tick costs
a millisecond. It is **not** taken here, and pacing the fixture removes it in any case.
FR-9.1-S1's wording is the same shape as FR-1.2-S2's and is open to the same objection;
it belongs to phase 6's brief and to the team lead, not to a fixture change here.

---

## Phase 4 — What a reload re-meshes, and against which content

**12 scenarios, 13 tests.** One scenario has two tests because its two halves need
two instruments and neither covers the other (FR-4.3-S2, below) — the same shape
phase 2's FR-4.5-S1 and phase 3's FR-1.1-S4 have.

Test command — the seven binaries these live in, by name, so a run of them is a run
of this phase and nothing else:

```
cargo nextest run -p mc-client -E 'binary(/^reload_(marks_sections|uncovers_culled_faces|supersedes_a_batch_in_flight|meshes_against_the_content_serving|appends_a_drawable_layer|draws_the_new_block|hand_shows_the_new_block)$/)' --test-threads 2
```

And the whole of the three crates, which is what a phase boundary runs:

```
cargo nextest run -p mc-sim -p mc-client -p mc-world --test-threads 2
```

`--test-threads 2` because two of these binaries hold a device, and
`docs/technical/testing.md` records a driver abort at nextest's default parallelism
with nine devices live at once.

**Everything in this phase lives in `mc-client`**, because every observation is
either what the client was left to mesh, what its own report handed the frame path,
or what a device drew from the two. `mc-sim`'s suites are untouched.

### The window nothing compiles in, and how wide it is

Seven names phase 4 adds do not exist yet — `Remeshed::{Scene, Superseded, Failed}`
as a three-armed enum, `Remesher::spawn`'s second argument, `Remesher::retire`,
`Session::mark_for_remesh`, `Retained` without its `registry`,
`ReloadReport::Accepted`'s `layers`, and `remesh` without a registry parameter — so
every binary naming one is a compile error rather than a red assertion. One of them
is named by `support/input/mod.rs`, which **every binary that drives the client
includes**, so this window is as wide as phase 3's: the seven binaries below plus
every other one that says `mod support;` and includes the input harness. That is
unavoidable — `mark_for_remesh` is the client's own door and the harness is the only
thing that may call it.

`cargo clippy --workspace --all-targets --all-features -- -D warnings` reports
**nothing but** those seven unresolved names (twelve `E0599`, three `E0061`, one
`E0063`, one `E0026`). No lint fired. `cargo fmt --all --check` is clean.

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-6.1-S1 | `crates/mc-client/tests/reload_marks_sections.rs` | `a_candidate_touching_neither_solidity_nor_a_texture_key_leaves_no_section_to_mesh` |
| FR-6.1-S2 | `crates/mc-client/tests/reload_marks_sections.rs` | `a_candidate_taking_stones_solidity_away_leaves_every_section_of_the_world_to_mesh` |
| FR-6.1-S3 | `crates/mc-client/tests/reload_marks_sections.rs` | `a_candidate_changing_all_four_declarations_leaves_each_section_of_the_world_once` |
| FR-6.1-S4 | `crates/mc-client/tests/reload_marks_sections.rs` | `a_reload_that_changes_no_geometry_and_one_that_does_are_told_apart_on_one_instrument` |
| FR-4.1-S3 | `crates/mc-client/tests/reload_uncovers_culled_faces.rs` | `a_face_culled_against_solid_stone_is_drawn_once_stone_has_stopped_being_solid` |
| FR-6.2-S1 | `crates/mc-client/tests/reload_supersedes_a_batch_in_flight.rs` | `a_batch_meshed_against_content_that_stopped_serving_is_discarded_and_the_next_one_is_drawn` |
| FR-6.2-S3 | `crates/mc-client/tests/reload_supersedes_a_batch_in_flight.rs` | `a_batch_that_cannot_be_packed_is_reported_and_the_worker_still_draws_the_next_one` |
| FR-6.2-S4 | `crates/mc-client/tests/reload_supersedes_a_batch_in_flight.rs` | `a_client_that_discards_a_batch_leaves_the_sections_it_would_have_meshed_waiting` |
| FR-6.2-S2 | `crates/mc-client/tests/reload_meshes_against_the_content_serving.rs` | `every_section_a_reloads_batch_covers_is_meshed_against_the_content_now_serving` |
| FR-6.2-S5 | `crates/mc-client/tests/reload_meshes_against_the_content_serving.rs` | `a_break_after_a_reload_is_meshed_against_the_block_the_new_content_left_behind` |
| **FR-4.3-S2** (the value at the boundary) | `crates/mc-client/tests/reload_appends_a_drawable_layer.rs` | `a_placement_of_a_newly_declared_block_is_packed_from_the_layer_that_reload_appended` |
| **FR-4.3-S2** (drawn on a device) | `crates/mc-client/tests/reload_draws_the_new_block.rs` | `a_placement_of_a_newly_declared_block_is_drawn_from_the_texels_its_layer_was_filled_with` |
| FR-4.3-S3 | `crates/mc-client/tests/reload_hand_shows_the_new_block.rs` | `the_held_block_indicator_draws_the_new_block_from_the_layer_the_reload_appended` |

**12 scenarios, 13 tests. FR-4.3-S2 has two and every other scenario has one.**

**Why FR-4.3-S2 has two, and it is the scenario's own wording rather than a
choice.** It says *append a layer for that key, **fill it**, and **draw** a placement
of `base:amber` from that layer* — three clauses across two instruments. That the
report hands over an assignment the packer writes the right index from is assertable
with no device; that the layer was *filled* and that the block samples *it* needs a
real one. Neither covers the other: a correct assignment written into every corner
of a scene nobody uploaded draws from whatever the allocator left in that layer, and
a frame drawn from layers a fixture assembled says nothing about what the report
carried.

### Which caller each scenario drove through

**All 13 drive through `Session`.** Ten hand the client a candidate through
`Session::adopt_content` (the client's own door, via `InputHarness::adopt`); three —
FR-4.3-S2's two halves and FR-4.3-S3 — go through the **watcher**, because their
subject is what `ReloadReport::Accepted` carries and only a driven
`ContentReload` produces one. Nothing here calls `Simulation::adopt` or
`ContentReload::at_tick_boundary` itself, so a `Session` that stopped driving either
reddens the whole phase.

Two readings are taken through doors that are the client's but not `Session`'s, and
each is named here rather than left to be discovered:

- **`Remesher`** (FR-6.2-S1, S3, S4) — the staleness comparison is the client's, made
  at collect time, and `Remesher::collect` is the call the frame path makes. There is
  no `Session` surface for it and there should not be: `App` owns the worker.
- **`remesh` and `build_section_geometry`** (FR-4.1-S3, FR-6.2-S2, FR-6.2-S5,
  FR-4.3-S2) — a batch `Session::take_remesh_work` handed over is meshed and packed
  by the same two functions the worker calls, because what a *batch* resolves against
  is the property under test and the worker would hide it behind a scene.

### Where every number came from

| Number | Where it is stated | Derived how |
|---|---|---|
| **256** | `reload_remesh::EVERY_SECTION_OF_THE_SHIPPED_WORLD` | `FOOTPRINT_COLUMNS * FOOTPRINT_COLUMNS * SECTIONS_PER_COLUMN` — `mc_sim::replay::world`'s four and `mc_world::column`'s sixteen. **Never written as a digit anywhere in this phase.** The *set* is derived the same way by `every_section_of_the_shipped_world`, over `(0..FOOTPRINT_COLUMNS)²  × (0..SECTIONS_PER_COLUMN)`, so an implementation marking the right *count* of the wrong sections fails as loudly as one marking too few |
| **~82** | nowhere, deliberately | Trap 6: it is a lower bound read as a count, and an expectation of it reddens against a conforming implementation. The comparison is against the whole footprint, and this file's header says so |
| the layer `base:amber` takes (4) | `reload_content::THE_NEXT_UNUSED_LAYER` (phase 2's) | `SHIPPED_KEYS.len()` — the count a launch spends, which is also the first index nothing holds. Never written as a digit |
| 20 corners for a placed block | `reload_upload::{A_BLOCK_ON_A_FLOOR_SHOWS, CORNERS_PER_QUAD}` | five of six faces (the downward one buried against the floor it stands on) × four corners per quad |
| 2 faces for a residue set into a floor | `reload_meshes_against_the_content_serving::A_BLOCK_SET_INTO_A_FLOOR_SHOWS` | its top, because nothing stands on it, and its bottom, because the cell under the floor is empty; its four sides abut stone |
| 1 downward face | `reload_uncovers_culled_faces::A_BLOCK_HAS_ONE_DOWNWARD_FACE` | there is exactly one grass voxel in that world, so the count of its `-Y` faces is one when the face is drawn and zero when it is culled. **Not a quad count greedy merging could move** |
| 2 sections marked by one break | `reload_meshes_against_the_content_serving::A_BREAK_IN_ONE_COLUMN_MARKS` | the section holding the edited cell and every face-adjacent neighbour the footprint holds — in a world of one column with the edit in its lowest section, the one above it and nothing else. The same two `mc-sim`'s `edit_replay.rs` already derives |
| 16 sections in a scene | `reload_remesh::a_scene_of_one_column` | `SECTIONS_PER_COLUMN` |
| how long a collect waits | `reload_remesh::a_batchs_patience` | the mesher's declared per-section budget of 200 µs (`crates/mc-render/CLAUDE.md`) × the sections a whole-world batch carries × a margin of 20. **The margin is not a measurement and does not have to be**: every assertion made against this bound is a *presence*, so a window too short fails on `Collected::NothingArrived` rather than passing over an absence |
| the camera and the 32-pixel square | `reload_draws_the_new_block`'s header | the placed face is the unit square at `y = 11` centred on `(9.5, 8.5)`; the eye stands 3 blocks over it and `3 · tan 30° = 1.732` along `+x`, so the view is 30° off vertical rather than straight down where a look-at has no unique answer. At `√(3² + 1.732²) = 3.464` blocks the 60° lens takes in `4.0` world units over 720 pixels — 180 px per block — so the face projects to roughly 180 × 156 px and a 32-px square on the target is well inside it and spans about 2.8 of the sixteen texels across a face, which is what makes both checkerboard colours appear |
| where the held-block swatch lands | nowhere | `Prediction::of(root, CAPTURE_SIZE).element("base:held-block").fill` — an independent prediction over the root's own declarations, so an author moving or resizing the element moves the reading with it. `hud_held_block.rs` states the same rectangle as a literal; restating that literal here would have been a second thing to keep in step |
| every tick count | nowhere | asserted as a difference across a run, never as an absolute |

### Arrival colours, measured — and two skeletons, because one was not enough

**Measured against a scratch skeleton the test author stood up in order to typecheck
the suite and reverted by hand.** `git diff --exit-code` is clean on every `src/`
tree afterwards and `cargo fmt --all --check` is clean, so these colours are against
*that* skeleton and not against the implementation's. What they establish is that
every test reaches its assertion and fails on its own subject.

The skeleton carried the whole shape — the batch carrying its registry and serial,
`remesh` reading it, the three-armed `Remeshed`, `retire` on the batches' own
channel, `mark_for_remesh`, `Session::mark_for_remesh`, `ReloadReport::Accepted`'s
`layers` field — and **deliberately less** in three places: no geometry predicate and
no marking on a reload (the phase 1–3 state), a `collect` that never compares serials
so `Superseded` is unreachable, and a report whose `layers` field is
`TextureLayers::default()`.

| Skeleton | Red | Green |
|---|---|---|
| **A** — as above | **10** | 3: FR-6.1-S1, FR-6.2-S3, FR-6.2-S5 |
| **B** — A, with the report's layers built honestly from the published content | 7 | 6: A's three plus FR-4.3-S2's two halves and FR-4.3-S3 |

**Two were needed and the pair is the finding.** Under A, FR-4.3-S2's drawn half
reddens by *erroring* on the packing (`Geometry(UnresolvedTexture { base:stone })`)
rather than on its assertion, which is a weaker red than the other nine. Under B all
three of T19's tests go green, which says exactly what grades them: **the report
carrying the layers the content now serving states, and the batch carrying its
registry.** Both are this phase's own additions, and neither is a marking.

The eight reds that are assertion failures on their own subject:

- **FR-6.1-S2/S3/S4** — `NoSectionAtAll` against `EverySectionOfTheShippedWorld { marked: 256 }`.
- **FR-4.1-S3** — `Faces::NoBatch` against `Faces::Showing(1)`.
- **FR-6.2-S1/S4** — `Scene { sections: 16 }` against `Superseded { keys: [(0,0,0), (0,0,1)] }`.
- **FR-6.2-S2** — `Meshed::NoBatch` against the 16-section oracle map.
- **FR-4.3-S2** (boundary) — `Packed::RefusedNaming("base:stone")` against `Packed::Faces { corners: 20, layer: 4, sharing: [] }`.
- **FR-4.3-S3** — `(Some("base:amber"), None, 576, 0, 576)` against `(Some("base:amber"), Some(4), 0, 2, 576)`: the block reaches the hand and the layer it would draw from does not exist, so the swatch is not painted and all 576 pixels of the fill stray.

**Nothing outside this phase reddened.** `cargo nextest run -p mc-sim -p mc-client
-p mc-world` against skeleton A is **695 tests, 685 passed, 10 failed** — phase 3's
682 plus this phase's 13, with the 10 above the only failures.

### Green on arrival, and what each is

| Scenario | Green because | The mutation that must redden it |
|---|---|---|
| **FR-6.1-S1** | `tasks.md` predicts it: phases 1–3 mark no section on a reload, so "a candidate changing nothing geometric meshes no section" is satisfied by an implementation that never meshes | make the geometry predicate return `true` unconditionally. **Its paired control is FR-6.1-S4 and the two run on one instrument** — `Marking`, read through `Session::take_remesh_work` |
| **FR-6.2-S3** | it is the existing `Failed` path, which `tasks.md` records as unchanged. Green the moment `Remeshed` has three arms | make the worker swallow a pack refusal and send the scene it had, or drop the `Failed` arm's cause |
| **FR-6.2-S5** | **green as soon as the batch carries its registry and `remesh` stops taking one** — which is the "unspellable rather than checked" `tasks.md` names. Not predicted as green-on-arrival there, and it is | give `remesh` a registry parameter again and pass the launch's: the break's residue is a block the launch's content never declared, so the batch stops meshing at all |

**FR-6.2-S5's greenness is a correction to `tasks.md`'s green-on-arrival list, not a
contradiction of its weak-instrument entry.** That entry says S2 and S5 become
unspellable once the batch carries its registry; what it does not say is that S5 is
therefore green the moment the structure lands, while **S2 is not** — S2 reads the
*reload's own* batch and so rides on the marking as well. The two weak instruments
are not equally weak, and which is which is worth knowing before somebody reads S5's
green as evidence.

### Weak instruments, named

- **FR-6.2-S2 and FR-6.2-S5** — `tasks.md` records both as structural confirmations
  rather than defences, because a batch carries the registry its world was resolved
  against and `remesh` takes no second opinion. That is right and both are written to
  be **behavioural anyway**: in each, the content now serving declares a block the
  content it replaced does not, so a batch resolved against the wrong registry cannot
  be meshed at all and the `Meshed::Refused` arm says so. The behavioural instrument
  for the hazard is still FR-6.2-S1, which is where a *stale* batch is discarded.
- **FR-6.1-S1** — satisfied by an implementation that never meshes, which is why it
  and FR-6.1-S4 are read on one instrument in one binary. It is not evidence on its
  own.

### The vacuity these scenarios are shaped against, and how each closes it

Phase 1's finding was that an observation taken through a channel the operation does
not update cannot see the operation. Phase 4's channel has the opposite hazard: the
dirty set is **taken**, so a second reading of it is empty and a scenario reading
that would call it "nothing was marked".

| Scenario | Vacuous under | The half in the same run that closes it |
|---|---|---|
| FR-6.1-S1 | an implementation that meshes nothing on any reload | FR-6.1-S4, on the same instrument in the same session: one candidate marks nothing and the next marks the world |
| FR-6.1-S2 | a launch that left sections outstanding, making the count the reload's plus something else | a drain before the reload, required to be `NoSectionAtAll` — which is both the guard and what leaves the set empty for the reload to fill |
| FR-6.1-S3 | a dirty set that stopped being a set, or one reload re-marking for the rest of the run | the marked arm requires the key list and its distinct set to be the same size *and* to be the footprint's own; a second reading afterwards requires `NoSectionAtAll` |
| FR-4.1-S3 | a reading of one value with nothing to compare it against | an independent whole-world mesh of the same declared blocks against the registry the content serving at launch produces, required to bury the face |
| FR-6.2-S1 | a client that discarded the stale batch and then drew nothing | a batch drained *after* the swap has to come back as a scene, and it holds the block only the new content declares — so a worker never told the layers now serving reports a failure instead |
| FR-6.2-S4 | a hand-back that does nothing, against a reload that marked the world anyway | **the candidate changes only `breakable`, so the reload marks nothing** — the sections waiting afterwards are the ones handed back and nothing else |
| FR-6.2-S2 | two contents that mesh the same world identically | a guard requiring the candidate's mesh to differ from the content it replaced |
| FR-6.2-S5 | a batch that never existed, or one that refused | `Faces` is a total verdict with arms for both, and the reload's own batch is drained and the drain after it required empty so the reading is the break's |
| FR-4.3-S2 (boundary) | a packer that wrote the same layer into one corner and something else into the rest, or a layer another block also draws from | every corner of every face is compared, and `sharing` has to be empty |
| FR-4.3-S2 (drawn) | a frame that drew nothing, or one whose pixels were already those colours | the same pose over the same world one reload earlier, required to stray from every one of those colours |
| FR-4.3-S3 | a rect that fell off the frame, or a swatch nobody painted | `considered` has to equal the predicted fill's own area, and the layer the assignment states has to be the appended one |

### Additional coverage

| Test or guard | Where | What it catches |
|---|---|---|
| the ticks advanced across a marked world | FR-6.1-S2's own assertion | the scenario's second clause — *while going on advancing ticks* — asserted rather than described: the tick the client publishes has to have moved by the ticks that were driven, so a client that stopped advancing while a whole-world re-mesh stood outstanding fails here |
| a second drain after the reload's | FR-6.1-S3 | one reload leaving one batch rather than a section re-marked for the rest of the run |
| the newly declared block is what the hand holds | FR-4.3-S2 (boundary) | a placement of the block the client launched with, which would make the packing reading about the wrong block |
| the break and the placement reached the world | FR-6.2-S1, S3, S4 | a click that changed nothing, which leaves no batch to submit and would report as a missing batch rather than as the staleness question |
| both readings covered the whole square | FR-4.3-S2 (drawn) | a rect that fell off the frame: a region accepting nothing makes "nothing strayed" true |
| the candidate's mesh differs from the content it replaced | FR-6.2-S2 | two registries that mesh the same world identically, under which a batch resolved against either satisfies the comparison |
| nothing outstanding before the edit under test | FR-4.1-S3, FR-6.2-S2, FR-6.2-S5 | a launch, or a reload, that left sections marked — which would make every count the edit's plus something else |

Every one of these sits inside a scenario's own test, as an element of its comparison
or as a `require` guard. They are recorded here so nobody later deletes one as noise.

### Fixtures this phase owns

| File | What it is |
|---|---|
| `crates/mc-client/tests/support/reload_remesh.rs` | **New.** The `Marking`, `Collected`, `Meshed`, `Faces` and `Reported` verdicts and the readings that produce them; the derived whole-world section set; the retained list a launch would have handed a worker; the layers and serial a client is publishing; the two edits the aims of `reload_world` reach. **487 non-blank lines against the 600 limit** — the next addition splits it, and the seam is the obvious one: what a batch *is*, and what a worker *made of one* |
| `crates/mc-client/tests/support/reload_upload.rs` | **New.** What the report of an accepted reload hands the frame path (`TakenUp`, `until_taken_up`), the author's edits made *after* the client launched, and `Packed` — what the packer wrote into the corners of one block's faces, read back rather than asked of the assignment |
| `crates/mc-client/tests/support/input/mod.rs` | Gains two forwards and nothing else: `take_remesh_work` and `mark_for_remesh`. Both decide nothing |

Both new modules are reached by `#[path]` for phase 1's reason: they name types the
implementation has not written.

**Two aims, and raw counts accumulate.** `placing_over_the_near_cell` asks for both
aims from a level look; `placing_over_the_near_cell_after_the_far_aim` asks for only
the difference, because a run that has already broken the far cell is 280 counts down
and asking for the whole nearer aim again would carry the look past the declared
pitch limit and clamp it — a third aim nothing derived. The first draft of the
staleness suite did exactly that.

### Two adaptations of pre-existing suites

`remesh` loses its registry parameter, which is what makes "a batch cannot be meshed
against a registry other than the one its world was resolved against" structural
rather than a rule. Two test files call it:

- `crates/mc-client/tests/edit_geometry.rs` — one argument dropped.
- `crates/mc-client/tests/saved_changes_need_no_edit.rs` — one argument dropped, and
  its `Handed` struct loses the `registry` field that had no other reader.

Both were run against the skeleton and pass, together with `edit_replay.rs`,
`reload_keeps_packed_layers.rs` and `documented_refusals.rs` — 18 of 18.

### Two things phase 4 is barred from, and no test here asks for either

**No test here needs the mesher to resolve a texture through the registry.**
`blocks/amber.luau` declares `texture = "base:amber"`, equal to its name, and both
new fixtures say why: SPEC-016's pin on the name-for-texture substitution turns red
when PRO-902 closes that gap, and that red is its success signal. A fixture whose
block named a different texture would be refused for *that* instead — red for the
wrong reason, reading as a defect in the layer policy when it is not one.

**No test here asserts a cleared player.** Where a player ends up when a reload makes
their cell solid is phase 5's, and every world these scenarios drive is one where
nothing the player's box overlaps becomes solid — the two that take stone's solidity
*away* cannot trap anybody, and the rest change no solidity at all.

### Correction after the implementation landed — FR-6.2-S4 called the hand-back itself

**Annotated rather than rewritten.** Everything above records what was believed when
the tests were authored and what was measured against the scratch skeleton at
`c21deeb`; those colours stand. The row in the mapping table is a pointer rather than
an observation, so it names the corrected test. This section is what changed and why.

**Verdict on the dispute: `test-wrong`, and the implementer's diagnosis is exact.**
The first draft of FR-6.2-S4 read the discard through `Remesher::collect` and then
called `Session::mark_for_remesh` itself:

```rust
let discarded = collected(&mut playing.remesher);
playing.client.mark_for_remesh(keys.clone());          // the test made the call
let waiting = playing.client.take_remesh_work().map(|work| keys_of(&work));
```

What that graded is that keys handed to `Session::mark_for_remesh` come back out of
the dirty set. That is true, and it is not the scenario. **The scenario is that
something hands them back at all**, and replacing the frame path's `Superseded` arm
with `drop(keys)` left **77 of 77 green**, FR-6.2-S4 included — measured by the
implementer, not predicted here.

This is `testing.md` §2's *policy is not wiring* in its purest form, and it is the
shape phase 1's own brief warned about in as many words: **a test that calls the
function itself is agreement between two callers of one function.** It is also the
one assertion this file had already flagged as possibly disputable — for the wrong
reason. What was in doubt was whether the comparison should be equality or a
superset; the defect was one level up, in who made the call.

**The corrected expectation.** The reading goes through the client's own collect, and
**nothing in the test puts a section back**:

```rust
let discarded = handled(&mut playing.client, &mut playing.remesher);
let waiting = playing.client.take_remesh_work().map(|work| keys_of(&work));
assert_eq!((answered, discarded, waiting), (accepted(DIRT), Handled::Discarded, Some(keys)));
```

`Handled::Discarded` says the batch really was discarded rather than merely
unfinished — without that arm, `NothingYet` and "discarded" look alike and the
comparison would be vacuous in a second way. The equality on `waiting` is unchanged
and its premise is unchanged: the candidate changes only `breakable`, so the reload
marks nothing of its own.

**The interface this needs, and why it is a design change rather than a fixture
change.** The hand-back has to move to the side of the seam a test can drive:

```rust
// crates/mc-client/src/session/reload.rs
/// What one collect from the re-mesh worker left the frame path to do.
pub enum Remeshing {
    NothingYet,
    Show(Arc<SceneGeometry>),
    /// A batch meshed against content that has stopped serving. Its sections are
    /// back among the ones waiting to be meshed.
    Discarded,
    Report(RemeshError),
}

impl Session {
    pub fn collect_remesh(&mut self, remesher: &mut Remesher) -> Remeshing;
    fn mark_for_remesh(&mut self, keys: Vec<SectionKey>);   // NO LONGER `pub`
}
```

`App::exchange_remesh` keeps `submit_remesh` and becomes one call, one upload and one
report — every one of them a forward, which is what D1 says the frame path's share
must be. **Under this shape the mutation that survived is unspellable**: there is no
`Superseded` arm in `App` to drop, and deleting the hand-back inside
`collect_remesh` reddens.

**Two doors were closed rather than one, and that is the part worth keeping.**
Correcting the assertion alone would leave the next author able to make the same
mistake, so `Session::mark_for_remesh` loses its `pub` and the input harness loses
its forward for it. **A fixture that offers a decision as a call of its own is how a
scenario about that decision comes to make it**, and the absence of that forward is
now stated in the harness where the next reader will meet it.

**One alternative was considered and rejected, recorded so it is not re-derived.**
`World::take_remesh_work` could *copy* rather than take, with a separate
acknowledgement clearing the dirty set on success — then a discarded batch needs no
hand-back at all, because nothing ever cleared those sections, and FR-6.2-S4 becomes
structural. It is rejected here for two reasons: it inverts the contract
`take_remesh_work`'s own doc comment states ("taking rather than reading is what
makes a section re-meshed once per edit instead of once per drain"), and it makes a
scenario's subject *disappear* rather than grading it — which is the right outcome
for a hazard the type system can absorb and the wrong one for a decision a caller
still has to make.

**The two scenarios beside it are unchanged and keep reading `Remesher::collect`.**
FR-6.2-S1 and FR-6.2-S3 are about the comparison and the failure path, and `collect`
is where both are made; the mutations the implementer ran redden them from there.
Only the hand-back needed the other side of the seam.

#### The same correction, measured — three runs, and the prescribed shape was the wrong one

**Annotated rather than rewritten.** The section above records what was proposed; this
is what was measured after the implementation landed. The interface landed as
proposed — `Session::collect_remesh(&mut Remesher) -> Remeshing`, with
`Session::mark_for_remesh` **private** and no forward for it on the input harness.

An intermediate design was put up first, in which `Remeshed::Superseded` carried a
`#[must_use]` `Stale` value and the frame path called `Stale::back_into(session)`, and
FR-6.2-S4 was to drive `back_into` itself. **That shape was built and measured rather
than argued about, and it does not close the gap:**

| Run | Mutation | Result |
|---|---|---|
| 1 | none — the prescribed shape, test calling `back_into` | 3 of 3 pass |
| 2 | the frame path's arm replaced with `drop(stale)` | **3 of 3 pass — the gap survives** |
| 3 | the landed shape, `collect_remesh`'s hand-back replaced with `drop(stale.into_keys())` | **FR-6.2-S4 reddens, and only it** |

Run 2 is the whole finding. **Which function a test calls makes no difference if the
test is the thing calling it** — `back_into` and `mark_for_remesh` are the same
mistake under two names, because neither runs the frame path. That is why the fix had
to move the *call* rather than rename it, and run 3 is what says the move worked:

```
left:  (Accepted { holding: "base:dirt" }, Discarded, None)
right: (Accepted { holding: "base:dirt" }, Discarded, Some([(0,0,0), (0,0,1)]))
```

The middle element still reads `Discarded` under the mutation, which is the reason
that arm has to exist and carry nothing: **a discard that lost its keys is still a
discard**, so the verdict alone cannot see the defect and the sections waiting
afterwards are what does.

**What the compiler holds, and the one spelling nobody needed to measure.** A named
binding the frame path ignores (`Superseded(stale) => {}`) is `unused_variables`,
which the gate denies — confirmed. A pattern-ignored one (`Superseded(_) => {}`)
would *not* be, because `#[must_use]` fires on unused expressions and not on discarded
bindings — but under the landed shape the question is moot and was left unmeasured
rather than reported as though it had been: **no keys reach the frame path at all**, so
there is no arm there to write either way. It is recorded because it is the reason a
`#[must_use]` value handed to a caller is a weaker guard than it looks, and the next
design tempted to hand one over should measure it first.

**Two doors closed rather than one, and the second is the durable half.**
`Session::mark_for_remesh` is private and the harness offers no forward for it, so the
mistake this correction is about is no longer *writable* by a later author. The
harness says so at the point where somebody would reach for it.

#### FR-6.2-S1 does not need the discarded keys' identity, and the accessor goes

**Ruled on the team lead's standing rule** — a test-only accessor either earns its
place by a production reader naming it, or goes. Nothing in `mc-client/src` calls
`Stale::keys()`, and there is no reader worth adding: a batch discarded because a
reload landed mid-flight is ordinary operation, so a report naming it every time
would be noise rather than information.

**The reason it is not needed, which is the part that had to be checked rather than
preferred.** The only defect the identity could catch on that side of the seam is a
worker that recorded the *wrong* keys for the batch in flight. FR-6.2-S4 already
catches exactly that, **through the effect**: it captures the batch's keys before
submitting, and compares them against what `Session::take_remesh_work` hands back
after the discard. If `submit` recorded the wrong set, `into_keys` yields the wrong
set, what ends up waiting is wrong, and S4 reddens. So carrying the keys in S1 would
re-prove S4's fact through the same `in_flight` bookkeeping — `testing.md` §1's
"re-proves what another test already proves *through the same code path*".

`Collected::Superseded` is therefore a unit variant, and it stays discriminating for
S1's own purpose: the arm is chosen by variant rather than by payload, so it goes on
rejecting `Scene`, `Failed` and `NothingArrived`, and the measured mutation that
reddens S1 — the staleness comparison never firing, which yields a `Scene` — is
unaffected by the payload's removal.

**And there is a positive argument for removing it, not merely an absence of one.**
It is this phase's own correction turned on its next step: *a fixture that offers a
decision as a call of its own is how a scenario about that decision comes to make
it.* An accessor handing a test the value the scenario is about is the same shape one
level along — it makes an assertion on the value easy to write in place of one on the
consequence, which is the defect FR-6.2-S4 had, wearing the new type. Removing it
takes the temptation out of reach structurally, exactly as making
`Session::mark_for_remesh` private did.

**Nothing mechanical would have caught this.** `Stale::keys()` is `pub` on a `pub`
type, so no lint reports it unused however few callers it has — which is why the rule
it falls under is a reviewer's rule rather than a gate stage, and why the answer had
to be a decision with a reason attached.

---

## Phase 5 — A player inside a cell that became solid is moved clear

**7 scenarios, 7 tests.** Each scenario has exactly one, and no scenario needs two:
the one clause that could have wanted a second instrument — FR-7.1-S5's *report* — is
read where the verdict crosses out of the client's core, and the half beyond it
(`App` printing a line) is reviewer-held for the reason `report_remesh`'s already is.

Test command — the three binaries these live in, by name, so a run of them is a run
of this phase and nothing else:

```
cargo nextest run -p mc-client -E 'binary(/^reload_(clears_a_trapped_player|leaves_the_player_alone|reports_nowhere_to_clear_to)$/)'
```

And the whole of the three crates, which is what a phase boundary runs:

```
cargo nextest run -p mc-sim -p mc-client -p mc-world --test-threads 2
```

**Everything in this phase lives in `mc-client`**, because every observation is either
what the client's own door answered or what its report handed the frame path, and the
position afterwards is read through the snapshot a further tick published. `mc-sim`'s
own suites are untouched — nothing here needed adapting.

### The window nothing compiles in, and how wide it is

Three names phase 5 adds do not exist yet: `mc_sim::world::Clearing`, a `clearing`
field on `mc_sim::simulation::Accepted`, and a `clearing` field on
`mc_client::session::reload::ReloadReport::Accepted`. Every binary naming one is a
compile error rather than a red assertion.

**The window is exactly three binaries**, measured with `--keep-going` rather than
inferred: `reload_clears_a_trapped_player`, `reload_leaves_the_player_alone` and
`reload_reports_nowhere_to_clear_to`. It is narrower than phase 3's and phase 4's
because neither new fixture module is named by `support/input/mod.rs` — the harness
needs no new forward, since `adopt`, `tick`, `attach_reload`, `take_reload_report` and
`content` are all already there. Every other test binary in `mc-client`, and the whole
of `mc-sim`, `mc-world`, `mc-render`, `mc-core` and `mc-testkit`, builds and lints
clean.

`cargo clippy --workspace --all-targets --all-features -- -D warnings` reports
**nothing but** those three unresolved names — one `E0432`, one `E0609`, one `E0026`,
the same three in each of the three binaries — and no lint of any kind.
`cargo fmt --all -- --check` is clean. The same clippy run with `--exclude mc-client`
is at exit 0, which is what says the window is `mc-client`'s test targets and nothing
else.

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-7.1-S1 | `crates/mc-client/tests/reload_clears_a_trapped_player.rs` | `a_reload_that_makes_the_players_own_cell_solid_puts_them_somewhere_clear` |
| FR-7.1-S2 | `crates/mc-client/tests/reload_clears_a_trapped_player.rs` | `a_reload_moves_the_player_sideways_where_sideways_and_upward_are_both_clear` |
| FR-7.1-S4 | `crates/mc-client/tests/reload_clears_a_trapped_player.rs` | `a_reload_moves_the_player_sideways_where_the_nearest_clear_cell_is_below_them` |
| FR-7.1-S7 | `crates/mc-client/tests/reload_clears_a_trapped_player.rs` | `a_reload_that_moves_a_rising_player_upward_takes_their_climb_away` |
| FR-7.1-S3 | `crates/mc-client/tests/reload_leaves_the_player_alone.rs` | `a_reload_that_makes_a_cell_no_part_of_the_player_stands_in_solid_moves_them_nowhere` |
| FR-7.1-S6 | `crates/mc-client/tests/reload_leaves_the_player_alone.rs` | `a_candidate_that_would_have_trapped_the_player_and_was_refused_moves_them_nowhere` |
| FR-7.1-S5 | `crates/mc-client/tests/reload_reports_nowhere_to_clear_to.rs` | `a_reload_with_nowhere_clear_inside_the_bound_leaves_the_player_and_says_so` |

**7 scenarios, 7 tests, each scenario exactly once.** T20 owns S1, S2, S3, S4, S5 and
S7; T21 owns S6.

### The reading a swap can reach, and why every test here takes it

Phase 1's finding is this phase's whole premise and it is worth restating where it now
bites: `Simulation::adopt` publishes no tick, so the snapshot standing at the moment a
candidate is handed over was written by the *previous* `advance` and **nothing the
clearing search does can change what it holds.** Phase 1 shipped FR-3.2-S1 and
FR-3.2-S3 green and vacuous for exactly that reason.

So every one of these seven reads the player through `client.tick()` *after* the swap
— the first snapshot a clearing move could have been written into — and every
destination is a cell whose own floor holds the player up, so that tick's gravity
resolves back onto the same face and the reading is the move rather than the move plus
a fall. **Measured rather than reasoned:** with the search deliberately reporting
`Unneeded` and moving nobody, all five moving scenarios fail on their own assertion,
and the two that expect no move pass.

### Which caller each scenario drove through

**All seven drive through `Session`.** Six hand the client a candidate through
`Session::adopt_content` (via `InputHarness::adopt`); FR-7.1-S5 goes through the
**watcher**, because its subject is what `ReloadReport::Accepted` carries and only a
driven `ContentReload` produces one. Nothing here calls `Simulation::adopt` or
`ContentReload::at_tick_boundary` itself, so a `Session` that stopped driving either
reddens the whole phase.

**FR-7.1-S5 is the only one that could not use `adopt`, and that is the scenario's own
wording.** It requires the system to *report* that a player could not be cleared.
`adopt_content` hands the verdict straight back to its caller and stashes no report,
so a test reading it there would grade a value the product computes and would say
nothing about whether anybody is ever told. The verdict is therefore read out of
`Session::take_reload_report`, which is where it crosses out of the client's core on
its way to `App`.

**What that still leaves ungraded, stated rather than left to be found:** `App`
turning that verdict into a line somebody reads. `crates/mc-client/src/app.rs` needs a
real window and nothing in this workspace constructs one, so the printing is held by
review exactly as `report_remesh`'s, `report_swatch`'s and the reload refusal's
already are. This is the same shape as phase 4's row 6 and it is smaller: what would
be lost is a diagnostic line, not a user-visible outcome.

### Where every number came from

| Number | Where it is stated | Derived how |
|---|---|---|
| the search bound, 8 | `reload_trap::A_SEARCH_OF` | written out once, as `spec.md`'s Declared Quantities table states it, and deliberately **not** read from whatever constant the implementation declares — that constant is on the other side of FR-7.1-S5's comparison, and an expectation assembled from it would read back whatever it became. Both cube generators derive their reach from it, so a fixture that has to block everything the search can look at follows the declared bound |
| **4 913 and 2 601** | nowhere, deliberately | the spec's cost ceiling and the share of it this search spends. Both are facts about cost rather than behaviour, and an assertion on either reddens against a conforming implementation — Trap 6's shape in a second place. What is graded is reachability: a clear space the search can see is found, and a world in which it can see none is reported as such |
| the floor row, the feet row and the head row | `reload_trap::{FLOOR_ROW, FEET_ROW, HEAD_ROW}` | `reload_world::FLOOR` and one and two above it. A box 1.8 blocks tall standing on that floor's top face occupies exactly two rows, which is what makes clearance a question about two cells |
| every destination | each scenario's own `const`, with its derivation beside it | worked out by hand from the declared order — `dy` first, then Chebyshev horizontal distance, then the `(dz, dx)` tie-break — over cell centres, as the first candidate whose box is clear given the cells the fixture wrote. **No expected position is copied from a run**, and each is checked by a guard that the cell really is clear |
| the rising player's speed, 9.0 | `reload_clears_a_trapped_player::RISING` | the declared jump speed (`crates/mc-sim/src/player/physics.rs`, where the constant is private). What the scenario needs of it is only that it be strictly positive — see the velocity note below |
| the off-centre spawn, 8.625 and 8.375 | `reload_leaves_the_player_alone::OFF_CENTRE` | an eighth of a block either side of the column's centre, both exact in binary, so that a move to the centre of the cell the player is already in is a move the reading can see |
| the two world widths | each binary's `ONE_COLUMN` / `TWO_COLUMNS` | one column is 16 blocks across, which reaches every candidate the small-world scenarios name. Two is 32, and it is required rather than generous — see finding 2 below |
| every velocity | nowhere | asserted as "at rest", which is `Vec3::ZERO`'s bits |

### Why the velocity scenario needs a *rising* player, derived rather than chosen

FR-7.1-S7's own reason is that the next tick must not carry the player back into the
cell they were moved out of — which reads as a scenario about a *downward* velocity.
It cannot be tested that way, and the reason is a property of the search rather than
of the fixture:

> Any upward candidate the search takes is a cell whose floor is the very thing that
> blocked the candidate one step lower. The candidate one `dy` lower covers rows
> `f − 1` and `f`; row `f` is empty because the destination is clear; so row `f − 1`
> is solid. **A cleared player therefore always lands supported.**

So one tick of a preserved *downward* velocity is spent by that tick's own collision —
resolved back onto the same face and zeroed by `settled` — and the reading is
identical whether the search zeroed it or not. A preserved *upward* velocity is not:
`settled` keeps a rise, so the tick carries the player 0.1417 blocks higher and
reports 8.5 blocks per second. Both halves of the comparison move, which is why the
scenario's player is mid-jump.

### Arrival colours, measured

**5 of 7 red, 2 green.** Measured against a scratch skeleton the test author stood up
in order to typecheck the suite and **reverted by hand** — `git diff --exit-code` is
clean on every `src/` tree afterwards, and `cargo fmt --all -- --check` plus
`cargo clippy --workspace --all-targets --all-features -- -D warnings` were clean with
it in place. So these colours are against *that* skeleton and not against the
implementation's; what they establish is that every test reaches its assertion and
fails on its own subject, and that **every fixture guard passes** — no premise had to
be relaxed.

The skeleton is the deliberately-less one: `Clearing` with its three arms, a `cleared`
that reports `Unneeded` and moves nobody, the verdict carried on `Accepted` and out
through `ReloadReport::Accepted`. An over-eager skeleton is the wrong one for the
phase as a whole — it would fail the two scenarios that are about *not* moving for the
right reason and say nothing about the five that are about moving — so it was applied
as a mutation instead, per scenario, below.

The five reds, each an assertion failure on its own subject:

- **FR-7.1-S1** — `(NoMoveNeeded, At (8.5, 10.0, 8.5))` against
  `(MovedTo (8.5, 10.0, 7.5), At (8.5, 10.0, 7.5))`.
- **FR-7.1-S2** — the same shape against `(MovedTo (7.5, 10.0, 8.5), …)`.
- **FR-7.1-S4** — against `(MovedTo (8.5, 10.0, 6.5), …)`.
- **FR-7.1-S7** — `(NoMoveNeeded, At (12.5, 9.2, 12.5))` against
  `(MovedTo (12.5, 11.0, 12.5), At (12.5, 11.0, 12.5))`. The 9.2 is the collision
  ejecting a rising player from the block the reload put around them, which is what an
  uncleared player actually gets.
- **FR-7.1-S5** — `NoMoveNeeded` against `NoClearSpaceWithin { blocks: 8 }`, with the
  position and solidity halves of the comparison already agreeing.

**Nothing outside this phase reddened.** `cargo nextest run -p mc-sim -p mc-client
-p mc-world --test-threads 2` is **702 tests, 697 passed, 5 failed** — phase 4's 695
plus this phase's 7, with the five above the only failures.

### Green on arrival, and the mutations that decide whether they are controls

`tasks.md` predicts FR-7.1-S3 and FR-7.1-S6 green on arrival and names a mutation for
each. **Both mutations were run by the test author against the scratch skeleton,
before any implementation exists, and both bit — each reddening only its own
scenario.** That is stronger evidence than a prediction and it is recorded here rather
than left to the implementation round.

| Scenario | Green because | Mutation | Outcome |
|---|---|---|---|
| FR-7.1-S3 | phases 1–4 move nobody, so "left exactly where they are" is satisfied by a search that never moves anybody | `cleared` returns `MovedTo(feet + (1, 0, 0))` where it would have returned `Unneeded`, and writes the player | **bit** — both halves: the verdict, and the position `9.625` against `8.625`. FR-7.1-S6 stayed green, correctly: the search is not reached from the refusal path |
| FR-7.1-S6 | `cleared` is reached only from the accepted path, so a refused candidate moves nobody because the code never runs | the search reached from the refusal path, guarded by an overlap test against the world's current solidity | **bit** — the player moves from `8.5` to `9.5`. FR-7.1-S3 stayed green |

**And a third measurement, which is the finding this phase should be remembered for.**
`tasks.md` states FR-7.1-S6's diagnosis as *"if it does not redden, the test is not
driving a refused candidate that would have made the player's cell solid"*. That is
necessary and **not sufficient**, and the difference was measured rather than argued:

> With the refusal-path mutation live and the fixture reduced to the naive one — a
> player whose box overlaps nothing solid until the candidate would make it so, which
> is exactly what FR-7.1-S6's wording describes — **the scenario passes.** 2 of 2
> green.

The reason, once stated, is structural: on the refusal path `World::adopt` never ran,
so the world still has the solidity it had, and a search called there asks about a box
that overlaps nothing and answers `Unneeded`. The candidate's own solidity does not
exist anywhere for it to read. **So the fixture has to put the player somewhere a
search running against the *serving* world would move them from**, which is why
FR-7.1-S6's player has their head inside a `base:stone` cell and their feet in the
`base:water` cell the candidate would make solid. Both cells are needed and neither is
decoration: the stone is what gives the mutation work to do, the water is what makes
the candidate one that would have trapped them.

That premise is asserted rather than described, by
`reload_trap::require_a_refusal_could_have_moved_them`, which refuses both the
already-wedged fixture and the naive one — so the next author cannot reduce this test
to filler without the guard saying so.

### The vacuity these scenarios are shaped against, and how each closes it

| Scenario | Vacuous under | The half in the same test that closes it |
|---|---|---|
| FR-7.1-S1 | a reading taken through the snapshot standing when the candidate was handed over, which the swap does not write | the position is read one tick later, and the verdict is compared beside it — a search reporting a move it never made, and a swap moving the player without saying so, are each rejected by the other half |
| FR-7.1-S2 | a world in which the upward cell was never clear, which makes "sideways" true for want of an alternative | a guard requires the box one block up to be clear once water is solid, so the choice really was a choice |
| FR-7.1-S3 | a reload that never happened, and a search that "moved" the player to the middle of the cell they were already in | the solidity the client is serving is in the same comparison and has to have moved; and the spawn is an eighth of a block off centre on two axes, so a move to the cell centre is visible |
| FR-7.1-S4 | a world in which nothing below was clear, under which a search that merely *ranked* downward last also passes | a guard requires the cell one block below to be clear once water is solid, and the floor under the player is deliberately missing to make it so |
| FR-7.1-S5 | a fixture with a gap left in two and a half thousand filled cells, and a run in which no boundary reported anything | a guard walks the whole cube the search walks and requires every position in it to be blocked; and `Clearance` has a `NothingReported` arm, so a run that gave up is not a pass |
| FR-7.1-S6 | a refusal-path search with nothing to clear the player out of — measured, see above | the guard above, which requires the box to overlap something solid *already* and one further cell once water is |
| FR-7.1-S7 | a downward velocity, which the next tick's collision zeroes either way | the player is rising, and the position moves with the velocity so both halves of the comparison discriminate |

### Additional coverage

Every extra assertion sits inside a scenario's own test as a `require` guard rather
than as a separate test, because each is about the fixture being the thing the
scenario describes. They are recorded here so nobody later deletes one as noise.

| Guard | Where | What it catches |
|---|---|---|
| the box overlaps nothing solid while serving and something solid once water is | FR-7.1-S1, S2, S4, S5 | a world that was already wedged before anybody edited anything, and one the candidate never reaches — either of which produces a clearing verdict a reader would take at face value |
| the named position is clear once water is solid | FR-7.1-S1, S2 (twice), S4 (twice) | a destination, or an alternative the scenario says was available, that was never available at all. FR-7.1-S2's upward cell and FR-7.1-S4's cell below are the scenarios' own premises, and this is what asserts them |
| nothing the candidate makes solid lies in the box | FR-7.1-S3 | a fixture whose "far away" cell was not far enough, which would make a `MovedTo` legitimate |
| every position the search may look at is blocked | FR-7.1-S5 | a gap in the filling, reported as the gap it is rather than as a search that went looking where it should not have. It counts *candidate positions*, which is a statement about the fixture — and is the one place a count of positions appears anywhere in this phase |
| the box overlaps something solid already, and one further cell once water is | FR-7.1-S6 | the naive fixture that leaves the scenario filler, measured above |
| the player is rising | FR-7.1-S7 | a fixture at rest or falling, under which "the velocity was taken away" is true of a tick that would have taken it away anyway |
| the client is still serving what it should be | FR-7.1-S3, S5, S6 | a reload that never happened (S3, S5) and one that was accepted after all (S6) — without which "the player did not move" is satisfied for good |

### Fixtures this phase owns

| File | What it is |
|---|---|
| `crates/mc-client/tests/support/reload_clearing.rs` | **New.** What a swap said and where the player was one tick later: `Clearance` and `Standing` as total verdicts, the bit comparison, and `until_cleared`, which reads the verdict out of `ReloadReport::Accepted` rather than out of a call a scenario makes. 186 lines |
| `crates/mc-client/tests/support/reload_trap.rs` | **New.** What a reload is driven over: the `Shape` a world is declared as, the two cube generators derived from the declared bound, the spawns, the clients, and the `Overlap` oracle plus the six premise guards built on it. 497 lines against the 600 limit |

**The split is by responsibility and the seam is each header's own.** They began as one
module at **664 lines**, over the gate's 600-line test-file limit, and the header had
already drawn the line the limit then forced: here is what a reading *is*, next door is
what a reload is driven *over*. Nothing was compressed.

Both are reached by `#[path]` for phase 1's reason: `reload_clearing` names types the
implementation has not written. `reload_trap` names none, and is reached the same way
so that the two halves of one fixture are included together rather than one of them
landing in every binary that says `mod support;`.

**The oracle shares no code with the search it grades.** `reload_trap::overlap_at`
walks the box over the world the fixture declared, restating the `0.6 × 1.8 × 0.6` box
FR-7.1-S1 names and the half-open `[v, v + 1)` rule, and asks a *named* list of blocks
which are solid. It reaches none of `collide::overlaps`, none of `SolidVoxels` and none
of the search — a guard that asked the subject's own predicate would agree with it
whatever it did, and would make every premise here unfalsifiable.

**The input harness gains nothing.** `adopt`, `tick`, `attach_reload`,
`take_reload_report` and `content` are all already forwards on it, which is why this
phase's non-compiling window is three binaries rather than the whole crate.

### An interface decision this phase makes, and what it costs elsewhere

`Clearing::MovedTo` carries a `Vec3`, which has no `Eq`, so
`mc_sim::simulation::Accepted` **drops its `Eq` derive** and keeps `PartialEq`. Nothing
in the tree requires `Accepted: Eq` — checked, not assumed. The alternative would be to
carry the position as something `Eq`, which would put a fixture's representation choice
inside a production type.

### Two things phase 5 is barred from, and no test here asks for either

**No test in this phase asserts a re-mesh key, a layer index, a content serial or a
drawn frame.** Making water solid changes geometry, so these reloads do mark the world
— and nothing here reads the marking, which is phase 4's.

**No test here asserts a count of positions the search tested.** See the number table
above: that is Trap 6's shape in a second place, and the one count that does appear is
a fixture guard about the world rather than an expectation about the subject.

### Findings this phase reports rather than resolves

1. **`tasks.md`'s diagnosis of FR-7.1-S6 is incomplete** — measured, above. Recorded
   here because the fixture that satisfies the stated diagnosis is filler, and the
   guard that now refuses it is the only thing standing between this scenario and a
   second witness on a path that already has one.
2. **A position outside the loaded world is not solid, so it is clear, and the search
   may put a player there.** `SolidVoxels::is_solid` answers `false` for every position
   past the footprint by construction, and neither `spec.md` nor D11 says anything about
   it. In the shipped four-column world the search may look 8 blocks out, so a player
   within 8 columns of an edge — which is every player in a 64-block footprint — has
   candidates outside it, and in a wedge those are the first clear ones a search would
   find. FR-7.1-S5 works around it by using a two-column world in which the whole cube
   is inside; no scenario grades it. **Whether a cleared player may be put outside the
   loaded world is an open question this phase surfaces and does not answer.**

---

## Phase 6 — The seam stays cut, the window is declared once, and the pages are true

**5 scenarios, 5 tests**, plus 8 more this phase owes and names below. Test command
— the six binaries these live in, by name, so a run of them is a run of this phase
and nothing else:

```
cargo nextest run -p mc-client -p mc-world -E 'binary(/^(client_names_no_content_door|capture_pipeline_watches_nothing|settling_window_declared_once|reload_remesh_blocks_no_tick|reload_build_runs_off_the_tick_thread|the_client_watches_the_content_it_plays)$/)'
```

And the whole of the three crates, which is what a phase boundary runs:

```
cargo nextest run -p mc-sim -p mc-client -p mc-world --test-threads 2
```

**There is no window in which nothing compiles.** Every name this phase reaches for
already exists — `SETTLING_WINDOW`, `NotifyContentWatch`, `ContentReload`,
`watching_shipped_content`, `Remesher`, `ContentReload::building` — because phase 6
builds instruments over properties phases 1–5 established rather than asking for
anything new. That is also why almost nothing here has a red step in the ordinary
sense, and why the controls below are this phase's whole falsifiability.

### The mapping

| Scenario | File | Test |
|---|---|---|
| FR-8.2-S1 | `crates/mc-client/tests/client_names_no_content_door.rs` | `the_clients_own_sources_name_none_of_the_doors_content_is_read_through` |
| FR-8.2-S2 | `crates/mc-client/tests/capture_pipeline_watches_nothing.rs` | `the_capture_pipelines_own_sources_name_no_door_that_watches_a_content_root` |
| FR-8.2-S3 | `crates/mc-client/tests/capture_pipeline_watches_nothing.rs` | `the_same_scan_reports_a_capture_source_that_watches_and_says_which_door_it_named` |
| FR-9.1-S1 | `crates/mc-client/tests/reload_remesh_blocks_no_tick.rs` | `the_ticks_a_whole_world_re_mesh_runs_over_are_the_ticks_a_run_with_no_reload_advances` |
| FR-9.1-S2 | `crates/mc-world/tests/settling_window_declared_once.rs` | `the_settling_window_is_declared_in_exactly_one_production_source` |

**5 scenarios, 5 tests, each scenario exactly once.** T22 owns S1; T23 owns S2 and
S3; T24 owns FR-9.1-S2; T25 owns FR-9.1-S1.

FR-8.2-S1 is an **extension of an inherited guard rather than a new test**: the scan
gained two needles — `NotifyContentWatch::watching` and `notify` — and the fixture
that names every door gained a line for each. Its expected report is derived from the
needle list, so the two arrived together by construction.

### Additional coverage

| Test | File | What it catches |
|---|---|---|
| `a_candidate_is_built_off_the_ticking_thread_and_no_tick_waits_for_it` | `reload_build_runs_off_the_tick_thread.rs` | **The phase's closing condition.** A candidate built on the tick thread, or built elsewhere with a tick waiting for it. Measured: it is the only test in the workspace that can see either |
| `the_clients_own_sources_put_the_root_it_plays_under_watch` | `the_client_watches_the_content_it_plays.rs` | **Red on arrival.** The shipped client never puts its content root under watch, so the capability this spec exists to deliver is unreachable. See the findings below |
| `a_scan_that_read_no_client_source_says_so_rather_than_reporting_an_unwatched_root` | same | that guard reporting "not reached" when it means "I could not look" |
| `the_same_scan_reports_a_client_source_that_does_put_its_root_under_watch` | same | a needle that cannot match the wiring even when it is present — the evidence that the red above is about the tree and not about a typo |
| `a_scan_that_could_not_read_the_pipelines_sources_says_which_ones_rather_than_reporting_clean` | `capture_pipeline_watches_nothing.rs` | a listed capture source renamed out from under the scan. Two files this guard reads were renamed inside this spec |
| `the_same_scan_reports_a_second_declaration_in_a_second_module_and_names_both` | `settling_window_declared_once.rs` | a second settling window in a second crate reading as one |
| `a_scan_whose_member_roots_hold_no_source_says_so_rather_than_reporting_one_declaration` | same | a walk that lost a member root while still reading hundreds of files from the other |
| the two needles added to the door fixture | `client_names_no_content_door.rs` | a watcher needle nobody has ever watched match anything |

### Arrival colours, measured

| Scenario | On arrival | Why |
|---|---|---|
| FR-8.2-S1 | **green the moment the needles compiled** | the client names no watcher door, and cannot name the vendor at all without a manifest entry |
| FR-8.2-S2 | **green the moment the scan compiled** | nothing on the capture path watches |
| FR-8.2-S3 | **green the moment the scan compiled** | it is FR-8.2-S2's control and is green by construction |
| FR-9.1-S2 | **green the moment the scan compiled** | phase 3 declared the window once |
| FR-9.1-S1 | **green** | the re-mesh transport ran off the tick thread before this spec touched it, exactly as `tasks.md` predicted |

**Nothing in this phase was red on arrival except a test no scenario asked for** —
the client-wiring guard, which is red because the product is. That is the whole
answer to "what did phase 6 make fail", and it is why the controls below are the
evidence rather than a formality.

### Every positive control, and the failing output it produced

Each was run by breaking the scan **by hand** — `production_text` filtered to return
nothing, which is the exact vacuity every one of these guards exists to refuse: a
scan that reads no text finds no needle and reports a clean tree. Reverted by hand;
`git diff --exit-code` clean; no `git checkout --`.

**1 — `client_names_no_content_door.rs`, under a scan that reads nothing.** The
headline assertion stayed **green** (`EveryContentDoorIsUnnamed`) and two controls
went red, the first naming all six doors including both new ones:

```
FAIL the_same_scan_reports_a_client_source_that_names_a_door_and_says_which_door_it_named
  left: EveryContentDoorIsUnnamed
 right: DoorsNamed(["src/startup.rs names `registry.apply(`", "src/startup.rs names `HudLayout::load`",
        "src/startup.rs names `BlockRegistry::new`", "src/startup.rs names `content_root`",
        "src/startup.rs names `NotifyContentWatch::watching`", "src/startup.rs names `notify`"])
FAIL a_door_named_in_a_sibling_unit_test_file_is_passed_over_and_the_module_beside_it_is_not
  left: EveryContentDoorIsUnnamed
 right: DoorsNamed(["src/startup.rs names `content_root`"])
```

**2 — `capture_pipeline_watches_nothing.rs`, same mutation.** The headline assertion
stayed **green** and its control went red:

```
FAIL the_same_scan_reports_a_capture_source_that_watches_and_says_which_door_it_named
  left: SourcesMissing([the seven listed files the fixture does not materialise, and crates/mc-testkit/src])
 right: WatcherDoorsNamed(["crates/mc-client/src/startup.rs names `ContentWatch`",
        "... names `watching_shipped_content`", "... names `ContentReload`", "... names `attach_reload`"])
```

**3 — `settling_window_declared_once.rs`, same mutation — and this one bit its own
headline assertion too**, which is the enumerated verdict paying for itself: a scan
that can no longer read a line answers `DeclaredNowhere`, and the good arm rejects it
for free.

```
FAIL the_settling_window_is_declared_in_exactly_one_production_source
  left: DeclaredNowhere
 right: DeclaredExactlyOnce { site: "crates/mc-world/src/content/watch/mod.rs" }
FAIL the_same_scan_reports_a_second_declaration_in_a_second_module_and_names_both
  left: DeclaredNowhere
 right: DeclaredIn(["crates/mc-sim/src/reload.rs declares it, on line 2",
        "crates/mc-world/src/watch.rs declares it, on line 2"])
```

**4 — `the_client_watches_the_content_it_plays.rs`, same mutation.** Both the headline
assertion and the can-it-see control went red on the same verdict while the
cannot-look control stayed green — which is the pair that tells a missing wiring apart
from a blind scan:

```
FAIL the_same_scan_reports_a_client_source_that_does_put_its_root_under_watch
FAIL the_clients_own_sources_put_the_root_it_plays_under_watch
  left: NotReached(["watching_shipped_content(", ".attach_reload("])
 right: TheReloadIsDrivenFromTheOneDoor
```

**The difference between control 3 and controls 1, 2 and 4 is worth keeping.** An
*absence* assertion cannot see its own blindness, so its control is the whole of its
falsifiability. A verdict that reports a **count** of what it found, and a
**presence** assertion, both redden on a blind scan by themselves. Where an
enumerated verdict is available it is strictly stronger, and this phase has one of
each to compare.

### The closing condition on this phase, measured

**`tasks.md` requires that a build running on the tick thread reddens something, or
that phase 6 escalates rather than closing. It reddens something.** The instrument is
`reload_build_runs_off_the_tick_thread.rs` and the shape is the one the condition
pointed at first: **identity rather than duration.** A build injected through the
product's own `ContentReload::building` door announces the `ThreadId` it began on,
waits to be released, and announces whether it was released or gave up waiting. The
test crosses one boundary, waits for the announcement, crosses **a second boundary
while the build is still blocked**, and only then releases it. Nothing here reads a
clock into an assertion and nothing counts boundaries.

Two mutations, run by hand on `crates/mc-sim/src/reload/mod.rs`, reverted by hand,
`git diff --exit-code` clean afterwards:

| Mutation | This test | Everything else |
|---|---|---|
| `begin_a_build` calls `build(&root, &spent)` **inline** and hands the result over through a trivial thread | **red**: `left: (OnTheTickingThread, [TakenUp])` | **green** — all four tests of `reload_builds_off_the_tick`, `require_a_build_in_flight` included, and FR-9.1-S1's new test |
| `collected` **joins** the builder instead of polling `is_finished` — phase 3's own blocking experiment | **red**: `left: (SomewhereElseButATickWaitedForIt, [TakenUp])` | **green** — the same five |

**The first row is stronger evidence than `tasks.md` predicted.** The inherited
fixture's own refusal — `require_a_build_in_flight`, whose doc comment says it does
not grade that no tick *waited* — also fails to grade a build that ran **inline**,
provided the outcome is still reported at the following boundary. So the reload
reports correctly, takes the candidate up correctly, puts the player exactly where an
independent run does, and runs the whole content read on the tick thread. One test in
the workspace can see it.

**The two arms are what make it a discrimination rather than a smoke alarm.** Row 1
answers `OnTheTickingThread` and row 2 answers `SomewhereElseButATickWaitedForIt`, so
the failure names which of the two defects it is.

### Where every number came from

| Number | Where it is stated | Derived how |
|---|---|---|
| the six doors, the four watcher doors, the two wiring spellings, the two window spellings | each guard's own `const` | chokepoints rather than type names, each with the reason it is one in the guard's header. Every fixture's expected report is **derived from the needle list**, so a needle and its fixture line arrive together |
| `crates/mc-world/src/content/watch/mod.rs` | `settling_window_declared_once::DECLARED_IN` | written out rather than discovered, because "declared in exactly one place" is a claim about *which* place: D2 sites the window with the port and the relevance rule, and a scan reporting wherever it found the value would agree with a declaration that had drifted |
| 256, twice | `reload_remesh::EVERY_SECTION_OF_THE_SHIPPED_WORLD` | the footprint's own two declarations, `FOOTPRINT_COLUMNS` squared times `SECTIONS_PER_COLUMN`. Never written as a literal, in the marking or in the scene's section count |
| 8 ticks | `reload_remesh_blocks_no_tick::TICKS_WHILE_IT_MESHES` | more than one, so a swallowed tick shows; few enough that a player dropped from 70 blocks has not reached a surface that tops out at 64 |
| 70 blocks | `reload_remesh_blocks_no_tick::IN_OPEN_AIR` | above `LANDMARK_TOP = 64`, the same spawn `reload_marks_sections.rs` uses and for the same reason: nothing a tick advances from there can edit a cell, so every section marked was marked by the reload |
| 40 settling windows | `reload_build_runs_off_the_tick_thread::WINDOWS_A_BUILD_WAITS` | the count `support/reload_watch.rs` already gives a whole run before it calls itself wedged, denominated in the one declared quantity rather than a number of its own. Reached only by a tick holding the build up; a healthy run spends one channel receive and one tick step getting to the release |
| every layer index | nowhere | the amber candidate is read through `candidate_against`, so the layer it takes is the session's answer and no test states it |

### Weak instruments, named

- **FR-9.1-S1** — a reload's whole-world re-mesh blocking no tick. Its property is
  held **structurally** and its own test cannot see it: `Session::tick` has no handle
  on the re-mesh worker at all — the collect lives in the frame path and is a
  `try_recv` — so `tasks.md`'s named mutation, "make `Session::tick` join the re-mesh
  worker before returning", **cannot be written without changing a signature.**
  Verified rather than assumed: `Session` holds no `Remesher` field and
  `collect_remesh` takes one as a parameter. This is the same shape as FR-6.2-S2 and
  S5 — unspellable rather than checked.

  **It is not inert, and that was measured.** Under `Simulation::adopt` nudging the
  player one block upward, the test reddens on the position bits alone with all four
  of its other components intact:

  ```
  left:  (([1091043328, 1116563046, 1091043328], 0, 0), 8, Accepted { holding: "base:amber" },
          EverySectionOfTheShippedWorld { marked: 256 }, Scene { sections: 256 })
  right: (([1091043328, 1116431974, 1091043328], 0, 0), 8, ...)
  ```

  So what it grades is that an accepted reload marking the whole world perturbs
  neither the player nor the tick count — which is worth having and is not what its
  wording promises. The property its wording promises is graded by the closing
  condition's instrument for the *other* transport, and by nothing for this one.

- **FR-8.2-S1's vendor needle** is weaker than the other five, in a stated direction:
  `mc-client` does not depend on `notify`, so Rust's extern-crate rules hold that half
  already. What the needle adds is a manifest entry and a use arriving together, and
  the adapter reached through `mc-world`'s re-export — which compiles perfectly well
  and is the reachable half.

### The vacuity these scenarios are shaped against, and how each closes it

| Scenario | The pass that would be worthless | What closes it |
|---|---|---|
| FR-8.2-S1 | a scan that reads nothing reports a clean client | the fixture control naming all six doors, the two filter controls, and the `NoClientSourceWasRead` arm |
| FR-8.2-S2 | the same, over the capture pipeline | FR-8.2-S3 **is** the control, and the missing-source arm reports a listed file renamed away |
| FR-8.2-S3 | a control that cannot tell a clean pipeline from an unread one | the third test: an empty tree answers `SourcesMissing([every listed source])`, never a clean report |
| FR-9.1-S2 | `assert!(sites.len() == 1)` over a walk that read one file | four enumerated arms: a blind scan answers `DeclaredNowhere` and a lost member root answers `RootsThatContributedNothing`, and the good arm rejects both |
| FR-9.1-S1 | an oracle that agrees because neither run moved | both players are falling, so position moves on every tick, and `require_moved` refuses a run that did not; the marking, the acceptance and the returned scene are asserted beside it so the re-mesh demonstrably happened |

### Fixtures this phase owns

- **`capture_pipeline_watches_nothing.rs`'s source list.** Eight files and one tree,
  each named, `crates/mc-testkit/src` walked whole. The precedence is stated in the
  header: a named door is reported ahead of a missing source, both being red.
- **`settling_window_declared_once.rs`'s member roots.** `crates` and `tools`, each
  counted on its own, following `no_hardcoded_block_names.rs` — whose header records
  why a total cannot see a root that contributed nothing.
- **The rendezvous in `reload_build_runs_off_the_tick_thread.rs`.** A `OnceLock`
  because `CandidateBuild` is a function pointer rather than a closure — the same
  reason the lost-worker fixture reaches for a static — and each test binary runs in
  its own process, so one rendezvous is one run.
- **No new support module.** Nothing in this phase is shared by two binaries, so
  nothing was put in `support/`.

### Two things phase 6 is barred from, and no test here asks for either

- **Nothing here greens SPEC-016's expiring pin** on the name-for-texture
  substitution. The amber declaration states `texture` equal to `name`, exactly as
  FR-4.3-S2 does and for the same reason.
- **Nothing here asserts an elapsed time, or a count of boundaries as a proxy for
  one.** The closing condition's own instruction, and phase 3's refusal of the same
  discriminator, are why the build's instrument asks for a `ThreadId` and an ordering
  instead.

### Findings this phase reports rather than resolves

1. **The shipped client never puts its content root under watch, so nothing this spec
   built is reachable by a mod author.** `Session::attach_reload` has no caller
   outside test fixtures — measured with `grep -rn attach_reload crates tools` —
   `App::collect_preparation` attaches a simulation and spawns the re-mesh worker and
   attaches no reload, and `main::run` moves its content root into
   `spawn_preparation` and keeps no copy. `spec.md`'s own capability paragraph ("leave
   `cargo run -p mc-client` running… save, and walk through stone") is unreachable,
   and no task in `tasks.md` owns the wiring. The guard this phase adds is red for
   exactly that reason and its two controls say the red is about the tree rather than
   about the needles. **What closes it is one call in `App::collect_preparation` plus
   keeping the root reachable there.**
2. **`require_a_build_in_flight` does not grade what its name suggests, in a second
   way its own doc comment does not name.** It records that it cannot see a tick that
   *waited*; measured here, it also cannot see a build that ran **inline on the tick
   thread**, provided the outcome is reported at the following boundary. Its header is
   a phase-3 file and this phase did not edit it; the fact is recorded here and in the
   closing-condition instrument's own header.
3. **A green clippy is no evidence about a lint, again, and the count was three.** The
   four new binaries compiled and every assertion in them ran before
   `cargo clippy --workspace --all-targets --all-features -- -D warnings` reported
   `excessive_nesting`, `too_many_lines` and `map_err_ignore` — none of which any test
   run could have shown. It also named three *separate* crate targets in one run
   rather than stopping at the first, which is the sibling half of phase 1's
   lint-reading rule.

### Strengthened after the phase closed — a content root in any form a caller hands over

**No new scenario, deliberately.** FR-1.1-S1 already says a declaration saved while
the simulation is running begins exactly one attempt. The tests that claimed it were
weaker than the scenario: every one of them handed the watcher a `tempfile`
directory, which is absolute, and **the shipped client hands over a relative root** —
`mc_sim::content::shipped_directory` is `["content", "base"].iter().collect()`,
carried unchanged to `watching_shipped_content`. `notify` reports absolute paths, the
relevance rule strips the root *as given*, every save was classified as not being
content, and hot reload did nothing at all in the shipped game with 1 188 tests
green. The owner found it by playing.

| Scenario strengthened | File | Test |
|---|---|---|
| FR-1.1-S1 | `crates/mc-client/tests/reload_takes_up_a_save_under_a_relative_root.rs` | `a_save_under_a_relative_content_root_is_taken_up_exactly_as_one_under_an_absolute_root` |
| FR-1.1-S1 · FR-1.1-S5 · FR-1.1-S7 · FR-1.1-S9 (the relevance rule they all rest on) | `crates/mc-world/tests/content_watch_root_forms.rs` | `a_saved_declaration_is_content_whatever_form_the_root_was_handed_over_in` |

**Additional coverage — what these catch:** *a content root in any form a caller may
hand over, because the vendor's path spelling is not the caller's to know.* The
client-level one asserts the accepted swap and what it changed — one attempt and
stone no longer solid — for a relative root against an absolute one; the port-level
one asserts the same directory watched three ways: absolute, relative to the
binary's own working directory, and the relative spelling with a leading `./`,
forward slashes and a trailing separator.

**The family, named because it is the one this project has paid for most often:**
this is `standards/global/testing.md`'s *policy is not wiring* one layer down. The
wiring landed — `App::collect_preparation` really does put the root under watch —
and **the thing it wires is a no-op**. What hid it is narrower than the usual
diagnosis and is the part worth carrying: *a fixture that supplies its input in a
form no caller uses*. Every root in this suite was absolute because `tempfile` makes
absolute directories, so the two spellings always agreed and no assertion anywhere
could tell them apart.

**A later reader will see three near-identical root forms and read them as
redundant.** They are not, and each has a different job: the absolute form is the
control that says the fixture works; the relative form is what the game hands over;
the dotted form is what makes the repair a rule about paths rather than a special
case for one spelling. Measured, not argued — see the next paragraph.

**What the vendor actually does, measured while writing these.** It does not
canonicalise the root; it **concatenates the caller's spelling onto the working
directory**, verbatim. Watching `..\..\target\x` reports
`E:\_PROJEKTE\MyCraft\crates\mc-world\..\..\target\x\blocks\probe.luau`; watching
`./../../target/x/` reports the same path with `./../../target/x/blocks` embedded
mid-path, forward slashes and all. **That is why the dotted form is load-bearing:** a
repair that absolutised the root at the call site with `current_dir()?.join(root)`
would accidentally reproduce the un-normalised prefix for the plain relative form and
go green there, while the dotted form stayed red. `fs::canonicalize` fails the other
way — it yields the `\\?\` verbatim form the vendor never reports.

The `\\?\` form is deliberately **not** among the three: no caller in this tree
produces one, so asserting on it would pin behaviour the spec does not state.

**Both tests hand the root to the adapter itself**, which is what makes them reject a
call-site repair by construction: absolutising the root in `main` or `App` changes
nothing either test does.

**One line to adapt when the repair lands.** `content_watch_root_forms.rs` names the
relevance rule in exactly one place — `calls_it_content(root, reported)` — so the
ruled repair (the adapter reporting paths relative to the root it was constructed
with, and `declares_content` becoming a predicate over a root-relative path) is one
line here rather than three assertions. **The repair changes `declares_content`'s
signature, so it cannot land without a test-file change** — `content_watch.rs`'s
listing test calls the two-argument form as well. Those edits are the test author's,
not the implementer's.

**Both are real-filesystem tests, and the constraints on them are stated because
they are easy to violate.** Neither ever calls `std::env::set_current_dir`: it is
process-global and this suite runs in parallel, so a chdir corrupts its neighbours
and the corruption reads as a flake. The relative spelling is computed from two
absolute paths instead. The scratch roots live under the repository's own gitignored
`target/` because a relative spelling has to *exist* — on Windows no relative path
crosses volumes, and a system temporary directory is routinely on another drive. Each
run's directory carries the process id and a timestamp and is removed by a `Drop`
guard, so a firing assertion does not accumulate trees; the empty parent directory
stays, because removing it would race a concurrent run.

**While the defect stands the client-level test spends its whole fifteen-second
patience on the relative half.** That is the shape of the red and not a hang — the
absolute half reports in about two hundred milliseconds and the assertion names both.

### The fixture bounds, split by what they deny — and the four flakes they caused

**Four reload fixtures failed only under the gate's own load**, each passing alone and
alone under coverage: `a_material_file_begins_no_attempt_while_the_same_watcher_begins_one_for_a_declaration`,
`a_corrected_declaration_is_taken_up_by_the_next_attempt_after_a_refusal`, and
`the_ticks_a_whole_world_re_mesh_runs_over_are_the_ticks_a_run_with_no_reload_advances`.
A different subset failed on each of four instrumented runs. **Not quarantined and
not deleted** — the subject is the capability this spec delivers, and parking its
timing tests would be the same failure the owner caught by playing.

**One defect, in three places: a bound that denies something was as short as a bound
that waits for something.**

| Bound | Kind | Where | Was | Is |
|---|---|---|---|---|
| a wrongly-begun attempt has ended | **minimum** | `support/reload_watch/runs.rs` | `SETTLING_WINDOW × 2` = 300 ms | `AN_ATTEMPT_MAY_NOT_OUTLAST` = 1.5 s |
| no *second* attempt is following | **minimum** | same | `SETTLING_WINDOW × 2` = 300 ms | the same 1.5 s |
| a run waits for the attempt it expects | **maximum** | same | `SETTLING_WINDOW × 40`, and the quiet timer started at the *first boundary* | `A_RUN_EXPECTING_ONE_MAY_NOT_OUTLAST` = 6 s, and the quiet timer starts at the first **report** |
| a collect waits for the re-mesh worker | **maximum** | `support/reload_remesh.rs` | `200 µs × 256 × 20` = 1.02 s | `A_COLLECT_MAY_NOT_OUTLAST` = 15 s |
| a blocked tick may hold a build | **maximum** | `reload_build_runs_off_the_tick_thread.rs` | `SETTLING_WINDOW × 40` | its own 6 s |
| a real save reaches the watch | **maximum** | both real-filesystem suites | 15 s, and a `SETTLING_WINDOW × 10` tail | 15 s, and the shared 1.5 s minimum for the tail |

**No test bound is denominated in `SETTLING_WINDOW` any more, and that was the actual
defect.** That constant is production policy — how long an editor's save is given to
settle — while a fixture bound is a statement about how slow *this machine* may be
while the suite runs. Sharing one number made a minimum and a maximum move together,
so the minimum was short exactly where it had to be long. `grep SETTLING_WINDOW` over
`crates/mc-client/tests` now returns nothing; `mc-world`'s `content_watch.rs` still
names it because there the window **is** the subject.

**Every bound states which direction it came from**, per `testing.md` §2, and none was
reached by loosening until green:

- **1.5 s, from below:** measured. Under the gate's instrumented run of 1 190 tests
  with the GPU suites in flight, an attempt had **not** completed inside 300 ms, while
  the same test completed in 0.70 s alone and 0.73 s under coverage. From above:
  nothing about correctness — waiting longer cannot turn a wrongly-begun or a second
  attempt into a pass, because what is asserted against it is that neither happened.
  The ceiling is runtime alone, and it is spent on every such run.
- **15 s for a collect, from below:** measured. A 256-section batch did not finish
  inside 1.02 s under `cargo llvm-cov nextest` over the workspace, against ~35 ms
  uninstrumented. How far above 1.02 s is *unknown*, because the run that exceeded it
  never completed — which is the argument for a wide multiple. From above: nothing, and
  it costs **nothing on a passing run**, because the loop returns the moment the worker
  answers.
- **6 s for a run's patience, from below:** four times the 1.5 s an attempt may take.
  From above: reaching it produces an empty attempt list, which is what every
  assertion compares against a non-empty one — so it can never make a green.

**The presence halves poll to a deadline; they do not sleep.** `until_settled`'s quiet
stretch now starts at the first *report* rather than at the first boundary, so a run
gives up only after `A_RUN_EXPECTING_ONE_MAY_NOT_OUTLAST` with nothing seen. The
port-level root-forms test returns the moment a reported path is content instead of
collecting for a fixed stretch. A fixed wait after the first event is what turns a
maximum into an equality against machine load, which is why these failed only when the
machine was busy.

**Measured, not argued.** A fixture with an injected build sleeping 1.2 s — four times
the old bound — reproduces the failure exactly (`left: []`, the gate's own signature)
under the old shape and passes under the new one:

```
old shape: FAIL [0.425s]  left: []   right: [TakenUp]
new shape: PASS [1.926s]
```

**What it costs, measured:** `mc-sim` + `mc-client` + `mc-world` went from 101 s to
126 s for 716 tests. That 25 s is the five absence runs and every presence run's tail
paying the raised minimum, and it is the price of closing the direction that fails
silently.

**One deviation from the split as it was first proposed, found by applying the
both-directions rule to it.** The proposal was three numbers: a minimum for the
absence half, a short tail for "no second attempt", and a generous maximum. The middle
one is **also** an absence bound — it denies a second attempt — and a second attempt
has to *build*, so a debouncer window is only how long its *report* would take to
arrive. It therefore shares the measured minimum rather than keeping a two-window
tail, which is where a silent false green would otherwise have stayed.

**A size limit forced a split, and the file's own header had named the seam.**
`support/reload_watch.rs` reached 630 non-blank against the 600 limit, so the run
loops and their bounds moved to `support/reload_watch/runs.rs` (226 lines) and are
re-exported, leaving `reload_watch.rs` at 433 and **no consumer changed**. Its header
already separated "how long a run has to be" from the double, the vocabulary and the
fixtures. `documented_refusals.rs` split the same way and for the same reason — see
below.

### The four reload refusals a page may quote, and the one string production does not hand over

**T26 asked for every refusal an author can meet to be quoted and held to a live run.
Four reload refusals now are**, produced through a running client: a watch reports a
change, a boundary collects the attempt, and the words come out of
`Session::take_reload_report` — the product's own output, not a rendering the fixture
performed.

| # | What the author did | The refusal's own words, beneath the reload's sentence |
|---|---|---|
| 5 | saved a typo (`slid` for `solid`) | `…blocks/stone.luau, block \`base:stone\`, field \`slid\`: …` |
| 6 | deleted two declarations the world holds | `the world holds \`base:grass\` and \`base:stone\` that this content does not declare` |
| 7 | took the solidity off everything | `the content registers no solid block, so a player would have nothing to place; …` |
| 8 | ran out of texture layers | `…needs 257 texture layers and a session has 256; …` |

`RootUnwatchable` and `BuilderLost` stay prose on the page and outside the
recogniser, on the page's own rule: **a refusal an author meets by making an ordinary
mistake is quoted and held; one they meet because their filesystem is unusual is
described.**

**The producers moved to `support/printed_refusals.rs`** — `documented_refusals.rs`
was at 567 of 600 non-blank and the producing half is what grows, while the comparing
half is the guard. The file's own header had already named that seam.

**One string in the fixture is a copy, and it is held mechanically rather than by
prose.** `App::report_reload` composes its line in an `eprintln!` inside a file
nothing in this workspace can run, so there is no function to ask for the sentence
above the chain. Everything else comes from production — the chain from the client's
own report, the joiner from `Ending::failed_under`, the prefix from
`mc_render::window::report` — but `NOT_TAKEN_UP` is a copy, and **a page held to a
copy is a page holding hands with a test.** So
`the_sentence_a_refused_reload_is_read_under_is_the_one_the_client_prints` requires
that spelling to appear in the source that prints it, over text with its line
continuations joined and its whitespace collapsed so formatting cannot break it. It is
a *presence* assertion, so a rewording reddens it rather than greening it.

**Two lines of production would retire that scan** — the sentence as a `pub const` and
`App` using it — and it is still owed. What neither shape closes is `App` ceasing to
print at all, which needs a window: the same residual `main.rs` accepts and
`shipped_binary.rs` closes for the launch path.

### An instrument that retired, and one that was owed

**The sentence scan is gone, and the reason it existed is worth keeping.** For one
commit, `support/printed_refusals.rs` carried its own copy of the sentence a refused
reload is read under, because `App::report_reload` composed it inside an `eprintln!`
nothing in this workspace can run. A page held to a fixture's copy of a sentence is a
page holding hands with a test, and neither would notice the program disagreeing with
both — so the copy was held by a scan requiring that spelling in the source that
printed it. `mc_client::session::reload::CONTENT_NOT_TAKEN_UP` now exists, the fixture
imports it, and **the scan is deleted rather than kept**: an instrument that exists
because a string had two homes retires when it has one. It was the weaker instrument
standing in until the stronger one existed, which is the same relationship
`client_names_no_content_door.rs` records with the dependency-closure guard.

Every part of a reload refusal's line now comes from production: the chain from the
client's own report, the sentence from that constant, the joiner from
`Ending::failed_under`, the prefix from `mc_render::window::report`.

### The nested-deeper clause had no witness, and now has one

`declares_content`'s own doc says a declaration nested one directory deeper is not
content, "because it is not something either loader reads". **Nothing asserted it.**
Measured twice, independently: with the rule changed to accept an ancestor rather than
the parent, I saw **10 of 10** candidate tests pass and the implementer saw **316 of
316 in `mc-world` and 87 of 87 in the reload family** pass. Why each near neighbour
misses is worth recording, because all three look like they cover it:

- the shipped-root listing oracle walks a root that **holds no nested declaration**, so
  the property is true of its fixture by accident rather than asserted;
- the editor's-scratch-file test varies the **extension**, not the depth;
- the root-forms test varied the root's **spelling**, not the depth.

The witness is now the third of those: each of the three root forms writes **two**
declarations — one directly under `blocks/` and one under `blocks/experiments/` — and
the verdict is `OnlyTheDeclarationADirectoryDeepIsContent`. Measured under the same
mutation: `[TheNestedDeclarationToo × 3]`, green after a hand revert, `git diff
--exit-code` clean.

**It is at the port and not at the client, deliberately.** A client-level "a nested
declaration begins no attempt" test is an **absence needing a window** — the flake
shape this spec has now paid for four times. At the port both halves are decided on
paths that **demonstrably arrived**: the run polls until both saves have been reported
and only then classifies, so the question is not "was the nested path never called
content" but "the nested path arrived, and was it". No window, no load sensitivity.
That the client consults the rule at all is separately witnessed by FR-1.1-S9.

### An absence two causes reached, one level in

`Collected::NothingArrived` could not tell *the worker has not finished* from *the
worker is gone*: `Remesher::collect` answers `None` for an empty channel **and** for a
disconnected one, and on the second it also clears the busy flag — so a worker thread
that ended takes the channel with it and every later collect answers "nothing yet"
forever, however long anybody waits. Both `Collected` and `Handled` now split it into
`StillMeshing` and `TheWorkerIsGone`, decided by asking `Remesher::is_free()` once
after the wait. **Waiting longer is the repair for one of them and no repair at all for
the other**, which is why one arm could not serve both.

What that leaves for the product: a client whose re-mesh worker dies stops showing
edits with no report of why, and `Remesher::submit`'s own doc already says so. The
instrument for it would be `collect` distinguishing a disconnected channel from an
empty one — deferred, and recorded with the other deferred shapes rather than built at
the end of a phase.

**Measured while diagnosing it:** the whole-world re-mesh test passes **alone under
`cargo llvm-cov nextest` in 4.18 s**, against 0.29 s uninstrumented. So instrumentation
alone does not reproduce the gate's failure — it needs the concurrent workspace run,
which is what makes a dead or starved worker the live hypothesis rather than a slow one.
The new arms will name it next time.

### A bound derived from the run rather than from a literal

`reload_takes_up_a_save_under_a_relative_root` reached its 15 s maximum inside a full
concurrent run while taking 3.5 s alone — a literal measured wrong in the direction the
ruling warns about. The second half's patience is now **twenty times what the control
half took**, floored at the old fifteen seconds. The absolute half does the same work at
the same moment on the same machine — a real watcher, a real debounce, a real build — so
its own time-to-report is the only honest scale for the other's. From above there is
nothing: a passing run returns at its first report, and under the defect this test
exists to catch **no** patience produces an attempt, because the save is classified as
not content forever.

### Additional coverage, two results measured by the implementation and recorded here

- **`uploaded_to` returning the layers without writing to the device reddens FR-4.3-S3
  alone**, at `strayed == 576` / `shown == 0`. So that scenario grades the *upload* and
  not merely obtaining the value — which holds only because the fixture drives the frame
  path's own call rather than cloning the borrow out. Had it cloned, `uploaded_to` would
  have had no caller anything in this workspace runs and the mutation would have bitten
  nothing.
- **The relevance rule's nested-deeper clause was unwitnessed**: accepting an ancestor
  rather than the parent left 403 tests across `mc-world` and the reload family green.
  Closed above, and the mutation now reddens exactly the one test whose subject it is.

### The absence became a production verdict, and what the fixture now says

`Remesher::collect` answers `Collecting::{NothingYet, WorkerGone, Finished}` instead of
`Option<Remeshed>`, so the fixture no longer infers a gone worker from `is_free()` after
a spent patience — **the channel says it on the first ask**. `collected` and `handled`
therefore report three absences rather than two:

| Arm | What it means | Who is at fault |
|---|---|---|
| `StillMeshing` | the patience is spent and the worker holds the batch | the machine, or the bound |
| `TheWorkerIsGone` | the channel is gone; said immediately | the client |
| `NothingWasEverHandedOver` | the patience is spent and the worker holds nothing | **the fixture** — nothing was submitted |

The third exists because the second no longer needs it: a channel that had gone says so
before the loop, so what is left after the loop is a batch nobody handed over. A run
meeting it should be read as "this scenario submitted nothing", never as a broken
client — which is the reading the single old arm made impossible.

**The gate failure that started this was never captured, and that is recorded rather
than glossed.** It failed at 28.6 s in one instrumented workspace run, passed alone
under coverage at 4.1 s, and passed in the next instrumented workspace run — the
widened bound landed between them and the summary lines were the only thing kept. What
is solid is that it is load-dependent and marginal.

**One hypothesis is ruled out by the evidence already in hand.** A stack overflow in
`rebuild_batches → rebuilt → splice → scene_of` — the deepest chain in the client,
packing 256 sections in one call — **aborts the process** rather than unwinding, so
nextest would have reported a signal and not an assertion. The process survived and the
assertion fired, so the remaining candidates are a panic that unwound (which leaves a
message in the captured stderr) and plain starvation past the bound. **Under the old
`Option` those two were indistinguishable; under `Collecting` they are not**, and the
next occurrence separates them at no extra cost: the arm names the first and the
captured stderr names the second. Nobody needs to re-run anything speculatively to get
that reading.

---

## Phase 8 — A candidate outside the loaded world is not eligible

**1 scenario, 1 test, plus its mandatory paired control.** Test command:

```
cargo nextest run -p mc-client -E 'binary(/^reload_will_not_clear_a_player_off_the_map$/)'
```

| Scenario | File | Test |
|---|---|---|
| FR-7.1-S8 | `crates/mc-client/tests/reload_will_not_clear_a_player_off_the_map.rs` | `a_reload_that_traps_a_player_at_the_world_edge_leaves_them_rather_than_putting_them_outside` |

**Additional coverage — the paired control, mandatory rather than thorough:**
`the_same_wedge_in_a_world_wide_enough_moves_the_player_to_a_position_inside_it`.
S8 asserts a **refusal**, so a search that finds nothing ever satisfies it — deleting
the candidate generator, or making every candidate ineligible, would leave it green. The
control is the same wedge in a two-column world with exactly one clear position four
blocks away, past three rings of blocked ground, and it requires the move. **It is green
today and must stay green**, which is what stops "outside is ineligible" being
implemented as "nothing is eligible".

### RED, measured

```
FAIL a_reload_that_traps_a_player_at_the_world_edge_leaves_them_rather_than_putting_them_outside
  left:  (MovedOffTheMap([3204448256, 1092616192, 3204448256]),
          At { feet: [3204448256, 1092607454, 3204448256], velocity: [0, 3204448256, 0] }, Some(true))
  right: (ToldNothingWasClear { blocks: 8 },
          At { feet: [1075838976, 1092616192, 1075838976], velocity: [0, 0, 0] }, Some(true))
PASS the_same_wedge_in_a_world_wide_enough_moves_the_player_to_a_position_inside_it
```

Decoded, those bits are the defect in one line: the player standing at
**(2.5, 10.0, 2.5)** is moved to **(-0.5, 10.0, -0.5)** — outside a world sixteen blocks
square — and one tick later they are at **(-0.5, 9.9917, -0.5)** with velocity
**y = -0.5**. They are not merely off the map; they are already falling out of it. The
reload itself stood (`Some(true)`: water is solid now), which is why nothing else in the
suite notices.

### Why the destination is asserted and "moved" is not

Today's behaviour **is** a move, so an assertion that the player moved is satisfied by
the defect, and one that they did not move is satisfied by a search that stopped working.
What is compared is an enumerated verdict over *where*: `MovedOffTheMap` and
`MovedInsideTheWorld` are different answers and the failure names which arrived. The
classification decodes the destination the client reported and bounds it against the
footprint — **an oracle sharing no code with the search**. Only the horizontal axes are
asked about, and that is exact rather than approximate: every destination the search takes
is a cell centre, so the 0.6-wide box spans `[c + 0.2, c + 0.8]` and lies inside the one
column its centre is in.

### Driven through the reload, and why not against `cleared`

Every reading is the client's own report, as phase 5's clearing scenarios are. The world's
extent has to reach the search for this to be fixable, and **how** it gets there is the
implementer's choice — a test written against `cleared`'s arguments would fail to compile
until they made it, and a compile error is not a RED for a behaviour scenario.

### The fixture's three premises, each a guard

- **the reload is what traps the player** — `require_the_reload_traps`, inherited;
- **some position the search may look at lies outside the world** — this scenario's own
  premise, and without it the fixture is about a wedge rather than about a boundary, which
  is the refusal FR-7.1-S5 already grades;
- **every position inside the world is blocked** — or the search would rightly find one.

The one-column world is what makes the second true: sixteen blocks across is **narrower
than the search is wide**, so a player two blocks from a corner has most of the cube
outside. The filter that keeps the fill inside the footprint is not a convenience — a
write past an edge is refused, rightly — and the cells it drops are exactly the ones the
search reads as clear because nothing is loaded there.

### No new vocabulary, and that is the ruling

A boundary wedge with no eligible candidate takes the refusal path FR-7.1-S5 states and
FR-7.1-S6 grades. Treating outside as *solid* was refused — a lie in a model that
collision, meshing and physics all read, and it inverts the moment the world streams — as
was declining to clear an off-map player at all.

### Additional coverage — the far edge, and why it is not the near edge written twice

| Test | What it catches |
|---|---|
| `a_reload_that_traps_a_player_at_the_far_edge_refuses_the_ground_past_it_too` | **that eligibility refuses ground past the far edge and not merely ground at negative coordinates** |

`holds` refuses a candidate in two steps: a negative coordinate names nothing the world
holds, and only then is the extent asked. **FR-7.1-S8's own fixture exercises the first
step alone.** Its player stands at `(2.5, 10.0, 2.5)` with a reach of eight, so the
candidate cube spans `[-6, 10]` against an edge at sixteen and never reaches the far
side — every out-of-world candidate it meets is *negative*, and the sign check carries
the whole scenario by itself.

The far-edge test is the same wedge at `(13.5, 10.0, 13.5)`: the cube spans `[5, 21]`,
**nothing in it is negative and everything outside the world is positive**, so only an
extent can refuse it. Its fixture guards both halves of that premise —
`require_every_candidate_outside_is_past_the_far_edge` fails if any candidate is negative
*or* if none is past the edge — because a fixture that drifted either way would silently
become the near-edge test again.

**It could not be red first and its non-vacuity is established the other way.** The
implementation already handled the far edge, so the test was green the moment it
compiled. Re-running the third mutation — `ground.contains` short-circuited to `true`
with the sign refusal kept, which is what left 1 192 of 1 192 green before this test
existed — reddens **exactly this test and nothing else**:

```
Summary [112.968s] 718 tests run: 717 passed, 1 failed
FAIL a_reload_that_traps_a_player_at_the_far_edge_refuses_the_ground_past_it_too
  left:  (MovedOffTheMap([1099169792, 1092616192, 1093140480]),
          At { feet: [1099169792, 1092607454, 1093140480], velocity: [0, 3204448256, 0] }, Some(true))
  right: (ToldNothingWasClear { blocks: 8 },
          At { feet: [1096286208, 1092616192, 1096286208], velocity: [0, 0, 0] }, Some(true))
```

Decoded: the player at **(13.5, 10.0, 13.5)** is moved to **(16.5, 10.0, 10.5)** — past
an edge at sixteen — and one tick later is at **(16.5, 9.9917, 10.5)** with velocity
**y = −0.5**, falling out of the world. The reload stood. Hand-reverted; `git diff
--exit-code` clean.

**Why the shipped game is the reason this exists rather than symmetry.** The shipped
world is 64 blocks square, so a player near its far edge has positive out-of-world
candidates. Had `contains` been wrong, they would be teleported off the map exactly as
the near-edge player was — the same live defect on the other side, and nothing in the
suite would have said so.
