[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$secureRoot = Join-Path $repoRoot 'out/secure/task-074'
New-Item -ItemType Directory -Force -Path $secureRoot | Out-Null
$input = Join-Path $secureRoot 'unsigned-test-input.bin'
[IO.File]::WriteAllBytes($input, [byte[]](0x46, 0x43, 0x49, 0x54, 0x58, 0x35))
$certificate = $null

try {
  $certificate = New-SelfSignedCertificate -Type CodeSigningCert `
    -Subject 'CN=Fcitx5 Windows Disposable Test Credential' `
    -CertStoreLocation 'Cert:\CurrentUser\My' -NotAfter (Get-Date).AddHours(1)
  try {
    & (Join-Path $PSScriptRoot 'sign-release.ps1') -Paths $input `
      -CertificateThumbprint $certificate.Thumbprint
    throw 'Production signing unexpectedly accepted a disposable self-signed credential.'
  } catch {
    if ($_.Exception.Message -notmatch 'rejects self-signed certificates') {
      throw
    }
  }
} finally {
  if ($certificate) {
    Remove-Item "Cert:\CurrentUser\My\$($certificate.Thumbprint)" -Force -ErrorAction SilentlyContinue
  }
  Remove-Item -LiteralPath $input -Force -ErrorAction SilentlyContinue
}

Write-Host 'Production signing credential separation check passed.'
