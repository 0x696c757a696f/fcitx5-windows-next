[CmdletBinding()]
param(
  [Parameter(Position = 0)]
  [ValidateSet('bootstrap', 'clean', 'dev', 'test', 'package', 'release')]
  [string] $Command = 'dev',

  [ValidateSet('all', 'x64', 'x86', 'arm64')]
  [string] $Architecture = 'all',

  [ValidateSet('Debug', 'Release')]
  [string] $Configuration = 'Debug'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$minimumCMake = [version]'3.29.0'
$buildTempRoot = Join-Path $repoRoot 'out/tmp'

function Invoke-Native {
  param(
    [Parameter(Mandatory)] [string] $Executable,
    [Parameter(Mandatory)] [string[]] $Arguments
  )

  # Some desktop hosts provide both `Path` and `PATH` in the native process
  # environment. MSBuild's .NET Framework task runner treats those names as
  # equal and aborts before invoking CL.exe. Rebuild a case-insensitive child
  # environment for every release-critical native process instead of mutating
  # the caller's environment or relying on a machine-specific shell state.
  $environment = [System.Collections.Generic.Dictionary[string, string]]::new(
    [System.StringComparer]::OrdinalIgnoreCase)
  foreach ($entry in [Environment]::GetEnvironmentVariables().GetEnumerator()) {
    $environment[[string] $entry.Key] = [string] $entry.Value
  }

  $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
  $startInfo.FileName = $Executable
  $startInfo.UseShellExecute = $false
  $startInfo.WorkingDirectory = (Get-Location).Path
  $startInfo.Environment.Clear()
  foreach ($entry in $environment.GetEnumerator()) {
    $startInfo.Environment[$entry.Key] = $entry.Value
  }
  foreach ($argument in $Arguments) {
    $startInfo.ArgumentList.Add($argument)
  }
  $process = [System.Diagnostics.Process]::Start($startInfo)
  $process.WaitForExit()
  if ($process.ExitCode -ne 0) {
    throw "Command failed ($($process.ExitCode)): $Executable $($Arguments -join ' ')"
  }
}

function Get-CMakeCommand {
  $commandInfo = Get-Command cmake -ErrorAction SilentlyContinue
  $cmakePath = if ($commandInfo) { $commandInfo.Source } else { $null }
  if (-not $cmakePath) {
    $standardPath = Join-Path $env:ProgramFiles 'CMake/bin/cmake.exe'
    if (Test-Path -LiteralPath $standardPath -PathType Leaf) {
      $cmakePath = $standardPath
    }
  }
  if (-not $cmakePath) {
    throw 'CMake 3.28+ is required. Install Visual Studio 2022 C++ tools with CMake support.'
  }

  $firstLine = (& $cmakePath --version | Select-Object -First 1)
  if ($firstLine -notmatch 'cmake version (?<version>\d+\.\d+\.\d+)') {
    throw "Unable to determine CMake version from: $firstLine"
  }

  $actualVersion = [version]$Matches.version
  if ($actualVersion -lt $minimumCMake) {
    throw "CMake $minimumCMake or newer is required; found $actualVersion."
  }

  return $cmakePath
}

function Get-Architectures {
  switch ($Architecture) {
    'x64' { return @('x64') }
    'x86' { return @('x86') }
    'arm64' { return @('arm64') }
    default { return @('x64', 'x86') }
  }
}

function Get-PresetName([string] $TargetArchitecture) {
  if ($Command -eq 'package' -and $Configuration -eq 'Release') {
    return "windows-$TargetArchitecture-release"
  }
  return "windows-$TargetArchitecture-dev"
}

function Get-BuildDirectory([string] $TargetArchitecture) {
  return Join-Path $repoRoot "out/build/$(Get-PresetName $TargetArchitecture)"
}

function Reset-BuildDirectoryIfGeneratorChanged([string] $TargetArchitecture) {
  $buildDirectory = [System.IO.Path]::GetFullPath((Get-BuildDirectory $TargetArchitecture))
  $allowedRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'out/build'))
  $allowedPrefix = $allowedRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
    [System.IO.Path]::DirectorySeparatorChar
  if (-not $buildDirectory.StartsWith($allowedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to inspect unexpected build directory: $buildDirectory"
  }

  $cachePath = Join-Path $buildDirectory 'CMakeCache.txt'
  if (-not (Test-Path -LiteralPath $cachePath -PathType Leaf)) { return }

  $generator = [System.IO.File]::ReadLines($cachePath) |
    Where-Object { $_ -like 'CMAKE_GENERATOR:INTERNAL=*' } |
    Select-Object -First 1
  if ($generator -and $generator -ne 'CMAKE_GENERATOR:INTERNAL=Ninja Multi-Config') {
    Write-Host "Removing stale non-Ninja build directory: $buildDirectory"
    Remove-Item -LiteralPath $buildDirectory -Recurse -Force
  }
}

function Get-VsWherePath {
  $path = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw 'vswhere.exe is required to locate the Visual Studio C++ toolchain.'
  }
  return $path
}

function Get-VcVarsArgument([string] $TargetArchitecture) {
  switch ($TargetArchitecture) {
    'x64' { return 'amd64' }
    'x86' { return 'amd64_x86' }
    'arm64' { return 'amd64_arm64' }
    default { throw "Unsupported architecture for Visual Studio environment: $TargetArchitecture" }
  }
}

function Import-MsvcEnvironment([string] $TargetArchitecture) {
  $vswhere = Get-VsWherePath
  $requires = @('Microsoft.VisualStudio.Component.VC.Tools.x86.x64')
  if ($TargetArchitecture -eq 'arm64') {
    $requires += 'Microsoft.VisualStudio.Component.VC.Tools.ARM64'
  }

  $vswhereArguments = @('-latest', '-products', '*')
  foreach ($component in $requires) {
    $vswhereArguments += @('-requires', $component)
  }
  $vswhereArguments += @('-property', 'installationPath')

  $installationPath = (& $vswhere @vswhereArguments | Select-Object -First 1)
  if ([string]::IsNullOrWhiteSpace($installationPath)) {
    throw "Visual Studio C++ toolchain missing for $TargetArchitecture. Install components: $($requires -join ', ')."
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
    if ($separator -le 0) {
      continue
    }
    $name = $line.Substring(0, $separator)
    $value = $line.Substring($separator + 1)
    [Environment]::SetEnvironmentVariable($name, $value, 'Process')
  }
}

function Assert-FastWindowsToolchain {
  foreach ($tool in @('ninja', 'clang-cl', 'lld-link')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
      throw "Default build requires $tool on PATH. Import Visual Studio C++ environment and install repo-local clang-cl/lld-link/Ninja via tools/prepare-fast-toolchain.ps1 -InstallLocal."
    }
  }
}

function Invoke-ConfigureAndBuild([string] $TargetArchitecture, [bool] $Analyze) {
  Import-MsvcEnvironment $TargetArchitecture
  Assert-FastWindowsToolchain
  Reset-BuildDirectoryIfGeneratorChanged $TargetArchitecture
  & (Join-Path $PSScriptRoot 'prepare-wtl.ps1')
  & (Join-Path $PSScriptRoot 'prepare-package-dependencies.ps1')
  $cmake = Get-CMakeCommand
  $preset = Get-PresetName $TargetArchitecture
  $analyzeValue = if ($Analyze) { 'ON' } else { 'OFF' }
  $configureArguments = @('--preset', $preset, "-DFCITX_ENABLE_MSVC_ANALYZE=$analyzeValue")
  if ($env:FCITX_ENABLE_SCCACHE -eq '1') {
    $sccacheCommand = Get-Command sccache -ErrorAction SilentlyContinue
    if (-not $sccacheCommand) {
      throw 'FCITX_ENABLE_SCCACHE=1 requires sccache on PATH.'
    }
    $sccachePath = [System.IO.Path]::GetFullPath($sccacheCommand.Source)
    $configureArguments += @(
      "-DCMAKE_C_COMPILER_LAUNCHER=$sccachePath",
      "-DCMAKE_CXX_COMPILER_LAUNCHER=$sccachePath"
    )
  }
  Invoke-Native $cmake $configureArguments
  # Keep MSVC child-process pressure bounded. Unbounded --parallel can fail
  # nondeterministically with D8040 on constrained runners.
  Invoke-Native $cmake @('--build', (Get-BuildDirectory $TargetArchitecture), '--config',
                         $Configuration, '--parallel', '4')
}

function Invoke-Tests([string] $TargetArchitecture) {
  $ctestInfo = Get-Command ctest -ErrorAction SilentlyContinue
  $ctestPath = if ($ctestInfo) { $ctestInfo.Source } else { $null }
  if (-not $ctestPath) {
    $ctestPath = Join-Path (Split-Path -Parent (Get-CMakeCommand)) 'ctest.exe'
  }
  if (-not (Test-Path -LiteralPath $ctestPath -PathType Leaf)) {
    throw 'CTest is required and should be installed with CMake.'
  }
  Invoke-Native $ctestPath @(
    '--test-dir', (Get-BuildDirectory $TargetArchitecture),
    '-C', $Configuration,
    '--output-on-failure'
  )
}

function Remove-BuildOutput {
  $outputPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'out'))
  $rootPrefix = $repoRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
    [System.IO.Path]::DirectorySeparatorChar
  if (-not $outputPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
      [System.IO.Path]::GetFileName($outputPath) -ne 'out') {
    throw "Refusing to clean unexpected path: $outputPath"
  }
  if (Test-Path -LiteralPath $outputPath) {
    Remove-Item -LiteralPath $outputPath -Recurse -Force
  }
  Write-Host "Clean output: $outputPath"
}

Push-Location $repoRoot
try {
  # Keep compiler, linker and packaging scratch files on the workspace drive.
  # This is especially important on development machines with a small system disk.
  New-Item -ItemType Directory -Force -Path $buildTempRoot | Out-Null
  $env:TEMP = $buildTempRoot
  $env:TMP = $buildTempRoot
  $env:RUSTUP_HOME = Join-Path $repoRoot 'out/toolchains/rust/rustup-home'
  $env:CARGO_HOME = Join-Path $repoRoot 'out/toolchains/rust/cargo-home'
  $env:RUSTUP_INIT_SKIP_PATH_CHECK = 'yes'
  $env:RUSTUP_IO_THREADS = '1'
  & (Join-Path $PSScriptRoot 'prepare-fast-toolchain.ps1') -InstallLocal
  & (Join-Path $PSScriptRoot 'configure-ci-cache.ps1')
  & (Join-Path $PSScriptRoot 'prepare-rust.ps1')
  $rustToolchainBin = Join-Path $env:RUSTUP_HOME 'toolchains/1.98.0-x86_64-pc-windows-msvc/bin'
  $env:PATH = "$rustToolchainBin;$env:PATH"
  switch ($Command) {
    'bootstrap' {
      $cmake = Get-CMakeCommand
      Write-Host "Repository: $repoRoot"
      Invoke-Native $cmake @('--version')
      & (Join-Path $PSScriptRoot 'prepare-wtl.ps1')
      & (Join-Path $PSScriptRoot 'prepare-package-dependencies.ps1')
      Write-Host 'Bootstrap check passed.'
    }
    'clean' {
      Remove-BuildOutput
    }
    'dev' {
      foreach ($targetArchitecture in Get-Architectures) {
        Invoke-ConfigureAndBuild $targetArchitecture $false
      }
    }
    'test' {
      foreach ($targetArchitecture in Get-Architectures) {
        Invoke-ConfigureAndBuild $targetArchitecture $true
        Invoke-Tests $targetArchitecture
        & (Join-Path $PSScriptRoot 'check-runtime-security.ps1') `
          -Architecture $targetArchitecture -Configuration $Configuration
      }
      & (Join-Path $PSScriptRoot 'check-secrets.ps1') -SelfTest
      & (Join-Path $PSScriptRoot 'check-secrets.ps1')
      & (Join-Path $PSScriptRoot 'check-licenses.ps1') -SelfTest
      & (Join-Path $PSScriptRoot 'check-licenses.ps1')
      & (Join-Path $PSScriptRoot 'check-dependencies.ps1')
      & (Join-Path $PSScriptRoot 'check-locales.ps1')
      & (Join-Path $PSScriptRoot 'check-text-format.ps1')
      Write-Host 'All build, test, and policy checks passed.'
    }
    'package' {
      if ($Architecture -ne 'all' -or $Configuration -ne 'Release') {
        throw 'Package requires -Architecture all -Configuration Release.'
      }
      foreach ($targetArchitecture in Get-Architectures) {
        Invoke-ConfigureAndBuild $targetArchitecture $true
        Invoke-Tests $targetArchitecture
        & (Join-Path $PSScriptRoot 'check-runtime-security.ps1') `
          -Architecture $targetArchitecture -Configuration $Configuration
      }
      & (Join-Path $PSScriptRoot 'test-fcitx.ps1') -Configuration Release
      if ($LASTEXITCODE -ne 0) { throw "Real Fcitx acceptance failed with exit code $LASTEXITCODE." }
      & (Join-Path $PSScriptRoot 'check-secrets.ps1')
      & (Join-Path $PSScriptRoot 'check-licenses.ps1')
      & (Join-Path $PSScriptRoot 'check-dependencies.ps1')
      & (Join-Path $PSScriptRoot 'check-locales.ps1')
      & (Join-Path $PSScriptRoot 'check-text-format.ps1')
      & (Join-Path $PSScriptRoot 'stage-package.ps1')
      & (Join-Path $PSScriptRoot 'test-portable.ps1')
      Write-Host 'Package gate passed using the tested Release artifacts.'
    }
    'release' {
      foreach ($name in @('FCITX_RELEASE_VERSION', 'FCITX_RELEASE_CERT_THUMBPRINT',
                           'FCITX_RELEASE_TRUSTED_KEYRING')) {
        if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) {
          throw "Release requires environment variable $name. Run package first; release never rebuilds."
        }
      }
      $channel = [Environment]::GetEnvironmentVariable('FCITX_RELEASE_CHANNEL')
      if ([string]::IsNullOrWhiteSpace($channel)) { $channel = 'stable' }
      & (Join-Path $PSScriptRoot 'release.ps1') `
        -Version $env:FCITX_RELEASE_VERSION -Channel $channel `
        -CertificateThumbprint $env:FCITX_RELEASE_CERT_THUMBPRINT `
        -TrustedKeyring $env:FCITX_RELEASE_TRUSTED_KEYRING
      if ($LASTEXITCODE -ne 0) { throw 'Release gate failed.' }
    }
  }
} finally {
  Pop-Location
}
