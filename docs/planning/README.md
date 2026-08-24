# Planning documents

This folder holds **design discussions for features in the planning state** —
feasibility analyses, open questions, and research syntheses for work that has
not been specced, let alone built.

**It is explicitly exempt from the "`docs/` describes the system as built"
rule** (`docs/INDEX.md`). Everything under `docs/planning/` is *forward-looking*
and carries no promise: any statement here may be contradicted by the spec that
eventually lands, and the spec wins. When a feature discussed here is actually
built, its permanent documentation is consolidated into the ordinary `docs/`
branches by the complete phase, and the planning document is either deleted or
trimmed to what remains unbuilt.

A planning document should:

- name the Linear issue(s) it elaborates,
- date its claims (measurements go stale; so do external references),
- separate **what was verified** (in-tree measurements, cited sources) from
  **what is derived** (reasoning that has not been observed anywhere), and
- record open questions as questions, not as decisions.

| File | Topic | Issues |
|------|-------|--------|
| city-generation.md | City generation as a partition: feasibility, infinite-height buildings, layered streets and transit, lighting | PRO-935 |
| client-server-split.md | What the client is allowed to evaluate: the agreement test, the three tiers, texture packs, and where the composition root belongs | PRO-917 |
| block-render-methods.md | A block declares which engine render method draws it; liquids get an inset, wavy surface; why water is invisible today and what the golden re-shoot costs | PRO-952, PRO-904, PRO-947 |
