# Validation Report — SPEC-018, entry-door clearing

**Verdict: PASS.** Pass 1 returned FAIL on three Minors with zero Blockers and zero Majors; all three
were closed and the closure verified mechanically. Rigor `high`. Issue PRO-948. Branch
`feature/PRO-948-entry-door-clearing`.

There is no pass 2, and §5 of the validate skill is why: every finding was cosmetic and
**mechanically checkable**. A grep answers each of them completely, and a mechanically checkable item
gets a mechanical check rather than a second 700 000-token review pass. The same ruling was made on
SPEC-017 for the same reason.

## Gate

Two runs, both mine rather than a subagent's, both on a tree confirmed quiet before and after
(`git status --short` empty, `git stash list` empty, HEAD matching origin):

| Commit | Result |
|---|---|
| `98ec656` (pass 1's tree) | exit 0 · 1221 tests, 1221 passed, 1 skipped · lines 94.36%, regions 92.54%, 9738 tracked |
| `19f2993` (after the fixes) | exit 0 · 1221 tests, 1221 passed, 1 skipped · lines 94.36%, regions 92.54%, 9738 tracked |

Every stage ok in both: format, lint + complexity, gpu-free, docs, size, deps, sast, secrets, tests,
coverage. PRO-953's memory-cap test passed first time in both runs.

## Pass 1 — reviewer summary

Workflow `wf_b85c1220-d6a`, 51-path manifest, six agents, 717k tokens.

| Dimension | Verdict | Blocker | Major | Minor | Info |
|---|---|---|---|---|---|
| correctness | PASS | 0 | 0 | 0 | 0 |
| coverage | PASS | 0 | 0 | 0 | 0 |
| quality | FAIL | 0 | 0 | 3 | 0 |

**Every one of the 18 scenarios was verdicted PASS.** 39 scenario verdicts recorded across the
reviewers, all PASS.

**Nothing was filtered out of the merge.** Three candidate findings, three confirmed by adversarial
verification, zero refuted. This was checked against the run journal rather than inferred from the
merged verdict, because a clean merged result and an absent reviewer look identical — and because on
SPEC-016 a real defect was ranked out of an otherwise clean PASS.

## The three Minors, and their closures

**1–2. Continuation indentation folded into five assertion-message literals.**
`crates/mc-client/tests/support/reload_trap.rs:297,306,312,319` carried 14-space runs mid-sentence;
`crates/mc-client/tests/reload_leaves_the_player_alone.rs:179` carried nine runs of exactly ten. The
cause was in the editing layer, not in rustfmt: a `\` continuation written through a Python string is
a *Python* continuation, which deletes the backslash and newline and **keeps the following
indentation**, so the Rust indent reached the file inside the literal. rustfmt never touched them, and
a hand-written continuation survives `rustfmt` unchanged.

Closed at `19f2993`, **whitespace only and provably so**: `git diff --word-diff` reports no added or
removed words in either file, and the diff is exactly 5 insertions and 5 deletions. The prose was not
shortened, re-worded or compressed — several of those strings are the only surviving explanation of
why that fixture is shaped as it is.

*A judgement call recorded rather than taken silently:* the house style would re-wrap the five
literals as `\` continuations, and that form was written and then reverted, because the extra source
lines push `require_a_refusal_could_have_moved_them` to 36/30 and the refused scenario to 33/30 under
`clippy::too_many_lines`. Buying that back means restructuring a guard and a scenario body. **A
cosmetic improvement is not worth spending structure on** — the same trade this spec already made
against a file-size cap, one lint over.

**3. `docs/INDEX.md:63` said "four non-fatal notices" where `architecture.md:1715` now says seven.**
Exactly the drift T13 existed to close, reappearing one level up: the count lived in two places and
only one of them was the one being edited.

Closed at `c7822e3`, and generalised rather than patched. The routing text now reads *"tabulating
every non-fatal notice"* — **the number is gone**, so there is nothing left for the page's table to
drift away from. Four further counts in the same row were removed on the same reasoning, along with a
`500-line` figure that belongs to `standards/global/code-quality.md` rather than to a routing summary.

> **A routing summary describes what a page answers, never how many answers it has.** A count in
> `INDEX.md` has nothing holding it to the page, no test can see it, and it is invisible to whoever
> edits the page itself.

## Closure verified mechanically

| Check | Result |
|---|---|
| `grep -c "[a-z]     *[a-z]" reload_trap.rs` | 0 |
| `grep -c "[a-z]     *[a-z]" reload_leaves_the_player_alone.rs` | 0 |
| `grep -c "four non-fatal notices" docs/INDEX.md` | 0 |
| `docs/INDEX.md:63` | reads *tabulating every non-fatal notice* |
| `git diff --word-diff --stat 98ec656..19f2993 -- crates/mc-client/tests/` | 5 insertions, 5 deletions, no word changes |
| Gate at `19f2993` | exit 0 |

## Known and accepted, carried forward deliberately

These were briefed to the reviewers as settled. None is a defect this spec should close; each is
recorded in `docs/` so it survives the spec folder being archived and pruned.

- **M13b — the only hole here with no instrument at all.** Hand-wrapping an inlined clause across a
  `\` continuation leaves 275/275 green with the const idiom abandoned. rustfmt cannot reach it; a
  person can; **only review sees it.** Named in the scan's own file header, not only in `test-map.md`.
- **The FR-1.3 scan's `published:` prose exposure.** A `///` → `//` edit reddens it with no door
  added. Tolerable because a red naming `reload/mod.rs:91` is a minute's diagnosis.
- **M5's non-bite.** The velocity rule is witnessed through the reload caller alone. A change dropping
  that path's clearing takes the only witness with it and **no entry test reports that**.
- **A join can still supply the wrong ground.** The omission a join cannot make is forgetting the
  rule; the mistake it can still make is passing the wrong extent. Whichever spec adds the join owes
  it a scenario.
- **`docs/user/gameplay.md` quotes both entry lines and sits outside the walked tree.** It can drift
  from what the program prints with the whole suite green — small, one-directional and deliberate,
  because widening the walk to all of `docs/` is not free.
- **`crates/mc-client/src/app/mod.rs` is at exactly 500 of its 500-line cap.** The next line added
  forces a split that should be planned rather than discovered.
  `crates/mc-client/tests/support/printed_refusals.rs` is at 555 of 600.
- **PRO-953** — a memory-cap refusal test failed once under full-suite load and passed in isolation
  and on re-run. Deliberately **not** quarantined against `testing.md` §8: one observation is not
  intermittency, and that test is the witness for *a bad mod never takes down the server*. Quarantine
  on a second observation.

## What this validation does not cover

**No human has played this.** The capability is a moved player and a sentence on a terminal at launch,
and a green suite is no evidence a shipped capability works. The journey is written out as step 6 of
`docs/modding/hot-reload.md`: quit standing in water, set `content/base/blocks/water.luau` to
`solid = true` while the game is not running, relaunch with `--load-changed-blocks`, read the line.
Until someone does that, this report says the tests pass and says nothing more.

## Sign-off

User sign-off is not required — the user has overridden it for conductor-driven work and the
conductor is the approving authority. **PASS on that authority**, at `19f2993`, gate exit 0.
