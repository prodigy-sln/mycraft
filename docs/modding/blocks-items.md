# Authoring Blocks

How to declare a block so the engine registers it. A block declaration is a
**Luau chunk**, and this is the whole contract.

If you have never written anything for this engine before, start at
[README.md](README.md) — it walks you from an empty file to a block you can go
and stand on. This page is the reference you come back to.

## File layout

One block per file, under `<mod>/blocks/`:

```
content/<mod>/blocks/<block>.luau
```

The loader reads every `*.luau` file **directly** under `<root>/blocks/`, and
nothing else:

- A file with any other extension is passed over in silence. Keep notes,
  scratch files and working copies beside your declarations freely.
- A subdirectory is not descended into. `blocks/nested/hidden.luau` is not a
  declaration.
- An entry that **wears** a declaration's name without being a file — a
  directory called `nested.luau` — is refused rather than passed over. Naming a
  thing `.luau` is how you say *this is a declaration*, so the loader answers
  when something says it and is not one.

Declarations are read in **file-name sorted order**, and the sort is binding
rather than an implementation detail. Two things depend on it: which block ends
up in a new player's hand (the first solid block registered), and which of two
files declaring the same name is reported as the first and which as the second.

## A declaration is code that ran

A declaration file is a chunk that **returns a table**. That is a different thing
from a document that was parsed, and the difference is usable: a block may
compute what it declares.

```luau
local ORE = "amber"

return {
	name = "example:" .. ORE,
	texture = "example:" .. ORE,
	solid = true,
}
```

That registers `example:amber` even though the string `example:amber` appears
nowhere in the file.

What the chunk must hand back is a **table**. A chunk that returns a function, a
number, or nothing at all is refused naming the file.

Your chunk runs in the same sandbox every other script in this engine runs in —
see [script-limits.md](script-limits.md) for the budget and the memory cap, and
[script-surface.md](script-surface.md) for what a chunk can and cannot reach. A
declaration calls nothing the engine provides: it returns a table and that is
all. There is no `mycraft.*` binding to use here.

## The nine fields

Three are required and six are optional.

| Field | Type | Required | Absent means | Bound |
|---|---|---|---|---|
| `name` | string, namespaced id | yes | — | 256 characters |
| `texture` | string, namespaced id, **or** a table of six facings | yes | — | 256 characters per key |
| `solid` | boolean | yes | — | — |
| `replaceable` | boolean | no | `false` | — |
| `breakable` | boolean | no | `true` | — |
| `breaks_into` | string, namespaced id | no | the cell is left empty | 256 characters |
| `drawn` | boolean | no | **whatever you wrote for `solid`** | — |
| `occludes` | boolean | no | **whatever you wrote for `solid`** | — |
| `targetable` | boolean | no | **whatever you wrote for `solid`** | — |

The last three are the only fields whose default is not a constant. Every other
absence means the same thing in every declaration; these three mean whatever the
*same* declaration said about `solid`. That is what lets a declaration written
before they existed go on meaning exactly what it meant — one field used to
answer all four questions at once.

```luau
return {
	name = "example:amber",        -- namespaced, required
	texture = "example:amber",     -- one key for all six faces, or a table of six
	solid = true,                  -- required boolean

	replaceable = false,           -- optional, absent means false
	breakable = true,              -- optional, absent means true
	breaks_into = "example:ash",   -- optional, absent means the cell empties

	drawn = true,                  -- optional, absent means whatever `solid` says
	occludes = true,               -- optional, absent means whatever `solid` says
	targetable = true,             -- optional, absent means whatever `solid` says
}
```

- **`name`** — the block's namespaced id. This is what the block is called
  everywhere: in other declarations, in save data, and in refusals.
- **`texture`** — a namespaced **key**, never a file path, either as one string
  for all six faces or as a table naming a key against each facing. What pixels a
  key resolves to is the renderer's concern, not a declaration's. It need not be
  the block's own name — see "A texture per face" and "Texture keys today", below.
- **`solid`** — does this block stop a player. See "Solidity is data", below.
- **`replaceable`** — may a placement overwrite this block. Absent means **not
  replaceable**, which is the conservative reading: forgetting the key loses you
  a placement rather than a block.
- **`breakable`** — can this block be broken at all. Absent means **breakable**,
  the opposite default from `replaceable` and deliberately so — a sandbox whose
  blocks were indestructible until each said otherwise would be the wrong burden
  to put on every content file.
- **`breaks_into`** — the namespaced id of the block a break leaves behind.
  Absent means the cell is left **empty**, which is a different claim from the
  block being indestructible, which is why they are different fields.
- **`drawn`** — is any face of this block emitted at all. Absent means whatever
  you wrote for `solid`. This is the field that lets a block be seen without
  stopping anybody, and the field that lets a block stop somebody without being
  seen.
- **`occludes`** — does this block hide the face of a neighbour that meets it.
  Absent means whatever you wrote for `solid`. Separate from `drawn` because a
  block may be seen *and* let you see what is behind it, which is the whole of
  what makes water look like water.
- **`targetable`** — can a swing find this block. Absent means whatever you wrote
  for `solid`. Whether the block then yields to that swing is `breakable`; this
  field decides only whether the swing arrives.

**The six optional fields are independent of one another.** A block may declare
`breakable = false` *and* a `breaks_into`; the residue is simply never reached,
and it is still there the day you make the block breakable again by editing one
line. The same holds across the three seeing fields: `drawn = true` on a
non-solid block says it is visible and says **nothing** about whether it hides
what is behind it or whether a swing can find it. Each of the three falls back to
`solid` on its own, so stating one does not state the other two.

**A residue is resolved when a break happens, not when the declaration is read.**
`breaks_into = "example:ash"` registers whether or not anything declares
`example:ash` — only the id's shape is checked here. That is what lets two mods
name each other's blocks without either having to load first.

**Optional means you may leave it out, never that any value will do.**
`replaceable = 1` is refused naming `replaceable`; it does not fall back to the
default. Falling back is the worst available outcome, because the block then
behaves exactly as it would have if you had written nothing at all — so there is
no symptom to notice and nothing to search for.

`drawn = 1` is refused the same way, and it is the one worth knowing about,
because a fall-back there would be invisible twice over. The default for `drawn`
is whatever you wrote for `solid` — so on a solid block a silently ignored
`drawn = 1` draws the block exactly as you intended, and you never learn the line
did nothing. The next declaration you write it in says `solid = false`, and then
your block is invisible for a reason nothing anywhere mentions. The refusal is
quoted in full under "Reading a refusal", below.

**A field the loader does not recognise is refused, not ignored.** A misspelled
`replacable` is a word anybody types once, and a loader that read the keys it
knows and never asked what else was there could not tell a typo from an absence.
The refusal names the field you wrote **and all nine you may write**, because a
name is only recognisable as a typo once you can see what it was nearly —
`drawnn` beside `drawn` explains itself where `drawnn` alone does not.

## The namespaced id rule

Every id — a `name`, a `texture` key, a `breaks_into` — follows one rule:
**exactly one `:`, with non-empty text on both sides.**

`example:amber:top` is refused outright, naming the field and the rule. It is
*not* quietly read as a block named `amber:top` inside namespace `example`:
splitting on the first colon would turn a typo into a plausible-looking id that
resolves to nothing, with no diagnostic pointing at what went wrong.

No character-set rule is enforced beyond that today, so uppercase letters and
unusual punctuation inside a namespace or a path are currently accepted. That is
deliberate rather than an oversight — a mod-id character set is a decision for
when mod ids become a real identity rather than a label, and guessing at one now
would be a compatibility promise made blind. The direction is strict-now,
permissive-later: a stricter rule can be relaxed without invalidating content
already written against it, and the reverse breaks everything.

## A texture per face

`texture` takes one of two forms and there is no third.

**One string** means all six faces draw that key. Every block the base game ships
states its texture this way, and it is what you want for a block that looks the
same all over:

```luau
texture = "example:amber",
```

**A table** names a key against each of the six facings, and it must state
**exactly** those six — no more, no fewer:

```luau
texture = {
	up = "example:grass_top",
	down = "example:dirt",
	north = "example:grass_side",
	south = "example:grass_side",
	east = "example:grass_side",
	west = "example:grass_side",
},
```

### The six words, and which way each of them points

The words are the declaration's, the axes are the world's, and this table is the
whole of the contract between them. Nothing is inferred from a block's name, its
shape or its position.

| Word | Axis | Which face |
|---|---|---|
| `up` | +Y | the top |
| `down` | −Y | the bottom |
| `north` | −Z | towards decreasing Z |
| `south` | +Z | towards increasing Z |
| `east` | +X | towards increasing X |
| `west` | −X | towards decreasing X |

The words are matched **exactly**. `Up` is not `up`, and `top` is not a facing at
all — both are refused rather than guessed at, because a table whose word was
quietly repaired would leave a face drawing a key you did not write.

### Naming one key twice is ordinary, and it is cheaper

A facing key is a key like any other, so two facings of one block may name the
same one and they then **share its array-texture layer**. The grass declaration
above states six facings and five distinct keys, and it spends five layers out of
[the session's budget](hot-reload.md#the-array-texture-layer-budget), not six.

There is no per-block allowance and no second pool: six facing keys come out of
the same 256 layers a single key comes out of, and a content root that would push
the session past the bound is refused whole, with every layer already assigned
still holding the key it held.

### A complete grass block

`content/example/blocks/grass.luau`, written out in full and ready to run:

```luau
-- Grass: turf on top, bare dirt underneath, and the same turf-over-dirt band on
-- all four sides. Five distinct keys across six faces.
return {
	name = "example:grass",
	texture = {
		up = "example:grass_top",
		down = "example:dirt",
		north = "example:grass_side",
		south = "example:grass_side",
		east = "example:grass_side",
		west = "example:grass_side",
	},
	solid = true,
	breaks_into = "example:dirt",
}
```

It registers, it spends five layers, and it draws: put one in the world and its
top, its bottom and its four sides each draw the key written against them. See
"Texture keys today", immediately below, for what those keys resolve to.

### Every refusal a texture table raises

Seven, and you can trip all of them on your first table. Each is quoted from a
real run.

**A table that leaves facings out names the ones you left out**, because that is
the edit you have to make:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `texture`: `texture` states no key for `south`, `east`, `west`; a texture table states all six of `up`, `down`, `north`, `south`, `east`, `west`
```

An empty table is the same refusal over all six:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `texture`: `texture` states no key for `up`, `down`, `north`, `south`, `east`, `west`; a texture table states all six of `up`, `down`, `north`, `south`, `east`, `west`
```

**A table carrying a name that is not a facing names the six you may write**,
because a word is only recognisable as a near miss once you can see what it was
nearly:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `texture`: `top` is not a facing a texture table may state; a texture table may state `up`, `down`, `north`, `south`, `east`, `west`
```

A capitalised facing is that same mistake, and is refused the same way — the
words are not case-folded:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `texture`: `Up` is not a facing a texture table may state; a texture table may state `up`, `down`, `north`, `south`, `east`, `west`
```

**A facing holding something that is not a string** is refused naming that
facing. The value is never rendered into the message: rendering it would run your
own `__tostring` at the moment the engine is reporting your mistake.

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `up`: `up` must be a string, but is a number
```

**A facing key that breaks the namespaced id rule** is refused by that rule, in
the same sentence you would get anywhere else an id is stated:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `north`: `base:grass:top` has more than one namespace separator — a namespaced id is written `namespace:path`
```

**A `texture` that is neither form** is refused naming both of them:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `texture`: `texture` must be a string or a table of six facings, but is a boolean
```

Two things these have in common with every other refusal on this page. A root is
refused **whole**, so a declaration with two mistakes in it is refused for
whichever the loader reaches first — fix that one and run again to see the next.
And a table's own metatable decides nothing: an `__index` supplying a facing you
did not write, or an `__iter` hiding a name you did, changes neither what the
loader sees nor what it refuses.

## Texture keys today

**A face draws the key its block declared against that facing, and never the key
its block's name spells.** Give a block a `texture` that is not its `name` and it
draws that texture; give it a table and each of its six faces draws its own. The
name is what the block is *called* and the key is what it is *drawn from*, and
nothing in the engine confuses the two.

**What a key resolves to in pixels depends on whether you have baked art for
it**, and both answers are ordinary:

- **A key your content root's built texture set covers** draws that key's image.
  You bake one by writing a voxel model, naming it in `textures.toml` against the
  key and a face of it, and running `voxforge build` —
  [`voxel-models.md`](voxel-models.md) is the whole of how, end to end.
- **A key nothing has baked** draws a **generated stand-in**: a two-colour
  pattern derived from the key's own spelling, deterministic and deliberately
  implausible. This is what your first block draws, it is never a refusal, and
  the client says so on startup so it does not read as something you did wrong.

The fallback is **per key**, not per set. One key baked and one not, in the same
content root, gives you one block drawing its art and its neighbour drawing a
stand-in — so you can add a model at a time.

Two things follow that are worth knowing before you name anything:

- **Two facings naming one key draw the same picture**, because they are one
  layer. That is true of art and of stand-ins alike.
- **Renaming a key you have not baked art for changes its colour**, because the
  stand-in comes from the spelling. Renaming one you *have* baked art for makes
  the set stale until you rebuild, and the client says so by name rather than
  drawing the old picture.

The base game's grass block is the worked example, and it is
`content/base/blocks/grass.luau` exactly as shipped:

```luau
return {
	name = "base:grass",
	texture = {
		up = "base:grass_top",
		down = "base:dirt",
		north = "base:grass_side_north",
		south = "base:grass_side_south",
		east = "base:grass_side_east",
		west = "base:grass_side_west",
	},
	solid = true,
}
```

Six facings and six distinct keys, because the model it is baked from is not
symmetrical and `content/base/textures.toml` bakes each of its four sides
separately. `down` names `base:dirt` — the block's underside *is* plain dirt, so
it shares the dirt block's image rather than having one of its own, and no dirt
model exists. That sharing costs a layer less than a seventh key would and is the
same mechanism "Naming one key twice is ordinary" describes, used for a reason
rather than for thrift.

### What happens when a key has no layer

A key the session could not assign a layer to — because the content root asked for
more than [the budget](hot-reload.md#the-array-texture-layer-budget) has left — is
**refused**, loudly, naming the block and the facing:

> the block `example:banded` draws nothing on its `north` face: the key
> `example:unlit` it declares there occupies no array layer

That refusal fails the whole section rather than drawing something else. Resolving
it to a fallback layer would draw whichever block owns that layer, which looks
entirely deliberate and is wrong. **A refusal on one facing is about that facing**
— the same block's other five keys still draw.

The held-block indicator says the same thing, once per run rather than once per
frame, and it names the facing it looked at — which is `north`:

> the held block `example:banded` draws no indicator: the key `example:unlit` it
> declares against `north` occupies no layer of the array texture

**The indicator looks at `north` and only at `north`**, on every block. A side
face is what makes a block recognisable in the hand — a grass block's side carries
both the turf and the earth, where its top is a green square that means "grass"
only to somebody who already knows.

## All-or-nothing loading

Loading a content root registers **every** declaration it holds or **none** of
them. A failure partway through — a chunk that will not compile, a duplicate
name, a bad field — leaves the registry exactly as it was, and never a partial
set of blocks.

A root that exists but declares **no blocks at all** is a refusal naming that
root, not an empty registry that loaded successfully. A root with no `blocks/`
directory is refused naming `<root>/blocks`.

## Reading a refusal

Every refusal names as much as it can: the **file**, the **block** as it named
itself, the **field** at fault, and the **cause**. Where there is nothing to
name — a chunk that never returned has no block to attribute — that part is
simply absent rather than guessed at.

A `blocks/amber.luau` declaring `slid = true` where it meant `solid` is refused
like this:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `slid`: `slid` is not a field a declaration may state; a declaration may state `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`, `drawn`, `occludes`, `targetable`
```

The parts read outermost first, separated by `: ` — the stage that failed, then
the file, then the block, then the field, then the cause.

The same refusal for a name one letter *past* a real field rather than one letter
short — `drawnn` for `drawn`, which is the typo the newest field on the list
invites:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `drawnn`: `drawnn` is not a field a declaration may state; a declaration may state `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`, `drawn`, `occludes`, `targetable`
```

A field that exists but holds the wrong kind of value is refused differently, and
that refusal is quoted under "Being seen is not being solid", below.

Three refusals whose shape is worth knowing before you meet them:

- **A chunk that will not compile** names the file and **the line the compiler
  named**, and no block — there is no declaration yet to attribute it to.
- **A chunk that raises an error of its own** names the file and the message you
  raised. `error("...")` in a declaration is a legitimate way to say a block
  cannot be declared, and what you said is what explains the refusal.
- **A name declared twice** names both files, in file-name order, and no field.

The file a refusal names is the **path you can open**, not the chunk name the
scripting host was given. Those are deliberately different things.

## The four bounds

Every quantity a content root supplies has a limit, and each accepts its own
limit and refuses the value above it.

| Bound | Limit | Refusal names |
|---|---|---|
| Declarations per content root | 4 096 | the `blocks/` directory, the count and the bound |
| Bytes per declaration file | 256 KiB | the file, its size and the bound |
| Characters per declared value | 256 | the field, the length and the bound |
| Fields per declaration | 64 | the file and the bound |
| Names in a texture table | 64 | the file and the bound |

Two things worth knowing about how they are checked:

- **The count is checked before any file is opened**, so a root of 4 097 files is
  refused on the count even when one of them is also broken.
- **A file's size is taken before it is read**, so an oversized file is refused
  on its size and not on whatever its text turned out to say.

Characters, not bytes: a 256-character id is accepted however many bytes it
takes to spell, so non-ASCII ids refuse at the length this page states rather
than at some shorter one. Each of a texture table's six keys is bounded on its
own by the same 256, and the refusal names the **facing** it was written against.

The last bound is a texture table's, and it is the same 64 a declaration's own
fields get, for the same reason: it bounds what the engine copies out of your
chunk. You will never meet it by writing six facings — it is there so that a
table with a thousand names in it is refused before it is read, rather than
after.

A declared value one character too long is refused like this — here a `texture`
key of 257 characters:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `texture`: `texture` holds 257 characters, and a declared value may hold at most 256
```

Both numbers are there on purpose: the one you wrote and the one you may write.
`name` and `breaks_into` are refused the same way, by the same bound.

Separately, what a declaration **prints** is bounded too. `print` works and is
useful while you are getting a declaration right; the host keeps the earliest
output up to its limit and then stops recording, and says how many lines it
stopped keeping. It does not refuse your declaration for printing too much.

## Being seen is not being solid

`solid` answers one question — does this block stop a player who walks into it —
and it used to answer three more by implication. Those three are now their own
fields:

| You want | Field |
|---|---|
| a block you can walk through but can see | `solid = false, drawn = true` |
| a block you can see through to what is behind | `occludes = false` |
| a block a swing passes straight through | `targetable = false` |

Each defaults to whatever the same declaration says about `solid`, so writing
none of them is writing what you always wrote. A block that is solid is drawn,
hides what is behind it and can be aimed at; a block that is not is none of the
three. **Stating one of them says nothing about the other two.**

Wrong kinds are refused rather than defaulted, and this is the refusal you will
meet if you reach for `1`:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `drawn`: `drawn` must be true or false, but is a number
```

Both halves are there on purpose: what the field accepts, and what it found.

### A pane you can see but walk through

A complete declaration, and the case the split exists for — visible, not solid,
and not hiding what is behind it:

```luau
-- A pane of amber glass. You can see it, you can see through it, and you can
-- walk straight into the cell it occupies.
return {
	name = "example:amber-pane",
	texture = "example:amber-pane",
	solid = false,
	drawn = true,
	occludes = false,
	targetable = true,
}
```

`solid = false` would once have made every one of the last three false with it,
which is why a block like this could not be declared at all. Read the four lines
as four separate answers: it does not stop you, it is drawn, it does not hide the
block behind it, and a swing can still find it — that last one being what lets
you break it, together with `breakable` defaulting to `true`.

Drop `targetable = false` in instead and you have a pane that cannot be broken by
aiming at it, whatever `breakable` says: `targetable` decides whether the swing
arrives, `breakable` decides what happens when it does.

**What consumes these three.** All three now act, and the pane above is a block
you can declare, see, walk through and break.

- **`drawn`** decides whether the mesher emits any face for your block. A block
  that is drawn and not solid is visible and walked through; one that is solid
  and not drawn stops you and is never seen.
- **`occludes`** decides whether your block hides the face of a neighbour that
  meets it, so `occludes = false` is what lets you see the block behind.
- **`targetable`** decides whether the walk a swing travels stops at your block.
  A ray stops at the first cell whose block declares it, and passes straight
  through everything else — including a block that stops a player.

**`targetable` is what makes `breakable` mean anything.** A swing has to arrive
before a block can refuse it, so a block declaring `targetable = false` is a block
whose `breakable` never comes up, whatever it says. That is not a special case:
it is the same two-claims-not-one shape as the rest of this section.

## Replaceability is not derived from solidity

**Placement reads `replaceable` and never consults `solid`.** Solidity is a
physics fact — does this block stop a player — and replaceability is a placement
rule — may a placement overwrite this block. A mod can declare a non-solid block
you cannot build through, and a solid block you can; the engine derives neither
from the other, because doing so would put a game rule in code that content is
supposed to own.

`base:water` is both non-solid *and* replaceable, but that pairing is not assumed
anywhere in the engine — each is its own declared fact.

**`replaceable` governs real blocks only.** An empty cell accepts a placement
because it is empty, not because content said so, and no declaration can make an
empty cell refuse one. Nothing is not content, so there is nothing there for a
content rule to be about.

**`replaceable` also decides *where* a placement lands, and this is the one place
in the engine it does more than permit.** A placement ordinarily goes in the cell
one step back from the one you are aiming at, on the side you are looking from —
so a block lands on the near face of what you aimed at rather than inside it. **If
the cell you aimed at is itself `replaceable`, the block goes into that cell
instead.**

It has to work that way, and the reason is worth a sentence because it explains
why the rule is not an inconsistency: the cell one step back is the cell the ray
was in immediately before it stopped, so if *that* cell had held something
replaceable, the ray would have stopped there instead. Without this rule there is
no aim at all that lands a block in a replaceable cell, and building through water
would have become impossible the moment water became aimable.

Everything else is asked about whichever cell was chosen, unchanged — a placement
into a replaceable cell your own body is standing in is still refused.

## There is no empty block

**A cell holds a block or holds nothing, and nothing names nothing.** The base
game declares no block meaning "empty", the engine knows no such block, and there
is no name you have to avoid, reserve or texture in order to describe empty
space.

- You declare only blocks that exist. There is nothing to write for the space
  between them.
- A break with no `breaks_into` leaves the cell empty. It does not leave behind
  some other block standing for emptiness.
- A name that *sounds* like empty space is an ordinary name. Declare
  `example:void` and it gets exactly the treatment every other block gets,
  including whatever solidity it declares.

## Solidity is data, not inference

Solidity is a **declared property**. Nothing about it is inferred from a block's
name or its runtime id, and no name and no id is special-cased anywhere in the
engine. An engine that treated some particular name as implicitly non-solid would
be writing a game rule into Rust that the base game's own mod-equivalent status
forbids: `base:stone`'s solidity is a fact `stone.luau` states, and your block is
solid or not for exactly the same reason and by exactly the same mechanism.

Declare a block named `example:air` as `solid = true` and cells holding it are
reported solid. Declare one named `example:stone` as `solid = false` and cells
holding it are reported non-solid.

**`drawn`, `occludes` and `targetable` are declared on exactly the same terms.**
Each is read from your declaration or defaulted from the `solid` in that same
declaration, and from nothing else. No name is treated as implicitly invisible,
implicitly transparent or implicitly unaimable-at — not `base:water`, and not
whatever you call yours.

## A complete example

A mod called `example` shipping two blocks: an ore that breaks into ash, and the
ash itself.

`content/example/blocks/amber.luau`:

```luau
-- Amber ore. Breaks into ash, which is declared beside it.
return {
	name = "example:amber",
	texture = "example:amber",
	solid = true,
	breaks_into = "example:ash",
}
```

`content/example/blocks/ash.luau`:

```luau
-- What amber leaves behind. Soft enough to build straight through.
return {
	name = "example:ash",
	texture = "example:ash",
	solid = true,
	replaceable = true,
}
```

Both register, and `amber.luau` sorts before `ash.luau`, so amber registers
first. If these were the only two blocks in the world, amber is what you would
find in your hand — the first solid block in registration order.

Note that `example:ash` is named by `amber.luau` before `ash.luau` has been read,
and that this is fine: a residue is resolved when a break happens.

## The base game's four blocks

`content/base/` ships exactly four declarations:

| File | `name` | `texture` | `solid` | `replaceable` | `breakable` | `breaks_into` | `drawn` | `occludes` | `targetable` |
|------|--------|-----------|---------|---------------|-------------|---------------|---------|------------|--------------|
| `dirt.luau` | `base:dirt` | `base:dirt` | `true` | *(absent)* | *(absent)* | *(absent)* | *(absent)* | *(absent)* | *(absent)* |
| `grass.luau` | `base:grass` | *(a table of six)* | `true` | *(absent)* | *(absent)* | *(absent)* | *(absent)* | *(absent)* | *(absent)* |
| `stone.luau` | `base:stone` | `base:stone` | `true` | *(absent)* | *(absent)* | *(absent)* | *(absent)* | *(absent)* | *(absent)* |
| `water.luau` | `base:water` | `base:water` | `false` | `true` | `false` | *(absent)* | `true` | `false` | `true` |

They are read in that order, which is why `base:dirt` — the first solid block —
is what a new player holds.

`base:water` is the only base block declaring `replaceable = true`, which is what
makes water placeable at all, and the only one declaring `breakable = false`. No
base block names a residue: breaking dirt, grass or stone leaves the cell empty.

`base:water` is also the only base block that states any of `drawn`, `occludes`
or `targetable`, and it is the reason those columns exist. Dirt, grass and stone
leave all three absent, so each reads its own `solid = true` and they are drawn,
occluding and targetable without saying so. Water is the case the split was for:
`solid = false` alone would have made it invisible, and it declares itself seen
(`drawn`), see-through (`occludes = false`) and aimable (`targetable`) against
that default. **A block that states none of the three behaves exactly as it did
before these fields existed** — that is what the defaults are for.

**Water's `breakable = false` is worth reading as a worked example, because it is
what a player now meets.** The ray a break travels stops at the first cell whose
block declares `targetable`, and water declares it — so a swing aimed at water
arrives, `breakable = false` refuses it, and the water stays in the cell with
whatever is behind it untouched. Take `targetable` away and the same declaration
goes inert: the swing passes through and breaks what is behind, which is exactly
what water did before it declared anything.

What it *does* change is every save in existence, and that is the lesson for your
own content: **`breakable` is one of the six fields a save folds into a block's
recorded behaviour**, alongside `name`, `solid`, `replaceable`, `breaks_into` and
`targetable`. Edit any of them and every existing world holding that block will
name it on the terminal at its next launch.

**Two different things produce that line, and telling them apart is the whole of
reading it.** A declaration you edited names the blocks *you* touched. An engine
build that added a field to the behaviour fold names **every block in the save**,
once, because the old record and the new one are folded over different field lists
and are not comparable at all. This build did the second: `targetable` joined the
fold, so the first launch of any world saved before it reports all four base
blocks together —

```
mycraft: `base:dirt`, `base:grass`, `base:stone`, `base:water` no longer behave as they did when this world was saved, and it was loaded anyway
```

— and the launch after that clean quit is back to naming only what you edit.

That is a notice and not a refusal — the world opens — and it is a one-shot: the
next clean quit rewrites the save against your new declaration and there is nothing
left to notice. Editing `texture` instead moves the *appearance* fold, which is
never reported. See `docs/technical/world-format.md` for the two folds and
`docs/modding/hot-reload.md` for the offline edit-and-relaunch loop.

`base:grass` is the only one stating a table, and it is the block the table form
exists for: six facings and six keys, five of them its own and one shared with
`base:dirt`. The other three state one string, so all six faces of each hold the
same key — the right shape for a block that looks the same all over.

**`texture` equal to `name` is a convention of three shipped files and never a
rule.** `base:grass` breaks it and nothing minds: a name is what a block is
called and a key is what it is drawn from.

## What is not here yet

Per-cell state, callbacks and components. Worldgen in script. Reading a second
content root. And `extends`, in every form — a declaration states its own nine
fields and inherits nothing.

**Declarations now reload while the game is running.** Save a file in `blocks/` and
the whole root is read again and taken up at the next tick boundary, or refused
whole. Which edits are visible, the one thing a reload cannot pick up, what survives
the swap and every refusal you can meet are on `hot-reload.md`.
