# MVP supervisor loop

A `/loop` prompt that drives an entire MVP to completion by supervising the
`sdd-conductor` agent.

## Usage

Paste the block below after `/loop`, with **no interval**:

```
/loop <paste the prompt from the next section>
```

Interval-less is deliberate. The conductor runs for hours and you are
re-invoked automatically when it finishes, so a fixed cadence would produce
mostly wasted ticks. The loop self-paces with long fallback delays.

To stop early: `/loop stop`, or tell the running loop to stop.

## The prompt (MVP 1)

```
Supervise the MyCraft MVP 1 build. You are a watchdog, not a builder — you
spawn at most one worker and otherwise stay out of the way.

Derive all state from Linear, git, and disk each tick. Never rely on memory of a
previous tick; assume you know nothing.

Evaluate in order and act on the FIRST match:

1. A sdd-conductor task is still running → do nothing. noop:true.
   Never spawn a second conductor. Two conductors on one branch will corrupt
   each other's work and the Linear board.

2. Any issue in Linear project f6cf7f2f-b4ad-4bb3-ab04-473b5ae68ec2 is in state
   "Waiting for Input" → the conductor needs the user. Surface the question and
   my recommendation exactly once, then noop:true every tick until answered.
   Do not answer it yourself and do not start other work around it.

3. Working tree dirty or local ahead of origin/main → commit if there is a
   coherent unit of work, then push. Unpushed work is unbacked work.

4. Every issue in that project is Done → verify before believing it:
   run scripts/sdd-gate.ps1 (must exit 0), confirm the tree is clean and
   pushed, and launch the client to confirm it is actually playable.
   All pass → report MVP 1 complete with what you verified, and STOP the loop.
   Any fail → treat as incomplete and continue to step 5.

5. Otherwise → spawn exactly one sdd-conductor, name: conductor-mvp1, to
   continue MVP 1 (Playable Sandbox) per product/roadmap.md. Tell it:
   - it owns MVP 1 end to end and should resume from current Linear/git state
   - rigor high, per product/mission.md
   - it must report back via SendMessage to: main
   Then noop:false.

Stop conditions — end the loop rather than continuing:
- MVP 1 verified complete (step 4). Do NOT roll on to MVP 2; the user reviews
  and gives feedback first.
- The same failure recurs across three ticks with no progress. Report the
  failure and stop; a loop that cannot make progress should not keep running.

Pacing: the conductor's completion notification is your real wake signal, so
schedule long fallback delays (1800s+). Do not poll it.
```

## Why it is shaped this way

**The loop supervises; the conductor builds.** The obvious version —
`/loop 30m spawn the conductor` — stacks a new conductor every 30 minutes, all
racing on the same branch and Linear board. Step 1 exists to prevent exactly
that, and it is checked first because it is the failure that destroys work.

**Step 4 does not trust Linear.** "All issues Done" is a claim made by an
agent. The loop independently runs the gate *and launches the game*, because
MVP 1's definition of done is "a game that starts and is playable" — a state no
number of closed issues can establish.

**It stops at the MVP boundary.** Rolling straight into the next MVP would
spend hours building on a foundation the user has not reviewed. Each increment
ends with a human check.

**Three-tick failure stop.** Prevents the standard loop pathology of burning
turns indefinitely re-attempting something broken.

## Adapting for later MVPs

Swap the project ID in step 2 and the MVP name in steps 4–5:

| MVP | Linear project ID |
|-----|-------------------|
| 1 Playable Sandbox | `f6cf7f2f-b4ad-4bb3-ab04-473b5ae68ec2` |
| 2 Scriptable Content | `9af3f5c2-8b51-4e08-8fa5-c04e38e43eca` |
| 3 Multiplayer | `fff6b8f3-6238-462d-8e97-373e1d734369` |
| 4 Survival Loop | `4bdd46a4-c442-4c51-a584-1acc24953533` |
| 5 Living World | `9e966760-3112-4b98-8b91-e4deaf47711c` |
| 6 Public & Polished | `417535c1-8843-4866-bf95-d48011f834e5` |

Also update step 4's playability check — what "playable" means differs per
increment. For MVP 3 it means two clients connected to one server, not one
client launching.

## Cost

Expect roughly six subagents per spec, and an MVP of 5–7 specs. A full MVP 1
run is many hours and a large number of agent invocations. Steering mid-run is
supported: messages are treated as authoritative and the conductor adjusts.
