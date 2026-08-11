# Spec Registry

Append-only record of every completed spec — one line each, written by
`/sdd-complete`. Lines persist even when spec folders are deleted or
pruned; the branch name resolves the full content through git history when
needed. This project has no pull requests (`standards/global/git-workflow.md`).

Format: `[folder] · [completed] · [rigor] · [tags] · [summary] · [branch]`

---

- 2026-08-11-frame-capture-harness · 2026-08-11 · high · rendering, testing, golden-frames, wgpu, mc-testkit · Headless frame-capture harness in `mc-testkit`: offscreen wgpu capture, CIELAB ΔE golden comparison and artifact reporting, landing before the renderer it verifies · `feature/PRO-849-frame-capture-harness`
