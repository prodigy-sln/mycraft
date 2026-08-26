# Requirements Ledger — SPEC-025

## Clarifications

- [resolved] Q: Is the stand-in texture itself the defect? → A: No. The stand-in
  is correct and must stay — it is what stops a missing key refusing a launch.
  This spec adds the missing art only. (PRO-972 description; conductor brief.)
- [resolved] Q: Which committed golden sets are re-shot? → A: All four. Every one
  of them holds the stand-in checkerboard today — measured 9.58 %, 9.58 %,
  18.96 % and 21.57 % of frame on 2026-08-26 against `a9c6663`.
- [resolved] Q: Does `SCENE_REVISION` move? → A: No, it stays at `r3`. The scene
  contract names pose, world, camera path, tick list, merge predicate and vertex
  format, and none of them moves: layers are assigned over the keys the block
  declarations name, `base:water` is already one of the eight and already sorts
  last, so baking an image for it adds no key and renumbers no layer. Same shape
  as the 2026-08-19 re-shoot, which held the revision for the same reason.
- [resolved] Q: Does this need transparency or alpha blending? → A: No. Water is
  drawn opaque; `occludes = false` is what lets the lakebed show through, by
  leaving faces unculled. Reading the renderer agrees with the brief — nothing in
  the draw path blends, and none is added. (Out of Scope.)
- [resolved] Q: Is SPEC-025 free? → A: Yes. SPEC-024 is
  `2026-08-26-absolute-pointer-fallback` (PRO-962) by its own frontmatter, and no
  folder or `docs/INDEX.md` row claims SPEC-025. Note the two places disagree
  about SPEC-023 — see spec.md `## Notes` item 3.
- [resolved] Q: What should water look like? → A: `#4c799e` / `#447196` /
  `#5481a6`, approved 2026-08-26. The register is derived — saturation, value,
  and the tone spread narrowing as the surface smooths — and the only free
  parameter left was the hue, which is not a contested judgement: water is blue.
  Escalating further would have gold-plated the process on a defect ranked third,
  and it is one hex character to reverse.
- [resolved] Q: Is the stale sentence at `docs/modding/voxel-models.md:354-358`
  in scope? → A: Yes, ruled in 2026-08-26. It is the sentence that lets this
  defect read as intended behaviour to a mod author, and it is a false statement
  in `docs/` that this spec's own subject falsifies — the category ranked Major
  twice on the previous spec. Repairing it here is cheaper than filing it.
- [resolved] Q: Anything else this fix falsifies? → A: Two more, both ruled in
  scope. `support/art.rs:55-62` states its tolerance as measured "over the seven
  images" — an eighth makes that premise false, so it is re-measured and the
  sentence rewritten. And the detection gap goes into
  `docs/technical/testing.md`, not only this folder: an oracle that falls back to
  the same generator the product does, paired with goldens minted from the
  renderer they grade, is a closed loop with no outside reference in it.
