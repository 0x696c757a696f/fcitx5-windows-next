[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $Path,
  [Parameter(Mandatory)] [ValidateSet('repository-index', 'package-manifest', 'release-manifest')]
  [string] $SignedObject,
  [Parameter(Mandatory)] [string] $SignaturePath,
  [Parameter(Mandatory)] [string] $TrustedKeyring,
  [Parameter(Mandatory)] [string] $SigningKey,
  # Empty only when re-signing metadata first published with ML-DSA-65 alone,
  # whose clients reject envelopes that name an algorithm they do not know.
  [string] $Ed25519SigningKey = '',
  [Parameter(Mandatory)] [string] $Signer,
  [Parameter(Mandatory)] [string] $PackageCli
)

# Official metadata carries two signatures over the same bytes: ML-DSA-65 and
# Ed25519. Package Core accepts it only when both verify, so a break in either
# algorithm alone cannot forge an update.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$dual = -not [string]::IsNullOrWhiteSpace($Ed25519SigningKey)
$inputs = @($Path, $TrustedKeyring, $SigningKey, $Signer, $PackageCli)
if ($dual) { $inputs += $Ed25519SigningKey }
foreach ($required in $inputs) {
  if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Required signing input is missing: $required" }
}
if ((Get-Item -LiteralPath $SigningKey).Length -ne 4032) {
  throw 'PQC signing secret must be an externally provisioned 4032-byte ML-DSA-65 key.'
}
if ($dual -and (Get-Item -LiteralPath $Ed25519SigningKey).Length -ne 32) {
  throw 'Ed25519 signing secret must be an externally provisioned 32-byte seed.'
}

function Invoke-Checked([string] $Executable, [string[]] $Arguments) {
  & $Executable @Arguments
  if ($LASTEXITCODE -ne 0) { throw "Command failed ($LASTEXITCODE): $Executable" }
}

function Get-OfficialPublicKey([string] $KeyId, [string] $Algorithm) {
  $values = foreach ($keyringPath in @($TrustedKeyring,
      (Join-Path $repoRoot 'security/trusted-keys.template.json'))) {
    $keyring = Get-Content -LiteralPath $keyringPath -Raw | ConvertFrom-Json
    $matching = @($keyring.keys | Where-Object {
      $_.key_id -eq $KeyId -and $_.algorithm -eq $Algorithm -and $_.status -eq 'trusted'
    })
    if ($keyring.format_version -ne 2 -or $matching.Count -ne 1 -or
        [string]::IsNullOrWhiteSpace($matching[0].public_key_base64)) {
      throw "Keyrings must contain exactly one trusted $KeyId key."
    }
    $matching[0].public_key_base64
  }
  if ($values[0] -cne $values[1]) {
    throw "Protected keyring does not match the product $KeyId public key."
  }
  return $values[0]
}

$mldsaPublic = Get-OfficialPublicKey 'official-2026-mldsa65' 'mldsa65'
$mldsaRaw = "$SignaturePath.mldsa65.raw"
$ed25519Raw = "$SignaturePath.ed25519.raw"
try {
  Invoke-Checked $Signer @('--sign', $Path, $mldsaRaw, $SigningKey, $mldsaPublic)
  $entries = @('official-2026-mldsa65', 'mldsa65', $mldsaRaw)
  if ($dual) {
    $ed25519Public = Get-OfficialPublicKey 'official-2026-ed25519' 'ed25519'
    Invoke-Checked $Signer @('--sign-ed25519', $Path, $ed25519Raw, $Ed25519SigningKey, $ed25519Public)
    $entries += @('official-2026-ed25519', 'ed25519', $ed25519Raw)
  }
  Invoke-Checked $PackageCli (@('--write-signature-envelope-v2', $SignedObject) + $entries + @($SignaturePath))
} finally {
  Remove-Item -LiteralPath $mldsaRaw, $ed25519Raw -Force -ErrorAction SilentlyContinue
}
