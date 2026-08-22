[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$rustRoot = Join-Path $repoRoot 'out/toolchains/rust'
$rustupHome = Join-Path $rustRoot 'rustup-home'
$cargoHome = Join-Path $rustRoot 'cargo-home'
$cargoTarget = Join-Path $rustRoot 'target'

New-Item -ItemType Directory -Force -Path $rustupHome, $cargoHome, $cargoTarget | Out-Null

$environment = [ordered]@{
  RUSTUP_HOME = $rustupHome
  CARGO_HOME = $cargoHome
  CARGO_TARGET_DIR = $cargoTarget
  RUSTUP_INIT_SKIP_PATH_CHECK = 'yes'
  RUSTUP_IO_THREADS = '1'
  CARGO_INCREMENTAL = '1'
  CARGO_TERM_COLOR = 'always'
  RUSTC_WRAPPER = 'sccache'
  SCCACHE_GHA_ENABLED = 'true'
  FCITX_ENABLE_SCCACHE = '1'
}

foreach ($entry in $environment.GetEnumerator()) {
  [Environment]::SetEnvironmentVariable($entry.Key, [string] $entry.Value, 'Process')
  if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_ENV)) {
    "$($entry.Key)=$($entry.Value)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  }
}

Write-Host "CI cache environment configured under $rustRoot"
