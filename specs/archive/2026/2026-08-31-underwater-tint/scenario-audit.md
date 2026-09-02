# Scenario audit — SPEC-032

Run against the 38-scenario draft. Every finding is recorded with what was done
about it, including the three that were declined.

## What the audit found in the repository, and what it changed here

Three of the spec's own assertions were contradicted by the tree. All three were
re-verified before acting.

| Claim as drafted | What the tree says | Change |
|---|---|---|
| "`#RRGGBB` and `#RRGGBBAA` already parse somewhere" | **two** shipped readers, neither reusable: `crates/mc-core/src/hud/element.rs:148` takes eight digits and refuses six, `tools/voxforge/src/material.rs:210` takes six and refuses eight, and each refusal claims its own form is the only one | the Existing Code row now prices a **new reader** taking both, rather than a reuse |
| FR-1.2-S3 refused `#3A6EA5FF` | that is exactly the form `content/base/hud/*.toml` **requires** — the field would refuse the only colour form the engine accepts | both forms are now accepted; the refusal narrowed to an alpha other than `FF` |
| case unstated | materials write lowercase, the HUD writes uppercase, both readers are case-insensitive by construction | FR-1.1-S1 now states all three accepted spellings |

**One correction to the audit itself, measured by the owner after it ran.** The
audit reported the six-digit reading as living only in a test helper. It does
not: `tools/voxforge/src/material.rs:210` is production and reads shipped
`content/base/materials/*.toml`. The conclusion is unchanged and the premise is
stronger — content already speaks both dialects, so accepting both adds no third
rule. Filed as **PRO-999**.

## Gaps accepted (6 scenarios added, 1 folded in)

| Gap | Where it went |
|---|---|
| The eye entering and leaving the medium is only tested a whole block either side, so a half-block rounding error passes everything — and the shipped player is submerged by **0.38 blocks** | folded into **FR-2.4-S2**, restated at `y = 34.98` / `35.02` across the cell's own face (this also removed a placeholder violation) |
| The player-reachability premise — the spec's own Principle 9 commitment — is asserted by nothing, and every FR-2 reading supplies its own pose | new **FR-2.6-S1** and its control **S2** |
| Architecture Delta §B's two candidates tint a translucent layer differently, and the sea *is* translucent | new **FR-2.1-S6** |
| A reload carrying the colour but keeping a stale distance passes all three reload scenarios | new **FR-4.1-S4** |
| FR-6.4 is an absence assertion with no positive control, where FR-6.2 and FR-6.3 both have one | new **FR-6.4-S2** |
| FR-5 has no unwanted-behaviour scenario, and a leading revision byte that moved in neither fold is invisible to every comparative witness | new **FR-5.1-S3** |

## Vacuity repairs (no slots spent)

Four scenarios could be satisfied by a wrong implementation.

- **FR-2.3-S1 and FR-2.4-S1** were pure frame-to-frame identity claims. A
  constant wash applied regardless of the eye's medium makes both frames
  identically wrong and passes both. The superseded test made an absolute
  per-sample claim *beside* its pixel comparison for exactly this reason; keeping
  only the comparative half was a strict weakening of a shipped instrument. Both
  now carry the absolute half in the same assertion, and FR-2's constraint 1 was
  rewritten so the two are no longer an exception to it.
- **FR-2.1-S4**, the only scenario telling radial distance from view depth,
  asserted two pixels were drawn *the same*. An implementation applying no tint
  at all satisfied that. Restated absolutely: `6.0` blocks at the centre against
  `6.74` at a quarter-frame offset, derived from the shipped camera
  (`FIELD_OF_VIEW_DEGREES = 60.0`, 1280×720 → θ = 27.16°, `6.0 / cos θ = 6.74`).
  A depth implementation draws both at the even mix and is now rejected.
- **FR-2.5-S1** was comparative *and* empty-set vacuous — "every HUD pixel" is
  satisfied over zero of them, so a frame that failed to composite the HUD passed.
  Now absolute against the declared colours, over at least one hundred pixels.
- **FR-3.1-S1** could pass by comparing a freshly minted set against the tree
  that minted it. Now bound to the captures committed **before this spec's first
  implementation commit**, byte for byte.

## Merges accepted (2 slots freed)

- **FR-1.3-S2 + S3** — `math.huge` and `0/0` reach one `!is_finite()` guard.
  Merged, **both values still named**: NaN fails a `> 0.0` comparison and
  infinity passes it, so the pair is what catches a wrong ordering of the two
  checks, and dropping either loses that.
- **FR-1.4-S1 + S2** — the both-or-neither pair. Merged, at the stated cost that
  one scenario now carries two refusal messages; it stays falsifiable only
  because both are named.

## Rule violations fixed

| Scenario | Was | Now |
|---|---|---|
| FR-2.1-S3 | a surface "at the eye" — undrawable, `NEAR_PLANE = 0.1` (`crates/mc-render/src/camera.rs:127`) and a surface at `d = 0` covers no pixels | `1.2` blocks, one tenth of the way toward the colour |
| FR-3.1-S1 | two claims — captures match **and** the revision is unchanged, the second pre-deciding what Architecture Delta §C hands to the architect | the revision clause dropped; S1 is the measurement, §C draws the verdict |
| FR-2.4-S2 | "tinted" and "untinted" as outcomes — placeholders | named predictions, via the boundary rewrite above |
| FR-2.2-S1 | "the **declared** clear colour" — the clear colour is a Rust constant, which this spec's own Notes say | "the colour a dry camera's sky is given" |
| FR-1.5-S1 | a **containment** claim over a hand-maintained list — the exact filtering failure `testing.md` §2 records, where two mirrors held at six while the loader grew to nine and neither reddened | the whole list, fifteen names, in the loader's own order |

## Declined, with reasons

- **A scenario for the eye inside an opaque block that declares a tint.** The
  state is reachable by declaration (FR-1.1-S4 admits it) but by no shipped
  content, and the answer falls out of the uniform rule. Recorded as prose in
  FR-2's preamble instead, so an implementer does not have to guess.
- **A scenario asserting the terminal names the appearance change on loading an
  older save.** Whether an appearance move is reported to a player is the
  existing changed-blocks mechanism's behaviour, not this spec's; asserting it
  here would test something this spec does not change. FR-5.1-S3 covers the half
  that is this spec's — the byte itself.
- **Merging FR-1.2-S1 with FR-1.3-S3** (a colour of the wrong kind and a
  distance of the wrong kind). They are one *shape* of refusal reached through
  two different readers, so the merge would buy a slot by deleting a falsifier
  on a path nothing else covers.

## Budget

38 drafted → 42 after two merges and six additions. **Above the ~40 guideline**,
so the count and this list went to the approver rather than being taken
silently. The cheapest further cut, if one is wanted, is FR-1.2-S1 or FR-1.3-S3
as noted above — each costs a refusal branch nothing else reaches.
