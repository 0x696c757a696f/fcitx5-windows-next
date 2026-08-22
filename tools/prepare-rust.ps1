[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$rustRoot = Join-Path $repoRoot 'out/toolchains/rust'
$rustupHome = Join-Path $rustRoot 'rustup-home'
$cargoHome = Join-Path $rustRoot 'cargo-home'
$toolchain = '1.98.0-x86_64-pc-windows-msvc'
$targets = @(
  'x86_64-pc-windows-msvc',
  'i686-pc-windows-msvc',
  'aarch64-pc-windows-msvc'
)

function Invoke-RustupRetry {
  param(
    [Parameter(Mandatory)] [string[]] $Arguments
  )

  $maximumAttempts = 4
  for ($attempt = 1; $attempt -le $maximumAttempts; ++$attempt) {
    & $script:RustupCommand @Arguments
    if ($LASTEXITCODE -eq 0) {
      return
    }

    $downloads = Join-Path $rustupHome 'downloads'
    if (Test-Path -LiteralPath $downloads) {
      Get-ChildItem -LiteralPath $downloads -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -eq 'downloaded' -or $_.Name.EndsWith('.partial') } |
        Remove-Item -Force -ErrorAction SilentlyContinue
    }

    if ($attempt -eq $maximumAttempts) {
      throw "rustup failed after $maximumAttempts attempts: rustup $($Arguments -join ' ')"
    }

    Start-Sleep -Seconds ([Math]::Min(30, 5 * $attempt))
  }
}

New-Item -ItemType Directory -Force -Path $rustupHome, $cargoHome | Out-Null
$env:RUSTUP_HOME = $rustupHome
$env:CARGO_HOME = $cargoHome
$env:RUSTUP_INIT_SKIP_PATH_CHECK = 'yes'
$env:RUSTUP_IO_THREADS = '1'

$repoRustup = Join-Path $cargoHome 'bin/rustup.exe'
$rustup = if (Test-Path -LiteralPath $repoRustup -PathType Leaf) {
  $repoRustup
} else {
  $command = Get-Command rustup -ErrorAction SilentlyContinue
  if ($command) { $command.Source } else { $null }
}
if (-not $rustup) {
  throw 'rustup is required to prepare the pinned Rust toolchain.'
}
$script:RustupCommand = $rustup

Invoke-RustupRetry -Arguments @('toolchain', 'install', $toolchain, '--profile', 'minimal',
  '--component', 'rustfmt', '--component', 'clippy', '--no-self-update')

foreach ($target in $targets) {
  Invoke-RustupRetry -Arguments @('target', 'add', $target, '--toolchain', $toolchain)
}

Write-Host "Pinned Rust toolchain verified: $toolchain"
