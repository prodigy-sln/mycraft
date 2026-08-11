# UI Design Standard — MyCraft

> Injected into every UI-touching prompt. These are instructions, not suggestions. Apply them
> directly. Deviations require explicit justification and user approval.

MyCraft's UI is a **game HUD and menu system**, not a web app. It is read at a glance, in motion,
often under threat, sometimes on a controller from three metres away. It is also **partly authored
by mod scripts**, which means consistency cannot rely on anyone's taste — it has to be enforced by
the token system and the widget set.

## 1. Priority order

When these conflict, resolve in this order. Do not reorder them for aesthetics.

1. **Legibility in motion** — readable over a moving, arbitrarily-coloured voxel world
2. **Information latency** — health, hunger, threat and hotbar state readable in under 200 ms
3. **Input directness** — the fewest inputs to the intended action
4. **Consistency** — base-game and mod UI look and behave like one product
5. **Aesthetics**

## 2. Legibility over an uncontrolled background

The world behind the HUD is any colour, any brightness, and it moves. Therefore:

- **Never** place text or an icon directly on the world with no separating treatment. Every HUD
  element carries one of: an opaque panel, a scrim (min 60% opacity), or a 1px contrast outline
  plus drop shadow.
- Minimum contrast ratio **4.5:1** against the *worst-case* backdrop, not the average. Test against
  snow (near-white) and a night cave (near-black) — those are the two failure cases.
- Critical state (health, damage, low durability) must survive the **entire** palette. Never encode
  it in hue alone.

## 3. Colour and tokens

- All colour comes from tokens in `content/base/ui/tokens.luau`. **No literal colour values in
  widget code**, engine-side or script-side. A mod that hardcodes `#FF0000` cannot be reskinned or
  made colourblind-safe, and it will look wrong on every custom theme.
- Semantic tokens, not descriptive: `danger`, `warning`, `affirm`, `surface`, `surface-raised`,
  `text-primary`, `text-muted`, `focus-ring`. Never `red`, `light-grey`.
- Every semantic colour pairs with a **non-colour redundant channel** — icon shape, position, text,
  or pattern. Deuteranopia, protanopia and tritanopia simulations are part of UI acceptance
  criteria, not a later audit.
- Ship at least: default, high-contrast, deuteranopia-safe, protanopia-safe.

## 4. Typography and scale

- One display face, one monospace face (for coordinates, counts, debug). No third face.
- Type scale is a fixed ramp: `12 / 14 / 16 / 20 / 28 / 40` at 1× UI scale. Nothing in between.
- **UI scale is user-controlled from 0.75× to 2.5×** and every layout must survive both ends.
  A layout that only works at 1× is broken.
- Never render text below 12px at 1× scale. Counts on hotbar items are the minimum size that exists.
- Numbers that change frequently (ammo, counts, coordinates) use **tabular figures** so they do not
  jitter.

## 5. Spacing and layout

- 4px base unit; spacing is `4 / 8 / 12 / 16 / 24 / 32 / 48`. No arbitrary values.
- Safe area: keep all HUD content within **5% inset** from every screen edge. TVs and ultrawides
  cut corners.
- Anchor HUD elements to screen corners and edges, never to absolute pixel positions — the same
  layout runs at 1280×720 and 5120×1440.
- The screen centre is reserved for the crosshair and interaction prompts. Nothing else may occupy
  it.

## 6. Input

- **Every action is remappable.** No hardcoded keybinds, including in mod UI. A mod that binds a key
  directly rather than declaring an action is a bug.
- Full keyboard navigation and full gamepad navigation for every menu. Mouse-only interaction is a
  defect.
- Focus is always visible — a `focus-ring` token, minimum 2px, never removed and never colour-only.
- Destructive actions (delete world, drop stack, kick player) require confirmation *or* are
  undoable. Prefer undoable.
- Hold-to-confirm for destructive actions in gamepad contexts, never a tiny target.

## 7. Feedback and game feel

- Every interaction acknowledges within **100 ms**, even if the result takes longer. Mining shows
  progress from the first tick.
- State changes animate in **80–150 ms**. Nothing in the HUD animates longer than 200 ms.
- **All animation respects a `reduce-motion` setting.** When set, transitions become instant cuts —
  never merely faster.
- Never rely on audio alone to convey state; every audio cue has a visual counterpart, and every
  spoken line has a subtitle.

## 8. Accessibility bar (non-negotiable)

These are acceptance criteria, not aspirations:

- Subtitles for all dialogue and significant sound effects, with a speaker label and a
  user-adjustable background opacity
- Colourblind-safe palettes covering all three common types
- UI scale 0.75×–2.5×
- Full remapping, keyboard and gamepad
- Reduce-motion setting honoured everywhere
- No pure-flash effects above 3 Hz (photosensitivity)
- Screen-shake and camera-bob independently disableable

## 9. Mod-authored UI

Mods build UI from the **same widget set** as the base game. This is what keeps a server running 40
mods from looking like 40 products.

- Mods compose from provided widgets and layout primitives; they do not draw raw rectangles and text
  for interface chrome.
- Mods cannot override global tokens. They may define *scoped* tokens for their own panels, which
  still resolve through the active theme.
- Mod UI is **never authoritative** — it requests, the server decides. A UI that displays a
  client-computed value as fact is a bug (see `crates/mc-net/CLAUDE.md`).
- Mod UI inherits accessibility settings automatically; a mod cannot opt out of reduce-motion, UI
  scale, or subtitles.

## 10. Anti-generic guidance

Do not ship the defaults. Specifically:

- **No stock egui look in player-facing UI.** `egui` is for debug and tooling only. If a player can
  see it during normal play, it is not egui.
- Do not copy Minecraft's UI. Matching genre conventions where they aid comprehension (hotbar at
  bottom-centre, inventory grid) is correct; copying its texture treatment, dirt-block backgrounds,
  or bevelled button style is not.
- No generic dark-grey-panel-with-blue-accent dashboard aesthetic. This is a game, not an admin
  console.
- Diegetic where it costs nothing, abstract where legibility demands it. Legibility wins every
  time — an in-world diegetic health readout that cannot be read while being attacked is a failure,
  however elegant.
- Commit to a deliberate visual identity and write it into the token file. "Whatever looked fine at
  the time" is how 40 inconsistent panels happen.

## 11. Review checklist

A UI change is not done until all of these hold:

- [ ] Readable against both snow and an unlit cave
- [ ] Correct at 0.75× and 2.5× UI scale
- [ ] Fully navigable by keyboard and by gamepad
- [ ] Focus visible on every interactive element
- [ ] No hardcoded colours, spacings, or keybinds
- [ ] Passes all three colourblind simulations
- [ ] Honours reduce-motion
- [ ] Any audio cue has a visual counterpart
- [ ] Nothing client-computed is presented as authoritative
