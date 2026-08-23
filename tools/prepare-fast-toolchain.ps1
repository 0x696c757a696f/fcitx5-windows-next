[CmdletBinding()]
param(
  [switch] $InstallForCI,
  [switch] $InstallLocal
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$fastRoot = Join-Path $repoRoot 'out/toolchains/fast'
$downloadRoot = Join-Path $fastRoot 'downloads'
$llvmVersion = '22.1.8'
$ninjaVersion = '1.13.2'
$sccacheVersion = '0.17.0'

function Add-PathForCurrentAndFutureSteps {
  param([Parameter(Mandatory)] [string] $Directory)
  if (-not (Test-Path -LiteralPath $Directory -PathType Container)) { return }
  $env:PATH = "$Directory;$env:PATH"
  if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_PATH)) {
    $Directory | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
  }
}

function Save-UriIfMissing {
  param(
    [Parameter(Mandatory)] [string] $Uri,
    [Parameter(Mandatory)] [string] $Destination,
    [int64] $MinimumBytes = 1
  )
  if (Test-Path -LiteralPath $Destination -PathType Leaf) {
    $existing = Get-Item -LiteralPath $Destination
    if ($existing.Length -ge $MinimumBytes) { return }
    Remove-Item -LiteralPath $Destination -Force
  }
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null
  Write-Host "Downloading $Uri"
  & curl.exe -L --fail --retry 5 --retry-delay 2 --output $Destination $Uri
  if ($LASTEXITCODE -ne 0) { throw "Failed to download $Uri." }
  $downloaded = Get-Item -LiteralPath $Destination
  if ($downloaded.Length -lt $MinimumBytes) {
    throw "Downloaded file is too small: $Destination ($($downloaded.Length) bytes)."
  }
}

function Expand-ZipIfMissing {
  param(
    [Parameter(Mandatory)] [string] $Archive,
    [Parameter(Mandatory)] [string] $Destination,
    [Parameter(Mandatory)] [string] $Probe
  )
  if (Test-Path -LiteralPath $Probe -PathType Leaf) { return }
  New-Item -ItemType Directory -Force -Path $Destination | Out-Null
  Expand-Archive -LiteralPath $Archive -DestinationPath $Destination -Force
}

function Expand-TarXzWith7zrIfMissing {
  param(
    [Parameter(Mandatory)] [string] $SevenZip,
    [Parameter(Mandatory)] [string] $Archive,
    [Parameter(Mandatory)] [string] $Destination,
    [Parameter(Mandatory)] [string] $Probe
  )
  if (Test-Path -LiteralPath $Probe -PathType Leaf) { return }
  if (Test-Path -LiteralPath $Destination -PathType Container) {
    Remove-Item -LiteralPath $Destination -Recurse -Force
  }
  New-Item -ItemType Directory -Force -Path $Destination | Out-Null
  $extractRoot = Join-Path $fastRoot 'extract-temp'
  New-Item -ItemType Directory -Force -Path $extractRoot | Out-Null
  $tarArchive = Join-Path $extractRoot ([System.IO.Path]::GetFileNameWithoutExtension($Archive))
  if (-not (Test-Path -LiteralPath $tarArchive -PathType Leaf) -or
      (Get-Item -LiteralPath $tarArchive).Length -lt 1000000000) {
    Remove-Item -LiteralPath $tarArchive -Force -ErrorAction SilentlyContinue
    & $SevenZip x $Archive "-o$extractRoot" -y
    if ($LASTEXITCODE -ne 0) { throw "Failed to extract xz archive $Archive." }
  }
  & tar.exe -xf $tarArchive -C $Destination
  if ($LASTEXITCODE -ne 0) { throw "Failed to extract tar archive $tarArchive." }
  Remove-Item -LiteralPath $extractRoot -Recurse -Force
  if (-not (Test-Path -LiteralPath $Probe -PathType Leaf)) {
    throw "Archive extracted but expected tool was not found: $Probe"
  }
}

function Install-LocalFastToolchain {
  New-Item -ItemType Directory -Force -Path $fastRoot, $downloadRoot | Out-Null

  $sevenZip = Join-Path $fastRoot '7zr.exe'
  Save-UriIfMissing -Uri 'https://www.7-zip.org/a/7zr.exe' -Destination $sevenZip -MinimumBytes 500000

  $llvmArchive = Join-Path $downloadRoot "clang+llvm-$llvmVersion-x86_64-pc-windows-msvc.tar.xz"
  $llvmUrl = "https://github.com/llvm/llvm-project/releases/download/llvmorg-$llvmVersion/clang+llvm-$llvmVersion-x86_64-pc-windows-msvc.tar.xz"
  $llvmDestination = Join-Path $fastRoot "llvm-$llvmVersion"
  $llvmBin = Join-Path $llvmDestination "clang+llvm-$llvmVersion-x86_64-pc-windows-msvc/bin"
  $llvmProbe = Join-Path $llvmBin 'clang-cl.exe'
  if (-not (Test-Path -LiteralPath $llvmProbe -PathType Leaf)) {
    Save-UriIfMissing -Uri $llvmUrl -Destination $llvmArchive -MinimumBytes 800000000
    Expand-TarXzWith7zrIfMissing -SevenZip $sevenZip -Archive $llvmArchive `
      -Destination $llvmDestination -Probe $llvmProbe
  }

  $ninjaArchive = Join-Path $downloadRoot "ninja-win-$ninjaVersion.zip"
  $ninjaUrl = "https://github.com/ninja-build/ninja/releases/download/v$ninjaVersion/ninja-win.zip"
  $ninjaDestination = Join-Path $fastRoot "ninja-$ninjaVersion"
  $ninjaProbe = Join-Path $ninjaDestination 'ninja.exe'
  if (-not (Test-Path -LiteralPath $ninjaProbe -PathType Leaf)) {
    Save-UriIfMissing -Uri $ninjaUrl -Destination $ninjaArchive -MinimumBytes 100000
    Expand-ZipIfMissing -Archive $ninjaArchive -Destination $ninjaDestination -Probe $ninjaProbe
  }

  $sccacheArchive = Join-Path $downloadRoot "sccache-v$sccacheVersion-x86_64-pc-windows-msvc.zip"
  $sccacheUrl = "https://github.com/mozilla/sccache/releases/download/v$sccacheVersion/sccache-v$sccacheVersion-x86_64-pc-windows-msvc.zip"
  $sccacheDestination = Join-Path $fastRoot "sccache-$sccacheVersion"
  $sccacheProbe = Join-Path $sccacheDestination "sccache-v$sccacheVersion-x86_64-pc-windows-msvc/sccache.exe"
  if (-not (Test-Path -LiteralPath $sccacheProbe -PathType Leaf)) {
    Save-UriIfMissing -Uri $sccacheUrl -Destination $sccacheArchive -MinimumBytes 1000000
    Expand-ZipIfMissing -Archive $sccacheArchive -Destination $sccacheDestination `
      -Probe $sccacheProbe
  }

  Remove-Item -LiteralPath $downloadRoot -Recurse -Force -ErrorAction SilentlyContinue
}

if ($InstallForCI) {
  $InstallLocal = $true
}

if ($InstallLocal) {
  Install-LocalFastToolchain
}

Add-PathForCurrentAndFutureSteps (Join-Path $fastRoot "llvm-$llvmVersion/clang+llvm-$llvmVersion-x86_64-pc-windows-msvc/bin")
Add-PathForCurrentAndFutureSteps (Join-Path $fastRoot "ninja-$ninjaVersion")
Add-PathForCurrentAndFutureSteps (Join-Path $fastRoot "sccache-$sccacheVersion/sccache-v$sccacheVersion-x86_64-pc-windows-msvc")
Add-PathForCurrentAndFutureSteps (Join-Path $env:ProgramFiles 'LLVM/bin')
Add-PathForCurrentAndFutureSteps (Join-Path $env:ProgramFiles 'CMake/bin')

$stillMissing = @(
  @('clang-cl', 'lld-link', 'ninja', 'sccache') |
    Where-Object { -not (Get-Command $_ -ErrorAction SilentlyContinue) }
)
if ($stillMissing.Count -ne 0) {
  Write-Host "PATH=$env:PATH"
  throw "Fast Windows toolchain is missing: $($stillMissing -join ', '). Run this script with -InstallLocal to install repo-local tools under $fastRoot."
}

Write-Host 'Fast Windows toolchain ready:'
foreach ($tool in @('clang-cl', 'lld-link', 'ninja', 'sccache')) {
  $command = Get-Command $tool -ErrorAction Stop
  Write-Host "  $tool -> $($command.Source)"
}
