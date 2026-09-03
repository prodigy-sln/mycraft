#Requires -Version 7.0
<#
.SYNOPSIS
    MyCraft deterministic quality gate. Must exit 0 at every phase end, before
    validation, and before completion.

.DESCRIPTION
    Stages, in order, each reported under the name it is listed by here. A stage
    is what a reader sees after `ok:`, which is why the two GPU-free crates and
    the two verdicts of the instrumented run are listed apart: the gate reports
    them apart, and a list that folded them together would under-report itself.
      1.   format    cargo fmt --check
      2.   lint      clippy, zero warnings, with complexity thresholds (clippy.toml)
      2b.1 gpu-free  mc-testkit, clippy + tests with --no-default-features
      2b.2 gpu-free  mc-render, clippy + tests with --no-default-features
      2c.  docs      rustdoc, no broken or private intra-doc links
      3.   size      file-length limits from code-quality.md §2
      4.   deps      unused dependency detection (cargo-machete)
      5.   sast      vulnerabilities, licenses, bans, sources (cargo-deny)
      6.   secrets   credential scan (gitleaks, if installed)
      7.   art       no built texture image is under version control
      8.   art       the texture set is built from its manifest
      9.1  tests     the suite, under coverage instrumentation
      9.2  coverage  the line percentage that run measured, against a threshold

    Every test invocation carries --no-fail-fast, so a red stage reports the
    whole of its suite rather than stopping at the first failure. The flag costs
    nothing on a green run. cargo-llvm-cov offers a neighbouring flag whose help
    text reads more like that ask and which exits 0 on a failing suite; it is
    forbidden here by name, and docs/technical/testing.md records the measurement
    and the name so this script need not carry it.

    Every stage runs even if an earlier one fails, so a single invocation reports
    the full list of problems rather than one at a time. Exits non-zero if any
    stage failed.

    There is one exception: when stage 8 refuses, stage 9 does not run. A refused
    build leaves the set built by the previous run on disk, so the suite would
    grade stale art. The skip is recorded as a failure of its own, so the summary
    says the tests did not run rather than simply listing one stage fewer.

    PowerShell 7 is cross-platform — this is deliberately the ONLY gate script,
    so there is no second implementation to drift out of sync.

    Coverage excludes mc-client and mc-server wholesale, and mc-render's
    GPU-resident subtree src/gpu/, per ADR-008 as narrowed by ADR-013
    (docs/technical/decisions.md); that subtree is verified by golden-frame
    tests, and mc-render's pure layer is counted like any other library code.

.PARAMETER SkipCoverage
    Skip coverage instrumentation and run tests directly. Fast local iteration
    only — CI and the validate phase must never use it.

.PARAMETER Quick
    Stages 1 through 3: format, lint, the gpu-free clippy and test passes on
    mc-testkit and mc-render, the rustdoc documentation build, and size. It runs
    two real test suites and a documentation build — it is not a no-test mode.
    For tight edit loops; not a substitute for the full gate at any phase
    boundary.

.PARAMETER ArtOnly
    Stages 7 and 8 only, then the summary. A stage selector like -Quick, and the
    only way to observe what the art stages do without running the whole gate.

.PARAMETER ContentRoot
    The content root stage 7 inspects, as `<ContentRoot>/textures`. Used as
    given: a relative value is relative to the repository, which this script has
    already pushed into, and an absolute one is left alone.

.PARAMETER Manifest
    The texture manifest stage 8 builds. Used as given, on the same terms as
    -ContentRoot. `voxforge build` writes beside the manifest it is handed, so a
    run aimed at a copy builds into that copy.
#>
[CmdletBinding()]
param(
    [switch]$SkipCoverage,
    [switch]$Quick,
    [switch]$ArtOnly,
    [string]$ContentRoot = 'content/base',
    [string]$Manifest    = 'content/base/textures.toml'
)

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest

$RepoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $RepoRoot

# ── Policy ─────────────────────────────────────────────────────────────────────

# standards/global/testing.md §4
$LineThreshold = 80

# ADR-013 (superseding part of ADR-008): the binary crates stay outside the
# coverage denominator wholesale, but of mc-render only the GPU-resident subtree
# does.
#
# ADR-008 excluded all of mc-render by path because golden frames were the only
# thing that could cover it. mc-render now carries a default-on `gpu` Cargo
# feature under which `wgpu::` is nameable only in src/gpu/, and that mechanical
# boundary did not exist when ADR-008 was written. Everything outside it —
# geometry, packing, frustum maths, texture resolution, window and surface
# policy — is a pure function that crates/mc-render/CLAUDE.md says gets no
# exemption, so it is counted.
#
# mc-client and mc-server stay excluded wholesale: after PRO-852, mc-client
# holds only the winit event-loop adapter and composition wiring, every policy
# having moved into mc-render's pure layer. If logic ever accretes there, that is
# a new ADR and not a quiet edit to this line.
#
# `*_test.rs` files are excluded too. They are the `#[path = "x_test.rs"] mod
# tests;` siblings used to unit-test private items (docs/technical/testing.md),
# and because they are compiled into the *lib* target llvm-cov cannot tell them
# from library code. Being test code they are ~100% covered by construction, so
# counting them inflates the figure rather than diluting it — measured at 325 of
# 2587 tracked lines, worth about 0.4 points, and growing with every sibling
# added.
#
# The filename term is the only mechanism available: --ignore-filename-regex is
# file-granular, and stable Rust has no region-level opt-out (`#[coverage(off)]`
# is unstable). Keeping unit tests in their own file is what makes the exclusion
# reachable at all, and is one of the reasons the sibling layout is this
# project's convention.
#
# That matters more here than it would elsewhere: the exclusion that remains
# rests on golden-frame tests covering it, and mc-testkit is the crate carrying
# that bet. Its coverage number is the one figure standing behind the exclusion,
# so it has to measure library code and nothing else.
#
# Note that `crates/*/tests/` needs no entry — llvm-cov never counted it.
# Integration tests are separate crates and are excluded by default; verified
# against the JSON per-file list, not assumed.
$CoverageExclude = 'crates[/\\](mc-client|mc-server)[/\\]|crates[/\\]mc-render[/\\]src[/\\]gpu[/\\]|_test\.rs$'

# code-quality.md §2 hard size limits. Rust has no "component vs service"
# distinction, so the general ceiling is the services limit (500) and test files
# get 600. The finer 400/200 caps stay reviewer judgement.
#
# A sibling `*_test.rs` counts as a test file, so a `src/` file is measured on
# its production code alone. That is the point: the 500-line ceiling should
# pressure the code somebody has to read to understand the module, and a test
# module bolted onto the bottom of it would spend that budget on something a
# reader of the module never has to read.
$MaxSourceLines = 500
$MaxTestLines   = 600

$Failures = [System.Collections.Generic.List[string]]::new()

# ── Helpers ────────────────────────────────────────────────────────────────────

function Write-StageHeader {
    param([string]$Name)
    Write-Host ""
    Write-Host "── $Name " -NoNewline -ForegroundColor Cyan
    Write-Host ('─' * [Math]::Max(0, 58 - $Name.Length)) -ForegroundColor DarkGray
}

function Write-Ok   { param([string]$m) Write-Host "ok: $m"   -ForegroundColor Green }
function Write-Fail { param([string]$m) Write-Host "FAIL: $m" -ForegroundColor Red }
function Write-Note { param([string]$m) Write-Host "     $m"  -ForegroundColor DarkGray }
function Write-Warn { param([string]$m) Write-Host "     $m"  -ForegroundColor Yellow }

function Invoke-Stage {
    param([string]$Name, [scriptblock]$Action)
    Write-StageHeader $Name
    & $Action
    if ($LASTEXITCODE -ne 0) {
        $script:Failures.Add($Name)
        Write-Fail $Name
        return
    }
    Write-Ok $Name
}

function Test-ToolPresent {
    param([string]$Name, [string]$InstallHint, [switch]$Optional)
    if (Get-Command $Name -ErrorAction SilentlyContinue) { return $true }
    if ($Optional) {
        Write-Warn "optional tool '$Name' not installed — stage skipped. Install: $InstallHint"
    }
    else {
        Write-Fail "required tool '$Name' not installed. Install: $InstallHint"
    }
    return $false
}

Write-Host "MyCraft quality gate" -ForegroundColor White
Write-Host "repo: $RepoRoot" -ForegroundColor DarkGray
if ($Quick) { Write-Host "mode: QUICK (format, lint, gpu-free tests, docs, size — two test suites; not a full gate)" -ForegroundColor Yellow }

$Banner = 'GATE'
if ($ArtOnly) {
    $Banner = 'ART STAGES'
    Write-Host "mode: ART ONLY (the two art stages — not a full gate)" -ForegroundColor Yellow
}

# ── Stages 1-6 ─────────────────────────────────────────────────────────────────
# -ArtOnly selects the art stages and nothing else, in one decision rather than
# six. It is a stage selector exactly as -Quick is, and it restates no stage:
# there is still one implementation of each, and what the selector changes is
# which of them run. It exists so the art stages can be observed from a test —
# a run of the whole gate inside the suite would run the suite a second time.
if (-not $ArtOnly) {
    # ── 1. Format ──────────────────────────────────────────────────────────────
    Invoke-Stage 'format (cargo fmt --check)' { cargo fmt --all -- --check }

    # ── 2. Lint + complexity ───────────────────────────────────────────────────
    # Thresholds (function length, argument count, nesting depth, cognitive
    # complexity, banned names) come from clippy.toml and mirror code-quality.md.
    Invoke-Stage 'lint + complexity (clippy, zero warnings)' {
        cargo clippy --workspace --all-targets --all-features -- -D warnings
    }

    # ── 2b. GPU-free configuration ─────────────────────────────────────────────
    # Two crates split into a GPU-free core and a wgpu layer behind a default-on
    # `gpu` feature: mc-testkit's frame harness, and mc-render's pure layer.
    # `--no-default-features` is the only configuration in which wgpu is absent from
    # the dependency graph, and therefore the only process in which no GPU adapter
    # *can* exist — which is what makes the comparison suite's "while the process
    # holds no GPU adapter" assert what it says. Every other stage above and below
    # runs with the feature on, so without this stage the seam decays to convention.
    #
    # Both crates are named explicitly rather than `--workspace`: the workspace flag
    # would unify features across members and re-enable `gpu` through mc-client,
    # which is the same mis-scoping mc-render's own dependency-graph test exists to
    # rule out.
    #
    # Deliberately NOT `--all-features`: that would re-enable `gpu` and make the
    # stage meaningless. Deliberately NOT `--no-tests=pass` either: a run with no
    # tests in it proves nothing, which is the one thing this stage exists to rule
    # out.
    #
    # Each crate is its own Invoke-Stage. As one stage of four `&&`-chained
    # commands, a failure in the first hid three and the summary still recorded a
    # single name; now a failing mc-testkit pair leaves the mc-render pair run and
    # reported.
    #
    # Within each stage the two commands are STILL chained with `&&`, because
    # Invoke-Stage inspects $LASTEXITCODE once, after the whole scriptblock — on
    # separate lines a clippy failure would be silently overwritten by a passing
    # test run. **That chain still cancels whatever follows a failing command**,
    # so a failing clippy here still hides its own crate's test run and the stage
    # still reports less than its full extent. `--no-fail-fast` bounds the other
    # half: a failing *test* now runs its whole suite and reports the complete
    # count. Removing the residual needs a change to how Invoke-Stage detects
    # failure, which is every stage's mechanism — filed as PRO-1011.
    Invoke-Stage 'gpu-free (mc-testkit, no default features)' {
        cargo clippy -p mc-testkit --no-default-features --all-targets -- -D warnings &&
        cargo nextest run -p mc-testkit --no-default-features --no-fail-fast
    }
    Invoke-Stage 'gpu-free (mc-render, no default features)' {
        cargo clippy -p mc-render --no-default-features --all-targets -- -D warnings &&
        cargo nextest run -p mc-render --no-default-features --no-fail-fast
    }

    # ── 2c. Documentation links ────────────────────────────────────────────────
    # rustdoc is the only tool that resolves intra-doc links, and nothing else in
    # this gate runs it. A `[`Type`]` or `[`module::func`]` pointing at something
    # that does not exist compiles, tests, lints and ships in silence — SPEC-004
    # carried a dangling `startup::prepare_scene` reference through a green gate for
    # exactly that reason, and it surfaced only when a human tried to import the
    # function the doc claimed existed.
    #
    # The stage was added with zero backlog: the whole workspace already passed the
    # day it went in, so it has never needed a grace period and a failure here is
    # always something introduced rather than something inherited.
    $PreviousRustdocFlags = $env:RUSTDOCFLAGS
    $env:RUSTDOCFLAGS = '-D warnings -D rustdoc::broken_intra_doc_links'
    Invoke-Stage 'docs (rustdoc, no broken intra-doc links)' {
        cargo doc --workspace --no-deps --quiet
    }
    $env:RUSTDOCFLAGS = $PreviousRustdocFlags

    # ── 3. File size limits ────────────────────────────────────────────────────
    # Every root holding Rust this project writes. `tools/` is here because it holds
    # a workspace member: walking `crates/` alone would leave anything under it
    # silently unmeasured, and the stage would report "all files within limits"
    # having never opened one.
    $SizeRoots = @('crates', 'tools')

    Write-StageHeader 'size (code-quality.md file limits)'
    $oversized = @()
    $measured = [ordered]@{}
    foreach ($root in $SizeRoots) {
        # Counted from what the walk returned, never from Test-Path: the
        # -ErrorAction below is what turns a mistyped root into zero results rather
        # than an error, so the count *is* the evidence the root was real.
        $found = @(Get-ChildItem -Path (Join-Path $RepoRoot $root) -Recurse -Filter '*.rs' -File -ErrorAction SilentlyContinue)
        $measured[$root] = $found.Count
        foreach ($file in $found) {
            $rel = $file.FullName.Substring($RepoRoot.Length + 1)
            $count = (Get-Content -LiteralPath $file.FullName | Measure-Object -Line).Lines
            $isTest = $rel -match '[/\\](tests|benches)[/\\]' -or $file.Name -match '_test\.rs$'
            $limit = if ($isTest) { $MaxTestLines } else { $MaxSourceLines }
            if ($count -gt $limit) {
                $oversized += [pscustomobject]@{ File = $rel; Lines = $count; Limit = $limit }
            }
        }
    }

    # Per root, never as a total. A total is vacuous at the granularity that
    # matters: crates/ contributes some four hundred files, so a mistyped `tools`
    # root contributes zero, the total is still four hundred, and the stage passes
    # while tools/ goes unmeasured — the very defect this guard exists to close, one
    # level down.
    foreach ($root in $SizeRoots) { Write-Note "root '$root': $($measured[$root]) file(s) measured" }

    $unmeasured = @($SizeRoots | Where-Object { $measured[$_] -eq 0 })
    if ($unmeasured.Count -gt 0) {
        $Failures.Add('size (declared root measured nothing)')
        foreach ($root in $unmeasured) {
            Write-Fail "root '$root' contributes no measured files — it is declared here but nothing under it was size-checked"
        }
    }

    if ($oversized.Count -gt 0) {
        $Failures.Add('size (files over limit)')
        Write-Fail "$($oversized.Count) file(s) exceed the size limit — split by responsibility:"
        foreach ($o in $oversized) { Write-Host "     $($o.File): $($o.Lines) lines (limit $($o.Limit))" -ForegroundColor Red }
    }
    elseif ($unmeasured.Count -eq 0) {
        Write-Ok "size (all files within limits)"
    }

    if ($Quick) {
        Write-Host ""
        Write-Host ('═' * 64) -ForegroundColor DarkGray
        if ($Failures.Count -eq 0) { Write-Host "QUICK CHECKS PASSED (not a full gate)" -ForegroundColor Yellow; Pop-Location; exit 0 }
        Write-Host "QUICK CHECKS FAILED — $($Failures.Count) stage(s):" -ForegroundColor Red
        foreach ($f in $Failures) { Write-Host "  · $f" -ForegroundColor Red }
        Pop-Location; exit 1
    }

    # ── 4. Unused dependencies ─────────────────────────────────────────────────
    # An unused dependency is dead supply-chain surface: it still gets audited,
    # compiled, and shipped in the lockfile.
    if (Test-ToolPresent 'cargo-machete' 'cargo install cargo-machete --locked') {
        Invoke-Stage 'deps (unused dependency scan)' { cargo machete }
    }
    else {
        $Failures.Add('deps (cargo-machete missing)')
    }

    # ── 5. SAST — supply chain ─────────────────────────────────────────────────
    # Vulnerabilities (RustSec), license compatibility, banned crates, and untrusted
    # sources. Configured in deny.toml.
    if (Test-ToolPresent 'cargo-deny' 'cargo install cargo-deny --locked') {
        Invoke-Stage 'sast (advisories, licenses, bans, sources)' {
            cargo deny --all-features check
        }
    }
    else {
        $Failures.Add('sast (cargo-deny missing)')
    }

    # ── 6. Secret scan ─────────────────────────────────────────────────────────
    # Optional: gitleaks is not a cargo tool, so a missing binary is a warning rather
    # than a gate failure. CI installs it and it becomes mandatory there.
    Write-StageHeader 'secrets (credential scan)'

    # .gitleaks.toml allowlists local credential files so a real key in .env is not
    # reported as a leak. That allowlist is ONLY safe while those paths stay
    # git-ignored — otherwise deleting a .gitignore line would make committed
    # secrets invisible to the scanner. Assert the precondition here.
    $secretFiles = @('.env', '.env.local')
    $notIgnored = @()
    foreach ($sf in $secretFiles) {
        if (Test-Path (Join-Path $RepoRoot $sf)) {
            git check-ignore -q $sf 2>$null
            if ($LASTEXITCODE -ne 0) { $notIgnored += $sf }
        }
    }
    if ($notIgnored.Count -gt 0) {
        $Failures.Add('secrets (credential file not git-ignored)')
        Write-Fail 'credential file is allowlisted in .gitleaks.toml but NOT git-ignored:'
        foreach ($f in $notIgnored) { Write-Host "     $f — restore its .gitignore entry" -ForegroundColor Red }
    }

    if (Test-ToolPresent 'gitleaks' 'winget install Gitleaks.Gitleaks' -Optional) {
        # `dir` scans the working tree, not git history — we want secrets caught
        # BEFORE they are committed, which is the only point at which the fix is
        # cheap. History scanning belongs in CI, where a rewrite is still possible.
        gitleaks dir . --no-banner --redact --exit-code 1
        if ($LASTEXITCODE -ne 0) {
            $Failures.Add('secrets (gitleaks findings)')
            Write-Fail 'secrets — potential credentials committed'
        }
        else {
            Write-Ok 'secrets (none detected)'
        }
    }
    else {
        Write-Note 'skipped — install gitleaks to enable locally'
    }
}

# ── 7. Generated art is not committed ──────────────────────────────────────────
# The texture set is derived from the manifest by `voxforge build`, so a built
# image under version control is a stale copy of something the gate rebuilds one
# stage later. This stage is what keeps the .gitignore entry for the generated
# set from quietly drifting back out: delete the entry and the next commit that
# sweeps the tree fails here, by name.
#
# `git` runs against the real repository, which is why the stage takes the path
# it inspects and not a repository root — pointed at a temporary tree, git
# refuses the pathspec as outside the worktree and a clean fixture and a dirty
# one fail identically, for a reason that has nothing to do with the property.
#
# Both halves are needed. A pathspec naming a directory that does not exist is
# not an error to git: it reports nothing and exits 0, which is also what a
# clean tree looks like. So git's exit code decides whether the stage could look
# at all, and what it reported decides the verdict.
Write-StageHeader 'art (generated set not committed)'
$GeneratedArt = "$ContentRoot/textures"
$committedArt = @(git ls-files -- $GeneratedArt)
if ($LASTEXITCODE -ne 0) {
    $Failures.Add('art (generated set not committed)')
    Write-Fail "git could not inspect '$GeneratedArt' — the stage reached no verdict"
}
elseif ($committedArt.Count -gt 0) {
    $Failures.Add('art (generated set not committed)')
    Write-Fail "$($committedArt.Count) generated image(s) under version control — the set is built, never committed:"
    foreach ($image in $committedArt) { Write-Host "     $image" -ForegroundColor Red }
}
else {
    Write-Ok 'art (generated set not committed)'
}

# ── 8. The art is built ────────────────────────────────────────────────────────
# The suite grades whatever set is on disk, so the set is built before the stage
# that runs it — on both coverage paths, which is why this sits outside the
# choice between them, and after the -Quick early exit, so a tight edit loop
# pays nothing for it.
#
# Keyed on the exit code, never on the output being empty. `voxforge build`
# reports every manifest key no block declaration names, and that report is
# advisory by design: the shipped manifest bakes five such keys today, for a
# per-face declaration nobody has written yet. A stage reading a non-empty
# report — or a non-silent stderr — as a failure would take the gate down on the
# base game's own art the day it lands.
$ArtBuildStage = 'art (voxforge build)'
Invoke-Stage $ArtBuildStage { cargo run -p voxforge --quiet -- build $Manifest }

# ── 9. Tests + coverage ────────────────────────────────────────────────────────
# cargo-llvm-cov runs the suite under instrumentation, so this is a single pass
# rather than two full test runs.
#
# This is the one exception to "every stage runs even if an earlier one fails".
# A refused build leaves the set built by the *previous* run on disk, so a suite
# run after one would grade stale art and report a green that is about nothing.
# The tests are skipped — and the skip is recorded, because a summary that
# simply lists one stage fewer cannot tell "the tests did not run" from "the
# tests are not in this list". A gate that omits a stage silently is one step
# from a gate that skips its way to green.
if ($Failures.Contains($ArtBuildStage)) {
    # Recorded whatever this run selected, -ArtOnly included. Both statements are
    # true either way: the build did refuse, and the tests did not run. Making
    # the record conditional on stage 9 having been selected looks like a tidy-up
    # — a failure about tests nobody asked for — and it deletes the only thing
    # any bounded test can observe about this skip.
    $Failures.Add('tests (not run: art build failed)')  # never make this conditional on which stages were selected
    Write-Fail 'tests — not run: the art build refused and the set on disk is the previous one'
}
elseif ($ArtOnly) {
    Write-Note 'tests not run — this run selected the art stages only'
}
elseif ($SkipCoverage) {
    Write-Host ""
    Write-Warn 'coverage skipped (-SkipCoverage). Not valid for CI or the validate phase.'
    if (Test-ToolPresent 'cargo-nextest' 'cargo install cargo-nextest --locked') {
        # --no-tests=pass: an empty suite is a valid skeleton state, not a failure.
        Invoke-Stage 'tests (nextest)' { cargo nextest run --workspace --no-tests=pass --no-fail-fast }
    }
    else {
        $Failures.Add('tests (cargo-nextest missing)')
    }
}
else {
    $haveTools = (Test-ToolPresent 'cargo-nextest'  'cargo install cargo-nextest --locked') -and
                 (Test-ToolPresent 'cargo-llvm-cov' 'cargo install cargo-llvm-cov --locked')

    if (-not $haveTools) {
        $Failures.Add('tests + coverage (tooling missing)')
    }
    else {
        Write-StageHeader 'tests + coverage (llvm-cov nextest)'
        $covJson = Join-Path ([System.IO.Path]::GetTempPath()) "mycraft-cov-$([guid]::NewGuid()).json"

        # Windows only, and it is why `os error 206` is not what you are reading.
        #
        # `llvm-cov export` is invoked with one `-object <path>` per test binary in
        # the workspace. At 383 binaries that command line measured 32 883
        # characters against `CreateProcess`'s hard limit of 32 767, and the stage
        # failed with ERROR_FILENAME_EXCED_RANGE *after every test had passed* —
        # a report-generation failure wearing the costume of a test failure.
        #
        # Only the repeated prefix matters. Under `target/` each argument carries
        # `target\llvm-cov-target\debug\deps\` (34 chars); from a short absolute
        # root it carries `C:\mcv\debug\deps\` (18). Measured on a throwaway crate
        # rather than derived: cargo-llvm-cov emits the path absolute when the
        # target directory sits outside the working directory, and 16 chars saved
        # across 383 binaries is ~6 KB — about ninety more test binaries of room.
        # The directory is named tersely on purpose; every character is multiplied
        # by the binary count.
        #
        # **This is a reprieve, not a fix.** The limit returns at the rate this
        # workspace adds test files, and the real remedy is an LLVM response file
        # (`llvm-cov @args`), which is cargo-llvm-cov's to emit. When this fails
        # again, that is the thing to ask for upstream — not a shorter directory.
        # Tracked as **PRO-997**; the decision is ADR-031 in
        # docs/technical/decisions.md.
        #
        # Two consequences worth knowing. Coverage artefacts now live **outside
        # the repository**, so a cleaner that only knows about `target/` will miss
        # several gigabytes. And nothing here changes on non-Windows, where the
        # argument limit is orders of magnitude larger and `target/` stays put.
        #
        # The directory is namespaced per worktree. git-workflow.md §5 has more
        # than one agent holding this tree at a time and the project uses
        # `git worktree`; one fixed directory would let two concurrent runs write
        # coverage artefacts into the same place — a percentage silently mixing
        # two branches, or lock contention wearing an unrelated failure's costume.
        # The key is the absolute repository root: stable across a run's stages
        # and across runs, distinct per worktree, and — unlike a branch name —
        # it does not move and leave the previous directory orphaned.
        if ($IsWindows) {
            $rootHash = [System.Security.Cryptography.SHA256]::HashData(
                [System.Text.Encoding]::UTF8.GetBytes($RepoRoot.ToLowerInvariant()))
            $slot = '{0:x2}{1:x2}' -f $rootHash[0], $rootHash[1]
            $env:CARGO_LLVM_COV_TARGET_DIR = Join-Path $env:SystemDrive 'mcv' $slot
        }

        # --no-tests=pass: an empty suite is a valid skeleton state. The coverage
        # threshold below is what catches "code exists but is untested".
        #
        # --no-fail-fast is forwarded verbatim to nextest rather than consumed,
        # and nextest's exit code is preserved — so a red suite still fails this
        # stage, and its count is the complete `N tests run` form rather than the
        # cancelled `N/M`.
        cargo llvm-cov nextest `
            --workspace `
            --no-tests=pass `
            --no-fail-fast `
            --ignore-filename-regex $CoverageExclude `
            --json --output-path $covJson --summary-only

        if ($LASTEXITCODE -ne 0) {
            $Failures.Add('tests (nextest under coverage)')
            Write-Fail 'tests'
        }
        elseif (-not (Test-Path $covJson)) {
            $Failures.Add('coverage (no report produced)')
            Write-Fail 'coverage — llvm-cov produced no report'
        }
        else {
            Write-Ok 'tests'
            $totals = (Get-Content -LiteralPath $covJson -Raw | ConvertFrom-Json).data[0].totals

            if ($totals.lines.count -eq 0) {
                # Skeleton state: nothing coverable outside the excluded crates yet.
                # Passing is correct — there is no code that failed to be tested.
                Write-Ok 'coverage — no coverable lines yet (skeleton)'
                Write-Note "threshold ${LineThreshold}% applies once library code exists"
            }
            else {
                $linePct   = [math]::Round($totals.lines.percent, 2)
                $regionPct = [math]::Round($totals.regions.percent, 2)
                Write-Note ("lines {0}%  regions {1}%  ({2} lines tracked)" -f $linePct, $regionPct, $totals.lines.count)
                if ($linePct -lt $LineThreshold) {
                    $Failures.Add("coverage ($linePct% < $LineThreshold%)")
                    Write-Fail "coverage $linePct% below threshold $LineThreshold%"
                }
                else {
                    Write-Ok "coverage $linePct%"
                }
            }
            Remove-Item -LiteralPath $covJson -Force -ErrorAction SilentlyContinue
        }
    }
}

# ── Summary ────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host ('═' * 64) -ForegroundColor DarkGray
if ($Failures.Count -eq 0) {
    Write-Host "$Banner PASSED" -ForegroundColor Green
    Pop-Location
    exit 0
}

Write-Host "$Banner FAILED — $($Failures.Count) stage(s):" -ForegroundColor Red
foreach ($f in $Failures) { Write-Host "  · $f" -ForegroundColor Red }
Pop-Location
exit 1
