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

## The fifteen fields

Three are required and twelve are optional.

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
| `swimmable` | boolean | no | `false` | — |
| `move_resistance` | number | no | `0.0` | not less than zero, and at most `3.4e38` |
| `swim_ascent` | number | no | `9.0` | not less than zero, and at most `3.4e38` |
| `opacity` | number | no | `1.0` | not less than zero, and at most `1.0` |
| `tint` | string, a colour | no | no tint at all | `#RRGGBB` or `#RRGGBBAA` in either case, and an alpha other than `FF` is refused |
| `tint_distance` | number, in blocks | no | no tint at all | finite and greater than zero, and at most `3.4e38` |

`drawn`, `occludes` and `targetable` are the only fields whose default is not a
constant. Every other absence means the same thing in every declaration; those
three mean whatever the *same* declaration said about `solid`. That is what lets a
declaration written before they existed go on meaning exactly what it meant — one
field used to answer all four questions at once.

**The three medium fields are not among them, and deliberately not.** No field has
ever answered whether a volume can be swum in, how much it slows you or how fast
it lifts you, so a default derived from `solid` would invent a claim you never
made — and it would make every solid block in the game a wall you can float
inside. `swimmable` absent is `false`, `move_resistance` absent is `0.0` and
`swim_ascent` absent is `9.0`, on a solid block and a non-solid one alike.

**Nor are the two medium-colour fields, and their absence is not a value at
all.** There is no default tint and no default distance anywhere in the engine,
so a block either says what its volume carries a view toward or says nothing
about the matter. `tint` and `tint_distance` are stated **together or not at
all**, and stating one without the other is refused naming the one you are
missing — see "Seeing from inside a block", below.

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

	swimmable = false,             -- optional, absent means false
	move_resistance = 0.0,         -- optional, absent means 0.0
	swim_ascent = 9.0,             -- optional, absent means 9.0

	opacity = 1.0,                 -- optional, absent means 1.0

	tint = "#3A6EA5",              -- optional, absent means no tint at all
	tint_distance = 12.0,          -- optional, and required beside `tint`
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
- **`occludes`** — does this block hide the face of a neighbour that meets it,
  and does it stop the swing of a player standing *inside* it. Absent means
  whatever you wrote for `solid`. Separate from `drawn` because a block may be
  seen *and* let you see what is behind it, which is the whole of what makes
  water look like water.
- **`targetable`** — can a swing find this block. Absent means whatever you wrote
  for `solid`. Whether the block then yields to that swing is `breakable`; this
  field decides only whether the swing arrives.
- **`swimmable`** — can a player hold itself up inside this block's volume.
  Absent means **`false`**, a constant. This is what makes a volume something to
  swim in rather than something to fall through; a block that says nothing is
  something you fall through.
- **`move_resistance`** — how much this block's volume slows what moves through
  it. Absent means **`0.0`**, which is exactly "unaffected". See "Declaring a
  medium", below, for the scale and what it is refused for.
- **`swim_ascent`** — how fast this block's volume lifts a swimmer who holds
  jump. Absent means **`9.0`**, the speed your own jump leaves the ground at, so
  a declaration written before this field existed lifts exactly as it did. See
  "Declaring a medium", below, for what you can predict from the number.
- **`opacity`** — how much of the light reaching this block it stops. Absent
  means **`1.0`**, a constant: all of it, which is what every block did before
  the field existed. `0.0` stops none, both ends are inclusive, and a block that
  states anything below `1.0` must also say it does not occlude. See "Seeing
  through a block", below.

**The ten optional fields are independent of one another**, with one stated
exception: a degree below `1.0` cannot stand beside occlusion, because those two
ask for opposite things. That refusal is quoted under "Seeing through a block". A block may declare
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
The refusal names the field you wrote **and all fifteen you may write**, because a
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
  implausible. This is what your first block draws and it is never a refusal.
  **The launch names it** — every uncovered key, ascending, on the error stream —
  so it does not read as something you did wrong and you do not have to spot the
  checkerboard yourself. A launch covering every declared key says nothing at all.
  [`voxel-models.md`](voxel-models.md) has the line and what to do about it.

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
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `slid`: `slid` is not a field a declaration may state; a declaration may state `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`, `drawn`, `occludes`, `targetable`, `swimmable`, `move_resistance`, `swim_ascent`, `opacity`, `tint`, `tint_distance`
```

The parts read outermost first, separated by `: ` — the stage that failed, then
the file, then the block, then the field, then the cause.

The same refusal for a name one letter *past* a real field rather than one letter
short — `drawnn` for `drawn`, which is the typo the newest field on the list
invites:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `drawnn`: `drawnn` is not a field a declaration may state; a declaration may state `name`, `texture`, `solid`, `replaceable`, `breakable`, `breaks_into`, `drawn`, `occludes`, `targetable`, `swimmable`, `move_resistance`, `swim_ascent`, `opacity`, `tint`, `tint_distance`
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
- **`occludes`** decides two things. It decides whether your block hides the face
  of a neighbour that meets it, so `occludes = false` is what lets you see the
  block behind. It also decides what happens to the swing of a player whose eye
  is **inside** your block: a block that can be seen through is passed over, and
  one that cannot is what that player is aiming at.
- **`targetable`** decides whether the walk a swing travels stops at your block.
  A ray stops at the first cell whose block declares it, and passes straight
  through everything else — including a block that stops a player.

**The cell a player's eye is inside is judged by both fields, and every other
cell by `targetable` alone.** That distinction only exists for a block somebody
can stand inside, which means a block declaring `solid = false` — and it only
matters for one that is also `targetable`. Water is exactly that block, and
getting it wrong is what made a swimmer unable to interact with anything: while
the eye's own cell was judged by `targetable` alone, every swing a swimmer made
found the water their head was in, and every placement was refused for want of a
face to build against.

So if you declare a block a player can walk into and aim at:

- `occludes = false` — standing in it, they aim **through** it at whatever is
  beyond. This is water, mist, a force field.
- `occludes = true` — standing in it, it **is** what they are aiming at, reported
  at no distance and with no face, so a swing can break it and a placement is
  refused for want of a face. This is a block that fills your view from inside.

Neither is a fault and neither prints anything; they are two designs and the
field is how you say which one you meant.

**`targetable` is what makes `breakable` mean anything.** A swing has to arrive
before a block can refuse it, so a block declaring `targetable = false` is a block
whose `breakable` never comes up, whatever it says. That is not a special case:
it is the same two-claims-not-one shape as the rest of this section.

## Seeing through a block

`occludes = false` lets the engine *draw* what stands behind your block.
`opacity` decides how much of it you actually see.

It is a **degree**, and it runs the way its name does: `1.0` stops all the light
and is what every block that says nothing means, `0.0` stops none, and anything
between is a partial. Both ends are inclusive, so `opacity = 0.0` and
`opacity = 1.0` are both declarations rather than mistakes.

**A block at `1.0` draws opaque however its texture's alpha reads.** Whether a
face is drawn blended at all is decided by this number and by nothing else, so an
image with a soft edge in a block that never states `opacity` draws exactly as it
always did. That is deliberate: what your block does to the light is something
you write down and can change while the game runs, not something an art tool
decides for you. Within a block that *does* state a degree below one, the
texture's own alpha still varies across the face — the two multiply — so a
stained-glass image works, one texel at a time.

### A pane you can see through

```luau
return {
  name = "example:glass",
  texture = "example:glass",
  solid = true,
  occludes = false,
  opacity = 0.4,
}
```

Solid, so you cannot walk through it. Not occluding, so the faces behind it are
built. `opacity = 0.4`, so six tenths of what is behind reaches your eye. Drop
that to `0.05` and the pane is very nearly a window; raise it to `1.0` and you
have an ordinary opaque block that happens to be spelled out.

### The bound, and what a value past it says

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `opacity`: `opacity` may not be more than one
```

Not clamped. `1.5` silently reduced to `1.0` is a block that draws correctly and
teaches you your scale runs to a hundred, so you write `100` on the next block
and meet the same wrong picture with no refusal to explain it. The floor is
worded the same way — `` `opacity` may not be less than zero `` — because they
are the two ends of one sentence. And a value that is not a number at all is
refused for *that* rather than for a bound: `math.huge` is greater than `1.0`, so
a ceiling checked first would send you looking for a smaller number when what you
wrote has no smaller spelling.

### You cannot pass light and hide what is behind you

These two lines ask for opposite things, and the engine refuses rather than
picking one:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `opacity`: `opacity` below one cannot be stated with `occludes = true`: a block light passes through cannot also hide what lies beyond it
```

`occludes = true` suppresses the face of whatever your block meets, so there is
nothing left behind it to show through. Delete that line and the pane works.

**And you can hit this without ever writing `occludes`.** It falls back to
whatever you wrote for `solid`, so `solid = true` on its own already says your
block hides what is behind it — which is why the refusal names the line that
actually did it:

```
mycraft: the shipped content could not be read: content/base/blocks/amber.luau, block `example:amber`, field `opacity`: `opacity` below one cannot be stated with `occludes = true`, and this block occludes by stating `solid = true` and no `occludes`: a block light passes through cannot also hide what lies beyond it
```

The remedy is the opposite one: a line to **add**, not to delete. Write
`occludes = false` beside your degree, as the pane above does. That is the whole
difference between the two sentences, and it is why there are two.

## Seeing from inside a block

`opacity` says how much of what is behind your block reaches an eye standing
**outside** it. `tint` and `tint_distance` say what the world looks like to an
eye standing **inside** it — inside the block's own cell, which is where you are
when you are under water.

They are two claims about two different views, and neither is derived from the
other. A block that stops all the light may still declare a tint, and it is not a
contradiction: your block's own faces point away along every ray leaving an eye
that stands in it, so what such a block does is draw the whole frame at the
colour you named.

**`tint`** is the colour a view through the volume is carried toward. **Absent
means no tint at all.**

**`tint_distance`** is how far a surface stands from the eye before it is drawn
wholly at that colour, in blocks. **Absent means no tint at all.**

A surface at half that distance is drawn halfway toward the colour, one at a
tenth of it a tenth of the way, and anything at or beyond it is drawn at the
colour outright. Nothing fades further than the colour: the ramp stops where
your distance says it does.

### Both, or neither

There is no default colour and no default distance anywhere in this engine, so
half a medium is not something the engine will complete for you. A declaration
stating one field and not the other is refused **naming the one that is
missing**, because that is the line you have to add:

| You write | The refusal |
|---|---|
| `tint` with no `tint_distance` | `` `tint_distance` is required beside `tint`: a colour with no distance does not say how far this medium lets an eye see `` |
| `tint_distance` with no `tint` | `` `tint` is required beside `tint_distance`: a distance with no colour does not say what this medium carries a view toward `` |

### Writing the colour

Either form, in either case: `#RRGGBB`, or `#RRGGBBAA` whose alpha is `FF`. So
`#3A6EA5`, `#3a6ea5` and `#3A6EA5FF` are three spellings of one colour and
register as one value — which matters beyond taste, because a save folds the
colour you declared, and three spellings hashing apart would tell every player
holding that block that it had been retextured.

**Both forms are accepted because both are already written in this tree.** A
material file spells a colour `#rrggbb` and a HUD file spells one `#RRGGBBAA`,
so whichever you copied from, your block works.

**A colour states no alpha.** How strongly the medium acts is how far it lets you
see, which is the other field — so an eight-digit colour is a *form* this field
takes and an alpha below `FF` is a *value* it refuses:

| You write | The refusal |
|---|---|
| `tint = 5` | `` `tint` must be a colour string, but is a number `` |
| `tint = "#GG0000"` | `` `tint` must be written `#RRGGBB` or `#RRGGBBAA`, in upper case or lower `` |
| `tint = "3A6EA5"` | the same — the lead is part of the form |
| `tint = "#3A6EA"` | the same — seven digits is neither form |
| `tint = "#3A6EA580"` | `` `tint` states no alpha: how strongly a medium acts is `tint_distance`, so an eight-digit colour must end `FF` `` |

That last one is its own refusal on purpose. Every character of `#3A6EA580` is a
hexadecimal digit and its length is one this field accepts, so it is not a
malformed colour and you are not told it is: told that, you would edit it down to
six digits, lose the strength you were reaching for, and never learn which field
carries it.

### Writing the distance

It is read by the same rules the two numbers under "Declaring a medium" are —
both of Luau's ways of writing a number, the same `3.4e38` ceiling, the same
refusal for a value that is not finite — **with one difference that is the only
exclusive floor on this declaration**:

| You write | The refusal |
|---|---|
| `tint_distance = 0.0` | `` `tint_distance` must be greater than zero `` |
| `tint_distance = -1.0` | the same |
| `tint_distance = math.huge` | `` `tint_distance` must be a finite number `` |
| `tint_distance = 0/0` | the same |
| `tint_distance = 1e40` | the same — past the width the engine keeps |
| `tint_distance = "far"` | `` `tint_distance` must be a number, but is a string `` |

Zero is refused rather than admitted, which is the opposite of every other number
here: a resistance of zero is "unaffected" and an opacity of zero is a pane with
no glass in it, but a medium reaching full strength at *no distance at all* would
hide everything including itself. If you want a volume that shows nothing, write
a very small distance — `0.001` is a legal declaration and does exactly that.

Note which refusal `math.huge` gets. Finiteness is asked before the floor, and it
has to be: an infinity passes a "greater than zero" test, so a floor checked
first would admit `math.huge` outright — and a NaN fails one, so the same wrong
order would tell you your `0/0` was too small.

### A worked example: water you can see a little way through

```luau
return {
	name = "example:pool",
	texture = "example:pool",
	solid = false,

	drawn = true,
	occludes = false,
	targetable = false,

	swimmable = true,
	move_resistance = 3.0,
	swim_ascent = 6.0,

	opacity = 0.5,
	tint = "#3A6EA5",
	tint_distance = 12.0,
}
```

Stand outside it and you see half of what is behind it, in whatever your texture
looks like. Stand *inside* it — put your eye in one of its cells — and everything
you look at is carried toward `#3A6EA5`, by how far away it is: a face two blocks
off is barely touched, one six blocks off is drawn halfway there, and anything
twelve blocks off or further is that blue and nothing else. A pixel with no
surface in it at all is that blue too, so the volume closes over the sky rather
than leaving a hole in it.

Lower `tint_distance` and the water thickens; raise it and it clears. Both of
those are edits you can make while the game is running — see
[hot-reload.md](hot-reload.md).

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

## Declaring a medium

Three fields say what a block's volume is to something moving *through* it, as
opposed to what it looks like or whether it stops you. Together they are what
makes a volume a **medium** rather than an absence.

**`swimmable`** says a player can hold itself up in the volume. **Absent means
`false`.**

**`move_resistance`** says how much the volume slows what moves through it.
**Absent means `0.0`.** The scale is a divisor: a speed through the volume is
divided by `1 + move_resistance`, so

| You write | What happens to a movement through it |
|---|---|
| `0.0`, or nothing at all | unaffected — the speed it has in air |
| `1.0` | half speed |
| `3.0` | a quarter speed |
| `9.0` | a tenth speed |

It is the first **number** a declaration may state, and it is checked rather than
coerced. A value is refused, naming `move_resistance`, when it is:

- **less than zero** — `move_resistance = -1` is refused with
  `` `move_resistance` may not be less than zero ``. There is nothing below
  "unaffected" for a declaration to mean, and a negative divisor would make the
  volume a place that speeds you up.
- **not a finite number** — `move_resistance = 0/0` and `move_resistance = 1/0`
  are both expressions Luau will evaluate for you, and both are refused with
  `` `move_resistance` must be a finite number ``.
- **larger than the engine can keep** — refused with the same sentence, because
  that is what it becomes. **The ceiling is `3.4e38`**, the largest finite value
  at the width the tick divides by; a declaration past it is an infinity by the
  time the engine holds it, and handing the physics an infinity nobody wrote is
  the silent coercion this field refuses everywhere else. Measured against the
  shipped loader: `1e30` and `3.4e38` register; `3.5e38` and `1e40` are refused,
  naming `move_resistance` and saying it must be a finite number.

  There is no *policy* ceiling — nothing here decides that some resistance is
  too much to mean. `1e30` is a block nothing can walk through and it registers.
  The only ceiling is the one the retained width imposes, and it is stated
  rather than left for you to discover.
- **not a number at all** — `move_resistance = true` is refused with
  `` `move_resistance` must be a number, but is a boolean ``, and
  `move_resistance = "1.0"` with `` …but is a string ``. Text that looks like a
  number is never parsed; see "Optional means you may leave it out", above, for
  why nothing on a declaration is coerced.

Write the number either way Luau writes one: `move_resistance = 4` and
`move_resistance = 4.5` are both accepted, and `4` registers as `4.0`.

**`swim_ascent`** says how fast the volume lifts a swimmer holding jump.
**Absent means `9.0`**, the speed your own jump leaves the ground at — so a
declaration written before this field existed lifts exactly as it always did.

It is read by the same rules as `move_resistance`: the same floor, the same
`3.4e38` ceiling, the same four refusals, and both of Luau's ways of writing a
number, so `swim_ascent = 4` registers as `4.0`. Each refusal names
`swim_ascent`:

| You write | The refusal |
|---|---|
| `swim_ascent = -1` | `` `swim_ascent` may not be less than zero `` |
| `swim_ascent = 0/0` | `` `swim_ascent` must be a finite number `` |
| `swim_ascent = math.huge` | `` `swim_ascent` must be a finite number `` |
| `swim_ascent = 1e40` | `` `swim_ascent` must be a finite number `` — past the width the engine keeps |
| `swim_ascent = true` | `` `swim_ascent` must be a number, but is a boolean `` |
| `swim_ascent = "3.5"` | `` `swim_ascent` must be a number, but is a string `` |

**A stated `0` is not an absence.** `swim_ascent = 0.0` declares a volume you
can be inside and cannot climb: holding jump does nothing whatever. That is a
different claim from leaving the field out, which lifts you at `9.0`. It is the
one field on a declaration where saying nothing and writing the smallest value
you may write mean opposite things, so it is worth writing the `0.0` out on
purpose rather than trusting a reader to see that you meant it.

**The three are independent in every direction.** A volume may resist without
being one you can swim in; one you can swim in need not resist at all; and a
block may state a `swim_ascent` while saying nothing about `swimmable`, in
which case the number is registered exactly as written and lifts nobody —
because there is nobody it holds up. Nothing here is derived from anything
else, so a block that states one of the three has said *nothing* about the
other two.

### What the numbers do, before you run them

Both numbers are read **once per tick, and the tick is 60 Hz**. That is what
makes them predictable rather than something to discover by swimming. A tick of
a swimmer holding jump ends at

```
rise = (swim_ascent - 0.5) / (1 + move_resistance)
```

and a tick of a swimmer holding a movement key ends at

```
horizontal = 4.5 / (1 + move_resistance)
```

where `0.5` is what gravity takes in one tick at 60 Hz and `4.5` is the speed
you walk on land. **The rise does not accumulate**: the ascent is re-applied
from scratch every tick, so the first tick in the water is already the whole of
it, and holding jump longer gets you there sooner rather than faster.

**The number worth picking first is the ratio of the two**, because
`move_resistance` divides both and cancels out of it completely:

```
rise / horizontal = (swim_ascent - 0.5) / 4.5
```

So `swim_ascent` alone decides whether your liquid climbs faster than it
carries you along, whatever you do to the resistance afterwards — and it is the
one thing about your medium's feel you cannot read off the field table.

| `swim_ascent` | rise / horizontal | What it feels like |
|---|---|---|
| `9.0`, or nothing at all | 1.89 | you climb nearly twice as fast as you swim along |
| `5.0` | 1.00 | up and along at the same speed |
| `3.5` | 0.67 | you swim along half again as fast as you climb |
| `0.5` | 0.00 | you hold your depth: jump exactly cancels one tick of gravity |
| `0.0` | −0.11 | jump buys you nothing at all and you sink anyway |

Pick the ratio from `swim_ascent`, then pick `move_resistance` for how fast the
whole thing happens. They are independent in exactly that way, which is why
they are two fields.

### A worked example: a resistant block that is not swimmable

`content/example/blocks/tar.luau`:

```luau
-- Tar. You wade through it at a quarter speed and you sink: it slows you down
-- without holding you up, which is the half of a medium `swimmable` is not.
return {
	name = "example:tar",
	texture = "example:tar",
	solid = false,

	drawn = true,                  -- non-solid, so this would otherwise default to false
	occludes = false,              -- you can see what is behind it
	targetable = true,             -- a swing finds it

	move_resistance = 3.0,         -- a quarter of your speed in air
	-- `swimmable` is not stated, so it is `false`: no jump lifts you here
}
```

Drop into a pool of it and you fall to the bottom four times more slowly than you
fall through air, and holding jump does nothing. Declare `swimmable = true`
alongside and the same pool becomes something you can swim up out of, at the same
reduced speed — which is what `base:water` does.

### A worked example: a liquid you swim along faster than you climb

`content/example/blocks/brine.luau`. Every field it needs is stated; drop the
file into a content root and it loads.

```luau
-- Brine. Denser than water: it carries you along well and lets you climb out
-- slowly, which is the pairing `swim_ascent` exists to let you write.
return {
	name = "example:brine",
	texture = "example:brine",
	solid = false,                 -- required, and `false` is what makes it a volume
	                               -- you enter rather than a wall you stop at

	drawn = true,                  -- non-solid, so this would otherwise default to false
	occludes = false,              -- you can see what is behind it
	targetable = false,            -- a swing passes straight through

	swimmable = true,              -- you can hold yourself up in it
	move_resistance = 0.5,         -- two thirds of your speed in air
	swim_ascent = 3.5,             -- ratio (3.5 - 0.5) / 4.5 = 0.67
}
```

What that declaration is worth, before you load it: holding a movement key
carries you along at `4.5 / 1.5` = **3.0 blocks per second**, two thirds of your
walking speed, and holding jump lifts you at `(3.5 - 0.5) / 1.5` = **2.0 blocks
per second**. You swim along half again as fast as you climb, and a four-block
pool takes two seconds to rise out of.

Change `move_resistance` to `0.0` and both numbers rise together — `4.5` along
and `3.0` up — and the ratio is still `0.67`. That is the cancellation above,
and it is why you tune the two fields for different things.

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
own content: **`breakable` is one of the eight fields a save folds into a block's
recorded behaviour**, alongside `name`, `solid`, `replaceable`, `breaks_into`,
`targetable`, `swimmable` and `move_resistance`. Edit any of them and every existing world holding that block will
name it on the terminal at its next launch.

**Two different things produce that line, and telling them apart is the whole of
reading it.** A declaration you edited names the blocks *you* touched. An engine
build that added a field to the behaviour fold names **every block in the save**,
once, because the old record and the new one are folded over different field lists
and are not comparable at all. This build did the second: `swimmable` and
`move_resistance` joined the fold, so the first launch of any world saved before
it reports all four base blocks together —

```
mycraft: `base:dirt`, `base:grass`, `base:stone`, `base:water` no longer behave as they did when this world was saved, and it was loaded anyway
```

— and the launch after that clean quit is back to naming only what you edit.

That is a notice and not a refusal — the world opens — and it is a one-shot: the
next clean quit rewrites the save against your new declaration and there is nothing
left to notice. Editing `texture` instead moves the *appearance* fold, which is
never reported. See `docs/technical/world-format.md` for the two folds and
`docs/modding/hot-reload.md` for the offline edit-and-relaunch loop.

**One-shot means once per move, and the fold has now grown twice running.** The
build before this one added `targetable`; this one adds the two medium fields. So
a world that already crossed the first move, and was quit normally afterwards, is
told a second time on its first launch here — the same line, naming the same four
blocks, for a different growth of the same list. Nothing has gone wrong and there
is nothing to fix: the two records were folded over different lists and no reading
of them could say the blocks are unchanged. Expect one report per fold growth your
players cross, not one report ever.

`base:grass` is the only one stating a table, and it is the block the table form
exists for: six facings and six keys, five of them its own and one shared with
`base:dirt`. The other three state one string, so all six faces of each hold the
same key — the right shape for a block that looks the same all over.

**`texture` equal to `name` is a convention of three shipped files and never a
rule.** `base:grass` breaks it and nothing minds: a name is what a block is
called and a key is what it is drawn from.

## What is not here yet

Per-cell state, callbacks and components. Worldgen in script. Reading a second
content root. And `extends`, in every form — a declaration states its own fifteen
fields and inherits nothing.

**Declarations now reload while the game is running.** Save a file in `blocks/` and
the whole root is read again and taken up at the next tick boundary, or refused
whole. Which edits are visible, the one thing a reload cannot pick up, what survives
the swap and every refusal you can meet are on `hot-reload.md`.
