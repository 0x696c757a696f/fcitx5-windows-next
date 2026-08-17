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
