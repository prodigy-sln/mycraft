# Testing & Verification

How MyCraft is checked, and — because much of this work is done by an agent that cannot look at a
screen — how results are verified without a human watching.

Governing standards: `standards/global/testing.md` (TDD, coverage, mocking) and
`standards/global/code-quality.md`. Coverage exclusions: ADR-008 in `decisions.md`.

## The quality gate

`scripts/sdd-gate.ps1` must exit 0 at every phase end, before validation, and before completion.
PowerShell 7 is cross-platform, so this is the only gate script — there is no `.sh` twin to drift
out of sync.

Every stage runs even when an earlier one fails, so one invocation reports every problem.

| # | Stage | Tool | Fails on |
|---|-------|------|----------|
| 1 | format | `cargo fmt --check` | any deviation |
| 2 | lint + complexity | `cargo clippy -D warnings` | any lint, incl. complexity thresholds |
| 3 | size | built-in | source > 500 lines, tests > 600 |
| 4 | deps | `cargo machete` | any unused dependency |
| 5 | sast | `cargo deny` | vulnerabilities, bad licenses, banned crates, untrusted sources |
| 6 | secrets | `gitleaks` | credentials in the working tree |
| 7 | tests + coverage | `cargo llvm-cov nextest` | test failure, or lines < 80% |

Flags: `-Quick` runs stages 1–3 only, for tight edit loops. `-SkipCoverage` runs tests without
instrumentation. Neither is valid at a phase boundary or in CI.

### Complexity thresholds

`clippy.toml` makes `code-quality.md` §2 machine-enforceable rather than reviewer-enforced:

| Limit | Value | Source |
|-------|-------|--------|
| Function length | 30 lines | code-quality.md §2 |
| Arguments | 4 | code-quality.md §2 |
| Nesting depth | 3 (= 2 nested blocks) | code-quality.md §2 |
| Cognitive complexity | 15 | firmer than clippy's default 25 |
| File length | 500 / 600 test | code-quality.md §2 |
| Banned names | `data`, `info`, `temp`, `result`, `item`, … | code-quality.md §3 |

Also lint-denied workspace-wide: `unwrap`, `expect`, `panic!`, `dbg!`, `todo!`, raw indexing,
`mem::forget`, float equality, swallowed errors. Raw indexing matters most in `mc-net`, where a
length prefix is attacker-controlled.

### Supply chain

`deny.toml` governs stage 5. The license list is an **allowlist**, not a denylist, because the
client ships to players and a copyleft dependency reaching the binary is a distribution problem.
Advisory suppressions require a dated justification inline — never a silent skip.

### Secrets — a coupling that must not be broken

Stage 6 scans the **working tree**, not git history, because a secret is cheapest to remove before
it is committed. That means the scanner also sees `.env`, which legitimately holds live API keys
for asset generation (ADR-009). `.gitleaks.toml` therefore allowlists `.env` and `.env.local`.

**That allowlist is only safe because the gate separately asserts those paths are still
git-ignored.** If someone removes the `.gitignore` entry, the allowlist would silently hide a
genuinely committed secret — the allowlist turns from a convenience into a blindfold. The gate
runs `git check-ignore` on each allowlisted path and fails if any is no longer ignored.

Do not remove one half without the other. The check is verified by a negative test: un-ignoring
`.env` must turn the gate red.

`.env.example` is deliberately **not** allowlisted. It is tracked, so it must never contain a real
value, and the scanner should catch it if it ever does.

## Verification without a human at the screen

Most of this project is built by an agent that cannot see the game window, cannot feel input lag,
and cannot invite 32 people to a server. Each of those gaps has a specific mechanism, and **the
mechanism is always built before the thing it verifies.**

### Rendering — golden frames

`mc-render` is excluded from coverage (ADR-008), so golden frames are the only automated check on
it. That makes the harness load-bearing.

- Headless wgpu renders to an offscreen texture and writes PNG; no window, no compositor.
- Fixed world seed, scripted camera path, fixed tick count — byte-identical inputs every run.
- Comparison is **perceptual**, not byte-wise, because GPU drivers legitimately differ.
- A changed golden must be justified in the commit. An unexplained golden update is a review stop
  (`validation-calibration.md`).
- Anything expressible as a pure function — meshing, vertex packing, culling maths, atlas layout,
  light propagation — is unit-tested normally and gets no exemption. Only GPU-resident work does.

Keeping logic out of the GPU-touching layer is therefore a testability requirement, not a style
preference.

### Multiplayer — bot clients

`mc-testkit` runs headless bots against the **real** network stack. A test that bypasses the
transport is not a network test.

- Scripted behaviour: join, authenticate, move, edit, chat, disconnect.
- Machine-readable output: tick time distribution, p99 latency, bandwidth per class, desync count.
- Scale target: 32 concurrent bots, p99 tick < 25 ms, zero desyncs over 10 minutes.

### Security — adversarial bots

The M4.5 suite is a baseline, not a ceiling. Each attack is a test asserting rejection *and*
logging, with the server unaffected: speed hack, reach hack, edit flood, chat flood, inventory
dupe, malformed and oversized packets, replayed auth challenges.

`proptest` fuzzes packet structure; explicit cases cover known attack shapes.

### Scripting — reload and sandbox

- Sandbox escapes get one explicit test each. A test asserting `io` is absent is worth more than
  ten happy-path binding tests.
- Every limit — instruction budget, memory cap, failure-disable threshold — has a test that trips
  it.
- Hot reload is tested for **state preservation**, not merely that new code ran.
- Every reload failure path (syntax error, failed validation, failing mod test, error inside
  `on_reload`) must leave the previous registry serving.
- Mods' own `tests/` run on every reload candidate before the swap, making them a safety
  mechanism rather than a formality.

### Performance — benchmarks as gates

`criterion` benchmarks with committed baselines, so regressions are caught numerically rather than
felt. Key budgets: chunk meshing < 200 µs/section; server tick p99 < 25 ms at 32 players and 500
active NPCs.

## What automation cannot check

Stated plainly, because pretending otherwise is how these get missed:

- **Game feel** — jump arc, mining cadence, camera response, input weight. Needs a human. Mitigated
  by making every such value hot-reloadable, so tuning never requires a rebuild.
- **Whether a golden frame looks *good*** — the harness proves it did not change, not that it was
  ever right.
- **Art direction and audio quality.**
- **Whether a scripting API is pleasant to write against** — completeness is provable (the base
  game uses it), ergonomics is not.
