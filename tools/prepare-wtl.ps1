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

# Locate the ATL headers through vswhere so the same script works on a local
# Build Tools layout (C:\BuildTools) and on GitHub Actions runners
# (C:\Program Files\Microsoft Visual Studio\2022\*).
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
  throw 'Visual Studio vswhere.exe is required to locate the ATL headers.'
}
$installation = (& $vswhere -latest -products * `
  -requires Microsoft.VisualStudio.Component.VC.ATL `
  -property installationPath | Select-Object -First 1)
if (-not $installation) {
  $installation = (& $vswhere -latest -products * `
    -property installationPath | Select-Object -First 1)
}
$atlRoot = if ($installation) { Join-Path $installation 'VC/Tools/MSVC' } else { 'C:/BuildTools/VC/Tools/MSVC' }
$atl = Get-ChildItem (Join-Path $atlRoot '*/atlmfc/include/atlbase.h') -File `
  -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $atl) {
  throw ('Visual Studio ATL is required for WTL. Install component ' +
         'Microsoft.VisualStudio.Component.VC.ATL in the pinned Build Tools instance.')
}
Write-Host "Pinned WTL $($lock.version) and ATL verified."
