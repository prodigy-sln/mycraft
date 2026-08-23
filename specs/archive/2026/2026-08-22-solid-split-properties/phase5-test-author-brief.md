# Phase 5 — test-author brief

**Spec**: `2026-08-22-solid-split-properties` (PRO-904), rigor `high`.
**Branch**: `feature/PRO-904-solid-split-properties`.
**Tree this brief was written against**: `477137d8d9d6e8d48c11b1eb8aef342ff8ad8361`
— *"docs: the renamed type across the living docs, and an ADR amendment"*.
Verified at the moment of writing with `git log -1 --format="%H %s"`,
`git status --short` (empty) and `git stash list` (empty). **Re-read the tree
before you act on anything here**: more than one agent may hold it, and an
observation of it ages exactly as fast as anybody else's.

**Phase 5 is the last phase.** It closes nine scenarios: FR-5.1-S1, FR-5.1-S2,
FR-5.1-S3, FR-5.2-S1, FR-5.2-S2, FR-5.3-S1, FR-7.2-S1, FR-7.2-S2, FR-7.2-S3.
Nothing here moves a pixel. The gate must be green at phase end.

Everything below was read off this tree. Where a line number appears it was
produced by a `grep -n` on this tree and is quoted with the text that was on it,
so a drifted number is recoverable from the words.

---

## 1. What the phase is, in one paragraph

A block's declaration now carries three fields the save format has never
recorded: `targetable`, `drawn`, `occludes`. This phase decides which of the
save's **two** folds each of them joins, and moves both revision bytes to say
so. `targetable` joins the **behaviour** fold (`BEHAVIOUR_REVISION` 1 → 2);
`drawn` and `occludes` join the **appearance** fold (`APPEARANCE_REVISION`
2 → 3). Separately and in the same phase, the **reload's geometry key** learns
`drawn` and `occludes`, loses `solid`, and stays ignorant of `targetable`.

**The two halves are each other's cross-check, which is why they are one
phase.** An implementation that folded all five fields into one key passes
FR-7.2-S1 and S2 and fails only FR-7.2-S3. One that put `drawn` on the
behaviour list passes FR-5.2 and fails FR-5.1-S2 and FR-5.3. Neither half
alone catches the mistake, so both halves need tests before either lands.

---

## 2. The nine scenarios, verbatim from `spec.md`

> - **FR-5.1-S1**: WHEN two content roots differ in nothing but one block's
>   `targetable` THE SYSTEM SHALL record different declared behaviour for that
>   block and identical declared appearance
> - **FR-5.1-S2**: WHEN two content roots differ in nothing but one block's
>   `drawn` THE SYSTEM SHALL record identical declared behaviour for that block
>   and different declared appearance
> - **FR-5.1-S3**: THE SYSTEM SHALL state a behaviour fold whose leading byte is
>   `2` and an appearance fold whose leading byte is `3`, each asserted as a byte
>   sequence built by hand
> - **FR-5.2-S1**: WHEN the committed pre-spec save is loaded against the shipped
>   content THE SYSTEM SHALL report a verdict whose changed list is exactly
>   `base:dirt`, `base:grass`, `base:stone`, `base:water` in ascending order,
>   whose missing list is empty and whose retextured list is empty, and SHALL open
>   the world naming those blocks on the error stream in one line
> - **FR-5.2-S2**: IF that load is asked to refuse changed blocks THEN THE SYSTEM
>   SHALL refuse it, naming all four
> - **FR-5.3-S1**: WHEN two content roots differ in nothing but one block's
>   `occludes` THE SYSTEM SHALL report a verdict whose retextured list names
>   exactly that block and whose changed and missing lists are empty
> - **FR-7.2-S1**: WHEN a reload candidate changes nothing but one block's `drawn`
>   THE SYSTEM SHALL re-mesh the world so the change is visible without a relaunch
> - **FR-7.2-S2**: WHEN a reload candidate changes nothing but one block's
>   `occludes` THE SYSTEM SHALL re-mesh the world so the change is visible without
>   a relaunch
> - **FR-7.2-S3**: WHEN a reload candidate changes nothing but one block's
>   `targetable` THE SYSTEM SHALL report an accepted reload whose published serial
>   advances and whose rebuilt-section count is zero

---

## 3. The code as it stands — verified, with the numbers

### 3.1 The two folds

`crates/mc-world/src/persistence/format.rs`

| Line | Text on it today |
|---|---|
| `:275` | `const BEHAVIOUR_REVISION: u8 = 1;` |
| `:276` | `const APPEARANCE_REVISION: u8 = 2;` |
| `:299` | `struct DeclaredBehaviour<'a> {` |
| `:331` | `struct DeclaredAppearance<'a> {` |
| `:339` | `pub(crate) fn behaviour_of(definition: &BlockDefinition) -> DefinitionHash {` |
| `:360` | `pub(crate) fn appearance_of(definition: &BlockDefinition) -> DefinitionHash {` |
| `:380` | `fn folded(declaration: &impl Serialize) -> DefinitionHash {` |

Today's field lists, read off the structs:

```rust
struct DeclaredBehaviour<'a> {
    input_version: u8,
    name: &'a str,
    is_solid: bool,
    replaceable: bool,
    breakable: bool,
    breaks_into: Option<&'a str>,
}

struct DeclaredAppearance<'a> {
    input_version: u8,
    name: &'a str,
    textures: [&'a str; 6],   // Face::ALL order
}
```

`folded` encodes with `postcard::to_stdvec` and folds with `fnv_1a_64` (which
now lives in `mc_core::hash`). Both structs are `#[derive(Serialize)]` and
**written out by hand rather than derived from `BlockDefinition`** — a derive
would bind every save to a struct that changes for other reasons.

### 3.2 The reload's geometry key

`crates/mc-sim/src/world/reload.rs`

| Line | Text on it today |
|---|---|
| `:53` | `let redraws = changes_geometry(world.registry(), &registry);` |
| `:58` | `world.mark_every_section();` |
| `:70` | `fn changes_geometry(serving: &BlockRegistry, candidate: &BlockRegistry) -> bool {` |
| `:83` | `fn drawn_of(registry: &BlockRegistry) -> BTreeMap<&BlockName, (bool, &FaceTextures)> {` |

`drawn_of` maps each name to `(declared.is_solid, &declared.textures)`.
`changes_geometry` compares the two maps; a difference marks **every** section.

### 3.3 The three new declaration fields, and their defaults

`crates/mc-core/src/block/definition.rs` — `BlockDefinition` carries
`drawn: bool` (`:84`), `occludes: bool` (`:91`), `targetable: bool` (`:99`),
appended after `breaks_into` and before `origin`.

`crates/mc-world/src/content/luau_declaration/mod.rs:189-209` —
**each of the three defaults to whatever the same declaration says about
`is_solid`** when the field is absent, through `defaulting_to_solidity`.
`replaceable` and `breakable` default to constants instead.

**This default is load-bearing for every fixture you write.** A declaration
that states only `solid = false` states *all four* of solid, drawn, occludes and
targetable as `false`. So:

- A candidate that says `solid = false` and nothing else **does** move the new
  geometry key — it moves `drawn` and `occludes` with it. The existing
  solidity-driven remesh scenarios therefore stay green after T21, and that is a
  fact about the defaults rather than about the key.
- A fixture meaning "nothing but `targetable` moved" must state
  `targetable = false` while leaving `solid = true`, so that `drawn` and
  `occludes` default to `true` on both sides.
- A fixture meaning "nothing but `drawn` moved" must state `drawn` explicitly
  and leave `solid` where it was.

### 3.4 The shipped content, read off disk

`content/base/blocks/` holds exactly four `.luau` declarations.

| Block | States |
|---|---|
| `base:dirt` | `name`, `texture`, `solid = true` |
| `base:grass` | `name`, six-facing `texture` table, `solid = true` |
| `base:stone` | `name`, `texture`, `solid = true` |
| `base:water` | `name`, `texture`, `solid = false`, `breakable = false`, `replaceable = true`, `drawn = true`, `occludes = false`, `targetable = true` |

So dirt, grass and stone resolve `drawn = occludes = targetable = true`, and
water resolves `drawn = true`, `occludes = false`, `targetable = true`.

### 3.5 The verdict, and what a load reports

`crates/mc-world/src/persistence/table.rs:56` — `pub struct RegistryVerdict`
with `missing`, `changed`, `retextured`, each `Vec<BlockName>`, each sorted at
`:138-140` where the verdict is reported.

`:149` — `fn judge(...)`: missing first; **else** `behaviour_of != recorded`
→ `changed`; **else** `appearance_of != recorded` → `retextured`. Behaviour is
asked first and answers alone, so once `BEHAVIOUR_REVISION` moves, **every**
resolvable name a pre-bump save holds lands in `changed` and `retextured` is
necessarily empty. That is the arithmetic behind FR-5.2-S1, and it is worth
stating because it is what makes the whole-verdict comparison decisive rather
than merely tidy.

### 3.6 What is public to an integration test

`crates/mc-world/src/persistence/mod.rs:45-52` re-exports
`RequiredBlock`, `SaveRequirements`, `requirements`, `saved_player`,
`stored_world_data`, `LoadedWorld`, `load_world`, `Acceptance`,
`RegistryVerdict`, `resolve`, `DefinitionHash`, `SaveNameId`, `SavedPlayer`,
`save_world`, `write_save`, `replace_atomically`.

`crates/mc-world/src/persistence/read/reader.rs:50-55`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredBlock {
    pub name: BlockName,
    pub behaviour: DefinitionHash,
    pub appearance: DefinitionHash,
}
```

`DefinitionHash` is `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`
(`format.rs:228-229`) with `from_raw` and `get`.

**`behaviour_of` and `appearance_of` are `pub(crate)`** — an integration test
under `crates/mc-world/tests/` cannot call them. It can reach both folds
through `requirements(&save_path)` → `RequiredBlock`, which is the shape
FR-5.1-S1 and S2 need (see §5.1). The sibling unit test
`crates/mc-world/src/persistence/format_test.rs` reaches them by
`use super::{appearance_of, behaviour_of};`.

---

## 4. What the implementation will do — the interface you may write against

This is what T19, T20 and T21 are specified to produce. It is stated here so
your tests can be written against it without having seen an implementation, and
it is binding on me: if I need to depart from it I will say so to you and to
`main` before I do.

### T19 — the two folds

```rust
const BEHAVIOUR_REVISION: u8 = 2;
const APPEARANCE_REVISION: u8 = 3;

struct DeclaredBehaviour<'a> {
    input_version: u8,
    name: &'a str,
    is_solid: bool,
    replaceable: bool,
    breakable: bool,
    breaks_into: Option<&'a str>,
    targetable: bool,            // appended
}

struct DeclaredAppearance<'a> {
    input_version: u8,
    name: &'a str,
    textures: [&'a str; 6],
    drawn: bool,                 // appended
    occludes: bool,              // appended
}
```

**Appended after the existing fields, never inserted among them.** `postcard`
encodes a struct positionally: a rename changes no byte and an insertion in the
middle changes every one. Both structs stay written out by hand and are never
derived from `BlockDefinition`.

**Self-merging is not a field in this spec** (`architecture.md` Decision 10), so
it joins neither fold; the spec's ruling-table row for it is vacated rather than
contradicted.

**Why `drawn` and `occludes` may not go on the behaviour list**: it would tell
every player in existence that every block they built with behaves differently,
on the strength of a rendering field — the exact ambiguity the two bytes exist
to prevent.

### T20 — the verdict a save is opened under

No production signature changes. What changes is the answer: against the
committed pre-spec save and the shipped content,

```
missing:    []
changed:    [base:dirt, base:grass, base:stone, base:water]
retextured: []
```

and the error stream carries one line naming all four.

### T21 — the reload's geometry key

```rust
fn drawn_of(registry: &BlockRegistry) -> BTreeMap<&BlockName, (bool, bool, &FaceTextures)>
// mapping (declared.drawn, declared.occludes, &declared.textures)
```

**`solid` leaves this key, and that is correct even though it looks like a
regression**: solidity changes no geometry once drawnness is its own field, and
keeping it would re-mesh the world for a physics edit. `targetable` never
enters it.

---

## 5. What RED must look like, scenario by scenario

`testing.md` §1: a compile error is acceptable RED only when the scenario is
genuinely about a type existing. Every scenario here is a behaviour scenario, so
**get an assertion failure**. All nine can reach one against the tree as it
stands today, because today's tree is a complete, compiling, *wrong* answer —
you do not need a skeleton, and you must not wait for one.

That is the unusual and pleasant property of this phase: **the pre-implementation
tree is itself the over-eager-and-under-eager pair.** It folds neither new field
(so FR-5.1 and FR-5.2 redden on the old answer) and it keys geometry on solidity
(so FR-7.2-S1 and S2 redden marking nothing, while FR-7.2-S3 is green for the
wrong reason — see §5.5).

### 5.1 FR-5.1-S1 and FR-5.1-S2 — both halves of one record

Each of these asserts **two** things about one block across two content roots:
one fold moved and the other did not. A verdict cannot witness that pair —
`judge` is an `else if`, so a `changed` answer says nothing at all about
appearance. Read the two hashes directly:

- Build root A and root B differing in exactly one field of one block.
- Write a save holding that block against root A; `requirements()` it.
- Write a save holding that block against root B; `requirements()` it.
- Compare the two `RequiredBlock`s **as a pair**, not one hash at a time:
  assert the whole `(behaviour_equal, appearance_equal)` shape, e.g. by
  comparing a two-field tuple or a small enumerated verdict against
  `(different, identical)` for S1 and `(identical, different)` for S2.

Asserting the two halves as one comparison is what makes an implementation that
moved *both* folds fail, and it is exactly the mistake FR-5.1 exists to catch.
Two separate `assert_ne!`/`assert_eq!` calls in one test give the same coverage;
one comparison gives a better failure message. Your call — but do not drop
either half.

**Today's tree reddens both**: neither `targetable` nor `drawn` reaches a fold,
so both roots record identical behaviour *and* identical appearance, and each
assertion's "different" half fails.

### 5.2 FR-5.1-S3 — the byte sequences, built by hand, both directions

**This is the only witness in the workspace that can see either revision byte
move.** Every other witness compares one fold to another and cannot see a
leading byte that moved in both. `docs/technical/world-format.md` records this
as measured, and names the two files that hold the instrument.

The two existing guards live in `crates/mc-world/src/persistence/format_test.rs`
— a sibling unit test, and therefore **yours**:

| Line | Text on it today |
|---|---|
| `:112` | `const STATED_BEHAVIOUR_REVISION: u8 = 1;` |
| `:113` | `const STATED_APPEARANCE_REVISION: u8 = 2;` |
| `:237` | `fn stated_behaviour_bytes(definition: &BlockDefinition) -> Vec<u8> {` |
| `:265` | `fn stated_appearance_bytes(definition: &BlockDefinition) -> Vec<u8> {` |

The file states the FNV constants a second time (`STATED_OFFSET_BASIS`,
`STATED_PRIME`), builds the byte sequence field by field with `push_text`,
`push_flag`, `push_length`, and folds it with its own `folded_here`. **No number
in that file came from a run of the code under test, and nothing in it calls the
fold it is judging.** Keep that property.

After T19 the stated sequences are:

```
behaviour: [2]
           len(name) name
           is_solid  replaceable  breakable
           breaks_into: 0x00, or 0x01 len(residue) residue
           targetable                                   <- appended

appearance: [3]
            len(name) name
            six × ( len(key) key )   in Face::ALL order
            drawn                                       <- appended
            occludes                                    <- appended
```

`FALSE_BYTE = 0x00`, `TRUE_BYTE = 0x01`, `ABSENT_BYTE = 0x00`,
`PRESENT_BYTE = 0x01` are already declared in that file.

**Both directions.** The scenario says *a behaviour fold whose leading byte is
`2` and an appearance fold whose leading byte is `3`*. Two guards, one per fold,
each asserting its own byte — a single guard over one fold cannot see the other
byte fail to move, and a shared constant would move both together, which is the
defect the file's own header says the split exists to report.

**The file's header prose is now false in three places and is yours to rewrite.**
It currently says the behaviour half is *"still the fold it was, over the fields
it was, under revision 1"*, that *"the behaviour constant staying at 1 while the
appearance one is 2 is the whole of what says the bump reached only the list that
grew"*, and (at `:107-111`) that *"the appearance list gained five fields and says
so in its own byte; the behaviour list gained nothing and its byte has not moved"*.
All three become wrong the moment T19 lands. **A test file's header is part of
the test file** — I will not touch it.

**These two guards are green from the moment they are written**, because they
state the same arithmetic the implementation will state. That is a known
property of this file, recorded in its own header, and the answer to it is
mutation rather than a first red run — see §9. Write them anyway, and write them
before the implementation: authored-after guards agree with whatever the change
produced, which is exactly the claim this file exists to *not* make.

### 5.3 FR-5.2-S1 and FR-5.2-S2 — the whole verdict, and the line

**Assert the whole verdict, never an absence, and never "some block is named".**
The committed fixture **already reports `base:water` as behaviour-changed
today**, so a scenario asking only that some block is named stays green against
an implementation that folded no new field at all and never bumped the byte.

The fixture is
`crates/mc-world/tests/fixtures/world_saved_against_the_toml_declarations.mcw`,
read by:

| File | Line | What it holds |
|---|---|---|
| `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | `:70` | `const OLDER_SAVE` |
| `crates/mc-client/tests/changed_blocks_named_on_the_error_stream.rs` | `:74` | `const OLDER_SAVE_FILE` |
| `crates/mc-client/tests/shipped_binary.rs` | `:138` | inside `const OLDER_SAVE` |
| `crates/mc-client/tests/support/launch_notices.rs` | `:142` | inside `const OLDER_SAVE` |

**It is never regenerated.** It was written from `content/base/` while the four
blocks were still TOML, with the shipped reader of the day. The day it is
regenerated it stops being evidence about anything.

The four existing readings that must move, all in
`shipped_declarations_and_an_older_save.rs`:

- `:100` `the_shipped_content_reports_water_as_behaving_differently_and_the_other_three_as_retextured`
  — the whole-verdict comparison. Becomes changed = all four, retextured empty.
  The **name of the test** is falsified along with the expectation.
- `:134` `the_same_save_loads_by_default_and_is_refused_naming_water_alone_when_strictness_is_asked_for`
  — this is FR-5.2-S2. Both arms in one reading: `Acceptance::ChangedBlocksToo`
  answers `None`, `Acceptance::OnlyUnchangedBlocks` answers
  `LoadError::Unresolvable { missing: [], changed: <all four> }`. **The strict
  arm is what carries the evidence** — the file's own doc comment says so, and it
  is right: the accepting arm answers `None` for any changed list at all.
  The **name is falsified** here too.
- `:212` `the_same_comparison_reports_a_block_whose_declaration_the_content_no_longer_holds`
  — the positive control, over a shipped root with `water.luau` removed.
  Becomes missing = `[base:water]`, changed = `[base:dirt, base:grass,
  base:stone]`, retextured = empty. Note the shape flip: the three survivors move
  out of `retextured` into `changed`. **Do not let this control go silent** — it
  is what says the comparison can report anything at all.
- `:189` `the_committed_save_really_does_need_all_four_of_the_blocks_the_base_game_ships`
  — unaffected, and it is the other control. Leave it.

The file's header (`:24-45`) argues at length that *"one shipped block's
behaviour really did move"* and that *"a shared byte reports all four blocks as
changed and none as retextured, which is a different answer from every
expectation here"*. **That argument inverts**: after the bump, all four changed
and none retextured is the *correct* answer, and the thing that would now be
wrong is a byte that failed to move. The header needs rewriting so the next
reader is not led by it — and it needs to keep the reason the two constants stay
separate, which is unchanged.

**"and SHALL open the world naming those blocks on the error stream in one
line"** is the second half of FR-5.2-S1 and it lives in the client:
`crates/mc-client/tests/changed_blocks_named_on_the_error_stream.rs:152`,
`the_committed_pre_luau_save_names_water_and_no_other_block_against_the_shipped_content`,
asserting `const WATER_ALONE` (`:64`).

The composer is `mc_client::notice::changed_blocks`
(`crates/mc-client/src/notice.rs:113`). It has a **singular and a plural
clause** (`:93-94`):

```
NO_LONGER_BEHAVES = "no longer behaves as it did when this world was saved"
NO_LONGER_BEHAVE  = "no longer behave as they did when this world was saved"
```

and ends every line with `", and it was loaded anyway"`. Names are backtick-
quoted and comma-joined. So the four-block line is:

```
mycraft: `base:dirt`, `base:grass`, `base:stone`, `base:water` no longer behave as they did when this world was saved, and it was loaded anyway
```

That exact text matters beyond this test — see §7.3.

`crates/mc-client/tests/shipped_binary.rs` is the wiring instrument for the same
line: it starts the **real binary** as a subprocess and reads its error stream.
`:120` holds `const THE_CLAUSE: &str = "no longer behaves"` — the singular
spelling, used as the fragment the reading waits for — and `:128` holds
`const NAMES_WATER` with the full singular line. Both move. **This is the only
test in the workspace that can see the client fail to *print*** (the composer
tests all reach it through a launch), so its clause fragment must stay a
fragment that the real four-block line actually contains.

### 5.4 FR-5.3-S1 — an appearance change reported to nobody

Two content roots differing in nothing but one block's `occludes`; the verdict
names that block in `retextured` and nothing anywhere else.

**Asserted whole for the mirror reason to FR-5.2**: *"no changed block was
named"* is also exactly what an `occludes` folded into **neither** list would
produce. An absence assertion here is green against the defect.

Today's tree reddens it: `occludes` reaches no fold, so the verdict is empty in
all three lists and the `retextured` half fails.

### 5.5 FR-7.2-S1, S2 and S3 — the wiring, not the policy

`spec.md` states this explicitly: the geometry-change test keys on
`(is_solid, textures)` today, so **a correct `drawn` field that this site never
learns about leaves an edited block looking unchanged until relaunch, with every
other scenario green.** Ask of `drawn_of`: what calls this, and what would go
red if it stopped?

The instrument is `Session::take_remesh_work`, which is what the client's own
frame path drains and what the re-mesh worker is handed. The harness is
`crates/mc-client/tests/support/reload_remesh.rs` and its three children under
`support/reload_remesh/`. What you need is already there:

- `marking.rs` — `pub enum Marking { NoSectionAtAll, EverySectionOfTheShippedWorld { marked }, Sections { marked, distinct, missing, beyond } }`,
  `pub fn marked(client: &mut InputHarness) -> Marking`, and
  `pub const fn every_section_once() -> Marking`.
  **A total verdict**, so an assertion against one arm rejects the arms meaning
  "there was nothing to look at". Use it — do not write `is_empty()`.
- `reload_remesh.rs:185` — `pub fn serial_serving(client: &InputHarness) -> Result<ContentSerial, Box<dyn Error>>`.
- `support/reload_content.rs:135` — `pub fn serial_reported(answered) -> Option<u32>`,
  and `:142-180` a `Run` verdict over a sequence of published serials with an
  arm for "one of the publications reported no serial at all". FR-7.2-S3 asks
  for a serial that **advances**, so a reading that only says "some serial came
  back" is not it.
- `support/reload.rs:142-253` (`impl Declaration` at `:157`) — the `Declaration` builder, which already has
  `.solid()`, `.replaceable()`, `.breakable()`, `.breaking_into()`,
  `.drawn()`, `.occludes()`, `.targetable()`, `.repointing_north()`. A field
  left unstated **writes no line at all** (`stated_fields`, `:286`), which is
  what lets a fixture spell *"says nothing about it"* apart from *"says
  false"* — and §3.3 is why that distinction decides your fixtures.
- `support/reload.rs:369` `shipped_restating_stone`, `:328` `restating`,
  `:353` `declaring`, `:316` `shipped`, `:445` `candidate`, `:477` `adoption`,
  `:518` `accepted`.

**The dirty set is *taken*, so reading it twice reads it once.** Every scenario
in `reload_marks_sections.rs` drains before it reloads
(`require_nothing_outstanding`, `:301`), both as a guard that the launch left
nothing and so that the reading afterwards is the reload's alone. Follow that
shape.

**What today's tree does with each:**

- **S1** (`drawn` alone) — today's key is `(is_solid, textures)`. A candidate
  stating `drawn = false` on a solid block moves neither, so today's answer is
  `Marking::NoSectionAtAll` and the assertion for `every_section_once()` fails.
  Genuine RED.
- **S2** (`occludes` alone) — same. Genuine RED.
- **S3** (`targetable` alone) — today's key ignores `targetable` too, so today's
  answer is already `NoSectionAtAll`. **This scenario is green before the change
  and green after it, and that is not evidence.** Say so in `test-map.md`, and
  understand what it is for: S3 is the negative half of the cross-check, and its
  falsification is the phase's mutation (§9), not a first red run. Do not
  contrive a way to redden it — a test bent until it fails once is worse than an
  honest green with a recorded mutation behind it.

  S3's other half **can** be made to carry weight now: the scenario asks for an
  **accepted** reload whose **published serial advances**. Assert the reload
  outcome, the serial movement and the marking as one comparison, so that a
  reload which was refused, or which published nothing, cannot satisfy it by
  marking nothing. A refused reload marks nothing too.

**Where these belong**: `crates/mc-client/tests/reload_marks_sections.rs`
(295 non-blank lines, limit 600) is the file whose whole subject is *which
sections a reload leaves to be meshed again*. Its two existing discrimination
scenarios — `:96` `a_candidate_touching_neither_solidity_nor_a_texture_key_leaves_no_section_to_mesh`
and `:118` `a_candidate_taking_stones_solidity_away_leaves_every_section_of_the_world_to_mesh`
— are the pair FR-7.2's three extend. Placing yours beside them is what keeps
the discrimination readable: **one of them marks nothing and the other marks the
world**, and neither reading means anything alone.

Its header (`:23-30`) states the rule as *"a candidate changing some block's
declared solidity or declared texture key, or adding or removing a block, marks
every section"*. **That sentence is falsified by T21** — solidity leaves the key.
It stays *incidentally* true of the shipped fixtures because of the derived
default (§3.3), which is precisely the kind of coincidence a header should name
rather than rest on. Yours to rewrite.

---

## 6. Fixture construction — what the shipped caller supplies

`testing.md` §2: *before writing a test, ask what the shipped caller supplies and
which shipped path reaches this.* For this phase the answers are:

- **A block declaration is a Luau chunk on disk**, read by
  `LuauFileDefinitionSource` over a content root containing a `blocks/`
  directory. A `BlockDefinition` constructed in memory skips
  `defaulting_to_solidity` entirely — which means an in-memory fixture can state
  a combination (`solid = true, drawn = false, occludes = true`) that no
  declaration produces by accident, and can also **miss** the defaulting
  behaviour that decides whether an edit is one field or four.

  FR-5.1 and FR-5.3 say *"two content roots"*. Take that literally: build roots
  and read them through the loader. It is the shipped path, and it is the path
  where the derived default lives.

  `crates/mc-world/tests/luau_common/mod.rs` has the pieces —
  `declaration_of(fields)` (`:105`), `text_field` (`:92`), `raw_field` (`:99`),
  `declaring(name)` (`:129`), `registry_from(root)` (`:152`),
  `BLOCKS_DIRECTORY` (`:63`). `crates/mc-world/tests/common/mod.rs:94`
  `content_root(directory, blocks)` writes a root from `(file_name, body)`
  pairs.

  *(Note: `common/mod.rs:109` `block_file(name, texture, solid)` emits
  `name = "…"\ntexture = "…"\nsolid = …` — TOML, from before the move to Luau —
  and a grep of `crates/mc-world/tests/` finds no caller. It is dead. Do not
  build on it. I am recording it as a deferred observation rather than deleting
  it, since it is a test file.)*

- **A save is written by `save_world` and read by `requirements`.** Both are
  public. Writing two saves against two roots and comparing their
  `RequiredBlock`s is a reading at the boundary the save actually crosses.

- **The committed pre-spec fixture is the only genuine "written before" oracle
  in the repository.** Everything else in this phase writes and reads within one
  revision and is self-consistent across a revision move by construction. That
  self-consistency is why the sweep in §7 is short — and it is also why the
  fixture is the only thing that can witness FR-5.2 at all.

- **A reload candidate is a content root handed through the client's own door**
  (`client.adopt(candidate(root)?)`), or through a watch. The scenarios about
  what a report carries attach a watch; the ones handing a candidate directly do
  not need one. `a_client_over` (`reload_remesh.rs:112`) builds a client with **no
  watcher**, which is what FR-7.2's three want.

---

## 7. Five things the survey found that no task text names

Every implementer on this spec has found something in the survey its brief had
missed. These are Phase 5's. Three of them are files that will go **red** when
T19 lands and that nobody has been told to touch.

### 7.1 `save_per_face_appearance.rs` is a third hand-built byte oracle, and it is not in the fallout sweep

`tasks.md` T20 lists the fallout sweep as
`save_acceptance`, `save_changed_blocks`, `save_declarations`, `save_resolution`,
`save_scale`, `shipped_declarations_and_an_older_save`, `mesh_determinism`,
`tests/common/mod.rs`, `mc-sim/tests/publication.rs`,
`mc-client/tests/{shipped_binary, art_and_renderer_failures_are_told_apart}`,
`mc-client/tests/support/{changed_blocks, reload}`.

**`crates/mc-world/tests/save_per_face_appearance.rs` is not on it, and it holds
a stated revision byte:**

| Line | Text on it today |
|---|---|
| `:85` | `const STATED_APPEARANCE_REVISION: u8 = 2;` |
| `:181` | `fn stated_appearance_bytes(name: &str, keys: [&str; 6]) -> Vec<u8> {` |

It builds `[revision, name, six keys]` by hand and folds with its own FNV, over a
fixture whose `BlockDefinition` (`:130-142`) states `drawn: true, occludes: true`.
After T19 it needs `2 → 3` **and** two appended `TRUE_BYTE`s, or it goes red.

This is not a stray find: `docs/technical/world-format.md:653-657` already names
this file and `format_test.rs` as *"exactly the two guards that build the expected
bytes by hand"*, and says **nothing else in the workspace** can see a revision byte
move. That was measured. It means the guards in this file and in `format_test.rs`
are, together, the entire instrument FR-5.1-S3 is about — and one of the two was
about to be walked past.

The file is a test file at 251 non-blank lines. Yours.

### 7.2 `support/launch_notices.rs` reads the committed fixture too, and it is not in the sweep either

`crates/mc-client/tests/support/launch_notices.rs:142` names the same
`world_saved_against_the_toml_declarations.mcw`, and `:157`
`a_launch_over_a_save_whose_block_behaves_differently()` composes a line from a
**real load** of it through `mc_client::notice::changed_blocks`.

It is **self-adjusting** — the fixture decides which save is read and never what
the sentence says — so it needs no edit. But what it feeds does, which is §7.3.

### 7.3 Three lines under `docs/modding/` are test *inputs*, not prose — and the gate reads them

There **is** an instrument for markdown, and it is
`crates/mc-client/tests/documented_refusals.rs`. It walks
`docs/modding/` (`support/quoted_refusals.rs:53-54`,
`pub const PAGES: [&str; 2] = ["docs", "modding"]`), recognises **every fenced
block whose first line begins `mycraft: `**, and compares it **line for line**
against a set of texts produced by real runs. The set is assembled in
`support/printed_refusals.rs:217`, which appends
`crate::launch_notices::launch_notices()?` — the composer output from §7.2. The
verdict is a three-armed enum, so a page that stops quoting anything fails rather
than passing quietly.

`grep -n "^mycraft: " docs/modding/*.md` finds exactly three occurrences of the
changed-blocks line, all identical:

| File | Line |
|---|---|
| `docs/modding/blocks-items.md` | `:724` |
| `docs/modding/hot-reload.md` | `:335` |
| `docs/modding/hot-reload.md` | `:519` |

all reading:

```
mycraft: `base:water` no longer behaves as it did when this world was saved, and it was loaded anyway
```

The producer that emits that line today is the committed-fixture load. After
T19 that load produces the **four-name plural** line, and **no producer emits the
singular water line at all** — so all three pages go red.

**Two of the three are not simply stale prose, and this needs your ruling rather
than mine.** `hot-reload.md:519` sits inside a hands-on loop where the *player*
writes the save with the current build and then edits `water.luau`: under the new
revision that save records revision-2 folds, only water's behaviour moves, and the
**singular water line is the factually correct thing for that page to show**.
`:335` is generic and a singular example is still plausible there. So the honest
end state is a page quoting a line that is true and that nothing in the run set
produces.

There are three ways out and only one of them is mine:

1. **Rewrite all three quotes to the four-name plural line.** Mine to do, needs
   no test change, and falsifies `:519`'s narrative — the page would show a
   four-block line for an edit that moves one block.
2. **Add a producer** for a save written by the current build against the shipped
   content and read against a root restating one block — which emits the singular
   water line honestly. That is a **test support file** (`support/printed_refusals.rs`
   or `support/launch_notices.rs`) and therefore **yours**, not mine. It keeps
   `:519` and `:335` true and lets `blocks-items.md:724` become the plural line,
   which is where the "every save in existence" claim actually lives.
3. Leave it red. Not an option.

**I recommend 2**, and I have asked `main` to rule. If the ruling is 2, the
producer is a test-file change and I will need it from you; if it is 1, I will do
it in T22 and nothing lands in your files. **Do not act on this section until
`main` has ruled** — I will relay the ruling to you.

### 7.4 A fourth doc site carries the same line and *nothing* checks it

`docs/user/gameplay.md:126` holds the identical singular line, and
`docs/user/` is outside the scanned root. `grep -rn "no longer behave" docs/`
finds exactly four doc occurrences: the three above plus this one. Nothing in
the gate will ever report it.

That is mine (T22), and I record it here so the count is on paper: **four doc
sites, three of them gate-visible, one of them not.** The scope of that grep is
`docs/` for the phrase `no longer behave`, on this tree.

`docs/user/gameplay.md:90-92` also carries the sentence `tasks.md` T22 quotes as
*"the third falsified player sentence"* — verified verbatim on this tree at
`:90-92` rather than at the `:68-70` the task states, because earlier phases
edited the page above it:

> What the unbreakable declaration also changes is your save: water is the one
> shipped block whose recorded behaviour moved, which is why a world saved before
> this build reports it by name on the terminal (see below).

And `docs/modding/blocks-items.md:718` carries the conductor's amendment target,
verified verbatim, matched by words rather than by line number:

> **`breakable` is one of the five fields a save folds into a block's
> recorded behaviour**, alongside `name`, `solid`, `replaceable` and `breaks_into`.

Six once `targetable` joins. All of §7.4 is mine.

### 7.5 The existing solidity-remesh scenarios survive T21 by a coincidence worth naming

`reload_marks_sections.rs:117`
`a_candidate_taking_stones_solidity_away_leaves_every_section_of_the_world_to_mesh`
drives `stone_that_is_not_solid()` = `Declaration::of(STONE).solid(false)`
(`support/reload.rs:380`). Under T21's key that candidate still marks the world —
**not because solidity is in the key, but because the declaration states nothing
about `drawn` or `occludes`, so both default to `false` along with it** (§3.3).

Likewise `shipped_restating_every_declaration` (`reload_marks_sections.rs:319`)
restates water as `Declaration::of(WATER).solid(true)` with nothing else, which
takes `occludes` from the shipped `false` to a defaulted `true` — a geometry
change under the new key for a different reason than under the old one.

Nothing to fix. It is worth writing down because it is exactly the shape a
reviewer would otherwise flag as "this test should have reddened and did not",
and because a future author who states `drawn` explicitly beside `solid` in one
of those fixtures would silently change what the scenario is about.

`grep -rn "\.drawn(\|\.occludes(\|\.targetable(" crates/mc-client/tests/ crates/mc-sim/tests/` on
this tree finds only three fixture sites outside the builder itself —
`support/reload_trap.rs:122` (`.solid(false).targetable(false)`, a player-trapping
fixture, no remesh assertion), `the_built_set_fills_its_layers.rs:270` and
`the_judge_marches_through_a_block_nothing_draws.rs:95` (both
`.solid(true).drawn(false)`, neither about remeshing). None of the three asserts
a `Marking`.

---

## 8. Boundaries

### Yours (I will not edit any of these)

- `crates/mc-world/src/persistence/format_test.rs` — **including its header**.
- `crates/mc-world/tests/**` — every file, `common/` and `luau_common/` included.
- `crates/mc-client/tests/**` — every file, `support/` included.
- `crates/mc-sim/tests/**`.
- Any new test file.

A test file's **header is a test file too**. Where §5 and §7 say a header's
prose is falsified, that is a report to you, not an edit I will make.

### Mine

- `crates/mc-world/src/persistence/format.rs` (T19) — including its doc
  comments, three of which state the current revision numbers and the claim that
  *"the behaviour list gained nothing and its byte has not moved since the format
  was written"* (`:266-268`), plus `behaviour_of`'s *"What revision 1 …"* (`:337`)
  and `appearance_of`'s *"What revision 2 …"* (`:350`).
- `crates/mc-sim/src/world/reload.rs` (T21) — `drawn_of`, `changes_geometry`
  and the doc comments that describe the key as *"solidity, the keys the six
  faces draw from, or the set of names"* (`:65-68`).
- `docs/technical/world-format.md`, `docs/user/gameplay.md`,
  `docs/modding/blocks-items.md`, `docs/modding/hot-reload.md` (T22) — subject
  to §7.3's ruling.
- `docs/technical/decisions.md` — if an amendment is owed. Read on this tree:
  `:1698` rejects *"One shared revision byte over both hash lists"*, and that
  rejection **stays true** and is not amended. **An ADR records what was decided
  *then*** — amended, never rewritten.

### Neither of ours without a ruling

- `content/base/blocks/*.luau` — the shipped declarations. Nothing in Phase 5
  changes them.
- The committed save fixture. **Never regenerated.**

---

## 9. The mutation this phase owes

`tasks.md` Notes, verbatim:

> Phase 5 — put `targetable` into `drawn_of`'s key; FR-7.2-S3 must redden alone.

That mutation is the falsification for FR-7.2-S3, which is green before the
change and green after it (§5.5). **"Redden alone" is the load-bearing half**,
and it is an observation only if the whole suite ran: a count is only a count
with `--no-fail-fast`. `cargo nextest`'s tell is the slash — `N/M tests run` is
a **cancelled** run and says nothing about the rest; a bare `N tests run` is
complete. The workspace count on `477137d` was **1428 passed, 1 skipped**, and
it is a free tree-state discriminator.

`format_test.rs`'s two guards are green from the moment they are written, so
they owe a mutation too — the file's header records that two of an earlier
phase's mutations reddened both (changing the offset basis by one; returning the
basis without folding). For this phase the sharper one is **leave one revision
byte behind while its list grows**: set `BEHAVIOUR_REVISION` back to 1 with
`targetable` still folded, and exactly the behaviour guard should redden while
the appearance guard stays green. That is the claim the two separate constants
exist to support, and it is measurable.

I run the mutations, not you. **I will announce every mutation window to you and
to `main` before I break anything**, with the baseline failing-test count and the
file — two agents mutating one tree produces exactly the signature of a flaky
test, and the count is what tells them apart afterwards. I revert by hand,
never with `git checkout --`, and confirm with `git diff --exit-code`.

---

## 10. `test-map.md`

`specs/active/2026-08-22-solid-split-properties/test-map.md` ends at Phase 4
(`## Phase 4 …` at `:1444`). Phase 5 gets its own `## Phase 5 — …` section at
the end, following the four earlier sections' shape:

- One row per scenario: scenario ID → test name → file.
- An **Additional coverage** heading for every test that is not a scenario's
  own, one line each stating what it catches. A test whose purpose is not
  written down is one nobody can later judge.
- The RED output, **with the invocation beside every count** — including counts
  that were sound for some other reason, because provenance recorded unevenly
  reads as provenance absent.
- Which tree each reading was taken on. Prefer a reading that dates itself: a
  run naming a test as FAILED cannot have come from the tree where it passes.
- The scenarios that are **green before the change**, named as such, with what
  falsifies each. FR-7.2-S3 and both FR-5.1-S3 guards are in this class and each
  of them is load-bearing — an unrecorded green is how a bogus test survives.

Test names stay behavioural. **No spec or scenario ID in a test name or in
code** — scenario IDs live in `tasks.md`, `test-map.md` and commit messages on
the branch, nowhere else.

---

## 11. Sizes, measured on this tree

`scripts/sdd-gate.ps1:128-129` — `$MaxSourceLines = 500`, `$MaxTestLines = 600`.
A file counts as a test when its path contains a `tests/` or `benches/` segment
or its name ends `_test.rs`. The count is
`Get-Content | Measure-Object -Line`, which does **not** count blank lines, so
these are non-blank counts (`grep -c '[^[:space:]]'`):

| File | Non-blank | Limit | Headroom |
|---|---|---|---|
| `crates/mc-client/tests/shipped_binary.rs` | 578 | 600 | **22** |
| `crates/mc-client/tests/support/reload.rs` | 491 | 600 | 109 |
| `crates/mc-client/tests/support/printed_refusals.rs` | 483 | 600 | 117 |
| `crates/mc-world/tests/shipped_declarations_and_an_older_save.rs` | 365 | 600 | 235 |
| `crates/mc-client/tests/reload_marks_sections.rs` | 295 | 600 | 305 |
| `crates/mc-world/src/persistence/format_test.rs` | 287 | 600 | 313 |
| `crates/mc-world/tests/common/mod.rs` | 281 | 600 | 319 |
| `crates/mc-world/tests/save_per_face_appearance.rs` | 251 | 600 | 349 |
| `crates/mc-client/tests/support/launch_notices.rs` | 229 | 600 | 371 |
| `crates/mc-client/tests/changed_blocks_named_on_the_error_stream.rs` | 196 | 600 | 404 |
| `crates/mc-world/src/persistence/format.rs` | 359 | **500** | 141 |
| `crates/mc-sim/src/world/reload.rs` | 95 | **500** | 405 |

**`shipped_binary.rs` has 22 non-blank lines of headroom.** Its two constants
move in place, so it should not grow — but if §7.3's ruling sends a new producer
anywhere near it, that is the file to keep away from. `printed_refusals.rs`
already carries a note (`:199`) saying it is *"within fifty non-blank lines of
the size the gate allows a test file"*, which was true of an earlier state and is
now 117 — a figure I am flagging as stale rather than correcting, since it is in
a test file and correcting it is yours if you think it worth doing.

---

## 12. The instruments, and what each can and cannot see

- **A green suite is no evidence about a lint.** Run
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  directly when you land tests. Anything less than `-D warnings` asks a different
  question, and without it cargo attributes a diagnostic to the first binary and
  marks the rest `(1 duplicate)` — meaning *this same diagnostic, repeated*, not
  *a pre-existing one lives elsewhere*.
- **`cargo clippy` does not resolve intra-doc links; only rustdoc does.** The
  gate's docs stage is
  `RUSTDOCFLAGS='-D warnings -D rustdoc::broken_intra_doc_links'` then
  `cargo doc --workspace --no-deps --quiet` (`sdd-gate.ps1:237-239`). Run the
  instrument that failed, not a proxy for it.
- **`scripts/sdd-gate.ps1` is `main`'s to run, not ours.** While a gate runs the
  tree is frozen. `main` says when one starts and ends. Scoped `cargo` runs are
  fine.
- **Nothing in the gate reads `docs/*.md`** beyond the `docs/modding/`
  quoted-refusal scan in §7.3 and the file-size stage's `.rs` walk. A stale doc
  outside `docs/modding/` is permanent until somebody greps.

---

## 13. Process

- **Tests first.** I write no implementation until your tests exist and are red,
  and I will display the failing output before anything of mine lands.
- **I never edit a test file.** A disputed failure comes to you with a verdict
  request and exactly three answers: `test-correct` (the implementation
  conforms and I fix it), `test-wrong` (you fix and commit), or
  `scenario-ambiguous` (the user decides).
- **Commits**: `test: add failing tests for …` is yours, `feat: implement …` is
  mine, never mixed. Explicit paths only — `git add -A` and `git add .` are
  banned with no "the tree is clean, I checked" exception. Read
  `git diff --cached --stat` immediately before every commit and
  `git branch --show-current` before every commit. Push at every task boundary.
- **Nothing either of us writes may cite a location inside `specs/active/`** —
  that folder is archived at completion, so a `docs/` page or a code comment
  pointing into it becomes a dangling reference.
- **A test red for a known reason is fixed before the phase closes, never
  annotated.** Red for a known reason hides red for an unknown one.
- **Do not loosen a threshold, budget or bound to reach green.** If a reason has
  stopped discriminating, say so rather than repairing it into a
  decisive-sounding one.
- `git status` can report a file modified whose content is byte-identical to
  `HEAD` — a stale stat cache. Three readings of content beat one of stat; never
  "clean" it.

---

## 14. Open, and blocking on `main`

**§7.3** — the three `docs/modding/` quoted lines. Whether a producer is added
(your files) or all three quotes become the four-name plural line (my file). I
have put the question to `main` and will relay the ruling. Everything else in
this brief is settled and you can start on it now: FR-5.1-S3's two byte guards,
FR-5.1-S1/S2, FR-5.2-S1/S2, FR-5.3-S1 and FR-7.2-S1/S2/S3 are all unaffected by
that ruling.
