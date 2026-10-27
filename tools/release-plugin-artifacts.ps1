[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $Version,
  [ValidateSet('stable', 'beta', 'nightly')] [string] $Channel = 'stable',
  [Parameter(Mandatory)] [ValidateRange(1, [UInt64]::MaxValue)] [UInt64] $ReleaseSequence,
  [Parameter(Mandatory)] [string] $StageRoot,
  [Parameter(Mandatory)] [string] $TrustedKeyring,
  [Parameter(Mandatory)] [string] $SigningKey,
  [Parameter(Mandatory)] [string] $Ed25519SigningKey,
  [string] $Signer = '',
  [string] $ForkPayloadRoot = ''
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
foreach ($path in @($Signer, $TrustedKeyring, $SigningKey, $Ed25519SigningKey)) {
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

function Invoke-MetadataSigning([string] $Path, [string] $SignedObject, [string] $SignaturePath) {
  & (Join-Path $PSScriptRoot 'sign-release-metadata.ps1') -Path $Path -SignedObject $SignedObject `
    -SignaturePath $SignaturePath -TrustedKeyring $TrustedKeyring -SigningKey $SigningKey `
    -Ed25519SigningKey $Ed25519SigningKey -Signer $Signer -PackageCli $packageCli
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

# Each package source names its payload root, the payload paths relative to it,
# and its provenance. Stage packages come from this repository's own build; fork
# packages come from the pinned fcitx5-plugins release that
# tools/prepare-fork-plugins.ps1 verified and laid out.
$packageSources = @()
foreach ($plugin in @($inventory.packages)) {
  $packageSources += [pscustomobject]@{
    Plugin = $plugin
    Root = $stage
    Payload = @($plugin.payload)
    PathPattern = '^[a-z0-9][a-z0-9._/-]*$'
    Repository = $plugin.source.repository
    Commit = $plugin.source.commit
    BuildScript = $plugin.build.script
    MinOs = '6.1-sp1'
    Permissions = @('native-code')
  }
}
if ([string]::IsNullOrWhiteSpace($ForkPayloadRoot)) {
  $ForkPayloadRoot = Join-Path $packageRoot 'fork-plugins'
}
$forkInventory = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'fork-plugin-inventory.json') -Raw |
  ConvertFrom-Json
foreach ($plugin in @($forkInventory.packages)) {
  if (@($inventory.packages.id) -contains $plugin.id) {
    throw "Fork plugin $($plugin.id) duplicates a package this repository builds."
  }
  $forkRoot = Join-Path $ForkPayloadRoot $plugin.id
  if (-not (Test-Path -LiteralPath $forkRoot -PathType Container)) {
    throw "Prepared fork plugin payload is missing: $forkRoot. Run tools/prepare-fork-plugins.ps1 first."
  }
  $packageSources += [pscustomobject]@{
    Plugin = $plugin
    Root = $forkRoot
    Payload = @(Get-ChildItem -LiteralPath $forkRoot -File -Recurse | Sort-Object FullName |
      ForEach-Object { [IO.Path]::GetRelativePath($forkRoot, $_.FullName).Replace('\', '/') })
    PathPattern = '^[A-Za-z0-9][A-Za-z0-9._/-]*$'
    Repository = $forkInventory.repository
    Commit = $forkInventory.commit
    BuildScript = $forkInventory.build_script
    MinOs = '10.0'
    Permissions = @($(if ($plugin.type -eq 'addon') { 'native-code' } else { 'input-data' }))
  }
}

$artifactRoot = Join-Path $packageRoot 'release-artifacts'
$payloadRoot = Join-Path $packageRoot ('plugin-staging-' + [guid]::NewGuid().ToString('N'))
$indexPath = Join-Path $artifactRoot 'index.json'
$indexSignaturePath = Join-Path $artifactRoot 'index.sig.json'
$verificationRoot = Join-Path $packageRoot ('plugin-verify-' + [guid]::NewGuid().ToString('N'))
$packageCli = Join-Path $stage 'bin/fcitx5-package.exe'
if (-not (Test-Path -LiteralPath $packageCli -PathType Leaf)) {
  throw "Verified package CLI is missing: $packageCli"
}

try {
  New-Item -ItemType Directory -Force -Path $artifactRoot, $payloadRoot | Out-Null
  $archives = @()
  $indexPackages = @()
  # Stage packages: foreach ($plugin in @($inventory.packages)); fork packages follow.
  foreach ($packageSource in $packageSources) {
    $plugin = $packageSource.Plugin
    $pluginPayloadRoot = Join-Path $payloadRoot $plugin.id
    New-Item -ItemType Directory -Force -Path $pluginPayloadRoot | Out-Null
    $manifestFiles = @()
    foreach ($relative in @($packageSource.Payload)) {
      if ($relative -notmatch $packageSource.PathPattern -or $relative.Contains('..')) {
        throw "Unsafe inventory path: $relative"
      }
      $source = Join-Path $packageSource.Root $relative.Replace('/', '\')
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
      min_os = $packageSource.MinOs
      core_api = '1'
      addon_abi = '1'
      dependencies = @()
      license = $plugin.license
      source_commit = $packageSource.Commit
      runtime_abi = '1'
      runtime_build = "$($packageSource.Commit)+$($packageSource.BuildScript)"
      source = [ordered]@{
        repository = $packageSource.Repository
        commit = $packageSource.Commit
        build_script = $packageSource.BuildScript
      }
      data_policy = [ordered]@{
        program = 'versioned'
        user_data = 'durable'
      }
      permissions = @($packageSource.Permissions)
      payload = @($manifestFiles)
      key_id = 'official-2026-mldsa65'
    }
    $pluginManifestPath = Join-Path $pluginPayloadRoot 'manifest.json'
    $pluginSignaturePath = Join-Path $pluginPayloadRoot 'manifest.sig.json'
    [IO.File]::WriteAllText($pluginManifestPath, (($manifest | ConvertTo-Json -Depth 8) + "`n"),
      [Text.UTF8Encoding]::new($false))
    Invoke-MetadataSigning $pluginManifestPath 'package-manifest' $pluginSignaturePath
    $archiveName = "$($plugin.id)-$Version-$($plugin.architecture).fcpkg"
    $archive = Join-Path $artifactRoot $archiveName
    Remove-Item -LiteralPath $archive -Force -ErrorAction SilentlyContinue
    Compress-Archive -Path (Join-Path $pluginPayloadRoot '*') -DestinationPath $archive -CompressionLevel Optimal
    $archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $assetBase = "https://github.com/0x696c757a696f/fcitx5-windows-next/releases/download/v$Version"
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
  Invoke-MetadataSigning $indexPath 'repository-index' $indexSignaturePath
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
