[CmdletBinding()]
param(
  [Parameter(Mandatory, Position = 0)]
  [ValidateSet('pr', 'package', 'desktop', 'release')]
  [string] $Gate,

  [ValidateSet('all', 'x64', 'x86', 'arm64')]
  [string] $Architecture = 'all',

  [ValidateSet('Debug', 'Release')]
  [string] $Configuration = 'Release',

  [string] $Version = '0.1.0',

  [ValidateSet('stable', 'beta', 'nightly')]
  [string] $Channel = 'stable'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$tools = $PSScriptRoot

if ($PSVersionTable.PSEdition -ne 'Core' -or $PSVersionTable.PSVersion.Major -ne 7) {
  throw "PowerShell 7 is required; found $($PSVersionTable.PSVersion)."
}

switch ($Gate) {
  'pr' {
    & (Join-Path $tools 'build.ps1') test -Architecture $Architecture `
      -Configuration $Configuration
  }
  'package' {
    & (Join-Path $tools 'build.ps1') package -Architecture all -Configuration Release `
      -Version $Version -Channel $Channel
  }
  'desktop' {
    & (Join-Path $tools 'test-desktop.ps1') -Configuration Release
  }
  'release' {
    & (Join-Path $tools 'build.ps1') release -Architecture all -Configuration Release
  }
}
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
