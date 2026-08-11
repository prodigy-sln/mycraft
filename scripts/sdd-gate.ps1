#Requires -Version 7.0
<#
.SYNOPSIS
    MyCraft deterministic quality gate. Must exit 0 at every phase end, before
    validation, and before completion.

.DESCRIPTION
    Stages, in order:
      1. format      cargo fmt --check
      2. lint        clippy, zero warnings, with complexity thresholds (clippy.toml)
      3. size        file-length limits from code-quality.md §2
      4. deps        unused dependency detection (cargo-machete)
      5. sast        vulnerabilities, licenses, bans, sources (cargo-deny)
      6. secrets     credential scan (gitleaks, if installed)
      7. tests+cov   suite under coverage instrumentation, with a threshold

    Every stage runs even if an earlier one fails, so a single invocation reports
    the full list of problems rather than one at a time. Exits non-zero if any
    stage failed.

    PowerShell 7 is cross-platform — this is deliberately the ONLY gate script,
    so there is no second implementation to drift out of sync.

    Coverage excludes mc-render, mc-client and mc-server per ADR-008
    (docs/technical/decisions.md); the renderer is verified by golden-frame tests.

.PARAMETER SkipCoverage
    Skip coverage instrumentation and run tests directly. Fast local iteration
    only — CI and /sdd-validate must never use it.

.PARAMETER Quick
    Format, lint and size only. For tight edit loops; not a substitute for the
    full gate at any phase boundary.
#>
[CmdletBinding()]
param(
    [switch]$SkipCoverage,
    [switch]$Quick
)

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest

$RepoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $RepoRoot

# ── Policy ─────────────────────────────────────────────────────────────────────

# standards/global/testing.md §3
$LineThreshold = 80

# ADR-008: GPU and binary crates are outside the coverage denominator.
$CoverageExclude = 'crates[/\\](mc-render|mc-client|mc-server)[/\\]'

# code-quality.md §2 hard size limits. Rust has no "component vs service"
# distinction, so the general ceiling is the services limit (500) and test files
# get 600. The finer 400/200 caps stay reviewer judgement.
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
if ($Quick) { Write-Host "mode: QUICK (format + lint + size only)" -ForegroundColor Yellow }

# ── 1. Format ──────────────────────────────────────────────────────────────────
Invoke-Stage 'format (cargo fmt --check)' { cargo fmt --all -- --check }

# ── 2. Lint + complexity ───────────────────────────────────────────────────────
# Thresholds (function length, argument count, nesting depth, cognitive
# complexity, banned names) come from clippy.toml and mirror code-quality.md.
Invoke-Stage 'lint + complexity (clippy, zero warnings)' {
    cargo clippy --workspace --all-targets --all-features -- -D warnings
}

# ── 3. File size limits ────────────────────────────────────────────────────────
Write-StageHeader 'size (code-quality.md file limits)'
$oversized = @()
Get-ChildItem -Path (Join-Path $RepoRoot 'crates') -Recurse -Filter '*.rs' -File -ErrorAction SilentlyContinue |
    ForEach-Object {
        $rel = $_.FullName.Substring($RepoRoot.Length + 1)
        $count = (Get-Content -LiteralPath $_.FullName | Measure-Object -Line).Lines
        $isTest = $rel -match '[/\\](tests|benches)[/\\]' -or $_.Name -match '_test\.rs$'
        $limit = if ($isTest) { $MaxTestLines } else { $MaxSourceLines }
        if ($count -gt $limit) {
            $oversized += [pscustomobject]@{ File = $rel; Lines = $count; Limit = $limit }
        }
    }
if ($oversized.Count -gt 0) {
    $Failures.Add('size (files over limit)')
    Write-Fail "$($oversized.Count) file(s) exceed the size limit — split by responsibility:"
    foreach ($o in $oversized) { Write-Host "     $($o.File): $($o.Lines) lines (limit $($o.Limit))" -ForegroundColor Red }
}
else {
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

# ── 4. Unused dependencies ─────────────────────────────────────────────────────
# An unused dependency is dead supply-chain surface: it still gets audited,
# compiled, and shipped in the lockfile.
if (Test-ToolPresent 'cargo-machete' 'cargo install cargo-machete --locked') {
    Invoke-Stage 'deps (unused dependency scan)' { cargo machete }
}
else {
    $Failures.Add('deps (cargo-machete missing)')
}

# ── 5. SAST — supply chain ─────────────────────────────────────────────────────
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

# ── 6. Secret scan ─────────────────────────────────────────────────────────────
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

# ── 7. Tests + coverage ────────────────────────────────────────────────────────
# cargo-llvm-cov runs the suite under instrumentation, so this is a single pass
# rather than two full test runs.
if ($SkipCoverage) {
    Write-Host ""
    Write-Warn 'coverage skipped (-SkipCoverage). Not valid for CI or /sdd-validate.'
    if (Test-ToolPresent 'cargo-nextest' 'cargo install cargo-nextest --locked') {
        # --no-tests=pass: an empty suite is a valid skeleton state, not a failure.
        Invoke-Stage 'tests (nextest)' { cargo nextest run --workspace --no-tests=pass }
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

        # --no-tests=pass: an empty suite is a valid skeleton state. The coverage
        # threshold below is what catches "code exists but is untested".
        cargo llvm-cov nextest `
            --workspace `
            --no-tests=pass `
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
    Write-Host "GATE PASSED" -ForegroundColor Green
    Pop-Location
    exit 0
}

Write-Host "GATE FAILED — $($Failures.Count) stage(s):" -ForegroundColor Red
foreach ($f in $Failures) { Write-Host "  · $f" -ForegroundColor Red }
Pop-Location
exit 1
