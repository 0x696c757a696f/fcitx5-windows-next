[CmdletBinding()]
param(
  [switch] $Timings
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$rustRoot = Join-Path $repoRoot 'out/toolchains/rust'

Push-Location $repoRoot
try {
  & (Join-Path $PSScriptRoot 'prepare-rust.ps1')
  $env:RUSTUP_HOME = Join-Path $rustRoot 'rustup-home'
  $env:CARGO_HOME = Join-Path $rustRoot 'cargo-home'
  if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    $env:CARGO_TARGET_DIR = Join-Path $rustRoot 'target'
  }
  $rustToolchainBin = Join-Path $env:RUSTUP_HOME 'toolchains/1.98.0-x86_64-pc-windows-msvc/bin'
  $env:PATH = "$rustToolchainBin;$env:PATH"

  Write-Host 'Running fast Rust type-check for the workspace.'
  cargo check --locked --workspace --all-targets
  if ($LASTEXITCODE -ne 0) { throw 'cargo check failed.' }

  Write-Host 'Checking Rust dependency graph for duplicate crate versions.'
  cargo tree --locked --workspace --duplicates
  if ($LASTEXITCODE -ne 0) { throw 'cargo tree duplicate check failed.' }

  if ($Timings) {
    Write-Host 'Generating Cargo timing report.'
    cargo build --locked --workspace --timings
    if ($LASTEXITCODE -ne 0) { throw 'cargo build --timings failed.' }
  }
} finally {
  Pop-Location
}
