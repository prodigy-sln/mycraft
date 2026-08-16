# What a script may spend

**What this answers:** every limit that ships with its real number, what the
budget actually counts, and how to work out before you write it whether a
workload fits. **Who it is for:** anyone sizing a job against the host.

Nothing can be authored in Luau today — see `script-writing.md` for what that
means and where authoring actually lives. The numbers below are what the host
enforces now.

Two of these limits are what you plan around; the rest you meet only when
something has gone wrong, and `script-faults.md` is where that is read.

## The limits that ship

A host constructed without configuration runs these. A server operator can
raise or lower any of them; these are the values that apply when nobody does.

| Limit | Default | What exceeding it does | What you do about it |
|---|---|---|---|
| Call-and-loop budget | 1,000,000 ticks | Aborts one entry into script — a chunk evaluation or one callback invocation — with *call and loop budget exhausted*. | Make fewer calls, or slice the work across invocations. Shortening the code does nothing; see the next section. |
| Memory cap | 256 KiB | Aborts one entry that adds more than this **above the baseline it started from**, with *allocation refused*. | Hold less at once. Build into one buffer you reuse rather than growing a table per invocation. |
| Memory backstop | 16 MiB | The ceiling the whole state may reach, allocator-enforced. Approaching it stops faults naming anybody — see host memory pressure in `script-faults.md`. | Retain less across invocations. This is the limit your *previous* invocations spend, not this one. |
| Fault threshold | 3 | Consecutive faults on one attachment stop it being invoked at all (`script-faults.md`). | Fix the callback and re-attach it, which clears the quarantine. |
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
