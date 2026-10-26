[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $IndexPath,
  [Parameter(Mandatory)] [string] $SignaturePath,
  [Parameter(Mandatory)] [string] $TrustedKeyring,
  [Parameter(Mandatory)] [string] $SigningKey,
  [Parameter(Mandatory)] [string] $Signer,
  [Parameter(Mandatory)] [string] $PackageCli,
  [ValidateRange(1, 7)] [int] $ValidDays = 7
)

# Clients reject a repository index after expires_at, and a release's index is
# only generated once. Re-sign a published index with the next sequence and a
# fresh validity window; its packages and targets stay exactly as published.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
foreach ($path in @($IndexPath, $TrustedKeyring, $SigningKey, $Signer, $PackageCli)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Required input is missing: $path" }
}
if ((Get-Item -LiteralPath $SigningKey).Length -ne 4032) {
  throw 'PQC signing secret must be an externally provisioned 4032-byte ML-DSA-65 key.'
}

function Invoke-Checked([string] $Executable, [string[]] $Arguments) {
  & $Executable @Arguments
  if ($LASTEXITCODE -ne 0) { throw "Command failed ($LASTEXITCODE): $Executable" }
}

$keyring = Get-Content -LiteralPath $TrustedKeyring -Raw | ConvertFrom-Json
$template = Get-Content -LiteralPath (Join-Path $repoRoot 'security/trusted-keys.template.json') -Raw |
  ConvertFrom-Json
$keys = foreach ($candidate in @($keyring, $template)) {
  $matching = @($candidate.keys | Where-Object {
    $_.key_id -eq 'official-2026-mldsa65' -and $_.algorithm -eq 'mldsa65' -and $_.status -eq 'trusted'
  })
  if ($matching.Count -ne 1) { throw 'Keyrings must contain exactly one trusted official-2026-mldsa65 key.' }
  $matching[0]
}
if ($keys[0].public_key_base64 -cne $keys[1].public_key_base64) {
  throw 'Protected keyring does not match the product official ML-DSA-65 public key.'
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
  $rawSignature = Join-Path $work 'index.sig.raw'
  $newSignature = Join-Path $work 'index.sig.json'
  [IO.File]::WriteAllText($newIndex, (($index | ConvertTo-Json -Depth 8 -Compress) + "`n"),
    [Text.UTF8Encoding]::new($false))
  Invoke-Checked $Signer @('--sign', $newIndex, $rawSignature, $SigningKey, $keys[0].public_key_base64)
  Invoke-Checked $PackageCli @('--write-signature-envelope-v2', 'repository-index',
    'official-2026-mldsa65', 'mldsa65', $rawSignature, $newSignature)
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
