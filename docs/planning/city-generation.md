# City generation: feasibility discussion (PRO-935)

> **Planning document** — not as-built documentation. Elaborates
> [PRO-935 "City generation: a partition, not a biome"](https://linear.app/prodigy-solutions/issue/PRO-935/city-generation-a-partition-not-a-biome).
> Written 2026-08-15 against the tree at `c10fc99`. The game is young: there is
> no worldgen, no light engine, no wire format, and everything here is subject
> to change. Statements about the tree were verified against it; statements
> about external systems carry their sources; everything else is derivation and
> is labelled as such.

This document does four things: assesses the feasibility of PRO-935's design
as a whole; collects its mathematical constraints, loopholes and edge cases —
including several the issue does not name; answers the three forward questions
(infinitely tall buildings, layered "cloud street" networks with transit, and
lighting); and ends with the pitfalls table and the open questions that should
survive into any spec.

---

## 1. The design in one page

PRO-935's core commitments, compressed for a reader who has not read the issue:

- A city is a **partition**, not a biome: `p → (cell, local frame, params)`
  rather than `p → params`. The partition evaluator sits *outside* the scalar
  density graph and feeds it scalar projections only.
- The partition signature is **`(p, layer)` from day one** — a stack of 2D
  partitions at different heights, not one partition extruded.
- Top-level structure: originally Voronoi districts + BSP lots; **revised to a
  single infinite quadtree** over the integer plane, warped by a bounded
  displacement (`descend(warp(p))`), because the quadtree gives every pair of
  points a common ancestor computable in O(1) — which is what makes any
  cross-lot agreement designatable at an ancestor and inherited down.
- **Articulation never queries a neighbour, only ancestors.** Cross-lot
  features (skybridges, cornices, party walls) are decided at the common
  ancestor or by ownership convention, enforced mechanically by a partition
  handle that hard-errors on out-of-lot queries.
- Buildings are a **vertical jigsaw of floor-plate templates** (`.mcvox`
  models), not a shape grammar; massing is a density term (`solid where
  y < tower_height(cell)`), articulation is templates.
- Cost tiers: invariants preserved by every split are free (Tier 0);
  bounded designation at an ancestor is O(k·depth) (Tier 1); district-scope
  aggregates cost one memoized enumeration of leaves (Tier 2); unbounded scope
  and rejection-with-feedback are prohibited (Tier 3).
- The working scale is **32–64-block-wide buildings, ~120 tall**, which
  collapses the earlier stress-case numbers: ~3:1 interior:exterior faces,
  ~8× terrain generation cost, no rendering spec needed first.
- One shipped precedent (The Lost Cities) confirms order-independence, bounded
  read radius, hashed sites, and per-floor part assembly — and avoided exactly
  the three hard parts: lot subdivision, height beyond ~30 blocks, and layered
  fabric.
- Before any of it: **hand-build one block, measure per column, render at
  night with faked emissive**; and spike the cheap anchor-lattice alternative
  to see whether quantised lot sizes read acceptably.

## 2. Feasibility assessment

**The design is feasible, and its self-corrections are the strongest evidence
of that.** The issue already caught its own worst numbers (the 16:1 interior
ratio, the 100–250× cost multiplier, the "rendering spec first" alarm) and
re-derived them at the real scale to ~3:1 and ~8×. Those revised numbers are
consistent with what the tree can already do: the mesher runs at ~136 µs per
terrain section (`product/roadmap.md`, measured), so even at several times
terrain cost per section, meshing a 12-chunk-view city working set is
hundreds of milliseconds of parallelisable CPU work, not a wall. The load-bearing
architectural moves — partition outside the density graph, scalars-only across
the boundary, ancestors-not-neighbours, declared read radius, declared minimum
lot size — are all either shipped practice somewhere (Lost Cities) or standard
arithmetic (quadtree LCA).

The honest risk ranking, roughly in decreasing order:

1. **Lighting** — in a genre whose signature aesthetic (neon night city) is
   made of light, and named by the issue as the first thing that breaks the
   architecture's purity property. Section 7 argues the threat is smaller
   and more specific than that: block light is a bounded-radius (15) pure
   fixpoint, fully compatible with the architecture; *skylight* is the
   genuinely non-local channel and the plan layer resolves it analytically.
   What keeps lighting at the top of this list is not architecture risk but
   coupling: three of its requirements land in specs that come first, and
   one decision (colour) is expensive to reverse after the formats freeze.
2. **Layered urban fabric** — genuinely unattempted anywhere (the issue's own
   research finding). Section 5 argues the reduction to "one more partition
   layer plus structural connectivity rules" is sound, but it is derived, not
   observed, and should be scoped as research.
3. **The warped LCA quadtree** — derived here, not shipped anywhere. Its
   failure modes are enumerable (Section 3/4) and none look fatal, but it has
   the most "unknown unknowns" per line of design.
4. **Vertical alignment / constraint-aware subdivision** — the issue calls it
   the highest-risk item; §3.4 proposes replacing rejection sampling with
   interval sampling, which removes the specific hazard named there.
5. **Everything else** (massing, jigsaw buildings, anchors, routes) — low
   risk; either precedented or plain arithmetic.

Two process observations. First, the issue's "measure before designing"
instruction is correct and cheap, and nothing in this document substitutes for
it — the hand-built block and the anchor-lattice spike remain the next
actions. Second, the four "cannot be deferred" decisions ((p, layer)
signature, no identities into the density graph, massing on the cell,
per-column evaluation context) are all *interface* decisions with near-zero
implementation cost today; they should simply be adopted. There is no scenario
where any of them is wrong-and-expensive, only scenarios where their absence
is expensive.

## 3. Mathematical constraints, collected and extended

The issue states several constraints; this section restates them precisely and
adds ones it does not name. Derived unless cited.

### 3.1 Identity width vs. f64 (stated in the issue, extended here)

Graph nodes evaluate to `f64`; integers above 2^53 are not representable, so a
64-bit cell id exposed as a scalar collides silently. The issue's rule —
identities never cross into the density graph — is right, and it has a second
edge the issue does not name: **Luau numbers are f64 too**. The scripting
boundary is therefore under the same constraint as the density graph. Any cell
or lot identity that crosses into a mod must cross as a string, a pair of
32-bit halves, or an opaque userdata handle — never as a Luau number. This
should be a stated rule of the worldgen API surface from v1, for exactly the
reason the issue gives: the failure is silent, distant, and testless.

### 3.2 Quadtree LCA arithmetic, signed coordinates, and the seam problem

The LCA level of two points is the index of the highest set bit of
`(x1 ^ x2) | (z1 ^ z2)` — standard and certain, **on unsigned integers**. World
coordinates are signed. In two's complement, any pair straddling 0 differs in
the sign bit, so their "LCA" is the root. The standard fix is to bias
coordinates (`u = (x as u32) ^ 0x8000_0000`, order-preserving), which is fine
— but it relocates rather than removes the underlying phenomenon:

> **The seam problem.** A quadtree's boundaries are not all equal. Two lots
> adjacent across a level-k boundary have their LCA at level k, and for large
> k that ancestor covers a 2^k-sized region. Adjacency in space does not imply
> adjacency in the tree.

How coarse, and how often? The arithmetic is worth doing. Two horizontally
adjacent columns straddling the line x = m have `(m−1) XOR m = 2^{t+1} − 1`
where t is the number of trailing zeros of m, so their LCA sits at level
t+1 and covers a cell of side 2^{t+1}. Over the integers, t is geometrically
distributed: half of all boundaries are as fine as possible, a quarter one
level coarser, and boundaries of coarseness 2^L occur with linear density
2^{−L} — **but each extends for 2^L blocks**. Frequency × extent is constant
per octave: every scale contributes the same total seam length per unit
area. So coarse seams are not a rare corner case that testing will
statistically miss *and* players will statistically hit — a traveller who has
walked distance D has, in expectation, crossed a seam whose correlated extent
is comparable to D, at every D. The artifact is self-similar; it grows with
exposure.

Two distinct consequences, one benign, one a genuine loophole:

- *Benign:* correctness and determinism are unaffected. Both sides compute the
  same LCA and the same hash; the descent is O(word size) at worst (~32
  levels), which is still O(1) per query. Nobody reads a neighbour.
- **Loophole — feature correlation along coarse seams.** If a cross-boundary
  feature (skybridge, shared cornice) is hashed *from the LCA node id alone*,
  then every boundary pair along that node's entire seam draws from the same
  hash — identical skybridges repeating down a 2-kilometre line, exactly at
  the world's most visible power-of-two boundaries, with the scaling law
  above guaranteeing every player eventually meets one. The fix is one rule:
  **boundary features hash from (LCA id, boundary segment coordinates), never
  from the node id alone.** Both sides can compute the segment coordinates
  without communication, so the property that matters survives; only the
  accidental correlation dies. This rule costs nothing and must be in the spec,
  because the failure is invisible until someone walks a seam.

A second seam consequence: **subdivision depth can differ across a coarse
boundary** (a wide lot is a node the descent stopped early at). Street
alignment across such a boundary is a T-junction problem. The warp does not
help — it is applied identically to both sides. Either the layer's street
datum is carried by the coarse levels (so both sides inherit it from ancestors
they *do* share — always true, since any two points share all levels above
their LCA), or district-level boundaries deliberately read as
discontinuities (arterials — the original Voronoi corollary, resurrected at
seams only). The first option is cleaner and appears free: street-bearing
levels are a fixed band of the hierarchy, and everything below inherits.

### 3.3 The warp, done properly — and a correction to the issue

Write the warp as `w(p) = p + d(p)` with displacement field `d`, and let
`L = sup‖∇d‖` (spectral norm of the displacement's Jacobian). The right
condition and its consequences are a classical theorem, not a heuristic:

**If L < 1, then w is a homeomorphism of the plane.** Injectivity is the
contraction estimate: `w(a) = w(b)` forces `‖a−b‖ = ‖d(b)−d(a)‖ ≤ L‖a−b‖`,
so a = b. Surjectivity is Banach fixpoint on `x ↦ y − d(x)`. So `L < 1` buys
strictly more than "no folds": no gaps either — every parameter cell has a
nonempty, connected world image, and the plane is tiled by warped lots with
no slivers and no overlaps. The Jacobian of w has singular values in
`[1−L, 1+L]`, so every length in a lot (street width, setback, frontage) is
distorted by at most that factor, and areas by at most `(1±L)²` — a warp
with L = 0.5 cannot make a 6-wide street narrower than 3 or wider than 9.
These are the *only* geometric guarantees articulation templates get about
the world-space shape of their lot, so L is a content constant templates are
entitled to read.

**Per-octave amplitude bound.** A displacement component of wavelength λ and
amplitude a has `‖∇d‖ ~ 2πa/λ`, so the no-fold budget forces
`a ≲ λ/2π` per octave, and the octave budgets **sum**: `Σᵢ 2πaᵢ/λᵢ < 1`
(with each basis function's gradient bound known analytically). At λ = 4096
the permitted amplitude is ~650 blocks; at λ = 32 it is ~5. That formalises
the issue's "coarse structure can be strongly organic, fine structure stays
near-rectilinear" — it is exactly the fractional-Brownian gradient budget,
and it is checkable at load by summing declared `aᵢ·gᵢ` terms, never by
sampling. A modder-authored warp with an unproven gradient bound is a fold
waiting for a seed that finds it; the bound is a content constant of the
same species as the declared finite-difference step and the read radius.

**Correction — range-query cost depends on L, not on amplitude.** The issue
charges a cost against the warp: "a warped box is not a box — it must be
inflated conservatively by the warp amplitude", so large amplitude inflates
per-chunk work. That is pessimistic by a large factor, and the fix is one
Lipschitz estimate. For any two points of a chunk box B,
`‖w(p)−w(q)‖ ≤ (1+L)‖p−q‖`, so **the image w(B) has diameter at most
(1+L)·diam(B) regardless of amplitude** — amplitude only *translates* the
image, it cannot spread it. Candidate lots for a chunk are therefore
gathered around `w(center(B))` (one warp evaluation) within radius
`(1+L)·8√2`, not within the amplitude: for a 16-block chunk and L < 1 the
candidate window is under ~46 blocks across, **even if the coarse octaves
displace by ten thousand blocks**. With minimum lot size `s_min`, candidate
count is bounded by `((1+L)·16/s_min + 2)²` — at `s_min = 8`, L = 0.5, that
is ≤ 25 lots per chunk, worst case. Consequence: the issue's stated tension
("warp amplitude bounded by the smallest cell size", two ceilings on one
constant) dissolves. **Amplitude is effectively free; the Lipschitz constant
is the one budgeted quantity, and it was already budgeted for injectivity.**
Strong organic coarse structure costs nothing at chunk-fill time.

### 3.4 Rejection sampling is avoidable, and should be avoided

The issue names constraint-aware floor subdivision (split positions
rejection-sampled against inherited forbidden regions) as the single
highest-risk determinism site: "a hash with rejection" degrades into
"deterministic given the same rejection order". The stronger move is to
**remove rejection rather than test it harder**: the forbidden set is a set of
intervals on the split axis; the allowed set is its complement — at most k+1
intervals for k forbidden regions. Draw one uniform value `u` from the node
hash, scale by total allowed length, and walk the allowed intervals to place
it (inverse-CDF sampling over a piecewise-uniform measure). This is exact,
O(k), rejection-free, and has **no order to get wrong** — the hazard class
disappears instead of being contained. Stated measure-theoretically:
rejection sampling *simulates* the conditional distribution given a
constraint; whenever the allowed set has computable measure — and a
complement of k intervals always does — the conditional distribution can be
sampled directly, and the simulation was never needed. The same holds for
slab cuts against rectangular forbidden regions (the allowed set is a union
of intervals along the cut axis). No constraint named anywhere in the issue
falls outside this. Rejection should be reserved for allowed sets with no
computable measure, the spec should say so, and if any survives it needs a
bounded attempt count with a deterministic fallback. The cache-disabled and
reverse-order tests remain pointed at this site either way.

### 3.5 Aggregate consistency across the memo boundary

Tier-2 aggregates ("40% residential", skyline silhouettes) are memoized per
district. The memo is an optimisation, and the issue's first test (memo
disabled ≡ memo enabled, bit-identical) covers it — but note the sharper
version of the requirement: the aggregate must be a **deterministic function
of the district's id alone**, evaluated by full descent of that district's
subtree, never incrementally updated as chunks generate. An incrementally
maintained aggregate is order-dependence with extra steps. This is implicit in
the issue; it should be explicit in the spec because "keep a running total" is
the natural first implementation of every aggregate.

### 3.6 Streets as level sets: the mathematics goes further than the issue takes it

The issue's three street-potential constraints (branch on `|∇φ| < g_min`
before dividing; finite-difference step as a declared constant; Morse
classification of the singular set) are correct as stated. Working the
mathematics further yields a cleaner construction and one new checkable
constraint.

**The two potentials should be one holomorphic function.** The issue authors
two independent scalar potentials `φ₁, φ₂` for the two street families and
notes the general-tensor-field integrability obstruction. But two
*independent* potentials carry no orthogonality guarantee — nothing stops
their level sets running near-parallel and producing sliver blocks. The
classical resolution: identify the ground plane with the complex plane via
`ζ = x + i·z`, take one holomorphic `W(ζ)`, and use `φ₁ = Re W`,
`φ₂ = Im W`. The Cauchy–Riemann equations give, wherever `W′(ζ) ≠ 0`:

- `∇φ₁ ⊥ ∇φ₂` — the two street families cross at right angles **by
  construction**, everywhere;
- `|∇φ₁| = |∇φ₂| = |W′|` — one shared gradient magnitude, so the issue's
  width normalisation `w·Δ/|∇φ|` is a **single** field for both families and
  both families' widths automatically match;
- the plaza predicate `|W′| < g_min` covers both families in one test, and
  the singular set is the zero set of a holomorphic function — isolated
  points, never curves, which kills a whole class of degenerate "smeared
  plaza" configurations that two independent potentials permit.

The Morse structure comes out automatically and better-behaved: at a simple
zero of `W′`, `W ≈ W(z₀) + c(z−z₀)²`, so both families form an X — a
four-way crossing; a zero of order k gives a star junction of valence
2(k+1). And the genre's radial centre is the canonical example:
`W(z) = A·log(z − c)` gives `Re W = A·ln r` (level sets: concentric ring
roads) and `Im W = A·θ` (level sets: radial avenues) — the ring-and-radial
downtown is *one basis function*.

**The new constraint — winding quantisation.** `Im(A·log)` is multivalued:
it grows by `2πA` per loop around the centre. The radial street family is
well-defined iff that period is an integer multiple of the street spacing Δ,
i.e. `A = nΔ/2π` — equivalently, **the number of radial avenues around each
centre must be an integer, and this is a load-checkable content constraint**.
With several centres `Σ Aⱼ log(z − cⱼ)`, each `Aⱼ` must quantise
independently (any closed loop's total period is then automatically an
integer multiple of Δ). An unquantised centre produces a spiral seam where
the street family tears — deterministic, stable, and wrong-looking. Same
species as the warp's gradient budget: an analytic property of the declared
basis, checked when content loads, impossible to detect reliably by
sampling.

Two footnotes to this construction. Conformal maps preserve angles, so
blocks stay locally square — the same "fine structure stays near-rectilinear"
property the warp's Lipschitz budget enforces, arrived at from the other
side. And `|W′|` for a log term decays like 1/r, so ring spacing widens
linearly with radius unless composed with other terms — a feature (peripheral
blocks are larger; true of real cities) that still wants a district-boundary
cutoff before spacing grows absurd.

One composition rule, unchanged from the draft of this section:
**level-set streets and partition streets must not both claim the same
layer.** `frac(φ/Δ)` streets are not edges of the partition; a lot whose BSP
says "street frontage here" and a potential whose level set says "street
there" will disagree somewhere. Level-set streets own specific layers
(organic districts), BSP/arterial streets own others, and a layer picks
exactly one mechanism.

### 3.7 Read radius, minimum lot size, maximum protrusion

Three constants of the same species, all needed, only two named in the issue:

- **Declared read radius** (issue): checkable by the
  chunk-by-chunk ≡ monolithic test.
- **Declared minimum lot size** (issue): bounds the range-query output per
  chunk.
- **Declared maximum protrusion** (new): the furthest any articulation
  (balcony, sign, skybridge deck) may extend beyond its lot's bounding box,
  vertically or horizontally. Without it, "which structures overlap this
  chunk" is unbounded even with a minimum lot size, because a lot far away
  could in principle reach in. With it, the candidate set for a chunk fill is
  the §3.3 Lipschitz window plus the protrusion — declared constants and a
  closed formula. Skylight seeding (Section 7) also needs exactly this bound.

### 3.8 The whole pipeline is a sliding block code, and read radii add

There is a name for the mathematical object this architecture keeps
gesturing at. A generator where chunk C's contents depend only on the seed
field within `C ⊕ B(R)` (the chunk dilated by the read radius) is a
**sliding block code** — a local map with window radius R, the object
symbolic dynamics studies. Three practical consequences fall out of taking
the definition seriously:

1. **Order-independence is not a property to test into the system; it is the
   definition.** A sliding block code has no evaluation order. The issue's
   reverse-order and monolithic-vs-chunked tests are exactly the tests that
   an implementation *is* the code it claims to be — they are the whole
   contract, not a heuristic pair.
2. **Radii compose additively.** If layer k reads layer k−1 within radius
   Rₖ (LayerProcGen's padded windows, §6.1), the composite map is itself a
   sliding block code with radius `Σ Rₖ` (each term scaled by its layer's
   lattice pitch). So the *total* read radius of a deep pipeline is a
   computable number, mechanically derivable from per-stage declarations —
   which is what makes "declared read radius must be checkable" scale
   beyond one stage. A pipeline whose stages declare radii has a checkable
   whole; a pipeline with one undeclared stage has none.
3. **Every feature request can be triaged against the window.** "Roads avoid
   the hill" asks the road layer to read terrain within an unbounded window
   (the road's whole course) — visible as a non-local map before anyone
   writes code. The issue's Tier 3 ("not virtual at any price") is precisely
   the class of maps that are not sliding block codes at any radius; naming
   the formalism gives the triage a one-line test.

### 3.9 Hashes are PRFs, and floats are not portable

Two foundations everything above stands on, neither named in the issue:

- **Every "hash" must behave as a pseudorandom function with domain
  separation.** The construction consumes randomness at (node, purpose)
  granularity — a split position here, a bridge verdict there — and any
  detectable correlation between related keys is a visible artifact (the
  ancestor chain differs between neighbouring lots in exactly one
  coordinate, so a weak coordinate mix bleeds structure straight into the
  street grid; Minecraft's weak per-chunk seeding is famous for lattice
  artifacts and for the community *recovering world seeds from structure
  positions*). Rule: one keyed 64-bit PRF (SplitMix64/xxhash-class at
  minimum), keys built as (world seed, node path, feature tag), never ad-hoc
  coordinate arithmetic. If anchor positions or loot should not be
  derivable by clients, the server's PRF must be cryptographic (SipHash) —
  a decision to make consciously, since ~30 hashes per column makes even
  that affordable.
- **The partition layer should be integer-exact; the analytic layer must
  avoid libm.** IEEE 754 base operations (+, −, ×, ÷, √) are exactly
  rounded and bit-identical everywhere; `sin`, `exp`, `pow` are **not** —
  they differ between libm implementations, so a warp or potential built on
  them generates different cities on different platforms, which the
  multiplayer authority model and the golden-frame discipline both forbid.
  Everything up to and including the descent (quadtree, splits, designation
  paths) can and should be pure integer/fixed-point. The warp and street
  potentials, which genuinely want smooth functions, must be built from
  polynomial/rational bases evaluated in a declared operation order — the
  project ships its own math kernel for worldgen, exactly the way it
  already refuses `bytemuck`'s native-endian casts in `mc-render` for
  determinism reasons (`rendering.md`).

## 4. Loopholes, edge cases, pitfalls

Collected from Sections 3, 5–6 and from reading the issue against the tree.
"Loophole" here means: a place where the design's stated guarantees can be
satisfied while the intended outcome silently fails.

1. **Seam correlation** (§3.2): ancestor-hashed boundary features repeat along
   coarse seams unless hashed with segment coordinates. Invisible in any test
   that doesn't walk a power-of-two boundary; visible to every player who does.
2. **Signed-coordinate LCA** (§3.2): sign-bit XOR sends origin-straddling
   pairs to the root. One biasing line, but a wrong one desynchronises every
   cross-boundary agreement near spawn — where every player starts.
3. **Warp folding** (§3.3): a sampled-not-proven gradient bound holds until a
   seed finds the fold. Load-time analytic check or it will ship broken for
   someone.
4. **Rejection-order determinism** (§3.4): removable entirely via interval
   sampling; if any rejection survives, it needs a bounded attempt count with
   a deterministic fallback (degrade to "no split"), because an unbounded loop
   is a Tier-3 violation arriving through a Tier-0 door.
5. **Incremental aggregates** (§3.5): the natural implementation of every
   Tier-2 feature is order-dependent. The memo-disabled test catches it only
   if the test's coverage includes an aggregate-consuming projection — worth
   pinning in test-map when the time comes.
6. **Layer connectivity is a percolation question unless made structural**
   (§5): hashing bridges/ramps in with probability p and hoping the layer
   stays connected is gambling against percolation thresholds; connectivity
   must come from a deterministic spanning construction, with hashed extras on
   top — the same shape as the issue's diagonal-augmentation rule, applied
   per layer and to inter-layer ramps.
7. **Second-pass detail vs. persistence**: deferred interiors ("shell now,
   interior on approach" — demoted to optional at current scale, but the
   machinery may return for megastructures) means a chunk can be generated
   twice at different detail levels. The save format currently stores whole
   worlds ("unmodified regions store nothing" is roadmap, not tree); whichever
   lands first, the second-pass rule must be: *the interior pass may only
   write cells the shell pass declared deferred-and-untouched, and player
   edits win unconditionally.* Cheap to state now, expensive to retrofit
   after persistence freezes.
8. **`Contents::Empty` vs. "not yet detailed"**: the tree deliberately has no
   placeholder blocks and no "unknown" state (`world-format.md`). A deferred
   interior region that is *stored* as empty is indistinguishable from one
   that is *finished* and empty. Deferral, if it returns, needs its own
   per-region flag outside the voxel data — never a sentinel block, which
   would violate the "nothing names nothing" decision.
9. **The evaluation-context cache and hot reload**: per-column partition
   context (decision 4) is a cache keyed by content; MVP 2's hot reload means
   content can change under it. The invalidation rule (any worldgen-content
   edit flushes all partition caches) is one line — its absence is a stale
   city that regenerates differently after restart, which reads exactly like
   the determinism bugs the tests hunt, and isn't one.
10. **Scale-collapse numbers depend on the scale cap.** The issue's cost
    retraction (8×, 3:1, no rendering spec) holds at 32–64-wide/~120-tall.
    The "theoretically infinite building" question (Section 5) deliberately
    exceeds that envelope; every number must be re-derived per scale class,
    and the doc below does.

---

## 5. Question 1 — a theoretically infinitely high building

Short answer: **yes, with four commitments** — 3D chunk addressing, an
analytic building envelope, two-level hash seeding with per-floor slices, and
regenerate-don't-persist storage. The generator itself imposes no height
limit; every real limit lives in the engine around it, and each one has a
known, shipped resolution. What follows is the derivation, then the
constraints, then the honest residue.

### 5.1 The generation model: envelope + slices

A building of unbounded height cannot be generated as an object — it must be
a **pure function answerable at any y in O(1)** (or O(log y)). The
construction, assembled from the issue's sliceability requirement plus the
precedents:

- **The silhouette is analytic.** `inside(x, y, z) = ‖(x,z) − center‖ <
  r(y, building_seed)` where `r(y)` is a closed-form (piecewise) function.
  Real skyscraper massing already has this shape — CGA-style base/shaft/crown
  splits and per-tier setbacks are piecewise functions of y by construction
  ([Müller et al. 2006](https://dl.acm.org/doi/10.1145/1141911.1141931)) — and
  Minecraft 1.18's density functions prove function-of-position solidity at
  production scale ([density functions](https://minecraft.wiki/w/Density_function)).
  This is the issue's "massing is just density", extended upward: nothing in
  `y < tower_height(cell)` requires `tower_height` to be finite.
- **Two-level seeding.** `building_seed = hash(world_seed, cell)` fixes
  footprint, height class, core/shaft positions, and the setback schedule —
  everything that must be *constant in y*. `floor_seed = hash(building_seed,
  floor_index)` fixes each slice's interior. This is exactly the pattern
  shipped by Greuter et al. 2003 (buildings regenerable in isolation from a
  position-derived seed, already built as vertical stacks of extruded floor
  plans — [paper](https://dl.acm.org/doi/10.1145/604471.604490)) and by The
  Lost Cities (per-floor part selection on a 6-block module,
  [docs](https://mcjty.eu/docs/mods/lost-cities/structure)). Vertical
  alignment of shafts falls out: shafts are level-1 data, constant in y, so
  every floor's constraint-aware subdivision (§3.4) inherits the same
  forbidden regions.
- **Interior hierarchy is Hahn et al. 2006**, the one published system for
  persistent lazy interiors: building → floor → room, each node re-deriving
  its children's seeds, regions generated on demand, discarded on exit,
  regenerated identically, with only player diffs persisted
  ([paper](https://dl.acm.org/doi/10.1145/1183316.1183342)). That is this
  project's determinism story applied to interiors, published twenty years
  ago.
- **Vertical coherence without a roof.** For a *finite* tall building,
  slice content may depend on fraction-of-height (`y / H`) since `H` is a
  level-1 param. For a *literally infinite* building there is no `H`, so
  vertical variation must come from absolute-y band structure: a 1D
  hierarchical descent over the y axis (bands of 2^k floors, split top-down
  by hash — the same machinery as the 2D descent, rotated). That gives
  setbacks, mechanical floors, sky lobbies and district-in-the-sky variation
  at O(log y) per query, which is O(1) for any y that fits in a word. The 2D
  quadtree's warp trick even applies to the band structure (warp y before
  descending) to break up power-of-two regularity. **The seam problem (§3.2)
  recurs vertically** and the same fix applies.

What recursive expansion cannot do, stated because it is the tempting
default: vanilla Minecraft's jigsaw — the machinery the issue's "vertical
jigsaw" borrows its name from — is a stateful grow-outward expansion capped
at **128 blocks from the structure start**, precisely because pieces are
placed relative to already-placed pieces
([Jigsaw structure](https://minecraft.wiki/w/Jigsaw_structure)). The vertical
jigsaw here must be jigsaw *in its part vocabulary only* (floor-plate
`.mcvox` templates with connectors), never jigsaw in its *algorithm*. Slice
selection is a hash, not an expansion.

### 5.2 What actually breaks at large y, with shipped resolutions

| Constraint | Where it bites | Resolution (precedent) |
|---|---|---|
| **Column addressing** | `[Section; 16]`, `COLUMN_HEIGHT = 256` in the tree today | 3D chunk addressing: load a ball of sections around the player, not columns. Cubic Chunks restructured Minecraft this way and reached ±2³⁰ blocks ([wiki](https://github.com/OpenCubicChunks/CubicChunks/wiki/About-the-mod)); Luanti has been 16³-block 3D-chunked from inception (±31000, [docs](https://docs.luanti.org/for-engine-devs/basic-data-structures/)); Hytale announced the same move in 2026, explicitly to unlock unbounded height ([announcement](https://hytale.com/news/2026/1/the-future-of-world-generation)). The issue already concludes the height decision precedes the wire format; this is the confirmation that the *addressed unit*, not the generator, is what decides height. |
| **Skylight** | "Below the topmost opaque block" is undecidable when the column above may be ungenerated/infinite | The single largest hidden cost every prior system hit. Cubic Chunks: a separately persisted per-column opacity oracle. Luanti: heuristics, yielding order-*dependent* shadow bugs ([#3421](https://github.com/minetest/minetest/issues/3421), [#5357](https://github.com/minetest/minetest/issues/5357)) — the exact failure class this project's tests exist to forbid. **Unique to this design: the plan layer rescues skylight analytically.** If the only unbounded occluders are buildings with closed-form envelopes, "is (x,z) open to sky above y" is an O(1) query against the plan — envelope + declared maximum protrusion (§3.7) — with no generated data consulted. See Section 7. |
| **f32 precision** | ULP reaches one block at y = 2²⁴; visible jitter far earlier (~0.06 m at y ≈ 10⁶; Godot's guidance: f32 fails for first-person past ~4–8 K units, doubles "usually required past 32–65 K" — [large world coordinates](https://github.com/godotengine/godot-docs/blob/4.2/tutorials/physics/large_world_coordinates.rst)) | Block coordinates are exact integers (i64 vertically); entity positions f64 or chunk-local f32; the renderer subtracts the camera's section origin before anything touches f32 (floating origin / camera-relative rendering — [overview](https://en.wikipedia.org/wiki/Floating_origin)). The tree is already camera-relative-shaped: packed vertices are section-local with the world frame reconstructed from the section table (`rendering.md`), so this is an extension of an existing decision, not a new one. |
| **Working set** | Every section in a tower column is non-empty for the whole height; at 4 KiB+/section a 64×64×10⁶ tower is ~10⁶ sections if persisted naively | **Regenerate, don't persist** (Hahn's model): a pristine section is a pure function and costs zero bytes in redb; only player diffs are stored. This also resolves the issue's "working set scales as N²×H" — the *resident* set is the loaded ball around the player, whose radius is a render decision, not a function of building height. |
| **Vertical streaming** | Free fall is ~50–78 blocks/s; Cubic Chunks measured ~1,300–2,200 cube generations/s needed at moderate view distance ([wiki](https://github.com/OpenCubicChunks/CubicChunks/wiki/About-the-mod)) | An O(1) analytic slice function is what makes this budget satisfiable — generating section (x, y₀, z) must never require touching the 62,000 sections below it. Sliceability is not just a cost nicety; it is the free-fall safety property. Prefetch is velocity-vector-biased, and elevators are the easy case. |
| **Luau/f64** | Coordinates and ids crossing the script boundary above 2⁵³ | Same rule as §3.1. Additionally, any `y` exposed to mods as a number is fine (2⁵³ blocks is beyond any practical height) but *packed* (x,y,z) keys are not. |

### 5.3 Scope recommendation: uncap the structure, not the world

No shipped engine exceeds ~±31 K blocks of height cheaply; the two that
pursued unbounded y (Cubic Chunks, Hytale) both treat it as a special
capability, and Hytale explicitly reserves it for non-default modes. The
pragmatic MyCraft path, compatible with everything above:

- Terrain lives in a bounded band (the measurement decides the constant —
  the issue's position, unchanged).
- Buildings may extend into an unbounded region **above** the band where the
  only permitted content is analytic-envelope structures. That sidesteps
  unbounded-terrain, 3D-biome and cave generality entirely while making
  "theoretically infinite building" literally true: `r(y)` never
  terminating is a valid height class.
- Whether the *addressed unit* goes 3D is then decided by the measured cost
  of a full-height column at the chosen band height — the issue's decision
  procedure, unchanged. If the band stays ≤ 512, columns survive; the
  infinite-tower region needs sparse vertical addressing either way, and a
  hybrid (columns for the band, cubes above) is uglier than cubes
  throughout. Derived, not observed: expect this to resolve to "cubes
  throughout" the moment vertical layering is real.

**Honest residue.** Three things "theoretically infinite" cannot have:
a roof (silhouette features must be locally generated forever — no crown,
no spire, which *is* an aesthetic cost); fraction-of-height styling; and any
Tier-2 aggregate over the building's floors ("this tower is 40% residential"
becomes a per-band property). All three vanish for merely-huge finite
heights, which is one more argument for height classes with an enormous
finite cap as the shipped default and `∞` as a curiosity the architecture
permits rather than a target it optimises for.

## 6. Question 2 — cloud streets and public transport

The issue establishes the interface ((p, layer) partition stack, ramps as
layer-adjacency edge features, routes as lattice-adjacency edges at a coarse
level). What it leaves open is the *construction* — how the layers, the
network and the transit lines are actually generated so they read as one
coherent city. The research resolves most of it; the findings converge hard.

### 6.1 The literature does not survive the constraints; the shipped tricks do

- **Parish & Müller 2001** grows roads from a priority queue where every new
  segment is checked against *all previously generated geometry* — the local
  constraints half is order-dependent by construction and does not survive.
  Its *global goals* (steering by density/pattern fields) are pure functions
  of position and do survive as parameter fields
  ([paper](https://cgg.mff.cuni.cz/~benes/PMoURN/data/ProceduralModellingOfUrbanRoadNetworks.pdf)).
- **Chen et al. 2008 tensor fields**: the field is a pure function of
  position (sum of hashed basis fields); only streamline tracing is global
  ([project](https://www.sci.utah.edu/~chengu/street_sig08/street_project.htm)).
  The issue's two-potential level-set variant is the correct localisation of
  this, and the research found nothing better.
- **Transit generation has no order-independent literature at all.** What
  exists is global optimisation (genetic algorithms, RL —
  [e.g.](https://www.researchgate.net/publication/330683273_The_Design_of_a_Metro_Network_Using_a_Genetic_Algorithm))
  — Tier-3 by definition. The gap the issue found for chunked city
  generation extends to transit. What the statistics literature *does*
  supply is the target shape: real metro networks converge on a **dense
  ring-and-core with radial branches**
  ([Barthélemy et al.](https://arxiv.org/pdf/1407.3915)), hub-and-spoke
  hierarchy emerges from purely local cost-benefit choices
  ([Louf/Jensen/Barthélemy, PNAS 2013](https://www.pnas.org/doi/10.1073/pnas.1222441110)),
  and typical metro graphs have average degree just over 2 with a minority of
  interchange nodes ([Derrible & Kennedy](https://link.springer.com/article/10.1007/s11116-009-9227-7)).
  A generator that hits those numbers will read as plausible; one that
  produces degree-4 meshes everywhere will read as a circuit board.
- **The Lost Cities' transit trick generalises and should be adopted
  verbatim:** the line topology is fixed by the lattice; local conditions
  decide only **station vs. pass-through**. A station generates where a city
  cell exists at the lattice point; otherwise the line runs through. The
  route can never break, because presence-of-line and presence-of-station are
  decoupled ([docs](https://mcjty.eu/docs/mods/lost-cities/structure)). Its
  lattice constants also hide a cheap variety trick: highways on multiples
  of 8, subway stations on a 10-chunk grid — near-coprime periods, so the
  two systems drift against each other instead of aligning.
- **LayerProcGen** (Rune Skovbo Johansen,
  [framework](https://github.com/unitycoder/LayerProcGen)) is the general
  formalisation the whole design already gestures at: generation layers at
  increasing detail, each reading only a *padded fixed-radius window* of
  coarser layers, never its own layer. That is precisely the issue's
  "declared read radius" made structural, and it is the shape the worldgen
  pipeline spec should have: partition level k may consult level k−1 within
  a declared window, never level k.

### 6.2 Real layered cities supply the design constants

The strongest research finding is that **real multi-level pedestrian systems
exist, are documented, and their legibility rules are exactly the ones the
issue derived abstractly**:

- **Calgary +15**: 86 bridges, 130 buildings, 16 km of network at *named
  datum heights* — +15 ft, with +30 and +45 where it stacks
  ([wiki](https://en.wikipedia.org/wiki/Plus_15)). The generative rule is
  per-building, not master-planned: developments were **required to connect
  to the network** in exchange for bonus floorspace. A per-lot connection
  obligation is a Tier-0 inherited param — the real city grew its skyway
  layer by exactly the mechanism this architecture offers for free.
- **Minneapolis Skyway**: ~80 blocks connected at second-floor level; the
  walkable graph is approximately **the street grid's dual, offset one floor
  up** — corridors pass through buildings and bridge mid-block
  ([wiki](https://en.wikipedia.org/wiki/Minneapolis_Skyway_System)). So a
  believable skyway layer is not an independent street network; it is a
  *derived* network: same partition, different geometric realisation.
- **Hong Kong**: continuous elevated spines feeding towers and terminals;
  HKU's 3D pedestrian network dataset classifies segments into **23
  categories across height levels** (sidewalk, footbridge, underground,
  ramp, rooftop — [project](https://www.uitlab.org/project/3d-ped-network/))
  — a ready-made edge-type vocabulary for the layer-adjacency graph.
- **Parameter targets**: urban station spacing ~500–900 m (Milan M1 mean
  590 m), 800 m as the walk-access ceiling; suburban 1.5–2 km. Lost Cities'
  10-chunk grid = 160 m is a ~1:4 game-scale compression of the real
  ~600–900 m, and players tolerate it. Pick the compression factor once,
  consciously, and derive every spacing from it.

Distilled legibility rules (each maps onto the architecture directly):

1. **Citywide datum heights, never per-chunk elevations.** Layers are a
   fixed table of named y-datums (content-declared constants). Lost Cities'
   everything-snaps-to-6 rule is what makes its multi-level output read as
   one city. This is also what makes the `(p, layer)` signature cheap:
   `layer` indexes a small static table, and cross-layer agreement reduces
   to shared constants.
2. **Layer specialisation**: transit below grade or on dedicated viaduct
   datums, vehicles at grade, pedestrians above — every real system
   converges on this assignment.
3. **Vertical circulation on a forced lattice.** Real systems fail where
   connections are sparse. Elevator cores / ramp spirals / station atria are
   hashed on a coarse lattice (every 2–4 chunks) and **forced present in
   every layer they span** — the 3D analogue of the spanning backbone.
   Making transit stations double as the multi-layer circulation nodes (as
   Hong Kong and Calgary do with tower lobbies) buys the "coherent city"
   reading almost for free, and answers the issue's ramps-as-edge-features
   sketch with a concrete rule: *the ramp lattice is layer-independent, so
   any layer reaches any other within a bounded walk.*
4. **Upper layers are sparser subsets.** Real skyways cover only the core.
   Per-layer presence derives from the same urbanness field with rising
   thresholds — which also delivers "you should never be able to see the sky
   cleanly" *in the core specifically*, where it belongs, rather than
   everywhere.

### 6.3 Connectivity is structural; probability is decoration

The percolation numbers make the issue's instinct precise. If optional links
are hashed i.i.d. with probability p on a square lattice, the bond threshold
is exactly 1/2 (site ≈ 0.5927). Two consequences:

- Below threshold: only finite islands. Above threshold: **only "a giant
  cluster exists"** — a constant fraction of local pockets remain cut off,
  and near the threshold the disconnected sub-networks are power-law-sized:
  arbitrarily large, plausible-looking, and unreachable. **Never derive
  reachability from a probability.**
- The correct pattern, which the issue's diagonal-augmentation rule already
  is for one level: a **deterministic spanning backbone by construction**
  (lattice adjacency; or Gabriel / relative-neighbourhood graphs on jittered
  sites, both of which contain the Euclidean MST and are therefore connected,
  decidable from a bounded radius when density is bounded below — β-skeletons
  with β > 2 can disconnect, so β ≤ 2 for anything load-bearing), **plus
  hashed decoration at p well above ~0.6** so the decoration itself mostly
  reads as connected. Apply per layer, *and* to the inter-layer ramp graph,
  *and* to skybridges.

For agreements without communication, two equivalent local mechanisms, both
already in the design: symmetric pair-hashing `h(min(a,b), max(a,b), seed)`
for cell-pair links, and the **edge-marker** trick (compute crossing points
on the shared chunk edge from the edge's own coordinates, so both sides
derive the identical marker —
[devlog](https://ozonewipeout.itch.io/ozone-wipeout/devlog/338218/devlog-january-23rd-2022)).
The LCA quadtree supersedes both *within* the tree, but pair-hashing remains
the answer at whatever level routes live at, and §3.2's seam rule (hash with
segment coordinates) applies to it.

### 6.4 The assembled construction

Putting it together — this is the shape a cloud-street spec should propose:

- **A declared layer table**: ~3–6 named datums (subway, grade, skyway-1,
  skyway-2, viaduct…), each with a street mechanism (BSP/arterial or
  level-set, §3.6), a presence threshold against urbanness, and an edge-type
  vocabulary borrowed from the HKU taxonomy.
- **Per layer**: a guaranteed backbone (lattice adjacency over that layer's
  hub lattice), decorated by hashed extras above the safety margin; the
  skyway layers realised as the *dual* of the block partition (Minneapolis),
  which the quadtree hands over for free — a skyway edge is a boundary
  feature of the split that made it, inherited by both lots (§3.2's rule
  applied at datum height).
- **Transit**: a coarser hub lattice (the issue's 4096-scale construct),
  lines as lattice-adjacency edges with hashed diagonal augmentation,
  **station-optional / line-always-continues**, station spacing from the
  compression factor, interchange nodes only where lines cross a hub that
  clears a rank threshold — targeting average degree ~2 and a ring-and-core
  shape by making ring edges a designated feature of the anchor table's
  downtown entry (§ anchors), not an emergent hope.
- **Vertical circulation**: the forced lattice of §6.2 rule 3, shared by all
  layers, coincident with stations where both exist.

Everything in that list is a pure function of (position, layer, seed) with a
declared read radius; nothing enumerates, relaxes, or paths. The one
genuinely new obligation this section adds to the issue's design is the
**dual-network realisation of skyways** — worth it, because it is what makes
a skyway layer read as belonging to the city below it rather than floating
over it.

## 7. Question 3 — what lighting requires

The issue names lighting as "the one subsystem that cannot be point-queried"
and "the first thing that will break the property the whole architecture
rests on". The mathematics says something more precise and considerably more
reassuring: **block light is a bounded-radius pure function; only skylight
has an unbounded dependency, and the plan layer resolves it analytically.**
The architecture survives. What lighting actually demands is a list of
concrete commitments, several of which touch specs that come *before*
lighting itself.

### 7.1 The mathematics: light is a confluent fixpoint with a 15-block window

Minecraft-family lighting (two 4-bit channels, 0–15, decrement per step,
opacity terminates — [mechanics](https://minecraft.wiki/w/Light)) has a clean
closed form: the light level at p is

    L(p) = max over sources s of ( emission(s) − d(s, p) )

where d is graph distance through transparent cells with per-block opacity
weights ≥ 1. That is a shortest-path problem in the (max, −) semiring — the
flood fill is just BFS because the weights are near-unit. Three consequences:

- **The fixpoint is unique and confluent.** L is the least fixpoint of a
  monotone operator (Knaster–Tarski); propagation order cannot change it.
  Determinism is a property of the *definition*; every order-dependence bug
  in shipped light engines is an implementation failing to reach the
  fixpoint — dropped queue entries, deferred async work, unreconciled chunk
  edges. Minecraft's "light suppression" exploit is exactly this: an
  update-queue cap plus a restart discards work, and wrong levels persist
  forever ([documented](https://techmcdocs.github.io/pages/BugsAndExploits/LightSupression/)).
  The rule that follows for MyCraft: **the light queue is drained to
  fixpoint synchronously within the tick that changed the world, never on a
  droppable async thread.** Bounded work makes that affordable (next
  point), and the property test is the one the research recommends:
  `full_relight(world) ≡ incremental_edits(world)`, bit-identical — the
  same shape as the generator's memo test.
- **Block light has read radius 15.** Every step costs ≥ 1, so
  emission 15 reaches at most 15 steps: L(p) depends only on geometry and
  sources within an L1-ball of radius 15 — about 4,991 cells
  (`(4r³+6r²+8r+3)/3`). Lighting is therefore *not* outside the
  architecture: it is a pure function of world state with a **declared read
  radius of 15**, checkable by the same chunk-vs-monolithic test as
  everything else. What distinguishes it from the partition is only its
  cost model — per-point evaluation is ~5k cells, so the natural
  implementation amortises per chunk (one seeded BFS to fixpoint at
  generation time) and updates incrementally on edit. The issue's collision
  dissolves from "breaks the property" to "same property, different
  amortisation".
- **Removal is the hard half, and it is a known algorithm.** Deleting a
  source is decremental shortest paths — harder than insertion, which is
  why every shipped engine uses the two-phase unlight-then-relight BFS
  (carry the old level outward zeroing strictly-dimmer cells, collect the
  equal-or-brighter frontier, re-propagate from it —
  [Seed of Andromeda writeup](https://notverymoe.github.io/md-gamedev-gems/voxel/lighting/soa/index.html)).
  Cost ~2× the lit volume, so a worst-case single edit is ~10–15k cell
  operations — microseconds, not milliseconds, with flat arrays.

**Skylight is the genuinely non-local channel, and the plan rescues it.**
Sky light seeds at 15 above the heightmap and propagates downward *without
attenuation*, so a cell's skylight can depend on geometry arbitrarily far
above it (a shaft lights its floor from a roof opening a thousand blocks
up). Under bounded height that is a per-column scan; under unbounded height
it is undecidable from voxels — which is exactly why Cubic Chunks' hardest,
buggiest subsystem was its separately-persisted per-column opacity oracle
([wiki](https://github.com/OpenCubicChunks/CubicChunks/wiki/About-the-mod)),
and why Luanti, which guesses, has order-dependent shadow artifacts
([#3421](https://github.com/minetest/minetest/issues/3421),
[#5357](https://github.com/minetest/minetest/issues/5357)) — the precise
failure class this project's tests exist to forbid. MyCraft's unusual
advantage: the city's occluders are *plan data with analytic envelopes*
(§5), so `H(x,z) = max` over candidate structures (bounded by §3.7's
protrusion constant) is an O(candidates) query against the plan, no voxels
consulted; and vertical openness *inside* a building (atria, light wells,
shafts) is level-1 building data, constant in y, answering "is this cell
open to sky" without materialising the tower above. Terrain contributes its
own analytic height field the same way. **Skylight seeding becomes a plan
query; only the ≤15-step lateral spill is computed from voxels.** This is
the one place the generator's contracts and the light engine genuinely
interlock, and it is why the issue is right that lighting must be
understood *before* the generator's contracts freeze — the plan API must
expose the envelope/heightmap query, and §3.7's protrusion bound must be
declared, for skylight to be computable at all.

### 7.2 What shipped engines settle

The research is unusually conclusive about the implementation family:

- **CPU flood fill is the authoritative field, done Starlight-style.**
  Vanilla Minecraft's engine did ~7× more light reads and 6–42× more block
  reads than necessary per operation (measured: 171,739 vs 24,535 light
  reads for identical work; one block removal 152k+181k reads vs
  20k+4k); Starlight's rewrite — push propagation, separate
  increase/decrease queues, heightmap-seeded skylight, flat arrays and ring
  buffers instead of hash maps — relit regions ~25–35× faster (~7 s vs
  ~170–220 s), and its ideas were absorbed into vanilla 1.20
  ([technical details](https://github.com/PaperMC/Starlight/blob/fabric/TECHNICAL_DETAILS.md),
  [author's retrospective](https://gist.github.com/Spottedleaf/6cc1acdd03a9b7ac34699bf5e8f1b85c)).
  There is no reason to relive the vanilla mistakes; the fast shape is
  published.
- **Cost in a dense emissive city is per lit volume, not per source.**
  Overlapping floods early-terminate against each other
  (`if neighbour ≥ mine − 1, stop`), so ten thousand neon signs in a
  district cost roughly (transparent volume within 15 of any source) ×
  small constant — and a city is mostly *walls*, which clamp propagation
  hard. Interiors make lighting cheaper, not dearer. The failure mode to
  design against is not steady-state cost but **batching discipline**:
  light a fresh chunk in one seeded pass (all sources enqueued together),
  coalesce same-tick edits, only enqueue improved values.
- **Storage is 2 KB per section per channel** (4 bits × 4096), with empty
  sections omitted. **Colored block light triples that channel and the
  propagation work** (RGB444 → 2 bytes/voxel with sky), and introduces the
  hue-drift artifact (channels attenuate at equal rates, so mixed colours
  shift hue with distance; Seed of Andromeda shipped RGB flood fill and
  mitigated by restricting sources to the 7 corner colours). GPU point
  lights are not the alternative: mods that mapped emissive blocks to real
  lights collapse at ~1.5–4k sources, a number a single MyCraft district
  exceeds. **Decision to make consciously and early: a neon city wants
  colored light, colored light must be voxel-grid flood fill, and
  retrofitting RGB into a shipped 4-bit engine changes save format, wire
  format and renderer at once.** This is the strongest argument in this
  whole document for settling lighting's data model before `mc-proto`
  exists.
- **Keep light out of the greedy-mesh merge predicate.** Per-vertex light
  narrows the merge predicate and shreds greedy meshing's reduction
  (practitioner consensus: with light+AO in the key, merging buys "not
  much, if at all"). The standard escape: **merge on material only, upload
  the light field as a per-chunk 3D texture, sample it in the fragment
  shader** (trilinear = smooth lighting for free; a relight becomes a
  texture upload with *no remesh*). For this project that choice is worth
  more than performance: `rendering.md` already warns that per-vertex AO
  would narrow the merge predicate and invalidate every golden — the
  3D-texture path keeps the mesher's contract *untouched by lighting
  forever*, preserves quad counts, and turns "neon sign flickers" from a
  remesh into an upload. AO can ride in the same texture. The packed
  vertex's spare bits stay spare.
- **Day/night is a shader multiplier, not a relight.** Sky light is stored
  once; the day-night cycle scales its contribution in the lightmap
  (`max(block, sky × daylight)`), so dusk costs zero propagation — which is
  what makes a *night* city free to render at night.
- **The GPU layer is presentation, not truth.** Cascaded shadow maps for
  sun/moon, clustered forward+ (routinely thousands of point lights) or a
  Photon-style camera-local propagation volume for neon sparkle — all
  client-side polish above the CPU field. The server keeps the CPU field
  because **gameplay reads light**: mob spawning, plant growth, and — under
  invariant 2 — Luau callbacks will ask "what is the light here", which
  makes the light field *state in Rust* that scripts consume, not an
  effect. Teardown-style full raytracing and Minecraft-RTX path tracing are
  poles that replace the renderer wholesale and price out low-end targets;
  neither fits a 32-player wgpu raster engine.
- **Persist light with an algorithm version, recompute on mismatch**
  (Starlight versions its saved light). Nibble arrays compress superbly
  under zstd; recompute-on-load is cheap in a Starlight-class engine, so
  the save carries light as a cache, never as truth.

### 7.3 The requirements list

What "make lighting work properly" concretely requires, in dependency order —
items 1–4 land in specs that precede any light engine:

1. **Block definitions carry light opacity and emission from the start.**
   PRO-904 already splits solid/drawn/occludes/…; light opacity and
   emission (level, and colour if RGB is chosen) belong in that same split,
   because a glass-and-neon city is precisely the place "solid",
   "occludes-view" and "occludes-light" part ways. Retrofitting a field
   into the block contract after content exists is a migration; adding it
   now is a line.
2. **The RGB decision, made once, before `mc-proto`.** 4-bit two-channel is
   the cheap floor; RGB444+sky is 4× the storage and 3× the propagation and
   is what the aesthetic wants. Either is fine; changing between them after
   the wire format and save format carry light is not.
3. **The plan exposes the skylight oracle**: analytic heightmap
   `H(x,z)` (envelope + protrusion bound) and per-building vertical
   openness. This is a generator API obligation, owed by the worldgen spec
   to a light engine that does not exist yet — the concrete instance of
   "the cheapest time to know is before the generator's contracts are
   written".
4. **A declared light read radius (15)** joins the declared constants, and
   chunk lighting is defined to consult the one-ring neighbourhood, so the
   monolithic-vs-chunked test covers light exactly as it covers the
   partition. Boundary reconciliation is thereby a tested property, not a
   convention — the chunk-border black patch, the genre's most famous
   lighting bug, is the failure this forbids.
5. **Starlight-shaped engine**: push propagation, separate
   increase/decrease queues, flat nibble arrays, two-phase removal,
   heightmap-seeded sky light, queue drained to fixpoint within the tick.
   Property tests: full-relight ≡ incremental (bit-identical), reverse-order
   ≡ forward, mutation tests on the queue-drop path.
6. **Renderer integration by 3D light texture**, leaving the mesher's merge
   predicate material-only; smooth light via trilinear sampling; AO in the
   same volume; goldens re-shot once, deliberately, when the lighting pass
   lands (`SCENE_REVISION` bump — the mechanism `rendering.md` already
   defines for exactly this).
7. **GPU polish later and optionally**: shadow cascade, clustered emissives,
   exposure/bloom for neon. None of it server-relevant, none of it blocking.

One measurement note closing the loop to the issue's prototype instruction:
"render it at night with faked emissive" needs none of the above — fullbright
emissive materials in the shader is one line and answers the aesthetic
question the prototype exists to ask. The list above is what *shipping* the
aesthetic requires, and item 2 is the only one that is expensive to get
wrong.

### 7.4 Would hardware ray tracing (RTX / AMD) make lighting easier? No.

Asked directly, researched directly, answered directly: **hardware RT makes
this project's lighting strictly harder** — it adds four to five subsystems,
removes none, shrinks the audience, and breaks the verification discipline.
The per-axis findings:

- **It does not replace the light field; it duplicates it.** The precedent
  is exact: Minecraft RTX still runs the 0–15 flood fill for gameplay —
  mob spawning and crop growth read it; RTX only changed pixels, and
  players filed feedback because they could no longer *see* where mobs
  spawn. For MyCraft the coupling is harder still: the server is
  authoritative and headless, and under invariant 2 Luau scripts will
  query light as state — so the deterministic CPU field must exist on
  machines with no GPU at all, whatever the client renders. RT can only
  ever be an *additional* system on top of §7.3, never a substitute for
  any part of it.
- **The API is experimental in this stack.** wgpu's ray tracing is
  `Features::EXPERIMENTAL_RAY_QUERY`, whose own spec says the features
  "may have major bugs" and are "subject to breaking changes"; ray-tracing
  *pipelines* are unimplemented, and the tracking issue lists undefined
  behaviour, AMD failures and Metal synchronisation among open items
  ([spec](https://github.com/gfx-rs/wgpu/blob/v30/docs/api-specs/ray_tracing.md),
  [tracking](https://github.com/gfx-rs/wgpu/issues/6762)). The WebGPU
  standard has no RT at all. A load-bearing subsystem cannot stand on that.
- **The hardware floor excludes the target audience's tail.** Nominal RT
  capability on Steam mid-2026 is around two-thirds of users; mandatory-RT
  titles set floors at RTX 2060 Super / RX 6600, and the Steam Deck runs
  Quake II RTX at ~216p to hold 60 fps. A mandatory-RT lighting path turns
  away roughly a third of the platform — or forces maintaining the raster
  path *as well*, which is the flood-fill system again, plus everything RT
  adds.
- **The engineering is the opposite of "easier".** Q2VKPT — the minimal
  serious voxel-era path tracer — is ~12,000 lines replacing the renderer
  of one of the simplest licensable games, before NVIDIA productised it.
  Minecraft RTX's frame is ~10 G-buffer targets, everything rendered twice
  for transmissives, three ray dispatches, an irradiance cache, SVGF
  temporal reprojection, per-signal multi-pass bilateral filters, and
  **mandatory DLSS** because native-resolution path tracing was not viable
  ([frame analysis](https://alain.xyz/blog/frame-analysis-minecraftrtx)).
  Every denoiser stage is its own failure-mode family (ghosting, boiling,
  disocclusion noise, light lag). None of that machinery exists in the
  flood-fill path.
- **Editable chunks are the worst case for acceleration structures.** DXR
  wants stable BLASes; refits require unchanged topology, and a
  greedy-meshed chunk changes triangle count on nearly every edit — so
  every block edit is a full per-chunk BLAS rebuild, the operation wgpu's
  own docs flag as slow, running continuously under 32-player streaming
  ([NVIDIA best practices](https://developer.nvidia.com/blog/rtx-best-practices/)).
- **Golden frames die.** The Vulkan spec makes ray-triangle intersection
  implementation-specific, with traversal order unspecified and
  watertightness guaranteed only within a geometry
  ([spec](https://docs.vulkan.org/spec/latest/chapters/raytraversal.html));
  acceleration structures are opaque vendor blobs. Even ray-*query* output
  is not bit-reproducible across vendors or drivers, and path-traced
  output additionally depends on accumulation history. The project's
  entire GPU verification strategy (ADR-008: golden frames because GPU
  code is otherwise unverifiable) is incompatible with a
  non-reproducible-by-specification draw path.
- **If traced lighting is ever wanted, the voxel-native route beats the
  hardware route.** Teardown ships fully traced-looking lighting by
  raymarching its voxel data in ordinary shaders — no DXR, minimum GPU a
  GTX 1070. Hardware RT cores accelerate *triangle* tests; custom AABB
  intersections measure ~2× slower than hardware triangle tests
  ([JCGT](https://jcgt.org/published/0011/03/06/paper-lowres.pdf)), and a
  chunked voxel world **already is an acceleration structure** — DDA
  traversal ([Amanatides–Woo](http://www.cse.yorku.ca/~amana/research/grid.pdf))
  traces the same occupancy data the game maintains, so the "AS rebuild"
  after an edit is the chunk update that happens anyway, deterministic and
  testable in plain compute.

**Verdict:** the flood-fill field stays the single source of truth for
gameplay and raster shading. If a premium GI mode is ever wanted, it is a
compute-shader DDA over the chunk grid as an optional client effect —
re-evaluate hardware RT only if wgpu's support exits experimental status,
and never as the lighting foundation. This slots into §7.3 as a refinement
of item 7, changing nothing above it.

---

## 8. Pitfalls, consolidated

The failure modes worth carrying into any spec, ranked roughly by how silent
they are:

| # | Pitfall | Silent until | Countermeasure |
|---|---------|-------------|----------------|
| 1 | Cell id through f64 (density graph **or Luau**) truncates above 2⁵³ and collides | Two districts look identical, months later | Ids never cross as numbers (§3.1) |
| 2 | Boundary features hashed from LCA node id alone repeat along coarse seams | A player walks a power-of-two boundary | Hash with segment coordinates (§3.2) |
| 3 | Signed-coordinate XOR sends origin-straddling pairs to the root ancestor | Cross-boundary agreements desynchronise at spawn | Bias to unsigned once, spelled in the spec (§3.2) |
| 4 | Warp folds under a sampled-not-proven gradient bound | One seed, one fold, interleaved geometry | Load-time analytic budget `Σ aᵢgᵢ < 1` (§3.3) |
| 5 | Rejection sampling makes determinism depend on rejection order | A cache or evaluation-order change | Interval sampling; no rejection at all (§3.4) |
| 6 | Tier-2 aggregates incrementally maintained | Any out-of-order generation | Aggregates are functions of district id, full descent (§3.5) |
| 7 | Unquantised radial-street winding tears the street family | A spiral seam at a district centre | Integer avenue counts, checked at load (§3.6) |
| 8 | Layer/ramp connectivity left to probability | Power-law-sized unreachable pockets near p_c | Structural backbone + decoration above ~0.6 (§6.3) |
| 9 | Weak coordinate hashing | Lattice artifacts; seed recovery by players | Keyed PRF, domain separation (§3.9) |
| 10 | libm transcendentals in worldgen | Different cities per platform | Shipped math kernel, integer-exact partition (§3.9) |
| 11 | Light queue async and droppable | Permanent wrong light after load spikes | Synchronous drain to fixpoint per tick (§7.1) |
| 12 | Per-vertex light in the merge predicate | Quad counts shred; every golden invalidated twice | 3D light texture; mesher stays light-free (§7.2) |
| 13 | RGB light retrofitted after formats freeze | Save + wire + renderer change at once | Decide colour before `mc-proto` (§7.3) |
| 14 | Skylight guessed where the column above is ungenerated | Order-dependent shadows (Luanti's standing bug) | Plan-analytic heightmap oracle (§7.1) |
| 15 | Deferred-detail second pass clobbers edits, or stores "not yet detailed" as `Empty` | First megastructure + persistence interaction | Detail-state flag outside voxel data; edits win (§4) |
| 16 | Free-fall outruns vertical generation | Player falls through ungenerated sections | Sliceability as a safety property; ~2k sections/s budget (§5.2) |

## 9. Open questions

Genuinely open — should survive into `/sdd-start` as questions, not answers:

1. **The measurement chunk and the anchor-lattice spike** (the issue's own
   next actions) — nothing here substitutes for either. The spike answers
   "do quantised lot sizes read acceptably", which decides whether the
   plan layer is built at all.
2. **Column height / 3D addressing**: does the measured full-height city
   column keep columns viable, or does the addressed unit grow a y extent
   (§5.3)? The issue's decision procedure stands; this document adds only
   the prediction that vertical layering resolves it to "cubes".
3. **RGB versus 4-bit light** (§7.3 item 2) — the single most
   expensive-to-reverse lighting decision, and a taste decision as much as
   a technical one. Wants the night-render prototype in front of eyes.
4. **The layer table**: how many datums, at which heights, with which street
   mechanism per layer (§6.4)? Wants the prototype, not argument.
5. **Party walls at 2-thick** — the issue's open question, unchanged;
   reasoned, not seen.
6. **Per-column context sufficiency**: does the eight-projection graph stay
   cheap under the per-column memo (§ issue decision 4)? Wants measuring.
7. **Seed secrecy**: is "players can compute anchor/loot positions from the
   seed" a problem this game cares about? Decides PRF strength (§3.9).
8. **How organic is organic**: the conformal construction (§3.6) is elegant
   and unshipped anywhere; the warp (§3.3) is proven safe but its look is
   untested. Both want the prototype extended by one district each before
   either is specced as the mechanism.

## 10. Recommended sequence

1. The issue's measurement block, unchanged, plus the night render with
   fullbright emissive (§7.3, one line of shader).
2. The anchor-lattice spike, unchanged — it is the cheap fork in the road.
3. Three decisions made early because they are cheap now and rewrites later:
   `(p, layer)` in the partition signature; light opacity + emission in the
   block contract (rides PRO-904); the RGB decision (§7.3).
4. The worldgen pipeline spec adopts: declared constants (read radius per
   stage, minimum lot size, maximum protrusion, warp budget, FD step),
   sliding-block-code framing with additive radii (§3.8), PRF hashing and
   the no-libm rule (§3.9), interval sampling (§3.4), seam rules (§3.2).
5. The plan layer, if the spike votes for it — with the skylight oracle
   (§7.1) in its API from the first draft.
6. Lighting as its own spec, Starlight-shaped (§7.3 items 4–6), before
   public-facing polish and after the generator's contracts exist.

## 11. Sources

Beyond the issue's own source list. Verified 2026-08-15.

**Vertical worlds and tall structures** —
[OpenCubicChunks wiki](https://github.com/OpenCubicChunks/CubicChunks/wiki/About-the-mod) ·
[Minecraft 1.18 height change](https://learn.microsoft.com/en-us/minecraft/creator/documents/worldheightchange?view=minecraft-bedrock-stable) ·
[Dimension height caps](https://minecraft.wiki/w/Dimension_type) ·
[Luanti data structures](https://docs.luanti.org/for-engine-devs/basic-data-structures/) ·
[Luanti #3421](https://github.com/minetest/minetest/issues/3421) /
[#5357](https://github.com/minetest/minetest/issues/5357) ·
[Hytale: The Future of World Generation](https://hytale.com/news/2026/1/the-future-of-world-generation) ·
[Jigsaw structure limits](https://minecraft.wiki/w/Jigsaw_structure) ·
[Hahn et al. 2006, Persistent Realtime Building Interior Generation](https://dl.acm.org/doi/10.1145/1183316.1183342) ·
[Greuter et al. 2003](https://dl.acm.org/doi/10.1145/604471.604490) ·
[Müller et al. 2006, CGA](https://dl.acm.org/doi/10.1145/1141911.1141931) ·
[Density functions](https://minecraft.wiki/w/Density_function) ·
[Godot large world coordinates](https://github.com/godotengine/godot-docs/blob/4.2/tutorials/physics/large_world_coordinates.rst) ·
[Floating origin](https://en.wikipedia.org/wiki/Floating_origin)

**Streets, transit, layered fabric** —
[Parish & Müller 2001](https://cgg.mff.cuni.cz/~benes/PMoURN/data/ProceduralModellingOfUrbanRoadNetworks.pdf) ·
[Chen et al. 2008](https://www.sci.utah.edu/~chengu/street_sig08/street_project.htm) ·
[Lost Cities structure docs](https://mcjty.eu/docs/mods/lost-cities/structure) ·
[LayerProcGen](https://github.com/unitycoder/LayerProcGen) ·
[Edge-marker technique](https://ozonewipeout.itch.io/ozone-wipeout/devlog/338218/devlog-january-23rd-2022) ·
[Infinite Modifying in Blocks](https://www.boristhebrave.com/2021/11/08/infinite-modifying-in-blocks/) ·
[Louf, Jensen, Barthélemy PNAS 2013](https://www.pnas.org/doi/10.1073/pnas.1222441110) ·
[Scaling in transportation networks](https://arxiv.org/pdf/1407.3915) ·
[Derrible & Kennedy](https://link.springer.com/article/10.1007/s11116-009-9227-7) ·
[Calgary +15](https://en.wikipedia.org/wiki/Plus_15) ·
[Minneapolis Skyway](https://en.wikipedia.org/wiki/Minneapolis_Skyway_System) ·
[HKU 3D pedestrian network](https://www.uitlab.org/project/3d-ped-network/) ·
[Gabriel graph spanning properties](https://luc.devroye.org/gabriel-spanningratio.pdf) ·
[Percolation thresholds](https://arxiv.org/pdf/1103.3243) ·
[Milan Metro spacing](https://en.wikipedia.org/wiki/Milan_Metro) ·
[Stop-spacing analysis](https://pedestrianobservations.com/2020/07/16/density-and-subway-stop-spacing/)

**Lighting** —
[minecraft.wiki: Light](https://minecraft.wiki/w/Light) ·
[Chunk format (light storage)](https://minecraft.wiki/w/Chunk_format) ·
[Starlight technical details](https://github.com/PaperMC/Starlight/blob/fabric/TECHNICAL_DETAILS.md) ·
[Starlight retrospective](https://gist.github.com/Spottedleaf/6cc1acdd03a9b7ac34699bf5e8f1b85c) ·
[Light suppression](https://techmcdocs.github.io/pages/BugsAndExploits/LightSupression/) ·
[Seed of Andromeda RGB flood fill](https://notverymoe.github.io/md-gamedev-gems/voxel/lighting/soa/index.html) ·
[0fps voxel AO](https://0fps.net/2013/07/03/ambient-occlusion-for-minecraft-like-worlds/) ·
[Greedy meshing vs light](https://playspacefarer.com/voxel-meshing/) ·
[Teardown frame breakdown](https://juandiegomontoya.github.io/teardown_breakdown.html) ·
[Minecraft RTX cost](https://www.pcworld.com/article/399042/minecraft-rtx-ray-tracing-nvidia-dlss-20.html) ·
[Photon LPV](https://deepwiki.com/sixthsurge/photon/5.3-light-propagation-volumes) ·
[Clustered shading](http://www.aortiz.me/2018/12/21/CG.html) ·
[DDGI overview](https://morgan3d.github.io/articles/2019-04-01-ddgi/overview.html) ·
[Colored Lux limits](https://www.curseforge.com/minecraft/mc-mods/colored-lux)

**Hardware ray tracing (§7.4)** —
[wgpu ray-tracing API spec](https://github.com/gfx-rs/wgpu/blob/v30/docs/api-specs/ray_tracing.md) ·
[wgpu RT tracking issue](https://github.com/gfx-rs/wgpu/issues/6762) ·
[Minecraft RTX frame analysis](https://alain.xyz/blog/frame-analysis-minecraftrtx) ·
[Q2VKPT](https://brechpunkt.de/q2vkpt/) ·
[NVIDIA RTX best practices (BLAS refit rules)](https://developer.nvidia.com/blog/rtx-best-practices/) ·
[Vulkan ray traversal (implementation-specific intersection)](https://docs.vulkan.org/spec/latest/chapters/raytraversal.html) ·
[SDF-grid vs hardware triangle intersection, JCGT](https://jcgt.org/published/0011/03/06/paper-lowres.pdf) ·
[Amanatides–Woo DDA](http://www.cse.yorku.ca/~amana/research/grid.pdf) ·
[Teardown minimum spec interview](https://80.lv/articles/teardown-developer-breaks-down-multiplayer-and-voxel-destruction-tech) ·
[Steam Deck RT measurements](https://www.digitaltrends.com/computing/ray-tracing-on-steam-deck-is-possible-with-a-catch/) ·
[Steam hardware survey](https://store.steampowered.com/hwsurvey/videocard/)
