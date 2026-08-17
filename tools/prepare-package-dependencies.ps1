[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$manifestPath = Join-Path $PSScriptRoot 'package-dependencies.json'
$toolchainRoot = Join-Path $repoRoot 'out/toolchains'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json

if ($manifest.schemaVersion -ne 1) {
  throw 'Unsupported package dependency manifest schema.'
}

function Get-VerifiedDownload($Package) {
  $target = Join-Path $toolchainRoot ([string]$Package.output)
  $targetDirectory = Split-Path -Parent $target
  New-Item -ItemType Directory -Force -Path $targetDirectory | Out-Null

  $valid = Test-Path -LiteralPath $target -PathType Leaf
  if ($valid) {
    $valid = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash -eq $Package.sha256
  }
  if (-not $valid) {
    $partial = "$target.partial"
    Remove-Item -LiteralPath $partial -Force -ErrorAction SilentlyContinue
    Invoke-WebRequest -UseBasicParsing -Uri $Package.url -OutFile $partial
    $actual = (Get-FileHash -LiteralPath $partial -Algorithm SHA256).Hash
    if ($actual -ne $Package.sha256) {
      Remove-Item -LiteralPath $partial -Force
      throw "Hash mismatch for $($Package.name): expected $($Package.sha256), got $actual."
    }
    Move-Item -LiteralPath $partial -Destination $target -Force
  }
  return $target
}

foreach ($package in $manifest.packages) {
  $download = Get-VerifiedDownload $package
  if ($package.PSObject.Properties.Name -contains 'extract') {
    $destination = Join-Path $toolchainRoot ([string]$package.extract)
    $stamp = Join-Path $destination '.source-sha256'
    $needsExtract = -not (Test-Path -LiteralPath $stamp -PathType Leaf) -or
      ((Get-Content -LiteralPath $stamp -Raw).Trim() -ne $package.sha256)
    if ($needsExtract) {
      if (Test-Path -LiteralPath $destination) {
        Remove-Item -LiteralPath $destination -Recurse -Force
      }
      New-Item -ItemType Directory -Force -Path $destination | Out-Null
      Expand-Archive -LiteralPath $download -DestinationPath $destination -Force
      Set-Content -LiteralPath $stamp -Value $package.sha256 -Encoding ascii -NoNewline
    }
  }
}

Write-Host 'Pinned package dependencies are present and hash-verified.'
