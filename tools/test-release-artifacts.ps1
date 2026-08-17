[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $ArtifactDirectory,
  [Parameter(Mandatory)] [string] $ReleaseManifest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$artifacts = [IO.Path]::GetFullPath($ArtifactDirectory)
$manifestPath = [IO.Path]::GetFullPath($ReleaseManifest)
$prefix = $artifacts.TrimEnd('\') + '\'
if (-not $manifestPath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
  throw 'Release manifest must be inside the artifact directory.'
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.format_version -ne 1 -or $manifest.source_commit -notmatch '^[0-9a-f]{40}$') {
  throw 'Release manifest identity is invalid.'
}
foreach ($item in $manifest.artifacts) {
  $path = Join-Path $artifacts ([string]$item.name)
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing artifact: $($item.name)" }
  $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne $item.sha256 -or (Get-Item -LiteralPath $path).Length -ne $item.size) {
    throw "Final artifact hash/size mismatch: $($item.name)"
  }
  if ($path.EndsWith('.exe', [StringComparison]::OrdinalIgnoreCase)) {
    $signature = Get-AuthenticodeSignature -LiteralPath $path
    if ($signature.Status -ne 'Valid') { throw "Invalid Authenticode signature: $path" }
  }
}
$portableEntry = @($manifest.artifacts | Where-Object name -Like '*-portable.zip')
if ($portableEntry.Count -ne 1) { throw 'Exactly one portable archive is required.' }
$smokeRoot = Join-Path $artifacts ('portable-smoke-' + [guid]::NewGuid().ToString('N'))
try {
  Expand-Archive -LiteralPath (Join-Path $artifacts $portableEntry[0].name) `
    -DestinationPath $smokeRoot
  $portableRoot = Join-Path $smokeRoot 'Fcitx5'
  $stageManifest = Get-Content -LiteralPath (Join-Path $portableRoot 'manifest.json') -Raw |
    ConvertFrom-Json
  foreach ($file in $stageManifest.files) {
    $path = Join-Path $portableRoot ([string]$file.path).Replace('/', '\')
    if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne
          $file.sha256) { throw "Portable staged-file verification failed: $($file.path)" }
  }
  & (Join-Path $portableRoot 'bin/fcitx5-config.exe') --self-test
  if ($LASTEXITCODE -ne 0) { throw 'Signed portable Config smoke failed.' }
  & (Join-Path $portableRoot 'bin/fcitx5-control.exe') --schema
  if ($LASTEXITCODE -ne 0) { throw 'Signed portable Control smoke failed.' }
} finally {
  $resolved = [IO.Path]::GetFullPath($smokeRoot)
  if ($resolved.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -and
      (Test-Path -LiteralPath $resolved)) {
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
}
$sbomPath = @($manifest.artifacts | Where-Object name -Like '*.spdx.json')
if ($sbomPath.Count -ne 1) { throw 'Exactly one SPDX SBOM is required.' }
$sbom = Get-Content -LiteralPath (Join-Path $artifacts $sbomPath[0].name) -Raw | ConvertFrom-Json
if ($sbom.spdxVersion -ne 'SPDX-2.3' -or $sbom.packages.Count -lt 2 -or
    $sbom.files.Count -lt 1) { throw 'SPDX SBOM does not describe the staged product.' }
$signaturePath = $manifestPath + '.p7s'
if (-not (Test-Path -LiteralPath $signaturePath -PathType Leaf)) {
  throw 'Detached release-manifest signature is missing.'
}
Add-Type -AssemblyName System.Security.Cryptography.Pkcs
$cms = [Security.Cryptography.Pkcs.SignedCms]::new(
  [Security.Cryptography.Pkcs.ContentInfo]::new([IO.File]::ReadAllBytes($manifestPath)), $true)
$cms.Decode([IO.File]::ReadAllBytes($signaturePath))
$cms.CheckSignature($true)
Write-Host 'Final signed portable, hash, SBOM, and detached-manifest smoke passed.'
