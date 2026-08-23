# Validation Report — SPEC-021, one bit answers four questions

**Verdict: PASS**, on pass 2, with zero findings at every severity across all
three dimensions. Rigor `high`. Issue PRO-904. Branch
`feature/PRO-904-solid-split-properties`. 54 scenarios.

Pass 1 returned **FAIL** on a single coverage finding — FR-2.6-S3 recorded as a
GAP — with zero Blockers, zero Majors and zero Minors anywhere. The finding was
correct, and what it identified was a **contradiction between the scenario and
`standards/global/testing.md` §2** rather than a defect in the implementation or
in the test. The scenario was amended; no test was touched. Pass 2 ran against
the amended tree and returned PASS.

That is the one thing in this report worth reading twice, because from outside it
is indistinguishable from a requirement loosened when implementing it got hard.
The record of why it is not is below and in `test-map.md`, and the substance of
it — the four instruments that guard the mesh and their four reaches — has been
promoted to `docs/technical/testing.md` so that it survives this folder.

## Gate

**`95e06eb`, exit 0, `GATE PASSED`**, read from the run's own log rather than
from an exit code an outer wrapper could have supplied (see
`docs/technical/testing.md` §"Reading a gate's verdict"):

```
1435 tests run: 1435 passed (6 slow), 1 skipped
lines 93,63%  regions 92,13%  (10844 lines tracked)
```

**A bare `1435 tests run`, with no slash** — a complete run rather than a
cancelled one. Every stage ok: format, lint + complexity, gpu-free, docs, size,
deps, sast, secrets, tests, coverage.

The gate reading is a statement about **the tree at `95e06eb`**, which is the tree
pass 2 reviewed and the tree this report is written against. `git status --short`
empty, `git stash list` empty, HEAD matching origin.

## Pass 2 — reviewer summary

| Dimension | Verdict | Blocker | Major | Minor | Info |
|---|---|---|---|---|---|
| correctness | PASS | 0 | 0 | 0 | 0 |
| coverage | PASS | 0 | 0 | 0 | 0 |
| quality | PASS | 0 | 0 | 0 | 0 |

**3 of 3 reviewers returned real structured results**, checked against the run's
per-reviewer payloads rather than inferred from the merged verdict — an absent
reviewer and a clean reviewer look identical in an aggregate, and this project has
shipped a real defect ranked out of an otherwise clean PASS once already
(`standards/global/testing.md` §2).

## Pass 1 — the one finding, and why the scenario was what was wrong

**FR-2.6-S3, coverage GAP.** As first written the scenario required a quad count
*"derived from an independent walk of the same world rather than one snapshotted
from a run of the mesher"*. The test mapped to it compares against the committed
`SCENE_QUAD_COUNT`, whose own doc comment says it is a snapshot and *"verifies
nothing"*. The scenario said derived, the test snapshots, so the mapping was
unmet. The reviewer added that no test in the workspace derives a quad count.
Both statements are true.

**The quantity the scenario asked for is not well-defined.** A quad count is a
property of the merge, not of the world: the mesher emits the scanline-greedy
decomposition — grow a run along the primary axis, then extend it along the
secondary while a whole row matches — and that is deliberately not the fewest
rectangles covering the same faces. A merger growing columns before rows would be
equally correct and would report a **different count for identical geometry**. Any
derivation agreeing with the mesher would have to repeat its ordering choices, at
which point it is a copy of its subject — precisely what
`standards/global/testing.md` §2 forbids, and what this spec had already refused
twice on the ground that an oracle asking its subject's question is not an oracle.

**Nothing that was derived stopped being derived.** FR-2.6-S1 and FR-2.6-S2
compare the mesher against an independent per-voxel walk and commit no number at
all; FR-2.5-S1's per-direction figures are counted from the heightmap. Only the
one quantity that never had an oracle to be compared against is restated as what
it has been since it was minted.

### The guard proposed for the residual was false, and was measured false

The amendment brief proposed stating the guard as *areas by independent
derivation, **merge shape by the goldens**, quad count as the tripwire*. **The
middle term is wrong.** A packed terrain vertex carries `x, y, z, facing, layer,
section` and no field derived from a quad's extent; texture coordinates come from
a corner's own section-local position; the terrain sampler is
`AddressMode::Repeat` on all three axes. One 4×1 quad and four 1×1 quads emit the
same texels at the same depths. **A re-partition of the same visible faces is
pixel-neutral by design**, and the goldens are compared perceptually against a
disagreement budget besides — they could not see the difference if there were one.

What closes the residual instead is stronger than the goldens would have been:
`crates/mc-world/tests/mesh_properties.rs` pins the quads to an **exact
partition** of the visible-face set, per face and per position, over randomly
generated section contents and all six surroundings — which catches a face
relocated with its area preserved, something no area sum can see.

**So the four instruments have four different reaches, each able to fail on its
own**, and a merge change is one of two things with no third case: it keeps the
partition, in which case it is area-neutral and pixel-neutral by construction and
observable only as a count — a strategy change somebody owes an explanation for,
which is what the tripwire extracts — or it breaks the partition, in which case
the property tests redden and name the face.

### Closure

- `54d2a78` — the scenario amended, with the reasoning recorded in `spec.md` at
  FR-2.6 and in `test-map.md`.
- `95e06eb` — two sites that asserted a moved quad count means every committed
  golden is stale made **conditional rather than deleted**:
  `SCENE_QUAD_COUNT`'s doc comment and the failure message of the test mapped to
  FR-2.6-S3. Both were true of the change they were written for — ambient
  occlusion, which is per-vertex and therefore does make merge shape visible — and
  false of a pixel-neutral re-partition, for which the instruction churns the
  whole golden set to reproduce identical images.

No golden was touched and `SCENE_REVISION` was not bumped: nothing in the closure
moves a pixel or a count.

## What this spec delivered, and to whom

Key Principle 7, asserted rather than claimed:

- **Player** — the sea is visible from the declared spawn, a swing at water is
  refused with the water left in the cell and the block behind it untouched, and a
  placement aimed at water still builds through it.
- **Mod author** — `drawn`, `occludes` and `targetable` are three independent
  optional booleans, each defaulting to whatever the same declaration says about
  `solid`, each with its own refusal naming the field.

## Carried forward, stated rather than left silent

- **No human has played this increment.** A green suite is not evidence that a
  shipped capability works. The manual journey nobody has walked: start a new
  world, look at the sea from the shore, swing at it and read the refusal, place a
  block into it, then quit, relaunch, and read the four-block behaviour line once.
- **`Refusal::Indestructible` is now reachable** and the fuse recording its
  unreachability is blown; the six sites that described that fuse are rewritten.
- **`merges_with_self` is an engine rule, not a declaration.** Two adjacent cells
  holding the same non-occluding block show no seam and there is no field that
  turns it off. PRO-952 is the named breaker and must turn it back into a field.
- **The judge's first-drawn-voxel march assumes every drawn block is opaque.**
  PRO-952 is the named breaker there too.
- **The declared spawn moved for a test's benefit**, recorded as a product change:
  a human player could already walk to the sea, only the automated capture could
  not see it.
- **The merge-shape instrument witnesses the partition, not the predicate.**
  `mesh_properties.rs`'s generated pool declares only `solid`, and the independent
  scan it compares against asks `Section::is_solid_at` on both sides — where the
  mesher asks `drawn`, `occludes` and key identity. Over this pool the three
  coincide and the same-block rule never decides anything, so the two sides remain
  comparable; the disagreeing cases are graded by FR-2.3 and FR-2.4's hand-built
  fixtures instead. **Whoever widens that pool must move the scan onto the same
  three questions in the same commit**, or the properties go vacuous rather than
  red. Recorded in `docs/technical/testing.md` as well, since that is where it will
  be read.
