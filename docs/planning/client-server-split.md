# The client/server split: what the client is allowed to evaluate

**Elaborates**: PRO-917 (blocks defined in Luau). Bears on PRO-918 (hot reload),
PRO-904 (the solid/drawn/occludes split), PRO-902 and PRO-914 (texture resolution
and per-face keys), PRO-893 and PRO-894 (the HUD content format and fonts),
PRO-919 (components attach behaviour), and on MVP 4's transport and content
streaming. Proposes one spec that does not yet exist: moving the composition
root.

**Dated 2026-08-16.** Every in-tree observation below was read at that date; line
numbers drift and the claims should be re-checked rather than trusted if this
document is being read much later.

Forward-looking, per this folder's rule. Nothing here describes built behaviour
and the spec that lands wins.

**Partly landed, 2026-08-17 (SPEC-016).** This document is kept whole rather than
trimmed, because the as-built record defers to it by name for the reasoning it
does not re-derive (`technical/architecture.md` §"The simulation loads content;
the client receives it resolved"). What has been *built* is the seam and nothing
else: the simulation reads the content root and the client receives a
`ResolvedContent` carrying names, texture keys, solidity and a layer assignment
it honours rather than derives; the client's own sources name none of the four
content doors, watched by a scan. **Everything else here remains a proposal** —
moving the composition root and restoring the dependency-closure guard, transport
and the wire format, the content-addressed cache, client-side script evaluation of
any kind, and a content-set identity or hash. Where this document and the as-built
pages disagree about what exists, the as-built pages are right.

---

## The question

MyCraft is a 32-player authoritative multiplayer game whose entire content layer
is Luau. Two things follow that were never written down together: **where content
is evaluated**, and **what a client is allowed to work out for itself**. Until
now the answer was accidental — MVP 1 built singleplayer as one process that does
everything, so the client reads content off disk because nothing stopped it.

The immediate trigger was a collision. PRO-917 makes block definitions Luau
chunks, and the crate that loads them gained a dependency on the scripting host.
`mc-client` depends on that crate, so the host arrived in the client's resolved
dependency closure — tripping a guard written for MVP 1 asserting it never could.
The guard was briefly retired, on the reasoning that the client must evaluate
Luau to know what blocks exist. **That reasoning was wrong** and the retirement is
being reversed; the client needs the *resolved definitions*, never the evaluator.

## The answer

> **The client never evaluates anything any other participant, the server
> included, must agree with.**
>
> Passing that test makes client evaluation *permissible*, not *obligatory* —
> performance and isolation rules still bind independently.

Both halves of the second sentence earn their place, and the reasons are below.

### Why "agree with" rather than "own"

Ownership is a fact about who wrote a file, and the engine cannot check it. Worse,
an ownership-phrased rule forecloses cases that should be allowed: a mod's
particle effect is server-authored and seen by one player, and there is no reason
to ban it.

Agreement is checkable and it is the property that actually matters.

### Three tiers, and the middle one is the dangerous one

| Tier | Definition | Who decides | Failure if wrong |
|------|-----------|-------------|------------------|
| **Authoritative** | Another player observes the effect | Server only; a client claim is a request | Cheating, desync, duplication |
| **Consensual** | Every participant must hold the *same* value, but nobody decides anything at runtime | Server dictates; the client receives and may not derive | **Silent, total corruption** |
| **Local** | A client may be wrong alone with no consequence to anyone | The client, freely | A cosmetic glitch on one machine |

**The consensual tier is in nobody's mental model, and it is where the danger
is.** "How much do we trust the client" has an answer for the other two and none
at all for this one, because **trust is not the variable there — agreement is.** A
perfectly honest client that computed its own content set would corrupt the world
exactly as thoroughly as a hostile one.

That gives a placement rule for every field this project will ever add: **ask what
another player observes, then ask whether disagreement would be detectable.**
Undetectable disagreement is consensual and must be shipped, never derived.

### Apply the test to the input, never to the appearance

Cosmetic things acquire meaning. A particle effect is cosmetic; a particle effect
that tells you a furnace is lit is *information*, and two players disagreeing
about whether smoke shows is a disagreement about what the world is doing — with
no code having changed.

So the durable cut is by what a thing is derived *from*:

- **Derived from state the server owns, shown only to you** — particles, a
  progress bar, HUD arithmetic. If these are ever scripted client-side, faults
  must travel back to the server and limits must be replicated from it.
- **Derived from nothing the server owns** — texture packs, UI skins. No sandbox
  question arises at all.

A cut made on appearance is not stable when meaning shifts. This one is.

### The test under-constrains, deliberately

Neighbour-dependent block appearance — a fence choosing a connected texture —
*passes* the agreement test: two clients showing different variants is ugly, and
nobody's world state differs. It must still be refused, because it is per-face
evaluation in the mesher's inner loop and the scripting rules forbid calling into
Lua per-event in a hot loop.

**The agreement test is the right root and it is not the only gate.** Stated
without that caveat, the first person to cite it will cite it as permission.

## The separation that keeps appearing

Three times, in three unrelated places, the same shape:

| | The server decides | The client or player decides |
|---|---|---|
| Textures | *which key* a block names | *what pixels* the key resolves to |
| HUD | *what is shown* and what it is called | *how it looks* — anchor, size, offset, colour |
| Containers | *what is in the furnace*, capacity, validity | *how the interface is drawn* |

The first of these is already built and already documented: a block definition
names a namespaced texture key and **never a file path**, because what pixels a
key resolves to is the renderer's concern.

The third was surfaced late, correcting an argument this document originally
carried. Container UI was proposed as the one case that genuinely cannot be
expressed as data — a furnace interface drags items, shows progress, refuses
invalid moves, and nobody else observes it. **That framing is wrong.** A furnace's
contents are block state: two players with the same furnace open must see the
same items, and one removing an item must update the other's open interface.
Otherwise opening a furnace would have to lock it against everyone else. Only the
*presentation* is local.

A fourth instance appeared while the third was being tested: **a movement rule
versus its parameters.**

**It is one rule, and — this is the part that matters — it is an *independent
second rule*, not a consequence of the agreement test.**

> **Every content concept has an agreed half and a free half. Name both. The
> server owns the agreed half; the client owns the free half; and if you cannot
> name the two halves, the concept is wrong.**

The agreement test is a **classifier**: hand it a fact and it says which side that
fact belongs on. It says nothing about whether a *concept* can be divided at all.
The halves rule is a **constructive obligation**: it asserts that every content
concept *can* be split, and requires the author to perform the split.

**The proof that they are independent, and it is short.** Apply the agreement test
alone to a HUD: does anybody else need to agree about what my HUD looks like? No —
nobody else sees it. So the agreement test hands the player **the whole thing**,
including which value an element reads. That is not skinning; that is modifying the
game. The second rule is what stops it.

So the two rules answer different questions and are orthogonal:

- **The agreement test decides *where code runs*.** Consensus.
- **The declaration rule decides *who may change what*.** Authorship.

Crossed, they produce four cells, and two of them are the interesting ones:

| | Author-owned | Player-overridable |
|---|---|---|
| **Must be agreed** | block state, furnace contents, the texture *key*, which slots exist | — |
| **Needs no agreement** | bar arithmetic, format strings, particle behaviour | texture pixels, HUD colour, size and anchor, UI skins |

**The empty cell is meaningful.** Player overrides are structurally confined to the
no-agreement row — which *derives* "these are allowed client modifications, and
they are all cosmetic" rather than asserting it.

**The bottom-left cell is the one neither rule alone can find**, which is why it
kept moving during this discussion: it is author-owned, needs no agreement, and is
derived from server state. Particles, a progress bar, a formatted label. It takes
both axes to name it, and once named it is small — see below.

For an author, one sentence, anchored on the thing they already learned in their
first block file:

> **Declare meaning, never appearance — the same way `texture` is a key and never
> a file path. What a thing *is* is yours; what it *looks like* is the player's.**

The whole-category condition — an override boundary must cover a whole category,
never a per-field list — is better read as a **test of that sentence than as an
extra rule**: if a boundary ever needs a per-field table, the sentence has failed,
because an author cannot derive the table from it.

That difference has teeth. It converts "is this consensual or local?" — a
judgement call, re-argued every spec, which is exactly how a rule erodes — into
"name the two halves", which is a construction that either succeeds or visibly
fails. **A concept that resists splitting is the alarm, and it fires at design
time.**

Tested against the hard cases: a lockpicking minigame splits (agreed half "the
lock is open", free half the entire minigame). The furnace splits. A machine
interface that is a node graph you wire up splits — a wire is machine state, so
connecting two ports is an edit intent the server validates, and the client's job
is hit-testing and drawing a line to the cursor. **A grappling hook with its own
physics does not split**, and that is the case the next section is about. The rule
correctly refuses the one thing that is genuinely hard, which is the best evidence
available that it is a real rule rather than a pattern noticed three times.

### The primitive that dissolved three objections

Every case where the client appeared to need to **decide** turned out to need only
to **guess and be corrected**. Placement preview, refusing an invalid item move,
and predicting a break were all introduced as client behaviour and all withdrew
under the same treatment: the client predicts, the server decides, and a wrong
prediction shows the block failing to appear or the item snapping back.

A guess needs no authority and no agreement. It needs a rollback that is visible
and harmless — a cheaper primitive than either of the things that kept being
reached for.

## The one place the invariant is known to come under pressure

**Client-side prediction is, by definition, the client computing something the
server also computes, where the two must agree.** That is the divergence problem
restated as a feature, and it is the sharpest challenge to the rule above.

Today it is safe, because what gets predicted is engine-owned Rust with a declared
constants table. The client can predict by running the same Rust the server runs.

**It stops being safe when movement becomes content-defined**, and that is
scheduled: MVP 5 is "items, inventory, tools, crafting — all script-defined". A
pair of speed boots is fine, because a speed multiplier is a parameter and the
client predicts with engine rules and content parameters. **A grappling hook with
its own physics is not** — predicting it requires the client to run the content's
code, which is exactly what the invariant forbids.

Three options, and the choice arrives around MVP 4/5:

1. Movement rules stay engine-owned and content supplies only parameters. The
   invariant holds, at the cost of "the base game is a mod" being untrue of
   movement.
2. Scripts ship to the client for prediction — the consent residue, above.
3. No prediction for content-defined movement, which feels bad over anything but
   a LAN.

**The constraint that costs nothing to state now:** *anything the client must
predict has to be expressible as parameters to engine-owned rules, never as
content-owned code.* That is a design obligation on every future movement or
physics hook, and it is the same shape as "a pack replaces the image, never the
key".

This is a reason **for** the invariant rather than against it: the rule is what
makes the tension visible at design time instead of at the first grappling hook.

It should be read as a thing to watch rather than a thing that is known. It could
dissolve the way container UI did — if every content-defined movement turns out to
be parameterisable, option 1 costs nothing and the tension was imaginary. Nothing
in the tree predicts anything yet.

## What this makes permissible, and what it closes

Three ways a capability can reach a client, and they are genuinely different:

| | Automatic on join | Expressive | Consent problem |
|---|---|---|---|
| Server ships **data** | yes | limited | none |
| Server ships **script** | yes | yes | **yes** |
| Player installs **script** | no | yes | none |

The middle row's only unique property is *automatic*, and it is the only row with
a consent problem — the same fact stated twice.

**The consent argument, which is the whole reason the threat question is not
symmetric:** a player joins a server having audited none of its mods. A player
installs a pack having chosen it. Both run on the player's machine. Only one has a
consent problem.

This matters because the sandbox that exists today was built to protect a *server*
from a *mod*. Server-shipped client script would make it protect a *player* from a
*server operator* — a direction nobody has examined and for which the project's
"security is not a priority here" calibration does not apply, since that
calibration was about operators trusting mods.

**Proposal: close the middle row, consent-gated rather than welded.** If a server
ever ships client-side script, the client asks the player. Naming the mechanism
now costs a sentence; discovering it is needed after a content format ships costs
a format break.

Player-installed is not player-authored, so a downloaded pack is still somebody
else's code and wants some containment — but that is the player's sandbox
protecting the player, which is the browser-extension model with decades of prior
art.

## Verified in the tree, 2026-08-16

Read directly; these are observations, not inferences.

**Targeting needs no script and none is possible.** `crates/mc-sim/src/world/action/trace.rs`
— `targeted(origin, direction, reach, &dyn Solidity) -> Option<Hit>` is a pure ray
walk returning the cell, **the face the ray entered through**, and the distance.
It consults exactly one thing about the world: whether a cell is solid.
Face-level targeting needs geometry, a reach constant, and one bool per block.

**The client's render path reads two of six definition fields.** `texture`, via
the registry's texture-key set, and `is_solid`, for face culling in the mesher.
Not `replaceable`, `breakable` or `breaks_into` — which are exactly the
world-mutation rules the server recomputes.

**The texture layer table is a projection of block definitions and of nothing
else.** The key set is a map over registered definitions; the layer index is a
key's position in that lexicographically sorted set. **A texture pack replacing
images therefore cannot reach it**, because a key only comes into existence by a
block definition naming it. A pack supplying an image for a key no block names
would occupy no layer and never be sampled; a key with no image is refused loudly
rather than drawn as layer zero.

**There is no image behind a key today.** Texels are generated from the key
itself, 16×16, in the GPU layer — same key, same texels, forever. The key *is* the
image. Real art has not landed.

> **Landed, 2026-08-20 (SPEC-019).** The paragraph above is false as of that spec
> and is kept because the reasoning after it does not depend on it. A key the
> content root's built set covers now draws a PNG baked from a voxel model, read at
> launch; a key nothing has baked still gets the generated texels described above,
> so the fallback is per key rather than universal. Which key a face draws also
> comes out of the block's declaration rather than its name, per facing — see
> `technical/rendering.md` §"A face draws what its block declared, and a `Quad`
> carries no key".

**A layer index rides inside every packed vertex.** This is the sharpest fact in
this document. **Insert one block the server does not have, and every layer index
after it shifts by one — the entire world is textured wrong, silently, with no
error anywhere.** The disagreement is not localised to the disputed block.

That is not hypothetical and it is not about networking: **it is a live defect on
hot reload today, in one process.** Adding a block renumbers every index already
on the GPU.

## What must be decided early because it is expensive to retrofit

Four items. Everything else about client-side content can wait, and deferring it
costs nothing because no data is authored against it meanwhile.

**1. The server ships an explicit layer assignment; the client honours it rather
than deriving it.** Key-to-index pairs, appended and never renumbered within a
session. Under derivation, adding one block forces a full world re-mesh and
re-upload, which would blow the sub-second hot-reload target; under an appended
assignment, existing vertices stay valid. It is falsifiable today: hand the client
an assignment that is deliberately *not* the positional one and assert the
rendered indices follow it — a test that cannot be satisfied by two copies of the
same sort agreeing with each other.

**2. The layer table is a projection of block definitions, never of available
images.** When real art arrives, the natural-looking implementation is "enumerate
the textures directory and build the table from what is there". That single change
makes the key set a function of installed packs, every client numbers layers
differently, and the whole suite stays green while blocks wear each other's
textures on some machines. Worth pinning with a test that the layer resolver has
exactly one call site and that site is fed the registry's key set.

**3. A pack may replace the image behind a key. It may never add, remove or rename
one.** "My pack adds a fancier variant" is the most natural first request a pack
author will make, and granting it once diverges every client silently.

**4. All block textures are one engine-declared size.** An array texture's layers
are all one size. If a pack may supply its own dimensions, that means one array
per size, multiple bind groups and a shader change; if the engine declares the
size and a pack either matches or is refused, nothing changes structurally.

Two smaller ones worth doing when the adjacent contract is next touched: **a golden
frame should declare which texture source it was shot against** (every golden today
is shot through the generated placeholders, and the moment the source is pluggable
a golden silently becomes a test of whatever is installed); and **the HUD's fields
should be classified as semantic or presentational** in the same commit as
whatever next changes that contract, because an override boundary discovered later
has to break declarations that already mix the two.

**Deliberately not on this list**, having been considered and dropped: a
key-to-texels port (one call site, so introducing the indirection later costs the
same as now); pack load order and stacking (nothing exists to order); an override
namespace (keys are already namespaced).

## Consequences for the dependency guard

A guard existed asserting that `mc-client`'s resolved closure excludes the
scripting host, in every dependency kind, with positive controls proving the walk
could see and that an absent crate is not reported as a clean exclusion.

**It cannot pass while one binary hosts both halves.** A binary's closure is the
union of the closures of everything inside it, and in singleplayer the client
binary *is* the server. Whichever crate loads content sits inside it. The only
arrangement in which that test passes today is the one where the client sources
content itself — **a guard green exactly when the rule is broken is inverted, not
weak.**

The binding constraint is not crate topology, though. It is that `mc-client` is
the composition root, and that is a choice rather than a law. Move the root and
the guard becomes reachable in one process:

| Crate | Change |
|---|---|
| `mc-sim` | gains the Luau loader and the scripting host |
| `mc-world` | drops the scripting host |
| `mc-client` | drops `mc-sim`; becomes a pure client library |
| `mc-server` | becomes the composition root and the singleplayer host |

No eleventh crate — `mc-server` is currently an empty stub whose own comment
already describes it as the headless authoritative simulation.

**The price is real.** Roughly forty test binaries live in `crates/mc-client/tests/`
precisely because the renderer and the simulation may not resolve each other, and
that home disappears. The published startup path the golden frames are shot
through moves with them. `cargo run -p mc-client` stops being how the game
launches, which is what the modding walkthrough tells a new author to type.

**Sequencing: this is not part of PRO-917.** That spec's exit criterion is "the
world renders identically" and the golden suites are the instrument that decides
it; moving the instrument and the subject in one spec is the verification-first
invariant exactly. PRO-917 should finish with the loader where it is and with the
simulation, not the client, constructing it. **A later spec moves the composition
root, and the restored closure test is that spec's exit criterion** — a better job
for it than guarding.

Until then the property is carried by a source scan, which is the weaker
instrument and should be recorded as such. **Its needles should name chokepoints
rather than type names** — the registry's apply call, the HUD layout loader, the
registry constructor, and the content-root resolver. Those are stable against a
future source kind being renamed, because renaming a source does not rename the
door. The tree documents that the apply call is the only way to populate a
registry at all, and that the layout loader is the only door into a layout.

**Known residual, which belongs in the guard's own doc comment:** somebody adding
a *second* door — a new public registration call — bypasses it, and no text scan
closes that.

## What this does to PRO-917

An amendment rather than a re-spec, and the implemented work survives. Most of its
64 scenarios say nothing about which side loads content; a handful are worded
"when the client loads" and need rewording without needing different tests. What
is added: the resolved content set carries the render and prediction fields and
not the mutation rules; the client's content view is constructible from that set
alone, in a test that builds no registry, opens no path and constructs no host —
which is the single assertion distinguishing a seam from a rename.

**Not added in MVP 2: a content-set identity or hash.** With one process nothing
can disagree, so nothing can falsify it, and a test that cannot fail reads as
evidence and is not. The reasoning is recorded here so the later spec inherits the
*reason* rather than rediscovering it; the field is small whenever it is wanted.

Worth reserving cheaply: a locality field on component declarations, accepting
only the server value and refusing the client value by name. One field and one
refusal test, free to remove (nothing was ever authored against the other value)
and free to keep — which makes it the dominant move while the question is open.

## Open questions

Recorded as questions, per this folder's rule.

**Does the accretion risk survive the split?** It converts rather than vanishing,
and into something better. A declarative HUD was argued to need five sub-languages
— a format-string dialect, conditional visibility, a numeric mapping, a
threshold/gradient colour spec, and a layout engine — which together are a
badly-specified programming language. Under the two-axis split, **two of the five
leave**: threshold colours and layout are presentation, so they become skinning,
where stops-and-colours and layout are genuinely declarative and contain no
arithmetic. What remains is one expression form, one format string, one
comparison. **Three primitives is a schema, not a language.**

The pressure moves instead onto **which scalars the server exposes.** Today the
readable-value set is exactly one entry. If authors want health, hunger, furnace
progress, slot counts, that list grows — and a prediction of sixty entries is
plausible. But **a namespace of named values is enumerable, documentable and
versionable in a way a grown language is not.** "We have a large but honest
catalogue" is an acceptable failure; "we built a programming language without
deciding to" is not.

**This analysis went further than the decision needed, and the excess should not
be mistaken for a design.** It mattered for one thing only — whether to keep a
client-side scripting door open — and once that closed, none of the conclusion
rests on it. **UI is a solved shape and should be treated as one when its time
comes: a reactive interface, meaning a declarative view bound to state and
re-rendered when the state changes.** Several engines embed an HTML/CSS runtime
for exactly this. Whatever lands here should start from that prior art rather
than from a set of formats invented one requirement at a time — which is the
failure this document warns about elsewhere, and which the reasoning above was
drifting toward. It is an MVP 5-plus question and nothing before then depends on
it.

**One thing the decomposition hides, which needs naming rather than solving:**
optimistic display and correction. When a player drags an item, does the interface
show it moved immediately or wait for the server? That is neither presentation nor
server rule — it is *sequencing*, and a naive skinning model has no way to express
"show this now, reconcile if the server disagrees". The tree already carries the
pattern, since action intents carry no position and the server re-derives and
refuses. It is the one place presentation has to know about time.

**None of this is buildable yet, and that should be said rather than implied.** A
furnace's contents are per-cell state, which is deferred along with the two specs
that depend on it. This is a contract being settled for something that cannot
exist for at least two more increments.

**Can `mc-client` shed `mc-sim` cleanly?** Several client files name simulation
types, and some of them — snapshots, movement and action intents — are vocabulary
a *pure* client legitimately needs. If that vocabulary cannot be separated from the
authority half, the composition-root move needs an eleventh crate and its price
rises. This is an afternoon of reading and it should happen before anyone commits
to the arrangement.

**Is a layer index the only registry-derived value riding inside a vertex?** The
geometry builder carries a comment suggesting it may resolve texture keys by block
name rather than through the layer table, and the architecture record flags that
as a known MVP 2 gap. If a second registry-derived value packs into a vertex, the
texture-pack guarantee above needs re-checking against it.

**When does the commitment get revisited?** Two observables, watching different
tiers, and both are wanted because neither sees the other's case:

- *The second time* a client-facing declarative format is designed containing a
  conditional or an arithmetic expression. One is a format; two is an interpreter
  built twice. This catches HUD and container-UI convergence.
- *A mod shipping a family of block registrations that differ only in appearance* —
  fifteen fence variants for one conceptual fence. That workaround contains no
  conditional anywhere, so the first observable cannot see it. This catches
  neighbour-dependent appearance.

## What would make this a mistake

Not that something could not be expressed — that it was expressed five times,
badly. A connection-rule format, an animation state-machine format, a
particle-emitter format, a HUD expression syntax and a container-UI format: none
versioned together, none sandboxed because none is "code", none budgeted for the
same reason, and at least one growing conditionals and then a loop and becoming
accidentally Turing-complete without anyone deciding it should. A mod author
learning five syntaxes instead of the one language they already write blocks in,
and every client-side capability gated on the engine team enumerating it first —
which is the exact inversion of "the base game is a mod, with no privileges a
third-party mod lacks".

The counter-pressure, and the reason the player-owned surface matters beyond
skinning: **every expressiveness request lands on the declarative format when
there is nowhere else for it to go.** A player-owned pack surface is what lets the
server-owned declarative format stay small.

## An aside worth more than the question that produced it

What actually blocks expressive blocks is not where script runs. It is **per-cell
state** — a furnace with contents, a stair with a facing, a door with an open flag.
That work is deferred (PRO-911, with PRO-912 and PRO-913 deferred on it), and
nothing in this document unblocks it.

## Documents that contradict this and need correcting

- The architecture record currently carries a paragraph asserting that a client
  which cannot reach the scripting host cannot draw the world. That is a
  non-sequitur — the client needs resolved definitions, not the evaluator — and it
  reads as an as-built record, which makes it more dangerous than the deleted
  guard, because a future spec author would follow it.
- The HUD modding guide tells an author the HUD becomes Luau in MVP 2, while the
  roadmap lists the HUD content format among MVP 2's deferred items.
- The roadmap says content streams to players "not just scripts", and separately
  that base content ships with the client and is therefore already cached. The
  second is safe **only** because the cache is hash-keyed: a client cannot serve
  itself content without being told which hash the world is on. A name-keyed
  lookup, or a direct disk read added to "optimise" singleplayer, reopens the
  consensus hole — and it will arrive inside a spec about startup latency, look
  entirely sensible in review, and nobody will connect it to consensus.
