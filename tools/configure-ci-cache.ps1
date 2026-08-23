[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$rustRoot = Join-Path $repoRoot 'out/toolchains/rust'
$rustupHome = Join-Path $rustRoot 'rustup-home'
$cargoHome = Join-Path $rustRoot 'cargo-home'
$cargoTarget = Join-Path $rustRoot 'target'
$sccacheDir = Join-Path $repoRoot 'out/toolchains/sccache'

New-Item -ItemType Directory -Force -Path $rustupHome, $cargoHome, $cargoTarget, $sccacheDir | Out-Null

$useGitHubSccache = $env:GITHUB_ACTIONS -eq 'true' -and
  -not [string]::IsNullOrWhiteSpace($env:ACTIONS_RUNTIME_TOKEN) -and
  (-not [string]::IsNullOrWhiteSpace($env:ACTIONS_CACHE_URL) -or
   -not [string]::IsNullOrWhiteSpace($env:ACTIONS_RESULTS_URL))

$environment = [ordered]@{
  RUSTUP_HOME = $rustupHome
  CARGO_HOME = $cargoHome
  CARGO_TARGET_DIR = $cargoTarget
  RUSTUP_INIT_SKIP_PATH_CHECK = 'yes'
  RUSTUP_IO_THREADS = '1'
  CARGO_TERM_COLOR = 'always'
  FCITX_ENABLE_SCCACHE = '1'
  SCCACHE_CACHE_SIZE = '30G'
  SCCACHE_IGNORE_SERVER_IO_ERROR = '1'
}

if ($useGitHubSccache) {
  $environment.RUSTC_WRAPPER = 'sccache'
  $environment.SCCACHE_GHA_ENABLED = 'true'
} else {
  [Environment]::SetEnvironmentVariable('CARGO_INCREMENTAL', $null, 'Process')
  Remove-Item Env:CARGO_INCREMENTAL -ErrorAction SilentlyContinue
  [Environment]::SetEnvironmentVariable('RUSTC_WRAPPER', $null, 'Process')
  Remove-Item Env:RUSTC_WRAPPER -ErrorAction SilentlyContinue
  [Environment]::SetEnvironmentVariable('SCCACHE_GHA_ENABLED', $null, 'Process')
  Remove-Item Env:SCCACHE_GHA_ENABLED -ErrorAction SilentlyContinue
  $environment.SCCACHE_DIR = $sccacheDir
}

foreach ($entry in $environment.GetEnumerator()) {
  [Environment]::SetEnvironmentVariable($entry.Key, [string] $entry.Value, 'Process')
  if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_ENV)) {
    "$($entry.Key)=$($entry.Value)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  }
}

Write-Host "CI cache environment configured under $rustRoot"
