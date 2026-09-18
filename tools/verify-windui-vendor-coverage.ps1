#!/usr/bin/env pwsh
# Coverage guard for the vendored WindUI tree.
#
# `tools/sync-windui.ps1` rebuilds `third_party/wind-ui-rust` from a pinned upstream
# commit plus the patches registered in that script. If a local WindUI file is ever
# added (or a tracked file changes) without a patch being registered, the next sync
# would silently drop it -- that is the failure mode this guard catches.
#
# It asserts:  every path in the vendored tree  <=  upstream paths + registered patch paths
#
# A path only mentioned by a patch is fine (it is produced during replay); a path that
# is in neither upstream nor any registered patch means the vendored delta is
# unreplayable. Cheap by design: it fetches the commit without a checkout and reads
# file names only, so it needs no toolchain and no cargo build.
#
# Usage:  pwsh -File tools/verify-windui-vendor-coverage.ps1
# Exit:   0 = every vendored path is accounted for; 1 = unaccounted paths (unreplayable)

[CmdletBinding()]
param(
  [string] $UpstreamUrl = 'https://github.com/huanfeng/wind-ui-rust.git'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$vendorDir = Join-Path $repoRoot 'third_party/wind-ui-rust'
$patchDir = Join-Path $repoRoot 'third_party/patches/wind-ui-rust'
$depsFile = Join-Path $repoRoot 'third_party/dependencies.json'
$syncScript = Join-Path $repoRoot 'tools/sync-windui.ps1'
$scratch = Join-Path $repoRoot 'out/tmp/windui-coverage'

function Fail([string] $message) {
  Write-Output "FAIL: $message"
  exit 1
}

foreach ($path in @($vendorDir, $patchDir, $depsFile, $syncScript)) {
  if (-not (Test-Path -LiteralPath $path)) { Fail "missing required path: $path" }
}

# The pin is owned by two independent records; require them to agree.
$deps = Get-Content -LiteralPath $depsFile -Raw | ConvertFrom-Json
$windUi = $deps.packages | Where-Object { $_.name -eq 'wind-ui-rust' }
if (-not $windUi) { Fail 'third_party/dependencies.json has no wind-ui-rust entry' }
$depCommit = ($windUi.source -split '/')[-1]
$constMatch = Select-String -LiteralPath (Join-Path $repoRoot 'rust/config-poc/src/main.rs') `
  -Pattern 'WIND_UI_RUST_REFERENCE_COMMIT: &str = "([0-9a-f]+)"'
if (-not $constMatch) { Fail 'WIND_UI_RUST_REFERENCE_COMMIT not found in rust/config-poc/src/main.rs' }
$constCommit = $constMatch.Matches[0].Groups[1].Value
if ($depCommit -ne $constCommit) {
  Fail "pin mismatch: dependencies.json=$depCommit config-poc=$constCommit"
}
Write-Output "pin: $depCommit"

# Registered patch list, in the order the sync script applies them.
$registered = [regex]::Matches((Get-Content -LiteralPath $syncScript -Raw),
  "Join-Path\s+\`$patchDir\s+'([^']+\.patch)'") | ForEach-Object { $_.Groups[1].Value }
if (-not $registered) { Fail 'no registered WindUI patches found in tools/sync-windui.ps1' }
Write-Output ("registered patches: " + ($registered -join ', '))

$patchPaths = [System.Collections.Generic.HashSet[string]]::new()
foreach ($name in $registered) {
  $patchPath = Join-Path $patchDir $name
  if (-not (Test-Path -LiteralPath $patchPath)) { Fail "registered patch is missing: $name" }
  foreach ($m in [regex]::Matches((Get-Content -LiteralPath $patchPath -Raw), '(?m)^diff --git a/(\S+) b/')) {
    [void] $patchPaths.Add($m.Groups[1].Value)
  }
}
Write-Output ("paths covered by registered patches: " + $patchPaths.Count)

# Upstream paths at the pinned commit: fetch without checking out (names only).
if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force }
New-Item -ItemType Directory -Force -Path $scratch | Out-Null
$cloneDir = Join-Path $scratch 'upstream'
& git.exe clone --quiet --filter=blob:none --no-checkout $UpstreamUrl $cloneDir
if ($LASTEXITCODE -ne 0) { Fail 'git clone failed (offline?)' }
& git.exe -C $cloneDir fetch --quiet origin $depCommit
if ($LASTEXITCODE -ne 0) { Fail "git fetch of pinned commit failed: $depCommit" }
$upstreamPaths = & git.exe -C $cloneDir ls-tree -r --name-only $depCommit
if ($LASTEXITCODE -ne 0) { Fail 'git ls-tree failed for the pinned commit' }
$upstream = [System.Collections.Generic.HashSet[string]]::new()
foreach ($p in $upstreamPaths) { [void] $upstream.Add($p.Replace('\', '/')) }
Write-Output ("upstream paths at pin: " + $upstream.Count)

# Vendored paths (the generated, untracked Cargo.lock is not part of the vendored delta).
$vendorBase = (Get-Item -LiteralPath $vendorDir).FullName.Length + 1
$unaccounted = @()
$vendorCount = 0
foreach ($file in (Get-ChildItem -LiteralPath $vendorDir -Recurse -File)) {
  if ($file.FullName -match '\\target\\') { continue }
  if ($file.Name -eq 'Cargo.lock') { continue }
  $rel = $file.FullName.Substring($vendorBase).Replace('\', '/')
  $vendorCount++
  if (-not $upstream.Contains($rel) -and -not $patchPaths.Contains($rel)) { $unaccounted += $rel }
}
Write-Output ("vendored paths checked: $vendorCount")

if ($unaccounted.Count -gt 0) {
  Write-Output 'unaccounted vendored paths (add them to a registered patch before syncing):'
  $unaccounted | Sort-Object | ForEach-Object { Write-Output "  $_" }
  Fail "$($unaccounted.Count) vendored path(s) are neither upstream nor covered by a registered patch"
}
Write-Output 'OK: every vendored WindUI path is upstream or covered by a registered patch'
exit 0
