# Architecture: A player entering a world is never left inside solid rock

Written 2026-08-18 against the tree at `18f4291`, and revised after one
`persona-architect` review round whose findings were each re-verified against
the tree. Every file and line cited below was read; nothing here is derived
from the spec's own citations.

Two decisions are the conductor's ruling and are designed **within**, not
revisited: the seam is the sole simulation constructor, and no join API, trait
or `Admission` abstraction is introduced. Decisions D1 and D2 record how they
are realised, not whether.

## Drivers

| Driver | Evidence in this tree |
|---|---|
| **Evolvability — one rule, not three copies** | Four ways into a world exist today (`crates/mc-sim/src/persistence.rs:153` resume, `:160` generate, `crates/mc-sim/src/simulation.rs:209` reload swap, and 22 test-fixture sites); exactly one of them clears the player. MVP 3 adds a fifth. The cost of getting this wrong is a rule restated per door. |
| **Correctness held by the compiler rather than by a scan** | `clearing::cleared` (`crates/mc-sim/src/world/clearing.rs:63`) is `pub(crate)` with one caller, `Simulation::clear_the_player` (`simulation.rs:243`), reached only from `adopt` (`:209`). Nothing structural stops a fifth door skipping it. |
| **Observability of a non-fatal event** | `report_clearing` (`crates/mc-client/src/app/reload.rs:96`) is the only place the reload's two sentences are composed, and reaching it needs a `wgpu::Surface` and a `winit` window that nothing in the workspace constructs. Its sentences are asserted by nothing. Repeating that shape gives FR-2 no observable at all. |
| **Determinism of the golden frames** | `crates/mc-client/tests/support/frames.rs:119` and `crates/mc-sim/tests/replay_player.rs:87` shoot every committed frame through `simulation_for`. An entry check that moved, cell-centred or grounded the derived spawn changes every golden image. |
| **Launch latency — a non-driver, measured** | `cleared` opens with `if !collide::overlaps_solid(feet, world) { return Unneeded; }` (`clearing.rs:64`). A 0.6-wide box lies inside one cell column, so a clear player costs the two cells their 1.8-tall box occupies. The 2 601-candidate ring is walked only by a trapped player. There is no latency driver here, which is why the search is unconditional. |
| **Operability of the change itself** | The largest risk in this spec is a *process* risk, not a design one: the return-type change reaches 43 test files. Sized in "Integration" and handled in "Phases". |

**Constraints.** No regulatory or data-sensitivity exposure: nothing here
touches personal data, a network, or a vendor. `mc-server` is a six-line stub,
so the composition root is `mc-client` for the whole of this spec. Rust 1.97,
edition 2024, `-D warnings` in the gate, no `rustfmt.toml` (so `max_width` is
100 and `format_strings` is off — string literals are never reflowed, which
matters to D6).

**What is volatile, what is expensive to reverse.** The public surface of
`mc-sim` (`simulation_at_launch`, `simulation_for`, and now the seating door)
is expensive to reverse because 43 test files bind to it. The sentence wording
is cheap to reverse. The scans are cheap to reverse. The demotion of
`Simulation::new` is the one-way door and is the subject of D1.

## Boundaries

No external dependency is added by this design. The table is not empty because
the design touches two existing boundaries.

| External dependency | Volatility (V/R/S/Sub) | Port | Adapter location | Direct-use justification |
|---|---|---|---|---|
| The save file on disk (`load_world`, `crates/mc-world/src/persistence/read/world.rs`) | low / none / stable / n.a. | none — `mc_world::persistence` **is** the repository | `crates/mc-world/src/persistence/` | architecture-principles §3: "repositories ARE the port for persistence, do not wrap the repository layer in a second port". This spec reads it through the existing `simulation_at_launch` and widens nothing — see D3. |
| The process's error stream (`eprintln!`) | none | none | n.a. | architecture-principles §3 excludes the standard library. The client's non-fatal notices already go straight to stderr (`crates/mc-client/src/app/mod.rs:475`, `app/reload.rs:83`, `:100`, `:110`); routing them through a reporting sink is Out of Scope, and doing it for one notice would create a second mechanism. |

## Decisions

### D1 — The door is `mc_sim::simulation::seat`, and `Simulation::new` becomes private to its own module. **BINDING**

The ruling fixes the seam. What it leaves open is the shape.

```rust
// crates/mc-sim/src/simulation.rs
pub struct Seated { pub simulation: Simulation, pub clearing: Clearing }

#[must_use]
pub fn seat(spawn: PlayerState, world: World, content: PublishedContent) -> Seated;

impl Simulation {
    fn new(spawn: PlayerState, world: World, content: PublishedContent) -> Self; // no `pub` at all
}
```

**Options.**

- **(a) `seat` as a free function in `simulation.rs`, `new` module-private.** One
  producer of a `Simulation` in the entire workspace.
- **(b) `seat` as an associated function `Simulation::seating`, `new`
  `pub(crate)`.** What the ruling literally describes.
- **(c) A `Seat` builder type.** A type with one construction path and one
  consumer, invented for a caller that does not exist.

**Evaluation.** (c) fails code-quality §1 outright — no abstraction before three
concrete uses, and it is not a port at an external boundary. Between (a) and
(b): `pub(crate)` leaves `persistence.rs` and `replay/spawn.rs` able to
construct a simulation directly, which is exactly the hole FR-1.3 exists to
watch, and it is then a hole a scan must cover. Module-private closes it in the
compiler for the whole crate, and it costs nothing — verified by reading every
caller: `Simulation::new` has exactly two callers in `mc-sim/src`
(`persistence.rs:153`, `replay/spawn.rs:103`), both of which move onto `seat`
regardless, and `mc-sim` has no sibling unit-test file for `simulation.rs`
(`crates/mc-sim/src/world/mod_test.rs` is the crate's only `*_test.rs`). The
`Seated` struct rather than a tuple, because `Accepted` (`simulation.rs:100`) is
already the reload's answer to the same question and reads at the call site as
`accepted.clearing`; `seated.clearing` matches it.

**Recommendation: (a).** It is one step stronger than the ruling in the ruling's
own direction — the compiler holds "one way seats a player" against every caller
including `mc-sim`'s own, not only against callers outside the crate. FR-1.3's
scan (D5) then guards the residual shapes: a second seating path added *inside
`simulation.rs`*.

**Strongest argument against.** Module privacy makes the FR-1.3 scan's needle
set depend on `simulation.rs` staying one file; if `Simulation` is ever split
across a module tree, `new` becomes visible to descendants again and the scan's
"one file" rule needs restating. Accepted: that split is a diff a reader sees
the whole of, and the scan reddens on it rather than going quietly green.

**Consequences for the two launch arms.** Both change return type, because a
verdict computed and dropped satisfies nothing:

```rust
// crates/mc-sim/src/replay/spawn.rs
pub fn simulation_for(world: &ReplayWorld, registry: Arc<BlockRegistry>,
                      content: PublishedContent) -> Result<Seated, SpawnError>;
// crates/mc-sim/src/persistence.rs
pub fn simulation_at_launch(save: &Path, launching: Launching) -> Result<Seated, LaunchError>;
```

`simulation_for` must carry it too: FR-2's sentence is deliberately true of a
generated world as well as a resume, and discarding the verdict at the generated
arm would make FR-1.3's own verdict name false.

### D2 — The rule is one `pub(crate)` function taking `&mut PlayerState` and `&World`. **BINDING**

This is the narrowest part of the design: how the rule is factored so MVP 3's
join calls it rather than restating it, while building nothing for that join.

```rust
// crates/mc-sim/src/world/clearing.rs
pub(crate) fn clear_the_player(
    player: &mut PlayerState,
    world: &dyn Solidity,
    ground: Extent,
) -> Clearing;
```

It is `Simulation::clear_the_player` (`simulation.rs:243`) with `&mut self`
replaced by the things it actually touches, and its doc comment moved with it.
`cleared` (`clearing.rs:63`) is untouched — Out of Scope forbids changing the
search — and the middle parameter stays `&dyn Solidity` so that D11's ruling
("`Solidity` is not widened; the world's extent is passed to `cleared`") is
carried forward to the letter.

**Two callers today, both in `mc-sim`:**

- `seat`, as `clearing::clear_the_player(&mut spawn, &world, world.extent())`,
  on the owned `spawn` before `Simulation::new` publishes the first snapshot —
  which is what makes FR-1.1-S7 structural rather than a second publish.
- `Simulation::adopt` (`simulation.rs:217`), as
  `clearing::clear_the_player(&mut self.player, &self.world, self.world.extent())`.
  Two disjoint field borrows of `self`, which the borrow checker accepts inside
  the method body; the method `clear_the_player` is deleted.

**Options for the signature.**

- **(a) `(&mut PlayerState, &World)`** — the extent is read inside, from the
  world being seated into, so `world.extent()` appears in exactly one line in
  the crate and no future caller can pass the wrong one.
- **(b) `(&mut PlayerState, &dyn Solidity, Extent)`** — mirrors `cleared` and
  leaves the extent an argument at every call site.

**Evaluation.** The case for (a) is real: an argument a future caller can get
wrong is a rule restated rather than reused, and on that ground (a) is the
better engineering. It is nevertheless not available here, for two reasons
found by reading:

1. **The spec binds it.** "the entry caller must pass the played world's own
   extent. FR-1.1-S6 catches an extent that is too large and FR-1.1-S1 catches
   one that is too small" — S1 is designed as S6's positive control *through the
   call-site argument*. Technical Considerations are binding.
2. **(a) would leave both scenarios with almost nothing to catch.** The
   tempting defence of (a) — "S6 still owns the loaded world's extent" — does
   not hold. It is not true that S6 reddens nothing in
   `crates/mc-client/tests/reload_*.rs`:
   `reload_will_not_clear_a_player_off_the_map.rs` exists for exactly the extent
   question, in both directions — a near-edge wedge carried by the sign check
   and a deliberate far-edge companion (`:201-210`) where *only* an extent can
   refuse the candidates. Nor is "the loaded world's extent might not round
   trip" a target S6 would own: `crates/mc-world/tests/save_round_trip.rs:170`
   already asserts a loaded world reports the extent it was saved with, against
   a written-out expectation. Under (a), S6's only remaining target would be
   `seat` being handed the wrong `World` outright.

One argument for (a) is stronger than it looks and still does not overcome the
above, so it is recorded rather than left for somebody to rediscover: (b)'s
usual defence is that `&dyn Solidity` keeps the fixture doubles usable, and that
defence is empty here. `cleared` is `pub(crate)` with exactly one caller
(`simulation.rs:244`), and the three `impl Solidity` doubles in the tree
(`crates/mc-client/tests/camera_lens.rs:60`,
`crates/mc-sim/tests/support/chamber.rs:150`,
`crates/mc-sim/tests/support/solidity.rs:70`) exist for `collide`, not for the
search. A concrete `&World` would cost nothing in testability. What decides it
is the spec and the scenarios, not the parameter type.

**Recommendation: (b).**

**Strongest argument against.** (b) is the weaker factoring for MVP 3: a join
must pass the extent itself, and can pass a wrong one. What contains that is
narrow and should be said plainly — the join calls the same three-argument
function the reload already calls, so the omission it cannot make is *forgetting
the rule*, and the mistake it can still make is *supplying the wrong ground*.
FR-1.3's scan makes the first visible; nothing here makes the second visible,
and a future spec that adds a join owes it a scenario of its own. Revisit (a)
if a third caller appears — at that point three call sites reading
`world.extent()` is a rule with three copies, which is the condition (a) was
right for.

**For the test author.** S1 must assert the exact destination (one cell
sideways, horizontally centred, feet on that cell's floor) and never be weakened
to "covers no solid cell" — that is what makes it S6's control. S6 must assert
the exact stayed-put position and its fixture must be a real save file read
through the shipped launch.

**What is deliberately not built.** No trait, no `Admission` type, no join API,
no parameterisation over "who is being seated". A networked join calls
`clearing::clear_the_player(&mut joining, &self.world, self.world.extent())`
with the player it is admitting and the world it is admitting them to, and gets
the same `Clearing` back. That is the whole inheritance, and it needs nothing
added today.

### D3 — Unconditional. **BINDING (ruling carried forward)**

Decided in `requirements.md` §3 and reconfirmed against the tree:
`RegistryVerdict` (`crates/mc-world/src/persistence/table.rs:52`) is computed
and dropped inside `load_world`, and `LoadedWorld` carries only
`{ world, player }`. Gating means widening `mc-world`'s persistence return,
which Out of Scope forbids. FR-1.2-S2 is what makes it falsifiable. No options
analysis: the alternative is out of scope by construction.

### D4 — The verdict is parked in `PreparedLaunch` and nowhere else. **BINDING**

FR-2.1-S6 asks that a run past its first drawn frame writes no further entry
sentence. The answer is structural, not a dedup field.

```rust
// crates/mc-client/src/launch.rs
pub struct PreparedLaunch { /* … */ pub clearing: Clearing }
pub fn simulation_to_play(save: &Path, launching: Launching)
    -> Result<(Seated, BlockName), PreparationError>;
```

**Options.**

- **(a) A field on `PreparedLaunch`, consumed at collection.** The verdict is
  produced on the preparation worker (`launch.rs:186`), rides the value the
  worker returns, and is moved out once by `App::collect_preparation`
  (`crates/mc-client/src/app/mod.rs:417`), which reaches its body only after
  `self.preparation.take()` (`:425`) leaves `None` behind.
- **(b) A field on `App` or `Session`, with a `reported_entry: bool` beside it.**
  The shape `reported`, `reported_remesh`, `reported_swatch` and
  `reported_reload` (`app/mod.rs:106-116`) already use.
- **(c) A field on `Simulation`, read by the frame path.**

**Evaluation.** (b) and (c) are the shape the spec warns about: "a parked value
read by the frame path repeats every frame", and the dedup flag is then the only
thing standing between the player and a sentence per frame. Those four existing
fields exist because their events *recur* — a frame keeps failing, a block keeps
having no swatch. An entry happens once per process run, so a dedup field would
guard against a repetition the design need never permit. (a) makes the
repetition unspellable: after collection there is no entry `Clearing` anywhere
in the client to read.

**Recommendation: (a).** Confirmed structural in review: the guard at `:418-424`
returns early unless the handle is finished, and `:425` takes it, so no second
frame reaches the body.

**Strongest argument against.** It puts a field on `PreparedLaunch` whose header
argues at length that the type carries nothing the frame path could pick the
wrong one of (`launch.rs:56-61`). A `Clearing` is a plain `Copy` verdict with no
second candidate, so the argument does not bite — but the header needs a
sentence saying why, and a reader who skips it sees the precedent rather than
the distinction.

### D5 — FR-1.3's instrument: `crates/mc-sim/tests/one_way_seats_a_player.rs`. **BINDING**

Shape: `crates/mc-client/tests/reporting_seam.rs` — production text with `///`
and `//!` lines stripped, sibling `*_test.rs` files skipped, `/`-separated
relative paths, `tempfile` trees as the positive controls. Roots:
`crates/mc-sim/src`, no exemptions.

**The verdict is total and enumerated:**

```rust
enum Seating {
    /// The one door makes the one simulation, and it hands the clearing back.
    OneWaySeatsAPlayerAndItReportsItsClearing,
    /// These sources put a player into a simulation too, or do it twice.
    AnotherSourceSeatsAPlayer(Vec<Site>),
    /// A spelling the door is known by is not where the rule says it is — it
    /// moved, was renamed, or the scan can no longer see it.
    TheDoorNoLongerSeatsAPlayer(Vec<String>),
    /// No production source was read.
    NoSourceWasRead,
}
```

`Site` carries `{ file, names, times }`. The `times` count is what closes the
hole `reporting_seam.rs` leaves open: that scan pushes one site per
(file, needle) pair, so a *second* offence in an already-named file is
invisible. Here a second construction inside `simulation.rs` moves `times` from
1 to 2 and the verdict changes.

**Needles. Each expected count below is a stated rule, never a number
transcribed from a green run** (testing.md §2 forbids snapshots):

| Needle | The rule | Verified in the tree today |
|---|---|---|
| `Simulation::new(` | once, and only in `src/simulation.rs` | 2 sites (`persistence.rs:153`, `replay/spawn.rs:103`), both moving onto `seat` |
| `Self::new(` | **never, anywhere** — a second door written as an associated function on `Simulation` is the shape it catches | 0 occurrences in all of `crates/mc-sim/src` |
| `published:` | exactly twice, and only in `src/simulation.rs` — the field is declared once and set once, so a second door building the struct by literal shows up whatever it spells the type or the initialiser as | 2 (`simulation.rs:118` declaration, `:145` construction) |
| `self.player =` | once, and only in `src/simulation.rs` — a second replacement of who a simulation's player is, which is the shape a join written as a method takes | 1 (`simulation.rs:183`, the tick) |
| `) -> Seated` | once, and only in `src/simulation.rs` — the door still hands the clearing back. `-> Result<Seated,` does not contain it, so the two launch arms are not sites | 0 today |

**Why a field spelling and not `Self {`.** With `new` module-private, the
natural way to add a second seating path *inside `simulation.rs`* is the struct
literal or `Self::new(` from an `impl` block — and neither contains
`Simulation::new(`, `self.player =` or `) -> Seated`. That path seats a player
with **no clearing at all**, so it is the one shape the scan most has to catch.
`Self {` would be the obvious needle and is a poor one: it also matches
`-> Self {` and stands at four lines of `simulation.rs` alone (`:86, :87, :143,
:144`), and a literal spelled `Simulation { … }` escapes it entirely. Every
construction must set `published`, whatever it spells the type or the
initialiser as.

**Two properties of the walker the needles depend on.** `production_text`
strips only `///` and `//!` lines (`reporting_seam.rs:307-316`), so an ordinary
`//` comment naming a needle *is* a site — commit C must not leave
`// moved off Simulation::new` behind. And the needle semantics here are the
inverse of `reporting_seam.rs`'s, where a needle found is the offence: each
needle carries an expected file and an expected count, and any departure is.
Derive the expected `Vec<Site>` from the table in code, the way
`every_needle_named_in` (`reporting_seam.rs:188-197`) derives from the needle
list, so a needle added without an expectation fails rather than standing
unwatched.

**The verdict's name is carried half by this scan and half by FR-1.1.**
`) -> Seated` present once says a function returns the type; nothing here says
`seat` calls `clearing::clear_the_player`. What the scan holds is that exactly
one place constructs a simulation and exactly one function hands a `Seated`
back; that the clearing actually runs is FR-1.1's behavioural tests.

**Positive controls — three, each feeding a different verdict:**

1. A tempdir tree whose `src/simulation.rs` is well-formed **plus** a
   `src/join.rs` that constructs a simulation → `AnotherSourceSeatsAPlayer`
   naming `src/join.rs`. This is FR-1.3-S2, and it is the MVP 3 shape.
2. A tempdir tree whose `src/simulation.rs` omits `) -> Seated` →
   `TheDoorNoLongerSeatsAPlayer`. Without it, a renamed door reads as a clean
   crate forever.
3. An empty tempdir → `NoSourceWasRead`.

**What this does not catch, written down rather than papered over.** A second
`pub fn` added to `simulation.rs` that calls `seat` and discards the `Clearing`.
That path still *runs the rule* — the player is still cleared — so the invariant
this spec is about holds; only the reporting is lost, and only for a caller that
does not exist. This is the residual hole, and D1 makes it narrower than the one
the ruling assumed under `pub(crate)`.

### D6 — FR-2.1-S5 and S6's instrument: `crates/mc-client/tests/the_entry_sentence_is_said_once.rs`. **BINDING**

Same shape, over `crates/mc-client/src`. It answers the three outcomes FR-2.1-S5
names — writes what was composed, composes one of its own, writes none — plus
FR-2.1-S6's "says it twice".

```rust
enum EntrySentence {
    /// One place composes it, one place says it, and no client state holds an
    /// entry verdict.
    ComposedOnceAndSaidWhereTheLaunchIsCollected,
    /// A source composes or says it somewhere else, or says it more than once.
    AnotherSourceComposesOrSaysIt(Vec<Site>),
    /// The composition is there and nothing asks for it.
    ComposedButNeverSaid,
    /// No production source was read.
    NoSourceWasRead,
}
```

**A needle may not be a whole sentence.** The obvious choice — the two composed
sentences verbatim — cannot match the source at all, in two independent ways:

- the refusal **interpolates its reach** — `Clearing::NoClearSpaceWithin` carries
  `blocks` (`crates/mc-sim/src/world/clearing.rs:53`) and the inherited print
  spells `nothing within {blocks} blocks is clear`
  (`crates/mc-client/src/app/reload.rs:110`), so the source never contains the
  literal `8`;
- the refusal is ~133 characters and must wrap across a `\` continuation, which
  `production_text` joins with `\n` before the `contains` check — exactly as
  `app/reload.rs:101` and `:110` already do.

So the needles are short, un-interpolated fragments that cannot be split, plus
one identifier. This also fixes the accompanying constraint on `notice.rs`:
**each distinguishing clause is declared as a `&str` const**, the idiom
`CONTENT_NOT_TAKEN_UP` (`crates/mc-client/src/session/reload.rs:57`, used at
`app/reload.rs:83`) already sets, so rustfmt puts each literal on a line of its
own and never inside a continuation.

| Needle | The rule | Verified in the tree today |
|---|---|---|
| `entered the world inside solid blocks` | once, and only in `src/notice.rs` — the clause both entry sentences open with, declared once | 0 |
| `so you were left inside them` | once, and only in `src/notice.rs` — the refusal's distinguishing tail, which carries no interpolation | 0 |
| `fn say_entering` | once, and only in `src/notice.rs` | 0 |
| `notice::say_entering(` | once, and only in `src/app/mod.rs` — **the call is module-qualified, and that spelling is BINDING**, so the count is the number of calls and cannot be moved by import style | 0 |
| `Clearing`, over the whole of `src` | named in exactly three files — `src/notice.rs` (it composes from one), `src/launch.rs` (`PreparedLaunch` carries one) and `src/session/reload.rs` (the reload's verdict rides one to the frame path). A fourth file is a verdict parked somewhere the frame path can re-read it | 7 today: `app/reload.rs:12, 96, 98, 99, 108`, all of which leave with `report_clearing`, and `session/reload.rs:15, 79`, which stay. `session/mod.rs:295` is the English word in a `///` line, not the type |

**The last needle is scoped to all of `src`, and `src/app` alone would be the
wrong scope.** The frame path's *state* does not live on `App`: `redraw` takes
`&mut Session` (`crates/mc-client/src/app/mod.rs:179`), and
`crates/mc-client/src/session/reload.rs:79` already parks a `Clearing` on
`ReloadReport::Accepted`. A verdict parked for re-reading would most naturally
be parked right beside it — D4's forbidden shape, with a working precedent three
files away — where a scan scoped to `src/app` could never see it. A file set is
the rule rather than per-file counts, because `notice.rs` and `launch.rs` will
legitimately name the type several times and a count there would churn.

Splitting `fn say_entering` from `notice::say_entering(` is what makes
"composed but never said" and "said twice" different answers: zero calls with
the definition present is `ComposedButNeverSaid`; two calls is
`AnotherSourceComposesOrSaysIt`.

**Positive controls:** a fixture whose `src/app/frame.rs` spells the clause
itself → `AnotherSourceComposesOrSaysIt`; a fixture holding `src/notice.rs`
alone → `ComposedButNeverSaid`; an empty tempdir → `NoSourceWasRead`.

**Residual hole, written down.** A site that imports `notice`'s consts and
composes a third sentence out of them, or a `say_entering` whose body stopped
writing. The first is caught by review only; the second by the composition tests
plus the fact that `say_entering`'s whole body is the `eprintln!`. What backs
the needles structurally is `-D warnings`: a name imported and not called is an
unused import and fails the gate, so "named but never called" is not a state
this tree can be in.

### D7 — The composition seam: `crates/mc-client/src/notice.rs`. **BINDING**

```rust
/// The clause both entry sentences open with. A const so that neither literal
/// is ever wrapped, and so the two sentences cannot drift apart.
const ENTERED_INSIDE_SOLID_BLOCKS: &str = "you would have entered the world inside solid blocks";

/// The sentence an entry owes the player, or nothing where it owes none.
pub fn entering(clearing: Clearing) -> Option<String>;
/// The sentence a reload owes a player who was already playing.
pub fn reloading(clearing: Clearing) -> Option<String>;
/// Says what `entering` composed, on the stream the client's notices use.
pub fn say_entering(clearing: Clearing);
/// Says what `reloading` composed.
pub fn say_reloading(clearing: Clearing);
```

`entering` and `reloading` are total functions of a `Copy` enum: no device, no
window, no `App`, no session. FR-2.1-S1, S2 and S3 assert `entering`'s three
answers against the exact strings; FR-2.1-S4 asserts `reloading`'s, which closes
the reload's existing hole and simultaneously guards against the two being
unified.

**Why not the shape that caused the hole.** `report_clearing`
(`app/reload.rs:96`) is a private free function in a module that exists only to
be called from `impl App`, and reaching it needs a `wgpu::Surface` and a
`winit::Window`. Two things change: the composition leaves that module entirely,
and the `say_*` pair is what remains at the device-bound call sites — one line
each, and that one line is what D6's scan grades.

**Why `mc-client` and not elsewhere.** `mc-render` is impossible: `Clearing` is
`mc_sim::world::Clearing`, and `crates/mc-render/tests/dependency_graph.rs`
asserts neither crate resolves the other. `mc-sim` is wrong: it is the server,
and the sentence is what a *client* tells the person at the keyboard.
`crates/mc-client/src/lib.rs:20` claims "This crate holds no policy" — that
claim already has this exception (the reload's two sentences have been composed
here since PRO-918), and this spec makes the exception visible and tested rather
than widening it. The lib header owes a sentence saying so.

**`notice.rs` is scanned by an existing guard and must stay inside it.**
`crates/mc-client/tests/reporting_seam.rs` walks all of `src` for `.to_string()`,
`Ending::Failed`, `{failure}`, `{cause}` and `{refused}`. Compose with `format!`
and bind the coordinates as `{x}`, `{y}`, `{z}` and the composed line as
`{said}`; a `Vec3` flattened with `.to_string()` reddens that guard, and the
failure will read as being about error rendering.

**Formatting is inherited, not chosen.** `eprintln!("… ({x}, {y}, {z})")` with
`Display` on `f32` is what `app/reload.rs:100-106` already does, and it is what
produces FR-2.1-S1's exact expected text: feet at `(12.5, 10.0, 12.5)` render as
`(12.5, 10, 12.5)`. **No width or precision specifier may be added** — `{:.1}`
renders `10.0` and reddens the scenario.

### D8 — Two scan copies, not one shared engine. **DEFERRED (revisit when a fifth guard is written, or when the copies are found to have drifted in a way that mattered)**

`crates/mc-client/tests/seam_boundaries.rs` and `reporting_seam.rs` already
carry one copy each of the walk-and-needle scaffolding; D5 and D6 make four. The
three-uses rule in code-quality §1 would now *permit* extracting it into
`mc-testkit`, and D5's `times` field means the copies are no longer identical
anyway.

Not extracted, for one reason: each of these guards is worth exactly as much as
a reader's ability to see the whole of it in one file, and a shared engine makes
every guard weakenable by an edit made for another guard's sake. That is the
failure mode `reporting_seam.rs:41-45` records about exemption lists, one level
up.

### Trivial decisions

- `Seated` derives `Debug` only. `Simulation` has a hand-written `Debug`
  (`simulation.rs:292`) and is neither `Clone` nor `PartialEq`.
- `#[must_use]` on `seat`, matching `Simulation::new`'s existing attribute
  (`simulation.rs:142`).
- `use glam::Vec3;` (`simulation.rs:41`) becomes unused when `clear_the_player`
  moves out — its only use is `Vec3::ZERO` at `:247`. Remove it in the same
  commit; `-D warnings` will insist.
- `notice` is declared `pub mod notice;` in `crates/mc-client/src/lib.rs`. The
  name is the tree's own word for these — `docs/technical/architecture.md`
  already calls them the client's non-fatal notices.

## Interfaces

```rust
// crates/mc-sim/src/world/clearing.rs        (new, pub(crate))
pub(crate) fn clear_the_player(player: &mut PlayerState, world: &dyn Solidity,
                               ground: Extent) -> Clearing;

// crates/mc-sim/src/simulation.rs            (new public surface)
#[derive(Debug)]
pub struct Seated { pub simulation: Simulation, pub clearing: Clearing }

#[must_use]
pub fn seat(spawn: PlayerState, world: World, content: PublishedContent) -> Seated;

impl Simulation {
    fn new(spawn: PlayerState, world: World, content: PublishedContent) -> Self;  // was `pub`
    fn clear_the_player(&mut self) -> Clearing;                                   // deleted
}

// crates/mc-sim/src/replay/spawn.rs          (return type changes)
pub fn simulation_for(world: &ReplayWorld, registry: Arc<BlockRegistry>,
                      content: PublishedContent) -> Result<Seated, SpawnError>;

// crates/mc-sim/src/persistence.rs           (return type changes)
pub fn simulation_at_launch(save: &Path, launching: Launching) -> Result<Seated, LaunchError>;

// crates/mc-client/src/launch.rs             (return type and struct change)
pub fn simulation_to_play(save: &Path, launching: Launching)
    -> Result<(Seated, BlockName), PreparationError>;
pub struct PreparedLaunch { /* … unchanged … */ pub clearing: Clearing }

// crates/mc-client/src/notice.rs             (new module)
pub fn entering(clearing: Clearing) -> Option<String>;
pub fn reloading(clearing: Clearing) -> Option<String>;
pub fn say_entering(clearing: Clearing);
pub fn say_reloading(clearing: Clearing);
```

**Error contracts.** None added. `seat` is infallible: `cleared` is total over
its inputs and `NoClearSpaceWithin` is a verdict, not a refusal. `LaunchError`
and `SpawnError` are unchanged — a player who cannot be cleared still launches
(requirements §4), and no new variant is introduced.

**Exact composed text**, from FR-2.1-S1, S3 and the two the reload already
writes. `entering` and `reloading` each answer `None` for `Clearing::Unneeded`:

```
mycraft: you would have entered the world inside solid blocks, so you were moved to (x, y, z)
mycraft: you would have entered the world inside solid blocks and nothing within 8 blocks is clear, so you were left inside them
mycraft: the reload made your cell solid, so you were moved to (x, y, z)
mycraft: the reload made your cell solid and nothing within 8 blocks is clear, so you were left where you were
```

The `8` is `Clearing::NoClearSpaceWithin { blocks }` interpolated, never a
literal — see D6.

## Data

No entity, field, migration or retention rule changes. The save format is
untouched: a resumed player is still placed from the stored position, yaw and
pitch through `resuming` (`persistence.rs:192`), with `velocity: Vec3::ZERO` and
`on_ground: false`. Clearing happens after that and before publication, which is
what FR-1.1-S3 (facing preserved) and FR-1.1-S7 (first snapshot shows the move)
both rest on.

**A cleared player arrives with `on_ground: false`, and that is deliberate.**
`resuming` sets it (`persistence.rs:199`) and the clearing move touches position
and velocity only (`simulation.rs:245-248`). So a player put on a cell floor at
entry is *standing on it while claiming no contact*, and tick 1 settles that the
way it settles every other resumed player — by falling a fraction and landing.
Setting it in the search would be a claim about contact that nothing checked,
which is the same reason `resuming` does not set it, and grounding the player is
explicitly Out of Scope. **The consequence for the test author is a trap:
FR-1.1-S7 is about the *first* snapshot.** Read the player at tick 1 instead of
tick 0 and the `y` differs, the test goes red against a correct implementation,
and the cheapest green is to ground the player or set `on_ground` in the search
— both forbidden. This is the twin of the `centre_of` `y + 0.5` trap in the risk
table, reachable from the spec text in the same way.

**The move is what the next save records, and that is what makes the feature
terminate.** `persistence::save` (`persistence.rs:99`) writes the position from
the snapshot the simulation last published, so a player cleared at entry who
plays and quits resumes at `Unneeded` next launch. Entry clearing is
self-limiting by that route rather than by a flag, which is why no "already
cleared" state is stored anywhere.

## Integration

**Production files touched (7):**

| File | What changes | What must not break |
|---|---|---|
| `crates/mc-sim/src/simulation.rs` | `Seated`, `seat`, `new` demoted, `clear_the_player` method deleted, `Vec3` import removed | `SimSnapshot` stays `Copy` and free of interior mutability (`tests/publication.rs` pins it by compiling). `Accepted` keeps its own `clearing` field — the two verdict carriers stay separate, which is what FR-2.1-S4 grades. |
| `crates/mc-sim/src/world/clearing.rs` | gains `clear_the_player`; `cleared` untouched | Out of Scope: reach, ring order, cell centres, no downward candidates, eligibility. |
| `crates/mc-sim/src/persistence.rs` | resume arm calls `seat`; returns `Seated` | The one distinction deciding resume-vs-generate is still "is there a save at all" (`:152`); a resume still does no worldgen. |
| `crates/mc-sim/src/replay/spawn.rs` | `simulation_for` calls `seat`, returns `Seated` | The derived spawn is unchanged — FR-1.1-S4 is the guard, and every golden frame depends on it. |
| `crates/mc-client/src/launch.rs` | `PreparedLaunch.clearing`; `simulation_to_play` returns `Seated` | The four-step order in `prepare_launch` (`:186`): which world first, then mesh, then pack. No device, no window, no working-directory change. |
| `crates/mc-client/src/app/mod.rs` | `collect_preparation` gains one line, `notice::say_entering(prepared.clearing);` | It still decides nothing. Placed after the uploads succeed, so a launch that fails to reach the device says nothing about where the player was put. |
| `crates/mc-client/src/app/reload.rs` | `report_clearing` deleted; `take_up_reloaded_content` calls `notice::say_reloading(clearing)` | The reload's two sentences stay character-identical; `report_reload`'s dedup field is untouched. |

**The test surface, counted rather than estimated — and a call-site grep cannot
size it, because this is a *type* change.** Four facts, each verified by
reading, and one piece of good news after them:

- **24 `Simulation::new` call sites**, 2 in production and 22 in test code
  across 18 files (14 in `mc-sim/tests`, 4 in `mc-client/tests`). Each becomes
  `seat(…).simulation`.
- **A return-type change reaches type positions no call-site grep sees.**
  `crates/mc-sim/tests/support/launch.rs` alone carries 10
  `Result<Simulation, LaunchError>` positions (`:183, 198, 212, 227, 253, 286,
  302, 318` ×2, `:323`), and
  `crates/mc-client/tests/support/persistence.rs:59` is a `pub type Launched =
  Result<(Simulation, BlockName), PreparationError>` alias. Not one of these is
  a call site.
- **A file can need editing without naming any of the four functions or a
  `Result<Simulation, _>`.** Four `mc-client` reload tests destructure the
  `Launched` alias (`tests/support/reload_save.rs:186`) and pass the simulation
  on to a local `playing_client(simulation: Simulation, …)` — the helper's
  signature is unaffected and the *call* changes:
  `reload_keeps_the_player.rs:198` and `:206`, `reload_keeps_the_world.rs:194`,
  `reload_mutation_rules.rs:154`, `reload_solidity.rs:155`. A fifth,
  `saved_changes_need_no_edit.rs:265`, holds a `simulation: Simulation` field.
- **43 test files name `Simulation`, `simulation_for`, `simulation_at_launch`,
  `simulation_to_play` or `PreparedLaunch`, and each must be inspected. The
  true edit surface is ≥37 files and ≥46 sites.** How many actually change is
  not knowable by grep: a `&Simulation` parameter is unaffected while a
  `Result<Simulation, _>` is not, and
  `crates/mc-client/tests/seam_boundaries.rs:148` names `Simulation` only as a
  guard needle — its `WINDOW_FACING_GUARD` is scoped `path != "src/events.rs"`
  (`:142-153`) so it should be unaffected, but it is exactly the kind of thing
  that reddens inside a window where nothing reports, so read it before
  committing B.

**`PreparedLaunch`'s new field costs the tests nothing, contrary to the obvious
expectation.** No test file constructs a `PreparedLaunch` — the only literal in
the workspace is `crates/mc-client/src/launch.rs:222`. The five test files
naming the type do so in type positions only
(`launch_and_capture_agree.rs`, `launch_builds_only_the_world_it_needs.rs`,
`launch_renders_the_saved_world.rs`, `saved_changes_need_no_edit.rs`,
`saved_world_texture_layers.rs`), and adding a field breaks none of them.
`tests/edit_geometry.rs:301` looks like a counter-example and is not: it builds
a local `Handed` struct that happens to have a `simulation` field.

**Nothing else in the workspace constructs a simulation.** `mc-testkit` does
not; `mc-server` is a stub with an empty `[dependencies]`.

## Phases, and the window where the gate is blind

Three phases. The order is forced: the client cannot say what entry did before
the verdict reaches it.

### Phase 1 — one door, and the whole of FR-1

Scenarios: FR-1.3-S1, S2; FR-1.1-S1..S7; FR-1.2-S1..S3.

The door and the search land together. Splitting them would make a later
phase's scenarios green on arrival, which is worse than a large phase.

1. **Test author, commit A** — `one_way_seats_a_player.rs` and its three
   controls, plus the FR-1.1 and FR-1.2 fixtures. The scan compiles and runs
   against today's tree and is **red for the right reason**:
   `AnotherSourceSeatsAPlayer([persistence.rs, replay/spawn.rs])`. Displayed
   before anything else happens.

   **The FR-1.1 and FR-1.2 fixtures in this commit are written against today's
   signatures** — `Result<Simulation, LaunchError>` and
   `Result<Simulation, SpawnError>` — and are adapted in commit B with
   everything else. Their RED here is behavioural: the player is not cleared.
   Written against `Seated` they would not compile, and the window would be two
   commits rather than one.
2. **Test author, commit B** — the adaptation, across the ≥37 files the
   return-type change reaches. **From this commit until step 3 lands the tree
   does not compile.** Nothing in commit B requires judgement; it is the four
   mechanical shapes listed in Assumptions.
3. **Implementer, commit C** — the door, with two deliberately wrong bodies run
   in sequence before the right one, per testing.md §2:
   - a **do-nothing** skeleton (`Clearing::Unneeded` always, no search) — this
     is the first tree that compiles, and it reddens FR-1.1-S1, S3, S5, S7 and
     FR-1.2-S1..S3 on their *assertions* rather than on a compile error;
   - an **over-eager** skeleton (always `MovedTo(centre_of)`) — the do-nothing
     one passes FR-1.1-S2, S4 and S6 vacuously, and only the over-eager one
     reddens them.

   Both RED outputs are displayed. Then the real body, and the whole suite plus
   the gate go green.

**The blind window is commit B, and it is one commit wide.** The alternative —
add `seat` beside a still-public `Simulation::new`, migrate, then demote — keeps
every commit compilable. It is rejected on two grounds:

- The zero-window route needs `simulation_for`, `simulation_at_launch` and
  `simulation_to_play` to exist in **both** return shapes at once, since a
  return type cannot be migrated in place. That is three duplicated public
  functions with duplicated documentation, and the migration commit still has to
  touch every affected file — the window is traded for a larger diff and a
  temporarily doubled public surface, not for less work.
- The FR-1.3 scan then sits red across three commits instead of one, and
  testing.md §2 names red-for-a-known-reason as the state in which a test stops
  reporting anything new.

**One argument for the one-window route is *not* available, and is recorded so
nobody reconstructs it:** "two public seating doors would end up in `main`" is
false. git-workflow §3 mandates a squash merge, so `main` receives one commit
either way and the doubled surface would never appear there. What the branch
history carries is a discipline question, not a `main` question. The
recommendation rests on the two points above alone; a reader who weighs them
differently is weighing something real.

**Who runs clippy, and when.** The gate has no compilable tree inside the
window, so:

- **Commit B is verified before it is committed, not after.** The test author
  applies a throwaway local `seat`/`Seated` stub to `simulation.rs`,
  `persistence.rs` and `replay/spawn.rs`, runs
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` and
  the full suite against the adapted tests, then reverts the stub **by hand** —
  never `git checkout --`, per git-workflow §2 — and confirms
  `git diff --exit-code` over `crates/*/src` is clean before staging the test
  paths explicitly. Nothing of the stub is committed, so the test-ownership
  rule is intact. What it buys is that the ≥37 adapted files are compiled and
  linted before they enter the one commit nobody can check, which removes the
  round trip budgeted below rather than merely planning for it. **This is the
  single thing that makes the window acceptable**, and skipping it turns a
  bounded risk into an unbounded one.
- Before committing B, the **test author** also runs
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` on the
  *pre-window* tree, so the phase does not open carrying an unrelated lint that
  will later be blamed on the adaptation.
- At the first moment commit C's tree compiles — before committing it — the
  **implementer** runs that same command in full, over the real door rather
  than the stub.
- Any finding **inside a test file** goes back to the test author; the
  implementation context does not edit test files, and a lint is not an
  exception to that. With the stub check above this should be empty; without
  it, budget a round trip.
- `-D warnings` and not a lower severity: without it cargo attributes the
  diagnostic to the first binary and marks the rest `(1 duplicate)`, which means
  *this same diagnostic repeated*, not *a pre-existing one elsewhere*.

### Phase 2 — the two sentences

Scenarios: FR-2.1-S1..S6.

`notice.rs`, its consts, the four functions, the two call sites, and
`the_entry_sentence_is_said_once.rs` with its three controls. The scan is a file
walk, so it compiles and runs before `notice.rs` exists and is red as
`ComposedButNeverSaid`. Two skeletons again: `None` always reddens S1, S3 and
S4; `Some(…)` always reddens S2.

No adaptation window — nothing existing changes signature.

### Phase 3 — documentation (Key Principle 3, part of done)

- **Player** — `docs/user/gameplay.md`: resuming a save now places you somewhere
  you can move even when a block became solid while the game was off; the exact
  line on the terminal; what happens when nothing within 8 blocks is clear.
- **Mod author** — `docs/modding/hot-reload.md`: a solidity change made offline
  is answered at the next entry exactly as one made live is answered at the swap,
  with both wordings quoted side by side; the same "only ground the world
  actually holds counts as clear" rule applies at entry.
- **Engine reader** — `docs/technical/architecture.md`: the seating door, why it
  is the constructor and not the launch path, what the compiler holds (D1) and
  what it does not, the factored rule and how a join inherits it (D2), why the
  search is unconditional (D3), where the verdict is parked (D4).
  `docs/technical/testing.md`: the entry-door fixtures, the compose/write split,
  both scans with their verdicts and controls, why a needle may not be a whole
  sentence (D6), and the mutations recorded against them.
  Three passages in `docs/technical/architecture.md` go stale and must be
  corrected in the same pass: the `simulation_for` signature at `:295`, the
  `simulation_at_launch` signature at `:647` and the launch ordering at `:670`,
  and the count and list of non-fatal notices at `:1607`, which names four
  (`app.rs`'s dropped-frame, unshowable-edit and swatch notices, and
  `events.rs`'s cursor-release notice) and does not yet include the reload's
  clearing notices at all. This spec adds the entry notice and moves both
  clearing notices into `notice.rs`. The observation that these bypass the
  reporting sink stays deferred — the count and the list do not.

The spec folder is archived and pruned at 365 days; none of the above may be
left to it.

## Assumptions

- **One local player, one entry per process run.** Stated in the spec. D4's
  once-ness rests on it. A second local player would need a second entry and
  this design would need revisiting — which is MVP 3's problem, and D2 is what
  makes it a small one.
- **`mc-server` stays a stub for the duration.** Composition root is
  `mc-client`. PRO-944 moves it and is not started.
- **Every adapted fixture places a clear player**, so each receives
  `Clearing::Unneeded` and no fixture's behaviour changes. Asserted by the spec
  and by the fixtures' own construction; **if any fixture turns out to place a
  trapped player deliberately, stop and raise it rather than adapting it inside
  the window** — that one would change behaviour.
- **The adaptation is mechanical in four shapes, not one.** `.simulation` at a
  `Simulation::new` call; a helper signature
  (`mc-sim/tests/support/launch.rs:183` and eight more); a type alias
  (`mc-client/tests/support/reload_save.rs:186`); and a borrow into the result
  (`support/launch.rs:318`, which becomes `&seated.simulation` — a body change
  rather than a call-shape change). All four need no judgement; none of them
  needs a decision made inside the window.
- **`Display` on `f32` is what FR-2.1-S1's expected text was written against.**
  `(12.5, 10.0, 12.5)` renders `(12.5, 10, 12.5)`. Inherited from the reload's
  existing print, not chosen here.
- **rustfmt's defaults hold** (no `rustfmt.toml`): `format_strings` off, so a
  string literal is never reflowed, and a `const` declaration puts its literal
  on a line of its own. D6's needles depend on this.

## Risks

| Risk | What to verify, and when |
|---|---|
| **The goldens move.** An entry check that cell-centres or grounds the derived spawn changes every committed frame. | FR-1.1-S4 asserts the derived spawn exactly and that nobody is moved. It is the first assertion to run against phase 1's over-eager skeleton, and the golden suites are the second witness. |
| **FR-1.1-S1 or S6 weakened.** They are each other's control through the extent argument, and both are cheap to soften into "covers no solid cell" or into an in-memory fixture — which takes them off the only path where they can fail. | D2's closing paragraph states what each must assert. Check it at test review, not at implementation. |
| **A future join supplies the wrong ground.** D2(b) leaves the extent an argument, so MVP 3's join can pass the wrong one; FR-1.3's scan sees a missing rule, not a wrong argument. | Recorded in D2 as owed a scenario by whichever spec adds the join. Not buildable here — there is no join to grade. |
| **A needle that cannot match.** A scan whose needle never matches passes forever, and D6 documents two spellings that look right and cannot match. | Every needle in D5 and D6 is listed with its count in today's tree. Each control must be seen to *fail* the scan before the phase closes — that is what the fixtures are for. |
| **The blind window widens.** A commit-B adaptation that needs judgement, or a phase-1 door that takes several attempts, leaves the tree uncompilable for longer than one commit. | Commit B is mechanical by construction (see Assumptions). If a fixture is found to place a trapped player, stop and raise it. |
| **A clippy finding lands in a test file after the window closes.** The implementer cannot fix it. | Named in the phase-1 procedure: it goes back to the test author. |
| **Reading the player at the wrong tick.** A cleared player arrives with `on_ground: false` and settles on tick 1, so a test that reads tick 1 rather than tick 0 sees a different `y`, reddens against a correct implementation, and is cheapest to green by grounding the player — which Out of Scope forbids. | FR-1.1-S7 is about the *first* snapshot. See the Data section; check it at test review. |
| **An over-tight assertion on the destination.** `centre_of` returns `(x + 0.5, y, z + 0.5)` (`clearing.rs:120-127`) — horizontally centred, feet on the cell *floor*. A test author deriving `y + 0.5` from "at that cell's centre" goes red against a **correct** implementation, and the cheapest green is to edit the search, which Out of Scope forbids. | Already caught once in the scenario audit. Restated because the trap is reachable from the spec text alone. |
| **A scan that can no longer look.** Both instruments are file walks over paths a refactor can move. | Both verdicts are total and enumerated, and both carry a "could not look" answer with its own control (`NoSourceWasRead`, `TheDoorNoLongerSeatsAPlayer`, `ComposedButNeverSaid`). An absence assertion would have gone green forever. |
| **Vendor-failure blast radius.** None. No vendor, no network, no nondeterministic source is added or reached. | — |
