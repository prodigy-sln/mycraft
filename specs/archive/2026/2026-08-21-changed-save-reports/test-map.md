# Test map — SPEC-020

Scenario → test. Test names are behavioural and carry no scenario id; this file is
the only place the mapping lives.

| Scenario | Test | File |
|---|---|---|
| S1 | `a_save_whose_blocks_were_redeclared_is_played_with_nothing_on_the_command_line` | `crates/mc-client/tests/launch_acceptance.rs` |
| S2 | `the_same_comparison_reports_a_block_whose_declaration_the_content_no_longer_holds` | `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` |
| S3 | `a_save_whose_two_blocks_behave_differently_is_named_one_line_ascending` | `crates/mc-client/tests/changed_blocks_named_on_the_error_stream.rs` |
| S4 | `a_save_whose_blocks_all_still_match_is_loaded_with_nothing_said_about_them` | same |
| S5 | `a_save_whose_block_only_looks_different_is_loaded_with_nothing_said_about_it` | same |
| S6 | `a_launch_with_no_save_to_read_generates_a_world_and_says_nothing_about_blocks` | same |
| S7 | `the_shipped_binary_over_a_save_whose_block_behaves_differently_names_it_on_its_error_stream` | `crates/mc-client/tests/shipped_binary.rs` |
| S8 | `a_save_whose_blocks_were_redeclared_is_refused_when_the_player_asked_for_strictness` | `crates/mc-client/tests/launch_acceptance.rs` |
| S8 | `the_shipped_binary_told_to_refuse_a_changed_save_leaves_it_shut_and_says_why` | `crates/mc-client/tests/shipped_binary.rs` |
| S9 | `a_save_whose_blocks_all_still_match_is_played_even_when_strictness_was_asked_for` | same |
| S10 | `an_argument_one_letter_short_of_the_real_one_is_not_the_real_one` | same |
| S11 | `the_refusal_names_the_argument_that_caused_it_as_the_thing_to_drop` | same |
| S12 | `the_shipped_content_declares_water_unbreakable_and_stone_breakable_still` | `crates/mc-sim/tests/shipped_water_is_not_broken_and_is_built_through.rs` |
| S13 | `a_placement_into_the_shipped_water_replaces_it_with_the_block_being_placed` | same |
| S14 | `the_shipped_content_reports_water_as_behaving_differently_and_the_other_three_as_retextured` | `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` |
| S15 | `the_committed_pre_luau_save_names_water_and_no_other_block_against_the_shipped_content` | `crates/mc-client/tests/changed_blocks_named_on_the_error_stream.rs` |

## S12 was reworded, and the measurement is why

S12 asked for a break at a water cell to be **refused** with the water left in
place. It cannot be: `targeted` (`crates/mc-sim/src/world/action/trace.rs`) returns
a hit only where `is_solid` answers true, water declares `solid = false`, so the ray
walks through the water cell and `broken` is never called on one. Measured against
the shipped content before anything was implemented — a break aimed at the water
cell answered `Changed { cell: (9,10,8), from: Holds(base:stone), to: Empty }`.
`Refusal::Indestructible` is unreachable for a non-solid block whatever the
declaration says.

Ruled by the project owner: assert the declaration against the registry the running
game resolves names against, which reddens the moment the line is reverted, and
record the fuse. The break's real answer is asserted as
`a_break_aimed_through_the_shipped_water_reaches_the_solid_block_behind_it`, which
**goes red the day water becomes targetable** — and the change that does that owes
the scenario S12 could not be. That handover is written on
`Refusal::Indestructible` and on `targeted`, where somebody changing targetability
reads, rather than only here.

The spec's claim that "a player can no longer break water" was false and is not
what shipped. The player capability this spec delivers is the other one: a world
that opens after a content update instead of being refused, with the changed blocks
named.

## Additional coverage

Each of these catches something the scenario table's floor does not.

| Test | What it catches |
|---|---|
| `the_same_save_loads_by_default_and_is_refused_naming_water_alone_when_strictness_is_asked_for` (`mc-world`) | A revision byte shared between the two hash lists. The accepting arm returns `None` for *any* changed list, so it cannot see this; the strict arm names the blocks and so disagrees with a shared byte by three names. The old test here asserted only the accepting arm's predecessor, and flipping its argument would have produced a test that passes however badly the fold breaks. |
| `every_changed_block_is_named_in_the_order_it_was_given_separated_once_each` (`notice_test.rs`) | A join that drops the last name, emits a trailing separator, or reverses the order. No fixture in the suite's launches produces a three-name list, so two names is the widest the scenarios reach and none of those defects is visible at one separator. |
| `one_changed_block_is_told_in_the_singular` (`notice_test.rs`) | A single sentence that reads correctly for the plural. One block is the common case and "blocks no longer behave" is wrong about it. |
| `the_composer_says_nothing_for_a_list_holding_nothing` | That the emptiness rule lives in the composer rather than in the loads that happen to hand it empty lists. A list held by no save at all is an input the launch-driven readings cannot supply. |
| `neither_run_of_the_binary_touches_the_save_it_read` | **The property `--refuse-changed-blocks` exists for**: somebody leaves a doubtful world shut precisely because opening it and quitting rewrites the hashes that made it doubtful, so a refusing run that wrote anything would destroy what it was asked to preserve. It covers the accepting run too, which is killed mid-startup on every pass and must not corrupt its own fixture. Safe because `opened_with_length` calls `File::open` and nothing else, and the only write is `ending_after_saving`, which saves on a clean close alone and so is reached by neither a killed child nor a refused launch — checked here rather than argued. |
| `a_break_aimed_through_the_shipped_water_reaches_the_solid_block_behind_it` | See above: the fuse for PRO-904. |
| `the_shipped_content_declares_water_unbreakable_and_stone_breakable_still` asserts stone too | A reader that answered `false` for every block would satisfy the water half on its own. |

## Falsification

Displayed red before implementation, in two passes, because an empty-output
skeleton passes an absence scenario for the wrong reason.

- **Inert skeleton** (composer returns `None`; the load drops the verdict; the flag
  constant renamed while its meaning stays) — **1385 run, 1372 passed, 13 failed**,
  every failure an assertion failure rather than a compile error. Covered S1, S3,
  S7, S8's sibling readings, S10, S11, S12, S14, S15 and the two composer unit
  readings.
- **Over-eager skeleton** (composer returns `Some` for any input including empty;
  the load reports every name the save carries; the parse always accepts) — the five
  remaining readings red: S4, S5, S6, S8 and the empty-list reading.

Mutations after green, each reverted by hand with `git diff --exit-code` confirmed
clean afterwards. Baseline 1385 passed, 0 failed, 1 skipped for the first four; 1386
for the fifth, which was run after the S8 subprocess reading was added.

| Mutation | Suite | Read as |
|---|---|---|
| `judge` puts a retextured block in `changed` | **7 failed** — S5, S14, S15, S7, and `save_changed_blocks`'s own retexture reading | Bites. This is the shape the two revision bytes exist to prevent, and S5's red comes from the load rather than from the composer. |
| `refuses` drops the changed-list emptiness guard, so strictness refuses unconditionally | **50 failed**, including S9 | Bites. S9 had no red of its own — it is a scenario about a save nothing is wrong with — and this is the only thing that reddens it. |
| `water.luau` loses `replaceable = true` | **2 failed** — S13 and `shipped_blocks_are_declared_in_luau` | Bites. S13 asserts pre-existing behaviour and could not be driven red by a skeleton. |
| `main`'s `run` passes `Acceptance::ChangedBlocksToo` instead of `acceptance_from(std::env::args())` — the binary ignoring its own `argv` | **1 failed — the S8 subprocess reading alone; 1385 of 1386 green** | Bites, and it is the reason that reading exists. No library-level test looks at what the *process* does with its arguments, so the binary could stop honouring `--refuse-changed-blocks` entirely and everything else would stay green. Verdict `LoadedAndNamedTheBlockAsANotice`, with the notice the child wrote in the failure message. |
| `prepare_launch` stops calling `say_changed_blocks` | **1 failed — S7 alone, 1384 of 1385 green** | Bites, and this is the measurement the whole shape of S7 rests on. Every other reading of the line reaches the composer through a launch or calls it directly, which is agreement between two copies of one decision. Only the subprocess can see the wire. |

## The refusing side needed a process too, and it cost something

S8 was witnessed only through `simulation_to_play` — a library call. The mutation
table above is why that was not enough: the binary can stop consulting its own parse
and nothing in the library sees it, and this is the highest-consequence path in the
spec, since somebody who asks for a doubtful world to be left shut and has it opened
anyway loses the very evidence they were protecting.

**Two things about it were not what they looked like, and both were measured rather
than assumed.**

- **It needs a device.** A refusal over the *save* is discovered on the preparation
  worker and surfaces only where the frame path collects it — `App::collect_preparation`,
  inside a redraw — so the child has to reach a window before it can say anything
  about the save. Unlike S7's reading, this one cannot be device-free. A machine with
  no device answers `NeverGotAsFarAsReadingTheSave`, which **fails** rather than
  skipping: an absent instrument and a clean one must not look alike.
- **`Command::output()` is not safe here.** Under the `argv` mutation the child loads
  the world and its window never closes, so `output()` wedged for **606 seconds** and
  the run had to be killed by hand. Both binary readings are bounded by `PATIENCE`
  now; the same mutation then answers in about a second and names the defect.

The reading buys one thing beyond S8: it is the only test in the workspace that
covers the way-out sentence reaching a real terminal, since `App::redraw` is its one
production line. `docs/technical/testing.md` recorded that as uncovered and now
records what changed it.

## What no test here covers, stated rather than left silent

- **The laundering-on-quit property.** A quit rewrites the save's hashes against the
  running registry, so the report is a one-shot and `--refuse-changed-blocks` is
  what somebody restoring a backup uses. This is pre-existing behaviour that this
  spec neither created nor changed; it was owed **documentation** and is now in
  `docs/technical/world-format.md` §"Known limitations, as built" and ADR-029. No
  test asserts it.
- **A window opening after a successful launch.** S7's subprocess is killed the
  moment the line arrives, before the client reaches `Session::new` in the ordinary
  case. On a machine whose GPU and window initialise faster than the child's content
  load, a window may briefly appear and confine the cursor. There is no product-side
  lever that avoids it — `gpu_startup::open` passes its backends explicitly, so
  `WGPU_BACKEND` is not consulted — and the alternative instruments (a source scan;
  a stop-after-preparing flag) were rejected as weaker and as unscoped surface
  respectively.
