# Test map — SPEC-015, a content refusal names the file, the block and the field

Scenario ↔ test, one line per scenario. Test names are behavioural and carry no
scenario id; this file is where the two are joined.

Paths are relative to the repository root.

## Phase 1 — one renderer, three doors, and no other way to report

| Scenario | Test file | Test name |
|----------|-----------|-----------|
| FR-1.1-S1 | `crates/mc-render/src/window_test.rs` | `a_failure_is_rendered_with_the_failure_beneath_it_after_a_separator` |
| FR-1.1-S2 | `crates/mc-render/src/window_test.rs` | `a_failure_with_nothing_beneath_it_is_rendered_as_its_own_message_alone` |
| FR-1.1-S3 | `crates/mc-render/src/window_test.rs` | `every_failure_beneath_a_failure_is_rendered_and_not_only_the_first` |
| FR-1.1-S4 | `crates/mc-render/src/window_test.rs` | `a_message_of_several_lines_is_rendered_whole_between_the_layers_around_it` |
| FR-2.1-S1 | `crates/mc-client/tests/content_refusals.rs` | `a_block_named_wrongly_is_refused_naming_the_file_the_block_and_the_field` |
| FR-2.1-S2 | `crates/mc-client/tests/content_refusals.rs` | `a_block_carrying_an_unrecognised_field_is_refused_naming_that_field` |
| FR-2.1-S3 | `crates/mc-client/tests/content_refusals.rs` | `two_files_declaring_one_block_are_refused_naming_both_files_and_the_name` |
| FR-2.1-S4 | `crates/mc-client/tests/content_refusals.rs` | `a_root_declaring_no_block_at_all_is_refused_naming_the_root_and_nothing_else` |
| FR-2.1-S5 | `crates/mc-client/tests/content_refusals.rs` | `a_block_file_that_is_not_toml_is_refused_naming_the_file_and_the_reason` |
| FR-2.1-S6 | `crates/mc-client/tests/content_refusals.rs` | `a_missing_content_root_is_refused_naming_the_directory_that_was_looked_for` |
| FR-3.1-S1 | `crates/mc-client/tests/hud_launch.rs` | `a_refused_hud_declaration_names_the_file_the_element_and_the_field` |
| FR-3.1-S2 | `crates/mc-client/tests/hud_launch.rs` | `a_hud_file_that_is_not_toml_is_refused_naming_the_file_and_the_reason` |
| FR-3.1-S3 | `crates/mc-client/tests/hud_launch.rs` | `a_content_root_declaring_no_hud_is_read_without_a_word_about_the_hud` |
| FR-6.1-S1 | `crates/mc-client/tests/content_refusals.rs` | `the_shipped_content_registers_one_block_for_each_declaration_and_says_nothing` |
| FR-6.1-S2 | `crates/mc-render/src/window_test.rs` | `a_run_the_player_ended_by_closing_the_window_says_nothing_at_all` |
| FR-6.1-S3 | `crates/mc-render/src/window_test.rs` | `a_run_that_found_no_adapter_to_draw_with_reports_that_refusal` |
| FR-6.1-S4 | `crates/mc-render/src/window_test.rs` | `a_run_that_lost_the_graphics_device_reports_the_reason_it_was_lost` |

Seventeen scenarios, seventeen tests. Phases 2–4 (FR-4, FR-5, FR-7) are a later
test author's and are not listed here.

### Additional coverage — Phase 1

Each line says what the test catches that no scenario's own test would.

| Test file | Test name | What it catches |
|-----------|-----------|-----------------|
| `crates/mc-render/src/window_test.rs` | `a_context_the_failure_does_not_know_is_said_above_the_whole_chain` | `failed_under` dropping its context sentence, joining it without `": "`, or not descending into the failure beneath it. Seven call sites go through that door in this phase and no scenario asserts any of their text — the tasks record them as structural-only, which leaves the constructor's own grammar unwatched. |
| `crates/mc-render/src/window_test.rs` | `a_refusal_with_nothing_beneath_it_is_reported_as_its_own_sentence` | `stated` appending a separator and an empty layer, or losing the prefix or the line ending. Its two call sites are held by the type system alone (a `&'static str` cannot be a `format!`), which says nothing about what is printed. |
| `crates/mc-render/src/window_test.rs` | `guidance_a_site_supplies_is_said_after_the_whole_chain_it_answers` | `failed` ignoring its guidance argument, or inserting it inside the chain instead of after it. Phase 1 passes `""` at both sites, so a `failed` that discarded guidance entirely would look correct until Phase 2 hung a player's only way back into their world on it. |

## Phase 2 — a cause is said once

| Scenario | Test file | Test name |
|----------|-----------|-----------|
| FR-5.1-S1 | `crates/mc-client/tests/refusals_state_a_cause_once.rs` | `a_save_that_cannot_be_read_is_named_once_and_its_reason_given_once` |
| FR-5.1-S2 | `crates/mc-client/tests/refusals_state_a_cause_once.rs` | `a_world_that_could_not_be_generated_names_the_missing_block_once` |
| FR-5.1-S3 | `crates/mc-client/tests/refusals_state_a_cause_once.rs` | `a_save_refused_only_for_redeclared_blocks_offers_the_way_out_once_after_it` |

Three scenarios, three tests. Every one counts occurrences rather than searching
for presence: "exactly once" is not a thing `contains` can see, and a report
saying the reason three times satisfies a presence check as readily as one saying
it once. The whole of what was written is compared beside the counts, because a
count cannot see a separator hung on an empty layer and a comparison alone would
go on agreeing if the reader quietly stopped filling the path in.

**FR-5.1-S3 could not be authored before `PreparationError::way_out()` existed**,
and that is a sequencing fact rather than a property of the scenario. The way-out
sentence lives in `PreparationError::Launch`'s own `Display` today and moves to
`way_out()` under Decision 4 (option C), where `Ending::failed` appends it after
the whole chain. A test rendering with an empty guidance would be red today *and*
red after the change — the sentence disappears entirely — so it would be red for
a reason nobody chose. Written against `way_out()`, with the three `Display`
strings still unchanged, it is red on the sentence appearing **twice**, which is
the double statement this phase exists to remove. The method was therefore landed
first, on its own, deliberately doing less than the phase needs.

**FR-5.1-S3's expected text takes the way-out sentence from `way_out()` rather
than spelling it out, and that is deliberate.** What the test pins is its
**placement** — after the whole chain, once, with nothing between it and the end
of the report. The sentence's own wording stays where Out of Scope leaves it,
unquoted by any test that could go stale against a rewording nobody is permitted
to make. Inlining the sentence would read as a strengthening and is the opposite:
it couples a test to text the spec forbids changing, so the test could only ever
go red for a change that is already ruled out. Do not "improve" it that way.

**FR-5.1-S3's ordering clause is green today and cannot go red on this path.**
The scenario has two clauses — the sentence appears exactly once, and it appears
after the refusal it answers — and only the count carries it. `Ending::failed`
already appends guidance after the whole chain, so the placement is true before
the change as well as after; the red that drove this scenario was the count going
from 2 to 1. The ordering clause is a second witness on `failed`'s placement,
whose first witness is
`guidance_a_site_supplies_is_said_after_the_whole_chain_it_answers` in
`crates/mc-render/src/window_test.rs`. A reviewer meeting the `true` in that
tuple should not read it as half a pass.

### Additional coverage — Phase 2

Each line says what the test catches that no scenario's own test would.

| Test file | Test name | What it catches | Landed |
|-----------|-----------|-----------------|--------|
| `crates/mc-client/tests/refusals_state_a_cause_once.rs` | (inside FR-5.1-S1's and FR-5.1-S2's tests) the way-out flag counted at **zero** | A way out welded to the call rather than to the type is one a site can supply where there is none. Moving the sentence out of `Display` puts the choice inside `way_out()`, and a `way_out()` that answered unconditionally would tell a player pointed at a corrupt file, or at a content root missing a block, to pass a flag that cannot help them — sending them round the same refusal a second time. Nothing else in the suite asks for its absence. | yes |
| `crates/mc-client/tests/support/persistence.rs` | `refusal` (used by `launch_acceptance.rs`'s three tests) | The way-out sentence reaching a real reader, by a **second route**: taken through `Ending::failed` and the shipped reporting rather than read off a `Display` nobody prints. It previously read the outermost layer's own sentence, so the acceptance suite's `--load-changed-blocks` assertion was watching a string the client never writes — it would have stayed green through a client that dropped a player's only way back into their world. | yes |
| `crates/mc-client/tests/launch_builds_only_the_world_it_needs.rs` | `refusal` (used by `a_launch_with_no_save_refuses_naming_the_block_the_generator_could_not_place`) | The same, one layer deeper: the block the generator could not place is named *below* the sentence saying why a world was being built, so a helper reading only the outermost sentence was asking whether a name survived a journey it never took. | yes |
| `crates/mc-sim/tests/launch_world.rs` | `a_launch_with_a_save_it_cannot_read_refuses_naming_the_file_and_carrying_the_reason` | A **direct assertion on the shared derivation**, in the crate where the change is made: this layer names the file, does *not* restate the reason, and carries it as its source. A defect in either half is localised here instead of arriving as three failures in the client's printing tests. | yes |

**Both helper adaptations go through the door rather than re-composing what the
door composes**, and that is the whole of why they are worth having. The task
breakdown's literal wording — render the chain, then append `way_out()` — would
produce identical text and a second copy of one decision: an `Ending::failed`
that stopped appending guidance altogether would leave both helpers green, which
is the failure mode of comparing two configurations to each other rather than
asserting at the boundary. Going through `Ending::failed` and the reporting makes
them readers of that decision. The departure was raised and approved rather than
taken quietly.

**A third site needed adapting and no planning artefact names it.**
`crates/mc-sim/tests/launch_world.rs` read `LaunchError`'s `Display` and searched
it for the reason, which the `Display` change moves one layer down. It could not
be repaired the way the two client helpers were: `mc-sim` may not resolve
`mc-render` in any dependency kind, and hand-walking `source()` there to assert a
rendered string would put a second renderer in the workspace, asserted against
its own output and reaching no printing — the exact shape this spec exists to
remove. It therefore asserts `mc-sim`'s own obligation, which is structural, and
its name and doc comment say so, so that no later reader mistakes it for a
witness on what a player reads.

## Phase 3 — nothing reports around the renderer

| Scenario | Test file | Test name |
|----------|-----------|-----------|
| FR-4.1-S1 | `crates/mc-client/tests/reporting_seam.rs` | `every_failure_this_client_reports_is_rendered_by_the_one_renderer` |
| FR-4.1-S2 | `crates/mc-client/tests/reporting_seam.rs` | `the_same_scan_reports_a_source_file_that_composes_a_report_of_its_own` |
| FR-4.1-S3 | `crates/mc-client/tests/reporting_seam.rs` | `a_scan_pointed_at_a_source_root_that_is_not_there_reports_that_it_read_nothing` |

Three scenarios, three tests. The guard follows Decision 6: root
`crates/mc-client/src`, every production `.rs`, `exempt: |_| false` — the
exemption slot filled in and holding nothing — sibling `*_test.rs` skipped, doc
comments stripped before matching, and an enumerated verdict rather than an
absence assertion.

**Needles, five, and the expected hit count derived from them.** `Ending::Failed`
(the raw variant spelling), `.to_string()`, `{failure}`, `{cause}`, `{refused}`.
The positive-control fixture commits **every one of them in one file**, and
FR-4.1-S2's expectation is built by mapping over `REPORTING_GUARD.needles` rather
than written out — so a needle added without a fixture to catch it reddens there
instead of standing unwatched, and a mistyped needle cannot report a clean scan
forever.

**What the guard proves and what it does not, kept in the file's own header.**
The first two needles plus `#[non_exhaustive]` carry the invariant: a reported
failure cannot be *composed* in `mc-client` at all, because the compiler refuses
the struct literal and `.to_string()` names the only remaining hand-flattening.
The last three are a **naming-convention guard over a narrow residual hole** — a
site handing `failed_under` a context it built by interpolating an error under a
*differently named* binding escapes them. That hole is narrow and it is real. It
is written down at the guard rather than implied, because a guard claiming a
totality it does not have is this spec's own defect one level up.

**FR-4.1-S1 went red against the production tree, on a real defect rather than a
skeleton.** `tasks.md` and `architecture.md` both state the clean verdict is true
once Phases 1–2 land. Measured, it was not: `Ending::Failed`, `.to_string()`,
`{cause}` and `{refused}` were all at zero, and `{failure}` had three sites —
`app.rs:431`, `app.rs:463` and `events.rs:394` (the scan reports one site per
file per needle, so it named two files). Two are genuine: a `FrameError` and a
`winit` grab failure, each flattened by interpolation and each becoming
`rendered(&failure)`. The third is a false positive of the naming convention:
`report_remesh` takes a `&str` its callers already produced with `rendered(...)`,
so there is no error there to render — resolved by renaming the parameter to say
what it is, never by dropping the needle or adding an exemption, either of which
would remove the only thing watching the convention.

**FR-4.1-S2 and FR-4.1-S3 were green on arrival, and no honest RED exists for
them.** Both are scenarios about the scan's own behaviour, and the scan is test
code in its entirety — there is no implementation they could be red against. The
only way to redden them is to write a deliberately broken scan first and then
repair it, which is theatre rather than evidence. They are recorded here as
green-on-arrival rather than dressed up. What makes them non-vacuous is that each
compares a **whole enumerated verdict**: FR-4.1-S2's expectation is a list of
sites derived from the needle list, so a scan reading the sibling `*_test.rs`
file, missing a needle, or reporting nothing all fail it; FR-4.1-S3's rejects
every verdict except "I could not look", including the clean one.

### Additional coverage — Phase 3

Each line says what the test catches that no scenario's own test would.

| Test file | Test name | What it catches |
|-----------|-----------|-----------------|
| `crates/mc-client/tests/shipped_binary.rs` | `the_shipped_binary_started_away_from_its_content_says_why_on_its_error_stream` | **A report that is never reached or is written to the wrong stream.** This is the other half of FR-4.1-S1 and the pairing is the point: the scan proves nothing *composes* a report, and only this proves one is *printed*. A `main` that dropped its `report` call entirely, or that wrote the refusal to standard output, passes the scan — there is nothing left in `src` composing a report because there is nothing printing one either — and leaves the whole suite green. Every other test of the reporting calls the library and grades what it decides, never whether the binary asks. |

**Its non-vacuity rests on a mutation, not on a RED, and that is stated rather
than glossed.** It asserts behaviour Phase 1 already shipped, so it was green the
first time it ran and could not have been otherwise. Its evidence is the two
mutations the implementer runs against `main.rs` — drop the `report` call, and
separately write the refusal to stdout — reverted by hand, with the outcome
recorded either way.

**What it deliberately does not assert.** Not that stdout *equals* the palette
notice, only that stdout is non-empty and free of the refusal. Pinning the whole
string would couple a test about refusal reporting to the wording of an unrelated
placeholder-texture message, so a reword would redden it and teach whoever fixed
it that this test is noisy rather than meaningful. The exit status is asserted as
non-zero through a three-valued verdict (`Zero` / `NonZero` / `WithoutACode`, so
a process killed by a signal cannot read as a refusal) and never as a particular
code: the ending-to-status mapping is Out of Scope and is graded at
`crates/mc-render/src/window_test.rs:36-47`.

**It does not witness the guidance supply, and this must not be over-read.** Its
refusal is `PreparationError::NoContentRoot`, whose way-out is empty by
construction. The one production line that can emit the way-out sentence is
`app.rs`'s, and it needs a device and a window; that line stays uncovered and
stays labelled. A test that closes a real hole is exactly when somebody is most
tempted to read it as closing the adjacent one.

Every expectation in it is derived rather than snapshotted: the expected stderr
is built by constructing the same typed refusal and rendering it through the
shipped `mc_render::window::rendered`, and the looked-for directory is assembled
from `["content", "base"]` so it spells itself per platform.

## Phase 4 — the documented refusal is the refusal that is printed

| Scenario | Test file | Test name |
|----------|-----------|-----------|
| FR-7.1-S1 | `crates/mc-client/tests/documented_refusals.rs` | `every_refusal_the_modding_pages_quote_is_a_refusal_the_client_prints` |
| FR-7.1-S2 | `crates/mc-client/tests/documented_refusals.rs` | `a_quoted_refusal_altered_to_text_the_client_never_prints_is_reported_with_both_sides` |
| FR-7.1-S3 | `crates/mc-client/tests/documented_refusals.rs` | `a_page_that_quotes_no_refusal_at_all_is_reported_as_quoting_none` |

Three scenarios, three tests. The guard follows Decision 7: a quoted refusal is
a fenced code block under `docs/modding/` whose first line begins `mycraft: `,
reached via the repository root above `CARGO_MANIFEST_DIR`, answering with an
enumerated verdict rather than an absence.

**What the recogniser accepts, stated here because two pages are written against
it.** A page is any `*.md` file under `docs/modding/`, read in sorted order. A
fence is a line whose first non-space character begins ```` ``` ````; the info
string after it is not read, so ```` ```text ````, ```` ```console ```` and a
bare fence are the same block. A block is a *quoted refusal* when its first line
begins `mycraft: ` **at column zero** — an indented block is not one. A quoted
refusal must equal a printed one **whole**, not contain it: every layer of the
chain and the parser's caret diagnostic, line for line, with interior indentation
significant. Two normalisations are applied to both sides and to nothing else —
each line's trailing whitespace and the block's trailing blank lines are dropped,
and path separators are compared in `/` spelling.

**The compared text comes from a real run over the two declarations the scenarios
already name** — `blocks/amber.toml` carrying `slid` (FR-2.1-S2's declaration)
and `hud/malformed-readout.toml` stating an extent of zero (FR-3.1-S1's) —
prepared by `mc_client::startup::prepare_scene` and rendered through the shipped
`report`. Nothing here spells an expected refusal out.

**The one place the guard rewrites what the client wrote, and why.** A refusal
names a declaration by the path the run was given, and a run over a temporary
copy names that copy — which no page can quote. The fixture root's own path is
therefore rewritten to `["content", "base"]` collected, so what a page is held to
is what a person running from their game directory reads. The rewrite is checked
rather than assumed: a refusal that does not name the fixture root fails the
scan, so a loader that stopped naming the file cannot make the rewrite a silent
no-op. The separator normalisation is the second half of the same problem — the
identical refusal reads `content/base/blocks/amber.toml` on one platform and
`content\base\…` on another, and a page can carry only one of the two. Both are
weakenings and both are written down at the guard.

**Reading nothing at all is not one of the three verdicts.** A documentation
directory that has moved, or one holding no page, fails the scan rather than
borrowing `NoQuotedRefusalWasFound` — that verdict is a statement about pages
that *were* read, and letting a vanished directory answer with it is the same
conflation this file's enumerated verdicts exist to prevent.

**FR-7.1-S1 went red against the real tree, on the tree's own state.** Measured
at the branch head before any page was touched: eight pages, 33 fenced blocks,
and **zero** whose first line begins `mycraft: `. The verdict was therefore
`NoQuotedRefusalWasFound`, and the failure message prints both refusals the
client writes today so that whoever repairs the pages pastes captured text rather
than composing it.

**A correction to `tasks.md`'s stated reason for that redness, recorded rather
than reconciled silently.** The task says the guard is red because
`docs/modding/README.md` "documents a refusal the program does not produce". The
page does describe the truncated refusal — but **in prose, inside an inline code
span, not in a fenced block** — so the recogniser never sees it. Same redness,
different mechanism: the tree quotes no refusal at all rather than quoting a
wrong one. The spec's "Why FR-7 exists" carries the same wording and is subject
to the same correction.

**FR-7.1-S2 and FR-7.1-S3 were green on arrival, and no honest RED exists for
them.** Both are scenarios about the scan's own behaviour, and the scan is test
code in its entirety — there is no implementation they could be red against,
exactly as for FR-4.1-S2 and FR-4.1-S3 in Phase 3. Writing a deliberately broken
scan first would be theatre rather than evidence. What makes them non-vacuous is
that each compares a **whole enumerated verdict**: FR-7.1-S2's expectation is
built from the printed refusal itself, so a scan that reported nothing, reported
only the drifted side, or named the wrong printed refusal all fail it;
FR-7.1-S3's rejects every verdict except "no page quotes one", the agreement
verdict included.

**FR-7.1-S2's drift is a field name three lines down, never the first line.** A
comparison that stopped at the sentence a refusal opens with would accept that
page forever, and the opening sentence is the part of a refusal a page is least
likely to get wrong. The altered text is derived by renaming `slid` in the
printed refusal, and the fixture refuses to build if the rename changed nothing —
a no-op would leave the page quoting the refusal correctly and the mismatch
missing for a reason about the fixture rather than about the guard.

### Additional coverage — Phase 4

Each line says what the test catches that no scenario's own test would.

| Test file | Test name | What it catches |
|-----------|-----------|-----------------|
| `crates/mc-client/tests/documented_refusals.rs` | `pages_quoting_the_printed_refusals_verbatim_agree_and_their_other_blocks_are_passed_over` | **A guard that can never report agreement, and a recogniser that treats every fenced block as a quotation.** Nothing else in the file watches the agreement verdict come out of a *non-empty* comparison: FR-7.1-S1 is red until the pages are written and FR-7.1-S3 reaches agreement over nothing at all, so a recogniser answering `Mismatch` for every block would leave both controls green while making a correct page unwritable — which is the state where somebody deletes the guard instead of the drift. It is two pages and both printed refusals, the shape the real tree is about to take, and each page carries a second fenced block that is a declaration rather than a refusal. |

Green on arrival, like the two scenario controls beside it and for the same
reason. What it rests on is that its expectation is a whole verdict over text
taken from a real run, and that the pages it writes are built from that text
rather than from a copy of it.

## Notes for the next author

- **FR-3.1-S3 is true on `main` today** and is kept as a regression guard, as the
  spec's audit disposition already records. Its test cannot run red on behaviour;
  it goes red only on a client that starts refusing a valid root, or on a
  reporting path that writes for an ending that is not a failure.
- **FR-6.1-S1 is largely true today too.** What is new in its test is that the
  count is derived by counting the declaration files and that what the run *said*
  is compared beside it.
- **Where an expectation comes from.** No expected string is snapshotted from a
  run of the reporting. The four exact renderings in FR-1.1 are the spec's own;
  every client-side expectation is derived from the typed refusal the loader or
  the registry produced, asked for separately through
  `support::content::block_refusal_over` and `hud_refusal_over`; FR-6.1-S1's
  count is derived from the files in `content/base/blocks/`.
- **Phase 2 drives FR-5.1-S2 through `SectionError::UnknownBlock`, never through
  `WorldGenError::UnnamedBlock`.** On the `UnnamedBlock` path a reader sees the
  name twice already — that variant quotes `{text}` and its own source
  `NamespacedIdError::MissingNamespace` quotes the same `{text}`. That is **not**
  the FR-5 defect (no layer states its own *cause* there) and repairing it means
  rewriting a sentence in `mc-core`, which Out of Scope forbids; it is recorded as
  a deferred observation in `architecture.md` and `tasks.md`. A test written
  against that path would be red for a reason this spec deliberately deferred.
  The path the scenario is satisfied on is a block the registry does not declare,
  which surfaces as `SectionError::UnknownBlock` through the transparent
  `WorldGenError::Section` — named once.
- **The spec's Technical Considerations table understates the `Load` defect by
  one axis, and FR-5.1-S1 is right.** The table reads "reason printed twice" and
  stops there. Because `PreparationError::Launch` interpolated the *whole* of
  `LaunchError::Load` and then carried it as a source, the save's **path** came
  out twice as well — measured, `(2, 3)` for path and reason against `(1, 1)`.
  FR-5.1-S1's "name the save's path once" is therefore describing a real
  duplication and not overreaching. Recorded here so a reviewer meeting the table
  first does not read the scenario as asking for more than the defect.
- **Falsification, recorded either way.** The mutation that restores `: {source}`
  to `LaunchError::Load`'s message was first set up where it could not be run —
  a test run was declined while an implementation file was edited — and was
  reverted by hand, verified byte-identical. **It was afterwards run properly,
  against a clean tree, and it bites three tests**: both save scenarios and the
  `mc-sim` one. That measurement is what counts; the argument below is why the
  gap was survivable in the meantime, not a substitute for it. FR-5.1-S1
  compares the whole report against a text derived from an independent read of
  the save, so a restated reason lengthens only the printed side; and
  `crates/mc-sim/tests/launch_world.rs` asserts that the layer's own message does
  **not** contain the reason, in the crate where the message lives. The mutation
  that drops the `#[source]` was not attempted for a structural reason worth
  knowing: the field is *named* `source`, so the derive carries it whether or not
  the attribute is present, and expressing that mutation means renaming a field
  matched in another crate. It is double-witnessed regardless — dropping the
  source would empty `beneath` in `mc-sim` and shorten the whole compared report
  in FR-5.1-S1.
- **One mutation was deliberately not run, and the reason matters more than the
  result would have.** `main.rs`'s `failed` site was left alone when the guidance
  supply was mutated. Every failure that reaches it is `NoContentRoot`, whose
  `way_out()` is the empty string, so replacing `&failure.way_out()` with `""`
  there yields a **semantically identical program**. A mutation that cannot
  change behaviour cannot bite, and its green result would sit in the table
  looking exactly like the evidence the other rows carry while being none at all.
  **Do not "complete" the table by adding it.** The site that was mutated —
  `app.rs`'s, the only production line that can emit the sentence — left all 999
  green, and that is the finding.
- **Nothing in Phase 2 pins a byte count from before the change.** The spec's
  prose claims the text after Decision 5's edits is byte identical. Measured, that
  holds only where the cause has no further source — true of the save path
  (`LoadError::Unresolvable` carries no `#[source]`), false of `WorldGen` reaching
  `UnnamedBlock`, which gains a layer nothing prints today. That is the feature
  working; the prose overstates and the scenarios are right.
- **Why each client scenario is both searched and compared whole.** Searching
  alone cannot tell a clean rendering from one that appended a separator to an
  empty layer. Comparing alone would go on agreeing if the loader quietly stopped
  filling in the block or the field, because both sides of the comparison would
  move together. The two together are what pin the scenario.
