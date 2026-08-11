# Rendering

`mc-render` — the actual mesher, draw path and lighting — does not exist yet.
This file is not premature: it records the conventions the renderer is built
*to*, established and asserted ahead of the renderer by the headless
frame-capture harness in `crates/mc-testkit` (module `frame`). Nothing below
describes renderer behaviour, because there is none yet. It describes the
contract the first line of `mc-render` inherits, so that PRO-852's terrain
renderer never has to rediscover it.

## Orientation: row 0 is the top, clip-space y is up

**Framebuffer row 0 is the top of the image, and stays the top through
readback, comparison, PNG encode and PNG decode. No stage flips rows.**

**Clip-space y is up.** wgpu's framebuffer origin and clip-space y point in
opposite directions — this mismatch is the single most common source of
flipped output across the wgpu/WebGPU ecosystem, and resolving it is the
caller's responsibility, not the graphics API's.

Consequently: **a caller filling the top half of a render target writes
y > 0**. Any draw work — the capture harness's self-verification scene today,
`mc-render`'s vertex shaders tomorrow — must place geometry accordingly.

### Why this is written down before there is a renderer

A capture path that silently inverted rows would make every golden this
project ever commits wrong in the same direction — consistently, and
therefore invisibly. Against a solid-colour test fixture that is easy to
catch. Against terrain it is not: a vertically mirrored world looks entirely
plausible on a screenshot, and the first instinct when it appears is to
suspect worldgen, the camera, or the mesher, not the capture path that has
been silently upside-down since before the renderer existed. Chasing that
bug against terrain would be expensive and confusing; chasing it against a
computed 64×64 fixture with analytically known pixel values is neither.
That is why the convention above was settled and asserted by the capture
harness itself, before any renderer draws a single triangle, rather than
left as an assumption for `mc-render` to get right on faith.

## Capture pixel format

The offscreen render target the capture harness allocates is
`Rgba8UnormSrgb`. The hardware performs the sRGB encode on write — there is
no CPU-side sRGB encoding step anywhere in the capture path, and readback
copies texels verbatim. This is the standard path and the one a renderer is
expected to use for its own colour target.

Captured pixels are therefore 8-bit sRGB-encoded RGBA with **straight
(non-premultiplied) alpha**: nothing in the capture path multiplies a colour
by its alpha on write, and nothing divides it back out on read. A clear to
`(1.0, 1.0, 1.0, 0.25)` reads back as `(255, 255, 255, 64)` — the RGB
channels are untouched by the alpha value.

One colour target only: no depth buffer, no stencil, no MSAA. A renderer
that needs depth or multisampling is adding a new draw path, not extending
this one.

## Relationship to the frame-capture harness

These conventions are asserted, not merely stated: `docs/technical/testing.md`
describes the harness that enforces them and the tolerance model comparisons
are judged against. See that file for how a captured frame is verified, and
`docs/technical/decisions.md` (ADR-008) for why golden-frame comparison is
the coverage strategy for GPU-resident rendering code at all.
