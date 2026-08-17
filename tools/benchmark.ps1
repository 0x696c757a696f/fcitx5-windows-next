[CmdletBinding()]
param(
  [ValidateSet('all', 'x64', 'x86')]
  [string] $Architecture = 'all'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$cmake = Join-Path $env:ProgramFiles 'CMake/bin/cmake.exe'
if (-not (Test-Path -LiteralPath $cmake -PathType Leaf)) {
  $cmake = (Get-Command cmake -ErrorAction Stop).Source
}
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
    & (Join-Path $binaryDirectory 'fcitx5_ipc_codec_bench.exe')
    if ($LASTEXITCODE -ne 0) { throw "Codec benchmark failed for $targetArchitecture." }
    & (Join-Path $binaryDirectory 'fcitx5_key_roundtrip_bench.exe') `
      (Join-Path $binaryDirectory 'fcitx5-mock-engine.exe')
    if ($LASTEXITCODE -ne 0) { throw "Roundtrip benchmark failed for $targetArchitecture." }
  }
} finally {
  Pop-Location
}
