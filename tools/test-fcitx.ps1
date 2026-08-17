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
$previousMsystem = [Environment]::GetEnvironmentVariable('MSYSTEM', 'Process')
$previousChere = [Environment]::GetEnvironmentVariable('CHERE_INVOKING', 'Process')
try {
  [Environment]::SetEnvironmentVariable('MSYSTEM', 'CLANG64', 'Process')
  [Environment]::SetEnvironmentVariable('CHERE_INVOKING', '1', 'Process')
  & $bash -lc "cmake -S '$repoMsys/native-engine' -B '$repoMsys/out/build/native-engine' -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX='$stageMsys' -DCMAKE_PREFIX_PATH='$stageMsys'; cmake --build '$repoMsys/out/build/native-engine' --parallel; cmake --install '$repoMsys/out/build/native-engine'"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  [Environment]::SetEnvironmentVariable('MSYSTEM', $previousMsystem, 'Process')
  [Environment]::SetEnvironmentVariable('CHERE_INVOKING', $previousChere, 'Process')
}

$testDataParent = [IO.Path]::GetFullPath((Join-Path $repoRoot 'out/test-data'))
$testDataRoot = Join-Path $testDataParent ('fcitx-engine-' + [guid]::NewGuid().ToString('N'))
$previousUserDataRoot = [Environment]::GetEnvironmentVariable('FCITX_USER_DATA_ROOT', 'Process')
New-Item -ItemType Directory -Path $testDataRoot | Out-Null
[Environment]::SetEnvironmentVariable('FCITX_USER_DATA_ROOT', $testDataRoot, 'Process')
try {
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
  if (-not (Get-ChildItem -LiteralPath $testDataRoot -Force | Select-Object -First 1)) {
    throw 'Fcitx did not consume FCITX_USER_DATA_ROOT; isolated test data is empty.'
  }
} finally {
  [Environment]::SetEnvironmentVariable(
    'FCITX_USER_DATA_ROOT', $previousUserDataRoot, 'Process')
  $resolvedTestRoot = [IO.Path]::GetFullPath($testDataRoot)
  $testPrefix = $testDataParent.TrimEnd('\') + '\'
  if (-not $resolvedTestRoot.StartsWith($testPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to remove unexpected test data root: $resolvedTestRoot"
  }
  if (Test-Path -LiteralPath $resolvedTestRoot) {
    Remove-Item -LiteralPath $resolvedTestRoot -Recurse -Force
  }
}

Write-Host 'Real Fcitx engine acceptance passed for x64 and x86 clients.'
