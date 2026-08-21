[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$manifestPath = Join-Path $repoRoot 'third_party/dependencies.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json

$cmakeFiles = @((Join-Path $repoRoot 'CMakeLists.txt'))
$cmakeFiles += Get-ChildItem -LiteralPath (Join-Path $repoRoot 'cmake') -File -Recurse |
  Select-Object -ExpandProperty FullName
$untrackedDependencyDirectives = [System.Collections.Generic.List[string]]::new()
foreach ($file in $cmakeFiles) {
  $lineNumber = 0
  foreach ($line in [System.IO.File]::ReadLines($file)) {
    $lineNumber++
    if ($line -match '\b(FetchContent_Declare|ExternalProject_Add|CPMAddPackage)\s*\(') {
      $untrackedDependencyDirectives.Add("${file}:${lineNumber}")
    }
  }
}

if ($untrackedDependencyDirectives.Count -gt 0) {
  throw "Network/build dependency directives require a pinned inventory record:`n$($untrackedDependencyDirectives -join "`n")"
}

$cargoLockPath = Join-Path $repoRoot 'Cargo.lock'
if (Test-Path -LiteralPath $cargoLockPath -PathType Leaf) {
  $cargoLock = Get-Content -LiteralPath $cargoLockPath -Raw
  $allowedCargoPackages = @(
    'arrayref',
    'arrayvec',
    'blake3',
    'cc',
    'cfg-if',
    'constant_time_eq',
    'find-msvc-tools',
    'shlex'
  )
  $cargoPackageMatches = [regex]::Matches(
    $cargoLock,
    '(?ms)\[\[package\]\]\s+name = "([^"]+)".*?(?=\n\[\[package\]\]|\z)'
  )
  $untrackedCargoPackages = [System.Collections.Generic.List[string]]::new()
  $blockedCargoPackages = [System.Collections.Generic.List[string]]::new()
  foreach ($match in $cargoPackageMatches) {
    $name = $match.Groups[1].Value
    $packageBlock = $match.Value
    $version = ''
    if ($packageBlock -match '(?m)^\s*version\s*=\s*"([^"]+)"\s*$') {
      $version = $Matches[1]
    }
    if ($name -eq 'arrayref' -and $version -eq '0.3.10') {
      $blockedCargoPackages.Add("$name $version")
    }
    $isRegistryCrate = $packageBlock -match '(?m)^\s*source\s*=\s*"registry\+https://github\.com/rust-lang/crates\.io-index"\s*$'
    if ($isRegistryCrate -and $allowedCargoPackages -notcontains $name) {
      $untrackedCargoPackages.Add($name)
    }
  }
  if ($blockedCargoPackages.Count -gt 0) {
    throw "Cargo.lock contains blocked crate versions from a RustSec/crates.io incident:`n$($blockedCargoPackages -join "`n")"
  }
  if ($untrackedCargoPackages.Count -gt 0) {
    throw "Cargo.lock contains untracked third-party crate sources; add Cargo dependency inventory/SBOM/license review:`n$($untrackedCargoPackages -join "`n")"
  }
}

if ($manifest.packages.Count -ne 0) {
  Write-Host "SCA baseline: $($manifest.packages.Count) declared dependencies require external advisory review."
} else {
  Write-Host 'SCA baseline passed: no third-party build/runtime dependencies are present.'
}
