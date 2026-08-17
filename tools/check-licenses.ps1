[CmdletBinding()]
param([switch] $SelfTest)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Test-PackageRecord($Package) {
  if (-not $Package.name -or -not $Package.version -or -not $Package.source -or
      -not $Package.license) {
    return $false
  }
  return $Package.license -notin @('NOASSERTION', 'UNKNOWN', 'NONE')
}

if ($SelfTest) {
  $goodCase = [pscustomobject]@{
    name = 'example'
    version = '1.0.0'
    source = 'https://example.invalid/source'
    license = 'MIT'
  }
  $badCase = [pscustomobject]@{
    name = 'example'
    version = '1.0.0'
    source = 'https://example.invalid/source'
    license = 'UNKNOWN'
  }
  if (-not (Test-PackageRecord $goodCase) -or (Test-PackageRecord $badCase)) {
    throw 'License checker paired self-test failed.'
  }
  Write-Host 'License checker paired self-test passed.'
  return
}

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$manifestPath = Join-Path $repoRoot 'third_party/dependencies.json'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
  throw 'Missing third_party/dependencies.json.'
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.schemaVersion -ne 1 -or $null -eq $manifest.packages) {
  throw 'Unsupported dependency manifest schema.'
}

$names = [System.Collections.Generic.HashSet[string]]::new(
  [System.StringComparer]::OrdinalIgnoreCase)
foreach ($package in $manifest.packages) {
  if (-not (Test-PackageRecord $package)) {
    throw "Incomplete or unknown license record for dependency '$($package.name)'."
  }
  if (-not $names.Add([string]$package.name)) {
    throw "Duplicate dependency record '$($package.name)'."
  }
}

$thirdPartyRoot = Join-Path $repoRoot 'third_party'
$vendoredDirectories = Get-ChildItem -LiteralPath $thirdPartyRoot -Directory
foreach ($directory in $vendoredDirectories) {
  if (-not $names.Contains($directory.Name)) {
    throw "Vendored directory '$($directory.Name)' has no dependency/license record."
  }
}

Write-Host "License inventory passed ($($manifest.packages.Count) dependencies)."
