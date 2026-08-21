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

function Stop-PortableSmokeProcesses {
  param([Parameter(Mandatory)] [string] $Root)
  $resolvedRoot = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
  $allowedNames = @(
    'fcitx5-launcher.exe',
    'fcitx5-ui.exe',
    'fcitx5-engine.exe',
    'fcitx5-config.exe',
    'fcitx5-control.exe'
  )
  Get-CimInstance Win32_Process |
    Where-Object {
      $_.ExecutablePath -and
      $allowedNames -contains [IO.Path]::GetFileName($_.ExecutablePath) -and
      [IO.Path]::GetFullPath($_.ExecutablePath).StartsWith(
        $resolvedRoot, [StringComparison]::OrdinalIgnoreCase)
    } |
    ForEach-Object {
      try {
        $process = Get-Process -Id $_.ProcessId -ErrorAction Stop
        Stop-Process -InputObject $process -Force -ErrorAction Stop
      } catch {
        Write-Warning "Failed to stop release portable smoke process $($_.ProcessId): $($_.Exception.Message)"
      }
    }
}

function Test-ArtifactDirectoryWritable {
  $probe = Join-Path $artifacts ('.release-smoke-write-probe-' + [guid]::NewGuid().ToString('N'))
  [IO.File]::WriteAllText($probe, "probe`n", [Text.UTF8Encoding]::new($false))
  Remove-Item -LiteralPath $probe -Force
}

function Test-NoPrivateKeyMaterial {
  param([Parameter(Mandatory)] [string] $Root)
  $securityRoot = Join-Path $Root 'security'
  if (-not (Test-Path -LiteralPath $securityRoot -PathType Container)) {
    throw 'Portable package is missing security trusted-key directory.'
  }
  $keyringPath = Join-Path $securityRoot 'trusted-keys.json'
  if (-not (Test-Path -LiteralPath $keyringPath -PathType Leaf)) {
    throw 'Portable package is missing trusted-keys.json.'
  }
  $keyring = Get-Content -LiteralPath $keyringPath -Raw | ConvertFrom-Json
  if ($keyring.format_version -ne 2) { throw 'Release package must carry v2 public trusted keys.' }
  $rawSecurityJson = Get-ChildItem -LiteralPath $securityRoot -File -Recurse |
    Where-Object Extension -eq '.json' |
    ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw -Encoding UTF8 }
  foreach ($marker in @('private_key', 'secret_key', 'seed_base64',
                        'private_key_base64', 'secret_key_base64')) {
    foreach ($content in $rawSecurityJson) {
      if ($content.IndexOf($marker, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
        throw "Private signing material marker found in release security metadata: $marker"
      }
    }
  }
}

try {
  Expand-Archive -LiteralPath (Join-Path $artifacts $portableEntry[0].name) `
    -DestinationPath $smokeRoot
  $portableRoot = Join-Path $smokeRoot 'Fcitx5'
  Test-NoPrivateKeyMaterial -Root $portableRoot
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
  & (Join-Path $portableRoot 'bin/fcitx5-config.exe') --ui-interaction-test
  if ($LASTEXITCODE -ne 0) { throw 'Signed portable Config interaction sweep failed.' }
  & (Join-Path $portableRoot 'bin/fcitx5-control.exe') --schema
  if ($LASTEXITCODE -ne 0) { throw 'Signed portable Control smoke failed.' }
} finally {
  Stop-PortableSmokeProcesses -Root $smokeRoot
  $resolved = [IO.Path]::GetFullPath($smokeRoot)
  if ($resolved.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -and
      (Test-Path -LiteralPath $resolved)) {
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
}
Test-ArtifactDirectoryWritable
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
