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
- `crates/mc-render/goldens/**/*.png` — the committed golden images. A changed
  golden is reviewed as a *decision* (was the visual change intended?), never as
  code. `artifacts/frames/` is scratch capture output and is not committed.
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
- Whether a golden-frame update was justified. ADR-013 narrowed ADR-008's exclusion, so the
  exempt subtree is `crates/mc-render/src/gpu/` only — the rest of `mc-render` **is** measured.
- Anything in `crates/mc-client/` or `crates/mc-server/`. Both are excluded from coverage
  **wholesale**, so a coverage figure says nothing whatever about them and review is the only
  check they get. Read them as if no percentage existed — because for them, none does.
- Whether a client-supplied value is trusted anywhere in `mc-net` or `mc-sim`
- Whether a new `mc-script` binding widens the sandbox
- Whether state that must survive hot reload is being held in Lua rather than the ECS

## Blocking

At rigor `medium`, Minor findings are reported, never blocking — they
become tracked issues at completion. At `high+`, only findings the
verification stage CONFIRMS can block; plausible findings are reported for
the human.

## Re-review

On a second validation pass, report only NEW findings of severity Major or
higher, and re-verify fixes in place of fresh full sweeps.
