---
name: sdd-complete
description: "Finalize a validated feature: consolidate into docs/, register the spec, dispose of the spec folder, then merge to main and push"
argument-hint: "[spec folder name]"
allowed-tools: Read, Write, Edit, Glob, Grep, Bash, Task
---

# Complete

Run after every feature, not once per release. This is what keeps `docs/`
as-built and `specs/active/` free of finished work.

**This project has no pull requests** (`standards/global/git-workflow.md`).
Completion is therefore a single pass with no approval wait: the quality gate
and a PASS validation *are* the merge condition. There is no second phase and
nothing to come back to later.

Prerequisites — all four, no exceptions:

- `validation-report.md` PASS (low tier: green gate note in `spec.md`)
- at high+, the user has signed off
- `scripts/sdd-gate.ps1` exits 0
- working tree clean

Any prerequisite unmet → report which one and stop. Never complete around a red
gate.

The disposal mode comes from CLAUDE.md Prospect settings:
`spec-disposal: delete` (default) or `archive` with `retention: [days]`.

## Steps

1. Run `scripts/sdd-gate.ps1` once more — red gate = stop.
2. Spec frontmatter: `status: implemented`, `completed: [today]`.
3. Consolidate into `docs/`: when `docs/INDEX.md` exists, delegate to the
   `docs-consolidator` agent with the spec folder path. When missing, offer to
   generate it from `specs/_templates/docs-index.template.md`; if declined,
   append a completion summary to `docs/CHANGELOG-features.md`.
   Register every new or updated file in `docs/INDEX.md` — the File Registry
   lists only files that exist.
4. Append one line to `specs/REGISTRY.md`:
   `[folder] · [date] · [rigor] · [topic tags] · [one-line summary] · [branch]`
5. Commit: `docs: consolidate [feature] into living docs`.
6. Dispose of the spec folder **on the feature branch, before merging**:
   - **delete mode**: `git rm -r specs/active/[folder]`, commit
     `chore: remove spec working folder`
   - **archive mode**: `git mv` to `specs/archive/YYYY/[folder]/`, and in the
     same commit `git rm` archive folders older than the retention setting
7. **Squash merge to `main` locally** — `git merge --squash`, then one commit.
   One spec is one commit on `main`, never a run of "feat docs docs feat test
   docs". `standards/global/git-workflow.md` §3 is explicit that there is no
   meaningful-history exception: the RED → GREEN → REFACTOR sequence is a
   discipline enforced while the branch is live, not a record `main` is required
   to carry. The spec folder was removed in step 6, so `main`'s tree never
   carries it either.
8. Push `main` to `origin` (backup remote), then delete the feature branch
   locally and on the remote.
9. Outstanding Info findings → one Linear issue each, referencing the registry
   line:
   `linear-cli issues create "<finding>" --team PRO --project <id> -s Backlog -d "<detail + registry ref>"`
   Then move the feature's own issue to Done:
   `linear-cli issues update PRO-123 -s Done`
   A `linear-cli` failure is reported, never a reason to leave the merge
   half-done.

## Output

```
Completed: [title]
Docs: [files updated] · Registry: [line appended]
Merged: [branch] → main ([sha]) · Pushed: [yes/no]
Spec disposal: [removed | archived to path]
Deferred: [n Info findings → PRO-…]
```
