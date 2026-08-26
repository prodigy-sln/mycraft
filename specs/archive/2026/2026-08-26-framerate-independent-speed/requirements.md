# Requirements — PRO-971

## Clarifications

- [resolved] Q: Is "the same speed at every framerate" approximate or exact?
  → A: Owner: "The speed should be the same regardless of framerate." Exact.
  With a carried residue below one quantum, two partitions of the same total
  elapsed time spend equal tick counts and give a bit-identical position; the
  tolerance derivation and its one hazard are in `spec.md`
  (§ Technical Considerations → The tolerance, derived from both directions).
- [resolved] Q: May the property be tested against a real clock?
  → A: No. Owner: "A test could easily even without attaching to real time and
  using ticks measure the speed at a certain framerate." Driving N frames at a
  simulated framerate through a fake clock the test owns is the required shape.
  No test sleeps.
- [resolved] Q: What bound applies to catch-up after a pathological frame gap?
  → A: Proposed 250 ms (15 ticks), clamped before accumulating, surplus
  discarded. Derived from a 100 ms floor — the frame interval of a 10 fps
  machine, the slowest rate at which the game is being played rather than hung —
  with 2.5x headroom. FR-3.1-S3 is what makes the floor falsifiable.
- [resolved] Q: Where does the pacing decision live so that a test can drive it?
  → A: In the client's drivable core (`crates/mc-client/src/session/`), fed an
  elapsed duration through the `OverlayClock` port the `App` already holds. Two
  independent constraints point there: `seam_boundaries.rs` forbids the tick
  vocabulary anywhere else under `crates/mc-client/src`, and the core is the only
  part of the client a test can construct without a window.
- [assumed] Q: Does the display rate the player saw actually differ from 60 Hz?
  → A: Assumed yes from the report ("warp around with super speed") and the
  arithmetic, which gives 2.4x at 144 Hz. The fix does not depend on the exact
  rate: it makes speed independent of every rate.

## Owner requirement, verbatim

Recorded here because it existed nowhere in this repository before this spec and
it is the acceptance bar:

> "A test could easily even without attaching to real time and using ticks
> measure the speed at a certain framerate. Also: The speed should be the same
> regardless of framerate."
