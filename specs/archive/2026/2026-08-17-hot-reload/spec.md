---
id: SPEC-017
title: Hot reload — edit a block definition on a running server and see it change
status: implemented
rigor: high
branch: feature/PRO-918-hot-reload
issue: PRO-918
created: 2026-08-17
updated: 2026-08-18
completed: 2026-08-18
author: spec-PRO-918
---

# Specification: Hot reload — edit a block definition on a running server and see it change

## Goal

A mod author edits a block declaration in a text editor while the game is
running, saves, and the change reaches the world without a restart and without
losing the world, the player or the save. A declaration they got wrong is
refused by name with the previous content still serving, so the fix is another
save rather than another launch.

## The capability this delivers

**Stakeholder: the mod author** — who, in singleplayer, is also the player at
the keyboard.

**What they can now do:** leave `cargo run -p mc-client` running, open
`content/base/blocks/stone.luau`, change `solid = true` to `solid = false`,
save, and walk through stone. The hole they dug a moment ago is still there,
they are still standing where they were, and the tick never stopped. Add
`content/base/blocks/amber.luau` and the block they just invented arrives in
their hand, drawn from a texture layer appended beside the four that were
already there, and they can go and place it. Get an edit wrong — a misspelled
field, a chunk that will not compile, two files claiming one name — and the game
keeps running while the terminal names the file, the block and the field. Fix
it, save again, and the next attempt lands.

**Player:** two things are different when they play. The world can now change
under them while they are in it, because content is read again whenever it
changes on disk rather than only at launch. And if something they are standing
in becomes solid, they are moved to the nearest clear space instead of being
trapped in it. Nothing else about playing changes: their world, their position
and their save survive a reload untouched.

**Engine reader:** the registry stops being a value held for the run and becomes
a value published and replaced — the first `ArcSwap` of a registry replacement
in this project, swapped between ticks, built off the tick thread in a host of
its own, and all-or-nothing so that a refused candidate changes nothing at all.
Beside it, the layer policy that makes this affordable: **appended, never
renumbered, within a session**, which is what keeps every vertex already on the
GPU valid across a reload.

## What this spec inherits, and the one thing it owns that nothing else can

SPEC-016 made the texture layer assignment **stated where content is read and
honoured where it is drawn**, rather than derived from a sort. That is the
enabling half and it is already in the tree.

**This spec owns the policy.** A layer index rides inside every packed vertex.
Under derivation, adding one block renumbers every index after it and the whole
world is textured wrong — silently, with no error anywhere, and not localised to
the block that caused it — which is why a reload would otherwise have to re-mesh
and re-upload the entire world before it could draw a single frame. Under an
appended assignment, every vertex already on the GPU stays valid and only the
sections whose blocks actually changed need meshing again. FR-5 is that policy;
FR-6 is the bound it buys.

## Declared quantities

Three numbers this spec introduces. Each is **declared in exactly one place** in
the tree, for the reason `LOAD_CHANGED_BLOCKS` already is: a value spelled twice
is two places for one decision to disagree with itself.

| Quantity | Value | Why this value |
|---|---|---|
| **Settling window** | 150 ms | Long enough to absorb one editor save, which is commonly a write-then-rename or several partial writes; short enough to leave the rest of the one-second target to the engine, which is the whole point of pinning it. |
| **Clearing search bound** | 8 blocks | The bound is a cost ceiling, not a terrain measurement: at worst 17³ positions tested against a bitset, which is a bounded constant, and far below anything that would walk the world. Whether it is *enough* for any particular world is exactly what FR-7.1-S5 is about. |
| **Array-texture layers** | 256 | Not introduced here — it is the packed vertex's 8-bit layer field (`crates/mc-render/src/geometry/vertex.rs`), and the array is already allocated to exactly that depth. This spec is what makes it a session budget rather than a launch-time ceiling. |

Two words are used precisely throughout, because the tree already spends the
obvious one:

- A **candidate** is a content set built from the root and not yet accepted.
- A **content serial** is the number an accepted content set is published under.
  Deliberately not "revision", which already names the golden-capture revision
  in `crates/mc-render/src/capture.rs`.

## User Stories

- As a **mod author**, I want a declaration I edit and save to reach the running
  game, so that getting a block right is a save rather than a relaunch.
- As a **mod author**, I want a declaration I got wrong to be refused by name
  with the game still running, so that a typo costs me one save instead of a
  restart and a walk back to where I was.
- As a **mod author**, I want a block I have just declared to be something I can
  go and place, so that adding content is a thing I can see rather than a thing
  I can only read about.
- As a **player**, I want a reload to leave my world, my position and my save
  exactly as they were, so that somebody editing content is not somebody
  resetting my game.
- As a **player**, I want to be moved clear rather than trapped when something I
  am standing in becomes solid, so that a content edit cannot wedge me.

## Functional Requirements

Scenario rules: `standards/global/scenario-guidelines.md`. Each scenario becomes
at least one test, mapped in this folder's `test-map.md`.

Two conventions, so that scenarios can be short without being abstract:

- **The shipped world** means the world the client launches into: four columns
  square, sixteen sections per column, **256 sections in all**. Where a scenario
  needs a world that is not that one, it says so.
- **The shipped content** means `content/base/`'s four declarations, read in
  file-name order — `dirt.luau`, `grass.luau`, `stone.luau`, `water.luau` — each
  declaring `texture` equal to `name`, which under the assignment in force gives
  `base:dirt` layer 0, `base:grass` 1, `base:stone` 2 and `base:water` 3, and
  makes `base:dirt` the first solid block in registration order.

### FR-1 — A saved edit becomes one reload attempt

- **FR-1.1**: A change under the content root is noticed, a burst of changes
  becomes one attempt, and a change to something that is not content becomes
  none.

  - FR-1.1-S1: WHEN `content/base/blocks/stone.luau` is written while a
    simulation is running THE SYSTEM SHALL begin exactly one reload attempt.
  - FR-1.1-S2: WHEN a declaration file that did not exist appears under
    `<root>/blocks/` THE SYSTEM SHALL begin a reload attempt.
  - FR-1.1-S3: WHEN a declaration file is deleted from `<root>/blocks/` THE
    SYSTEM SHALL begin a reload attempt.
  - FR-1.1-S4: WHEN five writes to `blocks/stone.luau` arrive inside the
    declared 150 ms settling window THE SYSTEM SHALL begin exactly one reload
    attempt rather than five.
  - FR-1.1-S5: WHEN a HUD declaration under `<root>/hud/` is written THE SYSTEM
    SHALL begin a reload attempt, a content root being one root.
  - FR-1.1-S6: WHILE nothing under the content root has changed THE SYSTEM SHALL
    begin no reload attempt, however many ticks are advanced.
  - FR-1.1-S7: WHEN `blocks/stone.luau.swp` is written under the content root
    THE SYSTEM SHALL begin no reload attempt, an editor's scratch file not being
    a file the loader would read.
  - FR-1.1-S8: IF the content root cannot be watched — it is not there when
    watching begins — THEN THE SYSTEM SHALL report that naming the directory and
    go on running the simulation on the content it already loaded.
  - FR-1.1-S9: WHEN `content/base/materials/dirt.toml` is written THE SYSTEM
    SHALL begin no reload attempt, a materials file being one the loader does
    not read — **while the same instrument begins one for a write to
    `blocks/stone.luau`**, so that a watcher which never fires, or one whose
    relevance rule has come to refuse everything, cannot satisfy this.

- **FR-1.2**: A candidate is built from the whole content root, off the tick
  thread, in a scripting host of its own.

  - FR-1.2-S1: WHEN a reload attempt begins after `blocks/stone.luau` alone
    changed THE SYSTEM SHALL produce a candidate registering `base:dirt`,
    `base:grass`, `base:stone` and `base:water`, not `base:stone` alone.
  - FR-1.2-S2: WHILE a candidate is being built THE SYSTEM SHALL advance the
    player over those ticks to the position the same ticks would have produced
    with no attempt in flight.
  - FR-1.2-S3: IF a declaration loops without returning THEN THE SYSTEM SHALL
    abort it at the shipped call-and-loop budget of 1 000 000, refuse the
    candidate naming the declaration file, and leave the simulation advancing.
  - FR-1.2-S4: WHEN a change is reported while a candidate is already being
    built THE SYSTEM SHALL begin exactly one further attempt once the one in
    flight has ended, rather than a second concurrent build or none at all.
  - FR-1.2-S5: IF the thread building a candidate ends without producing one
    THEN THE SYSTEM SHALL keep the previous content serving, report it once, and
    go on beginning attempts for later changes.

- **FR-1.3**: An accepted candidate takes effect between two ticks, never during
  one.

  - FR-1.3-S1: WHEN a candidate declaring `base:stone` unbreakable is accepted
    between two ticks THE SYSTEM SHALL let a break of stone submitted on the
    earlier tick succeed and refuse one submitted on the later tick as
    indestructible.
  - FR-1.3-S2: WHILE a candidate is being built THE SYSTEM SHALL answer every
    tick from the content that was in force when the build began.
  - FR-1.3-S3: WHEN a change is reported before any tick has been advanced THE
    SYSTEM SHALL hold the attempt until a tick boundary exists and swap there,
    never into a world no tick has yet run.

### FR-2 — A failed reload changes nothing

- **FR-2.1**: A candidate refused for any reason leaves the previous content
  serving, and the refusal locates itself the same way a launch refusal does.

  - FR-2.1-S1: IF a declaration chunk will not compile THEN THE SYSTEM SHALL
    keep the previous content serving and report a refusal naming the
    declaration file and the line the compiler named.
  - FR-2.1-S2: IF `blocks/stone.luau` states `slid = true` where it meant
    `solid` THEN THE SYSTEM SHALL keep the previous content serving and report a
    refusal naming the file, the block `base:stone`, and the field `slid`.
  - FR-2.1-S3: IF two declaration files claim the name `base:stone` THEN THE
    SYSTEM SHALL keep the previous content serving and report a refusal naming
    both files in file-name order.
  - FR-2.1-S4: IF the `blocks/` directory has been emptied of declarations THEN
    THE SYSTEM SHALL keep the previous content serving and report a refusal
    naming that directory.
  - FR-2.1-S5: WHEN a candidate is refused THE SYSTEM SHALL leave the four
    registered blocks, the layer assignment and the block in the player's hand
    exactly as they were.
  - FR-2.1-S6: WHEN a candidate is refused and the file is then corrected and
    saved THE SYSTEM SHALL accept the next candidate, a refusal ending one
    attempt rather than the watching.
  - FR-2.1-S7: IF a HUD declaration under the same root is refused THEN THE
    SYSTEM SHALL refuse the whole candidate, the block declarations included.
  - FR-2.1-S8: WHEN five successive attempts meet the same refusal THE SYSTEM
    SHALL report it once rather than five times, and report the next refusal
    that differs from it.

- **FR-2.2**: A candidate that does not declare a block the world holds is
  refused.

  - FR-2.2-S1: IF the world holds `base:stone` and the candidate declares no
    `base:stone` THEN THE SYSTEM SHALL refuse the candidate naming `base:stone`
    and keep the previous content serving.
  - FR-2.2-S2: WHEN a world in which no cell holds `base:water` is running and a
    candidate drops the `base:water` declaration THE SYSTEM SHALL accept the
    candidate.
  - FR-2.2-S3: IF a candidate declares neither `base:grass` nor `base:stone` and
    the world holds both THEN THE SYSTEM SHALL name `base:grass` before
    `base:stone` in the refusal, rather than whichever was found first.

- **FR-2.3**: A declaration that misbehaves is contained by the limits the
  scripting host already ships.

  - FR-2.3-S1: IF a declaration allocates past the per-entry memory cap of
    256 KiB THEN THE SYSTEM SHALL refuse the candidate naming the declaration
    file, with the process still running and the previous content still serving.
  - FR-2.3-S2: IF a declaration raises an error of its own THEN THE SYSTEM SHALL
    refuse the candidate naming the file and the message that was raised.

### FR-3 — A reload loses nothing the player has

**Throughout FR-3 the accepted candidate declares `base:stone` non-solid**,
whose effect is separately observable in the same test — so a reload that
changed nothing, or that was never applied, cannot satisfy these scenarios by
leaving everything alone.

- **FR-3.1**: The world the player has edited survives a reload.

  - FR-3.1-S1: WHEN a player has broken one block and placed another and a
    candidate is then accepted THE SYSTEM SHALL leave the broken cell empty and
    the placed cell holding what was placed.
  - FR-3.1-S2: WHEN a candidate is accepted THE SYSTEM SHALL leave every cell of
    the shipped world holding what it held, cell for cell.
  - FR-3.1-S3: WHEN the candidate declaring `base:stone` non-solid is accepted
    THE SYSTEM SHALL leave every cell holding what it held **while** stone has
    stopped stopping the player, so that a reload which changed nothing cannot
    satisfy FR-3.1-S2.

- **FR-3.2**: The player survives a reload.

  - FR-3.2-S1: WHEN a candidate is accepted and no cell the player's box
    overlaps became solid THE SYSTEM SHALL leave the player at the position and
    orientation the same ticks would have produced with no attempt in flight.
  - FR-3.2-S2: WHEN a candidate is accepted between two ticks THE SYSTEM SHALL
    publish the later tick as exactly the earlier one plus one, with the change
    the candidate declared in force on that later tick.
  - FR-3.2-S3: WHEN a candidate is accepted while the player is falling THE
    SYSTEM SHALL leave the player's velocity what the same tick would have
    produced with no attempt in flight, a reload being neither a landing nor a
    launch.

- **FR-3.3**: The block in the player's hand is re-derived from the content now
  serving.

  - FR-3.3-S1: WHEN a candidate is accepted and the block the player holds is
    still the first solid block in registration order THE SYSTEM SHALL leave
    them holding it.
  - FR-3.3-S2: WHEN a candidate adds `blocks/amber.luau` declaring a solid
    `base:amber`, whose file sorts before `dirt.luau`, THE SYSTEM SHALL put
    `base:amber` in the player's hand in place of `base:dirt`.
  - FR-3.3-S3: IF a candidate declares no solid block at all THEN THE SYSTEM
    SHALL refuse it, saying that a player would have nothing to place, and keep
    the previous content serving.
  - FR-3.3-S4: WHEN a candidate adds `blocks/amber.luau` declaring a solid
    `base:amber` and the player then places what is in their hand THE SYSTEM
    SHALL leave `base:amber` in the cell it was placed in.

- **FR-3.4**: A save written after a reload records what the blocks are now.

  - FR-3.4-S1: WHEN a player quits after a candidate changed `base:stone`'s
    declared solidity THE SYSTEM SHALL write a save that a relaunch against that
    same content root resumes without asking the player to accept changed
    blocks.
  - FR-3.4-S2: WHEN a save written *before* that candidate was accepted is
    relaunched against the changed content root THE SYSTEM SHALL report
    `base:stone` as changed, so that FR-3.4-S1's silent resume is evidence the
    save records what the blocks are now rather than evidence that nothing is
    ever compared.

### FR-4 — What a reload changes is what the author edited

- **FR-4.1**: A change of declared solidity reaches both the physics and the
  picture, through one answer rather than two.

  - FR-4.1-S1: WHEN a candidate declaring `base:stone` non-solid is accepted THE
    SYSTEM SHALL stop stone stopping the player.
  - FR-4.1-S2: WHEN a candidate declaring `base:water` solid is accepted THE
    SYSTEM SHALL make water stop the player.
  - FR-4.1-S3: WHEN a candidate declaring `base:stone` non-solid is accepted THE
    SYSTEM SHALL draw the faces its neighbours' solidity now implies, a face
    culled against solid stone appearing once stone is not solid.
  - FR-4.1-S4: WHEN a candidate declaring `base:stone` non-solid is accepted THE
    SYSTEM SHALL report a cell holding stone as not solid through both the view
    the physics reads and the view a placement's occupancy check reads, the two
    stating one answer.
  - FR-4.1-S5: IF a candidate declaring `base:stone` non-solid is refused for
    any other reason THEN THE SYSTEM SHALL leave stone stopping the player, the
    physics view being no more half-applied than the registry.

- **FR-4.2**: A change to a mutation rule reaches the next edit the player
  makes, and a residue is still resolved when a break happens.

  - FR-4.2-S1: WHEN a candidate declaring `base:stone` unbreakable is accepted
    THE SYSTEM SHALL refuse a break of stone as indestructible.
  - FR-4.2-S2: WHEN a candidate giving `base:stone` a `breaks_into` of
    `base:dirt` is accepted THE SYSTEM SHALL leave `base:dirt` in the cell where
    a stone was broken.
  - FR-4.2-S3: WHEN a candidate declaring `base:stone` replaceable is accepted
    THE SYSTEM SHALL let a placement overwrite a stone.
  - FR-4.2-S4: WHEN a candidate gives `base:stone` a `breaks_into` of
    `base:mithril`, which no declaration declares, THE SYSTEM SHALL accept the
    candidate — a residue is resolved when a break happens and not when a
    declaration is read, which is what lets two mods name each other's blocks.

- **FR-4.3**: A block declared for the first time is registered, drawable and
  visible in the hand that holds it.

  - FR-4.3-S1: WHEN a candidate adds `blocks/amber.luau` declaring `base:amber`
    THE SYSTEM SHALL register `base:amber` and answer for its declared fields by
    name.
  - FR-4.3-S2: WHEN a candidate adds `blocks/amber.luau` declaring `base:amber`
    with `texture = "base:amber"`, a key no registered block named, THE SYSTEM
    SHALL append a layer for that key, fill it, and draw a placement of
    `base:amber` from that layer rather than from another block's.
  - FR-4.3-S3: WHEN `base:amber` reaches the player's hand THE SYSTEM SHALL draw
    the held-block indicator from the layer `base:amber` was appended to.

- **FR-4.4**: Nothing the author did not edit moves.

  - FR-4.4-S1: WHEN a candidate changes only `blocks/stone.luau` THE SYSTEM
    SHALL leave `base:dirt`, `base:grass` and `base:water`'s declared fields
    exactly as they were.
  - FR-4.4-S2: WHEN a candidate whose four declarations are byte-identical to
    the content serving is accepted THE SYSTEM SHALL leave the layer assignment
    identical — `base:dirt` 0, `base:grass` 1, `base:stone` 2, `base:water` 3 —
    and publish it under a content serial later than the one before it, so that
    a skipped attempt cannot satisfy this.

- **FR-4.5**: The HUD the same root declares is applied with the blocks, because
  it is refused with them.

  - FR-4.5-S1: WHEN a candidate widening `hud/crosshair-horizontal.toml`'s
    `size` from `[9, 1]` to `[21, 1]` is accepted THE SYSTEM SHALL publish that
    element at its declared `[21, 1]` and compose the frame from it, applying
    the blocks and the HUD together — a candidate that refuses them together
    (FR-2.1-S7) and applies only one being the partial application that is a
    Blocker.

### FR-5 — Layers are appended within a session and never renumbered

- **FR-5.1**: A layer a key holds is a layer it keeps for the session, and only
  an accepted candidate spends the budget.

  - FR-5.1-S1: WHEN a candidate introduces the texture key `base:amber`, which
    sorts before `base:dirt`, THE SYSTEM SHALL give `base:amber` layer 4 and
    leave `base:dirt` on 0, `base:grass` on 1, `base:stone` on 2 and
    `base:water` on 3.
  - FR-5.1-S2: WHEN a candidate removes the only block naming a texture key,
    that key's block being placed in no cell, THE SYSTEM SHALL leave every
    remaining key on the layer it already held.
  - FR-5.1-S3: WHEN a candidate removes a texture key and a later candidate
    introduces the same key again THE SYSTEM SHALL give it the next unused layer
    rather than the layer it held before.
  - FR-5.1-S4: WHEN a section that was not meshed again after a candidate
    appended layer 4 is drawn THE SYSTEM SHALL pack its quads with the layer
    indices it carried before the reload — `base:stone` still 2 — the vertices
    already uploaded staying valid.
  - FR-5.1-S5: IF a candidate that would need a new layer is refused THEN THE
    SYSTEM SHALL append no layer, so that the next accepted candidate
    introducing `base:amber` gives it layer 4 and not layer 5.

- **FR-5.2**: The array texture's 256 layers are the session's budget, and
  running out is a refusal rather than a wrong picture.

  - FR-5.2-S1: IF a candidate would need a layer past the 256 the array texture
    holds THEN THE SYSTEM SHALL refuse it, naming the layers it would need, the
    256 available, and that relaunching reclaims every layer retired since the
    client started.
  - FR-5.2-S2: WHEN a candidate needs exactly the 256th layer THE SYSTEM SHALL
    accept it.
  - FR-5.2-S3: IF 255 layers are assigned and a candidate introduces two texture
    keys THEN THE SYSTEM SHALL refuse it and leave 255 assigned, rather than
    appending the one that fits and refusing the other.

### FR-6 — The re-mesh a reload causes is bounded, and never on the tick thread

- **FR-6.1**: Only a reload that changed what is drawn meshes anything again.

  - FR-6.1-S1: WHEN a candidate changes no block's declared solidity and no
    block's declared texture key THE SYSTEM SHALL mesh no section again.
  - FR-6.1-S2: WHEN a candidate declares `base:stone` non-solid THE SYSTEM SHALL
    mesh again every section whose own or whose neighbours' blocks include
    stone, while going on advancing ticks and drawing frames.
  - FR-6.1-S3: WHEN a candidate changes every one of the four declarations THE
    SYSTEM SHALL mesh exactly the shipped world's 256 sections, each once, for
    that one reload.
  - FR-6.1-S4: WHEN a candidate changing only `base:stone`'s `breakable` and a
    candidate changing `base:stone`'s declared solidity are each accepted in one
    session THE SYSTEM SHALL mesh no section for the first and some section for
    the second, measured on one instrument, so that an implementation which
    meshes nothing on any reload cannot satisfy FR-6.1-S1.

- **FR-6.2**: A section is meshed against the content now serving, and never
  against the content it replaced.

  - FR-6.2-S1: WHEN a reload lands while a batch meshed against the previous
    content is still in flight THE SYSTEM SHALL draw from a scene meshed against
    the content now serving and never from that batch's scene.
  - FR-6.2-S2: WHEN a reload's re-mesh runs THE SYSTEM SHALL resolve every
    section's blocks against the content now serving.
  - FR-6.2-S3: IF a reload's re-mesh cannot be completed THEN THE SYSTEM SHALL
    report it once and go on drawing the picture it already had, the run
    continuing.
  - FR-6.2-S4: WHEN a batch meshed against superseded content is discarded THE
    SYSTEM SHALL leave every section that batch carried marked for meshing
    again, so that no section stays stale for the rest of the run.
  - FR-6.2-S5: WHEN a player breaks a block after a candidate was accepted THE
    SYSTEM SHALL mesh that edit's sections against the content now serving.

### FR-7 — A player inside a cell that became solid is moved clear

- **FR-7.1**: The move is to the nearest clear position, sideways before upward,
  never downward, and bounded.

  - FR-7.1-S1: WHEN a reload makes solid the cell holding a player whose
    0.6 × 1.8 × 0.6 box overlaps it THE SYSTEM SHALL move the player to a
    position whose box overlaps nothing solid.
  - FR-7.1-S2: WHEN a clear position one block sideways and a clear position one
    block upward are both available THE SYSTEM SHALL move the player sideways.
  - FR-7.1-S3: WHEN a reload makes solid a cell the player's box does not
    overlap THE SYSTEM SHALL leave the player exactly where they are.
  - FR-7.1-S4: WHEN the nearest clear position is one block below and a clear
    position exists two blocks sideways THE SYSTEM SHALL move the player
    sideways, never downward.
  - FR-7.1-S5: IF no clear position exists within the declared search bound of
    8 blocks THEN THE SYSTEM SHALL leave the player where they are and report
    that they could not be cleared, the reload still standing.
  - FR-7.1-S6: IF a candidate that would make solid a cell the player's box
    overlaps is refused THEN THE SYSTEM SHALL leave the player where they are, a
    refused candidate moving nobody.
  - FR-7.1-S7: WHEN a reload moves a player upward to clear them THE SYSTEM
    SHALL leave their velocity zero, so that the next tick does not carry them
    straight back into the cell they were moved out of.
  - FR-7.1-S8: WHEN content is reloaded such that a player is left inside solid
    ground near the world boundary and every position the search could otherwise
    take lies outside the loaded world THEN THE SYSTEM SHALL report that the
    player could not be cleared and leave the player where they are.

  **FR-7.1-S8 asserts a refusal, so it is vacuously satisfied by a search that
  finds nothing ever.** A paired positive control is therefore **mandatory and
  lives in its own test function**: the same wedge, in a world wide enough to hold
  an eligible candidate, yields a move. Without it, deleting the candidate
  generator outright leaves S8 green.

  **Eligibility, which is what S8 is about.** A candidate is eligible only if
  every cell the player's box would cover is **known and clear**. Outside the
  loaded world is *unknown*, not clear, and a search over unknown ground is not a
  search. The shipped world is 64 blocks square and the reach is 8, so **any
  player trapped within 8 blocks of an edge has candidates outside the world**,
  and in a wedge those are the nearest "clear" ones the ring search meets — the
  player is then put where nothing is solid and falls out of the world. Reachable
  by walking to an edge and reloading a solidity change.

  **No new vocabulary.** A boundary wedge with no eligible candidate takes the
  refusal path FR-7.1-S5 states and FR-7.1-S6 already grades. And FR-7.1-S5's
  two-column, 32-block world stops being a fixture workaround and becomes the
  rule: a wedge fixture has to hold the whole cube for its premise to be about
  the search rather than about the footprint.

### FR-8 — A reload is published to readers, not applied locally

- **FR-8.1**: The content a reader draws with is published by the simulation
  under a serial, and a reader honours what it was handed.

  - FR-8.1-S1: WHEN a candidate is accepted THE SYSTEM SHALL publish the
    resolved content a reader draws with, together with the content serial it
    belongs to.
  - FR-8.1-S2: WHILE no candidate has been accepted THE SYSTEM SHALL leave a
    reader observing the content serial it last observed.
  - FR-8.1-S3: WHEN a reader is handed a published content set across a reload
    whose layer assignment is deliberately not the lexicographic one THE SYSTEM
    SHALL pack and draw the indices that assignment states.
  - FR-8.1-S4: WHEN two candidates are accepted in one session THE SYSTEM SHALL
    publish the second under a content serial distinct from the first, so that a
    reader can tell a reload that happened from one that did not.
  - FR-8.1-S5: IF a candidate is refused THEN THE SYSTEM SHALL leave the
    published content and its serial exactly as they were, no part of a refused
    candidate reaching a reader.

- **FR-8.2**: The seam SPEC-016 cut stays cut, and the capture pipeline watches
  nothing.

  - FR-8.2-S1: THE SYSTEM SHALL leave the client's own sources naming none of
    the doors that read a content root, the reload path and the watcher
    included.
  - FR-8.2-S2: THE SYSTEM SHALL leave the capture pipeline's own sources naming
    no watcher door, a golden frame being a claim about one content set,
    reported by a scan whose verdict tells a clean pipeline apart from sources
    it could not read.
  - FR-8.2-S3: WHEN that same scan is run over a source that does name the
    watcher door THE SYSTEM SHALL report that file and the spelling it named,
    rather than reporting a clean pipeline.

### FR-9 — A reload never blocks the tick, and its window is declared once

- **FR-9.1**: The gate asserts what is deterministic; the latency itself is
  measured rather than asserted.

  - FR-9.1-S1: WHEN a reload's whole-world re-mesh runs THE SYSTEM SHALL block
    no tick on it, the ticks advanced while it runs being the ticks the same
    inputs would have advanced with no reload in flight.
  - FR-9.1-S2: THE SYSTEM SHALL declare the 150 ms settling window in exactly
    one place, reported as an enumerated verdict that tells one declaration
    apart from several and from a scan that read nothing.

## Technical Considerations

### The scratch VM already exists; what is new is where it is called from

`LuauFileDefinitionSource` builds a `ScriptHost` inside `definitions()`, uses it
for every file and drops it before the call returns. So invariant 7's "candidate
registries are built in a scratch VM off the tick thread" needs no new
mechanism: it needs `mc_sim::content::load` called on a thread that is not the
tick's, which is what `spawn_preparation` already does at launch. The candidate
build is that call, on a worker, and the whole of it is already all-or-nothing —
`load` hands back a `LoadedContent` or a refusal and there is no third answer, so
partial application is unexpressible rather than avoided.

### The one structural claim this spec weakens, and it must be weakened deliberately

`crates/mc-sim/src/world/mod.rs` states that nothing outside its module can write
any of its three views and that exactly one function writes anything. A reload
replaces the registry and recomputes the solidity bitset from it, which is a
second write door into that type.

It must stay a *door* and not a *hole*: the swap has to write the registry and
recompute solidity together, from the same candidate, in one operation, exactly
as `World::write` settles solidity before either write. A swap that replaced the
registry and left the bitset to be refreshed by a caller would re-open the
disagreement the whole type exists to make unspellable — **and the replay's
overlap oracle would not catch it**, because that oracle re-reads the world
through the registry and would be agreeing with itself. FR-4.1-S4 is what asks
both views the same question, and it is in the spec because nothing else in the
tree can see this.

### The registry becomes published, which is what invariant 6 has always described

Today the registry is an `Arc<BlockRegistry>` shared three ways and never
replaced. Invariant 6 says mutation happens by building a replacement and
swapping via `ArcSwap` at a tick boundary. This spec is where that arrives.

The three holders each need a different answer and the architecture has to name
all three: the world's (swapped with its solidity, above), the re-mesh worker's
(which owns its copy on another thread, so it has to be *told*, and told before
it meshes anything against it — FR-6.2), and the renderer's texture layers
(uploaded from the frame path once the swap has happened).

**One in-tree doc comment is falsified by this and must be rewritten rather than
left standing.** `crates/mc-client/src/startup.rs`'s `scene_of` states that "the
registry does not change mid-session — so a re-mesh splicing its sections into
this list needs the same layers the rest of the scene was packed against, not a
second opinion about them." Its *conclusion* survives this spec intact and is
FR-5.1-S4; its stated *premise* does not. A reader who finds the old premise
after this lands will draw the wrong inference from it.

### The watcher is an external boundary and gets a port

`notify` and `notify-debouncer-full` are pinned in `[workspace.dependencies]`
and named by no crate. They are an external dependency in the
`code-quality.md` §5 sense, so they go behind a port named for the capability —
"has this content root changed?" — with the debouncing adapter behind it and an
in-memory double in front of it.

That is not ceremony here, it is what makes this spec testable at all: every
scenario in FR-1 through FR-8 has to be drivable without waiting on a real
filesystem to deliver a real event, and a spec whose acceptance depends on
inotify latency has no deterministic gate.

### The decision may not live in the frame path

`crates/mc-client/src/app.rs` needs a real window and nothing in this workspace
constructs one; the crate holds no coverage of its own and
`docs/technical/testing.md` records that coverage cannot say so. The tree has
already paid for this lesson twice — a client submitting a default intent every
tick left 406 of 406 tests green, and deleting the guard that stops a free cursor
turning the camera left the same.

So: **whether a candidate is accepted, what a reload swaps, what it refuses, what
it re-meshes and where a cleared player ends up are decided outside the frame
path.** What is left in `app.rs` is the upload of layers and of a scene it was
handed — the same share it already has of an edit. Ask of every function this
spec adds: what calls this, and what would go red if it stopped?

### What is observable in this increment, and what is not

The mesher resolves a quad's layer by parsing the block's **name** as a texture
key (`crates/mc-render/src/geometry/mod.rs`), and the held-block indicator does
the same. `crates/mc-render/CLAUDE.md` records both as one known gap;
`product/roadmap.md` gives it to PRO-902 and PRO-914 as a separate P0 item after
this one.

Two consequences this spec states rather than discovers:

- **Editing a placed block's `texture` field is not the demonstration.** It
  removes the block's old key from the layer table, and the mesher then looks up
  the block's name, finds no layer, and the batch fails. `docs/modding/blocks-items.md`
  already documents the load-time half of this ("will load and then not draw");
  the reload page must document the reload half, because nothing mechanical will
  tell the author. It is also why FR-4.3-S2 declares `texture` equal to `name`:
  a scenario asking for an independent texture key would be asking for the thing
  this spec puts out of scope.
- **What *is* observable** is solidity, the three mutation rules, and a newly
  declared block reaching the player's hand and being placed — which between them
  cover both halves of the definition-hash split and the layer append.
  `requirements.md` §Decision 3 enumerates them.

### The layer budget is spent monotonically, and that is the price of the policy

Never renumbering means never reclaiming. A session that reloads a hundred times,
each adding and removing one texture key, spends a hundred of the 256 layers. The
budget is fixed at 256 by the packed vertex's 8-bit layer field
(`crates/mc-render/src/geometry/vertex.rs`) and the array texture is allocated to
exactly that depth up front — so appending a layer is one `write_texture` of one
16×16 layer and never a re-creation of the array.

FR-5.2 is what keeps running out honest: a refusal naming the count, the bound
and the way out, rather than a wrong picture. FR-5.1-S5 is what keeps a *refused*
candidate from spending the budget silently, which would leak the 256 away with
every scenario green. Reclaiming retired layers within a session is a later
question and is in Out of Scope; nothing about the format has to change for it.

### Which sections a reload has to mesh again, and how that is decided

The save format already folds every definition into two values —
`behaviour_of` over `name`/`is_solid`/`replaceable`/`breakable`/`breaks_into`,
and `appearance_of` over `name`/`texture` — and its own record argues the split:
"a block whose texture changed is the same block to stand on, and a block whose
solidity or drop changed is not."

What decides FR-6.1 is narrower than either half and is stated as a rule rather
than a hash: **a reload meshes again only if some block's declared solidity or
declared texture key differs.** `replaceable`, `breakable` and `breaks_into`
change no geometry, and a block added but nowhere placed changes no geometry
either — it needs a layer, not a mesh.

Whether that comparison reuses the two folds or compares the fields directly is
the architecture's to settle. What the spec fixes is the rule and its bound:
**exactly** the shipped world's 256 sections for one reload that meshes at all,
and none for one that does not.

**"Exactly" and not "at most", and a measurement is what forces it.** The two
readings are not contradictory — exactly satisfies at most — and that is what
makes the loose one dangerous: it *invites* the selective design FR-6.1-S3
forecloses, and nobody would see the two disagreeing until an implementation
split the difference. Measured against `crates/mc-sim/src/replay/{height,world}.rs`
and `crates/mc-world/src/column.rs`: terrain runs from `LOWEST_SURFACE = 32` to
`HIGHEST_SURFACE = 48`, sea fills to 34, and one landmark pillar at world
`(12, 12)` reaches `LANDMARK_TOP = 64`, inside columns of `SECTIONS_PER_COLUMN = 16`
sections. **So the highest occupied section is 3 in fifteen of the sixteen
columns and 4 in one, and everything above holds no block at all.** A rule that
marked only the sections whose palettes hold a changed name, plus their
neighbours, would mark about **82 of the 256** — and fail FR-6.1-S3 outright.

So the marking is binary: a reload that changes some block's declared solidity
or declared texture key marks every section, and one that changes neither marks
none. **That is a measurement forcing a design and not a preference.** The
sections the binary rule adds over the selective one are exactly the empty ones,
which mesh to zero quads, so it costs almost nothing — but the reason it is not
optional is the count above. Narrowing it later is a change to FR-6.1-S3 and to
this paragraph, not an optimisation somebody may take while passing.

### Why the re-mesh cannot stall the tick, and what actually has to be bounded

It already cannot: a re-mesh runs on its own worker, one batch at a time, and
edits made meanwhile accumulate in the world's per-section dirty set. A reload
marking every section dirty is the *existing* transport carrying a bigger batch,
not a new path.

Two hazards that transport does not already handle, both in FR-6.2. A batch in
flight when a reload lands was meshed against superseded content, so its scene is
discarded — and **the sections it carried have to go back into the dirty set**,
or they stay stale for the rest of the run, which is a wrong picture with no
error anywhere (`crates/mc-client/src/remesh.rs`'s own header names this hazard
for the edit case). And the worker has to be told the new content *before* it
meshes anything with it.

What has to be bounded is latency, and the arithmetic is worth writing down
because it decides where the effort goes. The 256 sections mesh on rayon workers
at a benchmarked ~136 µs each for terrain (`product/roadmap.md`); the candidate
build evaluates four small chunks; the scene pack and upload are one frame's
work. **The settling window is the dominant term in the one-second target**,
which is why it is declared once (FR-9.1-S2) rather than left inside an adapter.

### Why the one-second target is measured and not gated

`CLAUDE.md` Key Principle 4 requires the gate to be deterministic, and
`testing.md` §8 requires a flaky test quarantined on sight. A wall-clock
assertion on shared hardware is a flake generator, and one that fails
intermittently teaches everybody to re-run it — which is the state in which it
reports nothing.

So the gate asserts the property that is deterministic — FR-9.1-S1, a reload
blocks no tick — and the latency is carried by a `criterion` benchmark run as a
standalone command, exactly as the mesher's < 200 µs/section budget already is —
`product/roadmap.md` records that one as "deliberately not a gate stage". The
end-to-end figure a player experiences, from an editor's save to a changed
picture, needs a real editor, a real filesystem and a real window, and is a named
manual acceptance check.

### Refusing a candidate that drops a placed block

The world's sections hold **names**, and `SolidVoxels::resolve` refuses a name
the registry does not know. The save path already refuses a `missing` name
outright, with reasoning this spec adopts unchanged: nothing can go in the cell,
and that is not a judgement to make on the author's behalf.

So a candidate is checked against the names the world actually holds before it is
accepted, and the refusal names every such block. The unknown-block path the
issue sketches — solid, occluding, placeholder texture, state preserved
opaquely — names two fields that do not exist and is in Out of Scope with its
reasoning.

### A dangling `breaks_into` is accepted, and that is the existing contract

`docs/modding/blocks-items.md` states it in as many words: "a residue is resolved
when a break happens, not when the declaration is read… That is what lets two
mods name each other's blocks without either having to load first." The loader
checks the id's *shape* and nothing else, and `BlockRegistry::apply` performs no
cross-reference pass.

A reload does not change that, and FR-4.2-S4 pins it rather than leaving it to
be quietly tightened. What it costs — a `breaks_into` naming nothing fails at
the break, not at the edit — is a property of the existing design and not
something this spec introduces or is scoped to fix.

### The whole root reloads, blocks and HUD together

`mc_sim::content::load` reads both and refuses them together because "a root that
is good for one and bad for the other is a root that failed". Splitting that for
reload would build a second, partial door into content, which is the partial
application invariant 7 calls a Blocker. So a reload attempt goes through the
same call, and a refused HUD declaration refuses the blocks with it (FR-2.1-S7).

### Mod tests do not gate this candidate, and saying so is the honest answer

`crates/mc-script/CLAUDE.md` and `content/CLAUDE.md` both describe a mod's
`tests/` running on every reload candidate before the swap. `content/CLAUDE.md`
also states why they cannot exist yet: there is no `mycraft.*` binding for a
declaration to call, so there is nothing for a mod-authored test to assert
against beyond what the loader already refuses.

What gates a candidate here is the loader's own all-or-nothing validation — the
same gate that runs at launch, refusing missing fields, unrecognised fields, bad
namespacing, duplicate names and every content-supplied quantity past its bound.
Both `CLAUDE.md` files are updated to say that plainly rather than leaving a
promise the tree does not keep.

### Per-cell state, and the USER RULING about resetting it

Per-cell state does not exist (PRO-911, deferred), so the issue's "texture change
→ state survives / shape change → state dropped" has nothing to bite on in this
increment, and no migration machinery is specified for state nothing can hold.
The ruling that changing a script may reset state is recorded here so the spec
that builds per-cell state inherits it rather than re-asking.

What *does* exist and survives is in FR-3: the world's placed blocks, the
player's position, orientation and velocity, the tick counter, the save.
Invariant 2 — state in Rust, behaviour in script — is what makes that true rather
than lucky, and this spec is the first thing that tests it for real.

## Existing Code to Leverage

| What | Location | Reuse |
|------|----------|-------|
| Reading a whole content root, all-or-nothing | `mc_sim::content::load` (`crates/mc-sim/src/content.rs:112`) | the candidate build, unchanged |
| Building a candidate off the tick thread | `spawn_preparation` (`crates/mc-client/src/launch.rs:143`) | the pattern the reload worker follows, `WorkerLost` included |
| A scratch host per read | `LuauFileDefinitionSource` (`crates/mc-world/src/content/luau_source.rs`) | invariant 7's scratch VM, already built |
| Refusals that name file, block and field | `DefinitionFault`, `RegistryError` | the reload's refusal vocabulary, unchanged |
| A recurring fault reported once | `App::report_remesh` (`crates/mc-client/src/app.rs:465`) | FR-2.1-S8's dedup contract |
| Publication that never waits on a reader | `ArcSwap` in `Simulation` (`crates/mc-sim/src/simulation.rs:61`) | the content publication's shape |
| The resolved seam a client draws from | `ResolvedContent` (`crates/mc-core/src/content.rs`), `ContentView` (`crates/mc-client/src/content.rs`) | what a reload republishes |
| Layers stated rather than derived | `TextureLayers::stated` (`crates/mc-render/src/texture/mod.rs:73`) | the honouring half; this spec adds the assigning policy |
| One array-texture layer written at a time | `write_layer` (`crates/mc-render/src/gpu/buffers.rs:169`) | filling an appended layer |
| Per-section dirty set and off-thread batches | `World::take_remesh_work`, `Remesher` | the reload's re-mesh, carrying a bigger batch |
| A failed batch reported and dropped, never fatal | `crates/mc-client/src/remesh.rs` header | FR-6.2-S3, unchanged |
| Two-value definition identity | `behaviour_of`/`appearance_of` (`crates/mc-world/src/persistence/format.rs`) | classifying what a candidate changed |
| Refusing a `missing` name, and why | `RegistryVerdict::refusal` (`crates/mc-world/src/persistence/table.rs`) | FR-2.2's rule and its reasoning |
| The held-block policy | `default_held_block` (`crates/mc-sim/src/world/action/mod.rs:324`) | re-derived on reload |
| Overlap against the solidity view | `crates/mc-sim/src/player/collide.rs` | the clearing search's predicate |
| A refusal quoted in the docs held to a real run | `crates/mc-client/tests/documented_refusals.rs` | the reload page's quoted refusals |
| A source scan with a three-way verdict and a control | `crates/mc-client/tests/client_names_no_content_door.rs` | FR-8.2, extended to the watcher door |
| A budget measured, not gated | the mesher's `cargo bench -p mc-world --bench meshing` | FR-9's latency benchmark |

## Documentation deliverable

Part of this spec's definition of done, not a follow-up (`CLAUDE.md` Key
Principle 3). Each audience must be able to act on it without reading Rust.

**Mod author**

- **`docs/modding/hot-reload.md` — new**, and `docs/INDEX.md` already routes
  "Hot-reload semantics and state migration" there. It states: what triggers a
  reload and what does not, including the 150 ms settling window; that the whole
  root is read, blocks and HUD together; every refusal an author can meet,
  quoted, with a real example held to a live run; what survives; that a newly
  declared block arrives in their hand and why that is a placeholder rule; the
  256-layer session budget, its refusal and that relaunching reclaims it; **the
  standing limitation that editing `texture` is not yet visible and what it does
  instead**; that a `breaks_into` naming nothing is still accepted and still
  fails at the break; and a complete worked example — edit one file, save, see
  the change — that runs.
- **`docs/modding/blocks-items.md`** — "What is not here yet" opens with "Hot
  reload of declarations: a declaration is read once, at load." That becomes the
  present tense, with a pointer to the new page.
- **`docs/modding/README.md`** — the first-block walkthrough gains its last
  step: keep the client running and edit the file you just wrote.
- **`content/CLAUDE.md`** — the mod-`tests/` paragraph says what actually gates a
  reload candidate today.

**Player**

- **`docs/user/gameplay.md`** — content edited while you are playing now reaches
  the world without a restart, and your world, your position and your save are
  untouched by it; if something you are standing in becomes solid you are moved
  to the nearest clear space rather than trapped, and told when you could not be.

**Engine reader**

- **`docs/technical/architecture.md`** — the reload seam end to end: the watcher
  port and its adapter, the candidate built off the tick thread through the
  existing content door, the tick-boundary swap, the second write door into
  `World` and why it must write the registry and the solidity view together, the
  content publication and its serial, how the re-mesh worker learns which content
  it is meshing against, and why a discarded batch's sections go back into the
  dirty set.
- **`crates/mc-client/src/startup.rs`** — `scene_of`'s doc comment states that
  the registry does not change mid-session. That premise is false after this
  spec while its conclusion holds; rewrite it rather than leave it to mislead.
- **`docs/technical/rendering.md`** — appended never renumbered within a session,
  what that buys (vertices already uploaded stay valid), the 256-layer budget,
  that appending writes one layer rather than re-creating the array, and that a
  retired layer is not reclaimed until relaunch.
- **`docs/technical/testing.md`** — how a reload is driven with no filesystem and
  no window, the mutation table for this spec, why the latency budget is a
  benchmark rather than a gate stage, and the one manual acceptance check no
  harness can drive (a real editor, a real save, a real window).
- **`docs/technical/decisions.md`** — an ADR for append-never-renumber and for
  refusing a candidate that drops a placed block, each with its rejected
  alternative.
- **`crates/mc-script/CLAUDE.md`** — invariant 7 gains the sentence saying where
  it is now realised, and the mod-`tests/` clause is corrected the same way
  `content/CLAUDE.md`'s is.
- **`docs/INDEX.md`** — register `modding/hot-reload.md`, add SPEC-017 to the
  Sources column of every file above.

## Out of Scope

Binding. Recorded rather than dropped.

- **The unknown-block path.** A placed block whose declaration has disappeared
  refuses the candidate (FR-2.2) rather than becoming a solid, occluding
  placeholder that preserves state opaquely. Two of those properties name fields
  that do not exist — `occludes` is PRO-904's and per-cell state is PRO-911's —
  and the tree has no concept of a block in the world but not in the registry.
  The reasoning for refusing is the save path's own and is preserved above.
- **Texture resolution through the registry, and per-face keys.** PRO-902 and
  PRO-914, sequenced after this spec by `product/roadmap.md`. Until they land,
  editing a declaration's `texture` field is not a visible change; the mod author
  is told so on their own page.
- **Cross-checking `breaks_into` at load or at reload.** Deliberately unchecked
  today so that two mods can name each other's blocks; FR-4.2-S4 pins the
  existing contract rather than changing it.
- **Reclaiming layers retired during a session.** Never renumbering means never
  reclaiming, and 256 layers is a generous session budget with an honest refusal
  at the end of it. Nothing about the format has to change to add reclamation
  later.
- **Per-cell state and its migration.** PRO-911, deferred. Including the schema
  hash the issue frames this work around, which has nothing to be about yet.
- **Mod-authored `tests/` gating a candidate.** Impossible until a `mycraft.*`
  binding exists for a test to assert against.
- **Reloading the rule expression graph, and reloading worldgen for an
  already-generated world.** Both named out of scope by the issue.
- **A content-set identity or hash.** With one process nothing can disagree, so
  nothing can falsify it — SPEC-016's reasoning, unchanged. The content serial is
  not this: it counts accepted reloads within one process and answers a question
  a reader in that process actually asks.
- **Moving the composition root and restoring the dependency-closure guard.** A
  later spec's, whose exit criterion it is.
- **An operator switch for whether a running server watches its content.** There
  is no server binary to configure and no operator surface to put it on.
- **Reloading anything but the content root** — engine configuration, bindings,
  shaders.

## Dependencies

- SPEC-016 (PRO-917): block declarations in Luau, the simulation reading content,
  `ResolvedContent`, and the stated-not-derived layer assignment. All merged.
- `notify` 8.2.0 and `notify-debouncer-full` 0.7.0, pinned in
  `[workspace.dependencies]` and reached by nothing yet.

## Assumptions

- One content root. Reading a second is not built and this spec does not add it.
- Singleplayer, one process, where the mod author and the player are the same
  person at the same keyboard. The publication shape (FR-8) is what keeps that an
  arrangement rather than an assumption baked into the design.
- The shipped world's 256 sections are the size a reload's re-mesh is bounded
  against. A world that outgrows a fixed footprint reopens FR-6.1-S3's bound, and
  that is stated where the bound is.
- A text editor's save is either an in-place write or a write-and-rename, and may
  produce sibling scratch files. Both are ordinary events for a debounced
  watcher; neither is assumed to be atomic.

## Open Questions

None.

## Clarifications

### Session 2026-08-17

- Q: Is the state-survival question vacuous, given that per-cell state is
  deferred? → A: For *per-cell* state, yes, and the spec says so rather than
  specifying machinery for state that cannot exist. The definition-identity half
  the issue reaches for is not vacuous: it exists in the save format's two-hash
  split and is reused to decide what a reload has to mesh again. What survives
  and is worth testing is the world, the player and the save (FR-3).
- Q: Does this spec own the routine that moves a player out of a newly solid
  cell? → A: Yes. There is no piston and no spec in MVP 2 that adds one, and
  placement into an occupied box is *refused* rather than resolved by moving
  anybody — so hot reload is not the first caller, it is the only thing in the
  tree that can create the situation, and it can create it today via
  `base:water`. Without it the headline feature can wedge a player permanently.
- Q: Does a texture edit demonstrate hot reload? → A: No, and that is
  `product/roadmap.md`'s ruling rather than a preference: the mesher resolves a
  layer by the block's name and PRO-902/PRO-914 own closing that, after this
  spec. The demonstration is solidity, the three mutation rules, and a newly
  declared block reaching the player's hand and being placed.
- Q: Is the block in the player's hand preserved or re-derived? → A: Re-derived.
  It is a policy over the registry standing in for an inventory, not something
  the player accumulated, and re-deriving it is the only way a newly declared
  block becomes reachable before PRO-929 lands.
- Q: Should a dangling `breaks_into` refuse a reload candidate? → A: No. The
  audit proposed it as a real hole, and it is a real consequence — but
  `docs/modding/blocks-items.md` states the late-resolution contract explicitly
  and gives its reason (two mods naming each other's blocks). Tightening it here
  would break a documented promise inside a spec that is not about it.
  FR-4.2-S4 pins the existing behaviour across a reload instead.

## Scenario audit

Run 2026-08-17 against the drafted scenarios by `sdd-scenario-auditor`, reading
the spec, this folder's `requirements.md`, the scenario guidelines, `testing.md`
and `CLAUDE.md`, and verifying its claims against the tree.

**Verdict: gaps found — 21 new scenario drafts and 13 revisions.** The count went
from 65 to 89. What it changed, by kind:

- **Scenarios that could pass for the wrong reason.** The whole of FR-3 asserted
  that things were unchanged and was satisfiable by a reload that never happened;
  it now runs against a candidate whose effect is separately observable in the
  same test (FR-3.1-S3). FR-6.1-S1 and S3 were jointly satisfied by an
  implementation that never re-meshes — FR-6.1-S4 is the paired control and S2/S3
  now carry counts. FR-4.4-S2 and FR-8.1-S2 were satisfied by a publisher whose
  serial never moves; FR-8.1-S4 and S5 close both directions. FR-8.2's absence
  assertion gained the positive control FR-8.2-S3.
- **Over-tight assertions.** FR-3.2-S1 demanded a position "exactly as it was"
  across a tick that applies gravity — it would have failed against a *correct*
  implementation, and the cheapest way to green it is to freeze the tick. It is
  now judged against an independent run of the same ticks.
- **A contradiction with this spec's own Out of Scope.** FR-4.3-S2 asked for a
  new block drawn from an independent texture key, which the mesher cannot do
  until PRO-902. It now declares `texture` equal to `name`.
- **Missing unwanted-behaviour scenarios** (guideline rule 4) in FR-1.1, FR-1.2,
  FR-4.1, FR-4.2, FR-5.1 and FR-7.1 — nine added.
- **Concrete values** (rule 3) throughout: real file names, the four blocks and
  their four layers, 1 000 000, 256 KiB, 150 ms, 8 blocks, 256 sections.
- **Boundaries**: two overlapping attempts, a change before the first tick, a
  builder thread lost, a refused candidate spending layer budget, 255 layers with
  two keys needed, and a discarded batch leaving sections permanently stale.
- **Un-testable as written**: FR-9.1-S1 was a wall-clock assertion in a
  deterministic gate. It is now the property that *is* deterministic — a reload
  blocks no tick — with the latency carried by a benchmark and a manual
  acceptance check. FR-8.2-S2 was an absence no implementation could fail; it is
  now a scan with an enumerated verdict.
- **A vocabulary collision**: "revision" already names the golden-capture
  revision, so the published counter is a **content serial**.

**One finding was rejected.** The audit proposed refusing a candidate whose
`breaks_into` names an undeclared block, having verified that no cross-reference
check exists. The observation is correct and the remedy is not: late residue
resolution is a documented, deliberate contract that exists so two mods can name
each other's blocks. It is recorded in Clarifications and in Out of Scope, and
FR-4.2-S4 pins the existing behaviour across a reload instead.

**Count after the audit: 89 scenarios** — FR-1: 16, FR-2: 13, FR-3: 12,
FR-4: 14, FR-5: 8, FR-6: 9, FR-7: 7, FR-8: 8, FR-9: 2.

### Architecture amendment, 2026-08-17

Two scenarios added after `/sdd-architect`, each closing a hole the architecture
stage found as an *assumption* — something the design had to answer that no
scenario asserted. Both are recorded here rather than folded in silently,
because an assumption that becomes a scenario is the one case where the
architecture is allowed to grow the spec.

- **FR-4.5-S1 — the HUD is applied with the blocks.** The more serious of the
  two. `mc_sim::content::load` refuses blocks and HUD together and FR-2.1-S7
  pins that; applying the blocks while leaving the HUD behind **is** partial
  application, which invariant 7 calls a Blocker. The spec therefore carried a
  Blocker-class property that nothing asserted. The architecture puts the HUD in
  the published content; this scenario is what stops the next reader removing it
  as an unused field.
- **FR-1.1-S9 — a change the loader would not read begins no attempt.** The
  over-eager-skeleton case: with no scenario for it, an implementation that
  rebuilds on any change anywhere under the content root passes every other
  scenario in FR-1.1. Because it asserts an *absence*, it carries its
  discriminating half in its own wording — the same instrument must begin an
  attempt for a file the loader does read — since an absence alone is satisfied
  by a watcher that never fires.

**Final count: 92 scenarios** — FR-1: **17**, FR-2: 13, FR-3: 12, FR-4: **15**,
FR-5: 8, FR-6: 9, FR-7: **8**, FR-8: 8, FR-9: 2.

FR-7 went from 7 to 8 after phases 1–7 had closed. FR-7.1-S8 was ruled in on the
strength of a defect phase 5 could not see: the clearing search's predicate is
"not solid", the world model answers `false` for every cell past its footprint,
and the two together put a player off the map. It is **phase 8's**, with its own
RED, and the count is stated here rather than left implied because a scenario
added late is the one a later reader most needs to find.
