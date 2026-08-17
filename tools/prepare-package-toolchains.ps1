[CmdletBinding()]
param([switch] $VerifyOnly)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$toolchainRoot = Join-Path $repoRoot 'out/toolchains'
$lock = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'package-toolchains.json') `
  -Raw -Encoding UTF8 | ConvertFrom-Json
if ($lock.format_version -ne 1) { throw 'Unsupported package toolchain lock version.' }

New-Item -ItemType Directory -Force -Path $toolchainRoot | Out-Null
foreach ($tool in $lock.tools) {
  $archive = Join-Path $toolchainRoot $tool.archive
  if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) {
    if ($VerifyOnly) { throw "Missing package toolchain archive: $($tool.archive)" }
    Invoke-WebRequest -Uri $tool.url -OutFile $archive
  }
  $item = Get-Item -LiteralPath $archive
  $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
  if ($item.Length -ne $tool.size -or $hash -ne $tool.sha256) {
    throw "Package toolchain verification failed: $($tool.id) $($tool.version)"
  }
  if ($tool.id -eq 'inno-setup') {
    $signature = Get-AuthenticodeSignature -LiteralPath $archive
    if ($signature.Status -ne 'Valid' -or
        $signature.SignerCertificate.Subject -ne $tool.authenticode_subject -or
        $signature.SignerCertificate.Thumbprint -ne $tool.authenticode_thumbprint) {
      throw 'Inno Setup Authenticode identity verification failed.'
    }
  }
}

if (-not $VerifyOnly) {
  $innoRoot = Join-Path $toolchainRoot 'inno-7.0.2'
  if (-not (Test-Path -LiteralPath (Join-Path $innoRoot 'ISCC.exe'))) {
    & (Join-Path $toolchainRoot 'innosetup-7.0.2-x64.exe') /CURRENTUSER /PORTABLE=1 `
      /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOICONS "/DIR=$innoRoot"
    if ($LASTEXITCODE -ne 0) { throw 'Unable to install the pinned portable Inno Setup.' }
  }
}

Write-Host 'Pinned package toolchains verified.'
