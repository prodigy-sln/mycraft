# When a script breaks

**What this answers:** what a fault carries, the exact text you will read for
each kind, when an attachment stops being invoked, and why some failures name
nobody at all. **Who it is for:** anyone holding a fault and wanting to know
whose problem it is.

Nothing can be authored in Luau today — see `script-writing.md` for what that
means and where authoring actually lives. The fault vocabulary below is built
and final.

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

**Raise with `error(text, 0)`, and the cause is exactly your text.** That second
argument is the whole of the advice here, and it is worth knowing before you
write your first `error` rather than after reading your first fault.

Here is why it matters. A raise from inside a callback comes back to the host
through a protected call as an ordinary return value, so it is reported exactly
as script left it, and two things follow:

- **The fault carries no line.** There is no `line` field on this path at all —
  not a lost one, not an empty one. The example above has none, and neither will
  yours.
- **Nothing is stripped off the front of your message.** So a bare
  `error("the furnace is jammed")` reports the backend's own position marker
  spliced ahead of your text — the marker naming the chunk and the line, in the
  backend's spelling rather than one this host chose — and because there is no
  `line` field for it to be lifted into, it stays in the cause where every
  consumer downstream inherits it. `0` suppresses the marker at the source,
  which is the only place it can be suppressed.

That is the reverse of a chunk-level fault, where the location *is* parsed out of
the message and reported as the `line` field — which is why the compilation
example above carries `line 3` and this one carries no line at all.

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
chunk `furnace.luau`, round 2, subject `base:furnace`, component `base:on_tick`: allocation refused: script allocated more than the 262144 bytes one invocation may hold above the memory it started with; 652192 bytes were in use when it was stopped
```

**The two numbers are in different frames, and reading them as one pair is the
mistake to avoid.** The first is a **delta** — what this invocation was allowed
to add above the memory the state already held on its way in. The second is an
**absolute** — the whole state's script memory at the moment the interrupt
stopped you, everything anybody has retained included, your own allowance among
it. So the second is always much the larger, and it is never "how much you asked
for".

The second figure is that run's own, and you can derive the floor it has to
clear rather than taking it on trust. A freshly built state weighs about
**385,952 bytes** before any content runs, so the ceiling this invocation was
held to is `385,952 + 262,144 = 648,096`, and the reading is the first one past
it — here one 4,096-byte buffer past, which is what the loop above allocates per
turn. **Nothing below 648,096 is reachable in that position on a fresh state**,
which is the check to apply if a number there ever looks small: it would mean
the reading is not what you think it is.

What you compare against the cap is the *difference* between the two, never the
second number on its own.

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
