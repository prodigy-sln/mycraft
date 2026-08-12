# Spec Registry

Append-only record of every completed spec — one line each, written by
`/sdd-complete`. Lines persist even when spec folders are deleted or
pruned; the branch name resolves the full content through git history when
needed. This project has no pull requests (`standards/global/git-workflow.md`).

Format: `[folder] · [completed] · [rigor] · [tags] · [summary] · [branch]`

---

- 2026-08-11-frame-capture-harness · 2026-08-11 · high · rendering, testing, golden-frames, wgpu, mc-testkit · Headless frame-capture harness in `mc-testkit`: offscreen wgpu capture, CIELAB ΔE golden comparison and artifact reporting, landing before the renderer it verifies · `feature/PRO-849-frame-capture-harness`
- 2026-08-11-chunk-storage-palette · 2026-08-12 · high · world-format, chunk-storage, palette, block-registry, block-identity, mc-core, mc-world · Palette-compressed 16³ sections in 256-block columns, plus a block registry populated only from data under `content/base/`: a section's storable identity is namespaced names, never runtime ids, so a world survives a change of registration order · `feature/PRO-850-chunk-storage-palette`
- 2026-08-12-greedy-mesher · 2026-08-12 · high · meshing, rendering, determinism, neighbour-culling, benchmarks, mc-world · Binary greedy mesher in `mc-world`: a section's visible faces merged into quads by a fixed scanline sweep and culled against six optional neighbour sections, emitted in a total order that *is* the loop nesting rather than a sort, so identical contents mesh byte-identically whatever the write history, compaction state or registration order — which is what makes a golden frame captured against the mesh reproducible · `feature/PRO-851-greedy-mesher`
