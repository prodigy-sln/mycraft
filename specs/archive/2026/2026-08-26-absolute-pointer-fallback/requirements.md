# Requirements — PRO-962

## Clarifications

- [resolved] Q: Should the client difference the absolute stream always, or only
  when it has measured one?
  → A: Owner, verbatim: *"implement the cursor fix, but only as a fallback."*
  The relative path stays primary. The absolute handling engages only when the
  stream is measured to be absolute — not on a build flag, not on a setting, and
  not unconditionally.
- [resolved] Q: Can look be derived from `WindowEvent::CursorMoved` instead
  (PRO-962 candidate 3)?
  → A: No. The probe's phase B — cursor grabbed, exactly what the shipped client
  does while the player plays — recorded **no `CursorMoved` at all**. Phase A's
  cursor data is healthy, so a design validated on phase A alone would have
  shipped a completely dead camera. Candidate 1 (difference the absolute stream)
  is the only survivor.
- [resolved] Q: What distinguishes an absolute stream from a relative one, using
  only what arrives?
  → A: A sample is position-shaped when both components lie in `0..=65535` and at
  least one is at least 1000, and two consecutive such samples are required
  before the regime changes. Threshold and corroboration both come from the
  recording: 584 of 584 samples clear 1000, minimum `|x|` 21093, while `|y|` fell
  to 35 — so one component suffices and both must not be required. A relative
  stream goes negative within moments of ordinary play and never repeats a large
  value exactly; the recording does the opposite on both counts.
- [resolved] Q: What happens when the regime changes mid-session (an RDP session
  resumed on the local console, or the reverse)?
  → A: The same two-sample hysteresis runs in both directions. The sample that
  decides a change is spent as the kind the *new* regime says it is, never
  differenced against a stale anchor, so no transition produces a spin. Cost: one
  dropped sample per transition, which is roughly 15 ms of look.
- [resolved] Q: What converts an absolute unit into a device count?
  → A: Owner's ruling — a declared nominal display of 1920 × 1080,
  `units × 1920/65536` and `units × 1080/65536`. Measured against the recording it
  gives 1.150 counts per pixel horizontally and 1.185 vertically, so the axes
  agree to 3.1% and the rate runs 15-19% fast. The exact alternative — one
  `PointerPlatform` method answering from `primary_monitor()` — is declined
  because **nobody has measured what `primary_monitor()` returns inside an RDP
  session**, the remote extent or the host's; testability does not separate the
  two routes, and an earlier draft of the spec wrongly said it did. Conditions
  attached to the ruling: the nominal is documented *as* a nominal with its
  measured error, the player is told what to do when it feels wrong, and the
  sensitivity setting becomes a tracked follow-up.
- [resolved] Q: Is "a real delta would be zero when the mouse is still" the tell?
  → A: No, and PRO-962's comment says so wrongly. `event_loop.rs:2587` guards
  emission with `if x != 0.0 || y != 0.0`, so winit never emits a zero motion on
  either stream. The tell is the opposite: a relative stream emits **nothing**
  while the mouse is still, and the absolute stream keeps re-sending the same
  position. The conclusion is unchanged; the mechanism is not, and `(0,0)` is
  unusable as a detection signal.
- [resolved] Q: Where must the regime decision live?
  → A: In `Session`. Two build-failing scans force it —
  `tests/winit_boundary.rs` (no second file in `mc-client/src` may name the
  library) and `tests/seam_boundaries.rs` (no decision may arrive in
  `events.rs`) — and `Session` is the only part of the client a test can
  construct without a window.
- [resolved] Q: Does the `fix` path at rigor `high` reach a validate phase?
  → A: Yes. PRO-962's description claims the resolver never assigns `validate` to
  `work-type: fix`, and that is stale under Prospect 3.0.1:
  `.prospect/prompts/matrix.tsv` carries
  `fix  medium|high|xhigh|max  validate  shared/preamble.md,fix/validate.md`.
  Work-type `fix` and rigor `high` are the owner's decision and are not reopened
  here.
- [assumed] Q: Do tablets, touchscreens and VM guest pointers behave the same
  way?
  → A: Assumed yes — they set `MOUSE_MOVE_ABSOLUTE` and reach the same winit
  guard — but none was measured, so no scenario claims them and they are listed
  Out of Scope.
