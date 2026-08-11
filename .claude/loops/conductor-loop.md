# Conductor self-pacing loop

The `/loop` prompt the **conductor invokes on itself** to drive an MVP to
completion across many hours.

This is not a supervisor wrapping the conductor. There is no outer manager: the
conductor owns the MVP, and `/loop` is simply how it survives running longer
than one context. Each tick advances the build by one step and schedules the
next wake-up.

The prompt is **generic** — it reads the current MVP and its definition of done
out of `product/roadmap.md` rather than naming either. Use it verbatim for every
MVP; nothing needs editing between increments.

## Usage

The conductor invokes this itself, early in its run, with **no interval** so it
self-paces:

```
/loop <the prompt below>
```

Interval-less matters here. A stage subagent runs for a long time and the
conductor is re-invoked automatically when it completes, so a fixed cadence
would produce mostly wasted ticks. The fallback delays exist only in case a
notification never arrives.

To stop early: `/loop stop`, or tell the running conductor to stop.

## The prompt

```
Continue driving the current MyCraft MVP to completion. You are the conductor:
you own this MVP end to end — decomposition, sequencing, architecture,
arbitration, commits and Linear are all yours.

This tick may have no memory of the last one. Reconstruct everything from disk
before deciding anything.

First establish scope:
  - product/roadmap.md marks exactly one MVP with "← current". That is your
    scope. Its feature table is the work; its "Exit criteria" section is the
    definition of done.
  - Resolve that MVP's Linear project by name (they are named
    "MyCraft MVP N: <title>") under team PRO:
      linear-cli projects list -o json
  - If no MVP is marked current, or the current one has no exit criteria, or
    more than one is marked — STOP and ask the user. Never guess scope.

Then read state:
  - Linear: which issues exist in that project and what state each is in
  - git log / git branch / git status: what merged, what branch is in flight,
    whether the tree is dirty
  - specs/active/: any spec mid-pipeline and which stage it reached

Then act on the FIRST match:

1. A stage subagent is still running → do nothing. noop:true.
   Never run two stages of the same spec concurrently.

2. You are waiting on the user (an issue is in "Waiting for Input") → do
   nothing but restate the open question once. noop:true until answered.

3. Every feature in the current MVP looks done → verify before believing it.
   Work through that MVP's exit criteria in roadmap.md one by one and confirm
   each. At minimum that always includes: scripts/sdd-gate.ps1 exits 0, the
   tree is clean and origin/main is in sync, every issue is Done, and the
   playable behaviour the exit criteria name actually works when you launch it.
   A build that does not do what the criteria say is not a finished MVP, no
   matter what the board says.
   All hold → report completion via SendMessage to main, stop the loop.
   Any fails → fix it; do not re-claim completion until it holds.

4. Otherwise advance the build by exactly ONE step, then noop:false:
   - No spec in flight → pick the next feature from the MVP's table by
     dependency order, create its Linear issue, move it to In Progress, and
     spawn the /sdd-start subagent.
   - A spec is mid-pipeline → spawn the next stage's subagent
     (start → architect → [discuss] → tasks → implement → validate → complete).
   - The last stage failed → decide: retry with corrected guidance, re-run an
     earlier stage, or escalate. Never re-spawn an identical prompt; it will
     fail identically.
   - The tree is dirty from an interrupted earlier tick → resolve it as owner
     before starting new work. Finish it, commit it, or discard it
     deliberately, and say which.

   Every spawned subagent must be told to report back via SendMessage to your
   own agent name — not main. A child whose turn simply ends returns nothing.
   Require an explicit end-of-turn signal too: every child sends a SendMessage
   on every handback, exactly "[DONE]" when it has nothing to say.

   Harness idle/finished notifications are stale often enough to be misleading —
   they can describe a turn that ended before your last message arrived, so an
   actively-working agent reads as available. Trust only an explicit message
   from the child, or a new commit plus a clean tree. When they disagree, ask
   the agent to finish the outstanding commit rather than waiting on it or
   editing its files yourself.

   Commit and push at every stage boundary. Unpushed work is unbacked work.

Stop the loop when:
- The current MVP is verified complete (step 3). Do NOT advance to the next
  MVP — the user reviews and gives feedback first, and may change what comes
  next based on playing this one.
- Three consecutive ticks produce no new commits and no Linear transitions.
  Something is systematically broken; report it and stop rather than burning
  turns.

Pacing: a stage subagent completing is your real wake signal, so schedule long
fallback delays (1800s+). Do not poll.
```

## Why it is shaped this way

**Generic by construction.** Scope, features and the definition of done all come
from whichever MVP `roadmap.md` marks `← current`. Promoting the next MVP is a
one-word edit to the roadmap; this prompt never changes. Hardcoding an MVP would
have meant maintaining six near-identical prompts that drift.

**It refuses to guess scope.** No current marker, no exit criteria, or two
markers → stop and ask. An autonomous agent that invents its own scope is far
worse than one that stalls, and this is the exact place that failure would
enter.

**One step per tick.** The conductor does not try to run a whole spec in one
invocation. Each tick makes one decision, spawns one subagent, and yields. That
is what makes a multi-hour MVP survivable: no single context has to hold it.

**State comes from disk, never memory.** A tick may begin with no recollection
of the previous one, so the roadmap, Linear, git and `specs/active/` are the
source of truth. Same property that makes Prospect phases resumable, applied one
level up — and the reason to push at every stage boundary.

**Verification before claiming done.** Step 3 walks the roadmap's exit criteria
and launches the build rather than trusting the Linear board. Same principle the
conductor applies to its own subagents — *"do not take a subagent's word that it
is green"* — turned on itself.

**No outer manager.** An earlier draft made this an external supervisor that
spawned conductors, verified their work and committed dirty trees. That
contradicted the conductor's ownership and put a second actor on the same
branch. Folding the loop *into* the conductor removes the conflict rather than
managing it.

**It stops at the MVP boundary.** Rolling into the next MVP would spend hours
building on a foundation the user has not reviewed — and feedback on a playable
increment routinely changes what should come next.

## Requirement this places on the roadmap

`product/roadmap.md` must always have **exactly one** MVP marked `← current`,
with a feature table and an **Exit criteria** section stating what "playable"
means for that increment. The loop reads all three. When promoting the next MVP,
move the marker and write its exit criteria before starting the conductor.

## Cost

Roughly six subagents per spec, and an MVP of 5–7 specs. A full MVP run is many
hours and a large number of agent invocations. Steering mid-run is supported:
user messages are authoritative and the conductor adjusts on the next tick.
