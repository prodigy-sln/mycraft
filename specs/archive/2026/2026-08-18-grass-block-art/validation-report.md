# Validation Report — SPEC-019, the grass block looks like a grass block

Issue PRO-947 · rigor `high` · branch `feature/PRO-947-grass-block-art` · two
review passes, both run through the three-specialist reviewer workflow with
per-finding adversarial verification.

## Verdict

**PASS**, after two passes and six fixed findings. Every finding was
documentation; **neither pass found a defect in product code or in a test.**

| Pass | Blocker | Major | Minor | Info | Outcome |
|------|---------|-------|-------|------|---------|
| 1 | 0 | 2 | 2 | 0 | all four fixed (`6c6b381`, `cbd7670`) |
| 2 | 0 | 2 | 0 | 0 | both fixed, plus ten more the sweep found (`5949583`, `c01005b`, `04c6628`) |

Per dimension in pass 2: correctness FAIL (2 findings), coverage PASS (0),
quality PASS (0). In pass 1: quality 3, coverage 1, correctness 0.

No third pass was run. A third pass is not started unprompted, and pass 2's
findings were fixed rather than carried.

## Gate

Four readings, every one `GATE PASSED` with exit 0 and all 12 stages green. Two
were taken by the conductor on a clean detached worktree rather than accepted on
report, at `63bac13` and at `cbd7670`, and the branch tip `04c6628` was read
again after the final documentation commits — because **a doc-only commit is not
gate-neutral in this repo**: the `docs` stage resolves intra-doc links and the
`size` stage counts non-blank lines.

```
Summary [ 118.311s] 1370 tests run: 1370 passed (2 slow), 1 skipped
lines 93,55%  regions 92,05%  (10716 lines tracked)
```

gpu-free stages: `69 passed, 1 skipped` and `106 passed, 0 skipped`.

One gate fact worth keeping: **the commit that fixed this spec's central defect
was not itself gate-green.** `b67e531`'s new doc comments linked from public
items to private ones, which rustdoc refuses outright, so `mc-render` did not
document at all and the `docs` stage failed. Repaired in `9d5a1e1` as plain code
spans, no table or logic touched. An intra-doc link to a private item is
invisible to a reviewer reading a diff for meaning; only the gate reports it.

## What the reviewers actually covered — stated, not implied

**The scenario verdicts behind this report are a sample, not a sweep, and pass
1's merged "no failed scenarios" line must not be read as one.** The spec carries
**104** scenarios — 101 plus the three orientation scenarios added on 2026-08-19,
by the command `test-map.md` itself prescribes, run against both files. Pass 1's
two scenario-enumerating reviewers named **25 and 15** between them — 40 — all
PASS; the quality reviewer names none by design. Pass 2 enumerated a deliberately
different region and returned three honest abstentions:

| Scenario | The reviewer's stated reason |
|----------|------------------------------|
| FR-2.1-S4 | re-pack code path not directly read |
| FR-3.3-S7, S9 | `voxforge` emit/seam **not in the manifest** |
| FR-8.1-S4..S5 | test file not directly opened; inferred from goldens and probe design |

These are recorded as abstentions, not gaps. A reviewer declining a verdict it
has not earned is the behaviour this project wants, and scoring it as a defect
would train the opposite. **The FR-3.3 pair was unreachable because the conductor
narrowed pass 2's manifest from 187 files to 74** — that abstention is a property
of the instrument, not of the code.

This matters because it is the same shape as the defect this spec was built
around: an instrument that cannot see something reports zero rather than
reporting that it cannot see. It applies to a reviewer panel exactly as it
applies to a test.

**The denominator in this section was itself wrong when first written, and that
is worth recording rather than quietly fixing.** It read 108 — a figure carried
in the conductor's own working notes for many days and never once measured, then
written into this report as though it were. The spec's prescribed command gives
104. It was caught by the agent writing the registry entry, which needed the
number for a different purpose and therefore had to compute it. That is the
**fourth** figure in this spec stated from something other than a measurement,
the second of the four that was the conductor's own, and the second caught by the
same mechanism: **a figure nothing computes is a figure nothing checks, and the
cheapest way to check one is to make something need it.**

## Findings

### Pass 1

1. **Major — `docs/user/gameplay.md`.** Two passages told a player the world is
   still flat placeholder colours and that real art "is the next thing to land",
   while a section added by this same spec, in the same file, said grass, dirt
   and stone draw baked pictures. Both rewritten to the per-key truth.
2. **Major — `docs/modding/hot-reload.md`.** "Art loaded from disk has not
   landed", on a page cross-referencing the very section this spec corrected to
   say the opposite. A mod author following the page's own pointer arrived at a
   contradiction.
3. **Minor — `crates/mc-core/src/hash.rs`.** Module doc claimed the index
   contract "does not exist yet"; `crate::art` implements it. Doc comment only.
4. **Minor — `spec.md`.** FR-8.1-S6/S7/S8 were written `**FR-8.1-Sn**` rather
   than `FR-8.1-Sn:`, so `test-map.md`'s own prescribed self-check counted 101
   against 104 and `comm -3` named those three. Now 104 and 104 with an empty
   `comm -3`, confirmed by running the two commands that file prescribes.

### Pass 2

5. **Major — `docs/modding/hot-reload.md`.** The worked example's comment: "The
   layer a block draws from is selected by its name today. Declaring anything
   else here loads and then will not pack." False since this spec closed PRO-902
   — `TextureResolution::key_of` reads the declaration, and neither consumer
   parses a name. **The comment's first two sentences were correct and the next
   two wrong**, which is how it survived being read.
6. **Major — `docs/technical/decisions.md`.** ADR-024's Consequences still
   stated the name-based gap as current behaviour. **Amended in place, not
   superseded**: `decisions.md` requires a new record to supersede, and ADR-024's
   decision (append-never-renumber) is untouched — only which key a face asks for
   changed.

## The pattern, which is the real finding

Five of the six were **statements this spec's own work falsified**, across three
claims: that art had not landed, that the index contract did not exist, and that
layer resolution went by a block's name. The sixth, the grep-format defect, is a
different kind and is counted as such.

The documentation obligation was met for every **new** surface — new guides, new
sections, ADR-028 — and missed everywhere an **old** passage had been made
untrue. Nothing in the pipeline asks *what did this change make untrue?*, which
is why an adversarial panel had to find it twice.

Acting on the class rather than on the two instances found **ten further stale
passages that neither review pass reached.** The worst was
`docs/modding/README.md`, the first-block tutorial — the first thing a mod author
reads — describing the lifted limitation as current. `docs/INDEX.md` restated it
twice, in wording the as-built pages do not use, which is precisely what defeats
a phrase-level grep. Two cross-references were first judged leavable and then
fixed, on the reasoning that a pointer describing a category its target no longer
has is the same drift one step removed.

The habit is recorded in `docs/technical/working-in-this-repo.md`
§"Lifting a limitation costs one grep per place it was stated", with the asymmetry
that explains it: a spec that *adds* a surface has an author who knows what was
built, while a spec that *lifts* a limitation **has no author in the passages
that recorded it**.

## Two defects found and deliberately not fixed here

Both are real, both were falsified by a different spec, and one spec is one
commit:

- `crates/mc-sim/CLAUDE.md` — "texture and solidity still come only from
  `content/base/blocks/*.toml`". Blocks are Luau. Falsified by PRO-917.
- `docs/planning/client-server-split.md` — "the layer index is a key's position
  in that lexicographically sorted set", falsified by ADR-024's own
  append-never-renumber decision.

Both are fixed directly on `main`, as their own commit, after this spec merges.

## What validation cannot say about this spec

The gate is green and both reviewer passes are clean, and **neither fact is
evidence that the increment looks right.** This spec's central defect — the
renderer drawing five of its six faces turned — survived 1366 passing tests,
because every reading in the suite measured *which colours* a face holds and none
measured *where they sit*: means, histograms, distinct-colour counts, pairwise ΔE
and landmark shares are all invariant under rotation, reflection and permutation.
It was found by the project owner looking at a golden frame. FR-8.1-S6, S7 and S8
close that hole for the four side faces; the two horizontal faces are held by the
bake and by ADR-028's stated convention, and the first anisotropic top or bottom
texture owes a scenario.

**No human has played this increment.** That obligation stands after this report.
