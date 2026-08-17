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

if ($manifest.packages.Count -ne 0) {
  Write-Host "SCA baseline: $($manifest.packages.Count) declared dependencies require external advisory review."
} else {
  Write-Host 'SCA baseline passed: no third-party build/runtime dependencies are present.'
}
