# Git Workflow Standards (Constitution)

> Immutable. All AI-assisted development MUST comply. Violations require
> explicit justification and user approval.

MyCraft is a solo, agent-driven project. There is no reviewer and no PR
ceremony. The quality gate carries the weight that human review carries
elsewhere, which is why it is strict and why a red gate is an absolute stop.

## 1. Branches

- Naming: issue-driven `feature/PRO-123-short-name`, `bugfix/` same pattern.
  Kebab-case, 2–4 words, max 50 characters total.
- The branch is created at the start of `/sdd-start`; all work — spec, tests,
  code — happens on it.
- It merges into `main` once the validate phase passes and the gate is green.
  Merge locally; nothing waits on review.
- Rebasing to tidy a feature branch before merge is encouraged. After an
  intentional rebase, publish it with `git push --force-with-lease`.
- `--force-with-lease` only. Never bare `--force`: the lease is what makes the
  push abort instead of silently discarding a commit that arrived after your
  last fetch.
- `main` always builds and always passes the gate. That is the only property
  `main` is required to have.

## 2. Commits

- Conventional Commits: `<type>(<scope>): <description>` with types
  feat, fix, test, refactor, docs, chore, style. Imperative mood, present
  tense, description ≤72 characters.
- TDD cadence: `test: add failing tests for X` once per phase (the test
  author's commit) → `feat: implement X` per task → `refactor: improve Y`
  once per phase, only when changes were made. Never mix test and
  implementation code in one commit.
- Commit when consolidating docs and registering the spec. Spec changes
  commit separately from code.
- Reference scenario IDs (FR-x.y-Sz) in commit messages on the feature branch —
  never in code or test names.
- Never: "WIP"/"temp" as messages, committing failing tests, bundling
  unrelated changes, secrets, `.env` files, build artifacts,
  commented-out code.

### Staging

- **Stage explicit paths.** `git add -A` and `git add .` are banned, with
  no exception for "the tree is clean, I checked". A sweep once pulled a
  test author's in-flight file into an implementation commit, which is
  exactly the separation the TDD sequence above exists to keep.
- **Revert a mutation check by hand** — re-edit the line you broke. Never
  `git checkout -- <file>`: that discards everything uncommitted in the
  file, and it once wiped a whole implementation that had not been
  committed yet. Confirm with `git diff --exit-code` afterwards.
- **No `.gitignore` rule for `*.proptest-regressions`.** A seed file is
  either evidence or litter, and which one depends on why the test failed:
  delete the seeds written by a deliberate mutation, and **commit** the
  seeds written by a genuine failure. That is proptest's own convention
  and the only artifact proving a real bug stays fixed.

## 3. Merging

- This project runs `review-mode: solo` (CLAUDE.md): the complete phase
  merges directly once the prerequisites below hold. The setting is what the
  resolver reads — changing this section without changing it is a no-op.
- The merge condition is: validate-phase PASS **and** `scripts/sdd-gate.ps1`
  exits 0 **and** spec status is `implemented` **and** docs are consolidated
  and the spec is registered in `specs/REGISTRY.md`.
- All four are required. A green gate alone is not sufficient, and no other
  approval substitutes for any of them.
- **Squash merge, always.** One spec is one commit on `main`, never a run of
  "feat docs docs feat test docs". There is no meaningful-history exception —
  a standing permission to keep the branch history is how the exception
  becomes the habit. The RED → GREEN → REFACTOR sequence is a discipline
  enforced while the branch is live, not a record `main` is required to
  carry.
- Delete the feature branch after merging.

## 4. Remote

- `origin` is `github.com/prodigy-sln/mycraft` and exists for **backup**, not
  review.
- Push `main` and the active feature branch often — at minimum at every stage
  boundary and before any long-running operation. Unpushed work is unbacked
  work.
- A push failure is reported, never silently retried in a loop. Rewriting
  history is a legitimate response to an intentional rebase, never a way to
  paper over a push you do not understand — diagnose first, then decide.

## 5. A shared working tree

More than one agent may hold this working tree at once, and there is no
mechanism that tells you when a second one arrives. The rule that follows
from that is one sentence:

**An observation of the tree ages exactly as fast as anybody else's.** A
`git status` is a statement about a moment, not a standing fact, and every
inference resting on one inherits that expiry — including inferences about
your own work.

- **Re-read the tree immediately before any merge, revert, or gate
  conclusion.** `git stash list && git status --short`, then act. A gate
  reading is a statement about a tree, not about a commit.
- **A red test you did not break may already be fixed.** Check whether
  `HEAD` moved before concluding anything from a failure — a gate was once
  held for an hour on failures that had been repaired before it finished
  running.
- **"Nothing to commit" can mean somebody else committed your edit.** The
  question is whether `HEAD` moved, not whether you remember committing.
- **Announce a mutation window before deliberately breaking the tree.** Two
  agents mutating one tree produces exactly the signature of a flaky test.
  The failing-test *count* is what distinguishes them afterwards, so record
  it.
- **Never stage a path you did not write.** This is why `git add -A` is
  banned above and why the ban has no "the tree is clean, I checked"
  exception: the check and the commit are two different moments.
- **Never revert a file to repair something you did not do.** Another
  agent's uncommitted work looks identical to litter. It is not yours to
  discard — and neither is a human's: acceptance edits made by hand have
  been reverted out from under a person mid-session by an agent tidying
  what it read as a stray change.
