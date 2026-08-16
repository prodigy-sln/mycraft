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
queue length rather than into call depth. Each entry is read raw — the field,
each slot and both identity strings — and an entry that is not two strings is
**passed over silently**. Nothing is fabricated in its place, and nothing is
reported to you either: a misspelled `componant` key is work that quietly never
runs. Check the spelling of a `follow_up` entry by hand, because the host will
not.

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

| Limit | Default | What it aborts |
|---|---|---|
| Call-and-loop budget | 1,000,000 ticks | One entry into script — a chunk evaluation or one callback invocation. |
| Memory cap | 256 KiB | One entry that adds more than this **above the baseline it started from**. |
| Memory backstop | 16 MiB | The whole script state, enforced by the allocator rather than by the interrupt. |
| Fault threshold | 3 | Consecutive faults on one attachment before it stops being invoked. |
| Round bound | 64 | Invocations one dispatch round performs before the rest waits for the next round. |
| Pending bound | 256 | Entries of follow-up work that may be waiting at once; work past it is refused and named. |

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
nothing of yours able to repair it.

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
