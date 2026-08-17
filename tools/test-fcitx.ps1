[CmdletBinding()]
param([ValidateSet('Debug', 'Release')] [string] $Configuration = 'Debug')

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$stage = Join-Path $repoRoot 'out/stage/fcitx5'
$engine = Join-Path $stage 'bin/fcitx5-engine.exe'
$bash = Join-Path $repoRoot 'out/toolchains/msys64/usr/bin/bash.exe'
$cmakeCommand = Get-Command cmake.exe -ErrorAction SilentlyContinue
$cmake = if ($cmakeCommand) { $cmakeCommand.Source } else { $null }
if (-not $cmake) { $cmake = Join-Path $env:ProgramFiles 'CMake/bin/cmake.exe' }

foreach ($path in @($bash, $cmake, (Join-Path $stage 'lib/cmake/Fcitx5Core/Fcitx5CoreConfig.cmake'))) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Missing Fcitx test prerequisite: $path. Run tools/bootstrap-fcitx.ps1 first."
  }
}

& (Join-Path $PSScriptRoot 'bootstrap-fcitx.ps1') -VerifyOnly
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$repoMsys = '/' + $repoRoot.Substring(0, 1).ToLowerInvariant() +
  $repoRoot.Substring(2).Replace('\', '/')
$stageMsys = '/' + $stage.Substring(0, 1).ToLowerInvariant() +
  $stage.Substring(2).Replace('\', '/')
& $bash -lc "export PATH=/clang64/bin:/usr/bin; cmake -S '$repoMsys/native-engine' -B '$repoMsys/out/build/native-engine' -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX='$stageMsys' -DCMAKE_PREFIX_PATH='$stageMsys'; cmake --build '$repoMsys/out/build/native-engine' --parallel; cmake --install '$repoMsys/out/build/native-engine'"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

foreach ($architecture in @('x64', 'x86')) {
  & $cmake --preset "windows-$architecture-dev"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $build = Join-Path $repoRoot "out/build/windows-$architecture-dev"
  & $cmake --build $build --config $Configuration --target fcitx5_engine_integration_test
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & (Join-Path $build "$Configuration/fcitx5_engine_integration_test.exe") $engine
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & (Join-Path $build "$Configuration/fcitx5_engine_integration_test.exe") `
    $engine --safe-mode
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host 'Real Fcitx engine acceptance passed for x64 and x86 clients.'
