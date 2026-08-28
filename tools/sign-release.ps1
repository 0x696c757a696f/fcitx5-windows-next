[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string[]] $Paths,
  [Parameter(Mandatory)] [string] $CertificateThumbprint,
  [string] $TimestampUrl = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$thumbprint = $CertificateThumbprint.Replace(' ', '').ToUpperInvariant()
if ($thumbprint -notmatch '^[0-9A-F]{40,64}$') { throw 'Signing certificate thumbprint is invalid.' }
$certificate = Get-Item "Cert:\CurrentUser\My\$thumbprint" -ErrorAction Stop
if (-not $certificate.HasPrivateKey) {
  throw 'Production signing certificate does not have an accessible private key.'
}
if ($certificate.Subject -eq $certificate.Issuer) {
  throw 'Production signing rejects self-signed certificates.'
}
$now = Get-Date
if ($now -lt $certificate.NotBefore -or $now -ge $certificate.NotAfter) {
  throw 'Production signing certificate is outside its validity period.'
}
$codeSigningOid = '1.3.6.1.5.5.7.3.3'
if ($certificate.EnhancedKeyUsageList.ObjectId.Value -notcontains $codeSigningOid) {
  throw 'Production signing certificate is missing the Code Signing EKU.'
}
$chain = [Security.Cryptography.X509Certificates.X509Chain]::new()
$chain.ChainPolicy.RevocationMode =
  [Security.Cryptography.X509Certificates.X509RevocationMode]::Online
$chain.ChainPolicy.RevocationFlag =
  [Security.Cryptography.X509Certificates.X509RevocationFlag]::EntireChain
$chain.ChainPolicy.VerificationFlags =
  [Security.Cryptography.X509Certificates.X509VerificationFlags]::NoFlag
if (-not $chain.Build($certificate)) {
  $statuses = @($chain.ChainStatus | ForEach-Object Status) -join ', '
  throw "Production signing certificate chain validation failed: $statuses"
}
$kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
$signtool = Get-ChildItem -LiteralPath $kits -Filter signtool.exe -File -Recurse |
  Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
  Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
if (-not $signtool) { throw 'SignTool was not found.' }
foreach ($path in $Paths) {
  $resolved = [IO.Path]::GetFullPath($path)
  if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) { throw "Signing input is missing: $resolved" }
  $arguments = @('sign', '/sha1', $thumbprint, '/fd', 'SHA256')
  if ($TimestampUrl) { $arguments += @('/tr', $TimestampUrl, '/td', 'SHA256') }
  $arguments += $resolved
  & $signtool @arguments
  if ($LASTEXITCODE -ne 0) { throw "SignTool failed: $resolved" }
  & $signtool verify /pa /all $resolved
  if ($LASTEXITCODE -ne 0) { throw "Authenticode verification failed: $resolved" }
}
