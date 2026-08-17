[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $ReleaseManifest,
  [Parameter(Mandatory)] [string] $BaseUrl,
  [Parameter(Mandatory)] [string] $OutputDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$manifest = Get-Content -LiteralPath $ReleaseManifest -Raw | ConvertFrom-Json
if ($manifest.format_version -ne 1 -or $manifest.channel -ne 'stable') {
  throw 'System package metadata is generated only for a Stable release manifest.'
}
$installer = @($manifest.artifacts | Where-Object name -Like '*-setup.exe')
if ($installer.Count -ne 1 -or $installer[0].sha256 -notmatch '^[0-9a-f]{64}$') {
  throw 'Release manifest does not identify exactly one signed installer.'
}
$url = $BaseUrl.TrimEnd('/') + '/' + $installer[0].name
$replacements = @{ '@VERSION@' = [string]$manifest.version; '@INSTALLER_URL@' = $url;
                   '@INSTALLER_SHA256@' = [string]$installer[0].sha256 }
function Expand-Template([string] $Source, [string] $Destination) {
  $text = Get-Content -LiteralPath $Source -Raw
  foreach ($entry in $replacements.GetEnumerator()) { $text = $text.Replace($entry.Key, $entry.Value) }
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null
  [IO.File]::WriteAllText($Destination, $text, [Text.UTF8Encoding]::new($false))
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
Expand-Template (Join-Path $repoRoot 'packaging/winget/Fcitx.Fcitx5Windows.yaml.in') `
  (Join-Path $output 'winget/Fcitx.Fcitx5Windows.yaml')
Expand-Template (Join-Path $repoRoot 'packaging/chocolatey/fcitx5-windows.nuspec.in') `
  (Join-Path $output 'chocolatey/fcitx5-windows.nuspec')
Expand-Template (Join-Path $repoRoot 'packaging/chocolatey/tools/chocolateyInstall.ps1.in') `
  (Join-Path $output 'chocolatey/tools/chocolateyInstall.ps1')
Write-Host "System package metadata generated: $output"
