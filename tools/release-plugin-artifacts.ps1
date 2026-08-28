[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $Version,
  [ValidateSet('stable', 'beta', 'nightly')] [string] $Channel = 'stable',
  [Parameter(Mandatory)] [ValidateRange(1, [UInt64]::MaxValue)] [UInt64] $ReleaseSequence,
  [Parameter(Mandatory)] [string] $StageRoot,
  [Parameter(Mandatory)] [string] $TrustedKeyring,
  [Parameter(Mandatory)] [string] $SigningKey,
  [string] $Signer = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$packageRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'out/package'))
$stage = [IO.Path]::GetFullPath($StageRoot)
$stagePrefix = $packageRoot.TrimEnd('\') + '\'
if (-not $stage.StartsWith($stagePrefix, [StringComparison]::OrdinalIgnoreCase) -or
    -not (Test-Path -LiteralPath $stage -PathType Container)) {
  throw 'Plugin package inputs must come from the verified package stage.'
}
if ([string]::IsNullOrWhiteSpace($Signer)) {
  $Signer = Join-Path $packageRoot 'release-tools/fcitx5-release-pqc-signer.exe'
}
foreach ($path in @($Signer, $TrustedKeyring, $SigningKey)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Required release input is missing: $path" }
}
if ((Get-Item -LiteralPath $SigningKey).Length -ne 4032) {
  throw 'PQC signing secret must be an externally provisioned 4032-byte ML-DSA-65 key.'
}

function Invoke-Checked([string] $Executable, [string[]] $Arguments) {
  & $Executable @Arguments
  if ($LASTEXITCODE -ne 0) { throw "Command failed ($LASTEXITCODE): $Executable" }
}

function Get-OfficialKey($Keyring, [string] $Description) {
  if ($Keyring.format_version -ne 2) { throw "$Description must use trusted-key format v2." }
  $matching = @($Keyring.keys | Where-Object {
    $_.key_id -eq 'official-2026-mldsa65' -and $_.algorithm -eq 'mldsa65' -and $_.status -eq 'trusted'
  })
  if ($matching.Count -ne 1 -or [string]::IsNullOrWhiteSpace($matching[0].public_key_base64)) {
    throw "$Description must contain exactly one trusted official-2026-mldsa65 key."
  }
  return $matching[0]
}

function Get-Blake3([string] $Path) {
  $digest = (& $Signer --blake3 $Path).Trim()
  if ($LASTEXITCODE -ne 0 -or $digest -notmatch '^[0-9a-f]{64}$') {
    throw "Could not calculate BLAKE3 for $Path"
  }
  return $digest
}

$templateKeyring = Get-Content -LiteralPath (Join-Path $repoRoot 'security/trusted-keys.template.json') -Raw |
  ConvertFrom-Json
$protectedKeyring = Get-Content -LiteralPath $TrustedKeyring -Raw | ConvertFrom-Json
$templateKey = Get-OfficialKey $templateKeyring 'Product trusted key template'
$protectedKey = Get-OfficialKey $protectedKeyring 'Protected release keyring'
if ($templateKey.public_key_base64 -cne $protectedKey.public_key_base64) {
  throw 'Protected release keyring does not match the product official ML-DSA-65 public key.'
}
foreach ($key in @($templateKey, $protectedKey)) {
  if (@($key.scope) -notcontains 'repository' -or @($key.scope) -notcontains 'package' -or
      @($key.channels) -notcontains $Channel) {
    throw "The official ML-DSA-65 key is not authorized for the $Channel channel."
  }
}

$inventory = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'release-plugin-inventory.json') -Raw |
  ConvertFrom-Json
if ($inventory.format_version -ne 1 -or @($inventory.packages).Count -lt 3) {
  throw 'Release plugin inventory must contain the reviewed ecosystem packages.'
}
foreach ($plugin in @($inventory.packages)) {
  if ($plugin.architecture -ne 'x64' -or $plugin.build.script -ne 'tools/bootstrap-fcitx.ps1' -or
      [string]::IsNullOrWhiteSpace($plugin.source.commit)) {
    throw "Invalid pinned build input for plugin $($plugin.id)."
  }
}

$artifactRoot = Join-Path $packageRoot 'release-artifacts'
$payloadRoot = Join-Path $packageRoot ('plugin-staging-' + [guid]::NewGuid().ToString('N'))
$archiveName = "$($plugin.id)-$Version-$($plugin.architecture).fcpkg"
$archive = Join-Path $artifactRoot $archiveName
$manifestPath = Join-Path $payloadRoot 'manifest.json'
$signaturePath = Join-Path $payloadRoot 'manifest.sig.json'
$manifestRawSignaturePath = Join-Path $payloadRoot 'manifest.sig.raw'
$indexPath = Join-Path $artifactRoot 'index.json'
$indexSignaturePath = Join-Path $artifactRoot 'index.sig.json'
$indexRawSignaturePath = Join-Path $payloadRoot 'index.sig.raw'
$verificationRoot = Join-Path $packageRoot ('plugin-verify-' + [guid]::NewGuid().ToString('N'))
$packageCli = Join-Path $stage 'bin/fcitx5-package.exe'
if (-not (Test-Path -LiteralPath $packageCli -PathType Leaf)) {
  throw "Verified package CLI is missing: $packageCli"
}

try {
  New-Item -ItemType Directory -Force -Path $artifactRoot, $payloadRoot | Out-Null
  $archives = @()
  $indexPackages = @()
  foreach ($plugin in @($inventory.packages)) {
    $pluginPayloadRoot = Join-Path $payloadRoot $plugin.id
    New-Item -ItemType Directory -Force -Path $pluginPayloadRoot | Out-Null
    $manifestFiles = @()
    foreach ($relative in @($plugin.payload)) {
      if ($relative -notmatch '^[a-z0-9][a-z0-9._/-]*$') { throw "Unsafe inventory path: $relative" }
      $source = Join-Path $stage $relative.Replace('/', '\')
      if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Missing staged $($plugin.id) input: $relative"
      }
      $destination = Join-Path (Join-Path $pluginPayloadRoot 'payload') $relative.Replace('/', '\')
      New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
      Copy-Item -LiteralPath $source -Destination $destination -Force
      $manifestFiles += [ordered]@{
        path = $relative
        size = (Get-Item -LiteralPath $destination).Length
        hashes = [ordered]@{
          blake3 = Get-Blake3 $destination
          sha256 = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
        }
      }
    }
    $manifest = [ordered]@{
      format_version = 2
      id = $plugin.id
      version = $Version
      type = $plugin.type
      architecture = $plugin.architecture
      min_os = '6.1-sp1'
      core_api = '1'
      addon_abi = '1'
      dependencies = @()
      license = $plugin.license
      source_commit = $plugin.source.commit
      runtime_abi = '1'
      runtime_build = "$($plugin.source.commit)+$($plugin.build.script)"
      source = [ordered]@{
        repository = $plugin.source.repository
        commit = $plugin.source.commit
        build_script = $plugin.build.script
      }
      data_policy = [ordered]@{
        program = 'versioned'
        user_data = 'durable'
      }
      permissions = @('native-code')
      payload = @($manifestFiles)
      key_id = 'official-2026-mldsa65'
    }
    $pluginManifestPath = Join-Path $pluginPayloadRoot 'manifest.json'
    $pluginSignaturePath = Join-Path $pluginPayloadRoot 'manifest.sig.json'
    $pluginRawSignaturePath = Join-Path $pluginPayloadRoot 'manifest.sig.raw'
    [IO.File]::WriteAllText($pluginManifestPath, (($manifest | ConvertTo-Json -Depth 8) + "`n"),
      [Text.UTF8Encoding]::new($false))
    Invoke-Checked $Signer @('--sign', $pluginManifestPath, $pluginRawSignaturePath,
      $SigningKey, $templateKey.public_key_base64)
    Invoke-Checked $packageCli @('--write-signature-envelope-v2', 'package-manifest',
      'official-2026-mldsa65', 'mldsa65', $pluginRawSignaturePath, $pluginSignaturePath)
    Remove-Item -LiteralPath $pluginRawSignaturePath -Force
    $archiveName = "$($plugin.id)-$Version-$($plugin.architecture).fcpkg"
    $archive = Join-Path $artifactRoot $archiveName
    Remove-Item -LiteralPath $archive -Force -ErrorAction SilentlyContinue
    Compress-Archive -Path (Join-Path $pluginPayloadRoot '*') -DestinationPath $archive -CompressionLevel Optimal
    $archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $assetBase = "https://github.com/0x696c757a696f/fcitx5-windows-next/releases/download/v$Version"
    $generatedAt = [DateTimeOffset]::UtcNow
    $expiresAt = $generatedAt.AddDays(7)
    $targetCanonical = "$($plugin.id)`t$Version`t$ReleaseSequence`t$($plugin.architecture)`t$archiveHash`n"
    $indexPackages += [ordered]@{
      id = $plugin.id
      title = $plugin.title
      summary = $plugin.summary
      version = $Version
      release_sequence = $ReleaseSequence
      type = $plugin.type
      architecture = $plugin.architecture
      download_url = "$assetBase/$archiveName"
      sha256 = $archiveHash
      dependencies = @()
    }
    $archives += [pscustomobject]@{
      Plugin = $plugin
      Archive = $archive
      Manifest = $pluginManifestPath
      Signature = $pluginSignaturePath
    }
  }
  $generatedAt = [DateTimeOffset]::UtcNow
  $expiresAt = $generatedAt.AddDays(7)
  $targetCanonical = ($indexPackages | ForEach-Object {
    "$($_.id)`t$Version`t$ReleaseSequence`t$($_.architecture)`t$($_.sha256)`n"
  }) -join ''
  $targetSha256 = (Get-FileHash -InputStream ([IO.MemoryStream]::new(
    [Text.Encoding]::UTF8.GetBytes($targetCanonical))) -Algorithm SHA256).Hash.ToLowerInvariant()
  $index = [ordered]@{
    format_version = 1
    repository_id = 'fcitx5-windows-next'
    channel = $Channel
    mirror_id = 'official'
    sequence = $ReleaseSequence
    generated_at = $generatedAt.ToString('yyyy-MM-ddTHH:mm:ssZ')
    expires_at = $expiresAt.ToString('yyyy-MM-ddTHH:mm:ssZ')
    key_id = 'official-2026-mldsa65'
    targets = [ordered]@{
      count = $indexPackages.Count
      sha256 = $targetSha256
    }
    packages = @($indexPackages)
  }
  [IO.File]::WriteAllText($indexPath, (($index | ConvertTo-Json -Depth 8 -Compress) + "`n"),
    [Text.UTF8Encoding]::new($false))
  Invoke-Checked $Signer @('--sign', $indexPath, $indexRawSignaturePath,
    $SigningKey, $templateKey.public_key_base64)
  Invoke-Checked $packageCli @('--write-signature-envelope-v2', 'repository-index',
    'official-2026-mldsa65', 'mldsa65', $indexRawSignaturePath, $indexSignaturePath)
  Invoke-Checked $packageCli @('--verify-repository-v2', $indexPath, $indexSignaturePath,
    $TrustedKeyring, $Channel)
   foreach ($archiveInfo in $archives) {
     Invoke-Checked $packageCli @('--install', $archiveInfo.Archive, $verificationRoot,
       "release-$($archiveInfo.Plugin.id)", $TrustedKeyring)
   }
   foreach ($path in @($archives | ForEach-Object { $_.Manifest; $_.Signature }) + @($indexPath, $indexSignaturePath)) {
    if ((Get-Content -LiteralPath $path -Raw -Encoding UTF8) -match '(?i)(private_key|secret_key|seed_base64)') {
      throw "Private key marker found in generated metadata: $path"
    }
  }
   Write-Host "Generated signed plugin packages and repository index: $artifactRoot"
} finally {
  if (Test-Path -LiteralPath $payloadRoot) { Remove-Item -LiteralPath $payloadRoot -Recurse -Force }
  if (Test-Path -LiteralPath $verificationRoot) { Remove-Item -LiteralPath $verificationRoot -Recurse -Force }
}
