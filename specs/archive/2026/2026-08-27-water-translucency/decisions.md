# Decisions taken during implementation — SPEC-031

Decisions the phases took that `architecture.md` did not settle in advance.
Architecture decisions stay there; this file holds what implementation had to
decide, and why.

## D-I1 — FR-1.3's condition reads the **resolved** `occludes`, not the written line

**Taken in phase 1, out of a test-author dispute. Verdict `test-wrong`, upheld by
the lead.**

`occludes` falls back to whatever the same declaration says about `solid`
(`luau_declaration/mod.rs`, `defaulting_to_solidity`), and **the mesher reads the
resolved value**. So the refusal fires on the resolved `occludes`.

**FR-1.3-S1 does not adjudicate between the two readings, and its wording is not
at fault.** It is an implication, not a biconditional: *"IF states-both THEN
refuse"* is satisfied *a fortiori* by the resolved reading, because an author who
writes `occludes = true` has a resolved value of `true` as well. It says nothing
about what else may be refused. The scenario reads as favouring the written line
only if an "only if" is read into it that is not there. **No amendment was made
to it**, deliberately — editing it would put a correction in the record for a
scenario that was never wrong, and send a later reader looking for a defect that
does not exist.

**What binds is `architecture.md` Decision 7**: the refusal *"is what guarantees a
translucent block never hides what lies beyond it"*. Under the written-line
reading that guarantee is false — `solid = true, opacity = 0.5`, which is glass
and the spec's own second user story, would register, draw a translucent face,
have the geometry behind it culled by the mesher, and **be refused by nothing**.
`optional_boolean`'s own doc names that outcome as the worst available: the block
behaves exactly as if the line had never been written, so there is no symptom to
notice. A refusal that misses the case it was written for is worse than no
refusal, because it reads as coverage.

The cost of the resolved reading is one line (`occludes = false`) an author has to
write anyway for the feature to work. Water is unaffected either way — it is not
solid.

**FR-1.3-S2 was added to the spec** rather than left as a test author's
preference: the dispute exposed that the refusal quoted `` `occludes = true` `` at
an author whose file contains no such line. Two situations, two remedies — one
names a line to delete, the other a line to add — and the scenario count moved
**44 → 45** deliberately and in the record. Adding a falsifier is never the
problem; removing one to satisfy a count is.

## D-I2 — the bounded reader takes its bounds as one value, not as a floor and a ceiling

`architecture.md` Decision 7 first wrote
`optional_number_within(declared, field, floor, ceiling, absent)` — five
parameters. **Measured:** `clippy.toml:10` sets `too-many-arguments-threshold = 4`,
so `-D warnings` refuses that spelling and the gate cannot pass with it.
`code-quality.md` §2 names the remedy — *"max 4 parameters (use an object
beyond that)"*.

The object carries the range **and the words each end is refused in**, because the
sentences are prose a mod author reads: `` `opacity` may not be more than one `` is
a sentence and *"more than 1"* is a diagnostic. Formatting the bound from the
number would have produced the second. `architecture.md` is corrected in place.

## D-I3 — the appearance-revision-3 fixture was minted after the byte had already moved

The order to mint crossed with T05 landing, so the fixture was written from a
**detached worktree at the pre-T05 commit** rather than from the live tree.

**That is legitimate, and `docs/technical/world-format.md` was corrected because it
said otherwise.** The page claimed *"There is no second chance at minting one"*.
That is true of a *run* — `APPEARANCE_REVISION` is a compile-time constant — and
false of a *repository*, which still holds the tree. What the page was protecting
is the next sentence: a save the suite wrote from the declarations under test
agrees with them by construction and cannot fail. **Independence, not scarcity, is
what makes a fixture evidence**, and independence survives the move intact.

**The provenance is carried as a measured property rather than as a commit sha**,
because the branch squashes at merge and a save cannot evidence its own revision
(`input_version` folds into a `DefinitionHash` and is never stored raw):

```
BEHAVIOUR IDENTICAL : [true,  true,  true,  true ]
APPEARANCE IDENTICAL: [false, false, false, false]
```

Re-derivable by anyone holding the two trees.

**And corroborated by a genuinely independent oracle, which is the strongest
evidence this fixture has.** `format_test.rs` builds the expected appearance
record **by hand**, and while it still stated revision 3 its four expected hashes
were **byte-identical** to the four the minted fixture carries:

```
base:dirt  3800391986783124173     base:stone  3118085479270330672
base:grass 14775375301334149993    base:water  1385907616569087297
```

**Two programs, two routes, one answer, and neither was written to check the
other** — the hand-built oracle predates this spec entirely. That is stronger than
the worktree comparison above *and* stronger than the mint itself, because both of
those read the same writer while this one reads none of it.

**Note what produced it: a test broken by accident, found by a run nobody scoped
to it.** The appearance byte moved in T05 and this guard reddened; a run scoped to
the four save binaries T05 was expected to touch reported `15 tests run: 15
passed` and said nothing about it. **A run scoped to what you expect to move
cannot report what you did not expect to move** — the same shape as a cancelled
`N/M` count, a result that reads like a verdict and is only a sample. The
unscoped run is what paid, and it paid twice: it caught the regression and it
produced the corroboration.

## D-I4 — window B carries the `TextureResolution` signature, and T07 leaves the tree drawing

**Taken in phase 2, at T07.** Two departures from `tasks.md`'s T07/T09 split, both
forced by the same rule: **a window is closed by the test author, so anything that
breaks a test file must break inside one.**

**The `TextureResolution` change moved from T09 into T07.** `architecture.md`
Decision 5 puts the opacity "beside the six keys", and `resolution.rs`'s own header
says why it may not be a second map: *"One type rather than two values travelling
side by side"*, because a resolution answering one question and not the other is
the plausible wrong picture that module exists to prevent. So `stating` takes a
block's name, its keys and its degree together, which is a signature change — and
had it landed at T09 it would have broken six test files **after** the test author
had finished with them, leaving an implementer holding a repair he may not make.

**Measured, and window B is larger than the tasks phase predicted.** It measured
`Vertex {` and `SectionRecord {` alone and found three sites in two files. With
`TextureResolution::stating` inside the window it is **ten sites across six
files** — `cargo check --workspace --all-targets --message-format short
--keep-going` at `bf1ed56`:

```
mc-render/src/camera_test.rs:176            SectionRecord
mc-render/src/geometry/mod_test.rs:190,207,231   stating
mc-render/src/geometry/vertex_test.rs:49,69,87   Vertex
mc-render/src/hud/held_test.rs:268          stating
mc-render/tests/frame_statistics.rs:96      stating
mc-render/tests/support/mod.rs:152          stating
```

**And T07 keeps the renderer drawing, which is why the shaders move a commit
early.** A 48-byte section record read by a 44-byte WGSL struct is every terrain
frame wrong, and a phase whose whole suite is red for that reason has no RED left
to read: `testing.md` §2 wants an *assertion* failure, and the tasks phase's own
reason for opening both windows on an implementation commit is exactly that. So
both `Section` structs gain the field at T07 and neither shader reads it. The same
argument carries `ARGS_BYTES` 20 → 40: `buffers.rs` states that reading the
indirect arguments before the first frame must report *a declared value rather than
whatever the allocator handed over*, so growing the buffer without declaring the
second record would leave twenty bytes that property does not cover. The upper
half's base is written there now, and the index buffer doubles with it, so
`args[1].first_index` is a true statement about the buffer from the commit that
first writes it.

**What T09 keeps.** The partition itself — opaque quads emitted first, then
translucent, and `SectionGeometry` reporting `opaque_quad_count` — plus the
`docs/INDEX.md` reload row. `SectionRecord.opaque_quad_count` is filled at T07 with
the section's whole quad count, which is not a placeholder: no block this tree
draws passes light, so every quad in it is opaque and the field is correct until
the packer partitions.

## D-I5 — FR-3.3-S1 was amended, because it was false under the chosen engine

**Taken in phase 2, out of an implementation report. Ruled by the lead; wording
composed here to constraints the lead set.**

`architecture.md` Decision 2 chose the **product** model: the declared degree
alone decides which draw a face lands in, and the texel's alpha modulates within
the blended one. The scenario as written asked for *the even blend* from a
texture carrying alpha `128`, which that model cannot reach — an even blend from
that texture needs a declared degree of one, and a block at one degree draws in
the opaque pass and never blends at all.

**Its `WHERE` clause has two readings and the scenario fails under both**, which
is what makes this an amendment rather than a preference:

- read as *takes alpha from the texture at all*, the chosen model enters the arm,
  the SHALL applies, and the engine never produces that blend — the scenario
  **contradicts a binding decision**;
- read as *takes alpha from the texture alone*, the chosen model never enters the
  arm, the scenario **fires never and asserts nothing**, and FR-3.3 is left with
  no falsifier at all, S1 being its only wording.

`architecture.md` itself takes the first reading: Decision 2 argues *for* A3
partly on the ground that under A1 "FR-3.3-S1's `WHERE` arm would never be
entered", treating the arm being entered as a benefit. So the document already
reads it as entered — which is the reading under which it is false. The clause is
dropped rather than repaired: a condition on which model gets chosen is dead
weight once the choice is binding.

**Why this does not follow D-I1's precedent, which went the other way.**
FR-1.3-S1 was **true as written**; it simply did not adjudicate a dispute an
"only if" had been read into it, and amending it would have put a correction in
the record for a scenario that was never wrong. FR-3.3-S1 is **not true under the
chosen engine**. Removing a false statement and correcting a true one are
different acts, and the precedent does not carry from one to the other.

**What the amendment keeps.** The requirement's heading is untouched. The named
even blend is still present, as the observable of the opaque-textured block. The
tolerance still refers to FR-2.1-S1, which was never wrong. And the two blocks
differ in **exactly one byte** — the alpha their textures carry — which is what
makes "the texel's alpha reaches the sampler" falsifiable at all: a single block's
colour is consistent with an alpha read from the declaration, from the texture,
or from their product, and only the pair separates them. Scenario count stays
**45**: nothing added, nothing removed.

**The commit order was wrong and the record briefly disagreed with itself.**
`c84f17f` reconciled the test file's header and `test-map.md` against this
amendment, and both cited "D-I5", **before this entry or the amended wording
existed**. For two further commits `spec.md:113` stated the original wording
while two other documents stated it had been replaced and named a decision that
could not be read. Written down because a later reader meeting the two halves out
of order deserves to know the disagreement was a sequencing mistake and not a
second dispute.

## D-I6 — three numbers stopped being true when the second draw landed, and each moved

**Taken in phase 2, at T10.** The blended pass is one design decision with three
consequences that `architecture.md` did not enumerate, each of which is a
statement somewhere in the tree that silently became false.

**`TERRAIN_DRAW_CALLS` 1 → 2, and it is not a weakening of what that constant
watches.** `mc-render/CLAUDE.md` binds terrain to one indirect draw and names a
per-*chunk* CPU draw call as the regression. Two fixed draws, one per layer,
keep the property that rule is about: the number moves with neither what is in
view, nor how many sections there are, nor how much of the world passes light,
because a frame declaring nothing translucent issues the second draw over **zero**
indices rather than skipping it. The count stays a property of the pipeline
rather than of the scene, which is what the constant's own doc says it is for.
Four existing tests pin the old number and two carry it in their names; that is a
dispute to the test author and never an edit.

**`read_drawn_index_count` adds both halves.** It read the first `u32` of the
arguments buffer, which is `args[0].index_count` — the opaque half. That answer
stays right for exactly as long as no content declares a degree below one and
then starts under-reporting with nothing to notice it, which is the shape of
failure the readback exists to catch. It now strides the arguments and sums, so a
third draw would be counted without the reading being edited.

**`SECTION_BYTES` is no longer a second literal.** R6 named `buffers.rs`'s `44`
and `scene.rs`'s `44` as two literals of one number and asked Decision 11 for a
stride check. The cheaper closure was to delete one of them: `scene.rs` states
`SECTION_RECORD_BYTES` and `buffers.rs` allocates at it. One definition beats a
check over two.

## D-I7 — the validator's new checks run last, because a stage may declare what it does not read

**Taken in phase 2, at T10, out of two tests going red.** Decision 11's checks
were first placed before the winding and plane-axis checks, and `shader_validation`'s
fixtures — minimal shaders written under the real files' names to exercise one
check each — were then reported as `SectionRecordMismatch` instead of as the
fault they were doctored to have.

**The ordering rule that resolves it is the one `validate_source` already
applied**, and it is about which report is most specific rather than about which
test passes: the tables a shader *reads* are checked before the layouts it merely
*declares*. The terrain stage declares the whole section record and reads only the
origin out of it, so a section-record complaint is the least specific thing that
can be said about a shader and goes last.

**Measured, four mutations, reverted by hand and both shader files confirmed back
to their exact pre-mutation `sha256`.** An `OPACITY_SHIFT` of 35 against 36
reddens the build; `terrain.wgsl`'s `Section` short a field reddens it;
`cull.wgsl`'s `min_x` and `min_y` exchanged reddens it. **The fourth did not bite
the check it was aimed at**: `cull.wgsl` short the same field is caught by naga's
own `invalid field accessor`, because that stage *reads* `opaque_quad_count`. So
the deletion case has an independent witness in one shader and only this check in
the other — which is precisely the terrain stage, and precisely why the check
exists.

**The revert check itself turned out to be the finding, and it is general
enough to have its own entry: see D-I8.**
## D-I8 — when `git diff --exit-code` is a valid revert check, and when it answers a different question

**Found in phase 2 while reverting T10's mutations.** Numbered D-I8 rather than
the D-I6 the instruction to record it named, because D-I6 and D-I7 were already
written when it arrived; the substance is unchanged.

`standards/global/git-workflow.md` says to revert a mutation by hand and confirm
with `git diff --exit-code`. That is right, and it has a precondition the
sentence does not state:

> **`git diff --exit-code` isolates a revert only when the file's *other* work is
> already committed.**

A mutation made in a file that also carries uncommitted work leaves `git diff`
non-empty whether the revert succeeded or not, so the check cannot answer the
question it is being asked — and it fails *open*, reading as "still dirty, as
expected". That is exactly the position an implementer is in mid-task, which is
the position the protocol is most often reached for from. The remedy is not a
better command: **commit the file's own work before mutating it**, and the
standing check works as written.

**A whole-file hash is the substitute, and it answers a third question.** With
`core.autocrlf=input` git normalises line endings before hashing, so
`git hash-object` and `sha256sum` disagree by construction about a file whose
endings moved. Which to reach for:

| Question | Instrument |
|---|---|
| is the *content* back, as git will record it | `git hash-object <file>` against `git rev-parse :<file>` |
| is the *file* back, byte for byte on disk | `sha256sum` against a baseline taken before mutating |
| is *this file's* diff empty | `git diff --exit-code`, **only** where its other work is committed |

Measured here twice, in opposite directions. A Python text-mode write on Windows
rewrites a newline as a carriage-return pair, so an edit script silently
converted two shaders LF to CRLF: `sha256sum -c` failed on a file whose content
was correct, and only the byte-level reading could say which kind of difference
it was. Later the same files read *clean* under `git hash-object` and `git diff`
while `git status` reported them modified — racily-clean stat entries that
survived `git update-index --really-refresh` and cleared only when the index was
rewritten by naming those paths to `git add`. **One instrument said changed and
the other said unchanged, and each was right about its own question.**

**A mutation window is the fourth producer, and the reflex it invites is the
dangerous one.** The sightings above all came from a file rewrite. A fourth came
from a **mutate-then-revert-by-hand** cycle, which this project runs deliberately
and will keep running. The hazard is not the entry; it is the reading a person
forms of it — *`git status` says modified after I reverted by hand* reads as **my
revert failed**, and the reflex that follows is `git checkout -- <file>`, the one
command `git-workflow.md` bans by name because it once wiped an uncommitted
implementation in this repository.

> **After a by-hand revert, expect the entry.** Confirm the revert with
> `git diff --numstat` or the three-way identity of worktree, index and `HEAD`,
> and clear it by naming the single path to `git add` — which rewrites the stat
> entry and stages nothing while the content matches. **Never `git checkout --`.**

And the artefact belongs to whoever's mutation produced it, **even when the file
is somebody else's**: ownership of the litter is not ownership of the code. That
is what lets the person who cannot stage the path ask, and the person who can
clear it without either of them breaking the staging rule.

**Stated at its true severity: nothing would have reached `main`.** The index
holds LF and git normalises on commit, so the conversion was invisible past the
working tree. What it cost was a wrong answer from a revert check, at the one
moment that check is load-bearing.

**This repo has met the hazard before and wrote it down for one case only.**
`.gitattributes` says line endings are *"the one thing standing between a golden
and a line-ending rewrite"* and pins them for byte-sensitive golden frames. This
is that hazard's other face — not a golden being rewritten, but an *evidence
check* unable to tell a rewrite from a change. Whether `git-workflow.md` should
carry the precondition is a constitution question and not this spec's to answer;
it is recorded here so it survives consolidation.

## D-I9 — what the build-time layout checks can and cannot see, measured cold

**Phase 2, after T10. The measurement was reserved to be taken by someone who did
not know the test author's answer, so the prediction below was written before the
run.**

Decision 11 added two checks over layouts this delta hand-duplicates. Both compare
a **shader** against `validate_tables`' copy. The open question was whether
anything at build time compares either of them against the **type** they stand
for — a check that had come to compare a shader against a stale constant would
bite on every shader-side mutation and stay silent forever on the half that
actually drifts.

**Predicted before running**: the build stays silent; the author's vertex
agreement test reddens; the translucency frame readings redden; `vertex_test.rs`'s
round trip stays green; every golden stays green.

**The first attempt was impure, and the number it produced answers the wrong
question.** `OPACITY_SHIFT` 36 → 35 reddened **23** tests — but at 35 the field
overlaps the section index at bits 26..36, so most of that list reports a
corrupted *section*: the probes, both drawn-side readings, the oracle, the
offscreen reading and **both golden suites**. It also reddened `vertex_test.rs`'s
round trip **against the prediction**, because the author had widened that test to
assert the section as well, on the stated ground that the new field "is packed
directly above the section index, which is the one edit that can shift its
neighbour without shifting anything else". Reasoning about a constant in isolation
missed what reasoning about its neighbour caught.

**The clean probe**: `OPACITY_SHIFT` 36 → **37**, into spare bits, no overlap —
only the degree wrong. `1634 tests run: 1627 passed, 7 failed, 1 skipped`, a bare
count.

| | |
|---|---|
| **Reddened** | `shader_validation::the_validators_vertex_layout_is_checked_against_the_bits_packing_actually_writes`, and six `mc-client` frame readings |
| **Did not redden** | **the build check**; **both golden suites**; `vertex_test.rs`'s round trip; `the_sea_the_camera_sees_is_the_water_layer`; `terrain_probes`; `replay_oracle` |

**Three conclusions, each from the did-not-redden half.**

1. **The build-time checks are blind to the real type**, confirmed rather than
   inferred. The two copies agree with each other about a number neither reads.
   This is written into `build/validate.rs`'s own header as a stated limit rather
   than fixed, because the witness that closes it already exists and is cheap.
2. **One test sees the real packed opacity without a device** — the agreement
   test added in `1b75e53`; before that commit, none. **This one is derived and
   not measured, and the distinction is the point**: the seven reddening tests
   were sorted by which suite they live in, six being `mc-client` frame readings
   that ask `support::device()` for an adapter. Nobody withheld a device and
   counted. The method that settles it is the test author's, used on the seam a
   day later: stack a second mutation on the device helper so it offers none, and
   read the count. **An environment variable you have not seen take effect is a
   hypothesis about your machine**, and `MYCRAFT_ALLOW_NO_GPU` proves nothing here
   because an adapter answers.
   **A narrowing that belongs with it, and it is the test author's.** The widened
   round trip reddening under the *impure* probe is **not** a witness for this
   field: it caught the opacity landing in the section's bits, which is geometry
   corruption. Under the clean probe it stayed green. Counting it here would turn
   one witness into two by mistaking a neighbour's alarm for this field's.
3. **The goldens are blind here, for the third recorded time on this line.** No
   shipped block declares a degree below one, so every face is drawn in the opaque
   pass and the wrong byte is written, read and discarded — the same shape as the
   zeroed-alpha finding in T08 and the empty-changed-set argument against T13's
   subset check. Three instruments, three different reasons, one seam. It expires
   the day shipped content declares a degree, which is phase 3.

## D-I10 — the instrument is part of the claim, and it fails in three distinguishable ways

**General, not spec-local.** It belongs in `docs/technical/testing.md` at
consolidation, and it is written to be lifted whole. Every one of the three was
committed by a real agent on this spec — the third by two of them independently
inside one phase — which is what makes it a taxonomy rather than a warning.

The claim being made is never only "X is true". It is "X is true, **and here is
the instrument that says so**". A reader who accepts the first half without the
second has accepted something nobody checked, and the three ways that happens
have three different tells.

**1. Unrun instrument — a prediction phrased as an observation.** Nothing was
measured; a sentence in the past tense says otherwise. The tell is the
**grammar**, and it is reliable enough to grep for: *"therefore it has never"*,
*"with the variable set"*, *"six of these need a device"*. Each is a claim about
what a run *would* report, written in the shape of what a run *did* report. The
repair is not to soften the wording — it is to run the thing, and if it cannot be
run, to say which reading is missing and why. `testing.md` §2's rule that a green
suite is no evidence about a lint is the same failure with the instrument named.

**2. Wrong instrument — a real measurement of the wrong quantity.** Something was
genuinely run; it answered a different question. `wc -l` counts every line and the
gate counts non-blank ones, so a file the gate accepts at 560 is reported over
budget at 640. The tell is **implausible scale**: a measurement that indicts
everything is indicting its own instrument. Nine files over a limit that has been
green for six phases is not nine defects. Before reporting a sweep, check one
case by the instrument that actually decides, and prefer that instrument outright
where it can be invoked.

**3. State read as cause — evidence for *what is*, extended into *why*.** The
observation is correct and the inference is not, because more than one history
produces it. The tell is a question that can be asked mechanically: **would a
second cause leave an identical observation?** A reverting mutation probe and a
premature edit leave the same tree. A stale filesystem stat entry and a live
editor leave the same ` M`. A file byte-identical to `HEAD` says nothing whatever
about who touched it or when — `git hash-object <path>` against
`git rev-parse :<path>` answers *whether it differs*, and no comparison of a tree
against itself answers *why*. Where the cause matters, the instrument has to be
one that records history: a reflog, a commit, an announced window with a
recorded failing count.

**The taxonomy caught its own author, four commits after it was written, and
that is recorded rather than tidied away.** Investigating whether a figure's
premise had moved, this context measured four composite colours against
`base:stone` and reported one at ΔE 5.98 as a possible tolerance failure. They
were `base:dirt` composites: **"lakebed" was taken to mean stone without checking
which layer the lakebed is made of.** A real measurement, of the wrong quantity —
entry 2, exactly. Two things follow. **The wrong-instrument tell is not always an
implausible magnitude; sometimes it is a noun nobody verified.** And **the
enumeration is what caught it** — not care, and not the author's own knowledge of
the rule, which had been written down days earlier and did not help. That is why
the entry prescribes enumerating the family rather than prescribing attention: a
rule that fails to protect its own author on first contact is reporting that its
tell is hard to see from the inside.

**The rate did not fall with practice, and that is the finding rather than an
aside.** Inside one phase, after the entry above was written, the same context
stated the wrong *quantity* three times: a composite measured against a layer
that is not its operand; a ceiling taken as the minimum over one column when two
bounded the same thing; and a tolerance bracket taken from the
layer-against-layer question when a blend is bound by
composite-against-its-own-operands — that last one worth a factor of four in the
claimed headroom, `(4.3, 24.93)` where the truth was `(4.3, 9.46)`.

**All three corrections came from somebody asking *which quantity is that*, and
none from the arithmetic being re-checked.** The arithmetic was right every time.
And the third was found only by re-applying the question to text nobody had
pointed at, which is what distinguishes fixing a class from fixing an instance.

**A fault whose rate does not fall with repetition is not an attention problem,
and instructing people to be careful about it will not work.** That is the whole
reason this entry prescribes *enumerating the family* and *naming the quantity*
rather than prescribing care: care was present, the rule was known, its author
had written it days earlier, and none of that helped. What helped was a
mechanical question asked by a second party. **Build the question into the
procedure, or expect the fourth instance.**

**What ties the three together** is that each looks exactly like a passing step
in a report. An unrun instrument reads as a run one; a wrong instrument reads as
a damning result; a cause read off state reads as a diagnosis. So the discipline
is not scepticism about conclusions — it is that **a reading names its
instrument**, and a reading whose instrument is not named has not been taken.


## D-I11 — when a guard and a document disagree, the guard is usually the one shaped wrongly

**General, not spec-local**, and it belongs beside D-I10 at consolidation.

A guard over prose that matches literal phrases constrains how a sentence may be
written. The moment it does, **it has stopped checking the document and started
editing it** — and the pressure lands on the reader, because the cheapest way to
green it is always to write worse prose.

Measured here: this spec's documentation guard looked for
`"Depth-write off in the second draw"` against a sentence reading
`Depth-write **off** in the second draw`, with the emphasis on the word the
sentence turns on. Removing that emphasis to satisfy a matcher would have been an
agreement test relocated into documentation, paid for in legibility.

**The test to apply is: which of the two has a reader?** The document does; the
guard does not. So the guard moves.

- **Prefer reading a structure over matching a phrase.** The repair here was to
  read the pass table's `depth write` column rather than a sentence about it —
  after which emphasis, rewording and sentence order are all free and the guard
  still reddens if the document stops stating the setting.
- **Where there is no structure to read, normalise before matching** — emphasis
  markers out of both haystack and needle. This is a patch rather than a fix: it
  frees a bolded word and still breaks on a reworded clause, so it belongs only
  where nothing structural exists to key on.
- **Compare stated numbers against measured ones, never against constants copied
  into the test.** A guard checking a list of strings against prose written to
  contain them is two copies agreeing with each other, and neither has to be true
  about the engine.
