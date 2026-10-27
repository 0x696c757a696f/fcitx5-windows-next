[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $IndexPath,
  [Parameter(Mandatory)] [string] $SignaturePath,
  [Parameter(Mandatory)] [string] $TrustedKeyring,
  [Parameter(Mandatory)] [string] $SigningKey,
  [string] $Ed25519SigningKey = '',
  [Parameter(Mandatory)] [string] $Signer,
  [Parameter(Mandatory)] [string] $PackageCli,
  [ValidateRange(1, 7)] [int] $ValidDays = 7
)

# Clients reject a repository index after expires_at, and a release's index is
# only generated once. Re-sign a published index with the next sequence and a
# fresh validity window; its packages and targets stay exactly as published.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

foreach ($path in @($IndexPath, $SignaturePath, $TrustedKeyring, $SigningKey, $Signer, $PackageCli)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Required input is missing: $path" }
}

function Invoke-Checked([string] $Executable, [string[]] $Arguments) {
  & $Executable @Arguments
  if ($LASTEXITCODE -ne 0) { throw "Command failed ($LASTEXITCODE): $Executable" }
}

# Keep each release's signature set: clients that shipped before Ed25519 reject
# an envelope naming it, and clients that require it reject one without it.
$published = Get-Content -LiteralPath $SignaturePath -Raw | ConvertFrom-Json
$dual = @($published.signatures | Where-Object { $_.algorithm -eq 'ed25519' }).Count -gt 0
if ($dual -and [string]::IsNullOrWhiteSpace($Ed25519SigningKey)) {
  throw 'This index was published with an Ed25519 signature; the Ed25519 signing secret is required.'
}

$index = Get-Content -LiteralPath $IndexPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($index.format_version -ne 1 -or $index.repository_id -ne 'fcitx5-windows-next' -or
    $index.key_id -ne 'official-2026-mldsa65') {
  throw 'Only official fcitx5-windows-next repository indexes can be refreshed.'
}
$sequence = [UInt64] $index.sequence
if ($sequence -ge [UInt64]::MaxValue) { throw 'Repository sequence cannot advance.' }
$generatedAt = [DateTimeOffset]::UtcNow
$index.sequence = $sequence + 1
$index.generated_at = $generatedAt.ToString('yyyy-MM-ddTHH:mm:ssZ')
$index.expires_at = $generatedAt.AddDays($ValidDays).ToString('yyyy-MM-ddTHH:mm:ssZ')

$work = Join-Path ([IO.Path]::GetTempPath()) ('fcitx5-index-refresh-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
try {
  $newIndex = Join-Path $work 'index.json'
  $newSignature = Join-Path $work 'index.sig.json'
  [IO.File]::WriteAllText($newIndex, (($index | ConvertTo-Json -Depth 8 -Compress) + "`n"),
    [Text.UTF8Encoding]::new($false))
  & (Join-Path $PSScriptRoot 'sign-release-metadata.ps1') -Path $newIndex -SignedObject 'repository-index' `
    -SignaturePath $newSignature -TrustedKeyring $TrustedKeyring -SigningKey $SigningKey `
    -Ed25519SigningKey $(if ($dual) { $Ed25519SigningKey } else { '' }) -Signer $Signer -PackageCli $PackageCli
  Invoke-Checked $PackageCli @('--verify-repository-v2', $newIndex, $newSignature,
    $TrustedKeyring, $index.channel)
  if ((Get-Content -LiteralPath $newIndex -Raw) -match '(?i)(private_key|secret_key|seed_base64)') {
    throw 'Private key marker found in the refreshed index.'
  }
  Copy-Item -LiteralPath $newIndex -Destination $IndexPath -Force
  Copy-Item -LiteralPath $newSignature -Destination $SignaturePath -Force
  Write-Host "Refreshed $($index.channel) index to sequence $($index.sequence), expires $($index.expires_at)"
} finally {
  Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
