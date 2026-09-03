# Requirements ledger — nothing fails silently

Source issues: PRO-961 (lead), PRO-990, PRO-941, PRO-940, PRO-949.
Each was read in full with `linear-cli issues get` before a scenario was written
for it, and every factual claim below was checked against the tree rather than
taken from the issue text.

## Clarifications

- [resolved] Q: Does any of the five defects need a new declared content field,
  a crate-boundary change, or a change to a published surface — which would
  force the batch above `low`? → A: PRO-938 did (the published mod-author
  `FaultKind` vocabulary, documented at `docs/modding/script-faults.md:18` with
  per-kind quarantine policy at `:225`). The owner split it into its own spec and
  ruled its design separately. None of the remaining five does.

- [resolved] Q: PRO-961 recommends keying the origin-cell rule on `occludes`.
  Is that a new content field? → A: No. `occludes` is already on
  `mc_core::block::BlockDefinition:97`, already declared in Luau
  (`crates/mc-world/src/content/luau_declaration/mod.rs:72`), and already stated
  by the shipped water at `content/base/blocks/water.luau:117`. `mc-sim` does not
  yet carry it, so the change is a fourth packed view inside `mc-sim`. No
  boundary crossed, no content change.

- [resolved] Q: Would Defect 1 invalidate the golden frames and force
  `SCENE_REVISION` to move? → A: No. `crates/mc-sim/src/world/action/trace.rs:19-25`
  records that the golden frames are judged against a separate `March` in
  `mc-client`'s test support, deliberately not this walk.

- [resolved] Q: PRO-949 describes `Remesher::collect` as returning `None` for
  both an empty and a disconnected channel. Is that still true? → A: No — the
  issue is stale. `crates/mc-client/src/remesh.rs:102` declares a three-arm
  `Collecting` including `WorkerGone`, and `crates/mc-client/src/app/mod.rs:229`
  reports it to the player. `git log -S "WorkerGone"` attributes it to
  `45b30b44` (PRO-918). The owner ruled: keep it as exactly one verification
  scenario, because nothing is yet known to assert through the shipped path.

- [resolved] Q: PRO-941 names four `eprintln!` sites. Is four the count? → A:
  No. The tree holds eight; `docs/technical/architecture.md:2442` says seven and
  itself records an earlier correction from four. All eight are in scope. See
  spec §"Decisions taken in this spec" item 1.

- [resolved] Q: Should the stand-in notice truncate its key list, as PRO-990's
  example sentence suggests? → A: No. `crates/mc-client/src/notice.rs:102-110`
  sets the opposite convention for the sibling changed-blocks notice, on
  reasoning that applies here unchanged. Every uncovered key is named, ascending.

- [resolved] Q: Defect 4 introduces a binding contract. Does the architect phase
  run? → A: No. `.prospect/prompts/matrix.tsv` declares `architect` rows only for
  `work-type: feature` at `high|xhigh|max`. There is no `fix` architect row at
  any rigor. The delta is settled in this spec and in the implement phase.

- [assumed] A: D1-S2's control — an eye inside a block that *occludes* — is
  reachable in a fixture even though no shipped block puts a player's eye inside
  a solid block during ordinary play. `docs/user/gameplay.md` documents the
  transient state (a reload making the cell you swim in solid), and
  `crates/mc-sim/tests/support/chamber.rs` can declare it directly. If the test
  author finds the state unreachable through a shipped path, that is a dispute
  for arbitration rather than a reason to drop the control.

- [resolved] Q: Which sink shape does Defect 3 route the eight notices through —
  the `&mut dyn Write` the reported ending already takes, or a notice-specific
  sink that can also be silenced? → A: **The existing caller-supplied
  `&mut dyn Write`**, ruled by the owner on 2026-09-03. The capability already
  exists — `crates/mc-client/src/main.rs:41` passes `&mut io::stderr()` to
  `report` — and a second sink abstraction would be two ways to do one thing.
  Silencing comes free, because a caller supplies a sink that discards, so a
  notice-specific sink buys nothing this spec needs and
  `standards/global/code-quality.md` §1 forbids the abstraction until a real
  need exists. If implementing it exposes a concrete reason the shared stream
  cannot work, that is an escalation rather than a licence to invent the second
  shape.

- [resolved] Q: The escalation reserved above was taken. A **borrowed**
  `&mut dyn Write` cannot reach two of the notices: `say_changed_blocks` (from
  `prepare_launch`) and `say_stand_ins` (from `spawn_preparation`) are emitted on
  a `std::thread::spawn`'d worker, `thread::spawn` requires `F: Send + 'static`,
  and `spawn_preparation` returns the `JoinHandle` so `thread::scope` is not
  available either. Which shape? → A: **`Arc<Mutex<Box<dyn Write + Send>>>`**,
  ruled by the owner on 2026-09-03.

  The ruling above stands unchanged in substance: what it forbade was a *second
  sink abstraction* — a notice-specific type sitting beside the reported ending's
  — and a shared handle is a sharing mechanism around the same `dyn Write`. One
  supplied sink still captures every notice, `main` still names `io::stderr()`
  exactly once, and silencing still comes free from a discarding sink. Only the
  spelling moved, and it moved because a thread boundary made it.

  **Two alternatives were considered and refused, and the reasoning is recorded
  because the rejected one is the tempting one.** Keeping the borrowed
  `&mut dyn Write` and having the worker hand its two lines back for the
  collector to say satisfies the letter of the earlier ruling exactly — and moves
  both notices *below the device*, which kills both `shipped_binary.rs` subprocess
  readings. Those are the only instruments in the workspace that can see whether
  those two notices are ever actually said: deleting the `say_stand_ins` call left
  639 of 639 green before that reading existed. Trading a wiring guard for a type
  is trading evidence for tidiness. The third option — a borrowed sink for the
  seven and an owned one handed to the worker — fails on its own terms, because a
  caller who must remember to supply two sinks can forget one, and forgetting is
  the defect class this spec exists to close.

  Three conditions attach to the ruling:

  1. **On a poisoned lock, recover the inner sink and carry on.** A `Mutex`
     guarding a byte sink has no invariant a panic can corrupt, so poisoning it
     would mean an unrelated panic silently disables every later notice — this
     spec's own defect, reintroduced by its fix.
  2. **A write failure on the notice path is swallowed**, with the reason stated
     in one line at the site: there is nowhere to report a failure to report, and
     a notice must never be more fatal than the thing it describes. It is the
     single place in the crate where swallowing an error is correct.
  3. **Mutation evidence is the verification for this defect.**
     `standards/global/validation-calibration.md` excludes `crates/mc-client/`
     from coverage wholesale, so the percentage says nothing about this diff, and
     at rigor `low` there is no reviewer. For each of the nine notices, deleting
     or bypassing the sink call must redden something — through the *shipped*
     path for the two worker notices. Any that do not bite are reported as
     findings.

- [resolved] Q: Is the count eight? → A: **Nine**, as of this spec's own Defect 2.
  `notice::say_stand_ins` did not exist when the Defect 3 table was written. The
  count has now been wrong at four (the issue), seven (the as-built table) and
  eight (this spec); it is nine because the tree was counted with
  `grep -rn "eprintln!" crates/mc-client/src/`, whose tenth hit
  (`session/reload.rs:49`) is a doc comment naming `eprintln!` rather than a call
  site. That a hand-maintained list of these sites has drifted on every count is
  the argument for routing them through one seam rather than for counting more
  carefully.
