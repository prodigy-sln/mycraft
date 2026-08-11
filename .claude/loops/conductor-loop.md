# Conductor self-pacing loop

The `/loop` prompt the **conductor invokes on itself** to drive an MVP to
completion across many hours.

This is not a supervisor wrapping the conductor. There is no outer manager: the
conductor owns the MVP, and `/loop` is simply how it survives running longer
than one context. Each tick advances the build by one step and schedules the
next wake-up.

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

## The prompt (MVP 1)

```
Continue driving MyCraft MVP 1 to completion. You are the conductor: you own
this MVP end to end — decomposition, sequencing, architecture, arbitration,
commits and Linear are all yours.

This tick may have no memory of the last one. Reconstruct state from disk
before deciding anything:
  - Linear project f6cf7f2f-b4ad-4bb3-ab04-473b5ae68ec2 — which issues exist
    and what state each is in
  - git log, git branch, git status — what merged, what branch is in flight,
    whether the tree is dirty
  - specs/active/ — any spec mid-pipeline and which stage it reached
  - product/roadmap.md — MVP 1 scope and exit criteria

Then act on the FIRST match:

1. A stage subagent is still running → do nothing. noop:true.
   Never run two stages of the same spec concurrently.

2. You are waiting on the user (an issue is in "Waiting for Input") → do
   nothing but restate the open question once. noop:true until answered.

3. Everything in MVP 1 looks done → verify before believing it. Run
   scripts/sdd-gate.ps1 (must exit 0), confirm the tree is clean and
   origin/main is in sync, confirm every issue is Done, then launch the client
   and confirm you can move and place a block. A game that does not start is
   not a finished MVP no matter what the board says.
   All hold → report completion via SendMessage to main, stop the loop.
   Any fails → fix that, do not re-claim completion until it holds.

4. Otherwise advance the build by exactly ONE step, then noop:false:
   - No spec in flight → pick the next item from MVP 1 by dependency order,
     create its Linear issue, move it to In Progress, and spawn the /sdd-start
     subagent.
   - A spec is mid-pipeline → spawn the next stage's subagent
     (start → architect → [discuss] → tasks → implement → validate → complete).
   - The last stage failed → decide: retry with corrected guidance, re-run an
     earlier stage, or escalate. Never re-spawn an identical prompt; it will
     fail identically.
   - The tree is dirty from an interrupted predecessor → resolve it as owner
     before starting new work. Finish it, commit it, or discard it
     deliberately, and say which.

   Every spawned subagent must be told to report back via SendMessage to your
   own agent name — not main. A child whose turn simply ends returns nothing.

   Commit and push at every stage boundary. Unpushed work is unbacked work.

Stop the loop when:
- MVP 1 is verified complete (step 3). Do NOT continue into MVP 2 — the user
  reviews and gives feedback first.
- Three consecutive ticks produce no new commits and no Linear transitions.
  Something is systematically broken; report it and stop rather than burning
  turns.

Pacing: a stage subagent completing is your real wake signal, so schedule long
fallback delays (1800s+). Do not poll.
```

## Why it is shaped this way

**One step per tick.** The conductor does not try to run a whole spec in one
invocation. Each tick makes one decision, spawns one subagent, and yields. That
is what makes a multi-hour MVP survivable: no single context has to hold it.

**State comes from disk, never memory.** A tick may begin with no recollection
of the previous one, so Linear, git, and `specs/active/` are the source of
truth. This is the same property that makes Prospect phases resumable, applied
one level up.

**Verification before claiming done.** Step 3 runs the gate and launches the
game rather than trusting the Linear board. It is the same principle the
conductor applies to its own subagents — *"do not take a subagent's word that
it is green"* — turned on itself. MVP 1's definition of done is "a game that
starts and is playable", which no number of closed issues can establish.

**No outer manager.** An earlier draft made this an external supervisor loop
that spawned conductors, verified their work, and committed dirty trees. That
contradicted the conductor's ownership and put a second actor on the same
branch. Folding the loop *into* the conductor removes the conflict rather than
managing it.

**It stops at the MVP boundary.** Rolling into the next MVP would spend hours
building on a foundation the user has not reviewed.

## Adapting for later MVPs

Swap the project ID and MVP name:

| MVP | Linear project ID |
|-----|-------------------|
| 1 Playable Sandbox | `f6cf7f2f-b4ad-4bb3-ab04-473b5ae68ec2` |
| 2 Scriptable Content | `9af3f5c2-8b51-4e08-8fa5-c04e38e43eca` |
| 3 Multiplayer | `fff6b8f3-6238-462d-8e97-373e1d734369` |
| 4 Survival Loop | `4bdd46a4-c442-4c51-a584-1acc24953533` |
| 5 Living World | `9e966760-3112-4b98-8b91-e4deaf47711c` |
| 6 Public & Polished | `417535c1-8843-4866-bf95-d48011f834e5` |

Also update step 3's playability check — "playable" differs per increment. For
MVP 3 it means two clients connected to one server, not one client launching.

## Cost

Roughly six subagents per spec, and an MVP of 5–7 specs. A full MVP 1 run is
many hours and a large number of agent invocations. Steering mid-run is
supported: user messages are authoritative and the conductor adjusts on the
next tick.
