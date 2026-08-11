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
- It merges into `main` once `/sdd-validate` passes and the gate is green.
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
- TDD sequence per task: `test: add failing tests for X` →
  `feat: implement X` → `refactor: improve Y` (refactor only when changes
  were made). Never mix test and implementation code in one commit.
- Commit after each RED, GREEN, and REFACTOR step, and when consolidating
  docs and registering the spec. Spec changes commit separately from code.
- Reference scenario IDs (FR-x.y-Sz) in commit messages on the feature branch —
  never in code or test names.
- Never: "WIP"/"temp" as messages, committing failing tests, bundling
  unrelated changes, secrets, `.env` files, build artifacts,
  commented-out code.

## 3. Merging

- The merge condition is: `/sdd-validate` PASS **and** `scripts/sdd-gate.ps1`
  exits 0 **and** spec status is `implemented` **and** docs are consolidated
  and the spec is registered in `specs/REGISTRY.md`.
- All four are required. A green gate alone is not sufficient, and no other
  approval substitutes for any of them.
- Squash merge preferred; merge commit acceptable when the branch history is
  meaningful (a full RED → GREEN → REFACTOR sequence usually is).
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
