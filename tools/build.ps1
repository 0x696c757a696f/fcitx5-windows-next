[CmdletBinding()]
param(
  [Parameter(Position = 0)]
  [ValidateSet('bootstrap', 'clean', 'dev', 'test')]
  [string] $Command = 'dev',

  [ValidateSet('all', 'x64', 'x86')]
  [string] $Architecture = 'all',

  [ValidateSet('Debug', 'Release')]
  [string] $Configuration = 'Debug'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$minimumCMake = [version]'3.28.0'

function Invoke-Native {
  param(
    [Parameter(Mandatory)] [string] $Executable,
    [Parameter(Mandatory)] [string[]] $Arguments
  )

  & $Executable @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed ($LASTEXITCODE): $Executable $($Arguments -join ' ')"
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
    default { return @('x64', 'x86') }
  }
}

function Get-PresetName([string] $TargetArchitecture) {
  return "windows-$TargetArchitecture-dev"
}

function Get-BuildDirectory([string] $TargetArchitecture) {
  return Join-Path $repoRoot "out/build/$(Get-PresetName $TargetArchitecture)"
}

function Invoke-ConfigureAndBuild([string] $TargetArchitecture, [bool] $Analyze) {
  $cmake = Get-CMakeCommand
  $preset = Get-PresetName $TargetArchitecture
  $analyzeValue = if ($Analyze) { 'ON' } else { 'OFF' }
  Invoke-Native $cmake @('--preset', $preset, "-DFCITX_ENABLE_MSVC_ANALYZE=$analyzeValue")
  Invoke-Native $cmake @('--build', (Get-BuildDirectory $TargetArchitecture), '--config', $Configuration, '--parallel')
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
  switch ($Command) {
    'bootstrap' {
      $cmake = Get-CMakeCommand
      Write-Host "Repository: $repoRoot"
      Invoke-Native $cmake @('--version')
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
      }
      & (Join-Path $PSScriptRoot 'check-secrets.ps1') -SelfTest
      & (Join-Path $PSScriptRoot 'check-secrets.ps1')
      & (Join-Path $PSScriptRoot 'check-licenses.ps1') -SelfTest
      & (Join-Path $PSScriptRoot 'check-licenses.ps1')
      & (Join-Path $PSScriptRoot 'check-dependencies.ps1')
      Write-Host 'All Phase 1A tests and policy checks passed.'
    }
  }
} finally {
  Pop-Location
}
