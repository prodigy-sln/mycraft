# Validation — SPEC-025 / PRO-972

**PASS.** Blockers 0 · Majors 0 · Minors 2 · Info 2. Both Minors are
documentation prose; neither touches a scenario, the fix, or a test.

**79 lines against the phase's 60, deliberately.** What overflows is the
measurement section — figures this phase took itself rather than read off the
implement report, including the one that contradicts a document in the diff. A
verdict a later reader cannot re-check is the defect class this whole spec is
about, so the readings stay and the overflow is declared instead.

## Gate

Taken on this tree by this phase; log outside the repository. `pwsh -NoProfile
-File scripts/sdd-gate.ps1` → exit 0, `GATE PASSED`, **12 `ok:` stages**,
`grep -cE "^\s*FAIL"` = 0, `warning:` = 0, no file-lock line. `Summary
[136.632s] 1544 tests run: 1544 passed (13 slow), 1 skipped` — a **bare** count,
so complete. Coverage 93.72% lines / 92.24% regions, 10 983 lines. Tree: HEAD
`25efa7c`, level with `origin`, no stash, only `metrics.md` modified.

## Root cause, coverage, scope

The fix bakes the missing key and adds the reading nothing asked — declared keys
against covered keys — rather than removing the stand-in fallback, which
FR-2.2-S1 requires to keep working; `drawn_texels` (`support/art.rs:79-83`) is
deliberately unchanged. Nine scenarios go red without the fix, the two
preservation ones stay green, and the diff holds only the fix, its tests and the
five ruled-in-scope premise repairs.

## Re-measured independently, not taken on report

- **Goldens — the strongest available reading, and it replaces the one on
  record.** Decoding all four old and new blobs outside the project's own code,
  the pixels that **differ** are *position*-identical to the old stand-in family:
  `differ&~famOld = 0` **and** `famOld&~differ = 0` in all four. Not merely an
  equal count — no pixel of extent moved, which is what rules out a renumbered
  layer. Totals 88 280 / 88 280 / 174 744 / 198 828.
- **Premise 2** — water's two means are both `[76,121,158]` = `#4c799e`, ΔE
  0.0000; 87.9% base; widest over the eight ΔE 2.38, so `MEANS_AGREE_WITHIN =
  3.0` is still derived.
- **Premise 4** — spread 3.16 (old 3.71), stone 25.34, dirt 51.30, grass sides
  51.54–51.87, grass_top 71.85; 8 sits inside both brackets.
- **FR-3** — blue dominates 256/256; nearest mean stone 25.34, rest ≥ 51.30;
  widest pairwise 6.29 against dirt's 16.10; zero stand-in texels; the three
  colours are the approved palette exactly.
- **FR-1.1 shape** — both lists derive from observed state (registry, on-disk
  index, decoded texels) and are compared whole by `BTreeSet::difference` both
  ways into a four-arm total verdict. `SHIPPED_TEXTURE_KEYS` is the expectation,
  not a filter, so a ninth key reddens; the control names exactly one key, so a
  regression in the shipped root reddens it too.
- **Single witness** — no scenario besides the repaired FR-2.1-S2 has that
  property. FR-3.1-S1 and FR-3.2-S1 are different predicates over different
  quantities, and M4 shows FR-3.1-S2 red with both green.

## Findings

- **Minor** — `docs/technical/rendering.md:1611-1612` states the confirming
  measurement as "the pixels whose blue channel dominates number 88 280, 174 744
  and 198 828". Measured on the committed blobs: **483 564 / 483 564 / 482 327 /
  650 891** — the sky `(135,206,235)` is 395 284 px in t000 and dominates blue
  too, so a reader re-checking the stated procedure cannot reproduce it. The
  conclusion is true; the differing-pixel reading above proves it more strongly
  and should replace the sentence.
- **Minor** — `docs/modding/voxel-models.md:356` and `:370` tell a mod author an
  uncovered key draws "a magenta-and-black checkerboard" and to act "if a face
  comes up magenta". Stand-in colours are FNV-1a over the key
  (`crates/mc-render/src/texture/placeholder.rs:73-91`) and vary; magenta is
  specific to `base:water`, as line 928 says correctly. An author whose
  `mymod:ore` draws a teal checkerboard reads 370, sees no magenta, and does not
  add the manifest entry.

## Info

- `support/art.rs:56` attributes the widest two-means separation to
  `grass_side_north`/`_south`; `_east` is the argmax (2.3776 vs 2.3757, all three
  2.38 at the stated precision). **Pre-existing at `a9c6663`**, not introduced.
- `content/base/textures/base__waterX.png` is an untracked orphan from mutation
  M1's rebuild: gitignored, absent from `index.txt`, invisible to every reading
  and both art stages. Housekeeping only.

## Re-check of the documentation repairs — FAIL

**Blockers 0 · Majors 2 · Minors 0 · Info 0.** New findings only, per the
re-review rule. The four repairs were each verified by instrument and **all four
hold**; both Majors are surviving instances of the same two defects elsewhere in
this spec's diff.

### Gate — taken on `26b2012`, log outside the repository

`pwsh -NoProfile -File scripts/sdd-gate.ps1` → exit 0, `GATE PASSED`, **12 `ok:`
stages**, `grep -cE "^\s*FAIL"` = 0, `grep -c "warning:"` = 0, no file-lock line.
`Summary [130.725s] 1544 tests run: 1544 passed (5 slow), 1 skipped` — a **bare**
count, so complete. Coverage 93.72%. Tree at the reading and after it: HEAD
`26b2012`, clean, no stash, level with `origin`. `(5 slow)` against the previous
reading's `(13 slow)` is wall-clock annotation, not disagreement.

### The repairs, re-measured outside the project's own code

Decoding all eight blobs with an independent PNG reader and an independent
CIELAB/CIE76 implementation, `famOld` defined as the doc defines it — the RGB box
`(140,38,131)`–`(160,58,151)`:

| capture | differ | famOld | differ&~fam | fam&~differ |
|---|---|---|---|---|
| t000 | 88 280 | 88 280 | 0 | 0 |
| hud-t000 | 88 280 | 88 280 | 0 | 0 |
| t059 | 174 744 | 174 744 | 0 | 0 |
| t119 | 198 828 | 198 828 | 0 | 0 |

**Third independent reproduction; the repaired sentence is true and its procedure
yields its number.** Frame 1280×720 = 921 600, so 9.58 %–21.57 % is exact. The
box corners are the stand-in pair itself: FNV-1a over `base:water` bands to
`[150,48,141]`, ±`VARIATION` 10 → `(140,38,131)` / `(160,58,151)`. The strict
triple including the observed minified `(150,49,141)` gives **77 987 / 77 987 /
165 232 / 191 792**, matching `rendering.md:1592`. Sidecars byte-identical, four
files modified, none added — verified. The old text's own procedure gives
**483 564 / 483 564 / 482 327 / 650 891**, confirming the Major was correctly
diagnosed. `voxel-models.md:356`/`:370` now name the flat two-tone checkerboard
as the tell with `mymod:ore` as the counter-example — a teal author is served.
`rendering.md:1593` enumerates four figures. `base__waterX.png` is absent, never
tracked, and ignored by `.gitignore:58`.

### Major — a live tolerance's stated derivation cites the figures this spec superseded

`crates/mc-client/tests/the_sea_the_camera_sees_is_the_water_layer.rs:89-92`
documents `SHOWS_THE_LAYER = 8.0` as "Derived from both directions in this
module's header: twice the ΔE **3.71** that *is* the furthest any texel of that
layer stands from its mean, and far below the ΔE **62.40** that separates it from
the nearest thing one of these pixels could be confused with." Commit `4109fa0`
rewrote that header to **3.16** and **25.34** and left this comment untouched, so
it now cites its own header for two numbers the header states only as superseded.

Measured here from the shipped PNGs: water's linear mean is `[76,121,158]`, its
furthest texel **ΔE 3.1616**, and the nearest wrong answer `base:stone`
(`[126,126,126]`) **ΔE 25.3391** — then sky 31.97, dirt 51.30, grass sides
51.54–51.87, grass_top 71.85. Both cited figures are false in the present tense.

**Failure scenario.** Someone widening this tolerance reads the constant's own
doc comment, takes the upper bound as 62.40, and picks anything in
(25.34, 62.40) — 30 looks to have 32 ΔE of margin. At 30 the guard accepts a
frame drawing **stone** where the sea should be and passes silently. This is the
only place in the tree still stating either figure as current; every other
mention (`rendering.md:1637`, `testing.md:4202`, `spec.md:402`) states them as
narrowed. `testing.md:4200-4202` argues in this spec's own words that a guard at
ΔE 62 versus ΔE 26 is the distinction that matters — while the constant it is
about still says 62.40.

### Major — three surviving "magenta-and-black checkerboard" statements

`docs/technical/rendering.md:1589`, `docs/technical/testing.md:4146`,
`docs/user/gameplay.md:58`. All three were introduced by this spec. The stand-in
pair is `(140,38,131)` and `(160,58,151)`: **ΔE 70.2 from black**, L\* 35.2 and
42.4, and only **ΔE 7.21 apart** from each other. Nothing black was ever on
screen — the sea was a low-contrast magenta pair that minifies to one flat
magenta, which is what `voxel-models.md:934-940` says correctly.

This is the repaired Minor's own defect, surviving at three sites. `5048b9c`
fixed `voxel-models.md:356` and `:370`; the same phrase persists, including
inside the repaired page. `rendering.md:1589` refutes itself in one sentence —
"the sea drew a magenta-and-black checkerboard. Counting the two stand-in texel
values" — where those two values are both magenta. `gameplay.md:58` tells a
**player** what they saw between 23 and 26 August, and its own line 119 ("a
two-colour checkerboard derived from the texture's name") contradicts it.

**Severity stated openly:** this sits at the Major/Minor boundary and is placed
above it on the precedent already applied in this spec — false statements in
`docs/`, in this spec's diff, of the class the repair addressed — plus the
player-audience reach and the self-contradiction inside a repaired page. If that
precedent is narrower than read here, this moves to Minor without disturbing the
first Major.

### Filtered out, not filed

- `rendering.md:1592`'s "the colour they minify to" is reproducible only as the
  observed third colour `(150,49,141)` (16 649 px at t000). The arithmetic mean
  `(150,48,141)` appears 42 times and gives **61 380**, not 77 987. The sentence
  is true and under-specified rather than false — **Info**, not Major.
- The stated box is a superset of a strict blend line, so it could over-include
  in principle. It does not: `fam&~differ` = 0 in all four.
- Items 1-7 of the standing rulings, re-checked as still applicable.

---

## Pass 4 — numeric-claim enumeration (independent)

**PASS with one Major.** Blockers 0 · Majors 1 · Minors 0 · Info 0. New findings
only. The Major does not touch an assertion, a tolerance or a shipped byte; it is
a stated derivation that does not produce its own figure, at two sites.

### Gate — taken on tree `6bcf15b`, log outside the repository

`pwsh -NoProfile -File scripts/sdd-gate.ps1` → exit 0, **12 `ok:` stages**,
`grep -cE "^\s*FAIL"` = 0, `warning:` = 0, no `Blocking waiting for file lock`
line. `Summary [132.841s] 1544 tests run: 1544 passed (11 slow), 1 skipped` — a
**bare** count, so a complete run. Coverage 93.72% lines / 92.24% regions,
10 983 lines tracked. `(11 slow)` against the repairer's `(12 slow)` is machine
variance.

**The reading dates itself.** The log names this spec's own new binaries green —
`no_committed_golden_shows_the_stand_in` at `(160/1544)`, the six
`the_sea_draws_its_baked_art` readings at `(378–383/1544)`,
`the_shipped_set_covers_every_key_it_declares` at `(424/1544)` and `(445/1544)`,
and `the_sea_the_camera_sees_is_the_water_layer` at `(568/1544)`. Those tests
exist only in this diff, so the run cannot have come from an earlier tree. HEAD
was `6bcf15b` before and after, clean, no stash. Provenance:
`git merge-base --is-ancestor a9c6663 main` → yes, `6bcf15b` → no.

### The enumeration, not a reading

`git diff a9c6663..HEAD` parsed to its 2 313 added lines, filtered to the **737**
carrying a digit or a colour word, reduced to **64 distinct numeric or colour
claims**. **62 reproduce.** Every figure was recomputed outside the project's
code — CIE76 reimplemented from `mc-testkit/src/frame/color.rs`, the goldens
decoded directly, the voxel model re-derived from the generator's own arithmetic.

Reproduced exactly, by group:

- **Stand-in and pre-spec goldens (18).** FNV-1a over `base:water` =
  `0x65098d12c9940ca1`; pair `(140,38,131)`/`(160,58,151)` at ΔE 7.2114, darker
  ΔE 70.2359 from black, L\* 35.2/42.4; minified `(150,49,141)`; arithmetic mean
  `(150,48,141)` occurring **42** times and giving **61 380**; strict triple
  **77 987 / 77 987 / 165 232 / 191 792**; box **88 280 / 88 280 / 174 744 /
  198 828**; **8.46–20.81%** and **9.58–21.57%** over **1280×720**; the defect
  table's columns 30 681 / 30 657 / 16 649; mean-only range **13 259 to 19 596**;
  the old text's own procedure **483 564 / 483 564 / 482 327 / 650 891** with sky
  395 284 at t000; `TEXTURE_EDGE` = 16. The re-shot goldens hold **zero** pixels
  of all three colours — counted here, not taken on report.
- **Water art (14).** `#4c799e`/`#447196`/`#5481a6`, H 207° S 52% V 62%, ±8 per
  channel (stone's ±29 confirmed likewise); linear mean **[76,121,158]**;
  furthest texel **3.1616**; two means byte-identical ΔE 0.0000; **87.9%** base
  tone (225 of 256 on the baked top face); widest pairwise **6.2889**; ΔE
  **59.1918** from the stand-in; blue dominant at all 256 texels; every distinct
  texel colour of all eight images is a declared material. The committed
  `water-block.mcvox` is **byte-identical** to `gen_water.py`'s output,
  re-derived here.
- **Cross-image separations (10).** stone **25.3391**, sky **31.969**, dirt
  **51.2985**, grass sides **51.5442–51.8695**, `grass_top` **71.8508**, in that
  order; grass sides **0.4744–1.0537** apart; dirt **9.5939** from
  `grass_side_west`; spreads dirt **16.10**, `grass_top` **17.01**, stone
  **22.87**; means `(138,106,70)`, `(104,165,78)`, `(126,126,126)`.
- **Tolerances (5).** `SHOWS_THE_LAYER` unchanged at **8.0**; 8/3.16 = 2.53
  ("over twice"); 25.34 − 8 = **17.34** and 62.40 − 8 = **54.40**, so "17 ΔE not
  54" holds; 30 > 25.34, so widening to 30 does admit stone; `MEANS_AGREE_WITHIN
  = 3.0` still the next whole number above 2.3776.
- **Counts and process (9).** Manifest seven → eight, and eight built PNGs.
  **Re-run here, not accepted:** the pre-mint selection gives `22 tests run: 22
  passed`, `golden_mismatch` **4 passed**, `golden_inventory` **3 passed**, and
  `terrain_goldens` + `hud_goldens` list **3** tests — every count in
  `rendering.md:1629-1634` reproduces. Four golden files modified, none added or
  removed. The defect window is real: water became `drawn` in `9fc98d7`,
  **2026-08-23**, so "the 23rd and the 26th" and "three days" are right, and
  "past 1 300 passing tests" is conservative against the registry's 1 370–1 435
  over that window.
- **Mutation table (2 of 6, the derivable ones).** M4's mutated palette gives
  water a mean of `(122,125,130)`, **ΔE 3.1438** from stone — the reported 3.14 —
  while keeping blue dominant and a spread of 6.30, so it does isolate FR-3.1-S2
  exactly as claimed. M3's `#a6a6a6` gives a spread of **33.56**, past dirt's
  16.10, and is not blue-dominant, so both reported failures follow.

Not re-run, by design: the M1/M2/M5/M6 red counts and the RED run's `9 tests
run`, which need either a mutated shared tree or the pre-fix tree; and the mint
step, which would rewrite goldens.

### Major — a tail described as one thing and counted as another

`crates/mc-client/tests/no_committed_golden_shows_the_stand_in.rs:31-33` and
`specs/active/2026-08-26-water-texture/test-map.md:49` both state: *"the frames
carry a tail one and two bytes either side of the mean — 10 293 of them at tick
0, 9 512 at tick 59, 7 036 at tick 119."*

The figures are right. The description does not produce them. Colours within two
bytes of `(150,49,141)` in the pre-spec `t000` blob, excluding the three exact
values, number **7 167**, not 10 293. What gives 10 293 is the *whole* RGB box
`(140,38,131)`–`(160,58,151)` minus the three — the same box `rendering.md:1617`
names, which reaches **ten** bytes from the mean, not two. Measured on `t000`,
by distance from the mean:

| bytes from mean | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
|---|---|---|---|---|---|---|---|---|---|---|
| pixels | 4 638 | 2 529 | 1 317 | 807 | 486 | 273 | 154 | 70 | 18 | 1 |

Cumulative at two bytes: 7 167 of 10 293. The stated bound accounts for **70%**
of the figure it is attached to; 3 126 pixels sit three to ten bytes out. The
alternatives all miss too — ±2 of the arithmetic mean gives 6 319, ±2 of any of
the three gives 7 211, ±1 of any of the three gives 4 646.

This is the same class as the three Majors above and the fourth instance found in
this spec: **a true number attached to a procedure that yields something else.**
It is placed at Major on that precedent rather than on impact — the sentence is
the module header explaining *why the scan is exact rather than tolerant*, so it
is the one place a later reader goes to decide whether a tolerance may be
introduced, and the arithmetic it offers them is wrong by a factor a reader
cannot see. The assertion itself is unaffected: it counts three exact colours and
expects zero, and it passes.

Secondary, in the same sentence: three figures are given for four captures, with
no "and in the HUD capture". Both tick-0 blobs do give 10 293 — verified — so it
is under-specified rather than false, and it is the shape `rendering.md:1594` and
lines 97-99 of the same test file were repaired *to* spell out. Repairing the
derivation is the moment to spell this out too.

**Repair:** state the counted set as the box, e.g. *"a tail of colours inside the
box the two texels span — 10 293 at tick 0 and in the HUD capture, 9 512 at tick
59, 7 036 at tick 119, reaching ten bytes from the mean"*, at both sites.

### Below the reporting bar, recorded so it is not re-found

- `the_sea_draws_its_baked_art.rs:94` and `spec.md:130` give the four grass
  sides' spread as **55.52**; it measures **55.5119**, so 55.51. All four sides
  agree to four decimals, and my reimplementation tracks the project's own metric
  to ~2e-4 elsewhere (51.2985 vs 51.2987, 71.8508 vs 71.8509), well inside the
  0.008 gap. It binds nothing — `NO_MORE_MOTTLED_THAN_DIRT` is 16.10 — so it is
  reported, not filed.
- `gen_water.py:25-27`'s "thirteen texels of each accent among two hundred and
  thirty" is the 5% *rate's* expectation, against a realised 225/21/10. It is
  explicitly prefaced "Rates, not counts", and the script prints the realised
  counts when it runs, so the stated procedure does produce the stated number.
  Not a finding — recorded because it reads at first like the class above and
  sits beside the 87.9% (225) asserted elsewhere.

### Standing rulings — checked, and one correction

Items 1-7 re-checked and still applicable. `art.rs:56`'s argmax attribution is
confirmed pre-existing: `git diff` shows "on `base:grass_side_north` and
`_south`" present in the removed lines too, so this spec only reflowed seven →
eight around it. Measured: `_east` **2.3776**, `_north`/`_south` **2.3757**.

One item was not on the list and is **also** out of boundary, noted so it is not
mistaken for new: `rendering.md:786`'s "Seven landed on 2026-08-19". The seven is
right — the manifest has exactly two commits ever, `bdbb021` with 7 entries and
`6ef024d` with 8 — but that spec is `2026-08-18-grass-block-art`, merged
**2026-08-21**, per its own registry line. The "2026-08-19" label is pre-existing
on `main` at `rendering.md:1354` and `:1383`, so this spec propagated a
convention rather than introducing a claim. Separately,
`measure_built_textures.py`'s "taken at `d2a342f`" cites a commit that is an
ancestor of neither `main` nor `HEAD`; that line is untouched by this diff. Both
are `main`'s to fix, not this spec's.

## Pass 5 — the repair's own claims, measured (independent)

**PASS.** Blockers 0 · Majors 0 · Minors 0 · Info 0. New findings only, Major
and above. Scope: the claims `d1ad355` *introduced* — pass 4's 62 reproduced
figures are settled and were not re-run.

### Gate — taken on tree `d1ad355`, log outside the repository

`pwsh -NoProfile -File scripts/sdd-gate.ps1` → exit 0, `GATE PASSED`, **12 `ok:`
stages**, `grep -cE "^\s*FAIL"` = 0, `warning:` = 0, no
`Blocking waiting for file lock` line (process table checked before starting: no
`cargo`, `rustc` or `nextest` live). `Summary [138.303s] 1544 tests run: 1544
passed (12 slow), 1 skipped` — a **bare** count, so a complete run; the two
earlier stage summaries are bare as well (`69 tests run`, `106 tests run`).
Coverage 93.72% lines / 92.24% regions, 10 983 lines tracked.

**The reading dates itself, and on the two files the commit under test edited.**
`no_committed_golden_shows_the_stand_in` green at `(160/1544)` and the six
`the_sea_draws_its_baked_art` readings at `(376–383/1544)` — both binaries carry
`d1ad355`'s edits, so the run cannot predate them. HEAD `d1ad355` before and
after, clean, no stash. Provenance by ancestry, not by name:
`git merge-base --is-ancestor a9c6663 main` → yes, `d1ad355` → no.

### The three introduced claims, each measured outside the project's code

CIE76 reimplemented from the CIE standard (sRGB→linear→XYZ D65→L\*a\*b\*,
Euclidean), the pre-spec blobs decoded directly from `a9c6663` and the built
PNGs from the working tree. Nothing below reads `mc-testkit`.

**1. The tail is the box minus the three, and the fourth figure is the HUD
capture.** Counting the RGB box `(140,38,131)`–`(160,58,151)` minus
`{(140,38,131), (160,58,151), (150,49,141)}`:

| capture | box | the three | **tail** |
|---|---|---|---|
| t000 | 88 280 | 77 987 | **10 293** |
| hud-t000 | 88 280 | 77 987 | **10 293** |
| t059 | 174 744 | 165 232 | **9 512** |
| t119 | 198 828 | 191 792 | **7 036** |

All four stated figures reproduce, and the HUD capture gives **10 293 with a
pixel-for-pixel identical distance profile to t000**, so "at tick 0 and in the
HUD capture" is exact rather than approximate. The subtraction is also
self-consistent with the sentence's own premise: 77 987 + 10 293 = 88 280,
165 232 + 9 512 = 174 744, 191 792 + 7 036 = 198 828.

**2. Ten is the realised maximum, confirmed rather than accepted.** Distance from
the mean `(150,49,141)`, per channel, over the t000 tail: 1→4 638, 2→2 529,
3→1 317, 4→807, 5→486, 6→273, 7→154, 8→70, 9→18, **10→1**. The single pixel in
the last bucket exists and is `(141,39,132)` — R 9, G **10**, B 9 from the mean.
It is not an artefact of one capture: the other three reach ten as well, at
`(141,39,132)` ×4 (t059) and `(141,39,132)` + `(160,58,150)` (t119). Nine would
be too low by one realised pixel; eleven is not reached, because the tail's
realised green never falls below 39. The claim is right in both directions.

**3. The grass sides spread 55.5119.** Widest pairwise separation over the eight
built PNGs, distinct colours only: dirt **16.0995**, `grass_top` **17.0112**,
stone **22.8713**, water **6.2889**, and each of the four grass sides
**55.5119** — identical to four decimals across all four, `(106,168,79)` against
`(119,88,64)` every time. 55.51 is correct and 55.52 was not; every companion
figure in the same sentence reproduces. In f64 the value sits 0.0031 clear of the
55.515 rounding boundary, far outside any arithmetic difference between this
reimplementation and the project's.

### The bounded sweep over `d1ad355` — complete, not a reading

10 insertions / 9 deletions across four files, parsed added-line against
removed-line. Six claims are genuinely new: the box framing, the two corner
triples at this site, "minus the three themselves", "and in the HUD capture",
"reaching ten bytes from the mean", and 55.51. **All six are measured above.**
Everything else on the added lines is reflow of surviving text whose figures
pass 4 enumerated (88 280 / 174 744 / 198 828, 77 987 / 165 232 / 191 792, "tens
of thousands of each"). The commit message's own three figures — 7 167 within
two bytes, 10 293 for the box, 55.5119 — all reproduce; 4 638 + 2 529 = 7 167.

No stale sibling survives the repair: `55.52` and the "one and two bytes either
side" phrasing now appear nowhere in the tree outside `validation-report.md`,
where they are quoted evidence. The repaired sentence agrees with
`rendering.md:1618`, which names the same box and says it "contains the minified
mean and every blend between them" — the repair moved the test header *onto* the
document rather than away from it.

### Considered and below the bar, recorded so it is not re-found

"Reaching ten bytes from the mean" is a statement about the **counted set**, and
the counted set does reach exactly ten. Read instead as a statement about the
box's geometry it would be wrong by one: the box spans 11 bytes below the mean in
green (38 against 49), because `(150,49,141)` is the minified colour and not the
pair's arithmetic mean `(150,48,141)`. The grammar attaches the clause to
"10 293 of those", the excluded corner `(140,38,131)` is the only pixel at green
38, and no other pixel in any capture reaches it — so the stated procedure does
produce the stated number. Not a finding; noted because the asymmetry is
re-findable.
