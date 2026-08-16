# Writing a script

**What this answers:** how a chunk is shaped, what it returns, and the two rules
you write everything under. **Who it is for:** anyone about to write Luau for
this engine.

The three pages beside this one answer the rest: `script-surface.md` for what a
script may reach, `script-limits.md` for what it may spend, and
`script-faults.md` for what a failure says.

## Nothing can be authored in Luau today

The engine carries a sandboxed Luau host — a place mod code will run, budgeted
and memory-capped and isolated — and **there is no `mycraft.*` binding of any
kind**: no block, world, entity or registry access, and no script-callable way
to declare a component, give it a name, or attach it to a subject. That surface
is Rust — the engine evaluates a chunk, takes the value it returns, and decides
what to attach it to — and content has no path into it.

**If you came here to make something, `README.md` is the page you want.**
Authoring works today and it is data files: a block, a HUD element, a voxel
model. It takes you from an empty file to something on screen.

So why read this at all? Because the shape of a chunk, its cost, its faults and
its limits are real and final now, and a binding is added *behind* them rather
than instead of them. Settling them first is what makes a capability judged
against enforcement that already exists rather than hardened afterwards. None of
it will move to accommodate a binding.

The authoring surface arrives in two steps. **Defining blocks in Luau** comes
first and retires the data-file loader. **Attaching behaviour to a subject** —
the thing every limit is charged against — comes after it, in the same
milestone.

## The shape of a script

A chunk is handed to the host as source plus a name to report it under — the
name is a label, not a path, and the host never opens a file. Whatever name
a mod author would recognise is the right one, because it is what a fault
points at.

A chunk is evaluated once and its return value is kept. To have behaviour
invoked later, return a function — and note the `local`, which is not optional:
the environment a chunk is evaluated against is frozen, so a global assignment
aborts the chunk where it stands (`script-surface.md`).

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
The pair is the unit of everything the host bounds — the budget is charged to it
(`script-limits.md`), faults are counted against it, and quarantine acts on it
(`script-faults.md`). Two mods attaching
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
| `follow_up` | table | up to the pending bound (`script-limits.md`), counted across every requester | a missing or non-table `follow_up` means no follow-up work. No signal. |
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
malformed or its target was quarantined (`script-faults.md`) — those are the two
silent cases, and they are the first two things to check.

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
cap (`script-limits.md`) bounds what one entry *adds*; it says nothing about
what the state is
already holding on your behalf. A callback keeping a fraction of its cap each
time never trips a limit and grows without limit. The aggregate is still
bounded — the backstop is a ceiling the whole state cannot pass — but per
attachment it is not, and the damage is misattribution: as the ceiling is
approached, other mods' ordinary allocations start failing, and the host, which
cannot tell whose retention filled the state, reports those failures against
nobody (`script-faults.md`). Keep only what you are actually going to read
again.

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

## A complete example

Everything above in one chunk. It is a furnace that smelts a queue too big for
one budget, hands off to a vent when it finishes, and is written so that no
abort can leave it inconsistent. The tick figures quoted after it come from
`script-limits.md`, which is where the arithmetic is derived.

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
