[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))

$inventoryPath = Join-Path $PSScriptRoot 'release-plugin-inventory.json'
$inventory = Get-Content -LiteralPath $inventoryPath -Raw | ConvertFrom-Json
if ($inventory.format_version -ne 1 -or @($inventory.packages).Count -ne 3) {
  throw 'Release plugin inventory shape is invalid.'
}
if ((@($inventory.packages.id) -join ',') -cne 'fcitx5-rime,fcitx5-lua,fcitx5-unikey') {
  throw 'Release plugin inventory does not preserve the reviewed package order.'
}
$expectedCommits = @{
  'fcitx5-rime' = '4e996319edea790495edc2c91893e9af4c4e6d6a'
  'fcitx5-lua' = '05db9ee519d448a64ccbe216044e8e0342e8c536'
  'fcitx5-unikey' = '53f82a1e01dc0484f46dc8ed419d586cebd2f114'
}
foreach ($plugin in @($inventory.packages)) {
  if ($plugin.architecture -ne 'x64' -or $plugin.source.commit -ne $expectedCommits[$plugin.id] -or
      $plugin.build.script -ne 'tools/bootstrap-fcitx.ps1' -or @($plugin.payload).Count -eq 0) {
    throw "Release plugin inventory has an invalid pinned build contract: $($plugin.id)."
  }
}
$lua = $inventory.packages | Where-Object id -eq 'fcitx5-lua'
$unikey = $inventory.packages | Where-Object id -eq 'fcitx5-unikey'
if (@($lua.payload) -notcontains 'lib/fcitx5/libluaaddonloader.dll' -or
    @($lua.payload) -notcontains 'share/fcitx5/addon/luaaddonloader.conf' -or
    @($unikey.payload) -notcontains 'lib/fcitx5/unikey.dll' -or
    @($unikey.payload) -notcontains 'share/fcitx5/addon/unikey.conf') {
  throw 'Lua or Unikey inventory payload does not match upstream install outputs.'
}
$bootstrap = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'bootstrap-fcitx.ps1') -Raw
foreach ($plugin in @($inventory.packages)) {
  if ($bootstrap.IndexOf("@{ Name = '$($plugin.id)';", [StringComparison]::Ordinal) -lt 0 -or
      $bootstrap.IndexOf($plugin.source.commit, [StringComparison]::Ordinal) -lt 0) {
    throw "$($plugin.id) release inventory does not match the pinned bootstrap source."
  }
}
if ($bootstrap.IndexOf("cmake -S '`$msysSources/fcitx5-lua'", [StringComparison]::Ordinal) -lt 0 -or
    $bootstrap.IndexOf("cmake -S '`$msysSources/fcitx5-unikey'", [StringComparison]::Ordinal) -lt 0) {
  throw 'Lua and Unikey must use the standard Fcitx CMake build-farm path.'
}
$generatorPath = Join-Path $PSScriptRoot 'release-plugin-artifacts.ps1'
$tokens = $null
$parseErrors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile(
  $generatorPath, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -ne 0) {
  throw "Release plugin generator has PowerShell syntax errors: $($parseErrors[0])"
}
$generator = Get-Content -LiteralPath $generatorPath -Raw
foreach ($required in @('4032-byte ML-DSA-65 key', 'official-2026-mldsa65',
    'releases/download/v$Version', 'manifest.sig.json', 'index.sig.json', 'release-tools',
    '--write-signature-envelope-v2', 'foreach ($plugin in @($inventory.packages))',
    '--verify-repository-v2', '--install')) {
  if ($generator.IndexOf($required, [StringComparison]::Ordinal) -lt 0) {
    throw "Release plugin generator is missing required production contract: $required"
  }
}
$releaseSigner = Get-Content -LiteralPath (Join-Path $repoRoot 'rust/release-pqc-signer/src/main.rs') -Raw
foreach ($required in @('--sign INPUT SIGNATURE_RAW SECRET_KEY', 'signature=mldsa65')) {
  if ($releaseSigner.IndexOf($required, [StringComparison]::Ordinal) -lt 0) {
    throw "Release signer is missing raw ML-DSA signature contract: $required"
  }
}
foreach ($forbidden in @('"format_version"', '"signed_object"', '"signature_base64"')) {
  if ($releaseSigner.IndexOf($forbidden, [StringComparison]::Ordinal) -ge 0) {
    throw "Release signer must not own signature-envelope JSON: $forbidden"
  }
}
$releaseSignerCxx = Join-Path $PSScriptRoot 'release_pqc_signer.cpp'
if (Test-Path -LiteralPath $releaseSignerCxx -PathType Leaf) {
  throw 'Release PQC signer must remain Rust-owned; C++ signer source is forbidden.'
}
$packageCli = Get-Content -LiteralPath (Join-Path $repoRoot 'rust/package-core/src/main.rs') -Raw
if ($packageCli.IndexOf('--write-signature-envelope-v2', [StringComparison]::Ordinal) -lt 0 -or
    $packageCli.IndexOf('format_signature_envelope_v2', [StringComparison]::Ordinal) -lt 0) {
  throw 'Rust Package Core CLI must own v2 signature-envelope formatting.'
}
$workflow = Get-Content -LiteralPath (Join-Path $repoRoot '.github/workflows/release.yml') -Raw
foreach ($required in @('PQC_SIGNING_SECRET_KEY_BASE64', 'RUNNER_TEMP',
    'FCITX_RELEASE_PLUGIN_SEQUENCE', 'Enforce monotonic official plugin sequence',
    'gh release download', '--verify-repository-v2')) {
  if ($workflow.IndexOf($required, [StringComparison]::Ordinal) -lt 0) {
    throw "Release workflow is missing protected plugin signing contract: $required"
  }
}
$tracked = & git -C $repoRoot ls-files --cached
if ($LASTEXITCODE -ne 0) { throw 'Could not enumerate tracked files.' }
foreach ($path in $tracked) {
  if ($path -match '(?i)(\.key$|\.pem$|\.pfx$|secret.*key)') {
    throw "Private key-like file is tracked: $path"
  }
}
$artifactRoot = Join-Path $repoRoot 'out/package/release-artifacts'
if (Test-Path -LiteralPath $artifactRoot -PathType Container) {
  foreach ($path in Get-ChildItem -LiteralPath $artifactRoot -File -Recurse) {
    if ($path.Name -match '(?i)(\.key$|\.pem$|\.pfx$|secret.*key)') {
      throw "Private key-like release artifact found: $($path.FullName)"
    }
    if ($path.Extension -in @('.json', '.txt', '.sig')) {
      $content = Get-Content -LiteralPath $path.FullName -Raw -Encoding UTF8
      if ($content -match '(?i)(private_key|secret_key|seed_base64)') {
        throw "Private key marker found in release metadata: $($path.FullName)"
      }
    }
  }
}
Write-Host 'Release plugin inventory, protected-input, and no-private-key source smoke passed.'
