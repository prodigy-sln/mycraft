# Validation Calibration

These rules govern severity, evidence, volume, and skip decisions for this
repository. Apply them exactly.

## Severity

- **Blocker**: breaks a primary user path, loses or corrupts data, or opens
  an auth-bypass, injection, or credential-exposure vulnerability.
- **Major**: produces wrong results, violates an acceptance scenario, fails
  silently, or omits spec-mandated behavior.
- **Minor**: unhandled spec edge case off the primary path, or a misleading
  error message — objectively verifiable defects two developers would agree
  on.
- **Info**: everything subjective. Style preferences are Info, never Minor.

## Evidence bar

Every finding MUST include a `file:line` citation and a concrete failure
scenario (inputs/state → wrong observable outcome). Findings missing either
are discarded. Cite code you read, never infer behavior from names.

## Volume

Report at most 5 Minors; summarize the remainder as a count. When nothing
blocks, lead the summary with "no blocking issues".

## Skip

Do not spend review effort on anything below — the gate (`scripts/sdd-gate.ps1`) already fails the
build on it, so reporting it is noise.

**Generated, vendored, or non-source:**
- `target/`, `Cargo.lock`, `**/*.profraw`, `**/*.profdata`, `coverage/`
- Vendored C sources pulled in by `mlua` (Luau) and other `-sys` crates
- `artifacts/frames/` golden images — a changed golden is reviewed as a *decision*
  (was the visual change intended?), never as code
- `.prospect-incoming` files

**Enforced by the gate — never report as a finding:**
- Formatting (`cargo fmt --check`)
- Any clippy lint, including complexity thresholds: function length > 30, arguments > 4, nesting
  depth > 3, cognitive complexity > 15, banned generic names (`clippy.toml`)
- `unwrap` / `expect` / `panic!` / `dbg!` / `todo!` / raw indexing — all lint-denied workspace-wide
- File length over 500 lines (600 for tests) — the size stage catches it
- Unused dependencies (`cargo machete`)
- Known vulnerabilities, disallowed licenses, banned crates, untrusted sources (`cargo deny`)
- Committed secrets (`gitleaks`)
- Coverage below 80% lines

**Do NOT skip these, even though they look mechanical** — the gate cannot see them:
- Whether a test actually asserts its scenario's outcome, as opposed to merely executing the path
- Whether a golden-frame update was justified (`mc-render` is coverage-exempt per ADR-008, so
  review is the only remaining check on it)
- Whether a client-supplied value is trusted anywhere in `mc-net` or `mc-sim`
- Whether a new `mc-script` binding widens the sandbox
- Whether state that must survive hot reload is being held in Lua rather than the ECS

## Re-review

On a second validation pass, report only NEW findings of severity Major or
higher.
