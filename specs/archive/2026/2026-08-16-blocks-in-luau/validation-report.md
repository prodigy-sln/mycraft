# Validation Report — Blocks are defined in Luau

- **Verdict: PASS** (pass 2)
- Rigor: `high` · Branch `feature/PRO-917-blocks-in-luau` at `ef146b9`
- Date: 2026-08-17

## Summary

| Reviewer | Blocker | Major | Minor | Info | Verdict |
|---|---|---|---|---|---|
| correctness | 0 | 0 | 0 | 0 | PASS |
| coverage | 0 | 0 | 0 | 0 | PASS |
| quality | 0 | 0 | 0 | 0 | PASS |

Pass 2 returned nothing at any severity. Each reviewer's payload was inspected
individually rather than only the merged verdict, because an absent reviewer and
a clean reviewer produce the same summary line.

## Pass 1, and its disposition

Pass 1 returned **two real defects**. Both were fixed on the branch and verified;
neither survived into pass 2.

1. **The declared-text refusal shipped with a hole punched through it.** The
   format string carried eighteen spaces between "at most" and the bound — a line
   continuation that was intended and never landed — so a mod author declaring a
   257-character `name` read a malformed message. Fixed in `5f2b89c`.

   **The instructive half is why no test could see it.**
   `luau_declaration_bounds.rs` asserts that both quantities are *mentioned* in
   the cause, and a substring check is equally satisfied by a well-formed message
   and a malformed one. The repair was therefore not a new assertion: a third
   fixture in `documented_refusals.rs` makes that refusal one the guard's run
   prints, and quoting it on `blocks-items.md` brings it under a comparison that
   is line for line, with the page as the oracle (`5dd5675`, `ef146b9`). Every
   other message literal in the content sources was scanned for the same shape;
   this was the only instance.

2. **`content/CLAUDE.md` still carried two MVP-1 TOML claims.** Its testing
   section said block files are TOML three sections after the same file states
   they are Luau. The spec's Documentation deliverable lists that file among the
   statements that must be retired, and leaving one standing while shipping the
   loader is a defect by the spec's own words. Rewritten in `8a75351`, together
   with the neighbouring one-concept-per-file note, and it now also says *why* a
   declaration carries no test of its own yet — there is no `mycraft.*` binding
   for one to assert against — rather than leaving the absence to read as an
   oversight.

## Gate

Run independently of the implementation, on the committed tree at `ef146b9`:

```
GATE_EXIT=0 — GATE PASSED
1084 tests run: 1084 passed, 1 skipped
lines 94.16%  regions 92.29%
```

The gate body was read rather than the exit code alone; no error line appears in
it outside test names.

## Scenarios

All **73** acceptance scenarios are implemented and covered:

| FR | S | FR | S | FR | S | FR | S |
|---|---|---|---|---|---|---|---|
| FR-1.1 | 5 | FR-2.2 | 4 | FR-3.3 | 3 | FR-6.1 | 3 |
| FR-1.2 | 4 | FR-2.3 | 4 | FR-4.1 | 4 | FR-6.2 | 2 |
| FR-1.3 | 4 | FR-2.4 | 3 | FR-4.2 | 5 | FR-7.1 | 3 |
| FR-2.1 | 7 | FR-3.1 | 3 | FR-4.3 | 9 | FR-7.2 | 2 |
| | | FR-3.2 | 1 | FR-5.1 | 3 | FR-7.3 | 1 |
| | | | | | | FR-7.4 | 3 |

The scenario ID sets in `spec.md` and in `test-map.md` were compared
mechanically and are identical in both directions. `tasks.md` carries no
unchecked task.

Phases 1–5 closed 67; phase 6 owns six, five of which opened red and one —
FR-7.4-S3 — green by design. The count is stated here so it cannot later read as
a discrepancy.

## The exit criterion, and the three pieces of evidence that decide it

The criterion is that the world renders identically and a player's save survives
the swap. Three results carry it, and each is recorded here because each lived
only in a phase log.

**1. The golden frames are 10/10 across five suites, run TWICE at the two
attributable moments.** `terrain_goldens`, `hud_goldens`, `golden_mismatch`,
`launch_and_capture_agree` and `replay_oracle` — once after the content
construction moved out of the client, and again after the layer assignment
changed. **Running once at the end could not have distinguished a wiring defect
in the move from a layer-assignment defect**, and that is the whole reason for
two runs: phase 5's clean result ruled out the wiring, so a difference in phase 6
would have been the assignment and nothing else. `golden_mismatch` passing is the
control saying the comparison can still fail. **A clean result at each of those
moments is evidence about that change, not an absence of news.**

**2. A world saved against the TOML declarations loads against the Luau ones
reporting no block as missing, changed or retextured.** This is the exit
criterion met directly, and it is the only scenario in the spec comparing a whole
resolved definition against an oracle computed before this spec existed.
`crates/mc-world/src/persistence/format.rs` folding a definition into two hashes
that deliberately exclude `origin` is what makes it possible — a save does not
depend on the path a definition was read from. It is the one test that would
catch a field mapped to the wrong place or a default resolved differently by the
new reader.

**3. And it discriminates rather than merely passing.** Flipping `solid` in
`content/base/blocks/water.luau` reddens it **alone**. A save written before any
of this work started catches a single wrong field in a single shipped
declaration.

## Stated limits of this verification

Recorded because a verification that claims more than it established is worse
than one that admits its edges.

1. **The seam's wiring is not behaviourally falsifiable in this arrangement.**
   Putting `TextureLayers::resolve(&registry.texture_keys())` back into
   `prepare_scene` — the production path deriving its own assignment again, which
   is precisely what FR-7.4 exists to close — left **every behavioural test
   green**, both FR-7.2 readings, FR-7.3's, all four of FR-7.4's, and both golden
   suites. Only the source scan reddened. A client that honours and one that
   derives answer identically for every content root that can be built today,
   because the assignment the simulation states *is* the order a positional
   derivation produces; permuting it permutes the array texture's fill in the same
   breath, so the goldens cannot see it either. **The scan is the only instrument
   that can**, and the property becomes falsifiable the moment an assignment is
   appended rather than renumbered, which is hot reload's.

2. **The source scan is the weaker instrument for FR-7.1's property.** Somebody
   adding a *second* door — a new public registration call — bypasses any text
   scan. The instrument that would catch it is a dependency-closure guard, and it
   **cannot pass while one binary hosts both halves**, so it is the
   composition-root spec's exit criterion rather than something this arrangement
   can assert. A guard green exactly when the rule is broken is inverted rather
   than weak.

3. **Seven scenarios were green on arrival inside their own phase, and all seven
   are controls rather than scenarios riding along** — measured, not assumed:
   FR-3.1-S2, FR-1.1-S5, FR-2.4-S2, FR-2.2-S1, FR-4.2-S5, and FR-4.3-S6/S7/S8 as
   accept-side controls. Flipping `BREAKABLE_BY_DEFAULT` reddens FR-2.2-S1, which
   is the measurement that separates *green on arrival* from *inert*.

4. **One property is named rather than tested.** The listing sort runs before the
   file-type check so that a root holding *two* offending entries refuses the same
   one on every run. No scenario has two offenders, so nothing pins it. It was
   left untested rather than given a test that could not be argued to catch
   something real today.

5. **The retained-output truncation is closed at the loader's boundary and filed
   at the host's.** `Printed` is one value rather than two accessors, so nothing
   can consult the lines without meeting the count. `ScriptHost` still exposes
   `printed()` and `dropped_print_lines()` separately, which makes the distinction
   available rather than unmissable — filed as a deferred observation against
   shipped PRO-916 surface rather than reshaped inside this spec.

## Mutation evidence

Twenty-six mutations were attempted across six phases — 1, 6, 7, 6, 3 and 3 —
and **twenty-five bit**. The one that did not is recorded as a fact about the
code's structure rather than as a gap (replacing `sort_by(file_name)` with `sort()` over the whole path —
every declaration shares one parent directory, so the two orders are the same
order and no fixture can separate them). Every mutation was reverted by hand with
`git diff --exit-code` confirmed clean afterwards.

**Four times a test no scenario asked for caught something real**, in four
separate phases: the count-before-file-check ordering, the `__len` sizing route,
the check that the production path derives no assignment, and the loader-boundary
truncation.

## Deferred, raised as their own issues

- **Most refusals a mod author can trip are quoted nowhere — PRO-946.** The
  verbatim guard covers three of roughly fourteen. The dependency runs
  page-follows-run, so quoting a refusal the run does not print *fails* the guard
  rather than covering it: each new quotation needs a fixture first, which is why
  this cannot be closed by editing a page. `tasks.md` carries the enumeration and
  the test author's note that the run is not free.
- **Texture resolution does not consult the registry — PRO-902/PRO-914.** The
  layer assignment is built from each block's declared `texture` and an entry is
  selected by the block's `name`, in **two** places:
  `crates/mc-render/src/geometry/mod.rs` and `crates/mc-render/src/hud/held.rs`.
  FR-7.4-S3 and its `held_swatch` twin pin both as tests rather than comments, and
  **they expire together when the per-face texture work lands — that red is the
  success signal, not a regression.**
- **The host's own truncation surface has the shape it closes.** Two accessors
  make a distinction available; a caller may read the record and never ask whether
  it is whole. Filed rather than fixed, because `ScriptHost` is shipped and merged
  surface and this spec's business with it was to bound a buffer.
- **A GPU-path flake, outside this diff.** The first full gate run of phase 2
  aborted `mc-client::terrain_probes` with `0xc0000005` and cancelled 914 unrun
  tests; it passed on the immediate re-run and on every run since. An access
  violation rather than an assertion failure. Recorded rather than absorbed,
  because `standards/global/testing.md` §8 wants a flake quarantined rather than
  tolerated.

## Next

`/sdd-complete`.
