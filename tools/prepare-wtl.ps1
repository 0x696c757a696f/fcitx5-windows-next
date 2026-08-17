[CmdletBinding()]
param([switch] $VerifyOnly)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$toolchainRoot = Join-Path $repoRoot 'out/toolchains'
$lock = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'wtl-toolchain.json') -Raw |
  ConvertFrom-Json
if ($lock.format_version -ne 1 -or $lock.version -ne '10.1.0') {
  throw 'Unsupported WTL toolchain lock.'
}

New-Item -ItemType Directory -Force -Path $toolchainRoot | Out-Null
$archive = Join-Path $toolchainRoot $lock.archive
if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) {
  if ($VerifyOnly) { throw "Missing pinned WTL archive: $archive" }
  Invoke-WebRequest -Uri $lock.url -OutFile $archive
}
if ((Get-Item -LiteralPath $archive).Length -ne $lock.size -or
    (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $lock.sha256) {
  throw 'WTL archive verification failed.'
}

$source = Join-Path $toolchainRoot $lock.source_directory
$include = Join-Path $source $lock.include_directory
if (-not (Test-Path -LiteralPath (Join-Path $include 'atlapp.h') -PathType Leaf)) {
  if ($VerifyOnly) { throw "Missing extracted WTL headers: $include" }
  Expand-Archive -LiteralPath $archive -DestinationPath $source
}

$atl = Get-ChildItem 'C:/BuildTools/VC/Tools/MSVC/*/atlmfc/include/atlbase.h' -File `
  -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $atl) {
  throw ('Visual Studio ATL is required for WTL. Install component ' +
         'Microsoft.VisualStudio.Component.VC.ATL in the pinned Build Tools instance.')
}
Write-Host "Pinned WTL $($lock.version) and ATL verified."
