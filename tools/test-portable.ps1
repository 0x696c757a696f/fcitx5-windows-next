[CmdletBinding()]
param([string] $Version = '0.1.0')

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$outRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'out'))
$zip = Join-Path $outRoot "package/artifacts/fcitx5-windows-$Version-portable.zip"
if (-not (Test-Path -LiteralPath $zip -PathType Leaf)) { throw "Missing portable ZIP: $zip" }

$smokeRoot = Join-Path $outRoot ('portable-smoke-' + [guid]::NewGuid().ToString('N'))
$first = Join-Path $smokeRoot 'location-a'
$second = Join-Path $smokeRoot 'location-b'
New-Item -ItemType Directory -Path $first, $second -Force | Out-Null
try {
  Expand-Archive -LiteralPath $zip -DestinationPath $first
  $app = Join-Path $first 'Fcitx5'
  foreach ($location in @('first', 'moved')) {
    $config = Start-Process -FilePath (Join-Path $app 'bin/fcitx5-config.exe') `
      -ArgumentList '--self-test' -Wait -PassThru -WindowStyle Hidden
    if ($config.ExitCode -ne 0) { throw "Config self-test failed at $location location." }
    $status = (& (Join-Path $app 'bin/fcitx5-control.exe') --status) | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw "Control status failed at $location location." }
    $actual = [IO.Path]::GetFullPath($status.data_root.Replace('/', '\')).TrimEnd('\')
    $expected = [IO.Path]::GetFullPath((Join-Path $app 'data')).TrimEnd('\')
    if ($actual -ne $expected) {
      throw "Portable data root mismatch at $location location: $actual != $expected"
    }
    if ($location -eq 'first') {
      $moved = Join-Path $second 'Fcitx5'
      Move-Item -LiteralPath $app -Destination $moved
      $app = $moved
    }
  }
  Write-Host 'Portable ZIP self-test and move test passed.'
} finally {
  $resolved = [IO.Path]::GetFullPath($smokeRoot)
  $prefix = $outRoot.TrimEnd('\') + '\portable-smoke-'
  if ($resolved.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -and
      (Test-Path -LiteralPath $resolved)) {
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
}
