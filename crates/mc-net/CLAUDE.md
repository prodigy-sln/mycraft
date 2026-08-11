# mc-net — QUIC transport, identity, and abuse resistance

This crate faces the public internet. Everything arriving here is hostile until proven otherwise.

Rationale: `docs/technical/decisions.md` ADR-006 (QUIC) and ADR-007 (public-key identity).
Protocol detail: `docs/technical/protocol.md`. Operator view: `docs/ops/administration.md`.

## The one rule

**The server is authoritative. A client message is a request, never a fact.**

Every value a client sends is recomputed or validated server-side against the registry and world
state. If a code path trusts a client-supplied number, that is a Blocker regardless of how
convenient it is.

Concretely, always server-side:
- position and velocity (validated against terrain and a speed/flight envelope)
- block reach distance and break time
- inventory contents and every transaction
- damage, health, and hit registration
- crafting results
- quest progress

## Validation posture

Clamp and log; do not kick. Legitimate players hit envelope violations during lag spikes, and a
server that disconnects on a hiccup is worse than one that corrects. Kicks and bans are for
sustained, repeated violation — a policy decision, not a validator's.

Rate-limit per connection *and* per identity: connect attempts, chat, block edits, inventory
operations, and chunk requests. An unbounded chunk request is a bandwidth amplification vector.

## Identity (ADR-007)

- The ed25519 public key **is** the account. Display names are claimed first-come and are not
  identity — never key anything off a display name.
- Login is a signed challenge-response over the already-TLS'd QUIC channel. Never send a private
  key, and never log one.
- Argon2 password login is an opt-in secondary path, off by default. If you are adding password
  handling, re-read `standards/global/code-quality.md` §7 first.
- Each server is its own trust root. There is no central authority to defer to.

## Wire protocol

- `mc-proto` owns the format; this crate owns the transport. Do not define packets here.
- Traffic class is a deliberate choice per message, not a default:
  chunk sections → reliable stream · entity snapshots → unreliable datagram ·
  block edits, chat, inventory, quests → reliable stream.
- Every packet is versioned. A client with a mismatched protocol version is rejected at handshake
  with an actionable message, never left to desync.
- Deserialization is fallible and adversarial. Length prefixes are bounds-checked before
  allocation — an attacker-controlled length that reaches `Vec::with_capacity` is a Blocker.
  `indexing_slicing` is lint-denied in this crate for exactly this reason.

## Auditability

Privileged and destructive actions are journalled with actor, timestamp, and enough context to
undo: block edits (for region rollback), permission changes, bans, kicks. `docs/ops/moderation.md`
depends on this journal being complete — an unjournalled mutation path is a defect.

Never log: private keys, session tokens, password material, or raw packet payloads that may contain
them.

## Testing

- Every validator gets an adversarial test, not just a happy-path one. The M4.5 suite (speed hack,
  reach hack, edit flood, chat flood, inventory dupe) is the baseline, not the ceiling.
- Malformed, truncated, oversized and replayed packets are fuzz targets. `proptest` for structure,
  explicit cases for known attack shapes.
- Auth tests cover: wrong signature, replayed challenge, expired challenge, unknown key, banned key,
  and concurrent logins on one identity.
- Load tests run through `mc-testkit` bots on the real stack. A test that bypasses the transport is
  not a network test.
