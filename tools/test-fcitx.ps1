[CmdletBinding()]
param([ValidateSet('Debug', 'Release')] [string] $Configuration = 'Debug')

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$stage = Join-Path $repoRoot 'out/stage/fcitx5'
$engine = Join-Path $stage 'bin/fcitx5-engine.exe'
$bash = Join-Path $repoRoot 'out/toolchains/msys64/usr/bin/bash.exe'

& (Join-Path $PSScriptRoot 'prepare-fast-toolchain.ps1') -InstallLocal

$cmakeCommand = Get-Command cmake.exe -ErrorAction SilentlyContinue
$cmake = if ($cmakeCommand) { $cmakeCommand.Source } else { $null }
if (-not $cmake) { $cmake = Join-Path $env:ProgramFiles 'CMake/bin/cmake.exe' }

function Get-VcVarsArgument([string] $TargetArchitecture) {
  switch ($TargetArchitecture) {
    'x64' { return 'amd64' }
    'x86' { return 'amd64_x86' }
    default { throw "Unsupported architecture for Visual Studio environment: $TargetArchitecture" }
  }
}

function Import-MsvcEnvironment([string] $TargetArchitecture) {
  $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
  if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
    throw 'vswhere.exe is required to locate the Visual Studio C++ toolchain.'
  }
  $components = @('Microsoft.VisualStudio.Component.VC.Tools.x86.x64')
  $arguments = @('-latest', '-products', '*')
  foreach ($component in $components) {
    $arguments += @('-requires', $component)
  }
  $arguments += @('-property', 'installationPath')
  $installationPath = (& $vswhere @arguments | Select-Object -First 1)
  if ([string]::IsNullOrWhiteSpace($installationPath)) {
    throw "Visual Studio C++ toolchain missing for $TargetArchitecture."
  }
  $vcvars = Join-Path $installationPath 'VC/Auxiliary/Build/vcvarsall.bat'
  if (-not (Test-Path -LiteralPath $vcvars -PathType Leaf)) {
    throw "vcvarsall.bat not found under Visual Studio installation: $installationPath"
  }
  $vcvarsArgument = Get-VcVarsArgument $TargetArchitecture
  $command = "`"$vcvars`" $vcvarsArgument >nul && set"
  $environmentLines = & $env:ComSpec /d /s /c $command
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to import Visual Studio environment for $TargetArchitecture using $vcvarsArgument."
  }
  foreach ($line in $environmentLines) {
    $separator = $line.IndexOf('=')
    if ($separator -le 0) { continue }
    [Environment]::SetEnvironmentVariable(
      $line.Substring(0, $separator), $line.Substring($separator + 1), 'Process')
  }
}

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
  & $bash -lc "set -e; cmake -S '$repoMsys/native-engine' -B '$repoMsys/out/build/native-engine' -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX='$stageMsys' -DCMAKE_PREFIX_PATH='$stageMsys'; cmake --build '$repoMsys/out/build/native-engine' --parallel; cmake --install '$repoMsys/out/build/native-engine'"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  [Environment]::SetEnvironmentVariable('MSYSTEM', $previousMsystem, 'Process')
  [Environment]::SetEnvironmentVariable('CHERE_INVOKING', $previousChere, 'Process')
}

$testDataParent = [IO.Path]::GetFullPath((Join-Path $repoRoot 'out/test-data'))
$previousUserDataRoot = [Environment]::GetEnvironmentVariable('FCITX_USER_DATA_ROOT', 'Process')
$previousLuaMarker = [Environment]::GetEnvironmentVariable('FCITX_LUA_TEST_MARKER', 'Process')
$testRoots = @()
try {
  foreach ($architecture in @('x64', 'x86')) {
    Import-MsvcEnvironment $architecture
    $testDataRoot = Join-Path $testDataParent ('fcitx-engine-' + [guid]::NewGuid().ToString('N'))
    $testRoots += $testDataRoot
    New-Item -ItemType Directory -Path $testDataRoot | Out-Null
    [Environment]::SetEnvironmentVariable('FCITX_USER_DATA_ROOT', $testDataRoot, 'Process')
    $luaExtensionDirectory = Join-Path $testDataRoot 'Fcitx5/lua/imeapi/extensions'
    $luaMarker = Join-Path $testDataRoot 'fcitx5-lua.marker'
    New-Item -ItemType Directory -Force -Path $luaExtensionDirectory | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot 'tests/fixtures/fcitx5-lua/functional.lua') `
      -Destination $luaExtensionDirectory -Force
    [Environment]::SetEnvironmentVariable('FCITX_LUA_TEST_MARKER', $luaMarker, 'Process')
    & $cmake --preset "windows-$architecture-dev"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $build = Join-Path $repoRoot "out/build/windows-$architecture-dev"
    & $cmake --build $build --config $Configuration --target fcitx5_engine_integration_test fcitx5_ui
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Copy-Item -LiteralPath (Join-Path $build "$Configuration/fcitx5-ui.exe") `
      -Destination (Join-Path $stage 'bin/fcitx5-ui.exe') -Force
    Write-Host "Running real Fcitx acceptance: $architecture baseline"
    & (Join-Path $build "$Configuration/fcitx5_engine_integration_test.exe") $engine
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host "Running real Fcitx acceptance: $architecture typing-fuzz"
    & (Join-Path $build "$Configuration/fcitx5_engine_integration_test.exe") `
      $engine --typing-fuzz
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host "Running real Fcitx acceptance: $architecture chttrans"
    & (Join-Path $build "$Configuration/fcitx5_engine_integration_test.exe") `
      $engine --chttrans
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host "Running real Fcitx acceptance: $architecture safe-mode"
    & (Join-Path $build "$Configuration/fcitx5_engine_integration_test.exe") `
      $engine --safe-mode
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    if (-not (Test-Path -LiteralPath $luaMarker -PathType Leaf) -or
        (Get-Content -LiteralPath $luaMarker -Raw) -ne "fcitx5-lua-ok`n") {
      throw "fcitx5-lua did not execute the isolated functional extension for $architecture."
    }
  }
  $rimeTestDataRoot = Join-Path $testDataParent ('fcitx-engine-' + [guid]::NewGuid().ToString('N'))
  $testRoots += $rimeTestDataRoot
  New-Item -ItemType Directory -Path $rimeTestDataRoot | Out-Null
  [Environment]::SetEnvironmentVariable('FCITX_USER_DATA_ROOT', $rimeTestDataRoot, 'Process')
  $luaExtensionDirectory = Join-Path $rimeTestDataRoot 'Fcitx5/lua/imeapi/extensions'
  $rimeUserDirectory = Join-Path $rimeTestDataRoot 'Fcitx5/rime'
  New-Item -ItemType Directory -Force -Path $luaExtensionDirectory, $rimeUserDirectory | Out-Null
  Copy-Item -LiteralPath (Join-Path $repoRoot 'tests/fixtures/fcitx5-lua/functional.lua') `
    -Destination $luaExtensionDirectory -Force
  $rimeLuaMarker = Join-Path $rimeTestDataRoot 'fcitx5-lua.marker'
  [Environment]::SetEnvironmentVariable('FCITX_LUA_TEST_MARKER', $rimeLuaMarker, 'Process')
  New-Item -ItemType Directory -Force -Path $rimeUserDirectory | Out-Null
  Get-ChildItem -LiteralPath (Join-Path $repoRoot 'tests/fixtures/rime-lua') -File |
    Copy-Item -Destination $rimeUserDirectory -Force
  & $engine --set-input-method rime
  if ($LASTEXITCODE -ne 0) { throw 'Could not select Rime in the isolated profile.' }
  foreach ($architecture in @('x64', 'x86')) {
    $build = Join-Path $repoRoot "out/build/windows-$architecture-dev"
    Write-Host "Running real Fcitx acceptance: $architecture rime-lua"
    & (Join-Path $build "$Configuration/fcitx5_engine_integration_test.exe") `
      $engine --rime-lua
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  }
  foreach ($required in @('build/luna_pinyin.schema.yaml', 'build/luna_pinyin.prism.bin')) {
    if (-not (Test-Path -LiteralPath (Join-Path $rimeUserDirectory $required) -PathType Leaf)) {
      throw "Rime did not deploy required artifact: $required"
    }
  }
  if (-not (Get-ChildItem -LiteralPath $rimeTestDataRoot -Force | Select-Object -First 1)) {
    throw 'Fcitx did not consume FCITX_USER_DATA_ROOT; isolated test data is empty.'
  }
} finally {
  [Environment]::SetEnvironmentVariable(
    'FCITX_USER_DATA_ROOT', $previousUserDataRoot, 'Process')
  [Environment]::SetEnvironmentVariable(
    'FCITX_LUA_TEST_MARKER', $previousLuaMarker, 'Process')
  foreach ($testRoot in $testRoots) {
    $resolvedTestRoot = [IO.Path]::GetFullPath($testRoot)
    $testPrefix = $testDataParent.TrimEnd('\') + '\'
    if (-not $resolvedTestRoot.StartsWith($testPrefix, [StringComparison]::OrdinalIgnoreCase)) {
      throw "Refusing to remove unexpected test data root: $resolvedTestRoot"
    }
    if (Test-Path -LiteralPath $resolvedTestRoot) {
      Remove-Item -LiteralPath $resolvedTestRoot -Recurse -Force
    }
  }
}

Write-Host 'Real Fcitx engine acceptance passed for x64 and x86 clients.'
