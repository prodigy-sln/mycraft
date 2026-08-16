# The sandbox: what a script may reach, and what stops it

Every line of content runs inside one sandboxed Luau state. The state is
closed before any chunk of yours is evaluated, every entry into it is
budgeted and memory-capped, and a callback that keeps failing stops being
invoked. None of that is special to third-party mods — the base game has no
privileged path into the host, and this document is the whole of what any
content can do.

## What this document is, and what content can do today

This page describes the **environment a script runs in** — what it may reach,
what stops it, and what a failure looks like when a limit is exceeded. It is not
an authoring API reference, because there is not yet an authoring API to
reference.

**Nothing can be authored in Luau today.** There is no `mycraft.*` binding of
any kind: no block, world, entity or registry access, and no script-callable way
to declare a component, give it a name, or attach it to a subject. That surface
is Rust — the engine evaluates a chunk, takes the value it returns, and decides
what to attach it to — and content has no path into it. Block definitions still
live in the data files under `content/base/`, which is where a block is declared
and the only place one can be.

**So if you came here to make something, this is the wrong page and `README.md`
is the right one.** Authoring works today and it is data files: a block, a HUD
element, a voxel model. `README.md` takes you from a clean checkout to a block of
your own, visible in the game, in one file and one command. Come back here when
you want to know what the ground will be like under Luau — none of which you
need in order to write content now.

The authoring surface arrives in two steps, and both are additions behind what
is written here rather than changes to it. **Defining blocks in Luau** comes
first and retires the data-file loader. **Attaching behaviour to a subject** —
the thing every limit below is charged against — comes after it, in the same
milestone.

The order is deliberate. The limits, the fault vocabulary and the sandbox
surface are what a binding is added *behind*, and settling them first is what
makes a capability judged against enforcement that already exists rather than
hardened afterwards. So this page belongs in the authoring documentation even
now: the budget, the memory cap, the denied globals and the way an abort behaves
are the constraints every line of content will be written under, and none of
them will move to accommodate a binding.

## The shape of a script

A chunk is handed to the host as source plus a name to report it under — the
name is a label, not a path, and the host never opens a file. Whatever name
a mod author would recognise is the right one, because it is what a fault
points at.

A chunk is evaluated once and its return value is kept. To have behaviour
invoked later, return a function:

```lua
local heat = 0
return function()
    heat = heat + 1
    return heat
end
```

The host registers that function against an **attachment**: a `(subject,
component)` pair, both opaque namespaced strings. The subject is the thing
behaviour is attached to and the component is the behaviour attached to it.
The pair is the unit of everything below — the budget is charged to it, faults
are counted against it, and quarantine acts on it. Two mods attaching
different components to one subject are independent; the same component on two
subjects is likewise two attachments.

Callbacks are invoked in **dispatch rounds**. A callback may ask for more work
in the same cascade by returning a table with a `follow_up` array:

```lua
return function()
    return { follow_up = { { subject = "stone-furnace", component = "vent" } } }
end
```

Follow-up work is **queued, never entered inline**, so a cascade converts into
queue length rather than into call depth.

### The returned value, field by field

This is the whole contract between your callback and the engine. Everything in
it is read **raw** — no metatable of yours is ever consulted — so `follow_up`
cannot be supplied through an `__index`, and the host cannot be made to run your
code while it reads.

| What | Type | Bound | If it is anything else |
|---|---|---|---|
| the return value | any | — | anything that is not a table ends the invocation with no follow-up. Returning nothing is normal and is not a failure. |
| `follow_up` | table | up to the pending bound, counted across every requester | a missing or non-table `follow_up` means no follow-up work. No signal. |
| each slot of `follow_up` | table | slots `1..n`, contiguous from 1 | a slot that is not a table is skipped. No signal. |
| `subject` | string | non-empty by convention; the host stores it without checking | a slot the host cannot read a string out of is skipped. No signal. |
| `component` | string | same | same. |

Only the array part is read, from slot 1 upwards, so a `follow_up` table keyed
by anything other than `1, 2, 3, …` is invisible — and a hole in the middle ends
the list early rather than skipping past it.

**Every malformed shape above is passed over silently, and you get nothing back
saying so.** Nothing is fabricated in its place — the host will not guess at an
attachment you did not name — but nothing is reported either, so a misspelled
`componant` key is work that quietly never runs, and the symptom is a mod that
does nothing rather than a mod that complains. Until that has a diagnostic,
**print what you are about to request while you are developing** and check it
against what actually ran:

```lua
return function()
    local next_up = { subject = "stone-furnace", component = "vent" }
    print(("requesting %s / %s"):format(next_up.subject, next_up.component))
    return { follow_up = { next_up } }
end
```

If that line appears and the vent's own callback never runs, the entry was
malformed or its target was quarantined — those are the two silent cases, and
they are the first two things to check.

## The reachable surface, exactly

The host declares the names a chunk can reach, and the declared set is compared
against what a chunk can *really* reach in both directions — a name reachable
but undeclared is reported, and so is a name declared but gone. That is the
guard against a capability arriving unannounced in a future Luau or `mlua`
release, which a deny list alone cannot see.

**Reachable — 31 names:**

```
_G          _VERSION    assert      bit32       buffer      coroutine
error       getmetatable            integer     ipairs      math
next        pairs       pcall       print       rawequal    rawget
rawlen      rawset      select      setmetatable            string
table       tonumber    tostring    type        typeof      unpack
utf8        vector      xpcall
```

**Denied — 14 names**, removed by the host itself before the sandbox is closed:

```
io          os          package     require     loadstring  load
dofile      loadfile    debug       getfenv     setfenv     collectgarbage
newproxy    gcinfo
```

Closing Luau's own sandbox removes five of those fourteen and leaves the rest
standing, so the host does not rely on which half is which and removes all
fourteen. Four of the denials are worth understanding rather than memorising:

- **`getfenv` / `setfenv`** are the escape pair. Either one hands a chunk
  another chunk's environment, which is the isolation everything else here
  rests on.
- **`collectgarbage`** drives the collector, which is a memory side channel
  and a way to move the ground under everybody else's allocation.
- **`newproxy`** is a `__gc` vector: a finaliser is script the host never asked
  to run, at a moment the host did not choose.
- **`gcinfo` is denied for determinism**, not for secrecy. It reports the heap
  size, so a script branching on it is a function of the collector's state
  rather than of its own inputs. World generation is the caller that cannot
  survive that: a generator branching on heap size returns different terrain
  for one seed, which loses a world rather than leaking a number. It is
  `collectgarbage`'s objection from the other direction — one reads the
  collector, one drives it.

`require` is **absent rather than confined**. There is no mod-directory concept
to confine it to, so a mod is one chunk; multi-file mods and a confined
`require` arrive together, with path traversal out of a mod's own directory
treated as the security question it is.

**`print` reaches the host, and only the host.** Luau's own `print` writes to C
`stdout` — a different buffer, outside every log the host controls — so the
host installs its own **before** the sandbox is closed. The ordering is the
rule: installing or removing a global after the sandbox is closed silently
succeeds and changes nothing, because a write to a global then lands in a child
table and removes nothing. Everything a chunk prints is collected by the host,
in order, as **unbounded text your mod chose**; whoever routes it to a log
inherits both its content and its length.

## The environment is frozen, so you write locals

Each chunk is evaluated against a fresh table of its own, frozen, whose
metatable reads through to the shared globals. Three consequences:

- **A global assignment is an error**, not a silently ignored write.
  `heat = 0` at the top of a chunk aborts that chunk at the assignment, naming
  it. Write `local heat = 0`.
- **`rawset` does not help.** `rawset(_G, "smuggled", 1)` is refused where it
  lands, because the table is frozen rather than guarded by a metamethod a
  write can decline to consult.
- **Shared library tables are frozen too.** Assigning to `string.format`, or
  hanging a new `__index` on the shared string metatable, is refused; the next
  chunk observes the original behaviour.

The environment is per chunk rather than shared, and that is containment rather
than tidiness: with one shared environment, a mod writing `print = function()
end` silences every other mod on the server. With its own environment, that
write is refused *and* would have been private if it had not been.

The refusal happens at the assignment, so a chunk that replaces a global and
then calls it never reaches the call at all. If you want to observe the refusal
without losing the rest of the chunk, wrap it: `pcall(function() print = f end)`
returns `false`, and the `print` that follows still reaches the host.

## The limits that ship

A host constructed without configuration runs these. A server operator can
raise or lower any of them; these are the values that apply when nobody does.

| Limit | Default | What exceeding it does | What you do about it |
|---|---|---|---|
| Call-and-loop budget | 1,000,000 ticks | Aborts one entry into script — a chunk evaluation or one callback invocation — with *call and loop budget exhausted*. | Make fewer calls, or slice the work across invocations. Shortening the code does nothing; see the next section. |
| Memory cap | 256 KiB | Aborts one entry that adds more than this **above the baseline it started from**, with *allocation refused*. | Hold less at once. Build into one buffer you reuse rather than growing a table per invocation. |
| Memory backstop | 16 MiB | The ceiling the whole state may reach, allocator-enforced. Approaching it stops faults naming anybody — see host memory pressure below. | Retain less across invocations. This is the limit your *previous* invocations spend, not this one. |
| Fault threshold | 3 | Consecutive faults on one attachment stop it being invoked at all. | Fix the callback and re-attach it, which clears the quarantine. |
| Round bound | 64 | Invocations one round performs before the rest waits for the next round; the overflow is reported as *cascade deferred*. | Nothing — deferred work runs, and one deferral per round is what a healthy long cascade looks like. |
| Pending bound | 256 | Entries of follow-up work that may be waiting at once. Work past it is **refused and named**, and never runs. | Request less fan-out per invocation. A refusal is work you lost, not work that waited. |

Every one of these is a non-zero value — none can be configured to zero or to
"unlimited", so there is no setting under which a limit is off. The two memory
figures are also constrained against each other: the backstop must leave room
above the state's own baseline for a whole memory cap, and a host configured
otherwise **refuses to start** rather than running with every fault blamed on
its own configuration:

```text
the scripting host could not start: the absolute memory backstop of 524288 bytes
leaves no room for one invocation: the state holds 385952 bytes before any
content runs, and each invocation may add 262144
```

The three byte counts are that host's own — the middle one is what its state
weighed before any content ran, so it moves with the backend rather than being a
figure to memorise. This is an operator's error rather than an author's, but it
is the one message meaning *the host never came up*: no chunk of yours ran, and
nothing you wrote is implicated.

Chunk evaluation is budgeted like everything else. There is no unbudgeted path
from the engine into script, and a chunk whose top level never returns is
stopped exactly as a callback is.

The budget is sized against the largest plausible **unsliceable** workload,
because a callback over budget has somewhere to go — the queue, across rounds —
and a chunk over budget has nowhere. A chunk walking a 64³ volume at one host
call per cell costs about 540,000 ticks, which a million admits with room and
refuses a workload an order of magnitude past it.

## The budget counts calls and loop edges — not instructions

This is the single most useful thing to know about the limit. The interrupt
fires at seven opcodes — call, fastcall, return, the two `for` iterations and
the two backward jumps — and at nothing else. So:

- **The body of a loop is free.** A loop of ten statements costs exactly what an
  empty loop costs.
- **A thousand straight-line statements cost one.**
- **A call into the host costs one; a call within script costs two.**

Measured on a 16³ pass — 4,096 cells:

| The pass does this | Ticks |
|---|---|
| Nothing but the loop | 4,369 |
| One host call per cell | 8,465 |
| One call to a Luau helper per cell | 12,561 |

Roughly one, two and three ticks a cell. The third of those aborts under a
budget of 10,000 while the first two do not, and the code is the same length in
all three.

**Cost is reduced by batching calls, never by shortening code.** Sizing a
workload against how much code it is, rather than against how many calls it
makes, is wrong by the size of every loop body in it. One call passing a batch
beats N calls passing one item, and it is the same advice binding overhead
already gives — it turns out to govern the budget too.

### Working out whether a workload fits

Count three things and add them. Nothing else in your code costs anything.

1. **Loop edges** — one per iteration of every loop, however long its body.
2. **Calls within script** — two each, one for the call and one for the return.
3. **Calls into the host** — one each.

Check the rules against the measured table above rather than taking them on
trust. All three rows are `4,096 × ticks-per-cell + 273`, where 273 is the
loop's own setup and the enclosing call — a constant that stops mattering at any
size worth counting:

| The pass does this | Per cell | Predicted | Measured |
|---|---|---|---|
| Nothing but the loop | 1 edge | 4,369 | 4,369 |
| One host call per cell | 1 edge + 1 call | 8,465 | 8,465 |
| One Luau helper call per cell | 1 edge + 2 | 12,561 | 12,561 |

So a pass over a 64³ volume — 262,144 cells — costs:

```text
one host call per cell     262,144 × 2 + 273  =    524,561   fits
one script call per cell   262,144 × 3 + 273  =    786,705   fits
both                       262,144 × 4 + 273  =  1,048,849   ABORTED
```

Note what changed and what did not. The volume is identical, the algorithm is
identical, and the file is a line or two longer. What moved it over the line was
one extra call per cell. Hoisting that helper's work out of the loop — one call
passing all 262,144 cells rather than 262,144 calls passing one — costs **two
ticks in total** instead of 524,288, and the body it runs is free either way.

As one formula, for a pass over `N` cells:

```text
ticks ≈ 273 + N × (1 + host_calls_per_cell + 2 × script_calls_per_cell)
```

**Read it backwards to size a slice**: divide the budget by the per-cell cost
and that is the most cells one invocation may visit. A callback that visits a
cell, calls a helper of its own and makes one host call from inside it costs
four ticks a cell, so it may visit 250,000 cells; inline the helper and the same
budget buys 500,000; batch the host call so it happens once for the whole pass
and it buys 1,000,000, because all that is left is the loop.

Two things this makes obvious that the rule alone does not. **Whatever happens
inside the body is free** — validating a value, building a string, ten
statements or one — so shortening a body moves no number in this section. And
**the 273 is the only fixed cost**, so splitting a job into more, smaller
invocations costs 273 ticks apiece and nothing else. Slicing is cheap; calling
is what is not.

## Two rules that sit next to each other

They pull against each other and neither may be softened. The tension is real;
it is documented rather than resolved by leaving one out.

### 1. A job bigger than one budget is carried by a closure upvalue

There is no continuation mechanism and no state API. The honest answer today is
a cursor held in a closure upvalue, advanced a slice at a time:

```lua
local TOTAL, PER_INVOCATION = 40000, 512
local cursor = 1

return function()
    local last = math.min(cursor + PER_INVOCATION - 1, TOTAL)
    for index = cursor, last do
        -- one slice of the job; whatever is between calls costs nothing
    end
    cursor = last + 1
    if cursor > TOTAL then
        return "done"
    end
    return { follow_up = { { subject = "stone-furnace", component = "grind" } } }
end
```

Each invocation begins on a **whole fresh budget** — never the remainder of an
exhausted one — so the only thing you have to size is one slice.

**What that retention costs is the thing the host cannot bound.** The memory
cap bounds what one entry *adds*; it says nothing about what the state is
already holding on your behalf. A callback keeping a fraction of its cap each
time never trips a limit and grows without limit. The aggregate is still
bounded — the backstop is a ceiling the whole state cannot pass — but per
attachment it is not, and the damage is misattribution: as the ceiling is
approached, other mods' ordinary allocations start failing, and the host, which
cannot tell whose retention filled the state, reports those failures against
nobody. Keep only what you are actually going to read again.

A suspended coroutine is the same mechanism by another route: it holds its own
stack, its locals and everything they reference for as long as a reference to it
survives, and a reference to it is an upvalue like any other.

### 2. An abort is not recoverable in script

When a limit trips, the abort **latches**. Every interrupt after it fails
immediately, so no script frame makes progress — including the frame that was
going to catch the error. This is why `while true do pcall(f) end` cannot
re-enter the budget indefinitely, and it is also why:

- **A `pcall` handler does not run after an abort.** Neither does any other
  cleanup you wrote in script.
- **Post-abort tidying is impossible.** Never leave a structure half-modified
  across a call that might be stopped.

So: **build, then swap — never mutate in place.** Construct the replacement in
a local, and assign it in one statement once it is complete. A structure
assembled field by field is a structure an abort can leave inconsistent, with
nothing of yours able to repair it:

```lua
local settings = { fuel = 0, output = nil, stage = "idle" }

return function(reading)
    -- WRONG. An abort between any two of these leaves `settings` describing a
    -- state the mod was never in, and the next invocation reads it as if it
    -- were true. No cleanup of yours runs to undo it.
    settings.stage = "smelting"
    settings.fuel = settings.fuel - burn_cost(reading)   -- may be aborted here
    settings.output = smelted(reading)

    -- RIGHT. Everything an abort could interrupt happens to a local nobody
    -- else can see. The one assignment that publishes it either happens whole
    -- or does not happen at all.
    local next_settings = {
        stage = "smelting",
        fuel = settings.fuel - burn_cost(reading),
        output = smelted(reading),
    }
    settings = next_settings
end
```

The same rule is why a `pcall` around the wrong half buys nothing: the handler
you were counting on is a script frame, and after the latch no script frame
runs.

The latch clears at the start of the next entry, so the abort is a property of
the invocation that caused it, not of your mod forever.

## What a fault tells you

A failure inside script is reported to the engine as a value, never as a crash.
It carries:

- **The kind**, one of: call-and-loop budget exhausted, allocation refused,
  script error, compilation failed, cascade refused, cascade deferred, host
  memory pressure.
- **The origin** — the chunk that *defined* the failing callback, plus the
  dispatch round. The defining chunk is the one that sends you to a file; the
  round only says where the engine was.
- **The subject and component**, for anything the invocation itself caused.
- **The line**, where the backend named one. An error raised inside a builtin
  carries no line at all, which is a real shape rather than a lost one.
- **For a refused cascade, the attachment whose work was turned away**, beside
  the one that asked for it — the only fault naming two attachments, because
  for work that will never run you have to know what was lost.
- **The cause.** For an allocation abort this is composed by the host, because
  the underlying error carries no message whatsoever; rendering it faithfully
  would produce a fault that names your component and then says nothing. The
  composed cause states the cap and how much was in use when the entry was
  stopped.

**A cause is unbounded, script-controlled text.** It is rendered raw, because
rendering it any other way would run a `__tostring` of yours on the host's
schedule — but raw and safe-to-splice are different properties, and every
consumer that logs it or shows it to a human inherits whatever a mod put there,
at whatever length.

Deferral and refusal are deliberately different kinds. A **deferred** cascade
runs next round and loses nothing; a **refused** one never runs at all. A
well-behaved terminating cascade emits one deferral per round before it
finishes, which is noise precisely because nothing was lost.

### How a fault reads

Every fault renders in one grammar. Everything in square brackets appears only
when the fault has it:

```text
<origin>[, subject `S`][, component `C`][, line N][, refused `S`/`C`]: <kind>: <cause>
```

`<origin>` has four shapes and all four occur:

| Shape | When you see it |
|---|---|
| ``chunk `NAME`, round N`` | a callback failed — the chunk is the one that *defined* it, not the one dispatching |
| ``chunk `NAME`` | a chunk failed while being evaluated, before any attachment exists |
| `round N` | nothing in anybody's file went wrong: a cascade fault, or one the host raised about its own condition |
| `unattributed` | the fault can place itself nowhere at all |

A fault that could not attribute itself says so rather than rendering a gap,
because a gap is what you would misread as a locator that got lost.

### Causing each fault, and what you will read

Every kind below is one you can cause. The chunk is named `furnace.luau` and the
attachment is `base:furnace` / `base:on_tick` throughout.

**Compilation failed** — the chunk does not parse. Nothing is attached, so there
is no attachment to name, and the line is the one the parser stopped on:

```lua
local heat = 0
return function()
    return heat +
end
```
```text
chunk `furnace.luau`, line 3: compilation failed: Expected identifier when parsing expression, got 'end'
```

**Script error** — your callback raised, whether by `error(...)` or by an
ordinary mistake. This is the one you will read most often:

```lua
return function()
    error("the furnace is jammed", 0)
end
```
```text
chunk `furnace.luau`, round 2, subject `base:furnace`, component `base:on_tick`: script error: the furnace is jammed
```

**Note the `0`, and note what is missing.** A raise from inside a callback comes
back to the host through a protected call as an ordinary value, so it is
rendered exactly as script left it — with **no line field at all**, and with no
location stripped out of the front of it. A bare `error("the furnace is
jammed")` therefore reports the backend's own position marker spliced onto the
front of your message, `[string "furnace.luau"]:2: `, and it stays there because
there is no `line` field for it to be lifted into. Pass `0` as the second
argument when you want the cause to be exactly your text.

That is the reverse of a chunk-level fault, where the location *is* parsed out
and reported as the `line` field — which is why the compilation example above
carries `line 3` and this one carries no line.

**Call and loop budget exhausted** — the invocation never returned inside its
budget. Note the cause: the host says only that a limit was passed, because at
that point no script frame is running to say anything more:

```lua
return function()
    while true do end
end
```
```text
chunk `furnace.luau`, round 2, subject `base:furnace`, component `base:on_tick`: call and loop budget exhausted: script exceeded a limit the host enforces
```

**Allocation refused** — the invocation held more than one entry may. The cause
here is composed by the host, because the underlying error carries no message at
all; the first number is the cap, the second is what was in use when you were
stopped, so you can tell "slightly too much" from "far too much":

```lua
return function()
    local kept = {}
    while true do
        kept[#kept + 1] = buffer.create(4096)
    end
end
```
```text
chunk `furnace.luau`, round 2, subject `base:furnace`, component `base:on_tick`: allocation refused: script allocated more than the 262144 bytes one invocation may hold above the memory it started with; 648432 bytes were in use when it was stopped
```

**The two numbers are in different frames, and reading them as one pair is the
mistake to avoid.** The first is a **delta** — what this invocation was allowed
to add above the memory the state already held on its way in. The second is an
**absolute** — the whole state's script memory at the moment the interrupt
stopped you, everything anybody has retained included. So the second is always
much the larger, and it is not "how much you asked for": under the shipped cap it
cannot even come out below the state's own baseline of roughly 386 KB plus the
262,144 you were allowed. The second figure is whatever that particular run
measured; what you compare against the cap is the *difference* between the two.

If the host also could not collect it afterwards, the same cause ends `, and the
host could not collect it afterwards` — which is about the host's condition
rather than yours, and is what the next invocation runs into.

**Cascade deferred** — your follow-up work did not fit this round. It names the
round and no chunk, because nothing in anybody's file went wrong. Nothing is
lost and there is nothing to fix:

```text
round 4, subject `base:furnace`, component `base:on_tick`: cascade deferred: the round reached its invocation bound with follow-up work still waiting; it runs next round
```

**Cascade refused** — the pending queue was full, so the work never ran. This is
the only fault naming two attachments: the one that asked, and the one that was
turned away:

```text
round 4, subject `base:furnace`, component `base:on_tick`, refused `base:vent`/`base:on_tick`: cascade refused: the pending queue was full, so this follow-up work was not admitted and will not run
```

**Host memory pressure** — the state had no room for your whole allowance before
you began. It names **no chunk, no subject and no component**, deliberately, and
it does not count against you:

```text
round 7: host memory pressure: the state had no room for this invocation's whole memory allowance before it began, so this failure may not be the running attachment's own
```

Reading that one on your own callback means the problem is the state rather than
your code — possibly your own retention from earlier invocations, possibly
somebody else's. It is the one fault where the right response is to look at what
is being *kept*, not at what just ran.

## Quarantine, as an author meets it

**Three consecutive faults on one attachment and it stops being invoked.** The
count resets on a success, so a callback that alternates failing and succeeding
is never quarantined — its cost is already bounded, one invocation at a time.
Quarantine is for the callback that is simply broken.

It is per `(subject, component)`. Your broken attachment on one subject leaves
the same component running on every other subject, and leaves every other
component on that subject running.

Two things lift it, and nothing else:

- **Releasing the attachment.**
- **Attaching a new callback to it** — which is what reloading a fixed mod *is*.
  A replace that left the attachment quarantined would silently fail at the one
  thing reloading a broken mod exists to do.

The invocation count is not reset by either. It is cumulative telemetry about
the attachment and resumes from where it froze; the counter that resets is the
consecutive-fault one.

Two fault kinds never count toward quarantine: cascade faults, which are about
the round's admission control rather than about your callback, and host memory
pressure.

## When the host is short of memory, faults name nobody

Before each invocation the host asks one derived question: *could this
invocation fail for a reason that is not its own?* It holds when the baseline
the state is already carrying, plus one whole memory cap, would carry it past
the backstop — that is, when the allowance this invocation is entitled to does
not fit.

While it holds, a failure is reported as **host memory pressure**: no subject,
no component, no chunk, and no advance toward quarantine. The reason is that
attributing it would disable an innocent mod and file the blame against the
wrong author, and an operator acting on that removes the wrong mod.

The cost is stated rather than hidden, and it is two-sided:

- While pressure holds, **a genuinely looping mod is not quarantined either.**
  That failure is loud — the server is slow and an operator notices — where a
  misattribution is silent and points at the wrong file.
- **An attachment whose own retention raised the baseline is immune**, because
  quarantining it would not give the memory back anyway: what it is holding
  lives in closure upvalues that survive quarantine.

That is the price of not keeping a ledger of retained bytes per attachment. It
is worth knowing while you write a mod that keeps things.

## A complete example

Everything above in one chunk. It is a furnace that smelts a queue too big for
one budget, hands off to a vent when it finishes, and is written so that no
abort can leave it inconsistent.

```lua
-- furnace.luau
--
-- Evaluated once. Everything it needs lives in locals, because the global
-- environment is frozen and an assignment to it aborts the chunk.

local PER_INVOCATION = 4096          -- one slice, sized against the budget below
local VENT = { subject = "base:furnace", component = "base:vent" }

local queue = {}                     -- retained across invocations, on purpose
local cursor = 1

for index = 1, 40000 do              -- 40,000 loop edges, one per iteration
    queue[index] = index             -- the body is free, however long it gets
end

return function()
    local last = math.min(cursor + PER_INVOCATION - 1, #queue)

    -- Build the slice's result in a local. Nothing published half-done.
    local smelted = {}
    for index = cursor, last do
        smelted[#smelted + 1] = queue[index] * 2
    end

    -- One publish, one statement. An abort either happened before this line or
    -- after it; there is no state in between for anyone to read.
    cursor = last + 1

    if cursor > #queue then
        print(("furnace finished %d items"):format(#queue))
        return { follow_up = { VENT } }
    end

    -- More to do: ask for ourselves again rather than looping past the budget.
    return { follow_up = { { subject = "base:furnace", component = "base:on_tick" } } }
end
```

**What it costs.** The chunk's own loop is 40,000 edges plus the fixed 273, so
evaluation costs about 40,300 ticks against a budget of 1,000,000 — comfortable,
and it is the one part that cannot be sliced, which is why the budget is sized
for chunk evaluation rather than for callbacks. Each invocation is 4,096 loop
edges plus a handful of calls — `math.min`, and on the last one `print` and
`format`. The `#queue` lengths and the arithmetic cost nothing at all. Call it
4,400 ticks, 0.4 % of the budget; ten invocations finish the job.

**What it holds.** `queue` is 40,000 numbers retained in a closure upvalue for
the whole life of the mod. That is the construction the host cannot bound per
attachment, used deliberately and kept as small as the job needs — and it is why
the last thing this callback should do is keep `smelted` too. It does not: each
slice's result goes out of scope when the invocation returns.

**What you would see if it went wrong.** Raise the slice to 400,000 and the
invocation still fits, because loop edges are cheap. Call a helper of your own
per item instead, and it costs three ticks an item rather than one — the change
that actually matters. Drop the `cursor = last + 1` line and the mod smelts the
same slice forever, faulting nothing and finishing never, which is the failure
shape no limit here catches for you.

**What it cannot do.** Nothing in this chunk reaches the game. `queue` holds
numbers because there is no way to ask for a block, a position or an entity, and
`follow_up` names attachments the engine already knows about rather than
anything the script created. That is the honest edge of this page: the shape of
a chunk, its cost, its faults and its limits are all real and all final — the
values it works on are what a later increment adds.
