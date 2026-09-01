[CmdletBinding()]
param(
  [ValidateSet('all', 'x64', 'x86')]
  [string] $Architecture = 'all'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$cmake = 'D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast\cmake-3.31.8\cmake-3.31.8-windows-x86_64\bin\cmake.exe'
if (-not (Test-Path -LiteralPath $cmake -PathType Leaf)) {
  $cmake = (Get-Command cmake -ErrorAction Stop).Source
}
$cargo = 'D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home\bin\cargo.exe'
$cargoHome = 'D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home'
$rustupHome = 'D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\rustup-home'
$sccache = 'D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast\sccache-0.17.0\sccache-v0.17.0-x86_64-pc-windows-msvc\sccache.exe'
$cargoTarget = Join-Path $repoRoot 'out/cargo-target'
$env:CARGO_HOME = $cargoHome
$env:RUSTUP_HOME = $rustupHome
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-msvc'
$env:RUSTUP_IO_THREADS = '1'
$env:CARGO_TARGET_DIR = $cargoTarget
$env:RUSTC_WRAPPER = $sccache
$architectures = if ($Architecture -eq 'all') { @('x64', 'x86') } else { @($Architecture) }

Push-Location $repoRoot
try {
  foreach ($targetArchitecture in $architectures) {
    $preset = "windows-$targetArchitecture-release"
    $buildDirectory = Join-Path $repoRoot "out/build/$preset"
    & $cmake --preset $preset -DFCITX_ENABLE_MSVC_ANALYZE=OFF
    if ($LASTEXITCODE -ne 0) { throw "Configure failed for $targetArchitecture." }
    & $cmake --build $buildDirectory --config Release --parallel
    if ($LASTEXITCODE -ne 0) { throw "Release build failed for $targetArchitecture." }

    $binaryDirectory = Join-Path $buildDirectory 'Release'
    & $cargo run --locked --release --manifest-path (Join-Path $repoRoot 'Cargo.toml') `
      -p fcitx5-protocol-core --bin fcitx5-protocol-bench --target (if ($targetArchitecture -eq 'x64') { 'x86_64-pc-windows-msvc' } else { 'i686-pc-windows-msvc' })
    if ($LASTEXITCODE -ne 0) { throw "Codec benchmark failed for $targetArchitecture." }
    & $cargo run --locked --release --manifest-path (Join-Path $repoRoot 'Cargo.toml') `
      -p fcitx5-measurement-core --bin fcitx5-key-roundtrip-bench `
      --target (if ($targetArchitecture -eq 'x64') { 'x86_64-pc-windows-msvc' } else { 'i686-pc-windows-msvc' }) `
      -- (Join-Path $binaryDirectory 'fcitx5-mock-engine.exe')
    if ($LASTEXITCODE -ne 0) { throw "Roundtrip benchmark failed for $targetArchitecture." }
    & (Join-Path $binaryDirectory 'fcitx5_focus_context_churn.exe')
    if ($LASTEXITCODE -ne 0) { throw "Focus/context churn failed for $targetArchitecture." }
    & (Join-Path $binaryDirectory 'fcitx5_handle_leak_soak.exe') `
      (Join-Path $binaryDirectory 'fcitx5-tsf.dll')
    if ($LASTEXITCODE -ne 0) { throw "Handle leak soak failed for $targetArchitecture." }
  }
} finally {
  Pop-Location
}
